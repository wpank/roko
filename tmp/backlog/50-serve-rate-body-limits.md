# Serve Rate and Body Limits

**Priority**: P1 — security
**Size**: S (1 day)
**Crate**: `crates/roko-serve/src/routes/mod.rs`, `crates/roko-serve/src/terminal.rs`

---

## Problem

The HTTP control plane has global rate limiting and body size caps, but several
route groups lack proportionate per-route limits. The global limits are generous
enough to allow individual expensive endpoints (terminal creation, inference
dispatch, agent registration) to be abused within the global budget.

Terminal PTY creation is especially sensitive: each session spawns a shell process,
allocates a PTY, and holds both until explicitly closed or timed out. An attacker who
stays within the global rate limit can still open dozens of PTY sessions, exhausting
system resources.

---

## Section A: Current State

**A1.** Global body limit: `DEFAULT_REQUEST_BODY_LIMIT_BYTES = 4 * 1024 * 1024` (4 MiB)
at `crates/roko-serve/src/routes/mod.rs` line 95, applied via `DefaultBodyLimit::max()`
at line 420. This is reasonable.

**A2.** Per-route body override for webhooks: `WEBHOOK_BODY_LIMIT_BYTES = 1024 * 1024`
(1 MiB) at `crates/roko-serve/src/routes/webhooks.rs` line 22, applied at lines 383
and 390. This is correct.

**A3.** Global rate limiting exists via the `governor` crate:
- `DEFAULT_GLOBAL_RATE_PER_SEC` (line 97-101) — global non-keyed limiter.
- `DEFAULT_PER_KEY_RATE_PER_SEC` (line 103-107) — per-caller keyed limiter (by API key
  hash or client IP).
- Both are applied as middleware at lines 416-429.
- Tests at lines 1900 and 2131 cover basic 429 behavior and per-key isolation.

**A4.** WebSocket size limits: `apply_ws_size_limits()` at
`crates/roko-serve/src/routes/ws.rs` lines 38-47 applies `max_message_size(1 MiB)` and
`max_frame_size(256 KiB)`. All WS upgrade handlers call this, including the terminal
WS handler at `crates/roko-serve/src/terminal.rs` line 1013.

**A5.** No per-route rate limits exist for:
- Terminal session creation (`POST /api/terminal/sessions`)
- Terminal WS upgrade (`GET /ws/terminal/{id}`)
- Inference dispatch (`POST /api/gateway/infer`, `POST /api/run`)
- Agent registration (`POST /agents/register`, `POST /agents/create`)
- Signal injection (`POST /api/signals`)

**A6.** The terminal session manager (`TerminalSessions`) has a configurable
`max_sessions` cap, but there is no rate limit on creation attempts. A client can
rapidly create-and-discard sessions, forcing the server to repeatedly spawn and
clean up shell processes.

---

## Section B: What To Do

**B1.** Add per-route-group rate limits using `governor`. Create a helper that builds
a keyed limiter from a `(per_second, burst)` pair. Suggested limits:

| Route group | Per-key limit | Rationale |
|---|---|---|
| Terminal creation + WS upgrade | 2/min burst 3 | Each spawns a PTY process |
| Inference dispatch (gateway/infer, run) | 30/min burst 10 | Each triggers an LLM call |
| Agent registration (register, create) | 5/min burst 5 | Disk writes + registry ops |
| Signal injection | 20/s burst 40 | High-volume but cheap |

Apply these as `axum::middleware::from_fn_with_state` on the sub-routers returned by
each module's `routes()` / `authenticated_routes()` function.

**B2.** Add a terminal-specific body limit. The terminal input endpoint
(`POST /api/terminal/sessions/{id}/input`) accepts raw bytes that are forwarded to a
PTY. Cap this at 256 KiB via `DefaultBodyLimit::max(256 * 1024)` on the terminal
sub-router. The global 4 MiB limit is excessive for keystroke/paste payloads.

**B3.** Make the global and per-key rate limits configurable via `[server]` config:
```toml
[server]
rate_limit_per_sec = 100        # global
rate_limit_per_key_per_sec = 30 # per API key / IP
```
Fall back to the current hardcoded defaults when unset.

**B4.** Ensure all rate limit 429 responses include a `Retry-After` header with the
governor's computed wait time. The current `rate_limit_middleware` at line 189 returns
a JSON body but no `Retry-After` header.

---

## Acceptance criteria

- [ ] Terminal creation and WS upgrade routes have a per-key rate limit (suggested 2/min)
- [ ] Inference dispatch routes have a per-key rate limit (suggested 30/min)
- [ ] Agent registration routes have a per-key rate limit (suggested 5/min)
- [ ] Terminal input body limit is 256 KiB
- [ ] Global and per-key rate limits are configurable via `[server]` config
- [ ] 429 responses include `Retry-After` header
- [ ] Existing rate limit and body limit tests pass
- [ ] New tests cover per-route rate limits returning 429

### Not in scope
- DDoS mitigation or distributed rate limiting
- WAF integration
- Changing the global 4 MiB body limit or 1 MiB webhook limit
- WebSocket message/frame size changes (already correct at 1 MiB / 256 KiB)
