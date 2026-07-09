# HDC Fingerprint — Invariants

> All invariants for the HdcFingerprint, HdcVector, and encoder system.

**Status**: Shipping  
**Crate**: `bardo-primitives`  
**Depends on**: [Overview](00-overview.md)  
**Last reviewed**: 2026-04-19

---

## Vector Invariants

| # | Invariant | Enforcement |
|---|---|---|
| V1 | `HdcVector` is always exactly `[u64; 160]` — 1,280 bytes | Fixed-size array type |
| V2 | `hamming_distance(v, v) == 0` | XOR self-cancels: all bits zero, popcount=0 |
| V3 | `hamming_distance(a, b) == hamming_distance(b, a)` | XOR is commutative |
| V4 | `hamming_distance(a, b) ∈ [0, 10_240]` | Popcount of XOR, bounded by bit count |
| V5 | `similarity(v, v) == 1.0` | Follows from V2 |
| V6 | `v.xor(&v) == HdcVector::zero()` | Mathematical identity |
| V7 | `bit(i)` panics in debug if `i >= 10_240` | `debug_assert!` in `bit()` |

---

## Fingerprint Invariants

| # | Invariant | Enforcement |
|---|---|---|
| F1 | `fingerprint` is excluded from `ContentHash` | `canonical_encode()` does not read `fingerprint` |
| F2 | Changing `fingerprint` does not change `id` | Follows from F1 |
| F3 | `encoder_version` equals the version that produced the `vector` | Struct created only via `HdcEncoder::encode()` |
| F4 | `fingerprint = None` for `Body::Binary` | `encode()` returns `None` for binary bodies |
| F5 | Empty body produces `None`, not `HdcVector::zero()` | `encode()` checks for empty token list |

---

## Encoder Invariants

| # | Invariant | Enforcement |
|---|---|---|
| E1 | `encode(body, version)` is deterministic for the same inputs | Seeded RNG with BLAKE3 seed |
| E2 | Same token always maps to the same vector within the same encoder version | `project_token()` seed includes `encoder_version` |
| E3 | Different encoder versions produce different vectors for the same token | `encoder_version` is in the seed |
| E4 | `similarity_checked()` returns `None` for cross-version comparison | Explicit version equality check |

---

## Re-encoding Invariants

| # | Invariant | Enforcement |
|---|---|---|
| R1 | Re-encoding does not change `ContentHash` | `fingerprint` excluded from hash |
| R2 | Re-encoding uses the original `Body`, not the old fingerprint | `reencode()` calls `encoder.encode(&engram.body)` |
| R3 | After migration, no Engram in the warm tier has a fingerprint with version < current | Migration job guarantee |

---

## Test Coverage

```rust
<!-- source: crates/bardo-primitives/tests/hdc_invariants.rs -->

#[test]
fn hamming_distance_self_is_zero() {
    let v = HdcVector::random(&mut rand::thread_rng());
    assert_eq!(v.hamming_distance(&v), 0);
}

#[test]
fn hamming_distance_symmetric() {
    let mut rng = rand::thread_rng();
    let a = HdcVector::random(&mut rng);
    let b = HdcVector::random(&mut rng);
    assert_eq!(a.hamming_distance(&b), b.hamming_distance(&a));
}

#[test]
fn encode_deterministic() {
    let encoder = HdcEncoder { version: 1 };
    let body = Body::Text("hello world".into());
    let f1 = encoder.encode(&body).unwrap();
    let f2 = encoder.encode(&body).unwrap();
    assert_eq!(f1.vector, f2.vector);
}

#[test]
fn binary_body_produces_none() {
    let encoder = HdcEncoder { version: 1 };
    let body = Body::Binary(vec![1, 2, 3]);
    assert!(encoder.encode(&body).is_none());
}

#[test]
fn reencode_does_not_change_hash() {
    let engram = make_test_engram();
    let original_id = engram.id;
    let encoder = HdcEncoder { version: 2 };
    let new_fp = reencode(&engram, &encoder);
    // Fingerprint is changed, but id is not
    assert_eq!(original_id, engram.id);
    assert!(new_fp.is_some());
    assert_eq!(new_fp.unwrap().encoder_version, 2);
}

#[test]
fn cross_version_similarity_is_none() {
    let encoder_v1 = HdcEncoder { version: 1 };
    let encoder_v2 = HdcEncoder { version: 2 };
    let body = Body::Text("test".into());
    let fp1 = encoder_v1.encode(&body).unwrap();
    let fp2 = encoder_v2.encode(&body).unwrap();
    assert!(fp1.similarity_checked(&fp2).is_none());
}
```

---

## Open Questions

- Should `bit()` have bounds checking in release builds as well (at a ~5% perf cost)?
- Should `majority_bundle` require an odd number of input vectors to avoid tie-breaking?

## See Also

- [`01-hdc-vector.md`](01-hdc-vector.md) — vector type
- [`03-similarity-distance.md`](03-similarity-distance.md) — distance invariants
- [`06-examples.md`](06-examples.md) — examples verifying each invariant
