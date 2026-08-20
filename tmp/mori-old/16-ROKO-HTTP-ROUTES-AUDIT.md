# Roko HTTP Control Plane Audit

Audited: 2026-08-19
Source: `crates/roko-serve/src/`

## Executive Summary

The `roko serve` HTTP control plane on port 6677 exposes **~350 unique API endpoints** across
49 route modules (the CLAUDE.md claim of "~317" is conservative). Adding terminal WebSocket
sessions, top-level probes, and the Prometheus scrape endpoint brings the actual addressable
surface to **~365 endpoints**. The majority serve real data from disk, in-memory state, or
connected runtime services. Only two route modules -- DeFi (8 routes) and Marketplace (5
routes) -- are explicit contract stubs returning 501/empty shapes. The SSE and WebSocket
endpoints stream genuine live events. Authentication and rate limiting are production-grade.

A dashboard/frontend can consume the vast majority of these routes today with real data,
assuming the workspace has `.roko/` state from plan execution.

---

## 1. Route Count

| Scope | Count |
|---|---|
| Routes inside `routes/` directory (unique path+method, excluding test modules) | 350 |
| Top-level routes outside `routes/` (`/health`, `/ready`, `/metrics`) | 3 |
| Terminal routes (`terminal.rs`) | 5 enabled + 3 disabled stubs |
| OpenAPI (`/api/openapi.json`) | 1 |
| Relay catch-all (`/relay/{*path}`, `/relay`) | 2 |
| Trigger catch-all (`/{*path}` public webhook ingress) | 1 |
| **Total addressable endpoints** | **~365** |

### Duplicate/alias routes

The learning module intentionally registers every endpoint under both `/learn/` and
`/learning/` prefixes (13 handler pairs = 26 routes). Some SSE aliases exist too
(`/api/events` and `/api/sse` both serve the same SSE handler). These inflate the raw count
but serve backward compatibility.

---

## 2. Routes by Category

### 2a. Core Workflow (Real Data)

| Category | Routes | Data Source | Verdict |
|---|---|---|---|
| **Status / Health** (`status/`) | 31 | `.roko/` JSONL files, StateHub snapshot, MetricRegistry, process supervisor, provider health tracker | **REAL** -- serves live uptime, plan counts, agent counts, gate verdicts, provider health, JWKS state, disk usage, parity matrix, retention violations |
| **Plans** (`plans.rs`) | 16 | `.roko/plans/` TOML/JSON files on disk, active plan handles in memory, CostProjector | **REAL** -- CRUD, execute, pause/resume, gate results, cost projection, reviews, diffs, chat, plan generation |
| **PRDs** (`prds.rs`) | 9 | `.roko/prd/` directory on disk, CliRuntime for agent-driven operations | **REAL** -- list/get/create ideas, draft/promote/plan/consolidate |
| **Run** (`run.rs`) | 2 | CliRuntime `run_once()`, in-memory RunHandle map | **REAL** -- spawns background WorkflowEngine runs, status polling |
| **Research** (`research.rs`) | 6 | `.roko/research/` directory, CliRuntime for agent-driven research | **REAL** -- list artifacts, topic research, enhance PRD/plan/tasks, analyze |
| **Episodes / Signals** (`status/episodes.rs`) | 4 | `.roko/engrams.jsonl`, `RuntimeProjectionSet`, signal store | **REAL** -- read/promote/prune signals, normalized episode rows |
| **Gates** (`status/gates.rs`) | 3 | `RuntimeProjectionSet` from `.roko/` JSONL files | **REAL** -- gate summary, history, per-gate time series, waterfall format |
| **Dashboard** (`status/dashboard.rs`) | 4 | CliRuntime, process supervisor, process session ledger, truth map | **REAL** -- dashboard scaffold, session status, operation status, entity truth registry |

### 2b. Agents (Real Data)

