# Depth: Agent Mesh Sync

> Parent: [16-COORDINATION](../../16-COORDINATION.md) -- Section 4

---

## Overview

The Agent Mesh is Roko's peer-to-peer connectivity layer for Mesh-scope
pheromone propagation, collective knowledge sharing, and morphogenetic
coordination signals. In the two-fabric model it is not a separate trait
family: MeshBus carries Pulses and MeshSubstrate replicates Signals. It
provides transport between agents in a Collective without requiring a
centralized relay broker.

---

## Transport Mechanisms

Three co-equal transports:

| Transport | Technology | Latency | NAT Traversal | Encryption | Best For |
|-----------|-----------|---------|---------------|------------|----------|
| **WebSocket** | Standard WSS | ~50ms | N/A (outbound) | TLS 1.3 | Always-available relay, store-and-forward |
| **Iroh** | QUIC + ed25519 | ~10ms LAN, ~100ms WAN | Hole-punching + relay fallback | QUIC TLS 1.3 | Direct P2P, gossip pub/sub, blob transfer |
| **Chain** | On-chain tx | ~seconds | N/A | Chain consensus | Global-scope pheromone commits |

### WebSocket Relay

The relay server accepts persistent WSS connections from agents and routes
messages based on `CollectiveId`. It provides:

- **Fan-out**: Each message relayed to N-1 collective members
- **Store-and-forward**: Messages queued for offline agents (7-day TTL)
- **Sequence guarantees**: Per-agent monotonic sequence numbers

### Iroh Gossip

Iroh uses HyParView + PlumTree for epidemic broadcast:

- **O(log N) delivery**: Each agent forwards to O(log N) peers
- **Bounded fan-out**: Per-agent outgoing bandwidth bounded by
  `degree x R_message`
- **Eventual delivery**: Not total ordering, but sufficient for fuzzy
  pheromone signals

### When to Use Each

| Factor | WebSocket | Iroh | Chain |
|--------|-----------|------|-------|
| Latency | ~50ms | ~10-100ms | ~seconds |
| Offline support | Store-and-forward | Relay fallback | Always available |
| Bandwidth per node | O(N) | O(log N) | O(1) |
| Setup complexity | Low | Medium (key exchange) | High (chain account) |
| Best Collective size | 2-50 | 10-10,000 | All (global) |

---

## Sync Protocol

### Message Types

The Agent Mesh carries five message types, all encoded as canonical envelope
messages:

| Type | Priority | Trigger | Content |
|------|----------|---------|---------|
| `PheromoneDeposit` | High (Threat, Anomaly) or Batch (others) | New pheromone deposit at Mesh scope | Serialized pheromone Signal |
| `KnowledgeSync` | Batch | Curator cycle (every 50 ticks) | Delta of NeuroStore entries promoted to Mesh |
| `MorphogeneticBroadcast` | Batch | Curator cycle | Role vector + specialization index (72 bytes) |
| `NicheVacancy` | Immediate | Agent departure | Vacated role vector + specialist dimensions |
| `RoleConflict` | Immediate | Sustained overlap detection | Conflicting agent IDs + suggested respecialization |

### Sync Triggers

Three triggers govern when messages are sent:

1. **Event-driven (immediate)**: High-priority signals push instantly.
   - `Threat` pheromones at any intensity
   - `Anomaly` at intensity > 0.7
   - `NicheVacancy` and `RoleConflict` alerts
   - Latency: milliseconds

2. **Curator-aligned (batch, every 50 ticks, ~12.5 minutes)**:
   - Lower-priority pheromones (`Pattern`, `Opportunity`, `Wisdom`)
   - Knowledge entries promoted to Mesh scope
   - Morphogenetic role vector broadcasts
   - Batched for bandwidth efficiency

3. **On-demand (request-response)**:
   - Full sync on boot/recovery
   - Agent requests `SyncRequest` with its version vector
   - Peer responds with all entries newer than the vector

---

## Version Vector Deduplication

