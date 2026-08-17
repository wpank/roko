# HTTP Control Plane & Persistence Audit

~175 routes across 30 modules, 40+ fields in AppState, 50+ persistence files under .roko/ — a comprehensive control plane with good patterns and some inconsistencies.

## The Problem

The HTTP layer is well-built: layered middleware, SSE/WebSocket streaming, hot-reload config, atomic persistence. The issues are: route dispatch duplication (4+ paths reimplmenting agent dispatch), persistence duplication (.roko/learn/ vs .roko/memory/), projection staleness not enforced, and no transactional multi-file persistence.

---

## 1. Route Architecture

### 30 Route Modules (~175 routes)

| Module | Routes | Purpose |
|---|---|---|
| agents.rs | 14 | Agent CRUD, lifecycle, message dispatch, token management |
| aggregator.rs | 29 | Agent discovery, relay health, distributed queries |
| auth.rs | 2 | OAuth/JWT, Privy verification |
| chain.rs | 3 | On-chain reads/writes via alloy |
| config.rs | 2 | Config hot-reload, strategy document loading |
| connectors.rs | 3 | MCP connector registry |
| deployments.rs | 7 | Railway/Fly/manual cloud deployment |
| diagnosis.rs | 1 | Health diagnostics |
| dream.rs | 1 | Offline consolidation triggers |
| feeds.rs | 2 | Feed registry management |
| gateway.rs | 5 | Inference completion, batch processing, model stats |
| heartbeats.rs | 2 | Agent heartbeat collection |
| integrations.rs | 2 | Third-party integrations |
| jobs.rs | 12 | Marketplace job CRUD, execution, dispatch |
| learning/mod.rs | 25 | Efficiency, cascade router, experiments, thresholds |
| neuro.rs | 2 | Knowledge store queries |
| plans.rs | 14 | Plan CRUD, execution, task review, estimation |
| prds.rs | 9 | PRD lifecycle, auto-plan trigger |
| projections.rs | 3 | Projection catalog, validation, refreshing |
| providers.rs | 8 | Model/provider info, routing config, health |
| research.rs | 6 | Research agent dispatch, topic analysis |
| run.rs | 2 | One-shot prompt execution |
| secrets.rs | 4 | Secret management, rotation |
| shared_runs.rs | 2 | Shareable HTML run pages (no auth) |
| sse.rs | 2 | Server-Sent Events streaming |
| status/ | ~10 | Dashboard, health, gates, metrics, episodes |
| subscriptions.rs | 5 | Event subscription CRUD |
| team.rs | 4 | Team/workspace management |
| templates.rs | 4 | Agent template registry |
| vision_loop.rs | 3 | Vision processing pipeline |
| webhooks.rs | 3 | Webhook CRUD, trigger replay |
| ws.rs | 2 | WebSocket real-time streaming |

### Route Assembly (mod.rs)

Layered middleware:
- **Public routes** (no auth): `/health`, `/runs/{id}`, `/api/webhooks`, `/api/terminal`
- **Protected API** (`/api`): 85+ routes with optional scope + API-key auth
- **WebSocket** (`/ws`, `/api/ws`): Optional bearer token auth
- **SPA fallback**: Embedded React app at unmatched routes

---

## 2. AppState (40+ fields)

```rust
pub struct AppState {
    // Config (hot-reloadable)
    workdir: PathBuf,
    layout: RokoLayout,
    roko_config: ArcSwap<RokoConfig>,

    // Lifecycle
    cancel: CancelToken,
    supervisor: Arc<ProcessSupervisor>,

    // Streaming
    event_bus: EventBus<ServerEvent>,       // Ring buffer + broadcast
    state_hub: SharedStateHub,             // Push-based event log

    // Execution tracking
    active_runs: RwLock<HashMap<String, RunHandle>>,
    active_plans: RwLock<HashMap<String, PlanHandle>>,

    // Learning
    cascade_router: RwLock<Option<CascadeRouter>>,
    provider_health: ProviderHealthTracker,
    gateway_model_counters: HashMap<String, GatewayModelCounters>,

    // Agent discovery
    discovered_agents: RwLock<HashMap<String, DiscoveredAgent>>,
    templates: RwLock<TemplateRegistry>,
    heartbeats: RwLock<VecDeque<HeartbeatPayload>>,

    // Bridge
    runtime: Arc<dyn CliRuntime>,          // Delegates to CLI operations

    // + 20 more fields (metrics, blockchain, deployment, scrubber, etc.)
}
```

---

## 3. Real-Time Streaming

### SSE (`/api/events`)

