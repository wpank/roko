# Substrate Invariants

> The properties that every correct `Substrate` implementation must uphold, regardless of
> backend. Violating any invariant is a bug.

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Trait Surface](./01-trait-surface.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

There are eleven invariants. The most critical: `put` followed by `get` must return the
record (durability); `query_similar` must never return records not in the store;
and `prune` must remove records from all indexes simultaneously.

---

## The Invariants

### I1 — Durability After Put

After `put(e)` returns `Ok(())`, a call to `get(e.hash)` on the same instance must return
`Ok(Some(e))`, unless `prune` has been called in between.

### I2 — Upsert Semantics

If `put(e1)` is called and a record with `hash == e1.hash` already exists, the old record is
replaced atomically. `get(e1.hash)` returns `e1`, not the old record.

### I3 — Missing = None, Not Error

`get(hash)` for a hash that was never stored, or was pruned, returns `Ok(None)`. It never
returns `Err(SubstrateError::NotFound(...))` — that error variant exists but must not be
returned from `get`.

### I4 — Query Completeness

`query(q)` returns every record in the store that matches `q`'s filter, subject to `limit`
and `offset`. No matching record is silently dropped.

### I5 — Query Stability

For the same store state and the same `SubstrateQuery`, `query` returns results in the same
order on every call.

### I6 — Similarity Result Subset

Every record returned by `query_similar(fp, k)` must be retrievable via `get`. The result
set is always a subset of the currently stored records.

### I7 — Fingerprint-less Exclusion

Records stored without a fingerprint (`engram.fingerprint == None`) must not appear in
`query_similar` results.

### I8 — Prune Atomicity (per-record)

When `prune` removes a record, that record must be removed from all indexes simultaneously.
A record that has been pruned must not appear in `get`, `query`, or `query_similar` results.

### I9 — Capacity Post-Prune

After `prune` returns, `len() <= max_capacity` (if a capacity was configured).

### I10 — `len` Consistency

`len()` equals the number of records that would be returned by an unconstrained `query` (no
filter, no limit, no offset).

### I11 — No Cross-Instance Contamination

Two `Substrate` instances opened on different files (or different in-memory instances) must
not share records. Operations on one instance must not affect the other.

---

## Testing the Invariants

`roko-core` ships a shared test suite (`substrate_suite`) that exercises all invariants:

```rust
// source: crates/roko-core/tests/substrate_suite.rs
use roko_core::substrate_suite;

#[test]
fn memory_substrate_invariants() {
    substrate_suite::run(|| MemorySubstrate::new());
}

#[test]
fn file_substrate_invariants() {
    let dir = tempdir().unwrap();
    substrate_suite::run(|| FileSubstrate::open(dir.path().join("test.jsonl")).unwrap());
}
```
<!-- source: crates/roko-core/tests/substrate_suite.rs -->

Any new backend implementation must pass this suite before merging.

---

## See Also

- [Failure Modes](./12-failure-modes.md) — what happens when invariants cannot be upheld due to external failures
- [Trait Surface](./01-trait-surface.md)
- [Pruning](./06-pruning.md) — I8 and I9 in depth

## Open Questions

- Should invariant violations be detected at runtime (panic, or `Result::Err`) or only in
  tests? A debug-mode assertion on `put` (verify `get` roundtrip) would catch bugs early.
