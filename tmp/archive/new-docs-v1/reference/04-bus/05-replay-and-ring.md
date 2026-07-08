# Replay and Ring Buffer

> Each `Topic` maintains a ring buffer of recent `Pulse`s. Subscribers can replay this
> buffer to catch up on events they missed. This page covers the ring buffer structure,
> replay semantics, and configuration.

**Status**: Specified
**Crate**: `roko-core` (planned)
**Depends on**: [Trait Surface](./01-trait-surface.md), [Topics](./02-topics.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Each topic keeps a ring buffer of the last N events (configurable, default: 1,024 per topic).
Subscribers request replay via `ReplaySpec` (by time, by sequence number, or by count).
Replay and live delivery are seamlessly joined — no gap.

---

## The Ring Buffer

Every `Topic` has a fixed-size ring buffer:

```
┌──────────────────────────────────────────────────────────┐
│  Pulse[seq=100]  Pulse[seq=101]  ...  Pulse[seq=1123]    │
│  oldest ←────────────────────────────────────→ newest    │
│  (evicted when buffer full)                               │
└──────────────────────────────────────────────────────────┘
capacity = ring_size (default: 1024)
```

When the buffer is full, the oldest event is evicted (FIFO). Sequence numbers are
monotonically increasing and never reset; they are not recycled when the buffer wraps.

---

## Replay Specs

Three ways to specify what to replay:

| `ReplaySpec` | Semantics |
|---|---|
| `Since(unix_ts)` | All events with `pulse.timestamp >= unix_ts`. Time-based window. |
| `FromSeq(seq)` | All events with sequence number ≥ seq. |
| `LastN(n)` | The last n events in the buffer (or all if fewer than n exist). |

---

## Replay + Live Join

When a subscriber provides `replay_from`:

1. The bus drains the ring buffer for matching events in order.
2. It switches to live delivery seamlessly.
3. If new events arrived during the drain, they are queued and delivered after the replay
   completes — no event is dropped between replay and live.

This is the key property that makes replay useful: a late-joining subscriber gets a
consistent view from the past to the present.

---

## Explicit Replay

Replay can also be requested independently of subscribing:

```rust
// source: crates/roko-core/src/bus.rs  [target-state]
let mut past_events = bus.replay(
    &Topic::new("loop.step.score"),
    ReplaySpec::LastN(256),
).await?;

while let Some(pulse) = past_events.next().await {
    // process historical event
}
```
<!-- source: crates/roko-core/src/bus.rs -->

This is used by the Delta-speed consolidation loop (Dreams) to process the last N loop ticks
without requiring live subscription.

---

## Configuration

<!-- ADDED -->

| Parameter | Default | Description |
|---|---|---|
| `ring_size` | 1,024 | Maximum events per topic ring buffer |
| `ring_ttl_secs` | 3,600 (1 hour) | Events older than this are eligible for eviction even if the buffer is not full |
| `replay_max_events` | 4,096 | Maximum events deliverable in a single replay request |

---

## Failure Modes

| Failure | Behaviour |
|---|---|
| Replay requested beyond buffer window | Returns only the available events (no error). The subscriber must handle gaps. |
| `FromSeq` references an evicted sequence | Returns events from the oldest available sequence. |
| Ring buffer full, new event arrives | Oldest event evicted (FIFO). |

---

## See Also

- [Publish / Subscribe](./04-publish-subscribe.md)
- [Delivery Semantics](./10-delivery-semantics.md)
- [Ordering Guarantees](./09-ordering-guarantees.md)

## Open Questions

- Should the ring buffer be per-topic or per-filter (subscriber-local buffer)?
- Should `ring_ttl_secs` be configurable per topic rather than globally?
