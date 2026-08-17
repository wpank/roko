# Workflow Artifact and Live State Notes

This file captures the current state of PRD, plan, task, and live execution surfaces in `roko`, plus the gaps needed for a UI that automatically renders PRDs, plans, generated tasks, gates, verification, and model routing as they appear.

## Short Answer

PRDs are queryable from the HTTP server today.

Plans and tasks are only partially queryable. The HTTP routes exist, but the current `roko-serve` plan/task API mostly understands older/simple `.roko/plans/{id}.toml` or `.json` files. It does not yet fully parse the modern workflow layout where generated work lands under `.roko/plans/{plan}/plan.md` and `.roko/plans/{plan}/tasks.toml`.

Live state is available today, but mostly through StateHub SSE/projection endpoints, not a first-class WebSocket artifact subscription. The existing `/ws` endpoint streams `ServerEvent` values from the serve event bus. The more useful workflow state is in StateHub and is exposed through `/api/events`, `/api/statehub/events`, `/api/statehub/snapshot`, and `/api/projections/{name}/stream`.

For the UI the user is describing, the best product shape is:

1. HTTP artifact APIs for current documents and task files.
2. A StateHub-backed subscription stream for deltas.
3. A file/artifact watcher that publishes StateHub updates when PRDs, plans, and `tasks.toml` files are created or changed.
4. A server-side execution path that uses the real runner/task graph, not only a prompt that asks an agent to execute a plan.

## Current HTTP Surfaces

### PRDs

Implemented in `crates/roko-serve/src/routes/prds.rs`.

Current routes:

```text
GET  /api/prds
POST /api/prds/ideas
GET  /api/prds/status
POST /api/prd/consolidate
POST /api/prds/consolidate
GET  /api/prds/{slug}
POST /api/prds/{slug}/draft
POST /api/prds/{slug}/promote
POST /api/prds/{slug}/plan
```

What works:

- `GET /api/prds` scans `.roko/prd/ideas`, `.roko/prd/drafts`, and `.roko/prd/published`.
- `GET /api/prds/{slug}` returns the PRD body and parsed frontmatter.
- `POST /api/prds/ideas` writes a new PRD idea markdown file under `.roko/prd/ideas`.
- `POST /api/prds/{slug}/draft` and `POST /api/prds/{slug}/plan` launch background agent operations.
- `POST /api/prds/{slug}/promote` moves a draft to published, records an episode, emits a `PrdPublished` audit event, and can auto-plan when enabled.

Current limitations:

- These routes expose PRD documents well, but do not provide an aggregate "workflow view" that joins PRD -> plan -> tasks -> execution state.
- PRD generation emits operation-level server events, not rich artifact-created deltas for every generated file.
- Startup StateHub seeding includes PRD summaries, but the task map is empty during bootstrap.

### Plans and Tasks

Implemented in `crates/roko-serve/src/routes/plans.rs`.

Current routes:

```text
GET  /api/plans
POST /api/plans
GET  /api/plans/{id}
GET  /api/plans/{id}/tasks
POST /api/plans/{id}/execute
GET  /api/plans/{id}/status
POST /api/plans/{id}/pause
POST /api/plans/{id}/resume
GET  /api/plans/{id}/gates
GET  /api/plans/{id}/reviews
POST /api/plans/{id}/tasks/{task_id}/review
GET  /api/plans/{id}/tasks/{task_id}/diff
POST /api/plans/{id}/chat
POST /api/plans/{id}/estimate
POST /api/plans/generate
```

What works:

- There is a plan API surface.
- `GET /api/plans/{id}/tasks` returns tasks for plans that fit the older route parser.
- `POST /api/plans/generate` asks the agent to create `plan.md` and `tasks.toml`.

Current limitations:

- `GET /api/plans` only scans immediate `.toml` and `.json` files under `.roko/plans`.
- `find_plan` only checks `.roko/plans/{id}.json` and `.roko/plans/{id}.toml`.
- The server route parser does not understand `.roko/plans/{id}/tasks.toml`.
- The local `RawTask` used by the serve routes only includes a small legacy subset: `id`, `description`, `depends_on`, `files`, and `completed`.
- Modern task fields such as `status`, `tier`, `model_hint`, `role`, `verify`, `acceptance`, `context`, `max_retries`, and `allowed_tools` are not returned by the HTTP plan/task routes.

This means the current plan/task HTTP server is not yet the right source of truth for the task UI the user wants.

## Modern Task Format

The richer task model already exists outside the serve plan routes.

Relevant files:

- `crates/roko-core/src/task.rs`
- `crates/roko-cli/src/task_parser.rs`

The modern `tasks.toml` parser supports:

- `meta.plan`
- `meta.iteration`
- `meta.total`
- `meta.done`
- `meta.status`
- `meta.max_parallel`
- `meta.estimated_total_minutes`
- `task[].id`
- `task[].title`
- `task[].description`
- `task[].role`
- `task[].status`
- `task[].tier`
- `task[].frequency`
- `task[].model_hint`
- `task[].replan_strategy`
- `task[].max_loc`
- `task[].files`
- `task[].allowed_tools`
- `task[].denied_tools`
- `task[].mcp_servers`
- `task[].depends_on`
- `task[].depends_on_plan`
- `task[].split_into`
- `task[].context`
- `task[].verify`
- `task[].timeout_secs`
- `task[].max_retries`
- `task[].acceptance`
- `task[].acceptance_contract`
- `task[].domain`

`verify` steps include:

- `phase`
- `command`
- `fail_msg`
- `timeout_ms`

This is the parser the HTTP server should use for modern workflow rendering.

## Current Live State Surfaces

### StateHub

StateHub is implemented in `crates/roko-core/src/state_hub.rs`.

The server owns a StateHub instance through `crates/roko-serve/src/state.rs`. It stores a materialized snapshot, broadcasts `DashboardEvent` values, retains a replay ring, and can persist events to `.roko/events.jsonl`.

At server startup, `crates/roko-serve/src/lib.rs` calls:

- `state.state_hub.bootstrap_from_workdir(&state.workdir)`
- marketplace job seeding
- PRD seeding into `DashboardEvent::AtelierPrdsUpdated`

This is the right backbone for the UI subscription model.

### StateHub HTTP and SSE

Implemented in:

- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/status/health.rs`
- `crates/roko-serve/src/routes/projections.rs`

Current endpoints:

```text
GET /api/events
GET /api/sse
GET /api/statehub/snapshot
GET /api/statehub/events
GET /api/projections/catalog
GET /api/projections/{name}
GET /api/projections/{name}/stream
```

Useful details:

- `/api/events` and `/api/sse` stream raw StateHub `DashboardEvent` values over SSE.
- `/api/statehub/snapshot` returns the current dashboard projection state.
- `/api/statehub/events` returns retained StateHub events and supports filters such as `after_seq`, `limit`, `run_id`, `plan_id`, `task_id`, and `event_type`.
- `/api/projections/{name}` returns the current materialized projection state.
- `/api/projections/{name}/stream` sends an initial `state` SSE event and then `delta` SSE events.

Useful projection names include:

- `dashboard`
- `agent_state`
- `plan_state`
- `active_tasks`
- `gate_state`
- `plans_list`
- `executor_state`
- `marketplace_jobs`
- `prds`

The existing projection stream is the closest thing today to "subscribe to StateHub and update the UI when things change."

### Existing WebSocket

Implemented in `crates/roko-serve/src/routes/ws.rs`.

Current routes:

```text
GET /ws
GET /roko-ws
```

Current client message shape:

```json
{
  "subscribe": ["projection:gate_pipeline", "topic:agent.*"],
  "cursor": 42,
  "back_pressure": "at_most_once"
}
```

Important limitations:

- This endpoint streams `ServerEvent` values from `state.event_bus`, not raw StateHub `DashboardEvent` values.
- On connect, it replays the retained server event bus before reading the subscribe message.
- The `projection:` and `topic:` filters are string filters over event type names. They do not create true StateHub projection subscriptions.
- The disabled StateHub-to-ServerEvent bridge means not every StateHub event will appear on `/ws`.

So `/ws` exists, but it is not currently the right API for the artifact subscription UX.

## Execution State and Runner Gap

The real runner publishes rich task state through StateHub.

Relevant files:

- `crates/roko-cli/src/runner/mod.rs`
- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-cli/src/tui_bridge.rs`

