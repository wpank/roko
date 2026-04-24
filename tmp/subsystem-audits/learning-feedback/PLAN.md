# Learning & Feedback: Implementation Plan

## Overview

This plan wires the existing learning subsystem to all live entry points and closes the
critical feedback gaps identified in ISSUES.md. The work is organized into 5 phases,
each building on the previous. Every phase produces measurable outcomes.

---

## Phase 0: Universal Feedback Wiring (P0, 2-3 days)

**Goal:** Every model call, regardless of entry point, records learning signals.

### 0.1 Wire FeedbackService to `roko chat`

**Issue:** I-01
**Files to modify:**
- `crates/roko-cli/src/chat_session.rs` -- add FeedbackService initialization
- `crates/roko-cli/src/chat_inline.rs` -- add FeedbackService initialization

**Steps:**
1. In chat session setup, create `FeedbackService::from_roko_dir_with_episodes(&roko_dir)`
2. After each model response, emit `FeedbackEvent::ModelCall` with:
   - `model`: current model slug
   - `role`: "chat"
   - `input_tokens`, `output_tokens`, `cost_usd`: from provider response
   - `latency_ms`: from wall clock
   - `success`: true (model responded)
   - `prompt_section_ids`: empty (chat has no structured prompt)
   - `knowledge_ids`: empty initially (future: knowledge-augmented chat)
3. On session end, call `feedback_service.flush_async().await`
4. Optional: emit `WorkflowComplete` on session close with aggregate cost/tokens

**Verification:** Run `roko chat`, have a conversation, check `.roko/learn/efficiency.jsonl`
has `model_call` records.

### 0.2 Wire FeedbackService to ACP

**Issue:** I-02
**Files to modify:**
- `crates/roko-acp/src/runner.rs` -- add FeedbackService initialization
- `crates/roko-acp/src/pipeline.rs` -- emit events after model calls and gates

**Steps:**
1. Create FeedbackService in ACP runner initialization
2. After each ACP model dispatch, emit `FeedbackEvent::ModelCall`
3. After each ACP gate run, emit `FeedbackEvent::GateResult` (currently only writes
   adaptive thresholds -- add event emission alongside)
4. Store FeedbackService reference in pipeline state for gate result access
5. Flush on pipeline completion

**Verification:** Run ACP with a code completion, check `.roko/learn/efficiency.jsonl`
has records.

### 0.3 Attach CascadeRouter to FeedbackService

**Issue:** I-03 (partial)
**Files to modify:**
- `crates/roko-cli/src/run.rs` -- attach router to FeedbackService
- `crates/roko-cli/src/dispatch_direct.rs` -- use FeedbackService for model call recording

**Steps:**
1. In `roko run` initialization, load CascadeRouter from `.roko/learn/cascade-router.json`
2. Attach to FeedbackService via `service.with_cascade_router(router)`
3. Now every model call automatically updates the router's bandit state
4. Remove separate router update logic in `record_completed_run()` (now redundant)

**Verification:** Run `roko run "test prompt"`, check CascadeRouter observation count
increases.

### 0.4 Deduplicate Episode Writes

**Issue:** I-07
**Files to modify:**
- `crates/roko-cli/src/run.rs` -- remove direct `append_episode_log()` call

**Steps:**
1. Remove the direct episode write at run.rs (append_episode_log)
2. Ensure LearningRuntime's episode write path covers all needed fields
3. Verify episode count in `.roko/learn/episodes.jsonl` matches expected

**Verification:** Run `roko run`, check only one episode appears per execution.

---

## Phase 1: Routing and Cost Intelligence (P1, 3-4 days)

**Goal:** Model selection uses learned state. Cost is enforced.

### 1.1 Full RoutingContext in `roko run`

**Issue:** I-04
**Files to modify:**
- `crates/roko-cli/src/run.rs` -- build full RoutingContext
- `crates/roko-learn/src/runtime_feedback.rs` -- accept RoutingContext in CompletedRunInput

