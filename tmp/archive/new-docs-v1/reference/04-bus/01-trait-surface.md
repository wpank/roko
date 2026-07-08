# Bus — Trait Surface

> The target-state Rust trait signature for `Bus`, with every method and return type
> annotated. This is a specification — no code has shipped yet.

**Status**: Specified
**Crate**: `roko-core` (planned)
**Depends on**: [Overview](./00-overview.md), [Topics](./02-topics.md), [Topic Filters](./03-topic-filters.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

`Bus` is an async-first, object-safe Rust trait with four methods: `publish`, `subscribe`,
`replay`, and `len`. Backends implement `Bus`; operators call it without knowing the backend.

---

## The Trait (Target State)

```rust
// source: crates/roko-core/src/bus.rs  [target-state, not yet shipped]

/// Transport fabric for ephemeral [`Pulse`] events.
///
/// Every Roko agent holds a `Box<dyn Bus>` (or `Arc<dyn Bus>`).
/// Operators publish [`Pulse`]s to named [`Topic`]s; other operators subscribe
/// via [`TopicFilter`]s and receive matching events.
///
/// # Object safety
/// The trait is object-safe. All methods operate on concrete types.
///
/// # Async
/// `Bus` is async-first (unlike the current sync `Substrate`). All I/O-touching
/// operations are `async`.
#[async_trait]
pub trait Bus: Send + Sync {
    /// Publish a [`Pulse`] to a [`Topic`].
    ///
    /// Returns `Ok(seq)` where `seq` is the sequence number assigned to this
    /// publication within the topic's ring buffer.
    ///
    /// # Errors
    /// Returns `BusError::TopicFull` if the ring buffer is at capacity and
    /// the backend's overflow policy is `Reject`.
    async fn publish(
        &self,
        topic: &Topic,
        pulse: Pulse,
    ) -> Result<u64, BusError>;

    /// Subscribe to all [`Pulse`]s matching a [`TopicFilter`].
    ///
    /// Returns a `Receiver` handle. The subscriber receives all future events
    /// matching the filter. Past events are not included unless `replay_from`
    /// is set.
    ///
    /// Dropping the receiver unsubscribes automatically.
    async fn subscribe(
        &self,
        filter: TopicFilter,
        replay_from: Option<ReplaySpec>,
    ) -> Result<BoxStream<'static, Pulse>, BusError>;

    /// Replay past events from a topic's ring buffer.
    ///
    /// `spec` controls the time window or sequence range to replay.
    /// Returns a `Stream` of `Pulse`s in publication order.
    async fn replay(
        &self,
        topic: &Topic,
        spec: ReplaySpec,
    ) -> Result<BoxStream<'static, Pulse>, BusError>;

    /// Return the current number of events in the ring buffer for `topic`.
    ///
    /// Returns `Ok(0)` for topics that do not exist or are empty.
    async fn len(&self, topic: &Topic) -> Result<usize, BusError>;
}
```
<!-- source: crates/roko-core/src/bus.rs -->

---

## Supporting Types

### `Topic`

```rust
// source: crates/roko-core/src/bus.rs  [target-state]
/// A hierarchical dot-separated name for a Pulse stream.
/// Example: `"agent.cognition.recall"`, `"agent.affect.valence"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Topic(String);
```
<!-- source: crates/roko-core/src/bus.rs -->

### `TopicFilter`

```rust
// source: crates/roko-core/src/bus.rs  [target-state]
/// A subscription matcher: exact, prefix, or glob pattern.
#[derive(Debug, Clone)]
pub enum TopicFilter {
    /// Matches exactly one topic.
    Exact(Topic),
    /// Matches all topics with this prefix (e.g., `"agent.cognition.*"`).
    Prefix(String),
    /// MQTT-style glob: `+` = one segment, `#` = any suffix.
    Glob(String),
}
```
<!-- source: crates/roko-core/src/bus.rs -->

### `ReplaySpec`

```rust
// source: crates/roko-core/src/bus.rs  [target-state]
pub enum ReplaySpec {
    /// Replay all events since this UNIX timestamp.
    Since(u64),
    /// Replay from this sequence number (inclusive).
    FromSeq(u64),
    /// Replay the last N events.
    LastN(usize),
}
```
<!-- source: crates/roko-core/src/bus.rs -->

### `BusError`

```rust
// source: crates/roko-core/src/bus.rs  [target-state]
#[derive(Debug, thiserror::Error)]
pub enum BusError {
    #[error("topic not found: {0}")]
    TopicNotFound(Topic),
    #[error("ring buffer full for topic: {0}")]
    TopicFull(Topic),
    #[error("subscriber channel closed")]
    ChannelClosed,
    #[error("backend error: {0}")]
    Backend(String),
}
```
<!-- source: crates/roko-core/src/bus.rs -->

---

## Method Summary

| Method | Returns | Description |
|---|---|---|
| `publish(topic, pulse)` | `Result<u64, BusError>` | Publish a Pulse; returns sequence number |
| `subscribe(filter, replay_from)` | `Result<BoxStream<Pulse>, BusError>` | Subscribe to matching Pulses |
| `replay(topic, spec)` | `Result<BoxStream<Pulse>, BusError>` | Replay past Pulses from ring buffer |
| `len(topic)` | `Result<usize, BusError>` | Ring buffer depth for a topic |

---

## See Also

- [Topics](./02-topics.md) — Topic naming rules
- [Topic Filters](./03-topic-filters.md) — matching semantics
- [Replay and Ring](./05-replay-and-ring.md) — `ReplaySpec` in depth
- [Today vs. Planned](./14-today-vs-planned.md) — how `EventBus<E>` maps to this trait

## Open Questions

- Should `publish` be fire-and-forget (no seq return) for better performance, with a
  separate `publish_ack` for guaranteed delivery?
- Should `Bus` expose a `topics()` method to enumerate live topics?
