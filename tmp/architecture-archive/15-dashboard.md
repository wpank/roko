# Dashboard architecture

> Part of the [Roko Architecture Specification](00-INDEX.md). Extracted from v2 redesign doc.
> Includes the API surface section as an appendix.

---

The dashboard works in two modes:

- **Backbone only**: connected to relay. Shows agent presence, chain data, feeds. No plans, PRDs, or learning -- those require roko-serve.
- **Full mode**: connected to relay AND roko-serve. Shows everything.

The UI gracefully degrades. When roko-serve is unreachable, workspace tabs (Plans, PRDs, Learning) show "Connect to a roko workspace to use this feature" instead of an error.

### Three-tier deployment model

> See `architecture-cross-reference.md` (section 1) for the full deployment model and conflict analysis.

The dashboard talks to three independent infrastructure tiers, each of which can operate independently:

| Tier | Components | Always required |
|------|-----------|-----------------|
| **Tier 1 -- Backbone** | Mirage chain (dev) / Korai (prod), relay, indexer | Yes (provides coordination substrate) |
| **Tier 2 -- Workspace** | roko-serve (HTTP API + WS), roko FS | No (enables agent lifecycle, plans, learning) |
| **Tier 3 -- Remote agents** | Per-agent sidecars on Fly/Railway | No (enables isolated cloud agents) |

**Conflict identified (cross-reference doc):** The `BackendOfflineBanner` component currently shows a single "backend offline" message. It does not distinguish between mirage down, roko down, or both down. The banner needs three-state detection: chain status, roko status, and remote agent status.

### Per-agent sidecar access

Per-agent sidecar endpoints (`/agent/status`, `/agent/config`, `/agent/events`, etc.) are accessed through the roko-serve proxy, not via direct dashboard-to-sidecar connections. This is the resolved architecture from the cross-reference doc:

- The dashboard connects to roko-serve (Tier 2) only
- roko-serve proxies requests to per-agent sidecars (Tier 3) on the dashboard's behalf
- This avoids CORS issues, simplifies auth (single Tier 2 token), and prevents the dashboard from tracking per-agent URLs
- The proxy routes are: `GET /api/agents/:id/sidecar/*` -> forwards to `{sidecar_url}/*`

### Data layer

Three components manage the flow from WebSocket events to rendered pixels.

**SubscriptionManager**: Multiplexes connections to agent, chain, relay, and workspace event streams. Each dashboard page declares its subscriptions on mount and releases them on unmount. The manager maintains a single WebSocket to the relay and a single WebSocket to roko-serve, using room-based subscription messages to filter server-side.

```typescript
// Per-page lifecycle
function useDashboardSubscriptions(rooms: string[]) {
  const manager = useSubscriptionManager();

  useEffect(() => {
    manager.subscribe(rooms);
    return () => manager.unsubscribe(rooms);
  }, [rooms]);
}
```

**EventAggregator**: Batches burst events with a 100ms flush window. High-frequency sources (heartbeats at 100ms, chain blocks at 2s) produce more events than the DOM can absorb per frame. The aggregator collects events during the flush window and delivers them as a single batch. A ring buffer (200 events) supports replay for components that mount after events have already fired.

**RenderScheduler**: Coordinates DOM and canvas updates. DOM updates are coalesced and applied in rAF callbacks. Canvas/WebGL renders (Three.js visualizations, real-time charts) run at 60fps on a separate requestAnimationFrame loop. The scheduler prevents DOM thrashing by batching state changes from the EventAggregator into single React renders.

**Three-tier motion system**:

| Tier | Source | Visual expression |
|------|--------|-------------------|
| Heartbeat rhythm | Per-agent heartbeat ticks (100ms-1s) | Agent card pulse, glow intensity |
| Event-paced tickers | Chain blocks (~2s), gate results, task completions | Counter increments, progress bar steps |
| Ambient decay | Knowledge staleness (Ebbinghaus curve), feed inactivity | Fade, desaturation, visual aging |

### Page-to-data-source mapping

Every dashboard section subscribes to specific WebSocket rooms and event types. REST fallbacks provide initial state on page load when WebSocket history is insufficient.

