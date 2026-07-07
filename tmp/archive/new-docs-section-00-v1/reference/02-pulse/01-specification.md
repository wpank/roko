# Pulse — Specification

> Target-state Pulse struct, fields, and lifecycle.

**Status**: Specified  
**Crate**: `roko-core` (planned)  
**Depends on**: [Overview](00-overview.md), [PulseSource](04-pulse-sources.md)  
**Last reviewed**: 2026-04-19

> **Target state — no code yet.**

---

## TL;DR

A Pulse is a struct with: a `Topic` (routing key), a `PulseSource` (origin), a typed
`payload`, and a timestamp. Pulses have no ContentHash, no Score, no Decay, and no Lineage.
They are created, routed, and discarded in sub-millisecond time. Their lifetime is bounded
by the subscriber delivery window.

---

## The Idea

Pulses need to be as lightweight as possible. Every additional field on a Pulse is overhead
paid on every event in the system. The design goal: a Pulse should be smaller than an
Engram by one order of magnitude.

---

## Specification

### Target Pulse Struct

```rust
<!-- source: crates/roko-core/src/pulse.rs (target state — not yet implemented) -->

/// An ephemeral event that flows over the Bus.
/// Not stored; no ContentHash; no Score; no Decay.
///
/// # Lifetime
/// A Pulse is alive from the moment of emission until the last subscriber
/// processes it (or the delivery timeout expires). It is then dropped.
///
/// # Target state
/// This struct is a specification target. The current codebase uses `EventBus<E>`.
#[derive(Clone, Debug)]
pub struct Pulse<P = Box<dyn Any + Send + Sync>> {
    /// Routing key. Subscribers register on Topics.
    pub topic: Topic,

    /// Origin attribution. Required in target state.
    pub source: PulseSource,

    /// The event payload. Typed by the P parameter.
    pub payload: P,

    /// Emission timestamp (Unix ms).
    pub emitted_at_ms: i64,

    /// Optional correlation id for distributed tracing.
    /// When a Pulse is graduated to an Engram, this id appears in the Engram's tags.
    pub correlation_id: Option<CorrelationId>,
}
```

### CorrelationId

```rust
<!-- source: crates/roko-core/src/pulse.rs (target state) -->

/// A lightweight distributed tracing token.
/// Carried from Pulse to any graduated Engram for log correlation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CorrelationId(pub [u8; 16]);  // 128-bit random id

impl CorrelationId {
    pub fn new() -> Self { CorrelationId(rand::random()) }
    pub fn to_hex(&self) -> String { /* ... */ }
}
```

---

## Field Reference

| Field | Type | Description |
|-------|------|-------------|
| `topic` | `Topic` | Routing key; subscribers register interest in topics |
| `source` | `PulseSource` | Who emitted this Pulse |
| `payload` | `P` | The event data; type-parameterized |
| `emitted_at_ms` | `i64` | Unix ms emission timestamp |
| `correlation_id` | `Option<CorrelationId>` | Distributed trace token |

---

## Lifecycle

1. **Emission**: A component calls `bus.emit(pulse)`. The Bus records the emission timestamp if not set.
2. **Routing**: The Bus looks up all subscriptions matching `pulse.topic` and `TopicFilter`.
3. **Delivery**: The Bus delivers the Pulse to each matching subscriber's handler.
4. **Graduation or discard**: Each subscriber either:
   - Graduates the Pulse to an Engram (builds an `EngramBuilder` from the payload and inserts into Substrate), or
   - Discards the Pulse (does nothing further).
5. **Drop**: After all subscribers have processed or timed out, the Pulse is dropped.

---

## What Pulses Are NOT

- **Not content-addressed.** Two Pulses with identical content are not the same Pulse.
- **Not scored.** Pulses carry no quality axes.
- **Not decaying.** Pulses live for their delivery window only.
- **Not in the lineage DAG.** A graduated Engram may reference the `correlation_id` in tags,
  but the Pulse itself is not a lineage parent.
- **Not retrievable.** Once a Pulse is delivered and dropped, it is gone. If you need to
  query it later, it should have been graduated to an Engram.

---

## Invariants

1. `emitted_at_ms > 0`
2. `topic` must be a valid Topic (non-empty, valid format)
3. Pulses are not stored in the Substrate
4. Pulses do not appear in the Engram lineage DAG

---

## Failure Modes

| Failure | Cause | Recovery |
|---------|-------|----------|
| Subscriber timeout | Subscriber takes too long | Bus drops the Pulse after timeout; logs event |
| No subscribers | Pulse emitted on unregistered topic | Pulse dropped; debug log if `debug_unrouted = true` |
| Payload deserialization error | Subscriber cannot deserialize payload | Subscriber logs error; Pulse dropped |

---

## See Also

- [`02-topics-and-filters.md`](02-topics-and-filters.md) — routing by Topic and TopicFilter
- [`03-graduation-rules.md`](03-graduation-rules.md) — when to convert a Pulse to an Engram
- [`05-today-vs-planned.md`](05-today-vs-planned.md) — current EventBus vs. this spec
