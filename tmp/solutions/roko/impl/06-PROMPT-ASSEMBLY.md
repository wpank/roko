# Implementation Plan: Prompt Assembly Subsystem

> 38 tasks across 7 phases. Each task names exact files, describes what to change,
> and specifies mechanically verifiable acceptance criteria. Line numbers are
> approximate and should be confirmed at execution time.
>
> Core thesis: **wire existing infrastructure into live paths.** The 9-layer
> builder, ContextTier, BudgetPredictor, SectionInfluence, MultiPatchForager,
> CompactionPolicy, VCG auction, and ModelAttentionCurves are all built and
> tested. None of them are connected to dispatch. This plan connects them.

---

## PHASE 1: Model-Aware Context Windowing (ISS-01 fix)

**Problem**: Per-role budgets in `templates/common.rs` total ~109K characters
(~27K tokens) for an Implementer. An Ollama model with 4K-8K context gets the
same prompt as Opus with 200K context. The `ContextTier` system in
`context_provider.rs` defines the right 4K/12K/24K budgets but is not wired
into the main builder path.

**Effort**: 2-3 days | **Impact**: Critical -- the user's core pain point
**Dependencies**: None
**Issue refs**: ISS-01, ISS-06

### Task 1.1: Add `model_slug` and `context_tier` Fields to PromptAssemblyService

**File**: `crates/roko-compose/src/prompt_assembly_service.rs`
**What**: Add optional `model_slug` and derived `context_tier` fields. When set,
the tier's `default_token_budget()` overrides the static `token_budget` field.

**Steps**:
1. Add fields to `PromptAssemblyService`:
   ```rust
   model_slug: Option<String>,
   context_tier: Option<ContextTier>,
   ```
2. Add builder methods `with_model_slug(slug: String)` and `with_context_tier(tier: ContextTier)`
3. In `assemble()`, compute `effective_budget`:
   - If `context_tier` is set, use `tier.default_token_budget()`
   - Else if `model_slug` is set, derive tier via `ContextTier::from_task_and_model()`, use its budget
   - Else if `token_budget` is set, use that
   - Else no budget (unbounded)
4. Use `effective_budget` where `self.token_budget` was previously used

**Acceptance criteria**:
- `PromptAssemblyService::new().with_model_slug("ollama/llama3.2".into())` produces an assembly with budget <= 4000 tokens
- `PromptAssemblyService::new().with_model_slug("claude-sonnet-4-20250514".into())` produces budget <= 12000 tokens
- `PromptAssemblyService::new().with_model_slug("claude-opus-4-20250514".into())` produces budget <= 24000 tokens
- Existing callers that set `token_budget` continue to work unchanged

### Task 1.2: Add `tier_scaled_budget()` to budget.rs

**File**: `crates/roko-compose/src/budget.rs`
**What**: Add a function that proportionally scales a `PromptBudget` (character
caps) to fit within a `ContextTier`'s token budget.

**Steps**:
1. Import `ContextTier` from `context_provider`
2. Add `tier_scaled_budget(base: PromptBudget, tier: ContextTier) -> PromptBudget`
3. Compute `base_total` as the sum of all `PromptBudget` field values
4. Compute `tier_total = tier.default_token_budget() * 4` (tokens to chars heuristic)
5. If `tier_total >= base_total`, return `base` unchanged
6. Scale each field by `tier_total / base_total`, with a minimum of 0 per field
7. Add unit tests verifying Surgical scales down dramatically and Full leaves Implementer budgets mostly intact

**Acceptance criteria**:
- `tier_scaled_budget(budget_for(AgentRole::Implementer), ContextTier::Surgical)` produces a budget whose total is <= 16000 chars (~4K tokens)
- `tier_scaled_budget(budget_for(AgentRole::Implementer), ContextTier::Full)` produces a budget whose total is <= 96000 chars (~24K tokens)
- No field goes negative

### Task 1.3: Add Tier-Dependent Section Eligibility

**File**: `crates/roko-compose/src/context_provider.rs`
**What**: Add `eligible_sections()` method to `ContextTier` that returns which
section categories are allowed per tier.

**Steps**:
1. Add method to `ContextTier`:
   ```rust
   pub fn eligible_sections(&self) -> &'static [&'static str]
   ```
2. Surgical: `["identity", "task", "tools", "anti_patterns", "verification"]`
3. Focused: Surgical + `["conventions", "playbooks", "gate_feedback", "brief", "file_context"]`
4. Full: Focused + `["domain", "context", "workspace_map", "prd", "research", "episodes", "affect"]`
5. Add `is_eligible(&self, section_name: &str) -> bool` convenience method

**Acceptance criteria**:
- `ContextTier::Surgical.is_eligible("workspace_map")` returns false
- `ContextTier::Surgical.is_eligible("task")` returns true
- `ContextTier::Full.is_eligible("workspace_map")` returns true
- Unit tests for all tiers and section combinations

### Task 1.4: Wire Tier Eligibility into PromptAssemblyService

