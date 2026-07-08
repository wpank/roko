# Bus as a First-Class Kernel Primitive

> **TL;DR**: Promote the event bus to a kernel trait in `roko-core`,
> paired with the existing `Substrate` trait. Substrate is the storage
> fabric; Bus is the transport fabric. Both are L0. Together they are
> the complete kernel of Roko's runtime.

> **For first-time readers**: Roko already has a `Substrate` trait in
> `crates/roko-core/src/traits.rs` (persist, query, prune durable Engrams)
> and an `EventBus<E>` struct in `crates/roko-runtime/src/event_bus.rs`
> (generic typed broadcast channel with replay ring). The Bus works, but it
> isn't in the architectural lexicon — no trait, no doc chapter, no stable
> name. This proposal promotes it: a `Bus` trait at kernel tier, `Pulse` as
> its payload (see 02), topics as routing handles, and a bounded ring as
> replay memory. Nothing in the current Bus implementation goes away — it
> becomes the default `BroadcastBus` implementation of the new trait.

## 1. The two fabrics

The current kernel presents one fabric — **storage** via the `Substrate`
trait at `crates/roko-core/src/traits.rs`. Every subsystem that needs
to communicate persistently uses it. Four backends already exist
(`MemorySubstrate`, `FileSubstrate`, `HdcSubstrate`, `ChainSubstrate`)
and they are API-identical from a caller's perspective.

The proposed kernel adds a second fabric — **transport** via a new
`Bus` trait. It already exists in spirit at
`crates/roko-runtime/src/event_bus.rs` as `EventBus<E>`. The refactor
canonicalizes its interface, moves it into `roko-core` at the same
layer as Substrate, and makes Pulse (not a user-defined enum) its
payload.

| | **Substrate** | **Bus** |
|---|---|---|
| Medium | Engram | Pulse |
| Shape | Put/Get/Query/Prune | Publish/Subscribe/Replay |
| Semantics | Idempotent content-addressed write; query by filter | Broadcast fan-out; topic-addressed; bounded ring for replay |
| Durability | Long-lived; decays over time | Brief; bounded by ring capacity |
| Concurrency | `Send + Sync`, handles concurrent puts/queries | `Send + Sync`, handles concurrent subscribers |
| Backends shipping today | Memory, File, (HDC built, Chain stubbed) | In-process broadcast (`tokio::sync::broadcast`) |
| Backends future | Any key-value or log store | NATS, Kafka, Redpanda, chain pubsub |
| Crate location | `roko-fs`, `roko-std`, `roko-neuro` impls | proposed: `roko-std` for broadcast, `roko-mesh` for NATS, `roko-chain` for chain pubsub |

## 2. Proposed trait

```rust
// crates/roko-core/src/traits.rs (new section)

use crate::{Pulse, Topic, TopicFilter, error::Result};
use async_trait::async_trait;

/// Transport fabric for Pulses.
///
/// A Bus delivers Pulses from publishers to subscribers. All Bus
/// implementations are API-identical from a caller's perspective — pick
/// the backend that matches your fan-out, durability, and latency needs.
///
/// # Delivery model
///
/// Bus is broadcast: every subscriber sees every Pulse on topics it
/// matches. There is no queuing or redelivery. Subscribers that fall
/// behind the ring buffer lose Pulses. For critical data, graduate
/// the Pulse to an Engram and subscribe to the Substrate.
///
/// # Replay
///
/// `replay_since(seq)` returns Pulses whose global sequence is
/// strictly greater than `seq` and still in the ring buffer. The
/// caller uses this to catch up after a brief disconnect or to
/// bootstrap a late subscriber.
///
/// # Concurrency
///
/// Buses are `Send + Sync`. Impls must handle concurrent publishers
/// and subscribers internally.
#[async_trait]
pub trait Bus: Send + Sync {
    /// Publish a Pulse. Returns its global sequence number.
    async fn publish(&self, pulse: Pulse) -> Result<u64>;

    /// Subscribe to a topic filter. Returns a receiver that yields
    /// Pulses in publish order. The receiver is cancel-safe.
    async fn subscribe(&self, filter: TopicFilter) -> Result<BusReceiver>;

    /// Replay Pulses newer than `since_seq` matching `filter`, up to
    /// the ring buffer's retention window. Used for resume after a
    /// brief disconnect.
    async fn replay_since(
        &self,
        since_seq: u64,
        filter: &TopicFilter,
    ) -> Result<Vec<Pulse>>;

    /// Current global sequence number (for checkpointing).
    async fn current_seq(&self) -> Result<u64>;

    /// Total Pulses published since bus start (for metrics).
    async fn total_published(&self) -> Result<u64>;

    /// Ring buffer current occupancy (for health checks).
    async fn ring_len(&self) -> Result<usize>;

    /// Ring buffer capacity (for health checks).
    fn ring_capacity(&self) -> usize;

    /// Human-readable name for logging/debugging.
    fn name(&self) -> &'static str {
        "unnamed_bus"
    }
}

/// A subscriber handle.
pub struct BusReceiver {
    pub inner: tokio::sync::mpsc::Receiver<Pulse>,
    pub last_seq: std::sync::atomic::AtomicU64,
}
```

