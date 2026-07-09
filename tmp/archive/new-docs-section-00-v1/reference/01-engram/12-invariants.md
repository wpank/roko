# Engram — Invariants

> A complete list of what must always be true about an Engram, and where each invariant is enforced.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Struct reference](01-struct-reference.md), [Builder](07-builder-pattern.md)  
**Used by**: testing, auditing, substrate ingest pipeline  
**Last reviewed**: 2026-04-19

---

## TL;DR

Engram invariants fall into three groups: identity invariants (properties of `id`),
structural invariants (field constraints), and consistency invariants (relationships
between fields). Most are enforced by `EngramBuilder::build()`. The Substrate
re-enforces hash integrity on every ingest. Tests should verify all invariants.

---

## Identity Invariants

| # | Invariant | Enforced by |
|---|-----------|-------------|
| I1 | `id == blake3(canonical_encode(kind, body, created_at_ms, provenance.author, lineage, tags))` | `EngramBuilder::build()`, `Substrate::ingest()` |
| I2 | `id` is exactly 32 bytes | Type system (`ContentHash([u8; 32])`) |
| I3 | `id` is unique within the Substrate (content addressing) | Substrate `insert` is idempotent on hash collision |
| I4 | Changing any hash-included field produces a different `id` | BLAKE3 preimage resistance |
| I5 | Changing hash-excluded fields (`score`, `decay`, `fingerprint`) does not change `id` | Enforcement by omission from canonical encoding |

---

## Structural Invariants

| # | Invariant | Enforced by |
|---|-----------|-------------|
| S1 | `created_at_ms > 0` | `EngramBuilder::build()` |
| S2 | `body` variant matches `kind` | `EngramBuilder::build()` |
| S3 | `lineage` entries are distinct ContentHashes | `EngramBuilder::build()` |
| S4 | `lineage` does not contain `id` (no self-reference) | `EngramBuilder::build()` |
| S5 | `tags` keys and values are valid UTF-8 | Rust type system (`String`) |
| S6 | `tags` is a `BTreeMap` (ordered) | Rust type system |
| S7 | `score.*` axis values are in [0.0, 1.0] | `Score` setter validation; `Substrate::update_score()` |
| S8 | `decay.weight_at(t) ∈ [0.0, 1.0]` for all t | Decay variant implementations |
| S9 | `Demurrage::balance ∈ [0.0, 1.0]` | Demurrage impl clamping |
| S10 | `CustomBody::type_tag` is non-empty | `EngramBuilder::build()` |
| S11 | JSON fields in Body variants contain valid JSON | `EngramBuilder::build()` |

---

## Consistency Invariants

| # | Invariant | Enforced by |
|---|-----------|-------------|
| C1 | No transitive lineage cycles | `Substrate::ingest()` cycle detection |
| C2 | `fingerprint.encoder_version` is a registered version (if `Some`) | `Substrate::ingest()` registry lookup |
| C3 | Tainted Engrams propagate taint to all descendants | `Substrate::taint()` propagation pass |
| C4 | `TrustLevel` upgrades are monotonic (no downgrade without explicit reset) | `Substrate::attest()` validation |
| C5 | `created_at_ms` clock skew ≤ 60 s produces a warning; > 3600 s produces an error | `Substrate::ingest()` |

---

## Invariant Enforcement Points

### EngramBuilder::build()

Enforces: I1, S1, S2, S3, S4, S10, S11

The build step is the primary enforcement gate. Any Engram that passes `build()` satisfies
the identity and structural invariants by construction.

### Substrate::ingest()

Enforces: I1 (re-verification), C1, C2, C5

The Substrate verifies the hash on every ingest. This catches:
- Engrams constructed by bypassing the builder.
- Engrams modified after construction.
- Engrams replicated from untrusted sources.

```rust
<!-- source: crates/roko-core/src/substrate.rs -->

fn ingest(&self, engram: Engram) -> Result<IngestResult, SubstrateError> {
    // Re-verify hash
    if !engram.verify_id() {
        return Err(SubstrateError::HashMismatch { id: engram.id });
    }
    // Check for lineage cycles
    if detect_cycle(self, engram.id, &engram.lineage) {
        return Err(SubstrateError::LineageCycle);
    }
    // Check encoder version if fingerprint present
    if let Some(fp) = &engram.fingerprint {
        if !HDC_ENCODER_REGISTRY.is_registered(fp.encoder_version) {
            return Err(SubstrateError::UnknownEncoderVersion(fp.encoder_version));
        }
    }
    // Clock skew check
    let skew_ms = (engram.created_at_ms - now_ms()).abs();
    if skew_ms > 3_600_000 { return Err(SubstrateError::ClockSkewTooLarge(skew_ms)); }
    if skew_ms > 60_000 { warn!("Clock skew {} ms for Engram {}", skew_ms, engram.id.to_hex()); }
    // Idempotent insert
    self.insert_if_absent(engram)
}
```

### Score and Decay Setters

Enforces: S7, S8, S9

```rust
<!-- source: crates/roko-core/src/substrate.rs -->

fn update_score(&self, id: &ContentHash, score: Score) -> Result<(), SubstrateError> {
    score.validate()?;  // checks all axis values in [0.0, 1.0]
    self.set_score(id, score)
}
```

---

## Testing Invariants

The `roko-core` test suite verifies all invariants:

```rust
<!-- source: crates/roko-core/tests/invariants.rs -->

#[test]
fn test_hash_includes_kind() {
    let e1 = EngramBuilder::new().kind(Kind::AgentOutput).body(/* ... */).build().unwrap();
    let e2 = EngramBuilder::new().kind(Kind::Observation).body(/* ... */).build().unwrap();
    assert_ne!(e1.id, e2.id);
}

#[test]
fn test_score_does_not_affect_hash() {
    let mut e = EngramBuilder::new().kind(Kind::AgentOutput).body(/* ... */).build().unwrap();
    let original_id = e.id;
    e.score = Score { confidence: 0.99, ..Score::default() };
    assert_eq!(e.id, original_id);  // hash unchanged
}

#[test]
fn test_cycle_detection() {
    // Attempt to create a cycle: A → B → A
    // Should fail at Substrate ingest of A with parent B
    // (B's lineage already contains A's id)
}
```

---

## Open Questions

- Should `created_at_ms` clock skew threshold be configurable? Currently 60s/3600s hard-coded.
- Should `lineage` have a maximum length? Currently unbounded; deep lineage chains could cause slow cycle detection.

---

## See Also

- [`07-builder-pattern.md`](07-builder-pattern.md) — primary enforcement gate
- [`01-struct-reference.md`](01-struct-reference.md) — field definitions
- [`02-content-hash.md`](02-content-hash.md) — hash invariant details
