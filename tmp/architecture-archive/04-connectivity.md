# Connectivity and relay

> Part of the [Roko Architecture Specification](00-INDEX.md). Extracted from v2 redesign doc.
> Merges the "Data flow: subscription-only" and "Agent connectivity" sections.

---

## Workspace discovery

Roko instances register with the relay on startup. Dashboards discover available workspaces automatically — no manual URL entry.

### How it works

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

The relay maintains a workspace directory alongside the agent directory. Dashboards query it:

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

The relay also pushes workspace events on the events WebSocket:

```json
{"type": "workspace_connected", "workspace_id": "ws-a1b2c3", "url": "https://..."}
{"type": "workspace_disconnected", "workspace_id": "ws-a1b2c3"}
```

### Dashboard connection flow

1. Dashboard loads, connects to relay
2. Fetches `GET /relay/workspaces` — lists all online roko instances
3. If user has a Privy wallet, auto-matches workspaces by `owner_wallet`
4. If exactly one match: auto-connect (zero friction)
5. If multiple matches: show picker ("You have 2 workspaces online — which one?")
6. If no match: show global-only view (agents from relay, chain data, no workspace features)
7. User can also manually add a workspace URL in Settings (for instances not registered with relay)

### Local development

`roko serve` on localhost registers with the relay if `relay.url` is configured. For pure local dev (no relay), the dashboard falls back to `VITE_ROKO_API_URL` env var or `localhost:6677`.

```toml
# roko.toml — relay registration
[relay]
url = "wss://relay.nunchi.dev"
workspace_name = "will-dev"
```

If `[relay]` is not configured, roko serves HTTP only — no relay registration, no auto-discovery. Dashboard must be pointed at it manually.

---

## Data flow: subscription-only

Every piece of data flows through WebSocket subscriptions. No polling.

### Event sources

```
Source              Transport           What it carries
──────              ─────────           ───────────────
Relay               WS /relay/ws        Agent presence, message lifecycle,
                                        relay health
roko-serve          WS /ws              Plan progress, gate results, episodes,
                                        learning metrics, job updates
Agent (direct)      WS (per-agent)      Heartbeats, streaming LLM output,
                                        decision traces
Agent (via relay)   WS /relay/ws        Same as direct, tunneled through relay
Chain               WS (RPC sub)        Blocks, contract events, ERC-8004
                                        registry updates
Agent chain feeds   WS (per-feed)       Raw RPC data, derived indicators,
                                        signals, analysis
```

### Subscription lifecycle

The dashboard subscribes when a page mounts and unsubscribes when it unmounts.

```typescript
// React hook pattern
function useAgentFeed(agentId: string) {
  const [state, setState] = useState<AgentState | null>(null);

  useEffect(() => {
    const ws = new WebSocket(`${relayUrl}/relay/ws`);

    ws.onopen = () => {
      // Subscribe to this agent's room
      ws.send(JSON.stringify({
        type: "subscribe",
        rooms: [`agent:${agentId}`, `agent:${agentId}:heartbeat`]
      }));
    };

    ws.onmessage = (e) => {
      const event = JSON.parse(e.data);
      setState(prev => applyEvent(prev, event));
    };

    return () => {
      ws.send(JSON.stringify({
        type: "unsubscribe",
        rooms: [`agent:${agentId}`, `agent:${agentId}:heartbeat`]
      }));
      ws.close();
    };
  }, [agentId]);

  return state;
}
```

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
|-------|------|---------|
| `seq` | `u64` | Monotonic sequence number per connection. Enables reconnection replay. |
| `ts` | `u64` | Unix milliseconds. Server clock. |
| `room` | `string` | Scoping. Clients subscribe to rooms, receive only matching messages. |
| `type` | `string` | Event discriminant. One of the types listed below. |
| `payload` | `object` | Type-specific data. |

### Room naming convention

```
agent:{id}                  Agent lifecycle events (spawn, stop, error)
agent:{id}:heartbeat        Heartbeat ticks (T0/T1/T2, cortical state)
agent:{id}:output           Streaming LLM output chunks
agent:{id}:trace            Decision traces per tick
agent:{id}:feed:{feed_id}   Chain data feeds exposed by the agent
plan:{id}                   Plan progress, task transitions, gate results
cluster:{id}                Cluster pipeline events
system                      Server health, provider status, cost updates
learning                    Experiment results, router updates, thresholds
```

