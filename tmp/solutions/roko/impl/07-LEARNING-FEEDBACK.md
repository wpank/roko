# Implementation Plan 07: Learning & Feedback Subsystem

> Every model call, regardless of entry point, must record learning signals. The
> learning subsystem must close feedback loops that are currently open: routing
> observations must flow back to model selection, anomalies must trigger
> interventions, knowledge scores must influence prompt assembly, and the dream
> cycle must run without manual invocation.
>
> Source analysis: `18-LEARN-AUDIT.md`, `18-LEARN-GOALS.md`, `18-LEARN-ISSUES.md`,
> `18-LEARN-PLAN.md`. All referenced structs and methods exist in the codebase
> today -- this plan wires them, it does not build new components.

---

## Phase 0: Universal Feedback Wiring

**Goal:** Every model-calling entry point emits `FeedbackEvent`s to a shared
`FeedbackService`. After this phase, `roko chat`, ACP, and `roko run` all
produce episode, cost, and routing data.

**Depends on:** Nothing.

### Task 0.1: Wire FeedbackService to `roko chat` session setup

**Issue:** I-01
**File:** `crates/roko-cli/src/chat_session.rs`

**Steps:**
1. Import `roko_learn::feedback_service::FeedbackService`
2. In the chat session constructor or `run()`/`start()` method, locate the point where
   the `.roko` directory is resolved
3. Create `FeedbackService::from_roko_dir_with_episodes(&roko_dir)` and store it on
   the session struct (or pass it through as a parameter)
4. Verify the `FeedbackService` is accessible where model calls are dispatched

**Acceptance criteria:**
- `FeedbackService` is instantiated in chat session setup
- Compiles cleanly with `cargo check -p roko-cli`

### Task 0.2: Emit ModelCall events from `roko chat`

**Issue:** I-01
**File:** `crates/roko-cli/src/chat_session.rs`
**Depends on:** Task 0.1

**Steps:**
1. After each model response is received, emit `FeedbackEvent::ModelCall` with:
   - `role`: `"chat"`
   - `model`: current model slug from the agent/backend config
   - `input_tokens`, `output_tokens`: from provider response usage data
   - `cost_usd`: from provider response or computed from token counts
   - `latency_ms`: wall-clock duration of the model call
   - `success`: `true` (model responded without error)
   - `prompt_section_ids`: empty vec (chat has no structured prompt sections)
   - `knowledge_ids`: empty vec (future: knowledge-augmented chat)
   - `run_id`: generate a UUID per chat session
2. Call `feedback_service.record(event).await` (or sync `flush()` depending on the
   async context)

**Acceptance criteria:**
- Run `roko chat`, send one message, exit
- `.roko/learn/efficiency.jsonl` contains a `model_call` record with the chat session's
  model, non-zero tokens, and `role: "chat"`

### Task 0.3: Emit WorkflowComplete from chat session close

**Issue:** I-01
**File:** `crates/roko-cli/src/chat_session.rs`
**Depends on:** Task 0.1

**Steps:**
1. On chat session end (user exits, Ctrl-C, or error), emit
   `FeedbackEvent::WorkflowComplete` with:
   - `event_type`: `"chat_session"`
   - `run_id`: same UUID as ModelCall events
   - `model`: last model used
   - `success`: `true` (session completed normally) or `false` (error exit)
   - `outcome`: `"completed"` or `"error: {reason}"`
   - `total_cost_usd`: sum of all model call costs in the session
2. Call `feedback_service.flush()` to ensure all buffered events are persisted

**Acceptance criteria:**
- After a chat session, `.roko/learn/efficiency.jsonl` contains a `workflow_complete`
  record with aggregate cost

### Task 0.4: Wire FeedbackService to `roko chat --inline` (chat_inline.rs)

**Issue:** I-01
**File:** `crates/roko-cli/src/chat_inline.rs`

**Steps:**
1. Apply the same pattern as Tasks 0.1-0.3 to the inline chat path
2. Create `FeedbackService::from_roko_dir_with_episodes()` in the inline session setup
3. Emit `ModelCall` after each model response
4. Emit `WorkflowComplete` on session close
5. Call `flush()` on exit

**Acceptance criteria:**
- Inline chat sessions produce the same feedback records as interactive chat

### Task 0.5: Wire FeedbackService to ACP runner

**Issue:** I-02
**File:** `crates/roko-acp/src/runner.rs`

**Steps:**
1. Import `roko_learn::feedback_service::FeedbackService`
2. In ACP runner initialization, create `FeedbackService::from_roko_dir(&roko_dir)`
3. Store the service on the runner struct or in shared pipeline state
4. Thread the service through to wherever model dispatch and gate execution happen

**Acceptance criteria:**
- `FeedbackService` is available in the ACP pipeline
- Compiles cleanly with `cargo check -p roko-acp`

### Task 0.6: Emit ModelCall and GateResult events from ACP pipeline

**Issue:** I-02
**File:** `crates/roko-acp/src/pipeline.rs`
**Depends on:** Task 0.5

**Steps:**
1. After each ACP model dispatch, emit `FeedbackEvent::ModelCall` with:
   - `role`: `"acp"` or the ACP bridge event type
   - `model`, `input_tokens`, `output_tokens`, `cost_usd`, `latency_ms`, `success`:
     from the model response
   - `run_id`: ACP session or request ID
2. After each ACP gate run (currently only writes adaptive thresholds), also emit
   `FeedbackEvent::GateResult` with gate name, passed status, duration
3. Flush on pipeline completion

**Acceptance criteria:**
- Run an ACP code completion or inline edit
- `.roko/learn/efficiency.jsonl` contains both `model_call` and `gate_result` records
  from the ACP session

### Task 0.7: Attach CascadeRouter to FeedbackService in `roko run`

**Issue:** I-03 (partial)
**File:** `crates/roko-cli/src/run.rs`

**Steps:**
1. Import `roko_learn::cascade_router::CascadeRouter`
2. In `roko run` initialization (near the `LearningRuntime` setup around line 2664),
   load CascadeRouter via `CascadeRouter::load_or_new(&learn_root.join("cascade-router.json"))`
3. Wrap in `Arc::new()`
4. Create `FeedbackService::from_roko_dir_with_episodes(&roko_dir)
   .with_cascade_router(Arc::clone(&router))`
5. Store the FeedbackService and emit `ModelCall` events after each model dispatch
6. After model call, the FeedbackService automatically calls `router.observe()` with
   the success/failure reward
7. On session end, persist router state via `router.save()`

