# HTTP Monitoring Guide

This guide documents the monitoring endpoints exposed by `roko serve` and how
to use them for observing plan execution, system health, and learning metrics.

Start the server:

```bash
cargo run -p roko-cli -- serve
# Listening on http://127.0.0.1:6677
```

All API endpoints are prefixed with `/api` unless noted otherwise. When auth
is disabled (the default for local development), no API key is needed.

## Table of Contents

- [Liveness and Readiness Probes](#liveness-and-readiness-probes)
- [System Health](#system-health)
- [Plan Monitoring](#plan-monitoring)
- [Run Observability](#run-observability)
- [Real-time Event Streams (SSE)](#real-time-event-streams-sse)
- [Metrics and Prometheus](#metrics-and-prometheus)
- [Learning and Feedback](#learning-and-feedback)
- [Provider Health](#provider-health)
- [Disk and Resource Monitoring](#disk-and-resource-monitoring)
- [Agent Monitoring](#agent-monitoring)
- [Gate Pipeline](#gate-pipeline)
- [Dashboard Integration](#dashboard-integration)

---

## Liveness and Readiness Probes

These top-level endpoints require no auth and have no `/api` prefix. They are
designed for load balancers and container orchestrators.

### GET /health

Bare liveness probe. Returns 200 while the process is alive.

```bash
curl -s http://localhost:6677/health | jq .
```

```json
{
  "status": "ok",
  "version": "0.1.0",
  "uptime_secs": 3421
}
```

### GET /ready

Readiness probe. Returns 200 when ready, 503 during graceful shutdown.

```bash
curl -s -o /dev/null -w '%{http_code}' http://localhost:6677/ready
# 200
```

```json
{
  "status": "ok",
  "version": "0.1.0",
  "uptime_secs": 3421
}
```

During shutdown the response changes to:

```json
{
  "status": "shutting_down",
  "version": "0.1.0",
  "uptime_secs": 3421
}
```

---

## System Health

### GET /api/health

Rich health check with provider status, JWKS state, and StateHub telemetry.
Returns HTTP 200 for "ok" or "degraded", HTTP 503 for "unhealthy".

```bash
curl -s http://localhost:6677/api/health | jq .
```

```json
{
  "status": "ok",
  "version": "0.1.0",
  "uptime_secs": 1200,
  "active_plans": 1,
  "active_agents": 3,
  "active_runs": 0,
  "providers": {
    "total": 2,
    "healthy": 2,
    "degraded": 0,
    "unhealthy": 0
  },
  "jwks": {
    "configured": false,
    "fresh": false,
    "key_count": 0,
    "age_secs": null,
    "stale": false,
    "fail_closed": true
  },
  "statehub": {
    "cursor": "0x2f",
    "events_retained": 47,
    "snapshot": {
      "plans_active": 1,
      "tasks_active": 2,
      "agents_active": 3,
      "gates_passed": 14,
      "gates_failed": 1,
      "episodes_total": 15,
      "errors_total": 0,
      "cost_usd_total": 0.42
    }
  }
}
```

Health status classification:
- **ok** -- all providers healthy, JWKS fresh (if configured)
- **degraded** -- some providers unhealthy/degraded, or JWKS stale
- **unhealthy** -- no healthy providers, or JWKS fail-closed when configured

### GET /api/status

Session status overview with process supervision state.

```bash
curl -s http://localhost:6677/api/status | jq .
```

```json
{
  "session_id": "abc123",
  "workdir": "/path/to/project",
  "daemon_running": false,
  "signal_count": 42,
  "episode_count": 15,
  "last_episode_passed": true,
  "supervised_processes": [],
  "process_session_ledger": "/path/to/project/.roko/process-sessions.json",
  "process_sessions": { "active": 0, "total": 5, "recent_24h": 2 }
}
```

---

## Plan Monitoring

### GET /api/plans

List all plans with completion status.

```bash
curl -s http://localhost:6677/api/plans | jq .
```

```json
[
  {
    "id": "demo-hello",
    "title": "Demo Hello Plan",
    "task_count": 1,
    "completed": true,
    "completed_task_count": 1
  },
  {
    "id": "my-feature",
    "title": "Implement Feature X",
    "task_count": 5,
    "completed": false,
    "completed_task_count": 3
  }
]
```

### GET /api/plans/{id}

Full plan details including all task data.

```bash
curl -s http://localhost:6677/api/plans/demo-hello | jq .
```

### GET /api/plans/{id}/tasks

Task list for a specific plan with dependency and completion info.

```bash
curl -s http://localhost:6677/api/plans/demo-hello/tasks | jq .
```

```json
{
  "plan_id": "demo-hello",
  "task_count": 1,
  "tasks": [
    {
      "id": "DEMO-T01",
      "description": "Create the Roko plan-runner smoke artifact",
      "depends_on": [],
      "files": ["demo/roko-plan-smoke-output.md"],
      "completed": true,
      "status": "completed"
    }
  ]
}
```

### GET /api/plans/{id}/status

Execution status for a running or completed plan.

```bash
curl -s http://localhost:6677/api/plans/demo-hello/status | jq .
```

### GET /api/plans/{id}/gates

Gate results for all tasks in a plan.

```bash
curl -s http://localhost:6677/api/plans/demo-hello/gates | jq .
```

### GET /api/plans/{id}/costs

Cost breakdown for a plan.

```bash
curl -s http://localhost:6677/api/plans/demo-hello/costs | jq .
```

### POST /api/plans/{id}/execute

Start plan execution via HTTP (returns immediately with an operation ID).

```bash
curl -s -X POST http://localhost:6677/api/plans/demo-hello/execute | jq .
```

### POST /api/plans/{id}/pause / POST /api/plans/{id}/resume

Pause or resume a running plan.

```bash
curl -s -X POST http://localhost:6677/api/plans/demo-hello/pause | jq .
curl -s -X POST http://localhost:6677/api/plans/demo-hello/resume | jq .
```

### POST /api/plans/{id}/estimate

Estimate cost and duration before execution.

```bash
curl -s -X POST http://localhost:6677/api/plans/demo-hello/estimate | jq .
```

---

## Run Observability

Per-run endpoints provide bounded, indexed access to execution data.

### GET /api/dashboard/runs

Summary of recent runs with status and key metrics.

```bash
curl -s http://localhost:6677/api/dashboard/runs | jq .
```

### GET /api/runs/{run_id}

Detailed information about a specific run.

```bash
curl -s http://localhost:6677/api/runs/abc123 | jq .
```

### GET /api/runs/{run_id}/events

Paginated event log for a run. Supports cursor-based pagination.

```bash
# First page
curl -s 'http://localhost:6677/api/runs/abc123/events?limit=50' | jq .

# Next page (using cursor from previous response)
curl -s 'http://localhost:6677/api/runs/abc123/events?cursor=50&limit=50' | jq .

# Filter by event type
curl -s 'http://localhost:6677/api/runs/abc123/events?types=gate_result,task_completed' | jq .
```

### GET /api/runs/{run_id}/tasks

Task-level detail for a run.

```bash
curl -s http://localhost:6677/api/runs/abc123/tasks | jq .
```

### GET /api/runs/{run_id}/tasks/{task_id}/attempts

Attempt history for a specific task (useful for debugging retries).

```bash
curl -s http://localhost:6677/api/runs/abc123/tasks/T01/attempts | jq .
```

### GET /api/runs/{run_id}/gates

Gate results for a run.

```bash
curl -s http://localhost:6677/api/runs/abc123/gates | jq .
```

### GET /api/runs/{run_id}/metrics

Aggregated metrics for a run (cost, tokens, timing).

```bash
curl -s http://localhost:6677/api/runs/abc123/metrics | jq .
```

### GET /api/runs/{run_id}/logs

Structured logs for a run with optional filtering.

```bash
curl -s 'http://localhost:6677/api/runs/abc123/logs?level=error&limit=20' | jq .
```

### GET /api/runs/{run_id}/events/stream

SSE stream scoped to a single run. Use this for live-tailing a specific
execution.

```bash
curl -N http://localhost:6677/api/runs/abc123/events/stream
```

---

## Real-time Event Streams (SSE)

### GET /api/events (alias: GET /api/sse)

Server-Sent Events stream of all dashboard events. This is the primary
real-time monitoring endpoint.

```bash
curl -N http://localhost:6677/api/events
```

Each event is a JSON payload with an `id:` field for reconnection:

```
id: 42
data: {"PlanStarted":{"plan_id":"demo-hello","tasks_total":1}}

id: 43
data: {"TaskStarted":{"plan_id":"demo-hello","task_id":"DEMO-T01"}}

id: 44
data: {"GateResult":{"plan_id":"demo-hello","task_id":"DEMO-T01","gate":"structural","passed":true}}
```

#### Reconnection

The stream supports cursor-based reconnection. When reconnecting after a
disconnect, provide the last seen event ID:

```bash
# Via Last-Event-ID header (highest precedence)
curl -N -H 'Last-Event-ID: 42' http://localhost:6677/api/events

# Via query parameter (for EventSource clients that cannot set headers)
curl -N 'http://localhost:6677/api/events?lastEventId=42'
```

If the requested cursor has fallen out of the ring buffer, the server sends
a single `event: gap` frame containing a full materialized snapshot, then
continues with live events.

#### Event types

| Event Type | Description |
|---|---|
| `plan_started` | A plan began execution |
| `plan_completed` | A plan finished (all tasks done or failed) |
| `task_started` | A task dispatch began |
| `task_completed` | A task finished (passed or failed) |
| `task_phase_changed` | A task moved between phases |
| `agent_spawned` | An agent process was created |
| `agent_completed` | An agent finished its work |
| `agent_output` | Streaming agent output text |
| `gate_result` | A gate rung pass/fail result |
| `gate_rung_started` | A gate rung began execution |
| `gate_output_line` | Streaming gate command output |
| `phase_transition` | Runner phase change |
| `efficiency_event` | Per-turn efficiency data |
| `episode_recorded` | An episode was persisted |
| `cascade_router_updated` | Model routing weights changed |
| `gate_thresholds_updated` | Adaptive thresholds changed |
| `error` | An error occurred |
| `diagnosis` | Conductor diagnosis event |

#### Keep-alive

The server sends a keep-alive comment every 8 seconds to prevent proxy
timeouts. The text is `keepalive`.

### GET /api/workflow/events

RuntimeEvent-typed SSE stream for the WorkflowEngine (single-prompt `roko run`
commands). Uses the same keep-alive and reconnection behavior.

```bash
curl -N http://localhost:6677/api/workflow/events
```

---

## Metrics and Prometheus

### GET /metrics

Standard Prometheus scrape endpoint (top-level, no `/api` prefix, no auth).

```bash
curl -s http://localhost:6677/metrics
```

### GET /api/metrics

Metric snapshots as JSON.

```bash
curl -s http://localhost:6677/api/metrics | jq .
```

### GET /api/metrics/summary

Aggregate execution and learning metrics for a time period.

```bash
curl -s 'http://localhost:6677/api/metrics/summary?period=last_7_days' | jq .
```

```json
{
  "period": "last_7_days",
  "active_plans": 1,
  "c_factor": 0.71,
  "experiments_active": 0
}
```

### GET /api/metrics/success_rate

Task success rate over time.

```bash
curl -s http://localhost:6677/api/metrics/success_rate | jq .
```

### GET /api/metrics/velocity

Task completion velocity.

```bash
curl -s http://localhost:6677/api/metrics/velocity | jq .
```

### GET /api/metrics/gate_rate

Gate pass/fail rates.

```bash
curl -s http://localhost:6677/api/metrics/gate_rate | jq .
```

### GET /api/metrics/model_efficiency

Per-model efficiency comparison.

```bash
curl -s http://localhost:6677/api/metrics/model_efficiency | jq .
```

### GET /api/metrics/c_factor

Composite c-factor metric with per-agent contributions and fleet summary.

```bash
curl -s http://localhost:6677/api/metrics/c_factor | jq .
```

### GET /api/metrics/coverage

Test and gate coverage metrics.

```bash
curl -s http://localhost:6677/api/metrics/coverage | jq .
```

### GET /api/metrics/feedback_latency

Feedback loop latency statistics.

```bash
curl -s http://localhost:6677/api/metrics/feedback_latency | jq .
```

### GET /api/metrics/engagement

Agent engagement metrics.

```bash
curl -s http://localhost:6677/api/metrics/engagement | jq .
```

### GET /api/metrics/experiments

Active and concluded prompt experiment metrics.

```bash
curl -s http://localhost:6677/api/metrics/experiments | jq .
```

---

## Learning and Feedback

### GET /api/learn/efficiency

Aggregated efficiency data: total cost, per-task cost, token usage, and
timing with cost trend.

```bash
curl -s http://localhost:6677/api/learn/efficiency | jq .
```

```json
{
  "total_cost": 2.45,
  "cost_per_task": 0.35,
  "tokens_per_task": 4200.0,
  "avg_task_duration": 12000.0,
  "data_quality": { "has_real_data": true, "entry_count": 7, "null_cost_count": 0 },
  "cost_trend": [
    { "timestamp": "2026-09-03T10:00:00Z", "cost_usd": 0.35, "cumulative_cost_usd": 0.35 }
  ],
  "tasks": []
}
```

### GET /api/learn/cascade-router

Cascade router snapshot showing model routing weights and availability.

```bash
curl -s http://localhost:6677/api/learn/cascade-router | jq .
```

### GET /api/learning/cascade

Detailed cascade routing with model weights and routing stats.

```bash
curl -s http://localhost:6677/api/learning/cascade | jq .
```

### GET /api/learn/cost-tiers

Model cost tier distribution.

```bash
curl -s http://localhost:6677/api/learn/cost-tiers | jq .
```

### GET /api/learn/gate-thresholds

Current adaptive gate threshold state.

```bash
curl -s http://localhost:6677/api/learn/gate-thresholds | jq .
```

### GET /api/learn/adaptive-thresholds

Structured adaptive threshold summary with per-rung EMA values.

```bash
curl -s http://localhost:6677/api/learn/adaptive-thresholds | jq .
```

```json
{
  "source": "/path/to/project/.roko/learn/gate-thresholds.json",
  "tracked_rungs": 3,
  "rungs": [
    {
      "rung": 1,
      "ema_pass_rate": 0.85,
      "total_observations": 50,
      "consecutive_passes": 12,
      "suggested_max_retries": 1,
      "should_skip_rung": false
    }
  ]
}
```

### GET /api/learn/experiments

Active and concluded prompt A/B experiments.

```bash
curl -s http://localhost:6677/api/learn/experiments | jq .
```

### GET /api/learning/costs

Runtime cost projection.

```bash
curl -s http://localhost:6677/api/learning/costs | jq .
```

### GET /api/learning/provider-outcomes

Provider/model outcome proof surface.

```bash
curl -s http://localhost:6677/api/learning/provider-outcomes | jq .
```

### GET /api/learning/retries

Retry attempt proof surface.

```bash
curl -s http://localhost:6677/api/learning/retries | jq .
```

### GET /api/learning/runtime-feedback

Joined feedback store overview.

```bash
curl -s http://localhost:6677/api/learning/runtime-feedback | jq .
```

### GET /api/c-factor/trend

C-factor trend over time with hourly buckets.

```bash
# Default: 24h window
curl -s http://localhost:6677/api/c-factor/trend | jq .

# 7-day window
curl -s 'http://localhost:6677/api/c-factor/trend?window=7d' | jq .
```

### GET /api/executor/state

Current executor state snapshot.

```bash
curl -s http://localhost:6677/api/executor/state | jq .
```

---

## Provider Health

### GET /api/providers

List all configured providers with health status and model counts.

```bash
curl -s http://localhost:6677/api/providers | jq .
```

### GET /api/providers/{id}/health

Health details for a specific provider.

```bash
curl -s http://localhost:6677/api/providers/anthropic/health | jq .
```

### POST /api/providers/{id}/test

Send a test prompt to verify provider connectivity.

```bash
curl -s -X POST http://localhost:6677/api/providers/anthropic/test | jq .
```

### GET /api/models

List all configured models.

```bash
curl -s http://localhost:6677/api/models | jq .
```

### GET /api/routing/explain

Explain current routing decisions (which model would be selected and why).

```bash
curl -s http://localhost:6677/api/routing/explain | jq .
```

### GET /api/relay/health

Relay connection diagnostics.

```bash
curl -s http://localhost:6677/api/relay/health | jq .
```

---

## Disk and Resource Monitoring

### GET /api/status/disk

Disk usage telemetry: mount-point stats plus roko-specific directory sizes.

```bash
curl -s http://localhost:6677/api/status/disk | jq .
```

```json
{
  "free_mb": 120000,
  "total_mb": 500000,
  "used_pct": 76,
  "roko_dir_mb": 45,
  "target_dir_mb": 3200,
  "worktrees_mb": 128,
  "pressure": "normal"
}
```

Pressure levels:
- **normal** -- plenty of disk space
- **warning** -- below configured `warn_disk_mb` threshold
- **critical** -- below configured `min_free_disk_mb` threshold (runner will refuse to start)

---

## Agent Monitoring

### GET /api/agents/{id}

Agent details and current state.

```bash
curl -s http://localhost:6677/api/agents/agent-001 | jq .
```

### GET /api/agents/{id}/episodes

Episode history for a specific agent.

```bash
curl -s http://localhost:6677/api/agents/agent-001/episodes | jq .
```

### GET /api/agents/{id}/logs

Proxy agent logs from the per-agent sidecar.

```bash
curl -s http://localhost:6677/api/agents/agent-001/logs | jq .
```

### GET /api/managed-agents

List all managed (supervised) agents.

```bash
curl -s http://localhost:6677/api/managed-agents | jq .
```

---

## Gate Pipeline

### GET /api/gates/summary

Aggregated gate statistics: pass rate, average duration, and per-rung breakdown.

```bash
curl -s http://localhost:6677/api/gates/summary | jq .
```

```json
{
  "compile": {
    "total_runs": 24,
    "pass_rate": 0.92,
    "avg_duration_ms": 1200.0,
    "last_run": {}
  },
  "test": {
    "total_runs": 20,
    "pass_rate": 0.85,
    "avg_duration_ms": 3400.0,
    "last_run": {}
  },
  "rungs": [
    { "rung": 1, "passed_runs": 22, "failed_runs": 2 },
    { "rung": 2, "passed_runs": 17, "failed_runs": 3 }
  ]
}
```

### GET /api/gates/history

Full gate execution history, most recent first.

```bash
# Latest 10 gate results
curl -s 'http://localhost:6677/api/gates/history?limit=10' | jq .
```

### GET /api/gates/{gate_name}/history

History for a specific gate type.

```bash
curl -s http://localhost:6677/api/gates/compile/history | jq .
```

---

## Dashboard Integration

### StateHub Snapshot and Events

These endpoints support building custom dashboards.

### GET /api/statehub/snapshot

Current materialized dashboard snapshot.

```bash
curl -s http://localhost:6677/api/statehub/snapshot | jq .
```

### GET /api/statehub/events

Bounded replay of retained dashboard events with optional filtering.

```bash
# Latest 50 events
curl -s 'http://localhost:6677/api/statehub/events?limit=50' | jq .

# Events after a specific sequence number
curl -s 'http://localhost:6677/api/statehub/events?after_seq=100&limit=50' | jq .

# Filter by event type
curl -s 'http://localhost:6677/api/statehub/events?type=gate_result' | jq .

# Filter by plan
curl -s 'http://localhost:6677/api/statehub/events?plan_id=demo-hello' | jq .

# Filter by task
curl -s 'http://localhost:6677/api/statehub/events?task_id=DEMO-T01' | jq .
```

```json
{
  "after_seq": 0,
  "limit": 50,
  "cursor": "0x2f",
  "events": [
    {
      "seq": 1,
      "cursor": "0x1",
      "ts_millis": 1725350400000,
      "event": { "PlanStarted": { "plan_id": "demo-hello", "tasks_total": 1 } }
    }
  ]
}
```

### GET /api/dashboard

Dashboard scaffold as JSON.

```bash
curl -s http://localhost:6677/api/dashboard | jq .
```

### GET /api/operations/{id}

Check the status of a background operation (returned by async endpoints like
plan execution).

```bash
curl -s http://localhost:6677/api/operations/op-abc123 | jq .
```

```json
{
  "id": "op-abc123",
  "kind": "plan_execution",
  "status": "Running"
}
```

### Additional Observability Endpoints

| Endpoint | Description |
|---|---|
| `GET /api/episodes` | Episode history |
| `GET /api/signals` | Signal log entries |
| `GET /api/retention` | Retention policies and violations |
| `GET /api/parity` | Cross-surface parity matrix |
| `GET /api/truth_map` | Entity truth-source registry |
| `GET /api/diagnosis` | Conductor diagnosis data |
| `GET /api/extensions` | Extension runtime status |

---

## Polling Pattern for Plan Execution

Here is a complete example of starting a plan via HTTP and polling until
completion:

```bash
#!/bin/bash
set -eu

BASE="http://localhost:6677"
PLAN_ID="my-feature"

# Start execution
RESULT=$(curl -s -X POST "$BASE/api/plans/$PLAN_ID/execute")
echo "Started: $RESULT"

# Poll plan status every 10 seconds
while true; do
  STATUS=$(curl -s "$BASE/api/plans/$PLAN_ID/status")
  COMPLETED=$(echo "$STATUS" | jq -r '.completed // false')

  if [ "$COMPLETED" = "true" ]; then
    echo "Plan completed."
    curl -s "$BASE/api/plans/$PLAN_ID/costs" | jq .
    break
  fi

  TASK_COUNT=$(echo "$STATUS" | jq -r '.completed_task_count // 0')
  TOTAL=$(echo "$STATUS" | jq -r '.task_count // 0')
  echo "Progress: $TASK_COUNT/$TOTAL tasks"
  sleep 10
done
```

## SSE Integration with JavaScript

```javascript
const eventSource = new EventSource('http://localhost:6677/api/events');

eventSource.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log(`[${event.lastEventId}]`, data);
};

// Handle gap events (reconnection after buffer overflow)
eventSource.addEventListener('gap', (event) => {
  const gap = JSON.parse(event.data);
  console.log(`Missed ${gap.missed_events} events, got full snapshot`);
  // Replace local state with gap.snapshot
});

eventSource.onerror = () => {
  console.log('Connection lost, EventSource will auto-reconnect');
};
```