- Axum `Sse<Stream>` with `KeepAlive`
- Replay mechanism: `Last-Event-ID` header → server replays from sequence number
- Live subscription: broadcast channel from `state_hub.subscribe_events()`
- Events: `DashboardEvent` envelopes with monotonic `seq` IDs
- On-disk log: `.roko/events.jsonl` for replay

### WebSocket (`/ws`, `/api/ws`, `/roko-ws`)

- Initial replay: on connect, replays all buffered events (ring buffer)
- Live streaming: broadcasts new events to all connected clients
- Filtering: client subscribes to event type patterns (`projection:gate_pipeline`, `topic:agent.*`)
- Back-pressure modes: `AtMostOnce`, `Coalesce`, `ResumeRequired`
- Cursor resume: request replay from specific sequence number
- Lagged detection: warns when events dropped, aggregates lag stats

### StateHub Pattern

- Event log: `.roko/events.jsonl` (append-only)
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

**13 routes** in `roko-agent-server`:

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

```
.roko/
  VERSION                           # Layout version
  engrams.jsonl                    # Main signal log
  events.jsonl                     # StateHub event log

  runtime/                         # PID files, locks

  memory/                          # Durable knowledge
    episodes.jsonl                 # Agent execution episodes
    playbooks/                     # Per-crate playbook index
    knowledge-seeds.jsonl          # Distillation seeds

  learn/                           # Learning artifacts
    efficiency.jsonl               # Per-turn efficiency events
    episodes.jsonl                 # ← DUPLICATE of memory/episodes.jsonl
    cascade-router.json            # Model routing bandit state
    gate-thresholds.json           # Adaptive EMA per rung
    gate-ratchet.json              # Gate failure ratchet
    experiments.json               # Prompt A/B store
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

  state/                           # Orchestration snapshots
    executor.json                  # Plan executor snapshot
    orchestrator.json              # Orchestrator state
    server-state.json              # HTTP server state

  plans/{plan_id}/                 # Per-plan artifacts
    tasks.toml
    execution.json

  runs/{run_id}/                   # Per-run traces
    metrics.jsonl
    spans.jsonl

  sessions/                        # Web UI PTY sessions
  daimon/                          # Affect engine state
  neuro/                           # Knowledge store
  research/                        # Research artifacts
  prd/                             # PRD lifecycle
  conductor/                       # Watcher state
  metrics/                         # Prometheus snapshots
  traces/                          # OTEL trace export
```

### Persistence Patterns

**1. Atomic JSON (write-tmp-rename):**
- Used for: cascade-router.json, gate-thresholds.json, experiments.json, conductor.json
- Pattern: write to `.json.tmp`, fsync, rename
- Guarantees: POSIX atomic on same filesystem

**2. Append-Only JSONL:**
- Used for: engrams.jsonl, efficiency.jsonl, episodes.jsonl, costs.jsonl
- Pattern: `OpenOptions::append(true)`
- Compaction: `FileSubstrate::compact()` rewrites + atomic replace

**3. Hot-Reload (ArcSwap):**
- Used for: `roko.toml` config
- Pattern: config watcher every 2 seconds checks mtime
- Lock-free reads via ArcSwap

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
| `roko-serve/src/lib.rs` | ~500 | Server init, route wiring |
| `roko-serve/src/state.rs` | ~400 | AppState (40+ fields) |
| `roko-serve/src/routes/mod.rs` | ~300 | Route assembly + middleware |
| `roko-serve/src/routes/gateway.rs` | ~400 | Inference dispatch |
| `roko-serve/src/routes/learning/mod.rs` | ~800 | Learning projections (25 routes) |
| `roko-serve/src/routes/plans.rs` | ~600 | Plan CRUD + execution |
| `roko-serve/src/routes/agents.rs` | ~500 | Agent lifecycle |
| `roko-serve/src/routes/sse.rs` | ~200 | SSE handler |
| `roko-serve/src/routes/ws.rs` | ~300 | WebSocket handler |
| `roko-serve/src/projection_contract.rs` | ~200 | Projection versioning |
| `roko-agent-server/src/lib.rs` | ~300 | Sidecar server |
| `roko-agent-server/src/features/messaging.rs` | ~400 | Real LLM dispatch |
| `roko-fs/src/atomic.rs` | ~100 | Atomic write helpers |
| `roko-fs/src/file_substrate.rs` | ~300 | Append-only JSONL |
| `roko-fs/src/layout.rs` | ~200 | .roko/ directory structure |
| **Total roko-serve** | ~6K+ | **30 route modules** |
