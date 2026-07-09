# Bus Overview

> `Bus` is the transport-fabric trait for Roko's ephemeral `Pulse` events. It is the planned
> abstraction over message passing — today, `EventBus<E>` ships this role; `Bus` is the
> target-state trait that will let Roko swap transport backends the way `Substrate` lets it
> swap storage backends.

**Status**: Specified
**Crate**: `roko-core` (planned trait), `roko-runtime` (EventBus shipping)
**Depends on**: [Pulse](../02-pulse/README.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

`Bus` is to ephemeral events what `Substrate` is to durable memories. Agents publish `Pulse`
records to named `Topic`s; other agents or operators subscribe and receive them. Today,
`EventBus<E>` fills this role as a concrete type. The `Bus` trait is specified but not yet
coded; this folder documents the target state.

---

## Two Mediums, Two Fabrics (Recap)

| Medium | Lifetime | Fabric | Status |
|---|---|---|---|
| `Engram` | Durable | `Substrate` | Shipping |
| `Pulse` | Ephemeral | `Bus` (target) / `EventBus<E>` (today) | Specified / Shipping |

A `Pulse` is a short-lived event — a trigger, a signal, a notification. It is not persisted
by default (though it may be graduated to an `Engram` by the graduation pipeline; see
[Pulse](../02-pulse/README.md)). Bus is the channel through which `Pulse`s flow between
producers and consumers.

---

## The Idea

Without a Bus abstraction, operators communicate through direct function calls or through
the concrete `EventBus<E>` type. This has two failure modes:

1. **Tight coupling** — operator A must know operator B's type and call it directly.
   Adding or removing operators requires changing both sides.
2. **Testability** — unit-testing an operator that publishes events requires spinning up the
   full `EventBus<E>` machinery.

`Bus` breaks both couplings. Operators publish to a `Topic` and subscribe to a `TopicFilter`.
They never know who else is listening or publishing.

---

## Topic-Based Routing

The central routing concept is a `Topic` — a hierarchical name like `agent.cognition.recall`
or `agent.affect.valence`. Publishers write to a topic; subscribers match topics with a
`TopicFilter` (exact, prefix, or glob). This is analogous to MQTT topic routing.

See [Topics](./02-topics.md) and [Topic Filters](./03-topic-filters.md).

---

## Replay and Ring Buffer

Unlike a simple event queue, `Bus` supports replay: a subscriber can request all events on
a topic from the last N seconds, or from a given event sequence number. This allows late-
joining agents and the Delta-speed consolidation loop to process past events. Replay is
backed by a ring buffer per topic.

See [Replay and Ring](./05-replay-and-ring.md).

---

## Today vs. Planned

| Today (`EventBus<E>`) | Target (`Bus`) |
|---|---|
| Shipping | Specified |
| Generic over event type `E` | Typed via `Pulse` |
| No `Topic` abstraction | `Topic` + `TopicFilter` routing |
| No replay | Ring-buffer replay |
| Single-process only | Distributed backend planned |
| Concrete type | Trait (swappable backend) |

The migration path is described in full in [Today vs. Planned](./14-today-vs-planned.md).

---

## Why a Trait?

Same reasoning as `Substrate` — see [Substrate Rationale](../03-substrate/16-rationale.md).
The key point: tests need a lightweight in-process bus; production may use a distributed
broker. A trait makes both possible without changing operator code.

---

## See Also

- [Trait Surface](./01-trait-surface.md)
- [Today vs. Planned](./14-today-vs-planned.md)
- [Substrate Overview](../03-substrate/00-overview.md)
- [Pulse](../02-pulse/README.md)

## Open Questions

- Should `Bus` and `Substrate` share a common `Fabric` supertrait with shared capability
  discovery methods?
- When `Bus` is shipped, should `EventBus<E>` be immediately deprecated or kept as a
  first-class backend?
