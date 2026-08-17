# Prompt Assembly: Implementation Plan

Phased plan to wire model-aware context windowing, progressive refinement,
and learning feedback into the existing prompt assembly infrastructure. Each
phase is independently shippable.

---

## Phase 1: Wire ContextTier into Dispatch (ISS-01, ISS-02)

**Goal:** Small models get small prompts. The single highest-impact change.

### 1.1 Add model_slug to PromptAssemblyService

**File:** `crates/roko-compose/src/prompt_assembly_service.rs`

Add a `model_slug: Option<String>` field to `PromptAssemblyService`. When set,
use `ContextTier::from_task_and_model()` to determine the token budget instead
of the static `token_budget` field.

```rust
// In PromptAssemblyService:
pub fn with_model_slug(mut self, slug: String) -> Self {
    self.model_slug = Some(slug);
    self
}

// In assemble():
let effective_budget = match (&self.model_slug, &self.token_budget) {
    (Some(slug), _) => {
        let tier = ContextTier::from_task_and_model(
            &task_tier_string,
            slug,
        );
        Some(tier.default_token_budget())
    }
    (_, Some(budget)) => Some(*budget),
    _ => None,
};
```

**Effort:** Small. Pure additive change.

### 1.2 Pass model_slug through dispatch_agent_with()

**File:** `crates/roko-cli/src/orchestrate.rs`

In `dispatch_agent_with()`, the model slug is already known (from CascadeRouter
selection). Thread it into `PromptAssemblyService` or into
`build_system_prompt_with_context_validated()`.

The key change: before calling `build_role_system_prompt_validated()`, compute
the `ContextTier` and use `tier.default_token_budget()` as the outer budget
envelope. The existing per-role budgets become per-section caps *within* the
tier budget.

```rust
let model_slug = selected_model.slug();
let tier = ContextTier::from_task_and_model(&task_tier, &model_slug);
let tier_budget_tokens = tier.default_token_budget();
// Pass tier_budget_tokens as the outer constraint
```

**Effort:** Medium. Requires understanding the existing prompt assembly call
chain in orchestrate.rs.

### 1.3 Adapt per-section budgets to tier

**File:** `crates/roko-compose/src/budget.rs`

Add a `tier_scaled_budget()` function that takes a `PromptBudget` and a
`ContextTier` and proportionally scales all section caps to fit within the
tier's total token budget.

```rust
pub fn tier_scaled_budget(
    base: PromptBudget,
    tier: ContextTier,
) -> PromptBudget {
    let base_total = total_budget(&base) as f64;
    let tier_total = (tier.default_token_budget() * 4) as f64; // tokens -> chars
    if base_total <= 0.0 || tier_total >= base_total {
        return base; // no scaling needed
    }
    let scale = tier_total / base_total;
    PromptBudget {
        plan: (base.plan as f64 * scale) as usize,
        workspace_map: (base.workspace_map as f64 * scale) as usize,
        // ... etc for all fields
    }
}
```

For Surgical tier, this would scale a 109K char Implementer budget down to
~16K chars (4K tokens * 4 chars/token). Many sections would effectively be
zeroed because their scaled cap would be tiny.

**Effort:** Small.

### 1.4 Tier-dependent section eligibility

**File:** `crates/roko-compose/src/context_provider.rs`

The `ContextTier` enum already knows which categories are eligible per tier.
Expose this as a method:

```rust
impl ContextTier {
    pub fn eligible_sections(&self) -> &[&str] {
        match self {
            Self::Surgical => &["identity", "task", "tools", "anti_patterns"],
            Self::Focused => &["identity", "task", "tools", "anti_patterns",
                              "conventions", "playbooks", "gate_feedback"],
            Self::Full => &["identity", "task", "tools", "anti_patterns",
                           "conventions", "playbooks", "gate_feedback",
                           "domain", "context"],
        }
    }
}
```

