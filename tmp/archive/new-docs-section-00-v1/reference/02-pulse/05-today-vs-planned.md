# Pulse — Today vs. Planned

> The current EventBus<E> implementation, the target-state Pulse/Bus design, and the migration path between them.

**Status**: Shipping (EventBus<E>) / Specified (Pulse/Bus)  
**Crate**: `roko-runtime` (EventBus), `roko-core` (planned Pulse/Bus)  
**Last reviewed**: 2026-04-19

---

## TL;DR

Today: `EventBus<E>` dispatches typed Rust events by type identity. It works but has no
named topics, no subscription filters, and no origin attribution. The Pulse/Bus design
adds all of these. Migration is forward-compatible: components that use `EventBus<E>`
today will wrap it in a `Bus` adapter.

---

## Today: EventBus&lt;E&gt;

```rust
<!-- source: crates/roko-runtime/src/event_bus.rs -->

/// Current shipped event transport.
/// Generic over the event type E.
/// Subscribers register handlers; events are dispatched to all handlers.
pub struct EventBus<E: Clone + Send + Sync + 'static> {
    // ...
}

impl<E: Clone + Send + Sync + 'static> EventBus<E> {
    pub fn new() -> Self;
    pub fn subscribe(&self, handler: impl Fn(E) + Send + Sync + 'static) -> SubscriptionId;
    pub fn unsubscribe(&self, id: SubscriptionId);
    pub fn emit(&self, event: E);
}
```

**Limitations:**

1. **No named topics.** Routing is by Rust type `E`. Two different event categories must
   have two different `EventBus<E>` instances; there is no wildcard subscription across
   them.

2. **No origin attribution.** Events carry no information about who produced them.
   A handler cannot distinguish an event from the orchestrator vs. from a chain node.

3. **No filter subscriptions.** Subscribers receive all events of type `E`. There is
   no "give me gate failures but not gate passes."

---

## Target State: Pulse/Bus

```rust
<!-- source: crates/roko-core/src/bus.rs (target state — not yet implemented) -->

/// Target-state transport fabric.
pub trait Bus: Send + Sync {
    fn subscribe<P, F>(
        &self,
        filter: TopicFilter,
        handler: F,
    ) -> SubscriptionHandle
    where
        P: 'static,
        F: Fn(Pulse<P>) + Send + Sync + 'static;

    fn unsubscribe(&self, handle: SubscriptionHandle);

    fn emit<P: 'static>(&self, pulse: Pulse<P>);
}
```

**Additions over EventBus<E>:**

1. `TopicFilter` for named-topic and wildcard subscriptions.
2. `PulseSource` on every event.
3. `CorrelationId` for distributed tracing.
4. `Pulse<P>` is typed but topic-routed, not type-routed.

---

## Migration Path

The migration is designed to be incremental and non-breaking:

**Phase 1 (current):** `EventBus<E>` is the only transport.

**Phase 2:** Introduce `Pulse<P>` and `Bus` as new types alongside `EventBus<E>`.
Components can emit Pulses to the new Bus; `EventBus<E>` continues to work unchanged.

**Phase 3:** An adapter wraps `EventBus<E>` in the Bus interface. Components that have
not migrated still emit typed events through the adapter, which wraps them in
`Pulse<Box<dyn Any>>` with `PulseSource::Subsystem`.

**Phase 4:** Components migrate to emit `Pulse<P>` directly. `EventBus<E>` adapter
is deprecated.

**Phase 5:** `EventBus<E>` removed from the codebase.

---

## See Also

- [`00-overview.md`](00-overview.md) — Engram vs Pulse distinction
- [`01-specification.md`](01-specification.md) — target Pulse spec
- [`07-open-questions.md`](07-open-questions.md) — migration open questions