Each agent maintains a version vector -- a map from `AgentId` to the last
sequence number seen from that agent [Lamport, L. "Time, Clocks, and the
Ordering of Events in a Distributed System." *CACM*, 21(7), 1978]:

```rust
/// Version vector for pheromone deduplication across transports.
///
/// Each agent tracks the last sequence number seen from every other
/// agent. When a message arrives with seq <= last_seen, it is a
/// duplicate and is dropped.
pub struct VersionVector {
    /// Map: AgentId -> last seen sequence number.
    pub entries: HashMap<AgentId, u64>,
}

impl VersionVector {
    /// Returns true if this message has already been processed.
    pub fn is_duplicate(&self, sender: &AgentId, seq: u64) -> bool {
        self.entries.get(sender).map_or(false, |&last| seq <= last)
    }

    /// Update the vector after processing a message.
    pub fn update(&mut self, sender: AgentId, seq: u64) {
        let entry = self.entries.entry(sender).or_insert(0);
        *entry = (*entry).max(seq);
    }
}
```

### Deduplication Across Transports

When a pheromone is received via both WebSocket and Iroh, the version vector
ensures it is processed only once:

```
Agent A deposits pheromone (seq=42)
    |
    +-- WebSocket relay -> Agent B (receives seq=42, updates vector)
    |
    +-- Iroh gossip -> Agent B (seq=42, vector says duplicate, dropped)
```

### Version Vector Size

For N agents: N x 16 bytes (8-byte AgentId + 8-byte seq).
For N = 1,000: ~16 KB. Exchanged on reconnection before delta sync begins.

---

## Store-and-Forward

The WebSocket relay maintains a per-agent queue for offline agents:

```
Queue size = N_offline x R_deposit x TTL
```

For 5 agents offline 24 hours at 25 deposits/day:
- Queue: 5 x 25 x 1 = 125 entries x ~1KB = ~125 KB

### TTL Management

| TTL Setting | Default | Effect |
|------------|---------|--------|
| `store_forward_ttl` | 7 days | Maximum age of queued messages |
| `max_queue_size` | 10,000 | Maximum messages per offline agent |
| `gc_interval` | 1 hour | How often expired messages are pruned |

When an agent reconnects:

1. Agent sends its version vector to the relay.
2. Relay filters queued messages to only those with seq > agent's last seen.
3. Delta is delivered in sequence order.
4. Agent acknowledges receipt; relay clears the queue.

---

## Atomic Cursor Restore

After reconnection, the sync protocol guarantees atomic cursor restore:
all messages since the agent's last activity are delivered before any new
messages are processed. This ensures the agent has a consistent view of the
pheromone field before making decisions.

```rust
/// Reconnection protocol.
///
/// 1. Agent sends VersionVector to relay/peers.
/// 2. Delta messages are collected and ordered.
/// 3. All delta messages are delivered atomically.
/// 4. Normal real-time delivery resumes.
pub async fn reconnect_sync(
    my_vector: &VersionVector,
    relay: &dyn MeshTransport,
) -> Result<Vec<MeshMessage>> {
    let delta = relay.request_delta(my_vector).await?;
    // Process delta atomically before accepting new messages
    Ok(delta)
}
```

---

## Bandwidth Analysis

### Typical Volume Per Agent

| Scenario | Entries/day | Sync messages/day |
|----------|-----------|------------------|
| Quiet (mostly idle) | 5-10 | ~12 batch + ~2 immediate |
| Active (steady development) | 15-25 | ~12 batch + ~5 immediate |
| Volatile (many changes) | 25-50 | ~12 batch + ~10 immediate |

### Total Bandwidth

For a 5-agent Collective in active development:
- Messages: 5 x 15 = 75 deposits/day
- Fan-out: 75 x 4 = 300 deliveries/day
- Bandwidth: 300 x 1KB = ~300 KB/day
- Morphogenetic: 5 x 12 x 72 bytes = ~4.3 KB/day
- **Total: ~$0.045/day = ~$1.35/month**

### WebSocket vs Iroh Per-Node Bandwidth

| Collective Size | WebSocket (per node) | Iroh (per node) | Winner |
|----------------|---------------------|-----------------|--------|
| 5 | 4KB/msg | 2KB/msg | Gossip |
| 50 | 49KB/msg | 6KB/msg | Gossip |
| 500 | 499KB/msg | 9KB/msg | Gossip |
| 5,000 | ~5MB/msg | 12KB/msg | Gossip |

Gossip provides better per-node scaling at all sizes, but the relay is
simpler for small deployments and provides store-and-forward.

---

## Security

### Transport Security

- **WebSocket**: TLS 1.3 with standard certificate validation.
- **Iroh**: QUIC TLS 1.3 with ed25519 key pairs. Each agent has a unique
  Iroh node identity derived from its agent key.
- **Chain**: Chain consensus provides Byzantine fault tolerance.

### Message Authentication

All mesh messages are signed with the sender's agent key. Recipients verify
the signature before processing. Unsigned or invalid messages are dropped
silently.

### Rate Limiting

Per-agent rate limits prevent flooding:

| Limit | Default | Effect |
|-------|---------|--------|
| Immediate messages per minute | 10 | Prevents alert spam |
| Batch messages per cycle | 50 | Prevents bandwidth saturation |
| Total messages per day | 1,000 | Hard cap for runaway agents |

---

## Implementation Status

The connectivity contract has one supervised HTTP JSON adapter. `agent-relay`,
the supervised client, and `roko-serve` provide bounded canonical-envelope
delivery, atomic cursor restore, fail-closed reconciliation, and ACK-after-
durable exact-room subscription terminalization. Additional transports, startup
discovery, and MCP/A2A execution remain product work.

---

## References

- [Fidge 1988] Timestamps in Message-Passing Systems, *ACSC*
- [Lamport 1978] Time, Clocks, and Events, *CACM*
- [Parunak, Brueckner & Sauter 2005] Digital pheromones, *E4MAS*
