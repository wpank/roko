# Ordering Guarantees

> What ordering `Bus` guarantees for events: per-topic ordering, total ordering, and causal
> ordering.

**Status**: Specified
**Crate**: `roko-core` (planned)
**Depends on**: [Publish / Subscribe](./04-publish-subscribe.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Bus guarantees **per-topic FIFO ordering** only. Events across topics have no ordering
guarantee. Total ordering and causal ordering are not provided by default — callers that
need them must carry causal metadata in `Pulse` fields.

---

## Per-Topic Ordering (Guaranteed)

Within a single `Topic`, all subscribers receive events in the order they were published.
If the publisher calls `publish(topic_A, e1)` and then `publish(topic_A, e2)`, every
subscriber of `topic_A` will see `e1` before `e2`.

This is enforced by the sequence number assigned on publish: seq numbers are monotonically
increasing per topic.

---

## Cross-Topic Ordering (Not Guaranteed)

If publisher P1 publishes to `topic_A` and publisher P2 publishes to `topic_B`
simultaneously, a subscriber of both topics may receive events in any order.

Example:
```
P1 publishes: topic_A → e1 (seq 1)
P2 publishes: topic_B → e2 (seq 1)
Subscriber (TopicFilter::Prefix("loop")): may see [e1, e2] or [e2, e1]
```

Callers that need cross-topic ordering must use logical timestamps or vector clocks in the
`Pulse` payload.

---

## Total Ordering (Not Provided)

A totally-ordered bus would require a global sequence number across all topics. This is
expensive in a distributed backend (requires a central sequencer or Paxos/Raft consensus).
Roko does not provide this.

---

## Causal Ordering (Not Provided)

A causally-ordered bus delivers event `b` only after event `a` if `b` causally depends on
`a` (b was published in response to a). This requires causal metadata in `Pulse` fields.

The active-inference design of Roko uses `prediction.*` and `prediction.error.*` events to
express causality. These are application-level causal chains, not bus-level ordering.

---

## Summary Table

| Guarantee | In-process Bus | Distributed Bus |
|---|---|---|
| Per-topic FIFO | Yes | Yes (same broker shard) |
| Total ordering | No | No |
| Causal ordering | No (carry in Pulse) | No (carry in Pulse) |
| Replay ordering | Same as per-topic FIFO | Same as broker retention |

---

## See Also

- [Delivery Semantics](./10-delivery-semantics.md)
- [Replay and Ring](./05-replay-and-ring.md)
