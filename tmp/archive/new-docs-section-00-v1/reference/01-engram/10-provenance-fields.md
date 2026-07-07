# Engram — Provenance Fields

> The `provenance: Provenance` field records who produced the Engram, at what trust level, and what taint it inherits.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Provenance type](../10-types/provenance/00-overview.md)  
**Used by**: Gate pipeline, reputation scoring, taint propagation  
**Last reviewed**: 2026-04-19

---

## TL;DR

Every Engram carries a `Provenance` that records the author (included in identity hash),
the trust level (not in hash), and any taint inherited from upstream. Taint is
one-directional: once an Engram is tainted, its descendants inherit that taint. Trust
levels form a hierarchy from `LocalAgent` to `ChainWitness`.

---

## The Idea

Without provenance, you cannot answer: "Who said this? Should I trust it? Has it been
contaminated by a known-bad source?" Provenance makes these questions answerable at
any point in the lineage chain.

Taint is the "blast radius" mechanism: if an Engram is discovered to be produced by a
compromised or untrustworthy source, its `taint` flag is set, and all descendant Engrams
inherit the taint automatically. The system can then quarantine the entire affected subtree.

---

## Specification

The `Provenance` type is specified in detail in
[`../10-types/provenance/`](../10-types/provenance/README.md). This page covers the
attachment of Provenance to Engram.

### The Provenance Field

```rust
<!-- source: crates/roko-core/src/engram.rs -->

/// Producer attribution and trust classification.
/// Only `author` is included in the identity hash.
pub provenance: Provenance,
```

### Provenance Sub-struct

```rust
<!-- source: crates/roko-core/src/provenance.rs -->

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    /// The agent, subsystem, or external source that produced this Engram.
    /// Included in the identity hash.
    pub author: String,

    /// Trust level of the author.
    /// Not included in the identity hash — can be upgraded by attestation.
    pub trust: TrustLevel,

    /// Whether this Engram is tainted by a known-bad upstream source.
    /// Not in the identity hash — can be set post-hoc by the trust system.
    pub tainted: bool,

    /// Custody record: chain-of-custody for auditable actions.
    /// Not in the identity hash.
    pub custody: Vec<CustodyRecord>,
}
```

### TrustLevel Hierarchy

```rust
<!-- source: crates/roko-core/src/provenance.rs -->

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Produced by local, unverified agent process.
    LocalAgent = 0,
    /// Self-verified: agent checked its own output against known constraints.
    SelfVerified = 1,
    /// Peer-verified: another agent in the mesh reviewed and attested.
    PeerVerified = 2,
    /// Chain-witnessed: recorded on a verifiable distributed ledger.
    ChainWitness = 3,
}
```

Higher `TrustLevel` → higher `reputation` score axis → higher effective score.

---

## Setting Provenance on Emission

```rust
<!-- source: crates/roko-core/src/engram_builder.rs -->

// Minimal: anonymous provenance (trust = LocalAgent)
let engram = EngramBuilder::new()
    .kind(Kind::AgentOutput)
    .body(/* ... */)
    .build()?;
// provenance = Provenance::anonymous()

// Full: attributed provenance
let engram = EngramBuilder::new()
    .kind(Kind::AgentOutput)
    .body(/* ... */)
    .provenance(Provenance {
        author: "roko-agent-v2.1".to_string(),
        trust: TrustLevel::SelfVerified,
        tainted: false,
        custody: vec![],
    })
    .build()?;
```

---

## Taint Propagation

Taint is a boolean flag that propagates through the lineage DAG. When an Engram's
`tainted` is set to `true`, all its descendants are automatically tainted by the Substrate's
taint propagation pass:

```rust
<!-- source: crates/roko-core/src/substrate.rs -->

fn taint(&self, id: &ContentHash, reason: &str) -> Result<usize, SubstrateError> {
    // Set tainted = true on this Engram and all descendants
    let affected = self.descendants(id, usize::MAX);
    for engram in &affected {
        self.set_tainted(engram.id, true)?;
    }
    self.record_taint_event(id, reason);
    Ok(affected.len())
}
```

Taint is **one-way**: once set, it is not automatically removed. Clearing taint requires
explicit operator action.

---

## Trust Upgrades via Attestation

Trust levels can be upgraded without changing the Engram's id:

```rust
<!-- source: crates/roko-core/src/substrate.rs -->

trait Substrate {
    /// Upgrade the trust level of an existing Engram.
    /// Does not change id. Records the attestation in custody.
    fn attest(
        &self,
        id: &ContentHash,
        new_trust: TrustLevel,
        attester: &str,
    ) -> Result<(), SubstrateError>;
}
```

---

## How Provenance Affects Other Subsystems

### Reputation Scoring

The `Scorer` converts `TrustLevel` to the `reputation` score axis:

```
LocalAgent  → 0.25
SelfVerified → 0.5
PeerVerified → 0.75
ChainWitness → 1.0
```

### Gate Pipeline

Gates can require a minimum trust level:

```
gate.min_trust = TrustLevel::PeerVerified
engram.provenance.trust < min_trust → fail
```

### Taint Filtering

Retrievals can exclude tainted Engrams:

```rust
substrate.find_similar(query, threshold, limit, /* exclude_tainted = */ true)
```

---

## Invariants

1. `provenance.author` is included in the identity hash; changing author → new Engram
2. `provenance.tainted` is not in the identity hash; taint can be set post-hoc
3. `provenance.trust` is not in the identity hash; trust can be upgraded by attestation
4. `TrustLevel` values are ordered: `LocalAgent < SelfVerified < PeerVerified < ChainWitness`

---

## Failure Modes

| Failure | Cause | Recovery |
|---------|-------|----------|
| Taint not propagated | Substrate taint pass not run | Run `substrate.propagate_taint()` on schedule |
| Incorrect trust level | Author misconfigured | `attest()` to correct; audit custody record |
| Anonymous Engram in production | `.provenance()` not called on builder | Lint rule: warn on `Provenance::anonymous()` in production |

---

## See Also

- [`../10-types/provenance/00-overview.md`](../10-types/provenance/00-overview.md) — full Provenance type
- [`../10-types/provenance/02-trust-levels.md`](../10-types/provenance/02-trust-levels.md)
- [`../10-types/provenance/03-taint.md`](../10-types/provenance/03-taint.md)
- [`06-lineage-dag.md`](06-lineage-dag.md) — lineage DAG that taint propagates through
