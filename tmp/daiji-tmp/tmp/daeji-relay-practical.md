# Daeji-Relay: Practical Design Given Current State

## What Changes From the Earlier Design Docs

The pr24-review redesign docs assumed roko had more relay infrastructure than it does. Here's what's different given reality:

| Earlier assumption | Reality | Implication |
|--------------------|---------|-------------|
| Roko relay has room-based pub/sub | Flat request/response only | Daeji-relay builds pub/sub from scratch (~150 lines for bus.rs) |
| PulseBus bridges to relay | PulseBus is in-process only | Bridge is a future roko-side addition (~50 lines), not blocking |
| Feed infrastructure exists | Metadata registry exists, no streaming | Feeds = topics with metadata, ~90 lines on relay side |
| Groups need complex state | Groups = topic partitions + chain events | ~100 lines for auto-create/close from chain events (Phase 2) |
| ~1,150 lines total | Bus + chain watcher alone is ~600 | ~600 for Phase 1, ~750 with feeds, ~850 with groups |

## Current-State Daeji-Relay (Phase 1, No Feeds)

The minimum useful relay. Improves on both PR #24 and roko's relay:

```
daeji-relay/
├── src/
│   ├── main.rs            # ~30 lines  — CLI args, start server
│   ├── server.rs          # ~180 lines — axum routes + WebSocket handler
│   ├── protocol.rs        # ~80 lines  — Frame types (hello, subscribe, publish, envelope)
│   ├── bus.rs             # ~150 lines — Topic pub/sub + ring buffer (per-connection)
│   ├── state.rs           # ~60 lines  — Agent registry + connection state
│   └── chain.rs           # ~100 lines — ERC-8004/8183 event watcher → publishes on bus
└── Cargo.toml             # axum, tokio, serde_json, alloy, dashmap
```

**~600 lines total.**

### What It Does

1. **Topic pub/sub** — agents subscribe to topics, publish to topics, relay routes by topic name. The single biggest addition over roko's relay.

2. **Ring buffer + resume** — each connection gets a ring buffer. On reconnect, client sends `resume_after: seq` and relay replays missed messages. Roko's `EventBus<E>` already implements this pattern in-process; this exposes it over WebSocket.

3. **Standard envelope** — `{ seq, ts, topic, from, type, payload }`. Relay routes on `topic`, ignores `type` and `payload` (opaque to relay). Applications define their own message semantics.

4. **Chain watcher** — subscribes to ERC-8004/8183 contract events via alloy WS provider, publishes as envelopes on `chain:nunchi` topic. Agents get chain events without running their own watcher.

5. **Agent directory** — same as roko's relay but enriched with chain identity. If an agent is registered on-chain via ERC-8004, their relay presence gets annotated with chain data.

6. **Request/response preserved** — keep roko's POST `/messages` pattern for direct agent-to-agent RPC. This coexists with pub/sub.

### What It Deliberately Doesn't Have

- No feed directory/registry (roko already has this as HTTP CRUD)
- No group lifecycle management
- No coordination mode awareness
- No AEAD/encryption (application-level concern)
- No payment gating

### Wire Protocol

**Client → Relay:**
```json
{ "type": "hello", "agent_id": "keeper-1", "resume_after": 0 }
{ "type": "subscribe", "topics": ["chain:nunchi", "feed:isfr:rates"] }
{ "type": "publish", "topic": "feed:isfr:rates", "payload": { ... } }
{ "type": "direct", "to": "agent-2", "payload": { ... } }
{ "type": "ping" }
```

**Relay → Client:**
```json
{ "type": "welcome", "seq": 12345 }
{ "type": "envelope", "seq": 12346, "ts": 1713960000, "topic": "feed:isfr:rates",
  "from": "keeper-1", "payload": { ... } }
{ "type": "gap", "from_seq": 100, "to_seq": 500 }
{ "type": "pong" }
```

**Standard envelope (every message through the bus):**
```json
{
  "seq": 12346,           // Relay-assigned, monotonic per connection
  "ts": 1713960000000,    // Millisecond timestamp
  "topic": "string",      // Topic name
  "from": "agent-id",     // Publisher
  "type": "string",       // Application-level (opaque to relay)
  "payload": { }          // Application-level (opaque to relay)
}
```

### Topic Hierarchy

```
system                      Agent lifecycle, relay health
agent:{id}                  Per-agent presence
feed:{id}:data              Data streams (ISFR rates, price feeds, etc.)
group:{id}                  Group broadcast
group:{id}:*                Group sub-channels
chain:{chain_id}            Chain events (ERC-8004/8183)
```

