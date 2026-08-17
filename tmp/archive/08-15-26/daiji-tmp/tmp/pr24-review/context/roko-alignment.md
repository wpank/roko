# Roko Alignment: Sketches and V2 Architecture

## Roko's daeji sketches (tmp/daeji/)

12 design documents envisioning chain-agent integration:
- Local devnet infra, deployment configs, Docker compose
- Agent ↔ chain mapping (ERC-8004 identity, ERC-8183 jobs)
- Novel features: HDC precompiles, VRF, commitment verification, Merkle proofs
- Knowledge layer: InsightBoard, half-life decay, shared learning
- Native design: wiring roko's orchestrate.rs to daeji subsystems

### PR #24 alignment with sketches

**Strong alignment:**
- On-chain identity → mesh identity (AgentRegistered → chain watcher → registry)
- Authenticated group communication (AEAD-encrypted rooms)

**Partial alignment:**
- Job lifecycle → room lifecycle (JobAwarded → room activation, but auto-join deferred)

**Gaps:**
- No programmatic event API (demo driver only, no channel for host process)
- Embedded in kora (sketches assume lightweight integration, reality is heavyweight)
- No knowledge-layer message types
- No WebSocket subscriptions from chain to agents

## Roko v2 architecture (docs/v2/)

The v2 spec fundamentally reshapes how the relay should be designed. Key concepts:

### Five primitives
- **Signal** — durable datum in Store (content-addressed, demurrage, balance, scoring)
- **Pulse** — ephemeral event on Bus (seq-numbered, topic-routed, ring-buffered)
- **Cell** — atomic computation unit (9 protocols, typed I/O, capabilities)
- **Graph** — typed DAG of Cells (composition primitive)
- **Protocol** — behavioral contract (Store, Score, Verify, Route, Compose, React, Observe, Connect, Trigger)

### Two fabrics
- **Store** — durable, content-addressed, pull-based. "The library."
- **Bus** — ephemeral, pub/sub, ring-buffered. "The nervous system."

### Key specializations for the relay
- **Feed** = Cell + Connect + Trigger + Store. Continuous data streams.
- **Group** = Space + Membership + CoordinationMode. Persistent agent collectives.
- **4 coordination modes:** stigmergic (pheromones), pipeline (DAG), broadcast, leader-follower

### The core realization

The daeji relay IS the Bus fabric. The daeji chain IS the Global Store.
Together they provide both fabrics from the v2 spec.

- Topics = Bus topics with standard naming (`agent:{id}`, `feed:{id}:data`, `group:{id}`)
- Envelopes = `{ seq, ts, room, type, payload }`
- Feeds = Pulses on Bus topics, registered in relay directory
- Groups = Bus partitions with sub-rooms, chain-driven lifecycle
- Chain events = Store → Bus projection (chain watcher publishes Pulses)
- Pheromones = Signals in Store, notified via Pulses on Bus

### Exoskeleton protocols
Four external standards roko builds on:
- **MCP** — tool/resource discovery (Linux Foundation)
- **A2A** — agent-card discovery (Linux Foundation)
- **ERC-8004** — on-chain identity/reputation (Ethereum)
- **x402** — stablecoin agent-to-agent payments (open standard)

### Agent topologies
- **In-process** — tokio tasks, mpsc channels, shared Bus
- **Remote** — outbound WebSocket to relay, NAT-traversed
- **Direct-reachable** — public URL + relay for events

### Three-tier routing fallback
1. In-process (mpsc) — ~0 latency
2. Direct HTTP — ~10-50ms
3. Relay-forwarded — ~50-200ms (universal fallback)

### Agent discovery: four sources merged
| Source | Truth claim |
|--------|-------------|
| Relay presence | Liveness |
| A2A agent cards | Capability |
| ERC-8004 on-chain | Identity |
| Deployment list | Reachability |
