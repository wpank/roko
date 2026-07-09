# W14-A: Compose & Template System Fixes

**Priority**: P2 -- architecture correctness
**Effort**: 2-3 hours
**Files to modify**: 3 files + 7 template files
**Dependencies**: None
**IMPROVEMENTS**: 10.1, 10.2, 10.3, 10.4, 10.5

## Problem

Five architectural issues in the compose/template subsystem:

1. **10.1**: `section_budget_cap()` only handles 5 of 11 section names. `role_identity`, `task_context`, `affect_guidance`, and `tool_hints` fall through to `None` -- no cap even with a `PromptBudget` set. A large `role_identity` (e.g., huge AGENTS.md) can exhaust the entire token budget.

2. **10.2**: The `build_with_counter` selection loop calls `candidate_fits` for each candidate, which calls `assemble_selected_sections` (full sort + reassembly) to measure token count. With N sections, this is O(N^2 log N).

3. **10.3**: Three separate `match` blocks enumerate section names as string literals (`section_order_rank`, `section_budget_cap`, `render_section`). Adding a section requires updating all three. `tool_hints` is already missing from `section_order_rank` (gets rank 11) and `section_budget_cap` (no cap).

4. **10.4**: Every template pushes `agents_instructions` with identical `SectionPriority::Critical`, `CacheLayer::Role`, `Placement::Start`. Copy-paste across 7 templates.

5. **10.5**: `RELEVANT_TECHNIQUES_TOKEN_BUDGET = 500` limits the greedy-fill loop. Then `apply_budget_profile` applies `budget.skills` (e.g., 8,000 chars) as a hard cap. The inner 500-token limit always wins, making the budget profile cap irrelevant. The two limits operate on different units (tokens vs chars).

## Root Cause

The compose system grew organically with each section added independently. No single source of truth for section metadata.

## Exact Code to Change

### Fix 10.1 -- Add budget caps for missing sections

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs`
**Lines**: 823-832

**Find this code:**
```rust
    fn section_budget_cap(&self, section_name: &str) -> Option<usize> {
        let budget = self.budget_profile?;
        match section_name {
            "conventions" | "tool_instructions" | "anti_patterns" => Some(budget.instructions),
            "domain_context" | "context_layer" | "pheromone_signals" => Some(budget.context),
            "gate_feedback" => Some(budget.context),
            "relevant_techniques" => Some(budget.skills),
            _ => None,
        }
    }
```

**Replace with:**
```rust
    fn section_budget_cap(&self, section_name: &str) -> Option<usize> {
        let budget = self.budget_profile?;
        match section_name {
            "conventions" | "tool_instructions" | "anti_patterns" => Some(budget.instructions),
            "domain_context" | "context_layer" | "pheromone_signals" => Some(budget.context),
            "gate_feedback" => Some(budget.context),
            "relevant_techniques" | "tool_hints" => Some(budget.skills),
            "role_identity" | "agents_instructions" => Some(budget.plan.min(8_000)),
            "task_context" => Some(budget.plan),
            "affect_guidance" => Some(budget.instructions),
            _ => {
                tracing::debug!(section_name, "no budget cap for section");
                None
            }
        }
    }
```

### Fix 10.2 -- Cache assembled prefix in candidate_fits

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs`
**Lines**: 1134-1146

**Find this code:**
```rust
fn candidate_fits(
    sections: &[RenderedSection],
    kept: &[Option<String>],
    index: usize,
    candidate: &str,
    cache_markers: bool,
    token_budget: usize,
    counter: &TokenCounter,
) -> bool {
    let mut next = kept.to_vec();
    next[index] = Some(candidate.to_string());
    counter.count(&assemble_selected_sections(sections, &next, cache_markers)) <= token_budget
}
```

**Replace with:**
```rust
fn candidate_fits(
    _sections: &[RenderedSection],
    _kept: &[Option<String>],
    _index: usize,
    candidate: &str,
    _cache_markers: bool,
    token_budget: usize,
    counter: &TokenCounter,
    cached_assembly: &str,
) -> bool {
    // Incremental measurement: append only the candidate to the already-assembled prefix.
    // This avoids the O(N) reassembly per probe that caused O(N^2 log N) overall.
    let probe = if cached_assembly.is_empty() {
        candidate.to_string()
    } else {
        format!("{cached_assembly}\n\n{candidate}")
    };
    tracing::debug!(
        token_budget,
        probe_len = probe.len(),
        "candidate_fits: incremental token check"
    );
    counter.count(&probe) <= token_budget
}
```

