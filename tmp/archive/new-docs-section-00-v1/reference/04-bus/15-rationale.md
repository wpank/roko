# Bus Rationale

> Why `Bus` is a trait, why `Topic`-based routing was chosen, and what alternatives were
> rejected.

**Status**: Specified
**Crate**: `roko-core` (planned)
**Depends on**: [Overview](./00-overview.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

A trait was chosen for the same reason as `Substrate` — backend swappability and testability.
Topic-based routing was chosen over direct operator calls or type-based dispatch because it
decouples publishers from subscribers and scales to distributed deployments.

---

## Why a Trait (Not a Concrete Type)

See [Substrate Rationale](../03-substrate/16-rationale.md) for the general argument; it
applies equally here. The Bus-specific reasons are:

1. **Test isolation** — tests of a single operator (e.g., a `Gate`) should not require
   spinning up a full bus implementation. A `MockBus: Bus` can record published events and
   assert on them.
2. **Distributed upgrade path** — when Roko scales to multi-machine deployments, replacing
   the in-process bus with a distributed broker should not touch operator code.
3. **Observability** — an instrumented `TracingBus: Bus` can wrap any real backend and add
   tracing / metrics without modifying operators.

---

## Why Topic-Based Routing (Not Direct Calls)

The alternative to a topic bus is direct operator wiring: Scorer calls `gate.evaluate(score)`
directly. This works for small, static pipelines but fails to:

1. **Support dynamic operators** — with topics, operators can be added, removed, or replaced
   at runtime without re-wiring the pipeline.
2. **Enable cross-cutting observers** — a Policy operator needs to observe all events on all
   topics. With direct calls, it would need to be injected into every operator. With topics,
   it subscribes to `TopicFilter::Prefix("agent")`.
3. **Support distributed fan-out** — topics naturally map to broker subjects. Direct calls do
   not.

---

## Why Not Type-Based Dispatch

Another alternative is a type-erased event bus where subscribers register by event type
(`bus.subscribe::<ScoreEvent>()`). This is what `EventBus<E>` approximates today.

Type-based dispatch:
- Works for single-process, single-type buses.
- Does not scale to heterogeneous events from multiple operators with different types.
- Cannot express routing at a finer grain than "all events of type T".

Topic-based routing subsumes type-based dispatch: a topic like `loop.step.score` is more
specific than the type `ScoreEvent`, because it carries location (the loop step) as well as
meaning.

---

## Why Not an Actor Model

The actor model (Erlang/Akka style) is a valid alternative: each operator is an actor; events
are messages sent to actor mailboxes.

Roko chose a bus over actors because:
- Rust's ownership model makes the actor pattern verbose without a framework.
- The cognitive loop is primarily sequential, not inherently concurrent. A bus fits the
  fan-out pattern better than point-to-point actor messaging.
- Operators are stateless transforms, not stateful actors.

---

## See Also

- [Overview](./00-overview.md)
- [Substrate Rationale](../03-substrate/16-rationale.md)
- [Today vs. Planned](./14-today-vs-planned.md)

## Open Questions

- Should `Bus` support request-reply semantics (publish a request, await a correlated
  reply)? Or is that always out of scope for the core trait?
