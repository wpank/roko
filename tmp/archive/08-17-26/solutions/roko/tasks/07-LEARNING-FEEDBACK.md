# Learning & Feedback Subsystem: Task Breakdown

> Wire every model-calling entry point to emit learning signals. Close open
> feedback loops: routing observations flow back to model selection, anomalies
> trigger interventions, knowledge scores influence prompt assembly, budget
> guardrails enforce cost limits, and the dream cycle runs automatically.
>
> Sources: `impl/07-LEARNING-FEEDBACK.md`, `18-LEARN-{AUDIT,ISSUES,PLAN}.md`,
> codebase analysis

---

## Overview

The learning subsystem spans 70+ source files across three crates (`roko-learn`,
`roko-neuro`, `roko-dreams`) and implements a multi-timescale feedback
architecture. Ten primary learning components form a closed loop: episode logger,
CascadeRouter (LinUCB bandit), efficiency events, prompt experiments (A/B),
model experiments, playbook store, conductor bandit, budget guardrails, adaptive
gate thresholds, and cost tracking. All are fully built, persisted, and tested.

The critical finding: the complete closed learning loop exists only in
`orchestrate.rs` (22K+ LOC dead code). The three live entry points record
varying amounts:

| Entry Point | What it Records | Gap |
|---|---|---|
| `roko run` (`run.rs`) | Episodes (dual write), cost records, routing observations (simplified context), playbook/experiment/regression/provider health/efficiency | No FeedbackService, no budget, no conductor, no anomaly, no CascadeRouter selection, no section effectiveness |
| `roko chat` (`chat_session.rs`) | Nothing | Total blind spot |
| ACP (`roko-acp/runner.rs`) | Adaptive gate thresholds only (rungs 0/1/2) | No episodes, no routing, no cost tracking |
| `dispatch_v2.rs` | FeedbackService instantiated, emits via ModelCallService | Partial -- only dispatch path, no gate/workflow events |

**Current state of key infrastructure**:
- `FeedbackService` (`roko-learn/src/feedback_service.rs`): Fully built, handles ModelCall/GateResult/WorkflowComplete, provenance tracking, knowledge scoring. Has `with_cascade_router()`, `with_episode_logger()`, `from_roko_dir_with_episodes()`. Used in `dispatch_v2.rs` only.
- `LearningRuntime` (`roko-learn/src/runtime_feedback.rs`): 18-step `record_completed_run()`. Used from `run.rs` only.
- `CascadeRouter` (`roko-learn/src/cascade_router.rs`): 3-stage LinUCB bandit, `select_for_frequency_among()`. Never called from any live path for model selection.
- `BudgetGuardrail` (`roko-learn/src/budget.rs`): 3-scope budget with 5 actions. Never instantiated.
- `ConductorBandit` (`roko-learn/src/conductor.rs`): 7 actions, 19-dim context. Never invoked from live paths.
- `AnomalyDetector` (`roko-learn/src/anomaly.rs`): 3 channels (prompt loops, cost spikes, quality drift). Used in `learning_helpers.rs` + `orchestrate.rs` only.
- `SectionEffectivenessRegistry` (`roko-learn/src/section_effect.rs`): Fully built, `with_section_effectiveness()` exists on `SystemPromptBuilder` and `PromptAssemblyService`. Not called from `run.rs`.
- `DreamRunner` (`roko-dreams/src/runner.rs`): Background scheduling fully built. `start_dream_loop()` exists in `roko-serve/src/dreams.rs` with `DreamLoopConfig.auto_dream` guard. Not spawned from `roko serve` startup.
- `ProviderHealthTracker` (`roko-learn/src/provider_health.rs`): Circuit breaker logic. `LearningRuntime.healthy_model_slugs()` exists. Not connected to CascadeRouter in live paths.

---

## Anti-Patterns to Remove

| ID | Anti-Pattern | Where | Severity |
|---|---|---|---|
| AP-BLIND | `roko chat` records zero learning signals | `crates/roko-cli/src/chat_session.rs` | Critical |
| AP-ACPBLIND | ACP records only gate thresholds, no episodes/routing/cost | `crates/roko-acp/src/runner.rs` | Critical |
| AP-DEADLOOP | Full learning loop only in dead code | `crates/roko-cli/src/orchestrate.rs` (22K LOC) | Critical |
| AP-DUAL | Dual episode writes in `roko run` | `run.rs` writes to `.roko/episodes.jsonl` directly AND via `LearningRuntime` to `.roko/learn/episodes.jsonl` | Medium |
| AP-NOBUDGET | No budget enforcement in any live path | `BudgetGuardrail` never instantiated | High |
| AP-NOCONDUCTOR | No conductor intervention in live paths; retry decisions hardcoded | `ConductorBandit` never invoked | High |
| AP-IMPOVERISHED | Simplified routing context in `roko run` (9 of 18 features zeroed) | `run.rs` -> `CompletedRunInput::from_episode()` | High |
| AP-NOANOMALY | Anomaly detector not wired to live paths | Only in `learning_helpers.rs` (not called from `run.rs` or `chat_session.rs`) | Medium |
| AP-NOHEALTH | Provider health circuit breaker not connected to CascadeRouter | `ProviderHealthRegistry` parameter exists but not wired | Medium |
| AP-NOSECTION | Section effectiveness collected but never used for prompt assembly | `run.rs` does not load or pass `SectionEffectivenessRegistry` | Medium |
| AP-NODREAM | Dream cycle has no automatic runtime trigger | `start_dream_loop` exists but never called from serve startup | Medium |

---

## Phase 1: Universal Feedback Wiring

Everything downstream depends on all entry points emitting `FeedbackEvent`s.

