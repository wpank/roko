# E — Budget Management (Doc 05)

Parity analysis of `docs/03-composition/05-token-budget-management.md` vs actual codebase.

---

## E.01 — Tier 1: Static Per-Role Budgets via `budget_for()` (Doc 05 §1 Tier 1)

- **Status**: PARTIAL
- **Priority**: P2
- **Estimated LOC**: 40
- **Dependencies**: None
- **Files to modify**: `crates/roko-compose/src/templates/implementer.rs`, `reviewer.rs`, `scribe.rs`, `strategist.rs`, `quick.rs`, `integration.rs`, `task_impl.rs`

### What the doc says

Tier 1 is "the foundation. Each role receives a fixed allocation across 9 section categories via `budget_for(role)`." The function returns a `PromptBudget` struct with 9 fields (`plan`, `workspace_map`, `prd2`, `context`, `brief`, `reviews`, `instructions`, `file_context`, `skills`). These represent the baseline assumption about what each role needs.

### What exists

`PromptBudget` struct and the `budget_for()` lookup table live at `crates/roko-compose/src/templates/common.rs:17-36` (struct) and `common.rs:44-124` (function). All 9 documented section fields are present (`common.rs:18-35`). The function returns per-role character caps for 6 named roles plus a default branch:

| Role (code) | Plan | Workspace | PRD2 | Context | Brief | Reviews | Instructions | File | Skills |
|------------|------|-----------|------|---------|-------|---------|--------------|------|--------|
| `Implementer` (line 46) | 50_000 | 20_000 | 12_000 | 4_000 | 8_000 | 3_000 | 4_000 | 8_000 | 8_000 |
| `Strategist` (line 57) | 50_000 | 20_000 | 12_000 | 4_000 | 6_000 | 3_000 | 4_000 | 0 | 4_000 |
| `Architect` / `Auditor` (line 68) | 50_000 | 6_000 | 6_000 | 2_000 | 4_000 | 3_000 | 4_000 | 6_000 | 4_000 |
| `Scribe` / `Critic` (line 79) | 50_000 | 6_000 | 16_000 | 4_000 | 6_000 | 3_000 | 4_000 | 6_000 | 4_000 |
| `QuickReviewer` (line 90) | 50_000 | 6_000 | 0 | 0 | 4_000 | 3_000 | 2_000 | 0 | 0 |
| `AutoFixer` (line 101) | 0 | 0 | 0 | 0 | 0 | 0 | 2_000 | 0 | 0 |
| default (line 112) | 50_000 | 8_000 | 6_000 | 4_000 | 4_000 | 2_000 | 4_000 | 6_000 | 4_000 |

Re-exported via `templates/mod.rs:21` and `lib.rs` so downstream callers can import it. `budget_for` is `const fn` so it can be used in `const` contexts.

**The gap**: `budget_for()` is defined but NOT actually called from the role templates that are supposed to consume it. Grep for `budget_for(` or `common::budget_for` in `crates/roko-compose/src/templates/*.rs` returns zero matches outside of tests and the common.rs definition. Instead, each template file hard-codes literal cap values. For example `implementer.rs:76` uses `with_hard_cap(50_000)` for `plan_spec`, `implementer.rs:105` uses `with_hard_cap(20_000)` for `workspace_map`, etc. The numbers match the `Implementer` row of the lookup table but are manually duplicated in each template file.

Neither the orchestrator nor any template module imports `budget_for`: `grep -rn 'use .*budget_for' crates/` returns only the intra-file use inside `crates/roko-compose/src/budget.rs:15` (which `adjusted_budget_for` wraps — see E.02).

### Gaps

| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| E.01.1 | `budget_for()` is dead code outside its own tests and `adjusted_budget_for`; templates hard-code the same cap values inline instead of calling the table | `templates/implementer.rs:76`, `reviewer.rs:130-160`, `scribe.rs`, etc. vs `templates/common.rs:44` | MEDIUM (duplication, drift risk) |
| E.01.2 | No role template imports `budget_for` via `use templates::common::budget_for` | all `templates/*.rs` | MEDIUM |
| E.01.3 | `PromptBudget` field units are "characters" (see `common.rs:17` doc comment and `budget.rs:40`), but the `Budget::tokens(...)` passed to `PromptComposer::compose` at `orchestrate.rs:10487` is in tokens. The two budget systems are not reconciled | `templates/common.rs:17`, `orchestrate.rs:10487` | LOW (documentation mismatch) |