**File**: `crates/roko-compose/src/prompt_assembly_service.rs`
**What**: When `context_tier` is set, hard-exclude sections that the tier does
not support, regardless of effectiveness score.

**Steps**:
1. In the assembly logic (where sections are conditionally included), check `tier.is_eligible(section_name)` before inclusion
2. This check runs *before* the existing effectiveness threshold check
3. Excluded sections should be logged at `debug!` level with the reason "tier ineligibility"
4. Adjust `WORKSPACE_MAP_LINE_LIMIT` based on tier: Surgical=0, Focused=100, Full=300

**Acceptance criteria**:
- Assembly with `ContextTier::Surgical` produces a prompt containing only identity, task, tools, anti-patterns, and verification content
- Assembly with `ContextTier::Surgical` has zero workspace_map content
- Assembly with `ContextTier::Full` includes all sections that pass effectiveness threshold
- `cargo test -p roko-compose` passes

### Task 1.5: Thread model_slug Through dispatch_agent_with()

**File**: `crates/roko-cli/src/orchestrate.rs`
**What**: Pass the CascadeRouter-selected model slug into prompt assembly so
that `ContextTier` is consulted before building the system prompt.

**Steps**:
1. In `dispatch_agent_with()`, after model selection (CascadeRouter), extract `model_slug`
2. Compute `ContextTier::from_task_and_model(&task_tier_string, &model_slug)`
3. Pass `tier` into `build_system_prompt_with_context_validated()` or directly into `PromptAssemblyService` via `.with_model_slug(model_slug)`
4. Use `tier_scaled_budget()` to scale the per-role budget before passing to the builder
5. Log the selected tier at `info!` level

**Acceptance criteria**:
- Run `roko plan run` with a task configured for Ollama backend
- System prompt for that task is <= 4K tokens (verify via log output)
- Run `roko plan run` with Opus backend
- System prompt is <= 24K tokens
- Log line shows "context_tier=Surgical" or "context_tier=Full" as appropriate

### Task 1.6: Thread model_slug Through run.rs Path

**File**: `crates/roko-cli/src/run.rs`
**What**: The `roko run` path also uses `build_role_system_prompt_validated()`.
Thread model slug into this path too.

**Steps**:
1. In the `roko run` handler, resolve the model slug from config or auth detection
2. Pass it through to the prompt assembly call
3. The same ContextTier logic from Task 1.5 applies here

**Acceptance criteria**:
- `roko run "test prompt"` with a configured Ollama model produces a system prompt <= 4K tokens
- `roko run "test prompt"` with Opus produces a system prompt <= 24K tokens

---

## PHASE 2: Wire BudgetPredictor (ISS-02 fix)

**Problem**: `BudgetPredictor` is fully built (EMA-based, with failure
inflation, partial-match fallback, persistence) but nobody calls
`predictor.predict()` before assembly or `predictor.record()` after gate results.

**Effort**: 1-2 days | **Impact**: Critical -- enables budget convergence
**Dependencies**: Phase 1 (tier provides outer envelope; predictor refines within it)
**Issue refs**: ISS-02

### Task 2.1: Load BudgetPredictor at Startup

**File**: `crates/roko-cli/src/orchestrate.rs`
**What**: Load `BudgetPredictor` from `.roko/learn/budget-predictor.json` at
plan run startup and make it available to the dispatch loop.

**Steps**:
1. Import `roko_compose::budget_predictor::{BudgetPredictor, TaskFeatures}`
2. In the plan runner init, load predictor via `BudgetPredictor::load_or_default(paths.roko_dir.join("learn/budget-predictor.json"))`
3. Wrap in `Arc<Mutex<BudgetPredictor>>` and attach to the runner context
4. At run end (or periodically), persist with `predictor.save()`

**Acceptance criteria**:
- `roko plan run` loads predictor without error (even if file does not exist -- defaults used)
- After run completes, `.roko/learn/budget-predictor.json` exists
- Second run loads the file produced by the first run

### Task 2.2: Call predict() Before Assembly

**File**: `crates/roko-cli/src/orchestrate.rs`
**What**: Before building the system prompt, call `predictor.predict()` with
the task's features. If the predictor has history, use its estimate (clamped to
the tier budget) instead of the static per-role budget.

**Steps**:
1. Construct `TaskFeatures` from the current task's role, complexity, and domain
2. Call `predictor.predict(&features)`
3. If `Some(predicted_budget)`, use `min(predicted_budget, tier.default_token_budget())`
4. If `None` (no history), use `tier.default_token_budget()` (Phase 1 default)
5. Pass the effective budget to prompt assembly

**Acceptance criteria**:
- First run uses tier defaults (no prediction history)
- After 10+ tasks with the same role/complexity/domain, `predict()` returns `Some`
- Predicted budget is within the tier envelope (never exceeds tier budget)

### Task 2.3: Call record() After Gate Results

