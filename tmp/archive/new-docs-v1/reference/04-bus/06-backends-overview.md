# Bus Backends Overview

> Two backend families are planned for `Bus`: the in-process `EventBus<E>` (today, shipping)
> and a distributed backend (planned). This page summarises the trade-offs and when to use
> each.

**Status**: Specified (overview); Shipping (EventBus<E>)
**Crate**: `roko-runtime` (`EventBus<E>`)
**Depends on**: [Overview](./00-overview.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

| Backend | Status | Use case |
|---|---|---|
| `EventBus<E>` (in-process) | Shipping | Single-process agents; today's default |
| `InProcessBus` (planned Bus trait impl) | Specified | Single-process; replaces EventBus<E> |
| Distributed Bus | Specified | Multi-process / multi-machine agents |

---

## Backend Comparison

| Property | EventBus (today) | InProcessBus (planned) | Distributed (planned) |
|---|---|---|---|
| Transport | In-process channels | In-process channels | Network (broker) |
| Latency | ~µs | ~µs | ~ms |
| Replay | No | Yes (ring buffer) | Yes (broker log) |
| Topic routing | No | Yes | Yes |
| Multi-process | No | No | Yes |
| Status | Shipping | Specified | Specified |

---

## Choosing a Backend

**Use `EventBus<E>` today** — it is what ships. No configuration needed; the runtime wires
it automatically.

**Use `InProcessBus` when** (future):
- The `Bus` trait has shipped.
- You need replay semantics for the Delta-speed loop.
- You are still running a single process.

**Use `Distributed Bus` when** (future):
- Multiple agents run across processes or machines and need shared event routing.
- You need durable-until-ack delivery (at-least-once across process restarts).

---

## See Also

- [Backend: EventBus](./07-backend-event-bus.md)
- [Backend: Distributed](./08-backend-distributed.md)
- [Today vs. Planned](./14-today-vs-planned.md)
