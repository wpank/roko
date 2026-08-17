# Prompt Assembly Subsystem Audit

9-layer SystemPromptBuilder, role templates, token budgeting, VCG auction,
demand-driven context tiers, multi-patch foraging, conversation compaction,
position-aware attention, section influence learning -- a sophisticated system
that most entry points bypass entirely.

### Architecture Runner Status (2026-04-29)

**Unified prompt assembly via PromptAssemblyService.** Phase 1.2 completed:
- `PromptAssemblyService` (`roko-compose/src/prompt_assembly_service.rs`) implements `PromptAssembler` trait
- All entry points assemble prompts through `WorkflowEngine` -> `EffectDriver` -> `PromptAssemblyService`
- Emits `PromptAssembled` RuntimeEvent with layer diagnostics
- **Remaining**: model-aware context windowing, progressive refinement, role config TOML (Phase 4)

---

## 1. The 9-Layer SystemPromptBuilder

`roko-compose/src/system_prompt_builder.rs` (~2081 LOC) -- emits 10 layers in
cache-aligned order:

| Layer | Content | Cache Tier | What |
|---|---|---|---|
| 1 | Role identity | System (stable) | Role name, responsibilities, constraints |
| 2 | Conventions | System (semi-stable) | Code style, naming, patterns |
| 3 | Domain context | Session (semi-stable) | Project-specific knowledge |
| 3b | Assembled context | Session (semi-stable) | Relevant code context for the task |
| 3c | Active signals | Session (semi-stable) | Pheromone/stigmergic guidance |
| 4 | Task context | Task (volatile) | Task description, acceptance criteria |
| 4b | Gate feedback | Dynamic | Prior gate failures, error details |
| 5 | Tool instructions | System (stable) | Available tools and usage |
| 6 | Relevant techniques | Task (volatile) | Playbooks + skills |
| 6b | Tool hints | Task (volatile) | Learned tool-sequence hints (LEARN-12) |
| 7 | Anti-patterns | Task (volatile) | Known failure patterns from neuro |
| 8 | Affect guidance | Dynamic | Daimon state |

**Build methods on `SystemPromptBuilder`:**
- `.build()` -- simple concatenation, no token budget enforcement
- `.build_with_counter()` -- greedy knapsack under a configured token budget
- `.build_sections()` -> `Vec<PromptSection>` for downstream composers

**Build methods on `RoleSystemPromptSpec` (higher-level API in `role_prompts.rs`):**
- `.build()` -- delegates to the builder's `.build()`
- `.build_sections()` -- raw section list
- `.build_with_context_window()` -- enforces token budget
- `.build_with_context_window_and_section_effectiveness()` -- full pipeline with learned section priority adjustments

**Cache alignment strategy:**
Layers 1 + 2 + 5 form the prefix-cacheable "system" tier (stable across calls).
Layers 3 + 3b + 3c form the "session" tier (stable within a plan run).
Layers 4 + 6 + 7 are per-task volatile content.
Layers 4b + 8 are dynamic retry/tone guidance.

Cache break markers (`<!-- cache:session -->`, etc.) are inserted between tiers
by the `AdjustedBudget` system in `budget.rs`, with break points at:
- `conventions` (end of System layer)
- `workspace_map` (end of Session layer)
- `file_context` (end of Task layer)

---

## 2. Role Templates

**11 templates** in `roko-compose/src/templates/`, each implementing `RolePromptTemplate`:

| Role | File | Used By |
|---|---|---|
| Strategist | `strategist.rs` | orchestrate.rs |
| Implementer | `implementer.rs` | orchestrate.rs |
| Reviewer (Combined/Scoped/Focused) | `reviewer.rs` | orchestrate.rs review path |
| Scribe | `scribe.rs` | orchestrate.rs doc revision |
| Researcher | `researcher.rs` | orchestrate.rs research |
| QuickReviewer | `quick.rs` | orchestrate.rs |
| QuickFix | `quick.rs` | orchestrate.rs |
| TaskImpl | `task_impl.rs` | Built, lightly used |
| Integration | `integration.rs` | Built, lightly used |
| Refactorer | `refactorer.rs` | Built, lightly used |
| Conductor | `conductor.rs` | Built, lightly used |

**Template format:** Each takes a typed input struct (no filesystem I/O) and emits `Vec<PromptSection>`.

**NOT used by:** `roko chat`, `roko "prompt"` (inline dispatch), ACP runner multi-role review.

---

## 3. Token Budgeting