**Steps:**
1. Add `routing_context: Option<RoutingContext>` to `CompletedRunInput`
2. In `roko run`, construct `RoutingContext` with:
   - `task_category`: derive from prompt analysis or default to Implementation
   - `complexity`: derive from prompt length or default to Standard
   - `role`: from agent config or Implementer
   - `crate_familiarity`: from episode history (count of past successes in same context)
   - `has_prior_failure`: from retry state
   - `conductor_load`: 0.0 for single-run (accurate)
   - `daimon_policy`: load from `.roko/daimon/affect.json` if exists
3. In LearningRuntime, use provided RoutingContext instead of deriving from episode

**Verification:** Run `roko run` twice with different prompts, check routing observations
in cascade-router.json have different context vectors.

### 1.2 Budget Enforcement

**Issue:** I-05
**Files to modify:**
- `crates/roko-cli/src/run.rs` -- add budget check before dispatch
- `crates/roko-cli/src/config.rs` -- load budget config
- `crates/roko-learn/src/budget.rs` -- add cross-session initialization

**Steps:**
1. Load budget config from `roko.toml`: `budget.max_task_usd`, `budget.max_session_usd`
2. Instantiate `BudgetGuardrail` at session start
3. After CostsDb aggregate for today, initialize `day_spent`
4. Before each model dispatch, check `guardrail.record_cost(estimated_cost, "task")`
5. On `BudgetAction::RouteToCheaper`: bias CascadeRouter to lower tier
6. On `BudgetAction::Block`: fail the dispatch with budget-exceeded error
7. Log budget warnings at WARN level

**Verification:** Set `budget.max_task_usd = 0.001` in roko.toml, run a task, verify
it blocks or downtiers.

### 1.3 CascadeRouter Model Selection in `roko run`

**Issue:** I-03 (continued)
**Files to modify:**
- `crates/roko-cli/src/run.rs` -- use CascadeRouter for model selection
- `crates/roko-cli/src/dispatch_direct.rs` -- accept model override from router

**Steps:**
1. Before dispatch, call `cascade_router.select_for_frequency_among(&ctx, &candidates)`
2. Use selected model slug instead of config default
3. Fall back to config default if router returns no candidate
4. Log routing decision with stage (Static/Confidence/UCB) and score

**Verification:** Run 60+ tasks, observe CascadeRouter transitions from Static to Confidence
stage in `.roko/learn/cascade-router.json`.

### 1.4 Section Effectiveness -> Prompt Assembly

**Issue:** I-09
**Files to modify:**
- `crates/roko-compose/src/prompt_assembly_service.rs` -- read section weights
- `crates/roko-cli/src/run.rs` -- thread weights through

**Steps:**
1. Load `FeedbackService.section_effectiveness()` weights
2. Pass to `PromptAssemblyService` as `section_weights` parameter
3. During assembly, multiply section token budget by weight
4. Log sections with weight < 0.7 (being deprioritized)
5. Log sections with weight > 1.3 (being boosted)

**Verification:** After 50+ tasks, check that sections with negative lift have reduced
token allocation in assembled prompts.

---

## Phase 2: Learned Intervention and Detection (P1-P2, 3-4 days)

**Goal:** Retry decisions are learned. Anomalies are detected. Regressions are surfaced.

### 2.1 Conductor Integration

**Issue:** I-06
**Files to modify:**
- `crates/roko-cli/src/run.rs` -- add conductor bandit to retry loop
- `crates/roko-learn/src/conductor.rs` -- verify persistence path

**Steps:**
1. Load `ConductorBandit::load_or_new(&conductor_path)` at session start
2. On task failure, build `ConductorState` from:
   - `iteration`: current retry count
   - `consecutive_failures`: count
   - `error_pattern`: classify from gate error output (Compile, Test, ToolCall, etc.)
   - `elapsed_ms`: wall clock since task start
   - `cost_so_far_usd`: from CostsDb or running total
   - `model_tier`: from current model
   - `task_complexity`: from routing context
3. Call `bandit.select_action(&state)` to get ConductorAction
4. Execute the action:
   - `Continue`: retry as normal
   - `InjectHint(ErrorDigest)`: add error summary to next prompt
   - `InjectHint(SkillSuggestion)`: query skill library, inject match
   - `InjectHint(SimplifyApproach)`: add simplification directive
   - `SwitchModel`: request different model from CascadeRouter
   - `Restart`: reset task state, retry from clean start
   - `Abort`: mark task as failed, stop retrying