| Category | Routes | Data Source | Verdict |
|---|---|---|---|
| **Agent management** (`agents.rs`) | 16 | ProcessSupervisor, discovery registry (in-memory), heartbeat map, config, agent lifecycle runtime | **REAL** -- list/create/register/start/stop/restart agents, send messages, proxy logs, token management, lifecycle observations |
| **Aggregator** (`aggregator.rs`) | 21 | Proxies to per-agent sidecar HTTP endpoints, knowledge store, in-memory task tracking | **REAL** -- agent topology, stats, skills, heartbeat, trace, prediction sessions/claims, knowledge entries/edges/search, tasks/stats, WebSocket upgrade |
| **Groups** (`groups.rs`) | 7 | Durable group registry (`.roko/groups/`) | **REAL** -- CRUD groups, invite/accept/remove members, permissions, message publication, events |

### 2c. Learning & Telemetry (Real Data)

| Category | Routes | Data Source | Verdict |
|---|---|---|---|
| **Learning** (`learning/`) | 26 (13 pairs) | `.roko/learn/` directory: `efficiency.jsonl`, `cascade-router.json`, `gate-thresholds.json`, `experiments.json`, `c-factor.jsonl`; `RuntimeProjectionSet`; provider health registry | **REAL** -- efficiency aggregation, cascade router snapshot with availability, cost tiers, experiments, adaptive gate thresholds, provider outcomes, retries, runtime feedback, executor state, c-factor trend |
| **Metrics** (status + top-level) | 14 | MetricRegistry (Prometheus), efficiency JSONL, experiment store, cascade router, template run records | **REAL** -- Prometheus exposition (`/metrics`), success rate, engagement, c-factor, model efficiency, gate rate, experiments, feedback latency, velocity, coverage, summary |
| **Projections** (`projections.rs`) | 12 | StateHub snapshot, Lens runtimes, dedicated projection contracts (Workbench, Inbox, Canvas, Minimap, Autonomy), telemetry lens data | **REAL** -- five named surface projections, catalog, SSE streaming per projection, telemetry stream, lens runtime status |

### 2d. Infrastructure (Real Data)

| Category | Routes | Data Source | Verdict |
|---|---|---|---|
| **SSE** (`sse.rs`) | 2 | StateHub event ring buffer + broadcast channel | **REAL** -- DashboardEvent streaming with replay, reconnection via Last-Event-ID |
| **WebSocket** (`ws.rs`) | 3 | EventBus ring buffer + broadcast channel | **REAL** -- ServerEvent streaming, topic/projection filtering, cursor resume |
| **Workflow SSE** (in `mod.rs`) | 1 | SseAdapter (RuntimeEvent broadcast) | **REAL** -- WorkflowEngine event streaming |
| **Config** (`config.rs`) | 4 | `roko.toml` on disk, ArcSwap live config | **REAL** -- read/write config, TOML view, hot reload |
| **Auth** (`auth.rs`) | 8 | In-memory API key store, agent token store, relay token store, audit log | **REAL** -- API key CRUD/rotate, agent token CRUD, relay token management, audit trail |
| **Secrets** (`secrets.rs`) | 4 | Namespaced secret store on disk | **REAL** -- CRUD secrets with namespace isolation, test endpoint |
| **Providers** (`providers.rs`) | 6 | Config `effective_providers()`/`effective_models()`, provider health tracker, CascadeRouter, ModelCallService | **REAL** -- list providers with health, list models with capabilities, health check, test with real LLM call, routing explanation |

### 2e. Connected Services (Real -- Requires External Config)