The runner can publish:

- `plan_started`
- `task_started`
- `task_phase_changed`
- `task_completed`
- `agent_spawned`
- `agent_output`
- `gate_result`
- `phase_transition`
- `efficiency_event`
- model selection entries

However, there is a practical gap between CLI runner execution and the HTTP server StateHub.

`crates/roko-cli/src/commands/plan.rs` creates or uses a CLI-local StateHub. `crates/roko-cli/src/commands/util.rs` contains a TODO noting that when `--serve` is active, DashboardEvents should flow into the server's StateHub/SSE/WebSocket/snapshot surfaces. Currently the CLI and serve StateHub types are distinct enough that the bridge is not complete.

There is another execution gap in `crates/roko-serve/src/routes/plans.rs`: `POST /api/plans/{id}/execute` starts an operation and asks the runtime to read and execute the plan, but it does not invoke the modern task runner directly. As a result, the server may only know about plan start/completion unless some other bridge emits task-level events.

For the requested UI, server-side execution should run the same task graph implementation that the CLI runner uses, wired into the server's StateHub.

## What Is Needed

### 1. Modern Artifact APIs

Add or update serve routes so the UI can query current workflow artifacts directly.

Recommended endpoints:

```text
GET /api/workflows
GET /api/workflows/{id}
GET /api/workflows/{id}/prd
GET /api/workflows/{id}/plan
GET /api/workflows/{id}/tasks
GET /api/workflows/{id}/events
```

Or, minimally fix existing routes:

```text
GET /api/plans
GET /api/plans/{id}
GET /api/plans/{id}/tasks
```

Required behavior:

- Scan both legacy files and modern plan directories.
- For `.roko/plans/{id}/plan.md`, return markdown body and metadata.
- For `.roko/plans/{id}/tasks.toml`, use the modern task parser from `roko-cli/src/task_parser.rs` or move that parser into `roko-core` so both CLI and serve can share it.
- Return full task fields needed by the UI: status, tier, model hint, role, dependencies, files, verification steps, acceptance, domain, tools, and retry policy.
- Include stable IDs so live StateHub task events can be merged into the artifact model.

### 2. Artifact Watcher

The server should publish StateHub deltas when workflow files are created or changed.

Watch:

```text
.roko/prd/**/*.md
.roko/plans/*/plan.md
.roko/plans/*/tasks.toml
```

On startup:

- Scan the workspace and publish the current artifact state.

On file create/update:

- Parse the changed artifact.
- Publish a typed StateHub event.
- Update the relevant projection.

Possible new events:

```text
workflow_prd_updated
workflow_plan_updated
workflow_tasks_updated
workflow_artifact_deleted
```

Alternative:

- Expand `AtelierPrdsUpdated` to carry plan and task data.
- Add corresponding `AtelierPlansUpdated` and `AtelierTasksUpdated` events.

### 3. StateHub-Backed Subscription

SSE already works for this shape:

```text
GET /api/projections/prds/stream
GET /api/projections/plan_state/stream
GET /api/projections/active_tasks/stream
GET /api/projections/gate_state/stream
```

If WebSocket is required, add a new StateHub-backed WebSocket rather than overloading the existing `/ws` semantics.

Suggested endpoint:

```text
GET /api/workflow/ws
```

Suggested subscribe message:

```json
{
  "type": "subscribe",
  "channels": [
    "workflow.prds",
    "workflow.plans",
    "workflow.tasks",
    "workflow.execution",
    "workflow.gates"
  ],
  "cursor": 0
}
```

Suggested server frames:

```json
{
  "type": "state",
  "channel": "workflow.tasks",
  "seq": 101,
  "data": {}
}
```

```json
{
  "type": "delta",
  "channel": "workflow.execution",
  "seq": 102,
  "event": {
    "type": "task_phase_changed",
    "task_id": "T-002",
    "phase": "verify"
  }
}
```

Required behavior:

- Send initial state after subscription, not before.
- Support replay from cursor.
- Allow filtering by workflow ID, plan ID, task ID, and event type.
- Use StateHub as the backing source, not the older serve event bus.

### 4. Server Execution Should Use the Real Runner

For end-to-end demos, `POST /api/plans/{id}/execute` should not only prompt an agent to read a plan. It should execute the parsed `tasks.toml` through the same runner path that powers task phases, gates, retries, model routing, and verification.

Needed work:

- Move shared task parsing into `roko-core` or a shared crate.
- Add a serve-side execution adapter that loads `.roko/plans/{id}/tasks.toml`.
- Pass the server's `state.state_hub` into the runner/event bridge.
- Emit every task phase, gate result, and model routing decision into the same StateHub instance used by HTTP/SSE/WebSocket.
- Keep the existing server `event_bus` bridge only for compatibility.

### 5. UI Data Flow

Recommended UI boot sequence:

```text
1. Fetch /api/projections/prds or /api/workflows.
2. Fetch selected PRD body from /api/prds/{slug}.
3. Fetch selected plan and tasks from /api/workflows/{id} or fixed /api/plans/{id}.
4. Open StateHub subscription stream.
5. Merge artifact deltas and execution deltas by PRD slug, plan ID, and task ID.
```

Current SSE option:

```text
EventSource('/api/projections/prds/stream')
EventSource('/api/projections/plan_state/stream')
EventSource('/api/projections/active_tasks/stream')
EventSource('/api/projections/gate_state/stream')
```

Future WebSocket option:

```text
WebSocket('/api/workflow/ws')
```

The UI should render:

- PRD document cards and full markdown body.
- Plan document body.
- Task graph or task list from `tasks.toml`.
- Task state: pending, active, done, blocked.
- Phases: planning, implementing, verifying, reviewing, complete.
- Gates and verification commands.
- Model routing: tier, model hint, selected model/provider, reasoning level, escalation.
- Live log entries and agent output.

## Recommended Implementation Order

1. Move or expose the modern `tasks.toml` parser to `roko-serve`.
2. Fix `/api/plans`, `/api/plans/{id}`, and `/api/plans/{id}/tasks` to read `.roko/plans/{id}/plan.md` and `.roko/plans/{id}/tasks.toml`.
3. Add one aggregate endpoint for UI simplicity, such as `GET /api/workflows/{id}`.
4. Add an artifact scanner/watcher that publishes StateHub events when PRDs, plans, and tasks appear.
5. Expand projections or add a new `workflow` projection.
6. Add a StateHub-backed WebSocket only if SSE is not sufficient for the demo UI.
7. Change server plan execution to call the real runner so task phases, gates, and model routing are real events.

