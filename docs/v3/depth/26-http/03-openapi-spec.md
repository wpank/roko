# 26.03 -- OpenAPI Specification

> Depth file for [26-HTTP-API.md](../../26-HTTP-API.md).

---

## Serving the Spec

The OpenAPI document is served at:

```
GET /api/openapi.json
```

The route is registered in `crates/roko-serve/src/openapi.rs` and merged into
the main API router before all domain route groups.

## Generation Approach

The spec is generated at compile time using the `utoipa` crate's derive macros.
A single `ApiDoc` struct carries the `#[derive(OpenApi)]` attribute with all
paths and schemas declared inline:

```rust
#[derive(OpenApi)]
#[openapi(
    info(
        title = "roko-serve API",
        version = env!("CARGO_PKG_VERSION"),
        description = "HTTP API exposed by roko-serve."
    ),
    servers((url = "/api")),
    tags(...),
    paths(...),
    components(schemas(...))
)]
struct ApiDoc;
```

The version field is populated from `CARGO_PKG_VERSION` at compile time, so the
OpenAPI document always reflects the built binary version.

## Tags

The spec is organized into 30 tags that mirror the route domain modules:

| Tag | Description |
|---|---|
| `status` | Health, metrics, and dashboard endpoints |
| `plans` | Plan CRUD and execution |
| `run` | Single prompt execution endpoints |
| `run-observability` | Bounded read-only run evidence and event indexes |
| `templates` | Template CRUD and deploy endpoints |
| `deployments` | Cloud deployment endpoints |
| `agents` | Agent registration and lifecycle endpoints |
| `research` | Research and enhancement endpoints |
| `config` | Configuration endpoints |
| `subscriptions` | Subscription endpoints |
| `webhooks` | Webhook ingress endpoints |
| `providers` | Provider and routing endpoints |
| `learning` | Learning and cascade endpoints |
| `aggregator` | Aggregation and knowledge endpoints |
| `diagnosis` | Diagnosis endpoints |
| `extensions` | Loaded extension metadata and health |
| `surfaces` | Typed StateHub-backed surface projections |
| `arenas` | Arena lifecycle, attempt execution, and settlement |
| `registries` | Passport and knowledge registry endpoints |
| `meta` | Meta-agent lineage proposal and activation |
| `connectors` | Authenticated connector transport lifecycle |
| `projections` | StateHub-backed projection and telemetry routes |
| `feeds` | Feed descriptor CRUD and runtime status |
| `recipes` | Recipe persistence and pure evaluation |
| `triggers` | Trigger binding CRUD and manual fire |
| `dreams` | Dream consolidation cycle and journal |
| `groups` | Persistent group membership, knowledge, and event APIs |
| `secrets` | Secret management endpoints |
| `jobs` | Marketplace job lifecycle and matching |
| `neuro` | Neuro knowledge query endpoint |

## Documented Paths

The `paths(...)` block documents the most-used endpoints with `utoipa::path`
macros. Representative documented operations:

### Status and Health

- `health` -- `GET /health`
- `session_status` -- `GET /session`
- `metrics_summary` -- `GET /metrics`
- `dashboard` -- `GET /dashboard`
- `episodes` -- `GET /episodes`
- `signals` -- `GET /signals`
- `operation_status` -- `GET /operation/{id}`

### Plans

- `list_plans` / `get_plan` / `create_plan` / `execute_plan` / `plan_status`
- `generate_plan` -- `POST /plans/generate` (body `{"prompt": "..."}`)

### Run and Observability

- `start_run` / `run_status`
- `run_observability_detail` / `run_observability_events` /
  `run_observability_event_stream`
- `run_observability_tasks` / `run_observability_attempts` /
  `run_observability_gates`
- `run_observability_logs` / `run_observability_metrics`

### Agents and Lifecycle

- Agent lifecycle observation types are registered as OpenAPI schemas:
  `AgentObservationCommit`, `AgentRuntimeObservation`, `ObservedLifecycleState`,
  `ObservedVitalityPhase`, `ObservedAgentMode`, `ObservedAgentRegime`,
  `CompletedAgentTick`

### Relay and Subscriptions

- Relay state types: `ReconciliationRecord`, `RelayStreamBinding`,
  `ServeRelayConnectionStatus`, `SubscriptionRelayStatus`

## Path Documentation Macros

Three helper macros reduce boilerplate for the ~200 documented paths:

### `doc_get!`

For simple GET endpoints with no path parameters:

```rust
doc_get!(health, "/health", "status");
```

Generates standard 200/400/404/500 responses with `body = Value`.

### `doc_get_param!`

For GET endpoints with a single path parameter:

```rust
doc_get_param!(get_plan, "/plans/{id}", "plans", "id");
```

### `doc_post_value!`

For POST endpoints accepting arbitrary JSON:

```rust
doc_post_value!(create_plan, "/plans", "plans");
```

Generates 200/201/202/400/401/404/409/500 responses.

## Schema Components

The `components(schemas(...))` block registers typed request/response schemas.
Key schema types include:

- `ApiErrorResponse` -- Standard error envelope
- `HealthResponse` -- Health check response
- `RunRequest` / `PlanCreateRequest` -- Execution inputs
- `AgentRegisterRequest` -- Agent registration body
- `AgentRuntimeObservation` / `AgentObservationCommit` -- Lifecycle observations
- `AgentSlotObservation` / `ObservedSlotState` -- Slot manager state
- `SubscriptionCreateRequest` / `SubscriptionUpdateRequest`
- `DeploymentCreateRequest` / `DeploymentCreateResponse`
- `ReconciliationRecord` / `SubscriptionRelayStatus` -- Relay state
- `WebhookPayload` -- Generic webhook ingress
- `SearchQueryRequest` -- Knowledge search parameters

## Server Base URL

The spec declares a single server entry:

```json
{
  "servers": [{ "url": "/api" }]
}
```

All documented paths are relative to `/api`. For example, the documented path
`/health` resolves to `GET /api/health` at runtime.

## Validation

The generated OpenAPI spec can be validated with standard tooling:

```bash
curl -s http://localhost:6677/api/openapi.json | python3 -m json.tool > /dev/null
```

The spec follows OpenAPI 3.0 conventions.

## Limitations

Not every canonical route (counted in `tools/http_route_inventory.snapshot.json`) is documented in the OpenAPI spec. The
`utoipa::path` macros cover the most-used endpoints (~200 paths). Routes added
by feature-gated modules (e.g., `chain` under `alloy-backend`) are documented
only when the feature is enabled at compile time.

Response schemas default to `Value` (arbitrary JSON) for most endpoints rather
than fully typed schemas. This is a documentation-level limitation; the runtime
handlers use concrete Rust types for serialization.

## Source

- `crates/roko-serve/src/openapi.rs` -- OpenAPI assembly and route handler stubs
