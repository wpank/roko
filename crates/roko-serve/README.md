# roko-serve

The Roko HTTP control plane. Single port (6677 by default), a broad REST surface,
SSE + WebSocket streams, and webhook ingestion. Designed so a dashboard,
CLI, or external integration can talk to one base URL and reach every agent,
plan, PRD, metric, and learning signal.

## Start it

```bash
roko serve                                     # default 127.0.0.1:6677
roko serve --bind 0.0.0.0 --port 9090
```

Or embed it:

```rust
use roko_serve::{RokoServer, ServerConfig};

let server = RokoServer::new(ServerConfig {
    bind: "0.0.0.0:6677".parse()?,
    data_dir: Some(".roko".into()),
    ..Default::default()
}).await?;
server.serve().await?;
```

## Route surface

### Health + metrics

| Method | Path | What |
|--------|------|------|
| `GET` | `/api/health` | liveness probe |
| `GET` | `/api/status` | full status JSON |
| `GET` | `/api/metrics` | all metrics (Prometheus-style) |
| `GET` | `/api/metrics/summary` | compact dashboard summary |
| `GET` | `/api/metrics/c_factor` | composite capability score |
| `GET` | `/api/metrics/{success_rate|engagement|gate_rate|experiments|feedback_latency|velocity|coverage|model_efficiency}` | specific dimensions |
| `GET` | `/api/dashboard` | pre-rolled dashboard payload |
| `GET` | `/api/gates/summary`, `/api/gates/history`, `/api/gates/{name}/history` | gate stats |
| `GET` | `/api/episodes`, `/api/signals`, `/api/operations/{id}` | event-log views |

### Plans

| Method | Path | What |
|--------|------|------|
| `GET` | `/api/plans` | list |
| `POST` | `/api/plans` | create |
| `GET` | `/api/plans/{id}` | detail |
| `POST` | `/api/plans/{id}/execute` | start execution |
| `GET` | `/api/plans/{id}/status` | progress |
| `POST` | `/api/plans/generate` | agent-driven plan generation from a PRD |

### PRDs

| Method | Path | What |
|--------|------|------|
| `GET` | `/api/prds` | list |
| `POST` | `/api/prds/ideas` | capture an idea |
| `GET` | `/api/prds/status` | coverage report |
| `GET` | `/api/prds/{slug}` | detail |
| `POST` | `/api/prds/{slug}/draft` | create/update draft |
| `POST` | `/api/prds/{slug}/promote` | promote draft → published |
| `POST` | `/api/prds/{slug}/plan` | generate plan from PRD |

### Run

| Method | Path | What |
|--------|------|------|
| `POST` | `/api/run` | execute a single prompt through the universal loop |
| `GET` | `/api/run/{id}/status` | progress |

### Research

| Method | Path | What |
|--------|------|------|
| `GET` | `/api/research` | list past research runs |
| `POST` | `/api/research/topic` | deep-research a topic |
| `POST` | `/api/research/enhance-prd/{slug}` | enhance a PRD with research |
| `POST` | `/api/research/enhance-plan/{plan}` | enhance a plan |
| `POST` | `/api/research/enhance-tasks/{plan}` | split / optimize tasks |
| `POST` | `/api/research/analyze` | analyze execution data |

### Agents — registration + management

| Method | Path | What |
|--------|------|------|
| `GET` | `/api/managed-agents` | list roko-managed agents |
| `POST` | `/api/agents/register` | register an external agent |
| `POST` | `/api/agents/{id}/stop` | graceful shutdown |
| `POST` | `/api/agents/{id}/message` | proxied to sidecar `POST /message` |
| `POST` | `/api/agents/{id}/observation` | durably commit a canonical lifecycle sample, then emit Agent-scoped Lens events |
| `GET` | `/api/agents/{id}` | detail |
| `GET` | `/api/agents/{id}/episodes` | per-agent episode log |
| `GET/POST` | `/api/agents/{id}/token` | agent auth token |

Agent observations use a per-Agent monotonic `sequence`. Exact retries remain idempotent
across server restarts; stale or conflicting sequences fail before emission. The baseline
is persisted before best-effort Lens fanout. Modern Agent tokens require `bus_publish`, and
all Agent credentials are bound to the `{id}` in the request path.