| Section | WS rooms | Event types | REST fallback |
|---------|----------|-------------|---------------|
| Pulse / Command center | `system`, `agent:*:heartbeat` | `heartbeat_aggregate`, `agent_status`, `presence_join`, `presence_leave` | `GET /relay/api/agents` |
| Pulse / Live console | `agent:*`, `agent:*:heartbeat` | `heartbeat`, `output_chunk`, `gate_result` | -- |
| Pulse / Event stream | `system`, `agent:*` | All event types (filtered client-side by user selection) | -- |
| Fleet / Agent fleet | `system` | `presence_join`, `presence_leave`, `heartbeat_aggregate` | `GET /relay/api/agents` |
| Fleet / Agent detail | `agent:{id}`, `agent:{id}:heartbeat`, `agent:{id}:output`, `agent:{id}:trace` | `heartbeat`, `output_chunk`, `gate_result`, `trace`, `cost_update` | -- |
| Forge / Plans | `plan:*` | `task_started`, `task_completed`, `phase_transition` | `GET /api/plans` |
| Forge / Execution | `plan:*`, `agent:*` | `task_started`, `task_completed`, `gate_result`, `output_chunk` | `GET /api/plans/:id` |
| Knowledge / Store | `chain:knowledge` | `knowledge_published`, `knowledge_validated`, `knowledge_challenged` | `GET /mirage/api/knowledge` |
| Knowledge / Stigmergy | `chain:pheromones` | `pheromone_deposited`, `pheromone_decayed` | `GET /mirage/api/pheromones` |
| Treasury / ISFR | `chain:isfr` | `isfr_updated`, `rate_changed` | `GET /mirage/api/isfr` |
| Treasury / Positions | `agent:{id}` | `position_opened`, `position_closed`, `pnl_update` | -- |
| Treasury / Cost | `system` | `cost_update`, per-request from gateway WS | `GET /api/gateway/stats` |
| Arena / Leaderboard | `arena:{id}` | `attempt_completed`, `score_updated` | `GET /api/arenas/:id` |
| System / Providers | `system` | `provider_status`, per-request from gateway WS | `GET /api/gateway/stats` |
| System / Jobs | `system` | `job_created`, `job_assigned`, `job_completed` | `GET /api/jobs` |

### Chain feed integration

The dashboard consumes agent-exposed chain feeds defined in the [Feeds and Data Streams](05-feeds.md) section.

**Agent list**: Each agent's card shows its registered feeds (from the relay feed registry). A chain agent with three feeds shows three small feed indicators with live rates.

**Agent detail page**: When viewing a single agent, its active chain feeds are displayed as live data panels -- raw RPC data, derived indicators, signals, and analysis. This is the same data the agent is processing, made visible to the operator.

**Treasury pages**: Raw and derived price/rate feeds from chain agents flow directly into Treasury views. ISFR rates, position P&L, and cost data all arrive via WebSocket subscription. No polling.

**Feed subscription rule**: All chain data flows through the relay's WebSocket room system (`agent:{id}:feed:{feed_id}`). The dashboard subscribes on page mount and unsubscribes on unmount. Feeds are never polled.

### Adaptive information density

The dashboard adjusts its information density based on the system's `CorticalState`. Three display regimes:

```
Regime      Trigger                     What changes
──────      ───────                     ────────────
Cruise      All agents calm,            Minimal display. Green status dots.
(calm)      no active plans,            Aggregated metrics only. Agent cards
            PE < 0.15 avg               collapsed to single-line summaries.

Volatile    1+ agents in T2,            Affected agents expand automatically.
            active gate failures,       Healthy agents stay collapsed.
            PE 0.15-0.40 avg            Event stream highlights anomalies.

Crisis      Multiple gate failures,     Full traces visible. Remediation
            agent errors,               suggestions shown inline. Per-tick
            PE > 0.40 avg               timeline appears. All agents expand.
```

The regime transitions smoothly (CSS transitions, not hard cuts). The dashboard computes the regime from the aggregate cortical state of all connected agents, updated on each heartbeat aggregate event.

### Progressive disclosure

Three interaction layers control how much detail is visible.

