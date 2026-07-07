# API Reference

> Quick-reference for all `Substrate` trait methods, supporting types, and return values.
> For semantics and contracts, see the dedicated pages for each method.

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Trait Surface](./01-trait-surface.md)
**Last reviewed**: 2026-04-19

---

## `Substrate` Trait Methods

| Method | Signature | Mut? | Returns |
|---|---|---|---|
| `put` | `put(&mut self, engram: Engram) -> Result<(), SubstrateError>` | Yes | Unit or error |
| `get` | `get(&self, id: &ContentHash) -> Result<Option<Engram>, SubstrateError>` | No | Option or error |
| `query` | `query(&self, q: &SubstrateQuery) -> Result<Vec<Engram>, SubstrateError>` | No | Vec or error |
| `query_similar` | `query_similar(&self, fp: &HdcFingerprint, k: usize) -> Result<Vec<Engram>, SubstrateError>` | No | Vec or error |
| `prune` | `prune(&mut self) -> Result<usize, SubstrateError>` | Yes | Count removed or error |
| `len` | `len(&self) -> usize` | No | Record count |
| `is_empty` | `is_empty(&self) -> bool` | No | `self.len() == 0` |

---

## `SubstrateError`

| Variant | Meaning |
|---|---|
| `SubstrateError::Io(std::io::Error)` | OS-level I/O failure |
| `SubstrateError::Serialization(String)` | Cannot serialize/deserialize an `Engram` |
| `SubstrateError::NotFound(ContentHash)` | Not used by `get` (returns `None`); reserved for future use |
| `SubstrateError::Backend(String)` | Backend-specific error (e.g., dimension mismatch) |

---

## `SubstrateQuery`

| Field | Type | Default | Meaning |
|---|---|---|---|
| `kind` | `Option<Kind>` | `None` | Filter by `Kind` variant |
| `min_confidence` | `Option<f32>` | `None` | Minimum `score.confidence` |
| `created_after` | `Option<u64>` | `None` | Created ≥ this UNIX timestamp |
| `created_before` | `Option<u64>` | `None` | Created < this UNIX timestamp |
| `limit` | `usize` | `0` (no limit) | Max results |
| `offset` | `usize` | `0` | Skip this many results |

---

## Concrete Backend Constructors

### `FileSubstrate` (crate: `roko-fs`)

```rust
// source: crates/roko-fs/src/lib.rs
FileSubstrate::open(path: impl AsRef<Path>) -> Result<Self, SubstrateError>
FileSubstrate::compact(&mut self) -> Result<(), SubstrateError>
```
<!-- source: crates/roko-fs/src/lib.rs -->

### `MemorySubstrate` (crate: `roko-runtime`)

```rust
// source: crates/roko-runtime/src/memory_substrate.rs
MemorySubstrate::new() -> Self
MemorySubstrate::with_capacity(max: usize) -> Self
```
<!-- source: crates/roko-runtime/src/memory_substrate.rs -->

---

## See Also

- [Trait Surface](./01-trait-surface.md) — full annotated trait
- [Put, Get, Query](./02-put-get-query.md)
- [Query Similar](./03-query-similar.md)
- [Failure Modes](./12-failure-modes.md)