5. After outcome, call `bandit.observe(&state, action, reward)`
6. Save bandit state

**Verification:** Run a task that fails, verify conductor JSON is updated with observation
count > 0. Run 20+ failing tasks, verify conductor starts selecting non-Continue actions.

### 2.2 Anomaly Detection Wiring

**Issue:** I-14
**Files to modify:**
- `crates/roko-cli/src/run.rs` -- create AnomalyDetector, check on each turn
- `crates/roko-cli/src/chat_session.rs` -- create AnomalyDetector for chat

**Steps:**
1. Create `AnomalyDetector::new(session_start_ms)` at session start
2. After each model call, check:
   - `detector.check_prompt(prompt_hash)` -> warn on loops
   - `detector.check_cost(cost_usd)` -> warn on spikes
   - `detector.check_quality(gate_pass_rate)` -> warn on degradation
3. On `Anomaly::PromptLoop`: log warning, consider aborting
4. On `Anomaly::CostSpike`: log warning, trigger budget pressure
5. On `Anomaly::QualityDrift`: log warning, consider model switch

**Verification:** Deliberately create a prompt loop (repeat same input 10x), verify
warning is logged.

### 2.3 Regression Alerting

**Issue:** I-13
**Files to modify:**
- `crates/roko-cli/src/run.rs` -- surface regression reports
- `crates/roko-cli/src/commands/mod.rs` -- add regression output to `roko status`

**Steps:**
1. After `record_completed_run()`, check `learning_update.regression_report`
2. If report contains alerts with severity `Alert`:
   - Log at WARN level: "Regression detected: {metric} dropped from {baseline} to {current}"
   - If pass_rate_drop: consider switching model tier
   - If cost_increase: consider enabling budget pressure
3. Surface in `roko status` output under "Regressions" section
4. Surface in `roko learn all` output

**Verification:** Create a baseline of 10 passing tasks, then 5 failing tasks. Verify
regression alert appears in `roko status`.

### 2.4 Provider Health -> CascadeRouter

**Issue:** I-19
**Files to modify:**
- `crates/roko-learn/src/cascade_router.rs` -- filter by provider health
- `crates/roko-cli/src/run.rs` -- load provider health state

**Steps:**
1. Load ProviderHealthTracker from `.roko/learn/provider-health.json`
2. Pass to CascadeRouter during initialization
3. Before UCB scoring, filter out models whose provider circuit is open
4. After model call failure, update provider health tracker
5. Log when a provider is circuit-broken

**Verification:** Simulate provider failure (set invalid API key for one provider), verify
router stops selecting models from that provider.

---

## Phase 3: Knowledge Integration (P2, 4-5 days)

**Goal:** Durable knowledge informs dispatch. The dream cycle runs automatically.

### 3.1 Knowledge-Informed Model Routing

**Issue:** I-10
**Files to modify:**
- `crates/roko-learn/src/cascade_router.rs` -- load dream routing advice
- `crates/roko-dreams/src/routing_advice.rs` -- verify advice generation
- `crates/roko-cli/src/run.rs` -- load advice at startup

**Steps:**
1. At router initialization, load `DreamRoutingAdvice` from
   `.roko/learn/dream-routing-advice.json` (path from `routing_advice.rs`)
2. Apply `dream_advice_to_routing_bias()` to convert advice to `RoutingBias`
3. Feed bias into CascadeRouter model selection
4. After routing outcome, feed back to knowledge store via FeedbackService

**Verification:** Run dream cycle, check advice file is created. Run tasks, verify
router considers advice in model selection.

### 3.2 Dream Cycle Automatic Trigger

**Issue:** I-08
**Files to modify:**
- `crates/roko-serve/src/runtime.rs` -- spawn dream loop as background task
- `crates/roko-cli/src/daemon.rs` -- include dream loop in daemon lifecycle

**Steps:**
1. Add `[dreams]` config section to `roko.toml`:
   ```toml
   [dreams]
   enabled = true
   schedule = "0 0 3 * * *"  # 3 AM daily
   budget_usd = 0.10
   model = "claude-haiku-3-5"
   ```