**File**: `crates/roko-cli/src/orchestrate.rs`
**What**: After gate results are known, call `predictor.record()` with the
actual token usage and success/failure outcome.

**Steps**:
1. After gate verdict for a task, construct `BudgetOutcome` with actual `input_tokens` used and `success: bool`
2. Call `predictor.record(&features, &outcome)`
3. If task failed, the predictor applies 1.3x failure inflation automatically

**Acceptance criteria**:
- Run a plan with 5+ tasks
- Verify `budget-predictor.json` has entries for each feature combination
- Verify observation counts increase with each run
- Verify failed tasks have inflated EMA values compared to successful tasks with similar features

### Task 2.4: Blend Static and Predicted Budgets During Warmup

**File**: `crates/roko-compose/src/budget_predictor.rs`
**What**: Add a blending mode where early predictions (< 50 observations) are
blended 50/50 with the static per-role budget, transitioning to full prediction
after 50 observations.

**Steps**:
1. Add `predict_with_fallback(&self, features: &TaskFeatures, static_budget: usize) -> usize`
2. If observation count < 10: return `static_budget`
3. If observation count 10-50: return `(static_budget + predicted) / 2`
4. If observation count > 50: return `predicted` (with 20% safety margin already applied by predict())
5. Minimum floor of 1000 tokens always applies

**Acceptance criteria**:
- 0 observations: returns static budget unchanged
- 15 observations: returns average of static and predicted
- 60 observations: returns predicted (ignores static)
- Unit tests cover all three bands

---

## PHASE 3: Section Effectiveness Feedback (ISS-04 fix)

**Problem**: `SectionInfluence` tracks per-section lift (which sections causally
improve task success) but the weights are not fed back into budget allocation.
The system collects data about what helps and ignores it.

**Effort**: 1-2 days | **Impact**: High -- closes the learning loop
**Dependencies**: Phase 2 (predictor wired)
**Issue refs**: ISS-04, ISS-11

### Task 3.1: Wire SectionInfluence Weights into PromptComposer

**File**: `crates/roko-compose/src/prompt.rs`
**What**: After scoring sections with the existing scorer, multiply each
section's score by the `SectionInfluence.weights()` multiplier.

**Steps**:
1. Add `section_influence_weights: Option<HashMap<String, f64>>` parameter to the composition path
2. After computing base scores, apply multipliers:
   ```rust
   if let Some(weights) = &influence_weights {
       if let Some(&w) = weights.get(&section.name) {
           section.effective_score *= w; // [0.5, 1.5]
       }
   }
   ```
3. Re-sort sections by adjusted score before knapsack allocation

**Acceptance criteria**:
- A section with influence weight 0.5 gets its score halved (demoted in allocation)
- A section with influence weight 1.5 gets its score boosted 50%
- Sections without influence data keep their base score (weight = 1.0 implicit)
- `cargo test -p roko-compose` passes

### Task 3.2: Replace Binary Effectiveness Threshold with Graduated Scaling

**File**: `crates/roko-compose/src/prompt_assembly_service.rs`
**What**: Replace the binary `should_include()` (score < 0.1 = excluded) with
proportional per-section budget scaling.

**Steps**:
1. Remove or soften the hard 0.1 threshold in `should_include()`
2. Add `section_budget_multiplier(section_name: &str) -> f64`:
   - Returns the effectiveness score for the section, clamped to [0.0, 1.5]
   - Default 1.0 when no effectiveness data exists
3. In assembly, each section's cap becomes `base_cap * section_budget_multiplier(name)`
4. A section at 0.05 effectiveness gets 5% of its normal cap (nearly excluded)
5. A section at 0.3 gets 30% of its cap (included at reduced size)

**Acceptance criteria**:
- Section with effectiveness 0.05 gets ~5% of normal budget (not hard-excluded)
- Section with effectiveness 0.3 gets ~30% of normal budget
- Section with effectiveness 1.0 gets full budget
- Section with no effectiveness data gets full budget (safe default)
- `cargo test -p roko-compose` passes

### Task 3.3: Record Section Inclusion in CognitiveWorkspace Audit

**File**: `crates/roko-compose/src/cognitive_workspace.rs`
**What**: Ensure the `CognitiveWorkspace` audit records which influence weights
were applied to which sections, so the learning loop is observable.

**Steps**:
1. Add `influence_weights_applied: HashMap<String, f64>` to `CognitiveWorkspace`
2. Populate during assembly with the actual weights that were used
3. Include in the audit trail alongside existing section-level data

**Acceptance criteria**:
- After a dispatch, `CognitiveWorkspace` contains the influence weight for each section
- Sections without influence data show weight 1.0
- `cargo test -p roko-compose` passes

---

## PHASE 4: Conversation Compaction and Chat Convergence (ISS-03, ISS-09)

**Problem**: `roko chat` and `dispatch_direct.rs` bypass the builder entirely.
`compact_history()` is ready but not wired into any live path. Long chat
sessions grow without bound.

