# Backend: Distributed Bus

> The planned distributed Bus backend enables multi-process and multi-machine agents to
> share `Pulse` events through an external broker. Status: Specified (no code).

**Status**: Specified
**Crate**: — (not yet assigned)
**Depends on**: [Backends Overview](./06-backends-overview.md), [Trait Surface](./01-trait-surface.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

A distributed `Bus` backend routes `Pulse`s through an external message broker (NATS or
Kafka are the leading candidates). It enables multiple Roko agents on different machines to
share a single logical bus. This is the transport layer for the planned `Mesh` agent-network
layer.

---

## Use Cases

- **Multi-agent collaboration** — Agent A publishes a `Pulse` on `agent.request.answer`;
  Agent B (different machine) subscribes and responds on `agent.response.answer`.
- **Distributed cognition** — The Delta-speed loop runs on a separate machine and consumes
  `loop.step.*` events published by the Gamma-speed loop.
- **Monitoring / observability** — An external metrics agent subscribes to all `agent.*`
  topics without modifying the runtime.

---

## Candidate Brokers

| Broker | Notes |
|---|---|
| NATS | Lightweight, Go-native, excellent Rust client (`async-nats`), supports JetStream for persistence and replay. Preferred candidate. |
| Kafka | Higher throughput at scale, heavier operational footprint, good at replay via consumer offsets. |
| Redis Streams | Low operational overhead, good for simple cases, limited fan-out compared to NATS. |

The broker is an implementation detail of the backend, not part of the `Bus` trait. Swapping
brokers is a backend-level change.

---

## Specification

### `publish`

Publishes a `Pulse` to the broker using the `Topic` path as the broker subject/topic key.
Returns `Ok(seq)` once the broker acknowledges receipt.

### `subscribe`

Creates a durable subscription on the broker. The broker delivers matching events to an
async channel; the stream drains the channel.

### `replay`

Uses broker-native replay (NATS JetStream consumer, Kafka consumer offset) to replay events
from a specified point. The `ReplaySpec` is translated to the broker's native API.

### `len`

Queries the broker for the current depth of the topic's stream/subject.

---

## Failure Mode Differences vs. In-Process

| Failure | In-Process | Distributed |
|---|---|---|
| Broker unavailable | Not applicable | `BusError::Backend("broker unreachable")` |
| Network partition | Not applicable | Publisher: may queue locally; subscriber: stale until reconnect |
| Message loss | Never (in-process) | Possible under at-most-once; replay recovers under at-least-once |

---

## See Also

- [Today vs. Planned](./14-today-vs-planned.md)
- [Delivery Semantics](./10-delivery-semantics.md)
- [Failure Modes](./11-failure-modes.md)

## Open Questions

- Which broker ships first? NATS is the current preference; confirm before implementation.
- Should the distributed backend support end-to-end encryption of `Pulse` payloads?