Topics are dynamic — subscribing creates the topic if it doesn't exist.

### How bus.rs Works

```rust
// Core data structure
struct Bus {
    // topic → set of subscriber connection IDs
    subscriptions: DashMap<String, HashSet<ConnId>>,
    // connection ID → (sender, ring buffer)
    connections: DashMap<ConnId, ConnectionState>,
}

struct ConnectionState {
    sender: mpsc::UnboundedSender<Envelope>,
    ring: VecDeque<Envelope>,  // bounded, e.g., 64K entries
    seq: AtomicU64,            // per-connection sequence counter
}

impl Bus {
    fn subscribe(&self, conn: ConnId, topic: &str);
    fn unsubscribe(&self, conn: ConnId, topic: &str);
    fn publish(&self, topic: &str, from: &str, msg_type: &str, payload: Value) -> u64;
    fn resume(&self, conn: ConnId, after_seq: u64) -> Vec<Envelope>;
}
```

This is ~150 lines. The pattern is identical to roko's `EventBus<E>` but keyed by topic and exposed over WebSocket instead of in-process channels.

## Adding Feeds (~90 additional lines)

Feeds on the relay are just **topics with a registered producer and metadata**:

| What's needed | Lines | Notes |
|---------------|-------|-------|
| Feed = a topic with `feed:` prefix | 0 | Already works — topic pub/sub handles it |
| `GET /feeds` endpoint | ~40 | Query topics with `feed:` prefix, return metadata |
| `POST /feeds` (register metadata) | ~30 | Store name, kind, schema, producer agent_id |
| FeedMeta struct | ~20 | Can reuse roko-core's FeedInfo directly |
| **Total** | **~90** | |

The honest answer: feeds are ~90 lines on top of topic pub/sub. They're just topics with metadata. The real work is the pub/sub infrastructure, not feeds specifically.

## Adding Groups (~100 additional lines)

Groups are **topic partitions auto-managed by chain events**:

| What's needed | Lines | Notes |
|---------------|-------|-------|
| Group struct (id, members, state) | ~20 | In-memory registry |
| Auto-create on JobFunded chain event | ~30 | Chain watcher creates `group:job-{id}:*` topics |
| Auto-close on terminal chain event | ~20 | Chain watcher removes group topics |
| `GET /groups`, `GET /groups/{id}` | ~30 | HTTP discovery |
| **Total** | **~100** | |

## Build Phases

### Phase 1: Pub/sub relay with chain events (~600 lines)

| Module | Lines | What |
|--------|-------|------|
| bus.rs | ~150 | Topic pub/sub + ring buffer |
| server.rs | ~180 | axum routes + WebSocket handler with subscribe/publish |
| protocol.rs | ~80 | Frame types and envelope |
| chain.rs | ~100 | ERC-8004/8183 watcher → publishes to bus |
| state.rs | ~60 | Agent registry + connection state |
| main.rs | ~30 | CLI + server startup |

This alone replaces PR #24 entirely and gives any-language WebSocket + topic messaging + chain event delivery.

### Phase 2: Feeds + groups (+~190 lines, total ~790)

- Feed metadata registry + HTTP endpoints (~90 lines)
- Group auto-create/close from chain events (~100 lines)

### Phase 3: Roko integration (+~100 lines on roko side)

- Add subscribe/publish to roko's relay_client.rs (~50 lines)
- FeedPublisherExt bridging in-process PulseBus to relay (~50 lines)

## Roko Relay vs Daeji-Relay: Relationship

They serve different purposes today:

| | Roko relay | Daeji relay |
|---|---|---|
| Pattern | Request/response (RPC) | Pub/sub (event streaming) |
| Use case | "Send task to agent, get response" | "Broadcast rate to all subscribers" |
| State | Agent directory + cards | Agent directory + topic subscriptions + ring buffers |
| Chain awareness | None | Built-in chain watcher |
| Language | Rust (server), any (client) | Same |

### Options

**Option A: Coexist.** Roko relay handles RPC, daeji-relay handles pub/sub. Agents connect to both. Simple but two processes.

**Option B: Daeji-relay absorbs roko's relay.** Add request/response forwarding to daeji-relay (~50 lines — `direct` frame + pending response tracking). One relay, one connection per agent. This is the natural end state.

**Option C: Upgrade roko's relay in-place.** Add bus.rs + chain.rs to the existing roko relay. Same result as Option B but keeps the code in roko's repo. Daeji-specific parts (chain watcher) could be a feature flag.

Option B or C are equivalent — the question is just which repo owns it.