### Verify

```bash
# Confirm budget_for is not called from templates:
grep -rn 'budget_for\b' /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/ | grep -v 'fn budget_for' | grep -v '#\[test' | grep -v 'tests::' | grep -v test_
# Expected: no non-test hits

# Confirm templates hard-code the same values:
grep -n 'with_hard_cap' /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/implementer.rs
```

---

## E.02 — Tier 2: Complexity-Adaptive Budgets via `adjusted_budget_for()` (Doc 05 §1 Tier 2)

- **Status**: PARTIAL
- **Priority**: P1
- **Estimated LOC**: 60
- **Dependencies**: E.01
- **Files to modify**: `crates/roko-cli/src/orchestrate.rs`, `crates/roko-compose/src/templates/*.rs`

### What the doc says

> Overlaid on the static budgets. The `adjusted_budget_for(role, complexity)` function scales allocations up or down based on task complexity:
>
> | Complexity | Effect on Budget |
> |------------|------------------|
> | Trivial    | Drop PRD, context, skills. Halve workspace_map and brief. ~70% reduction. |
> | Standard   | No change. Base budget applies. |
> | Complex    | +50% workspace_map, +100% context, +50% file_context. ~40% increase. |

The scaled `AdjustedBudget` carries the adjusted `PromptBudget` plus metadata: dropped section names, cache break hints, the complexity band, and the role.

### What exists

`Complexity` enum (Trivial / Standard / Complex) at `crates/roko-compose/src/budget.rs:23-31` with `Standard` as `#[derive(Default)]`. Doc comments on each variant match §1 Tier 2 precisely.

`AdjustedBudget` struct at `budget.rs:38-51` with all 5 documented fields:
- `budget: PromptBudget` — the adjusted caps (line 40)
- `dropped_sections: Vec<&'static str>` — sections zeroed for Trivial (line 42)
- `cache_breaks: Vec<&'static str>` — cache layer boundaries (line 46)
- `complexity: Complexity` — the band that produced it (line 48)
- `role: AgentRole` — the base role (line 50)

`adjusted_budget_for(role, complexity)` at `budget.rs:66-121` implements the three-branch algorithm exactly as documented:
- Trivial (`budget.rs:70-88`): zeroes `prd2`, `context`, `skills`, halves `workspace_map` and `brief`, pushes dropped-section names into the vec.
- Standard (`budget.rs:89-91`): no-op branch, uses base.
- Complex (`budget.rs:92-97`): `workspace_map *= 3/2`, `context *= 2`, `file_context *= 3/2` (implemented as `saturating_mul(3)/2` and `saturating_mul(2)`).

Cache break hints are hardcoded to `["conventions", "workspace_map", "file_context"]` (`budget.rs:108-112`), matching the 4-tier `CacheLayer` model (System / Session / Task / Dynamic).

Unit tests `trivial_drops_prd_context_skills`, `trivial_halves_workspace_and_brief`, `complex_inflates_workspace_and_context`, etc. at `budget.rs:159-268` validate every branch.

Public re-exports at `crates/roko-compose/src/lib.rs:37`: `pub use budget::{AdjustedBudget, Complexity, adjusted_budget_for};`.

**The gap**: Like `budget_for`, the `adjusted_budget_for` function is never called from the orchestrator or any role template. `grep -rn 'adjusted_budget_for' crates/` outside `crates/roko-compose/src/budget.rs` returns zero matches in `crates/roko-cli/` and in `crates/roko-compose/src/templates/`. The function exists, is tested, and is exported — but no runtime consumer invokes it. Doc 13 line 54 claims "**Wired** via adjusted_budget_for()" which is incorrect given the code.

The orchestrator does pass a single fixed token total via `Budget::tokens(self.config.prompt.token_budget)` at `orchestrate.rs:10487`; the default is `10_000` (`config.rs:496-498`). There is no branching on `TaskComplexityBand` in the prompt-budget pipeline, even though `TaskComplexityBand` (Trivial/Standard/Complex) exists in `roko-core` and is populated on tasks by the curriculum/daimon (`crates/roko-learn/src/curriculum.rs:140-180`, `crates/roko-daimon/src/lib.rs:2235`).

