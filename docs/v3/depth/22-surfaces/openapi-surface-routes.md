# Depth: OpenAPI Surface Routes

> Parent: [22-SURFACES](../../22-SURFACES.md) SS6

This file documents the five dedicated HTTP routes that serve typed surface
projections, plus the supporting projection catalog, SSE streaming, and
StateHub history endpoints.

---

## 1. Route Table

All routes are defined in `crates/roko-serve/src/routes/projections.rs` and
registered under the `/api` prefix by the `roko-serve` router.

### Five Named Surface Routes

| Method | Path | Handler | Response Type |
|---|---|---|---|
| GET | `/api/projections/workbench` | `get_workbench_surface` | `ProjectionEnvelope<WorkbenchProjection>` |
| GET | `/api/projections/inbox` | `get_inbox_surface` | `ProjectionEnvelope<InboxProjection>` |
| GET | `/api/projections/canvas` | `get_canvas_surface` | `ProjectionEnvelope<CanvasProjection>` |
| GET | `/api/projections/minimap` | `get_minimap_surface` | `ProjectionEnvelope<MinimapProjection>` |
| GET | `/api/projections/autonomy` | `get_autonomy_surface` | `ProjectionEnvelope<AutonomyProjection>` |

### Supporting Routes

| Method | Path | Handler | Purpose |
|---|---|---|---|
| GET | `/api/projections/catalog` | `projections_catalog` | Returns all 30 projection names, versions, and invalidation policies |
| GET | `/api/projections/telemetry` | `get_telemetry` | Current telemetry state: watchers, circuit breakers, observation counts |
| GET | `/api/projections/telemetry/stream` | `stream_telemetry` | SSE stream of telemetry updates |
| GET | `/api/projections/{name}` | `get_projection` | Generic projection by name with query parameters |
| GET | `/api/projections/{name}/stream` | `stream_projection` | SSE stream for any named projection |
| GET | `/api/statehub/lens-runtimes` | `get_lens_runtimes` | All live queued Lens runtimes |
| GET | `/api/statehub/lens-runtimes/{runtime_id}` | `get_lens_runtime` | One Lens runtime |
| POST | `/api/statehub/lens-runtimes/{runtime_id}/{lens}/reset` | `reset_lens_runtime` | Reset a Lens runtime |
| POST | `/api/statehub/lens-runtimes/{runtime_id}/{lens}/enable` | `enable_lens_runtime` | Enable a Lens |
| POST | `/api/statehub/lens-runtimes/{runtime_id}/{lens}/disable` | `disable_lens_runtime` | Disable a Lens |
| GET | `/api/statehub/{projection_id}` | `get_statehub_projection` | Current materialized Lens projection |
| GET | `/api/statehub/{projection_id}/history` | `get_statehub_projection_history` | Bounded retained versions |
| POST | `/api/surface-events` | `handle_surface_event` | Accept SurfaceEvent commands (in `routes/run.rs`) |

---

## 2. Surface Route Behavior

Each named surface route follows the same pattern:

```rust
async fn get_workbench_surface(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ProjectionEnvelope<WorkbenchProjection>>, ApiError> {
    let projections = RuntimeProjectionSet::load(&state).await?;
    let data = projections.workbench_surface();
    Ok(Json(projections.envelope("workbench", data)))
}
```

1. Load the `RuntimeProjectionSet` from live StateHub + durable feedback
2. Construct the typed surface projection from the loaded state
3. Wrap in a `ProjectionEnvelope<T>` with version, cursor, and freshness metadata
4. Return as typed JSON

The five surface routes do **not** accept query parameters. The generic
`GET /api/projections/{name}` route does accept `ProjectionQuery` parameters
for filtering by plan_id, task_id, agent_id, role, status, etc.

---

## 3. Projection Catalog

`GET /api/projections/catalog` returns all known projections with their schema
versions and invalidation policies:

```json
{
  "projections": [
    {
      "name": "workbench",
      "version": 1,
      "policy": {
        "max_age_secs": 5,
        "incremental": false,
        "invalidation_triggers": [
          "plan_started", "task_started", "task_completed",
          "agent_spawned", "agent_completed", "gate_result",
          "projection_updated"
        ]
      }
    },
    ...
  ]
}
```

There are 30 registered projections in total. The five surface projections
(`workbench`, `inbox`, `canvas`, `minimap`, `autonomy`) are non-incremental
with 5-second max age. Infrastructure projections like `dashboard` and
`execution_trace` are incremental with wildcard triggers.

### Alias Resolution

The `projection_version()` function resolves aliases before lookup:

| Input | Canonical Name |
|---|---|
| `"dashboard_snapshot"` | `"dashboard"` |
| `"agents"` or `"agent_trails"` | `"agent_state"` |
| `"plans"` or `"plans_list"` | `"plan_state"` |
| `"gates"` | `"gate_state"` |
| `"learning"` or `"learning_policy"` | `"learning_policy_state"` |
| `"events"` | `"event_log"` |
| `"providers"` or `"provider_outcomes"` | `"provider_state"` |
| `"retries"` | `"retry_state"` |
| `"costs"` | `"cost_state"` |
| `"trace"` or `"proof"` | `"execution_trace"` |
| `"feedback"` | `"runtime_feedback"` |
| `"jobs"` | `"marketplace_jobs"` |
| `"knowledge_entries"` | `"knowledge"` |
| `"watchers"`, `"circuit_breakers"`, or `"observations"` | `"telemetry"` |