### Event types

```
Type                    Room pattern            Payload
────                    ────────────            ───────
presence_join           system                  { agent_id, mode, profile }
presence_leave          system                  { agent_id, reason }
heartbeat               agent:{id}:heartbeat    { tick, tier, pe, cortical_state }
output_chunk            agent:{id}:output       { content, done, usage }
trace                   agent:{id}:trace        { tick, steps[], gate_result }
task_started            plan:{id}               { task_id, phase }
task_completed          plan:{id}               { task_id, outcome }
gate_result             plan:{id}               { task_id, gate, rung, passed }
phase_transition        plan:{id}               { from, to }
feed_data               agent:{id}:feed:{fid}   { feed_id, data }
feed_registered         system                  { agent_id, feed_id, schema }
cost_update             system                  { agent_id, delta, total }
provider_status         system                  { provider, healthy, latency_ms }
experiment_result       learning                { experiment_id, winner, p_value }
router_update           learning                { model, weight, reason }
```

### Backpressure and coalescing

High-frequency events (heartbeats at 100ms, chain blocks at 2s) need throttling for dashboard consumption.

```
Strategy                Applies to              Behavior
────────                ──────────              ────────
Coalesce                heartbeat               Relay buffers heartbeats per agent,
                                                sends latest every 500ms to
                                                dashboard subscribers
Drop-oldest             output_chunk            Ring buffer per agent (1024 chunks).
                                                Slow consumers miss old chunks,
                                                catch up from latest.
Lossless                gate_result,            Queue with backpressure. If client
                        task_completed          can't keep up, relay applies
                                                TCP-level flow control.
Sample                  feed_data               Agent-configurable sample rate.
                                                Default: every Nth update where
                                                N = ceil(source_rate / 2Hz).
```

### Reconnection

Clients track the last received `seq`. On reconnect:

```json
{ "type": "resume", "last_seq": 4821 }
```

The relay replays missed events from its ring buffer (default: 64K entries, ~10 minutes at moderate throughput). If the gap exceeds the buffer, the relay sends a `snapshot` event with current state followed by live events.

### Reconnection recovery protocol

Full reconnection sequence:

```
Client                                  Relay
  │                                       │
  │──── WS connect ─────────────────────►│
  │                                       │
  │──── { "type": "resume",             │
  │       "last_seq": 4821 } ──────────►│
  │                                       │
  │                           ┌───────────┤
  │                           │ Check gap │
  │                           └───────────┤
  │                                       │
  │  Case 1: gap <= 64K entries           │
  │◄──── replay events 4822..4900 ────────│
  │◄──── live events continue ────────────│
  │                                       │
  │  Case 2: gap > 64K entries            │
  │◄──── { "type": "snapshot",           │
  │        "state": {                     │
  │          "agents": [...],             │
  │          "feeds": [...],              │
  │          "rooms": [...]               │
  │        }} ────────────────────────────│
  │◄──── live events continue ────────────│
  │                                       │
```

**Snapshot format**: The snapshot contains the minimum state needed to rebuild client-side views:

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

**Gap detection on the client**: Clients track the last received `seq` and check every incoming message for continuity. A gap (missing sequence numbers between the last received and the current message) means events were lost. On gap detection, the client should reconnect and send a `resume` message.

### Multi-instance handling

Each roko instance connects to the relay with a unique `instance_id` (generated at startup, format: `inst_{ulid}`).

**Conflict resolution**: If two roko instances claim the same `agent_id`, the relay uses last-write-wins. The most recent connection for that `agent_id` becomes authoritative. The old connection receives a supersession notice:

```json
{ "type": "superseded", "agent_id": "coder-1", "by": "inst_01HZ3X9K2M..." }
```

On receiving `superseded`, the old instance must stop publishing events and heartbeats for that agent. It can continue operating other agents that are not in conflict. This prevents ghost presence where two instances both claim an agent is online.

**Typical scenario**: A developer restarts their roko process. The new process connects before the old WebSocket times out. The relay transfers ownership to the new connection immediately rather than waiting for the old one to disconnect.

---

## Agent connectivity

