# Body — API Reference and Invariants

> Methods and invariants for the Body type.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md)  
**Last reviewed**: 2026-04-19

---

## Methods

```rust
<!-- source: crates/roko-core/src/body.rs -->

impl Body {
    /// Return the canonical byte encoding for ContentHash computation.
    pub fn canonical_bytes(&self) -> Vec<u8>;

    /// Return true if this Body variant can produce an HDC fingerprint.
    pub fn is_fingerprintable(&self) -> bool {
        !matches!(self, Body::Binary(_))
    }

    /// Return the approximate size in bytes for storage estimation.
    pub fn approximate_size(&self) -> usize {
        match self {
            Body::Text(s)       => s.len(),
            Body::Embedding(v)  => v.len() * 4,
            Body::Json(j)       => serde_json::to_vec(j).map(|v| v.len()).unwrap_or(0),
            Body::Binary(b)     => b.len(),
            Body::Structured(m) => m.values().map(|v| v.approximate_size()).sum::<usize>()
                                   + m.keys().map(|k| k.len()).sum::<usize>(),
        }
    }

    /// Return the variant name as a static string.
    pub fn variant_name(&self) -> &'static str {
        match self {
            Body::Text(_)       => "Text",
            Body::Embedding(_)  => "Embedding",
            Body::Json(_)       => "Json",
            Body::Binary(_)     => "Binary",
            Body::Structured(_) => "Structured",
        }
    }
}
```

---

## Invariants

| # | Invariant | Enforcement |
|---|---|---|
| B1 | `canonical_bytes()` is deterministic — same Body always produces the same bytes | Pure function |
| B2 | `canonical_bytes()` begins with a variant tag byte (0x01–0x05) | Enforced in each match arm |
| B3 | Two distinct Body values produce different `canonical_bytes()` outputs (except NaN edge case) | Variant tags disambiguate variants; content disambiguates within variants |
| B4 | `Body::Binary` is not fingerprintable | `is_fingerprintable()` returns false |
| B5 | `Body::Structured` keys are sorted in canonical encoding | `BTreeMap` guarantees sorted iteration |

---

## Equality Notes

`Body` derives `PartialEq` but note:
- `Body::Embedding` uses exact f32 bit comparison — no floating-point tolerance.
- `Body::Json` comparison is structural (serde_json `PartialEq`).
- `Body::Structured` comparison is recursive.

NaN in `Body::Embedding` causes `PartialEq` to return `false` even when comparing a value
to itself — avoid NaN in embeddings.

---

## Open Questions

- Should `Body` implement `Eq` (not just `PartialEq`)? Currently blocked by `f32` which
  is not `Eq`. Could be resolved by wrapping floats in an ordered newtype.
- Should `approximate_size()` be used to enforce a per-Kind soft limit at construction
  time?

## See Also

- [`02-canonical-bytes.md`](02-canonical-bytes.md) — canonical_bytes implementation
- [`01-variant-reference.md`](01-variant-reference.md) — variant descriptions
- [`../hdc-fingerprint/02-encoding-pipeline.md`](../hdc-fingerprint/02-encoding-pipeline.md) — fingerprinting for each variant