### Task 7.1: Wire FeedbackService to `roko chat` Session Setup
**Priority**: P0
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_session.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/Cargo.toml` (if `roko-learn` dep not already present for chat path)
**Depends On**: none

#### Context
`ChatAgentSession` (at `chat_session.rs:307`) is the interactive REPL session. It manages a `ClaudeCliAgent`, sends turns via `send_turn()` / `send_turn_api()` / `send_turn_streaming()`, and receives `TurnResult` (at line 260) with `input_tokens`, `output_tokens`, `cost_usd`, `duration`. It has no `FeedbackService` field.

`FeedbackService::from_roko_dir_with_episodes()` (at `feedback_service.rs:140`) creates a service with an `EpisodeLogger` attached. It accepts a `.roko` path and auto-creates the `learn/` subdirectory. The `FeedbackEvent::ModelCall` variant (at `roko-core/src/foundation.rs:200-225`) has `run_id`, `model`, `role`, `input_tokens`, `output_tokens`, `cost_usd`, `latency_ms`, `success` fields.

`chat_session.rs` currently imports nothing from `roko_learn`.

#### Implementation Steps
1. Add a `feedback: Option<Arc<FeedbackService>>` field to `ChatAgentSession` struct at line 307.
2. In `ChatAgentSession::new()` (line 341), resolve the `.roko` directory from the workdir (same pattern as `dispatch_v2.rs:62`). Create `FeedbackService::from_roko_dir_with_episodes(&roko_dir)` and store as `Some(Arc::new(svc))`.
3. Generate a `run_id: String` (UUID) per chat session, store on the struct. This groups all turns in one session.
4. Verify that `roko-learn` is in `roko-cli/Cargo.toml` dependencies (it is -- used by `run.rs` already).
5. No behavior change to chat flow yet -- just the service is created and held.

#### Verification Criteria
- [ ] `cargo check -p roko-cli` compiles
- [ ] `ChatAgentSession` struct has `feedback` field
- [ ] `FeedbackService` is created in `new()` without panicking

---

### Task 7.2: Emit ModelCall Events from `roko chat` Turns
**Priority**: P0
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_session.rs`
**Depends On**: Task 7.1

#### Context
`TurnResult` (at `chat_session.rs:260`) contains `input_tokens: u64`, `output_tokens: u64`, `cost_usd: f64`, `duration: Duration`, and `cancelled: bool`. The model slug is available from `self.model` (set during `ChatAgentSession::new()`).

After each turn completes (in `send_turn()` at line 887, `send_turn_api()` at line 489, `send_turn_oneshot()` at line 957), a `TurnResult` is returned. This is the emission point.

`FeedbackService` implements `FeedbackSink` (async trait at `foundation.rs:250`). `record()` is async. Chat sessions are async.

#### Implementation Steps
1. After each successful `TurnResult` is produced, emit `FeedbackEvent::ModelCall` with:
   - `run_id`: `Some(self.session_run_id.clone())`
   - `model`: `Some(self.model.clone())`
   - `role`: `"chat".to_string()`
   - `input_tokens`: from `turn_result.input_tokens`
   - `output_tokens`: from `turn_result.output_tokens`
   - `cost_usd`: from `turn_result.cost_usd`
   - `latency_ms`: `turn_result.duration.as_millis() as u64`
   - `success`: `!turn_result.cancelled`
   - `prompt_section_ids`: `vec![]`
   - `knowledge_ids`: `vec![]`
2. Call `self.feedback.as_ref().unwrap().record(event).await` (or `let _ = ...` to avoid panicking on feedback errors).
3. Add the emission after all three send paths (`send_turn_api`, `send_turn_oneshot`, `send_turn_streaming`). Use a helper method `emit_model_call(&self, turn: &TurnResult)` to avoid duplication.

#### Verification Criteria
- [ ] Run `roko chat`, send one message, exit
- [ ] `.roko/learn/efficiency.jsonl` contains a `model_call` record with `role: "chat"`, non-zero tokens

---

### Task 7.3: Emit WorkflowComplete on Chat Session End
**Priority**: P0
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_session.rs`
**Depends On**: Task 7.2

#### Context
Chat sessions end in multiple ways: user types `/exit`, Ctrl-C, or the REPL loop exhausts. The session struct tracks total cost and token counts across turns (or can accumulate them). `FeedbackEvent::WorkflowComplete` (at `foundation.rs:233-248`) has `event_type`, `run_id`, `model`, `success`, `total_input_tokens`, `total_output_tokens`, `total_cost_usd`, `total_latency_ms`, `gate_results`.

#### Implementation Steps
1. Add running accumulators to `ChatAgentSession`: `total_input_tokens: u64`, `total_output_tokens: u64`, `total_cost_usd: f64`, `total_latency_ms: u64`, `turn_count: u32`. Update after each turn.
2. On session end (wherever the REPL loop exits), emit `FeedbackEvent::WorkflowComplete` with:
   - `event_type`: `"chat_session"`
   - `run_id`: session UUID
   - `model`: last model used
   - `success`: true (session completed normally)
   - Accumulated totals
   - `gate_results`: empty vec (no gates in chat)
3. Call `feedback.flush()` (sync) or `feedback.flush_async().await` before the session drops.
4. Consider implementing this in a `Drop`-adjacent cleanup method since `FeedbackService::drop()` already flushes, but explicit is better.

#### Verification Criteria
- [ ] After a multi-turn chat session, `.roko/learn/efficiency.jsonl` shows one WorkflowComplete plus N ModelCall records
- [ ] Accumulated totals match sum of individual turn records

---

### Task 7.4: Wire FeedbackService to ACP Pipeline
**Priority**: P0
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/pipeline.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/Cargo.toml`
**Depends On**: none

#### Context
The ACP runner (`roko-acp/src/runner.rs`) handles editor integration (VS Code, etc.). At line 1666-1667, it defines `THRESHOLDS_PATH` and writes adaptive gate thresholds. It does not import or use `FeedbackService`. The pipeline (`pipeline.rs`) orchestrates ACP model calls and gate runs. Neither emits `FeedbackEvent`s.

`roko-acp/Cargo.toml` needs `roko-learn` as a dependency (check if already present; `roko-learn` may already be pulled transitively through `roko-core`).

#### Implementation Steps
1. Add `roko-learn` to `roko-acp/Cargo.toml` `[dependencies]` if not present.
2. In ACP runner initialization, create `FeedbackService::from_roko_dir_with_episodes(&workdir.join(".roko"))`.
3. Store the service on the runner struct or pass through the pipeline.
4. After each ACP model dispatch in `pipeline.rs`, emit `FeedbackEvent::ModelCall` with `role: "acp"`.
5. After each ACP gate run in `runner.rs` (where it currently writes only adaptive thresholds), emit `FeedbackEvent::GateResult` alongside the existing threshold write.
6. Flush on pipeline completion.

#### Verification Criteria
- [ ] `cargo check -p roko-acp` compiles
- [ ] ACP pipeline emits ModelCall events visible in `.roko/learn/efficiency.jsonl`
- [ ] Gate threshold writes still work unchanged

---

### Task 7.5: Attach CascadeRouter to FeedbackService in `roko run`
**Priority**: P0
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
**Depends On**: none

#### Context
`dispatch_v2.rs` (at line 60-89) already creates a `FeedbackService` and attaches it to `ModelCallService` via `with_feedback_sink()`. But `run.rs` does not create a `FeedbackService` at all -- it uses `LearningRuntime::record_completed_run()` (at line 2680) which handles learning after the fact.

`FeedbackService::with_cascade_router()` (at `feedback_service.rs:133`) accepts `Arc<CascadeRouter>`. When attached, every `ModelCall` event automatically updates the router's bandit state.