### Gaps

| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| E.02.1 | `adjusted_budget_for` is never called at runtime. Doc 13's claim of "Wired" is false | `orchestrate.rs` vs `budget.rs:66` | HIGH (advertised feature not wired) |
| E.02.2 | `TaskComplexityBand` is set on tasks by curriculum/daimon but never read when building a prompt budget | `orchestrate.rs:10487`, `crates/roko-learn/src/curriculum.rs:140` | HIGH |
| E.02.3 | Cache break hints in `AdjustedBudget` are hardcoded (not computed per complexity) — the tree layer boundaries are right but the doc implied they could differ per complexity | `budget.rs:108-112` | LOW |

### Verify

```bash
# adjusted_budget_for is defined, tested, but never consumed:
grep -rn 'adjusted_budget_for\|AdjustedBudget' /Users/will/dev/nunchi/roko/roko/crates/ --include='*.rs' | grep -v 'crates/roko-compose/src/budget.rs' | grep -v 'crates/roko-compose/src/lib.rs'
# Expected: no hits in crates/roko-cli/ or crates/roko-compose/src/templates/

# Complexity band exists and is populated:
grep -n 'TaskComplexityBand::Trivial\|complexity_band = Some' /Users/will/dev/nunchi/roko/roko/crates/roko-daimon/src/lib.rs
```

---

## E.03 — Tier 3: Context-Tier Budgets (Surgical / Focused / Full) (Doc 05 §1 Tier 3)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says

> The outermost constraint. The context tier (Surgical/Focused/Full) sets the absolute maximum token budget:
>
> | Context Tier | Max Tokens | Model Class |
> |--------------|------------|-------------|
> | Surgical     | 4,000      | Haiku, Ollama, local models |
> | Focused      | 12,000     | Sonnet |
> | Full         | 24,000     | Opus |

The tightest constraint wins; tier derivation is based on task + model slug, with local models forced to Surgical.

### What exists

`ContextTier` enum at `crates/roko-compose/src/context_provider.rs:34-42` with the 3 documented variants, with doc comments that match §1 Tier 3 word-for-word.

`ContextTier::from_task_and_model(task_tier, model_slug)` at `context_provider.rs:50-60`:
- local model -> `Surgical`
- task_tier == "mechanical" -> `Surgical`
- task_tier == "architectural" -> `Full`
- otherwise -> `Focused`

`ContextTier::default_token_budget()` at `context_provider.rs:64-70` returns exactly the documented constants: `Surgical => 4_000`, `Focused => 12_000`, `Full => 24_000`.

`is_local_model()` at `context_provider.rs:95-111` classifies model slugs; supports `ollama/`, `llama`, `gemma`, `qwen`, `mistral`, `codellama`, `deepseek`, `phi`, `starcoder`, and the generic `":"`-containing pattern with exclusions for `claude`/`gpt`/`composer`/`cursor`.

`ContextBudgets` config at `context_provider.rs:297-314` exposes the tier caps as a struct so callers can override. `with_budgets(...)` builder at `context_provider.rs:481` wires it into `ContextProvider`. Test at `context_provider.rs:1326-1331` asserts defaults.

The tier is wired end-to-end through `roko.toml`:
- `ContextBudgetConfig` config struct at `crates/roko-cli/src/config.rs:431-441` with default constants `4_000 / 12_000 / 24_000` at lines 444-452.
- `to_context_budgets()` converter at `config.rs:456-462` maps to `roko_compose::ContextBudgets`.
- Actually applied at `crates/roko-cli/src/orchestrate.rs:10133` via `.with_budgets(self.config.prompt.context_budgets.to_context_budgets())`.

Bidirectional `From` conversion with `OperatingFrequency` at `context_provider.rs:73-91` (`Gamma`↔`Surgical`, `Theta`↔`Focused`, `Delta`↔`Full`) integrates with the 3-speed policy.

### Gaps

None at the tier-budget layer. The three defaults match the doc, the roko.toml wiring works, and local-model forcing matches the doc's "Surgical for local models" rule.

### Verify

