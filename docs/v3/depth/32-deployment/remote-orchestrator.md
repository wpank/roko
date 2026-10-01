# 32-deployment/13 -- Remote Orchestrator

> roko-serve as a deployed HTTP service: REST API, authentication,
> multi-project management, webhook integration, cost tracking, and the
> local-to-remote transition.

**Parent:** [32-DEPLOYMENT](../../32-DEPLOYMENT.md)

**Source:** `crates/roko-serve/src/` on :6677 (route counts in `tools/http_route_inventory.snapshot.json`)

---

## 1. Overview

The remote orchestrator transforms Roko from a local CLI tool into a
deployed service. The same orchestration engine (Graph executor, agent
dispatch, gate pipeline, Signal persistence, Bus-backed live progress)
runs behind an HTTP API instead of a terminal interface.

Use cases:
- **Team usage**: One instance, API keys per member, shared progress
- **CI integration**: Trigger plan runs from GitHub Actions
- **Autonomous operation**: Process webhooks and subscriptions
- **Remote access**: Interact from any device with a browser

---

## 2. Starting the Server

### roko-serve (dedicated binary)

```bash
roko-serve --port 6677 --bind 0.0.0.0
roko-serve --port 6677 --data-dir /data --config ~/.config/roko/config.toml
```

`roko-serve` is built from `crates/roko-serve/`. It exposes the full API
without the TUI. Preferred for server deployments.

### roko serve (CLI server mode)

```bash
roko serve                    # Default: 127.0.0.1:6677
roko serve --bind 0.0.0.0    # Public binding
```

The CLI can expose the HTTP API alongside the TUI. Useful for development
-- TUI locally, API accessible remotely.

---

## 3. HTTP API Surface

roko-serve exposes REST routes (counts in `tools/http_route_inventory.snapshot.json`)
plus SSE and WebSocket endpoints on a single port (6677 by default).

### Core REST Endpoints

```
GET    /readyz                         Health/readiness check
GET    /v1/status                      Server status

GET    /v1/plans                       List plans
POST   /v1/plans/:id/run              Start a plan run
GET    /v1/plans/:id/runs             List runs
GET    /v1/plans/:id/runs/:run_id     Run details

GET    /v1/agents                      List agents
POST   /v1/agents                      Create agent
GET    /v1/agents/:name/status        Agent health

GET    /v1/providers                   List providers and health
GET    /v1/models                      Available models
GET    /v1/routing/stats              Routing statistics

GET    /v1/knowledge/query            Query knowledge store
GET    /v1/costs                      Aggregate costs
```

### Real-Time Streaming

```
GET    /projections/:name              Query current projection state
GET    /projections/:name/stream       SSE stream for one projection
WS     /ws/*                           Bidirectional WebSocket binding
```

Subscribers receive frames with cursors; reconnecting clients resume from
their last cursor when retained history still exists.

---

## 4. Authentication

### API Keys

```bash
roko auth create-key --scope admin --label "my-laptop"
# -> roko_sk_a1b2c3d4...
```

Three scopes:

| Scope | Capabilities |
|-------|-------------|
| `read` | View plans, runs, agents, subscribe to streams |
| `write` | Create plans, start runs, create agents |
| `admin` | Manage API keys, server config, delete projects |

Keys are stored in the server's state directory, rotatable and revocable.

### Authentication Header

```bash
curl -H "Authorization: Bearer roko_sk_..." \
  https://roko-serve.fly.dev/v1/status

curl -H "x-api-key: roko_sk_..." \
  https://roko-serve.fly.dev/v1/plans
```

### Rate Limiting

| Scope | Limit |
|-------|-------|
| `read` | 1000 req/min |
| `write` | 100 req/min |
| `admin` | 50 req/min |

Token bucket algorithm. `x-ratelimit-remaining` and `x-ratelimit-reset`
headers included in responses.

---

## 5. Multi-Project Management

One roko-serve instance can manage multiple projects. Each project has:

- Its own `.roko/` state directory
- PRDs, plans, and context artifacts
- Provider configuration (inherits server defaults, overridable)
- Run history, signal and episode logs

Projects are isolated. A run in project A does not affect project B.

---

## 6. Webhook Integration

The server receives webhooks from GitHub:

```toml
[webhooks.github]
events = ["push", "pull_request.opened", "issue_comment.created"]
secret = "${GITHUB_WEBHOOK_SECRET}"
```

### GitHub Webhook Flow

1. **Push**: Pull latest changes, trigger plan run
2. **PR opened**: Auto-run plans against the PR branch, post results as
   PR comments
3. **Issue comment `/roko run 03-05`**: Parse command, trigger specified
   plans, stream results as issue comments

All webhooks verify HMAC-SHA256 signature before processing.

---

## 7. The Local-to-Remote Transition

```
1. Local:      roko init -> write PRDs -> roko plan run
               (same tool, same config, same pipeline)

2. Deploy:     roko-serve on Fly.io / Railway
               (same engine, HTTP interface)

3. Remote:     curl POST .../plans/:id/run
               (same Graph executor, same gates)

4. Watch:      GET /projections/active_tasks/stream
               (same progress state as TUI)
```

Critical property: **same tool, same config, same artifacts**. The only
difference is where the binary runs. A project's `.roko/` directory,
plan files, and config work identically whether executed by the local CLI
or the remote server.

---

## 8. Cost Tracking

```
GET /v1/costs                       Aggregate costs
GET /v1/plans/:id/runs/:id/costs   Per-run cost breakdown
```

The cost tracker records model, provider, input/output tokens, cost per
request, and cumulative cost per task/plan/run.

### Budget Limits

```toml
[budgets]
max_per_run_usd = 5.00
max_per_project_daily_usd = 50.00
max_daily_usd = 200.00
```

When a budget limit is reached, the server pauses the run and notifies
subscribers on the real-time surface. The run resumes after budget
increase or daily reset.

---

## 9. Cloud Execution Config

```rust
pub struct CloudExecutionConfig {
    pub bind: String,
    pub port: u16,                   // Default: 6677
    pub data_dir: PathBuf,
    pub max_concurrent_runs: usize,
    pub max_total_agents: usize,
    pub auth_db: PathBuf,
    pub webhooks_enabled: bool,
    pub webhook_port: Option<u16>,
    pub tls: Option<TlsConfig>,
}
```

---

## 10. Login and Credentials

The CLI authenticates with remote instances:

```bash
roko login https://roko-serve.example.com    # Browser or API key
roko logout
roko whoami
```

Credentials are stored locally and sent as bearer tokens on subsequent
API calls.

---

## 11. Implementation Status

> **Implementation status:** roko-serve is wired on port 6677 (route counts
> in `tools/http_route_inventory.snapshot.json`). The `roko serve` CLI command starts the HTTP
> control plane. Authentication middleware, rate limiting, and SSE/WS
> streaming are implemented. The full REST API surface is operational.
> Multi-project management, webhook integration, and budget enforcement
> are wired. `roko login/logout/whoami` handle remote credentials.