| Category | Routes | Data Source | Verdict |
|---|---|---|---|
| **Gateway** (`gateway.rs`) | 10 | CascadeRouter, ModelCallService, provider health registry, gateway cost counters | **REAL** -- inference completion (dispatches to real LLM), stats, batch submit/flush/result, model list, rate limits |
| **Chain** (`chain.rs`) | 7 | Alloy RPC client (requires `[chain]` config), in-memory ring buffers for blocks/txs/events | **REAL when configured** -- reads on-chain agent registry/bounty market, block/tx/event buffers, watcher status. Returns structured error when chain not configured |
| **Relay proxy** (`relay_proxy.rs`) | 4 | Proxies to agent-relay service (`ROKO_AGENT_RELAY_URL`) | **REAL when configured** -- HTTP reverse proxy + WebSocket bridge. Returns 503 when relay not set |
| **RPC proxy** (`rpc_proxy.rs`) | 5 | Proxies to mirage-rs JSON-RPC (`ROKO_MIRAGE_URL`) | **REAL when configured** -- JSON-RPC POST, WebSocket subscriptions, health passthrough, REST API catch-all. Returns 503 when mirage not set |
| **Connectors** (`connectors.rs`) | 7 | Connector supervisor, HTTP JSON transport adapters | **REAL** -- register/list/delete connectors, health check, restart, query/execute against connected transports |
| **Feeds** (`feeds.rs`) | 12 | Feed descriptor registry (in-memory), runtime feed status, feed catalog, x402 payment gating | **REAL** -- CRUD feed descriptors, catalog built-in feed agents, runtime status, discover/search, health, start/stop, paid feed 402 enforcement |
| **Triggers** (`triggers.rs`) | 6 + catch-all | Trigger binding store, trigger history (durable), trigger runtime | **REAL** -- CRUD trigger bindings, manual fire, history, dynamic webhook catch-all ingress |
| **Webhooks** (`webhooks.rs`) | 3 | HMAC-SHA256 verification, Signal persistence, EventBus | **REAL** -- GitHub (deployment status signals), Slack, generic webhook ingress |

### 2f. Knowledge & Dreams (Real Data)

| Category | Routes | Data Source | Verdict |
|---|---|---|---|
| **Neuro** (`neuro.rs`) | 2 | `KnowledgeStore::for_layout()` (durable neuro store) | **REAL** -- POST query and GET alias both search the knowledge store, emit Lens observations |
| **Dream** (`dream.rs`) | 2 | Dream consolidation cycle (agent-driven), dream journal (durable) | **REAL** -- trigger dream run, read dream journal |

### 2g. Registries & Arenas (Real Data, Durable State Machines)

| Category | Routes | Data Source | Verdict |
|---|---|---|---|
| **Registries** (`registries.rs`) | 7 | `AgentRegistry`, `KnowledgeRegistry` (durable local JSON state), optional `EventIndexer` for on-chain events | **REAL** -- passport CRUD with delegation, knowledge entries with challenge, stats, event index, indexer rebuild/sync |
| **Arenas** (`arenas.rs`) | 6 | `ArenaRegistry` (durable JSON state with Pulse Bus event outbox) | **REAL** -- CRUD arenas, attempt submission, settlement, leaderboard |
| **Meta-agents** (`meta.rs`) | 7 | Durable meta-agent lineage store with authority/evidence constraints | **REAL** -- proposal/activation/morph/rollback/deactivation with bounded lineage |

### 2h. Explicit Stubs (Not Functional)

| Category | Routes | Response | Verdict |
|---|---|---|---|
| **DeFi** (`defi.rs`) | 8 | All return 501 `{ "status": "not_implemented", "message": "DeFi product endpoints are Phase 2" }` | **STUB** -- instruments, bonds, options, insurance, indices, risk/portfolio |
| **Marketplace** (`marketplace.rs`) | 5 | Reads return `{ "artifacts": [], "stub": true }`, writes return `{ "status": "not_implemented" }` | **STUB** -- browse/search return empty, publish/fork acknowledge but do nothing |

### 2i. Other (Real Data)