### 2.1 TopicFilter

```rust
/// A declarative filter for Bus subscriptions.
pub enum TopicFilter {
    /// Match exactly one topic.
    Exact(Topic),
    /// Match a glob pattern, e.g. "agent.*" or "gate.verdict.*".
    Glob(String),
    /// Match any topic from the set.
    AnyOf(Vec<Topic>),
    /// Match all topics.
    All,
    /// Boolean AND.
    And(Box<TopicFilter>, Box<TopicFilter>),
    /// Boolean OR.
    Or(Box<TopicFilter>, Box<TopicFilter>),
    /// Boolean NOT.
    Not(Box<TopicFilter>),
}
```

## 3. Backends

### 3.1 Broadcast (in-process)

The default, shipping immediately. Wraps `tokio::sync::broadcast`. The
current `EventBus<E>` in `roko-runtime` becomes this, with the
signature simplified to take `Pulse` instead of a generic `E`.

### 3.2 MultiBus

Composes multiple Bus backends behind a single interface. Used by the
dashboard to see both in-process agent Pulses and incoming HTTP
webhook Pulses as one stream.

### 3.3 NATS / Redpanda / Kafka

For multi-process deployments (the `roko serve` control plane + remote
agent workers pattern described in `docs/VISION-RUN-ANYWHERE.md`).
Shipping this is post-Phase-1 but the trait supports it today.

### 3.4 ChainBus

Phase 2+. `chain.*` topics map to on-chain event logs. Subscribers
tail the chain via RPC. Replay maps to block scanning. Mirrors the
`ChainSubstrate` model.

### 3.5 MemoryBus

For testing. Drop-in for BroadcastBus without spawning a Tokio task.

## 4. Wiring: L0 Runtime becomes complete

Current L0 (per `docs/00-architecture/12-five-layer-taxonomy.md`):

> **Layer 0: Runtime** — Process lifecycle, event bus, supervision,
> cancellation, I/O, adaptive clock.
>
> Key Crates: `roko-primitives`, `roko-runtime`
>
> Synapse Traits at L0: `Substrate`

The doc already lists "event bus" as an L0 concern, but there's no
Synapse trait for it. After the refactor:

> Synapse Traits at L0: `Substrate`, `Bus`

And the two-fabric story becomes the kernel's executive summary:

> The Roko kernel is two fabrics — `Substrate` for durable Engrams and
> `Bus` for ephemeral Pulses. Every subsystem talks to the rest of
> Roko through one or both of these fabrics. Dependencies flow
> downward from higher layers to L0; higher-layer communication
> never bypasses the fabrics.

## 5. How this fixes doc 23's layer violation

`docs/00-architecture/23-architectural-analysis-improvements.md` §3.2
flagged exactly one confirmed violation: `roko-conductor → roko-learn`.
Root cause:

> `roko-conductor` imports learning types for circuit breaker state
> tracking. The Conductor needs to know about historical failure rates
> (a learning concern) to make circuit breaker decisions (a harness
> concern).

Doc 23's fix: extract a `HealthMetrics` trait into `roko-core` L0.
That works but adds a third trait surface.

**The Bus-first fix is simpler and subsumes it.** `roko-learn` already
emits gate-verdict-derived stats. Instead of `roko-conductor` calling
into `roko-learn` types, both subsystems subscribe to the same topic
family:

- `gate.verdict.emitted` — published by GatePipeline.
- `gate.failure.rate` — computed by `roko-learn`'s `CircuitBreakerPolicy`
  over a rolling window, published at some cadence.

`roko-conductor` subscribes to `gate.failure.rate` and reacts. No
compile-time dependency on `roko-learn`. Both crates depend only on
`roko-core` (which now owns `Bus` and `Pulse`).

This pattern generalizes. Every cross-layer coupling in the codebase
can be audited with the question "could this be a Bus topic instead?"
— and most of the time the answer is yes.

## 6. What `roko-runtime` becomes

Today `roko-runtime` owns:

- `event_bus` (becomes a Bus backend — move to `roko-std`)
- `process` (ProcessSupervisor — stays)
- `cancel` (cancellation tokens — stays)
- `metrics` (JSONL metric recording — consider moving to `roko-obs`)
- `resource` (limits, tracking — stays)

