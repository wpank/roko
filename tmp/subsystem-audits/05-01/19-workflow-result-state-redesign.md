# 19 - Workflow Result and State Redesign

Scope: `crates/roko-runtime/src/workflow_engine.rs`, `crates/roko-runtime/src/effect_driver.rs`, `crates/roko-runtime/src/pipeline_state.rs`, `crates/roko-runtime/src/projection.rs`, `crates/roko-runtime/src/jsonl_logger.rs`

This pass checks whether the workflow runtime has a durable state/result contract or whether it reconstructs truth after the fact. The current implementation is closer to "run effects, emit events, then infer a report" than to a typed workflow execution model. That explains several bugs where success, model usage, provider, gate status, and commits are reported inaccurately.

## Findings

### CRITICAL: workflow reports are inferred from replayed global events

`workflow_engine.rs:152-155` captures the global runtime event bus sequence at run start. `workflow_engine.rs:500-515` builds the final report by calling `collect_run_events`, and `workflow_engine.rs:558-572` replays the global bus from that sequence and filters by `run_id`.

That makes the return value dependent on event-bus replay rather than the pipeline's own typed state. Any event dropped, malformed, emitted under the wrong run id, or emitted after the report boundary changes the report without changing the actual workflow outcome.

Expected design: `run_with_cancel` should accumulate a typed `RunLedger` as effects return. Events should be a projection of that ledger, not the source of truth for the returned report.

### HIGH: the report shape loses the actual model/provider contract

`WorkflowRunReport` has `model: String` and `provider: Option<String>` at `workflow_engine.rs:67-96`. `report_from_events` picks a model from `AgentSpawned` events and always sets `provider: None` at `workflow_engine.rs:592-659`.

But `effect_driver.rs:148-179` emits `AgentSpawned` with `self.services.default_model` before `ModelCallService` routing/fallback happens. The actual response model is only known later. Feedback recording also sets `provider: None` at `effect_driver.rs:199-221`.

Expected design: model events must distinguish requested model, routed model, attempted model, final model, provider id, and fallback source. The report should be built from the actual completed attempt, not the pre-dispatch default.

### HIGH: noop commit is encoded as successful commit hash

`effect_driver.rs:335-420` documents that a clean tree returns a noop hash. When `git commit` reports "nothing to commit", it emits `CommitDone { hash: "noop" }` at `effect_driver.rs:400-410`. `pipeline_state.rs:726-733` treats every `CommitDone` as `WorkflowOutcome::Success { commit_hash: Some(hash) }`.

That turns "there was no artifact to commit" into a successful workflow result. A caller has to special-case the string `"noop"` to know whether anything was produced.

Expected design: `CommitOutcome` should be an enum: `Created { hash }`, `NoChanges`, `Rejected`, `Failed`. The pipeline should decide explicitly whether `NoChanges` is success for the current workflow type.

### HIGH: effect outcomes are collapsed into narrow pipeline inputs

`PipelineInput` only represents broad events like `AgentCompleted`, `GateFailed`, and `CommitDone` (`pipeline_state.rs:430-485`). `EffectDriver::spawn_agent` records detailed side effects, but returns only output text and file count on success or a string error on failure (`effect_driver.rs:185-273`).

This forces the runtime to rediscover detail from events and feedback sinks. It also prevents the state machine from reacting to typed distinctions such as provider retry exhausted, model construction failed, budget exceeded, tool unavailable, auth missing, or unknown usage.

Expected design: effects should return typed domain outcomes. The state machine can still stay pure, but its inputs need enough structure to preserve why an effect succeeded, failed, skipped, or produced no artifact.

### MEDIUM: cancellation is cooperative only at loop boundaries

`workflow_engine.rs:136-146` states cancellation is checked only at the top of the loop and any in-flight effect is awaited to completion. That is acceptable as an implementation detail only if every effect has its own timeout/cancel propagation. The current dispatch/gate paths are not consistently modeled around cancellation tokens.

Expected design: pass cancellation into `ModelCallRequest`, provider adapters, gate execution, and subprocess wrappers. The workflow engine should record whether cancellation interrupted an effect or waited for it.

### MEDIUM: event persistence silently drops corrupt or failed writes

`projection.rs:64-72` ignores JSON parse failures when reading event JSONL. `jsonl_logger.rs:89-92` ignores write errors from `write_event`. The logger flushes at `jsonl_logger.rs:75-82`, but there is no durability/error contract exposed to the workflow.

This makes runtime observability look more reliable than it is. A corrupt event log should be a diagnosable health issue, not an invisible gap in projections.

Expected design: event persistence should return explicit health/errors, count corrupt lines, and support strict replay for tests. Silent skip should be opt-in for best-effort dashboards only.

## Redesign Direction

1. Add a typed `RunLedger` that is updated directly by effect outcomes; derive events and reports from it.
2. Replace stringly commit success with `CommitOutcome`.
3. Expand pipeline inputs to carry typed effect failure categories and actual dispatch metadata.
4. Model cancellation as part of every long-running effect, not just the workflow loop.
5. Make event logging/replay strict by default in tests and observable in production.
6. Stop using default model/provider placeholders in final reports once routing or fallback occurs.
