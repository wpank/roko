# Backend: Chain (`ChainSubstrate`)

> A chain-backed `Substrate` implementation that stores `Engram`s on-chain for
> verifiability, cross-agent sharing, and permanent provenance. Status: Specified (no code).

**Status**: Specified
**Crate**: — (not yet assigned)
**Depends on**: [Backends Overview](./07-backends-overview.md), [Trait Surface](./01-trait-surface.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

`ChainSubstrate` is the planned on-chain backend. It maps `put`/`get`/`query` to smart
contract calls on the Roko chain layer. The primary use cases are cross-agent shared memory
and immutable provenance. No code exists yet; this page is the specification.

---

## Why Chain?

The JSONL and in-memory backends are single-process stores. They cannot serve multiple agents
sharing memory, and they offer no cryptographic immutability — records can be deleted or
modified locally. For use cases that require:

- **Cross-agent shared memory** (multiple agents reading from the same memory pool),
- **Verifiable provenance** (a third party can prove a record existed at a given block height),
- **Permanent storage** (records cannot be pruned without on-chain governance),

a chain-backed substrate is the right architecture.

---

## Specification

### `put`

Submits an `Engram` to a smart contract that:
1. Verifies the `ContentHash` matches the serialised content.
2. Stores the record in on-chain storage (or IPFS with on-chain hash pointer).
3. Emits a `EngramStored(hash, block_height)` event.

Because block confirmation takes seconds, `put` is **asynchronous in practice** even if the
trait is synchronous today. The synchronous wrapper blocks until the transaction is confirmed.
The async trait (see [Concurrency Model](./05-concurrency-model.md)) is required for
production chain substrate.

### `get`

Calls a view function on the contract. O(1) if the contract maintains a hash → IPFS-CID map.

### `query`

Chain backends do not natively support range queries over `Kind` or score. The plan is:
1. Maintain an off-chain index (identical to `FileSubstrate`'s in-memory index) populated
   by listening to `EngramStored` events.
2. Serve `query` from the local index.
3. Verify individual records on-chain on demand.

### `prune`

On-chain records cannot be pruned — they are permanent. `prune` on `ChainSubstrate` is a
no-op that always returns `Ok(0)`. Capacity-driven eviction is not applicable.

### `query_similar`

Same as other backends — linear scan of the local HDC index.

---

## Today vs. Planned

| Today | Target state |
|---|---|
| No code | Smart contract + local index |
| — | Async-first (block time requires async) |
| — | IPFS for large bodies, on-chain for hashes |
| — | Multi-agent read access via shared contract |

---

## See Also

- [Backends Overview](./07-backends-overview.md)
- [Concurrency Model](./05-concurrency-model.md) — async trait requirements
- [Rationale](./16-rationale.md)

## Open Questions

- Should `ChainSubstrate` maintain a full local replica (optimistic local reads) or always
  query on-chain (higher latency, guaranteed consistency)?
- What is the gas cost per `put`? Is IPFS + on-chain hash pointer the right split?
- Should `prune` tombstone records on-chain (marking them inactive) rather than being a
  pure no-op?
