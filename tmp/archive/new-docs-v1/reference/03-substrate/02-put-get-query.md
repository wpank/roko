# Put, Get, Query — Core Substrate Operations

> Three methods form the core of `Substrate`: `put` (write), `get` (point lookup), and
> `query` (filtered scan). This page covers their semantics, pre-conditions, post-conditions,
> and the contracts callers can rely on.

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Trait Surface](./01-trait-surface.md), [Engram](../01-engram/README.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

`put` is an upsert keyed on `ContentHash`. `get` is an exact-key lookup returning
`Option<Engram>`. `query` is a structured filter returning `Vec<Engram>`. None of these
methods overlap with `query_similar` (similarity search), which is covered separately in
[Query Similar](./03-query-similar.md).

---

## `put(engram: Engram) → Result<(), SubstrateError>`

### Purpose

Store an `Engram`. If a record with the same `ContentHash` already exists, replace it
atomically. `put` is an **upsert** — idempotent on content, destructive on older state.

### Pre-conditions

| Condition | Description |
|---|---|
| `engram.hash` is set | `ContentHash` must be populated. `EngramBuilder` always sets this. |
| `engram` is valid | All required fields must be non-default. `Engram::validate()` should pass. |

### Post-conditions

| Condition | Description |
|---|---|
| Durability | After `put` returns `Ok(())`, the record survives a process restart (for file/DB backends). |
| Fingerprint | If `engram.fingerprint` was `None`, the implementation populates it before writing. |
| Idempotence | Calling `put` twice with the same content is safe and produces the same stored state. |

### Error Cases

| Error | Meaning |
|---|---|
| `SubstrateError::Io` | Disk full, permission denied, or other OS-level I/O failure. |
| `SubstrateError::Serialization` | `Engram` could not be serialized to the backend format. |

### Example

```rust
// source: crates/roko-core/src/substrate.rs
let engram = EngramBuilder::new()
    .body(Body::Text("Eiffel Tower is 330m tall.".into()))
    .kind(Kind::Fact)
    .build()?;

substrate.put(engram)?;
```
<!-- source: crates/roko-core/src/substrate.rs -->

---

## `get(id: &ContentHash) → Result<Option<Engram>, SubstrateError>`

### Purpose

Retrieve an `Engram` by its exact `ContentHash`. Returns `Ok(None)` if the record does not
exist. This is a point lookup — O(1) for hash-indexed backends, O(n) for linear scan backends
like the JSONL file backend (which builds an in-memory index on open).

### Pre-conditions

None beyond a valid `ContentHash`.

### Post-conditions

| Condition | Description |
|---|---|
| Consistency | If `put(e)` returned `Ok(())` before `get(e.hash)`, `get` returns `Ok(Some(e))` unless pruning has run. |
| No mutation | `get` never modifies the store. |

### Error Cases

| Error | Meaning |
|---|---|
| `SubstrateError::Io` | Could not read from backing store. |
| `SubstrateError::Serialization` | Record exists but could not be deserialized. |

### When `Ok(None)` is the right answer

`Ok(None)` means the record was never stored, was pruned, or was written to a different
Substrate instance. It is **not** an error. Callers must handle the absent case gracefully —
the cognitive loop should not panic on a cache miss.

---

## `query(q: &SubstrateQuery) → Result<Vec<Engram>, SubstrateError>`

### Purpose

Return all `Engram`s that match a structured filter. Filter fields are ANDed; unset fields
(`None`) match everything.

### `SubstrateQuery` Fields

| Field | Type | Meaning |
|---|---|---|
| `kind` | `Option<Kind>` | Restrict to a specific `Kind` variant |
| `min_confidence` | `Option<f32>` | Return only records where `score.confidence >= min_confidence` |
| `created_after` | `Option<u64>` | UNIX timestamp lower bound (inclusive) |
| `created_before` | `Option<u64>` | UNIX timestamp upper bound (exclusive) |
| `limit` | `usize` | Max results to return; `0` = no limit |
| `offset` | `usize` | Skip this many matching results (for pagination) |

### Post-conditions

| Condition | Description |
|---|---|
| Stability | Result order is stable for identical queries on unchanged data. |
| Completeness | No result is silently dropped unless `limit` or `offset` truncate it. |
| No side effects | `query` never modifies the store. |

### Example — fetch recent high-confidence facts

```rust
// source: crates/roko-core/src/substrate.rs
let results = substrate.query(&SubstrateQuery {
    kind: Some(Kind::Fact),
    min_confidence: Some(0.8),
    created_after: Some(yesterday_unix),
    limit: 32,
    ..Default::default()
})?;
```
<!-- source: crates/roko-core/src/substrate.rs -->

### Performance Notes

`query` is a scan operation on most current backends. The JSONL file backend holds an
in-memory index; the in-memory backend does a linear scan of its `HashMap`. Both are
acceptable for the current agent scale (thousands of records). See [Performance](./13-performance.md)
for benchmarks and scaling advice.

---

## Relationship Between the Three Operations

```
put  ──→  store (upsert by ContentHash)
            │
get  ←──── exact point lookup by hash
            │
query ←──── filtered scan (kind, score, time, paginated)
```

`query_similar` is a fourth operation layered on top of `query` — it uses the HDC
fingerprint index for approximate nearest-neighbour search rather than a structured filter.
See [Query Similar](./03-query-similar.md).

---

## See Also

- [Query Similar](./03-query-similar.md)
- [Fingerprint Population](./04-fingerprint-population.md)
- [Performance](./13-performance.md)
- [Failure Modes](./12-failure-modes.md)

## Open Questions

- Should `query` support sorting (e.g., by `Score::confidence DESC`) or is sort-on-caller
  the right approach?
- Should `SubstrateQuery` support a free-text body match, or is that `query_similar`'s job?
