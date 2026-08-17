# HTTP Control Plane & Persistence Audit

208 `.route()` calls in `src/routes/` (~200 in module table; some are test-only mock routes in `#[cfg(test)]` blocks), 39 fields in AppState, 50+ persistence files under .roko/ — a comprehensive control plane with good patterns and some inconsistencies.

### Architecture Runner Status (2026-04-28)
**HTTP/SSE adapter and persistence infrastructure:**
- `SseAdapter` (P3B, `roko-serve/src/adapters.rs`) implements `EventConsumer` for SSE streaming
- `RuntimeProjection` (P3C) provides materialized view for REST endpoints
- `JsonlLogger` (P3C) provides durable event journal
- `PipelineStateV2` (P2A) includes checkpoint/resume logic
- **Remaining**: REST endpoint wiring to RuntimeProjection, HTTP entry point convergence

## The Problem

The HTTP layer is well-built: layered middleware, SSE/WebSocket streaming, hot-reload config, atomic persistence. The issues are: route dispatch duplication (4+ paths reimplementing agent dispatch), persistence duplication (.roko/learn/ vs .roko/memory/), projection staleness not enforced, and no transactional multi-file persistence.

---

## 1. Route Architecture

### 30 Route Modules (~198 production `.route()` registrations)

Note: the `routes/` directory contains 208 total `.route()` calls, but 9 are test-only helpers inside `middleware.rs` and 1 is the top-level `/health` in `mod.rs`, leaving ~198 production API routes. Many `.route()` calls also handle multiple HTTP methods (e.g., `.route("/jobs", get(list_jobs).post(create_job))`), so the effective HTTP endpoint count is higher still.

| Module | .route() calls | Purpose |
|---|---|---|
| agents.rs | 13 | Agent CRUD, lifecycle, message dispatch, token management |
| aggregator.rs | 25 | Agent discovery, relay health, distributed queries |
| auth.rs | 2 | OAuth/JWT, Privy verification |
| chain.rs | 3 | On-chain reads/writes via alloy |
| config.rs | 2 | Config hot-reload, strategy document loading |
| connectors.rs | 3 | MCP connector registry (GET+POST on /connectors, DELETE on /{name}, GET /{name}/health) |
| deployments.rs | 6 | Railway/Fly/manual cloud deployment |
| diagnosis.rs | 1 | Health diagnostics (/diagnosis/recent) |
| dream.rs | 2 | Offline consolidation triggers |
| feeds.rs | 2 | Feed registry management (GET+POST on /feeds, GET+DELETE on /feeds/{id}) |
| gateway.rs | 5 | Inference completion, batch processing, model stats |
| heartbeats.rs | 2 | Agent heartbeat collection |
| integrations.rs | 3 | Third-party integrations |
| jobs.rs | 10 | Marketplace job CRUD, execution, dispatch |
| learning/mod.rs | 25 | Efficiency, cascade router, experiments, thresholds |
| neuro.rs | 2 | Knowledge store queries |
| plans.rs | 14 | Plan CRUD, execution, task review, estimation |
| prds.rs | 9 | PRD lifecycle, auto-plan trigger |
| projections.rs | 3 | Projection catalog, validation, refreshing |
| providers.rs | 5 | Model/provider info, routing config, health (nested under /providers, /models, /routing) |
| research.rs | 6 | Research agent dispatch, topic analysis |
| run.rs | 2 | One-shot prompt execution |
| secrets.rs | 4 | Secret management, rotation |
| shared_runs.rs | 2 | Shareable HTML run pages (no auth) |
| sse.rs | 2 | Server-Sent Events streaming (/api/events, /api/sse) |
| status/ | 27 | Dashboard, health, gates, metrics, episodes, statehub |
| subscriptions.rs | 5 | Event subscription CRUD |
| team.rs | 4 | Team/workspace management |
| templates.rs | 3 | Agent template registry |
| vision_loop.rs | 3 | Vision processing pipeline |
| webhooks.rs | 3 | Webhook CRUD, trigger replay |
| ws.rs | 2 | WebSocket real-time streaming |

### Route Assembly (mod.rs)

Layered middleware:
- **Public routes** (no auth): `/health`, `/runs/{id}`, `/api/webhooks`, `/api/terminal`
- **Protected API** (`/api`): routes with optional scope + API-key auth
- **WebSocket** (`/ws`, `/roko-ws`): Optional bearer token auth — mounted at the top level, NOT under `/api`
- **SPA fallback**: Embedded React app at unmatched routes