```bash
# Tier defaults match doc:
grep -n 'Self::Surgical => 4_000\|Self::Focused => 12_000\|Self::Full => 24_000' /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/context_provider.rs

# roko.toml wiring:
grep -n 'context_budgets' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/config.rs /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs
```

---

## E.04 — Complexity Enum + AdjustedBudget Metadata (Doc 05 §1)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: E.02 (to actually use the metadata)
- **Files to modify**: None (defined correctly; unused — see E.02)

### What the doc says

Tier 2 adjustments must record which sections were dropped (for audit/logging) and emit cache-break hints so prompt caching boundaries align with the stability tier model.

### What exists

Both structures are fully specified and correct:

`Complexity` enum at `budget.rs:23-31`:
```rust
pub enum Complexity {
    Trivial,          // "Drop PRD, research, decomposition sections"
    #[default] Standard, // "Full budget at role defaults"
    Complex,          // "Inflated budgets for context"
}
```
Doc comments match the doc 05 table row-for-row.

`AdjustedBudget` struct at `budget.rs:38-51` — all 5 fields present:

| Field | Type | Line | Doc match |
|-------|------|------|-----------|
| `budget` | `PromptBudget` | 40 | MATCH |
| `dropped_sections` | `Vec<&'static str>` | 42 | MATCH |
| `cache_breaks` | `Vec<&'static str>` | 46 | MATCH |
| `complexity` | `Complexity` | 48 | MATCH |
| `role` | `AgentRole` | 50 | MATCH |

Helper functions present and tested:
- `total_budget(&PromptBudget) -> usize` at `budget.rs:128-138` sums all 9 section caps.
- `is_cache_break(&AdjustedBudget, section_name: &str) -> bool` at `budget.rs:142-144`.
- `cache_marker(layer_name: &str) -> String` at `budget.rs:151-153` emits HTML-comment markers like `<!-- cache:session -->` that the prompt renderer can read to set `cache_control` on API calls.

Cache break list (`["conventions", "workspace_map", "file_context"]` at `budget.rs:108-112`) maps to:
- `conventions` -> end of System layer
- `workspace_map` -> end of Session layer
- `file_context` -> end of Task layer

Dropped-section tracking: `Trivial` branch pushes `"prd2"`, `"context"`, `"skills"` into `dropped_sections` (`budget.rs:74-84`). Test `trivial_drops_prd_context_skills` at `budget.rs:160-168` confirms all three names appear in the vec.

Notable nuance: the `AutoFixer` role test at `budget.rs:197-204` verifies that sections which are *already zero* for a role are NOT added to `dropped_sections` — only sections that had a positive allocation and were then zeroed appear there. This is correct behavior (nothing to drop).

### Gaps

| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| E.04.1 | The struct is correct but unused at runtime because E.02 is not wired — `cache_marker()` output never reaches the prompt renderer | `budget.rs:151` (defined), `orchestrate.rs` (not called) | MEDIUM (inherits from E.02.1) |

### Verify

```bash
# All three Complexity variants and AdjustedBudget fields:
grep -n 'pub enum Complexity\|pub struct AdjustedBudget\|pub struct PromptBudget\|dropped_sections\|cache_breaks' /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/budget.rs

# Tests cover all three branches:
grep -n '^    fn ' /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/budget.rs
```

---

## E.05 — TokenCounter (Tiktoken + HuggingFace + Heuristic) (Doc 05 implicit, §11 references `estimate_tokens`)

- **Status**: DONE (exceeds spec)
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says

Budget accounting requires counting tokens accurately per model family. The doc references `estimate_tokens(&task.description)` in §11.2 and shows a `TaskFeatureExtractor` producing token counts. Doc 05 does not prescribe a specific tokenizer library but implies real-tokenizer semantics ("under the model's context window") rather than a character-based approximation.

### What exists

`TokenCounter` enum at `crates/roko-compose/src/token_counter.rs:9-19` with 3 strategies:

| Variant | Library | Applies to |
|---------|---------|-----------|
| `Tiktoken(tiktoken_rs::CoreBPE)` (line 11) | `tiktoken-rs` (OpenAI BPE) | Claude, GPT, `o1` models |
| `HuggingFace(tokenizers::Tokenizer)` (line 13) | `tokenizers` crate | GLM, Kimi (loaded from local HF cache) |
| `Heuristic { chars_per_token: f64 }` (line 15) | char-based fallback | unknown models |