**Also update the call site in `truncate_to_fit` (lines 1148-1190).**

`truncate_to_fit` also calls `candidate_fits` and must pass the new `cached_assembly` parameter.

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs`
**Lines**: 1148-1190

**Find this code:**
```rust
fn truncate_to_fit(
    sections: &[RenderedSection],
    kept: &[Option<String>],
    index: usize,
    rendered: &str,
    cache_markers: bool,
    token_budget: usize,
    counter: &TokenCounter,
) -> Option<String> {
    let mut boundaries = rendered
        .char_indices()
        .map(|(boundary, _)| boundary)
        .collect::<Vec<_>>();
    boundaries.push(rendered.len());

    let mut low = 0usize;
    let mut high = boundaries.len();
    let mut best = None;

    while low < high {
        let mid = (low + high) / 2;
        let candidate = &rendered[..boundaries[mid]];

        if !candidate.is_empty()
            && candidate_fits(
                sections,
                kept,
                index,
                candidate,
                cache_markers,
                token_budget,
                counter,
            )
        {
            best = Some(candidate.to_string());
            low = mid + 1;
        } else {
            high = mid;
        }
    }

    best
}
```

**Replace with:**
```rust
fn truncate_to_fit(
    sections: &[RenderedSection],
    kept: &[Option<String>],
    index: usize,
    rendered: &str,
    cache_markers: bool,
    token_budget: usize,
    counter: &TokenCounter,
    cached_assembly: &str,
) -> Option<String> {
    let mut boundaries = rendered
        .char_indices()
        .map(|(boundary, _)| boundary)
        .collect::<Vec<_>>();
    boundaries.push(rendered.len());

    let mut low = 0usize;
    let mut high = boundaries.len();
    let mut best = None;

    while low < high {
        let mid = (low + high) / 2;
        let candidate = &rendered[..boundaries[mid]];

        if !candidate.is_empty()
            && candidate_fits(
                sections,
                kept,
                index,
                candidate,
                cache_markers,
                token_budget,
                counter,
                cached_assembly,
            )
        {
            best = Some(candidate.to_string());
            low = mid + 1;
        } else {
            high = mid;
        }
    }

    best
}
```

---

**Also update the call site in `build_with_counter`.**

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs`
**Lines**: 381-423

**Find this code:**
```rust
        let mut kept = vec![None; rendered_sections.len()];
        let mut selection_order = (0..rendered_sections.len()).collect::<Vec<_>>();
        selection_order.sort_by(|&a, &b| {
            rendered_sections[b]
                .section
                .priority
                .cmp(&rendered_sections[a].section.priority)
                .then_with(|| {
                    rendered_sections[a]
                        .section
                        .cache_layer
                        .cmp(&rendered_sections[b].section.cache_layer)
                })
                .then_with(|| a.cmp(&b))
        });

        for index in selection_order {
            let rendered = &rendered_sections[index].rendered;
            if candidate_fits(
                &rendered_sections,
                &kept,
                index,
                rendered,
                self.cache_markers,
                token_budget,
                counter,
            ) {
                kept[index] = Some(rendered.clone());
                continue;
            }

            if rendered_sections[index].section.priority == SectionPriority::Critical {
                kept[index] = truncate_to_fit(
                    &rendered_sections,
                    &kept,
                    index,
                    rendered,
                    self.cache_markers,
                    token_budget,
                    counter,
                );
            }
        }
```

