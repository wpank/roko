# Depth: Validation Registry

> Gate-based proof attestation, the 3-registry pattern, duplicate rejection,
> independent attestation, and verification lookup.

**Parent chapter:** [37-SHARED-ECONOMY](../../37-SHARED-ECONOMY.md) Section 8
**Source:** `crates/roko-chain/src/validation_registry.rs`

---

## The 3-Registry Pattern

Three registries form a complete trust record:

| Registry | Question | Data |
|---|---|---|
| **Identity** (Agent Registry) | Who is this agent? | Passport, capabilities, tier, prompt hash |
| **Reputation** (Reputation Registry) | How well do they perform? | 7-domain EMA scores, discipline state |
| **Validation** (Validation Registry) | What have they done? | Work proofs, gate scores, attestations |

Each registry is independent but cross-referenced. A reputation update
references a validation proof. A validation proof references an identity
passport. This separation of concerns means each registry can be queried,
audited, and maintained independently.

---

## Proof Submission

When an agent completes work and it passes gate verification, a proof is
submitted:

```rust
pub struct WorkProof {
    pub passport_id: AgentId,
    pub job_hash: [u8; 32],
    pub deliverable_merkle_root: [u8; 32],
    pub gate_results: Vec<u8>,
    pub clearing_cert: Vec<u8>,
    pub block_number: u64,
    pub timestamp: u64,
}

pub struct GateScore {
    pub gate_kind: String,
    pub score: f64,        // [0.0, 1.0]
    pub passed: bool,
}
```

### Acceptance Criteria

A proof is accepted when its overall gate pass rate meets the configured
minimum:

```rust
let pass_count = gate_scores.iter().filter(|g| g.passed).count();
let overall_pass_rate = pass_count as f64 / gate_scores.len().max(1) as f64;
let accepted = overall_pass_rate >= self.config.min_gate_pass_rate;
```

Default `min_gate_pass_rate`: 0.5 (at least half of gates must pass).

### Duplicate Rejection

When configured (`reject_duplicates = true`, the default), a second proof
submission for the same job by the same agent is rejected:

```rust
if self.config.reject_duplicates
    && existing.iter().any(|r| r.proof.passport_id == proof.passport_id)
{
    return Err(ValidationError::DuplicateProof { job_hash, passport_id });
}
```

Different agents can submit proofs for the same job -- this supports
independent attestation.

---

## Independent Attestation

An optional `attester_passport_id` allows a third-party agent to vouch for
the proof:

```rust
pub struct ValidationRecord {
    pub proof: WorkProof,
    pub gate_scores: Vec<GateScore>,
    pub overall_pass_rate: f64,
    pub accepted: bool,
    pub attester_passport_id: Option<AgentId>,
}
```

Attested proofs carry higher credibility in downstream trust computations.
The verification result includes an `attested` flag:

```rust
VerificationResult::Verified {
    pass_rate: 0.95,
    block_number: 100,
    attested: true,  // Third-party verified
}
```

---

## Verification Lookup

Any participant can verify a proof's existence and acceptance:

```rust
pub fn verify_proof(&self, job_hash: &[u8; 32], passport_id: AgentId) -> VerificationResult {
    // Returns Verified { pass_rate, block_number, attested }
    //      or Rejected { pass_rate, threshold }
    //      or NotFound
}
```

This is a read-only operation. The registry does not modify state on
verification queries.

---

## Configuration

```rust
pub struct ValidationRegistryConfig {
    pub min_gate_pass_rate: f64,        // 0.5 default
    pub recent_proof_window_blocks: u64, // Window for "recent" queries
    pub reject_duplicates: bool,         // true default
}
```

### Recent Proofs

The `recent_accepted` query filters by a configurable window:

```rust
pub fn recent_accepted(&self, current_block: u64) -> Vec<&ValidationRecord> {
    let cutoff = current_block.saturating_sub(self.config.recent_proof_window_blocks);
    self.records.values()
        .flat_map(|records| records.iter()
            .filter(|r| r.accepted && r.proof.block_number >= cutoff))
        .collect()
}
```

This supports reputation systems that weight recent performance more heavily
than historical records.
