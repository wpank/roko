# Fix: HTTP Event Sink for CLI Subprocess Forwarding

## Summary

When `roko plan run` executes as a subprocess (spawned from a PTY terminal, CI, or
external tooling), its events are trapped in the subprocess. This doc describes the
HTTP event sink pattern that lets any roko CLI process forward events to a running
`roko serve` instance.

## Problem

```
┌─────────────────────────────────┐
│  PTY terminal (demo IDE)        │
│  $ roko plan run plans/foo/     │
│       │                         │
│       ├── TuiBridge → local hub │  ← nobody subscribes
│       ├── stderr (spinners)     │  ← user sees this
│       └── events.jsonl          │  ← persisted but not live
│                                 │
│  No connection to roko serve    │
└─────────────────────────────────┘
```

The CLI runner emits events to:
1. `TuiBridge` → local `SharedStateHub` (if `--tui` mode, otherwise just stderr)
2. `FeedbackFacade` → episode/routing/knowledge sinks (file persistence)
3. `Projection` → raw runtime events (unused without subscribers)

None of these reach a remote `roko serve` process.

## Solution: HTTP Event Sink in the Runner

### Design

When `ROKO_SERVE_URL` is set in the environment, the CLI runner creates an
`HttpEventSink` that POSTs `ServerEvent` JSON to the serve ingest endpoint on
every event emission. This is fire-and-forget with a bounded queue to avoid
blocking the runner.

### Implementation

```rust
// New: crates/roko-cli/src/runner/http_sink.rs

use tokio::sync::mpsc;
use crate::serve::events::ServerEvent;

/// Non-blocking HTTP event sink. Queues events and POSTs them to roko-serve.
pub struct HttpEventSink {
    tx: mpsc::Sender<ServerEvent>,
}

impl HttpEventSink {
    /// Creates a sink if ROKO_SERVE_URL is set. Spawns a background task.
    pub fn from_env() -> Option<Self> {
        let url = std::env::var("ROKO_SERVE_URL").ok()?;
        let (tx, mut rx) = mpsc::channel::<ServerEvent>(512);

        tokio::spawn(async move {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap();

            while let Some(event) = rx.recv().await {
                // Best-effort delivery; don't block runner on network failures
                let _ = client
                    .post(format!("{}/api/events/ingest", url))
                    .json(&event)
                    .send()
                    .await;
            }
        });

        Some(Self { tx })
    }

    /// Queue an event for delivery. Non-blocking; drops if queue full.
    pub fn emit(&self, event: ServerEvent) {
        let _ = self.tx.try_send(event);
    }
}
```

### Integration Point

In `crates/roko-cli/src/runner/mod.rs` (or wherever the runner event loop lives),
add the sink alongside existing TuiBridge/FeedbackFacade:

```rust
// During runner initialization:
let http_sink = HttpEventSink::from_env();

// In the event emission path (wherever emit_runner_event is called):
if let Some(sink) = &http_sink {
    if let Some(server_event) = dashboard_event_to_server_event(&event) {
        sink.emit(server_event);
    }
}
```

### Conversion: DashboardEvent → ServerEvent

The runner emits `DashboardEvent`. The serve bus expects `ServerEvent`. A conversion
function already exists in `orchestrate.rs` (`server_event_to_dashboard` at line 19563).
We need the reverse direction:

```rust
fn dashboard_event_to_server_event(event: &DashboardEvent) -> Option<ServerEvent> {
    match event {
        DashboardEvent::PlanStarted { plan_id } =>
            Some(ServerEvent::PlanStarted { plan_id: plan_id.clone() }),
        DashboardEvent::TaskStarted { plan_id, task_id, description } =>
            Some(ServerEvent::TaskStarted { plan_id: plan_id.clone(), task_id: task_id.clone(), description: description.clone() }),
        DashboardEvent::TaskCompleted { plan_id, task_id, success } =>
            Some(ServerEvent::TaskCompleted { plan_id: plan_id.clone(), task_id: task_id.clone(), success: *success }),
        DashboardEvent::AgentSpawned { agent_id, role, model } =>
            Some(ServerEvent::AgentSpawned { agent_id: agent_id.clone(), role: role.clone(), model: model.clone() }),
        DashboardEvent::AgentOutput { agent_id, content, done, .. } =>
            Some(ServerEvent::AgentOutput { agent_id: agent_id.clone(), run_id: None, content: content.clone(), done: *done, metadata: None }),
        DashboardEvent::GateResult { plan_id, task_id, gate, rung, passed } =>
            Some(ServerEvent::GateResult { plan_id: plan_id.clone(), task_id: task_id.clone(), gate: gate.clone(), rung: *rung, passed: *passed }),
        DashboardEvent::PlanCompleted { plan_id, success } =>
            Some(ServerEvent::PlanCompleted { plan_id: plan_id.clone(), success: *success }),
        DashboardEvent::PhaseTransition { plan_id, from, to } =>
            Some(ServerEvent::PhaseTransition { plan_id: plan_id.clone(), from: from.clone(), to: to.clone() }),
        DashboardEvent::Error { message } =>
            Some(ServerEvent::Error { message: message.clone() }),
        // Events that don't have a direct ServerEvent mapping:
        _ => None,
    }
}
```

## Serve-Side Ingest Endpoint

Same endpoint as described in `02-ACP-BRIDGE.md`:

```rust
/// POST /api/events/ingest — universal event ingestion
/// Used by: CLI subprocess, ACP sessions, PTY commands
async fn ingest_event(
    State(state): State<AppState>,
    Json(event): Json<ServerEvent>,
) -> StatusCode {
    // Optionally validate auth token from ROKO_SERVER_AUTH_TOKEN header
    state.event_bus.publish(event.clone());

    // Also apply to state hub snapshot for late-joining clients
    if let Some(dashboard_event) = server_event_to_dashboard(&event) {
        state.state_hub.publish(dashboard_event);
    }

    StatusCode::ACCEPTED
}
```

## Batching Optimization (Optional)

For high-frequency events (AgentOutput with streaming tokens), batch multiple events
into one POST to reduce HTTP overhead:

```rust
// In the background sender task:
let mut batch = Vec::with_capacity(32);
loop {
    // Drain up to 32 events or wait 50ms
    match tokio::time::timeout(Duration::from_millis(50), rx.recv()).await {
        Ok(Some(event)) => {
            batch.push(event);
            // Drain remaining without waiting
            while batch.len() < 32 {
                match rx.try_recv() {
                    Ok(event) => batch.push(event),
                    Err(_) => break,
                }
            }
        }
        Ok(None) => break, // Channel closed
        Err(_) => {} // Timeout, flush what we have
    }

    if !batch.is_empty() {
        let _ = client.post(format!("{}/api/events/ingest/batch", url))
            .json(&batch)
            .send()
            .await;
        batch.clear();
    }
}
```

## Security Considerations

- The ingest endpoint should validate `Authorization: Bearer <token>` using the
  same `ROKO_SERVER_AUTH_TOKEN` that the TUI uses
- Rate limiting: cap at 1000 events/sec per source to prevent DoS
- Only accept events from localhost by default (configurable for remote workers)

## Verification

1. Start `roko serve` on :6677
2. In a new terminal: `ROKO_SERVE_URL=http://127.0.0.1:6677 roko plan run plans/test/`
3. In another terminal: `curl -N http://127.0.0.1:6677/api/events/stream`
4. Confirm real-time events appear in the SSE stream as the plan executes
