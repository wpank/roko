# Serve Rate Limiting

**Priority**: P1 — no request rate limits on public-facing HTTP server
**Size**: S (1 day)
**Crate**: `crates/roko-serve/`

---

## Problem

`roko serve` exposes ~317 HTTP routes on port 6677 with no global rate limiting
middleware. Any client can send unlimited requests to any endpoint, including
computationally expensive routes like inference dispatch, agent creation, and
webhook ingestion.

The trigger runtime has its own per-trigger `rate_limit` (window + max_fires), but
that only throttles fired trigger events — not inbound HTTP requests. There is no
per-IP, per-API-key, or per-route rate limiting at the HTTP layer.

This means a misconfigured client, runaway script, or hostile actor can exhaust server
resources (CPU, memory, open connections, LLM API quota) without any back-pressure.

---

## Where to look

- `crates/roko-serve/src/routes/middleware.rs` — existing middleware stack (auth,
  CORS, body limits, secret scrubbing)
- `crates/roko-serve/src/routes/mod.rs` — router construction
- `crates/roko-serve/src/lib.rs` — server startup and layer composition

---

## What to do

**Step 1.** Add `tower-governor` (or `tower_http::limit`) as a dependency. `tower-governor`
wraps the `governor` rate-limiting crate for Tower services and supports per-IP keying
out of the box.

**Step 2.** Add a global rate limiter as an Axum layer. Suggested defaults:

| Route class | Limit | Rationale |
|---|---|---|
| Global default | 100 req/s per IP | Reasonable baseline |
| `/api/run`, `/api/gateway/*` | 10 req/s per API key | LLM dispatch is expensive |
| `/api/webhooks/*` | 30 req/s per IP | GitHub/Slack webhooks burst |
| `/ws`, SSE endpoints | 5 connections per IP | Long-lived connections |

**Step 3.** Make limits configurable via `roko.toml`:

```toml
[serve.rate_limit]
enabled = true
global_rps = 100
inference_rps = 10
webhook_rps = 30
```

**Step 4.** Return `429 Too Many Requests` with a `Retry-After` header when limits
are exceeded.

---

## Acceptance criteria

- [ ] Global rate limiter applied to all HTTP routes
- [ ] Inference/gateway routes have a tighter per-key limit
- [ ] `429 Too Many Requests` returned with `Retry-After` header
- [ ] Limits configurable via `roko.toml [serve.rate_limit]`
- [ ] Rate limiting disabled when `serve.rate_limit.enabled = false`
- [ ] All existing tests pass (`cargo test -p roko-serve`)

### Not in scope

- DDoS protection (that's infrastructure-level, not application-level)
- Per-user rate limiting (requires user identity, which depends on auth mode)

---

**Origin**: productionizing audit (2026-08-13), binary-issues audit (2026-08-17)