**Effort**: 2-3 days | **Impact**: High -- most common interactive entry points
**Dependencies**: Phase 1 (tier-aware assembly)
**Issue refs**: ISS-03, ISS-05, ISS-09

### Task 4.1: Wire PromptAssemblyService into dispatch_direct.rs

**File**: `crates/roko-cli/src/dispatch_direct.rs`
**What**: Before spawning the Claude CLI subprocess, call
`PromptAssemblyService::assemble()` to get a system prompt. Pass it via
`--system-prompt` or as the system message.

**Steps**:
1. Import `PromptAssemblyService` and `ContextTier`
2. Resolve the model slug from config or the auth detection result
3. Build a `PromptAssemblyService` with:
   - Default conventions from workspace detection
   - Model slug for tier selection
   - No episodes or playbooks (cold start for direct dispatch)
4. Call `assemble()` with role=Implementer and task=user_prompt
5. Pass the resulting system prompt to the agent subprocess
6. If assembly fails, fall back to a minimal system prompt (role identity only)

**Acceptance criteria**:
- `roko "test prompt"` produces a system prompt with role identity and conventions
- The system prompt is tier-appropriate for the configured model
- Existing `roko "test prompt"` behavior is not broken (agent still runs)
- `cargo test -p roko-cli` passes

### Task 4.2: Wire PromptAssemblyService into roko chat

**File**: `crates/roko-cli/src/chat_session.rs` (or equivalent chat entry point)
**What**: Build a system prompt for the chat session using
`PromptAssemblyService`. Configure for the Focused tier by default.

**Steps**:
1. At chat session init, build a `PromptAssemblyService` with workspace conventions
2. Assemble a system prompt with role=Implementer (or user-selected role)
3. Set the system prompt on the first message to the agent
4. Re-assemble periodically (e.g., every 20 turns) to incorporate new knowledge

**Acceptance criteria**:
- `roko chat` starts with a system prompt containing role identity and conventions
- The system prompt is visible in debug logs
- `cargo test -p roko-cli` passes

### Task 4.3: Wire Conversation Compaction into Chat Loop

**File**: `crates/roko-cli/src/chat_session.rs`
**What**: After each user turn, check if conversation history should be
compacted using `compact_history()`.

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
3. After each assistant response, check `should_compact(&messages, &policy)`
4. If true, call `compact_history()`:
   - Use a summarizer agent (the current chat agent or a dedicated Haiku call)
   - Replace compacted messages with the summary message
5. Continue the session with compacted history

**Acceptance criteria**:
- Chat session with 30+ turns triggers compaction
- After compaction, system messages and recent 8 turns are preserved verbatim
- Gate results and tool outcomes from compacted region are carried forward in structured JSON
- Chat continues working normally after compaction

### Task 4.4: Replace ACP Inline Prompts with Template Calls

**File**: `crates/roko-acp/src/runner.rs`
**What**: Replace the `format!()` role descriptions in `run_multi_role_review()`
with `ReviewerTemplate` calls.

**Steps**:
1. Import `roko_compose::templates::reviewer::{ReviewerTemplate, ReviewerInput, Reviewer}`
2. Replace the hardcoded "Architect Reviewer" format string (~line 525-541) with:
   ```rust
   let architect_sections = ReviewerTemplate.render(&ReviewerInput {
       reviewer: Reviewer::ScopedArchitect,
       ...
   });
   ```
3. Replace the hardcoded "Security & Correctness Auditor" format string similarly
4. Replace the review variant prompts (quick/thorough/default at ~405-424) with template variants
5. Remove the now-dead inline strings

**Acceptance criteria**:
- `run_multi_role_review()` no longer contains `format!()` role descriptions
- ACP reviews produce the same structured output format as before
- `cargo test -p roko-acp` passes
- `grep -n 'You are the.*Reviewer\|You are the.*Auditor' crates/roko-acp/src/runner.rs` returns 0 results

### Task 4.5: Replace Orchestrate.rs Inline Prompts with Template Calls

**File**: `crates/roko-cli/src/orchestrate.rs`
**What**: Replace inline `format!()` prompt strings for retry/escalation/replan
with template-based construction.

**Steps**:
1. Create template variants or builder helpers for:
   - Gate failure retry hint (currently inline at ~line 11212)
   - Model escalation prompt (~line 11285)
   - Replan prompt (~line 11395)
   - Verification-failed fix prompt (~line 13080)
   - Fallback task prompts (~lines 8894, 9347, 9941, 14014)
2. Each template takes structured inputs (gate error, prior model, task context) and produces the prompt
3. Replace each `format!()` site with a template call
4. Verify no inline prompt strings remain in orchestrate.rs

**Acceptance criteria**:
- `grep -c 'format!.*Implement\|format!.*Plan:.*Task:' crates/roko-cli/src/orchestrate.rs` returns 0
- Gate failure retry still works end-to-end
- Model escalation still works end-to-end
- `cargo test -p roko-cli` passes

---