2. In `roko serve`, spawn `DreamRunner::start()` with cron trigger
3. In `roko daemon`, include dream loop in managed background tasks
4. Log dream cycle start/completion with report summary
5. Surface dream status in `roko status`

**Verification:** Start `roko serve`, wait for cron trigger (or set schedule to "every 5 min"
for testing), verify dream report is written.

### 3.3 Knowledge Feedback Scoring Integration

**Files to modify:**
- `crates/roko-cli/src/run.rs` -- record knowledge usage after dispatch
- `crates/roko-compose/src/prompt_assembly_service.rs` -- track last_knowledge_ids

**Steps:**
1. After prompt assembly, capture `assembler.last_knowledge_ids()`
2. After gate result, call `feedback_service.record_knowledge_usage(run_id, knowledge_ids, passed, model)`
3. On next assembly, knowledge scores influence retrieval ranking
4. Entries with consistently negative scores are eventually deprioritized

**Verification:** Add a knowledge entry, run tasks that use it, check
`.roko/learn/knowledge-scores.json` shows accumulating score.

### 3.4 StagingBuffer Promotion Without Full Dream Cycle

**Issue:** I-17
**Files to modify:**
- `crates/roko-learn/src/runtime_feedback.rs` -- add lightweight promotion check

**Steps:**
1. In `LearningRuntime::record_completed_run()`, after knowledge seed append:
2. Check StagingBuffer for entries at Validated stage
3. If found, promote to KnowledgeStore directly
4. This runs inline with normal recording, no dream cycle needed

**Verification:** Generate a knowledge candidate that reaches Validated stage, verify
it appears in the durable store without running a dream cycle.

---

## Phase 4: Experiment Automation and Advanced Learning (P2-P3, 5-7 days)

**Goal:** The system proposes and concludes its own experiments. Advanced learning
mechanisms are active.

### 4.1 Experiment Winner Auto-Application

**Issue:** I-11
**Files to modify:**
- `crates/roko-learn/src/prompt_experiment.rs` -- add auto_apply method
- `crates/roko-cli/src/run.rs` -- load and apply winners at startup

**Steps:**
1. Add `ExperimentStore::apply_winners()` method
2. At startup, load `experiment-winners.json`
3. For each winner, apply the winning value to the corresponding config
4. Log auto-applied winners at INFO level
5. Add `auto_apply_winners: bool` to config (default true)
6. Guard with config check

**Verification:** Create and conclude an experiment (manually set stats), restart,
verify winner is auto-applied.

### 4.2 Automated Experiment Proposal

**Files to modify:**
- `crates/roko-dreams/src/cycle.rs` -- propose experiments from dream insights
- `crates/roko-learn/src/prompt_experiment.rs` -- add create_from_insight()

**Steps:**
1. During dream cycle integration phase, identify insights with actionable implications
2. For insights about prompt sections: create experiment with current text vs. insight-derived text
3. For insights about model selection: create model experiment
4. Use InsightRecord confidence as initial prior for UCB exploration
5. Log proposed experiments for operator review

**Verification:** Run dream cycle with sufficient episodes, check new experiments appear
in experiments.json.

### 4.3 Error Pattern Store Integration

**Files to modify:**
- `crates/roko-learn/src/error_pattern_store.rs` -- persistence
- `crates/roko-learn/src/conductor.rs` -- consume error patterns

**Steps:**
1. On gate failure, classify error and store in `error_pattern_store`
2. Track frequency of each error pattern per model, role, task category
3. Feed high-frequency patterns into conductor context
4. Feed into CascadeRouter: models with high error rates for specific patterns get deprioritized
5. Expose error pattern stats in `roko learn all`

**Verification:** Run tasks that consistently fail with the same error pattern, verify
pattern store accumulates counts.

### 4.4 Post-Gate Reflection -> Playbook Candidates

**Files to modify:**
- `crates/roko-learn/src/post_gate_reflection.rs` -- wire promotion path
- `crates/roko-learn/src/runtime_feedback.rs` -- trigger promotion check

**Steps:**
1. After recording post-gate reflection, check promotion eligibility
2. `ReflectionPromotionConfig` defines thresholds (min confidence, min validations)
3. Promoted reflections become playbook candidates
4. Candidates with sufficient success count become full playbooks
5. Log promotions

