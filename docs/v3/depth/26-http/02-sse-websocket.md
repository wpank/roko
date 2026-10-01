# 26.02 -- SSE and WebSocket Protocol

> Depth file for [26-HTTP-API.md](../../26-HTTP-API.md).

---

## SSE Endpoints

Two SSE endpoints stream real-time dashboard events to browser and TUI clients.

### `/api/events` and `/api/sse`

Both aliases resolve to the same `sse_handler` in `crates/roko-serve/src/routes/sse.rs`.

```
GET /api/events
GET /api/sse
```

The handler streams `DashboardEvent` payloads as SSE `data:` frames. Each event
carries a monotonic `id:` field for reconnection.

### `/api/workflow/events`

A separate SSE endpoint for workflow-specific `RuntimeEvent` payloads, defined
directly in `mod.rs`:

```
GET /api/workflow/events
```

This streams `SseEvent` payloads from the `SseAdapter`, which bridges the
`StateHub` push-based event system.

## Event Wire Format

Each SSE frame follows the standard format:

```
id: 42
data: {"type":"PlanStarted","plan_id":"my-plan","tasks_total":5}

```

The `id` field is a monotonically increasing sequence number from the `StateHub`
ring buffer. Clients use this for reconnection.

## Reconnection Protocol

### Cursor Precedence

When a client reconnects, the replay start position follows this precedence:

1. `Last-Event-ID` HTTP header (standard SSE reconnection)
2. `?lastEventId=<id>` query parameter (browser `EventSource` cannot set headers)
3. `?n=<id>` query parameter (legacy alias)
4. Sequence 0 (full replay from ring buffer start)

The replay starts at `lastSeen + 1` to avoid re-delivering the last acknowledged
event.

```rust
fn replay_start(headers: &HeaderMap, query: &ReplayQuery) -> u64 {
    let last_seen = headers
        .get("Last-Event-ID")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .or(query.last_event_id)
        .or(query.n);
    last_seen.map_or(0, |id| id.saturating_add(1))
}
```

### Gap Detection and Snapshot Resync

If the client's cursor has fallen out of the ring buffer (events expired) or the
retained replay suffix exceeds `MAX_REPLAY_EVENTS` (256), the server sends a
single `gap` event containing a full `DashboardSnapshot`:

```
id: 100
event: gap
data: {"missed_events":50,"last_materialized_seq":100,"snapshot":{...}}

```

The gap event replaces truncated replay to avoid silently skipping events. After
the gap, the stream continues with live events from the snapshot cursor.

The same logic fires when a live client lags behind the broadcast ring buffer:

```rust
Err(broadcast::error::RecvError::Lagged(n)) => {
    warn!(n, "SSE client lagged; sending materialized snapshot resync");
    let cursor = state.state_hub.cursor_snapshot();
    // Send gap event with atomic snapshot
}
```

## Keep-Alive

Both SSE endpoints use an 8-second keep-alive interval, shorter than the default
15s, to survive aggressive proxy timeouts:

```rust
KeepAlive::new()
    .interval(std::time::Duration::from_secs(8))
    .text("keepalive")
```

This is specifically tuned for Railway (30s timeout) and Nginx (60s timeout).

## Response Headers

The `sse_response_headers()` function sets proxy-bypass headers:

| Header | Value | Purpose |
|---|---|---|
| `X-Accel-Buffering` | `no` | Disable Nginx buffering |
| `Cache-Control` | `no-cache, no-store, no-transform, must-revalidate` | Prevent caching |
| `Connection` | `keep-alive` | Maintain persistent connection |

## WebSocket Endpoints

### `/ws`, `/roko-ws`, `/ws/agents`

The `ws::routes()` module provides three WebSocket paths that all resolve to the
same upgrade handler:

```rust
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/ws", get(ws_upgrade))
        .route("/roko-ws", get(ws_upgrade))
        .route("/ws/agents", get(ws_upgrade))
}
```

When auth is enabled, these routes require API key authentication.

### WebSocket Size Limits

Every WebSocket upgrade enforces per-frame and per-message ceilings:

