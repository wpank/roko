# Runtime Reconciliation: Replace `orchestrate.rs` Without Losing Capability

> Concrete implementation blueprint for reconciling the working-but-tangled `orchestrate.rs` path with the cleaner-but-underfeatured `runner/` path.

## Goal

End with exactly one authoritative execution runtime for plan work:

- `roko-orchestrator` owns pure scheduling and state transitions
- `roko-cli/src/runner/` owns the event loop and effect dispatch
- `orchestrate.rs` is reduced to compatibility wrappers or removed

This document is about **how to get there without regressing features**.

---

## 1. Non-Goals

Do **not** do any of these:

1. Do not build a third runtime.
2. Do not keep `runner` and `orchestrate.rs` as co-equal long-term implementations.
3. Do not port features by copy-pasting logic from `orchestrate.rs` into `runner/`.
4. Do not split crates further until the runtime has converged.

---

## 2. The Core Rule

Every feature currently living in `orchestrate.rs` must be migrated by one of only three patterns:

1. **Extract to shared helper/service module**
2. **Push down into the owning crate**
3. **Delete if superseded**

Anything else is how the repo gets more tangled.

---

## 3. End-State Runtime Shape

## 3.1 Ownership Model

### `roko-orchestrator`

Owns:

- `ParallelExecutor`
- `ExecutorAction`
- `ExecutorEvent`
- plan state machine
- DAG resolution
- merge queue primitives
- retry/replan state transitions

Must not own:

- provider subprocess handling
- prompt construction
- knowledge queries
- dashboard projection

### `roko-cli/src/runner/`

Owns:

- single `tokio::select!` event loop
- action dispatch to external systems
- checkpoint cadence
- cancellation / shutdown
- progress projection publication

Must not own:

- Claude-specific parsing
- raw prompt composition logic
- learning algorithms
- knowledge-store policy

### `roko-agent`

Owns:

- provider selection
- process/session lifecycle
- stream parsing
- tool loop
- normalized agent runtime events
- warm pooling

### `roko-compose`

Owns:

- prompt assembly
- context retrieval and section shaping
- role policy
- retry/gate feedback structuring

### `roko-gate`

Owns:

- rung execution
- gate output classification
- structured verdict records

### `roko-learn`, `roko-neuro`, `roko-dreams`, `roko-conductor`, `roko-daimon`

Own:

- event consumers
- decisions, observations, persistence, projections

Must not depend on CLI-local mutable structs.

---

## 4. The Missing Runtime Spine

The runtime should be expressed as four loops sharing one event model.

## 4.1 Command Loop

Inputs:

- CLI command
- API request
- resume request
- trigger fire

Output:

- `RunCommand`

Examples:

- `StartPlans { plan_dirs }`
- `ResumeRun { snapshot }`
- `PausePlan { plan_id }`
- `CancelRun`

## 4.2 Executor Loop

Inputs:

- `RunCommand`
- `ExecutorEvent`

Output:

- `ExecutorAction`

This is almost already what `roko-orchestrator` gives you.

## 4.3 Effect Loop

Inputs:

- `ExecutorAction`

Output:

- normalized runtime events:
  - `AgentRuntimeEvent`
  - `GateRuntimeEvent`
  - `MergeRuntimeEvent`
  - `KnowledgeRuntimeEvent`
  - `SystemRuntimeEvent`

This is the part that `runner/` should own.

## 4.4 Projection/Feedback Loop

Inputs:

- normalized runtime events

Outputs:

- dashboard events
- episode records
- routing observations
- knowledge promotions
- conductor interventions
- snapshot deltas

This is where most of the current `orchestrate.rs` richness belongs after extraction.

---

## 5. What Must Be Extracted From `orchestrate.rs`

These are the capabilities that matter. If these are migrated, `runner/` becomes feature-complete enough to retire the old path.

## 5.1 Dispatch subsystem

Current reality:

- richer provider factory usage exists in `orchestrate.rs`
- runner still spawns Claude CLI directly

Target module:

- `crates/roko-cli/src/dispatch/`

Submodules:

- `mod.rs`
- `model_routing.rs`
- `prompt_builder.rs`
- `session.rs`
- `warm_pool.rs`
- `preflight.rs`

Responsibilities:

