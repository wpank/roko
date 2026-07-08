# Bus Performance

> Throughput targets, latency budget, and hot-path analysis for `Bus`.

**Status**: Specified
**Crate**: `roko-core` (planned)
**Depends on**: [Trait Surface](./01-trait-surface.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

The in-process Bus target is < 5 µs per `publish` at < 64 subscribers and < 1 million
events/second. `EventBus<E>` today achieves ~1–2 µs per publish. The distributed backend
targets < 5 ms for broker-acknowledged publish.

---

## Target Latency SLAs

<!-- ADDED -->

| Operation | Backend | Target P99 |
|---|---|---|
| `publish` | In-process | < 5 µs |
| `publish` | Distributed (ack) | < 5 ms |
| `subscribe` setup | In-process | < 100 µs |
| `replay(LastN=1024)` | In-process | < 2 ms |
| Event fan-out to 16 subscribers | In-process | < 10 µs total |

---

## Hot Path: `publish` on Every Loop Tick

The cognitive loop publishes to `loop.step.*` topics on every tick. At Gamma speed (sub-
second ticks), this happens 1–10 times per second per agent. At 10 agents on one machine,
10–100 publishes/second — well within the in-process budget.

The hot path for a single `publish`:
1. TopicFilter matching for all active subscribers: O(n_subscribers × depth).
2. Write to each matching subscriber's channel: O(n_matching).
3. Write to ring buffer: O(1).

Total: O(n_subscribers × depth + n_matching).

At 16 subscribers (typical), depth 3: ~50 comparisons. Sub-µs.

---

## Memory Overhead

Each topic ring buffer:
- `ring_size` × `sizeof(Pulse)` bytes.
- Default: 1,024 × ~256 bytes = ~256 KB per topic.
- 20 standard topics: ~5 MB total.

Subscriber channels:
- `capacity` × `sizeof(Arc<Pulse>)` bytes.
- Default capacity 256: ~2 KB per subscriber.
- 16 subscribers: ~32 KB.

Total Bus memory: ~5–10 MB at typical agent scale. Negligible.

---

## `EventBus<E>` Baseline

Today's `EventBus<E>` using `tokio::sync::broadcast` achieves:
- Publish: ~1–2 µs (channel send).
- Receive: ~1 µs (channel recv).
- Fan-out: O(n_subscribers) channel clones — auto-handled by `broadcast`.

The target-state `Bus` should match or beat these numbers after the routing overhead.

---

## See Also

- [Today vs. Planned](./14-today-vs-planned.md)
- [Failure Modes](./11-failure-modes.md)

## Open Questions

- What is the real-world latency overhead of `TopicFilter::Glob` matching at scale?
  Should a pre-compiled automaton replace the recursive matcher for hot paths?
