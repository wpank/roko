# Serve Inbound Request Timeout

**Priority**: P2 — unbounded request durations can exhaust connections
**Size**: S (½ day)
**Crate**: `crates/roko-serve/`

---

## Problem

`roko serve` has no inbound request timeout. A slow client, stalled connection, or
runaway handler can hold a connection open indefinitely. While outbound reqwest
timeouts are configured (provider HTTP calls have deadlines), the inbound HTTP layer
has no `TimeoutLayer`.

This affects non-streaming routes. SSE and WebSocket routes are long-lived by design
and should be excluded from the timeout.

---

## Where to look

- `crates/roko-serve/src/routes/middleware.rs` — existing middleware stack
- `crates/roko-serve/src/routes/mod.rs` — router construction
- `crates/roko-serve/src/lib.rs` — server startup

---

## What to do

**Step 1.** Add `tower_http::timeout::TimeoutLayer` to the middleware stack for
non-streaming routes. Suggested default: 30 seconds.

**Step 2.** Exclude SSE (`/events`, `/sse/*`), WebSocket (`/ws`, `/roko-ws`), and
long-running inference routes (`/api/run`, `/api/gateway/chat`) from the timeout layer.
These should use their own per-request deadlines.

**Step 3.** Make the timeout configurable:

```toml
[serve]
request_timeout_ms = 30000
```

**Step 4.** Return `408 Request Timeout` when the deadline is exceeded.

---

## Acceptance criteria

- [ ] Non-streaming routes have a configurable inbound timeout (default 30s)
- [ ] SSE, WebSocket, and inference routes are excluded from the global timeout
- [ ] `408 Request Timeout` returned when deadline exceeded
- [ ] Timeout configurable via `roko.toml [serve.request_timeout_ms]`
- [ ] All existing tests pass (`cargo test -p roko-serve`)

---

**Origin**: productionizing audit (2026-08-13)
