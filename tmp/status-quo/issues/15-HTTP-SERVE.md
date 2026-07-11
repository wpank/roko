# HTTP Control Plane (roko-serve) Issues

## Critical

### Relay proxy completely unauthenticated
- `routes/relay_proxy.rs:23-31`, `routes/mod.rs:247-248`: Mounted outside `/api` nest, outside auth middleware. `/relay/{*path}` + 2 WS routes — full GET/POST/DELETE/WS unauthed.

### Read-scope auth fallback allows mutation
- `middleware.rs:413-417`: `require_scope` falls back to `"read"` when `AuthContext` absent. Missing extension should be 401, not silent grant.
- `middleware.rs:355-386`: `POST /api/run`, `POST /api/research/*`, `POST /api/jobs`, `POST /api/inference/complete`, etc. not listed → fall through to `"read"` default.

## High

### Scrub middleware buffers 16 MiB synchronously
- `middleware.rs:564`: Every non-SSE response buffered up to 16 MiB before forwarding. Under concurrent load with large responses = memory pressure.

### Slack webhook signature bypass
- `webhooks.rs:107-121`: `url_verification` challenge returned before HMAC signature check.

## Medium

### WebSocket proxy silently drops on upstream failure
- `proxy_ws.rs:13-15`: Client socket dropped with no close frame or error payload.

### `/metrics` and `/health` expose internal state without auth
- `routes/mod.rs:235-239`: Plan/agent counts, provider stats, TTFT histograms all unauthenticated.

### CORS allows any localhost origin
- `middleware.rs:489-521`: Default permits any `localhost:*` origin. Risk if server reachable outside loopback with auth disabled.

### `/relay/` trailing slash hits wildcard → double prefix in upstream URL
- `relay_proxy.rs:29`: Request to `/relay/` constructs `{base}/relay/` — likely 404s upstream.