### 3.1 Per-Role Character Budgets (`templates/common.rs`)

Hardcoded per-role character caps for 9 section categories:

| Role | plan | workspace_map | prd2 | context | brief | reviews | instructions | file_context | skills |
|---|---|---|---|---|---|---|---|---|---|
| Implementer | 50K | 20K | 12K | 4K | 8K | 3K | 4K | 8K | 8K |
| Strategist | 50K | 20K | 12K | 4K | 6K | 3K | 4K | 0 | 4K |
| Architect/Auditor | 50K | 6K | 6K | 2K | 4K | 3K | 4K | 6K | 4K |
| Scribe/Critic | 50K | 6K | 16K | 4K | 6K | 3K | 4K | 6K | 4K |
| QuickReviewer | 50K | 6K | 0 | 0 | 4K | 3K | 2K | 0 | 0 |
| AutoFixer | 0 | 0 | 0 | 0 | 0 | 0 | 2K | 0 | 0 |
| Default | 50K | 8K | 6K | 4K | 4K | 2K | 4K | 6K | 4K |

**Problem**: These are *character* caps, not token caps. The 4:1 char/token heuristic
is used everywhere (`estimate_tokens = len / 4`). These budgets are also static --
they do not adapt to model context window size. A 50K plan budget consumes ~12.5K
tokens, which is fine for Opus (200K context) but catastrophic for a 4K Ollama model.

### 3.2 Complexity-Adjusted Budgets (`budget.rs`)

Three complexity bands modify the base per-role budgets:

| Band | Adjustment |
|---|---|
| Trivial | Zero out prd2, context, skills. Halve workspace_map, brief. |
| Standard | Use base budget as-is. |
| Complex | Inflate workspace_map 1.5x, context 2x, file_context 1.5x. |

### 3.3 Composition Strategies (`strategy.rs`)

| Strategy | How | Actually Used? |
|---|---|---|
| DensityGreedy | Greedy knapsack, sort by value/cost | Yes (default when cold) |
| WeightedSum | Backward-compatible alias for DensityGreedy | Resolves to DensityGreedy |
| VCG | Welfare auction with affect modulation | Activates after 10+ observations per bidder |
| Auto | Selects DensityGreedy when cold, VCG when warm | Default setting |

**Reality:** DensityGreedy dominates. VCG warmup threshold is 10 observations per
bidder (`DEFAULT_VCG_WARMUP_OBSERVATIONS = 10`). In practice this warmup is rarely
reached, so DensityGreedy runs almost always.

### 3.4 BudgetPredictor (`budget_predictor.rs`)

EMA-based token budget predictor using task features (role x complexity x domain).
Persists to `.roko/learn/budget-predictor.json`. Supports:
- Partial-match fallback (same role+complexity, any domain)
- Failure inflation (1.3x when task fails)
- 20% safety margin over EMA
- Minimum floor of 1000 tokens

**Status:** Built and tested. Not wired into the live dispatch path. The predictor
could replace the static per-role budgets with learned values, but nobody calls
`predictor.predict()` before assembly today.

---

## 4. Demand-Driven Context Tiers (`context_provider.rs`)

The most model-aware component in the system. Three tiers mapped from task
complexity and model slug:

| Tier | Models | Budget | Context Includes |
|---|---|---|---|
| Surgical | Haiku, Ollama, Gemma, DeepSeek, Phi, StarCoder | ~4K tokens | Inline files, symbol signatures, anti-patterns, verification |
| Focused | Sonnet | ~12K tokens | Surgical + task brief, dependency graph, prior task outputs |
| Full | Opus | ~24K tokens | Focused + plan brief, cross-plan context, research memo, invariants |

**Local model detection** (`is_local_model()`) catches: ollama/, llama*, gemma*,
qwen*, mistral*, codellama*, deepseek*, phi*, starcoder*, and anything with a colon
that is not claude/gpt/composer/cursor.

**Critical gap:** The tier system exists and is well-designed, but the actual context
provider (`ContextProvider`) is not the primary path used by `dispatch_agent_with()`.
The main path goes through `SystemPromptBuilder` / `RoleSystemPromptSpec`, which
does not consult `ContextTier` at all. The tier budgets (4K/12K/24K) and the
per-role budgets (up to 117K total characters) are completely separate systems.

---

## 5. Position-Aware Attention (`attention.rs`)

U-shaped attention model accounting for "lost in the middle" effects:

```
attention(pos) = primacy * exp(-decay * pos) + recency * exp(-decay * (1-pos)) + baseline
```

