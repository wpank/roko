# Bus — Transport Fabric

> `Bus` is the planned transport-fabric trait in Roko. It is the seam between agent logic
> and how `Pulse` events are published, routed, and consumed. Today, `EventBus<E>` is the
> shipping implementation; `Bus` is the target-state abstraction that will eventually
> supersede it.

**Status**: Specified (`Bus` trait); Shipping (`EventBus<E>`)
**Crate**: `roko-runtime` (`EventBus<E>`); `roko-core` (planned trait)
**Depends on**: [Engram](../01-engram/README.md), [Pulse](../02-pulse/README.md)
**Last reviewed**: 2026-04-19

---

## What This Folder Contains

`Bus` is the ephemeral counterpart to [`Substrate`](../03-substrate/README.md). Where
Substrate is for durable `Engram` records, Bus is for short-lived `Pulse` events — things
that happen now and do not need to persist. The folder covers the target-state `Bus` trait,
Topic-based routing, replay semantics, and the migration from today's `EventBus<E>` to the
Bus target state.

## Contents

| # | Page | What it covers | Status |
|---|---|---|---|
| 00 | [Overview](./00-overview.md) | What Bus is, why a transport-fabric trait, today vs. planned | Specified |
| 01 | [Trait Surface](./01-trait-surface.md) | Target-state Bus trait signature | Specified |
| 02 | [Topics](./02-topics.md) | Topic naming, hierarchy, wildcards | Specified |
| 03 | [Topic Filters](./03-topic-filters.md) | TopicFilter semantics, matching rules | Specified |
| 04 | [Publish / Subscribe](./04-publish-subscribe.md) | Pub/sub semantics, delivery guarantees | Specified |
| 05 | [Replay and Ring](./05-replay-and-ring.md) | Ring-buffer replay, time-window replay | Specified |
| 06 | [Backends Overview](./06-backends-overview.md) | Backend families | Specified |
| 07 | [Backend: EventBus](./07-backend-event-bus.md) | `EventBus<E>` (shipping) as a Bus implementer | Shipping |
| 08 | [Backend: Distributed](./08-backend-distributed.md) | Distributed backend (planned) | Specified |
| 09 | [Ordering Guarantees](./09-ordering-guarantees.md) | Per-topic, total, causal ordering | Specified |
| 10 | [Delivery Semantics](./10-delivery-semantics.md) | At-most-once, at-least-once, exactly-once | Specified |
| 11 | [Failure Modes](./11-failure-modes.md) | Dropped messages, lag, reconnection | Specified |
| 12 | [Performance](./12-performance.md) | Throughput targets, latency budget | Specified |
| 13 | [API Reference](./13-api-reference.md) | Quick-reference for trait methods | Specified |
| 14 | [Today vs. Planned](./14-today-vs-planned.md) | `EventBus<E>` today → `Bus` target; migration path | Shipping |
| 15 | [Rationale](./15-rationale.md) | Why Bus as a trait; alternatives rejected | Specified |

## Suggested Reading Order

**First-time reader**: 00 → 02 → 04 → 14.

**Implementer (writing a new Bus backend)**: 00 → 01 → 02 → 03 → 09 → 10 → 11.

**Migration (EventBus → Bus)**: 14 → 00 → 01.

## See Also

- [Substrate](../03-substrate/README.md) — the sibling durable-storage fabric
- [Pulse](../02-pulse/README.md) — the ephemeral event type carried by Bus
- [Universal Cognitive Loop](../06-loop/README.md) — where Bus events flow