Wire into `PromptAssemblyService.should_include()` to hard-exclude sections
that the tier does not support, regardless of effectiveness score.

**Effort:** Small.

### 1.5 Wire BudgetPredictor into the assembly loop

**File:** `crates/roko-compose/src/prompt_assembly_service.rs`

Add `predictor: Option<Arc<Mutex<BudgetPredictor>>>` to the service. When
present and the predictor has history for the current task features, use
`predictor.predict()` as the token budget. When no history, fall back to
the tier default.

After assembly, record the outcome in a deferred callback (the service does
not know if the task succeeded yet -- the orchestrator records that later via
`predictor.record()`).

**Effort:** Medium.

---

## Phase 2: Progressive Context Refinement (ISS-04, ISS-08)

**Goal:** Context retrieval is scored, ranked, and budget-allocated rather than
dumped wholesale.

### 2.1 Wire SectionInfluence into PromptComposer

**File:** `crates/roko-compose/src/prompt.rs`

After scoring sections with `SectionScorer` / `GoalDirectedHeuristicScorer`,
multiply each section's score by the `SectionInfluence.weights()` multiplier.
Sections with negative lift get demoted; sections with positive lift get boosted.

```rust
let influence_weights = section_influence.weights();
for section in &mut scored_sections {
    if let Some(&weight) = influence_weights.get(&section.name) {
        section.effective_score *= weight; // [0.5, 1.5]
    }
}
```

**Effort:** Small.

### 2.2 Wire MultiPatchForager into context retrieval

**File:** `crates/roko-cli/src/orchestrate.rs`

Replace the direct queries in `dispatch_agent_with()` with a forager-driven
retrieval loop:

1. Build `SourceForagingProfile` entries for each active context source
2. Call `forager.optimal_order()` to determine visitation order
3. For each source, call `forager.optimal_iterations()` for iteration count
4. After each retrieval batch, check `should_stop_searching()` with
   `estimate_context_sufficiency()`
5. Stop early when sufficiency is met or MVT ratio drops

Initial g_max/lambda/travel_cost values can be hardcoded constants (calibrated
from the first few runs) and then learned via EMA like `BudgetPredictor`.

**Effort:** Large. Requires profiling the retrieval paths.

### 2.3 Graduated section effectiveness

**File:** `crates/roko-compose/src/prompt_assembly_service.rs`

Replace the binary `should_include()` (threshold 0.1) with proportional
budget scaling:

```rust
fn section_budget_multiplier(&self, section: &str) -> f64 {
    self.section_effectiveness
        .as_ref()
        .and_then(|scores| scores.get(section).copied())
        .unwrap_or(1.0)
        .max(0.0)  // no negatives
}
```

Then in assembly, each section's per-section cap is:
`base_cap * section_budget_multiplier(name)`. A section at 0.3 effectiveness
gets 30% of its normal cap. A section at 0.0 is effectively excluded (cap = 0).

**Effort:** Small.

---

## Phase 3: Chat and ACP Path Convergence (ISS-03, ISS-05)

**Goal:** All entry points use PromptAssemblyService.

### 3.1 Wire PromptAssemblyService into dispatch_direct

**File:** `crates/roko-cli/src/dispatch_direct.rs`

Before spawning the Claude CLI subprocess, call
`PromptAssemblyService::assemble()` with role=Implementer (or a user-selected
role) to get a system prompt. Pass it to the subprocess via `--system-prompt`
or as the first system message.

The service should be configured with:
- Default conventions (from workspace detection)
- No episodes or playbooks (cold start for chat)
- Token budget from model slug (if known) or a conservative 8K default

**Effort:** Medium.

### 3.2 Replace ACP inline prompts with templates

**File:** `crates/roko-acp/src/runner.rs`

Replace the `format!()` strings in `run_multi_role_review()` with calls to
`ReviewerTemplate` with the appropriate `Reviewer` variant:

```rust
// Before:
let architect_prompt = format!("You are the Architect Reviewer...");

// After:
let architect_sections = ReviewerTemplate.render(&ReviewerInput {
    reviewer: Reviewer::ScopedArchitect,
    // ... other fields
});
```

This ensures ACP reviews use the same role definitions as orchestrate.rs.

**Effort:** Small.

### 3.3 Wire conversation compaction into roko chat

**File:** `crates/roko-cli/src/chat_session.rs`

After each user turn, check if compaction should trigger:

```rust
let policy = CompactionPolicy {
    trigger_threshold: 0.70,
    anchor_roles: vec!["system".into()],
    preserve_last_n_turns: 8,
    summary_budget_tokens: 128,
};
if should_compact(&messages, &policy) {
    messages = compact_history(&messages, &policy, &summarizer_agent).await;
}
```

**Effort:** Medium. Requires a summarizer agent to be available in the chat
context.

---

## Phase 4: Advanced Techniques (Future)

### 4.1 Per-model attention curve fitting

**Files:**
- `crates/roko-compose/src/attention.rs` (ModelAttentionCurves)
- New: `crates/roko-compose/src/attention_experiments.rs`

Run placement experiments for each model: dispatch the same task with critical
information placed at different positions (0.0, 0.25, 0.5, 0.75, 1.0). Measure
task success rate at each position. Fit the U-curve parameters (primacy_weight,
primacy_decay, recency_weight, recency_decay) to the observed data.

Persist fitted curves in `.roko/learn/attention-curves.json`.

**Effort:** Large. Requires experiment infrastructure.

### 4.2 Tier-adaptive knowledge confidence thresholds

**File:** `crates/roko-compose/src/prompt_assembly_service.rs`

Make knowledge confidence thresholds tier-dependent:

| Tier | Domain Facts | Techniques | Anti-Patterns |
|---|---|---|---|
| Surgical | >= 0.8 | >= 0.7 | >= 0.5 |
| Focused | >= 0.5 | >= 0.3 | >= 0.2 |
| Full | >= 0.3 | >= 0.2 | >= 0.1 |

Surgical tier only includes highly confident, proven knowledge. Full tier
includes more speculative knowledge that large models can evaluate and filter.

**Effort:** Small.

### 4.3 Task-relevant workspace map filtering

**File:** `crates/roko-compose/src/prompt_assembly_service.rs`

Instead of listing the first 200 files from `src/`, filter the workspace map
to show only files in crates/modules relevant to the current task:

1. Extract crate/module names from task description
2. Filter file listing to matching directories
3. Include sibling files (same directory as referenced files)
4. Cap at tier-dependent limit (50/150/300/500)

**Effort:** Medium.

### 4.4 Role identity from TOML config

**Files:**
- New: `.roko/roles/` directory with TOML files per role
- `crates/roko-compose/src/role_prompts.rs` (role_identity_for)

Load role identity text from TOML files at startup. Fall back to compiled-in
defaults when config files are absent.

```toml
# .roko/roles/implementer.toml
[role]
name = "Implementer"
identity = """
You are the Implementer. Your job is to write clean, correct code that passes
all verification gates...
"""
output_format = "code_with_explanation"
```

**Effort:** Medium.

### 4.5 Content-type-aware token estimation

**File:** `crates/roko-compose/src/token_counter.rs`

Add content-type detection to improve token estimation accuracy:

```rust
fn content_aware_chars_per_token(content: &str) -> f64 {
    let code_indicators = content.matches("fn ").count()
        + content.matches("struct ").count()
        + content.matches("impl ").count();
    let total_words = content.split_whitespace().count().max(1);
    let code_ratio = code_indicators as f64 / total_words as f64;

    if code_ratio > 0.05 {
        3.0  // code-heavy: more tokens per character
    } else if content.contains("##") || content.contains("- ") {
        5.0  // markdown-heavy: fewer tokens per character
    } else {
        4.0  // prose default
    }
}
```

**Effort:** Small.