**Replace with:**
```rust
        let mut kept = vec![None; rendered_sections.len()];
        let mut selection_order = (0..rendered_sections.len()).collect::<Vec<_>>();
        selection_order.sort_by(|&a, &b| {
            rendered_sections[b]
                .section
                .priority
                .cmp(&rendered_sections[a].section.priority)
                .then_with(|| {
                    rendered_sections[a]
                        .section
                        .cache_layer
                        .cmp(&rendered_sections[b].section.cache_layer)
                })
                .then_with(|| a.cmp(&b))
        });

        // Maintain a cached assembly of already-accepted sections to avoid O(N^2) reassembly.
        let mut cached_assembly = String::new();

        for index in selection_order {
            let rendered = &rendered_sections[index].rendered;
            if candidate_fits(
                &rendered_sections,
                &kept,
                index,
                rendered,
                self.cache_markers,
                token_budget,
                counter,
                &cached_assembly,
            ) {
                kept[index] = Some(rendered.clone());
                // Update the cached assembly incrementally.
                if !cached_assembly.is_empty() {
                    cached_assembly.push_str("\n\n");
                }
                cached_assembly.push_str(rendered);
                continue;
            }

            if rendered_sections[index].section.priority == SectionPriority::Critical {
                let truncated = truncate_to_fit(
                    &rendered_sections,
                    &kept,
                    index,
                    rendered,
                    self.cache_markers,
                    token_budget,
                    counter,
                    &cached_assembly,
                );
                if let Some(ref text) = truncated {
                    if !cached_assembly.is_empty() {
                        cached_assembly.push_str("\n\n");
                    }
                    cached_assembly.push_str(text);
                }
                kept[index] = truncated;
            }
        }
```

### Fix 10.3 -- Unify section registries into SectionSpec table

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs`
**Lines**: 1192-1207

Add this struct and constant table immediately **before** the existing `section_order_rank` function (i.e., before line 1192):

```rust
/// Single source of truth for section metadata.
///
/// All three section-name registries (`section_order_rank`, `section_budget_cap`,
/// `render_section`) are derived from this table. Adding a new section means
/// adding one entry here.
struct SectionSpec {
    name: &'static str,
    order_rank: u8,
    heading: Option<&'static str>,
}

const SECTION_SPECS: &[SectionSpec] = &[
    SectionSpec { name: "role_identity",       order_rank: 0,  heading: None },
    SectionSpec { name: "conventions",         order_rank: 1,  heading: Some("## Project Conventions") },
    SectionSpec { name: "tool_instructions",   order_rank: 2,  heading: Some("## Tool Instructions") },
    SectionSpec { name: "domain_context",      order_rank: 3,  heading: Some("## Domain Context") },
    SectionSpec { name: "context_layer",       order_rank: 4,  heading: None },
    SectionSpec { name: "pheromone_signals",   order_rank: 5,  heading: Some("## Active Signals") },
    SectionSpec { name: "task_context",        order_rank: 6,  heading: Some("## Current Task") },
    SectionSpec { name: "gate_feedback",       order_rank: 7,  heading: None },  // self-prefixed check
    SectionSpec { name: "relevant_techniques", order_rank: 8,  heading: None },
    SectionSpec { name: "anti_patterns",       order_rank: 9,  heading: Some("## Anti-Patterns") },
    SectionSpec { name: "affect_guidance",     order_rank: 10, heading: Some("## Affect Guidance") },
    SectionSpec { name: "tool_hints",          order_rank: 11, heading: None },
];

fn spec_for(name: &str) -> Option<&'static SectionSpec> {
    SECTION_SPECS.iter().find(|s| s.name == name)
}
```

Then **replace** the `section_order_rank` function at lines 1192-1207:

**Find this code:**
```rust
fn section_order_rank(name: &str) -> u8 {
    match name {
        "role_identity" => 0,
        "conventions" => 1,
        "tool_instructions" => 2,
        "domain_context" => 3,
        "context_layer" => 4,
        "pheromone_signals" => 5,
        "task_context" => 6,
        "gate_feedback" => 7,
        "relevant_techniques" => 8,
        "anti_patterns" => 9,
        "affect_guidance" => 10,
        _ => 11,
    }
}
```

**Replace with:**
```rust
fn section_order_rank(name: &str) -> u8 {
    spec_for(name).map_or(12, |s| s.order_rank)
}
```

Then **replace** the `render_section` function at lines 1076-1096:

**Find this code:**
```rust
fn render_section(section: &PromptSection) -> String {
    match section.name.as_str() {
        "role_identity" => section.content.clone(),
        "conventions" => format!("## Project Conventions\n\n{}", section.content),
        "tool_instructions" => format!("## Tool Instructions\n\n{}", section.content),
        "domain_context" => format!("## Domain Context\n\n{}", section.content),
        "relevant_techniques" => section.content.clone(),
        "pheromone_signals" => format!("## Active Signals\n\n{}", section.content),
        "anti_patterns" => format!("## Anti-Patterns\n\n{}", section.content),
        "affect_guidance" => format!("## Affect Guidance\n\n{}", section.content),
        "task_context" => format!("## Current Task\n\n{}", section.content),
        "gate_feedback" => {
            if section.content.trim_start().starts_with("## ") {
                section.content.clone()
            } else {
                format!("## Gate Feedback\n\n{}", section.content)
            }
        }
        _ => section.content.clone(),
    }
}
```

**Replace with:**
```rust
fn render_section(section: &PromptSection) -> String {
    // gate_feedback has a special self-prefix check
    if section.name == "gate_feedback" {
        return if section.content.trim_start().starts_with("## ") {
            section.content.clone()
        } else {
            format!("## Gate Feedback\n\n{}", section.content)
        };
    }

    match spec_for(&section.name) {
        Some(spec) => match spec.heading {
            Some(heading) => format!("{heading}\n\n{}", section.content),
            None => section.content.clone(),
        },
        None => section.content.clone(),
    }
}
```

### Fix 10.4 -- DRY agents_instructions across templates

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/common.rs`