Default parameters: primacy_weight=0.35, recency_weight=0.30, baseline=0.35.

**`dynamic_placement()`** reassigns non-critical sections toward high-attention
prompt edges (start/end) based on information density relative to the task query.
Uses term overlap, uniqueness, and compactness as proxy signals.

**`ModelAttentionCurves`** supports per-model fitted curves. Persists to disk.
Currently only the default curve exists -- no model-specific fits have been trained.

**Status:** Built and tested. Used in `role_prompts.rs` placement logic. The U-curve
model is research-validated but the per-model parameterization is empty.

---

## 6. Multi-Patch Foraging (`foraging.rs`)

Marginal Value Theorem (MVT) applied to context retrieval. Each context source
(knowledge entries, episodes, files, symbols) is modeled as a "patch" with:
- `g_max`: asymptotic relevance available
- `lambda`: saturation rate (diminishing returns curve)
- `travel_cost`: switching cost between sources

Active inference bias parameter controls exploration vs exploitation. Higher bias
means more patches visited with shorter stays in each.

**Stopping criteria:** `should_stop_searching()` halts when either:
1. MVT ratio drops below threshold (marginal gain < average gain + switching cost)
2. Sufficiency estimate exceeds threshold (term coverage of task requirements)

**Calibration integration:** `calibration_to_foraging_factor()` adjusts stopping
threshold based on prediction accuracy history. Well-calibrated models stop sooner;
poorly-calibrated models search longer.

**Status:** Built and fully tested. `MultiPatchForager` is exported but not
instantiated in the live dispatch path. Context retrieval in `dispatch_agent_with()`
uses direct queries rather than the foraging optimizer.

---

## 7. Conversation Compaction (`compaction.rs`)

Iterative summarization of conversation history with structured anchor preservation:

- **Anchor preservation:** System messages and error tool results are never compacted
- **Gate result carry-forward:** Gate verdicts extracted from compacted region and
  embedded in summary payload as structured JSON
- **Tool outcome carry-forward:** Tool results similarly preserved
- **LLM summarizer:** Compactable messages are summarized by an agent call
- **Fallback:** Heuristic summary (first 4 messages + count) when summarizer fails
- **Trigger threshold:** Only compact when compactable region exceeds configurable
  fraction of total context (default: 70%)

**CompactionPolicy:**
```rust
CompactionPolicy {
    trigger_threshold: 0.70,      // compact when 70%+ is old history
    anchor_roles: ["system"],     // always preserve
    preserve_last_n_turns: 13,    // keep recent messages verbatim
    summary_budget_tokens: 64,    // summary size cap
}
```

**Status:** Fully implemented and tested. Available for use in long-running chat
sessions and multi-turn agent interactions. Not yet wired into the default
`roko chat` path.

---

## 8. Section Influence Learning (`budget_predictor.rs`)

Leave-one-out approximation for measuring each prompt section's causal impact on
task success:

- Tracks per-section `successes_with / failures_with / successes_without / failures_without`
- Computes lift: `rate_with - rate_without`
- Maps lift to weights in [0.5, 1.5] for budget allocation
- Neutral prior (0.5) for unobserved conditions
- Minimum 10 observations before influencing decisions

**Status:** Built, tested, serializable. Persistence helpers in
`.roko/learn/section-influence.json`. Not yet wired into the live path.

---

## 9. Who Uses What

| Entry Point | Builder? | Templates? | Budgeting? | Effectiveness? | Context Tier? | Attention? |
|---|---|---|---|---|---|---|
| `roko "prompt"` (dispatch_direct) | No | No | No | No | No | No |
| `roko chat` (REPL) | No | No | No | No | No | No |
| `roko run` | Yes (9-layer) | No | Yes | No | No | No |
| `roko plan run` (orchestrate.rs) | Yes (full 9-layer) | No | Yes | Yes | No | Yes |
| ACP runner (standard pipeline) | No (inline strings) | No | No | No | No | No |
| ACP bridge_events | No (inline strings) | No | No | No | No | No |
| orchestrate.rs review | Template only | Yes | No | No | No | No |
| orchestrate.rs scribe | Template only | Yes | No | No | No | No |
| orchestrate.rs retry/replan | No (inline format!) | No | No | No | No | No |