After the refactor:

- `roko-core` owns the `Bus` trait, `Pulse`, `Topic`, `TopicFilter`,
  `BusReceiver`, `GraduationPolicy`.
- `roko-std` owns `BroadcastBus` (the in-process impl) and `MemoryBus`.
- `roko-runtime` owns `ProcessSupervisor`, cancellation,
  resource-limit primitives. It depends on `roko-core::Bus` to publish
  process lifecycle Pulses.
- `roko-mesh` (new, Phase 2) owns `NatsBus`, `KafkaBus`.
- `roko-chain` extends to own `ChainBus` alongside `ChainSubstrate`.

## 7. Breaking change surface

Because today there's no `Bus` trait, there's nothing to break. The
refactor adds a trait, adds a type, and migrates internal call sites
from ad-hoc `EventBus<SomeEnum>` to `Bus` + `Pulse`.

The `Envelope<E>` type stays around as a deprecated alias for one
release so in-flight PRs aren't blocked. Then it's removed.

## 8. Open questions for this proposal

1. **Does `Bus` need its own `prune`?** Substrate has `prune` for
   decay-based eviction. Bus has ring-buffer eviction which is FIFO,
   not decay-aware. Probably don't need `prune` on `Bus`.
2. **Should the Bus carry `Context`?** Substrate methods take
   `&Context`. Bus publish doesn't naturally need it (the publisher
   knows its own context). Subscribe might benefit from it for
   authorization. Lean: don't add `Context` to Bus methods until we
   have a concrete authorization story — see `32-safety-sandbox-provenance.md`
   for where that story lands.
3. **Schema evolution for topics.** If a Pulse's Body shape changes,
   how do subscribers know? For now: reuse Engram's approach
   (non-exhaustive enums, `Custom(String)` escape hatch, `Body::Json`
   for structured). Formal topic schemas can wait for a v2.
4. **Wildcard unsubscribe.** If a subscriber uses `Glob("agent.*")`
   and a new topic `agent.heartbeat` is introduced, does the
   subscriber want it? Probably yes — documented behavior.
5. **Replay-window sizing.** 4096 Pulses is the default ring capacity.
   On a quiet system this is minutes of history. On a hot streaming
   agent, it can be under a second. Per-topic overrides plus an
   observable `bus.ring.occupancy` Pulse let operators spot the
   problem before data is lost. A cluster-scale Bus (NATS / Kafka)
   inherits that backend's retention policy, not ours.
6. **Ordering across topics.** Per-topic sequence numbers are
   monotonic. Cross-topic sequence numbers are monotonic only within
   a single Bus instance. Multi-bus deployments that need a global
   order need a `MultiBus` that stamps a global sequence at fan-in
   (§3.2). Most consumers don't care; the replication ledger
   (`16-research-to-runtime.md`) and chain witnesses
   (`09-phase-2-implications.md` §1) do.
7. **Authentication of publishers.** A subscriber can trust the Bus
   delivered the Pulse it claims was published, but can they trust
   the `source` field? For in-process publishers, yes (compile-time
   boundary). For cross-process publishers (HTTP webhook, remote
   agent) the Bus needs a signed-publish primitive. Punt to
   `32-safety-sandbox-provenance.md`.

## 9. Why this proposal is low-risk

Three concrete reasons the refactor is safe to commit to:

1. **The runtime primitive already exists.** We are giving a name,
   trait, and doc page to behavior that is live in production today.
   The `BroadcastBus` implementation is about 150 lines of wrapping
   around `tokio::sync::broadcast` plus a `VecDeque` for replay.
2. **The migration is compile-checkable.** Every call site that
   publishes or subscribes goes through `Bus::publish` or
   `Bus::subscribe`. If we rip out the current ad-hoc enums, the
   compiler tells us every place that needs updating. There is no
   "spooky action at a distance" coupling that would produce runtime
   surprises.
3. **No data shape changes.** Pulse reuses `Kind` and `Body` from
   the existing Engram taxonomy. No new serialization formats, no
   new on-disk representations, no migration scripts. The Substrate
   remains the only persistence surface.

Compare to the `roko-conductor → roko-learn` fix that doc 23
originally proposed (extract a `HealthMetrics` trait): that fix adds
a third trait surface and splits the failure-rate EMA logic across
two crates with a shared vocabulary. The Bus-first fix *removes* the
direct dependency without introducing a new trait — the shared
vocabulary becomes a topic string.

See `08-code-sketches.md` for the actual Rust signatures and
`06-refactoring-plan.md` for the migration steps. See
`33-observability-telemetry.md` for the metrics the Bus should
expose out of the box.