**Acceptance criteria:**
- Run `roko run "test"` twice
- `.roko/learn/cascade-router.json` exists after first run
- Observation count in the JSON increases after second run

### Task 0.8: Deduplicate episode writes in `roko run`

**Issue:** I-07
**File:** `crates/roko-cli/src/run.rs`
**Depends on:** Task 0.7

**Steps:**
1. Locate the direct `append_episode_log()` call at line ~1301
2. Remove it -- `LearningRuntime::record_completed_run()` already writes episodes
   to `.roko/learn/episodes.jsonl`
3. Verify all episode fields (HDC fingerprint, gate verdicts, prompt snapshot) are
   populated in the `CompletedRunInput` path
4. If any fields are only populated in the direct-write path, copy that population
   logic to the `CompletedRunInput` construction
5. Also remove the helper `async fn append_episode_log()` at line ~2545 if it becomes
   dead code

**Acceptance criteria:**
- Run `roko run "test"`
- Exactly one episode record per execution appears (not two)
- Check both `.roko/episodes.jsonl` (if it exists) and `.roko/learn/episodes.jsonl`
  to confirm no double-writes

### Task 0.9: Wire FeedbackService to `dispatch_v2.rs`

**File:** `crates/roko-cli/src/dispatch_v2.rs`

**Steps:**
1. `dispatch_v2.rs` already imports and creates a `FeedbackService` (lines 60, 89)
   but only constructs it -- verify it actually emits events
2. If events are not emitted after model calls, add `ModelCall` emission
3. Ensure the FeedbackService is flushed on dispatch completion
4. If a CascadeRouter is available from the caller context, attach it

**Acceptance criteria:**
- Code paths through `dispatch_v2.rs` produce feedback records in `efficiency.jsonl`

---

## Phase 1: CascadeRouter Live Model Selection

**Goal:** `roko run` uses the CascadeRouter to select models instead of always
using the config default. The router accumulates observations and transitions
through its 3 stages (Static -> Confidence -> UCB).

**Depends on:** Phase 0 (router must be loaded and receiving observations).

### Task 1.1: Use CascadeRouter for model selection in `roko run`

**Issue:** I-03
**File:** `crates/roko-cli/src/run.rs`
**Depends on:** Task 0.7

**Steps:**
1. Before model dispatch, construct a `RoutingContext` from available state:
   - `task_category`: derive from prompt analysis or default `TaskCategory::Implementation`
   - `complexity`: from prompt length heuristic or `ComplexityBand::Standard`
   - `role`: from agent config or `AgentRole::Implementer`
   - `iteration`: current retry count (0 on first attempt)
   - `has_prior_failure`: `iteration > 0`
   - `crate_familiarity`: 0.0 initially (enriched in Task 1.2)
2. Call `cascade_router.select_for_frequency_among(&ctx, &candidate_models)`
3. Use the selected model slug for dispatch instead of config default
4. Fall back to config default if router returns no candidate or is in error
5. Log routing decision: `info!("routing: stage={}, model={}, score={}", ...)`

**Acceptance criteria:**
- `roko run` dispatches to the model selected by CascadeRouter
- Routing decision is visible in logs
- When CascadeRouter has insufficient observations, falls back to config default

### Task 1.2: Enrich RoutingContext with full 18-dimensional features

**Issue:** I-04
**File:** `crates/roko-cli/src/run.rs`, `crates/roko-learn/src/runtime_feedback.rs`

**Steps:**
1. Add `routing_context: Option<RoutingContext>` to `CompletedRunInput`
2. In `roko run`, construct the full `RoutingContext` (18 dims) with:
   - `conductor_load`: 0.0 for single-run (accurate for non-orchestrated)
   - `active_agents`: 0 for single-run
   - `daimon_policy`: load from `.roko/daimon/affect.json` if it exists, else default
   - `plan_context_tokens`: from prompt assembly token count
   - `crate_familiarity`: compute from episode history (`success_count / total_count`
     for the current working directory or crate)
   - `cache_affinity`: 1.0 when the candidate model matches the previous model used
3. In `LearningRuntime::record_completed_run()`, use the provided `RoutingContext`
   for the router observation instead of the derived simplified one

**Acceptance criteria:**
- Run `roko run` with different prompt types (simple question vs. complex implementation)
- Context vectors in `cascade-router.json` show differentiated features (not all zeros)

### Task 1.3: Wire section effectiveness weights into PromptAssemblyService

**Issue:** I-09
**File:** `crates/roko-compose/src/prompt_assembly_service.rs`,
`crates/roko-cli/src/run.rs`

**Steps:**
1. Load `SectionEffectivenessRegistry` from `.roko/learn/section-effects.json`
2. Extract budget weights via `registry.budget_weights()` (returns
   `HashMap<String, f64>` mapping section name to weight in [0.5, 1.5])
3. Pass weights to `PromptAssemblyService` via its existing `section_weights` field
   or a new `with_section_weights()` builder method
4. During prompt assembly, multiply each section's token budget allocation by its
   effectiveness weight
5. Log sections with weight < 0.7: `warn!("deprioritizing section '{}': lift={}", ...)`
6. Log sections with weight > 1.3: `info!("boosting section '{}': lift={}", ...)`

**Acceptance criteria:**
- After 50+ tasks with varying success, sections with negative lift have reduced
  token allocation in assembled prompts
- Log output shows section priority adjustments

### Task 1.4: Wire provider health circuit breaker to CascadeRouter

**Issue:** I-19
**File:** `crates/roko-learn/src/cascade_router.rs`, `crates/roko-cli/src/run.rs`

**Steps:**
1. Load `ProviderHealthTracker` from `.roko/learn/provider-health.json` at session
   start (or create new if file missing)
2. Pass health tracker to CascadeRouter (it already has a `ProviderHealthRegistry`
   parameter in helper functions)
3. Before UCB scoring in `select_for_frequency_among()`, filter out models whose
   provider circuit breaker is open (failure rate exceeds threshold)
4. After model call failure, call `health_tracker.record_failure(provider)`
5. After model call success, call `health_tracker.record_success(provider)`
6. Log when a provider circuit opens: `warn!("circuit open for provider '{}', excluding from routing", ...)`

**Acceptance criteria:**
- After 5+ consecutive failures from a provider, its models are excluded from routing
- Circuit closes after a configurable recovery period
- Excluded providers appear in `roko status` output

---

## Phase 2: Budget Enforcement and Cost Protection

**Goal:** Budget limits are enforced in live paths. Cost aggregation works across
sessions. No runaway agent can spend unlimited money.