## Practical Demo Path

For the near-term demo UI, the fastest useful path is:

1. Use existing PRD routes for PRD document creation and retrieval.
2. Patch plan/task HTTP parsing to understand modern plan directories.
3. Use `/api/projections/{name}/stream` SSE for live state instead of waiting for a new WebSocket.
4. Render generated files immediately after they appear by polling or by an artifact watcher.
5. Wire execution to StateHub events so UI state changes are real, not simulated.

The demo can still show three examples:

- A super simple CLI task.
- A slightly more complex integration task.
- The stage job: "Build a CLI that fetches BTC funding rates from Hyperliquid and emails me an alert when funding flips negative."

For each example, the UI should make the differences visible:

- Number of generated tasks.
- Dependency graph shape.
- Verification gates.
- Model tier routing, such as T1/T2/T3 or `model_hint`.
- Tool and integration requirements.
- State transitions from pending -> active -> verify/gate -> done.

## Answer To The User's Core Question

Are tasks, plans, and PRDs available to query from the agent HTTP server?

- PRDs: yes, mostly.
- Plans: partially.
- Tasks: partially for older/simple plan files, not fully for modern generated `tasks.toml`.
- Live state: yes through StateHub SSE/projections, but not through a clean StateHub-backed WebSocket yet.

What is needed?

- Teach the HTTP server to parse modern plan directories and `tasks.toml`.
- Add artifact update events into StateHub.
- Use StateHub projection streams or add a new StateHub-backed workflow WebSocket.
- Run generated plans through the real runner so task states, gates, and model routing events are emitted as first-class live data.

---

# Expanded Workflow Architecture Context

This section folds in the broader UX audit, the unified Lens/StateHub/Surface spec, the current demo implementation, and the Symphony/OpenAI orchestration direction.

The key distinction:

- The current code has real StateHub, real DashboardEvents, real projection routes, real PRD routes, and a real runner that can publish task lifecycle events.
- The current code does not yet have a unified Workflow/Board/Epic/Task data model, a modern plan/task HTTP API, first-class artifact change events, or a StateHub-backed workflow WebSocket.
- The unified docs describe a more general Lens architecture, but the current crate implementation only has a simple `Observe` trait and no concrete Lens runtime such as `CostLens`, `QualityLens`, `TrendLens`, `AnomalyLens`, `LensScope`, or `ObservableEvent`.
- The current PRD pipeline demo has useful UI pieces, but it is not using the right data path. It shells into a terminal, scrapes `.roko` files with Python, and sometimes projects active task state in the UI. That should be replaced with server artifact APIs and StateHub projection streams.

## Related Local Documents

Important local files:

- `tmp/subsystem-audits/ux/AUDIT.md`
- `tmp/subsystem-audits/ux/GOALS.md`
- `tmp/subsystem-audits/ux/PLAN.md`
- `tmp/subsystem-audits/ux/FEATURES.md`
- `tmp/subsystem-audits/ux/ISSUES.md`
- `tmp/subsystem-audits/ux/MORI-REFERENCE.md`
- `tmp/subsystem-audits/ux/SYMPHONY-ANALYSIS.md`
- `tmp/subsystem-audits/ux/TRACKER-INTEGRATIONS.md`
- `tmp/unified/00-INDEX.md`
- `tmp/unified/02-CELL.md`
- `tmp/unified/15-TELEMETRY.md`
- `tmp/unified/20-SURFACES.md`
- `tmp/unified/27-ORCHESTRATOR.md`

Important code files:

- `crates/roko-core/src/state_hub.rs`
- `crates/roko-core/src/dashboard_snapshot.rs`
- `crates/roko-core/src/traits.rs`
- `crates/roko-cli/src/task_parser.rs`
- `crates/roko-cli/src/runner/mod.rs`
- `crates/roko-cli/src/runner/plan_loader.rs`
- `crates/roko-cli/src/runner/task_dag.rs`
- `crates/roko-cli/src/runner/tui_bridge.rs`
- `crates/roko-cli/src/commands/util.rs`
- `crates/roko-serve/src/routes/prds.rs`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/projection_contract.rs`
- `crates/roko-serve/src/truth_map.rs`
- `crates/roko-serve/src/parity.rs`
- `demo/demo-app/src/components/PrdPipelinePanel.tsx`
- `demo/demo-app/src/lib/scenarios.ts`
- `demo/demo-app/src/lib/prd-pipeline-sample.ts`

## UX Audit Takeaways

The UX audit describes Roko as having a lot of surface area but no coherent shared workflow model yet.

Current surfaces:

- TUI dashboard with F1-F10 tabs.
- Inline chat modes.
- HTTP control plane.
- ACP/editor integration.
- Demo web/app.

The high-value target is:

```text
Board -> Epic -> Task
  plus PRD -> Plan -> Tasks
  plus live execution/gates/routes/agents
  rendered consistently by TUI, web, editor, and CLI.
```

The audit's critical gaps:

- No canonical task data model shared across CLI, serve, TUI, and web.
- No Board/Epic/Task store.
- Task details are too thin.
- No task lifecycle UX for creation, enrichment, editing, dependency visualization, or batch operations.
- TUI and web do not share a ViewModel.
- Built inline primitives are mostly unwired.
- The demo app is a demo surface, not the product workflow surface.

The audit's architectural recommendation:

```text
roko-tasks or roko-workflow crate
  -> canonical Board/Epic/Task/Workflow types
  -> validation and DAG computation
  -> event emission
  -> atomic persistence
  -> REST API through roko-serve
  -> StateHub projections for every surface
```

## Unified Spec Takeaways

The unified docs provide the generalized vocabulary.

Core primitives:

- `Signal`: durable, content-addressed data in Store.
- `Pulse`: ephemeral event on Bus.
- `Cell`: atomic computation.
- `Graph`: DAG of Cells.
- `Protocol`: behavior contract for Cells.

The relevant protocol is `Observe`.

Spec shape:

```rust
pub trait ObserveProtocol: Cell {
    async fn observe(&self, ctx: &ObserveContext) -> Result<Vec<Signal>>;
}
```

Telemetry doc shape:

```rust
pub trait Observe: Cell {
    async fn observe(&self, event: &ObservableEvent) -> Result<Vec<Signal>>;
    fn observes(&self) -> &[ObservableEventKind];
    fn scope(&self) -> LensScope;
}
```

The current code shape is much smaller:

```rust
pub trait Observe: crate::cell::Cell {
    fn observe(&self) -> Vec<Engram>;
}
```

That means the Lens system in the unified docs is mostly future architecture, not current implementation.

The intended Lens pipeline:

```text
Runtime events
  -> ObservableEvent
  -> scoped read-only Lenses
  -> observation Signals
  -> StateHub projection builders
  -> typed projections
  -> surfaces