---

## 2. AppState (39 fields)

```rust
pub struct AppState {
    // Core
    pub workdir: PathBuf,
    pub layout: RokoLayout,
    pub signal_store: SignalStore,          // Lazily-initialized engrams.jsonl writer
    pub cancel: CancelToken,
    pub started_at: Instant,
    pub metrics: Arc<MetricRegistry>,
    pub supervisor: Arc<ProcessSupervisor>,
    pub affect_engine: Mutex<DaimonState>,

    // Streaming (two independent buses)
    pub event_bus: EventBus<ServerEvent>,       // Ring buffer (16 384) + broadcast for WS
    pub state_hub: roko_core::SharedStateHub,   // Ring buffer (1 024) + broadcast for SSE + TUI

    // Config + subscriptions
    pub subscriptions: SubscriptionRegistry,
    pub runtime: Arc<dyn CliRuntime>,       // Delegates to CLI operations
    pub roko_config: ArcSwap<RokoConfig>,   // Lock-free hot-reload

    // Provider health + latency
    pub provider_health: ProviderHealthTracker,
    pub latency_registry: LatencyRegistry,

    // Execution tracking
    pub active_runs: RwLock<HashMap<String, RunHandle>>,
    pub active_plans: RwLock<HashMap<String, PlanHandle>>,
    pub operations: RwLock<HashMap<String, OperationHandle>>,

    // Agent discovery + templates
    pub templates: RwLock<TemplateRegistry>,
    pub deploy_backend: Arc<dyn DeployBackend>,
    pub deployments: RwLock<HashMap<String, Deployment>>,
    pub template_runs: RwLock<HashMap<String, Vec<TemplateRunRecord>>>,
    pub scrubber: Arc<LogScrubber>,
    pub jwks_cache: Arc<JwksCache>,
    pub http_client: reqwest::Client,
    pub discovered_agents: RwLock<HashMap<String, DiscoveredAgent>>,
    pub aggregator_cache: RwLock<HashMap<String, CachedJsonValue>>,
    pub heartbeats: RwLock<VecDeque<HeartbeatPayload>>,

    // Chain (alloy, optional)
    pub chain_client: Option<Arc<AlloyChainClient>>,
    pub chain_wallet: Option<Arc<AlloyChainWallet>>,
    pub agent_count: Arc<AtomicU32>,
    pub relay_health: Arc<RwLock<RelayHealth>>,

    // Registries
    pub connectors: RwLock<ConnectorRegistry>,
    pub feeds: RwLock<FeedRegistry>,

    // Gateway + learning
    pub cascade_router: RwLock<Option<CascadeRouter>>,
    pub gateway_model_counters: RwLock<HashMap<String, Arc<GatewayModelCounters>>>,
    pub batch_progress: RwLock<HashMap<String, Arc<BatchProgress>>>,

    // Terminal sessions
    pub terminal_sessions: SessionManager,
}
```

---

## 3. Real-Time Streaming

### SSE (`/api/events`, `/api/sse`)

- Axum `Sse<Stream>` with `KeepAlive`
- Replay mechanism: `Last-Event-ID` header → server replays from sequence number via `state_hub.replay_from()`
- Live subscription: broadcast channel from `state_hub.subscribe_events()`
- Events: `DashboardEvent` envelopes with monotonic `seq` IDs
- On-disk log: `.roko/events.jsonl` written at startup by `StateHub::with_event_log()`

### WebSocket (`/ws`, `/roko-ws`)

Note: WebSocket endpoints are at `/ws` and `/roko-ws`, mounted at the top level (not under `/api`). There is no `/api/ws` route.

- Initial replay: on connect, replays buffered `ServerEvent`s from `event_bus` (ring buffer of 16 384)
- Live streaming: broadcasts new events to all connected clients via `event_bus.subscribe()`
- Filtering: client sends JSON `{subscribe: [...], cursor: N, back_pressure: "..."}` message
- Subscription filter patterns: plain substring match, `projection:<name>`, `topic:<pattern>` (with `*` wildcard), `engram-stream:<name>`
- Back-pressure modes: `AtMostOnce` (default), `Coalesce`, `ResumeRequired`
- Cursor resume: client sends `cursor` to replay missed events from a sequence number
- Lagged detection: warns when events dropped, aggregates lag with 5s throttle