**Layer 0 -- Summary** (visible without interaction):
```
12 agents online    3 plans active    $4.23/hr burn    99.2% gate pass rate
```
One line. Scannable in under a second.

**Layer 1 -- Detail** (one click/hover):
```
Per-agent costs:    coder-1 $0.02/hr  |  research $0.08/hr  |  coder-2 $0.15/hr
Per-model split:    Sonnet 72%  |  Haiku 24%  |  Opus 4%
Cache savings:      $12.40 saved today (L1: 68%, L2: 12%)
```
Breakdown appears in a popover or expanded card.

**Layer 2 -- Trace** (second interaction from Layer 1):
```
Full token log, diff view, per-request cost, HDC fingerprint, gate rung detail,
tool call history, convergence/loop detection events, thinking token breakdown
```
Opens in a slide-out panel or dedicated sub-page.

### Epistemic aesthetics

Visual properties react to system state. These are not decorative -- each visual channel encodes a data dimension.

| Visual property | Data source | Encoding |
|----------------|-------------|----------|
| Glow intensity | Epistemic confidence (gate pass rate, neuro store match quality) | Brighter = higher confidence in agent's knowledge |
| Fade / decay | Knowledge staleness (Ebbinghaus forgetting curve) | Faded entries need re-validation or consolidation |
| Turbulence | Contested knowledge entries (challenged in neuro store) | Shimmering/jittering indicates active dispute |
| Velocity streaks | Active agent output (tokens/sec) | Faster streaks = higher throughput |
| Heartbeat pulse | Per-agent tick cadence (gamma/theta/delta) | Visible rhythm matches agent's adaptive clock |
| Saturation | Validation strength (gate rung depth) | Deeper validation = richer color |

Implemented via Three.js shaders for canvas elements and CSS custom properties for DOM elements. The `CorticalState` broadcast drives all six channels.

### Layout

```
┌─────────────────────────────────────────────────────────────────┐
│ Nunchi              [+ Agent]  [+ Cluster]  [user@email v]      │
├──────┬──────────────────────────────────────────────────────────┤
│      │                                                          │
│ Nav  │  Overview                                                │
│      │  ┌────────────────────────────────────────────────────┐  │
│ Home │  │ * 5 agents   2 clusters   $4.23 today   ^ 3d 2h   │  │
│      │  └────────────────────────────────────────────────────┘  │
│      │                                                          │
│Agents│  Agents                                                  │
│      │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐   │
│      │  │ coder-1  │ │ research │ │ chain-1  │ │ coder-2  │   │
│Feeds │  │ * T0     │ │ * T1     │ │ * T0     │ │ o T2     │   │
│      │  │ coding   │ │ research │ │ chain    │ │ coding   │   │
│      │  │ idle     │ │ querying │ │ monitor  │ │ building │   │
│Plans │  │ $0.02/hr │ │ $0.08/hr │ │ $0/hr    │ │ $0.15/hr │   │
│      │  └──────────┘ └──────────┘ └──────────┘ └──────────┘   │
│      │                                                          │
│Learn │  Cluster: feature-xyz                                    │
│      │  ┌────────────────────────────────────────────────────┐  │
│      │  │ researcher --> impl-1 --> reviewer                 │  │
│Costs │  │ + done        o working   . waiting                │  │
│      │  │               impl-2 --/                           │  │
│      │  │               o working                            │  │
│ Logs │  └────────────────────────────────────────────────────┘  │
│      │                                                          │
│  ⚙   │  Agent: coder-2 (expanded)                              │
│      │  ┌────────────────────────────────────────────────────┐  │
│      │  │ Status: T2 reasoning  |  Uptime: 12m  |  Cost: $1.8│  │
│      │  │ Task: Implement pagination in users API            │  │
│      │  │                                                    │  │
│      │  │ Heartbeat ─────────────────────────────            │  │
│      │  │ T0 T0 T0 T0 T1 T0 T0 T2 T0 T0 T0 T1 [T2]       │  │
│      │  │                                                    │  │
│      │  │ Logs (live -- WebSocket, not polling)              │  │
│      │  │ 14:32:21 [T2] PE=0.73 -> full reasoning           │  │
│      │  │ 14:32:25 [T2] action: edit src/users.rs:142       │  │
│      │  │ 14:32:30 [T0] verify: cargo test -> 47 passed     │  │
│      │  │                                                    │  │
│      │  │ [Stop] [Restart] [View Full Trace] [Open in CLI]  │  │
│      │  └────────────────────────────────────────────────────┘  │
└──────┴──────────────────────────────────────────────────────────┘
```

