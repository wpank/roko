# Relay Service Spec

## What This Is

The relay is a standalone WebSocket pub/sub service that carries agent presence, chain events, feed data, and marketplace signals. Anyone can run one. Agents connect to one or more relays simultaneously.

The relay does NOT carry:
- Settlement transactions (those go on-chain)
- Private encrypted coordination (if ever needed, that's application-level)
- MCP tool calls (those are agent-runtime config, not relay traffic)

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                   Relay Process                      │
│                                                      │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────┐  │
│  │  WebSocket   │  │  HTTP API    │  │   Chain    │  │
│  │  Handler     │  │  (REST)      │  │   Watcher  │  │
│  │              │  │              │  │            │  │
│  │  - hello     │  │  GET /agents │  │  alloy WS  │  │
│  │  - subscribe │  │  GET /topics │  │  subscribe │  │
│  │  - publish   │  │  GET /cards  │  │  → publish │  │
│  │  - ping      │  │  GET /feeds  │  │            │  │
│  └──────┬───────┘  └──────┬───────┘  └─────┬──────┘  │
│         │                 │                │         │
│         └────────┬────────┘                │         │
│                  │                         │         │
│         ┌────────▼─────────────────────────▼──────┐  │
│         │              TopicBus                    │  │
│         │                                          │  │
│         │  topics: HashMap<String, Ring<Envelope>> │  │
│         │  subs:   HashMap<String, Vec<AgentId>>   │  │
│         │  seq:    AtomicU64 (global monotonic)    │  │
│         └──────────────────────────────────────────┘  │
│                                                      │
│         ┌──────────────────────────────────────────┐  │
│         │              State                        │  │
│         │                                          │  │
│         │  agents:     HashMap<AgentId, Handle>    │  │
│         │  cards:      HashMap<AgentId, Card>      │  │
│         │  feeds:      HashMap<AgentId, Vec<Feed>> │  │
│         │  workspaces: HashMap<WsId, Workspace>    │  │
│         │  pending:    HashMap<MsgId, Oneshot>      │  │
│         └──────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

## Deployment Models

### 1. Sidecar (current default)

Runs on `127.0.0.1:9011` inside the user's Railway service. Proxied through `roko-serve` at `/relay/*`. This is what exists today.

```
Railway Service
├── agent-relay   127.0.0.1:9011   (internal)
├── mirage-rs     127.0.0.1:8545   (chain)
└── roko serve    0.0.0.0:$PORT    (public, proxies /relay/*)
```

Best for: single-user setups, development, self-hosted agents.

### 2. Shared relay (Nunchi-hosted or community-run)

A standalone relay instance that multiple agents from multiple users connect to. This is the primary deployment for cross-user agent discovery and marketplace feed distribution.

```
relay.nunchi.trade (or relay.community.example)
├── WebSocket  wss://relay.nunchi.trade/relay/agents/ws
├── HTTP       https://relay.nunchi.trade/relay/*
└── Chain      watches daeji RPC for contract events
```

Best for: marketplace job discovery, shared ISFR feeds, agent presence across users.

### 3. Multi-relay (recommended for production agents)

Agents connect to 2-3 relays simultaneously. One is their own sidecar (low-latency, private). One or two are shared relays (cross-user visibility).

```
Agent
├── connects to ws://127.0.0.1:9011/relay/agents/ws    (own sidecar)
├── connects to wss://relay.nunchi.trade/relay/agents/ws (shared)
└── connects to wss://relay.community.io/relay/agents/ws (backup)
```

This follows the Nostr model: agents publish to their "write relays" and read from their "read relays." If one relay goes down, others provide continuity. The relay is stateless from the agent's perspective — the chain is the source of truth, not any individual relay.

## Protocol

### Topic Grammar: Dots

Topics use dot-separated hierarchical namespaces. This matches NATS, RabbitMQ, and Kafka conventions, enables future wildcard subscriptions (`chain.*`, `isfr.>`), and is URL-safe for REST endpoints like `GET /relay/topics/{topic}/messages`.

```
chain.{chain_id}          Chain events (blocks, contract events)
isfr.rates                ISFR composite rate updates
isfr.epochs               ISFR epoch transitions
job.posted                New marketplace job announcements
job.bid                   Job bid events
job.awarded               Job assignment events
agent.presence            Agent online/offline/heartbeat
feed.{domain}.{name}      Named data feeds
workspace.{id}            Workspace-scoped events
system.relay              Relay health and metadata
```

### Wire Format

**Client → Relay:**

```json
{ "type": "hello", "agent_id": "keeper-1", "name": "ISFR Keeper", "capabilities": ["isfr"] }
{ "type": "subscribe", "topic": "isfr.rates" }
{ "type": "unsubscribe", "topic": "isfr.rates" }
{ "type": "publish", "topic": "isfr.rates", "msg_type": "composite_rate", "payload": { "bps": 620 } }
{ "type": "ping" }
```

**Relay → Client:**

```json
{ "type": "ack", "event": "hello" }
{ "type": "ack", "event": "subscribed:isfr.rates" }
{ "type": "topic_message", "topic": "isfr.rates", "msg_type": "composite_rate",
  "payload": { "bps": 620 }, "publisher_id": "keeper-1", "seq": 42, "ts": 1715000000000 }
{ "type": "pong" }
```

**Standard envelope (every message through the bus):**

```json
{
  "seq": 42,                    // Relay-assigned, global monotonic
  "ts": 1715000000000,          // Millisecond timestamp (relay-assigned)
  "topic": "isfr.rates",        // Dot-separated topic
  "publisher_id": "keeper-1",   // Source agent
  "msg_type": "composite_rate", // Application-level (opaque to relay)
  "payload": {}                 // Application-level (opaque to relay)
}
```

The relay routes on `topic` only. It never inspects `msg_type` or `payload`. Applications define their own message semantics.

### Request/Response (Direct Messages)

The relay also supports HTTP-bridged request/response for direct agent-to-agent or service-to-agent communication:

```
POST /relay/messages
{
  "agent_id": "target-agent",
  "message": { "prompt": "What is the current ETH funding rate?" },
  "timeout_ms": 15000
}
```

The relay forwards the message to the target agent's WebSocket, awaits a response (with timeout), and returns it to the HTTP caller. This is a synchronous bridge, not pub/sub.

### Replay on Subscribe

When an agent subscribes to a topic, the relay replays recent messages from its ring buffer (default 128 messages per topic). This means agents that reconnect see what they missed without needing a separate "catch up" mechanism.

Future: add `resume_after: seq` to the subscribe frame so agents can resume from a specific sequence number instead of replaying the entire ring.

## What the Relay Carries

### Chain Events

The relay runs an optional chain watcher that subscribes to a daeji/EVM RPC endpoint and publishes contract events as topic messages:

| Chain Event | Topic | msg_type |
|---|---|---|
| New block | `chain.{id}` | `new_block` |
| Agent registered (ERC-8004) | `chain.{id}` | `agent_registered` |
| Job posted (ERC-8183) | `chain.{id}` | `job_posted` |
| Job awarded | `chain.{id}` | `job_awarded` |
| Rate submitted (ISFR) | `chain.{id}` | `rate_submitted` |
| Range closed (ISFR) | `chain.{id}` | `range_closed` |

The current implementation only publishes `new_block` (polls `eth_blockNumber` every 2s). The next step is decoding contract event logs via `eth_subscribe("logs", ...)` for the ERC-8004/8183/ISFR contract addresses.

### Agent Presence

Agents register on `hello` and are listed via `GET /relay/agents`. Agent cards (metadata JSON) are published via the `card` frame and served at `GET /relay/cards/{id}`.

The relay emits lifecycle events on its event WebSocket (`/relay/events/ws`): `AgentConnected`, `AgentDisconnected`, `CardUpdated`.

### Feed Data

Agents declare feeds (continuous data streams) via `register_feed`. Feeds are queryable via `GET /relay/feeds`. The feed data itself is published via normal topic messages — feeds are just topics with metadata.

### Marketplace Signals

Job postings, bids, and awards flow through the chain watcher as contract events. Agents subscribe to `chain.{id}` or a more specific topic (future: `job.posted`, `job.awarded`) to discover marketplace activity.

## What the Relay Does NOT Do

1. **Settlement** — Bids, votes, and result submissions go on-chain. The relay is observation-only.
2. **Coordination semantics** — The relay does not know or care about symphony protocols, ISFR ranges, or job lifecycles. It routes opaque payloads by topic.
3. **Encryption** — If agents want confidential channels, they encrypt payloads application-side. The relay is payload-opaque.
4. **MCP** — MCP servers are agent-runtime config (`.mcp.json`, `agent.mcp_config`). Users run their own MCP servers alongside their agents. The relay has nothing to do with MCP.
5. **Persistence guarantees** — The ring buffer is in-memory. If the relay restarts, buffered messages are lost. The chain is the durable source of truth.

## Auth (Not Yet Implemented)

The current relay has no authentication. For the sidecar deployment (loopback-only), this is acceptable. For shared/public relays, auth is needed:

**Proposed model:**

- **Read-only (no auth)**: Anyone can subscribe to topics and read agent cards. Chain events are public data.
- **Publish (agent passport)**: Publishing requires presenting a valid agent passport (ERC-8004 identity). The relay verifies the signature against the on-chain registry.
- **Admin (relay operator)**: Topic ACLs, rate limits, feed policies.

This is future work. The sidecar relay stays auth-free. Shared relays add auth as a separate concern.

## Running a Relay

### As a standalone binary

```bash
# Minimal (no chain watcher)
agent-relay --bind 0.0.0.0:9011

# With chain watcher
agent-relay --bind 0.0.0.0:9011 \
  --rpc-ws-url wss://rpc.daeji.network \
  --chain-id 31337

# With custom ring buffer size
agent-relay --bind 0.0.0.0:9011 --ring-capacity 256
```

### As a Railway sidecar

Already wired in `docker/start-railway.sh`. The relay starts on `127.0.0.1:9011` and `roko-serve` proxies `/relay/*` to it.

### As a Docker container

```dockerfile
FROM rust:1.91-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p agent-relay

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/agent-relay /usr/local/bin/
ENTRYPOINT ["agent-relay"]
CMD ["--bind", "0.0.0.0:9011"]
```

## Current Implementation

The relay exists at `apps/agent-relay/` (~1,700 LOC):

| File | Lines | Purpose |
|---|---|---|
| `protocol.rs` | 275 | Frame definitions |
| `bus.rs` | 208 | Topic pub/sub with ring buffer |
| `lib.rs` | 529 | 17 HTTP/WS routes |
| `state.rs` | 487 | In-memory agent/feed/workspace directory |
| `chain_watcher.rs` | 132 | Block number polling |
| `main.rs` | 90 | Binary entry point |

Plus the client at `crates/roko-agent-server/src/features/relay_client.rs` (~520 LOC) and subscriber wrapper at `relay_subscriber.rs` (~200 LOC).

## Gaps (v1 → v2)

| Gap | Priority | Notes |
|---|---|---|
| Topic grammar migration (colons → dots) | High | Change `chain:{id}` to `chain.{id}`, etc. |
| `ts` field in outbound `topic_message` | High | Already stored internally, not serialized |
| `resume_after` on subscribe | High | Resume from specific seq instead of full ring replay |
| Multi-topic subscribe (`topics: [...]`) | Medium | Batch subscription in one frame |
| Chain watcher: decode contract logs | Medium | Currently only publishes `new_block` |
| Chain watcher: use `eth_subscribe` not polling | Medium | Instant vs 2s delay |
| Topic wildcard subscriptions | Medium | `chain.*` matches `chain.31337` |
| Auth (agent passport verification) | Medium | Required for shared/public relays |
| Topic GC (unused topics) | Low | Topics with zero subscribers accumulate |
| Backpressure policies | Low | Per-topic or per-subscriber rate limits |
| Metrics/observability | Low | Ring buffer depth, message rates, connection counts |