`TokenCounter::count(&self, text) -> usize` at `token_counter.rs:24-33` dispatches to the backend. Tiktoken uses `encode_with_special_tokens(text).len()`. HuggingFace uses `tokenizer.encode(text, false).get_ids().len()`. Heuristic divides by `chars_per_token` and rounds up (`token_counter.rs:70-76`).

`TokenCounter::for_model(slug)` at `token_counter.rs:37-61` selects the best available strategy per model slug:
- `"claude-"`, `"gpt-"`, `"o1"` -> `Tiktoken` using `tiktoken_rs::o200k_base()` with heuristic fallback `chars_per_token: 4.0`.
- `"glm-"` -> HuggingFace `zai-org/GLM-4.7` with heuristic `3.8` fallback.
- `"kimi-"` -> HuggingFace `moonshotai/Kimi-K2-Instruct` with heuristic `3.5` fallback.
- default -> `Heuristic { chars_per_token: 4.0 }`.

HF tokenizer discovery at `token_counter.rs:78-145` walks standard HF cache roots (`HF_HUB_CACHE`, `HUGGINGFACE_HUB_CACHE`, `HF_HOME/hub`, `~/.cache/huggingface/hub`) looking for `tokenizer.json` either directly, via `refs/main` snapshot, or first match in the snapshots directory.

Tests at `token_counter.rs:147-175`:
- `token_counter_for_claude_uses_tiktoken` — asserts Claude picks Tiktoken
- `token_counter_for_glm_counts_reasonably` — 1..=6 tokens for "hello world"
- `token_counter_heuristic_is_conservative_for_non_empty_text` — `"a"` -> 1, `"hello world"` -> 3

This is **a real tokenizer**, not a char-approximation. The doc question "is it wrapping tiktoken or char-approx?" resolves to: **both, smartly chosen by model family**, with char-approx only as a documented fallback.

### Gaps

| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| E.05.1 | Claude models route to `tiktoken_rs::o200k_base()`, which is the GPT-4o tokenizer — not Anthropic's actual BPE. Character-count parity between o200k_base and Anthropic's tokenizer is approximate, not exact | `token_counter.rs:38-44` | LOW (no public Anthropic BPE available in Rust) |
| E.05.2 | HuggingFace discovery requires the tokenizer to be pre-downloaded into the HF cache. No runtime download path | `token_counter.rs:78-109` | LOW (matches the doc's "no LLM calls for counting" principle) |

### Verify

```bash
# Confirm real tokenizers (not char-based for Claude/GPT):
grep -n 'tiktoken_rs::CoreBPE\|tokenizers::Tokenizer\|o200k_base' /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/token_counter.rs

# Confirm dependency on tiktoken-rs and tokenizers:
grep -n 'tiktoken-rs\|tokenizers' /Users/will/dev/nunchi/roko/roko/crates/roko-compose/Cargo.toml
```

---

## E.06 — Dynamic Token Budget + A/B Experiments + Min-Tokens Guard (Doc 05 §3 + §5 + §11 + §12)

- **Status**: PARTIAL (A/B learned section weights wired; BudgetPredictor + BudgetOutcome + leave-one-out influence NOT built)
- **Priority**: P2
- **Estimated LOC**: 400
- **Dependencies**: E.01, E.02
- **Files to modify**: `crates/roko-compose/src/budget.rs`, `crates/roko-cli/src/orchestrate.rs`, `crates/roko-learn/src/` (new `budget_outcomes.rs`)

### What the doc says

Three related capabilities specified in doc 05:

1. **Min-tokens guard (§3)**: each section has a `min_tokens` floor; if remaining budget can't fit at least `min_tokens` of content for a section, drop it entirely rather than include a uselessly truncated stub.
2. **A/B testing protocol (§5)**: integrate with `ExperimentStore` from `roko-learn` to A/B-test individual sections across 50+ plans, apply a decision matrix keyed on pass-rate delta and cost delta.
3. **Budget prediction + learning (§11, §12)**: `BudgetPredictor` with regression model and `BudgetObservation`/`BudgetOutcome` structs for recording and learning which section allocations correlate with gate passes; leave-one-out influence computation (`compute_section_influence`) persists to `.roko/learn/budget-allocations.json`.

Doc 13 status line 648 says A/B is "Scaffold (ExperimentStore exists)"; lines 649-651 say prediction / learning / density-scoring are "Designed" only.

### What exists

**A/B infrastructure (partial):**
- `ExperimentStore` at `crates/roko-learn/src/prompt_experiment.rs:349` — exists, supports persistence (`load_or_new`, `save`). Referenced from `roko-serve` routes, `roko-cli` main, `orchestrate.rs`, TUI dashboard. But not used to modulate composer budgets.
- `ModelExperimentStore` at `crates/roko-learn/src/model_experiment.rs:207` — active A/B store for cascade-router model selection (different problem than section-budget A/B).

**Learned section-tuning IS wired (surprise vs doc 13):**
- `SectionEffectivenessRegistry` at `crates/roko-learn/src/section_effect.rs:114` tracks `(section_name, role)` -> `SectionEffect { included_trials, included_passes, excluded_trials, excluded_passes }`. Computes `lift()` (`section_effect.rs:73-77`) and `lift_weight()` clamped to `[0.5, 1.5]` (`section_effect.rs:84-86`). Persists to `.roko/learn/section-effects.json` (`DEFAULT_SECTION_EFFECTS_PATH` at line 13). Requires `>=20` included + `>=5` excluded trials before recommending a priority change (`section_effect.rs:97`).
- `SystemPromptBuilder::apply_learned_budget_tuning` at `crates/roko-compose/src/system_prompt_builder.rs:630-698` actively consumes this data: per-section lift weights scale each section's share of the token budget, then floors/remainders redistribute the budget to keep the sum equal to `token_budget`. Tracing logs per-section `base_tokens`, `weight`, `tuned_cap` at line 689-696.
- Wired to the orchestrator: `orchestrate.rs:10255` calls `self.learning.section_effectiveness_snapshot()` and passes the registry into `SystemPromptBuilder` via `with_section_effectiveness(role, &registry)` (`orchestrate.rs:10271`), which stores it as `SectionEffectivenessConfig` at `system_prompt_builder.rs:82-85`. Effective priority shifts are applied at `orchestrate.rs:10291-10445` for every section type (learned, tools, relevant techniques, anti-patterns, etc.).
- Feedback loop: `RuntimeFeedback::section_effectiveness_snapshot` at `crates/roko-learn/src/runtime_feedback.rs:597` exposes the tracker; `record_outcome` at `section_effect.rs:155-170` is updated from task results.

**Min-tokens guard — NOT implemented:**
- `grep -rn 'min_tokens' crates/` returns zero matches anywhere. The `PromptComposer::compose` algorithm at `crates/roko-compose/src/prompt.rs:322-369` splits sections into Critical vs optional and then runs a greedy auction-style allocator over the optional sections, but it does not enforce a per-section floor. Truncation uses `hard_cap` (`prompt.rs:337` calls `enforce_hard_cap`) which is a per-section maximum, not a minimum viability threshold.
- The "better drop than truncate to 100 tokens" logic the doc prescribes (`05-token-budget-management.md:107-119`) is entirely absent.

**BudgetPredictor / BudgetOutcome / section influence (§11 + §12) — NOT implemented:**
- `grep -rn 'BudgetPredictor\|BudgetOutcome\|section_influence\|compute_section_influence\|budget-allocations' crates/` returns zero matches. None of these types exist.
- `SectionAllocationRecord` struct specified at doc 05 §12.1 lines 485-492 not present.
- `.roko/learn/budget-allocations.json` persistence path specified at doc 05 §12.5 line 613 does not exist (only `.roko/learn/section-effects.json` from the lift-weight system exists).
- No leave-one-out computation, no n-gram novelty scoring (doc §12.4), no regression model for per-task budget prediction.

**History compaction (Doc 05 §6) — DONE but orthogonal:**
- `compact_history` at `crates/roko-compose/src/compaction.rs:70` with `CompactionPolicy`. Re-exported via `lib.rs:38`. Separate compaction implementation at `crates/roko-agent/src/tool_loop/compaction.rs`. Not called "dynamic budget" but related to the §6 algorithm.

**Context anxiety mitigation (Doc 05 §7) — partially wired:**
- Most Claude backends request max context window. The doc's prescription "always request 1M tokens regardless of actual usage" is not enforced in one place but individual dispatchers set large limits.

### Gaps

| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| E.06.1 | No `min_tokens` per-section floor anywhere in the composer — sections are silently truncated to uselessly small sizes instead of dropped. Violates doc 05 §3's "A workspace map truncated to 100 tokens is worse than no workspace map" principle | `crates/roko-compose/src/prompt.rs:322-369`, `templates/*.rs` | MEDIUM |
| E.06.2 | `BudgetPredictor` regression model (doc §11.2) not implemented | does not exist | HIGH (design-only per doc 13) |
| E.06.3 | `BudgetObservation` / `BudgetOutcome` / `SectionAllocationRecord` structs (doc §12.1) not implemented | does not exist | HIGH |
| E.06.4 | Leave-one-out section influence computation (doc §12.2) not implemented. The `SectionEffectivenessRegistry` lift-weight tracker is a simpler correlate (requires randomly-dropped inclusion events rather than controlled experiments), not the specified mutual-information formulation | `section_effect.rs:73` vs doc §12.2 | MEDIUM (overlap but not equivalent) |
| E.06.5 | No `.roko/learn/budget-allocations.json` persistence (doc §12.5). Only `.roko/learn/section-effects.json` exists, which stores trial counts rather than adjusted allocations | `section_effect.rs:13` vs doc §12.5 line 613 | MEDIUM |
| E.06.6 | Section A/B testing (doc §5) decision matrix and 50-plan threshold are not implemented. `ExperimentStore` is wired for model A/B but not for per-section budget A/B | `crates/roko-learn/src/prompt_experiment.rs`, no section-budget callers | HIGH |
| E.06.7 | N-gram information-density scoring (doc §12.4) not implemented | does not exist | LOW (optimization, not core) |
| E.06.8 | Doc 13 line 648 ("A/B testing framework — Scaffold") underreports: `SectionEffectivenessRegistry` + `apply_learned_budget_tuning` is an independently-wired lift-weight mechanism that goes beyond scaffold. Doc should credit this as PARTIAL DONE, not "Scaffold". But it's NOT the §5 / §12 design the doc specifies | `system_prompt_builder.rs:630`, doc 13:648 | LOW (doc outdated in opposite direction from expectation) |

### Verify

```bash
# min_tokens is nowhere:
grep -rn 'min_tokens\|MinTokens\|min_useful' /Users/will/dev/nunchi/roko/roko/crates/ --include='*.rs'
# Expected: zero hits

# BudgetPredictor / BudgetOutcome absent:
grep -rn 'BudgetPredictor\|BudgetOutcome\|SectionAllocationRecord\|compute_section_influence' /Users/will/dev/nunchi/roko/roko/crates/ --include='*.rs'
# Expected: zero hits

# No budget-allocations.json:
grep -rn 'budget-allocations.json' /Users/will/dev/nunchi/roko/roko/crates/ --include='*.rs'
# Expected: zero hits

# Learned lift weighting IS wired:
grep -n 'apply_learned_budget_tuning\|section_lift_weight\|SectionEffectivenessRegistry' /Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs
grep -n 'section_effectiveness_snapshot\|with_section_effectiveness' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs | head -5
```

---

## Section Summary

| Item | Topic | Status | Parity |
|------|-------|--------|--------|
| E.01 | Static per-role budgets via `budget_for()` | PARTIAL | 70% — table exists, templates duplicate values inline instead of calling the function |
| E.02 | Complexity-adaptive budgets via `adjusted_budget_for()` | PARTIAL | 50% — function + tests exist, never called at runtime (doc 13 claim of "Wired" is incorrect) |
| E.03 | Context-tier budgets Surgical / Focused / Full | DONE | 100% — exact constants, roko.toml wiring, local-model forcing all match |
| E.04 | `Complexity` enum + `AdjustedBudget` metadata (dropped sections, cache breaks) | DONE | 100% — struct and enum match spec, tests cover all branches (usage gap inherits from E.02) |
| E.05 | TokenCounter | DONE | 110% — real Tiktoken / HF tokenizers, char-heuristic only as fallback. Exceeds the "is it char-based?" expectation |
| E.06 | Min-tokens guard + dynamic A/B budgets + prediction/learning | PARTIAL | 30% — lift-weight section tuning is wired (beyond doc 13 claim); min_tokens absent; BudgetPredictor/BudgetOutcome/leave-one-out/n-gram scoring all unbuilt |

### Status counts

- DONE: 2 (E.03, E.04, E.05 — actually 3)
- PARTIAL: 3 (E.01, E.02, E.06)
- NOT DONE: 0
- SCAFFOLD: 0

### Big surprises

1. **`budget_for()` and `adjusted_budget_for()` are dead code at runtime.** They are defined, tested, exported, and referenced by doc 13 as "Wired" — but `grep -rn` proves no template file and no orchestrator path calls them. Templates hard-code the same cap values by hand (`templates/implementer.rs:76` says `.with_hard_cap(50_000)` matching `budget_for(Implementer).plan = 50_000` at `common.rs:47`). The Tier 1 lookup table is a source-of-truth specification that nobody reads.
2. **Complexity-adaptive budgets are a pure spec + test artifact.** `TaskComplexityBand` is populated on tasks by curriculum/daimon but never consumed at prompt-build time. The documented 70% / 40% adjustments have no runtime effect.
3. **Doc 13 under-credits the learned section tuning.** The `SectionEffectivenessRegistry` + `apply_learned_budget_tuning` pipeline at `system_prompt_builder.rs:630` actively reshapes section hard caps based on observed inclusion/pass-rate lift, persists to `.roko/learn/section-effects.json`, and is wired through `orchestrate.rs:10255` for every section type. Doc 13 calls A/B "Scaffold" — that's wrong in the generous direction: a real learned-tuning system exists, just not the one doc 05 §5 specifies.
4. **TokenCounter is a real tokenizer.** The doc left the question open; the code uses `tiktoken_rs::o200k_base()` for Claude/GPT/o1 and `tokenizers::Tokenizer::from_file(...)` for GLM/Kimi via the HF cache. Char-approx is only a fallback for unknown models. This exceeds typical "scaffold" expectations.
5. **`min_tokens` is entirely absent.** The "Sufficient Context" / LLMLingua-derived principle — drop a section rather than truncate it to uselessness — has zero implementation. Sections that don't fit get hard-capped (max only), not dropped.

### Priority actions

1. **P1** (E.02.1): Wire `adjusted_budget_for(role, complexity)` into the orchestrator so `TaskComplexityBand` actually modulates the prompt budget. Replace the flat `Budget::tokens(self.config.prompt.token_budget)` at `orchestrate.rs:10487` with a complexity-aware lookup. Correct doc 13 status line 54.
2. **P2** (E.01.1): Refactor role templates to call `budget_for(role)` instead of duplicating the cap numbers. Makes the lookup table the single source of truth.
3. **P2** (E.06.1): Add a `min_tokens` floor to `PromptSection` (or `PromptBudget`) and implement the drop-vs-truncate rule in `PromptComposer::compose`. Even a blunt 100-token floor across the board would close the biggest hole.
4. **P2** (E.06.2 + E.06.3): Implement `BudgetOutcome` + `SectionAllocationRecord` persistence so outcome data exists; defer the `BudgetPredictor` regression model until there is training data.

---

## Agent Execution Notes

### E.01 / E.02 — Budget Activation

These should be composition-owned, runtime-facing batches.

Recommended sequence:

1. static role budgets first,
2. complexity modulation second,
3. then add stronger drop-vs-truncate behavior.

Acceptance criteria:

- the live prompt path no longer ignores the documented budget system,
- complexity affects prompt composition on at least one real path,
- the resulting behavior is visible in tests or dry-run output.

### E.06 — Min-Useful-Context Guard

Do not widen this into full predictive budgeting.

Good outcome:

- small unusable sections get dropped,
- the drop is observable in prompt metadata,
- future learning/eval work has a clearer, safer baseline.

Defer from this item:

- `BudgetPredictor`,
- `BudgetOutcome`,
- leave-one-out section influence,
- n-gram density scoring.