### Feeds page

A dedicated page for browsing agent chain feeds:

```
┌──────────────────────────────────────────────────────────────────┐
│ Feeds                                                            │
│                                                                  │
│ Active feeds (6)                              [Subscribe to new] │
│                                                                  │
│ ┌──────────────────────────────────────────────────────────────┐ │
│ │ eth-mainnet-blocks    chain-watcher-1    raw     0.08 Hz    │ │
│ │ public | 3 subscribers                                       │ │
│ │ Latest: block 21,432,891 | 142 txs | 15.2 gwei             │ │
│ ├──────────────────────────────────────────────────────────────┤ │
│ │ eth-gas-trend         chain-watcher-1    derived  0.5 Hz    │ │
│ │ paid ($1/hr) | 1 subscriber                                 │ │
│ │ Latest: trend=rising, 5m_avg=18.4, 1h_avg=15.7             │ │
│ ├──────────────────────────────────────────────────────────────┤ │
│ │ base-dex-swaps        defi-scanner      derived  2.0 Hz    │ │
│ │ public | 5 subscribers                                       │ │
│ │ Latest: WETH/USDC swap 12.5 ETH @ $3,241.50                │ │
│ └──────────────────────────────────────────────────────────────┘ │
│                                                                  │
│ Feed detail: eth-mainnet-blocks                                  │
│ ┌──────────────────────────────────────────────────────────────┐ │
│ │ [live chart: block times + gas prices, last 100 blocks]     │ │
│ │                                                              │ │
│ │ Recent events:                                               │ │
│ │ 14:32:45  block 21,432,891  142 txs  15.2 gwei  0.8s       │ │
│ │ 14:32:33  block 21,432,890  98 txs   14.8 gwei  12.1s      │ │
│ │ 14:32:21  block 21,432,889  203 txs  16.1 gwei  12.0s      │ │
│ └──────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

### Settings page

```
Settings
|-- Account
|   |-- Profile (name, email, avatar from Privy)
|   +-- Wallet (Privy embedded wallet address, delegation status)
|
|-- Provider keys
|   |-- Anthropic (Claude) ---- * connected
|   |-- Perplexity (Sonar) ---- o not set
|   |-- Google (Gemini) ------- o not set
|   |-- Moonshot (Kimi) ------- o not set
|   |-- ZAI (GLM) ------------ o not set
|   |-- OpenRouter ----------- o not set
|   |-- Ollama --------------- * found (localhost:11434)
|   +-- Storage: [server-side v] / [client-only]
|
|-- Integrations
|   |-- GitHub -- o not set (enables PR creation)
|   |-- Slack --- o not set (enables notifications)
|   +-- Railway - * connected (OAuth)
|
|-- Infrastructure
|   |-- Fly.io -- o not set (enables isolated agents)
|   |-- Control plane: https://my-roko.up.railway.app -- * healthy
|   |-- Relay: wss://relay.nunchi.dev -- * healthy
|   +-- Mirage: https://mirage-devnet.fly.dev -- * healthy
|
|-- API keys
|   |-- github-actions (agent:write) -- created 2d ago
|   |-- [+ Create key]
|   +-- [Manage keys]
|
+-- Team (phase 2)
    |-- Members
    |-- Invitations
    +-- Roles