**Verification:** Generate 5+ successful reflections for the same pattern, verify
playbook candidate is created.

### 4.5 Forensic Replay API Wiring

**Files to modify:**
- `crates/roko-learn/src/forensic_replay.rs` -- expose via CLI
- `crates/roko-cli/src/commands/mod.rs` -- add `roko learn replay` command

**Steps:**
1. Add `roko learn replay <episode-id>` command
2. Load episode from log, reconstruct decision context
3. Show: model selected, routing stage, conductor state, budget state
4. Show: what the system would choose now vs. what it chose then
5. Surface counterfactual: "with current router state, this task would have used {model}"

**Verification:** Run a task, then `roko learn replay <id>`, verify decision context
is displayed.

---

## Phase 5: Continuous Optimization (P3, ongoing)

**Goal:** The system continuously improves without human intervention.

### 5.1 Calibration Policy Loop

**Files to modify:**
- `crates/roko-learn/src/calibration_policy.rs` -- wire to runtime
- `crates/roko-learn/src/runtime_feedback.rs` -- emit calibration events

**Steps:**
1. Before dispatch, publish predicted success probability from CascadeRouter
2. After gate, record actual outcome
3. Compute calibration error: `|predicted - actual|`
4. Feed calibration error into alpha schedule: high error -> higher exploration
5. Dashboard: plot calibration curve (predicted vs. actual)

### 5.2 Cross-Session Cost Aggregation

**Issue:** I-12
**Files to modify:**
- `crates/roko-learn/src/costs_db.rs` -- add aggregate_since()
- `crates/roko-cli/src/commands/learn.rs` -- add cost summary output

**Steps:**
1. Add `CostsDb::aggregate_since(since: DateTime<Utc>) -> CostAggregate`
2. Show daily/weekly/monthly cost breakdowns in `roko learn efficiency`
3. Initialize BudgetGuardrail.day_spent from today's aggregate

### 5.3 Pareto Frontier Active Use

**Files to modify:**
- `crates/roko-learn/src/cascade_router.rs` -- activate Pareto in live selection

**Steps:**
1. Pareto frontier is already computed and cached in CascadeRouter
2. During UCB scoring, apply `pareto_adjusted_alpha()` to down-weight dominated models
3. This is already implemented in helpers -- just needs to be called from the live path

### 5.4 Force-Backend Override Learning

**Files to modify:**
- `crates/roko-learn/src/cascade_router.rs` -- wire override recording
- `crates/roko-cli/src/run.rs` -- detect manual model overrides

**Steps:**
1. When user specifies `--model` flag (force backend), detect it as an override
2. Call `cascade_router.record_override_outcome(model, &ctx, success)`
3. Router updates static table with `OVERRIDE_LEARNING_RATE`
4. After N successful overrides for a pattern, router starts there automatically

### 5.5 Curriculum Ordering

**Files to modify:**
- `crates/roko-learn/src/curriculum.rs` -- activate curriculum in plan execution

**Steps:**
1. Before plan execution, sort tasks by curriculum ordering
2. Start with simpler tasks (lower complexity band) to build routing signal
3. Graduate to complex tasks once router has confidence
4. This reduces early waste from cold-start routing on complex tasks

---

## Verification Matrix

| Phase | What | How to Verify | Success Criteria |
|---|---|---|---|
| 0.1 | Chat feedback | `roko chat` + check efficiency.jsonl | ModelCall records in log |
| 0.2 | ACP feedback | ACP completion + check efficiency.jsonl | ModelCall + GateResult records |
| 0.3 | Router attachment | `roko run` + check cascade-router.json | Observation count increases |
| 0.4 | Single episode write | `roko run` + count episodes | Exactly one episode per run |
| 1.1 | Full routing context | Compare context vectors across runs | Different prompts yield different vectors |
| 1.2 | Budget enforcement | Set low budget + run task | Task blocked or downtired |
| 1.3 | Router selection | 60+ tasks + check stage transitions | Static -> Confidence transition |
| 1.4 | Section weights | 50+ tasks + check prompt assembly | Token budgets adjusted by lift |
| 2.1 | Conductor | Failing tasks + check conductor.json | Non-Continue actions selected |
| 2.2 | Anomaly detection | Prompt loop + cost spike scenarios | Warnings logged |
| 2.3 | Regression alerting | Pass then fail sequence | Regression alert in status |
| 2.4 | Provider health | Provider failure + check routing | Provider excluded from selection |
| 3.1 | Knowledge routing | Dream cycle + task execution | Advice influences model selection |
| 3.2 | Dream auto-trigger | Start serve, wait for cron | Dream report written |
| 3.3 | Knowledge scoring | Tasks with knowledge entries | Scores in knowledge-scores.json |
| 4.1 | Winner auto-apply | Conclude experiment + restart | Winner config applied |
| 4.2 | Experiment proposal | Dream cycle with episodes | New experiments in store |