```

Built-in Lenses described by the spec:

- `CostLens`
- `LatencyLens`
- `QualityLens`
- `EfficiencyLens`
- `ErrorLens`
- `DriftLens`
- `BudgetLens`
- `TrendLens`
- `AnomalyLens`
- `UsageLens`
- `CollectiveIntelligenceLens`

Composition modes:

- Stacking: multiple Lenses observe the same target.
- Chaining: one Lens observes another Lens's output.
- Scoping: Cell, Graph, Agent, Space, Lens, Global.

What exists today:

- StateHub.
- DashboardSnapshot.
- DashboardEvent.
- projection routes.
- projection catalog and invalidation policy.
- materialized projections such as `plan_state`, `active_tasks`, `gate_state`, `cost_state`, `prds`.

What does not exist yet:

- `ObservableEvent` enum matching the unified spec.
- `LensScope`.
- `LensRegistry`.
- built-in Lens implementations.
- observation Signal kinds for cost/quality/error/trend/anomaly.
- TOML-configured Lens attachment to Graphs.
- StateHub projection builders that consume Lens output rather than direct DashboardEvents.

## Current StateHub Reality

`StateHub` is currently the best foundation for the workflow UI.

Current data flow:

```text
ServerEvent or runner event
  -> DashboardEvent
  -> StateHub::publish
  -> DashboardSnapshot::apply
  -> watch channel + event ring + optional .roko/events.jsonl
  -> projection REST/SSE or TUI watch receiver
```

Important current DashboardEvents:

- `PlanStarted`
- `PlanCompleted`
- `TaskStarted`
- `TaskCompleted`
- `TaskPhaseChanged`
- `AgentSpawned`
- `AgentOutput`
- `GateResult`
- `PhaseTransition`
- `EfficiencyEvent`
- `EpisodeRecorded`
- `TaskOutputAppended`
- `EventLogEntry`
- `CascadeRouterUpdated`
- `GateThresholdsUpdated`
- `MarketplaceJobsUpdated`
- `AtelierPrdsUpdated`
- `KnowledgeEntriesUpdated`

Current projection endpoints:

```text
GET /api/projections/catalog
GET /api/projections/{name}
GET /api/projections/{name}/stream
```

Current useful projections:

- `dashboard`
- `agent_state`
- `plan_state`
- `active_tasks`
- `gate_state`
- `learning_policy_state`
- `cohort_health`
- `alerts`
- `recent_episodes`
- `event_log`
- `task_outputs`
- `cost_meter`
- `cost_state`
- `provider_state`
- `retry_state`
- `execution_trace`
- `runtime_feedback`
- `executor_state`
- `marketplace_jobs`
- `prds`
- `knowledge`

Current StateHub issue:

- CLI runner events and server StateHub are not fully converged in every path.
- `crates/roko-cli/src/commands/util.rs` still has a TODO for sharing the server StateHub when `--serve` is active.
- `roko-serve` intentionally does not start the reverse StateHub-to-EventBus bridge because it can cause feedback loops.

## Current Demo Reality

The PRD pipeline demo is visually useful but architecturally wrong for the final product.

Current demo flow in `demo/demo-app/src/lib/scenarios.ts`:

```text
terminal command runs roko prd idea
terminal command runs roko prd draft new
terminal command runs roko prd draft promote
terminal command runs roko prd plan
terminal command runs roko plan run
monitor terminal runs Python that scans .roko/prd and .roko/plans
React state is patched from scraped JSON
if no active task appears, UI marks a pending task active for presentation
```

What that proves:

- The panel can render PRDs, plans, tasks, gates, and model routing.
- The three examples are already represented:
  - super simple status CLI
  - GitHub release watcher
  - BTC funding alert stage job

What it does not prove:

- Real server artifact APIs.
- Real StateHub subscription.
- Real workflow projections.
- Real task phase updates from server execution.
- Real model route events.

Final product flow should be:

```text
UI starts PRD/plan/task operations via HTTP
server writes artifacts and publishes artifact events
server exposes workflow projection
UI subscribes to workflow projection stream
runner emits task/gate/agent/model events into server StateHub
UI merges artifact state and execution state by workflow_id/plan_id/task_id
```

## Target Workflow Abstraction

The right domain model is broader than "plans" and narrower than "all of Roko."

Recommended crate:

```text
crates/roko-workflow
```

or:

```text
crates/roko-tasks
```

Core types:

```rust
pub struct Workflow {
    pub id: WorkflowId,
    pub title: String,
    pub source: WorkflowSource,
    pub prd: Option<PrdDoc>,
    pub plans: Vec<PlanDoc>,
    pub status: WorkflowStatus,
    pub execution: Option<WorkflowExecutionState>,
}

pub enum WorkflowSource {
    RokoPrd { slug: String },
    Board { board_id: String },
    Linear { issue_id: String },
    GitHub { issue_number: u64 },
    WorkflowMd { path: PathBuf },
    Webhook { source: String, external_id: String },
}

pub struct PrdDoc {
    pub slug: String,
    pub title: String,
    pub status: PrdStatus,
    pub path: PathBuf,
    pub frontmatter: serde_json::Value,
    pub body_markdown: String,
    pub requirements: Vec<Requirement>,
    pub acceptance: Vec<AcceptanceCriterion>,
}

pub struct PlanDoc {
    pub id: String,
    pub title: String,
    pub path: PathBuf,
    pub plan_markdown: Option<String>,
    pub task_file: Option<TasksFile>,
    pub task_graph: TaskGraph,
}

pub struct TaskNode {
    pub id: String,
    pub global_id: GlobalTaskId,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub role: Option<String>,
    pub tier: String,
    pub model_hint: Option<String>,
    pub selected_model: Option<String>,
    pub selected_provider: Option<String>,
    pub files: Vec<String>,
    pub depends_on: Vec<GlobalTaskId>,
    pub verify: Vec<VerifyStep>,
    pub acceptance: Vec<String>,
    pub context: Option<TaskContext>,
    pub execution: Option<TaskExecutionState>,
}
```

Status model:

```text
WorkflowStatus:
  idea -> drafting -> published -> planning -> tasks_ready -> running -> review -> done | failed

PlanStatus:
  pending -> implementing -> gating -> verifying -> reviewing -> ready -> merging -> complete | failed

TaskStatus:
  pending -> enriching -> ready -> active -> gating -> done | failed | blocked | skipped
```

IDs:

```text
workflow_id = stable slug or external source id
plan_id = directory name or external epic id
task_id = local id, such as T1
global_task_id = plan_id:task_id
```

This avoids collisions and enables cross-plan task DAGs.

## Target Event Taxonomy

Current DashboardEvent can be extended, but it is probably cleaner to define workflow-specific events and then map them into DashboardEvent-compatible snapshots.

Recommended workflow event names:

```text
workflow.created
workflow.updated
workflow.phase_changed
workflow.deleted

artifact.prd_created
artifact.prd_updated
artifact.prd_published
artifact.plan_created
artifact.plan_updated
artifact.tasks_created
artifact.tasks_updated
artifact.deleted

task.created
task.updated
task.status_changed
task.phase_changed
task.blocked
task.completed
task.failed
task.skipped

dag.recomputed
dag.ready_frontier_changed
dag.critical_path_changed

gate.started
gate.completed
gate.failed

agent.assigned
agent.started
agent.output
agent.completed

route.requested
route.selected
route.escalated

proposal.created
proposal.approved
proposal.rejected
```

Suggested StateHub mapping:

```text
WorkflowEvent::TaskStatusChanged
  -> DashboardEvent::TaskStarted / TaskPhaseChanged / TaskCompleted

WorkflowEvent::GateCompleted
  -> DashboardEvent::GateResult

WorkflowEvent::AgentOutput
  -> DashboardEvent::AgentOutput

WorkflowEvent::ArtifactPrdUpdated
  -> DashboardEvent::AtelierPrdsUpdated or new DashboardEvent::WorkflowArtifactsUpdated
```

The long-term Lens-compatible shape:

```text
WorkflowEvent
  -> ObservableEvent::WorkflowLifecycle
  -> WorkflowLens/TaskLens/GateLens/RouterLens
  -> Observation Signals
  -> StateHub projections