| Category | Routes | Data Source | Verdict |
|---|---|---|---|
| **Bench** (`bench.rs`) | 20 | Benchmark suite/run management, cost summaries, Pareto frontier, model comparison | **REAL** -- run benchmarks, compare models, export results |
| **SWE-Bench** (`swe_bench.rs`) | 4 | SWE-bench dataset integration | **REAL** -- run SWE-bench evaluations, list datasets/runs |
| **Jobs** (`jobs.rs`) | 10 | In-memory job store, matchmaking | **REAL** -- CRUD jobs, match/assign/start/execute/submit/evaluate/cancel |
| **Team** (`team.rs`) | 5 | In-memory team/invitation store | **REAL** -- me/members/invites/invite/join |
| **Shared Runs** (`shared_runs.rs`) | 4 | Transcript store (durable), share token generation | **REAL** -- share a run, read by ID or share token |
| **Templates** (`templates.rs`) | 3 | Template registry, deployment dispatch | **REAL** -- list/create templates, deploy |
| **Deployments** (`deployments.rs`) | 4 | Deployment backends (Railway, manual), worker callbacks | **REAL** -- task dispatch, callback handling, log proxy |
| **Extensions** (`extensions.rs`) | 2 | Extension chain runtime (circuit breaker state) | **REAL** -- list loaded extensions with health, detail view |
| **Heartbeats** (`heartbeats.rs`) | 3 | In-memory heartbeat store, network stats | **REAL** -- heartbeat CRUD, network stats |
| **Integrations** (`integrations.rs`) | 2 | Config-driven integration descriptors | **REAL** -- list/detail configured integrations |
| **Subscriptions** (`subscriptions.rs`) | 3 | Subscription catalog (event sources from config) | **REAL** -- catalog, enable/disable subscriptions |
| **Event Ingest** (`event_ingest.rs`) | 2 | EventBus, Signal persistence | **REAL** -- single and batch event ingestion |
| **Recipes** (`recipes.rs`) | 4 | Durable recipe DAG store | **REAL** -- CRUD recipes |
| **Workflows** (`workflows.rs`) | 7 | Active workflow tracking, SSE/WS streaming | **REAL** -- list/detail workflows, latest with stream, task view, WebSocket |
| **Workspaces** (`workspaces.rs`) | 3 | Workspace registry | **REAL** -- list/detail/create workspaces |
| **Vision Loop** (`vision_loop.rs`) | 3 | Vision-capable agent dispatch | **REAL** -- start vision loop, status, cancel |
| **Diagnosis** (`diagnosis.rs`) | 1 | Diagnosis history from conductor | **REAL** -- recent diagnoses |
| **Runs dashboard** (`runs.rs`) | 1 | Active run handles | **REAL** -- dashboard run list |
| **OpenAPI** (`openapi.rs`) | 1 | utoipa-generated OpenAPI 3.0 JSON | **REAL** -- `/api/openapi.json` |

### 2j. Terminal (Real -- Config-Gated)

| Category | Routes | Data Source | Verdict |
|---|---|---|---|
| **Terminal** (`terminal.rs`) | 5 | `portable-pty` real shell sessions, WebSocket bridge | **REAL when enabled** -- create/list/kill PTY sessions, WebSocket terminal, session resize. Gated by `serve.terminal_enabled` config. Returns 403 when disabled |

---

## 3. Stub Assessment Summary

| Status | Route Count | Percentage |
|---|---|---|
| **Real (serves data)** | ~340 | ~97% |
| **Real but requires external config** (chain, relay, mirage) | ~16 | ~5% |
| **Explicit stubs** (DeFi + Marketplace) | 13 | ~3.5% |

The 13 stub routes are intentionally explicit: they return structured JSON with
`"status": "not_implemented"` and proper HTTP status codes (501 for DeFi, 200 with
`"stub": true` for Marketplace reads). No route silently returns empty data while
pretending to be functional.

---

## 4. SSE Endpoints

### `GET /api/events` and `GET /api/sse` -- DashboardEvent SSE Stream

**Status: FULLY FUNCTIONAL**

- Backed by StateHub's broadcast channel and ring buffer
- Streams `DashboardEvent` payloads as `data:` frames with monotonic `id:` for reconnection
- Supports replay via:
  - `Last-Event-ID` header (highest priority)
  - `?lastEventId=N` query parameter
  - `?n=N` query parameter (legacy)
- Gap handling: when the cursor has fallen out of the ring or replay is too large (>256
  events), sends an explicit `gap` event containing a full `DashboardSnapshot` instead of
  silently truncating
- Keep-alive: 8-second interval (tuned for Railway 30s / Nginx 60s proxy timeouts)
- Secret scrubbing via `LogScrubber` on all payloads before transmission
- 39 distinct event types are published through this channel (see `DashboardEvent` enum)

### `GET /api/workflow/events` -- RuntimeEvent SSE Stream

**Status: FULLY FUNCTIONAL**

- Backed by `SseAdapter` which subscribes to the global `RuntimeEvent` broadcast bus
- Streams WorkflowEngine lifecycle events (workflow started, phase transitions, gate
  results, output, completion)
- Same SSE response headers and keep-alive configuration as the dashboard stream