---

## 4. SSE Streaming Protocol

### Initial State

The stream opens with a `state` event containing the full projection snapshot:

```
event: state
id: 42
data: {"name":"workbench","canonical_name":"workbench","version":1,"channel":"projection:workbench","cursor":"0x2a","computed_at":"2026-09-15T12:00:00Z","recovered":false,"freshness":{"state":"live","cursor":"0x2a"},"state":{...},"data":{...}}
```

### Delta Events

Subsequent events carry individual DashboardEvent deltas:

```
event: delta
id: 43
data: {"type":"delta","channel":"projection:workbench","cursor":"0x2b","delta":{"type":"task_completed","plan_id":"p1","task_id":"t1","outcome":"success"}}
```

### Lag Recovery

If the subscriber falls behind the ring buffer:

1. A `RecvError::Lagged(skipped)` is received
2. A warning is logged with the projection name and skip count
3. A fresh `RuntimeProjectionSet` is loaded
4. A replacement `state` event is emitted with the new cursor
5. The subscriber re-subscribes from the new cursor
6. Normal `delta` streaming resumes

### Event Filtering

`projection_accepts_event()` determines which DashboardEvent variants reach
each surface stream. The filtering uses both direct event-type matching and
ProjectionUpdated routing:

**Direct matching** (event type -> surface):
- `workbench`: PlanStarted, PlanCompleted, PhaseTransition, TaskStarted,
  TaskCompleted, TaskPhaseChanged, AgentSpawned, AgentCompleted,
  EfficiencyEvent, GateResult
- `inbox`: InboxItemReceived, InboxApprove, InboxReject, InboxDefer,
  InboxDismiss, AgentSpawned, AgentCompleted, EfficiencyEvent
- `canvas`: PlanStarted, PlanCompleted, TaskStarted, TaskCompleted,
  TaskPhaseChanged, GateResult
- `minimap`: AgentSpawned, AgentCompleted, EfficiencyEvent, CFactorTrendUpdated
- `autonomy`: AgentSpawned, AgentCompleted, EfficiencyEvent, CFactorTrendUpdated

**ProjectionUpdated routing** (when a Lens projection updates):
- `workbench` accepts updates to: active_tasks, agent_vitality, gate_pipeline, cost_meter
- `inbox` accepts updates to: agent_vitality, cohort_health
- `canvas` accepts updates to: active_tasks, gate_pipeline
- `minimap` accepts updates to: agent_vitality, c_factor, cohort_health, knowledge_health
- `autonomy` accepts updates to: agent_vitality, c_factor

---

## 5. StateHub History Endpoint

`GET /api/statehub/{projection_id}/history` provides time-series access to
retained projection versions.

### Query Parameters

| Parameter | Type | Description |
|---|---|---|
| `from` | RFC 3339 string | Start of time range (inclusive) |
| `to` | RFC 3339 string | End of time range (inclusive) |
| `from_version` | u64 | Start of version range (inclusive) |
| `to_version` | u64 | End of version range (inclusive) |
| `resolution` | string | Time bucket size: `{amount}{unit}` where unit is ms/s/m/h/d |
| `limit` | usize | Maximum entries to return (default 250, max 10,000) |

### Resolution Coalescing

When `resolution` is specified, versions are bucketed by time and only the
newest version per bucket is retained. This reduces response size for
high-frequency projections over long time ranges.

Supported units:
- `ms` -- milliseconds (multiplier: 1)
- `s` -- seconds (multiplier: 1,000)
- `m` -- minutes (multiplier: 60,000)
- `h` -- hours (multiplier: 3,600,000)
- `d` -- days (multiplier: 86,400,000)

### Response Shape

```json
{
  "projection_id": "active_tasks",
  "retained": 1200,
  "matched": 800,
  "coalesced": 48,
  "resolution": "1h",
  "capacity": 5000,
  "retention_seconds": 604800,
  "retention_ms": 604800000,
  "history": [...]
}
```

---

## 6. OpenAPI Status

The surface routes produce typed JSON responses, but the OpenAPI specification is
not yet generated from the Rust types. Response bodies appear as generic JSON objects
in any schema discovery tool. This is an explicit product residual.

What exists:
- Typed Rust structs with `#[derive(Serialize, Deserialize)]`
- Consistent response envelope (`ProjectionEnvelope<T>`)
- Catalog endpoint documenting all projection names and versions

What remains:
- Generated OpenAPI 3.x schema from the projection types (via `utoipa` or similar)
- Per-surface response schema references in the catalog
- Typed error responses in the OpenAPI spec

---

## 7. Error Handling

Surface routes return `ApiError` on failure. Common error cases:

| Condition | HTTP Status | Error |
|---|---|---|
| StateHub not bootstrapped | 500 | Internal error during `RuntimeProjectionSet::load()` |
| Unknown projection name | 404 | `"projection '{name}' not found"` |
| Invalid history parameters | 400 | Descriptive validation error |
| History `from` after `to` | 400 | `"history 'from' timestamp must not be after 'to'"` |
| Invalid resolution unit | 400 | `"history 'resolution' has an unsupported unit: use ms, s, m, h, or d"` |
| Zero resolution | 400 | `"history 'resolution' must be greater than zero"` |