**`roko plan run` uses the full builder.** `dispatch_agent_with()` in `orchestrate.rs`
calls `build_system_prompt_with_context_validated()` which calls
`build_role_system_prompt_validated()` -> `RoleSystemPromptSpec.build_with_context_window_and_section_effectiveness()`.
This includes playbook injection (Layer 6), anti-pattern injection from neuro store
(Layer 7), affect state (Layer 8), pheromone chunks (Layer 3c), and code context
(Layer 3b). This is live code.

**`roko run` also uses the builder** via `build_role_system_prompt_validated()` in `run.rs`.

**Everything else bypasses the builder entirely.**

---

## 10. Inline Prompt Strings (Anti-Pattern #2)

Hardcoded prompts found at:

| File | Purpose |
|---|---|
| `orchestrate.rs` (~8894, ~9347, ~9941, ~14014) | Fallback task prompt ("Plan: X\nTask: Y\n\nImplement...") |
| `orchestrate.rs` (~11212-11215) | Gate failure retry hint |
| `orchestrate.rs` (~11285, ~11395) | Model escalation / replan prompts |
| `orchestrate.rs` (~13080) | Verification-failed fix prompt |
| `roko-acp/runner.rs` (405-424) | ACP review variants (quick/thorough/default) |
| `roko-acp/runner.rs` (525-541) | ACP multi-role review: architect + auditor role descriptions |

The ACP runner's `run_multi_role_review()` function has the most egregious inline
prompts -- full role descriptions for "Architect Reviewer" and "Security & Correctness
Auditor" hardcoded in `format!()` strings. These duplicate what the template system
already provides via `ReviewerTemplate`.

---

## 11. Context Section Assembly in dispatch_agent_with()

In the `roko plan run` path, several context sources are queried before calling
`build_system_prompt_with_context_validated()`:

1. **Code context** -> `code_context_for_task()` -> keyword extraction + index search
2. **Playbooks** -> `playbook_query_context()` (Layer 6)
3. **Skills** -> matched from skill library (Layer 6)
4. **Anti-patterns** -> `query_anti_knowledge_patterns()` from neuro store (Layer 7)
5. **Pheromones** -> `active_pheromone_chunks()` (Layer 3c)
6. **Section effectiveness** -> `learning.section_effectiveness_snapshot()` -> learned priority adjustments per role
7. **Daimon state** -> `task_affect_state` (Layer 8)

**Scoring:** Priority + learned adjustment (`SectionEffectivenessRegistry`) + daimon
modulation -- all applied inside `build_role_system_prompt_validated()`.

---

## 12. PromptAssemblyService Implementation

`prompt_assembly_service.rs` (1049 LOC) implements `PromptAssembler` trait with
the following assembly pipeline:

1. Resolve role from string -> `AgentRole` (defaults to Implementer)
2. Look up role identity text
3. Detect or use default conventions
4. Load recent episodes (last 5) for context layer
5. Query playbook store for relevant techniques
6. Build domain context (static + knowledge store entries with confidence >= 0.5)
7. Build workspace map from source file listing (capped at 200 lines)
8. Set task text
9. Set tool instructions
10. Inject gate feedback
11. Query knowledge store for technique insights (confidence >= 0.3)
12. Query knowledge store for anti-patterns (confidence >= 0.2)
13. Apply section effectiveness filtering (skip sections scoring < 0.1)
14. If token budget set: scale budget by effectiveness ratio, build with counter
15. Otherwise: concatenate all layers

**Key parameters:**
- `SOURCE_SAMPLE_LIMIT = 12` -- max source files read for convention detection
- `WORKSPACE_MAP_LINE_LIMIT = 200` -- max workspace map lines
- Section effectiveness threshold: 0.1 (below = excluded)
- Knowledge confidence floors: 0.5 (facts), 0.3 (techniques), 0.2 (anti-patterns)

---

## 13. Cognitive Workspace Audit (`cognitive_workspace.rs`)

Every dispatch builds a `CognitiveWorkspace` audit object recording:
- Selected role profile and prompt policy with version refs
- Context policy audit reference
- Included context sections with scope, purpose, and decision metadata
- Rejected context candidates with rejection reasons
- Prompt section audit (which sections were included, at what priority)
- Capability grants derived from role profile
- Gate expectations
- Model choice metadata

This provides full prompt assembly traceability -- every decision about what went
into a prompt (and what was excluded and why) is recorded.

---

## 14. Built But Not Connected Inventory

