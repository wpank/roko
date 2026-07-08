# Query Similar — HDC Similarity Search

> `query_similar` retrieves the `k` `Engram`s whose HDC fingerprint is closest (by Hamming
> distance) to a supplied query fingerprint. This is Roko's associative recall: given a
> context vector, find the memories most like it.

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Trait Surface](./01-trait-surface.md), [HDC Fingerprint](../10-types/hdc-fingerprint.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

`query_similar(fingerprint, k)` returns the top-`k` records by Hamming similarity to the
query vector. It is the primary mechanism for associative recall in Roko agents — equivalent
to "find memories that feel like this context." All records lacking a fingerprint are excluded
from ranking.

---

## The Idea

Exact lookup (`get`) and structured filtering (`query`) are necessary but not sufficient for
a cognitive agent. An agent that can only retrieve memories it already knows the exact hash of
has no associative recall — no ability to ask "what do I know that is related to this?"

HDC (Hyperdimensional Computing) fingerprints solve this. Each `Engram` stores a
high-dimensional binary vector derived from its content. Vectors that are semantically related
— derived from similar or overlapping content — tend to be close in Hamming distance. Querying
by fingerprint proximity is therefore a lightweight form of semantic search that requires no
external model.

---

## Method Signature

```rust
// source: crates/roko-core/src/substrate.rs
fn query_similar(
    &self,
    fingerprint: &HdcFingerprint,
    k: usize,
) -> Result<Vec<Engram>, SubstrateError>;
```
<!-- source: crates/roko-core/src/substrate.rs -->

---

## Parameters

| Parameter | Type | Description |
|---|---|---|
| `fingerprint` | `&HdcFingerprint` | The query vector. Typically derived from the current agent context (the `Engram` being processed, or a thinned combination of recent contexts). |
| `k` | `usize` | Maximum number of results to return. If fewer than `k` fingerprinted records exist, all are returned. |

---

## Semantics

1. The implementation collects all stored `Engram`s that have a populated `fingerprint` field.
2. For each, it computes the Hamming distance between the stored fingerprint and the query
   fingerprint.
3. It returns the `k` records with the lowest Hamming distance (most similar), in ascending
   distance order.
4. Records without a fingerprint are excluded from the result set.
5. Ties are broken by `Score::confidence` descending (higher confidence wins).

### Hamming Distance

For two binary vectors `a` and `b` of the same dimensionality `D`:

```
hamming(a, b) = popcount(a XOR b)
```

`popcount` counts the number of set bits — i.e., the number of dimensions where the vectors
disagree. Dimensionality `D` is fixed at compile time (default: 10,000 bits / 1,250 bytes per
fingerprint). See [HDC Fingerprint](../10-types/hdc-fingerprint.md) for the full spec.

---

## Generating a Query Fingerprint

The caller is responsible for constructing the query fingerprint. The typical pattern in the
cognitive loop is to derive it from the current context `Engram`:

```rust
// source: crates/roko-core/src/substrate.rs
// In the RECALL step of the cognitive loop:
let context_fp = current_engram.fingerprint
    .as_ref()
    .ok_or(SubstrateError::Backend("no fingerprint on context".into()))?;

let similar = substrate.query_similar(context_fp, 16)?;
```
<!-- source: crates/roko-core/src/substrate.rs -->

A second pattern is to build a thinned combination (bundle) of several fingerprints to query
for records related to a composite concept:

```rust
// source: crates/roko-core/src/substrate.rs
// Bundle (XOR-majority-vote) of multiple fingerprints:
let bundle = HdcFingerprint::bundle(&[fp_a, fp_b, fp_c]);
let related = substrate.query_similar(&bundle, 8)?;
```
<!-- source: crates/roko-core/src/substrate.rs -->

---

## Performance Characteristics

| Backend | Index type | Time complexity |
|---|---|---|
| `MemorySubstrate` | Linear scan | O(n · D/64) where n = record count, D = fingerprint bits |
| `FileSubstrate` | In-memory index loaded on open | O(n · D/64) after O(n) startup |
| Future: LSH index | Approximate | O(log n) amortised |

For agents with < 50,000 engrams and D = 10,000 bits, a linear scan completes in
< 5 ms on modern hardware. At 500,000 records, an LSH (locality-sensitive hashing) index
would be needed for sub-millisecond recall. See [Performance](./13-performance.md).

---

## Failure Modes

| Failure | Behaviour |
|---|---|
| No records have fingerprints | Returns `Ok(vec![])` — empty result, not an error. |
| Query fingerprint has wrong dimensionality | `SubstrateError::Backend("fingerprint dimension mismatch")`. |
| I/O error reading records | `SubstrateError::Io(...)`. |

---

## Invariants

- The result set is always a subset of records stored via `put`.
- Records without a fingerprint never appear in the result.
- Result order is deterministic for the same store state and query vector.

---

## See Also

- [Fingerprint Population](./04-fingerprint-population.md) — how fingerprints are built on `put`
- [HDC Fingerprint](../10-types/hdc-fingerprint.md) — the vector type spec
- [Put, Get, Query](./02-put-get-query.md)

## Open Questions

- Should a maximum Hamming distance threshold (`max_distance`) be added to filter out
  records that are too dissimilar even if `k` has not been reached?
- Should `query_similar` accept a combined HDC + `SubstrateQuery` filter (e.g., only search
  among records of a specific `Kind`)?
