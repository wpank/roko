# Substrate — Durable Storage Fabric

> `Substrate` is the trait that every storage backend in Roko must implement. It is the
> single seam between the rest of the framework and wherever `Engram` records actually live
> on disk, in memory, or on-chain. If you are storing, retrieving, or searching memories in
> Roko, you are calling through `Substrate`.

**Status**: Shipping
**Crate**: `roko-core` (trait), `roko-fs` (file backend), `roko-runtime` (wiring)
**Depends on**: [Engram](../01-engram/README.md), [Score](../10-types/score.md), [Decay](../10-types/decay.md)
**Last reviewed**: 2026-04-19

---

## What This Folder Contains

`Substrate` is a large surface — it is the only place in Roko where durability lives. The
folder is broken into focused pages so that an implementer writing a new backend only needs
to read the trait surface and one backend page, while an operator concerned with pruning
only needs the pruning page.

## Contents

| # | Page | What it covers | Status |
|---|---|---|---|
| 00 | [Overview](./00-overview.md) | What Substrate is, why a storage-fabric trait exists, 60-second mental model | Shipping |
| 01 | [Trait Surface](./01-trait-surface.md) | Exact Rust trait signature, every method annotated | Shipping |
| 02 | [Put, Get, Query](./02-put-get-query.md) | The three core operations, semantics, pre/post-conditions | Shipping |
| 03 | [Query Similar](./03-query-similar.md) | HDC similarity search via fingerprint | Shipping |
| 04 | [Fingerprint Population](./04-fingerprint-population.md) | How fingerprints are populated on `put` | Shipping |
| 05 | [Concurrency Model](./05-concurrency-model.md) | Threading, sync vs async, shared-state rules | Shipping |
| 06 | [Pruning](./06-pruning.md) | Decay-driven and capacity-driven eviction | Shipping |
| 07 | [Backends Overview](./07-backends-overview.md) | Enumerate backend families; how to choose | Shipping |
| 08 | [Backend: JSONL File](./08-backend-file-jsonl.md) | JSONL append-log backend (shipping) | Shipping |
| 09 | [Backend: In-Memory](./09-backend-in-memory.md) | In-memory hashmap backend (shipping) | Shipping |
| 10 | [Backend: Chain](./10-backend-chain.md) | On-chain substrate backend (planned) | Specified |
| 11 | [Invariants](./11-invariants.md) | What must always be true of every Substrate implementation | Shipping |
| 12 | [Failure Modes](./12-failure-modes.md) | Disk full, corruption, schema migrations | Shipping |
| 13 | [Performance](./13-performance.md) | Hot paths, allocation, target latency | Shipping |
| 14 | [API Reference](./14-api-reference.md) | Quick-reference for all trait methods and return types | Shipping |
| 15 | [Examples](./15-examples.md) | End-to-end usage patterns | Shipping |
| 16 | [Rationale](./16-rationale.md) | Why a trait rather than a concrete type; alternatives rejected | Shipping |

## Suggested Reading Order

**First-time reader** (understand the concept): 00 → 01 → 02 → 11.

**Backend implementer** (write a new backend): 00 → 01 → 02 → 03 → 04 → 05 → 11 → 12.

**Operator / deployment engineer**: 06 → 07 → 08 → 12 → 13.

**Contributor (performance work)**: 01 → 05 → 13 → 11.

## See Also

- [Bus Transport Fabric](../04-bus/README.md) — the sibling fabric for ephemeral events
- [Engram Data Type](../01-engram/README.md) — the record type that Substrate stores
- [HDC Fingerprint](../10-types/hdc-fingerprint.md) — the vector used by `query_similar`
- [Decay Variants](../10-types/decay.md) — drives pruning decisions
- [Universal Cognitive Loop](../06-loop/README.md) — Substrate is called in the RECALL and STORE steps