**Depends on:** Phase 0 (cost tracking via FeedbackService must be active).

### Task 2.1: Load budget configuration from roko.toml

**Issue:** I-05
**File:** `crates/roko-cli/src/config.rs`

**Steps:**
1. Ensure the `[budget]` section in `roko.toml` is parsed:
   ```toml
   [budget]
   max_task_usd = 1.00
   max_session_usd = 10.00
   max_day_usd = 50.00
   ```
2. Map to `roko_learn::budget::BudgetConfig` (or create a config adapter)
3. Default values: task=1.00, session=10.00, day=50.00
4. Make all three optional -- missing means no limit for that scope

**Acceptance criteria:**
- `roko.toml` with `[budget]` section loads without error
- Missing `[budget]` section results in defaults

### Task 2.2: Instantiate BudgetGuardrail in `roko run`

**Issue:** I-05
**File:** `crates/roko-cli/src/run.rs`
**Depends on:** Task 2.1

**Steps:**
1. Import `roko_learn::budget::{BudgetGuardrail, BudgetAction}`
2. Load budget config from parsed `roko.toml`
3. Compute today's already-spent amount via `CostsDb::aggregate_since(today_start)`
   from `.roko/learn/costs.jsonl`
4. Initialize `BudgetGuardrail` with config limits and `day_spent` pre-loaded
5. Store guardrail on the run context

**Acceptance criteria:**
- `BudgetGuardrail` is instantiated with correct limits and prior spend
- Compiles cleanly

### Task 2.3: Enforce budget before each model dispatch

**Issue:** I-05
**File:** `crates/roko-cli/src/run.rs`
**Depends on:** Task 2.2

**Steps:**
1. Before each model dispatch, estimate cost from model pricing and prompt token count
2. Call `guardrail.check(estimated_cost, scope)` which returns a `BudgetAction`
3. Handle each action:
   - `BudgetAction::Ok`: proceed normally
   - `BudgetAction::Warn`: log warning, proceed
   - `BudgetAction::RouteToCheaper`: bias CascadeRouter to select a lower-tier model
     (set `budget_pressure: true` in routing context)
   - `BudgetAction::BlockNewSessions`: reject new session starts, allow current to finish
   - `BudgetAction::Block`: fail the dispatch with a budget-exceeded error
4. After model call completes, record actual cost: `guardrail.record_cost(actual_cost, scope)`

**Acceptance criteria:**
- Set `max_task_usd = 0.001` in `roko.toml`, run a task
- Task is either blocked or downtiered to a cheaper model
- Warning log appears before blocking

### Task 2.4: Cross-session cost aggregation for daily budget

**Issue:** I-12
**File:** `crates/roko-learn/src/costs_db.rs`

**Steps:**
1. Add `CostsDb::aggregate_since(since: chrono::DateTime<Utc>) -> f64` method
2. Implementation: iterate JSONL records, sum `cost_usd` where `timestamp >= since`
3. For efficiency: if the file is large, seek from end (costs are append-only, so
   recent records are at the end)
4. Used by Task 2.2 to initialize daily spend

**Acceptance criteria:**
- Unit test: write 10 cost records spanning 3 days, aggregate for today returns
  only today's costs
- `aggregate_since(24h_ago)` returns correct sum

### Task 2.5: Surface budget state in `roko status`

**File:** `crates/roko-cli/src/commands/mod.rs`
**Depends on:** Task 2.2

**Steps:**
1. In the `status` command handler, load budget config and compute current spend
2. Display budget utilization:
   ```
   Budget:
     Task:    $0.12 / $1.00 (12%)
     Session: $1.45 / $10.00 (14.5%)
     Day:     $8.30 / $50.00 (16.6%)
   ```
3. Color-code: green < 50%, yellow 50-80%, red > 80%
4. If no budget config, show "Budget: not configured"

**Acceptance criteria:**
- `roko status` shows budget utilization matching actual cost records

---

## Phase 3: Conductor Integration and Learned Retry

**Goal:** Retry decisions are learned via the ConductorBandit instead of
hardcoded. The conductor observes failure patterns and selects interventions
that improve over time.

**Depends on:** Phase 0 (feedback events from retry attempts).

### Task 3.1: Load ConductorBandit at session start

**Issue:** I-06
**File:** `crates/roko-cli/src/run.rs`

**Steps:**
1. Import `roko_learn::conductor::{ConductorBandit, ConductorAction, ConductorState}`
2. Load from `.roko/learn/conductor.json` via `ConductorBandit::load_or_new(&path)`
3. Store on the run context alongside CascadeRouter and BudgetGuardrail
4. If file is missing or corrupt, start from a new bandit (graceful degradation)

**Acceptance criteria:**
- ConductorBandit is loaded and available in the retry loop
- Missing file does not cause errors

### Task 3.2: Call conductor on task failure for intervention selection

**Issue:** I-06
**File:** `crates/roko-cli/src/run.rs`
**Depends on:** Task 3.1

**Steps:**
1. On task failure (gate fail or agent error), build `ConductorState`:
   - `iteration`: current retry count
   - `consecutive_failures`: count of consecutive failures for this task
   - `error_pattern`: classify from gate/agent error output using the 10 `ErrorPattern`
     variants (Compile, Test, ToolCall, Timeout, etc.)
   - `elapsed_ms`: wall clock since task start
   - `cost_so_far_usd`: cumulative cost for this task
   - `model_tier`: hash of current model slug
   - `task_complexity`: from routing context complexity band
2. Call `bandit.select_action(&state)` to get `ConductorAction`
3. Execute the selected action:
   - `Continue`: retry with same prompt and model (current default behavior)
   - `InjectHint(ErrorDigest)`: add error summary to the next prompt iteration
   - `InjectHint(SkillSuggestion)`: query SkillLibrary for matching skill, inject
   - `InjectHint(SimplifyApproach)`: add simplification directive to prompt
   - `SwitchModel`: request a different model from CascadeRouter for the retry
   - `Restart`: reset task state, clear context, retry from scratch
   - `Abort`: mark task as permanently failed, stop retrying

**Acceptance criteria:**
- On task failure, conductor selects an action (logged at INFO)
- After 20+ failures across multiple tasks, conductor starts selecting
  non-`Continue` actions with measurable frequency

### Task 3.3: Record conductor reward after retry outcome

**Issue:** I-06
**File:** `crates/roko-cli/src/run.rs`
**Depends on:** Task 3.2

**Steps:**
1. After a retry attempt completes (success or failure), compute reward:
   - Success after intervention: reward = 1.0
   - Failure after intervention: reward = 0.0
   - Success with Continue (no intervention): reward = 0.5 (baseline)
