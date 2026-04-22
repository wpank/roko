# 24 - Runtime/Gate Ledger Redesign Plan

Scope: `roko-runtime`, `roko-core` runtime/gate event contracts, `roko-gate`, PRD/plan artifact validation, `roko-serve` terminal command execution, and demo command consumers.

Goal: make workflow truth a typed ledger produced by effects, not a reconstruction from JSONL, prompt scraping, status strings, or side-channel booleans.

## Target Ownership

| Owner module | Responsibility |
|---|---|
| `crates/roko-runtime/src/run_ledger.rs` | `RunLedger`, typed effect outcomes, report/event projection, cancellation history |
| `crates/roko-runtime/src/effect_driver.rs` | Execute effects and return typed outcomes; no report inference |
| `crates/roko-runtime/src/pipeline_state.rs` | Pure state transitions over typed outcomes |
| `crates/roko-core/src/runtime_event.rs` | Wire-stable events derived from ledger |
| `crates/roko-runtime/src/jsonl_logger.rs`, `projection.rs` | Durable event store and strict/best-effort replay |
| `crates/roko-core/src/foundation.rs` or `src/gate_types.rs` | Shared `GateId`, `GateStatus`, `GateVerdict`, `ArtifactOutcome` types |
| `crates/roko-gate/src/registry.rs` | Single gate registry: aliases, rung, executor, required inputs, config validation, resource policy |
| `crates/roko-serve/src/command_runner.rs`, `terminal.rs` | Typed command/session events; PTY output is display only |
| `demo/demo-app/src/lib/terminal-session.ts`, scenario runners | Consume typed command/result/workflow events, not prompt/output scraping |

## New Core Types

Add these before changing behavior, then migrate call sites behind adapters.

```rust
pub struct RunLedger {
    pub run_id: String,
    pub prompt: String,
    pub workflow: WorkflowConfig,
    pub started_at_ms: u64,
    pub phase_history: Vec<PhaseTransitionRecord>,
    pub agent_attempts: Vec<AgentOutcome>,
    pub gate_runs: Vec<GateRunOutcome>,
    pub artifacts: Vec<ArtifactOutcome>,
    pub commit: Option<CommitOutcome>,
    pub cancellation: Option<CancellationOutcome>,
    pub event_persistence: EventPersistenceHealth,
    pub checkpoint_path: Option<PathBuf>,
}

pub enum EffectOutcome {
    Agent(AgentOutcome),
    Gates(GateRunOutcome),
    Commit(CommitOutcome),
    Artifact(ArtifactOutcome),
    Command(CommandOutcome),
    Cancelled(CancellationOutcome),
}

pub enum AgentOutcome {
    Completed {
        role: String,
        output: String,
        files_changed: u32,
        requested_model: String,
        routed_model: Option<String>,
        final_model: String,
        provider_id: String,
        usage: UsageObservation,
        request_id: Option<String>,
    },
    Failed { role: String, kind: EffectErrorKind, message: String },
}

pub enum CommitOutcome {
    Created { hash: String, message: String },
    NoChanges { reason: String },
    Rejected { reason: String },
    Failed { command: String, status: Option<i32>, stderr: String },
}

pub enum GateStatus {
    Passed,
    Failed { failure_domain: Option<String>, retryable: bool },
    Skipped { reason: String, adaptive: bool },
    NotWired { reason: String },
    InvalidConfig { reason: String },
    TimedOut { timeout_ms: u64 },
    Cancelled,
}

pub struct GateVerdict {
    pub gate_id: GateId,
    pub display_name: String,
    pub rung: u8,
    pub status: GateStatus,
    pub output: String,
    pub duration_ms: u64,
}

pub enum ArtifactOutcome {
    Valid { artifact_type: String, path: PathBuf, report: serde_json::Value },
    Invalid { artifact_type: String, path: Option<PathBuf>, report: serde_json::Value },
    NotProduced { artifact_type: String, reason: String },
    ValidationUnavailable { artifact_type: String, reason: String },
}

pub enum CommandOutcome {
    Exited { command_id: String, exit_code: i32, duration_ms: u64 },
    SpawnFailed { command_id: String, message: String },
    Cancelled { command_id: String },
}
```

`RunLedger::to_report()` is the only builder for `WorkflowRunReport`. Events are projections via `RunLedger::event_delta(...)`, never inputs to the final report.

## Phase 1 - Ledger and Typed Outcomes

1. Add `run_ledger.rs` with `RunLedger`, `EffectOutcome`, `AgentOutcome`, `CommitOutcome`, `EventPersistenceHealth`, and conversion helpers.
2. Change `EffectDriver::{spawn_agent,run_gates,commit}` to return typed outcomes internally, with temporary `impl From<...> for PipelineInput` adapters.
3. Extend `ModelCallResponse`/feedback usage to carry actual provider/model and optional usage. Stop setting `provider: None` or missing usage to zero in new ledger paths.
4. Change `WorkflowEngine::run_with_cancel` to:
   - create a ledger at run start;
   - record each phase transition and effect outcome immediately;
   - derive events from each ledger update;
   - return `ledger.to_report()` instead of `report_from_events(...)`.