### Two Independent Buses

The server maintains **two** separate event buses:

| Bus | Field | Capacity | Consumer | Event type |
|---|---|---|---|---|
| `EventBus<ServerEvent>` | `event_bus` | 16 384 ring buffer | WebSocket (`/ws`, `/roko-ws`) | `ServerEvent` |
| `SharedStateHub` | `state_hub` | 1 024 ring buffer + `.roko/events.jsonl` | SSE (`/api/events`, `/api/sse`) + TUI | `DashboardEvent` |

### StateHub Pattern

- Event log: `.roko/events.jsonl` (opened with `StateHub::with_event_log(1024, path)`)
- Broadcast channel: all events published live to subscribers
- Ring buffer: last 1024 events for late-joiner replay
- Monotonic sequence: each envelope carries `seq: u64` for cursor-based reconnection

---

## 4. Gateway Dispatch

### Inference Gateway (`POST /api/inference/complete`)

1. Model selection via CascadeRouter (or explicit hint)
2. Routing hints: task_category, complexity, role, iteration
3. Dispatch via `state.runtime.run_once()`
4. Token accounting → `gateway_model_counters`
5. Event publishing → `ServerEvent::AgentOutput` to SSE/WS
6. Cost tracking from model profile pricing

### Batch Inference (`POST /api/inference/batch/submit`)

- Submits batch of `CompletionRequest` items
- Returns `batch_id` for polling progress
- Per-batch progress in `state.batch_progress`

---

## 5. CLI/Serve Overlap

### Shared Runtime Bridge

The serve routes do **not reimplement** CLI logic — they delegate via `CliRuntime` trait:
- `state.runtime.run_once(workdir, prompt)` → embedded CLI
- Used by: `/api/run`, `/api/inference/complete`, `/api/plans/{id}/execute`, `/api/research/*`

### Duplicated Patterns

| Area | CLI | Serve | Issue |
|---|---|---|---|
| Plan execution | Direct orchestration | Background task + event tracking | Different error handling |
| Agent lifecycle | Process supervisor commands | Proxy via HTTP + registration | Extra indirection |
| Status/Learning | `roko status` command | `/api/learn/*`, `/api/executor/state` | Should share projection logic |

---

## 6. Per-Agent Sidecar

**13 `.route()` registrations** in `roko-agent-server` (`crates/roko-agent-server/src/`):

| Route | Purpose |
|---|---|
| `/health` | Liveness probe |
| `/stats` | Agent metrics |
| `/logs` | Recent log tail |
| `/message` | Real LLM dispatch (tool loop) |
| `/stream` | Streaming completion (WS) |
| `/predictions` | Outcome prediction |
| `/research` | Research task dispatch |
| `/tasks` | Task execution |

**Key differences from main serve:**
- Single agent focus (one agent per sidecar)
- Real LLM dispatch (direct `LlmBackend + ToolDispatcher`, not runtime bridge)
- Reports to main serve via heartbeat at `/api/heartbeats` every ~10s
- Features are optional (can disable messaging, predictions, etc.)

---

## 7. Persistence Locations

### Directory Structure (.roko/)

The canonical layout is defined in `roko-fs/src/layout.rs` (`RokoLayout`). The 8 top-level directories created by `ensure_dirs()` are: `runtime/`, `memory/`, `plans/`, `runs/`, `state/`, `config/`, `cache/`, `learn/`. Many additional files/directories are created at runtime by specific subsystems.

