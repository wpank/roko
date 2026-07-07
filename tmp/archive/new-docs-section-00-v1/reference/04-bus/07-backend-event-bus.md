# Backend: EventBus\<E\>

> `EventBus<E>` is the shipping in-process event bus. It is generic over an event type `E`,
> uses `tokio::sync::broadcast` channels internally, and has been the Roko transport layer
> since the initial release.

**Status**: Shipping
**Crate**: `roko-runtime`
**Depends on**: [Backends Overview](./06-backends-overview.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

`EventBus<E>` is a `broadcast::Sender<E>` wrapper. Any event type `E: Clone + Send` can be
transported. Subscribers receive all events from the time they subscribe onwards. No replay,
no topic routing, no filters — but zero overhead and zero configuration.

---

## Structure

```rust
// source: crates/roko-runtime/src/event_bus.rs

pub struct EventBus<E: Clone + Send + 'static> {
    sender: broadcast::Sender<E>,
}

impl<E: Clone + Send + 'static> EventBus<E> {
    /// Create a new bus with a given broadcast channel capacity.
    /// Events beyond capacity are dropped (oldest first).
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Publish an event to all current subscribers.
    /// Returns the number of subscribers that received the event.
    pub fn publish(&self, event: E) -> usize {
        self.sender.send(event).unwrap_or(0)
    }

    /// Subscribe to future events.
    /// The returned receiver receives all events published after this call.
    pub fn subscribe(&self) -> broadcast::Receiver<E> {
        self.sender.subscribe()
    }
}
```
<!-- source: crates/roko-runtime/src/event_bus.rs -->

---

## Characteristics

| Property | Value |
|---|---|
| Event type | Generic `E: Clone + Send` |
| Transport | In-process `tokio::sync::broadcast` channel |
| Delivery | At-most-once (oldest-first drop when full) |
| Replay | None |
| Topic routing | None — all subscribers receive all events |
| Ordering | Publication order within the channel |
| Capacity | Configurable; default varies by runtime use |

---

## Limitations vs. Target Bus Trait

| Limitation | Target Bus Trait |
|---|---|
| Generic `E` — no typed `Pulse` | `Pulse` type with topic routing |
| No topic hierarchy | `Topic` + `TopicFilter` |
| No replay | Ring buffer replay |
| No `len` per topic | `len(topic)` method |

---

## Usage Pattern

```rust
// source: crates/roko-runtime/src/event_bus.rs
use roko_runtime::EventBus;
use roko_core::SomeEvent;

let bus: EventBus<SomeEvent> = EventBus::new(1024);

// Publisher:
bus.publish(SomeEvent { /* ... */ });

// Subscriber (in another task):
let mut rx = bus.subscribe();
tokio::spawn(async move {
    while let Ok(event) = rx.recv().await {
        // handle event
    }
});
```
<!-- source: crates/roko-runtime/src/event_bus.rs -->

---

## See Also

- [Today vs. Planned](./14-today-vs-planned.md) — migration path to the Bus trait
- [Backends Overview](./06-backends-overview.md)

## Open Questions

- Should `EventBus<E>` implement the `Bus` trait once it ships (as a compatibility shim)?