### Agents — aggregator (discovery + fan-out)

| Method | Path | What |
|--------|------|------|
| `GET` | `/api/agents` (supports `?owner=`) | discovery union across all sidecars |
| `GET` | `/api/agents/topology` | graph of agents + their relationships |
| `GET` | `/api/agents/{id}/stats`, `/skills`, `/heartbeat`, `/trace` | sidecar read-through |
| `GET` | `/api/predictions/sessions`, `/sessions/{id}`, `/claims`, `/calibration/{agent_id}` | prediction union |
| `GET` | `/api/knowledge/entries`, `/edges`, `/search`, `/kinds` | knowledge graph |
| `GET` | `/api/tasks`, `/tasks/stats`, `/tasks/{id}` | task union |
| `GET` | `/api/ws` | aggregated per-agent event stream |

### Learning (aliases: `/api/learn/*` and `/api/learning/*`)

| Method | Path | What |
|--------|------|------|
| `GET` | `/api/learn/efficiency` | cost/token/latency rollup |
| `GET` | `/api/learn/cascade-router`, `/cascade` | router state |
| `GET` | `/api/learn/cost-tiers` | tier distribution |
| `GET` | `/api/learn/experiments` | A/B variants + win rates |
| `GET` | `/api/learn/adaptive-thresholds`, `/gate-thresholds` | EMA gate thresholds |

### Dashboard projections

Dashboard and product surfaces should prefer StateHub projections over private
runtime internals. Each projection response includes `name`, `version`,
`cursor`, `computed_at`, `recovered`, `freshness`, and a stable `state` object.
Use `GET /api/projections/catalog` for versions and invalidation triggers.

| Method | Path | What |
|--------|------|------|
| `GET` | `/api/projections/agent_state` | agent roster, active state, output/cost counters |
| `GET` | `/api/projections/plan_state` | plan and task execution state |
| `GET` | `/api/projections/gate_state` | gate verdicts, trends, failures, threshold availability |
| `GET` | `/api/projections/learning_policy_state` | learning and policy summaries available through StateHub |
| `GET` | `/api/projections/{name}/stream` | SSE initial state plus projection deltas |

Missing projection data is represented as `{"state":"missing", ...}` or
`{"state":"empty", ...}` with a reason and, when applicable, the stable
endpoint that owns the data.

### Subscriptions / templates / deployments / config / providers

| Area | Paths |
|------|-------|
| Subscriptions | `GET/POST /api/subscriptions`, `GET /api/subscriptions/catalog`, `GET /api/subscriptions/relay/status`, `PUT/DELETE /api/subscriptions/{id}`, `POST /api/subscriptions/{id}/{enable|disable}` |
| Templates | `GET/POST /api/templates`, `GET/DELETE /api/templates/{name}`, `POST /api/templates/{name}/deploy` |
| Deployments | `GET/POST /api/deployments`, `GET/DELETE /api/deployments/{id}`, `GET /api/deployments/{id}/logs`, `POST /api/deployments/{id}/task`, `POST /api/deployments/callback` |
| Config | `GET/PUT /api/config`, `POST /api/config/reload` |
| Providers | `GET /api/providers/`, `GET /api/providers/{id}/health`, `POST /api/providers/{id}/test`, `GET /api/models/`, `GET /api/models/routing/explain` |

### Connectors / arenas / meta-agents

| Area | Paths |
|------|-------|
| Supervised HTTP JSON connectors | `GET/POST /api/connectors`, `DELETE /api/connectors/{name}`, `GET /api/connectors/{name}/health`, `POST /api/connectors/{name}/{restart|query|execute}` |
| Durable local arenas | `GET/POST /api/arenas`, `GET/PATCH /api/arenas/{id}`, `GET /api/arenas/{id}/leaderboard`, `GET/POST /api/arenas/{id}/attempts`, `GET /api/arenas/{id}/attempts/{attempt_id}`, `POST /api/arenas/{id}/attempts/{attempt_id}/{submit|settle}` |
| Durable meta-agents | `GET/POST /api/meta/agents`, `GET /api/meta/agents/{id}`, `POST /api/meta/agents/{id}/{validate|morph|deactivate}`, `POST /api/meta/agents/{id}/morph/rollback` |

