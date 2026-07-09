# Fix: Wire Plan Execution Events to Serve SSE/WebSocket

## Summary

Thread the server's `SharedStateHub` into the plan runner when executed from `roko serve`,
so that all `DashboardEvent` emissions reach SSE/WebSocket clients in real time.

## Current State

```rust
// serve_runtime.rs line 274 — creates DISCONNECTED hub
let state_hub = crate::state_hub::shared_state_hub();

// serve_runtime.rs lines 571-572 — nulls out feedback
feedback_facade: None,
projection: None,
```

The runner's `TuiBridge` publishes events to this local hub. Nobody subscribes to it.
Meanwhile, `AppState.state_hub` (created in `roko-serve/src/state.rs` line 528) is
what SSE/WS handlers read from — but the runner never touches it.

## The Fix

### Step 1: Pass `SharedStateHub` from AppState into CliRuntime

`CliRuntime` is constructed in `roko-serve/src/state.rs`. It needs to hold a reference
to the serve-side `SharedStateHub`.

```rust
// serve_runtime.rs — add field
pub struct RokoCliRuntime {
    config: Config,
    repo_registry: RepoRegistry,
    knowledge_store: OnceLock<KnowledgeStore>,
    playbook_store: OnceLock<PlaybookStore>,
    state_hub: SharedStateHub,  // <-- ADD THIS
}
```

Wire it in wherever `RokoCliRuntime::new()` is called (in `state.rs`), passing
`app_state.state_hub.clone()`.

### Step 2: Use the serve-side hub in build_runner_config

Replace line 274's local hub creation:

```rust
// BEFORE:
let state_hub = crate::state_hub::shared_state_hub();

// AFTER:
let state_hub = self.state_hub.clone();
```

This makes the runner's `TuiBridge` publish to the serve-side hub. All existing
`DashboardEvent` emissions from the runner loop will now flow to SSE/WS clients.

### Step 3: Wire FeedbackFacade and Projection (same as CLI path)

Copy the wiring from `commands/plan.rs` lines 420-508 into `build_runner_config()`:

```rust
// Create episode sink
let episode_sink = EpisodeSink::at(&workdir.join(".roko/episodes.jsonl"));

// Create routing observation sink
let cascade_router = CascadeRouter::load_or_default(&workdir.join(".roko/learn"));
let routing_sink = RoutingObservationSink::new(cascade_router);

// Create knowledge ingestion sink
let knowledge_sink = KnowledgeIngestionSink::at(&workdir.join(".roko/knowledge"))
    .with_ingestor(Arc::new(NeuroKnowledgeIngestor::new(/* ... */)));

let feedback_facade = Arc::new(
    FeedbackFacade::new()
        .with_sink(Arc::new(episode_sink))
        .with_sink(Arc::new(routing_sink))
        .with_sink(Arc::new(knowledge_sink)),
);

let projection = Arc::new(Projection::new(run_uuid.clone()));

// In RunConfig:
feedback_facade: Some(feedback_facade),
projection: Some(projection),
```

### Step 4: Wire server_event_bus into orchestrate.rs (legacy path)

If the legacy `PlanRunner` (orchestrate.rs) is still used from serve, also call
`set_server_event_bus()`:

```rust
// In the code path that creates PlanRunner for serve:
let bus = app_state.event_bus.clone();
runner.set_server_event_bus(bus);
runner.set_state_hub(app_state.state_hub.sender());
```

## Events That Will Flow After Fix

Once wired, these `DashboardEvent` variants will reach the frontend:

| Event | When |
|-------|------|
| `PlanStarted` | Plan execution begins |
| `TaskStarted` | Each task begins |
| `TaskCompleted` | Each task finishes |
| `PhaseTransition` | Plan changes phase |
| `AgentSpawned` | Agent dispatched |
| `AgentOutput` | Agent produces output (streaming) |
| `AgentCompleted` | Agent run finishes |
| `GateResult` | Gate verdict for a task |
| `EfficiencyEvent` | Per-turn efficiency metrics |
| `CascadeRouterUpdated` | Model routing updated |
| `CFactorTrendUpdated` | C-factor metrics |
| `Error` | Any error during execution |

## Verification

After implementing:

1. Start `roko serve`
2. Open browser to `http://localhost:6677` (or demo-app on :5173)
3. Execute a plan via `POST /api/plans/{id}/execute` or use TUI to trigger one
4. Confirm SSE stream at `/api/events/stream` shows `plan_started`, `task_started`, etc.
5. Confirm WebSocket at `/api/ws` shows same events

## Risks

- **Performance**: The serve hub uses `watch::Sender` (always latest snapshot) +
  `broadcast::Sender` (event bus). Thousands of events/sec is fine.
- **Concurrency**: Multiple plan runs share one hub. Events are tagged with `plan_id`,
  so frontend can filter. This matches the existing design.
- **Lifetime**: `SharedStateHub` is `Arc<StateHub>` — clone is cheap.
