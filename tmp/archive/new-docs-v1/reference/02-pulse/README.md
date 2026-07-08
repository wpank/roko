# Pulse — Ephemeral Event Medium

> Pulse is the target-state ephemeral transport medium. This folder is the canonical reference.

**Status**: Specified (no code yet)  
**Crate**: `roko-core` (planned)  
**Last reviewed**: 2026-04-19

> **Legend:**  
> Shipped today: `EventBus<E>` (live transport)  
> Target state: `Pulse`, `Bus`, `Topic`, `TopicFilter`, `PulseSource`

---

## What Is a Pulse?

A Pulse is a lightweight, ephemeral event that carries signals between components without
being stored as an Engram. Where an Engram is a durable record that persists in the
Substrate, a Pulse is a transient message that is delivered to subscribers and then
discarded (unless it meets the conditions for graduation into an Engram).

The key distinction:
- **Engram**: durable, content-addressed, scored, decaying, in the Substrate.
- **Pulse**: ephemeral, routed, transient, on the Bus.

---

## Contents

| # | Page | What it covers | Status |
|---|------|----------------|--------|
| [00](00-overview.md) | Overview | What Pulse is; Engram vs Pulse distinction | Specified |
| [01](01-specification.md) | Specification | Pulse struct, fields, lifecycle | Specified |
| [02](02-topics-and-filters.md) | Topics & filters | Topic and TopicFilter routing | Specified |
| [03](03-graduation-rules.md) | Graduation rules | When a Pulse becomes an Engram | Specified |
| [04](04-pulse-sources.md) | Pulse sources | PulseSource origin attribution | Specified |
| [05](05-today-vs-planned.md) | Today vs. planned | EventBus today; Pulse/Bus target; migration | — |
| [06](06-examples.md) | Examples | Example Pulses and routing | Specified |
| [07](07-open-questions.md) | Open questions | Unresolved design decisions | — |

---

## Suggested Reading Order

**New to the event system:** 00 → 05 → 02 → 01  
**Implementing a subscriber:** 01 → 02 → 03  
**Migration planning:** 05 → 07  

---

## See Also

- [`reference/01-engram/README.md`](../01-engram/README.md) — durable records (Engram)
- [`reference/04-bus/README.md`](../04-bus/README.md) — the Bus transport fabric (planned)