```
.roko/
  VERSION                           # Layout version (V1)
  engrams.jsonl                    # Main signal log (was signals.jsonl, legacy path still readable)
  events.jsonl                     # StateHub event log (written by StateHub::with_event_log)
  custody.jsonl                    # Append-only custody audit chain
  witness.jsonl                    # Append-only witness DAG log

  runtime/                         # PID files, locks
    roko.pid
    roko.lock

  memory/                          # Durable knowledge (layout.memory_dir())
    episodes.jsonl                 # Agent execution episodes (canonical location)
    playbooks/                     # Per-crate playbook index
    playbook.toml                  # Active playbook
    skills/                        # Learned skills directory
    knowledge-seeds.jsonl          # Distillation seeds
    cascade-router.json            # Also present here (duplicate of learn/)
    costs.jsonl
    efficiency-summaries.jsonl
    experiment-winners.json
    latency-stats.json
    local-rewards.json
    provider-model-outcomes.jsonl

  learn/                           # Learning artifacts (layout.learn_dir())
    efficiency.jsonl               # Per-turn efficiency events
    efficiency-summaries.jsonl     # Aggregated efficiency summaries
    episodes.jsonl                 # ← DUPLICATE of memory/episodes.jsonl
    cascade-router.json            # Model routing bandit state
    gate-thresholds.json           # Adaptive EMA per rung
    gate-ratchet.json              # Gate failure ratchet
    experiment-winners.json        # Winner selections
    provider-model-outcomes.jsonl  # Per-provider success rates
    task-metrics.jsonl             # Complexity/cost baselines
    c-factor.jsonl                 # Coordination trend
    costs.jsonl                    # Per-task cost breakdown
    conductor.json                 # Conductor state
    skills.json                    # Skill registry
    latency-stats.json             # Provider latency percentiles
    local-rewards.json             # Bandit arm rewards
    knowledge-seeds.jsonl          # ← DUPLICATE of memory/knowledge-seeds.jsonl
    heartbeat.json                 # Agent heartbeat state
    heartbeat.jsonl                # Agent heartbeat log
    # Note: experiments.json is NOT present by default; created on first experiment write

  state/                           # Orchestration snapshots (layout.state_dir())
    executor.json                  # Plan executor snapshot (layout.executor_snapshot())
    events.json                    # Event log snapshot (layout.event_log_snapshot())
    server-state.json              # HTTP server state (discovered_agents + template_runs)
    sessions/                      # Per-session directories (layout.sessions_dir())
      {session_id}/

  config/                          # Config files (layout.config_dir())
    config.toml

  cache/                           # Build + context caches (layout.cache_dir())
    cargo-target/
    context-pack-cache/

  plans/{plan_id}/                 # Per-plan artifacts (layout.plan_dir())
    tasks.toml
    execution.json

  runs/{run_id}/                   # Per-run traces (layout.run_dir())
    metrics.jsonl
    traces/

  sessions/                        # Web UI PTY sessions
  daimon/                          # Affect engine state
    affect.json
  neuro/                           # Knowledge store
  research/                        # Research artifacts
  prd/                             # PRD lifecycle
  conductor/                       # Watcher state
  metrics/                         # Prometheus snapshots
  traces/                          # OTEL trace export
```

### Persistence Patterns

**1. Atomic JSON (write-tmp-rename):**
- Used for: cascade-router.json, gate-thresholds.json, experiments.json (when present), conductor.json, server-state.json
- Pattern: write to `.json.tmp`, fsync, rename
- Guarantees: POSIX atomic on same filesystem
- Example: `AppState::save_snapshot()` writes to `server-state.json.tmp` then renames

**2. Append-Only JSONL:**
- Used for: engrams.jsonl, efficiency.jsonl, episodes.jsonl, costs.jsonl, events.jsonl
- Pattern: `OpenOptions::append(true)` (serialized through tokio Mutex in `FileSubstrate`)
- Compaction: `FileSubstrate::compact()` takes in-memory snapshot, writes to `.tmp`, renames
- Note: `FileSubstrate` deduplicates on `ContentHash`; re-writing a known signal is a no-op

**3. Hot-Reload (ArcSwap):**
- Used for: `roko.toml` config
- Pattern: `config_watcher` polls mtime every 2 seconds, 500ms debounce on change
- Lock-free reads via `ArcSwap<RokoConfig>` — `state.load_roko_config()` / `state.store_roko_config()`

**4. Projections:**
- Loaded on-demand via `RuntimeProjectionSet::load()`
- Per-projection max-age (5s dashboard, 30s learning, 10s gates)
- Wrapped in `ProjectionEnvelope<T>` with schema version + cursor

---

## 8. Anti-Patterns