### `GET /api/projections/{name}/stream` and `GET /api/projections/telemetry/stream`

**Status: FULLY FUNCTIONAL**

- Per-projection SSE streams filtered from the StateHub broadcast
- Streams delta frames for named surfaces (Workbench, Inbox, Canvas, Minimap, Autonomy)
- Telemetry stream serves Lens runtime observations

---

## 5. WebSocket Endpoints

### `GET /ws` and `GET /roko-ws` and `GET /ws/agents` -- Main Event WebSocket

**Status: FULLY FUNCTIONAL**

- Bidirectional: server streams `ServerEvent` JSON frames, client sends subscription filters
- Replay from ring buffer on connection (default from seq 0)
- Client control messages support:
  - `subscribe: ["projection:gate_pipeline", "topic:agent.*"]` -- topic/projection filtering
    with glob-style wildcards
  - `cursor: 42` -- resume from sequence number
  - `back_pressure: "at_most_once"` -- only `at_most_once` is implemented; `coalesce` and
    `resume_required` log warnings and fall back
- Size limits: 1 MiB max message, 256 KiB max frame
- Secret scrubbing on all outbound payloads
- Lag detection with periodic warnings (5s debounce)

### `GET /ws/terminal/{id}` -- PTY Terminal WebSocket

**Status: FULLY FUNCTIONAL (when terminal_enabled)**

- Bridges a real `portable-pty` shell process to a WebSocket
- Supports resize, scrollback ring buffer (512 chunks), 60-second grace period for
  disconnects
- Always auth-gated (even on loopback)

### `GET /relay/agents/ws` and `GET /relay/events/ws` -- Relay Proxy WebSocket

**Status: FUNCTIONAL when relay configured**

- Bidirectional bridge via `proxy_ws::bridge_ws()` to upstream agent-relay WebSocket
- Returns 503 when `ROKO_AGENT_RELAY_URL` is not set

### `GET /api/rpc` (WS upgrade) and `GET /api/rpc/events` (WS upgrade) -- Mirage RPC WebSocket

**Status: FUNCTIONAL when mirage configured**

- Bridges `eth_subscribe` and live events WebSocket to upstream mirage-rs
- Returns 503 when `ROKO_MIRAGE_URL` is not set

### `GET /api/aggregator/ws` -- Agent Aggregator WebSocket

**Status: FUNCTIONAL**

- WebSocket upgrade inside the aggregator module, for agent event streaming

### `GET /api/workflow/ws` -- Workflow WebSocket

**Status: FUNCTIONAL**

- Workflow event streaming via WebSocket

---

## 6. Authentication & Authorization

### API Key Auth (`middleware.rs`)

**Status: PRODUCTION-GRADE**

Three-layer middleware stack applied when `serve.auth.enabled = true`:

1. **`require_api_key`** -- validates `X-Api-Key` header or `Authorization: Bearer` token
   against configured API keys. SHA-256 hashes keys for comparison (constant-time via
   `hash_api_key()`). Supports Privy JWT validation via JWKS cache for external auth.

2. **`require_scope`** -- enforces scope-based access control. Every route is classified
   into a required scope (`read`, `write`, `admin`, `agent:write`, `plan:write`,
   `terminal:write`). A comprehensive static manifest maps path prefixes to scopes. Unknown
   routes fail closed to `SCOPE_WRITE_UNCLASSIFIED`. Plugin-registered extension routes have
   dynamic scope lookups.

3. **`require_route_permission`** (`rbac_middleware.rs`) -- RBAC layer that maps API key
   roles to permissions and enforces per-route permission checks.

### Additional Auth Features

- **Per-caller rate limiting**: keyed by API-key hash or client IP (30 req/s per caller)
- **Global rate limiting**: 100 req/s shared across all requests
- **Secret scrubbing**: `LogScrubber` middleware redacts API keys/tokens from JSON responses
  before transmission
- **x402 payment gating**: paid feeds require `X-Payment-Authorization` header with valid
  payment authorization covering the feed price
