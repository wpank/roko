# Substrate Overview

> `Substrate` is the storage-fabric trait in Roko. Every `Engram` that an agent remembers
> is written through a `Substrate` implementation. Every retrieval — by exact ID, by topic
> range, or by semantic similarity — comes back through the same interface. Backends are
> swappable; the calling code never changes.

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Engram](../01-engram/README.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

`Substrate` is a Rust trait with three core operations — `put`, `get`, and `query` — plus
a similarity search extension (`query_similar`). Any type that implements `Substrate` can
serve as the durable memory of a Roko agent. Today two backends ship: a JSONL append-log
file backend (`FileSubstrate`) and an in-memory hashmap (`MemorySubstrate`). A chain-backed
substrate is specified for future work.

---

## The Idea

Roko agents improve through use. For that to work, they must remember things. But _how_ they
remember is an infrastructure decision — it depends on deployment constraints (disk available?
network latency? distributed cluster?), not on the agent logic itself.

`Substrate` is the seam that separates the "what to remember" logic (agent operators, the
cognitive loop) from the "how to store it" decision (file, memory, database, chain). This is
the classic repository pattern applied to agent memory.

A first-time reader can think of `Substrate` as: **the interface your agent uses to talk to
its own long-term memory store**.

---

## Two Mediums, Two Fabrics

Roko distinguishes two kinds of information:

| Medium | Lifetime | Fabric | Status |
|---|---|---|---|
| `Engram` | Durable — persists across restarts | `Substrate` | Shipping |
| `Pulse` (target-state) | Ephemeral — in-flight only | `Bus` | Specified |

`Substrate` is the fabric for `Engram`. The companion fabric `Bus` (target-state) will
handle the ephemeral `Pulse` stream. The two fabrics are siblings — Substrate handles
persistence, Bus handles transport. This document covers Substrate only.

---

## Why a Trait?

Without a trait, agent code would import a concrete backend (`FileSubstrate` or a database
client) directly. That coupling means:

1. You cannot swap the backend in tests without rewriting agent logic.
2. You cannot add a new backend (in-memory, chain-backed) without forking the agent.
3. Performance profiling mixes agent logic with I/O.

The trait breaks all three couplings. Agent code calls `substrate.put(engram)` — it does
not know or care what is on the other side. See [Rationale](./16-rationale.md) for the full
design decision record.

---

## What Substrate Stores

`Substrate` stores and retrieves [`Engram`](../01-engram/README.md) records. An `Engram` is:

- Content-addressed by `ContentHash` (BLAKE3 over its canonical bytes).
- Optionally annotated with an HDC (Hyperdimensional Computing) fingerprint — a
  high-dimensional binary vector derived from the `Engram`'s content, used for approximate
  nearest-neighbour similarity search.
- Tagged with a `Score` (seven-axis appraisal: confidence, novelty, utility, reputation, and
  three extended axes) and a `Decay` schedule.
- Linked into a lineage DAG via parent hashes.

`Substrate` is not a general key-value store — it is specifically designed for the
`Engram` record shape. The `query_similar` method exploits the HDC fingerprint for
associative recall that goes beyond exact lookup.

---

## The Three Core Operations

| Method | Purpose |
|---|---|
| `put(engram)` | Store or update a single `Engram`. Populate its HDC fingerprint if absent. |
| `get(id)` | Retrieve an `Engram` by its `ContentHash`. |
| `query(filter)` | Retrieve a set of `Engram`s matching a structured filter (kind, score range, time window). |

The similarity search extension `query_similar(fingerprint, k)` is built on top of these
operations and described separately in [Query Similar](./03-query-similar.md).

---

## Lifecycle in the Cognitive Loop

In the [Universal Cognitive Loop](../06-loop/README.md), `Substrate` is touched at two steps:

1. **RECALL** (step 2 of 7) — before the agent decides what to do, it queries Substrate to
   retrieve relevant memories. Typical call: `substrate.query_similar(context_fingerprint, k=16)`.
2. **STORE** (step 6 of 7) — after the agent acts and receives outcome signals, it writes new
   or updated `Engram`s back via `substrate.put(engram)`.

Pruning runs asynchronously or lazily — not in the hot path of every loop tick.

---

## Today vs. Planned

| Today (shipping) | Target state (specified) |
|---|---|
| `FileSubstrate` (JSONL) | Distributed / cloud backends |
| `MemorySubstrate` (hashmap) | Chain-backed substrate |
| Sync `put`/`get`/`query` | Async trait (`async_trait` or native async) |
| Manual fingerprint population on `put` | Automatic fingerprint derivation from `Body` content |

---

## See Also

- [Trait Surface](./01-trait-surface.md) — the exact method signatures
- [Bus Overview](../04-bus/00-overview.md) — the ephemeral-event sibling
- [Engram Data Type](../01-engram/README.md)
- [Rationale](./16-rationale.md)

## Open Questions

- Should `Substrate` expose a streaming/cursor interface for large queries, or is pagination
  via filter offsets sufficient?
- How does the HDC fingerprint interact with encrypted backends where content is opaque?
