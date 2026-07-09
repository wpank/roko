# Pulse — Overview

> A Pulse is a lightweight, ephemeral event that flows across the Bus without being stored as an Engram.

**Status**: Specified  
**Crate**: `roko-core` (planned)  
**Depends on**: [Engram](../01-engram/00-overview.md), [Bus](../04-bus/README.md) (planned)  
**Last reviewed**: 2026-04-19

> **Legend:**  
> Shipped today: `EventBus<E>` — current live transport  
> Target state: `Pulse`, `Bus`, `Topic`, `TopicFilter`, `PulseSource` — no code yet

---

## TL;DR

The Roko system uses two kinds of records: Engrams (durable, stored in the Substrate)
and Pulses (ephemeral, routed over the Bus). Most runtime signals — heartbeat ticks,
probe results, prediction errors, gate events — do not need to persist. They are Pulses.
When a Pulse is significant enough to warrant durable storage, a subscriber graduates it
into an Engram.

---

## The Idea

In the shipped system, `EventBus<E>` handles all event routing. It is a generic,
in-process pub-sub mechanism where `E` is the event type. This works but it lacks two
properties:

1. **Named channels.** `EventBus<E>` routes on the Rust type `E`. There is no concept of
   a named topic that multiple event types can share, or of a filter that selects a subset
   of events from a topic.

2. **Origin attribution.** There is no built-in notion of "who produced this event." Events
   from the orchestrator, from external tools, and from chain nodes look identical to
   subscribers.

Pulse addresses both: it is a typed event envelope with a named `Topic`, a `TopicFilter`
subscription mechanism, and a `PulseSource` origin attribution field.

### Engram vs. Pulse: When to Use Which

| Question | Engram | Pulse |
|----------|--------|-------|
| Should it be retrievable tomorrow? | Yes → Engram | No → Pulse |
| Is it content-addressed? | Yes | No |
| Does it need scoring? | Yes | No (Pulses have no score axis) |
| Does it need decay? | Yes | No (Pulses have no decay; they expire on delivery) |
| Is it in the lineage DAG? | Yes | No (Pulses are not lineage parents) |
| Is it routed to multiple subscribers? | Via Substrate scan | Via Bus topic broadcast |
| Is it produced by the current heartbeat tick? | Possibly | Typically |

A useful heuristic: if you would ever want to retrieve this event by content hash or
by semantic similarity 24 hours later, it should be an Engram. If it is purely for
immediate notification, it should be a Pulse.

---

## Architecture Position

```
External world
     │
     ▼
[Source: agent / tool / sensor / chain]
     │
     │  Pulse  (ephemeral, typed, attributed)
     ▼
  [Bus] ──topic──► [Subscriber A]  ──graduate──► [Substrate] (Engram)
              │──► [Subscriber B]  (discard if not significant)
              └──► [Subscriber C]
```

The Bus receives Pulses, routes them by Topic and TopicFilter, and delivers them to
registered subscribers. Subscribers decide whether to graduate a Pulse into an Engram.
The graduation decision is governed by configurable rules (see
[`03-graduation-rules.md`](03-graduation-rules.md)).

---

## Today vs. Target State

See [`05-today-vs-planned.md`](05-today-vs-planned.md) for the full migration plan.

**Today:** `EventBus<E>` with type-based dispatch. No named topics, no filter subscriptions,
no `PulseSource`. The Engram type (as `Signal`) flows through both the Bus and the Substrate.

**Target state:** `Pulse` envelope with `Topic`, `TopicFilter`, and `PulseSource`. The
`EventBus<E>` is wrapped or replaced by `Bus`. Engrams are never sent over the Bus directly;
a `Pulse` wraps a reference or a `Datum` union type.

---

## See Also

- [`01-specification.md`](01-specification.md) — Pulse struct fields and lifecycle
- [`../01-engram/00-overview.md`](../01-engram/00-overview.md) — Engram (durable counterpart)
- [`05-today-vs-planned.md`](05-today-vs-planned.md) — migration plan