## PHASE 5: VCG Auction Activation and Foraging (ISS-08, ISS-10)

**Problem**: VCG warmup threshold of 10 observations per bidder is rarely
reached, so DensityGreedy dominates. MultiPatchForager is built but context
retrieval uses direct queries. The auction system is 688 LOC of sophisticated
mechanism design that is purely decorative.

**Effort**: 3-4 days | **Impact**: Medium-High -- enables smart context retrieval
**Dependencies**: Phase 3 (influence feedback feeds auction observations)
**Issue refs**: ISS-08, ISS-10

### Task 5.1: Lower VCG Warmup and Wire Observation Recording

**File**: `crates/roko-compose/src/strategy.rs`
**What**: Lower `DEFAULT_VCG_WARMUP_OBSERVATIONS` from 10 to 5 and ensure
observations are actually recorded per bidder during dispatch.

**Steps**:
1. Change `DEFAULT_VCG_WARMUP_OBSERVATIONS` from 10 to 5
2. In `orchestrate.rs`, after each dispatch, increment the bidder observation count for each `AttentionBidder` that contributed sections to the prompt
3. Persist bidder observations alongside the existing learning state

**Acceptance criteria**:
- After 5 tasks (not 10), VCG allocation activates when strategy is `Auto`
- `CompositionStrategy::auto_select()` returns `Vcg` with 5+ observations per bidder
- Observation counts persist across runs
- `cargo test -p roko-compose` passes

### Task 5.2: Wire VCG Allocation as Actual Allocator (Not Just Diagnostic)

**File**: `crates/roko-compose/src/prompt.rs` and `crates/roko-compose/src/auction.rs`
**What**: When strategy resolves to `Vcg`, use the VCG welfare-maximizing
allocation to determine section inclusion, not just as post-hoc diagnostics.

**Steps**:
1. In `PromptComposer::compose()`, when resolved strategy is `Vcg`:
   - Call `vcg_allocate()` with the current sections, budget, and bidder values
   - Use the VCG allocation result to determine which sections are included
   - Store payments in the `CompositionManifest` for observability
2. When strategy is `DensityGreedy` (cold start): keep existing greedy behavior
3. Add a config guard: `composition.vcg_enabled = true` (default true) to allow disabling

**Acceptance criteria**:
- With 5+ warm bidders and `vcg_enabled = true`, VCG allocation determines section inclusion
- VCG allocation respects the tier token budget as a hard ceiling
- Payments are recorded in `CompositionManifest`
- DensityGreedy still works when VCG is disabled or bidders are cold
- `cargo test -p roko-compose` passes

### Task 5.3: Wire MultiPatchForager into Context Retrieval

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
5. After each batch, check `should_stop_searching()` with sufficiency estimate
6. Stop early when sufficiency >= 0.85 or MVT ratio drops below threshold

**Acceptance criteria**:
- Context retrieval visits sources in priority order (not unconditionally)
- Retrieval stops early when sufficient context is gathered
- Simple tasks (Surgical tier) do fewer retrievals than complex tasks (Full tier)
- Log output shows foraging decisions: "visited knowledge_store (3 iterations), stopped: sufficiency=0.87"
- `cargo test -p roko-cli` passes

### Task 5.4: Persist and Learn Foraging Profile Parameters

**File**: `crates/roko-compose/src/foraging.rs`
**What**: After each dispatch, record actual retrieval outcomes to update
foraging profile parameters (g_max, lambda, travel_cost) via EMA.

**Steps**:
1. Add `record_outcome(source: &str, iterations: usize, items_found: usize, relevance_sum: f64)`
2. Update `g_max` and `lambda` via EMA from observed data
3. Persist profiles to `.roko/learn/foraging-profiles.json`
4. Load profiles at startup, falling back to hardcoded defaults

**Acceptance criteria**:
- After 10+ runs, foraging profiles in `.roko/learn/foraging-profiles.json` reflect actual retrieval patterns
- Profile values drift toward observed data (not stuck at initial defaults)
- `cargo test -p roko-compose` passes

---

## PHASE 6: Per-Model Attention Fitting and Progressive Refinement

**Problem**: `ModelAttentionCurves` supports per-model U-curve parameters but
only the default curve is populated. `dynamic_placement()` uses the same curve
for all models even though they have different position sensitivities.

**Effort**: 2-3 days | **Impact**: Medium -- improves section placement quality
**Dependencies**: Phase 1 (tier selection), Phase 3 (effectiveness feedback)
**Issue refs**: ISS-07

### Task 6.1: Populate Initial Attention Curves for Major Models

**File**: `crates/roko-compose/src/attention.rs`
**What**: Add hardcoded initial curves for Claude Opus, Sonnet, Haiku, GPT-4,
and GPT-4o-mini based on published research and empirical observations.