`CascadeRouter` persists to `.roko/learn/cascade-router.json` via `CascadeSnapshot`.

#### Implementation Steps
1. In the `roko run` initialization path, load `CascadeRouter` from `.roko/learn/cascade-router.json` (use `CascadeRouter::load_or_new(path, model_slugs)` -- find the existing constructor).
2. Create `FeedbackService::from_roko_dir_with_episodes(&roko_dir).with_cascade_router(Arc::new(router))`.
3. Store the `FeedbackService` on the run context so it can be used for model call events.
4. After the agent dispatch returns, emit `FeedbackEvent::ModelCall` with the actual usage data from the `AgentResult`.
5. After gates complete, emit `FeedbackEvent::GateResult` for each verdict.
6. At run end, emit `FeedbackEvent::WorkflowComplete`.
7. Flush the service before returning.

#### Verification Criteria
- [ ] Run `roko run "test prompt"`, check `.roko/learn/cascade-router.json` observation count increases
- [ ] `efficiency.jsonl` has ModelCall + GateResult + WorkflowComplete records

---

### Task 7.6: Deduplicate Episode Writes in `roko run`
**Priority**: P1
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
**Depends On**: Task 7.5

#### Context
`run.rs` writes episodes twice:
1. Direct call at line 1301: `append_episode_log(...)` -> `.roko/episodes.jsonl`
2. Via `LearningRuntime::record_completed_run()` at line 2680 -> `.roko/learn/episodes.jsonl`

The `append_episode_log` function is defined at line 2545 of `run.rs`. `LearningRuntime` internally calls `EpisodeLogger::append()` inside `record_completed_run()`.

#### Implementation Steps
1. Remove the direct `append_episode_log()` call at line 1301 and the function definition at line 2545.
2. Verify `LearningRuntime::record_completed_run()` writes all required episode fields that `append_episode_log` was writing (agent_id, role, model, tokens, gate verdicts, cost, HDC fingerprint).
3. If any fields are missing from `CompletedRunInput::from_episode()` (at `runtime_feedback.rs:863`), add them.
4. Update any code that reads from `.roko/episodes.jsonl` (root path) to read from `.roko/learn/episodes.jsonl` instead, or ensure `LearningRuntime` writes to the root path.
5. Remove dead imports related to the removed function.

#### Verification Criteria
- [ ] Run `roko run`, count episodes -- exactly one per execution
- [ ] Episode has all fields: gate verdicts, cost, tokens, model, HDC fingerprint
- [ ] No duplicate entries across episode log files

---

## Phase 2: Routing and Cost Intelligence

Model selection uses learned state. Cost is enforced. Section effectiveness informs prompt budget.

### Task 7.7: Build Full RoutingContext in `roko run`
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/runtime_feedback.rs`
**Depends On**: Task 7.5

#### Context
`RoutingContext` (at `model_router.rs:130`) has 18 features:
- `task_category: TaskCategory`
- `complexity: ComplexityBand`
- `iteration: u32`
- `role: AgentRole`
- `crate_familiarity: f64`
- `has_prior_failure: bool`
- `conductor_load: f64`
- `active_agents: u32`
- `ready_queue_depth: u32`
- `max_queue_wait_hours: f64`
- `daimon_policy: DaimonPolicy`
- `thinking_level: Option<ThinkingLevel>`
- `temperament: Option<f64>`
- `plan_context_tokens: Option<u32>`
- `tier_thresholds: Option<TierThresholds>`

Currently `CompletedRunInput::from_episode()` (at `runtime_feedback.rs:863`) derives a simplified context where 9 of these features are zeroed.

#### Implementation Steps
1. Add `routing_context: Option<RoutingContext>` to `CompletedRunInput` struct (at `runtime_feedback.rs:283`).
2. In `roko run`, after resolving the prompt and model, construct a `RoutingContext`:
   - `task_category`: derive from prompt analysis or agent role, default `TaskCategory::Implementation`
   - `complexity`: derive from prompt length/structure, default `ComplexityBand::Standard`
   - `iteration`: retry count (0 for first attempt)
   - `role`: from agent config or `AgentRole::Implementer`
   - `crate_familiarity`: query episode history for success rate in same context (default 0.5)
   - `has_prior_failure`: from retry state
   - `conductor_load`: 0.0 for single-run (accurate for non-orchestrated mode)
   - `active_agents`: 0 for single-run
   - `daimon_policy`: load from `.roko/daimon/affect.json` if exists, else default
3. In `LearningRuntime::record_completed_run()`, if `input.routing_context` is `Some`, use it instead of deriving from the episode.
4. Pass the context through to the CascadeRouter observation.

#### Verification Criteria
- [ ] Run `roko run` with two different prompts, check `cascade-router.json` observations have distinct context vectors
- [ ] Fields that were previously 0 now have meaningful values

---

### Task 7.8: Wire Budget Enforcement to `roko run`
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/budget.rs`
**Depends On**: Task 7.5

#### Context
`BudgetGuardrail` (at `budget.rs:8`) has 3 scope limits (`max_task_usd`, `max_session_usd`, `max_day_usd`) and returns `BudgetAction` (Ok, Warn, RouteToCheaper, BlockNewSessions, Block). It is never instantiated.

`roko.toml` supports budget config fields (see `roko-core/src/config/schema.rs` for `BudgetConfig`). `CostsDb` (at `costs_db.rs:472`) has `by_session()`, `by_model()`, `total_cost()` but no `aggregate_since()` for daily aggregation.

#### Implementation Steps
1. Add `aggregate_since(since: chrono::DateTime<Utc>) -> f64` to `CostsDb` that sums `cost_usd` for records with `timestamp >= since`.
2. In `roko run` initialization, load budget config from `Config` (check for `budget.max_task_usd`, `budget.max_session_usd`, `budget.max_day_usd` fields).
3. If any budget field is set, instantiate `BudgetGuardrail` with the configured limits.
4. Initialize `day_spent` from `CostsDb::aggregate_since(today_midnight)`.
5. Before each model dispatch, check `guardrail.record_cost(estimated_cost, "task")`:
   - `BudgetAction::Ok` or `BudgetAction::Warn`: proceed (log at WARN for Warn)
   - `BudgetAction::RouteToCheaper`: set a flag to bias CascadeRouter toward cheaper models
   - `BudgetAction::Block` or `BudgetAction::BlockNewSessions`: return error before dispatch
6. After dispatch, update guardrail with actual cost from `AgentResult`.

#### Verification Criteria
- [ ] Set `budget.max_task_usd = 0.001` in roko.toml, run a task, verify block or downgrade
- [ ] Budget warnings logged at WARN level
- [ ] `aggregate_since()` returns correct daily total

