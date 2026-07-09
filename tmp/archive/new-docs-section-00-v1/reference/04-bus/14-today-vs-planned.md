# Today vs. Planned — EventBus\<E\> → Bus Migration

> A precise mapping from today's `EventBus<E>` to the target-state `Bus` trait, with
> migration steps.

**Status**: Shipping (EventBus<E>); Specified (Bus)
**Crate**: `roko-runtime`, `roko-core`
**Depends on**: [Overview](./00-overview.md), [Trait Surface](./01-trait-surface.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

`EventBus<E>` ships today and handles in-process event fan-out. The `Bus` trait is the
target abstraction — once shipped, `EventBus<E>` becomes a `Bus` backend. Migration is
additive; existing `EventBus<E>` callers can be updated one at a time.

---

## Feature Mapping

| Feature | EventBus<E> today | Bus target state |
|---|---|---|
| Transport | In-process broadcast channel | Pluggable (in-process, distributed) |
| Event type | Generic `E: Clone + Send` | `Pulse` (concrete, with topic metadata) |
| Routing | None — all subscribers receive all events | `Topic` + `TopicFilter` |
| Replay | None | Ring-buffer replay with `ReplaySpec` |
| Async API | Async receive (tokio) | Async publish + subscribe |
| Backend swap | Not possible (concrete type) | `Box<dyn Bus>` / `Arc<dyn Bus>` |
| Configuration | `capacity: usize` | `ring_size`, `ring_ttl_secs`, overflow policy |
| Multi-process | No | Yes (distributed backend) |

---

## Today: How the Runtime Uses EventBus

```rust
// source: crates/roko-runtime/src/agent.rs
// Current wiring (simplified):
let bus: EventBus<LoopEvent> = EventBus::new(1024);

// Loop step emits:
bus.publish(LoopEvent::ScoreComputed { engram_id, score });

// Operator subscribes:
let mut rx = bus.subscribe();
tokio::spawn(async move {
    while let Ok(event) = rx.recv().await {
        match event {
            LoopEvent::ScoreComputed { .. } => { /* ... */ }
            _ => {}
        }
    }
});
```
<!-- source: crates/roko-runtime/src/agent.rs -->

Filtering is done inside subscriber match arms — there is no topic routing.

---

## Target: How the Runtime Will Use Bus

```rust
// source: crates/roko-runtime/src/agent.rs  [target-state]
// Target wiring (simplified):
let bus: Arc<dyn Bus> = Arc::new(InProcessBus::new(BusConfig::default()));

// Loop step emits:
bus.publish(&Topic::new("loop.step.score"), Pulse::new(/* ... */)).await?;

// Operator subscribes (only receives score events):
let stream = bus.subscribe(
    TopicFilter::Exact(Topic::new("loop.step.score")),
    None,
).await?;
```
<!-- source: crates/roko-runtime/src/agent.rs -->

Topic routing replaces in-subscriber filtering.

---

## Migration Path

1. **Step 1** — Ship the `Bus` trait and `InProcessBus` backend (`roko-core`).
2. **Step 2** — Add a `CompatBus<E>` adapter that wraps `EventBus<E>` and implements `Bus`.
   This allows existing call sites to keep working.
3. **Step 3** — Migrate loop steps from `bus.publish(LoopEvent::X)` to
   `bus.publish(&Topic::new("loop.step.x"), Pulse::new(...))` one step at a time.
4. **Step 4** — Migrate operator subscribers from `rx.recv()` with match arms to
   `bus.subscribe(TopicFilter::Exact(...))`.
5. **Step 5** — Remove `CompatBus<E>` once all callers are migrated.
6. **Step 6** — Mark `EventBus<E>` as deprecated; keep as a `Bus` backend implementation.

---

## What Does Not Change

- The cognitive loop structure and step names remain the same.
- `Engram` and `Substrate` are unaffected.
- Agent configuration files remain compatible.
- Existing operator traits (`Scorer`, `Gate`, etc.) do not need to change unless they
  directly call `EventBus<E>`.

---

## See Also

- [Trait Surface](./01-trait-surface.md)
- [Backend: EventBus](./07-backend-event-bus.md)
- [Overview](./00-overview.md)
