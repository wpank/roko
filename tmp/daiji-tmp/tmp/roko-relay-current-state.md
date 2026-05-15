# Roko Relay: What Actually Exists Today

## Architecture

The roko relay (`apps/agent-relay/`, ~350 lines) is a **flat, stateless, request/response message broker**. It is not a pub/sub system.

### What It Has

- **Request/response forwarding** — client POSTs to `/relay/messages` with `{ agent_id, message, timeout_ms }`, relay forwards to agent's WebSocket, agent responds, relay returns HTTP response. 15s default timeout, 60s max.
- **Agent directory** — `GET /relay/agents` returns list of all connected agents with name, capabilities, rest_endpoint, card_uri, connected_at_ms.
- **Card hosting** — agents send card JSON in a `Card` frame, relay stores in-memory, serves at `GET /relay/cards/{id}`.
- **Event broadcast** — dashboard-only WebSocket at `/relay/events/ws` for lifecycle events (AgentConnected, AgentDisconnected, CardUpdated, MessageDelivered, MessageResponded). Not for agent-to-agent messaging.
- **Workspace registration** — roko instances register via `POST /relay/workspaces/register` for multi-instance discovery. 30s heartbeat, 60s stale expiry.

### What It Does NOT Have

- **No topics, no rooms, no pub/sub** — single flat namespace. All agents equally visible. No subscribe/publish pattern.
- **No feeds** — `FeedRegistry` exists in `roko-core` as HTTP metadata CRUD but has no data streaming. Just register/list/get/delete of `FeedInfo` structs.
- **No ring buffer, no resume** — reconnection loses all state. No sequence numbers. No replay.
- **No chain awareness** — relay knows nothing about daeji chain, contracts, or events.
- **No agent-to-agent broadcast** — messages go client→relay→one agent. No fan-out.

### WebSocket Protocol

```
Agent → Relay:
  Hello { agent_id, name, capabilities, rest_endpoint, card, card_uri, metadata }
  Card { card, card_uri }
  Response { message_id, response }
  Error { message_id, error }
  Ping

Relay → Agent:
  Ack { event }           — acknowledges hello/card
  Message { message_id, message }  — forwarded request to handle
  Error { message_id, error }
  Pong
```

Handshake: agent connects → sends Hello → gets Ack → sends Card → gets Ack → enters message loop.

### State Model

All in-memory, no persistence:

```rust
struct RelayStateInner {
    agents: HashMap<String, ConnectedAgentHandle>,   // agent_id → WebSocket handle
    cards: HashMap<String, Value>,                    // agent_id → card JSON
    pending: HashMap<String, PendingResponse>,        // message_id → oneshot channel
    workspaces: HashMap<String, ConnectedWorkspace>,  // workspace_id → metadata
}
```

### Concurrency

- `parking_lot::RwLock` for shared state
- Per-agent: unbounded `mpsc::UnboundedSender` for outbound frames, split sink/stream
- `tokio::sync::broadcast` (capacity 256) for event stream to dashboards
- `tokio::sync::oneshot` for per-message request/response correlation

### Source Files

- `/Users/will/dev/nunchi/roko/roko/apps/agent-relay/src/lib.rs` — routes + WebSocket handler (~346 lines)
- `/Users/will/dev/nunchi/roko/roko/apps/agent-relay/src/main.rs` — server startup
- `/Users/will/dev/nunchi/roko/roko/apps/agent-relay/src/protocol.rs` — frame types (~182 lines)
- `/Users/will/dev/nunchi/roko/roko/apps/agent-relay/src/state.rs` — state management
- `/Users/will/dev/nunchi/roko/roko/apps/agent-relay/tests/integration.rs` — integration tests

### Agent Client Side

`roko-agent-server/src/features/relay_client.rs`:
- `RelayClientConfig` wrapping relay URL
- `connect(config, agent_state, agent_card)` — establishes outbound WebSocket
- Sends Hello + Card on connect
- Handles inbound Message frames by dispatching to agent's prompt handler
- Returns Response frame with result
- No subscribe/publish capability

## PulseBus: Built But Not Connected to Relay

Roko has a full in-process pub/sub system that is NOT exposed over WebSocket:

### What Exists (in `roko-core` and `roko-runtime`)

- **Pulse type** (`roko-core/src/pulse.rs`, 507 lines) — `{ seq, topic, kind, body, created_at_ms, tags }`
- **Topic** — hierarchical string paths like `feed:eth-gas-trend:data`, `agent:coder-1:heartbeat`
- **TopicFilter** — Exact, Prefix, or All matching
- **PulseBus** (`roko-core/src/pulse_bus.rs`, 207 lines) — wraps EventBus, supports publish/subscribe/replay
- **EventBus** (`roko-runtime/src/event_bus.rs`, 300+ lines) — generic bus with bounded replay ring (VecDeque), tokio broadcast for live fan-out, monotonic sequence numbering
- **Bus backends** (`roko-core/src/bus_backends.rs`) — BroadcastBus (no replay), MemoryBus (replay ring), MultiBus (fan-out to multiple backends)

### What Uses It Today

- Gate verdict publishing (internal)
- Agent events (turn start/complete, costs)
- Plan revision events
- Cognitive signals

All in-process. None of it bridges to the relay WebSocket.

### FeedRegistry

`roko-core/src/feed.rs` (272 lines):
- In-memory `FeedRegistry` with CRUD operations
- `FeedInfo` struct: id, name, kind (Raw/Derived/Composite/Meta), access (Public/Private/Paid), agent_id, description, schema
- HTTP endpoints in `roko-serve/src/routes/feeds.rs`: GET/POST/DELETE `/api/feeds`
- **Metadata only** — no data streaming, no WebSocket integration

### Migration Note in Source

From `feed.rs` line 8-12:
```rust
//! **Migration note (Phase 1, §1.12):** Feeds will become Pulse streams on
//! the Bus, managed via the `Connect` + `Trigger` protocols defined in
//! `docs/v2/11-CONNECTIVITY.md`. The `FeedRegistry` is actively used by
//! `roko-serve` HTTP routes and will be migrated in M037. Do not add new
//! callers — prefer Bus-based Pulse streams once available.
```

### What's Missing

- `FeedPublisherExt` — extension that bridges in-process Bus to relay WebSocket (spec only)
- Relay-side subscribe/publish frames — relay has no concept of topics
- Connect/Trigger/Store protocols — spec only
- Bus-to-relay bridge — PulseBus is in-process, relay is separate process, no bridge

## Key Insight

The roko relay and PulseBus are two separate systems that don't talk to each other. The relay handles agent presence + request/response. PulseBus handles in-process event streaming. A daeji-relay (or upgraded roko relay) would unify these by exposing topic pub/sub over WebSocket.