Agents communicate across users and across machines. The relay is the rendezvous point -- any agent connected to the relay can discover and message any other agent, regardless of who owns them. There is no concept of "my agents" vs "their agents" at the protocol level. Ownership and access control are handled by auth (Privy JWT, wallet signatures, API keys), not by network isolation.

```
User A's roko process          Relay            User B's Fly Machine
┌──────────────┐                                ┌──────────────┐
│ agent-alpha  │──── WS ────►┌────────┐◄── WS ──│ agent-beta   │
│              │             │ Relay  │          │              │
│ Can message  │◄── relay ───│        │── relay─►│ Can message  │
│ agent-beta   │  forwarding │        │forwarding│ agent-alpha  │
└──────────────┘             │        │          └──────────────┘
                             │        │
User C's dashboard           │        │          User D's agent (local)
┌──────────────┐             │        │          ┌──────────────┐
│ Dashboard    │──── WS ────►│        │◄── WS ──│ agent-gamma  │
│ sees all 3   │             └────────┘          │ behind NAT   │
│ agents       │                                 └──────────────┘
└──────────────┘
```

Cross-user agent communication patterns:

- **Direct messaging**: Agent A sends a message to Agent B via relay. B receives it in its inbox, processes it during the next tick, and can respond.
- **Feed subscription**: Agent A subscribes to Agent B's feed (free or paid). Data flows B → relay → A continuously.
- **Pheromone signaling**: Agents deposit pheromones on-chain. Any agent can read them -- stigmergic coordination without explicit messaging.
- **Cluster collaboration**: Agents from different users can join the same cluster if authorized. The cluster pipeline orchestrates them together.
- **Knowledge sharing**: Agents publish knowledge to the InsightStore (on-chain). Any agent can query it regardless of owner.

Auth controls what an agent can do, not who it can talk to:

| Action | Auth required |
|--------|--------------|
| Discover agents on relay | None (public) |
| Read agent card / capabilities | None (public) |
| Send message to agent | Privy JWT or API key |
| Subscribe to free feed | None |
| Subscribe to paid feed | MPP session or x402 payment |
| Join a cluster | Cluster owner's invitation token |
| Read on-chain knowledge | None (public chain data) |
| Publish knowledge on-chain | Agent wallet signature |

### In-process agents

The default. Agents run as tokio tasks inside the roko process. Communication happens through channels.

```
┌──────────────────────────────────────────────────────────────┐
│                        roko process                          │
│                                                              │
│  ┌──────────────┐     mpsc          ┌──────────────────┐    │
│  │ Control      │ ◄──────────────── │ AgentRuntime     │    │
│  │ Plane        │ ────────────────► │ "coder-1"        │    │
│  │              │     mpsc          │                   │    │
│  │              │                   │ Extensions:       │    │
│  │ Routes msgs  │     mpsc          │  - GitExt         │    │
│  │ to agents    │ ◄──────────────── │  - CompilerExt    │    │
│  │ via channel  │ ────────────────► │  - TestRunnerExt  │    │
│  │ map          │     mpsc          │                   │    │
│  │              │                   └──────────────────┘    │
│  │  agent_id →  │                                           │
│  │  Sender      │     mpsc          ┌──────────────────┐    │
│  │              │ ◄──────────────── │ AgentRuntime     │    │
│  └──────────────┘ ────────────────► │ "research"       │    │
│                       mpsc          └──────────────────┘    │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐    │
│  │ Inference Gateway                                     │    │
│  │ (shared by all in-process agents via InferenceHandle) │    │
│  └──────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────┘
```

Benefits: zero serialization overhead, shared inference gateway, shared memory structures, no network latency.

### Remote agents

For isolation or NAT traversal, agents connect OUTBOUND to the relay. No inbound server required.

```
┌───────────────┐         ┌─────────────┐        ┌───────────────┐
│ Remote agent  │ ──WS──► │   Relay     │ ◄──WS──│ Control plane │
│ (Fly Machine) │         │             │        │ (roko-serve)  │
│               │         │ Routes msgs │        │               │
│ Connects out. │         │ by agent_id │        │ Routes msgs   │
│ No inbound    │         │             │        │ to relay for  │
│ ports needed. │         │             │        │ remote agents │
└───────────────┘         └─────────────┘        └───────────────┘
```