| Component | File | Status |
|---|---|---|
| BudgetPredictor | `budget_predictor.rs` | Built, tested, not called from dispatch |
| SectionInfluence | `budget_predictor.rs` | Built, tested, not called from dispatch |
| ContextTier system | `context_provider.rs` | Built, tested, not integrated with SystemPromptBuilder |
| ModelAttentionCurves (per-model) | `attention.rs` | Struct exists, only default curve populated |
| MultiPatchForager | `foraging.rs` | Built, tested, not instantiated in dispatch |
| CompactionPolicy | `compaction.rs` | Built, tested, not used in `roko chat` |
| VCG auction payments | `auction.rs` | Diagnostic only, greedy allocation dominates |
| ContextBidderRegistry | `context_provider.rs` | Built, complex, partially integrated |
| CostAttribution | `cost_attribution.rs` | Built, not wired |

---

## 15. File Inventory

| File | LOC | Role |
|---|---|---|
| `system_prompt_builder.rs` | 2081 | Core 9-layer builder |
| `prompt_assembly_service.rs` | 1049 | PromptAssembler trait implementation |
| `role_prompts.rs` | ~1500 | RoleSystemPromptSpec, higher-level API |
| `prompt.rs` | ~1200 | PromptSection, PromptComposer, VCG integration |
| `context_provider.rs` | ~2000 | Demand-driven context tiers, bidders, learning |
| `auction.rs` | 688 | VCG with affect modulation |
| `budget_predictor.rs` | 679 | EMA budget prediction + section influence |
| `budget.rs` | 270 | Complexity-adjusted budgets |
| `attention.rs` | 190 | Position attention, U-curve, dynamic placement |
| `foraging.rs` | 438 | Multi-patch MVT foraging |
| `compaction.rs` | 488 | Conversation history compaction |
| `scorer.rs` | ~300 | Section scoring (priority/recency/utility) |
| `cognitive_workspace.rs` | ~200 | Audit trail construction |
| `context_assembler.rs` | 4 | Re-export from roko-neuro |
| `strategy.rs` | 97 | Composition strategy enum |
| `conventions.rs` | ~300 | Convention detection |
| `token_counter.rs` | ~150 | Token estimation utilities |
| `templates/common.rs` | 347 | Per-role budgets, stanza constants |
| `templates/*.rs` (11 files) | ~4817 total | Role template implementations |

---

## Sources

Key source files verified for this audit:

- `crates/roko-compose/src/system_prompt_builder.rs` -- SystemPromptBuilder, all build methods, layer definitions
- `crates/roko-compose/src/prompt_assembly_service.rs` -- PromptAssemblyService, full assembly pipeline
- `crates/roko-compose/src/role_prompts.rs` -- RoleSystemPromptSpec, all `build_*` methods
- `crates/roko-compose/src/prompt.rs` -- PromptSection, PromptComposer, estimate_tokens, VCG integration
- `crates/roko-compose/src/context_provider.rs` -- ContextTier, ContextBidderRegistry, is_local_model, LearningContextBidder
- `crates/roko-compose/src/budget.rs` -- Complexity enum, adjusted_budget_for(), cache break hints
- `crates/roko-compose/src/budget_predictor.rs` -- BudgetPredictor, SectionInfluence, persistence
- `crates/roko-compose/src/attention.rs` -- PositionAttentionModel, ModelAttentionCurves, dynamic_placement
- `crates/roko-compose/src/foraging.rs` -- MultiPatchForager, social_foraging_boost, should_stop_searching
- `crates/roko-compose/src/compaction.rs` -- compact_history, CompactionPolicy, anchor preservation
- `crates/roko-compose/src/strategy.rs` -- CompositionStrategy, DEFAULT_VCG_WARMUP_OBSERVATIONS = 10
- `crates/roko-compose/src/scorer.rs` -- SectionScorer, GoalDirectedHeuristicScorer
- `crates/roko-compose/src/cognitive_workspace.rs` -- build_cognitive_workspace, audit trail
- `crates/roko-compose/src/templates/common.rs` -- PromptBudget, budget_for(), reusable stanzas
- `crates/roko-compose/src/templates/mod.rs` -- 11 Template structs
- `crates/roko-compose/src/lib.rs` -- public exports
- `crates/roko-cli/src/orchestrate.rs` -- dispatch_agent_with(), build_system_prompt_with_context_validated()
- `crates/roko-cli/src/run.rs` -- roko run path using build_role_system_prompt_validated()
- `crates/roko-cli/src/dispatch_direct.rs` -- bare Claude CLI subprocess, no system prompt
- `crates/roko-acp/src/runner.rs` -- run_multi_role_review() inline prompts
