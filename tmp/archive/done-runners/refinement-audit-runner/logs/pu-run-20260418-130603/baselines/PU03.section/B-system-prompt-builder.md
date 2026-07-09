# B — SystemPromptBuilder (Doc 02)

Parity analysis of `docs/03-composition/02-system-prompt-builder-7-layer.md` vs actual codebase.

---

## B.01 — SystemPromptBuilder Struct Size and Shape (Doc §1, §2)

- **Status**: DONE (exceeds spec)
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
> Canonical source: `crates/roko-compose/src/system_prompt_builder.rs` (726 lines, 12 tests).

Doc §2 sketches an 8-field struct (`role_identity`, `conventions`, `domain_context`, `relevant_context`, `task_context`, `tool_instructions`, `anti_patterns`, `affect_guidance`) backed by a fluent builder API with `build()` + `build_sections()`.

### What exists
`SystemPromptBuilder` at `crates/roko-compose/src/system_prompt_builder.rs:52`. Actual size is **1628 lines** and **27 `#[test]` functions**, not 726 LOC / 12 tests as the doc claims.

The struct (lines 52-79) carries **13 fields** across **9 layers** (rather than doc's 7):

| Code field | Line | Doc field |
|----|----|----|
| `role_identity: String` | 54 | `role_identity` (Layer 1) |
| `conventions: Option<String>` | 56 | `conventions` (Layer 2) |
| `domain: Option<String>` | 58 | `domain_context` (Layer 3a) |
| `context: Option<String>` | 60 | `relevant_context` (Layer 3b) |
| `pheromones: Vec<ContextChunk>` | 62 | -- (Layer 3c, not in doc) |
| `task: Option<String>` | 64 | `task_context` (Layer 4) |
| `tools: Option<String>` | 66 | `tool_instructions` (Layer 5) |
| `relevant_skills: Vec<Skill>` | 68 | -- (Layer 6, not in doc) |
| `anti_patterns: Vec<String>` | 70 | `anti_patterns` (Layer 7) |
| `affect_state: Option<PadState>` | 72 | `affect_guidance` (Layer 8; doc Layer 7) |
| `cache_markers: bool` | 74 | cache-alignment flag (§3) |
| `token_budget: Option<usize>` | 76 | -- (budget enforcement, not in doc struct) |
| `section_effectiveness: Option<SectionEffectivenessConfig>` | 78 | -- (learned priorities, not in doc struct) |

The 9-layer counter-claim is explicit in the module doc comment at line 1 ("Composable system prompt builder with 9 layers") and in `layer_count()` at line 452.

Fluent builder methods all present: `new` (line 117), `with_conventions` (137), `with_domain` (144), `with_context` (151), `with_pheromones` (158), `with_task` (165), `with_tools` (172), `with_skills` (179), `with_anti_patterns` (186), `add_anti_pattern` (193), `with_affect_state` (200), `with_cache_markers` (211), `with_token_budget` (218), `with_section_effectiveness` (225).

Build methods: `build()` at line 242, `build_sections()` at line 325, plus `build_with_counter()` at line 254 (budget-aware build, not in doc).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| B.01.1 | Doc says 726 lines / 12 tests. Actual is 1628 LOC / 27 tests (more than 2x both). | doc 02 line 4 vs system_prompt_builder.rs | LOW (doc is stale; code shipped further) |
| B.01.2 | Doc's 8-field struct sketch uses `String` throughout; code uses `Option<String>` for layers 2-5 plus `Vec` containers for layers 3c/6/7. | doc 02 §2 vs system_prompt_builder.rs:52-79 | LOW (cosmetic; code handles "not set" cleanly) |
| B.01.3 | Doc omits `token_budget` and `section_effectiveness` fields which are production wiring. | doc 02 §2 | LOW (undocumented enhancements) |

### Verify
```bash
wc -l /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs
grep -c '#\[test\]' /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs
```

---

## B.02 — Layer 1: Role Identity (Doc §1)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Layer 1 is the foundation "Role Identity" layer. Content source: `role_prompts.rs`. Cache tier: System. Purpose: "Who the agent is, what it specializes in." Required on every prompt.

### What exists
Layer 1 at `system_prompt_builder.rs:328-334`:
```rust
sections.push(
    PromptSection::new("role_identity", &self.role_identity)
        .with_priority(self.effective_priority("role_identity", SectionPriority::Critical))
        .with_cache_layer(CacheLayer::Role)
        .with_placement(Placement::Start),
);
```

- `role_identity` is the only **required** constructor parameter (line 117 — `new(role_identity: impl Into<String>)`).
- Priority is `Critical` (non-droppable).
- Cache layer is `CacheLayer::Role` (equivalent to doc's "System" tier).
- Placement is `Start` (consistent with U-shape / directive-last).
- Fed from `role_identity_for(self.role)` in `role_prompts.rs:273`, which maps each `AgentRole` to its template.
- Render special-case at line 771: role identity emits content with no header prefix, to keep cache-prefix stability.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| B.02.1 | Doc calls the tier "System"; code calls the enum variant `CacheLayer::Role`. | doc 02 §1 vs prompt.rs:47 | LOW (renaming; tag string is "role") |

### Verify
```bash
grep -n '"role_identity"' /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs
```

---

## B.03 — Layer 2: Conventions (Doc §1)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Layer 2 holds project-level rules: coding style, safety constraints, project-specific patterns, architecture rules. Cache tier: System. Loaded from `CLAUDE.md`, `roko.toml`.

### What exists
Layer 2 at `system_prompt_builder.rs:336-348`:
```rust
if let Some(ref conv) = self.conventions {
    if !conv.is_empty() {
        sections.push(
            PromptSection::new("conventions", conv)
                .with_priority(self.effective_priority("conventions", SectionPriority::High))
                .with_cache_layer(CacheLayer::Role)
                .with_placement(Placement::Start),
        );
    }
}
```

- Priority is `High` (not `Critical` as the doc's cache-tier framing might suggest) — the section can be dropped under extreme budget pressure.
- Lives in the Role cache layer (same tier as identity and tools — doc's "System").
- Conventions are populated by `RoleSystemPromptSpec::conventions_text()` at `role_prompts.rs:244` which concatenates `CONTEXT_LAYOUT_STANZA` + `DEFAULT_CONVENTIONS_SUFFIX` plus optional extras.
- Render at line 772 prefixes with `## Project Conventions\n\n`.

No file read happens inside the builder — anti-pattern #8 forbids `std::fs` (per module doc at line 30). Content must arrive via builder methods. This is a deliberate design choice; callers read CLAUDE.md / roko.toml externally.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| B.03.1 | Doc implies Layer 2 is Critical (safety constraints must be verbatim); code sets `High`, droppable. | doc 02 §1 vs system_prompt_builder.rs:342 | LOW (budget pressure rarely reaches it) |

### Verify
```bash
grep -n 'conventions' /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs | head -10
```

---

## B.04 — Layer 3a / 3b / 3c: Domain + Relevant Context + Active Signals (Doc §1)

- **Status**: DONE (exceeds spec)
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Doc splits Layer 3 into 3a (domain context, session tier) and 3b (relevant context, task tier).
- **3a**: PRD extracts, workspace map, cross-plan context.
- **3b**: Retrieved knowledge entries, episode summaries, enrichment artifacts.

### What exists
Code splits Layer 3 into **three** sub-layers, adding a 3c not in the doc.

**Layer 3a — Domain Context** at `system_prompt_builder.rs:364-376`:
- Name: `domain_context`
- Priority: `High`
- Cache layer: `CacheLayer::Workspace` (doc's "Session")
- Placement: `Middle`
- Fed from `TaskContext::domain_layer()` at `role_prompts.rs:125`, which concatenates plan id, task summary, goal, workspace label, domain notes.

**Layer 3b — Relevant Context** at `system_prompt_builder.rs:378-390`:
- Name: `context_layer` (note: not `relevant_context` as the doc names it)
- Priority: `High`
- Cache layer: `CacheLayer::Workspace` (same tier as 3a — doc says this should be Task-tier but code treats it as Session-tier so it caches across iterations)
- Placement: `Middle`
- Content is rendered with a `## Relevant Context\n` header (see line 382).
- Fed from `TaskContext::context_layer()` at `role_prompts.rs:145`.

**Layer 3c — Active Signals (Pheromones)** at `system_prompt_builder.rs:392-395` and `pheromone_section()` at `system_prompt_builder.rs:895-927`:
- Name: `pheromone_signals`
- Priority: `High`
- Cache layer: `CacheLayer::Workspace`
- Placement: `Middle`
- Hard cap: 1500 tokens (line 925).
- Content is sorted by relevance, then threat-priority, then content lexicographically (lines 905-912).
- Labelled output rendered as `- [Threat|Warning|Opportunity|Signal] <content>` with optional recency / confidence / track_record tags (lines 930-943).
- Header is `## Active Signals` (line 776).
- Accepts `ContextChunk` from `roko-neuro` (imported via `crate::ContextChunk`).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| B.04.1 | Doc has no layer 3c (pheromones / stigmergic signals); code implements one as a first-class layer. | doc 02 §1 vs system_prompt_builder.rs:392 | LOW (code exceeds spec) |
| B.04.2 | Code puts layer 3b (relevant context) in the Session (Workspace) tier; doc says 3b should be Task-tier so iterations differ. | doc 02 §1 vs system_prompt_builder.rs:386 | MEDIUM (cache-reuse implications on iteration 2+ — but acceptable if iteration memory rides in layer 4 instead) |
| B.04.3 | Section name `context_layer` differs from doc's `relevant_context`. | system_prompt_builder.rs:382 | LOW (cosmetic) |

### Verify
```bash
grep -n 'pheromone_signals\|context_layer\|domain_context' /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs | head -10
```

---

## B.05 — Layer 4: Task Context (Doc §1)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Layer 4 carries the current task spec: task TOML, task brief (What/Why/How), gate errors from previous attempts, iteration memory. Cache tier: Task. Most volatile task-specific layer.

### What exists
Layer 4 at `system_prompt_builder.rs:397-409`:
```rust
if let Some(ref task) = self.task {
    if !task.is_empty() {
        sections.push(
            PromptSection::new("task_context", task)
                .with_priority(self.effective_priority("task_context", SectionPriority::Critical))
                .with_cache_layer(CacheLayer::Plan)
                .with_placement(Placement::End),
        );
    }
}
```

- Priority: `Critical` (non-droppable; matches doc's intent that the task cannot be dropped).
- Cache layer: `CacheLayer::Plan` (doc's "Task" tier).
- Placement: `End` (directive-last, per doc §8.1).
- Fed from `TaskContext::task_layer()` at `role_prompts.rs:106`, which emits `Plan: ... Goal: ... Task: ...` lines.
- Render at line 779 prefixes with `## Current Task\n\n`.

No explicit `gate_errors` or `iteration_memory` field — callers can stuff these into the task string via `TaskContext`. No structured representation of prior-attempt reflections.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| B.05.1 | Doc lists iteration memory / gate errors as structured inputs to Layer 4; code treats task as a single string, so callers must stringify errors into the task. | doc 02 §1 vs role_prompts.rs:106 | LOW (structural — no user-visible effect) |

### Verify
```bash
grep -n 'task_context' /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs | head -5
```

---

## B.06 — Layer 5: Tool Instructions (Doc §1)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Tool definitions (alphabetical for cache stability), MCP config, tool-specific instructions, tool restrictions. Cache tier: Task.

### What exists
Layer 5 at `system_prompt_builder.rs:350-362`:
```rust
if let Some(ref tools) = self.tools {
    if !tools.is_empty() {
        sections.push(
            PromptSection::new("tool_instructions", tools)
                .with_priority(self.effective_priority("tool_instructions", SectionPriority::Normal))
                .with_cache_layer(CacheLayer::Role)
                .with_placement(Placement::Middle),
        );
    }
}
```

- Priority: `Normal` (droppable under budget pressure).
- Cache layer: `CacheLayer::Role` — **not** `Plan` as doc says — code groups tools in the System cache tier alongside identity and conventions so all three share the stable prefix.
- Placement: `Middle`.
- Fed from `tool_allowlist_instructions()` at `role_prompts.rs:473`, which emits `Claude tool allowlist: {csv}\n\nUse only the tools granted to your role.` or a fallback if empty.
- Alphabetical sorting of tool names is enforced by `canonical_tool_order()` at line 102 (`pub fn canonical_tool_order(tools: &mut [ToolDef])`), called by callers before passing into the builder.
- Test `prompt_normalization_canonical_tool_order_produces_identical_output` at line 1548 verifies that two tool lists in different order produce byte-identical prompts when `canonical_tool_order` is applied.
- Render at line 773 prefixes with `## Tool Instructions\n\n`.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| B.06.1 | Doc says Layer 5 is Task-tier; code places it in the Role (System) tier instead. This is a cache-optimization upgrade: tools rarely change, so identical-across-tasks gets 90% cache hit. | doc 02 §1 vs system_prompt_builder.rs:358 | LOW (cache-wise a positive deviation) |
| B.06.2 | Doc lists "tool-specific instructions (e.g., 'prefer using Read over cat')" — code's `tool_allowlist_instructions` is short and generic; no per-tool guidance. | role_prompts.rs:473 | LOW (caller can supply richer tools string if needed) |

### Verify
```bash
grep -n 'tool_instructions\|canonical_tool_order' /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs | head -5
```

---

## B.07 — Layer 6/7: Anti-Patterns + Relevant Techniques (Doc §1)

- **Status**: DONE (exceeds spec)
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Doc's Layer 6 is "Anti-Patterns" only — playbook rules matching current task's file paths / crates, common mistakes from episode history, anti-knowledge entries. Cache tier: Dynamic.

### What exists
Code separates these into **two** distinct layers (code's Layer 6 = Relevant Techniques, code's Layer 7 = Anti-Patterns). Doc's single Layer 6 maps only to code's Layer 7.

**Code Layer 6 — Relevant Techniques** at `system_prompt_builder.rs:411-414` plus `relevant_techniques_section()` at lines 551-596:
- Name: `relevant_techniques`
- Priority: `High`
- Cache layer: `CacheLayer::Plan`
- Placement: `End`
- Hard cap: 500 tokens (constant `RELEVANT_TECHNIQUES_TOKEN_BUDGET` at line 87).
- Accepts `Vec<Skill>` from `roko_learn::skill_library::Skill`.
- Emits `## Relevant Techniques\n\n### <skill>\n\nWhen to use: ...\nHow to apply: ...\nSuccess rate: N%\n` blocks via `render_skill` at line 701.
- Trims skills to fit the 500-token budget (lines 562-570) and logs kept/dropped counts.
- Test `relevant_skills_section_is_injected_and_budgeted` at line 1510 confirms the rendering.

**Code Layer 7 — Anti-Patterns** at `system_prompt_builder.rs:416-432`:
- Name: `anti_patterns`
- Priority: `Normal`
- Cache layer: `CacheLayer::Plan` (doc's "Dynamic" — but note the doc's Dynamic tier doesn't match cache semantics; code keeps it in Plan so within-task iterations can cache).
- Placement: `End` (near the generation boundary — matches doc §8.3 "anti-patterns-near-output" recency principle).
- Rendered as bullets: `Do NOT:\n- a\n- b\n...` (line 425).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| B.07.1 | Doc has only one layer here (Anti-Patterns); code splits into two (Relevant Techniques + Anti-Patterns) with the techniques layer feeding from a Skill library. | doc 02 §1 vs system_prompt_builder.rs:411-432 | LOW (code exceeds spec) |
| B.07.2 | Doc tier is "Dynamic" for Layer 6; code uses `CacheLayer::Plan`. With no `Dynamic` cache layer defined (only `Role`, `Workspace`, `Plan`, `Volatile` exist — see B.09), anti-patterns in `Plan` preserve within-task iteration cache hits. | prompt.rs:45 vs doc 02 §1 | LOW (design deviation; caches better than doc suggests) |

### Verify
```bash
grep -n 'anti_patterns\|relevant_techniques' /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs | head -10
```

---

## B.08 — Layer 8: Affect Guidance (PAD State) (Doc §1 layer 7, §5)

- **Status**: DONE (exceeds spec)
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Layer 7 (in doc's numbering) is affect guidance from Daimon PAD (Pleasure-Arousal-Dominance) state. Doc §5.1-5.3 specifies:
- **Arousal ≥ 0.35**: "You are under time pressure..."
- **Arousal ≤ -0.35**: "You have time to explore..."
- **Pleasure ≤ -0.35**: "Recent attempts have had issues..."
- **Dominance**: reserved for future use.

### What exists
Layer 8 (in code's numbering) at `system_prompt_builder.rs:434-444`. The actual guidance logic is in `affect_guidance()` at lines 484-525 and **is fully wired**, taking a `PadState` value from `roko-neuro::context::PadState` (at `crates/roko-neuro/src/context.rs:148`).

Actual thresholds (with guidance strings):
- `affect.arousal >= 0.35` → `"You are under time pressure, focus on the most critical path."` (line 488-489)
- `affect.arousal <= -0.35` → `"You have time to explore thoroughly."` (line 490-491)
- `affect.pleasure <= -0.25` → `"Prefer proven approaches, verify early, and surface uncertainty explicitly."` (line 494-497) — **-0.25, not -0.35 as doc says**
- `affect.pleasure >= 0.35` → `"Keep the solution lean and avoid over-engineering."` (line 498-499) — **not in doc**
- `affect.dominance <= -0.20` → `"Reduce scope until the next concrete checkpoint is clear."` (line 502-503) — **dominance wired, not "reserved"**
- `affect.dominance >= 0.30` → `"Execute decisively, but keep claims grounded in evidence."` (line 504-505)
- Somatic valence + intensity branches at lines 508-518 (negative valence = "prior failure territory", positive = "prior success territory") — **not in doc**

Rendered as a single space-joined string under `## Affect Guidance` (line 778).

- Section name: `affect_guidance`
- Priority: `Normal`
- Cache layer: `CacheLayer::Volatile`
- Placement: `End`

`PadState` is `roko_neuro::context::PadState` (re-exported by `roko-compose`). Tests `affect_guidance_reflects_arousal` (line 1288), `affect_guidance_mentions_negative_somatic_signal` (line 1307) verify the branches.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| B.08.1 | Doc says pleasure threshold is `-0.35`; code uses `-0.25`. | doc 02 §5.2 vs system_prompt_builder.rs:494 | LOW (threshold calibration) |
| B.08.2 | Doc says Dominance is "reserved for future use"; code wires it with two thresholds (line 502 and 504). | doc 02 §5.3 vs system_prompt_builder.rs:502-505 | LOW (code exceeds spec) |
| B.08.3 | Code adds a high-pleasure branch (`>= 0.35`) and a somatic valence/intensity branch (lines 508-518); doc has neither. | doc 02 §5 vs system_prompt_builder.rs:498-518 | LOW (code exceeds spec) |
| B.08.4 | Doc §12 status table says "Dominance affect guidance: Not yet"; verified false — dominance is wired. | doc 02 §12 | LOW (doc 02 §12 status table is stale) |

### Verify
```bash
grep -n 'affect.arousal\|affect.pleasure\|affect.dominance\|somatic_valence' /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs | head -10
```

---

## B.09 — Cache Alignment Markers and TIER Names (Doc §3, §2 layer markers)

- **Status**: PARTIAL
- **Priority**: P3
- **Estimated LOC**: 5
- **Dependencies**: None
- **Files to modify**: `crates/roko-compose/src/system_prompt_builder.rs:969-975`

### What the doc says
Doc §2 shows four cache-alignment markers in the flat `build()` output, one per tier:
```
<!-- roko:layer:system -->    (before Role Identity + Conventions)
<!-- roko:layer:session -->   (before Domain Context + Relevant Context)
<!-- roko:layer:task -->      (before Task Context + Tools)
<!-- roko:layer:dynamic -->   (before Anti-Patterns + Affect Guidance)
```

Tier names per doc §3.1: **System / Session / Task / Dynamic**.

### What exists
Code uses a different marker prefix and emits only **two** of the four markers.

`cache_marker()` at `system_prompt_builder.rs:969-975`:
```rust
const fn cache_marker(layer: CacheLayer) -> Option<&'static str> {
    match layer {
        CacheLayer::Role => Some("<!-- cache:system -->"),
        CacheLayer::Workspace => Some("<!-- cache:session -->"),
        CacheLayer::Plan | CacheLayer::Volatile => None,
    }
}
```

Deviations:
- Prefix is `cache:` not `roko:layer:`.
- Only `Role` → `<!-- cache:system -->` and `Workspace` → `<!-- cache:session -->` emit markers. `Plan` and `Volatile` return `None`, so no `task` or `dynamic` markers are ever emitted.
- Markers are placed **between** layers at tier boundaries (at the end of each tier's last section), not at the start of each tier. See `assemble_selected_sections` at line 784-818.

The four `CacheLayer` enum variants at `prompt.rs:45-55` are `Role` (0), `Workspace` (1), `Plan` (2, default), `Volatile` (3). These map conceptually to doc's System / Session / Task / Dynamic tiers, but the markers in the rendered output only cover the first two. No `<!-- roko:layer:task -->` or `<!-- roko:layer:dynamic -->` is ever rendered.

Tests:
- `cache_markers_inserted_between_tiers` (line 1098-1119) confirms system + session markers.
- `cache_markers_omitted_when_disabled` (line 1121-1130) confirms no marker prefix at all when `.with_cache_markers()` isn't called.
- `no_cache_session_marker_when_no_session_layers` (line 1376-1386) confirms session marker is skipped when there's no Workspace-tier content.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| B.09.1 | Doc prefix is `roko:layer:`; code prefix is `cache:`. The rendered strings never contain `roko:layer:`. | doc 02 §2 vs system_prompt_builder.rs:971-972 | LOW (cosmetic; downstream gateway has to match the actual emitted strings anyway) |
| B.09.2 | Doc expects four tier markers (system/session/task/dynamic); code emits two (system, session only). Plan and Volatile tier boundaries have no marker. | doc 02 §2 vs system_prompt_builder.rs:973 | LOW-MEDIUM (downstream cache_control breakpoints may still be inferable from position, but explicit markers would simplify inference gateways) |
| B.09.3 | Enum-variant names diverge from doc's tier names: `Role`/`Workspace`/`Plan`/`Volatile` (code) vs `System`/`Session`/`Task`/`Dynamic` (doc). The serde `rename_all = "snake_case"` at prompt.rs:44 means tag strings are `role`/`workspace`/`plan`/`volatile`, not `system`/`session`/`task`/`dynamic`. | prompt.rs:45-55 | LOW (cosmetic; consistent set of names internally) |

### Verify
```bash
grep -n '<!-- cache\|cache_marker' /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs
```

---

## B.10 — Learned Layer Ordering / SectionEffectivenessRegistry (Doc §8, §12)

- **Status**: PARTIAL
- **Priority**: P2
- **Estimated LOC**: 150-250 (for `LayerOrderPolicy`)
- **Dependencies**: B.01
- **Files to modify**: `crates/roko-compose/src/system_prompt_builder.rs`, `crates/roko-learn/src/`

### What the doc says
Doc §8 specifies `LayerOrderPolicy` — a learned ordering that permutes the 7 layers per task category via Thompson sampling over ordering variants. Doc §12 status table says "Dynamic layer ordering (§8): Designed — LayerOrderPolicy specified".

Separately, doc §12 mentions nothing about a `SectionEffectivenessRegistry` — that name appears only in the investigation target, not the doc itself.

### What exists
Two distinct features here, and the doc conflates them.

**1. `LayerOrderPolicy` — NOT implemented.** No struct by that name exists anywhere in `crates/`. Grep for `LayerOrderPolicy`, `layer_order`, `ordering_policy` returns only two matches: test functions named `cache_layer_ordering` (prompt.rs:950) and `layer_order_is_correct` (system_prompt_builder.rs:1053), both of which verify the fixed canonical ordering.

**2. `SectionEffectivenessRegistry` — IMPLEMENTED and wired.** Defined at `crates/roko-learn/src/section_effect.rs:114`. It is **not** the `LayerOrderPolicy` from doc §8; it tracks *per-section-per-role inclusion/exclusion outcomes* (a simpler priority adjustment, not a permutation). Call sites:
- Struct field in builder: `system_prompt_builder.rs:78` (`section_effectiveness: Option<SectionEffectivenessConfig>`).
- Builder method: `with_section_effectiveness(role, registry)` at `system_prompt_builder.rs:225-235`.
- Priority adjustment: `effective_priority()` at `system_prompt_builder.rs:527-537` plus `adjusted_priority()` at 599-610. A section's base priority (e.g. `High`) can be bumped up/down by ±1 priority level based on `registry.recommend_priority_change(section, role)`.
- Budget tuning: `apply_learned_budget_tuning()` at `system_prompt_builder.rs:630-698` uses `section_lift_weight()` to re-weight each section's token cap under a learned budget multiplier.
- Wired from orchestrator via `role_prompts.rs:269` (`builder_with_section_effectiveness`) and `crates/roko-cli/src/prompting.rs:60` (`build_role_system_prompt_validated` passes `Option<&SectionEffectivenessRegistry>`).
- Persistence via `crates/roko-learn/src/runtime_feedback.rs:343` — `RuntimeFeedback` holds `parking_lot::Mutex<SectionEffectivenessRegistry>` and saves to `.roko/learn/section-effects.json` (default path at `section_effect.rs:13`).
- Priority-change thresholds (`section_effect.rs:97-109`): needs `included_trials >= 20 && excluded_trials >= 5`; then lift > 0.05 → Increase, lift < -0.02 → Decrease.
- Tests: `section_priority_adjustment_increases_positive_lift_sections` (line 1428), `..._decreases_negative_lift_sections` (1458), `..._ignores_insufficient_data` (1488), plus tests in `section_effect.rs` lines 273-385.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| B.10.1 | Doc §8 `LayerOrderPolicy` (per-task-category layer permutation via Thompson sampling) is NOT implemented. Ordering is fixed in `section_order_rank()` at `system_prompt_builder.rs:878-892`. | doc 02 §8 vs system_prompt_builder.rs:878 | MEDIUM (expected; doc labels it "Designed") |
| B.10.2 | `SectionEffectivenessRegistry` exists and is fully wired into builder + CLI + persistence; this is a *different* (simpler) learning mechanism than doc §8 — priority bumping, not reordering. The doc does not mention it. | section_effect.rs:114 | LOW (code exceeds spec in one dimension; doesn't address the other) |
| B.10.3 | Doc §12 status "Dynamic anti-patterns from knowledge store: Scaffold" — code does not bridge anti-patterns to a knowledge store inside the builder. Anti-patterns are a plain `Vec<String>` supplied by the caller. | system_prompt_builder.rs:70 | LOW (caller's responsibility) |

### Verify
```bash
grep -rn 'LayerOrderPolicy\|layer_order_policy' /Users/will/dev/nunchi/roko/roko/crates/ --include='*.rs'
# Expected: no matches (unimplemented)

grep -rn 'SectionEffectivenessRegistry' /Users/will/dev/nunchi/roko/roko/crates/ --include='*.rs' | head -20
# Expected: definition + usage in compose/cli/learn
```

---

## B.11 — Compression Integration (Doc §9)

- **Status**: NOT DONE
- **Priority**: P2
- **Estimated LOC**: 400-600 (full `CompressionBudgetController`)
- **Dependencies**: None
- **Files to modify**: new file `crates/roko-compose/src/compression.rs`

### What the doc says
Doc §9 specifies a `CompressionBudgetController` with per-layer `LayerCompressionConfig { compressible, max_ratio, method, floor_tokens }` and a `CompressionMethod` enum (`None`, `Extractive`, `TokenPruning`, `Dedup`, `Abstractive`). Intended compression ratios per layer (§9.1): 1:1 for identity/conventions/anti-patterns/affect; 3-6:1 for domain context; 2-5:1 for relevant context. Doc §9.4 mentions Chain of Draft integration. Doc §12 status: "Prompt compression integration (§9): Designed — CompressionBudgetController specified".

### What exists
Nothing by these names. Grep for `CompressionBudgetController`, `LayerCompressionConfig`, `CompressionMethod` across `crates/` returns zero matches.

The only compression-adjacent behavior in the builder is **truncation** (not semantic compression):
- `enforce_hard_cap()` at `prompt.rs:182-204` — head-truncation with `…[truncated N tokens]` suffix.
- `truncate_to_fit()` at `system_prompt_builder.rs:834-876` — binary search over char boundaries to fit a token budget, preserving the prefix.
- `apply_learned_budget_tuning()` at `system_prompt_builder.rs:630-698` — redistributes token caps per section weighted by learned lift; the resulting caps are enforced via truncation, not compression.

There is no extractive summarizer, LLMLingua-2 hook, RECOMP integration, or method-aware per-layer compression config. Layer 3a domain context is treated identically to layer 7 anti-patterns: both are either truncated (head) or dropped whole.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| B.11.1 | `CompressionBudgetController` not implemented; no per-layer compression config struct exists. | doc 02 §9.3 | MEDIUM |
| B.11.2 | `CompressionMethod` enum (with 5 variants: None/Extractive/TokenPruning/Dedup/Abstractive) not implemented. | doc 02 §9.3 | MEDIUM |
| B.11.3 | No compression floor-tokens enforcement — current truncation can cut a layer to zero tokens under tight budget. | system_prompt_builder.rs:834 | LOW |
| B.11.4 | Chain-of-Draft integration (doc §9.4) not applied to role identity templates. | role_prompts.rs:273 | LOW (future work) |
| B.11.5 | Doc §12 status table says "Scaffold"; reality is "not started" — no types, no methods, no tests. | doc 02 §12 | LOW (doc 02 §12 status is stale) |

### Verify
```bash
grep -rn 'CompressionBudgetController\|LayerCompressionConfig\|CompressionMethod' /Users/will/dev/nunchi/roko/roko/crates/ --include='*.rs'
# Expected: no matches
```

---

## Section Summary

| Item | Doc reference | Status | Parity |
|------|-----|--------|--------|
| B.01 | SystemPromptBuilder struct (§2) | DONE | 130% — 9 layers vs 7, 1628 LOC vs 726, 27 tests vs 12 |
| B.02 | Layer 1 Role Identity (§1) | DONE | 100% |
| B.03 | Layer 2 Conventions (§1) | DONE | 95% — slightly lower priority than doc implies |
| B.04 | Layer 3a/3b/3c (§1) | DONE | 115% — code adds layer 3c pheromones |
| B.05 | Layer 4 Task Context (§1) | DONE | 90% — no structured iteration memory |
| B.06 | Layer 5 Tool Instructions (§1) | DONE | 105% — moved to Role tier for better cache |
| B.07 | Layer 6/7 Relevant Techniques + Anti-Patterns (§1) | DONE | 110% — code adds Relevant Techniques layer |
| B.08 | Layer 7 (doc) / Layer 8 (code) Affect Guidance (§5) | DONE | 125% — dominance + somatic wired |
| B.09 | Cache Alignment Markers (§3) | PARTIAL | 50% — only 2 of 4 tier markers emitted; prefix is `cache:` not `roko:layer:` |
| B.10 | Learned layer ordering / SectionEffectivenessRegistry (§8) | PARTIAL | LayerOrderPolicy 0%; SectionEffectivenessRegistry 100% (different mechanism) |
| B.11 | Compression integration (§9) | NOT DONE | 0% — no types, methods, or tests |

### Priority actions
1. **P2** (B.09): Add the missing `<!-- cache:task -->` and `<!-- cache:dynamic -->` markers (or update doc §2 to reflect the 2-marker reality). ~5 LOC change or a doc edit.
2. **P2** (B.10): Implement `LayerOrderPolicy` from doc §8 if learned ordering per task-category is wanted; or update doc §12 to note that `SectionEffectivenessRegistry` is the shipped alternative and §8 is deferred.
3. **P2** (B.11): Build `CompressionBudgetController` (doc §9) for tighter budgets on domain/relevant-context layers, or update doc §12 so the "Designed" claim matches the "not started" reality.

---

## Agent Execution Notes

### B.09 — Cache Marker Coverage

This is a good narrow overnight batch.

Recommended slice:

1. decide whether 4-tier markers are truly the intended runtime contract,
2. implement or explicitly rationalize the missing tiers,
3. lock the behavior in with tests.

Acceptance criteria:

- marker behavior is complete or intentionally constrained,
- tests make the behavior obvious to later agents,
- downstream systems do not need to reverse-engineer the rendered prompt.

### B.10 / B.11 — Learned Ordering And Compression

Do not default to implementing these in batch `03`.

- `LayerOrderPolicy` is better handled as learning-policy work.
- compression-controller work should follow budget activation, not precede it.
