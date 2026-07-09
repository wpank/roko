# S-term-1: Add /api/terminal/sessions/{id}/events typed-event WebSocket

## Task
Add a WebSocket endpoint at `GET /api/terminal/sessions/{id}/events` that emits typed `CommandEvent`s. Distinct from the IO socket (`/io`); auth-required; size-capped (T3-26).

## Runner Context
Runner audit-2026-05-01, group S. No dependencies. Wave 1.

## Source plan
`tmp/subsystem-audits/implementation-plans/26-terminal-demo-truth.md` § Phase 1.

## Why
Demo automation (and any consumer) needs typed lifecycle events, not raw PTY bytes. This endpoint emits `CommandEvent` JSON over WebSocket.

## Exact changes

### 1. `crates/roko-serve/src/terminal/events_ws.rs` (new)

```rust
use std::sync::Arc;
use axum::{extract::{Path, State, WebSocketUpgrade}, response::IntoResponse};
use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};

use crate::state::AppState;
use crate::command_events::CommandEvent;

pub async fn terminal_events_handler(
    Path(session_id): Path<String>,
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.max_message_size(64 * 1024)
        .max_frame_size(16 * 1024)
        .on_upgrade(move |socket| handle_events(socket, state, session_id))
}

async fn handle_events(socket: WebSocket, state: Arc<AppState>, session_id: String) {
    let (mut tx, _rx) = socket.split();
    let session = match state.terminal_sessions.get(&session_id).await {
        Some(s) => s,
        None => {
            let payload = CommandEvent::SpawnFailed {
                session_id: session_id.clone(),
                reason: "session not found".into(),
            };
            let _ = tx.send(Message::Text(serde_json::to_string(&payload).unwrap())).await;
            return;
        }
    };

    let mut events = session.subscribe_events();
    while let Ok(event) = events.recv().await {
        let json = match serde_json::to_string(&event) {
            Ok(s) => s,
            Err(e) => { tracing::warn!(error = %e, "serialize CommandEvent"); continue; }
        };
        if tx.send(Message::Text(json)).await.is_err() {
            break;   // client disconnected
        }
    }
}
```

`session.subscribe_events()` returns a broadcast receiver of `CommandEvent`. If `TerminalSession` doesn't already broadcast, add it (probably as a `tokio::sync::broadcast::Sender<CommandEvent>` member).

### 2. Mount in `routes/mod.rs`

```rust
.route("/api/terminal/sessions/:id/events", get(terminal_events_handler))
```

Layer with the auth middleware:

```rust
let terminal = if terminal_requires_auth {
    terminal.layer(...require_api_key)
} else { terminal };
```

(The existing terminal sub-router already does this; just confirm `events_handler` is added inside the auth-gated section.)

### 3. Tests

```rust
#[tokio::test]
async fn events_endpoint_emits_typed_command_event() {
    let app = build_test_app().await;
    // Create a session, drive a command, connect to /events, assert JSON.
}
```

## Write Scope
- `crates/roko-serve/src/terminal.rs` (or `terminal/mod.rs`)
- `crates/roko-serve/src/terminal/events_ws.rs` (new)
- `crates/roko-serve/src/routes/mod.rs`

## Read-Only Context
- `crates/roko-serve/src/command_events.rs`
- `crates/roko-serve/src/state.rs`

## Verify

```bash
ls crates/roko-serve/src/terminal/events_ws.rs

rg '/events' crates/roko-serve/src/routes/mod.rs
# Expect: at least 1 hit (the route line)
```

## Do NOT

- Do NOT serve `CommandEvent`s on the existing `/io` WebSocket — distinct endpoint.
- Do NOT skip WS message-size caps (T3-26).
- Do NOT skip auth.
- Do NOT bundle with other S-term batches.
- Do NOT remove the broadcast receiver when the WS disconnects mid-command (the session keeps emitting; only this client connection drops).
