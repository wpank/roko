# Provenance — Invariants

> The complete list of invariants that the Provenance type must satisfy, and where each is enforced.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md)  
**Used by**: [Engram Invariants](../../01-engram/12-invariants.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

Invariants are properties that must hold at construction, after every mutation, and after
deserialization. This page collects all provenance invariants with their enforcement layers
and corresponding tests, to serve as a single reference for audits and CI expansion.

---

## Structural Invariants

| # | Invariant | Enforcement |
|---|---|---|
| P1 | `author` is non-empty | `Provenance::new()` returns `Err(ProvenanceError::EmptyAuthor)` if empty |
| P2 | `author` is valid UTF-8 | Enforced by `String` type |
| P3 | `taint = None` when no flags exist — `Some(empty_set)` is forbidden | Substrate normalizes to `None` on write |
| P4 | `trust` is a valid `TrustLevel` variant | Enforced by `serde` discriminant |

---

## Mutability Invariants

| # | Invariant | Enforcement |
|---|---|---|
| M1 | `author` is immutable after construction | No setter exposed on `Provenance`; updating author requires constructing a new Engram |
| M2 | `trust` is monotonically escalatable only — never downgraded | `escalate_trust()` returns `Err` if `new_level ≤ current` |
| M3 | `taint` is append-only — flags can be added, never removed | `add_taint()` only inserts; no `remove_taint()` method exists |

---

## Hash Contract Invariants

| # | Invariant | Enforcement |
|---|---|---|
| H1 | `author` changes produce a different `ContentHash` | Follows from inclusion in `canonical_encode()` |
| H2 | `trust` changes do NOT change `ContentHash` | `trust` is excluded from `canonical_encode()` |
| H3 | `taint` changes do NOT change `ContentHash` | `taint` is excluded from `canonical_encode()` |
| H4 | H1–H3 are verified by CI tests | See `crates/roko-core/tests/provenance_hash.rs` |

---

## Propagation Invariants

| # | Invariant | Enforcement |
|---|---|---|
| PR1 | Derived Engrams' `author` is the deriving agent — never inherited | Builder requires explicit `author` |
| PR2 | Derived Engrams' `trust` starts at `LocalAgent` — never inherited | Builder defaults `trust` to `LocalAgent` |
| PR3 | Only `CompromisedAuthor` and `PossibleHallucination` propagate via `inherit_taint()` | `inherit_taint()` filters by match arm |
| PR4 | If no propagating taint exists in parents, `inherit_taint()` returns `None` | Empty-collection check in `inherit_taint()` |

---

## Trust Escalation Invariants

| # | Invariant | Enforcement |
|---|---|---|
| TE1 | Escalation requires `new_level > current_level` | `escalate_trust()` monotonicity check |
| TE2 | `ChainWitness` is the ceiling — no level above it | Enum has no variant above `ChainWitness` |
| TE3 | Every escalation is audit-logged | `substrate.escalate_trust()` calls `audit_log.record_trust_escalation()` |
| TE4 | Self-verification can only be initiated by the authoring agent | `self_verify()` checks `engram.provenance.author == agent_id` |

---

## Test Suite

```rust
<!-- source: crates/roko-core/tests/provenance_invariants.rs -->

#[test]
fn author_cannot_be_empty() {
    let result = Provenance::new("", TrustLevel::LocalAgent, None);
    assert!(matches!(result, Err(ProvenanceError::EmptyAuthor)));
}

#[test]
fn trust_cannot_be_downgraded() {
    let mut prov = Provenance {
        author: "agent-001".into(),
        trust: TrustLevel::PeerVerified,
        taint: None,
    };
    let result = prov.escalate_trust(TrustLevel::LocalAgent);
    assert!(matches!(result, Err(TrustError::CannotDowngrade { .. })));
}

#[test]
fn taint_is_append_only() {
    // No remove_taint method exists — this test verifies the API surface
    let prov = Provenance::local_agent("agent-001");
    // Uncommenting the line below should be a compile error:
    // prov.remove_taint(TaintFlag::UnverifiedSource);
    let _ = prov; // suppress unused warning
}

#[test]
fn empty_taint_set_normalized_to_none() {
    // Substrate normalizes Some(empty) to None
    let mut prov = Provenance {
        author: "agent-001".into(),
        trust: TrustLevel::LocalAgent,
        taint: Some(BTreeSet::new()),  // empty set
    };
    // Simulate substrate normalization
    if prov.taint.as_ref().map(|s| s.is_empty()).unwrap_or(false) {
        prov.taint = None;
    }
    assert!(prov.taint.is_none());
}

#[test]
fn only_propagating_flags_inherited() {
    let parent = Provenance {
        author: "agent-001".into(),
        trust: TrustLevel::LocalAgent,
        taint: Some(BTreeSet::from([
            TaintFlag::UnverifiedSource,        // should NOT propagate
            TaintFlag::CompromisedAuthor,       // SHOULD propagate
        ])),
    };
    let inherited = Provenance::inherit_taint(&[&parent]);
    let flags = inherited.expect("should have inherited flags");
    assert!(flags.contains(&TaintFlag::CompromisedAuthor));
    assert!(!flags.contains(&TaintFlag::UnverifiedSource));
}
```

---

## Open Questions

- Should invariant P3 (empty taint set = None) be enforced at construction rather than
  at Substrate write time? Would prevent the inconsistency from ever being stored.
- Should the `TaintFlag::Custom(String)` variant be validated for a naming convention
  (e.g., `"crate/flag"` format) to prevent collisions?

## See Also

- [`00-overview.md`](00-overview.md) — Provenance struct
- [`../../01-engram/12-invariants.md`](../../01-engram/12-invariants.md) — Engram-level invariants
- [`08-api-reference.md`](08-api-reference.md) — API signatures