**Steps**:
1. Add `pub fn default_model_curves() -> ModelAttentionCurves` that returns curves for:
   - `claude-opus-4` / `claude-3-opus`: `primacy=0.30, recency=0.35, baseline=0.35` (less middle degradation)
   - `claude-sonnet-4` / `claude-3.5-sonnet`: default curve
   - `claude-haiku` / `claude-3-haiku`: `primacy=0.40, recency=0.25, baseline=0.35` (stronger primacy bias)
   - `gpt-4` / `gpt-4o`: `primacy=0.35, recency=0.30, baseline=0.35` (similar to default)
   - `gpt-4o-mini`: `primacy=0.38, recency=0.27, baseline=0.35` (slightly more primacy sensitive)
2. Wire `default_model_curves()` as the initialization path when no persisted curves exist

**Acceptance criteria**:
- `ModelAttentionCurves::default_model_curves().for_model("claude-opus-4")` returns the Opus-specific curve (not default)
- `ModelAttentionCurves::default_model_curves().for_model("unknown-model")` returns the default curve
- `cargo test -p roko-compose` passes

### Task 6.2: Wire Per-Model Curves into dynamic_placement()

**File**: `crates/roko-compose/src/attention.rs` and `crates/roko-compose/src/role_prompts.rs`
**What**: When `dynamic_placement()` is called, look up the model-specific
curve instead of always using the default.

**Steps**:
1. Add `model_slug: Option<&str>` parameter to `dynamic_placement()` or its caller
2. Load `ModelAttentionCurves` from `.roko/learn/attention-curves.json` (or use `default_model_curves()`)
3. Use `curves.for_model(slug)` to get the appropriate curve
4. Apply the model-specific curve when computing placement decisions

**Acceptance criteria**:
- Dispatching to Haiku uses the Haiku curve (stronger primacy -> more critical sections at start)
- Dispatching to Opus uses the Opus curve (less aggressive placement optimization)
- `cargo test -p roko-compose` passes

### Task 6.3: Add Attention Curve Learning from Gate Outcomes

**File**: `crates/roko-compose/src/attention.rs`
**What**: After each dispatch, if the task's critical information was at a known
position and the gate outcome is known, update the model's curve parameters.

**Steps**:
1. Add `record_placement_outcome(model: &str, position: f64, success: bool)` to `ModelAttentionCurves`
2. Track per-model, per-position-bin (5 bins: 0.0-0.2, 0.2-0.4, ...) success rates
3. After 20+ observations per bin, refit the curve parameters (primacy_weight, recency_weight) to match observed success rates
4. Persist updated curves to `.roko/learn/attention-curves.json`

**Acceptance criteria**:
- After 20+ tasks with position-tracked critical info, per-model curves are updated
- Updated curves reflect observed position-success patterns
- Curves persist across runs
- `cargo test -p roko-compose` passes

### Task 6.4: Tier-Adaptive Knowledge Confidence Thresholds

**File**: `crates/roko-compose/src/prompt_assembly_service.rs`
**What**: Make knowledge store confidence thresholds dependent on ContextTier
instead of using fixed 0.5/0.3/0.2 for all tiers.

**Steps**:
1. Add `fn confidence_thresholds(tier: ContextTier) -> (f64, f64, f64)` returning (domain_facts, techniques, anti_patterns):
   - Surgical: `(0.8, 0.7, 0.5)` -- only proven knowledge
   - Focused: `(0.5, 0.3, 0.2)` -- current defaults
   - Full: `(0.3, 0.2, 0.1)` -- include speculative knowledge
2. Replace the hardcoded threshold values with calls to this function
3. When no tier is set, use Focused thresholds as default

**Acceptance criteria**:
- Surgical tier assembly includes fewer knowledge entries (higher threshold)
- Full tier assembly includes more knowledge entries (lower threshold)
- `cargo test -p roko-compose` passes

### Task 6.5: Tier-Adaptive Episode Count

**File**: `crates/roko-compose/src/prompt_assembly_service.rs`
**What**: Replace the fixed "last 5 episodes" with tier-dependent counts.

**Steps**:
1. Add `fn episode_limit(tier: ContextTier) -> usize`:
   - Surgical: 0 (no episodes)
   - Focused: 3
   - Full: 5
2. Replace the hardcoded 5 in `format_episode_context()` with `episode_limit(tier)`
3. For Focused tier, filter to episodes with the same role as the current task

**Acceptance criteria**:
- Surgical tier assembly includes zero episode history
- Focused tier includes at most 3 episodes
- Full tier includes at most 5 episodes
- `cargo test -p roko-compose` passes

---

## PHASE 7: Prompt Versioning, A/B Testing, and Multi-Agent Coordination

**Problem**: No systematic way to test prompt variants. Multi-agent plans lack
shared vocabulary. Cross-agent context injection is partial.

**Effort**: 3-4 days | **Impact**: Medium -- enables systematic prompt improvement
**Dependencies**: Phase 3 (effectiveness feedback), Phase 5 (VCG)

### Task 7.1: Add Prompt Version Tagging

**File**: `crates/roko-compose/src/system_prompt_builder.rs`
**What**: Tag each assembled prompt with a version identifier so that learning
data can be attributed to specific prompt versions.