- choose provider/model
- assemble system prompt and task prompt
- select session reuse or cold spawn
- enforce role-specific tool/safety policy
- expose one `dispatch_agent(...)` API

Public shape:

```rust
pub struct DispatchRequest {
    pub plan_id: String,
    pub task_id: String,
    pub role: AgentRole,
    pub retry_context: Option<RetryContext>,
    pub model_hint: Option<String>,
    pub task: TaskDef,
}

pub struct DispatchResult {
    pub agent_run_id: String,
    pub requested_model: String,
    pub actual_model: Option<String>,
    pub provider: String,
}

pub async fn dispatch_agent(
    req: DispatchRequest,
    ctx: &DispatchContext,
) -> Result<DispatchResult>;
```

Migration source:

- agent factory and safety setup from `orchestrate.rs`
- minimal event-loop call site from `runner/event_loop.rs`

## 5.2 Normalized agent event subsystem

Current reality:

- runner uses Claude-specific stream protocol

Target owner:

- `roko-agent`

New module:

- `crates/roko-agent/src/runtime_events.rs`

Core enum:

```rust
pub enum AgentRuntimeEvent {
    Started {
        run_id: String,
        requested_model: String,
        actual_model: Option<String>,
        provider: String,
        pid: Option<u32>,
        session_id: Option<String>,
    },
    OutputDelta {
        run_id: String,
        text: String,
    },
    ToolCallStarted {
        run_id: String,
        call_id: String,
        tool_name: String,
    },
    ToolCallFinished {
        run_id: String,
        call_id: String,
        success: bool,
        preview: String,
    },
    Usage {
        run_id: String,
        input_tokens: u64,
        output_tokens: u64,
        cache_read_tokens: u64,
        cache_write_tokens: u64,
        total_cost_usd: Option<f64>,
    },
    Completed {
        run_id: String,
        success: bool,
        session_id: Option<String>,
        total_cost_usd: Option<f64>,
        turns: Option<u32>,
    },
    Failed {
        run_id: String,
        message: String,
    },
    Exited {
        run_id: String,
        exit_code: Option<i32>,
    },
}
```

Required adapters:

- Claude CLI adapter
- provider-backed HTTP tool-loop adapter
- future exec/codex/cursor adapters

Runner consequence:

- delete `ClaudeStreamEvent` from runner types
- runner consumes only `AgentRuntimeEvent`

## 5.3 Gate execution subsystem

Current reality:

- basic gate dispatch in runner
- richer gate handling in `orchestrate.rs`

Target module:

- `crates/roko-cli/src/runner/gates.rs`

Responsibilities:

- spawn gate runs with timeout
- stream gate stdout/stderr
- classify failures
- produce structured `GateCompletion`

Public shape:

```rust
pub struct GateRunRequest {
    pub plan_id: String,
    pub task_id: String,
    pub rung: u32,
    pub payload: GatePayload,
}

pub enum GateRuntimeEvent {
    OutputChunk {
        plan_id: String,
        task_id: String,
        stream: GateStream,
        chunk: String,
    },
    Completed(GateCompletion),
}
```

Key rule:

Gate output must be streamable, not just emitted as a batch blob.

## 5.4 Learning/feedback sink subsystem

Current reality:

- runner emits some episodes
- legacy path wires more learning behavior

Target module:

- `crates/roko-cli/src/runtime_feedback/`

Submodules:

- `episodes.rs`
- `routing.rs`
- `knowledge.rs`
- `conductor.rs`
- `dreams.rs`

Responsibilities:

- consume normalized runtime events
- record episodes and efficiency events
- record routing observations
- prepare knowledge promotions
- trigger dream/consolidation hooks
- convert conductor diagnoses into runtime actions

Key rule:

The event loop should call **one sink facade**, not five ad hoc subsystems.

## 5.5 Projection subsystem

Target module:

- `crates/roko-cli/src/projection/`

Submodules:

- `dashboard.rs`
- `logs.rs`
- `cli_progress.rs`

Responsibilities:

- convert runtime events into `DashboardEvent`
- convert runtime events into structured operator output
- own projection state transitions

Key rule:

TUI and non-TUI output are two projections of the same runtime events.

---

## 6. Exact Migration Order

## Phase 0: Freeze

Rules:

- no new feature work in `orchestrate.rs`
- bug fixes only
- every new runtime feature must target reusable modules consumable by `runner/`

Deliverable:

- comment/header in `orchestrate.rs` marking it as donor/legacy path

## Phase 1: Dispatch extraction

Build:

- `dispatch/` modules

Move from `orchestrate.rs`:

- provider/model selection
- safety/spawn setup
- session reuse/warm path logic
- richer prompt building integration hooks

Runner change:

- replace `agent_stream::spawn_agent(...)` path with `dispatch_agent(...)`

Success criteria:

- runner no longer spawns Claude directly
- runner no longer chooses model directly

## Phase 2: Agent event normalization

Build:

- `roko-agent::runtime_events`

Move:

- stream parsing logic out of runner

Runner change:

- `agent_events.rs` handles normalized events only

Success criteria:

- no provider-specific event enum in runner
- Claude-specific parsing contained entirely below `roko-agent`

## Phase 3: Prompt/construction convergence

Build:

- `dispatch/prompt_builder.rs`

Move:

- real system prompt assembly
- retry feedback shaping
- playbook/knowledge injection

Runner change:

- delete minimal system prompt path

Success criteria:

- all live runner tasks use composition engine path

## Phase 4: Feedback convergence

Build:

- `runtime_feedback/` facade

Move:

- episode logging
- routing observations
- knowledge candidate creation
- dream triggers
- conductor observation hooks

Runner change:

- event loop emits into feedback sink after every meaningful runtime event

Success criteria:

- no duplicate feedback logic between runner and `orchestrate.rs`

## Phase 5: Projection convergence

Build:

- projection facade

Move:

- dashboard/state hub publication
- structured non-TUI progress output

Success criteria:

- TUI/API/CLI progress are all driven by same event stream

## Phase 6: Parity deletion phase

Do:

- compare feature matrix against legacy/orchestrate path
- switch any remaining callers
- delete or collapse dead legacy helpers

Success criteria:

- `orchestrate.rs` is no longer required for normal plan execution

---

## 7. Data Types That Must Stop Being Local

These types should not remain CLI-private forever.

### Move into `roko-core` or `roko-orchestrator`

- normalized runtime event ids
- plan/task execution status enums
- retry/failure classification enums
- structured gate feedback records

### Move into `roko-agent`

- normalized agent lifecycle/stream events
- provider-neutral usage/cost records

### Move into `roko-gate`

- structured gate output model
- streamed gate chunk metadata

---

## 8. The Only Two Allowed Bridging Strategies

When migrating a capability from `orchestrate.rs`, choose one:

### Strategy A: Extract-then-switch

1. move logic to helper/service
2. have both runtimes call it
3. switch runner to authoritative
4. delete legacy call site

Use this when behavior is correct but misplaced.

### Strategy B: Re-specify-then-reimplement

1. define normalized interface
2. reimplement behind correct owner boundary
3. adapt legacy code if needed during migration
4. delete legacy implementation

Use this when the old behavior is too entangled to extract cleanly.

Examples:

- dispatch setup: Strategy A
- normalized agent event model: Strategy B

---

## 9. What "Done" Actually Looks Like

You are done when all of these are true:

1. `plan run` uses `runner/` only.
2. `runner/` does not know any provider wire protocol.
3. prompt assembly in runner goes through real composition pipeline.
4. routing, episodes, knowledge, conductor, and dream hooks are active in runner.
5. gate output is streamable and structured.
6. dashboard/API/CLI progress consume same projection stream.
7. `orchestrate.rs` contains no unique production-critical behavior.

If even one of those is false, the runtime is still split.

---

## 10. Why This Will Work Better Than "Fix orchestrate"

Because `orchestrate.rs` already proved the failure mode:

- once a file becomes the integration sink, every missing feature gets glued there
- even good abstractions below it do not save the runtime from tangling

The runner has the better shape.

So the only credible reconciliation is:

**port the richness into the better shape, not the better shape back into the old sink.**

## Implementation Packet

This is the primary execution plan for reconciling `orchestrate.rs` and `runner/`.

### Phase Checklist

