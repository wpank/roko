# 26.04 -- Per-Agent Sidecar API

> Depth file for [26-HTTP-API.md](../../26-HTTP-API.md).

---

## Overview

Each agent can run its own HTTP sidecar server via `roko agent serve`. The
sidecar is implemented in `crates/roko-agent-server/` and exposes up to 14
routes depending on enabled feature flags.

## Architecture

```
                  +------------------+
                  |  roko-serve      |
                  |  (control plane) |
                  |  :6677           |
                  +--------+---------+
                           |
                  heartbeat POST /api/heartbeats
                           |
              +------------+------------+
              |                         |
    +---------+---------+     +---------+---------+
    | agent-server      |     | agent-server      |
    | agent-alpha       |     | agent-beta        |
    | :7001             |     | :7002             |
    +-------------------+     +-------------------+
```

The sidecar binds its own port and communicates back to the control plane via
heartbeat POSTs.

## Server Builder

The `AgentServer` uses a builder pattern (`AgentServerBuilder`) to configure:

```rust
AgentServer::builder()
    .agent_id("agent-alpha")
    .bind("127.0.0.1:7001")
    .auth(BearerAuth::new("secret-token"))
    .llm_backend(backend)
    .knowledge_store(store)
    .enable_messaging()
    .enable_predictions()
    .enable_research()
    .enable_tasks()
    .build()
    .serve()
    .await?;
```

## Feature Modules

Feature flags control which route groups are mounted:

| Feature | Module | Routes Added |
|---|---|---|
| (always) | `health` | `/health`, `/capabilities`, `/stats` |
| (always) | `logs` | `/logs` |
| `messaging` | `messaging` | `/message`, `/stream` |
| `predictions` | `predictions` | `/predictions`, `/predictions/residuals`, `/predictions/{id}` |
| `research` | `research` | `/research` |
| `tasks` | `tasks` | `/tasks`, `/tasks/{id}/accept`, `/tasks/{id}/complete` |

## Route Reference

### Public Routes (No Auth)

#### `GET /health`

Returns agent liveness status:

```json
{
  "status": "ok",
  "agent_id": "agent-alpha",
  "uptime_s": 3600
}
```

#### `GET /capabilities`

Returns the agent's full capability manifest as JSON.

### Protected Routes (Bearer Auth)

All protected routes require `Authorization: Bearer <token>` when auth is
configured.

#### `GET /stats`

Returns runtime statistics including message counts, latencies, and resource
usage.

#### `GET /logs`

Returns recent agent log entries.

#### `POST /message`

Send a prompt to the agent's LLM backend and receive a synchronous response.

Request:

```json
{
  "prompt": "Explain the Signal type",
  "context": {}
}
```

Response:

```json
{
  "response": "The Signal type is...",
  "reasoning": null,
  "usage": { "input_tokens": 50, "output_tokens": 200 },
  "session": { "state": "active" },
  "finish_reason": "end_turn",
  "signal_id": "signal-<uuid>"
}
```

#### `GET /stream` (WebSocket)

Upgrades to a WebSocket connection for streaming agent interaction. The client
sends text messages containing prompts; the server streams back response
fragments as `StreamEvent` frames.

#### `GET /predictions`

List all stored predictions for this agent.

#### `POST /predictions`

Create a new prediction with idempotency support.

#### `GET /predictions/{id}`

Retrieve a specific prediction by ID.

#### `GET /predictions/residuals`

Return residual analysis for past predictions.

#### `POST /research`

Submit a research query. Supports `mode=local_knowledge` for knowledge-store
queries. Active web research returns 501 if not configured.

#### `GET /tasks`

List the agent's task queue.

#### `POST /tasks`

Create a task with idempotency key support. Returns 201 for new tasks,
200 for duplicate keys, 409 for key conflicts with different bodies.

#### `POST /tasks/{id}/accept`

Accept a task (mark as in-progress).

#### `POST /tasks/{id}/complete`

Complete a task with a result payload.

## Authentication

The sidecar uses a simple bearer token scheme:

```rust
pub struct BearerAuth {
    token: String,
}
```

The `require_bearer_auth` middleware validates the `Authorization: Bearer`
header. Health and capabilities routes are public; all other routes are
protected.

## State Persistence

The sidecar stores durable state in a JSON file envelope:

```json
{
  "schema_version": 3,
  "updated_at": "2026-09-01T00:00:00Z",
  "predictions": [...],
  "tasks": [...],
  "events": [...]
}
```

State is restored on startup via `restore_state()` and flushed after mutations.
The `FileStateStore` handles atomic writes via rename.

## Heartbeat Loop

When `serve_url` is configured, the sidecar spawns a background heartbeat loop:

```rust
async fn heartbeat_loop(state, url, interval_secs) {
    let mut tick = tokio::time::interval(Duration::from_secs(interval_secs));
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tick.tick().await;
        // POST heartbeat snapshot to control plane
        let snapshot = state.heartbeat_snapshot().await;
        let _ = client.post(&url).json(&snapshot).send().await;
    }
}
```

The heartbeat carries agent ID, uptime, message counts, and resource stats.

## Agent Card and Registration

On startup, the sidecar can self-register with the control plane by publishing
an `AgentCard`:

```rust
pub struct AgentCard {
    pub agent_id: String,
    pub owner: Option<String>,
    pub version: Option<String>,
    pub capabilities: Vec<String>,
    pub endpoints: AgentCardEndpoints,
}
```

Registration uses `POST /api/agents` on the control plane.

## Source

- `crates/roko-agent-server/src/lib.rs` -- Server builder and router assembly
- `crates/roko-agent-server/src/state.rs` -- Shared state and persistence
- `crates/roko-agent-server/src/features/` -- Route modules
- `crates/roko-agent-server/src/auth/` -- Bearer authentication
- `crates/roko-agent-server/src/registration.rs` -- Agent card and registration