Add this helper function after the `budget_for` function (after the closing `}` of `budget_for`, which is around line 100+):

```rust
/// Build the standard `agents_instructions` section used by all role templates.
///
/// This is the canonical constructor -- all templates should use this instead
/// of manually building the section to avoid drift in priority/cache/placement.
pub fn agents_instructions_section(agents_md: &str) -> PromptSection {
    PromptSection::new("agents_instructions", agents_md)
        .with_priority(SectionPriority::Critical)
        .with_cache_layer(CacheLayer::Role)
        .with_placement(Placement::Start)
}
```

**Also add** these imports to the top of `common.rs` (after the existing `use roko_core::AgentRole;` on line 8):

```rust
use crate::prompt::{CacheLayer, Placement, PromptSection, SectionPriority};
```

Then update each of 7 template files. Each file has the same pattern but different line numbers and slightly different comment text.

---

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/implementer.rs` (lines 79-85)

**Find this code:**
```rust
        // 1. agents_instructions — System / Critical
        sections.push(
            PromptSection::new("agents_instructions", &input.agents_md)
                .with_priority(SectionPriority::Critical)
                .with_cache_layer(CacheLayer::Role)
                .with_placement(Placement::Start),
        );
```

**Replace with:**
```rust
        // 1. agents_instructions — System / Critical / Start
        sections.push(common::agents_instructions_section(&input.agents_md));
```

**Also update import** on line 5. Find:
```rust
use super::common::budget_for;
```
Replace with:
```rust
use super::common::{self, budget_for};
```

---

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/strategist.rs` (lines 79-85)

**Find this code:**
```rust
        // 1. agents_instructions — System / Critical / Start
        sections.push(
            PromptSection::new("agents_instructions", &input.agents_md)
                .with_priority(SectionPriority::Critical)
                .with_cache_layer(CacheLayer::Role)
                .with_placement(Placement::Start),
        );
```

**Replace with:**
```rust
        // 1. agents_instructions — System / Critical / Start
        sections.push(common::agents_instructions_section(&input.agents_md));
```

**Also update import** on line 8. Find:
```rust
use super::common::budget_for;
```
Replace with:
```rust
use super::common::{self, budget_for};
```

---

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/reviewer.rs` (lines 121-127)

**Find this code:**
```rust
        // 1. agents_instructions — System / Critical
        sections.push(
            PromptSection::new("agents_instructions", &input.agents_md)
                .with_priority(SectionPriority::Critical)
                .with_cache_layer(CacheLayer::Role)
                .with_placement(Placement::Start),
        );
```

**Replace with:**
```rust
        // 1. agents_instructions — System / Critical / Start
        sections.push(common::agents_instructions_section(&input.agents_md));
```

**Also update import** on line 7. Find:
```rust
use super::common::budget_for;
```
Replace with:
```rust
use super::common::{self, budget_for};
```

---

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/scribe.rs` (lines 118-124)

