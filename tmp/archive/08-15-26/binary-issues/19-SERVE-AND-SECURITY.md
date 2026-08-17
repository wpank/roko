# 19 — HTTP Server, Dashboard, and Security Audit

**Status**: open (critical — security issues)
**Scope**: `crates/roko-serve/`, `crates/roko-cli/src/tui/`, `demo/demo-app/`, `crates/roko-cli/src/share.rs`

## What This Document Covers

The HTTP control plane (`roko serve`), TUI dashboard, demo app, WebSocket/SSE streaming,
terminal PTY, and sharing features. Includes several security-critical findings.

---

## 1. Security Issues (Critical)

### SEC1. Terminal PTY routes have no authentication (`routes/mod.rs:137-138`)

```rust
// PTY terminal sessions for web UI — no auth
.merge(crate::terminal::routes())
```

Terminal routes are mounted **outside** the `/api` prefix and **outside** the auth
middleware. Anyone who can reach the server gets full shell access. The terminal spawns
real shell processes via `portable-pty` with `TERM=xterm-256color`.

Combined with SEC2 and SEC3, this means:
- Locally: any website the user visits can create a shell session via CORS
- Deployed: anyone on the network gets shell access

### SEC2. Auth disabled by default (`config/serve.rs:54-63`)

`ServeAuthConfig::default()` sets `enabled: false`. All ~85 API routes are unprotected
out of the box. The auth system is well-implemented when enabled (API keys, JWT, scopes,
secret scrubbing) — but it's off.

### SEC3. `PORT` env var switches to 0.0.0.0 without auth warning (`lib.rs:225-233`)

When `PORT` is set (standard in Railway/Fly deployments), the server binds to `0.0.0.0`
(publicly accessible). The auth config stays disabled. There is a log message but no
warning about the security implication.

**Combined impact**: A Railway deployment with default config exposes an unauthenticated
HTTP server with full shell access to the internet.

### SEC4. CORS is fully permissive by default (`routes/middleware.rs:426-437`)

```rust
if cors_origins.is_empty() {
    CorsLayer::permissive()  // Any origin, any header, any method
}
```

Default config has `cors_origins: Vec::new()`, so CORS is permissive. Any website can
make requests to the local server. Combined with no auth, any website can:
- Read plan data, episodes, config
- Create terminal sessions (shell access)
- Trigger plan runs

### SEC5. `--share` creates PUBLIC GitHub gists with unscrubbed output (`share.rs:84`)

```rust
"--public"
```

The gist includes full agent output text which could contain codebase source, error
messages with file paths, or any content the agent generated. No secret scrubbing is
applied to the share payload.

### SEC6. Terminal accepts arbitrary commands from request (`terminal.rs:115-125`)

The `create_session` handler accepts arbitrary `command` strings and `workdir` paths
from the HTTP request without validation or sandboxing. Even with auth enabled, a
compromised or malicious client can spawn arbitrary processes.

---

## 2. `roko serve` — HTTP Server

### Startup (`lib.rs:287-345`)

1. Resolves bind address from config (default `127.0.0.1:6677`)
2. Checks `PORT` env var override (rebinds to `0.0.0.0:PORT`)
3. Binds TCP listener
4. Starts axum with graceful shutdown on Ctrl-C

### Issues

**SV1. No helpful error on port conflict** (`lib.rs:287-289`)

User gets: `Error: bind to 127.0.0.1:6677 / Caused by: Address already in use (os error 48)`

No suggestion to use `--port`, no detection of what process holds the port, no auto-retry
with alternative port.

**SV2. `create_share` writes empty transcripts** (`routes/shared_runs.rs:89-104`)

`POST /api/runs/{id}/share` creates a `RunTranscript` with all empty fields (empty agent,
empty prompt, `success: false`, no gates, no output). Does not look up actual run data.
The "share" endpoint creates a useless placeholder.

**SV3. Share.tsx fetches from wrong endpoint** (`demo/demo-app/src/pages/Share.tsx:28`)

```typescript
get<Receipt>('/api/share/${token}')  // WRONG - always 404
```

Backend route is `/api/shared/${token}` (note: `shared` not `share`). The `ShareView.tsx`
in the dashboard directory correctly uses `/api/shared/${token}`. The top-level Share page
is permanently broken.

---

## 3. TUI Dashboard (`roko dashboard`)

### What works well
- 10 tabs (F1-F10): Dashboard, Plans, Agents, Git, Logs, Config, Inspect, Marketplace,
  Atelier, Learning