---

### Task 7.9: Wire CascadeRouter Model Selection in `roko run`
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_direct.rs`
**Depends On**: Task 7.7

#### Context
`CascadeRouter::select_for_frequency_among()` (at `cascade_router.rs:329`) takes a `RoutingContext` and candidate model slugs, returns a model slug. It transitions through 3 stages: Static (< 50 obs), Confidence (50-200), UCB (200+).

Currently `roko run` uses the model from config (`resolve_effective_model()` at `run.rs:20`). There is no router consultation.

`dispatch_direct.rs` handles the actual agent dispatch. It could accept a model override parameter.

#### Implementation Steps
1. After loading `CascadeRouter` (from Task 7.5) and building `RoutingContext` (from Task 7.7), call `router.select_for_frequency_among(&ctx, &candidate_slugs)`.
2. `candidate_slugs` = all configured model slugs from `Config.agent.models` or provider config.
3. Use the router-selected model instead of the config default, unless the user specified `--model` (force override).
4. Log the routing decision: `info!(model = %selected, stage = ?stage, "CascadeRouter selected model")`.
5. Fall back to config default if router returns no candidate or if `candidate_slugs` is empty.
6. Thread the selected model through to `dispatch_direct.rs` dispatch call.

#### Verification Criteria
- [ ] Run 60+ tasks, observe CascadeRouter `stage` transitions in logs
- [ ] `cascade-router.json` shows increasing observation counts
- [ ] User `--model` flag still overrides router selection

---

### Task 7.10: Wire Section Effectiveness to Prompt Assembly
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
**Depends On**: Task 7.5

#### Context
`SectionEffectivenessRegistry` is already wired into `SystemPromptBuilder` and `RoleSystemPromptSpec`:
- `SystemPromptBuilder::with_section_effectiveness()` (at `system_prompt_builder.rs:335`)
- `RoleSystemPromptSpec::build_with_section_effectiveness()` (at `role_prompts.rs:445`)
- `RoleSystemPromptSpec::compose_build_with_budget_and_section_effectiveness()` (at `role_prompts.rs:518`)
- `PromptAssemblyService::with_section_effectiveness()` (at `prompt_assembly_service.rs:180`)

`FeedbackService::section_effectiveness()` (at `feedback_service.rs:277`) returns `HashMap<String, f64>` with lift-based weights per section.

`SectionEffectivenessRegistry::load_or_new(path)` (at `context_provider.rs:463`) loads from `.roko/learn/section-effects.json`.

But `run.rs` never loads or passes section effectiveness data to prompt assembly.

#### Implementation Steps
1. In the prompt composition path in `run.rs` (around line 1110-1174 where the prompt is built), load `SectionEffectivenessRegistry::load_or_new(&roko_dir.join("learn/section-effects.json"))`.
2. Pass the registry to the prompt builder via `with_section_effectiveness()`.
3. If using `PromptAssemblyService`, call `.with_section_effectiveness(feedback.section_effectiveness())`.
4. Log sections with weight < 0.7 (deprioritized) and > 1.3 (boosted) at DEBUG level.

#### Verification Criteria
- [ ] After 50+ tasks with varying gate results, `section-effects.json` has entries
- [ ] Prompt assembly uses non-default section weights
- [ ] Low-effectiveness sections get reduced token allocation

---

## Phase 3: Learned Intervention and Detection

Retry decisions are learned, not hardcoded. Anomalies and regressions are detected and surfaced.

### Task 7.11: Wire Conductor Bandit to `roko run` Retry Loop
**Priority**: P1
**Estimated Effort**: 5 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
**Depends On**: Task 7.9

#### Context
`ConductorBandit` (at `conductor.rs:110`) manages retry decisions with 7 actions: Continue, InjectHint(ErrorDigest), InjectHint(SkillSuggestion), InjectHint(SimplifyApproach), SwitchModel, Restart, Abort. It uses Thompson+linear blended scoring over a 19-dimension context.

`ConductorState` (at `conductor.rs:40`) requires: `iteration`, `consecutive_failures`, `error_pattern` (one-hot for 10 ErrorPattern variants), `elapsed_ms`, `cost_so_far_usd`, `model_tier`, `task_complexity`.

`ConductorBandit::load_or_new(path)` (at `conductor.rs:146`) loads from `.roko/learn/conductor.json`.

Currently `roko run` has no retry loop -- it runs once and returns. But the `WorkflowEngine` (used from `run.rs`) supports retry via `WorkflowRunConfig`. The conductor should be consulted when the engine decides whether to retry.

#### Implementation Steps
1. Load `ConductorBandit::load_or_new(&roko_dir.join("learn/conductor.json"))` at run initialization.
2. After a task fails (gate failure or agent error), build `ConductorState`:
   - `iteration`: current retry count
   - `consecutive_failures`: count of consecutive failures
   - `error_pattern`: classify from gate error output (use `ErrorPattern` enum variants)
   - `elapsed_ms`: wall clock since task start
   - `cost_so_far_usd`: accumulated cost from `AgentResult` usage
   - `model_tier`: hash of current model slug
   - `task_complexity`: from `RoutingContext.complexity`
3. Call `bandit.select_action(&state)` to get `ConductorAction`.
4. Execute the action:
   - `Continue`: proceed with retry as normal
   - `InjectHint(ErrorDigest)`: append error summary to next prompt
   - `InjectHint(SkillSuggestion)`: query `SkillLibrary` for matching skills, inject into prompt
   - `InjectHint(SimplifyApproach)`: add simplification directive to prompt
   - `SwitchModel`: request a different model from `CascadeRouter` (exclude current model from candidates)
   - `Restart`: reset task state, retry from clean start
   - `Abort`: mark task as failed, stop retrying
5. After retry outcome, call `bandit.record_outcome(&state, action, reward)` where reward is computed from success/failure.
6. Save bandit state via `bandit.save(&conductor_path)`.

#### Verification Criteria
- [ ] Run a task that fails, verify `conductor.json` observation count > 0
- [ ] After 20+ failing tasks, conductor starts selecting non-Continue actions
- [ ] Conductor decisions are logged at INFO level

---

### Task 7.12: Wire Anomaly Detection to `roko run` and `roko chat`
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_session.rs`
**Depends On**: Task 7.2

#### Context
`AnomalyDetector` (at `anomaly.rs:20`) has session-local state for 3 detection channels:
- `check_prompt(prompt_hash: u64)` -> `Option<Anomaly::PromptLoop>`: sliding window of 20 hashes, alert at 5+ repeats
- `check_cost(cost_usd: f64)` -> `Option<Anomaly::CostSpike>`: EWMA baseline, z-score > 3.0
- `check_quality(score: f64)` -> `Option<Anomaly::QualityDrift>`: recent 5 vs prior 10 window
- `check_budget(limit_usd: f64)` -> `Option<Anomaly::BudgetExceeded>`: total cost vs limit