The connector adapter performs a real bounded transport probe before reporting connected and
uses generation-safe cancellation for replace/restart/delete. Connector status never returns
credentials or untrusted manifest metadata.

With relay configuration, enabled exact-room subscriptions run through the supervised
`roko-agent-server` client. `Subscribe(last_seq)` atomically restores the room set and durable
cursor; a standalone live-socket `Resume` is rejected. Serve persists dispatch intent first,
then atomically commits ordered dispatch-or-suppression receipts and room/subscription/global
cursors to `.roko/state/subscription-relay-journal.json` before returning handler success and
permitting ACK. The journal is integrity-checked and bounded to 4 MiB, 4,096 entries, and
4,096 cursor keys. Failed/interrupted dispatch, unsafe stream changes, and generic snapshots
enter explicit reconciliation instead of replaying side effects blindly. The authenticated
status route exposes those diagnostics, including unsupported wildcard triggers.

Arena mutations are owner/participant/admin authorized as applicable and persisted atomically
in `.roko/chain/arena-state.json`; external scoring evidence, attempt settlement, prize and
reputation effects, leaderboard state, and the durable projection outbox commit together.
Meta-agent proposals are owner-bound and durable. Activation requires single-use R03 evidence
for the exact artifact and the fixed five-head safety order; descendant grants cannot widen,
role morphs narrow with exact rollback, and deactivation is descendant-safe.

### Streaming + webhooks

| Method | Path | What |
|--------|------|------|
| `GET` | `/ws` | top-level WebSocket (system events) |
| `GET` | `/api/ws` | aggregator WebSocket (per-agent union) |
| `GET` | `/api/events` | SSE stream of platform events |
| `POST` | `/webhooks/github`, `/webhooks/slack`, `/webhooks/generic` | webhook ingestion |

## Smoke-test recipe

```bash
# start the server
roko serve &

# health
curl http://localhost:6677/api/health

# discover agents
curl http://localhost:6677/api/agents

# fetch dashboard payload
curl http://localhost:6677/api/dashboard | jq .

# subscribe to SSE
curl -N http://localhost:6677/api/events

# POST a prompt through the universal loop
curl -X POST http://localhost:6677/api/run \
  -H "Content-Type: application/json" \
  -d '{"prompt": "hello", "agent": "nunchi-intelligence"}'
```

## Architecture notes

```
┌───────────────┐
│ Dashboard /   │   single base URL
│ External CLI  │   (:6677 or any bind)
└──────┬────────┘
       │
┌──────▼────────────────────────────────────┐
│   roko-serve (Axum router)                │
│   - /api/* REST                           │
│   - /ws, /api/ws, /api/events streams     │
│   - /webhooks/* ingestion                 │
│   - durable relay/arena/meta state         │
└──────┬────────────────────────────────────┘
       │
       ├──► roko-core (signals, config, episodes)
       ├──► roko-learn (efficiency, c-factor, experiments)
       ├──► roko-gate (gate history, thresholds)
       ├──► roko-cli runner (plan DAG, tasks)
       ├──► roko-runtime (supervised HTTP connectors)
       ├──► roko-chain (durable local arena registry)
       ├──► agent-relay (bounded canonical event transport)
       │
       └──► aggregator ──► roko-agent-server (per agent)
                           /message, /stream, /predictions, ...
```

## Tests

```bash
cargo test -p roko-serve
cargo clippy -p roko-serve --no-deps -- -D warnings
```

Routes added in T18 (`deployments`, `templates`, `mcp-code`) have
dedicated integration tests that exercise happy and error paths.

## What it is not

- **Not an agent**, it only proxies to them. Messaging happens in
  `roko-agent-server`.
- **Not a scheduler**, it only triggers. Execution runs in `roko-cli`
  via the `roko-cli` runner.
- **Not the complete connectivity product**: additional transports, startup discovery,
  MCP auto-registration, A2A/x402, finality/reorg processing, and dashboard auto-connect remain.
- **Not the complete arena/autonomy product**: evals, flywheel learning, on-chain/token/transfer
  systems, Loop 4, ADAS, HGM, and autonomous generated-agent execution remain outside the
  scoped local R03/R04 services.

## Related

- `crates/roko-agent-server/README.md` — per-agent sidecar contract
- Top-level `README.md` — full Deployment section with recipes for
  daemon, worker, and cloud targets