- **Does not require `roko serve`** — file-based via filesystem watchers
- Real-time file watching with 200ms debounce, polling fallback
- Optional WebSocket connection to serve for live agent output
- Approval modal for orchestrator approval requests
- Graceful degradation with no data (empty widgets, not crashes)

### Issues

**TUI1. No ability to control agents from dashboard**

Can observe agents and approve requests, but cannot: stop agents, restart agents, send
prompts, inject signals. Interaction is limited to approvals and observation.

**TUI2. WebSocket reconnection is silent** (`tui/ws_client.rs`)

WS client uses exponential backoff (1s-30s) on disconnect. No indicator in the TUI that
the connection is broken or reconnecting. Agent output silently stops updating.

---

## 4. Demo App

### Connection handling
- Hardcoded to `http://{hostname}:6677` (`serve-url.ts`)
- `useApiWithFallback` probes `/health` with 2-second timeout
- Falls back to hardcoded demo data if server unreachable

### Issues

**DEMO1. `useServerHealth` lies about connection status** (`useServerHealth.ts:22-29`)

```typescript
// On first check failure, show as connected (demo mode)
// so the landing page looks alive for investors
setStatus(checked ? 'disconnected' : 'connected');
```

First health check failure reports `'connected'`. Explicitly for investor demos. Misleads
actual users about whether serve is running.

---

## 5. SSE / WebSocket

### What works
- SSE: `GET /api/events`, `GET /api/sse` — real data from StateHub with replay support
- SSE: `GET /api/workflow/events` — real RuntimeEvents
- WS: `/ws`, `/roko-ws` — event bus with topic filtering and back-pressure
- WS: `/ws/terminal/{id}` — PTY bridge with resize support
- Heartbeat/keepalive on SSE endpoints
- Cursor-based replay for reconnection

### Issues

**WS1. No server-side WebSocket ping** (`routes/ws.rs`)

The `/ws` handler never sends `Ping` frames. Idle connections may be dropped by
intermediate proxies without the server knowing.

---

## Anti-Patterns

1. **Security-off-by-default**: Auth, CORS restrictions, and terminal sandboxing are all
   disabled by default. The deployment path (`PORT` env var) makes things worse by
   switching to public bind without enabling any protections.

2. **Defense in depth missing**: Even when auth is enabled, terminal routes bypass it.
   Command injection is possible even with auth. There's no principle of least privilege.

3. **Demo lies propagating**: The `useServerHealth` hack ("looks alive for investors")
   will mislead real users. Demo-mode hacks should be behind explicit flags.

4. **Endpoint naming inconsistency**: `/api/share/` vs `/api/shared/` breaks the frontend.
   No integration test catches this.

---

## Root Cause Fix

### Security (must-fix before any public deployment)

1. **Auth enabled by default** — or at minimum, enabled automatically when binding to
   0.0.0.0. The `PORT` env var path should force auth on.

2. **Terminal routes behind auth middleware** — mount inside the `/api` prefix with
   admin scope required. No exceptions.

3. **CORS restricted by default** — default to `["http://localhost:*"]` not permissive.
   Require explicit `cors_origins` config for production use.

4. **Terminal command allowlist** — only allow spawning configured shells, not arbitrary
   commands. Validate workdir against workspace root.

5. **`--share` uses private gists** — or at minimum, runs the `LogScrubber` on the
   payload before sharing.

### UX

6. **Port conflict suggestion** — detect the conflicting process and suggest `--port`.
7. **Fix Share.tsx endpoint** — `/api/share/` → `/api/shared/`.
8. **Remove investor demo hack** — or gate behind `ROKO_DEMO_MODE` env var.

---

## Checklist

### Security (P0)
- [ ] Enable auth by default (or auto-enable on 0.0.0.0 bind)
- [ ] Move terminal routes inside auth middleware
- [ ] Restrict default CORS to localhost
- [ ] Add terminal command allowlist
- [ ] Validate terminal workdir against workspace root
- [ ] Run LogScrubber on share payloads
- [ ] Change `--share` to private gists by default

### Server
- [ ] Helpful error on port conflict (suggest `--port`)
- [ ] Fix `create_share` to populate actual run data
- [ ] Fix `Share.tsx` endpoint path (`/api/share/` → `/api/shared/`)

### Dashboard
- [ ] Add agent control from TUI (stop/restart)
- [ ] Show WebSocket connection status indicator

### Demo
- [ ] Remove `useServerHealth` investor hack (or gate behind env var)
- [ ] Add integration test for Share page endpoint
