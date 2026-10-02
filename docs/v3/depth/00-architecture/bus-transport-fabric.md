# The Bus Transport Fabric

> **v3 depth file** -- `/docs/v3/depth/00-architecture/bus-transport-fabric.md`
> Canonical source: v1 `docs/v1/00-architecture/07b-bus-transport-fabric.md`
> Implementation: `crates/roko-core/src/traits.rs` (Bus trait), `crates/roko-runtime/` (PulseBus)
> Status: **Shipping** -- Bus trait with `publish` and `subscribe`, PulseBus implementation,
> Topic and TopicFilter types. EventBus<E> remains the internal transport primitive.

---

## 1. Role in the Architecture

Bus is the kernel's ephemeral transport fabric at L0. It exists for communication, not
durable storage. Where Store preserves Signals, Bus delivers Pulses to subscribers that
care about a topic family. The two fabrics together form the complete kernel surface at L0:
storage lives in Store, transport lives in Bus.

The design separates transport from persistence so that:

- high-frequency Pulses can move without forcing storage writes,
- subscribers can catch up from bounded replay,
- cross-layer couplings can be expressed as topics instead of direct crate dependencies.

### 1.1 Two-Fabric Summary

| Fabric | Medium | Core operations | Retention model | Implementations |
|---|---|---|---|---|
| Store | Signal | `put`, `get`, `query`, `query_similar`, `prune` | Long-lived storage with content identity and decay-aware pruning | MemorySubstrate, FileSubstrate |
| Bus | Pulse | `publish`, `subscribe` | Broadcast with topic routing | PulseBus |

---

## 2. Trait Surface

From `crates/roko-core/src/traits.rs`:

```rust
/// Publish/subscribe transport for ephemeral Pulses.
///
/// The Bus is the real-time transport layer that complements the durable Store.
/// Pulses flow through the Bus for immediate downstream reactions; only those
/// worth persisting get promoted to Signals and stored in a Store.
pub trait Bus: Send + Sync {
    /// The receiver type returned by subscribe.
    type Receiver: Send;

    /// Publish a pulse to all matching subscribers. Returns the assigned
    /// sequence number.
    fn publish(&self, pulse: Pulse) -> Result<u64>;

    /// Subscribe to pulses matching the given topic filter.
    fn subscribe(&self, filter: TopicFilter) -> Result<Self::Receiver>;
}
```

### 2.1 Payload and Routing Terms

- **Pulse** is the ephemeral message type carried by the Bus. Typed, sequence-numbered,
  topic-addressed.
- **Topic** is the routing handle on a Pulse. Dot-separated lowercase strings such as
  `gate.verdict.emitted`, `agent.msg.chunk`, or `prediction.error`.
- **TopicFilter** is the declarative subscription matcher.
- **Receiver** is the subscriber handle returned by `subscribe()`. It yields Pulses in
  publish order.

The Bus does not content-address messages and does not persist them by default. If a message
must survive beyond the session, graduate it to a Signal and store it.

### 2.2 TopicFilter

```rust
pub enum TopicFilter {
    Exact(Topic),
    Glob(String),
    AnyOf(Vec<Topic>),
    All,
    And(Box<TopicFilter>, Box<TopicFilter>),
    Or(Box<TopicFilter>, Box<TopicFilter>),
    Not(Box<TopicFilter>),
}
```

Topics are dot-separated lowercase strings. They name transport intent rather than durable
storage identity. Examples:

| Topic | Purpose |
|---|---|
| `gate.verdict.emitted` | Gate pipeline completed a verdict |
| `agent.msg.chunk` | Agent produced a streaming message chunk |
| `prediction.error` | A prediction was resolved with an error signal |
| `safety.approval.requested` | A high-risk action needs confirmation |
| `conductor.circuit.tripped` | A circuit breaker was triggered |
| `plan.revision.requested` | A plan needs replanning after failures |
| `substrate.engram.stored` | A Signal was persisted (Store-Bus bridge) |
| `heartbeat.gamma.tick` | Gamma-speed heartbeat |
| `heartbeat.theta.tick` | Theta-speed heartbeat |
| `heartbeat.delta.tick` | Delta-speed heartbeat |