```

### Tech stack

| Layer | Library | Version | Purpose |
|-------|---------|---------|---------|
| Framework | React | 19 | Component model, concurrent features |
| Build | Vite | 8 | Dev server, production bundling |
| Data fetching | TanStack Query | 5 | REST cache, stale-while-revalidate |
| State | Zustand | 5 | Client-side stores (agent state, UI preferences) |
| Blockchain | ethers.js | 6 | Chain reads, contract interaction |
| 3D / Canvas | Three.js | latest | Epistemic visualizations, particle systems |
| Charts | Recharts | latest | Time series, cost breakdowns, gate pass rates |
| Auth | Privy | 3 | Wallet + social login, embedded wallets |

### Performance targets

| Metric | Target | How |
|--------|--------|-----|
| FCP | < 1.2s | Code splitting per route, preloaded critical CSS |
| LCP | < 2.0s | SSR for initial state, streaming HTML |
| CLS | < 0.05 | Reserved layout slots for async data |
| WS event-to-render p95 | < 100ms | EventAggregator batching + rAF scheduling |
| Canvas/WebGL | >= 60fps sustained | Separate render loop, instanced geometry |
| Initial JS bundle | < 250KB gzipped | Tree shaking, dynamic imports for heavy deps (Three.js, ethers) |

---

## Appendix: API surface

### Agent lifecycle

```
POST   /api/agents                    Create agent
GET    /api/agents                    List agents (status, health, cost)
GET    /api/agents/:id                Agent detail (full status, heartbeat history)
POST   /api/agents/:id/start         Start a stopped agent
POST   /api/agents/:id/stop          Graceful stop
DELETE /api/agents/:id                Destroy agent + clean up resources
GET    /api/agents/:id/logs           Agent logs (paginated, filterable)
GET    /api/agents/:id/trace/:tick    Full decision trace for a specific tick
POST   /api/agents/:id/message       Send a message/task to an agent
POST   /api/agents/:id/token         Issue/rotate agent bearer token
GET    /api/agents/:id/token/status   Token status (exists, expiry)
GET    /api/agents/:id/feeds          List feeds exposed by this agent
```

### Clusters

```
POST   /api/clusters                  Create cluster with pipeline
GET    /api/clusters                  List clusters
GET    /api/clusters/:id              Cluster status (pipeline progress)
POST   /api/clusters/:id/stop         Stop all agents in cluster
DELETE /api/clusters/:id              Destroy cluster + all agents
```

### Inference gateway

```
POST   /api/inference/proxy           Proxied inference for remote agents
GET    /api/inference/stats           Gateway stats (cache hit rate, costs, latency)
GET    /api/inference/models          Available models + routing weights
POST   /api/inference/models/:id/pin  Pin a model for an agent (override router)
```

### Feeds

```
GET    /api/feeds                     List all registered feeds
GET    /api/feeds/:feed_id            Feed detail + recent data
POST   /api/feeds/:feed_id/subscribe  Subscribe to a feed
DELETE /api/feeds/:feed_id/subscribe  Unsubscribe from a feed
```

### Secrets

```
GET    /api/secrets                    List secret namespaces + keys (not values)
POST   /api/secrets/:ns/:key          Set a secret
DELETE /api/secrets/:ns/:key          Remove a secret
POST   /api/secrets/:ns/:key/test     Test if a secret is valid
```

### Auth

```
POST   /auth/login                     Email/password login
POST   /auth/privy/verify              Verify Privy JWT, create roko session
POST   /auth/device/authorize          Start device flow
POST   /auth/device/token              Poll for device flow token
GET    /auth/callback                  PKCE OAuth callback (for CLI)
POST   /auth/refresh                   Refresh access token
GET    /auth/session                   Current user info
POST   /api/api-keys                   Create API key
DELETE /api/api-keys/:id               Revoke API key
GET    /api/api-keys                   List API keys
```

### WebSocket

```
WS     /ws                            roko-serve event stream (plans, gates, episodes)
WS     /relay/ws                      Relay event stream (presence, feeds, messages)
```

Both WebSocket endpoints support the room-based subscription protocol described in [Connectivity and Relay](04-connectivity.md).

### Infrastructure

```
GET    /api/health                     Service health
GET    /api/providers                  Configured LLM providers + health
GET    /api/costs                      Cost summary (per agent, per day, per model)
GET    /api/costs/:agent_id            Cost breakdown for one agent
GET    /api/relay/health               Relay connection health
```

### Existing routes

All ~85 existing routes in roko-serve (plans, PRDs, gates, episodes, signals, knowledge, learning, research, diagnosis, deployments, etc.) remain unchanged. They get auth middleware added.
