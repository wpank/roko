# Task Plan: Prompt Assembly Subsystem

> 38 tasks across 7 phases. Wire existing infrastructure into live paths.
>
> Core thesis: the 9-layer SystemPromptBuilder, ContextTier, BudgetPredictor,
> SectionInfluence, MultiPatchForager, CompactionPolicy, VCG auction, and
> ModelAttentionCurves are all built and tested. None of them are connected to
> dispatch. This plan connects them.

---

## Overview

The prompt assembly subsystem in `roko-compose` is the most feature-rich and
least-wired component in the entire codebase. The 9-layer SystemPromptBuilder
(2081 LOC), demand-driven ContextTier (4K/12K/24K token budgets), EMA-based
BudgetPredictor (679 LOC), leave-one-out SectionInfluence, MultiPatchForager
(438 LOC), CompactionPolicy (488 LOC), VCG auction (688 LOC), and
per-model ModelAttentionCurves are all built, tested, and serializable.

The problem: almost none of them are actually called from the runtime.

- `dispatch_agent_with()` in orchestrate.rs never calls `ContextTier::from_task_and_model()`
- `BudgetPredictor.predict()` is never called before assembly
- `BudgetPredictor.record()` is never called after gate results
- `SectionInfluence.weights()` are computed but never fed back into allocation
- `roko chat` uses `SystemPromptBuilder` directly but not `PromptAssemblyService`
- `dispatch_direct.rs` has zero system prompt (no builder, no templates, nothing)
- ACP `run_multi_role_review()` hardcodes role descriptions in `format!()` strings
- `MultiPatchForager` is exported but never instantiated in dispatch
- `compact_history()` is ready but not called from `roko chat`
- VCG warmup threshold (10) means DensityGreedy always dominates
- `ModelAttentionCurves` only has the default curve (no per-model fits)
- Knowledge confidence thresholds are hardcoded (0.5/0.3/0.2) regardless of tier
- Episode context is always "last 5" regardless of tier
- Workspace map cap is fixed at 200 lines regardless of tier

This plan wires every built component into its correct runtime path.

---

## Anti-Patterns to Remove

### AP-1: Static per-role budgets ignore model context window
`crates/roko-compose/src/templates/common.rs` `budget_for()` returns
character caps designed for 200K-context models. An Implementer's total
budget is ~109K chars (~27K tokens). An Ollama model with 4K context gets
the same prompt. The `ContextTier` system in `context_provider.rs` defines
the right budgets but is not wired into the builder path.

### AP-2: Inline `format!()` prompt strings in orchestrate.rs and ACP runner
Multiple sites in `orchestrate.rs` (lines ~9399, ~9846, ~10437, ~14131,
~14558) use `format!("Plan: {plan_id}\nTask: {task_id}\n\nImplement...")`.
ACP `runner.rs` (lines ~1527-1544) hardcodes full role descriptions for
"Architect Reviewer" and "Security & Correctness Auditor" in `format!()`
strings that duplicate `ReviewerTemplate`.

### AP-3: Binary section effectiveness threshold
`prompt_assembly_service.rs` line 189: `should_include()` hard-excludes
sections with score < 0.1. Score 0.09 = dropped. Score 0.11 = full budget.
No graduated scaling.

### AP-4: Flat 4.0 chars/token heuristic everywhere
`prompt.rs` and `prompt_assembly_service.rs` use `chars_per_token: 4.0`
universally. Code-heavy content is closer to 3.0 chars/token; markdown
is closer to 5.0. The `TokenCounter` already supports `Tiktoken` and
`HuggingFace` variants but `PromptAssemblyService` always uses `Heuristic`.

### AP-5: Fixed "last 5 episodes" and "200-line workspace map"
`prompt_assembly_service.rs` constants `WORKSPACE_MAP_LINE_LIMIT = 200`
and hardcoded `.take(5)` for episodes are not tier-aware.

### AP-6: Hardcoded knowledge confidence thresholds
`prompt_assembly_service.rs`: domain facts >= 0.5, techniques >= 0.3,
anti-patterns >= 0.2. These are appropriate for Full tier but too
permissive for Surgical (wastes precious context on uncertain knowledge).

---

## Phase 1: Model-Aware Context Windowing

**Problem**: Per-role budgets total ~109K chars for an Implementer. An
Ollama model with 4K context gets the same prompt as Opus with 200K.
The `ContextTier` system defines the right budgets but is not wired.