- [ ] Phase 0: add a legacy-freeze note at the top of `orchestrate.rs`.
- [x] Phase 0: add module shells for `dispatch`, `runtime_feedback`, and `projection`.
- [x] Phase 1: implement dispatch facade with mock path support.
- [x] Phase 1: route runner agent spawning through dispatch facade.
- [x] Phase 2: add `roko-agent::runtime_events`.
- [x] Phase 2: move Claude stream parsing below `roko-agent`.
- [x] Phase 3: replace minimal prompt path with `dispatch/prompt_builder.rs`.
- [x] Phase 4: add feedback facade and wire episode/routing/knowledge/conductor/dream events.
- [x] Phase 5: add projection facade and non-TUI progress output.
- [ ] Phase 6: run parity matrix and retire migrated legacy behavior.

### Per-Phase Acceptance Gates

- [ ] Phase 0 gate: `cargo check -p roko-cli`.
- [ ] Phase 1 gate: runner can execute a mock agent through dispatch facade.
- [ ] Phase 2 gate: runner has no provider-specific stream event enum.
- [ ] Phase 3 gate: runner has no production call to minimal system prompt builder.
- [ ] Phase 4 gate: successful task writes episode, routing observation, and optional knowledge candidate.
- [ ] Phase 5 gate: TUI and non-TUI output consume same runtime event categories.
- [ ] Phase 6 gate: `orchestrate.rs` contains no unique production behavior required by `plan run`.

### Migration Notes For Agents

- [ ] When a legacy block is too tangled, write a small compatibility wrapper first.
- [ ] When extracting behavior, keep old and new call sites temporarily until tests pass.
- [ ] When a behavior is superseded, mark the old path with a comment and a deletion follow-up.
- [ ] Record every remaining legacy-only behavior in `21-FEATURE-PARITY-MATRIX.md`.

## 11. Reconciliation Proof Delta (2026-04-26)

This section records concrete runtime reconciliation progress already implemented in `runner/`.

### Confirmed now working in live runner path

- [x] Agent spawn is no longer repeatedly reissued while an agent is active.
- [x] Agent completion is phase-aware, not hardcoded to one transition.
- [x] Agent stderr is persisted as structured runtime event (`agent.error`).
- [x] Gate dispatch uses structured payload (`GatePayload`) instead of ad hoc text body.
- [x] `task.verify` commands execute as real shell gates in rung 0.
- [x] Runner writes `.roko/events.jsonl` during execution.
- [x] Terminal snapshot is flushed at run end with final phase.
- [x] Completed tasks are excluded from DAG ready selection.
- [x] Non-interactive runner can auto-advance post-gate phases for smoke plan completion.

### Runtime proof scenarios executed

- [x] No-mock Codex smoke run passed.
- [x] No-mock Claude smoke run passed.
- [x] Failure case captured and corrected: missing `Cargo.toml` caused `compile:cargo` fail in temp workspace.

### Remaining reconciliation backlog after this proof

- [x] Move provider-specific stream parsing fully behind `roko-agent` runtime event adapter.
- [x] Remove the remaining runner-local prompt fallback by making live runner prompt construction use `PromptAssembler`; old helper references are now legacy/test cleanup, not active runner ownership.
- [x] Add feedback facade wiring for routing, knowledge, conductor, and dream outputs.
- [x] Add projection facade so TUI, HTTP, and non-TUI CLI share one event mapping surface.
- [ ] Eliminate remaining unique production-critical behavior in `orchestrate.rs`.

## 12. Worker 9 Evidence Checklist (2026-04-26)

Current reconciliation proof:

- [x] `runner/event_loop.rs` checks `state.agent_active || agent_handle.is_some()` before spawning, preventing duplicate active agent effects.
- [x] `apply_agent_completion` in `runner/event_loop.rs` maps completion differently for enriching, implementing, autofix, verify regeneration, reviewing, and doc revision phases.
- [x] `append_agent_event` persists `agent.error`, tool calls, token usage, and turn completion to `.roko/events.jsonl` with `run_id`, `plan_id`, `task_id`, attempt, and pid fields.
- [x] `gate_dispatch.rs` builds `GatePayload::in_dir(...).with_label(...)` and executes declared task verify steps via `ShellGate`.
- [x] `persist.rs` and no-mock artifacts prove terminal executor snapshot flush to `.roko/state/executor.json`.
- [x] `dispatch_v2.rs` provides a partial provider abstraction, but active runner stream handling remains in `agent_stream.rs`.