**Steps**:
1. Add `prompt_version: String` field to `SystemPromptBuilder`
2. Derive the version from a hash of: template version, role identity text, section set, ordering strategy
3. Include the version in the `CompositionManifest` and `CognitiveWorkspace`
4. Store the version in episode metadata for attribution

**Acceptance criteria**:
- Every assembled prompt has a non-empty `prompt_version` string
- Changing the role identity text changes the version
- Changing the section set changes the version
- Version is recorded in episode metadata
- `cargo test -p roko-compose` passes

### Task 7.2: Wire Prompt A/B Testing via ExperimentStore

**File**: `crates/roko-compose/src/prompt_assembly_service.rs` and `crates/roko-learn/src/experiments.rs`
**What**: Support A/B testing prompt variants through the existing
`ExperimentStore`. Each experiment defines variants and tracks gate pass rates.

**Steps**:
1. Add `experiment_store: Option<Arc<ExperimentStore>>` to `PromptAssemblyService`
2. Define prompt experiment types: `reasoning_depth`, `anti_pattern_format`, `section_ordering`
3. In assembly, if an active experiment covers a prompt dimension, use the experiment's selected variant
4. After gate results, record the outcome against the variant
5. Periodically (every 50 observations), promote the winning variant

**Acceptance criteria**:
- Can define an experiment: `reasoning_depth` with variants `["suppress", "brief", "deep"]`
- Assembly uses the experiment's assigned variant for the current task
- Gate outcomes are recorded per variant
- After 50 observations, the experiment has a `current_winner`
- `cargo test -p roko-compose` and `cargo test -p roko-learn` pass

### Task 7.3: Add ReasoningDepth Tier-Based Default

**File**: `crates/roko-compose/src/system_prompt_builder.rs`
**What**: Add a `ReasoningDepth` enum (Suppress/Brief/Deep) and include
tier-appropriate reasoning instructions in the role identity layer.

**Steps**:
1. Add enum:
   ```rust
   pub enum ReasoningDepth {
       Suppress,  // "Do not explain. Just implement."
       Brief,     // "Briefly explain your approach, then implement."
       Deep,      // "Think step by step. Analyze, explain, implement."
   }
   ```
2. Add `with_reasoning_depth(depth: ReasoningDepth)` builder method
3. Inject reasoning instructions into Layer 1 (role identity) based on depth
4. Default: derive from `ContextTier` (Surgical -> Suppress, Focused -> Brief, Full -> Deep)
5. Allow experiment override from Task 7.2

**Acceptance criteria**:
- Surgical tier prompts contain "Do not explain" or equivalent
- Full tier prompts contain "Think step by step" or equivalent
- Experiment override changes the reasoning depth regardless of tier
- `cargo test -p roko-compose` passes

### Task 7.4: Wire Shared Vocabulary Injection for Multi-Agent Plans

**File**: `crates/roko-compose/src/context_mesh.rs` (if exists) or new helper in `prompt_assembly_service.rs`
**What**: When multiple agents work on tasks in the same plan, inject shared
vocabulary definitions to ensure consistent naming across agents.

**Steps**:
1. Add `shared_vocabulary: Option<Vec<(String, String)>>` to `PromptAssemblyService`
2. Builder method: `with_shared_vocabulary(vocab: Vec<(String, String)>)`
3. In assembly, if vocabulary is present, inject as a section in Layer 3c (active signals):
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

### Task 7.5: Wire Dependency Chain Context into Prompt Assembly

**File**: `crates/roko-compose/src/context_provider.rs` and `crates/roko-cli/src/orchestrate.rs`
**What**: When a task depends on completed prior tasks, inject a structured
summary of what those tasks produced and their gate outcomes.

**Steps**:
1. Expand `PriorTaskOutput` (if it exists) or create a struct:
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
4. Format as a "Completed Dependencies" section and inject into Layer 3 (domain context)
5. Only for Focused and Full tiers (Surgical skips this)

**Acceptance criteria**:
- A task with dependencies receives a "Completed Dependencies" section listing predecessors
- The section includes gate outcomes ("PASSED all gates", "FAILED clippy, retried, PASSED")
- Surgical tier tasks do not receive dependency context
- `cargo test -p roko-cli` passes

### Task 7.6: Content-Type-Aware Token Estimation

**File**: `crates/roko-compose/src/token_counter.rs`
**What**: Replace the flat 4.0 chars/token heuristic with content-type detection
for better budget accuracy.

**Steps**:
1. Add `content_aware_chars_per_token(content: &str) -> f64`:
   - Detect code indicators: `fn `, `struct `, `impl `, `pub `, `let `, `use `, `mod `
   - If code-heavy (> 5% of words are code keywords): return 3.0
   - If markdown-heavy (contains `##` or many `- ` lines): return 5.0
   - Otherwise: return 4.0 (prose default)