2. Call `bandit.observe(&state, action, reward)` with the original state and action
3. Persist bandit state via `bandit.save(&path)` after each observation
4. Record the conductor decision in the episode: add `conductor_action` field to
   `CompletedRunInput.extra`

**Acceptance criteria:**
- `.roko/learn/conductor.json` shows observation counts > 0 after retried tasks
- Episode records include the conductor action taken

### Task 3.4: Surface conductor state in `roko learn all`

**File:** `crates/roko-cli/src/commands/learn.rs`
**Depends on:** Task 3.1

**Steps:**
1. Load ConductorBandit from `.roko/learn/conductor.json`
2. Display action selection distribution:
   ```
   Conductor (56 observations):
     Continue:              42%  (Thompson mean: 0.35)
     InjectHint(Error):     23%  (Thompson mean: 0.62)
     SwitchModel:           18%  (Thompson mean: 0.71)
     InjectHint(Simplify):   9%  (Thompson mean: 0.48)
     Abort:                  5%  (Thompson mean: 0.20)
     Restart:                3%  (Thompson mean: 0.15)
   ```
3. Show total observations, most effective action, least effective action

**Acceptance criteria:**
- `roko learn all` shows conductor action distribution with success rates

---

## Phase 4: Anomaly Detection and Regression Alerting

**Goal:** The system detects prompt loops, cost spikes, quality degradation, and
metric regressions in real time. Detected anomalies trigger warnings and may
feed conductor decisions.

**Depends on:** Phase 0 (feedback events must be flowing).

### Task 4.1: Wire AnomalyDetector to `roko run`

**Issue:** I-14
**File:** `crates/roko-cli/src/run.rs`

**Steps:**
1. Import `roko_learn::anomaly::AnomalyDetector`
2. Create `AnomalyDetector::new(session_start_ms)` at session start
3. After each model call, feed data to the detector:
   - `detector.check_prompt(prompt_hash)` where prompt_hash is a hash of the
     rendered prompt text
   - `detector.check_cost(cost_usd)` with the model call cost
4. After each gate result:
   - `detector.check_quality(pass_rate)` with the rolling pass rate

**Acceptance criteria:**
- AnomalyDetector is instantiated and receiving data in `roko run`
- No crashes or errors from detector checks

### Task 4.2: Handle detected anomalies

**File:** `crates/roko-cli/src/run.rs`
**Depends on:** Task 4.1

**Steps:**
1. When `check_prompt()` returns `Some(Anomaly::PromptLoop)`:
   - `warn!("Prompt loop detected: same prompt hash seen {} times", count)`
   - If count >= 5: set a flag that biases conductor toward `Abort` or `Restart`
2. When `check_cost()` returns `Some(Anomaly::CostSpike)`:
   - `warn!("Cost spike: ${:.4} exceeds EWMA baseline by {:.1}x", cost, ratio)`
   - Increase budget pressure on next dispatch
3. When `check_quality()` returns `Some(Anomaly::QualityDrift)`:
   - `warn!("Quality degradation: recent pass rate {:.0}% below baseline", rate * 100)`
   - Consider triggering a model switch via conductor

**Acceptance criteria:**
- Create a prompt loop (repeat same input 10x): warning appears at iteration 5+
- Create a cost spike (one very expensive call after cheap ones): warning appears

### Task 4.3: Wire AnomalyDetector to `roko chat`

**File:** `crates/roko-cli/src/chat_session.rs`
**Depends on:** Task 0.1

**Steps:**
1. Create `AnomalyDetector` in chat session setup
2. After each model response, check for cost spikes
3. Prompt loop detection is less useful in chat (user naturally repeats questions),
   so only check cost anomalies and optionally warn on repeated identical messages
4. Log warnings but do not abort chat sessions (user is interactive)

**Acceptance criteria:**
- Chat session logs cost spike warnings when they occur

### Task 4.4: Surface regression alerts from LearningRuntime

**Issue:** I-13
**File:** `crates/roko-cli/src/run.rs`

**Steps:**
1. After `record_completed_run()` returns `LearningUpdate`, check
   `update.regression_report`
2. If the report contains alerts:
   - For `AlertSeverity::Warning`: `warn!("Regression: {}", alert.description)`
   - For `AlertSeverity::Alert`: `error!("Severe regression: {}", alert.description)`
   - For pass_rate_drop alerts: log suggestion to check model selection
   - For cost_increase alerts: log suggestion to check budget config
3. Persist recent regression alerts to `.roko/learn/regression-alerts.jsonl`

**Acceptance criteria:**
- After a sequence of passing tasks followed by failing tasks, regression alert
  appears in logs
- Alert is persisted to the JSONL file

### Task 4.5: Show regression alerts in `roko status`

**Issue:** I-13
**File:** `crates/roko-cli/src/commands/mod.rs`
**Depends on:** Task 4.4

**Steps:**
1. Load recent regression alerts from `.roko/learn/regression-alerts.jsonl`
2. Display in `roko status` under a "Regressions" section:
   ```
   Regressions (last 24h):
     [!] Pass rate dropped from 85% to 60% (task_category=Implementation)
     [!] Cost per task increased from $0.05 to $0.12 (model=claude-sonnet-4-20250514)
   ```
3. If no regressions, show "No regressions detected"

**Acceptance criteria:**
- `roko status` shows regression alerts when they exist

---

## Phase 5: Dream Consolidation Triggers

**Goal:** The dream cycle runs automatically on schedule and on plan completion,
producing knowledge entries, playbooks, and routing advice without manual
invocation.

**Depends on:** Phase 0 (episodes must be recorded for dreams to consolidate).

### Task 5.1: Add dream configuration to roko.toml

**Issue:** I-08
**File:** `crates/roko-core/src/config/serve.rs` or relevant config module

**Steps:**
1. Define `[dreams]` config section:
   ```toml
   [dreams]
   enabled = true
   schedule = "0 3 * * *"    # cron: daily at 3 AM
   budget_usd = 0.10
   model = "claude-haiku-3-5"
   min_episodes = 10         # minimum episodes before running
   ```
2. Parse into a `DreamConfig` struct
3. Default: enabled=false (opt-in), budget=0.10, min_episodes=10

**Acceptance criteria:**
- Config section parses without error
- Missing section yields defaults with enabled=false

### Task 5.2: Spawn dream loop in `roko serve`

**Issue:** I-08
**File:** `crates/roko-serve/src/runtime.rs`
**Depends on:** Task 5.1

