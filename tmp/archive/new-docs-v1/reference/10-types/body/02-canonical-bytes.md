# Body — Canonical Bytes

> The exact byte encoding for each Body variant used in the ContentHash computation.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md), [ContentHash Canonical Encoding](../content-hash/01-canonical-encoding.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

`Body::canonical_bytes()` produces a deterministic byte vector for each variant. This
vector is passed (with a length prefix) to the BLAKE3 hasher in `canonical_encode()`.
The encoding is designed to be unambiguous: no two distinct Body values produce the same
byte sequence (collision-free by construction, modulo hash collisions in the final BLAKE3).

---

## Specification

```rust
<!-- source: crates/roko-core/src/body.rs -->

impl Body {
    /// Return the canonical byte encoding for this Body variant.
    /// The output is deterministic and collides only if the Body values are equal.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        match self {
            Body::Text(s) => {
                // Variant tag 0x01 + UTF-8 bytes
                let mut out = vec![0x01u8];
                out.extend_from_slice(s.as_bytes());
                out
            }
            Body::Embedding(vec) => {
                // Variant tag 0x02 + little-endian f32 concatenation
                let mut out = vec![0x02u8];
                for f in vec {
                    out.extend_from_slice(&f.to_le_bytes());
                }
                out
            }
            Body::Json(v) => {
                // Variant tag 0x03 + compact JSON bytes
                let mut out = vec![0x03u8];
                // serde_json::to_vec produces compact, key-sorted JSON
                out.extend_from_slice(
                    &serde_json::to_vec(v).expect("JSON must be serializable")
                );
                out
            }
            Body::Binary(bytes) => {
                // Variant tag 0x04 + raw bytes
                let mut out = vec![0x04u8];
                out.extend_from_slice(bytes);
                out
            }
            Body::Structured(fields) => {
                // Variant tag 0x05 + sorted key-value pairs
                let mut out = vec![0x05u8];
                // BTreeMap guarantees sorted order
                for (key, value) in fields {
                    let key_bytes = key.as_bytes();
                    out.extend_from_slice(&(key_bytes.len() as u64).to_le_bytes());
                    out.extend_from_slice(key_bytes);
                    let val_bytes = value.canonical_bytes();
                    out.extend_from_slice(&(val_bytes.len() as u64).to_le_bytes());
                    out.extend_from_slice(&val_bytes);
                }
                out
            }
        }
    }
}
```

---

## Variant Tags

The first byte of every `canonical_bytes()` output is a variant discriminant:

| Variant | Tag byte |
|---|---|
| `Text` | `0x01` |
| `Embedding` | `0x02` |
| `Json` | `0x03` |
| `Binary` | `0x04` |
| `Structured` | `0x05` |

These tags ensure that an empty `Text` (`b"\x01"`) and an empty `Binary` (`b"\x04"`)
produce different byte sequences and thus different hashes.

---

## JSON Determinism Note

`serde_json::to_vec()` in compact mode is **not guaranteed to be deterministic** across
serde_json versions when object keys are not sorted. However, Roko requires that all
`Body::Json` objects use `serde_json::Value::Object(Map<String, Value>)` which
preserves insertion order. For hash-critical use, always sort the map before passing to
`Body::Json`.

This is a known limitation. The `Body::Structured` variant avoids this issue entirely
because `BTreeMap` always sorts keys.

<!-- ADDED: JSON determinism warning — not in source docs; inferred from serde_json
behavior and the importance of hash determinism. -->

---

## Invariants

1. `canonical_bytes()` returns a non-empty byte vector (minimum 1 byte for the variant tag).
2. Two distinct Body values (excluding floating-point NaN edge cases) produce different
   canonical bytes.
3. `canonical_bytes()` is pure — no external state, no randomness.
4. The variant tag byte uniquely identifies the variant.
5. `Body::Structured` encoding is sorted by key — `BTreeMap` guarantees this.
6. For `Body::Json`, keys should be sorted before hashing for determinism across serialization
   versions.

---

## Open Questions

- Should `Body::Json` require pre-sorted keys at construction time? Currently a convention,
  not enforced.
- Should floating-point NaN in `Body::Embedding` be rejected? NaN != NaN in IEEE 754;
  two embeddings with NaN values might appear unequal even if semantically identical.

## See Also

- [`../content-hash/01-canonical-encoding.md`](../content-hash/01-canonical-encoding.md) — how canonical_bytes is used
- [`00-overview.md`](00-overview.md) — Body overview
- [`03-api-reference.md`](03-api-reference.md) — methods and invariants