**Find this code:**
```rust
        // 1. agents_instructions — System / Critical
        sections.push(
            PromptSection::new("agents_instructions", &input.agents_md)
                .with_priority(SectionPriority::Critical)
                .with_cache_layer(CacheLayer::Role)
                .with_placement(Placement::Start),
        );
```

**Replace with:**
```rust
        // 1. agents_instructions — System / Critical / Start
        sections.push(common::agents_instructions_section(&input.agents_md));
```

**Also update import** on line 6. Find:
```rust
use super::common::budget_for;
```
Replace with:
```rust
use super::common::{self, budget_for};
```

---

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/quick.rs` (lines 70-76)

**Find this code:**
```rust
        // 1. agents_instructions — System / Critical / Start
        sections.push(
            PromptSection::new("agents_instructions", &input.agents_md)
                .with_priority(SectionPriority::Critical)
                .with_cache_layer(CacheLayer::Role)
                .with_placement(Placement::Start),
        );
```

**Replace with:**
```rust
        // 1. agents_instructions — System / Critical / Start
        sections.push(common::agents_instructions_section(&input.agents_md));
```

**Also update import** on line 14. Find:
```rust
use super::common::{budget_for, format_prior_review, format_verdict_instructions};
```
Replace with:
```rust
use super::common::{self, budget_for, format_prior_review, format_verdict_instructions};
```

---

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/integration.rs` (lines 62-68)

**Find this code:**
```rust
        // 1. agents_instructions — System / Critical / Start
        sections.push(
            PromptSection::new("agents_instructions", &input.agents_md)
                .with_priority(SectionPriority::Critical)
                .with_cache_layer(CacheLayer::Role)
                .with_placement(Placement::Start),
        );
```

**Replace with:**
```rust
        // 1. agents_instructions — System / Critical / Start
        sections.push(common::agents_instructions_section(&input.agents_md));
```

**Also update import** on line 8. Find:
```rust
use super::common::budget_for;
```
Replace with:
```rust
use super::common::{self, budget_for};
```

---

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/task_impl.rs` (lines 107-113)

Note: `task_impl.rs` uses 4-space indent (not 8-space) because `push_base_sections` is a free function.

**Find this code:**
```rust
    // 1. agents_instructions — System / Critical / Start
    sections.push(
        PromptSection::new("agents_instructions", &input.agents_md)
            .with_priority(SectionPriority::Critical)
            .with_cache_layer(CacheLayer::Role)
            .with_placement(Placement::Start),
    );
```

**Replace with:**
```rust
    // 1. agents_instructions — System / Critical / Start
    sections.push(common::agents_instructions_section(&input.agents_md));
```

**Also update import** on line 9. Find:
```rust
use super::common::budget_for;
```
Replace with:
```rust
use super::common::{self, budget_for};
```

### Fix 10.5 -- Unify relevant_techniques budget source

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs`

**Step 1**: Remove the constant at line 107.

**Find this code:**
```rust
const RELEVANT_TECHNIQUES_TOKEN_BUDGET: usize = 500;
```

**Replace with:**
(delete this line entirely)

**Step 2**: Rewrite `relevant_techniques_section` at lines 743-807.