**Steps:**
1. Import `roko_dreams::runner::DreamRunner`
2. In `roko serve` startup, if `dreams.enabled`:
   - Create `DreamRunner` with config from Task 5.1
   - Spawn as a background tokio task: `DreamRunner::start()` with cron trigger
   - Register the task handle for graceful shutdown
3. Log dream cycle start/completion with report summary:
   ```
   info!("Dream cycle completed: {} clusters, {} knowledge entries, {} playbooks",
         report.clusters, report.new_knowledge, report.new_playbooks)
   ```

**Acceptance criteria:**
- Start `roko serve` with dreams enabled
- Dream cycle runs at scheduled time (use short schedule for testing, e.g., every 5 min)
- Dream cycle report is written to `.roko/dreams/`

### Task 5.3: Trigger dream cycle on plan completion

**Issue:** I-08
**File:** `crates/roko-cli/src/run.rs`
**Depends on:** Task 5.1

**Steps:**
1. After a plan completes (all tasks done), check dream config
2. If `dreams.enabled` and episode count >= `min_episodes`:
   - Spawn a one-shot dream cycle in a background task
   - Do not block the main execution path
3. Log: `info!("Triggering post-plan dream consolidation ({} episodes)", count)`

**Acceptance criteria:**
- After plan completion with sufficient episodes, dream cycle runs in background
- Main execution returns immediately (not blocked by dream)

### Task 5.4: Surface dream status in `roko status`

**File:** `crates/roko-cli/src/commands/mod.rs`
**Depends on:** Task 5.2

**Steps:**
1. Check for dream cycle reports in `.roko/dreams/`
2. Display last cycle info:
   ```
   Dreams:
     Last cycle: 2026-04-29 03:00 UTC (4 clusters, 12 knowledge entries)
     Next scheduled: 2026-04-30 03:00 UTC
     Total cycles: 7
   ```
3. If dreams are disabled: "Dreams: disabled"
4. If no cycles have run: "Dreams: enabled, awaiting first run"

**Acceptance criteria:**
- `roko status` shows dream cycle information

---

## Phase 6: Knowledge Feedback Scoring and Routing Intelligence

**Goal:** Knowledge entries that contribute to gate passes get boosted; entries
that correlate with failures get deprioritized. Dream routing advice informs
CascadeRouter model selection. Knowledge-informed routing is active.

**Depends on:** Phase 0 + Phase 1 (FeedbackService and CascadeRouter must be live).

### Task 6.1: Record knowledge entry usage after prompt assembly

**File:** `crates/roko-cli/src/run.rs`, `crates/roko-compose/src/prompt_assembly_service.rs`

**Steps:**
1. After prompt assembly, capture the list of knowledge entry IDs that were included
   in the prompt via `assembler.last_knowledge_ids()` (or equivalent accessor)
2. Store these IDs alongside the `run_id` for later attribution
3. Include `knowledge_ids` in the `FeedbackEvent::ModelCall` emission

**Acceptance criteria:**
- When knowledge entries are included in prompts, their IDs appear in the ModelCall
  event's `knowledge_ids` field

### Task 6.2: Update knowledge scores on gate outcome

**File:** `crates/roko-learn/src/feedback_service.rs`
**Depends on:** Task 6.1

**Steps:**
1. Verify that `FeedbackService` already resolves provenance on `GateResult`:
   - On `ModelCall`: stores `run_id -> knowledge_ids` in provenance map
   - On `GateResult`: looks up provenance for `run_id`, applies +1 (pass) or -1 (fail)
2. Ensure this path is exercised in the live `roko run` flow (not just tests)
3. Verify scores are persisted to `.roko/learn/knowledge-scores.json` on flush
4. Verify scores are loaded on restart via `load_knowledge_scores()`

**Acceptance criteria:**
- Add a knowledge entry, run tasks that include it, check
  `.roko/learn/knowledge-scores.json` shows accumulating scores
- Scores persist across sessions

### Task 6.3: Use knowledge scores in prompt assembly retrieval ranking

**File:** `crates/roko-compose/src/prompt_assembly_service.rs`

**Steps:**
1. Load knowledge scores from `.roko/learn/knowledge-scores.json`
2. During knowledge entry retrieval/ranking, multiply the base relevance score by
   a knowledge-score factor: `factor = 1.0 + (score * 0.05).clamp(-0.5, 0.5)`
3. Entries with positive scores get boosted; entries with negative scores get
   deprioritized (but not excluded -- anti-knowledge gating handles exclusion)
4. Log when a knowledge entry is deprioritized due to negative score

**Acceptance criteria:**
- An entry with score +10 ranks higher than an equally-relevant entry with score -5
- Log output shows score-based ranking adjustments

### Task 6.4: Load dream routing advice into CascadeRouter

**Issue:** I-10
**File:** `crates/roko-cli/src/run.rs`, `crates/roko-learn/src/cascade_router.rs`

**Steps:**
1. At session start, call `roko_dreams::load_dream_routing_advice(&workdir)` to load
   `DreamRoutingAdvice` from `.roko/learn/dream-routing-advice.json`
2. Convert to routing bias via `dream_advice_to_routing_bias(&advice, &model_configs)`
3. Apply bias to CascadeRouter: the bias adjusts UCB scores for recommended model
   changes (e.g., "use haiku for simple compile tasks" shifts scores)
4. Log applied biases: `info!("Dream routing advice: {} recommendations applied", count)`

**Acceptance criteria:**
- After a dream cycle produces routing advice, subsequent runs show the advice
  influencing model selection in the routing log
- Without dream advice file, routing proceeds normally (no error)

### Task 6.5: Feed routing outcomes back to knowledge store

**File:** `crates/roko-cli/src/run.rs`
**Depends on:** Task 6.4

**Steps:**
1. After a routing decision based on dream advice leads to a gate outcome, record
   whether the advice was correct
2. If advice said "use model X for pattern Y" and model X succeeded: +1 to the
   knowledge entry that generated the advice
3. If advice said "use model X for pattern Y" and model X failed: -1
4. This closes the loop: dreams produce advice -> advice influences routing ->
   routing outcomes validate advice -> scores update

**Acceptance criteria:**
- Knowledge entries that generated good routing advice accumulate positive scores
- Knowledge entries that generated bad routing advice accumulate negative scores

---

## Phase 7: Experiment Automation and Calibration

**Goal:** Experiments conclude automatically and winners are applied. The dream
cycle proposes new experiments. Calibration tracks predict-vs-actual accuracy.

