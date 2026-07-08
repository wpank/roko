# Bus API Reference

> Quick-reference for all `Bus` trait methods, `EventBus<E>` API, and supporting types.

**Status**: Specified (`Bus`); Shipping (`EventBus<E>`)
**Crate**: `roko-core` (Bus), `roko-runtime` (EventBus)
**Depends on**: [Trait Surface](./01-trait-surface.md)
**Last reviewed**: 2026-04-19

---

## `Bus` Trait Methods (Target State)

| Method | Signature | Returns |
|---|---|---|
| `publish` | `async publish(&self, topic: &Topic, pulse: Pulse) -> Result<u64, BusError>` | Sequence number or error |
| `subscribe` | `async subscribe(&self, filter: TopicFilter, replay_from: Option<ReplaySpec>) -> Result<BoxStream<Pulse>, BusError>` | Async stream |
| `replay` | `async replay(&self, topic: &Topic, spec: ReplaySpec) -> Result<BoxStream<Pulse>, BusError>` | Async stream |
| `len` | `async len(&self, topic: &Topic) -> Result<usize, BusError>` | Ring buffer depth |

---

## `EventBus<E>` API (Shipping)

```rust
// source: crates/roko-runtime/src/event_bus.rs
EventBus::<E>::new(capacity: usize) -> Self
bus.publish(event: E) -> usize         // returns subscriber count
bus.subscribe() -> broadcast::Receiver<E>
```
<!-- source: crates/roko-runtime/src/event_bus.rs -->

---

## `Topic`

```rust
// source: crates/roko-core/src/bus.rs  [target-state]
Topic::new(s: impl Into<String>) -> Topic
topic.as_str() -> &str
```
<!-- source: crates/roko-core/src/bus.rs -->

---

## `TopicFilter` Constructors

```rust
// source: crates/roko-core/src/bus.rs  [target-state]
TopicFilter::Exact(Topic::new("loop.step.score"))
TopicFilter::Prefix("agent.affect".into())
TopicFilter::Glob("loop.+.score".into())
```
<!-- source: crates/roko-core/src/bus.rs -->

---

## `ReplaySpec`

```rust
// source: crates/roko-core/src/bus.rs  [target-state]
ReplaySpec::Since(unix_ts: u64)
ReplaySpec::FromSeq(seq: u64)
ReplaySpec::LastN(n: usize)
```
<!-- source: crates/roko-core/src/bus.rs -->

---

## `BusError`

| Variant | Meaning |
|---|---|
| `TopicNotFound(Topic)` | Reserved for backends that require explicit topic creation |
| `TopicFull(Topic)` | Reject policy; ring buffer at capacity |
| `ChannelClosed` | Subscriber channel was closed |
| `Backend(String)` | Backend-specific error |

---

## See Also

- [Trait Surface](./01-trait-surface.md)
- [Publish / Subscribe](./04-publish-subscribe.md)
- [Today vs. Planned](./14-today-vs-planned.md)