**Effort**: 2-3 days | **Impact**: Critical (user's core pain point)
**Dependencies**: None
**Issue refs**: ISS-01, ISS-06

---

### Task 6.1: Add `model_slug` and `context_tier` to PromptAssemblyService

**File**: `crates/roko-compose/src/prompt_assembly_service.rs`

**What**: Add optional `model_slug` and derived `context_tier` fields. When
set, the tier's `default_token_budget()` overrides the static `token_budget`.

**Steps**:
1. Add fields to `PromptAssemblyService`:
   ```rust
   model_slug: Option<String>,
   context_tier: Option<ContextTier>,
   ```
2. Add builder methods:
   - `with_model_slug(slug: String) -> Self` -- sets `model_slug`
   - `with_context_tier(tier: ContextTier) -> Self` -- sets `context_tier`
3. In `assemble()`, compute `effective_budget`:
   - If `context_tier` is set: use `tier.default_token_budget()`
   - Else if `model_slug` is set: derive tier via `ContextTier::from_task_and_model()`, use its budget
   - Else if `token_budget` is set: use that
   - Else: no budget (unbounded, existing behavior)
4. Use `effective_budget` where `self.token_budget` was previously used (line 469)
5. Initialize both fields to `None` in `new()`

**Search before implementing**:
```bash
grep -rn 'model_slug\|context_tier\|ContextTier' crates/roko-compose/src/prompt_assembly_service.rs
```

**Acceptance criteria**:
- `PromptAssemblyService::new().with_model_slug("ollama/llama3.2".into())` produces assembly with budget <= 4000 tokens
- `PromptAssemblyService::new().with_model_slug("claude-sonnet-4-20250514".into())` produces budget <= 12000 tokens
- `PromptAssemblyService::new().with_model_slug("claude-opus-4-20250514".into())` produces budget <= 24000 tokens
- Existing callers that set `token_budget` continue to work unchanged
- `cargo test -p roko-compose` passes

---

### Task 6.2: Add `tier_scaled_budget()` to budget.rs

**File**: `crates/roko-compose/src/budget.rs`

**What**: Add a function that proportionally scales a `PromptBudget` to fit
within a `ContextTier`'s token budget. Uses the existing `total_budget()`
helper at line 128.

**Steps**:
1. Import `ContextTier` from `crate::context_provider`
2. Add `pub fn tier_scaled_budget(base: PromptBudget, tier: ContextTier) -> PromptBudget`
3. Compute `base_total = total_budget(&base)` (existing function, line 128)
4. Compute `tier_total = tier.default_token_budget() * 4` (tokens-to-chars heuristic)
5. If `tier_total >= base_total`: return `base` unchanged
6. Compute `scale = tier_total as f64 / base_total as f64`
7. Scale each field: `(field as f64 * scale) as usize`
8. Add unit tests

**Acceptance criteria**:
- `tier_scaled_budget(budget_for(AgentRole::Implementer), ContextTier::Surgical)` produces total <= 16000 chars (~4K tokens)
- `tier_scaled_budget(budget_for(AgentRole::Implementer), ContextTier::Full)` produces total <= 96000 chars (~24K tokens)
- No field goes negative
- `cargo test -p roko-compose` passes

---

### Task 6.3: Add Tier-Dependent Section Eligibility to ContextTier

**File**: `crates/roko-compose/src/context_provider.rs`

**What**: Add `eligible_sections()` and `is_eligible()` methods to
`ContextTier` (currently defined at line 39).

**Steps**:
1. Add method to `ContextTier` impl block (starts at line 48):
   ```rust
   pub fn eligible_sections(&self) -> &'static [&'static str]
   ```
2. Return values:
   - Surgical: `["identity", "task", "tools", "anti_patterns", "verification"]`
   - Focused: Surgical + `["conventions", "playbooks", "gate_feedback", "brief", "file_context"]`
   - Full: Focused + `["domain", "context", "workspace_map", "prd", "research", "episodes", "affect"]`
3. Add convenience method:
   ```rust
   pub fn is_eligible(&self, section_name: &str) -> bool
   ```
4. Add unit tests for all tier/section combinations

**Acceptance criteria**:
- `ContextTier::Surgical.is_eligible("workspace_map")` returns false
- `ContextTier::Surgical.is_eligible("task")` returns true
- `ContextTier::Full.is_eligible("workspace_map")` returns true
- `cargo test -p roko-compose` passes

---

### Task 6.4: Wire Tier Eligibility into PromptAssemblyService

**File**: `crates/roko-compose/src/prompt_assembly_service.rs`

**What**: When `context_tier` is set, hard-exclude sections that the tier
does not support, before the effectiveness threshold check.

**Steps**:
1. In `should_include()` (line 185), add a tier eligibility check before the
   effectiveness check:
   ```rust
   if let Some(tier) = self.context_tier {
       if !tier.is_eligible(section) {
           tracing::debug!(section, ?tier, "excluded by tier ineligibility");
           return false;
       }
   }
   ```
2. Make `WORKSPACE_MAP_LINE_LIMIT` tier-dependent:
   - Surgical: 0 (no workspace map)
   - Focused: 100
   - Full: 300
   - No tier set: 200 (current default)
3. Use the tier-aware limit in `workspace_map_for_spec()` or `workspace_map_from_file_listing()`

**Acceptance criteria**:
- Assembly with `ContextTier::Surgical` produces a prompt containing only identity, task, tools, anti-patterns, and verification content
- Assembly with `ContextTier::Surgical` has zero workspace_map content
- Assembly with `ContextTier::Full` includes all sections that pass effectiveness threshold
- `cargo test -p roko-compose` passes

---

### Task 6.5: Thread model_slug Through dispatch_agent_with()

**File**: `crates/roko-cli/src/orchestrate.rs`

**What**: Pass the CascadeRouter-selected model slug into prompt assembly
so `ContextTier` is consulted before building the system prompt. The
function is at line 14469.

**Steps**:
1. In `dispatch_agent_with()`, after model selection (CascadeRouter), extract `model_slug`
2. Compute `ContextTier::from_task_and_model(&task_tier_string, &model_slug)`
3. Pass model slug/tier into prompt assembly (via `PromptAssemblyService::with_model_slug()` or directly into `build_system_prompt_with_context_validated()`)
4. Use `tier_scaled_budget()` to scale the per-role budget before passing to the builder
5. Log the selected tier at `info!` level: `"context_tier={tier:?} model={model_slug}"`

**Search before implementing**:
```bash
grep -n 'CascadeRouter\|model_slug\|selected_model\|model_selection' crates/roko-cli/src/orchestrate.rs | head -30
```

**Acceptance criteria**:
- Run `roko plan run` with a task configured for Ollama backend: system prompt <= 4K tokens (verify via log)
- Run with Opus backend: system prompt <= 24K tokens
- Log line shows "context_tier=Surgical" or "context_tier=Full" as appropriate
- `cargo test -p roko-cli` passes

---

### Task 6.6: Thread model_slug Through run.rs Path

**File**: `crates/roko-cli/src/run.rs`

**What**: The `roko run` path also uses `build_role_system_prompt_validated()`
(line ~1401). Thread model slug into this path.

**Steps**:
1. In the `roko run` handler, resolve the model slug from config or the
   `EffectiveModelSelection`
2. Compute ContextTier from model slug
3. Pass tier budget as the `context_window_tokens` parameter (currently
   hardcoded from `config.prompt.token_budget` at line 1398)
4. The same ContextTier logic from Task 6.5 applies here

**Acceptance criteria**:
- `roko run "test prompt"` with a configured Ollama model produces system prompt <= 4K tokens
- `roko run "test prompt"` with Opus produces system prompt <= 24K tokens
- `cargo test -p roko-cli` passes

---

## Phase 2: Wire BudgetPredictor

**Problem**: `BudgetPredictor` is fully built (EMA-based, failure
inflation, partial-match fallback, persistence, 679 LOC) but nobody calls
`predictor.predict()` before assembly or `predictor.record()` after gate results.

**Effort**: 1-2 days | **Impact**: Critical (enables budget convergence)
**Dependencies**: Phase 1 (tier provides outer envelope; predictor refines within)
**Issue refs**: ISS-02

---

### Task 6.7: Load BudgetPredictor at Plan Run Startup

**File**: `crates/roko-cli/src/orchestrate.rs`

**What**: Load `BudgetPredictor` from `.roko/learn/budget-predictor.json` at
plan run startup.

**Steps**:
1. Import `roko_compose::budget_predictor::{BudgetPredictor, TaskFeatures, load_predictor, persist_predictor}`
2. In plan runner init, load predictor: `load_predictor(&learn_dir).unwrap_or_default().unwrap_or_default()`
3. Wrap in `Arc<Mutex<BudgetPredictor>>` and attach to the runner context
4. At run end (or periodically), persist with `persist_predictor(&predictor, &learn_dir)`

**Search before implementing**:
```bash
grep -rn 'BudgetPredictor\|budget_predictor' crates/roko-cli/src/orchestrate.rs
```

**Acceptance criteria**:
- `roko plan run` loads predictor without error (even if file does not exist)
- After run completes, `.roko/learn/budget-predictor.json` exists
- Second run loads the file produced by the first run
- `cargo test -p roko-cli` passes

---

### Task 6.8: Call predict() Before Assembly

**File**: `crates/roko-cli/src/orchestrate.rs`

**What**: Before building the system prompt, call `predictor.predict()`.
If the predictor has history, use its estimate (clamped to tier budget).

**Steps**:
1. Construct `TaskFeatures` from the current task's role, complexity, and domain
2. Call `predictor.lock().unwrap().predict(&features)`
3. If predictor `has_history(&features)`: use `min(predicted_budget, tier.default_token_budget())`
4. If no history: use `tier.default_token_budget()` (Phase 1 default)
5. Pass the effective budget to prompt assembly

**Acceptance criteria**:
- First run uses tier defaults (no prediction history)
- After 10+ tasks with same role/complexity/domain, `predict()` returns learned value
- Predicted budget is within the tier envelope (never exceeds tier budget)
- `cargo test -p roko-cli` passes

---

### Task 6.9: Call record() After Gate Results

**File**: `crates/roko-cli/src/orchestrate.rs`

**What**: After gate results are known, call `predictor.record()` with
actual token usage and success/failure outcome.

**Steps**:
1. After gate verdict for a task, extract actual `input_tokens` from the agent response
2. Determine `success: bool` from gate verdict
3. Call `predictor.lock().unwrap().record(&features, actual_tokens, success)`
4. The predictor applies 1.3x failure inflation automatically on failure

**Acceptance criteria**:
- Run a plan with 5+ tasks
- Verify `budget-predictor.json` has entries for each feature combination
- Verify observation counts increase with each run
- Verify failed tasks have inflated EMA values
- `cargo test -p roko-cli` passes

---

### Task 6.10: Blend Static and Predicted Budgets During Warmup

**File**: `crates/roko-compose/src/budget_predictor.rs`

**What**: Add a blending mode where early predictions (< 50 observations)
blend with the static per-role budget, transitioning to full prediction.

**Steps**:
1. Add `pub fn predict_with_fallback(&self, features: &TaskFeatures, static_budget: u64) -> u64`
2. Determine observation count for the feature key
3. If count < 10: return `static_budget`
4. If count 10-50: return `(static_budget + self.predict(features)) / 2`
5. If count > 50: return `self.predict(features)` (full prediction)
6. Minimum floor of 1000 tokens always applies
7. Add unit tests covering all three bands

**Acceptance criteria**:
- 0 observations: returns static budget unchanged
- 15 observations: returns average of static and predicted
- 60 observations: returns predicted (ignores static)
- Unit tests cover all three bands
- `cargo test -p roko-compose` passes

---

## Phase 3: Section Effectiveness Feedback

**Problem**: `SectionInfluence` tracks per-section lift but weights are
not fed back into budget allocation. The system collects data about what
helps and ignores it.

**Effort**: 1-2 days | **Impact**: High (closes the learning loop)
**Dependencies**: Phase 2
**Issue refs**: ISS-04, ISS-11

---

### Task 6.11: Wire SectionInfluence Weights into PromptComposer

**File**: `crates/roko-compose/src/prompt.rs`

**What**: After scoring sections with the existing scorer, multiply each
section's score by the `SectionInfluence.weights()` multiplier.

**Steps**:
1. Add `section_influence_weights: Option<HashMap<String, f64>>` parameter to the composition path (either as a new method parameter or via builder)
2. After computing base scores, apply multipliers:
   ```rust
   if let Some(weights) = &influence_weights {
       if let Some(&w) = weights.get(&section.name) {
           section.effective_score *= w; // w in [0.5, 1.5]
       }
   }
   ```
3. Re-sort sections by adjusted score before knapsack allocation

**Search before implementing**:
```bash
grep -n 'effective_score\|PromptComposer\|compose(' crates/roko-compose/src/prompt.rs | head -20
```

**Acceptance criteria**:
- A section with influence weight 0.5 gets its score halved
- A section with influence weight 1.5 gets its score boosted 50%
- Sections without influence data keep their base score (implicit 1.0)
- `cargo test -p roko-compose` passes

---

### Task 6.12: Replace Binary Effectiveness Threshold with Graduated Scaling

**File**: `crates/roko-compose/src/prompt_assembly_service.rs`

**What**: Replace the binary `should_include()` (score < 0.1 = excluded,
line 185-190) with proportional per-section budget scaling.

**Steps**:
1. Remove or soften the hard 0.1 threshold in `should_include()` (keep threshold at 0.0 to still exclude truly zero-value sections)
2. Add `fn section_budget_multiplier(&self, section_name: &str) -> f64`:
   - Returns the effectiveness score for the section, clamped to [0.0, 1.5]
   - Default 1.0 when no effectiveness data exists
3. In assembly, each section's character cap becomes `base_cap * section_budget_multiplier(name)`
4. A section at 0.05 effectiveness gets 5% of its normal cap (nearly excluded)
5. A section at 0.3 gets 30% of its cap (included at reduced size)

**Acceptance criteria**:
- Section with effectiveness 0.05 gets ~5% of normal budget (not hard-excluded)
- Section with effectiveness 0.3 gets ~30%
- Section with effectiveness 1.0 gets full budget
- Section with no effectiveness data gets full budget (safe default)
- `cargo test -p roko-compose` passes

---

### Task 6.13: Record Section Inclusion in CognitiveWorkspace Audit

**File**: `crates/roko-compose/src/cognitive_workspace.rs`

**What**: Ensure `CognitiveWorkspace` audit records which influence weights
were applied to which sections during assembly.

**Steps**:
1. Add `influence_weights_applied: HashMap<String, f64>` to `CognitiveWorkspaceInput` (currently at line 21)
2. Include the weights in the `CognitiveWorkspace` via an additional builder method or field
3. Populate during assembly with the actual weights that were used
4. Sections without influence data show weight 1.0

**Acceptance criteria**:
- After a dispatch, `CognitiveWorkspace` contains the influence weight for each section
- Sections without influence data show weight 1.0
- `cargo test -p roko-compose` passes

---

## Phase 4: Conversation Compaction and Chat/ACP Convergence

**Problem**: `roko chat` and `dispatch_direct.rs` bypass the builder
partially or entirely. `compact_history()` is ready but not wired. Long
chat sessions grow without bound.

**Effort**: 2-3 days | **Impact**: High (most common interactive entry points)
**Dependencies**: Phase 1
**Issue refs**: ISS-03, ISS-05, ISS-09

---

### Task 6.14: Wire PromptAssemblyService into chat_session.rs System Prompt

**File**: `crates/roko-cli/src/chat_session.rs`

**What**: The `build_chat_system_prompt()` function (line 1350) already uses
`SystemPromptBuilder` directly. Upgrade it to use `PromptAssemblyService`
with model-aware tier selection.

**Steps**:
1. In `build_chat_system_prompt()`, resolve the model slug from the config
2. Create a `PromptAssemblyService` with:
   - Default conventions from workspace detection (already done at line 1355)
   - Model slug for tier selection
   - No episodes or playbooks (cold start for chat)
3. Call `assemble()` with role = None (defaults to Implementer) and task = None
4. If assembly fails, fall back to the existing `SystemPromptBuilder` path
5. The resulting prompt replaces the current `builder.build()` output

**Search before implementing**:
```bash
grep -n 'build_chat_system_prompt\|SystemPromptBuilder' crates/roko-cli/src/chat_session.rs | head -10
```

**Acceptance criteria**:
- `roko chat` starts with a system prompt containing role identity and conventions
- The system prompt is tier-appropriate for the configured model
- Existing chat behavior is not broken
- `cargo test -p roko-cli` passes

---

### Task 6.15: Wire Conversation Compaction into Chat Loop

**File**: `crates/roko-cli/src/chat_session.rs`

**What**: After each assistant response, check if conversation history
should be compacted using `compact_history()`.

**Steps**:
1. Import `roko_compose::compaction::{compact_history, CompactionPolicy, ChatMessage}`
2. Define a default policy:
   ```rust
   CompactionPolicy {
       trigger_threshold: 0.70,
       anchor_roles: vec!["system".into()],
       preserve_last_n_turns: 8,
       summary_budget_tokens: 128,
   }
   ```
3. Convert between `ChatAgentSession::api_history` format and `ChatMessage` format
4. After each assistant response, check `should_compact(&messages, &policy)`:
   - Estimate if compactable region > 70% of total context
5. If true, call `compact_history()`:
   - Use a dedicated Haiku call or the current chat agent as summarizer
   - Replace compacted messages with the summary message
6. Continue the session with compacted history

**Acceptance criteria**:
- Chat session with 30+ turns triggers compaction
- After compaction, system messages and recent 8 turns are preserved verbatim
- Gate results and tool outcomes from compacted region are carried forward
- Chat continues working normally after compaction
- `cargo test -p roko-cli` passes

---

### Task 6.16: Replace ACP Inline Prompts with Template Calls

**File**: `crates/roko-acp/src/runner.rs`

**What**: Replace the `format!()` role descriptions in
`run_multi_role_review()` (lines 1527-1544) with `ReviewerTemplate` calls.

**Steps**:
1. Import `roko_compose::templates::reviewer::{ReviewerTemplate, ReviewerInput, Reviewer}`
2. Replace the hardcoded "Architect Reviewer" format string (line 1527-1533):
   ```rust
   let template = ReviewerTemplate::new(Reviewer::Architect);
   let sections = template.render(&ReviewerInput { ... });
   let architect_prompt = sections_to_prompt_string(&sections);
   ```
3. Replace the hardcoded "Security & Correctness Auditor" format string (line 1537-1543) similarly, using `Reviewer::Auditor`
4. Remove the now-dead inline strings
5. Ensure the review JSON schema instruction (`REVIEW_JSON_SCHEMA`) is still appended

**Acceptance criteria**:
- `run_multi_role_review()` no longer contains `format!()` role descriptions
- ACP reviews produce the same structured output format as before
- `cargo test -p roko-acp` passes
- `grep -n 'You are the.*Reviewer\|You are the.*Auditor' crates/roko-acp/src/runner.rs` returns 0 results

---

### Task 6.17: Replace Orchestrate.rs Inline Prompts with Template Calls

**File**: `crates/roko-cli/src/orchestrate.rs`

**What**: Replace inline `format!()` prompt strings for fallback task prompts,
retry hints, escalation, and replan.

**Steps**:
1. Identify all inline prompt sites:
   ```bash
   grep -n 'format!.*Plan:.*Task:.*Implement\|format!.*Retry\|format!.*escalat' crates/roko-cli/src/orchestrate.rs
   ```
   Known sites: lines ~9399, ~9846, ~10437, ~14131, ~14558
2. Create helper functions or template structs for:
   - `fn fallback_task_prompt(plan_id: &str, task_id: &str) -> String`
   - `fn gate_failure_retry_hint(gate_error: &str) -> String`
   - `fn model_escalation_prompt(prior_model: &str, target_model: &str) -> String`
   - `fn replan_prompt(task_context: &str, failure_reason: &str) -> String`
3. Replace each `format!()` site with the appropriate helper call
4. Place helpers in a dedicated module or in the existing prompting module

**Acceptance criteria**:
- `grep -c 'format!.*Plan:.*Task:.*Implement' crates/roko-cli/src/orchestrate.rs` returns 0
- Gate failure retry still works end-to-end
- Model escalation still works end-to-end
- `cargo test -p roko-cli` passes

---

## Phase 5: VCG Auction Activation and Foraging

**Problem**: VCG warmup threshold of 10 observations per bidder is rarely
reached. DensityGreedy dominates. MultiPatchForager is built but context
retrieval uses direct queries.

**Effort**: 3-4 days | **Impact**: Medium-High
**Dependencies**: Phase 3 (influence feedback feeds auction observations)
**Issue refs**: ISS-08, ISS-10

---

### Task 6.18: Lower VCG Warmup and Wire Observation Recording

**File**: `crates/roko-compose/src/strategy.rs`

**What**: Lower `DEFAULT_VCG_WARMUP_OBSERVATIONS` from 10 to 5 (line 10)
and ensure observations are recorded per bidder during dispatch.

**Steps**:
1. Change `DEFAULT_VCG_WARMUP_OBSERVATIONS` from 10 to 5 (line 10)
2. In `orchestrate.rs`, after each dispatch, increment the bidder observation
   count for each `AttentionBidder` that contributed sections to the prompt
3. Persist bidder observations alongside existing learning state
4. Update the test at line 79 that asserts on the warmup threshold

**Acceptance criteria**:
- After 5 tasks (not 10), VCG allocation activates when strategy is `Auto`
- `CompositionStrategy::auto_select()` returns `Vcg` with 5+ observations per bidder
- Observation counts persist across runs
- `cargo test -p roko-compose` passes

---

### Task 6.19: Wire VCG Allocation as Actual Allocator

**Files**: `crates/roko-compose/src/prompt.rs`, `crates/roko-compose/src/auction.rs`

**What**: When strategy resolves to `Vcg`, use VCG welfare-maximizing
allocation to determine section inclusion, not just post-hoc diagnostics.

**Steps**:
1. In `PromptComposer::compose()` (or equivalent), when resolved strategy is `Vcg`:
   - Call `vcg_allocate()` with current sections, budget, and bidder values
   - Use VCG allocation result to determine which sections are included
   - Store payments in `CompositionManifest` for observability
2. When strategy is `DensityGreedy` (cold start): keep existing greedy behavior
3. Add config guard: `composition.vcg_enabled = true` (default true) to allow disabling

**Search before implementing**:
```bash
grep -rn 'vcg_allocate\|CompositionManifest' crates/roko-compose/src/ --include='*.rs'
```

**Acceptance criteria**:
- With 5+ warm bidders and `vcg_enabled = true`, VCG determines section inclusion
- VCG allocation respects the tier token budget as a hard ceiling
- Payments are recorded in `CompositionManifest`
- DensityGreedy still works when VCG is disabled or bidders are cold
- `cargo test -p roko-compose` passes

---

### Task 6.20: Wire MultiPatchForager into Context Retrieval

**File**: `crates/roko-cli/src/orchestrate.rs`

**What**: Replace direct knowledge/playbook/anti-pattern queries with
forager-driven retrieval that optimizes visitation order and stopping.

**Steps**:
1. Import `roko_compose::foraging::{MultiPatchForager, SourceForagingProfile}`
2. Build `SourceForagingProfile` entries for each context source:
   - Knowledge store: `g_max=0.8, lambda=0.3, travel_cost=0.05`
   - Playbook store: `g_max=0.6, lambda=0.5, travel_cost=0.03`
   - Code index: `g_max=0.7, lambda=0.4, travel_cost=0.1`
   - Episode history: `g_max=0.4, lambda=0.6, travel_cost=0.02`
3. Call `forager.optimal_order()` to determine which sources to visit first
4. For each source, call `forager.optimal_iterations()` for iteration count
5. After each batch, check `should_stop_searching()` with `estimate_context_sufficiency()`
6. Stop early when sufficiency >= 0.85 or MVT ratio drops below threshold

**Acceptance criteria**:
- Context retrieval visits sources in priority order (not unconditionally)
- Retrieval stops early when sufficient context is gathered
- Simple tasks (Surgical tier) do fewer retrievals than complex tasks (Full tier)
- Log output shows foraging decisions: "visited knowledge_store (3 iterations), stopped: sufficiency=0.87"
- `cargo test -p roko-cli` passes

---

### Task 6.21: Persist and Learn Foraging Profile Parameters

**File**: `crates/roko-compose/src/foraging.rs`

**What**: After each dispatch, record actual retrieval outcomes to update
foraging profile parameters (g_max, lambda, travel_cost) via EMA.

**Steps**:
1. Add `pub fn record_outcome(&mut self, source: &ContextSource, iterations: usize, items_found: usize, relevance_sum: f64)` to `MultiPatchForager`
2. Update `g_max` and `lambda` via EMA from observed data
3. Add `pub fn save(&self, path: &Path) -> std::io::Result<()>` and `pub fn load(path: &Path) -> std::io::Result<Option<Self>>` persistence methods
4. Persist profiles to `.roko/learn/foraging-profiles.json`
5. Load profiles at startup, falling back to hardcoded defaults

**Acceptance criteria**:
- After 10+ runs, foraging profiles in `.roko/learn/foraging-profiles.json` reflect actual retrieval patterns
- Profile values drift toward observed data (not stuck at initial defaults)
- `cargo test -p roko-compose` passes

---

## Phase 6: Per-Model Attention Fitting and Progressive Refinement

**Problem**: `ModelAttentionCurves` supports per-model parameters but only
the default curve is populated. Hardcoded knowledge thresholds and episode
counts ignore tier.

**Effort**: 2-3 days | **Impact**: Medium
**Dependencies**: Phase 1 (tier selection), Phase 3 (effectiveness feedback)
**Issue refs**: ISS-07, ISS-13, ISS-14

---

### Task 6.22: Populate Initial Attention Curves for Major Models

**File**: `crates/roko-compose/src/attention.rs`

**What**: Add hardcoded initial curves for major model families. The
`ModelAttentionCurves` struct is at line 58; `PositionAttentionModel`
default is at line 28.

**Steps**:
1. Add `pub fn default_model_curves() -> ModelAttentionCurves`
2. Populate curves for:
   - `claude-opus-4` / `claude-3-opus`: `primacy=0.30, recency=0.35, baseline=0.35` (less middle degradation)
   - `claude-sonnet-4` / `claude-3.5-sonnet`: default curve (0.35, 0.30, 0.35)
   - `claude-haiku` / `claude-3-haiku`: `primacy=0.40, recency=0.25, baseline=0.35` (stronger primacy bias)
   - `gpt-4` / `gpt-4o`: `primacy=0.35, recency=0.30, baseline=0.35`
   - `gpt-4o-mini`: `primacy=0.38, recency=0.27, baseline=0.35`
3. Wire `default_model_curves()` as initialization path when no persisted curves exist

**Acceptance criteria**:
- `ModelAttentionCurves::default_model_curves().for_model("claude-opus-4")` returns Opus curve (not default)
- `ModelAttentionCurves::default_model_curves().for_model("unknown-model")` returns default curve
- `cargo test -p roko-compose` passes

---

### Task 6.23: Wire Per-Model Curves into dynamic_placement()

**Files**: `crates/roko-compose/src/attention.rs`, `crates/roko-compose/src/role_prompts.rs`

**What**: When `dynamic_placement()` is called (line 98), look up the
model-specific curve instead of always using the default.

**Steps**:
1. Add `model_slug: Option<&str>` parameter to `dynamic_placement()` signature
2. Load `ModelAttentionCurves` from `.roko/learn/attention-curves.json` (or use `default_model_curves()`)
3. Use `curves.for_model(slug)` to get the appropriate curve
4. Apply the model-specific curve when computing the information density threshold for placement decisions
5. Update callers in `role_prompts.rs` to pass the model slug

**Acceptance criteria**:
- Dispatching to Haiku uses the Haiku curve (stronger primacy -> critical sections at start)
- Dispatching to Opus uses the Opus curve (less aggressive placement optimization)
- `cargo test -p roko-compose` passes

---

### Task 6.24: Add Attention Curve Learning from Gate Outcomes

**File**: `crates/roko-compose/src/attention.rs`

**What**: After each dispatch, if the task's critical information was at a
known position and the gate outcome is known, update model curve parameters.

**Steps**:
1. Add `pub fn record_placement_outcome(&mut self, model: &str, position: f64, success: bool)` to `ModelAttentionCurves`
2. Track per-model, per-position-bin (5 bins: 0.0-0.2, 0.2-0.4, ...) success rates
3. After 20+ observations per bin, refit curve parameters to match observed patterns
4. Persist updated curves to `.roko/learn/attention-curves.json`

**Acceptance criteria**:
- After 20+ tasks with position-tracked critical info, per-model curves are updated
- Updated curves reflect observed position-success patterns
- Curves persist across runs
- `cargo test -p roko-compose` passes

---

### Task 6.25: Tier-Adaptive Knowledge Confidence Thresholds

**File**: `crates/roko-compose/src/prompt_assembly_service.rs`

**What**: Make knowledge store confidence thresholds dependent on
`ContextTier` instead of hardcoded 0.5/0.3/0.2.

**Steps**:
1. Add `fn confidence_thresholds(tier: Option<ContextTier>) -> (f64, f64, f64)` returning (domain_facts, techniques, anti_patterns):
   - Surgical: `(0.8, 0.7, 0.5)` -- only proven knowledge
   - Focused: `(0.5, 0.3, 0.2)` -- current defaults
   - Full: `(0.3, 0.2, 0.1)` -- include speculative knowledge
   - None: `(0.5, 0.3, 0.2)` -- Focused as safe default
2. Replace the hardcoded `>= 0.5` in `relevant_knowledge_for_spec()` (line 547) with `domain_threshold`
3. Replace `>= 0.3` in `query_techniques()` (line 228) with `technique_threshold`
4. Replace `>= 0.2` in `query_anti_patterns()` (line 241) with `anti_pattern_threshold`

**Acceptance criteria**:
- Surgical tier assembly includes fewer knowledge entries (higher threshold)
- Full tier assembly includes more knowledge entries (lower threshold)
- `cargo test -p roko-compose` passes

---

### Task 6.26: Tier-Adaptive Episode Count

**File**: `crates/roko-compose/src/prompt_assembly_service.rs`

**What**: Replace the fixed "last 5 episodes" (line 344, `.take(5)`) with
tier-dependent counts.

**Steps**:
1. Add `fn episode_limit(tier: Option<ContextTier>) -> usize`:
   - Surgical: 0 (no episodes)
   - Focused: 3
   - Full: 5
   - None: 5 (current default)
2. Replace `.take(5)` at line 344 with `.take(episode_limit(self.context_tier))`
3. For Focused tier, filter to episodes with the same role as the current task (if role is known)

**Acceptance criteria**:
- Surgical tier assembly includes zero episode history
- Focused tier includes at most 3 episodes
- Full tier includes at most 5 episodes
- `cargo test -p roko-compose` passes

---

## Phase 7: Prompt Versioning, A/B Testing, and Multi-Agent Coordination

**Problem**: No systematic way to test prompt variants. Multi-agent plans
lack shared vocabulary. Cross-agent context injection is partial.

**Effort**: 3-4 days | **Impact**: Medium
**Dependencies**: Phase 3, Phase 5
**Issue refs**: ISS-12, ISS-15, ISS-16

---

### Task 6.27: Add Prompt Version Tagging

**File**: `crates/roko-compose/src/system_prompt_builder.rs`

**What**: Tag each assembled prompt with a version identifier so learning
data can be attributed to specific prompt versions.

**Steps**:
1. Add `prompt_version: Option<String>` field to `SystemPromptBuilder`
2. Derive the version from a hash of: template version, role identity text, section names, ordering strategy
3. Use a simple hash (e.g., first 8 hex chars of SHA-256 of the concatenated inputs)
4. Include the version in the output via a `<!-- prompt_version:abc12345 -->` comment
5. Expose the version via `pub fn prompt_version(&self) -> Option<&str>`

**Acceptance criteria**:
- Every assembled prompt has a non-empty prompt_version string
- Changing the role identity text changes the version
- Changing the section set changes the version
- `cargo test -p roko-compose` passes

---

### Task 6.28: Wire Prompt A/B Testing via ExperimentStore

**Files**: `crates/roko-compose/src/prompt_assembly_service.rs`, `crates/roko-learn/src/experiments.rs`

**What**: Support A/B testing prompt variants through the existing
`ExperimentStore`.

**Steps**:
1. Add `experiment_store: Option<Arc<ExperimentStore>>` to `PromptAssemblyService`
2. Add builder method `with_experiment_store(store: Arc<ExperimentStore>) -> Self`
3. Define prompt experiment types: `reasoning_depth`, `anti_pattern_format`, `section_ordering`
4. In assembly, if an active experiment covers a prompt dimension, use the experiment's selected variant
5. After gate results (in orchestrate.rs), record the outcome against the variant
6. Periodically (every 50 observations), promote the winning variant

**Search before implementing**:
```bash
grep -rn 'ExperimentStore\|pub struct Experiment' crates/roko-learn/src/experiments.rs | head -10
```

**Acceptance criteria**:
- Can define an experiment: `reasoning_depth` with variants `["suppress", "brief", "deep"]`
- Assembly uses the experiment's assigned variant for the current task
- Gate outcomes are recorded per variant
- After 50 observations, the experiment has a `current_winner`
- `cargo test -p roko-compose` and `cargo test -p roko-learn` pass

---

### Task 6.29: Add ReasoningDepth Tier-Based Default

**File**: `crates/roko-compose/src/system_prompt_builder.rs`

**What**: Add a `ReasoningDepth` enum and include tier-appropriate
reasoning instructions in the role identity layer.

**Steps**:
1. Add enum:
   ```rust
   pub enum ReasoningDepth {
       Suppress,  // "Do not explain. Just implement."
       Brief,     // "Briefly explain your approach, then implement."
       Deep,      // "Think step by step. Analyze, explain, implement."
   }
   ```
2. Add `pub fn with_reasoning_depth(mut self, depth: ReasoningDepth) -> Self` builder method
3. Inject reasoning instructions into Layer 1 (role identity) based on depth
4. Default: derive from `ContextTier` (Surgical -> Suppress, Focused -> Brief, Full -> Deep)
5. Allow experiment override from Task 6.28

**Acceptance criteria**:
- Surgical tier prompts contain "Do not explain" or equivalent
- Full tier prompts contain "Think step by step" or equivalent
- Experiment override changes the reasoning depth regardless of tier
- `cargo test -p roko-compose` passes

---

### Task 6.30: Wire Shared Vocabulary Injection for Multi-Agent Plans

**File**: `crates/roko-compose/src/prompt_assembly_service.rs` (with support from `context_mesh.rs`)

**What**: When multiple agents work on tasks in the same plan, inject shared
vocabulary definitions. The `ContextMesh` struct already exists in
`context_mesh.rs`.

**Steps**:
1. Add `shared_vocabulary: Option<Vec<(String, String)>>` to `PromptAssemblyService`
2. Builder method: `with_shared_vocabulary(vocab: Vec<(String, String)>) -> Self`
3. In assembly, if vocabulary is present, inject as a section:
   ```
   ## Shared Vocabulary (plan coordination)
   - "tier" = ContextTier (Surgical/Focused/Full)
   - "budget" = token budget, not character budget
   ```
4. In orchestrate.rs, extract vocabulary from plan metadata and pass to assembly

**Acceptance criteria**:
- Plan with `shared_vocabulary` in metadata injects vocabulary into agent prompts
- All agents in the plan see the same vocabulary definitions
- `cargo test -p roko-compose` passes

---

### Task 6.31: Wire Dependency Chain Context into Prompt Assembly

**Files**: `crates/roko-compose/src/context_provider.rs`, `crates/roko-cli/src/orchestrate.rs`

**What**: When a task depends on completed prior tasks, inject a structured
summary of what those tasks produced and their gate outcomes.

**Steps**:
1. Create struct (in context_provider.rs or a new helper):
   ```rust
   pub struct DependencyContext {
       pub task_id: String,
       pub summary: String,
       pub gate_outcome: String,  // "PASSED", "FAILED (clippy)", etc.
       pub files_modified: Vec<String>,
   }
   ```
2. In orchestrate.rs, after a task completes, store its `DependencyContext`
3. Before dispatching a dependent task, collect all predecessor `DependencyContext` entries
4. Format as a "Completed Dependencies" section and inject into Layer 3
5. Only for Focused and Full tiers (Surgical skips this -- check `tier.is_eligible("context")`)

**Acceptance criteria**:
- A task with dependencies receives a "Completed Dependencies" section listing predecessors
- The section includes gate outcomes
- Surgical tier tasks do not receive dependency context
- `cargo test -p roko-cli` passes

---

### Task 6.32: Content-Type-Aware Token Estimation

**File**: `crates/roko-compose/src/token_counter.rs`

**What**: The `TokenCounter` (line 9) already supports `Tiktoken` and
`HuggingFace` variants. Add a `ContentAware` variant that detects content
type for better budget accuracy than the flat 4.0 heuristic.

**Steps**:
1. Add helper `fn content_aware_chars_per_token(content: &str) -> f64`:
   - Detect code indicators: `fn `, `struct `, `impl `, `pub `, `let `, `use `, `mod `
   - If code-heavy (> 5% of words are code keywords): return 3.0
   - If markdown-heavy (contains `##` or many `- ` lines): return 5.0
   - Otherwise: return 4.0 (prose default)
2. Add `ContentAware` variant to `TokenCounter` enum
3. Implement `count()` for `ContentAware`: calls `content_aware_chars_per_token()` then divides
4. Wire `ContentAware` as the default counter in `PromptAssemblyService` (line 473, currently `Heuristic { chars_per_token: 4.0 }`)
5. Keep `Heuristic` as fallback for callers that do not need accuracy

**Acceptance criteria**:
- Code-heavy content (Rust source) estimates ~3 chars/token
- Markdown documentation estimates ~5 chars/token
- Prose text estimates ~4 chars/token
- Budget enforcement is tighter (fewer over-budget assemblies)
- `cargo test -p roko-compose` passes

---

### Task 6.33: Role Identity from TOML Config

**File**: `crates/roko-compose/src/role_prompts.rs`

**What**: Load role identity text from `.roko/roles/<role>.toml` files at
startup, falling back to compiled-in defaults.

**Steps**:
1. Add `pub fn load_role_identity(role: &str, roko_dir: &Path) -> String`:
   - Try to read `.roko/roles/<role>.toml`
   - Parse `[role].identity` field
   - Fall back to `role_identity_for()` static strings
2. Add `[role.tier_adjustments]` support:
   - Surgical: use `tier_adjustments.surgical` text (terse)
   - Focused: use `tier_adjustments.focused` text (moderate)
   - Full: use base `identity` text (comprehensive)
3. Cache loaded roles for the duration of a plan run (use a HashMap)

**Acceptance criteria**:
- Without `.roko/roles/` directory, existing static role identities are used (no regression)
- With `.roko/roles/implementer.toml`, the custom identity is used for Implementer role
- Tier adjustments work: Surgical gets terse identity, Full gets comprehensive
- `cargo test -p roko-compose` passes

---

## Phase Summary

| Phase | Tasks | Issues Addressed | Key Files | Effort | Impact |
|---|---|---|---|---|---|
| 1 (Tier Wiring) | 6.1-6.6 | ISS-01, ISS-06 | prompt_assembly_service.rs, budget.rs, context_provider.rs, orchestrate.rs, run.rs | 2-3d | Critical |
| 2 (Budget Prediction) | 6.7-6.10 | ISS-02 | orchestrate.rs, budget_predictor.rs | 1-2d | Critical |
| 3 (Effectiveness Loop) | 6.11-6.13 | ISS-04, ISS-11 | prompt.rs, prompt_assembly_service.rs, cognitive_workspace.rs | 1-2d | High |
| 4 (Chat + ACP) | 6.14-6.17 | ISS-03, ISS-05, ISS-09, ISS-17 | chat_session.rs, compaction.rs, runner.rs, orchestrate.rs | 2-3d | High |
| 5 (VCG + Foraging) | 6.18-6.21 | ISS-08, ISS-10 | strategy.rs, prompt.rs, auction.rs, foraging.rs, orchestrate.rs | 3-4d | Medium-High |
| 6 (Attention + Refinement) | 6.22-6.26 | ISS-07, ISS-13, ISS-14 | attention.rs, prompt_assembly_service.rs | 2-3d | Medium |
| 7 (Versioning + Multi-Agent) | 6.27-6.33 | ISS-12, ISS-15, ISS-16 | system_prompt_builder.rs, experiments.rs, token_counter.rs, role_prompts.rs, context_provider.rs | 3-4d | Medium |

**Total**: 33 tasks, ~15-22 days estimated effort

**Recommended execution order**: Phase 1 -> Phase 2 -> Phase 3 -> Phase 4 -> Phase 5 -> Phase 6 -> Phase 7

Phase 1 fixes the user's core pain point (small model overload). Phase 2
closes the budget learning loop. Phase 3 makes learning actionable. Phase 4
brings all entry points to parity. Phases 5-7 are optimization and polish.

---

## Success Criteria

### Phase 1+2 Complete (Critical Milestone):
- `is_local_model("ollama/llama3.2")` returns true
- Dispatching to Ollama produces system prompt <= 4K tokens
- Dispatching to Sonnet produces system prompt <= 12K tokens
- Dispatching to Opus produces system prompt <= 24K tokens
- `BudgetPredictor.predict()` is called before assembly in `dispatch_agent_with()`
- `BudgetPredictor.record()` is called after gate results are known
- After 10+ tasks, predicted budgets are used (blended with static defaults)

### Phase 3+4 Complete (High Impact Milestone):
- `SectionInfluence.weights()` is applied as a multiplier during composition
- Sections with negative lift are visibly demoted in budget after 20+ tasks
- `roko chat` produces system prompt with role identity and conventions
- ACP `run_multi_role_review()` uses `ReviewerTemplate` (zero inline format strings)
- Long chat sessions compact after 30+ turns

### Phase 5-7 Complete (Full Implementation):
- VCG allocation activates after 5+ warm observations per bidder
- MultiPatchForager is used for context retrieval with early stopping
- Per-model attention curves are populated for 5+ model families
- Prompt A/B testing is functional via ExperimentStore
- All prompts have version tags for learning attribution
- Knowledge confidence thresholds adapt to tier
- Episode history count adapts to tier
- Content-type-aware token estimation is the default counter

---

## Measurement Criteria

### Prompt Quality
- Gate pass rate by tier: Surgical >= 70%, Focused >= 80%, Full >= 85%
- Token efficiency: system prompt size / model context window <= 15% (Focused/Full), <= 30% (Surgical)
- Learning convergence: after 50 tasks, BudgetPredictor estimates within 30% of actual for 80%+ of types

### Entry Point Coverage
- 100% of dispatches go through PromptAssemblyService or SystemPromptBuilder
- 0 dispatches with empty system prompts
- 0 inline `format!()` role descriptions

### Performance
- Assembly latency: < 50ms for Surgical, < 200ms for Full (excluding knowledge store queries)
- Memory: per-dispatch allocation < 1MB

---

## File Inventory

| File | Path | LOC | Role | Tasks |
|---|---|---|---|---|
| prompt_assembly_service.rs | `crates/roko-compose/src/prompt_assembly_service.rs` | 1049 | PromptAssembler impl | 6.1, 6.4, 6.12, 6.14, 6.25, 6.26, 6.28, 6.30 |
| context_provider.rs | `crates/roko-compose/src/context_provider.rs` | ~2000 | ContextTier, is_local_model | 6.3, 6.31 |
| budget.rs | `crates/roko-compose/src/budget.rs` | 270 | Complexity-adjusted budgets | 6.2 |
| budget_predictor.rs | `crates/roko-compose/src/budget_predictor.rs` | 679 | EMA prediction + section influence | 6.10 |
| attention.rs | `crates/roko-compose/src/attention.rs` | 190 | U-curve, dynamic_placement | 6.22, 6.23, 6.24 |
| foraging.rs | `crates/roko-compose/src/foraging.rs` | 438 | MultiPatchForager | 6.20, 6.21 |
| compaction.rs | `crates/roko-compose/src/compaction.rs` | 488 | compact_history | 6.15 |
| strategy.rs | `crates/roko-compose/src/strategy.rs` | 97 | VCG warmup | 6.18 |
| prompt.rs | `crates/roko-compose/src/prompt.rs` | ~1200 | PromptComposer, scoring | 6.11, 6.19 |
| auction.rs | `crates/roko-compose/src/auction.rs` | 688 | vcg_allocate | 6.19 |
| cognitive_workspace.rs | `crates/roko-compose/src/cognitive_workspace.rs` | ~200 | Audit trail | 6.13 |
| system_prompt_builder.rs | `crates/roko-compose/src/system_prompt_builder.rs` | 2081 | 9-layer builder | 6.27, 6.29 |
| token_counter.rs | `crates/roko-compose/src/token_counter.rs` | 190 | Token estimation | 6.32 |
| role_prompts.rs | `crates/roko-compose/src/role_prompts.rs` | ~1500 | RoleSystemPromptSpec | 6.23, 6.33 |
| templates/common.rs | `crates/roko-compose/src/templates/common.rs` | 347 | Per-role budgets | -- (read-only ref) |
| templates/reviewer.rs | `crates/roko-compose/src/templates/reviewer.rs` | ~300 | ReviewerTemplate | 6.16 |
| context_mesh.rs | `crates/roko-compose/src/context_mesh.rs` | ~200 | SharedContextEntry | 6.30 |
| orchestrate.rs | `crates/roko-cli/src/orchestrate.rs` | ~15000 | dispatch_agent_with | 6.5, 6.7, 6.8, 6.9, 6.17, 6.20, 6.31 |
| run.rs | `crates/roko-cli/src/run.rs` | ~1500 | roko run path | 6.6 |
| chat_session.rs | `crates/roko-cli/src/chat_session.rs` | ~2400 | Chat REPL | 6.14, 6.15 |
| runner.rs | `crates/roko-acp/src/runner.rs` | ~1600 | ACP multi-role review | 6.16 |

---

## Sources

- `crates/roko-compose/src/system_prompt_builder.rs` -- 9-layer builder, build methods
- `crates/roko-compose/src/prompt_assembly_service.rs` -- PromptAssemblyService, assembly pipeline
- `crates/roko-compose/src/context_provider.rs` -- ContextTier, is_local_model, budgets
- `crates/roko-compose/src/budget.rs` -- adjusted_budget_for, Complexity
- `crates/roko-compose/src/budget_predictor.rs` -- BudgetPredictor, SectionInfluence
- `crates/roko-compose/src/attention.rs` -- PositionAttentionModel, ModelAttentionCurves
- `crates/roko-compose/src/foraging.rs` -- MultiPatchForager, should_stop_searching
- `crates/roko-compose/src/compaction.rs` -- compact_history, CompactionPolicy
- `crates/roko-compose/src/prompt.rs` -- PromptComposer, section scoring
- `crates/roko-compose/src/auction.rs` -- vcg_allocate, LearningBidder
- `crates/roko-compose/src/strategy.rs` -- CompositionStrategy, VCG warmup
- `crates/roko-compose/src/cognitive_workspace.rs` -- CognitiveWorkspace audit trail
- `crates/roko-compose/src/token_counter.rs` -- TokenCounter heuristic
- `crates/roko-compose/src/role_prompts.rs` -- RoleSystemPromptSpec, role_identity_for
- `crates/roko-compose/src/templates/common.rs` -- PromptBudget, budget_for
- `crates/roko-compose/src/templates/reviewer.rs` -- ReviewerTemplate
- `crates/roko-compose/src/context_mesh.rs` -- SharedContextEntry, ContextMesh
- `crates/roko-cli/src/orchestrate.rs` -- dispatch_agent_with, model selection, inline prompts
- `crates/roko-cli/src/run.rs` -- roko run path
- `crates/roko-cli/src/chat_session.rs` -- chat REPL, build_chat_system_prompt
- `crates/roko-acp/src/runner.rs` -- run_multi_role_review, inline prompts