**Depends on:** Phase 1 (CascadeRouter) + Phase 5 (dream cycle).

### Task 7.1: Auto-apply experiment winners on startup

**Issue:** I-11
**File:** `crates/roko-learn/src/prompt_experiment.rs`, `crates/roko-cli/src/run.rs`

**Steps:**
1. Add `ExperimentStore::concluded_winners() -> Vec<ExperimentWinner>` method that
   returns all experiments in `Concluded` state with their winning variant
2. At `roko run` startup, load `experiment-winners.json`
3. For each winner with `auto_apply: true` (or when global `auto_apply_winners`
   config is true):
   - Apply the winning variant value to the corresponding config setting
   - For prompt section experiments: update the section text in prompt config
   - For model experiments: update the default model for the experiment's role
4. Log auto-applied winners: `info!("Auto-applied experiment winner: '{}' -> variant '{}'", name, winner)`
5. Add `auto_apply_winners: bool` to `[learning]` config (default: true)

**Acceptance criteria:**
- Conclude an experiment manually (set sufficient stats), restart `roko run`
- Winner is auto-applied and logged
- With `auto_apply_winners = false`, winner is not applied

### Task 7.2: Propose experiments from dream cycle insights

**Issue:** Dream cycle improvements
**File:** `crates/roko-dreams/src/cycle.rs`, `crates/roko-learn/src/prompt_experiment.rs`

**Steps:**
1. During dream cycle integration phase, after knowledge consolidation:
   - Identify insights with actionable implications for prompt sections
   - For each actionable insight, check if an experiment already exists for that target
2. If no experiment exists and the insight confidence is above threshold (0.6):
   - Create `ExperimentProposal` with: target section, control text, variant text
   - Log: `info!("Dream proposed experiment: test '{}' for section '{}'", ...)`
3. Proposed experiments go into a staging area (`proposed-experiments.json`) for
   review before activation
4. Add `roko learn experiments --proposed` to list staged proposals

**Acceptance criteria:**
- After a dream cycle with sufficient episodes, proposed experiments appear in
  `proposed-experiments.json`
- Proposals include control text, variant text, and the insight that motivated them

### Task 7.3: Activate proposed experiments

**File:** `crates/roko-cli/src/commands/learn.rs`
**Depends on:** Task 7.2

**Steps:**
1. Add `roko learn experiments activate <id>` command
2. Moves experiment from `proposed-experiments.json` to `experiments.json` with
   status `Running`
3. Add `roko learn experiments activate --all` to activate all pending proposals
4. Log activation

**Acceptance criteria:**
- `roko learn experiments --proposed` shows proposals
- `roko learn experiments activate <id>` moves the experiment to active

### Task 7.4: Wire calibration predict-publish-correct loop

**File:** `crates/roko-learn/src/calibration_policy.rs`,
`crates/roko-cli/src/run.rs`

**Steps:**
1. Before dispatch, record a prediction from CascadeRouter:
   `predicted_success = router.predict_success_probability(&ctx, &model)`
2. After gate outcome, record actual: `actual_success = gate_passed`
3. Compute calibration error: `|predicted - actual|` for this observation
4. Feed to `CalibrationPolicy`:
   - Track rolling calibration error (EMA)
   - When error is high (> 0.3): increase CascadeRouter alpha (more exploration)
   - When error is low (< 0.1): decrease alpha (more exploitation)
5. Persist calibration state to `.roko/learn/calibration.json`

**Acceptance criteria:**
- Calibration state file shows predictions vs. actuals
- High calibration error leads to increased exploration (visible in routing logs)

### Task 7.5: Surface calibration metrics in `roko learn all`

**File:** `crates/roko-cli/src/commands/learn.rs`
**Depends on:** Task 7.4

**Steps:**
1. Load calibration state from `.roko/learn/calibration.json`
2. Display calibration summary:
   ```
   Calibration (142 predictions):
     Mean error:     0.18
     Brier score:    0.14
     Overconfidence: 12% (predicted > actual)
     Underconfidence: 8% (predicted < actual)
     Alpha adjustment: +0.03 (exploring more due to miscalibration)
   ```

**Acceptance criteria:**
- `roko learn all` shows calibration metrics after sufficient observations

---

## Phase 8: Error Pattern Intelligence and Playbook Promotion

**Goal:** Error patterns are classified, stored, and feed into conductor and
routing decisions. Successful gate reflections promote to playbooks. The system
builds a library of failure modes and proven recovery strategies.

**Depends on:** Phase 3 (Conductor) + Phase 4 (Anomaly Detection).

### Task 8.1: Wire error pattern classification and storage

**File:** `crates/roko-learn/src/error_pattern_store.rs`,
`crates/roko-cli/src/run.rs`

**Steps:**
1. On gate failure, classify the error output into one of the 10 `ErrorPattern`
   variants (Compile, Test, Lint, Timeout, ToolCall, Permission, Network,
   ResourceExhausted, LogicError, Unknown)
2. Call `error_pattern_store.record(pattern, model, role, task_category)`
3. Track per-pattern frequency by model, role, and task category
4. Persist to `.roko/learn/error-patterns.json`

**Acceptance criteria:**
- After tasks with compile errors, error-patterns.json shows Compile pattern
  with correct model and frequency counts

### Task 8.2: Feed error patterns into conductor context

**File:** `crates/roko-learn/src/conductor.rs`
**Depends on:** Task 8.1, Task 3.2

**Steps:**
1. When building `ConductorState` for a retry decision, query
   `ErrorPatternStore` for the current error pattern
2. If this pattern has high frequency for the current model:
   - Bias conductor toward `SwitchModel` action
3. If this pattern has historical success with `InjectHint(ErrorDigest)`:
   - Bias conductor toward that action
4. The bias is applied through the 19-dim context vector -- the error pattern
   one-hot encoding already encodes the pattern type

**Acceptance criteria:**
- Error pattern history influences conductor action selection
- A model with recurring Compile errors gets model-switch suggestions

### Task 8.3: Feed error patterns into CascadeRouter

**File:** `crates/roko-learn/src/cascade_router.rs`
**Depends on:** Task 8.1

**Steps:**
1. Before model selection, query error pattern store for patterns associated with
   each candidate model for the current task category
2. Models with high error rates for the current task category get a penalty
   applied to their UCB score
3. Penalty = `error_rate * ERROR_PATTERN_PENALTY` (suggest 0.2)
4. This complements the bandit's own observations with structured error knowledge

**Acceptance criteria:**
- A model with 80% compile error rate for Implementation tasks gets deprioritized
  relative to a model with 20% error rate