**Find this code:**
```rust
    fn relevant_techniques_section(&self) -> Option<PromptSection> {
        if self.relevant_skills.is_empty() && self.relevant_playbooks.is_empty() {
            return None;
        }

        let mut rendered = String::from("## Relevant Techniques");
        let mut kept_playbooks = 0usize;
        let mut kept_skills = 0usize;
        let mut total_tokens = estimate_tokens(&rendered);

        for playbook in self.relevant_playbooks.iter().take(3) {
            let block = render_playbook(playbook);
            let candidate = format!("{rendered}\n\n{block}");
            let candidate_tokens = estimate_tokens(&candidate);
            if candidate_tokens > RELEVANT_TECHNIQUES_TOKEN_BUDGET {
                break;
            }
            rendered = candidate;
            kept_playbooks += 1;
            total_tokens = candidate_tokens;
        }

        for skill in &self.relevant_skills {
            let block = render_skill(skill);
            let candidate = format!("{rendered}\n\n{block}");
            let candidate_tokens = estimate_tokens(&candidate);
            if candidate_tokens > RELEVANT_TECHNIQUES_TOKEN_BUDGET {
                break;
            }
            rendered = candidate;
            kept_skills += 1;
            total_tokens = candidate_tokens;
        }

        if kept_playbooks < self.relevant_playbooks.len().min(3)
            || kept_skills < self.relevant_skills.len()
        {
            tracing::info!(
                kept_playbooks,
                dropped_playbooks = self.relevant_playbooks.len().min(3) - kept_playbooks,
                kept_skills,
                dropped_skills = self.relevant_skills.len() - kept_skills,
                token_budget = RELEVANT_TECHNIQUES_TOKEN_BUDGET,
                used_tokens = total_tokens,
                "trimmed relevant techniques to fit the prompt budget"
            );
        } else {
            tracing::info!(
                kept_playbooks,
                kept_skills,
                token_budget = RELEVANT_TECHNIQUES_TOKEN_BUDGET,
                used_tokens = total_tokens,
                "included relevant techniques in the prompt"
            );
        }

        self.apply_budget_profile(
            PromptSection::new("relevant_techniques", rendered)
                .with_priority(SectionPriority::High)
                .with_cache_layer(CacheLayer::Plan)
                .with_placement(Placement::End)
                .with_bidder(AttentionBidder::PlaybookRules)
                .with_hard_cap(RELEVANT_TECHNIQUES_TOKEN_BUDGET),
        )
    }
```

**Replace with:**
```rust
    fn relevant_techniques_section(&self) -> Option<PromptSection> {
        if self.relevant_skills.is_empty() && self.relevant_playbooks.is_empty() {
            return None;
        }

        // Derive the token budget from the budget profile (skills field, char-to-token
        // approximation) instead of the old hardcoded 500-token constant.
        let skill_token_budget = self
            .budget_profile
            .map(|b| b.skills / 4)
            .unwrap_or(500);

        let mut rendered = String::from("## Relevant Techniques");
        let mut kept_playbooks = 0usize;
        let mut kept_skills = 0usize;
        let mut total_tokens = estimate_tokens(&rendered);

        for playbook in self.relevant_playbooks.iter().take(3) {
            let block = render_playbook(playbook);
            let candidate = format!("{rendered}\n\n{block}");
            let candidate_tokens = estimate_tokens(&candidate);
            if candidate_tokens > skill_token_budget {
                break;
            }
            rendered = candidate;
            kept_playbooks += 1;
            total_tokens = candidate_tokens;
        }

        for skill in &self.relevant_skills {
            let block = render_skill(skill);
            let candidate = format!("{rendered}\n\n{block}");
            let candidate_tokens = estimate_tokens(&candidate);
            if candidate_tokens > skill_token_budget {
                break;
            }
            rendered = candidate;
            kept_skills += 1;
            total_tokens = candidate_tokens;
        }

        if kept_playbooks < self.relevant_playbooks.len().min(3)
            || kept_skills < self.relevant_skills.len()
        {
            tracing::info!(
                kept_playbooks,
                dropped_playbooks = self.relevant_playbooks.len().min(3) - kept_playbooks,
                kept_skills,
                dropped_skills = self.relevant_skills.len() - kept_skills,
                token_budget = skill_token_budget,
                used_tokens = total_tokens,
                "trimmed relevant techniques to fit the prompt budget"
            );
        } else {
            tracing::debug!(
                kept_playbooks,
                kept_skills,
                token_budget = skill_token_budget,
                used_tokens = total_tokens,
                "included relevant techniques in the prompt"
            );
        }

        // No longer apply_budget_profile here -- the greedy loop already respects
        // the skills budget. The hard_cap is set to skill_token_budget directly.
        Some(
            PromptSection::new("relevant_techniques", rendered)
                .with_priority(SectionPriority::High)
                .with_cache_layer(CacheLayer::Plan)
                .with_placement(Placement::End)
                .with_bidder(AttentionBidder::PlaybookRules)
                .with_hard_cap(skill_token_budget),
        )
    }
```

## Verification