- **Agent tokens**: per-agent token issuance with SHA-256 hash storage and expiry
- **Relay tokens**: deployment-scoped opaque IDs with hashed verifiers
- **CORS**: configurable origins, unsafe public CORS with logged warning
- **Body limits**: 4 MiB global, 1 MiB for webhooks
- **Terminal routes**: always auth-gated even when global auth is disabled, require
  `terminal:write` scope

### Public (Unauthenticated) Routes

- `GET /health` -- liveness probe
- `GET /ready` -- readiness probe
- `GET /metrics` -- Prometheus scrape
- `POST /webhooks/github` -- HMAC-verified
- `POST /webhooks/slack` -- HMAC-verified
- `POST /webhooks/generic`
- `GET /api/shared/{token}` -- share-receipt reader
- `/{*path}` -- dynamic webhook ingress / SPA fallback

---

## 7. Data Sources

### Disk State (`.roko/`)

| Path | Routes That Read It |
|---|---|
| `.roko/engrams.jsonl` | `/api/signals`, signal promote/prune |
| `.roko/episodes.jsonl` | `/api/episodes` |
| `.roko/learn/efficiency.jsonl` | `/api/learn/efficiency`, metrics |
| `.roko/learn/cascade-router.json` | `/api/learn/cascade-router`, `/api/learn/cascade` |
| `.roko/learn/gate-thresholds.json` | `/api/learn/gate-thresholds`, `/api/learn/adaptive-thresholds` |
| `.roko/learn/experiments.json` | `/api/learn/experiments` |
| `.roko/learn/c-factor.jsonl` | `/api/c-factor/trend`, `/api/metrics/c_factor` |
| `.roko/prd/` | `/api/prds/*` |
| `.roko/plans/` (also `plans/`) | `/api/plans/*` |
| `.roko/research/` | `/api/research` |
| `.roko/state/state-snapshot.json` | `/api/executor/state` |
| `.roko/groups/` | `/api/groups/*` |
| `.roko/chain/arena-state.json` | `/api/arenas/*` |
| `.roko/metrics/` | Telemetry observer output |
| `.roko/knowledge/` | `/api/neuro/query`, `/api/knowledge` |
| `roko.toml` | `/api/config/*` |

### In-Memory State (AppState)

| Field | Routes That Read It |
|---|---|
| `state_hub` (StateHub) | SSE, WebSocket, `/api/statehub/*`, projections, status |
| `event_bus` (EventBus) | WebSocket replay/streaming |
| `supervisor` (ProcessSupervisor) | `/api/managed-agents`, health |
| `discovered_agents` (RwLock<HashMap>) | Agent routes, aggregator |
| `active_runs` / `active_plans` / `operations` | Run/plan status, health |
| `provider_health` (ProviderHealthTracker) | Health, learning, gateway |
| `provider_health_registry` (ProviderHealthRegistry) | Gateway rate limits |
| `latency_registry` (LatencyRegistry) | Health degradation detection |
| `model_call_service` (ModelCallService) | Gateway inference, provider test |
| `cascade_router` (CascadeRouter) | Gateway model selection |
| `metrics` (MetricRegistry) | `/metrics`, `/api/metrics/*` |
| `sse_adapter` (SseAdapter) | `/api/workflow/events` |
| `heartbeats` | Agent heartbeat routes |
| `template_runs` | Success rate, engagement metrics |
| `chain_client` / `chain_wallet` | Chain routes |
| `chain` (ring buffers) | Chain blocks/txs/events/watcher |
| `arenas` (ArenaRuntime) | Arena routes |
| `config` (ArcSwap<RokoConfig>) | Provider, model, config routes |
| `scrubber` (LogScrubber) | All streaming + response middleware |

### External Services (Proxied)

| Service | Connection | Routes |
|---|---|---|
| Per-agent sidecar | HTTP to `agent.endpoints.base_url` | Aggregator routes |
| Agent relay | `ROKO_AGENT_RELAY_URL` | Relay proxy routes |
| Mirage JSON-RPC | `ROKO_MIRAGE_URL` | RPC proxy routes |
| Chain RPC | Alloy provider from `[chain]` config | Chain routes |

---

## 8. PeriodicObserver (Telemetry Sampling)

**Status: FULLY FUNCTIONAL**