2. Add `TokenCounter::ContentAware` variant that uses this function
3. Wire `ContentAware` as the default counter in `PromptAssemblyService`
4. Keep `Heuristic { chars_per_token: 4.0 }` as fallback for callers that don't need accuracy

**Acceptance criteria**:
- Code-heavy content (Rust source) estimates ~3 chars/token
- Markdown documentation estimates ~5 chars/token
- Prose text estimates ~4 chars/token
- Budget enforcement is tighter (fewer over-budget assemblies)
- `cargo test -p roko-compose` passes

### Task 7.7: Role Identity from TOML Config

**File**: `crates/roko-compose/src/role_prompts.rs`
**What**: Load role identity text from `.roko/roles/<role>.toml` files at
startup, falling back to compiled-in defaults when config files are absent.

**Steps**:
1. Add `load_role_identity(role: &str, roko_dir: &Path) -> String`:
   - Try to read `.roko/roles/<role>.toml`
   - Parse `[role].identity` field
   - Fall back to `role_identity_for()` static strings
2. Add `[role.tier_adjustments]` support:
   - Surgical: use `tier_adjustments.surgical` text (terse)
   - Focused: use `tier_adjustments.focused` text (moderate)
   - Full: use base `identity` text (comprehensive)
3. Cache loaded roles for the duration of a plan run

**Acceptance criteria**:
- Without `.roko/roles/` directory, existing static role identities are used (no regression)
- With `.roko/roles/implementer.toml`, the custom identity is used for Implementer role
- Tier adjustments work: Surgical gets terse identity, Full gets comprehensive
- `cargo test -p roko-compose` passes

---

## Phase Summary

| Phase | Tasks | Issues Addressed | Key Files | Effort | Impact |
|---|---|---|---|---|---|
| 1 (Tier Wiring) | 1.1-1.6 | ISS-01, ISS-06 | prompt_assembly_service.rs, budget.rs, context_provider.rs, orchestrate.rs, run.rs | 2-3d | Critical |
| 2 (Budget Prediction) | 2.1-2.4 | ISS-02 | orchestrate.rs, budget_predictor.rs | 1-2d | Critical |
| 3 (Effectiveness Loop) | 3.1-3.3 | ISS-04, ISS-11 | prompt.rs, prompt_assembly_service.rs, cognitive_workspace.rs | 1-2d | High |
| 4 (Chat + ACP) | 4.1-4.5 | ISS-03, ISS-05, ISS-09, ISS-17 | dispatch_direct.rs, chat_session.rs, compaction.rs, runner.rs, orchestrate.rs | 2-3d | High |
| 5 (VCG + Foraging) | 5.1-5.4 | ISS-08, ISS-10 | strategy.rs, prompt.rs, auction.rs, foraging.rs, orchestrate.rs | 3-4d | Medium-High |
| 6 (Attention + Refinement) | 6.1-6.5 | ISS-07, ISS-13, ISS-14 | attention.rs, prompt_assembly_service.rs | 2-3d | Medium |
| 7 (Versioning + Multi-Agent) | 7.1-7.7 | ISS-12, ISS-15, ISS-16 | system_prompt_builder.rs, experiments.rs, token_counter.rs, role_prompts.rs, context_provider.rs | 3-4d | Medium |

**Recommended execution order**: Phase 1 -> Phase 2 -> Phase 3 -> Phase 4 -> Phase 5 -> Phase 6 -> Phase 7.

Phase 1 fixes the user's core pain point (small model overload). Phase 2
closes the budget learning loop. Phase 3 makes learning actionable. Phase 4
brings all entry points to parity. Phases 5-7 are optimization and polish.

---

## Success Criteria

### Phase 1+2 Complete (Critical Milestone):
- `is_local_model("ollama/llama3.2")` returns true
- Dispatching to Ollama produces a system prompt <= 4K tokens
- Dispatching to Sonnet produces a system prompt <= 12K tokens
- Dispatching to Opus produces a system prompt <= 24K tokens
- `BudgetPredictor.predict()` is called before assembly in `dispatch_agent_with()`
- `BudgetPredictor.record()` is called after gate results are known
- After 10+ tasks, predicted budgets are used (blended with static defaults)

### Phase 3+4 Complete (High Impact Milestone):
- `SectionInfluence.weights()` is applied as a multiplier during composition
- Sections with negative lift are visibly demoted in budget after 20+ tasks
- `roko chat` produces a system prompt with role identity and conventions
- `roko "prompt"` produces a system prompt (not bare)
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
- `crates/roko-cli/src/orchestrate.rs` -- dispatch_agent_with, model selection, inline prompts
- `crates/roko-cli/src/run.rs` -- roko run path
- `crates/roko-cli/src/dispatch_direct.rs` -- bare dispatch, no system prompt
- `crates/roko-cli/src/chat_session.rs` -- chat REPL
- `crates/roko-acp/src/runner.rs` -- run_multi_role_review, inline prompts
- `crates/roko-learn/src/experiments.rs` -- ExperimentStore