### Task 8.4: Promote post-gate reflections to playbook candidates

**File:** `crates/roko-learn/src/post_gate_reflection.rs`,
`crates/roko-learn/src/runtime_feedback.rs`

**Steps:**
1. After recording a post-gate reflection in `LearningRuntime`, check if the
   reflection pattern has reached promotion threshold:
   - Minimum 3 successful validations of the same pattern
   - Confidence above 0.7
2. If eligible, create a `PlaybookCandidate` from the reflection:
   - Extract the action sequence that led to the gate pass
   - Set initial confidence from the reflection's validation count
3. Add candidate to `PlaybookStore` via `store.add_candidate(candidate)`
4. Log promotion: `info!("Promoted reflection to playbook candidate: '{}'", pattern)`

**Acceptance criteria:**
- Generate 5+ successful reflections with the same pattern
- Playbook candidate appears in playbook store

### Task 8.5: Surface error patterns and playbook candidates in `roko learn all`

**File:** `crates/roko-cli/src/commands/learn.rs`
**Depends on:** Task 8.1, Task 8.4

**Steps:**
1. Load error pattern store and display top patterns:
   ```
   Error Patterns (last 30 days):
     Compile:  45 occurrences (sonnet: 60%, haiku: 30%, opus: 10%)
     Test:     23 occurrences (sonnet: 50%, haiku: 40%, opus: 10%)
     Lint:     12 occurrences (haiku: 75%, sonnet: 25%)
   ```
2. Load playbook candidates and display:
   ```
   Playbook Candidates (3 pending promotion):
     "Fix compile error via import check" (4 validations, confidence: 0.82)
     "Handle test timeout with retry" (3 validations, confidence: 0.71)
   ```

**Acceptance criteria:**
- `roko learn all` shows error pattern distribution and playbook candidates

---

## Phase 9: StagingBuffer Promotion and Force-Backend Override Learning

**Goal:** Validated knowledge candidates promote to the durable store without
requiring a full dream cycle. Manual model overrides teach the routing table.

**Depends on:** Phase 5 (dream cycle) + Phase 1 (CascadeRouter).

### Task 9.1: Promote validated staging entries without dream cycle

**Issue:** I-17
**File:** `crates/roko-learn/src/runtime_feedback.rs`

**Steps:**
1. In `LearningRuntime::record_completed_run()`, after the knowledge seed append
   step, check the StagingBuffer for entries at `Validated` stage
2. For each validated entry:
   - Submit to the LightAdmissionGate for fast-path admission
   - If admitted, write to the durable KnowledgeStore
   - Mark as promoted in the StagingBuffer
3. This runs inline with normal recording -- no dream cycle required
4. Log promotions: `info!("Promoted {} validated entries to knowledge store", count)`

**Acceptance criteria:**
- A knowledge candidate that reaches Validated stage (through confirmation or
  multiple positive gate outcomes) appears in the durable store
- This happens during normal `roko run`, not requiring `roko knowledge dream run`

### Task 9.2: Wire force-backend override learning to CascadeRouter

**Issue:** UX34
**File:** `crates/roko-cli/src/run.rs`, `crates/roko-learn/src/cascade_router.rs`

**Steps:**
1. When user specifies `--model <slug>` flag (force backend), detect this as a
   manual override
2. After the overridden task completes, call
   `cascade_router.record_override(model, &routing_context, success)`
3. The router's `ForceBackendOverrideRecorder` implementation updates the static
   routing table with `OVERRIDE_LEARNING_RATE = 0.1`:
   - After N successful overrides for a pattern, the static table starts routing
     there automatically
4. Log: `info!("Override recorded: {} for pattern {:?}, success={}", model, pattern, success)`

**Acceptance criteria:**
- Use `--model opus` flag 5 times for similar tasks, all succeeding
- CascadeRouter static table shows increased weight for opus in that pattern
- After sufficient overrides, router selects opus without `--model` flag

### Task 9.3: Pareto frontier active use in UCB model selection

**File:** `crates/roko-learn/src/cascade_router.rs`

**Steps:**
1. The Pareto frontier is already computed and cached in CascadeRouter
   (`pareto_recompute_interval` triggers recalculation)
2. During UCB scoring in the live selection path, call `pareto_adjusted_alpha()`
   to down-weight dominated models
3. Dominated models (worse on both cost and quality than another model) get reduced
   exploration incentive
4. This is already implemented in helper functions -- wire the call into the live
   `select_for_frequency_among()` method

**Acceptance criteria:**
- After sufficient observations, a dominated model receives less exploration
  than a Pareto-optimal model
- Pareto frontier is visible in `roko learn router` output

---

## Dependency Graph

```
Phase 0 (Universal Feedback)
  |
  +-- Phase 1 (CascadeRouter Live Selection)
  |     |
  |     +-- Phase 2 (Budget Enforcement)
  |     |
  |     +-- Phase 6 (Knowledge Feedback & Routing Intelligence)
  |     |     |
  |     |     +-- Phase 9 (StagingBuffer & Override Learning)
  |     |
  |     +-- Phase 3 (Conductor Integration)
  |           |
  |           +-- Phase 8 (Error Patterns & Playbook Promotion)
  |
  +-- Phase 4 (Anomaly Detection & Regression Alerting)
  |
  +-- Phase 5 (Dream Consolidation Triggers)
        |
        +-- Phase 7 (Experiment Automation & Calibration)
```

Phase 0 is the foundation. Phases 1, 4, and 5 can begin in parallel after Phase 0.
Later phases build on their dependencies as shown.

---

## Verification Matrix