---

## 3. Semantics

### 3.1 Publish and Subscribe

`publish()` fans a Pulse out to every matching subscriber. The delivery model is broadcast,
not queue-based work stealing. Every subscriber sees every matching Pulse.

`subscribe()` returns a receiver that is cancel-safe. Callers can keep the receiver open
for a narrow topic family or for broad catch-all monitoring.

### 3.2 Sequence Numbers

Every published Pulse receives a monotonically increasing sequence number (bus-scoped).
Subscribers can use sequence numbers for gap detection and ordered replay.

### 3.3 Graduation: Pulse to Signal

The Bus does not persist Pulses. If a Pulse carries information that needs durable lineage
and audit, the runtime graduates it to a Signal:

```rust
impl Signal {
    pub fn from_pulse_synthetic(pulse: &Pulse) -> Signal {
        // Creates a synthetic Signal from a Pulse for scoring/verification
    }
}
```

Graduation is a deliberate step, not an architectural accident. The Signal inherits the
Pulse's content but gains content addressing, lineage, provenance, score, and decay.

---

## 4. Implementation: PulseBus

The current implementation (`PulseBus` in `roko-runtime`) wraps `EventBus<Pulse>` with topic
filtering:

```rust
pub struct PulseBus {
    inner: EventBus<Pulse>,
    // topic routing and filtering logic
}
```

`EventBus<E>` is the live generic broadcast-channel transport abstraction. It provides
concurrent publish/subscribe with `tokio::sync::broadcast` semantics internally.

The current live `RokoEvent` transport has production variants including `PlanRevision`.
Every production event variant is wired into the telemetry lens system.

---

## 5. Backend Families

### 5.1 PulseBus (Shipping)

Default in-process implementation. Wraps `EventBus<Pulse>` for single-process agents, tests,
and local tooling.

### 5.2 Planned Backends

| Backend | Purpose | Status |
|---|---|---|
| MultiBus | Aggregates multiple backends into one stream | Planned |
| NATS / Kafka / Redpanda | Multi-process and distributed transport | Planned |
| ChainBus | On-chain Pulse transport for chain-integrated deployments | Planned |
| MemoryBus | Test-only backend with no background task | Planned |

---

## 6. Concurrency

All Bus implementations are `Send + Sync`. They must handle concurrent publishers and
subscribers internally.

- In-process backends rely on concurrent broadcast channels.
- Multi-process backends would use broker or log semantics.
- The API is designed so callers do not need external locks.

---

## 7. Bus-Mediated Cross-Layer Decoupling

The Bus dissolves direct cross-layer crate dependencies. Instead of importing types from
another layer, a subsystem subscribes to topic streams:

**Before** (layer violation):
```
roko-conductor (L3) --> imports --> roko-learn (L2)
```

**After** (Bus-mediated):
```
roko-learn publishes on "gate.failure.rate" topic
roko-conductor subscribes to "gate.failure.rate" on the Bus
```

No direct dependency. No layer violation. The topic name is the contract.

This pattern explains how the five-layer taxonomy works in practice: shared state and live
coordination flow through the kernel fabrics (Store and Bus) instead of through direct crate
imports.

---

## 8. Two-Fabric Kernel Story

The kernel surface is intentionally small:

- **Store** persists durable Signals.
- **Bus** transports ephemeral Pulses.

This split lets Roko express durable knowledge and live communication without forcing every
message through the same mechanism. It also makes cross-layer communication auditable through
topic names rather than direct dependencies.

The architectural invariant:
- **Store** holds durable audit truth.
- **Bus** delivers real-time visibility and reaction triggers.
- **Custody** and **Attestation** lift selected actions to stronger guarantees within Store.
- **Pulses** on Bus enable immediate intervention.

---

## Cross-References

- `substrate-trait.md` -- The Store sibling fabric
- `synapse-traits-12.md` -- All 12 kernel traits including Bus
- `five-layer-taxonomy.md` -- Where Bus fits in the layer diagram
- `provenance-and-attestation.md` -- How the two fabrics divide audit responsibility
- `naming-and-glossary.md` -- Canonical terminology for Bus, Topic, TopicFilter, Pulse
