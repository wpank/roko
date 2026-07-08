# Substrate — Trait Surface

> The complete Rust trait signature for `Substrate`, with every method, parameter, and
> return type annotated.

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Engram](../01-engram/README.md), [ContentHash](../10-types/content-hash.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

`Substrate` is a synchronous (today) object-safe Rust trait. It exposes four methods:
`put`, `get`, `query`, and `query_similar`. Implementations must uphold the invariants in
[Invariants](./11-invariants.md).

---

## The Trait

```rust
// source: crates/roko-core/src/substrate.rs

/// Storage fabric for durable [`Engram`] records.
///
/// Every Roko agent holds a `Box<dyn Substrate>` (or `Arc<dyn Substrate>`).
/// The trait is the only interface through which the cognitive loop reads and
/// writes long-term memory.
///
/// # Object safety
/// The trait is object-safe. All parameters are concrete types; no associated
/// types or generic methods appear on the trait itself.
///
/// # Sync vs Async
/// The current trait is synchronous. An `async`-compatible version is planned
/// but not yet shipped. Implementations that perform I/O must block internally.
pub trait Substrate: Send + Sync {
    /// Store or update a single [`Engram`].
    ///
    /// If an `Engram` with the same `ContentHash` already exists, the
    /// implementation MUST replace it atomically. It is not an error to
    /// `put` the same content twice.
    ///
    /// Implementations SHOULD populate the HDC fingerprint if it is absent
    /// on the incoming record. See [`fingerprint_population`].
    ///
    /// # Errors
    /// Returns `SubstrateError::Io` on I/O failure (disk full, permission
    /// denied). Returns `SubstrateError::Serialization` if the `Engram`
    /// cannot be serialized.
    fn put(&mut self, engram: Engram) -> Result<(), SubstrateError>;

    /// Retrieve a single [`Engram`] by its content hash.
    ///
    /// Returns `Ok(None)` if no record with that hash exists. Never returns
    /// an error for a missing record — only for I/O or deserialization faults.
    fn get(&self, id: &ContentHash) -> Result<Option<Engram>, SubstrateError>;

    /// Retrieve a set of [`Engram`]s matching a structured filter.
    ///
    /// Filter fields are ANDed. An empty `SubstrateQuery` matches all records
    /// (subject to the `limit` field). Order of results is implementation-
    /// defined but MUST be stable across identical calls on unchanged data.
    ///
    /// # Pagination
    /// The `limit` and `offset` fields on [`SubstrateQuery`] provide
    /// cursor-free pagination. Large result sets SHOULD use a non-zero `limit`.
    fn query(&self, q: &SubstrateQuery) -> Result<Vec<Engram>, SubstrateError>;

    /// Find the `k` [`Engram`]s most similar to the supplied HDC fingerprint.
    ///
    /// Similarity is measured by Hamming distance on the HDC binary vector.
    /// If fewer than `k` records exist in the store, all records are returned.
    ///
    /// Records without a fingerprint are excluded from similarity ranking but
    /// MAY be returned if the underlying index cannot distinguish them; callers
    /// must not rely on fingerprint-less records appearing or not appearing.
    fn query_similar(
        &self,
        fingerprint: &HdcFingerprint,
        k: usize,
    ) -> Result<Vec<Engram>, SubstrateError>;

    /// Prune expired or low-priority [`Engram`]s.
    ///
    /// Called by the runtime on a schedule or when the store approaches
    /// capacity. Implementations decide which records to evict based on
    /// `Decay` schedules and score rankings. See [Pruning](./06-pruning.md).
    ///
    /// Returns the number of records removed.
    fn prune(&mut self) -> Result<usize, SubstrateError>;

    /// Return the number of [`Engram`]s currently in the store.
    fn len(&self) -> usize;

    /// Return `true` if the store contains no records.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
```
<!-- source: crates/roko-core/src/substrate.rs -->

---

## Supporting Types

### `SubstrateError`

```rust
// source: crates/roko-core/src/substrate.rs

#[derive(Debug, thiserror::Error)]
pub enum SubstrateError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Record not found: {0}")]
    NotFound(ContentHash),

    #[error("Backend error: {0}")]
    Backend(String),
}
```
<!-- source: crates/roko-core/src/substrate.rs -->

### `SubstrateQuery`

```rust
// source: crates/roko-core/src/substrate.rs

/// Structured filter for [`Substrate::query`].
///
/// All set fields are ANDed. Unset fields (`None`) match any value.
#[derive(Debug, Default, Clone)]
pub struct SubstrateQuery {
    /// Match only records of this `Kind`.
    pub kind: Option<Kind>,

    /// Match only records whose `Score::confidence` is at least this value.
    pub min_confidence: Option<f32>,

    /// Match only records created after this UNIX timestamp (seconds).
    pub created_after: Option<u64>,

    /// Match only records created before this UNIX timestamp (seconds).
    pub created_before: Option<u64>,

    /// Maximum number of results to return. `0` means no limit.
    pub limit: usize,

    /// Skip this many results (for pagination). `0` means no skip.
    pub offset: usize,
}
```
<!-- source: crates/roko-core/src/substrate.rs -->

---

## Method Contracts at a Glance

| Method | Mutates store | Can error | Returns |
|---|---|---|---|
| `put` | Yes | Yes | `()` or error |
| `get` | No | Yes (I/O only) | `Option<Engram>` |
| `query` | No | Yes (I/O only) | `Vec<Engram>` |
| `query_similar` | No | Yes (I/O only) | `Vec<Engram>` |
| `prune` | Yes | Yes | count removed |
| `len` | No | Never | `usize` |
| `is_empty` | No | Never | `bool` (default impl) |

---

## Object Safety Notes

`Substrate` is intentionally object-safe so that code can hold `Box<dyn Substrate>` and
swap backends at runtime (e.g., in tests). This means:

- No associated types.
- No generic methods (`query` takes a concrete `SubstrateQuery`, not a generic `Q`).
- No `Self` in return position.

---

## See Also

- [Put, Get, Query](./02-put-get-query.md) — semantics of the three core ops
- [Query Similar](./03-query-similar.md) — HDC similarity search
- [Invariants](./11-invariants.md) — what every implementation must uphold
- [Failure Modes](./12-failure-modes.md) — when and how errors propagate

## Open Questions

- Should `prune` accept a budget parameter (`prune(max_removals: usize)`) to bound
  latency?
- Is `is_empty` worth keeping as a default method, or should it be removed to keep the
  trait minimal?