- Runs every 30 seconds via `start_periodic_telemetry_observer()`
- Samples three lenses from the default `LensRegistry`: `token-usage`, `latency`, `cost`
- Each cycle produces one `TelemetryObservation` per lens with a shared timestamp
- Output persisted as JSONL under `.roko/metrics/` via rotation-bounded append
- Respects `resources.log_rotation_max_mb` config for file size limits
- Cancellation-safe: stops cleanly on server shutdown via `CancelToken`
- Does not retain `AppState` (captures only cloneable components, preventing ownership cycles)

---

## 9. StateHub

**Status: FULLY FUNCTIONAL -- Central Real-Time Data Hub**

The `StateHub` (from `roko-runtime`) is the backbone of all real-time data flows:

- **Ring buffer**: retains recent `DashboardEvent` envelopes with monotonic sequence numbers
- **Snapshot materialization**: maintains a `DashboardSnapshot` that aggregates:
  - Active plans/tasks/agents counts
  - Gates passed/failed totals
  - Episodes recorded
  - Errors accumulated
  - Cost USD running total
  - Per-plan and per-task state maps
- **Publish/subscribe**: broadcast channel for live event distribution to:
  - SSE handler (`/api/events`)
  - WebSocket handler (`/ws`)
  - Projection stream handlers (`/api/projections/{name}/stream`)
  - Dashboard event bridge
- **Replay**: `subscribe_events_from(seq)` returns historical events from the ring +
  current cursor for reconnection
- **Cursor snapshot**: atomic point-in-time snapshot with sequence number for gap detection

### What Gets Published to StateHub

All 39 `DashboardEvent` variants are published by the runner, serve layer, and background
tasks:

Plan lifecycle, task lifecycle, task phase changes, agent spawned/output/completed, gate
results, phase transitions, efficiency events, diagnoses, experiment winners, c-factor
trends, projection updates, episode recordings, task output appends, event log entries,
cascade router updates, gate threshold updates, marketplace jobs, atelier PRDs, knowledge
entries, efficiency trends, job execution, job progress, errors, chain block/tx/events,
feed tick/online/offline, payment received, settlement completed, inbox items and actions.

---

## 10. What a Dashboard/Frontend Can Consume Today

### Immediately Usable (No Configuration Beyond `roko serve`)

1. **Health monitoring**: `/health`, `/ready`, `/api/health` (uptime, provider health,
   JWKS state, active counts)
2. **Prometheus metrics**: `/metrics` (standard scrape endpoint with labelled counters/
   histograms)
3. **Plan management**: full CRUD, execute/pause/resume, status, gates, costs, reviews
4. **PRD lifecycle**: ideas, drafts, promote, plan generation
5. **Signal/Episode inspection**: read, promote tiers, prune
6. **Gate history**: summary, per-gate time series, waterfall visualization format
7. **Learning dashboards**: efficiency trends, cascade router state, experiments,
   adaptive thresholds, c-factor, cost tiers, provider outcomes
8. **Agent fleet management**: list/create/start/stop/restart, token management
9. **Real-time streaming**: SSE events stream, WebSocket events, workflow events
10. **Projection surfaces**: Workbench, Inbox, Canvas, Minimap, Autonomy
11. **Config management**: read/write/reload `roko.toml`
12. **Knowledge queries**: search the neuro store
13. **Research artifacts**: list/create research
14. **OpenAPI spec**: `/api/openapi.json`
15. **Benchmark suite**: run/compare/export benchmarks
16. **Trigger management**: CRUD bindings, manual fire, history
17. **Feed management**: CRUD descriptors, runtime status, health
18. **Recipe management**: CRUD recipe DAGs
19. **Extension health**: list loaded extensions with circuit breaker state
20. **Retention/parity**: policy violations, cross-surface parity matrix

### Requires External Service Configuration

21. **Chain data**: requires `[chain]` config with RPC endpoint and contract addresses
22. **Relay proxy**: requires `ROKO_AGENT_RELAY_URL`
23. **RPC proxy**: requires `ROKO_MIRAGE_URL`
24. **Terminal**: requires `serve.terminal_enabled = true`

### Not Usable (Stubs)

25. DeFi endpoints (8 routes, all 501)
26. Marketplace mutations (2 routes, acknowledged but no-op)