---

## Dependencies

```
Phase 0 (Foundation)
  |
  +-- Phase 1 (Routing & Cost)
  |     |
  |     +-- Phase 2 (Intervention & Detection)
  |     |     |
  |     |     +-- Phase 4 (Experiment Automation)
  |     |
  |     +-- Phase 3 (Knowledge Integration)
  |           |
  |           +-- Phase 5 (Continuous Optimization)
  |
  +-- Phase 0.3 and 0.4 are independent of Phase 1
```

Phase 0 is the foundation -- everything depends on universal feedback wiring.
Phases 1-3 can partially overlap. Phase 4 depends on Phases 1 and 2. Phase 5 is ongoing.

---

## File Index

All files referenced in this plan:

| File | Phases |
|---|---|
| `crates/roko-cli/src/run.rs` | 0.3, 0.4, 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 2.4, 3.3, 5.4 |
| `crates/roko-cli/src/chat_session.rs` | 0.1, 2.2 |
| `crates/roko-cli/src/chat_inline.rs` | 0.1 |
| `crates/roko-cli/src/dispatch_direct.rs` | 0.3, 1.3 |
| `crates/roko-cli/src/config.rs` | 1.2 |
| `crates/roko-cli/src/commands/mod.rs` | 2.3, 4.5 |
| `crates/roko-cli/src/commands/learn.rs` | 5.2 |
| `crates/roko-cli/src/daemon.rs` | 3.2 |
| `crates/roko-acp/src/runner.rs` | 0.2 |
| `crates/roko-acp/src/pipeline.rs` | 0.2 |
| `crates/roko-learn/src/feedback_service.rs` | 0.1, 0.2, 0.3 |
| `crates/roko-learn/src/runtime_feedback.rs` | 1.1, 3.4, 5.1 |
| `crates/roko-learn/src/cascade_router.rs` | 1.3, 2.4, 3.1, 5.3, 5.4 |
| `crates/roko-learn/src/budget.rs` | 1.2, 5.2 |
| `crates/roko-learn/src/conductor.rs` | 2.1 |
| `crates/roko-learn/src/anomaly.rs` | 2.2 |
| `crates/roko-learn/src/regression.rs` | 2.3 |
| `crates/roko-learn/src/provider_health.rs` | 2.4 |
| `crates/roko-learn/src/prompt_experiment.rs` | 4.1, 4.2 |
| `crates/roko-learn/src/section_effect.rs` | 1.4 |
| `crates/roko-learn/src/error_pattern_store.rs` | 4.3 |
| `crates/roko-learn/src/post_gate_reflection.rs` | 4.4 |
| `crates/roko-learn/src/forensic_replay.rs` | 4.5 |
| `crates/roko-learn/src/calibration_policy.rs` | 5.1 |
| `crates/roko-learn/src/costs_db.rs` | 5.2 |
| `crates/roko-learn/src/curriculum.rs` | 5.5 |
| `crates/roko-compose/src/prompt_assembly_service.rs` | 1.4, 3.3 |
| `crates/roko-neuro/src/knowledge_store.rs` | 3.1, 3.3 |
| `crates/roko-dreams/src/cycle.rs` | 4.2 |
| `crates/roko-dreams/src/runner.rs` | 3.2 |
| `crates/roko-dreams/src/routing_advice.rs` | 3.1 |
| `crates/roko-dreams/src/staging.rs` | 3.4 |
| `crates/roko-serve/src/runtime.rs` | 3.2 |