```

## Target Projections

Add a `workflow` projection family.

Recommended projections:

```text
workflow_overview
workflow_detail
workflow_artifacts
workflow_task_graph
workflow_active_tasks
workflow_gate_matrix
workflow_model_routing
workflow_agent_streams
workflow_timeline
workflow_inbox
```

Example `workflow_detail`:

```json
{
  "workflow_id": "btc-funding-alert-cli",
  "title": "BTC Funding Alert CLI",
  "source": {
    "kind": "roko_prd",
    "slug": "btc-funding-alert-cli"
  },
  "phase": "running",
  "prd": {
    "slug": "btc-funding-alert-cli",
    "status": "published",
    "title": "BTC Funding Alert CLI",
    "body_markdown": "..."
  },
  "plans": [
    {
      "id": "btc-funding-alert-cli",
      "title": "BTC Funding Alert CLI",
      "plan_markdown": "...",
      "tasks": []
    }
  ],
  "execution": {
    "tasks_total": 8,
    "tasks_done": 3,
    "tasks_active": 2,
    "gates_passed": 4,
    "gates_failed": 0,
    "cost_usd": 0.42
  }
}
```

Example `workflow_task_graph`:

```json
{
  "workflow_id": "btc-funding-alert-cli",
  "nodes": [
    {
      "id": "btc-funding-alert-cli:T1",
      "plan_id": "btc-funding-alert-cli",
      "task_id": "T1",
      "title": "Implement Hyperliquid API client",
      "status": "active",
      "tier": "T2",
      "model_hint": "claude-sonnet",
      "selected_model": "claude-sonnet-4-6",
      "files": ["src/hyperliquid.rs"],
      "verify": [
        { "phase": "test", "command": "cargo test hyperliquid_fixture" }
      ]
    }
  ],
  "edges": [
    { "from": "btc-funding-alert-cli:T1", "to": "btc-funding-alert-cli:T2", "kind": "depends_on" }
  ],
  "ready_frontier": ["btc-funding-alert-cli:T3"],
  "critical_path": ["btc-funding-alert-cli:T1", "btc-funding-alert-cli:T2", "btc-funding-alert-cli:T5"]
}
```

## Integration Option A: Minimal Demo Fix

This is the smallest path that makes the demo honest.

Build:

- Fix server plan/task parsing for modern plan directories.
- Add `GET /api/workflows` and `GET /api/workflows/{id}` as read-only aggregate endpoints.
- Add `GET /api/workflows/{id}/stream` as SSE, backed by existing StateHub where possible.
- Update demo app to call those endpoints instead of terminal scraping.
- Keep command execution through terminal panes for now.
- Remove projected/simulated active task state from the UI.

Pros:

- Fast.
- Makes the UI intuitive.
- Avoids building the whole Board/Epic/Task store first.
- Reuses existing PRD routes and projection routes.

Cons:

- Still file-backed and partly polling/watch based.
- Does not solve click-to-execute.
- Does not solve external trackers.
- Does not implement full Lens architecture.

Estimated work:

- Small to medium.
- Mostly `roko-serve` routes plus React API hook changes.

Best when:

- The immediate goal is a credible end-to-end demo.

## Integration Option B: Production Workflow Projection

This turns PRD/plan/task artifacts into first-class StateHub projections.

Build:

- Shared workflow parser crate.
- Artifact scanner on startup.
- Artifact watcher for `.roko/prd/**/*.md`, `.roko/plans/*/plan.md`, `.roko/plans/*/tasks.toml`.
- New workflow DashboardEvents or WorkflowEvents.
- New projection contract entries.
- New `workflow_*` projections.
- UI subscribes to projection SSE.

Pros:

- Gives all surfaces one consistent source.
- Makes artifact updates automatic.
- Fits the current StateHub design.
- Can later be fed by real Lenses.

Cons:

- Requires careful event naming and source-of-truth rules.
- Requires de-duplication with existing `prds`, `plan_state`, and `active_tasks` projections.

Estimated work:

- Medium.

Best when:

- The goal is production-quality workflow viewing across web and TUI.

## Integration Option C: StateHub-Backed WebSocket

This adds the subscription model the user originally described.

Build:

```text
GET /api/workflow/ws
```

Subscribe:

```json
{
  "type": "subscribe",
  "projections": [
    "workflow_detail",
    "workflow_task_graph",
    "workflow_gate_matrix",
    "workflow_model_routing"
  ],
  "workflow_id": "btc-funding-alert-cli",
  "cursor": 0
}
```

Frames:

```json
{
  "type": "state",
  "projection": "workflow_task_graph",
  "cursor": "0x20",
  "data": {}
}
```

```json
{
  "type": "delta",
  "projection": "workflow_task_graph",
  "cursor": "0x21",
  "event": {}
}
```

Pros:

- More ergonomic than multiple EventSource connections.
- Allows one subscription handshake for many projections.
- Better for interactive surfaces.

Cons:

- SSE already works for many use cases.
- Requires a replay/cursor protocol.
- Must avoid the current `/ws` mistake of sending backlog before subscription.

Estimated work:

- Medium.

Best when:

- The UI needs a single connection and richer subscription controls.

## Integration Option D: Full Board/Epic/Task Store

This implements the UX audit's long-term core.

Build:

```text
crates/roko-tasks or crates/roko-workflow
  Board
  Epic
  Task
  TaskMetadata
  TaskDAG
  Queue
  Proposal
  ExecutionState
  TrackerMapping