### 4.6 Cross-agent context injection

**Files:**
- `crates/roko-compose/src/context_mesh.rs` (SharedContextEntry, ContextMesh)
- `crates/roko-cli/src/orchestrate.rs`

When multiple agents run in parallel (e.g., parallel tasks in a plan), share
findings between agents via `ContextMesh`:

1. After each task completes, publish key findings as `SharedContextEntry`
2. Before dispatching the next task, query the mesh for relevant entries
3. Inject shared entries as Layer 3c (active signals)

The `ContextMesh` struct already exists in `context_mesh.rs`. Wire it into
the plan runner's task dispatch loop.

**Effort:** Large.

---

## Phase Summary

| Phase | Issues Addressed | Key Files | Effort | Impact |
|---|---|---|---|---|
| 1 (Tier Wiring) | ISS-01, ISS-02 | prompt_assembly_service.rs, budget.rs, orchestrate.rs | Medium | Critical: fixes small model overload |
| 2 (Refinement) | ISS-04, ISS-08 | prompt.rs, orchestrate.rs, foraging.rs | Large | High: enables learning feedback |
| 3 (Convergence) | ISS-03, ISS-05 | dispatch_direct.rs, runner.rs, chat_session.rs | Medium | Critical: all paths get prompts |
| 4 (Advanced) | ISS-07, ISS-11-17 | attention.rs, token_counter.rs, role_prompts.rs | Large | Medium: polish and optimization |

**Recommended order:** Phase 1 -> Phase 3 -> Phase 2 -> Phase 4.

Phase 1 fixes the most impactful user pain point (small model overload).
Phase 3 ensures all entry points benefit from the fix. Phase 2 adds the
learning loop. Phase 4 is optimization.

---

## Success Criteria

### Phase 1 Complete When:
- `is_local_model("ollama/llama3.2")` returns true
- Dispatching to an Ollama model produces a system prompt <= 4K tokens
- Dispatching to Sonnet produces a system prompt <= 12K tokens
- Dispatching to Opus produces a system prompt <= 24K tokens
- `BudgetPredictor.predict()` is called before assembly in `dispatch_agent_with()`
- `BudgetPredictor.record()` is called after gate results are known

### Phase 3 Complete When:
- `roko chat` produces a system prompt with role identity, conventions, and task context
- ACP `run_multi_role_review()` uses `ReviewerTemplate` instead of inline strings
- `roko chat` supports conversation compaction for sessions > 50 turns

### Phase 2 Complete When:
- `SectionInfluence.weights()` is applied as a multiplier during composition
- After 20+ tasks, sections with negative lift are visibly demoted in the budget
- `MultiPatchForager` is instantiated with calibrated profiles for 3+ sources
- Context retrieval stops early when sufficiency >= 0.85

---

## Sources

- `crates/roko-compose/src/prompt_assembly_service.rs` -- PromptAssemblyService
- `crates/roko-compose/src/context_provider.rs` -- ContextTier, is_local_model
- `crates/roko-compose/src/budget.rs` -- adjusted_budget_for, tier scaling target
- `crates/roko-compose/src/budget_predictor.rs` -- BudgetPredictor, SectionInfluence
- `crates/roko-compose/src/attention.rs` -- ModelAttentionCurves, dynamic_placement
- `crates/roko-compose/src/foraging.rs` -- MultiPatchForager, should_stop_searching
- `crates/roko-compose/src/compaction.rs` -- compact_history, CompactionPolicy
- `crates/roko-compose/src/prompt.rs` -- PromptComposer, section scoring
- `crates/roko-compose/src/templates/common.rs` -- PromptBudget, budget_for
- `crates/roko-cli/src/orchestrate.rs` -- dispatch_agent_with, model selection
- `crates/roko-cli/src/dispatch_direct.rs` -- bare dispatch, no system prompt
- `crates/roko-acp/src/runner.rs` -- run_multi_role_review, inline prompts
