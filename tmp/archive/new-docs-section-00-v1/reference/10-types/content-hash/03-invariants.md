# ContentHash — Invariants and Collision Resistance

> Security properties, correctness invariants, and their enforcement locations.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md), [Canonical Encoding](01-canonical-encoding.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

`ContentHash` is a 32-byte BLAKE3 digest. Its security and correctness rest on three
guarantees: determinism (same input, same output), second-preimage resistance (different
input, almost certainly different output), and field isolation (changes to mutable fields
do not change the hash). This page lists every invariant with its enforcement location
and a test case.

---

## Correctness Invariants

| # | Invariant | Enforcement |
|---|---|---|
| C1 | `ContentHash::compute(e) == ContentHash::compute(e)` — same Engram always produces the same hash | Pure function, no external state |
| C2 | Two Engrams with identical canonical-encoding inputs have the same hash | BLAKE3 determinism |
| C3 | Two Engrams with any difference in canonical fields have a different hash | BLAKE3 collision resistance |
| C4 | `ContentHash::verify(engram)` returns `true` iff the stored hash matches the recomputed hash | Explicit recomputation |
| C5 | `canonical_encode()` reads only the 6 included fields; never reads `score`, `decay`, `provenance.trust`, `provenance.taint`, `fingerprint` | Code-level constraint; verified by CI audit |

---

## Mutability Invariants

| # | Invariant | Enforcement |
|---|---|---|
| M1 | Changing `decay` on an Engram does not change its `id` | `decay` excluded from `canonical_encode()` |
| M2 | Changing `score` on an Engram does not change its `id` | `score` excluded from `canonical_encode()` |
| M3 | Escalating `provenance.trust` does not change the `id` | `trust` excluded from `canonical_encode()` |
| M4 | Adding a `provenance.taint` flag does not change the `id` | `taint` excluded from `canonical_encode()` |
| M5 | Updating `fingerprint` does not change the `id` | `fingerprint` excluded from `canonical_encode()` |

---

## Security Invariants

| # | Property | Basis |
|---|---|---|
| S1 | Collision probability ≤ 2^-128 | BLAKE3 provides 128-bit collision resistance |
| S2 | Preimage resistance | BLAKE3 provides preimage resistance up to 256 bits |
| S3 | Second-preimage resistance | BLAKE3 provides second-preimage resistance |
| S4 | No length-extension attacks | BLAKE3 is not Merkle-Damgård; length extension is not applicable |

---

## Structural Invariants

| # | Invariant | Enforcement |
|---|---|---|
| ST1 | `ContentHash` is always exactly 32 bytes | `[u8; 32]` type |
| ST2 | `ContentHash::zero()` returns `[0u8; 32]` | Literal |
| ST3 | Zero hash is reserved and must not be produced by `compute()` on a valid Engram | Checked in `compute()` — BLAKE3 would not produce all-zeros for any real input |
| ST4 | `from_hex()` rejects strings that are not exactly 64 lowercase hex characters | Validation in `from_hex()` |

---

## Test Coverage

```rust
<!-- source: crates/roko-core/tests/content_hash_invariants.rs -->

#[test]
fn same_engram_same_hash() {
    let e = make_test_engram();
    assert_eq!(ContentHash::compute(&e), ContentHash::compute(&e));
}

#[test]
fn different_body_different_hash() {
    let e1 = make_engram_with_text("Hello");
    let e2 = make_engram_with_text("Hello!");
    assert_ne!(e1.id, e2.id);
}

#[test]
fn decay_change_no_hash_change() {
    let mut e = make_test_engram();
    let original_hash = e.id;
    if let Decay::Demurrage(ref mut p) = e.decay {
        p.balance = 0.3;
    }
    assert_eq!(original_hash, ContentHash::compute(&e));
}

#[test]
fn trust_escalation_no_hash_change() {
    let e = make_test_engram();
    let mut e2 = e.clone();
    e2.provenance.trust = TrustLevel::ChainWitness;
    assert_eq!(ContentHash::compute(&e), ContentHash::compute(&e2));
}

#[test]
fn taint_addition_no_hash_change() {
    let e = make_test_engram();
    let mut e2 = e.clone();
    e2.provenance.taint = Some(BTreeSet::from([TaintFlag::PossibleHallucination]));
    assert_eq!(ContentHash::compute(&e), ContentHash::compute(&e2));
}

#[test]
fn verify_detects_tampering() {
    let e = make_test_engram();
    let mut tampered = e.clone();
    tampered.tags.insert("extra".into(), "tag".into());
    // Stored hash is still e.id, but canonical fields changed
    assert!(!e.id.verify(&tampered));
}

#[test]
fn hex_roundtrip() {
    let e = make_test_engram();
    let hex = e.id.to_hex();
    let recovered = ContentHash::from_hex(&hex).unwrap();
    assert_eq!(e.id, recovered);
}

#[test]
fn hash_is_32_bytes() {
    let e = make_test_engram();
    assert_eq!(e.id.as_bytes().len(), 32);
}
```

---

## Failure Modes

| Failure | Cause | Recovery |
|---|---|---|
| `verify()` returns false for unmodified Engram | Clock skew causing `created_at_ms` to change | `created_at_ms` must be frozen at construction |
| Hash mismatch after deserialization | Field encoding changed between versions | All canonical encoding changes are breaking; never change without version bump |
| All-zero hash produced | Hardware RNG failure or bug | `compute()` assertion guards against this; fail hard |

---

## Open Questions

- Should `ContentHash` include an algorithm identifier byte so that future migration
  to a different hash function can be detected? Not currently planned.
- Should `verify()` be called automatically on every Substrate `get()` operation?
  Currently opt-in; might add as a debug/audit mode.

## See Also

- [`01-canonical-encoding.md`](01-canonical-encoding.md) — the encoding that is hashed
- [`04-examples.md`](04-examples.md) — examples demonstrating each invariant