```

Persistence:

```text
.roko/boards/{board_id}/board.toml
.roko/boards/{board_id}/epics/{epic_id}.toml
.roko/boards/{board_id}/tasks/{task_id}.toml
.roko/boards/{board_id}/events.jsonl
```

or:

```text
.roko/workflows/{workflow_id}/workflow.toml
.roko/workflows/{workflow_id}/prd.md
.roko/workflows/{workflow_id}/plans/{plan_id}/plan.md
.roko/workflows/{workflow_id}/plans/{plan_id}/tasks.toml
```

APIs:

```text
POST   /api/boards
GET    /api/boards
GET    /api/boards/{id}
POST   /api/boards/{id}/epics
GET    /api/epics/{id}
POST   /api/epics/{id}/tasks
PATCH  /api/tasks/{id}
DELETE /api/tasks/{id}
POST   /api/tasks/{id}/enrich
POST   /api/tasks/{id}/execute
GET    /api/boards/{id}/dag
GET    /api/boards/{id}/stream
```

Pros:

- Best product architecture.
- Enables click-to-execute.
- Enables Kanban, DAG, queue, batch review, external tracker sync.
- Creates a durable model beyond PRD-generated plans.

Cons:

- Larger project.
- Needs migration decisions for existing `.roko/plans`.
- Needs clear source-of-truth rules.

Estimated work:

- Large.

Best when:

- The goal is Roko as an autonomous work operating system, not only a PRD demo.

## Integration Option E: Lens-First Generalization

This implements the unified Lens concept first, then builds workflow UX on top.

Build:

- `ObservableEvent`.
- `ObservableEventKind`.
- `LensScope`.
- `Lens`/`Observe` trait with scoped async event observation.
- `LensRegistry`.
- Lens config parsing from TOML.
- Lens invocation runtime.
- Observation Signal kinds.
- StateHub projection builders that consume Lens Signals.
- Built-in Lenses such as Cost, Quality, Gate, Task, Router, Agent, Error.

Pros:

- Most aligned with unified architecture.
- Generalizes beyond workflow to all observability.
- Clean separation: Lenses observe, StateHub projects, surfaces render.

Cons:

- Big foundation project.
- Slower path to fixing the demo.
- Requires event normalization across many crates.

Estimated work:

- Large.

Best when:

- The goal is to implement the unified telemetry architecture, not only workflow.

Pragmatic recommendation:

- Do not block the workflow UI on Lens-first architecture.
- Build workflow projections now using DashboardEvents.
- Design event names and projection shapes so they can later be fed by real Lenses.

## Integration Option F: Symphony-Compatible Mode

This implements OpenAI Symphony-style orchestration as an alternate entry point.

Command:

```bash
roko symphony WORKFLOW.md
```

Core idea:

```text
External tracker or board is the control plane.
Tasks in ready states are candidates.
Roko claims work, creates isolated workspace, runs agent, opens PR, updates review state.
```

This can be implemented at different compatibility levels.

### F1: Strict Symphony-Compatible

Implement the simplest shape:

- Parse Symphony-style `WORKFLOW.md`.
- Use a Linear adapter.
- Use Codex app-server/app-client compatibility where needed.
- One agent per issue.
- No DAG.
- No Roko gates except optional hooks.
- Agent is responsible for tracker writes, matching the Symphony philosophy.

Pros:

- Easiest for Symphony users.
- Drop-in conceptual replacement.
- Useful for teams already using Linear and Codex.

Cons:

- Leaves Roko's DAG/gate/learning advantages unused.
- More Codex/OpenAI-specific.

### F2: Roko-Native Symphony Pattern

Borrow the UX, not the limitations.

- `WORKFLOW.md` maps into Roko config.
- Tracker issues become Roko WorkItems.
- Roko enriches issues into tasks.
- Roko computes DAG where possible.
- Roko runs gate pipeline.
- Roko uses cascade routing and multi-backend dispatch.
- Roko updates external tracker according to configured policy.

Pros:

- Best blend of Symphony simplicity and Roko power.
- Keeps Roko differentiated.

Cons:

- Not strict compatibility.
- Requires clear docs for behavior differences.

### F3: Linear as External Roko Board

Treat Linear as a frontend to the Roko Board/Epic/Task store.

Mapping:

```text
Linear Workspace/Project -> Roko Board
Linear Issue             -> Roko Epic or Task
Linear Sub-issue         -> Roko Task
Linear State             -> Roko status
Linear Labels            -> Roko tags/domain/tier
Linear Priority          -> Roko queue priority
Linear Comments          -> Roko event/proposal/review history
```

Pros:

- Lets teams use Linear as the UI.
- Makes Roko the execution engine.
- Similar user experience to Symphony.

Cons:

- Bidirectional sync is harder than read-only polling.
- Conflict resolution needs care.

### F4: Agent App-Server Backend Adapter

OpenAI's Symphony materials describe a Codex app-server/app-client style where the orchestrator and agent communicate over a structured protocol. Roko can support this as one backend, not the only backend.

Build:

```text
AgentBackend::CodexAppServer
  start server process
  handshake
  send task/run request
  stream events
  expose tracker/tool calls
  terminate or resume sessions
```

Pros:

- Lets Roko run Codex/Symphony-compatible workers.
- Keeps backend abstraction open.

Cons:

- Need to avoid baking OpenAI-only assumptions into the workflow core.

## Official OpenAI Symphony Context

Official source checked:

- `https://openai.com/index/open-source-codex-orchestration-symphony/`

Key context from the official post:

- Symphony is an open-source experiment/spec and reference implementation for Codex orchestration.
- It is intended to show how task-tracker workflows can supervise coding agents.
- The OpenAI post frames the task tracker as the primary orchestration/control plane.
- The reference implementation uses Codex and issue-tracker integration.
- The post positions Symphony as an open-source reference, not a fully managed product surface.

Practical implication for Roko:

- Implementing "OpenAI Symphony things" should mean implementing a compatible mode and borrowing the core interaction model, not replacing Roko's richer runner with Symphony's simpler one-agent-per-issue loop.

## What It Takes To Implement Symphony-Style Support

### 1. `WORKFLOW.md` Loader

Support a file with frontmatter/config plus prompt template.

Needed:

- Markdown frontmatter parser.
- YAML/TOML config parser.
- Liquid/Handlebars-style template rendering.
- Schema validation.
- Hot reload with versioning.

Internal mapping:

```text
tracker config       -> WorkSource config
workspace config     -> WorkspaceManager config
agent config         -> RunConfig / ProcessSupervisor limits
codex config         -> AgentBackend config
hooks                -> pre/post task hook pipeline
prompt template      -> task prompt layer or SystemPromptBuilder input
```

Possible command:

```bash
roko symphony WORKFLOW.md
```

Possible roko-native variant:

```bash
roko workflow watch WORKFLOW.md
```

### 2. WorkSource Abstraction

`TrackerAdapter` is too narrow if the goal includes GitHub, Linear, Slack, Sentry, Figma, webhooks, and MCP.

Use a broader abstraction:

```rust
#[async_trait]
pub trait WorkSource: Send + Sync {
    async fn fetch_candidates(&self) -> Result<Vec<WorkItem>>;
    async fn fetch_state(&self, ids: &[ExternalWorkId]) -> Result<Vec<WorkItem>>;
    async fn subscribe(&self, sink: WorkEventSink) -> Result<Option<SubscriptionHandle>>;
    async fn enrich(&self, id: &ExternalWorkId) -> Result<WorkContext>;
}

#[async_trait]
pub trait WorkSink: Send + Sync {
    async fn claim(&self, id: &ExternalWorkId, claim: ClaimInfo) -> Result<()>;
    async fn update_status(&self, id: &ExternalWorkId, status: ExternalStatus) -> Result<()>;
    async fn post_comment(&self, id: &ExternalWorkId, comment: &str) -> Result<()>;
    async fn attach_pr(&self, id: &ExternalWorkId, pr: PullRequestRef) -> Result<()>;
}
```

Built-in sources/sinks:

- Roko Board.
- Linear.
- GitHub Issues.
- GitHub Actions/webhooks.
- Plane.
- Jira.
- Notion.
- Generic webhook.
- TOML file watcher.
- WORKFLOW.md parser.
- MCP-backed sources.

### 3. Poll/Reconcile Loop

Symphony's core loop:

```text
poll tracker
reconcile running work
fetch candidates
sort/filter
dispatch up to concurrency limit
process retry queue
clean completed workspaces
repeat
```

Roko version:

```text
fetch WorkItems from WorkSources
normalize into Workflow/Task records
run enrichment if configured
compute DAG and ready frontier
claim eligible tasks
create workspace/worktree
dispatch agent through selected backend
stream output into StateHub
run gates
publish PR/review state
sync external status
record episode/learning data
```

Required components:

- `WorkflowDaemon`.
- `Reconciler`.
- `ReadyFrontier`.
- `RetryQueue`.
- `WorkspaceManager`.
- `AgentRunManager`.
- `GateRunner`.
- `ExternalSync`.
- `StateHubPublisher`.

### 4. Workspace Isolation

Symphony uses isolated workspaces per issue. Roko should make this configurable per task.

```toml
[workflow.execution]
default_isolation = "worktree" # worktree | branch | in_place
cleanup = "on_done"            # never | on_done | after_ttl
fresh_rework = true
```

Task-level override:

```toml
[[task]]
id = "T3"
isolation = "worktree"
exclusive_files = true
```

Needed:

- Create worktree.
- Checkout branch.
- Run setup hooks.
- Persist workspace path.
- Attach workspace to task execution state.
- Cleanup policy.
- Recovery on restart.

### 5. Agent Backend Integration

Roko should keep backends abstract.

```rust
pub trait AgentBackend {
    async fn start_run(&self, request: AgentRunRequest) -> Result<AgentRunHandle>;
    async fn stream_events(&self, run: &AgentRunHandle) -> Result<AgentEventStream>;
    async fn cancel(&self, run: &AgentRunHandle) -> Result<()>;
}
```

Backends:

- Existing Claude/Claude CLI.
- Codex CLI/app-server.
- Cursor.
- OpenAI-compatible.
- Gemini/Ollama/etc.

Symphony compatibility adds:

- Codex app-server process management.
- Protocol event parser.
- Tool call bridge.
- Tracker tool exposure.

### 6. Tool and Tracker Write Policy

Symphony has an important separation:

- The orchestrator reads the tracker and dispatches.
- The agent updates tracker state through tools.

Roko can support both modes.

```toml
[workflow.tracker_writes]
mode = "agent"        # agent | orchestrator | hybrid
approval = "propose"  # autonomous | propose
```

Modes:

- `agent`: closer to Symphony. Agent uses Linear/GitHub tools to move issues and comment.
- `orchestrator`: Roko updates external state after verified task transitions.
- `hybrid`: Roko claims/releases; agent comments/links PRs.

Recommendation:

- Use `hybrid` as Roko default.
- Use `agent` for strict Symphony compatibility.

### 7. Gates and Review

Symphony leans on CI and human review. Roko should keep built-in gates.

Task completion path:

```text
agent finishes
  -> local gates: compile/test/clippy/custom verify
  -> if pass: create PR or mark review
  -> if fail: rework or auto-fix
  -> publish gate_result events
  -> update external tracker
```

Review states:

```text
ready -> active -> gates -> review -> merge -> done
                    |
                    v
                  rework
```

Needed:

- Per-task verify steps from `tasks.toml`.
- Gate result projection.
- Review queue/inbox projection.
- External tracker mapping for review/rework/done.

### 8. State and Recovery

Symphony rebuilds state from tracker + filesystem. Roko should do both:

- Rebuild from external tracker and `.roko/workflows`.
- Replay `.roko/events.jsonl`.
- Restore executor state.
- Reconcile active workspaces and agent processes.

Persistent files:

```text
.roko/workflows/{id}/workflow.toml
.roko/workflows/{id}/events.jsonl
.roko/workflows/{id}/executor.json
.roko/workflows/{id}/workspaces.toml
.roko/workflows/{id}/external.toml
```

Recovery algorithm:

```text
load workflow records
load executor snapshots
replay events
fetch external states
detect orphan workspaces
detect stale running tasks
resume or mark needs_reconcile
publish StateHub snapshot
```

### 9. Dashboard and Subscription

Symphony-compatible UI should not be separate from the Roko workflow UI.

Use:

```text
GET /api/workflows
GET /api/workflows/{id}
GET /api/workflows/{id}/events
GET /api/projections/workflow_detail
GET /api/projections/workflow_task_graph
GET /api/projections/workflow_gate_matrix
GET /api/projections/workflow_model_routing
GET /api/projections/workflow_detail/stream
```

Optional:

```text
GET /api/workflow/ws
```

UI pages:

- Workflow list.
- PRD/Plan document viewer.
- Task board.
- Task DAG.
- Task detail.
- Gate matrix.
- Agent stream.
- Model routing/cost panel.
- Review/proposal inbox.
- External tracker sync panel.

## Integration Hub Architecture

The tracker integration notes make a good point: `TrackerAdapter` alone is too narrow. The broader platform shape should be a hub.

```text
WorkSource
  -> normalized WorkItem
  -> WorkflowStore
  -> TaskDAG
  -> Runner
  -> GatePipeline
  -> WorkSink
  -> StateHub projections
```

External sources:

- GitHub Issues.
- GitHub Actions.
- Linear.
- Jira.
- Plane.
- Sentry.
- Slack.
- Figma.
- Notion.
- Vercel/Netlify.
- Generic webhook.
- MCP servers.
- n8n/Zapier/Composio.
- TOML files.
- WORKFLOW.md.

The model:

```rust
pub struct WorkItem {
    pub source: String,
    pub external_id: String,
    pub title: String,
    pub body: String,
    pub status: ExternalStatus,
    pub priority: Option<i32>,
    pub labels: Vec<String>,
    pub links: Vec<ExternalLink>,
    pub assignee: Option<String>,
    pub context_refs: Vec<ContextRef>,
}

pub enum WorkEvent {
    ItemCreated(WorkItem),
    ItemUpdated { id: ExternalWorkId, changes: Vec<Change> },
    CommentAdded { id: ExternalWorkId, body: String },
    CiFailed { repo: String, sha: String, logs: String },
    ErrorReported { source: String, payload: serde_json::Value },
    DesignReady { source: String, payload: serde_json::Value },
    DeployReady { source: String, preview_url: String },
    GenericWebhook(serde_json::Value),
}
```

Combination workflows this unlocks:

```text
Sentry -> Roko task -> GitHub PR -> Slack notification -> Vercel deploy verification

Figma -> Roko task -> Linear issue -> GitHub PR -> Vercel preview -> designer review

Slack request -> Roko epic -> parallel tasks -> gates -> PRs -> Slack summary

Notion spec -> Jira epic -> Roko tasks -> GitHub PRs -> Jira status updates
```

## Model Routing In Workflow UI

The UI should show both the requested route and the actual selected model.

Task fields:

```text
tier
model_hint
role
frequency
domain
max_loc
allowed_tools
denied_tools
mcp_servers
context_weight
quality_profile
speed_priority
reasoning_level
```

Runtime route events:

```text
route.requested
route.selected
route.escalated
route.completed
```

Projection shape:

```json
{
  "task_id": "btc-funding-alert-cli:T3",
  "declared": {
    "tier": "T3",
    "model_hint": "claude-opus",
    "role": "integrator"
  },
  "selected": {
    "provider": "claude",
    "model": "claude-opus-4-6",
    "reason": "network/email integration risk",
    "estimated_cost_usd": 0.18
  },
  "actual": {
    "input_tokens": 12000,
    "output_tokens": 1800,
    "cost_usd": 0.21,
    "duration_ms": 92000
  }
}
```

UI rendering:

- T1/T2/T3 cards with counts.
- Per-task route badge.
- Selected model/provider tooltip.
- Escalation marker if retries moved task to stronger model.
- Cost delta panel for cold-start vs warm execution.

## Three Demo Examples As Real Workflows

The three examples should share one pipeline but produce visibly different artifacts.

### Example 1: Simple Status CLI

Expected shape:

```text
1 PRD
1 plan
3-4 tasks
mostly T1
local-only gates
no secrets
no network
```

Likely tasks:

- Add status command.
- Add `--json` output.
- Add tests.
- Run compile/test gates.

UI emphasis:

- Even a small request becomes explicit tasks and gates.
- Fast routing.
- Minimal dependencies.

### Example 2: GitHub Release Watcher

Expected shape:

```text
1 PRD
1 plan
5-7 tasks
T1 + T2 mix
HTTP client but offline fixture tests
JSON parsing
CLI output modes
```