| Task | What | How to Verify | Success Criteria |
|---|---|---|---|
| 0.1-0.3 | Chat feedback | `roko chat` + check efficiency.jsonl | ModelCall and WorkflowComplete records |
| 0.4 | Inline chat feedback | `roko chat --inline` + check efficiency.jsonl | Same records as interactive |
| 0.5-0.6 | ACP feedback | ACP completion + check efficiency.jsonl | ModelCall + GateResult records |
| 0.7 | Router attachment | `roko run` + check cascade-router.json | Observation count increases |
| 0.8 | Single episode write | `roko run` + count episodes | Exactly one episode per run |
| 0.9 | dispatch_v2 feedback | dispatch_v2 path + check efficiency.jsonl | Records present |
| 1.1 | Router model selection | `roko run` + check logs | Routing decision logged |
| 1.2 | Full routing context | Compare context vectors across runs | Differentiated features |
| 1.3 | Section weights | 50+ tasks + check assembly | Token budgets adjusted by lift |
| 1.4 | Provider health | Simulated provider failure | Provider excluded from routing |
| 2.1-2.3 | Budget enforcement | Set low budget + run task | Task blocked or downtiered |
| 2.4 | Cost aggregation | Multi-session + aggregate | Daily total correct |
| 2.5 | Budget in status | `roko status` | Utilization percentages shown |
| 3.1-3.3 | Conductor | Failing tasks + check conductor.json | Non-Continue actions selected |
| 3.4 | Conductor display | `roko learn all` | Action distribution shown |
| 4.1-4.2 | Anomaly detection | Prompt loop scenario | Warning logged at 5+ repeats |
| 4.3 | Chat anomalies | Chat cost spike | Warning in chat session |
| 4.4-4.5 | Regressions | Pass then fail sequence | Alert in logs and status |
| 5.1-5.2 | Dream auto-trigger | `roko serve` + wait for cron | Dream report written |
| 5.3 | Dream on plan complete | Complete a plan | Background dream triggered |
| 5.4 | Dream in status | `roko status` | Last cycle info shown |
| 6.1-6.2 | Knowledge scoring | Tasks with knowledge entries | Scores in knowledge-scores.json |
| 6.3 | Score-weighted retrieval | Entries with varied scores | Higher-scored entries rank higher |
| 6.4 | Dream routing advice | Dream cycle + tasks | Advice influences routing |
| 6.5 | Advice feedback | Routing outcomes | Advice entries get scored |
| 7.1 | Winner auto-apply | Conclude experiment + restart | Winner config applied |
| 7.2-7.3 | Experiment proposals | Dream cycle + activate | New experiments in store |
| 7.4-7.5 | Calibration | Predict + observe | Calibration metrics in learn output |
| 8.1 | Error patterns | Gate failures | Patterns in error-patterns.json |
| 8.2-8.3 | Patterns -> decisions | Patterns + routing/conductor | Patterns influence selection |
| 8.4 | Reflection promotion | Repeated pattern success | Playbook candidate created |
| 8.5 | Patterns display | `roko learn all` | Patterns and candidates shown |
| 9.1 | Staging promotion | Validated entry | Entry in durable store |
| 9.2 | Override learning | `--model` flag repeated | Static table updated |
| 9.3 | Pareto frontier | Sufficient observations | Dominated models deprioritized |

---

## File Index

All source files modified by this plan:

| File | Tasks |
|---|---|
| `crates/roko-cli/src/chat_session.rs` | 0.1, 0.2, 0.3, 4.3 |
| `crates/roko-cli/src/chat_inline.rs` | 0.4 |
| `crates/roko-acp/src/runner.rs` | 0.5 |
| `crates/roko-acp/src/pipeline.rs` | 0.6 |
| `crates/roko-cli/src/run.rs` | 0.7, 0.8, 1.1, 1.2, 1.4, 2.2, 2.3, 3.1, 3.2, 3.3, 4.1, 4.2, 4.4, 6.1, 6.4, 6.5, 7.4, 8.1, 9.2 |
| `crates/roko-cli/src/dispatch_v2.rs` | 0.9 |
| `crates/roko-cli/src/config.rs` | 2.1 |
| `crates/roko-cli/src/commands/mod.rs` | 2.5, 4.5, 5.4 |
| `crates/roko-cli/src/commands/learn.rs` | 3.4, 7.3, 7.5, 8.5 |
| `crates/roko-cli/src/daemon.rs` | 5.2 |
| `crates/roko-learn/src/feedback_service.rs` | 0.1-0.9, 6.2 |
| `crates/roko-learn/src/cascade_router.rs` | 0.7, 1.1, 1.4, 6.4, 8.3, 9.2, 9.3 |
| `crates/roko-learn/src/runtime_feedback.rs` | 1.2, 9.1 |
| `crates/roko-learn/src/budget.rs` | 2.2, 2.3 |
| `crates/roko-learn/src/costs_db.rs` | 2.4 |
| `crates/roko-learn/src/conductor.rs` | 3.1, 3.2, 3.3, 8.2 |
| `crates/roko-learn/src/anomaly.rs` | 4.1, 4.2, 4.3 |
| `crates/roko-learn/src/regression.rs` | 4.4 |
| `crates/roko-learn/src/section_effect.rs` | 1.3 |
| `crates/roko-learn/src/prompt_experiment.rs` | 7.1, 7.2 |
| `crates/roko-learn/src/calibration_policy.rs` | 7.4 |
| `crates/roko-learn/src/error_pattern_store.rs` | 8.1, 8.2, 8.3 |
| `crates/roko-learn/src/post_gate_reflection.rs` | 8.4 |
| `crates/roko-learn/src/provider_health.rs` | 1.4 |
| `crates/roko-compose/src/prompt_assembly_service.rs` | 1.3, 6.1, 6.3 |
| `crates/roko-neuro/src/knowledge_store.rs` | 6.2, 6.3, 6.5 |
| `crates/roko-core/src/config/serve.rs` | 5.1 |
| `crates/roko-serve/src/runtime.rs` | 5.2 |
| `crates/roko-dreams/src/cycle.rs` | 7.2 |
| `crates/roko-dreams/src/runner.rs` | 5.2, 5.3 |
| `crates/roko-dreams/src/routing_advice.rs` | 6.4 |

---

## Effort Estimates

| Phase | Tasks | Effort | Priority |
|---|---|---|---|
| Phase 0: Universal Feedback Wiring | 0.1 - 0.9 | 2-3 days | P0 |
| Phase 1: CascadeRouter Live Selection | 1.1 - 1.4 | 2-3 days | P0 |
| Phase 2: Budget Enforcement | 2.1 - 2.5 | 2 days | P1 |
| Phase 3: Conductor Integration | 3.1 - 3.4 | 2-3 days | P1 |
| Phase 4: Anomaly Detection & Regression | 4.1 - 4.5 | 2 days | P1 |
| Phase 5: Dream Consolidation Triggers | 5.1 - 5.4 | 2 days | P2 |
| Phase 6: Knowledge Feedback & Routing | 6.1 - 6.5 | 3-4 days | P2 |
| Phase 7: Experiment Automation & Calibration | 7.1 - 7.5 | 3-4 days | P2 |
| Phase 8: Error Patterns & Playbook Promotion | 8.1 - 8.5 | 2-3 days | P2 |
| Phase 9: StagingBuffer & Override Learning | 9.1 - 9.3 | 1-2 days | P3 |
| **Total** | **44 tasks** | **~20-27 days** | |
