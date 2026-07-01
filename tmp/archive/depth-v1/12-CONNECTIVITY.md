# 12 — Connectivity

> Connectors provide external I/O. Feeds are Signal streams on Bus topics. The relay enables cross-agent communication.

**Subsumes**: Connector trait (arch-04), Feed system (arch-05), relay, workspace discovery, agent connectivity, data flow, reconnection protocol.

**Source**: Refactored from `tmp/architecture/04-connectivity.md` and `tmp/architecture/05-feeds.md` using unified vocabulary.

---

## 1. Connect Protocol

The **Connect protocol** is one of the 9 Block protocols (see [doc-02, section 3.8](02-BLOCK.md#38-connect--connect--query--execute--disconnect)). A Block that implements Connect can establish connections to external systems, execute queries and mutations, and report health.

```rust
pub trait Connect: Block {
    /// Establish connection to external system.
    async fn connect(&mut self, config: &ConnectConfig) -> Result<()>;
    /// One-shot read query.
    async fn query(&self, request: QueryRequest) -> Result<QueryResponse>;
    /// Mutating operation.
    async fn execute(&self, request: ExecuteRequest) -> Result<ExecuteResponse>;
    /// Health check.
    async fn health(&self) -> Result<HealthStatus>;
    /// Graceful disconnect.
    async fn disconnect(&mut self) -> Result<()>;
}
```

### ConnectorKind

```rust
pub enum ConnectorKind {
    ChainRpc,       // Ethereum, Solana, etc.
    Exchange,       // Hyperliquid, Binance, etc.
    McpServer,      // MCP tool servers
    Database,       // Postgres, SQLite, etc.
    Webhook,        // External HTTP endpoints
    Api,            // Generic REST/gRPC APIs
}
```

### Connector = Block + Connect + lifecycle

A **Connector** is a Block specialization (see [doc-04, section 11](04-SPECIALIZATIONS.md#11-connector)). It implements the Connect protocol and manages a connection lifecycle: connect at Agent startup, health-check periodically, disconnect at shutdown.

```rust
pub struct ConnectorManifest {
    pub name: String,
    pub kind: ConnectorKind,
    pub version: Version,
    pub description: String,
    pub config_schema: TypeSchema,      // what ConnectConfig expects
    pub capabilities: Vec<Capability>,  // what system resources it needs
    pub health_interval: Duration,      // how often to check health (default: 30s)
}
```

---

## 2. Connector Discovery

Connectors are discovered from three sources.

| Source | Mechanism | Example |
|---|---|---|
| **Config** | `connectors = [...]` in agent config | `connectors = ["postgres", "hyperliquid"]` |
| **MCP auto-register** | MCP servers in `agent.mcp_config` | MCP server auto-registers as `McpConnector` |
| **Extension-provided** | Extension registers Connectors in `on_init()` | Chain reader Extension registers `ChainRpcConnector` |

There is no registry-based discovery for Connectors (unlike Extensions). Connectors are always explicitly declared in agent config or provided by Extensions.

### Config-based discovery

```toml
# roko.toml
[[agents]]
name = "chain-watcher"
connectors = [
  { name = "ethereum-rpc", kind = "chain_rpc", url = "wss://eth.llamarpc.com" },
  { name = "base-rpc", kind = "chain_rpc", url = "wss://base.llamarpc.com" },
  { name = "postgres", kind = "database", url = "postgres://localhost/roko" },
]
```

### MCP auto-registration

Any MCP server configured in `agent.mcp_config` automatically registers as a `McpConnector`. The MCP tools become available through the Connector's `query()` and `execute()` methods.

```toml
[agent.mcp_config]
servers = [
  { name = "github", command = "roko-mcp-github" },
  { name = "code-intel", command = "roko-mcp-code" },
]
# These auto-register as McpConnectors named "github" and "code-intel"
```

### Extension-provided discovery

Extensions can register Connectors during their `on_init()` hook:

```rust
impl Extension for ChainReaderExt {
    async fn on_init(&mut self, ctx: &mut AgentContext) -> Result<()> {
        // Register a ChainRpc Connector
        ctx.register_connector(Box::new(EthereumConnector::new(
            &self.config.rpc_url,
        )))?;
        Ok(())
    }
}
```

---

## 3. Distinction: Connector vs Extension

| Aspect | Connector | Extension |
|---|---|---|
| Protocol | Connect (connect/query/execute/health/disconnect) | Hook-based interception (22 hooks across 8 layers) |
| Purpose | Provide bidirectional I/O with external systems | Modify Agent behavior through pipeline interception |
| Agent relationship | Agent *uses* Connectors | Agent *loads* Extensions |
| Data direction | Bidirectional (Agent <-> external system) | Interceptor (sits in the pipeline) |
| Lifecycle | Independent connection lifecycle (connect/disconnect) | Tied to Agent lifecycle (on_init/on_shutdown) |
| Composition | An Extension can *wrap* a Connector | A Connector cannot intercept an Extension |

An Agent *loads* Extensions but *uses* Connectors. An Extension can *wrap* a Connector to add retry logic, rate limiting, or caching — but the reverse is not possible.

---

## 4. Feed as Signal Stream

In the unified vocabulary, a **Feed** is not a separate primitive. A Feed is **ephemeral Signals published to a Bus topic by a Connector**. The Feed concept is a naming convention for a common pattern: a Connector continuously producing Signals on a well-known Bus topic.

### How it works

1. A Connector establishes a connection to an external system (chain RPC, exchange API, webhook, etc.).
2. The Connector subscribes to events from the external system.
3. Each event becomes an ephemeral Signal published to a Bus topic.
4. Other Blocks subscribe to that Bus topic to consume the stream.

```rust
// A ChainRpcConnector publishes block Signals to the Bus
impl ChainRpcConnector {
    async fn on_new_block(&self, block: EthBlock, bus: &BusHandle) -> Result<()> {
        let signal = Signal::ephemeral(
            Kind::ChainBlock,
            block.to_payload(),
        );
        bus.publish("feed:eth-mainnet-blocks", signal).await
    }
}
```

### Feed registration

Connectors register their Feed topics with the relay for discovery:

```rust
ctx.relay.register_feed(FeedRegistration {
    feed_id: "eth-mainnet-blocks",
    agent_id: ctx.agent_id.clone(),
    kind: FeedKind::Raw,
    schema: FeedSchema::EthBlock,
    rate_hz: 0.08,  // ~1 block per 12s
    access: FeedAccess::Public,
})?;
```

### Feed kinds

| Kind | Description | Example |
|---|---|---|
| **Raw** | Unprocessed data from external system | Ethereum blocks, exchange trades |
| **Derived** | Processed or computed from raw Feeds | Gas trend indicators, MACD signals |

### Feed access

| Access | Description |
|---|---|
| **Public** | Any subscriber can consume for free |
| **Paid** | Requires payment (MPP session or x402) per time unit |

---

## 5. Workspace Discovery

Roko instances register with the relay on startup. Dashboards discover available workspaces automatically.

### Registration

When `roko serve` starts, it connects to the relay and announces itself:

```json
{
  "type": "workspace_hello",
  "workspace_id": "ws-a1b2c3",
  "name": "will-dev",
  "url": "https://my-roko.up.railway.app",
  "version": "0.1.0",
  "capabilities": ["agents", "plans", "prds", "learning", "gateway"],
  "owner_wallet": "0x7f3b...2c4a",
  "agents_count": 3,
  "uptime_secs": 3600
}
```

### Discovery flow

The relay maintains a workspace directory. Dashboards query it:

```
GET /relay/workspaces
-> [
    {
      "workspace_id": "ws-a1b2c3",
      "name": "will-dev",
      "url": "https://my-roko.up.railway.app",
      "owner_wallet": "0x7f3b...2c4a",
      "agents_count": 3,
      "online": true,
      "last_seen_ms": 1713960000000
    }
  ]
```

### Dashboard connection flow

1. Dashboard loads, connects to relay.
2. Fetches `GET /relay/workspaces` — lists all online roko instances.
3. If user has a Privy wallet, auto-matches workspaces by `owner_wallet`.
4. If exactly one match: auto-connect (zero friction).
5. If multiple matches: show picker ("You have 2 workspaces online — which one?").
6. If no match: show global-only view (agents from relay, chain data, no workspace features).
7. User can also manually add a workspace URL in Settings.

### Local development

`roko serve` on localhost registers with the relay if `relay.url` is configured. For pure local dev (no relay), the dashboard falls back to `VITE_ROKO_API_URL` env var or `localhost:6677`.

```toml
# roko.toml
[relay]
url = "wss://relay.nunchi.dev"
workspace_name = "will-dev"
```

If `[relay]` is not configured, roko serves HTTP only — no relay registration, no auto-discovery. Dashboard must be pointed at it manually.

---

## 6. Data Flow: Subscription-Only

Every piece of data flows through WebSocket subscriptions. No polling.

### Event sources

| Source | Transport | What it carries |
|---|---|---|
| **Relay** | WS `/relay/ws` | Agent presence, message lifecycle, relay health |
| **roko-serve** | WS `/ws` | Plan progress, gate results, episodes, learning metrics, job updates |
| **Agent (direct)** | WS (per-agent) | Heartbeats, streaming LLM output, decision traces |
| **Agent (via relay)** | WS `/relay/ws` | Same as direct, tunneled through relay |
| **Chain** | WS (RPC sub) | Blocks, contract events, ERC-8004 registry updates |

### WebSocket message envelope

Every message through the relay uses the same envelope:

```json
{
  "seq": 4821,
  "ts": 1713974400123,
  "room": "agent:coder-1:heartbeat",
  "type": "heartbeat",
  "payload": { ... }
}
```

| Field | Type | Purpose |
|---|---|---|
| `seq` | `u64` | Monotonic sequence number per connection. Enables reconnection replay. |
| `ts` | `u64` | Unix milliseconds. Server clock. |
| `room` | `string` | Scoping. Clients subscribe to rooms, receive only matching messages. |
| `type` | `string` | Event discriminant. |
| `payload` | `object` | Type-specific data. |

### Room naming convention

```
agent:{id}                  Agent lifecycle events (spawn, stop, error)
agent:{id}:heartbeat        Heartbeat ticks (T0/T1/T2, cortical state)
agent:{id}:output           Streaming LLM output chunks
agent:{id}:trace            Decision traces per tick
agent:{id}:feed:{feed_id}   Signal streams exposed by the agent
plan:{id}                   Plan progress, task transitions, gate results
cluster:{id}                Cluster pipeline events
system                      Server health, provider status, cost updates
learning                    Experiment results, router updates, thresholds
```

### Event types

| Type | Room pattern | Payload |
|---|---|---|
| `presence_join` | system | `{ agent_id, mode, profile }` |
| `presence_leave` | system | `{ agent_id, reason }` |
| `heartbeat` | agent:{id}:heartbeat | `{ tick, tier, pe, cortical_state }` |
| `output_chunk` | agent:{id}:output | `{ content, done, usage }` |
| `trace` | agent:{id}:trace | `{ tick, steps[], gate_result }` |
| `task_started` | plan:{id} | `{ task_id, phase }` |
| `task_completed` | plan:{id} | `{ task_id, outcome }` |
| `gate_result` | plan:{id} | `{ task_id, gate, rung, passed }` |
| `phase_transition` | plan:{id} | `{ from, to }` |
| `feed_data` | agent:{id}:feed:{fid} | `{ feed_id, data }` |
| `feed_registered` | system | `{ agent_id, feed_id, schema }` |
| `cost_update` | system | `{ agent_id, delta, total }` |
| `provider_status` | system | `{ provider, healthy, latency_ms }` |
| `experiment_result` | learning | `{ experiment_id, winner, p_value }` |
| `router_update` | learning | `{ model, weight, reason }` |

---

## 7. Backpressure and Coalescing

High-frequency events (heartbeats at 100ms, chain blocks at 2s) need throttling for dashboard consumption.

| Strategy | Applies to | Behavior |
|---|---|---|
| **Coalesce** | heartbeat | Relay buffers heartbeats per agent, sends latest every 500ms to dashboard subscribers |
| **Drop-oldest** | output_chunk | Ring buffer per agent (1024 chunks). Slow consumers miss old chunks, catch up from latest. |
| **Lossless** | gate_result, task_completed | Queue with backpressure. If client can't keep up, relay applies TCP-level flow control. |
| **Sample** | feed_data | Agent-configurable sample rate. Default: every Nth update where N = ceil(source_rate / 2Hz). |

---

## 8. Reconnection Protocol

Clients track the last received `seq`. On reconnect, the client sends a `resume` message:

```json
{ "type": "resume", "last_seq": 4821 }
```

### Recovery flow

```
Client                                  Relay
  |                                       |
  |---- WS connect ---------------------->|
  |                                       |
  |---- { "type": "resume",              |
  |       "last_seq": 4821 } ----------->|
  |                                       |
  |                           +-----------+
  |                           | Check gap |
  |                           +-----------+
  |                                       |
  |  Case 1: gap <= 64K entries           |
  |<---- replay events 4822..4900 --------|
  |<---- live events continue ------------|
  |                                       |
  |  Case 2: gap > 64K entries            |
  |<---- { "type": "snapshot",            |
  |        "state": { ... } } ------------|
  |<---- live events continue ------------|
```

### Snapshot format

The snapshot contains the minimum state needed to rebuild client-side views:

```json
{
  "type": "snapshot",
  "seq": 71042,
  "state": {
    "agents": [
      { "id": "coder-1", "online": true, "mode": "persistent", "profile": "coding" },
      { "id": "research", "online": true, "mode": "ephemeral", "profile": "research" }
    ],
    "feeds": [
      { "feed_id": "eth-gas-trend", "agent_id": "chain-watcher-1", "schema": "gas_trend_v1" }
    ],
    "rooms": ["agent:coder-1", "agent:coder-1:heartbeat", "plan:current"]
  }
}
```

### Gap detection on the client

Clients track the last received `seq` and check every incoming message for continuity. A gap (missing sequence numbers) means events were lost. On gap detection, the client reconnects and sends a `resume` message.

### Relay ring buffer

The relay maintains a ring buffer (default: 64K entries, ~10 minutes at moderate throughput). If the gap exceeds the buffer, the relay sends a `snapshot` event with current state followed by live events.

---

## 9. Agent Connectivity

Agents communicate across users and across machines. The relay is the rendezvous point — any Agent connected to the relay can discover and message any other Agent, regardless of ownership.

```
User A's roko process          Relay            User B's Fly Machine
+----------------+                              +----------------+
| agent-alpha    |---- WS --->+--------+<-- WS -| agent-beta     |
|                |            | Relay  |        |                |
| Can message    |<-- relay --| routes |-- relay>| Can message    |
| agent-beta     |            | by id  |        | agent-alpha    |
+----------------+            |        |        +----------------+
                              |        |
User C's dashboard            |        |        User D's agent
+----------------+            |        |        +----------------+
| Dashboard      |---- WS -->|        |<-- WS -| agent-gamma    |
| sees all 3     |            +--------+        | behind NAT     |
| agents         |                              +----------------+
+----------------+
```

### Communication patterns

| Pattern | Mechanism | Description |
|---|---|---|
| **Direct messaging** | Relay-routed | Agent A sends message to Agent B via relay. B processes it in next tick. |
| **Feed subscription** | Bus topic | Agent A subscribes to Agent B's Feed (Signal stream on Bus topic). |
| **Pheromone signaling** | On-chain Signals | Agents deposit pheromone Signals. Any Agent can read them — stigmergic coordination. |
| **Cluster collaboration** | Cluster pipeline | Agents from different users join the same cluster if authorized. |
| **Knowledge sharing** | InsightStore | Agents publish knowledge Signals on-chain. Any Agent can query. |

### Auth controls

Auth controls what an Agent can do, not who it can talk to:

| Action | Auth required |
|---|---|
| Discover agents on relay | None (public) |
| Read agent card / capabilities | None (public) |
| Send message to agent | Privy JWT or API key |
| Subscribe to free Feed | None |
| Subscribe to paid Feed | MPP session or x402 payment |
| Join a cluster | Cluster owner's invitation token |
| Read on-chain knowledge | None (public chain data) |
| Publish knowledge on-chain | Agent wallet signature |

---

## 10. Agent Topologies

### In-process agents (default)

Agents run as tokio tasks inside the roko process. Communication happens through channels.

```
+------------------------------------------------------------+
|                      roko process                           |
|                                                             |
|  +-----------+       mpsc         +------------------+      |
|  | Control   | <----------------> | AgentRuntime     |      |
|  | Plane     |       mpsc         | "coder-1"        |      |
|  |           |                    |                   |      |
|  | Routes    |       mpsc         +------------------+      |
|  | msgs via  | <----------------> | AgentRuntime     |      |
|  | channel   |       mpsc         | "research"       |      |
|  | map       |                    +------------------+      |
|  +-----------+                                              |
|                                                             |
|  +--------------------------------------------------------+ |
|  | Inference Gateway (shared by all in-process agents)     | |
|  +--------------------------------------------------------+ |
+------------------------------------------------------------+
```

Benefits: zero serialization overhead, shared inference gateway, shared memory structures, no network latency.

### Remote agents

For isolation or NAT traversal, Agents connect OUTBOUND to the relay. No inbound server required.

```bash
# On the Fly Machine / Railway container
roko agent run \
  --name "isolated-coder" \
  --relay wss://relay.nunchi.dev \
  --inference-proxy https://my-roko.up.railway.app/api/inference \
  --auth-token $AGENT_TOKEN
```

The Agent:
1. Connects to the relay WebSocket.
2. Announces presence with its Agent ID and capabilities.
3. Enters the standard `run()` loop.
4. Routes inference requests to the parent's gateway via HTTPS proxy.
5. Publishes heartbeats and events through the relay.

### Direct-reachable agents

Some remote Agents have public URLs (Railway services, dedicated VMs). These can receive messages directly via HTTP in addition to the relay path.

```toml
# roko.toml
[[remote_agents]]
name = "staging-monitor"
url = "https://staging-monitor.fly.dev"
auth_token_ref = "secrets.staging_monitor_token"
```

The control plane prefers direct HTTP for request-response patterns (lower latency) and uses the relay for event streaming and presence.

---

## 11. Agent Discovery: Three Sources Merged

Agents are discovered from three sources, merged client-side:

| Source | Provides | Availability |
|---|---|---|
| **Relay presence** | Who is online right now. Liveness truth. | Always (if relay configured) |
| **ERC-8004 on-chain registry** | Wallet, reputation, stake, capabilities, feed adverts. Identity truth. | Always (public chain data) |
| **User deployment list** | Railway/Fly URLs, manually added endpoints. | Per-user (localStorage / roko.toml) |

### Merged agent view

```rust
pub struct MergedAgent {
    pub id: String,
    pub name: String,

    // From relay
    pub online: bool,
    pub last_seen: u64,
    pub mode: AgentMode,
    pub profile: String,

    // From chain (ERC-8004)
    pub wallet: Option<Address>,
    pub reputation: Option<f64>,
    pub stake: Option<u128>,
    pub tier: Option<AgentTier>,       // gray | copper | silver | gold | amber
    pub capabilities: Option<Vec<String>>,
    pub feeds: Option<Vec<FeedAdvertisement>>,

    // From deployment list
    pub direct_url: Option<String>,
    pub deploy_platform: Option<DeployPlatform>,
}

pub struct FeedAdvertisement {
    pub feed_id: String,
    pub schema: String,
    pub rate_hz: f64,
    pub access: FeedAccess,
    pub description: String,
}
```

---

## 12. Message Routing

The control plane routes messages based on Agent location, with a three-tier fallback:

```rust
impl ControlPlane {
    async fn send_to_agent(
        &self,
        agent_id: &AgentId,
        msg: AgentMessage,
    ) -> Result<()> {
        // 1. In-process agents first (fastest path — channel send)
        if let Some(sender) = self.local_agents.get(agent_id) {
            return sender.send(msg).await.map_err(Into::into);
        }

        // 2. Direct-reachable agents (HTTP — low latency)
        if let Some(url) = self.deployment_urls.get(agent_id) {
            return self.http_client
                .post(format!("{url}/api/message"))
                .json(&msg)
                .send()
                .await
                .map_err(Into::into);
        }

        // 3. Fall back to relay (works for NAT traversal)
        self.relay.send(agent_id, msg).await
    }
}
```

### Routing priority

| Priority | Path | Latency | Use case |
|---|---|---|---|
| 1 | In-process (mpsc) | ~0 | Default local Agents |
| 2 | Direct HTTP | ~10-50ms | Remote Agents with public URLs |
| 3 | Relay-forwarded | ~50-200ms | NAT-traversed Agents, no public URL |

---

## 13. Multi-Instance Handling

Each roko instance connects to the relay with a unique `instance_id` (generated at startup, format: `inst_{ulid}`).

### Conflict resolution

If two roko instances claim the same `agent_id`, the relay uses **last-write-wins**. The most recent connection becomes authoritative. The old connection receives a supersession notice:

```json
{ "type": "superseded", "agent_id": "coder-1", "by": "inst_01HZ3X9K2M..." }
```

On receiving `superseded`, the old instance must stop publishing events and heartbeats for that Agent. It can continue operating other Agents that are not in conflict.

### Typical scenario

A developer restarts their roko process. The new process connects before the old WebSocket times out. The relay transfers ownership to the new connection immediately rather than waiting for the old one to disconnect.

---

## 14. Feed Discovery and Subscription

### Feed registry

The relay maintains a feed registry. Dashboards and Agents discover Feeds dynamically.

```
GET /relay/feeds
-> [
    {
      "feed_id": "eth-mainnet-blocks",
      "agent_id": "chain-watcher-1",
      "kind": "raw",
      "schema": "eth_block",
      "rate_hz": 0.08,
      "access": "public",
      "subscribers": 3
    }
  ]
```

### Pagination

`GET /relay/feeds` supports cursor-based pagination and filtering:

| Parameter | Type | Default | Description |
|---|---|---|---|
| `limit` | u32 | 50 | Results per page (max 200) |
| `cursor` | string | (none) | Opaque cursor from previous response |
| `kind` | string | (none) | Filter: "raw" or "derived" |
| `access` | string | (none) | Filter: "public" or "paid" |
| `agent_id` | string | (none) | Filter to feeds from a specific Agent |
| `schema` | string | (none) | Filter by schema name (exact match) |
| `search` | string | (none) | Full-text search across feed_id and description |

Cursors are opaque base64-encoded JSON. When `next_cursor` is `null`, there are no more results.

### Subscribing to a Feed

```typescript
// Dashboard subscribes to an Agent's Feed
function useFeed(agentId: string, feedId: string) {
  const [data, setData] = useState<Signal[]>([]);

  useEffect(() => {
    const ws = new WebSocket(`${relayUrl}/relay/ws`);
    ws.onopen = () => {
      ws.send(JSON.stringify({
        type: "subscribe",
        rooms: [`agent:${agentId}:feed:${feedId}`]
      }));
    };
    ws.onmessage = (e) => {
      const event = JSON.parse(e.data);
      if (event.type === "feed_data") {
        setData(prev => [...prev.slice(-999), event.payload]);
      }
    };
    return () => ws.close();
  }, [agentId, feedId]);

  return data;
}
```

### Agent-to-agent Feed subscription

Agents can subscribe to each other's Feeds, creating a data marketplace:

```rust
// Agent B subscribes to Agent A's derived Feed
let subscription = ctx.relay.subscribe_feed(SubscribeFeedRequest {
    feed_id: "eth-gas-trend",
    source_agent_id: "chain-watcher-1",
})?;

// For paid Feeds, payment is handled automatically via the inference gateway's
// cost tracking. The subscribing Agent's budget is debited per hour.
```

### Dynamic Feed registration

Agents can register new Feeds at runtime. When an Agent discovers a new data source or creates a derived Feed, it announces it to the relay:

```rust
ctx.relay.register_feed(FeedRegistration {
    feed_id: format!("dex-{}-swaps", dex_address),
    agent_id: ctx.agent_id.clone(),
    kind: FeedKind::Derived,
    schema: FeedSchema::Custom("dex_swap_v1"),
    rate_hz: 2.0,
    access: FeedAccess::Public,
})?;
```

The dashboard discovers this Feed dynamically because it subscribes to the "system" room and receives `feed_registered` events.

---

## 15. TOML Configuration

### Relay configuration

```toml
[relay]
url = "wss://relay.nunchi.dev"
workspace_name = "will-dev"
reconnect_interval_ms = 5000
ring_buffer_size = 65536        # 64K entries for reconnection replay
```

### Connector configuration

```toml
[[agents]]
name = "chain-watcher"
connectors = [
  { name = "ethereum-rpc", kind = "chain_rpc", url = "wss://eth.llamarpc.com" },
  { name = "base-rpc", kind = "chain_rpc", url = "wss://base.llamarpc.com" },
  { name = "postgres", kind = "database", url = "postgres://localhost/roko" },
]

# MCP servers auto-register as Connectors
[agent.mcp_config]
servers = [
  { name = "github", command = "roko-mcp-github" },
  { name = "code-intel", command = "roko-mcp-code" },
]
```

### Remote agent configuration

```toml
[[remote_agents]]
name = "staging-monitor"
url = "https://staging-monitor.fly.dev"
auth_token_ref = "secrets.staging_monitor_token"

[[remote_agents]]
name = "prod-watcher"
url = "https://prod-watcher.railway.app"
auth_token_ref = "secrets.prod_watcher_token"
```

### Feed configuration

```toml
[feeds]
default_sample_rate_hz = 2.0
max_feeds_per_agent = 50
```

---

## 16. Acceptance Criteria

| Criterion | Verification |
|---|---|
| Connect protocol compiles with all 5 methods | `cargo check` on Connector trait |
| Config-based Connector discovery loads declared Connectors | Integration test: declare Connector in config, verify loaded |
| MCP auto-registration creates McpConnector per server | Integration test: configure MCP server, verify McpConnector registered |
| Extension-provided Connector registered via on_init | Integration test: Extension registers Connector, verify available |
| Connector health check runs at configured interval | Integration test: verify periodic health calls |
| Connector disconnect called on Agent shutdown | Integration test: shutdown Agent, verify disconnect called |
| Feed registration publishes to relay | Integration test: register Feed, query relay, verify listed |
| Feed subscription delivers Signals via Bus topic | Integration test: register Feed, subscribe, verify data flow |
| Workspace hello registers with relay on startup | Integration test: start `roko serve`, verify relay has workspace |
| Dashboard auto-connects by owner_wallet | Integration test: match wallet, verify auto-select |
| WebSocket message envelope has seq, ts, room, type, payload | Unit test: serialize message, verify all fields present |
| Coalesce strategy buffers heartbeats to 500ms | Integration test: send 10 heartbeats in 500ms, verify 1 delivered |
| Drop-oldest strategy uses ring buffer for output_chunk | Integration test: overflow buffer, verify old chunks dropped |
| Lossless strategy queues gate_result events | Integration test: slow consumer, verify all events delivered |
| Reconnection with resume replays from last_seq | Integration test: disconnect, reconnect with seq, verify replay |
| Reconnection gap > 64K sends snapshot | Integration test: large gap, verify snapshot received |
| In-process Agent routing via mpsc (priority 1) | Unit test: local Agent, verify channel send |
| Direct HTTP routing for remote Agents (priority 2) | Integration test: remote Agent with URL, verify HTTP post |
| Relay fallback routing (priority 3) | Integration test: NAT Agent, verify relay forwarding |
| Supersession notice on duplicate agent_id | Integration test: two instances same agent_id, verify superseded |
| Feed pagination returns cursor-based results | Integration test: 100 Feeds, paginate with limit=20, verify 5 pages |
| Agent discovery merges relay + chain + deployment list | Integration test: populate all three, verify merged view |

---

## 17. Crate Mapping

| Crate | Responsibility |
|---|---|
| `roko-core` | Connect protocol trait, ConnectorKind, QueryRequest/Response, HealthStatus |
| `roko-agent` | Connector discovery, lifecycle management, MCP auto-registration |
| `roko-runtime` | Bus (Signal pub/sub), relay WebSocket client, reconnection protocol |
| `roko-serve` | HTTP control plane, WebSocket server, workspace registration |
| `roko-agent-server` | Per-agent sidecar, direct HTTP for remote Agents |
| `roko-chain` | ChainRpcConnector, on-chain feed sources |
| `roko-cli` | Connector and relay configuration in roko.toml |
