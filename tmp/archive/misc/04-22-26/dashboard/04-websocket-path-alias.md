# Task 04: WebSocket Path Alias + SSE Compatibility

**Priority**: P0
**Crate**: `roko-serve`
**Files**: `crates/roko-serve/src/routes/ws.rs`, `crates/roko-serve/src/lib.rs` (router registration)

## Problem

The dashboard's WebSocket client connects to `/roko-ws`, but roko-serve exposes `/ws`.

### Path mismatch

**Dashboard code** (`nunchi-dashboard/src/services/rokoWs.ts`):
```typescript
const wsUrl = `${BASE_URL.replace(/^http/, 'ws')}/roko-ws`;
```

**Roko-serve** (`crates/roko-serve/src/routes/ws.rs`):
```rust
// Registered as GET /ws
```

The dashboard will get a 404 when trying to connect to `/roko-ws`.

### SSE fallback path

The dashboard falls back to SSE after 3 failed WebSocket attempts. It connects to:
```typescript
const sseUrl = `${BASE_URL}/api/events`;
```

Roko-serve has SSE at both `/api/events` and `/api/sse`. Verify `/api/events` works correctly.

### CORS for WebSocket upgrade

The dashboard runs on a different port (typically :3000 or :5173 for Vite dev server).
WebSocket upgrade requests need CORS headers. Verify roko-serve's CORS middleware
applies to the WebSocket upgrade path.

## Implementation

### Step 1: Add `/roko-ws` alias

In the router registration (likely in `crates/roko-serve/src/lib.rs` or wherever the
axum router is assembled):

```rust
// Keep existing /ws for backwards compatibility
.route("/ws", get(ws_handler))
// Add alias for dashboard
.route("/roko-ws", get(ws_handler))
```

Both paths should point to the same handler. Do NOT remove `/ws` — other consumers may use it.

### Step 2: Verify SSE at /api/events

The dashboard expects Server-Sent Events at `GET /api/events` with this format:

```
id: 42
data: {"type":"plan_started","plan_id":"uuid"}

id: 43
data: {"type":"task_completed","task_id":"uuid","plan_id":"uuid","success":true}
```

Key requirements:
- Each event has a monotonic `id:` field
- `data:` is a single-line JSON object
- Events are the same payloads as WebSocket events (same `type` field values)
- Supports `Last-Event-ID` header for replay on reconnect

Read the SSE handler in `crates/roko-serve/src/routes/sse.rs` and verify these properties.

### Step 3: Verify CORS covers WebSocket

Check the CORS middleware configuration in roko-serve. It should:
- Allow `Origin: http://localhost:3000` (and other dev origins)
- Allow `Origin: http://localhost:5173` (Vite dev server)
- Allow `Upgrade: websocket` header
- Not block the WebSocket handshake

The CORS middleware is likely in `lib.rs` or a middleware module. Look for
`tower_http::cors::CorsLayer` or similar.

If CORS is too restrictive, add the dev origins. For production, this should be
configurable via `roko.toml` or environment variables.

## Files to modify

| File | Change |
|------|--------|
| `crates/roko-serve/src/lib.rs` | Add `/roko-ws` route alias |
| `crates/roko-serve/src/routes/ws.rs` | No changes expected (handler stays the same) |
| CORS middleware file | Add dev origins if missing |

## Verification

### Automated

```bash
cargo build -p roko-serve
cargo test -p roko-serve
cargo clippy -p roko-serve --no-deps -- -D warnings
```

### Manual — WebSocket alias

```bash
cargo run -p roko-cli -- serve &

# Test old path still works
websocat ws://127.0.0.1:6677/ws --one-message 2>&1 | head -1
# Should connect (not 404)

# Test new path works
websocat ws://127.0.0.1:6677/roko-ws --one-message 2>&1 | head -1
# Should connect (not 404)

# If websocat is not installed, use curl to test upgrade request:
curl -i -N \
  -H "Connection: Upgrade" \
  -H "Upgrade: websocket" \
  -H "Sec-WebSocket-Version: 13" \
  -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" \
  http://127.0.0.1:6677/roko-ws
# Should return 101 Switching Protocols (not 404)
```

### Manual — SSE fallback

```bash
# Test SSE endpoint
curl -N -H "Accept: text/event-stream" http://127.0.0.1:6677/api/events &
SSE_PID=$!

# Wait for keepalive or initial event
sleep 3

# Trigger an event (e.g., create an idea to generate a ServerEvent)
curl -s -X POST http://127.0.0.1:6677/api/prds/ideas \
  -H 'Content-Type: application/json' \
  -d '{"text": "test SSE event"}'

# Check SSE output
sleep 2
kill $SSE_PID

# The SSE stream should have shown events with id: and data: lines
```

### Manual — CORS

```bash
# Test CORS preflight for WebSocket upgrade origin
curl -i -X OPTIONS http://127.0.0.1:6677/roko-ws \
  -H "Origin: http://localhost:5173" \
  -H "Access-Control-Request-Method: GET" \
  -H "Access-Control-Request-Headers: Upgrade"

# Should return Access-Control-Allow-Origin header
# Should NOT return 403 or missing CORS headers
```

## Acceptance criteria

- [ ] `ws://host/roko-ws` connects successfully (WebSocket upgrade)
- [ ] `ws://host/ws` still works (backward compat)
- [ ] Both paths deliver the same events
- [ ] `GET /api/events` streams SSE with `id:` and `data:` fields
- [ ] SSE supports `Last-Event-ID` for replay
- [ ] CORS allows connections from `localhost:5173` and `localhost:3000`
- [ ] All existing tests still pass
- [ ] No new clippy warnings