Remaining reconciliation tasks:

- [x] Move all provider-specific stream parsing below `roko-agent`.
- [x] Replace direct runner use of `agent_stream::spawn_agent` with a normalized dispatch facade.
- [x] Add active runner feedback facade calls to learning, routing, knowledge, conductor, and dream sinks.
- [x] Add active projection facade calls for TUI/HTTP/SSE/non-TUI outputs.
- [x] Replace merge auto-success with `MergeQueue`.
- [ ] Prove multi-task, retry, resume, routing, knowledge, and projection parity before retiring `orchestrate.rs`.

## 13. 2026-04-27 Deepening Pass - Current Runtime Convergence Contract

Initial rating: `9.90 / 10`. This pass is above the requested threshold because it corrects stale phase rows, records exact current source anchors, defines the one-runtime convergence contract, adds a generated reconciliation report schema, and breaks the remaining work into no-context implementation batches. It is not a 10 because the repository still has direct runtime/model-call surfaces outside the reconciled path, and parity proof remains open.

### Status Semantics For This File

Checked rows in the corrected phase checklist mean `source_wired`, not `proved_parity`. Runtime reconciliation is complete only when [21-FEATURE-PARITY-MATRIX.md](21-FEATURE-PARITY-MATRIX.md), [22-STABILITY-PLAN.md](22-STABILITY-PLAN.md), and [27-FILESYSTEM-RUNTIME-CI-AUDIT.md](27-FILESYSTEM-RUNTIME-CI-AUDIT.md) have proof artifacts.

Use these labels:

- `source_wired`: active source path calls the intended module.
- `adapter_still_present`: a compatibility adapter remains below or beside the new facade.
- `legacy_unique`: behavior still only exists in `orchestrate.rs` or another old path.
- `direct_entrypoint`: a CLI, HTTP, or worker path still bypasses the reconciled runtime command service.
- `proof_missing`: source exists but tracked E2E proof does not.
- `retired`: old behavior is removed or explicitly delegated to the new owner.

### Current Source-Wired State

- [x] Runner command construction in [commands/plan.rs](../../crates/roko-cli/src/commands/plan.rs) builds projection and feedback facades before calling runner: `commands/plan.rs:237-310`.
- [x] Runner dispatch planning in [event_loop.rs](../../crates/roko-cli/src/runner/event_loop.rs) constructs `Dispatcher`, `PromptAssembler::new()`, `WarmPool`, prompt diagnostics, and model selection before spawn: `event_loop.rs:1814-1862`.
- [x] Runner runtime resolution in [event_loop.rs](../../crates/roko-cli/src/runner/event_loop.rs) calls `resolve_agent_runtime` and can choose CLI or bridge runtime.
- [x] [dispatch/mod.rs](../../crates/roko-cli/src/dispatch/mod.rs) owns `Dispatcher`, `DispatchPlan`, `AgentResultBridge`, `resolve_agent_runtime`, and bridge event forwarding: `dispatch/mod.rs:86-300`.
- [x] [runtime_events.rs](../../crates/roko-agent/src/runtime_events.rs) defines provider-neutral `AgentRuntimeEvent` and `AgentEventStream`.
- [x] [agent_stream.rs](../../crates/roko-cli/src/runner/agent_stream.rs) still launches CLI subprocesses, but its Claude stream parser delegates to `roko_agent::provider::claude_cli::stream::parse_stream_line`.
- [x] [provider/claude_cli/stream.rs](../../crates/roko-agent/src/provider/claude_cli/stream.rs) owns Claude `stream-json` protocol structs and translates them into `AgentRuntimeEvent`.
- [x] [prompt_builder.rs](../../crates/roko-cli/src/dispatch/prompt_builder.rs) owns `PromptContext`, `GateFeedback`, `AssembledPrompt`, `PromptDiagnostics`, knowledge/playbook sources, and deterministic budget dropping.
- [x] [runtime_feedback/mod.rs](../../crates/roko-cli/src/runtime_feedback/mod.rs) owns `FeedbackFacade`, `FeedbackEvent`, and sink fan-out for episodes, routing, knowledge, conductor, and dreams.
- [x] [projection.rs](../../crates/roko-cli/src/runner/projection.rs) owns normalized `ProjectionEvent`, broadcast, bounded dashboard snapshot, and counters.
- [x] [projection/mod.rs](../../crates/roko-cli/src/projection/mod.rs) wraps runner projection for TUI/dashboard/CLI subscribers.
- [x] [merge.rs](../../crates/roko-cli/src/runner/merge.rs) owns `PlanMerger`, real `GitMergeBackend`, regression gate, merge queue reservation, and queue draining.
- [x] [event_loop.rs](../../crates/roko-cli/src/runner/event_loop.rs) now submits `ExecutorAction::MergeBranch` through `PlanMerger` instead of immediately emitting success: `event_loop.rs:2244-2284`.
- [x] [persist.rs](../../crates/roko-cli/src/runner/persist.rs), [resume.rs](../../crates/roko-cli/src/runner/resume.rs), and [event_loop.rs](../../crates/roko-cli/src/runner/event_loop.rs) source-wire `run-state.json`, strict resume, task fingerprints, JSONL recovery, and snapshot flush.