```bash
# 1. Compile the compose crate
cargo check -p roko-compose

# 2. Run compose tests
cargo test -p roko-compose

# 3. Verify no references to the removed constant
grep -rn 'RELEVANT_TECHNIQUES_TOKEN_BUDGET' crates/roko-compose/ --include='*.rs'
# Should return nothing

# 4. Verify all templates use the common helper
grep -rn 'agents_instructions.*Critical.*CacheLayer::Role' crates/roko-compose/src/templates/ --include='*.rs'
# Should only show common.rs, not individual templates

# 5. Verify SectionSpec table covers all section names
grep -rn 'section_order_rank\|section_budget_cap\|render_section' crates/roko-compose/src/system_prompt_builder.rs
# Should show the functions delegating to spec_for()
```

## Agent Prompt

```
You are implementing W14-A: five compose/template system fixes in the roko codebase.
Workspace root: /Users/will/dev/nunchi/roko/roko/

Read the batch file at /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W14-A-compose-sections.md for full instructions.

## Files to modify

1. `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs`
   - Fix 10.1 (line 823): Add budget caps for role_identity, task_context, affect_guidance, tool_hints, agents_instructions
   - Fix 10.2 (line 1134): Add cached_assembly param to candidate_fits; update build_with_counter (line 381) to maintain it
   - Fix 10.3 (line 1192): Add SectionSpec struct + SECTION_SPECS table; rewrite section_order_rank; rewrite render_section (line 1076)
   - Fix 10.5 (line 107): Remove RELEVANT_TECHNIQUES_TOKEN_BUDGET; rewrite relevant_techniques_section (line 743) to use budget_profile.skills/4

2. `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/common.rs`
   - Fix 10.4: Add `agents_instructions_section()` helper after budget_for. Add imports for PromptSection, SectionPriority, CacheLayer, Placement.

3. Seven template files (Fix 10.4 -- replace manual agents_instructions block with common::agents_instructions_section call, update imports):
   - `templates/implementer.rs` (lines 79-85, import line 5)
   - `templates/strategist.rs` (lines 79-85, import line 8)
   - `templates/reviewer.rs` (lines 121-127, import line 7)
   - `templates/scribe.rs` (lines 118-124, import line 6)
   - `templates/quick.rs` (lines 70-76, import line 14)
   - `templates/integration.rs` (lines 62-68, import line 8)
   - `templates/task_impl.rs` (lines 107-113, import line 9)

## Key details
- The batch file has exact "Find this code:" / "Replace with:" pairs for every change
- Read each source file FIRST to verify line numbers before editing
- Add `tracing::debug!` instrumentation where noted
- Do NOT run cargo build/test/clippy/fmt -- compilation is deferred
```

## Commit

This batch is committed with all Wave 14 batches together. Do not commit individually.

## Checklist

- [ ] 10.1: `section_budget_cap` covers all 11 section names
- [ ] 10.2: `candidate_fits` uses cached assembly string instead of full reassembly
- [ ] 10.2: `truncate_to_fit` updated to accept and pass `cached_assembly` parameter
- [ ] 10.2: `build_with_counter` maintains `cached_assembly` incrementally
- [ ] 10.3: `SectionSpec` struct and `SECTION_SPECS` table added
- [ ] 10.3: `section_order_rank` delegates to `spec_for()`
- [ ] 10.3: `render_section` delegates to `spec_for()`
- [ ] 10.4: `agents_instructions_section()` helper in `common.rs`
- [ ] 10.4: `common.rs` has PromptSection/SectionPriority/CacheLayer/Placement imports
- [ ] 10.4: `implementer.rs` uses `common::agents_instructions_section`
- [ ] 10.4: `strategist.rs` uses `common::agents_instructions_section`
- [ ] 10.4: `reviewer.rs` uses `common::agents_instructions_section`
- [ ] 10.4: `scribe.rs` uses `common::agents_instructions_section`
- [ ] 10.4: `quick.rs` uses `common::agents_instructions_section`
- [ ] 10.4: `integration.rs` uses `common::agents_instructions_section`
- [ ] 10.4: `task_impl.rs` uses `common::agents_instructions_section`
- [ ] 10.5: `RELEVANT_TECHNIQUES_TOKEN_BUDGET` constant removed
- [ ] 10.5: `relevant_techniques_section` uses `budget_profile.skills / 4` with fallback 500
- [ ] 10.5: `apply_budget_profile` call removed from `relevant_techniques_section`
- [ ] Pre-commit checks pass

## Audit Status

Audited: 2026-05-05. PASS no changes needed.
