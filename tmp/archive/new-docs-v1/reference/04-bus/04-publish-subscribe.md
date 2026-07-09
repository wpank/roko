# Publish / Subscribe

> The pub/sub semantics of `Bus`: who can publish, how subscribers receive events, what
> ordering is guaranteed, and how backpressure works.

**Status**: Specified
**Crate**: `roko-core` (planned)
**Depends on**: [Trait Surface](./01-trait-surface.md), [Topics](./02-topics.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Any code holding a `Bus` handle can publish to any `Topic`. Subscribers receive events via
an async `Stream`. Delivery is at-most-once by default; the ring buffer provides replay for
at-least-once when needed. See [Delivery Semantics](./10-delivery-semantics.md) for the full
matrix.

---

## Publishing

```rust
// source: crates/roko-core/src/bus.rs  [target-state]
let seq = bus.publish(
    &Topic::new("loop.step.score"),
    Pulse::new(/* ... */),
).await?;
println!("published as seq {seq}");
```
<!-- source: crates/roko-core/src/bus.rs -->

Publishing:
1. Assigns a monotonically increasing sequence number within the topic.
2. Writes the `Pulse` to the topic's ring buffer.
3. Notifies all active subscribers whose `TopicFilter` matches the topic.

`publish` is non-blocking for the in-process backend — it writes to a channel and returns.
For distributed backends, it may block until the broker acknowledges receipt.

---

## Subscribing

```rust
// source: crates/roko-core/src/bus.rs  [target-state]
let mut stream = bus.subscribe(
    TopicFilter::Prefix("loop.step".into()),
    None, // no replay — only future events
).await?;

while let Some(pulse) = stream.next().await {
    // process pulse
}
```
<!-- source: crates/roko-core/src/bus.rs -->

Subscribing:
1. Creates a `Receiver` channel (or async stream) in the bus backend.
2. The stream delivers all future `Pulse`s matching the filter.
3. Dropping the stream unsubscribes automatically (no explicit unsubscribe needed).

---

## Subscribing with Replay

```rust
// source: crates/roko-core/src/bus.rs  [target-state]
let stream = bus.subscribe(
    TopicFilter::Exact(Topic::new("loop.step.score")),
    Some(ReplaySpec::Since(one_minute_ago_unix)),
).await?;
```
<!-- source: crates/roko-core/src/bus.rs -->

When `replay_from` is set, the stream first drains the ring buffer for matching past events,
then switches to live delivery. There is no gap between the replay window and live events.

---

## Delivery Order

Events on a single `Topic` are delivered to subscribers in publication order (FIFO). Events
across different `Topic`s have no ordering guarantee unless the subscriber explicitly
correlates sequence numbers. See [Ordering Guarantees](./09-ordering-guarantees.md).

---

## Backpressure

If a subscriber is slow to consume and its internal channel fills:

| Backend | Overflow behaviour |
|---|---|
| EventBus (today) | Drop oldest (ring-buffer eviction) |
| In-process Bus (planned) | Configurable: `Drop` \| `Block` \| `Error` |
| Distributed Bus (planned) | Consumer lag tracked; `Error` above a lag threshold |

The default overflow policy is `Drop` — the Bus never blocks the publisher. Subscribers that
need every event must consume fast enough to avoid lag.

---

## See Also

- [Delivery Semantics](./10-delivery-semantics.md)
- [Ordering Guarantees](./09-ordering-guarantees.md)
- [Replay and Ring](./05-replay-and-ring.md)

## Open Questions

- Should there be a `subscribe_with_ack` variant that requires explicit ACK for at-least-once
  delivery without replay?