`learning_helpers.rs` (at line 11) imports `AnomalyDetector` and defines helper functions that accept `&mut AnomalyDetector` but these are not called from `run.rs` or `chat_session.rs`.

#### Implementation Steps
1. In `roko run`, create `AnomalyDetector::new(now_unix_ms_i64())` at session start.
2. After each model call, call:
   - `detector.check_cost(cost_usd)` -> if `Some(Anomaly::CostSpike { .. })`, log at WARN
   - `detector.check_prompt(hash_of_prompt)` -> if `Some(Anomaly::PromptLoop { .. })`, log at WARN, consider aborting
3. In `roko chat`, create `AnomalyDetector::new(now_unix_ms_i64())` at session start.
4. After each turn, check cost spike and prompt loop.
5. On anomaly detection: emit a warning to the user via stderr/tracing, but do not abort by default. Add a config option `anomaly.abort_on_loop = false` for future use.

#### Verification Criteria
- [ ] Repeat the same prompt 10 times in `roko chat`, verify prompt loop warning
- [ ] Send a very expensive prompt, verify cost spike warning
- [ ] Normal operation produces no anomaly warnings

---

### Task 7.13: Wire Regression Alerting to `roko run` and `roko status`
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/status.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/learn.rs`
**Depends On**: Task 7.5

#### Context
`LearningRuntime::record_completed_run()` returns `LearningUpdate` (at `runtime_feedback.rs:345`). `LearningUpdate` has a `regression_report: Option<RegressionReport>` field. `RegressionReport` (at `regression.rs:93`) has `regressions()`, `warnings()`, `improvements()` methods returning `Vec<&RegressionAlert>`. Each `RegressionAlert` has `metric_name`, `baseline`, `current`, `severity`.

Currently the `LearningUpdate` return value from `record_completed_run()` (at `run.rs:2680`) is discarded with `map_err()`.

#### Implementation Steps
1. In `run.rs`, capture the `LearningUpdate` from `record_completed_run()`.
2. If `update.regression_report` contains alerts with `severity >= Alert`:
   - Log each at WARN: `"Regression detected: {metric} dropped from {baseline:.2} to {current:.2}"`
3. In `commands/status.rs`, add a "Regressions" section that loads recent `RegressionReport` from the learning state.
4. In `commands/learn.rs`, add regression summary to `roko learn all` output.
5. Save regression report to a file (e.g., `.roko/learn/regressions.json`) for dashboard consumption.

#### Verification Criteria
- [ ] Create 10 passing tasks then 5 failing tasks, verify regression alert in logs
- [ ] `roko status` shows regression section when regressions exist
- [ ] `roko learn all` includes regression summary

---

### Task 7.14: Wire Provider Health Circuit Breaker to CascadeRouter
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`
**Depends On**: Task 7.9

#### Context
`ProviderHealthTracker` (at `provider_health.rs`) implements circuit breaker logic per provider. `LearningRuntime` already has `healthy_model_slugs()` (at `runtime_feedback.rs:1466`) that filters models by provider health.

`CascadeRouter` accepts `ProviderHealthRegistry` in helper functions (at `cascade_router.rs:658, 689`) but these are not called from the live selection path in `select_for_frequency_among()`.

#### Implementation Steps
1. In `roko run`, load `ProviderHealthTracker` from the `LearningRuntime` (it is created during `LearningRuntime::open()`).
2. Before calling `CascadeRouter::select_for_frequency_among()`, filter candidate slugs through `runtime.healthy_model_slugs(&all_slugs, provider_of_fn)`.
3. Pass only healthy models as candidates to the router.
4. After a model call failure, update provider health via `LearningRuntime` (already done in `record_completed_run()` step 3).
5. Log when a provider circuit opens: `warn!(provider = %p, "Circuit breaker open, excluding models")`.

#### Verification Criteria
- [ ] When a provider has high failure rate, its models are excluded from routing candidates
- [ ] Router falls back to healthy providers
- [ ] Circuit breaker recovery restores models to candidate pool

---

## Phase 4: Knowledge Integration

Durable knowledge informs dispatch. The dream cycle runs automatically.

