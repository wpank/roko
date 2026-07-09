# Provenance — API Reference

> Complete signatures for every public method on the Provenance type family.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md)  
**Last reviewed**: 2026-04-19

---

## `struct Provenance`

```rust
<!-- source: crates/roko-core/src/provenance.rs -->

pub struct Provenance {
    pub author: String,
    pub trust: TrustLevel,
    pub taint: Option<BTreeSet<TaintFlag>>,
}
```

### Constructors

| Method | Signature | Description |
|---|---|---|
| `new` | `(author: impl Into<String>, trust: TrustLevel, taint: Option<BTreeSet<TaintFlag>>) -> Result<Self, ProvenanceError>` | Full constructor with validation |
| `local_agent` | `(id: impl Into<String>) -> Self` | Convenience: `author="agent-{id}"`, `trust=LocalAgent`, `taint=None` |

### Mutation Methods

| Method | Signature | Description |
|---|---|---|
| `escalate_trust` | `(&mut self, new_level: TrustLevel) -> Result<(), TrustError>` | Monotonically upgrade trust level |
| `add_taint` | `(&mut self, flag: TaintFlag)` | Add a taint flag to the set |

### Query Methods

| Method | Signature | Description |
|---|---|---|
| `is_tainted` | `(&self) -> bool` | True if any taint flags are present |
| `has_flag` | `(&self, flag: &TaintFlag) -> bool` | True if the specific flag is in the taint set |
| `trust_weight` | `(&self) -> f64` | Delegates to `self.trust.weight()` |

### Static Methods

| Method | Signature | Description |
|---|---|---|
| `inherit_taint` | `(parents: &[&Provenance]) -> Option<BTreeSet<TaintFlag>>` | Compute the propagating taint set from parent provenances |

---

## `enum TrustLevel`

```rust
<!-- source: crates/roko-core/src/provenance.rs -->

pub enum TrustLevel {
    LocalAgent,
    SelfVerified,
    PeerVerified,
    ChainWitness,
}
```

| Method | Signature | Description |
|---|---|---|
| `weight` | `(&self) -> f64` | Numeric weight: LocalAgent=0.25, SelfVerified=0.50, PeerVerified=0.75, ChainWitness=1.00 |
| `Default::default` | `() -> Self` | Returns `TrustLevel::LocalAgent` |

`TrustLevel` implements `PartialOrd` and `Ord` (ascending order: LocalAgent < … < ChainWitness).

---

## `enum TaintFlag`

```rust
<!-- source: crates/roko-core/src/provenance.rs -->

pub enum TaintFlag {
    UnverifiedSource,
    PossibleHallucination,
    CompromisedAuthor,
    SuspiciousContext,
    OutdatedAt { superseded_by: ContentHash },
    Custom(String),
}
```

`TaintFlag` implements `PartialOrd`, `Ord`, `Hash` (required for `BTreeSet` membership).

| Flag | Propagates through lineage? |
|---|---|
| `UnverifiedSource` | No |
| `PossibleHallucination` | Yes |
| `CompromisedAuthor` | Yes |
| `SuspiciousContext` | No |
| `OutdatedAt` | No |
| `Custom(_)` | No |

---

## Error Types

```rust
<!-- source: crates/roko-core/src/provenance.rs -->

#[derive(Debug, thiserror::Error)]
pub enum ProvenanceError {
    #[error("author must not be empty")]
    EmptyAuthor,
}

#[derive(Debug, thiserror::Error)]
pub enum TrustError {
    #[error("cannot downgrade trust from {current:?} to {attempted:?}")]
    CannotDowngrade {
        current: TrustLevel,
        attempted: TrustLevel,
    },
}
```

---

## Substrate Methods (for reference)

These live in `roko-fs`, not `roko-core`, but are documented here for discoverability:

| Method | Crate | Signature |
|---|---|---|
| `escalate_trust` | `roko-fs` | `(&self, id: &ContentHash, new_level: TrustLevel, evidence: TrustEvidence) -> Result<(), SubstrateError>` |
| `add_taint` | `roko-fs` | `(&self, id: &ContentHash, flag: TaintFlag, reason: &str) -> Result<(), SubstrateError>` |
| `self_verify` | `roko-core` (verifier) | `(substrate: &Substrate, id: &ContentHash, agent_id: &str, result: SelfCheckResult) -> Result<(), VerifierError>` |
| `peer_verify` | `roko-core` (verifier) | `(substrate: &Substrate, id: &ContentHash, peer_id: &str, verdict: PeerVerdict) -> Result<(), VerifierError>` |

---

## Open Questions

- Should `add_taint` be available directly on `Provenance` struct or only through the
  Substrate? Currently on the struct (for in-memory construction) and on the Substrate
  (for persisted Engrams).
- Should there be a `clear_taint()` method for use in testing? Not in production API;
  could be gated behind `#[cfg(test)]`.

## See Also

- [`09-examples.md`](09-examples.md) — worked examples for every method
- [`07-invariants.md`](07-invariants.md) — invariants for every method
- [`06-trust-escalation.md`](06-trust-escalation.md) — escalation protocol
