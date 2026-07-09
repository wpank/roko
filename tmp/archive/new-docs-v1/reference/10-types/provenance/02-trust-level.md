# Provenance — Trust Level

> The four-tier verification scale that determines how much weight an Engram's provenance contributes to reputation scoring.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md)  
**Used by**: [Score](../score/00-overview.md), [Trust Escalation](06-trust-escalation.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

`TrustLevel` is an enum with four variants, each carrying a numeric weight: `LocalAgent(0.25)`,
`SelfVerified(0.50)`, `PeerVerified(0.75)`, `ChainWitness(1.00)`. The weight is used in the
`Score::reputation` calculation. Trust level is **not** part of the ContentHash — it can
be upgraded as an Engram gains verification, without changing its identity.

---

## The Idea

Not all knowledge is created equal. A claim produced by a single local agent without any
external verification deserves less epistemic weight than one that three peer agents have
independently confirmed. `TrustLevel` formalises this intuition as an ordered scale from
unverified (LocalAgent) to chain-witnessed (ChainWitness).

The numeric weights (`0.25`, `0.50`, `0.75`, `1.00`) are the reputation multipliers fed
into `Score::reputation`. A ChainWitness Engram can achieve a reputation score 4× that of
a LocalAgent Engram with identical content quality.

---

## Specification

```rust
<!-- source: crates/roko-core/src/provenance.rs -->

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Produced by a single local agent. No external verification.
    /// Weight: 0.25
    LocalAgent,

    /// The producing agent has self-attested the claim (e.g., run its own
    /// validation logic over the content).
    /// Weight: 0.50
    SelfVerified,

    /// One or more peer agents have independently verified and confirmed
    /// the claim.
    /// Weight: 0.75
    PeerVerified,

    /// The claim has been committed to a distributed ledger and witnessed
    /// by the chain consensus.
    /// Weight: 1.00
    ChainWitness,
}
```

---

## Numeric Weights

```rust
<!-- source: crates/roko-core/src/provenance.rs -->

impl TrustLevel {
    /// Return the reputation weight for this trust level.
    pub fn weight(&self) -> f64 {
        match self {
            TrustLevel::LocalAgent   => 0.25,
            TrustLevel::SelfVerified => 0.50,
            TrustLevel::PeerVerified => 0.75,
            TrustLevel::ChainWitness => 1.00,
        }
    }
}
```

---

## Use in Score::reputation

`Score::reputation` is computed at Engram creation as:

```
reputation = author_base_reputation × trust_level.weight()
```

Where `author_base_reputation` is looked up from the Substrate's agent registry (defaults
to 0.5 for unknown authors). Thus:

| Author rep | TrustLevel | Computed reputation |
|---|---|---|
| 0.5 | LocalAgent | 0.125 |
| 0.5 | SelfVerified | 0.250 |
| 0.5 | PeerVerified | 0.375 |
| 0.5 | ChainWitness | 0.500 |
| 0.8 | PeerVerified | 0.600 |
| 1.0 | ChainWitness | 1.000 |

---

## Ordering

`TrustLevel` derives `PartialOrd` and `Ord`. The ordering is:
```
LocalAgent < SelfVerified < PeerVerified < ChainWitness
```

This ordering enables trust escalation checks:
```rust
if new_trust > current_trust {
    engram.provenance.trust = new_trust;
}
```

Trust can only be **upgraded** (escalated), never downgraded. Downgrading trust would
be modelled as adding a taint flag, not lowering the trust level.

---

## Default

```rust
<!-- source: crates/roko-core/src/provenance.rs -->

impl Default for TrustLevel {
    fn default() -> Self {
        TrustLevel::LocalAgent
    }
}
```

---

## Today vs. Planned

> Shipped today: `LocalAgent`, `SelfVerified`, `PeerVerified`. ChainWitness variant exists
> in the enum but the chain integration that would produce it is Deferred.
>
> Target state: Chain-integrated Roko nodes automatically escalate Engrams to `ChainWitness`
> after sufficient consensus confirmations.

---

## Invariants

1. `TrustLevel` is monotonically escalatable — upgrades only, never downgrades.
2. `weight()` is in `[0.0, 1.0]`.
3. `ChainWitness` has the maximum weight (`1.0`) — no trust level exists above it.
4. `Default::default()` returns `LocalAgent` — the most conservative trust level.
5. Trust escalation does not change the Engram's `ContentHash`.

---

## Failure Modes

| Failure | Cause | Recovery |
|---|---|---|
| Trust downgraded | Bug in escalation path | Enforce monotonicity check before write |
| ChainWitness assigned to non-chain Engram | Manual override | Substrate validates source before setting ChainWitness |
| Weight returns value outside [0.0, 1.0] | Bug in `weight()` | Unit test `trust_weights_in_range` covers all variants |

---

## Open Questions

- Should trust levels be granular within a tier (e.g., `PeerVerified(n)` where `n` is the
  number of verifying peers)? Not currently planned; a fixed four-tier scale is simpler.
- Should `ChainWitness` carry a `block_height` or `tx_hash` for audit? Deferred to chain
  integration phase.

## See Also

- [`06-trust-escalation.md`](06-trust-escalation.md) — how trust is upgraded
- [`../score/00-overview.md`](../score/00-overview.md) — how trust feeds reputation
- [`03-taint-flags.md`](03-taint-flags.md) — the complementary reliability signal