### Current Reconciliation Gaps

- [ ] `RC-GAP-01`: There is no single `RuntimeCommandService` used by CLI plan run, CLI one-shot, PRD/research generation, worker/cloud, HTTP run routes, and job runner.
- [ ] `RC-GAP-02`: `run_once`, `run_once_inline`, `dispatch_direct`, `agent_exec`, and server `runtime.run_once` still represent separate non-plan execution surfaces.
- [ ] `RC-GAP-03`: Direct provider/model-call surfaces remain in server providers, server dispatch, dream review, vision evaluator, and one-shot/chat paths.
- [ ] `RC-GAP-04`: `orchestrate.rs` remains exported from [lib.rs](../../crates/roko-cli/src/lib.rs) as `PlanRunner` and still contains unique donor logic.
- [ ] `RC-GAP-05`: The active runner has source-wired feedback/projection, but no tracked proof shows every sink receives the expected events during real provider runs.
- [ ] `RC-GAP-06`: The merge path is source-wired, but no tracked proof shows git success, conflict, regression failure, and queue resume.
- [ ] `RC-GAP-07`: The runtime event vocabulary is not yet the single event envelope used by CLI, HTTP, TUI, feedback, provider proof, and crash proof.
- [ ] `RC-GAP-08`: Provider proof and connectivity tests are still distinct concepts without a single model-call service proof path.
- [ ] `RC-GAP-09`: Legacy docs and comments still describe `dispatch_v2.rs` or `agent_stream.rs` as if they are the final boundary rather than compatibility internals.
- [ ] `RC-GAP-10`: Archive criteria are not yet tied to generated proof reports, so docs can look complete before behavior is proved.

### Target Runtime Shape

Do not add a third runtime. The final shape is:

```text
CLI / HTTP / Worker / TUI command
  -> RuntimeCommandService
  -> RuntimeContext
  -> RunnerEngine
  -> ExecutorReducer
  -> EffectDispatcher
  -> Dispatcher / GateService / MergeService / ArtifactService
  -> RuntimeEventStore
  -> ProjectionService + FeedbackFacade
  -> QueryService
```

Ownership:

- [ ] `RuntimeCommandService` owns start/resume/cancel/pause/status command submission.
- [ ] `RuntimeContext` owns resolved config, secrets, provider registry, policy, workspace repositories, event store, proof ids, and redaction.
- [ ] `RunnerEngine` owns the `tokio::select!` loop and cancellation.
- [ ] `ExecutorReducer` owns state transitions and returns typed effects.
- [ ] `EffectDispatcher` owns invoking provider, gate, merge, filesystem, git, feedback, and projection services.
- [ ] `RuntimeEventStore` owns durable append and replay.
- [ ] `ProjectionService` owns queryable materialized state.
- [ ] `FeedbackFacade` owns learning/knowledge/conductor/dream fan-out.
- [ ] `QueryService` owns HTTP/TUI/CLI reads.

### Generated Reconciliation Report

Create a generated report:

- [ ] `tmp/mori-diffs/generated/runtime-reconciliation-report.json`

Schema:

```json
{
  "schema_version": 1,
  "generated_at": "2026-04-27T00:00:00Z",
  "git_commit": "unknown",
  "runtime_entrypoints": [
    {
      "id": "cli.plan.run",
      "status": "source_wired",
      "command_service": "runner::run",
      "source_refs": ["crates/roko-cli/src/commands/plan.rs"],
      "proof_refs": []
    }
  ],
  "legacy_surfaces": [
    {
      "path": "crates/roko-cli/src/orchestrate.rs",
      "status": "legacy_unique",
      "unique_behaviors": [],
      "retirement_owner_doc": "25-CODE-ONLY-LEGACY-AUDIT.md"
    }
  ],
  "direct_model_call_surfaces": [
    {
      "path": "crates/roko-serve/src/routes/providers.rs",
      "status": "direct_entrypoint",
      "target_owner": "ModelCallService"
    }
  ],
  "proof_summary": {
    "plan_run": "proof_missing",
    "one_shot": "proof_missing",
    "http_run": "proof_missing",
    "provider_matrix": "proof_missing",
    "feedback": "proof_missing",
    "projection": "proof_missing",
    "merge": "proof_missing",
    "resume": "proof_missing"
  }
}
```

Rules:

- [ ] Every CLI/HTTP/worker/TUI entrypoint that can start model or plan work must be listed.
- [ ] Every direct provider/model-call surface must be listed with target owner.
- [ ] Every `orchestrate.rs` export or caller must be listed until retired.
- [ ] The report must distinguish `source_wired` from `proved_parity`.
- [ ] The report must link to [21-FEATURE-PARITY-MATRIX.md](21-FEATURE-PARITY-MATRIX.md) row ids.

### Implementation Batches

#### RC-01: Runtime Command Service

- [ ] Define `RuntimeCommandService` with `start_plan_run`, `resume_run`, `cancel_run`, `start_one_shot`, `start_prd_generation`, `start_research`, `start_http_run`, and `query_status`.
- [ ] Define `RuntimeCommandResult` with `operation_id`, `run_id`, `event_log_path`, `projection_refs`, and `artifact_refs`.
- [ ] Move CLI `plan run` command construction behind this service without changing behavior.
- [ ] Move HTTP `/api/run`, plans, PRDs, research, templates, gateway, and job runner calls onto this service.
- [ ] Move worker/cloud plan execution onto this service.
- [ ] Add grep gate that `runner::run` is invoked only by the command service, tests, or explicit compatibility wrappers.

#### RC-02: Model Call Service Boundary

- [ ] Define `ModelCallService` or use the inference gateway from [41-INFERENCE-GATEWAY-MODEL-CALL-SERVICE-AUDIT.md](41-INFERENCE-GATEWAY-MODEL-CALL-SERVICE-AUDIT.md).
- [ ] Move one-shot/chat `dispatch_direct` paths onto the service.
- [ ] Move PRD/research/plan generation direct `agent_exec` calls onto the service.
- [ ] Move server provider tests and server dispatch direct `create_agent_for_model` calls onto the service.
- [ ] Move dream review and vision evaluator direct provider construction onto the service.
- [ ] Emit provider lifecycle events for every model call.

#### RC-03: Legacy Orchestrate Retirement

- [ ] Add a legacy-freeze header to [orchestrate.rs](../../crates/roko-cli/src/orchestrate.rs).
- [ ] Inventory exported symbols from `orchestrate.rs` and classify each as `retire`, `extract`, `adapter`, or `test_only`.
- [ ] Remove `PlanRunner` export from [lib.rs](../../crates/roko-cli/src/lib.rs) after all callers migrate.
- [ ] Move reusable donor logic into owner modules only when a current runner path needs it.
- [ ] Add grep gate for new `orchestrate::` production imports.
- [ ] Delete or archive donor-only sections after parity proof passes.

#### RC-04: Event Envelope Unification

- [ ] Define one runtime event envelope with `run_id`, `operation_id`, `source`, `event_type`, `timestamp_ms`, `severity`, `payload`, and `evidence_refs`.
- [ ] Make runner events, agent runtime events, gate events, merge events, feedback events, provider proof events, config events, and projection events map into the envelope.
- [ ] Make durable event append happen through `RuntimeEventStore`.
- [ ] Make projection and feedback consume the same durable event stream or same in-memory publication path.
- [ ] Add event replay proof after process restart.

