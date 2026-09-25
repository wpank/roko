# Monitoring Guide

> Operator reference for the `roko serve` HTTP control plane (~376 canonical
> routes on `:6677`). This guide covers the minimum set of endpoints needed
> for production monitoring, alerting, and dashboarding.
>
> For the full route inventory see `docs/v3/26-HTTP-API.md` and
> `GET /api/openapi.json`.

---

## 1. Health Endpoints

### `GET /health`

Bare liveness probe — no authentication required. Use this from load balancers
and uptime checkers.

```
200 OK
{ "status": "ok", "uptime_secs": 3721 }

503 Service Unavailable
{ "status": "shutting_down" }
```

### `GET /api/health`

Richer readiness probe. Returns provider health summary, active plan count,
supervised agent count, JWKS cache health, and disk/memory pressure signals.
Requires API key auth when `serve.auth.enabled = true`.

```json
{
  "status": "ok",
  "uptime_secs": 3721,
  "active_plans": 2,
  "active_agents": 4,
  "providers": {
    "total": 3,
    "healthy": 2,
    "degraded": 1,
    "unhealthy": 0
  },
  "jwks": { "healthy": true }
}
```

**Alert triggers:**
- `status != "ok"`
- `providers.unhealthy > 0`
- `providers.healthy == 0`

### `GET /api/providers/{id}/health`

Per-provider circuit-breaker state. Returns `HealthState` (`healthy`,
`probing`, `unhealthy`), error rate, and p95 latency. Use this to surface
exactly which provider is in trouble.

```json
{
  "provider": "anthropic",
  "state": "healthy",
  "error_rate": 0.02,
  "latency_p95_ms": 1850
}
```

**Alert triggers:**
- `state == "unhealthy"` for any provider
- `error_rate > 0.10` (10%)
- `latency_p95_ms > 30000` (30 s)

### `GET /api/relay/health`

Relay client connectivity. Use when the agent relay feature is enabled.

```json
{ "connected": true, "pending_ack": 0 }
```

---

## 2. Metrics Endpoints

### `GET /api/metrics`

Per-30-second sampled workspace metrics snapshot. Returns token counts,
latency histograms, gate pass rates, c-factor, and cost totals. This is the
primary feed for a time-series dashboard.

### `GET /api/metrics/summary`

Single-request rollup for a status widget: task counts, gate pass/fail ratio,
average cost per task, total session spend.

### `GET /api/metrics/prometheus`

Prometheus text-format scrape endpoint. Mount as a Prometheus scrape target:

```yaml
scrape_configs:
  - job_name: roko
    static_configs:
      - targets: ["localhost:6677"]
    metrics_path: /api/metrics/prometheus
    bearer_token: "<your-api-key>"
```

Key metrics exported:
- `roko_gate_pass_total` / `roko_gate_fail_total` — per-rung gate counters
- `roko_token_input_total` / `roko_token_output_total` — token usage counters
- `roko_cost_usd_total` — cumulative USD spend
- `roko_agent_active` — currently supervised agents
- `roko_plan_active` — currently running plans

### `GET /api/metrics/gate_rate`

Gate pass rate time series (last N samples). Use to alert on gate regression.

```json
{ "pass_rate": 0.87, "samples": 50 }
```

**Alert trigger:** `pass_rate < 0.70` sustained for 5+ minutes.

### `GET /api/learning/costs` (alias: `/api/learn/costs`)

Accumulated cost totals from the learning subsystem. Returns per-model and
per-role breakdown for budget enforcement dashboards.

```json
{
  "total_usd": 4.27,
  "by_model": { "claude-opus-4-6": 3.10, "claude-sonnet-4-6": 1.17 },
  "by_role": { "implementer": 2.85, "researcher": 1.42 }
}
```

**Alert trigger:** `total_usd > budget_ceiling`.

### `GET /api/plans/{id}/costs`

Per-plan cost breakdown. Useful when attributing spend to specific work items.

---

## 3. SSE / WebSocket Subscription

### `GET /api/events` or `GET /api/sse`

Server-Sent Events stream for real-time workspace events. The server replays
up to the last 512 events from the StateHub ring buffer on connect, then
streams new events as they arrive.

```
Accept: text/event-stream
Authorization: Bearer <api-key>
```

Each event is a newline-delimited JSON-RPC notification:

```
event: plan.task.completed
data: {"kind":"plan.task.completed","session_id":"...","task_id":"env-setup","gate_passed":true}

event: provider.health.changed
data: {"kind":"provider.health.changed","provider":"anthropic","state":"unhealthy"}
```

Common event kinds for alerting:

| Kind | Alert on |
|---|---|
| `provider.health.changed` | `state == "unhealthy"` |
| `plan.task.failed` | any occurrence |
| `gate.failed` | repeated failures on same rung |
| `budget.exceeded` | any occurrence |
| `agent.crashed` | any occurrence |

**Reconnect pattern:** send `Last-Event-ID` header with the last received
event ID. The StateHub ring buffer will replay missed events up to 512 entries.

### `GET /ws` or `GET /roko-ws`

WebSocket variant of the same event stream. Supports cursor-based replay and
server-side event filtering.

```
ws://localhost:6677/ws?filter=plan.*,provider.*&cursor=<last-event-id>
```

---

## 4. Common Alerting Patterns

### Provider circuit break

```
GET /api/health
providers.unhealthy > 0
→ GET /api/providers/{id}/health for each known provider
→ page on state == "unhealthy"
```

Or via SSE:

```
event: provider.health.changed
data.state == "unhealthy"
```

### Cost overrun

```
GET /api/metrics/summary every 5 min
total_usd > configured ceiling
→ alert + pause plan execution via POST /api/plans/{id}/pause
```

### Plan failure rate

```
GET /api/metrics/gate_rate every 2 min
pass_rate < 0.70 for 3 consecutive samples
→ alert + check GET /api/gates/history for recent failures
```

### Disk pressure

```
GET /api/status/disk every 15 min
free_bytes < 2 GB
→ alert → roko doctor disk → roko cache prune
```

### Plan stuck / no progress

```
GET /api/status
active_plans > 0 and last_completed_task_at older than 10 min
→ check GET /api/plans/{id}/status for checkpoint details
→ GET /api/operations/{id} for running operation detail
```

---

## 5. Authentication

Set `ROKO_API_KEY` or configure `serve.auth.api_keys` in `roko.toml`.
Pass the key as `Authorization: Bearer <key>` or `X-API-Key: <key>`.

`GET /health` is exempt from auth and safe for unauthenticated probes.

---

> For route permissions, rate limit tiers, and the full OpenAPI schema, see
> `docs/v3/26-HTTP-API.md` and `GET /api/openapi.json`.
