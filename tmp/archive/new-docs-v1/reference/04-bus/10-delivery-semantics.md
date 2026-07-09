# Delivery Semantics

> At-most-once, at-least-once, and exactly-once delivery — what each means and what `Bus`
> provides.

**Status**: Specified
**Crate**: `roko-core` (planned)
**Depends on**: [Publish / Subscribe](./04-publish-subscribe.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

The default delivery semantic is **at-most-once** (best effort, no replay). Subscribers
that need **at-least-once** delivery use ring-buffer replay on reconnect. **Exactly-once**
is not provided and is out of scope for the core `Bus` abstraction.

---

## Delivery Semantic Definitions

| Semantic | Meaning | When used |
|---|---|---|
| **At-most-once** | Event delivered 0 or 1 times. May be dropped. | Default; low-latency, non-critical events |
| **At-least-once** | Event delivered ≥ 1 times. May be duplicated. | Events that must not be lost; use replay to recover |
| **Exactly-once** | Event delivered exactly 1 time. No drops, no duplicates. | Out of scope for Bus core |

---

## Default: At-Most-Once

The default Bus delivery is at-most-once. Events are dropped when:
- The subscriber's channel buffer is full (fast publisher, slow subscriber).
- The ring buffer has evicted the event before the subscriber replays.
- The process crashes between publish and delivery.

This matches the semantics of `EventBus<E>` today (broadcast channel drops on full).

---

## At-Least-Once via Replay

A subscriber that requires at-least-once delivery must:
1. Subscribe with a `replay_from` spec (e.g., `ReplaySpec::Since(last_processed_ts)`).
2. Persist `last_processed_ts` durably (e.g., in `Substrate`) after processing each event.
3. On reconnect, subscribe with the stored timestamp.

```rust
// source: crates/roko-core/src/bus.rs  [target-state]
// On startup, load the last processed timestamp from substrate.
let last_ts = load_checkpoint_from_substrate(&substrate)?;

// Subscribe with replay from checkpoint.
let stream = bus.subscribe(
    TopicFilter::Exact(Topic::new("loop.step.score")),
    Some(ReplaySpec::Since(last_ts)),
).await?;

for pulse in stream {
    process(pulse).await;
    save_checkpoint_to_substrate(&substrate, pulse.timestamp)?;
}
```
<!-- source: crates/roko-core/src/bus.rs -->

This pattern combines `Bus` replay with `Substrate` checkpointing for durable at-least-once
delivery.

---

## Exactly-Once

Exactly-once delivery requires distributed coordination (deduplication tables, idempotency
keys, or two-phase commit). This is not part of the `Bus` core abstraction. If an operator
needs exactly-once semantics, it must implement idempotency at the application layer (e.g.,
by tracking processed sequence numbers in `Substrate`).

---

## Summary

| Backend | Default semantic | At-least-once available? |
|---|---|---|
| EventBus<E> (today) | At-most-once | No (no replay) |
| InProcessBus (planned) | At-most-once | Yes (ring buffer replay) |
| Distributed Bus (planned) | At-most-once | Yes (broker retention + replay) |

---

## See Also

- [Replay and Ring](./05-replay-and-ring.md)
- [Ordering Guarantees](./09-ordering-guarantees.md)
- [Failure Modes](./11-failure-modes.md)