| Anti-Pattern | Where | Impact |
|---|---|---|
| **Persistence duplication** | episodes.jsonl in both `.roko/learn/` AND `.roko/memory/` | Stale data risk, unclear canonical source |
| **No transactional multi-file writes** | CascadeRouter + GateThresholds written separately | Crash between writes → inconsistent state |
| **Projection staleness not enforced** | `InvalidationPolicy.max_age_secs` defined but no TTL check | Projections can serve stale data beyond max_age |
| **Route dispatch duplication** | agents, research, plans, run routes each reimplement dispatch | Error handling changes must update 4+ places |
| **StateHub/learning disconnect** | SSE publishes DashboardEvent; learning reads AgentEfficiencyEvent | Real-time view doesn't match learning state |
| **Config validation asymmetry** | HTTP uses validator crate; CLI uses custom logic | HTTP may accept payloads CLI rejects |
| **Terminal sessions not persisted** | SessionManager in-memory only | Server restart loses all PTY sessions |
| **Aggregator cache no invalidation** | Short-lived cache but no trigger-based invalidation | TTL conflicts with heartbeat timing |

---

## 9. What Works Well

- **StateHub push pattern** is clean — ring buffer, monotonic sequence, cursor-based replay
- **Atomic JSON persistence** prevents partial-write corruption
- **ArcSwap config** hot-reload is lock-free and well-tested
- **SSE Last-Event-ID replay** allows seamless reconnection
- **WebSocket back-pressure** modes prevent slow clients from blocking fast events
- **CliRuntime bridge** avoids reimplementing CLI logic in routes (mostly)

---

## 10. File Inventory

| File | LOC | Status |
|---|---|---|
| `roko-serve/src/lib.rs` | 1 322 | Server init, route wiring |
| `roko-serve/src/state.rs` | 973 | AppState (39 fields) |
| `roko-serve/src/routes/mod.rs` | 147 | Route assembly + middleware |
| `roko-serve/src/routes/gateway.rs` | 1 345 | Inference dispatch |
| `roko-serve/src/routes/learning/mod.rs` | 930 | Learning projections (25 routes) |
| `roko-serve/src/routes/plans.rs` | 1 653 | Plan CRUD + execution |
| `roko-serve/src/routes/agents.rs` | 2 086 | Agent lifecycle |
| `roko-serve/src/routes/sse.rs` | 64 | SSE handler |
| `roko-serve/src/routes/ws.rs` | 233 | WebSocket handler |
| `roko-serve/src/routes/status/mod.rs` | 644 | Status, health, metrics, episodes |
| `roko-serve/src/projection_contract.rs` | 2 815 | Projection versioning + `RuntimeProjectionSet` |
| `roko-fs/src/atomic.rs` | 128 | Atomic write helpers |
| `roko-fs/src/file_substrate.rs` | 551 | Append-only JSONL + `FileSubstrate` |
| `roko-fs/src/layout.rs` | 544 | .roko/ directory structure (`RokoLayout`) |
| **Total roko-serve** | ~15K+ | **30 route modules, 208 `.route()` calls (some in test code)** |

---

## Sources

Key source files verified for this audit:

| File | What was verified |
|---|---|
| `crates/roko-serve/src/routes/mod.rs` | Route assembly, middleware layers, WebSocket mount path |
| `crates/roko-serve/src/state.rs` | AppState fields (39 exact), field types, ring buffer sizes, snapshot path |
| `crates/roko-serve/src/routes/sse.rs` | SSE route paths (`/api/events`, `/api/sse`), replay mechanism |
| `crates/roko-serve/src/routes/ws.rs` | WS route paths (`/ws`, `/roko-ws`), back-pressure modes, filter syntax |
| `crates/roko-serve/src/routes/learning/mod.rs` | Learning route count (25), aliases |
| `crates/roko-serve/src/routes/status/mod.rs` | Status route count (27) |
| `crates/roko-serve/src/config_watcher.rs` | Poll interval (2s), debounce (500ms) |
| `crates/roko-serve/src/projection_contract.rs` | `RuntimeProjectionSet`, `ProjectionEnvelope` |
| `crates/roko-fs/src/layout.rs` | `RokoLayout` canonical paths, `ensure_dirs()` (8 top-level dirs) |
| `crates/roko-fs/src/file_substrate.rs` | `FileSubstrate` append-only JSONL, compaction, dedup |
| `crates/roko-fs/src/atomic.rs` | Atomic write helpers |
| `.roko/learn/` (live) | Actual files present: includes `efficiency-summaries.jsonl`, `heartbeat.json`, `heartbeat.jsonl`; `experiments.json` absent |
| `.roko/memory/` (live) | Actual files present: includes duplicated files from `learn/` |