### Task 7.15: Wire Knowledge-Informed Model Routing
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`
**Depends On**: Task 7.9

#### Context
`DreamRoutingAdvice` (at `roko-dreams/src/routing_advice.rs:19`) contains pattern-based model recommendations generated during dream cycles. `load_dream_routing_advice()` (at line 241) loads from `.roko/learn/dream-routing-advice.json`. `dream_advice_to_routing_bias()` (at line 154) converts advice to routing bias.

The dream advice file is written during `DreamCycle::run()` but never read at dispatch time.

#### Implementation Steps
1. At CascadeRouter initialization in `roko run`, call `load_dream_routing_advice(&workdir)`.
2. If advice exists, call `dream_advice_to_routing_bias(&advice)` to get bias values.
3. Apply bias to the routing context before `select_for_frequency_among()`: adjust alpha or modify candidate scoring.
4. After routing outcome (success/failure with selected model), this is already fed back through `FeedbackService` -> `CascadeRouter` observation (from Task 7.5). No additional wiring needed for the feedback direction.
5. Log when dream advice influences model selection.

#### Verification Criteria
- [ ] Run `roko knowledge dream run` to generate advice, then run tasks -- verify advice file is loaded
- [ ] Router considers dream advice in model selection (visible in logs)
- [ ] Advice has no effect when the file is missing (graceful fallback)

---

### Task 7.16: Wire Dream Cycle Automatic Trigger in `roko serve`
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/lib.rs` (or startup module)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/dreams.rs`
**Depends On**: none

#### Context
`start_dream_loop()` (at `dreams.rs:39`) is fully implemented: it spawns a background tokio task that checks `DreamLoopConfig.auto_dream`, runs `build_dream_cycle()`, and loops with `DREAM_CHECK_INTERVAL` (60s). `DreamLoopConfig` has `auto_dream: bool`, `interval`, `min_episodes_for_dream`, `agent` (model config).

The function exists but is never called from `roko serve` startup. The `roko-serve` library has the function exported but no startup code invokes it.

#### Implementation Steps
1. In `roko serve` startup (likely in `roko-serve/src/lib.rs` or the route builder that creates `AppState`), after constructing `AppState`, call `start_dream_loop(Arc::clone(&state), dream_config)`.
2. Load `DreamLoopConfig` from `roko.toml` config (add `[dreams]` section support if not present):
   ```toml
   [dreams]
   auto_dream = true
   interval_secs = 3600
   min_episodes = 20
   budget_usd = 0.10
   model = "claude-haiku-3-5"
   ```
3. Default `auto_dream = false` so existing deployments are not affected.
4. Store the `JoinHandle` from `start_dream_loop` so it can be cancelled on server shutdown.
5. Surface dream status in `roko status` output (last dream run timestamp, episode count since last dream).

#### Verification Criteria
- [ ] Start `roko serve` with `auto_dream = true`, verify dream loop starts (visible in logs)
- [ ] Dream cycle runs after sufficient episodes accumulate
- [ ] `auto_dream = false` (default) does not start the loop
- [ ] Server shutdown cancels the dream loop cleanly

---

### Task 7.17: Wire Knowledge Feedback Scoring Integration
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
**Depends On**: Task 7.5, Task 7.10

#### Context
`FeedbackService` tracks knowledge provenance: on `ModelCall`, it remembers which `knowledge_ids` were used. On `GateResult`, it resolves provenance for the `run_id` and applies `KnowledgeOutcome` (+1/-1 score). Scores persist to `knowledge-scores.json`.

`FeedbackService::record_knowledge_usage()` (at `feedback_service.rs:300`) takes `run_id`, `knowledge_ids`, `passed`, `model`. But the caller needs to provide `knowledge_ids` -- these come from prompt assembly when knowledge entries are included.

#### Implementation Steps
1. During prompt assembly in `run.rs`, if knowledge entries are included in the prompt (from `roko-neuro` query), capture their IDs.
2. Pass `knowledge_ids` in the `FeedbackEvent::ModelCall.knowledge_ids` field.
3. After gates complete, `FeedbackService` automatically resolves provenance from the `run_id` and updates scores (this is already implemented in the `GateResult` handler).
4. On next prompt assembly, load `knowledge-scores.json` and use scores to influence knowledge retrieval ranking (higher-scored entries prioritized).
5. Log when a knowledge entry's score drops below 0 (consistently unhelpful).

#### Verification Criteria
- [ ] Add a knowledge entry, run tasks that use it, check `knowledge-scores.json` shows accumulating score
- [ ] Knowledge entries with negative scores are deprioritized in subsequent retrievals

---

### Task 7.18: Wire StagingBuffer Promotion Without Full Dream Cycle
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/runtime_feedback.rs`
**Depends On**: none

#### Context
`StagingBuffer` (at `roko-dreams/src/staging.rs:93`) holds dream-generated knowledge candidates that progress through Raw -> Replayed -> Validated stages. Promotion to the durable `KnowledgeStore` only happens during a full `DreamCycle::run()`. Without a running dream cycle, the staging buffer grows without bound.

`LearningRuntime::record_completed_run()` (at `runtime_feedback.rs:2075`) has a knowledge seed append step. This is the natural place to add a lightweight promotion check.

#### Implementation Steps
1. Add `staging_buffer: Option<StagingBuffer>` to `LearningRuntime` (or load on demand from `.roko/learn/staging-buffer.json`).
2. At the end of `record_completed_run()`, after knowledge seed append:
   - Load `StagingBuffer` from disk if not in memory.
   - Check for entries at `StagingStage::Validated`.
   - For each validated entry, promote to `KnowledgeStore` via the store's `append()` method.
   - Remove promoted entries from the staging buffer.
   - Save the buffer back.
3. This promotion is lightweight (no LLM calls, no clustering) -- it just moves already-validated candidates into the durable store.
4. Log promotions at INFO level.

#### Verification Criteria
- [ ] Generate knowledge candidates that reach Validated stage
- [ ] After `record_completed_run()`, validated entries appear in the durable store
- [ ] No full dream cycle required for promotion

---

## Phase 5: Experiment Automation and Advanced Learning

The system proposes and concludes its own experiments. Advanced learning mechanisms are active.

### Task 7.19: Wire Experiment Winner Auto-Application
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/prompt_experiment.rs`
**Depends On**: Task 7.5

#### Context
`ExperimentStore::apply_winners()` (at `prompt_experiment.rs:522`) already exists and writes winning variants to a static overrides path. `apply_winners_to()` (at line 532) writes to a specified path. But no startup code loads and applies winners.

The `experiment-winners.json` file is written when an experiment concludes (Wilson CI convergence). Winners are `ExperimentWinner` with `experiment_id`, `winning_variant`, `value`, `confidence`.

#### Implementation Steps
1. At `roko run` startup, load `experiment-winners.json` from `.roko/learn/`.
2. If winners exist and `auto_apply_winners` config is true (default true), call `store.apply_winners(&winners)`.
3. The applied overrides should influence prompt assembly (applied as static section overrides).
4. Log auto-applied winners at INFO: `"Auto-applied experiment winner: {experiment_id} -> {variant}"`.
5. Add `learning.auto_apply_winners: bool` to config schema (default true).
6. Guard with config check -- skip if disabled.

#### Verification Criteria
- [ ] Manually conclude an experiment (set stats to trigger convergence), restart, verify winner is auto-applied
- [ ] `auto_apply_winners = false` skips application
- [ ] Log entry confirms auto-application

---

### Task 7.20: Wire Error Pattern Store Integration
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/error_pattern_store.rs`
**Depends On**: Task 7.11

#### Context
`ErrorPatternStore` (at `error_pattern_store.rs:229`) tracks gate failure patterns with `observe_gate_failure()` (line 299), `top_patterns()` (line 359), `format_for_prompt()` (line 433), `bounded_summary()` (line 376). It persists via `save()` / `load()`.

`GateFailureObservation` (at line 62) takes `error_digest`, `gate_name`, `model`, `role`, `task_category`, `cost_usd`.

The store exists but is not loaded or written to from any live path.

#### Implementation Steps
1. Load `ErrorPatternStore::load(&roko_dir.join("learn/error-patterns.json"))` at run initialization.
2. On gate failure, call `store.observe_gate_failure(GateFailureObservation::new(...))` with the gate error output.
3. Before dispatch, call `store.bounded_summary(model, role, category, limit)` to get relevant patterns.
4. If patterns exist, inject them into the prompt as a hint section (e.g., "Known failure patterns for this type of task: ...").
5. Feed high-frequency patterns into the conductor's `error_pattern` context feature (from Task 7.11).
6. Save the store after each observation.
7. Add GC: `store.gc(max_age: Duration::from_secs(30 * 86400), max_patterns: 500)` periodically.

#### Verification Criteria
- [ ] Run tasks that consistently fail with the same error, verify pattern store accumulates counts
- [ ] `error-patterns.json` shows stored patterns with frequency counts
- [ ] High-frequency patterns appear in `roko learn all` output