Likely tasks:

- CLI args and output contract.
- GitHub release API client.
- Version comparison logic.
- Fixture-based tests.
- JSON output.
- Error handling.
- Gates.

UI emphasis:

- Work separates into implementation and verification.
- Network is abstracted behind fixtures.
- Routing shifts from T1 mechanical to T2 implementation.

### Example 3: BTC Funding Alert Stage Job

Expected shape:

```text
1 PRD
1 or more plans
7-10 tasks
T1 + T2 + T3
DeFi API client
email integration
state persistence
dry-run/smoke gates
secrets/env config
```

Likely tasks:

- CLI contract and config.
- Hyperliquid funding-rate client.
- Funding flip detector.
- State persistence.
- Email notifier abstraction.
- Dry-run mode.
- Offline fixtures.
- Integration smoke command.
- Gate pipeline.

UI emphasis:

- Multi-skill work creates richer task graph.
- Verification gates matter more.
- T3/integration-risk route appears.
- Tool/integration requirements are visible.
- Cold-start vs warm execution cost can be shown.

## Implementation Phases

### Phase 0: Stop Demo From Scraping Terminal

Build:

- `GET /api/workflows`
- `GET /api/workflows/{id}`
- `GET /api/workflows/{id}/tasks`
- modern `.roko/plans/*/tasks.toml` parser in serve
- React API hook for workflows
- remove task status projection in `monitorImplementation`

Verification:

- Create sample `.roko/prd` and `.roko/plans/foo/tasks.toml`.
- `GET /api/workflows/foo` returns PRD, plan, tasks.
- Demo renders from HTTP response.

### Phase 1: Artifact Events and Projection

Build:

- artifact scanner.
- artifact watcher.
- `WorkflowArtifactsUpdated` event or equivalent.
- `workflow_detail` and `workflow_task_graph` projections.
- SSE streams.

Verification:

- Write a new `tasks.toml`; UI updates without reload.
- Edit PRD markdown; UI updates.

### Phase 2: Server Runner Path

Build:

- server endpoint that executes modern `tasks.toml` through runner v2.
- share server StateHub with runner.
- publish task/gate/agent/model events.

Verification:

- `POST /api/workflows/{id}/execute`.
- UI task states change from pending to active to done from real StateHub events.
- Gates appear from real gate results.

### Phase 3: Workflow WebSocket

Build:

- `/api/workflow/ws`.
- subscription handshake.
- initial state frames.
- delta frames.
- cursor replay.
- filters by workflow/plan/task/projection.

Verification:

- Connect late and replay missed events.
- Subscribe only to one workflow.
- Disconnect/reconnect without losing state.

### Phase 4: Board/Epic/Task Store

Build:

- canonical workflow/task crate.
- CRUD APIs.
- DAG engine.
- task detail model.
- execution queue.
- proposal/review state.

Verification:

- Create/edit tasks from UI.
- DAG recomputes and ready frontier updates.
- All surfaces show same ViewModel.

### Phase 5: Symphony-Compatible Mode

Build:

- `roko symphony WORKFLOW.md`.
- WORKFLOW.md loader.
- Linear/GitHub/RokoBoard WorkSource.
- workspace isolation.
- poll/reconcile loop.
- external sync.
- optional Codex app-server backend.

Verification:

- A Linear/GitHub issue in a configured ready state creates or claims a Roko task.
- Roko executes in isolated workspace.
- PR link/comment posts back.
- Rework state causes reset/retry.

### Phase 6: Lens Runtime

Build:

- `ObservableEvent`.
- `LensScope`.
- `LensRegistry`.
- built-in workflow/task/gate/router/cost/error Lenses.
- projection builders over observation Signals.

Verification:

- Removing Lenses changes visibility only, not behavior.
- Lenses can stack and chain.
- Projection updates are produced by Lens output.

## Key Design Decisions To Make

### Source Of Truth

Options:

1. Files are source of truth.
2. StateHub snapshot is source of truth.
3. WorkflowStore is source of truth; files are snapshots.

Recommendation:

- For PRD/plan/task artifacts, files remain durable source initially.
- For live execution, StateHub is source of truth.
- For long-term Board/Epic/Task CRUD, WorkflowStore should be source of truth and export human-readable TOML snapshots.

### SSE vs WebSocket

Recommendation:

- Use SSE projection streams immediately.
- Add workflow WebSocket later for multi-projection subscription and richer replay.

### Strict Symphony vs Roko-Native

Recommendation:

- Implement strict Symphony compatibility as a mode.
- Default to Roko-native orchestration with DAG, gates, learning, and multi-backend routing.

### Agent Writes vs Orchestrator Writes

Recommendation:

- Use hybrid mode.
- Orchestrator claims/releases and records verified state.
- Agent can comment and perform tracker-specific workflow actions when explicitly granted tools.

### Lens-First vs Projection-First

Recommendation:

- Projection-first for workflow.
- Lens-compatible event names and schemas now.
- Build Lens runtime later.

## Concrete First PR Set

PR 1: Shared workflow parsing

- Move modern `TasksFile` parser into shared crate or expose equivalent in `roko-core`.
- Add parser for `.roko/plans/{id}/plan.md`.
- Add parser for PRD markdown with frontmatter.
- Add tests with sample tasks.

PR 2: Workflow HTTP endpoints

- `GET /api/workflows`
- `GET /api/workflows/{id}`
- `GET /api/workflows/{id}/tasks`
- Support modern plan directories.
- Keep old `/api/plans` behavior compatible.

PR 3: Demo app data path

- Replace terminal artifact scraping with workflow endpoints.
- Keep terminal panes only for showing commands/output.
- Remove simulated active task fallback.
- Render loading/error/degraded states.

PR 4: Workflow projection

- Add `workflow_detail` and `workflow_task_graph` projection entries.
- Add artifact scan on server startup.
- Publish artifact update event after PRD/plan generation routes complete.

PR 5: Real execution events

- Add server endpoint to execute modern plan through runner v2.
- Pass server StateHub sender into runner.
- Ensure task/gate/agent/model route events update workflow UI.

PR 6: Symphony foundation

- Add `WorkSource`/`WorkSink` traits.
- Add `WORKFLOW.md` parser.
- Add RokoBoard source first.
- Add `roko symphony WORKFLOW.md --dry-run` to show normalized work items without execution.

## Final Architecture Picture

```text
External sources:
  PRD CLI, Roko Board, WORKFLOW.md, Linear, GitHub, Slack, Sentry, Figma, webhooks, MCP

Normalize:
  WorkSource -> WorkItem -> WorkflowStore

Artifacts:
  PRD docs, plan.md, tasks.toml, task graph, gate contracts

Execution:
  WorkflowDaemon / Runner v2 / AgentBackend / GatePipeline / WorkspaceManager

Events:
  WorkflowEvent + DashboardEvent + future ObservableEvent

State:
  StateHub snapshot + replay ring + .roko/events.jsonl

Projections:
  workflow_detail, workflow_task_graph, active_tasks, gate_matrix, model_routing, agent_streams

Surfaces:
  Web Workbench, TUI Workbench, ACP/editor sidebar, CLI commands, external tracker comments

Future generalization:
  Lenses observe workflow/agent/gate/model events and emit observation Signals.
  StateHub projections are built from those Signals.
```

The important product rule:

```text
Surfaces should never scrape files, parse terminal output, or invent task state.
They should query typed projections and subscribe to typed deltas.
```