5. Keep legacy runtime events emitted for UI compatibility, but mark `collect_run_events` as projection-only.

Deletion after migration: remove `report_from_events`, workflow-report dependence on `event_start_seq`, and any model/provider fallback in report building.

## Phase 2 - Commit Semantics

1. Replace `PipelineInput::CommitDone { hash }` and `CommitFailed { error }` with `CommitFinished { outcome: CommitOutcome }`.
2. In `EffectDriver::commit`, return `CommitOutcome::NoChanges` for clean trees; never encode `"noop"` as a hash.
3. Add `WorkflowConfig.commit_no_changes_policy`:
   - default `Halt` for implementation workflows;
   - opt-in `AllowSuccess` only for workflows explicitly declared observational/read-only.
4. Update `WorkflowOutcome` to include the typed commit result, or retain old fields only as serialized compatibility fields derived from `CommitOutcome::Created`.

Deletion after migration: remove all `"noop"` special cases and `CommitDone` success transitions.

## Phase 3 - Gate Registry and Gate Status

1. Add `roko-gate/src/registry.rs`:
   - `GateSpec { id, aliases, rung, kind, required_inputs, executor, result_schema, resource_group, concurrency }`;
   - `GateRegistry::resolve(alias)`;
   - `GateRegistry::validate_config(&GateConfig) -> GateConfigValidation`.
2. Move all gate/rung maps out of `gate_service.rs`, `effect_driver.rs`, and CLI runner code. Runtime must query the registry.
3. Replace `GateVerdict { passed, skipped, skip_reason }` with `GateStatus`.
4. Distinguish required non-executable gates from adaptive skips:
   - required `judge` with no real executor => `InvalidConfig` at preflight;
   - custom/shell missing command => `InvalidConfig`;
   - unknown gate => `NotWired`;
   - adaptive skip => `Skipped { adaptive: true }`.
5. Change `GateReport` helpers to explicit predicates:
   - `blocking_failures()`;
   - `all_required_passed()`;
   - `has_invalid_config()`.
6. Update dashboard/runtime events from `GatePassed/GateFailed` to a single typed `GateCompleted { status, rung, output }`; keep old events as compatibility projections until UI migration lands.

Deletion after migration: remove duplicate `rung_for_gate_name`, serialized-string failure classification checks, and boolean gate result APIs outside UI-only adapters.

## Phase 4 - Artifact Validity as Outcome

1. Promote `ArtifactOutcome` to shared core/runtime types.
2. Convert `GenerationOutcome { process_success, artifact_valid, validation_report }` into an adapter over `ArtifactOutcome`.
3. Make PRD and plan generation return:
   - process/agent outcome separately;
   - `ArtifactOutcome::{Valid, Invalid, NotProduced, ValidationUnavailable}` for produced PRDs/plans/tasks.
4. Add `PipelineInput::ArtifactValidated(ArtifactOutcome)` for workflows that produce required artifacts.
5. `RunLedger::to_report()` must fail required-artifact workflows when any required artifact is `Invalid`, `NotProduced`, or `ValidationUnavailable`.
6. Learning receives typed artifact provenance; positive learning/router updates require `ArtifactOutcome::Valid` plus real gate evidence.

Deletion after migration: remove success decisions based on adjacent `process_success` and `artifact_valid` booleans; remove default "missing artifact_valid means true" from new records.

## Phase 5 - Cancellation and Timeouts

1. Pass `CancelToken` and optional deadline into each long-running effect: model calls, provider streams, gates, subprocess commands, commits, and terminal commands.
2. In `WorkflowEngine`, use `tokio::select!` around in-flight effects. If an effect cannot stop immediately, record `CancellationOutcome::WaitedForEffect`; if interrupted, record `Interrupted { effect_id }`.
3. Add typed `EffectErrorKind::{Cancelled, TimedOut, BudgetExceeded, AuthMissing, ProviderUnavailable, PromptAssemblyFailed, ToolUnavailable, Unknown}`.
4. Persist cancellation in ledger and event stream, not only as `PipelineInput::UserCancel` at loop boundaries.

Acceptance: cancelling a long gate/model/command returns a cancelled ledger outcome without waiting for the next workflow-loop iteration.

## Phase 6 - Strict Event Persistence

1. Introduce `JsonlEventStore`:
   - `append(envelope) -> Result<AppendAck>`;
   - `flush() -> Result<()>`;
   - `replay(path, ReplayMode::{Strict, BestEffort}) -> ReplayReport`.
2. `JsonlLogger` may remain an `EventConsumer` adapter, but it must log append errors and update `EventPersistenceHealth`; the engine-owned event store must return errors.
3. `projection.rs` must not silently skip corrupt lines in strict mode. Best-effort mode returns summaries plus `corrupt_line_count` and diagnostics.
4. Tests run strict mode by default. Dashboards may choose best-effort and display health.

Deletion after migration: remove `Err(_) => continue` JSONL parsing and ignored `write_event` results from runtime-critical paths.

## Phase 7 - Terminal and Demo Truth