---

### Task 7.21: Wire Post-Gate Reflection Promotion to Playbook Candidates
**Priority**: P3
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/runtime_feedback.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/post_gate_reflection.rs`
**Depends On**: none

#### Context
`PostGateReflectionStore` (at `post_gate_reflection.rs:191`) records post-gate reflections with `observe()` (line 230). It has a `ReflectionPromotionConfig` (at line 164) with `min_confidence`, `min_validations`, `min_consistency` thresholds.

Reflections accumulate but are never checked for promotion eligibility. Promoted reflections should become playbook candidates.

#### Implementation Steps
1. After calling `PostGateReflectionStore::observe()` in `LearningRuntime::record_completed_run()`, check if the reflection meets promotion thresholds from `ReflectionPromotionConfig`.
2. If eligible, create a `Playbook` from the reflection's action sequence using `extract_playbook_from_episode()` (from `playbook.rs`).
3. Add the playbook candidate to `PlaybookStore` via `store.add()` or equivalent.
4. Log promotions at INFO.

#### Verification Criteria
- [ ] After 5+ successful reflections for the same pattern, a playbook candidate is created
- [ ] Promoted playbooks appear in `PlaybookStore` and are queryable

---

### Task 7.22: Wire Forensic Replay CLI Command
**Priority**: P3
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/learn.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/mod.rs`
**Depends On**: Task 7.7

#### Context
`ForensicReplay` (at `forensic_replay.rs:50`) reconstructs decision context from episodes: `from_episodes(task_id, all_episodes)` (line 75), `summary()` (line 216), `turn_count()`, `failed_gate_count()`. `replay()` async function (line 250) provides the full replay.

There is no CLI command to invoke forensic replay. `roko learn` currently has: All, Route, Experiments, Efficiency, Episodes, Tune.

#### Implementation Steps
1. Add `Replay { episode_id: String, workdir: Option<PathBuf> }` variant to the `LearnCmd` enum in `commands/learn.rs`.
2. In `dispatch_learn()`, add the `LearnCmd::Replay` match arm:
   - Load episodes from `.roko/learn/episodes.jsonl`
   - Call `ForensicReplay::from_episodes(&episode_id, &episodes)`
   - Display: model selected, gate verdicts, cost, duration, turn count
   - If CascadeRouter state is loadable, show "with current router state, this task would use {model}" comparison
3. Register the new subcommand in clap arg parsing.

#### Verification Criteria
- [ ] Run a task, then `roko learn replay <episode-id>` shows decision context
- [ ] Counterfactual model selection displayed when router state exists
- [ ] Missing episode ID produces a clear error message

---

## Phase 6: Continuous Optimization

Ongoing improvements that compound over time.

### Task 7.23: Wire Cross-Session Cost Aggregation
**Priority**: P3
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/costs_db.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/learn.rs`
**Depends On**: Task 7.8

#### Context
`CostsDb` (at `costs_db.rs:472`) has rich querying (`by_model`, `by_provider`, `by_role`, `by_plan`, `summary_by_model`) but no time-bounded aggregation. `BudgetGuardrail` needs daily totals. `roko learn efficiency` should show daily/weekly/monthly breakdowns.

#### Implementation Steps
1. Add `aggregate_since(since: DateTime<Utc>) -> CostSummary` to `CostsDb` that filters `records` by timestamp and calls `CostSummary::from_records()`.
2. Add `aggregate_range(from: DateTime<Utc>, to: DateTime<Utc>) -> CostSummary`.
3. In `commands/learn.rs` under the `Efficiency` arm, add cost breakdown output:
   - Today: `costs_db.aggregate_since(today_midnight)`
   - This week: `costs_db.aggregate_since(week_start)`
   - This month: `costs_db.aggregate_since(month_start)`
   - By model (all time): `costs_db.summary_by_model()`
4. Initialize `BudgetGuardrail.day_spent` from `aggregate_since(today_midnight)` (connects to Task 7.8).

#### Verification Criteria
- [ ] `roko learn efficiency` shows daily/weekly/monthly cost breakdowns
- [ ] Aggregation matches sum of individual records
- [ ] Budget guardrail initializes with correct daily spend

---

### Task 7.24: Wire Force-Backend Override Learning
**Priority**: P3
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`
**Depends On**: Task 7.9

#### Context
`CascadeRouter` implements `ForceBackendOverrideRecorder` trait (at `cascade_router.rs:134`) with `record_override_outcome(model_slug, success) -> bool`. This is already implemented: it updates the static table with `OVERRIDE_LEARNING_RATE` and returns whether the override was a "surprise" (router would have chosen differently).

But `roko run` does not detect when the user specifies `--model` and does not call `record_override_outcome()`.

#### Implementation Steps
1. In `roko run`, detect when the user provides `--model` flag (force backend override).
2. After the task completes, if a model override was active, call `router.record_override_outcome(&model_slug, success)`.
3. Log the result: if the override was a "surprise", note it: `"Override learning: router would have chosen {other_model}, user forced {model} (success={success})"`.
4. After N successful overrides for a pattern, the router's static table is updated and the router starts choosing that model automatically.
5. Save router state after override recording.

#### Verification Criteria
- [ ] Use `--model opus` flag 10 times, verify override count in `cascade-router.json`
- [ ] Router static table entry for the override pattern shows updated weights
- [ ] Without `--model`, router incorporates override learnings into selection

---

### Task 7.25: Wire Pareto Frontier Active Use in CascadeRouter
**Priority**: P3
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`
**Depends On**: Task 7.9

#### Context
The CascadeRouter already computes and caches a Pareto frontier (recomputed every `PARETO_RECOMPUTE_INTERVAL` observations). Helper functions for `pareto_adjusted_alpha()` exist but are not called from the live selection path in `select_for_frequency_among()`.

#### Implementation Steps
1. In `select_for_frequency_among()` (at `cascade_router.rs:329`), after UCB scoring, apply `pareto_adjusted_alpha()` to down-weight models dominated on the Pareto frontier (higher cost AND lower success than another model).
2. This should only activate once the router has enough observations for a meaningful frontier (> 100 observations total).
3. Log when a model is deprioritized due to Pareto dominance.

#### Verification Criteria
- [ ] After 100+ observations, dominated models get lower selection probability
- [ ] Pareto frontier computation visible in logs
- [ ] Non-dominated models are not penalized

---

### Task 7.26: Wire Calibration Policy Loop
**Priority**: P3
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/calibration_policy.rs`
**Depends On**: Task 7.9