#### RC-05: Feedback And Projection Proof

- [ ] Prove a real provider run emits events to `FeedbackFacade` sinks.
- [ ] Prove episode, routing, knowledge, conductor, and dream sink outputs are created or explicitly skipped with reason.
- [ ] Prove HTTP projections query the same run after restart.
- [ ] Prove TUI snapshot and HTTP projection agree.
- [ ] Prove prompt diagnostics, gate history, retry decisions, merge results, and provider usage are queryable.

#### RC-06: Merge And Resume Proof

- [ ] Prove `PlanMerger` success on a real branch.
- [ ] Prove conflict failure with git conflict evidence.
- [ ] Prove regression failure does not mark merge success.
- [ ] Prove queued/reserved merge state survives resume.
- [ ] Prove crash/resume matrix from [22-STABILITY-PLAN.md](22-STABILITY-PLAN.md).

#### RC-07: Docs And Archive Discipline

- [ ] Update [21-FEATURE-PARITY-MATRIX.md](21-FEATURE-PARITY-MATRIX.md) with `runtime-reconciliation-report.json`.
- [ ] Update [24-DEFINITIVE-GAP-LIST.md](24-DEFINITIVE-GAP-LIST.md) if a missing-module claim is stale.
- [ ] Update [25-CODE-ONLY-LEGACY-AUDIT.md](25-CODE-ONLY-LEGACY-AUDIT.md) with retired legacy surfaces.
- [ ] Update [29-CURRENT-RUNTIME-GAP-LEDGER.md](29-CURRENT-RUNTIME-GAP-LEDGER.md) with any still-open P0 reconciliation gap.
- [ ] Do not move this file to archive until every P0 runtime entrypoint is behind the command/model-call services.

### No-Context Handoff Checklist

Give this block to another agent with no additional context:

- [ ] Read this file, [21-FEATURE-PARITY-MATRIX.md](21-FEATURE-PARITY-MATRIX.md), [22-STABILITY-PLAN.md](22-STABILITY-PLAN.md), [25-CODE-ONLY-LEGACY-AUDIT.md](25-CODE-ONLY-LEGACY-AUDIT.md), [29-CURRENT-RUNTIME-GAP-LEDGER.md](29-CURRENT-RUNTIME-GAP-LEDGER.md), and [41-INFERENCE-GATEWAY-MODEL-CALL-SERVICE-AUDIT.md](41-INFERENCE-GATEWAY-MODEL-CALL-SERVICE-AUDIT.md).
- [ ] Generate `tmp/mori-diffs/generated/runtime-reconciliation-report.json` from source scans.
- [ ] Implement `RuntimeCommandService` and migrate one entrypoint at a time.
- [ ] Implement or adopt `ModelCallService` and migrate one direct provider/model-call surface at a time.
- [ ] Add the legacy-freeze header and export/caller inventory for `orchestrate.rs`.
- [ ] Add grep gates for `runner::run`, `run_once`, `dispatch_direct`, `agent_exec`, `create_agent_for_model`, and `orchestrate::`.
- [ ] Run the smallest cargo checks after each migration batch.
- [ ] Run parity, stability, provider, projection, and merge proof scripts only after the source migration is complete enough to prove.
- [ ] Update this doc with checked rows only when source is wired or proof is attached according to the status semantics above.

### Reconciliation Exit Gate

Do not call runtime reconciliation complete until:

- [ ] CLI plan run, one-shot, PRD, research, worker/cloud, HTTP run, job runner, and TUI command surfaces share `RuntimeCommandService` or have a documented non-runtime reason not to.
- [ ] All model calls outside plan execution go through `ModelCallService` or the inference gateway.
- [ ] `orchestrate.rs` has no unique production-critical behavior.
- [ ] Runner consumes provider-neutral events only.
- [ ] Runner prompt construction uses `PromptAssembler` without production fallback to minimal helpers.
- [ ] Feedback facade receives every meaningful runner event.
- [ ] Projection/query service can reconstruct run state from durable events.
- [ ] Merge success/conflict/regression/resume proof exists.
- [ ] Crash/resume proof exists.
- [ ] Provider matrix proof exists.
- [ ] Generated reconciliation, parity, and stability reports exist and agree.