```rust
pub(crate) const WS_MAX_MESSAGE_SIZE: usize = 1024 * 1024;   // 1 MiB
pub(crate) const WS_MAX_FRAME_SIZE: usize = 256 * 1024;      // 256 KiB
```

These prevent a hostile client from forcing the server to buffer arbitrary
amounts of memory before the application code runs.

### Client Subscribe Message

After upgrade, the client sends a JSON subscribe message to configure filtering
and replay:

```json
{
  "subscribe": ["projection:gate_pipeline", "topic:agent.*"],
  "cursor": 42,
  "back_pressure": "at_most_once"
}
```

The `subscribe` field is a list of topic patterns for event filtering. The
`cursor` field requests replay from that sequence number onwards.

### Back-Pressure Modes

The WebSocket handler supports three back-pressure modes selectable per
connection via the `back_pressure` field:

#### `at_most_once` (default)

Deliver every event, dropping only on transport failure. No buffering. Suitable
for dashboards that can tolerate missed events.

#### `coalesce`

Buffer up to 16 events (`COALESCE_BUFFER`) when the client is lagging. When
the client catches up, send a JSON array batch:

```json
{
  "type": "coalesced",
  "events": [{ ... }, { ... }]
}
```

Older events are evicted from the buffer only when the buffer is full and a new
event arrives. Events are never silently discarded.

#### `resume_required`

On lag, immediately halt event delivery and send a control frame:

```json
{
  "type": "resume_required",
  "last_event_id": 42
}
```

The server stops forwarding live events until the client acknowledges with:

```json
{
  "type": "resume",
  "cursor": 42
}
```

The server then replays missed events from the cursor position using the ring
buffer before resuming live delivery.

### WebSocket Topic Filtering

Events can be filtered by topic pattern before delivery. The subscribe field
accepts string patterns that are matched against event type names. When the
filter list is non-empty, only matching events are forwarded. An empty filter
list delivers all events.

### Secret Scrubbing

Both SSE and WebSocket streams scrub payloads through the shared `LogScrubber`
before serialization. This redacts API keys, GitHub tokens, and other secrets
that may appear in error messages or debug payloads. Tests verify that patterns
like `sk-ant-api03-*` and `ghp_*` are replaced with `[REDACTED]` markers.

## WebSocket Proxy Bridge

The `proxy_ws` module in `crates/roko-serve/src/routes/proxy_ws.rs` provides a
bidirectional bridge for reverse-proxying upstream WebSocket services:

```rust
pub(crate) async fn bridge_ws(server_socket: WebSocket, upstream_url: String) {
    let (upstream, _) = connect_async(&upstream_url).await;
    // Bidirectional frame shuttling
    loop {
        tokio::select! {
            msg = server_rx.next() => { /* forward to upstream */ }
            msg = upstream_rx.next() => { /* forward to client */ }
        }
    }
}
```

Frame types forwarded: Text, Binary, Ping, Pong. Close frames terminate the
bridge. Both sides are explicitly closed on exit.

## DashboardEvent Types

The `DashboardEvent` enum in `roko-core` defines the event vocabulary:

- `PlanStarted` -- Plan execution started
- `PlanCompleted` -- Plan finished (with result)
- `TaskStarted` / `TaskCompleted` / `TaskFailed` -- Task lifecycle
- `GateVerdict` -- Gate pass/fail result
- `AgentSpawned` / `AgentStopped` -- Agent lifecycle
- `CostUpdate` -- Running cost accumulation
- `EpisodeRecorded` -- Episode written to log
- `ConfigReloaded` -- Configuration changed
- `Error` -- Error message

Events are scrubbed through `LogScrubber` before serialization to redact any
API keys or tokens that may appear in payloads.

## Source

- `crates/roko-serve/src/routes/sse.rs` -- SSE handler and replay logic
- `crates/roko-serve/src/routes/ws.rs` -- Dashboard WebSocket with back-pressure
- `crates/roko-serve/src/routes/proxy_ws.rs` -- WebSocket proxy bridge
- `crates/roko-serve/src/adapters.rs` -- SseAdapter and event bridging
