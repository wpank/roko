# Agent Server (Sidecar) Issues

## High

### Non-constant-time token comparison
- `auth/bearer.rs:35`: `hash(token) == self.token_hash` uses standard `==` (short-circuits on first differing byte). Timing side-channel.

### Silent no-auth mode
- `lib.rs:82-89`: When `auth` is `None`, all protected routes exposed without authentication. No warning.

### Unbounded WebSocket stream channel
- `messaging.rs:171-173`: `mpsc::unbounded_channel()` between dispatcher and socket. Slow client → unbounded memory growth.

### One bad relay frame kills entire connection
- `relay_client.rs:224`: Single malformed frame → `run()` returns `Err` → relay permanently dead. No reconnect.

### `await_hello_ack` has no timeout
- `relay_client.rs:413-438`: If relay never responds, `connect()` blocks forever.

## Medium

### No WebSocket keepalive or idle timeout
- `messaging.rs:66-82`: No ping/pong. Silent client → socket held open indefinitely.

### Ping frames not ponged
- `messaging.rs:78`: Binary and Ping frames silently dropped. Client liveness probes get no response.

### Heartbeat reports zeroed counters
- `lib.rs:427-430`: `active_tasks: 0, completed_tasks: 0, failed_tasks: 0` always. Discovery registry always sees idle.

### `tool_calls: Vec::new()` in chat response
- `state.rs:158-164`: Backend tool call payloads silently discarded. Callers get empty array.

### No request body size limit
- `messaging.rs:36-40`: Multi-MB prompt forwarded directly to LLM. No `RequestBodyLimitLayer`.

### Task state transitions not guarded
- `tasks.rs:27-29`: `accept_task` on `Completed` task succeeds. No state precondition check.
- `state.rs:839-853`: `complete_task` can overwrite already-completed task.

### Predictions Vec unbounded
- `state.rs:481`: No eviction, TTL, or max-size policy.

## Low

### `/capabilities` always public — exposes topology
- `lib.rs:79`: `agent_id`, `owner`, `registered_at`, full route list.

### Log writes errors silently absorbed
- `state.rs:544-557`: Disk-full → `tracing::warn` only. Callers never know.
