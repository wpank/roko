# Bus Failure Modes

> What can go wrong in the `Bus`, how each backend responds, and what the caller must do.

**Status**: Specified
**Crate**: `roko-core` (planned)
**Depends on**: [Delivery Semantics](./10-delivery-semantics.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

The primary Bus failure modes are: dropped messages (ring buffer full), subscriber lag
(slow consumer), reconnection (process restart or network partition). The Bus never panics
on these — they are returned as `BusError` variants or result in silent event drops depending
on the delivery semantic.

---

## Failure Catalogue

### F1 — Ring Buffer Full (Dropped Messages)

**Scenario**: `publish` is called faster than subscribers consume events, and the ring buffer
fills.

**Behaviour**: The oldest event is evicted. `publish` returns `Ok(seq)` — the caller is not
notified of the eviction. Slow subscribers that try to replay the evicted event receive
events from the oldest available sequence (see [Failure Modes: Replay beyond window](./05-replay-and-ring.md)).

**Recovery**: Increase `ring_size`. Reduce publish rate. Speed up subscribers.

---

### F2 — Subscriber Lag

**Scenario**: A subscriber's internal channel buffer fills because the subscriber is slower
than the publisher.

**Behaviour** (at-most-once): Oldest events in the subscriber's channel are dropped. The
subscriber resumes from the next available event without notification.

**Behaviour** (at-least-once): Subscriber must use `ReplaySpec::Since(last_processed_ts)`
on reconnect to recover dropped events.

**Recovery**: Speed up subscriber processing. Increase channel buffer. Switch to at-least-once
with replay checkpointing.

---

### F3 — Subscriber Disconnects (Channel Closed)

**Scenario**: A subscriber task panics or drops its stream handle.

**Behaviour**: The Bus detects the closed channel (next `publish` to that subscriber fails).
The subscriber is automatically removed. `BusError::ChannelClosed` is returned to the
publisher's `publish` call only for direct channels; fan-out publish does not surface
per-subscriber errors.

**Recovery**: The subscriber task should restart and re-subscribe (with replay if needed).

---

### F4 — Process Restart (In-Process Bus)

**Scenario**: The process hosting the in-process `Bus` crashes.

**Behaviour**: All ring buffers, subscriptions, and in-flight events are lost. After restart,
the `Bus` is empty.

**Recovery**: Subscribers that need at-least-once delivery must use `ReplaySpec::Since` with
checkpoints stored in `Substrate`. See [Delivery Semantics](./10-delivery-semantics.md).

---

### F5 — Broker Unreachable (Distributed Bus)

**Scenario**: The external broker (NATS, Kafka) is unreachable.

**Behaviour**:
- `publish` returns `BusError::Backend("broker unreachable")`.
- `subscribe` returns `BusError::Backend("broker unreachable")`.
- The backend retries with exponential backoff (configurable).

**Recovery**: Broker availability. The backend may implement a local queue (configurable
`local_buffer_size`) to hold events during outage; these are flushed on reconnect.

---

### F6 — Publish to Unknown Topic

**Scenario**: `publish` is called with a `Topic` that has no subscribers.

**Behaviour**: `Ok(seq)` is returned. The event is written to the ring buffer but delivered
to zero subscribers. This is not an error — topics are implicitly created on first publish.

---

## Error Propagation Guidance

| Caller | On `BusError` | Notes |
|---|---|---|
| Cognitive loop publish | Log warning, continue | A failed publish is not fatal |
| Subscriber stream | Log, re-subscribe | Use replay on reconnect |
| Runtime health check | Report degraded | Alert if sustained |

---

## See Also

- [Delivery Semantics](./10-delivery-semantics.md)
- [Replay and Ring](./05-replay-and-ring.md)

## Open Questions

- Should the in-process Bus support a local persistent queue (write to `Substrate`) as a
  durability bridge during downtime?
