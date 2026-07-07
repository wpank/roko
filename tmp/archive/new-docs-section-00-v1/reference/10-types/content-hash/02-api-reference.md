# ContentHash — API Reference

> Every public method on the ContentHash type.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md)  
**Last reviewed**: 2026-04-19

---

## `struct ContentHash`

```rust
<!-- source: crates/roko-core/src/content_hash.rs -->

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ContentHash([u8; 32]);
```

### Constructors

| Method | Signature | Description |
|---|---|---|
| `compute` | `(engram: &Engram) -> Self` | Hash an Engram using canonical encoding + BLAKE3 |
| `from_bytes` | `(bytes: [u8; 32]) -> Self` | Construct from raw bytes (use with caution — no validation) |
| `from_hex` | `(s: &str) -> Result<Self, HashError>` | Decode from a 64-character lowercase hex string |

### Accessors

| Method | Signature | Description |
|---|---|---|
| `as_bytes` | `(&self) -> &[u8; 32]` | Return reference to the 32-byte inner array |
| `to_hex` | `(&self) -> String` | Encode as a 64-character lowercase hex string |

### Utility

| Method | Signature | Description |
|---|---|---|
| `verify` | `(&self, engram: &Engram) -> bool` | Recompute hash and compare — returns true if consistent |
| `zero` | `() -> Self` | The all-zeros hash (reserved for null/sentinel uses) |
| `is_zero` | `(&self) -> bool` | True if all 32 bytes are zero |

---

## Free Functions

```rust
<!-- source: crates/roko-core/src/content_hash.rs -->

/// Compute the canonical encoding of an Engram (without hashing).
pub fn canonical_encode(engram: &Engram) -> Vec<u8>;

/// Hash a pre-computed canonical byte vector.
pub fn hash_bytes(bytes: &[u8]) -> ContentHash;
```

---

## Error Types

```rust
<!-- source: crates/roko-core/src/content_hash.rs -->

#[derive(Debug, thiserror::Error)]
pub enum HashError {
    #[error("invalid hex string: expected 64 lowercase hex chars, got {0} chars")]
    InvalidHexLength(usize),
    #[error("invalid hex character at position {pos}: '{ch}'")]
    InvalidHexChar { pos: usize, ch: char },
}
```

---

## Display

`ContentHash` implements `std::fmt::Display` as the lowercase hex string:

```rust
let hash: ContentHash = ContentHash::compute(&engram);
println!("{}", hash);  // prints: "a3f2b9..."  (64 hex chars)
```

---

## Open Questions

- Should `from_hex` accept uppercase as well as lowercase? Currently lowercase only.
- Should `verify` be the canonical way to check integrity, or should it be a Substrate
  responsibility?

## See Also

- [`01-canonical-encoding.md`](01-canonical-encoding.md) — what canonical_encode produces
- [`03-invariants.md`](03-invariants.md) — hash invariants
- [`04-examples.md`](04-examples.md) — worked examples
