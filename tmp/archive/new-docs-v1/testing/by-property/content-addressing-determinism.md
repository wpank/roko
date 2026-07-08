# Content-Addressing Determinism

> The same bytes always produce the same `ContentHash`, regardless of who computes it, when, or where.

**Crate**: `roko-core`
**Test type**: Property-based (proptest)
**Enforcement**: `ContentHash::from_bytes`, `Engram::new`
**Last reviewed**: 2026-04-19

---

## Statement

For all byte slices `b`: `ContentHash::from_bytes(b) == ContentHash::from_bytes(b)`.

More precisely: `ContentHash` is a pure function of the byte content. Two `ContentHash` values are equal if and only if the byte slices that produced them are equal (modulo hash collision probability, which is negligible for BLAKE3).

---

## Why It Matters

The entire Roko substrate is content-addressed: an Engram's identity IS its hash. If the hash were non-deterministic (depended on time, machine state, or random inputs), then:
- The same Engram stored on two machines would have different IDs.
- Write idempotence would be impossible (the second write would always create a new ID).
- Cross-agent deduplication would fail.
- The provenance chain (which is a hash chain) would be unverifiable.

---

## Implementation

```rust
// crates/roko-core/src/content_hash.rs
use blake3::Hasher;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    pub fn from_bytes(data: &[u8]) -> Self {
        let hash = blake3::hash(data);
        Self(*hash.as_bytes())
    }
}
```

<!-- source: crates/roko-core/src/content_hash.rs -->

BLAKE3 is a deterministic hash function by construction. The implementation adds no non-determinism.

---

## Property Test

```rust
proptest! {
    #[test]
    fn content_hash_determinism(bytes in any::<Vec<u8>>()) {
        let h1 = ContentHash::from_bytes(&bytes);
        let h2 = ContentHash::from_bytes(&bytes);
        prop_assert_eq!(h1, h2,
            "ContentHash must be deterministic for input of length {}", bytes.len());
    }
}
```

**File**: `crates/roko-core/src/content_hash.rs` (test module)
**Cases**: 256 per run (512 in CI with `PROPTEST_CASES=512`)

---

## Failure Mode

If this property fails:
1. The hash implementation has a bug (non-deterministic seed, mutable global state, OS randomness).
2. The bytes passed to the two invocations are different (likely a test bug, not an implementation bug).

A failing counterexample from proptest would show the minimal byte slice that triggers non-determinism.

---

## Related Properties

- [content-hash-collision-resistance.md](content-hash-collision-resistance.md) — `h1 == h2 ↔ bytes1 == bytes2`
- [engram-serialization-roundtrip.md](engram-serialization-roundtrip.md) — depends on determinism for round-trip equality
- [substrate-idempotence.md](substrate-idempotence.md) — write idempotence depends on deterministic IDs

## See also

- [../by-subsystem/subsystem-core.md](../by-subsystem/subsystem-core.md) — roko-core test coverage
