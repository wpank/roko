# Provenance — Trust Escalation

> The protocol by which an Engram's TrustLevel is upgraded from LocalAgent toward ChainWitness.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Trust Level](02-trust-level.md)  
**Used by**: [Substrate](../../../subsystems/substrate/)  
**Last reviewed**: 2026-04-19

---

## TL;DR

Trust escalation is a monotonic upgrade: `LocalAgent → SelfVerified → PeerVerified →
ChainWitness`. Each step requires evidence from an external source (the agent itself,
peer agents, or a chain). Escalation updates the `trust` field in the Substrate's
provenance metadata without changing the Engram's `ContentHash`. Escalation can be
triggered explicitly (by a verification call) or automatically (by the Substrate's
background verifier).

---

## The Idea

A freshly minted Engram from a local agent has `TrustLevel::LocalAgent`. As it is used,
observed to be accurate, and confirmed by other agents, its trust level rises. The system
automatically rewards reliable knowledge with higher trust.

The escalation protocol answers: "What counts as evidence for each level?"

| From → To | Required evidence |
|---|---|
| LocalAgent → SelfVerified | The authoring agent runs its own consistency check and signs it |
| SelfVerified → PeerVerified | At least one peer agent independently validates the content |
| PeerVerified → ChainWitness | The Engram hash is committed to the distributed ledger |

---

## Specification

```rust
<!-- source: crates/roko-core/src/provenance.rs -->

impl Provenance {
    /// Attempt to escalate trust to `new_level`.
    /// Returns Ok(()) if escalation is valid (new_level > current trust).
    /// Returns Err(TrustError::CannotDowngrade) if new_level ≤ current trust.
    pub fn escalate_trust(&mut self, new_level: TrustLevel) -> Result<(), TrustError> {
        if new_level <= self.trust {
            return Err(TrustError::CannotDowngrade {
                current: self.trust,
                attempted: new_level,
            });
        }
        self.trust = new_level;
        Ok(())
    }
}
```

---

## Evidence Requirements

### LocalAgent → SelfVerified

The authoring agent calls a self-verification routine and records the result:

```rust
<!-- source: crates/roko-core/src/verifier.rs -->

/// Mark an Engram as self-verified after the authoring agent runs its own checks.
pub fn self_verify(
    substrate: &Substrate,
    id: &ContentHash,
    agent_id: &str,
    check_result: SelfCheckResult,
) -> Result<(), VerifierError> {
    let mut engram = substrate.get_mut(id)?;
    // Only the authoring agent can self-verify
    if engram.provenance.author != format!("agent-{}", agent_id) {
        return Err(VerifierError::NotAuthor);
    }
    if !check_result.passed {
        return Err(VerifierError::CheckFailed(check_result.reason));
    }
    engram.provenance.escalate_trust(TrustLevel::SelfVerified)?;
    substrate.put(engram)?;
    Ok(())
}
```

### SelfVerified → PeerVerified

A peer agent reviews the Engram and submits a verification vote:

```rust
<!-- source: crates/roko-core/src/verifier.rs -->

/// Submit a peer verification vote.
/// Once PEER_VERIFY_QUORUM votes are accumulated, trust is escalated.
pub fn peer_verify(
    substrate: &Substrate,
    id: &ContentHash,
    peer_id: &str,
    verdict: PeerVerdict,
) -> Result<(), VerifierError> {
    substrate.verifier_log.record_peer_vote(id, peer_id, verdict)?;
    let votes = substrate.verifier_log.count_confirm_votes(id);
    if votes >= PEER_VERIFY_QUORUM {
        let mut engram = substrate.get_mut(id)?;
        engram.provenance.escalate_trust(TrustLevel::PeerVerified)?;
        substrate.put(engram)?;
    }
    Ok(())
}

/// Minimum number of confirming peer votes for PeerVerified escalation.
pub const PEER_VERIFY_QUORUM: usize = 2;
```

### PeerVerified → ChainWitness

Chain witness is set by the chain integration module (deferred; see "Today vs. Planned").

---

## Substrate Integration

Escalation is a **metadata update** — it does not recompute the ContentHash:

```rust
<!-- source: crates/roko-fs/src/substrate.rs -->

pub fn escalate_trust(
    &self,
    id: &ContentHash,
    new_level: TrustLevel,
    evidence: TrustEvidence,
) -> Result<(), SubstrateError> {
    let mut engram = self.get_mut(id)?;
    engram.provenance.escalate_trust(new_level)?;
    self.put(engram)?;
    // Audit log records evidence for each escalation
    self.audit_log.record_trust_escalation(id, new_level, evidence);
    Ok(())
}
```

---

## Today vs. Planned

> Shipped today: `LocalAgent`, `SelfVerified`, `PeerVerified` escalation. The verifier
> infrastructure is shipped. `PEER_VERIFY_QUORUM = 2` is hardcoded.
>
> Target state: `ChainWitness` escalation via distributed ledger commit. `PEER_VERIFY_QUORUM`
> configurable per deployment. Automatic escalation from background verifier loop.

---

## Invariants

1. Trust escalation is monotonic — `new_level > current_level` is required.
2. `ChainWitness` is the ceiling — no escalation above it exists.
3. Escalation does not change the Engram's `ContentHash`.
4. Every escalation is recorded in the audit log with its evidence.
5. Only the authoring agent may initiate `SelfVerified` escalation.

---

## Open Questions

- Should there be a decay on trust level? (I.e., if an Engram is not accessed for a long
  time, does its trust level decrease?) Not currently planned; trust is monotonic.
- Should `PEER_VERIFY_QUORUM` be per-Kind (KnowledgeEntry might require 3; ToolTrace only 1)?

## See Also

- [`02-trust-level.md`](02-trust-level.md) — TrustLevel enum and weights
- [`07-invariants.md`](07-invariants.md) — full provenance invariants
- [`../score/00-overview.md`](../score/00-overview.md) — how trust feeds reputation