The relay acts as a message router. Both the agent and the control plane maintain persistent WebSocket connections to the relay. Messages are routed by agent ID.

Remote agent startup:

```bash
# On the Fly Machine / Railway container
roko agent run \
  --name "isolated-coder" \
  --relay wss://relay.nunchi.dev \
  --inference-proxy https://my-roko.up.railway.app/api/inference \
  --auth-token $AGENT_TOKEN
```

The agent:
1. Connects to the relay WebSocket
2. Announces presence with its agent ID and capabilities
3. Enters the standard `run()` loop
4. Routes inference requests to the parent's gateway via HTTPS proxy
5. Publishes heartbeats and events through the relay

### Direct-reachable agents

Some remote agents have public URLs (Railway services, dedicated VMs). These can receive messages directly via HTTP in addition to the relay path.

```toml
# User's local deployment list (stored in dashboard localStorage or roko.toml)
[[remote_agents]]
name = "staging-monitor"
url = "https://staging-monitor.fly.dev"
auth_token_ref = "secrets.staging_monitor_token"
```

The control plane prefers direct HTTP for request-response patterns (lower latency) and uses the relay for event streaming and presence.

### Agent discovery: three sources merged

```
┌─────────────────┐  ┌──────────────────┐  ┌────────────────────┐
│ Relay presence   │  │ ERC-8004 on-chain│  │ User's deployment  │
│                  │  │ registry         │  │ list               │
│ Who's online     │  │                  │  │                    │
│ right now.       │  │ Wallet address,  │  │ Railway/Fly URLs,  │
│                  │  │ reputation,      │  │ manually added     │
│ Source of truth  │  │ stake, caps,     │  │ endpoints.         │
│ for liveness.    │  │ feed adverts.    │  │                    │
│                  │  │                  │  │ Per-user. Stored   │
│ Always available.│  │ Source of truth   │  │ in localStorage.   │
│                  │  │ for identity +   │  │                    │
│                  │  │ feed discovery.  │  │ Always available.  │
│                  │  │                  │  │                    │
│                  │  │ Always available. │  │                    │
└────────┬────────┘  └────────┬─────────┘  └────────┬───────────┘
         │                    │                      │
         └────────────────────┼──────────────────────┘
                              ▼
                   ┌─────────────────────┐
                   │ Merged agent list   │
                   │                     │
                   │ Each agent has:     │
                   │ - id, name          │
                   │ - online (relay)    │
                   │ - reputation (chain)│
                   │ - endpoints (deploy)│
                   │ - capabilities      │
                   │ - mode, profile     │
                   │ - feeds (chain+relay│
                   └─────────────────────┘
```

The dashboard merges all three sources client-side. The relay provides liveness. The chain provides identity and reputation. The deployment list provides connectivity.

```typescript
interface MergedAgent {
  id: string;
  name: string;

  // From relay
  online: boolean;
  lastSeen: number;
  mode: "ephemeral" | "persistent" | "reactive";
  profile: string;

  // From chain (ERC-8004)
  wallet?: string;
  reputation?: number;
  stake?: bigint;
  tier?: "gray" | "copper" | "silver" | "gold" | "amber";
  capabilities?: string[];
  cardUri?: string;
  feeds?: FeedAdvertisement[];  // feeds registered in passport

  // From deployment list
  directUrl?: string;
  deployPlatform?: "fly" | "railway" | "manual";
}

interface FeedAdvertisement {
  feedId: string;
  schema: string;
  rateHz: number;
  access: "public" | { paid: { pricePerHour: number } };
  description: string;
}
```

### Message routing

The control plane routes messages based on agent location:

```rust
impl ControlPlane {
    async fn send_to_agent(&self, agent_id: &AgentId, msg: AgentMessage) -> Result<()> {
        // 1. Check in-process agents first (fastest path)
        if let Some(sender) = self.local_agents.get(agent_id) {
            return sender.send(msg).await.map_err(Into::into);
        }

        // 2. Check direct-reachable agents (HTTP)
        if let Some(url) = self.deployment_urls.get(agent_id) {
            return self.http_client
                .post(format!("{url}/api/message"))
                .json(&msg)
                .send()
                .await
                .map_err(Into::into);
        }

        // 3. Fall back to relay (works for NAT-traversal)
        self.relay.send(agent_id, msg).await
    }
}
```