1. Add a typed command channel in serve:
   - REST or WS command start: `{ command_id, workspace_id, argv_or_shell, cwd, env, timeout_ms }`;
   - events: `CommandStarted`, `CommandOutput { stream, chunk }`, `CommandExited { exit_code }`, `CommandSpawnFailed`, `CommandCancelled`.
2. Demo `showCmd` waits for `CommandExited`, not shell prompt regex. Gate/cost/token panels subscribe to workflow/command events, not `outputBuffer` regexes.
3. Keep PTY terminal for manual interaction and display. Prompt detection remains a manual fallback only.
4. Fix terminal session lifecycle while touching this path:
   - reject WS upgrade or send typed `TerminalError` then close on PTY spawn failure;
   - validate requested session IDs and prevent silent remap collisions;
   - parse resize messages as typed JSON, not prefix strings;
   - move clean-shell startup behind explicit demo mode and clean temp resources.
5. Scenario runners should use workspace IDs instead of absolute root query params when subscribing to workflow state.

Deletion after migration: remove scenario correctness from `PROMPT_RE`, `waitForPrompt`, `detectFromOutput`, and `outputBuffer` scraping; remove default `ZDOTDIR` prompt rewriting.

## Recurrence Checks

Add CI/static checks before another runner wave:

- `rg '"noop"|CommitDone \\{ hash' crates/roko-runtime crates/roko-cli` must fail.
- `rg 'report_from_events|collect_run_events\\(' crates/roko-runtime/src/workflow_engine.rs` must fail outside compatibility tests.
- `rg 'rung_for_gate|match .*gate.*as_str\\(\\)' crates/roko-runtime crates/roko-cli crates/roko-gate` must allow only `registry.rs` and tests.
- `rg 'passed: bool|skipped: bool|skip_reason' crates/roko-core crates/roko-gate crates/roko-runtime` must fail for runtime gate contracts.
- `rg 'provider: None|total_tokens: 0|cost_usd: 0.0' crates/roko-runtime crates/roko-learn crates/roko-agent` must fail in telemetry paths unless the field is explicitly display-only or test data.
- `rg 'Err\\(_\\) => continue|let _ = .*write_event|let _ = .*persist' crates/roko-runtime crates/roko-serve crates/roko-cli` must fail for runtime-critical persistence.
- `rg 'PROMPT_RE|detectFromOutput|waitForPrompt\\(' demo/demo-app/src/lib demo/demo-app/src/lib/scenario-runners` must fail outside manual terminal helpers.
- `rg 'starts_with\\(\"\\{\\\\\"type\\\\\":\\\\\"resize' crates/roko-serve/src/terminal.rs` must fail.
- `rg 'process_success.*artifact_valid|artifact_valid.*process_success' crates/roko-cli crates/roko-learn crates/roko-runtime` must fail after `ArtifactOutcome` migration.

## Acceptance Tests

Required tests for the full change:

1. `workflow_report_uses_ledger_when_event_log_fails`: disable/poison event append; workflow report still has correct outcome, model/provider, gates, commit, and artifact status, with event persistence health marked failed.
2. `commit_no_changes_is_not_created_commit`: clean tree returns `CommitOutcome::NoChanges`; default implementation workflow halts or reports configured no-change policy, never `commit_hash = "noop"`.
3. `actual_model_provider_survive_routing`: requested model differs from routed/final model; report and feedback show requested, routed, final model, provider id, and optional usage accurately.
4. `required_judge_not_wired_is_config_error`: required `judge` without executor fails gate preflight as `InvalidConfig`, not skipped or failed later.
5. `custom_gate_missing_command_is_invalid_config`: shell/custom gate without command is blocking config error.
6. `adaptive_skip_is_not_gate_failure`: optional/adaptive skip is preserved as `GateStatus::Skipped { adaptive: true }` and does not masquerade as pass/fail.
7. `artifact_invalid_blocks_success`: PRD/plan generation with process success but invalid validation report records `ArtifactOutcome::Invalid` and cannot produce workflow success or positive learning.
8. `strict_replay_rejects_corrupt_jsonl`: strict projection returns an error with line number; best-effort returns corrupt count.
9. `cancel_interrupts_inflight_effect`: cancel during a sleeping command/gate records typed cancellation and stops the child/provider stream.
10. `terminal_spawn_failure_is_typed`: invalid shell/workdir produces typed terminal error and closed connection, not a connected dead terminal.
11. `demo_command_false_returns_failed_result`: command `false` completes with `exit_code=1`; no prompt/output scraping is needed.
12. `resize_messages_are_typed`: malformed resize JSON is rejected as a typed client error and never treated as input.

## Rollout Order

1. Land new types and adapters without changing external behavior.
2. Switch `WorkflowEngine` report generation to `RunLedger`.
3. Migrate commit outcomes and update state-machine tests.
4. Migrate gate registry/status and update runtime/runner/dashboard adapters.
5. Migrate artifact validity to `ArtifactOutcome` and learning gates.
6. Introduce strict event store/replay and wire health into reports.
7. Add typed command runner and migrate demo scenarios.
8. Remove compatibility shims and enable recurrence checks as CI gates.