#### Context
`CalibrationPolicy` (at `calibration_policy.rs`) tracks predict-publish-correct cycles. `process_event()` (line 87) takes an `AgentEvent` and returns `Option<CalibrationCorrection>`. The `CalibrationTracker` maintains predicted vs. actual success rates.

#### Implementation Steps
1. At run initialization, create `CalibrationPolicy::new()` (with optional `with_bias_threshold()` and `with_min_samples()`).
2. Before dispatch, publish predicted success probability from `CascadeRouter` UCB score.
3. After gate result, record actual outcome.
4. Compute calibration error: `|predicted - actual|`.
5. Feed calibration error into the CascadeRouter's alpha schedule: high error -> higher exploration (more alpha).
6. Persist calibration state to `.roko/learn/calibration.json`.

#### Verification Criteria
- [ ] Calibration corrections are computed after sufficient samples
- [ ] High calibration error increases exploration (higher alpha)
- [ ] `roko learn all` shows calibration metrics

---

### Task 7.27: Wire Curriculum Ordering for Plan Execution
**Priority**: P3
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs` (or runner/plan execution path)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/curriculum.rs`
**Depends On**: Task 7.9

#### Context
`CurriculumScheduler` (at `curriculum.rs:116`) reorders tasks by difficulty. `DifficultyModel` (at line 58) learns task difficulty from outcomes. `reorder_tasks()` (at line 492) takes tasks and a difficulty model.

This applies to plan execution (multi-task), not single `roko run`. The entry point is the runner/plan execution path.

#### Implementation Steps
1. In the plan execution path (likely `runner/` or `plan run` command), before executing tasks:
2. Load `DifficultyModel` from `.roko/learn/curriculum.json` (or create new).
3. Call `reorder_tasks(&tasks, &model)` to sort tasks by difficulty (easier first).
4. Execute in curriculum order to build routing signal from simpler tasks before attempting complex ones.
5. After each task, call `model.observe(&task, success)` to update difficulty estimates.
6. Save model state after plan completion.

#### Verification Criteria
- [ ] Plan with mixed-difficulty tasks executes simpler tasks first
- [ ] Difficulty model accumulates observations across plan runs
- [ ] Task order changes as model learns from outcomes

---

## Dependency Graph

```
Phase 1 (Foundation)
  Task 7.1 -> 7.2 -> 7.3   (chat feedback chain)
  Task 7.4                   (ACP feedback, independent)
  Task 7.5 -> 7.6           (run.rs feedback + dedup)
  |
Phase 2 (Routing & Cost)
  Task 7.5 -> 7.7 -> 7.9    (routing context -> model selection)
  Task 7.5 -> 7.8            (budget enforcement)
  Task 7.5 -> 7.10           (section effectiveness)
  |
Phase 3 (Intervention & Detection)
  Task 7.2 -> 7.12           (anomaly in chat)
  Task 7.9 -> 7.11           (conductor)
  Task 7.9 -> 7.14           (provider health)
  Task 7.5 -> 7.13           (regression alerting)
  |
Phase 4 (Knowledge)
  Task 7.9 -> 7.15           (knowledge-informed routing)
  Task 7.16                  (dream auto-trigger, independent)
  Task 7.5 + 7.10 -> 7.17   (knowledge scoring)
  Task 7.18                  (staging promotion, independent)
  |
Phase 5 (Experiments & Advanced)
  Task 7.5 -> 7.19           (winner auto-apply)
  Task 7.11 -> 7.20          (error patterns)
  Task 7.21                  (reflection promotion, independent)
  Task 7.7 -> 7.22           (forensic replay)
  |
Phase 6 (Continuous)
  Task 7.8 -> 7.23           (cross-session cost)
  Task 7.9 -> 7.24           (override learning)
  Task 7.9 -> 7.25           (pareto frontier)
  Task 7.9 -> 7.26           (calibration)
  Task 7.9 -> 7.27           (curriculum)
```

## File Index

All files referenced in this plan:

| File | Tasks |
|---|---|
| `crates/roko-cli/src/chat_session.rs` | 7.1, 7.2, 7.3, 7.12 |
| `crates/roko-cli/src/run.rs` | 7.5, 7.6, 7.7, 7.8, 7.9, 7.10, 7.11, 7.12, 7.13, 7.14, 7.15, 7.17, 7.19, 7.20, 7.24, 7.26, 7.27 |
| `crates/roko-cli/src/dispatch_direct.rs` | 7.9 |
| `crates/roko-cli/src/commands/learn.rs` | 7.13, 7.22, 7.23 |
| `crates/roko-cli/src/commands/status.rs` | 7.13 |
| `crates/roko-cli/src/commands/mod.rs` | 7.22 |
| `crates/roko-acp/src/runner.rs` | 7.4 |
| `crates/roko-acp/src/pipeline.rs` | 7.4 |
| `crates/roko-acp/Cargo.toml` | 7.4 |
| `crates/roko-learn/src/feedback_service.rs` | 7.1, 7.2, 7.5 |
| `crates/roko-learn/src/runtime_feedback.rs` | 7.7, 7.18, 7.21 |
| `crates/roko-learn/src/cascade_router.rs` | 7.9, 7.14, 7.15, 7.24, 7.25 |
| `crates/roko-learn/src/model_router.rs` | 7.7 |
| `crates/roko-learn/src/budget.rs` | 7.8 |
| `crates/roko-learn/src/conductor.rs` | 7.11 |
| `crates/roko-learn/src/anomaly.rs` | 7.12 |
| `crates/roko-learn/src/regression.rs` | 7.13 |
| `crates/roko-learn/src/provider_health.rs` | 7.14 |
| `crates/roko-learn/src/section_effect.rs` | 7.10 |
| `crates/roko-learn/src/prompt_experiment.rs` | 7.19 |
| `crates/roko-learn/src/error_pattern_store.rs` | 7.20 |
| `crates/roko-learn/src/post_gate_reflection.rs` | 7.21 |
| `crates/roko-learn/src/forensic_replay.rs` | 7.22 |
| `crates/roko-learn/src/costs_db.rs` | 7.23 |
| `crates/roko-learn/src/calibration_policy.rs` | 7.26 |
| `crates/roko-learn/src/curriculum.rs` | 7.27 |
| `crates/roko-compose/src/system_prompt_builder.rs` | 7.10 |
| `crates/roko-compose/src/prompt_assembly_service.rs` | 7.10, 7.17 |
| `crates/roko-compose/src/role_prompts.rs` | 7.10 |
| `crates/roko-dreams/src/routing_advice.rs` | 7.15 |
| `crates/roko-dreams/src/staging.rs` | 7.18 |
| `crates/roko-serve/src/dreams.rs` | 7.16 |
| `crates/roko-serve/src/lib.rs` | 7.16 |
