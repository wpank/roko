# Core Concept: The Relay IS the Bus

## The two fabrics

In roko v2, all data flows through two fabrics:

- **Store** — durable, content-addressed, pull-based. Signals live here. Query to retrieve.
- **Bus** — ephemeral, pub/sub, ring-buffered. Pulses live here. Subscribe to receive.

The daeji relay is the **cross-network Bus implementation**. When agents run in-process,
the Bus is `tokio::sync::broadcast`. When agents are on different machines, the Bus is the
relay's WebSocket transport.

The daeji chain is the **Global Store**. ERC-8004 identities, ERC-8183 job escrow,
InsightBoard knowledge claims — all durable, verifiable, on-chain.

**Daeji provides both fabrics.** Chain = Store. Relay = Bus.

## Signal/Pulse duality

**Signals** are durable. Content-addressed. Have a `balance` that decays via demurrage.
Stored locally (agent's sled/postgres) or globally (chain). Retrieved by query.

**Pulses** are ephemeral. Sequence-numbered. Topic-routed. Delivered via Bus. Gone when
the ring buffer evicts them.

The relay carries **Pulses only**. It does not store Signals.

### Graduation: Pulse → Signal

Some Pulses are valuable enough to persist. The agent decides:

| Pulse type | Graduate to Signal? | Why |
|-----------|-------------------|-----|
| Heartbeat | No | Next one is coming in 100ms |
| Feed data point | Sometimes | Routine tick: no. Trend breakout: yes. |
| Group message | Sometimes | Routine coordination: no. Key decision: yes. |
| Pheromone notification | Yes (by convention) | Pheromones are Signals with demurrage |
| Chain event | Yes (important ones) | Job funded: yes. |
| Decision trace | Yes | Learning depends on it |

### Projection: Signal → Pulse

The reverse: when a Signal is created in Store, the agent can project it as a Pulse to
notify others. Knowledge sharing = create Signal → project as Pulse → relay delivers →
other agents graduate to their own Store.

## Room = Bus topic

The relay's rooms map 1:1 to Bus topics. Standard naming convention:

```
agent:{id}                  Agent lifecycle events
agent:{id}:heartbeat        Heartbeat ticks
agent:{id}:output           Streaming LLM output
agent:{id}:feed:{feed_id}   Agent-produced data streams
feed:{id}:data              Feed data (equivalent topic form)
group:{id}                  Group broadcast messages
group:{id}:knowledge        Knowledge publish/validate events
group:{id}:pheromones       Pheromone deposit/decay notifications
group:{id}:coordination     Task assignment, status updates
chain:{chain_id}            Chain events (blocks, contract events)
system                      Server health, provider status
```

## Envelope format

Every message uses the same envelope:

```json
{
  "seq": 4821,
  "ts": 1713974400123,
  "room": "group:job-42",
  "type": "group.message",
  "from": "roko-alpha-1",
  "payload": { ... }
}
```

The relay routes by `room`. It does not interpret `type` or `payload`. Application semantics
are agent-level concerns.

## What the relay maintains

1. **Connection state** — who's connected, what topics they subscribe to
2. **Ring buffer** — per-connection ring of recent messages for reconnection replay
3. **Feed directory** — registered feeds for discovery
4. **Group directory** — active groups for management

What it does NOT maintain: message history, Signals, knowledge bases, pheromone state,
reputation. The relay is stateless beyond the ring buffer.

## The three-layer architecture

```
┌────────────────────────────────────────┐
│         Global Store (daeji chain)      │
│  ERC-8004 identities, ERC-8183 jobs    │
│  InsightBoard knowledge claims          │
│  Verifiable, durable, on-chain          │
└──────────────────┬─────────────────────┘
                   │ chain watcher reads events
                   ▼
┌────────────────────────────────────────┐
│           Bus (daeji relay)             │
│  WebSocket pub/sub, ring-buffered       │
│  Carries Pulses: feeds, coordination,   │
│  heartbeats, chain events, pheromones   │
│  Ephemeral — no persistence beyond ring │
└──────────────────┬─────────────────────┘
                   │ agents subscribe
                   ▼
┌────────────────────────────────────────┐
│          Local Store (per agent)        │
│  Knowledge, heuristics, episodes        │
│  Pheromone state, decision history      │
│  Private, durable, agent-controlled     │
└────────────────────────────────────────┘
```
