# Pulse — Open Questions

> Unresolved design decisions for the Pulse/Bus system.

**Status**: Specified  
**Last reviewed**: 2026-04-19

---

## OQ-1: Backpressure

**Question:** What happens when a subscriber is slow and the Bus emits faster than it can
process? Should the Bus block, drop Pulses, or buffer?

**Current assumption:** Pulse delivery is best-effort. Slow subscribers are dropped from
the delivery set after a configurable timeout. If this causes critical events to be missed,
they should be stored as Engrams before emission.

---

## OQ-2: Persistence Across Restarts

**Question:** Should the Bus persist Pulses across process restarts for subscribers that
were offline? (At-least-once delivery?)

**Current assumption:** No persistence. Pulses are ephemeral. If at-least-once delivery
is needed, the emitter should store an Engram before emitting the Pulse.

---

## OQ-3: Datum Union Type

**Question:** The glossary introduces `Datum` as a planned "polymorphic `Engram` or
`Pulse` input" type. Where does Datum fit in the architecture?

**Current assumption:** `Datum` is used at operator boundaries that accept either an
Engram reference or a Pulse for immediate processing. It is a thin enum:

```rust
pub enum Datum<'a> {
    Engram(&'a Engram),
    Pulse(&'a Pulse),
}
```

Not yet specified in full.

---

## OQ-4: Chain Sources

**Question:** For chain-originated Pulses (`PulseSource::Chain`), should there be a
cryptographic attestation attached to the PulseSource?

**Current assumption:** Chain attestation happens at graduation time (when the Pulse
becomes an Engram with `TrustLevel::ChainWitness`). The PulseSource carries the
chain_id and node identifier; actual attestation is deferred to Provenance.

---

## OQ-5: Dead Letter Queue

**Question:** Should there be a special topic (dead-letter topic) for Pulses that have
no matching subscribers?

**Current assumption:** Unrouted Pulses are silently dropped (with an optional debug log).
A dead-letter topic would be implemented as `TopicFilter::All` subscriber.

---

## OQ-6: Ordering Guarantees

**Question:** Should Pulses on the same topic be delivered in emission order?

**Current assumption:** No ordering guarantees within a single process; FIFO across
a single subscriber's handler queue.

---

## OQ-7: Migration Sequencing

**Question:** Can `EventBus<E>` and `Bus` coexist long-term, or does the migration need
to complete before a certain release?

**Current assumption:** They can coexist indefinitely via the adapter pattern (Phase 3
in `05-today-vs-planned.md`). Migration is driven by individual component readiness.

---

## See Also

- [`05-today-vs-planned.md`](05-today-vs-planned.md) — migration plan
- [`02-topics-and-filters.md`](02-topics-and-filters.md) — routing design
