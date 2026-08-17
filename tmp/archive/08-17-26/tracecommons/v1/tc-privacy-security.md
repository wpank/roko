# TraceCommons: Privacy and Security Enhancement Specification

## What is TraceCommons?

TraceCommons is a hosted server-side control plane for collecting, scoring,
deduplicating, and distributing coding-agent traces. Contributors (developers
running AI coding assistants such as Claude Code or Codex) submit redacted
recordings of their agent sessions. The server scores each trace for
quality (perplexity, novelty) inside a TEE (Trusted Execution Environment)
backed by dstack, stores encrypted artifacts in object storage
(GCS) with envelope encryption (AES-256-GCM DEKs wrapped by a pluggable
KMS), and maintains an append-only audit chain per tenant.

The system is PostgreSQL-only, enforces Row-Level Security (RLS) on every
table via `trace_current_tenant_id()`, authenticates contributors through
Ed25519 device keypairs and upload claims (EdDSA-signed JWTs issued by a
dedicated claim-issuer binary), and has a multi-stage privacy pipeline:

1. **Client-side deterministic redaction** (`DeterministicTraceRedactor`) --
   regex + heuristic secret/PII scrubbing before data leaves the machine.
2. **Privacy-filter sidecar** -- an optional subprocess for deeper PII
   detection (configurable; operator supplies the binary).
3. **Server-side re-scrub** -- a second deterministic pass on ingest.
4. **NEAR AI PII backstop** (optional) -- LLM-based PII detection as a
   gate before traces reach `Accepted` status.

Existing security infrastructure includes:

- `KmsKeyWrapper` trait hierarchy (local, cloud KMS, dstack stub) for
  DEK wrapping with context-hash binding (`KekContext`).
- `AttestationSigningState` and `ScoreAttestationClaims` for Ed25519-signed
  score attestations (JWTs with `kid`-based key rotation).
- `AuditChain` with per-tenant hash-chain integrity verification.
- `ReplayCache` and `InstanceRateLimiter` for enrollment nonce replay
  prevention and per-instance rate limiting.
- SimHash-based deduplication (`trace_simhash`, Hamming distance).
- Trait-based scoring seams (`PerplexityScorer`, `Embedder`, `VectorIndex`)
  designed for enclave substitution.

This document specifies ten privacy/security enhancements to build **within
TraceCommons** -- extending its existing crates, traits, and database schema.
Each section is self-contained: threat model motivation, Rust code sketches,
PostgreSQL migrations, crate dependencies, and integration points.

---

## Table of Contents

1. [Differential Privacy](#1-differential-privacy-p0)
2. [Zero-Knowledge Proofs](#2-zero-knowledge-proofs-p0)
3. [C2PA v2.3 Integration](#3-c2pa-v23-integration-p0)
4. [EU AI Act Compliance](#4-eu-ai-act-compliance-p0)
5. [Homomorphic Encryption Considerations](#5-homomorphic-encryption-considerations-p1)
6. [SCITT (RFC 9943)](#6-scitt-rfc-9943-p1)
7. [W3C DIDs + Verifiable Credentials](#7-w3c-dids--verifiable-credentials-p1)
8. [Private Similarity Search](#8-private-similarity-search-p1)
9. [CaMeL Capabilities Model](#9-camel-capabilities-model-p2)
10. [Secure Multi-Party Computation](#10-secure-multi-party-computation-p2)

---

## 1. Differential Privacy (P0)

### Motivation

TraceCommons exposes aggregate statistics to operators and consumers:
trace counts per contributor, quality score distributions, deduplication
rates, acceptance rates by channel. Without formal privacy guarantees, an
adversary who controls the query interface can reconstruct whether a
specific contributor submitted a trace by comparing aggregates before and
after that submission -- the classic membership inference attack.

The existing RLS enforcement isolates tenants at the row level but does
not protect against statistical inference within a tenant. Differential
privacy (DP) provides a mathematically rigorous bound on the information
any single contributor's participation leaks.

### Design

#### Framework: OpenDP

OpenDP provides calibrated noise mechanisms with composable privacy
accounting. The Rust bindings (`opendp`) expose the Laplace and Gaussian
mechanisms, privacy amplification by subsampling, and Renyi Differential
Privacy (RDP) accountants that track cumulative privacy loss across
multiple queries.

#### Mechanisms

| Statistic type | Mechanism | Sensitivity | Notes |
|---|---|---|---|
| Trace count (per contributor, per channel) | Laplace | 1 (each contributor contributes at most 1 to any count) | Integer output, clamp to non-negative |
| Mean quality score | Gaussian | `(upper - lower) / n` | Bounded range `[0, 10_000_000]` micros |
| Acceptance rate | Laplace | `1/n` | Ratio query |
| Novelty distribution histogram | Laplace | 2 (add-or-remove adjacency) | Per-bin noise |
| Dedup collision count | Laplace | 1 | SimHash hamming threshold crossings |

#### Privacy budget

Each contributor gets a configurable per-epoch privacy budget (epsilon).
The epoch boundary is configurable (daily, weekly) and resets the budget.
An RDP accountant tracks privacy loss across all queries touching a
contributor's data within an epoch. When the remaining budget for a
contributor falls below a configurable floor, queries that would touch
that contributor's data return a `PrivacyBudgetExhausted` error rather
than silently degrading noise calibration.

### Rust code

#### New module: `trace-commons-server/src/differential_privacy.rs`

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Privacy budget configuration, loaded from `roko.toml` or env.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifferentialPrivacyConfig {
    /// Per-contributor per-epoch epsilon. Default 1.0 (moderate privacy).
    pub epsilon_per_epoch: f64,
    /// Delta parameter for (epsilon, delta)-DP. Default 1e-6.
    pub delta: f64,
    /// Epoch duration in seconds. Default 86400 (daily).
    pub epoch_duration_seconds: i64,
    /// Minimum remaining epsilon before queries are refused. Default 0.1.
    pub epsilon_floor: f64,
    /// RDP alpha orders for the accountant. Default [2, 5, 10, 25, 50, 100].
    pub rdp_alpha_orders: Vec<f64>,
}

impl Default for DifferentialPrivacyConfig {
    fn default() -> Self {
        Self {
            epsilon_per_epoch: 1.0,
            delta: 1e-6,
            epoch_duration_seconds: 86400,
            epsilon_floor: 0.1,
            rdp_alpha_orders: vec![2.0, 5.0, 10.0, 25.0, 50.0, 100.0],
        }
    }
}

/// Tracks per-contributor privacy consumption within an epoch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyBudgetEntry {
    pub contributor_ref: String,
    pub tenant_id: String,
    pub epoch_start: DateTime<Utc>,
    pub epsilon_spent: f64,
    /// Per-alpha RDP epsilon spent, for tight composition.
    pub rdp_epsilon_spent: Vec<f64>,
    pub query_count: u32,
    pub last_query_at: DateTime<Utc>,
}

/// Mechanism selection for a specific query type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DpMechanism {
    Laplace,
    Gaussian,
}

/// A query that has been annotated with its DP parameters before execution.
#[derive(Debug, Clone)]
pub struct CalibratedQuery {
    pub query_id: Uuid,
    pub mechanism: DpMechanism,
    pub sensitivity: f64,
    pub epsilon: f64,
    pub delta: Option<f64>,
    /// Contributors whose budget will be consumed by this query.
    pub affected_contributors: Vec<String>,
}

/// Error returned when a query would exceed a contributor's privacy budget.
#[derive(Debug, thiserror::Error)]
pub enum PrivacyError {
    #[error("PrivacyBudgetExhausted: contributor {contributor_ref} has {remaining:.4} epsilon remaining, query requires {required:.4}")]
    BudgetExhausted {
        contributor_ref: String,
        remaining: f64,
        required: f64,
    },
    #[error("PrivacyConfigMissing: differential privacy not configured for tenant {tenant_id}")]
    ConfigMissing { tenant_id: String },
}

/// The DP layer wraps query results with calibrated noise. Constructed once
/// at server startup; holds a reference to the DB pool for budget tracking.
pub struct DifferentialPrivacyLayer {
    config: DifferentialPrivacyConfig,
    // pool: deadpool_postgres::Pool,  -- injected at construction
}

impl DifferentialPrivacyLayer {
    /// Check that all affected contributors have sufficient budget, then
    /// atomically debit their budgets and return a noise-addition closure.
    pub async fn prepare_query(
        &self,
        query: &CalibratedQuery,
    ) -> Result<NoiseAdder, PrivacyError> {
        // 1. For each contributor in affected_contributors:
        //    - Load or create PrivacyBudgetEntry for current epoch
        //    - Check epsilon_spent + query.epsilon <= epsilon_per_epoch - epsilon_floor
        //    - If any contributor would exceed budget, return BudgetExhausted
        // 2. Atomically increment epsilon_spent for all contributors
        // 3. Return NoiseAdder configured with mechanism + calibrated scale
        todo!()
    }

    /// Add Laplace noise calibrated to sensitivity/epsilon.
    fn laplace_noise(sensitivity: f64, epsilon: f64) -> f64 {
        let scale = sensitivity / epsilon;
        // Sample from Laplace(0, scale) using inverse CDF
        let u: f64 = rand::random::<f64>() - 0.5;
        -scale * u.signum() * (1.0 - 2.0 * u.abs()).ln()
    }

    /// Add Gaussian noise calibrated to sensitivity, epsilon, delta.
    fn gaussian_noise(sensitivity: f64, epsilon: f64, delta: f64) -> f64 {
        let sigma = sensitivity * (2.0 * (1.25 / delta).ln()).sqrt() / epsilon;
        // Sample from N(0, sigma^2) using Box-Muller
        let u1: f64 = rand::random();
        let u2: f64 = rand::random();
        sigma * (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

/// Returned by `prepare_query`; applies pre-calibrated noise to a scalar
/// result. The caller applies this AFTER computing the true aggregate and
/// BEFORE returning it to the API consumer.
pub struct NoiseAdder {
    pub mechanism: DpMechanism,
    pub scale: f64,
}

impl NoiseAdder {
    /// Add calibrated noise to a true aggregate value.
    pub fn apply(&self, true_value: f64) -> f64 {
        match self.mechanism {
            DpMechanism::Laplace => {
                let u: f64 = rand::random::<f64>() - 0.5;
                true_value - self.scale * u.signum() * (1.0 - 2.0 * u.abs()).ln()
            }
            DpMechanism::Gaussian => {
                let u1: f64 = rand::random();
                let u2: f64 = rand::random();
                true_value + self.scale * (-2.0 * u1.ln()).sqrt()
                    * (2.0 * std::f64::consts::PI * u2).cos()
            }
        }
    }

    /// Apply to an integer count, clamping to non-negative.
    pub fn apply_count(&self, true_count: i64) -> i64 {
        self.apply(true_count as f64).round().max(0.0) as i64
    }
}
```

### PostgreSQL migration: `V_NN__privacy_budgets.sql`

```sql
-- Per-contributor per-epoch privacy budget tracking.
-- RLS: tenant-scoped via trace_current_tenant_id().

CREATE TABLE IF NOT EXISTS trace_privacy_budgets (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       TEXT NOT NULL,
    contributor_ref TEXT NOT NULL,
    epoch_start     TIMESTAMPTZ NOT NULL,
    epoch_end       TIMESTAMPTZ NOT NULL,
    epsilon_spent   DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    -- JSONB array of per-alpha RDP epsilon values for tight composition
    rdp_epsilon_spent JSONB NOT NULL DEFAULT '[]'::jsonb,
    query_count     INTEGER NOT NULL DEFAULT 0,
    last_query_at   TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, contributor_ref, epoch_start)
);

ALTER TABLE trace_privacy_budgets ENABLE ROW LEVEL SECURITY;
CREATE POLICY trace_privacy_budgets_tenant_isolation
    ON trace_privacy_budgets
    USING (tenant_id = trace_current_tenant_id());

CREATE INDEX idx_privacy_budgets_lookup
    ON trace_privacy_budgets (tenant_id, contributor_ref, epoch_start);

-- Audit log for DP queries (who queried what, how much budget was consumed).
-- Hash-only: no raw query content or contributor identity beyond the ref.
CREATE TABLE IF NOT EXISTS trace_privacy_query_log (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       TEXT NOT NULL,
    query_hash      TEXT NOT NULL,
    mechanism       TEXT NOT NULL CHECK (mechanism IN ('laplace', 'gaussian')),
    sensitivity     DOUBLE PRECISION NOT NULL,
    epsilon_consumed DOUBLE PRECISION NOT NULL,
    delta_consumed  DOUBLE PRECISION,
    affected_contributor_count INTEGER NOT NULL,
    queried_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE trace_privacy_query_log ENABLE ROW LEVEL SECURITY;
CREATE POLICY trace_privacy_query_log_tenant_isolation
    ON trace_privacy_query_log
    USING (tenant_id = trace_current_tenant_id());
```

### Integration points

- **Extends**: `trace-commons-server/src/lib.rs` -- add `pub mod differential_privacy;`
- **Wraps**: aggregate query handlers in `trace-commons-ingest.rs` (trace count,
  quality distribution, acceptance rate endpoints)
- **Config**: new `[privacy.differential]` section in server config, read via
  `DifferentialPrivacyConfig::from_env()`

### Dependencies

```toml
# In trace-commons-server/Cargo.toml
rand = "0.8"           # Already likely present; for noise sampling
# opendp = "0.11"      # Optional: use opendp's calibrated mechanisms
#                        instead of hand-rolled sampling above
```

### Complexity: Medium

Budget tracking is straightforward PostgreSQL. The subtlety is in RDP
composition accounting -- for Phase 1, basic epsilon composition suffices.
RDP accounting (tighter bounds) can be added incrementally.

---

## 2. Zero-Knowledge Proofs (P0)

### Motivation

TraceCommons already issues Ed25519-signed score attestations
(`ScoreAttestationClaims` in `trace_score_attestation.rs`). These
attestations prove that the server scored a set of submissions, but they
reveal the exact scores. A contributor who wants to prove "I submitted N
traces that all passed the quality gate" currently has to reveal every
individual score.

ZK proofs let a contributor prove properties of their scores without
revealing the scores themselves. This is useful for:

1. **Reputation without leakage**: prove "my average quality score exceeds
   threshold T" without revealing which traces or what scores.
2. **Compliance attestation**: prove "I submitted N traces to corpus X"
   without revealing trace content or scores.
3. **Cross-instance verification**: prove to instance B that instance A
   scored your traces above a threshold, without trusting B with the
   scores themselves.

### Design

#### Libraries

| Library | Purpose | When to use |
|---|---|---|
| `arkworks` (`ark-groth16`, `ark-bn254`) | R1CS circuits for general statements | Score threshold proofs, count proofs |
| `bulletproofs` (`curve25519-dalek`) | Efficient range proofs without trusted setup | "Score in [X, Y]" range proofs |
| `risc0-zkvm` | General-purpose ZK computation | Proving TEE scoring was done correctly (long-term) |

Phase 1 uses Bulletproofs for range proofs (no trusted setup, efficient
for single-statement proofs). arkworks Groth16 proofs are Phase 2 for
compound statements ("average of N scores exceeds T"). RISC Zero is
Phase 3 for full scoring verification.

#### Proof types

| Statement | Circuit | Verifier |
|---|---|---|
| `score >= threshold` | Bulletproof range proof on `score - threshold` | Constant-time verify |
| `count(passed) >= N` | Committed sum of pass/fail bits | Pedersen commitment opening |
| `mean(scores) >= T` | Groth16 over committed score vector | Pairing check |
| `TEE scored correctly` | RISC Zero guest program | STARK verify |

### Rust code

#### New module: `trace-commons-server/src/zk_attestation.rs`

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A ZK attestation that proves a property of a contributor's scores
/// without revealing the scores themselves.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkAttestation {
    pub attestation_id: Uuid,
    pub schema_version: String,
    pub tenant_id: String,
    /// Hash of the contributor's auth_principal_ref (not the ref itself).
    pub contributor_ref_hash: String,
    pub statement: ZkStatement,
    /// The serialized proof bytes, base64-encoded.
    pub proof_bytes_b64: String,
    /// The proof system used.
    pub proof_system: ProofSystem,
    /// Public inputs to the verifier (e.g., the threshold, the count).
    pub public_inputs: serde_json::Value,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProofSystem {
    Bulletproofs,
    Groth16Bn254,
    Risc0,
}

/// The class of statement being proven.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ZkStatement {
    /// "My score for submission X is >= threshold T."
    ScoreAboveThreshold {
        submission_id: Uuid,
        threshold_micros: u64,
    },
    /// "I have at least N submissions that passed the gate."
    PassedCountAbove {
        minimum_count: u32,
    },
    /// "My average perplexity score across submissions [ids] is >= T."
    MeanScoreAbove {
        submission_count: u32,
        threshold_micros: u64,
    },
    /// "The TEE scoring of submission X was performed correctly."
    ScoringIntegrity {
        submission_id: Uuid,
        gate_version_hash: String,
    },
}

/// Missing-control label: the ZK proof service is not configured. The
/// endpoint returns 503 rather than silently skipping proof generation.
pub const ZK_ATTESTATION_SERVICE_UNCONFIGURED: &str = "zk_attestation_service_unconfigured";

/// Service that generates and verifies ZK attestations. Constructed once
/// at startup from env-sourced config.
pub struct ZkAttestationService {
    /// Bulletproofs generators -- reused across proofs for performance.
    // bp_generators: BulletproofGens,  // from bulletproofs crate
    /// Server's attestation signing state -- ZK attestations are ALSO
    /// signed by the server's Ed25519 key so verifiers can authenticate
    /// the attestation wrapper without re-running the ZK verification.
    // attestation_state: AttestationSigningState,
}

/// Trait for pluggable proof backends. The service dispatches to the
/// appropriate backend based on the requested ProofSystem.
pub trait ZkProofBackend: Send + Sync {
    /// Generate a proof for the given statement with the given witness
    /// (private inputs). Returns serialized proof bytes.
    fn prove(
        &self,
        statement: &ZkStatement,
        witness: &ZkWitness,
    ) -> anyhow::Result<Vec<u8>>;

    /// Verify a proof against public inputs.
    fn verify(
        &self,
        statement: &ZkStatement,
        proof_bytes: &[u8],
        public_inputs: &serde_json::Value,
    ) -> anyhow::Result<bool>;
}

/// Private witness data -- never serialized, never leaves the server.
/// The scores are loaded from the DB under the contributor's authenticated
/// context and used only as circuit inputs.
pub struct ZkWitness {
    /// The actual scores (private inputs to the circuit).
    pub scores_micros: Vec<u64>,
    /// Blinding factors for Pedersen commitments.
    pub blinding_factors: Vec<[u8; 32]>,
}
```

#### Bulletproof range proof sketch

```rust
/// Prove that `score_micros >= threshold_micros` without revealing
/// `score_micros`. Uses a Bulletproofs range proof on
/// `value = score_micros - threshold_micros` to prove `value >= 0`
/// within a 64-bit range.
pub fn prove_score_above_threshold(
    score_micros: u64,
    threshold_micros: u64,
    // bp_gens: &BulletproofGens,
    // pc_gens: &PedersenGens,
) -> anyhow::Result<Vec<u8>> {
    anyhow::ensure!(
        score_micros >= threshold_micros,
        "ZkProveRefused: score below threshold, proof would be unsatisfiable"
    );
    let value = score_micros - threshold_micros;
    // In production:
    // let mut transcript = Transcript::new(b"trace_commons_score_range_proof_v1");
    // let (proof, committed_value) = RangeProof::prove_single(
    //     bp_gens, pc_gens, &mut transcript, value, &blinding, 64,
    // )?;
    // Ok(proof.to_bytes())
    let _ = value;
    todo!("wire bulletproofs crate")
}

/// Verify a range proof that `committed_value >= threshold_micros`.
pub fn verify_score_above_threshold(
    proof_bytes: &[u8],
    threshold_micros: u64,
    // committed_value: &CompressedRistretto,
    // bp_gens: &BulletproofGens,
    // pc_gens: &PedersenGens,
) -> anyhow::Result<bool> {
    let _ = (proof_bytes, threshold_micros);
    todo!("wire bulletproofs crate")
}
```

### PostgreSQL migration: `V_NN__zk_attestations.sql`

```sql
CREATE TABLE IF NOT EXISTS trace_zk_attestations (
    attestation_id    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id         TEXT NOT NULL,
    contributor_ref_hash TEXT NOT NULL,
    statement_type    TEXT NOT NULL,
    proof_system      TEXT NOT NULL CHECK (proof_system IN (
        'bulletproofs', 'groth16_bn254', 'risc0'
    )),
    proof_bytes_hash  TEXT NOT NULL,  -- sha256 hash of proof (not the proof itself)
    public_inputs     JSONB NOT NULL,
    issued_at         TIMESTAMPTZ NOT NULL,
    expires_at        TIMESTAMPTZ NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE trace_zk_attestations ENABLE ROW LEVEL SECURITY;
CREATE POLICY trace_zk_attestations_tenant_isolation
    ON trace_zk_attestations
    USING (tenant_id = trace_current_tenant_id());

CREATE INDEX idx_zk_attestations_contributor
    ON trace_zk_attestations (tenant_id, contributor_ref_hash, statement_type);
```

### Integration points

- **Extends**: `trace_score_attestation.rs` -- the ZK attestation endpoint sits
  alongside the existing score attestation endpoint
- **New route**: `POST /v1/admin/zk-attestation` in `trace-commons-ingest.rs`
- **New route**: `GET /v1/admin/zk-attestation/{id}/verify` for third-party verification
- **Auth**: same `auth_principal_ref`-from-upload-claim pattern as `score_attestation_handler`

### Dependencies

```toml
# Phase 1: Bulletproofs (no trusted setup)
bulletproofs = "4.0"
curve25519-dalek = { version = "4", features = ["serde"] }
merlin = "3"  # Fiat-Shamir transcript

# Phase 2: Groth16 (trusted setup, more expressive)
# ark-groth16 = "0.5"
# ark-bn254 = "0.5"
# ark-relations = "0.5"
# ark-serialize = "0.5"
# ark-std = "0.5"

# Phase 3: RISC Zero (general purpose ZK)
# risc0-zkvm = "1.2"
```

### Complexity: High

Bulletproofs range proofs are well-understood and the crate is mature.
Groth16 requires a trusted setup ceremony (or a universal SRS like
Plonk/KZG). RISC Zero requires compiling the scoring logic as a RISC-V
guest program, which is a significant engineering effort.

---

## 3. C2PA v2.3 Integration (P0)

### Motivation

The Coalition for Content Provenance and Authenticity (C2PA) defines a
standard for embedding provenance manifests in digital content. By
attaching a C2PA manifest to each trace bundle, TraceCommons provides
cryptographic proof of:

- **Origin**: which contributor tool (Claude Code, Codex) produced the trace
- **Chain of custody**: every transformation (redaction, re-scrub, scoring)
  the trace underwent
- **Tamper detection**: any modification to the trace after manifest creation
  invalidates the manifest signature

This aligns with TraceCommons' existing audit chain (`AuditChain`) but
adds an industry-standard, interoperable provenance format that external
consumers and regulators can verify using standard C2PA tooling.

### Design

Each trace submission generates a C2PA manifest with:

1. **Claim**: contributor identity (pseudonymous ref), tool version, timestamp
2. **Assertions**: redaction pipeline version, scoring results (hash-only),
   consent scopes
3. **Signature**: signed by the server's attestation key (Ed25519, same key
   material as `AttestationSigningState`)

The manifest is stored alongside the trace artifact in GCS, retrievable
via a new `/api/v1/traces/{submission_id}/provenance` endpoint.

### Rust code

#### New module: `trace-commons-server/src/c2pa_manifest.rs`

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// C2PA manifest metadata stored alongside trace artifacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceC2paManifest {
    pub manifest_id: Uuid,
    pub submission_id: Uuid,
    pub tenant_id: String,
    /// C2PA manifest version. Tracks the c2pa-rs crate version.
    pub c2pa_version: String,
    /// SHA-256 of the serialized C2PA manifest store bytes.
    pub manifest_store_hash: String,
    /// The artifact kind this manifest covers.
    pub artifact_kind: String,
    pub created_at: DateTime<Utc>,
}

/// Configuration for C2PA manifest generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2paConfig {
    /// Whether to generate manifests for new submissions. Default false
    /// (opt-in during rollout).
    pub enabled: bool,
    /// Label for the C2PA claim generator field.
    pub generator_label: String,
    /// Whether to include scoring results in manifest assertions.
    pub include_scoring_assertions: bool,
}

impl Default for C2paConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            generator_label: "TraceCommons/1.0".to_string(),
            include_scoring_assertions: true,
        }
    }
}

/// Assertion types included in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceC2paAssertion {
    /// Records the redaction pipeline version and redaction counts.
    RedactionProvenance {
        pipeline_version: String,
        redaction_count: u32,
        residual_pii_risk: String,
    },
    /// Records gate scoring results (hash-only, no raw scores in the
    /// manifest -- scores are referenced by the gate_decision hash).
    GateScoringRef {
        gate_version_hash: String,
        gate_passed: bool,
        decision_hash: String,
    },
    /// Records consent scope at submission time.
    ConsentRecord {
        policy_version: String,
        scopes: Vec<String>,
    },
    /// Records contributor tool metadata.
    ToolProvenance {
        agent: String,
        version: String,
        channel: String,
    },
}

/// Service that generates C2PA manifests for trace artifacts.
///
/// Uses the `c2pa` crate to create and sign manifest stores. The signing
/// key is the same Ed25519 key used for score attestations.
pub struct C2paManifestService {
    config: C2paConfig,
    // builder: c2pa::Builder,  -- from c2pa crate
    // signer: Box<dyn c2pa::Signer>,
}

impl C2paManifestService {
    /// Generate a C2PA manifest for a submitted trace envelope.
    ///
    /// Called after the trace passes the gate and is accepted. The manifest
    /// is stored as a sibling artifact in GCS. The manifest store hash is
    /// recorded in the DB for retrieval.
    pub async fn generate_manifest(
        &self,
        submission_id: Uuid,
        tenant_id: &str,
        assertions: Vec<TraceC2paAssertion>,
        artifact_bytes: &[u8],
    ) -> anyhow::Result<TraceC2paManifest> {
        // 1. Create a c2pa::Builder with the configured generator label
        // 2. Add each assertion as a C2PA assertion
        // 3. Set the claim's created_at to now
        // 4. Sign the manifest with the server's attestation key
        // 5. Serialize the manifest store
        // 6. Compute sha256 of the manifest store bytes
        // 7. Return TraceC2paManifest with the hash
        todo!()
    }

    /// Verify a C2PA manifest against its artifact. Returns the parsed
    /// assertions if valid, or an error describing the verification failure.
    pub fn verify_manifest(
        &self,
        manifest_store_bytes: &[u8],
        artifact_bytes: &[u8],
    ) -> anyhow::Result<Vec<TraceC2paAssertion>> {
        todo!()
    }
}
```

### PostgreSQL migration: `V_NN__c2pa_manifests.sql`

```sql
CREATE TABLE IF NOT EXISTS trace_c2pa_manifests (
    manifest_id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    submission_id       UUID NOT NULL,
    tenant_id           TEXT NOT NULL,
    c2pa_version        TEXT NOT NULL,
    manifest_store_hash TEXT NOT NULL,
    artifact_kind       TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE trace_c2pa_manifests ENABLE ROW LEVEL SECURITY;
CREATE POLICY trace_c2pa_manifests_tenant_isolation
    ON trace_c2pa_manifests
    USING (tenant_id = trace_current_tenant_id());

CREATE INDEX idx_c2pa_manifests_submission
    ON trace_c2pa_manifests (tenant_id, submission_id);

-- The actual manifest store bytes are stored in GCS alongside the trace
-- artifact, NOT in PostgreSQL. Only the hash is stored here for lookup.
```

### Integration points

- **New route**: `GET /v1/traces/{submission_id}/provenance` in ingest binary
- **Hook into**: the post-gate acceptance path in the vector worker -- after
  `GateDecision` is persisted, generate the C2PA manifest
- **Artifact store**: store manifest bytes via `ServiceOwnedTraceArtifactStore`
  with a new `TraceArtifactKind::C2paManifest` variant

### Dependencies

```toml
c2pa = "0.40"  # C2PA manifest creation and validation
```

### Complexity: Medium

The `c2pa-rs` crate handles manifest serialization, signing, and
verification. The main integration work is wiring the manifest generation
into the post-gate acceptance path and adding the new artifact kind.

---

## 4. EU AI Act Compliance (P0)

### Motivation

The EU AI Act's Article 12 (mandatory logging for high-risk AI systems)
took effect August 2, 2026. Article 50 requires AI content marking.
TraceCommons is positioned as compliance infrastructure: organizations
using AI coding assistants can route their session traces through TC to
satisfy logging requirements with cryptographic guarantees.

### Design

#### Article 12 required fields

The AI Act requires that high-risk AI systems maintain logs containing:

| Required field | TC mapping | Notes |
|---|---|---|
| Timestamp | `TraceContributionEnvelope.created_at` | Already captured |
| Model version | `IronclawTraceMetadata.version` + `model_name` | Already captured |
| Input hash | SHA-256 of redacted user messages | New: compute during envelope creation |
| Output hash | SHA-256 of redacted assistant messages | New: compute during envelope creation |
| Confidence scores | `PerplexityResult` + `GateDecision` | Already scored |
| Duration | `TraceContributionEvent.latency_ms` sum | Already captured per-event |
| Tool invocations | `TraceContributionEvent` with `ToolCall` type | Already captured |
| Failure indicators | `OutcomeMetadata.failure_modes` | Already captured |

#### Compliance record structure

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// EU AI Act Article 12 compliance record. Generated from a
/// TraceContributionEnvelope and its GateDecision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiActComplianceRecord {
    pub record_id: Uuid,
    pub submission_id: Uuid,
    pub tenant_id: String,
    /// Schema version for forward compatibility.
    pub schema_version: String,

    // --- Article 12: Logging ---
    pub timestamp: DateTime<Utc>,
    pub model_identifier: String,
    pub model_version: String,
    pub input_hash: String,          // SHA-256 of redacted input
    pub output_hash: String,         // SHA-256 of redacted output
    pub session_duration_ms: u64,
    pub tool_invocation_count: u32,
    pub tool_names_hash: String,     // SHA-256 of sorted tool name list
    pub confidence_score_micros: u64, // from gate scoring
    pub gate_passed: bool,
    pub failure_mode_count: u32,
    /// Hash-only reference to failure modes (not raw strings).
    pub failure_modes_hash: String,

    // --- Article 50: Content marking ---
    /// Whether the AI-generated content in this trace is marked per Art. 50.
    pub ai_content_marked: bool,
    /// Marking method used (e.g., "c2pa_manifest", "metadata_tag").
    pub marking_method: Option<String>,

    // --- Jurisdiction ---
    /// Which regulatory frameworks this record satisfies.
    pub frameworks: Vec<ComplianceFramework>,

    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceFramework {
    EuAiAct,
    SingaporeImda,
    NistAiRmf,
}

/// Configuration for compliance record generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceConfig {
    /// Whether to generate compliance records. Default false.
    pub enabled: bool,
    /// Frameworks to generate records for.
    pub frameworks: Vec<ComplianceFramework>,
    /// Retention period in days. EU AI Act requires minimum 6 months for
    /// high-risk systems, but operators may want longer.
    pub retention_days: u32,
    /// Whether to export records in EU-mandated format on request.
    pub export_enabled: bool,
}

impl Default for ComplianceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            frameworks: vec![ComplianceFramework::EuAiAct],
            retention_days: 365,
            export_enabled: true,
        }
    }
}

/// Service for generating and exporting compliance records.
pub struct ComplianceService {
    config: ComplianceConfig,
}

impl ComplianceService {
    /// Generate a compliance record from a trace envelope and its gate decision.
    pub fn generate_record(
        &self,
        envelope: &TraceContributionEnvelope,
        gate_decision: &GateDecision,
        tenant_id: &str,
    ) -> AiActComplianceRecord {
        let input_events: Vec<_> = envelope.events.iter()
            .filter(|e| matches!(e.event_type, TraceContributionEventType::UserMessage))
            .collect();
        let output_events: Vec<_> = envelope.events.iter()
            .filter(|e| matches!(e.event_type, TraceContributionEventType::AssistantMessage))
            .collect();
        let tool_events: Vec<_> = envelope.events.iter()
            .filter(|e| matches!(e.event_type, TraceContributionEventType::ToolCall))
            .collect();

        let input_hash = sha256_of_redacted_content(&input_events);
        let output_hash = sha256_of_redacted_content(&output_events);
        let total_latency: u64 = envelope.events.iter()
            .filter_map(|e| e.latency_ms)
            .sum();

        let mut tool_names: Vec<String> = tool_events.iter()
            .filter_map(|e| e.tool_name.clone())
            .collect();
        tool_names.sort();
        tool_names.dedup();
        let tool_names_hash = sha256_of_string(&tool_names.join(","));

        let failure_modes_hash = sha256_of_string(
            &format!("{:?}", envelope.outcome.failure_modes)
        );

        AiActComplianceRecord {
            record_id: Uuid::new_v4(),
            submission_id: envelope.submission_id,
            tenant_id: tenant_id.to_string(),
            schema_version: "trace_commons.ai_act_compliance.v1".to_string(),
            timestamp: envelope.created_at,
            model_identifier: envelope.ironclaw.version.clone(),
            model_version: envelope.ironclaw.engine_version
                .clone().unwrap_or_default(),
            input_hash,
            output_hash,
            session_duration_ms: total_latency,
            tool_invocation_count: tool_events.len() as u32,
            tool_names_hash,
            confidence_score_micros: gate_decision.perplexity_micros,
            gate_passed: gate_decision.perplexity_passed
                && gate_decision.novelty_passed,
            failure_mode_count: envelope.outcome.failure_modes.len() as u32,
            failure_modes_hash,
            ai_content_marked: false,
            marking_method: None,
            frameworks: self.config.frameworks.clone(),
            created_at: Utc::now(),
        }
    }

    /// Export compliance records in the EU-mandated audit trail format.
    /// Returns JSONL-formatted records for a date range.
    pub async fn export_audit_trail(
        &self,
        tenant_id: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        framework: ComplianceFramework,
    ) -> anyhow::Result<Vec<u8>> {
        todo!()
    }
}

fn sha256_of_redacted_content(events: &[&TraceContributionEvent]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    for event in events {
        if let Some(content) = &event.redacted_content {
            hasher.update(content.as_bytes());
        }
    }
    format!("sha256:{:x}", hasher.finalize())
}

fn sha256_of_string(s: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}
```

### PostgreSQL migration: `V_NN__compliance_records.sql`

```sql
CREATE TABLE IF NOT EXISTS trace_compliance_records (
    record_id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    submission_id       UUID NOT NULL,
    tenant_id           TEXT NOT NULL,
    schema_version      TEXT NOT NULL,
    framework           TEXT NOT NULL,
    timestamp           TIMESTAMPTZ NOT NULL,
    model_identifier    TEXT NOT NULL,
    model_version       TEXT NOT NULL,
    input_hash          TEXT NOT NULL,
    output_hash         TEXT NOT NULL,
    session_duration_ms BIGINT NOT NULL,
    tool_invocation_count INTEGER NOT NULL,
    tool_names_hash     TEXT NOT NULL,
    confidence_score_micros BIGINT NOT NULL,
    gate_passed         BOOLEAN NOT NULL,
    failure_mode_count  INTEGER NOT NULL,
    failure_modes_hash  TEXT NOT NULL,
    ai_content_marked   BOOLEAN NOT NULL DEFAULT false,
    marking_method      TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Retention: auto-expire based on operator config
    expires_at          TIMESTAMPTZ
);

ALTER TABLE trace_compliance_records ENABLE ROW LEVEL SECURITY;
CREATE POLICY trace_compliance_records_tenant_isolation
    ON trace_compliance_records
    USING (tenant_id = trace_current_tenant_id());

CREATE INDEX idx_compliance_records_submission
    ON trace_compliance_records (tenant_id, submission_id);
CREATE INDEX idx_compliance_records_framework_time
    ON trace_compliance_records (tenant_id, framework, timestamp);
CREATE INDEX idx_compliance_records_expiry
    ON trace_compliance_records (expires_at)
    WHERE expires_at IS NOT NULL;

-- NIST AI RMF mapping table: links TC concepts to NIST categories.
CREATE TABLE IF NOT EXISTS trace_compliance_nist_mapping (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    nist_function       TEXT NOT NULL,  -- GOVERN, MAP, MEASURE, MANAGE
    nist_category       TEXT NOT NULL,  -- e.g. "GV-1", "MP-2.3"
    tc_component        TEXT NOT NULL,  -- e.g. "gate_scoring", "audit_chain"
    tc_evidence_type    TEXT NOT NULL,  -- e.g. "gate_decision", "compliance_record"
    description         TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### Integration points

- **New route**: `GET /v1/admin/compliance/export` with `framework`, `from`, `to` params
- **New route**: `GET /v1/admin/compliance/records/{submission_id}`
- **Hook into**: post-gate acceptance path, alongside C2PA manifest generation
- **Config**: `[compliance]` section in server config

### Dependencies

No new crate dependencies. Uses existing `sha2`, `chrono`, `serde`, `uuid`.

### Complexity: Medium

The record generation is straightforward mapping. The export format needs
to track the evolving EU technical standards documentation, which is
currently in draft.

---

## 5. Homomorphic Encryption Considerations (P1)

### Motivation

TraceCommons already runs scoring inside a TEE (dstack-attested enclave).
Within a single TC instance, the TEE provides confidentiality -- the
operator cannot see trace content during scoring. But when two separate
TC instances want to perform cross-instance queries (e.g., "does instance
B already have a similar trace to this one?"), neither instance can trust
the other's TEE. Homomorphic encryption (HE) enables computation on
encrypted data without decryption, solving the cross-instance trust
problem.

### TEE vs HE tradeoff analysis

| Property | TEE (dstack) | HE (CKKS) |
|---|---|---|
| Performance | Near-native speed | 1000x-10000x slower for arithmetic |
| Trust model | Trust hardware vendor + attestation | Trust math only |
| Single-instance scoring | Best choice | Overkill |
| Cross-instance similarity | Requires trusting remote TEE | Works without trust |
| Embedding comparison | Already works in-enclave | Enables encrypted comparison |
| Noise budget | N/A | Must manage carefully |

**Conclusion**: HE adds value specifically for cross-instance operations
where you cannot trust the remote instance's TEE attestation chain. For
single-instance scoring, the existing TEE approach is superior.

### Design: CKKS for encrypted embeddings

The CKKS (Cheon-Kim-Kim-Song) scheme supports approximate arithmetic on
encrypted floating-point vectors -- exactly what cosine similarity over
embeddings requires.

```rust
use serde::{Deserialize, Serialize};

/// Configuration for homomorphic encryption operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomomorphicEncryptionConfig {
    /// CKKS polynomial modulus degree. Default 8192 (128-bit security).
    pub poly_modulus_degree: usize,
    /// CKKS scale (encoding precision). Default 2^40.
    pub scale_bits: u32,
    /// Maximum multiplicative depth before noise budget is exhausted.
    pub max_depth: u32,
    /// Whether to enable encrypted similarity search. Default false.
    pub enable_encrypted_similarity: bool,
}

impl Default for HomomorphicEncryptionConfig {
    fn default() -> Self {
        Self {
            poly_modulus_degree: 8192,
            scale_bits: 40,
            max_depth: 4,
            enable_encrypted_similarity: false,
        }
    }
}

/// An embedding vector encrypted under CKKS. The ciphertext is opaque to
/// anyone without the secret key, but supports homomorphic inner-product
/// computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedEmbedding {
    /// Serialized CKKS ciphertext, base64-encoded.
    pub ciphertext_b64: String,
    /// Public key fingerprint for the key this was encrypted under.
    pub public_key_fingerprint: String,
    /// Remaining noise budget (number of multiplications still possible).
    pub remaining_depth: u32,
}

/// Trait for homomorphic embedding operations. Implementations use
/// concrete-rs or tfhe-rs.
pub trait HomomorphicEmbedder: Send + Sync {
    /// Encrypt a plaintext embedding vector.
    fn encrypt_embedding(&self, embedding: &[f32]) -> anyhow::Result<EncryptedEmbedding>;

    /// Compute cosine similarity between an encrypted query and an
    /// encrypted corpus entry. Returns an encrypted similarity score
    /// that must be decrypted by the key holder.
    fn encrypted_cosine_similarity(
        &self,
        query: &EncryptedEmbedding,
        corpus_entry: &EncryptedEmbedding,
    ) -> anyhow::Result<EncryptedEmbedding>;

    /// Decrypt a similarity score. Only the key holder can call this.
    fn decrypt_scalar(&self, ciphertext: &EncryptedEmbedding) -> anyhow::Result<f64>;
}
```

### Practical limitations

1. **Performance**: CKKS cosine similarity on 256-dim vectors takes ~100ms
   per comparison (vs. <1us for plaintext). Searching 10K vectors would
   take ~17 minutes. Batch optimizations (SIMD packing) can reduce this
   to ~1 minute.

2. **Noise budget**: each CKKS multiplication consumes noise budget. Cosine
   similarity requires inner product (1 multiplication per dimension) +
   normalization (1 multiplication). At `max_depth=4`, this is feasible
   but leaves no room for further operations on the result.

3. **Key management**: each TC instance generates an HE keypair. To compute
   cross-instance similarity, one instance must encrypt under the other's
   public key, or both use a jointly generated key (requires MPC for key
   generation).

### Dependencies

```toml
# Choose one:
# concrete = "0.4"    # Zama's FHE library (CKKS + TFHE)
# tfhe = "0.9"        # Zama's TFHE-rs (Boolean + integer, faster for some ops)
```

### Complexity: Very High

HE libraries are maturing rapidly but remain complex to use correctly.
The noise budget management requires careful circuit design. Recommend
prototyping with `tfhe-rs` on a small embedding dimension (16-32) before
committing to the full 256-dim integration.

---

## 6. SCITT (RFC 9943) (P1)

### Motivation

TraceCommons' existing audit chain (`AuditChain` in `audit_chain.rs`)
provides per-tenant hash-chain integrity: each event's hash depends on
the previous event's hash, creating a tamper-evident log. But this chain
is tenant-private and not independently verifiable by third parties.

SCITT (Supply Chain Integrity, Transparency, and Trust, RFC 9943) defines
a standard for append-only transparency logs with Merkle tree receipts.
Each submission gets a SCITT receipt -- a Merkle inclusion proof that
anchors the submission in a global, publicly auditable log. This enables:

1. **Third-party auditability**: anyone with a receipt can verify a
   submission existed at a specific time.
2. **Non-repudiation**: the operator cannot retroactively remove a
   submission from the log without invalidating all subsequent receipts.
3. **Cross-instance consistency**: multiple TC instances can verify each
   other's receipts against a shared transparency log root.

### Design

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// A SCITT receipt proving inclusion of a trace submission in the
/// transparency log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScittReceipt {
    pub receipt_id: Uuid,
    pub submission_id: Uuid,
    pub tenant_id: String,
    /// The Merkle tree root hash at the time of inclusion.
    pub tree_root: String,
    /// The Merkle inclusion proof (sibling hashes from leaf to root).
    pub inclusion_proof: Vec<String>,
    /// The leaf index in the Merkle tree.
    pub leaf_index: u64,
    /// The tree size (number of leaves) at the time of inclusion.
    pub tree_size: u64,
    /// The hash of the submission statement (SCITT "statement" envelope).
    pub statement_hash: String,
    /// Timestamp of inclusion.
    pub included_at: DateTime<Utc>,
    /// Signature over (tree_root, tree_size) by the transparency log
    /// operator key.
    pub log_signature: String,
}

/// The SCITT transparency log backed by PostgreSQL.
///
/// Implements an append-only Merkle tree using the RFC 6962 (Certificate
/// Transparency) hash structure, adapted for SCITT's COSE_Sign1 envelope
/// format.
pub struct ScittLedger {
    // pool: deadpool_postgres::Pool,
}

impl ScittLedger {
    /// Append a new leaf to the transparency log. Returns a receipt
    /// with the Merkle inclusion proof.
    pub async fn append(
        &self,
        submission_id: Uuid,
        tenant_id: &str,
        statement_bytes: &[u8],
    ) -> anyhow::Result<ScittReceipt> {
        // 1. Hash the statement bytes: sha256(statement_bytes)
        // 2. Insert as a new leaf in the Merkle tree
        // 3. Recompute affected path from leaf to root
        // 4. Generate inclusion proof (collect sibling hashes)
        // 5. Sign the new root
        // 6. Return ScittReceipt
        todo!()
    }

    /// Verify a receipt's inclusion proof against the current tree root.
    pub async fn verify_receipt(
        &self,
        receipt: &ScittReceipt,
    ) -> anyhow::Result<bool> {
        // Walk the inclusion proof from leaf to root, verify
        // the computed root matches receipt.tree_root
        todo!()
    }

    /// Get the current tree root and size.
    pub async fn tree_head(&self) -> anyhow::Result<(String, u64)> {
        todo!()
    }
}

/// Pure Merkle tree proof verification. Given a leaf hash, its index,
/// the sibling hashes, and the expected root, verify the inclusion.
pub fn verify_merkle_inclusion(
    leaf_hash: &[u8; 32],
    leaf_index: u64,
    tree_size: u64,
    proof: &[[u8; 32]],
    expected_root: &[u8; 32],
) -> bool {
    let mut current = *leaf_hash;
    let mut index = leaf_index;
    for sibling in proof {
        let mut hasher = Sha256::new();
        if index % 2 == 0 {
            hasher.update([0x01]); // internal node prefix
            hasher.update(current);
            hasher.update(sibling);
        } else {
            hasher.update([0x01]);
            hasher.update(sibling);
            hasher.update(current);
        }
        current = hasher.finalize().into();
        index /= 2;
    }
    current == *expected_root
}
```

### PostgreSQL migration: `V_NN__scitt_ledger.sql`

```sql
-- Merkle tree leaves for the SCITT transparency log.
CREATE TABLE IF NOT EXISTS trace_scitt_leaves (
    leaf_index      BIGINT PRIMARY KEY,
    tenant_id       TEXT NOT NULL,
    submission_id   UUID NOT NULL,
    statement_hash  TEXT NOT NULL,
    leaf_hash       TEXT NOT NULL,
    inserted_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Merkle tree internal nodes. Stored for efficient proof generation.
-- (level, index) uniquely identifies a node; level 0 = leaves.
CREATE TABLE IF NOT EXISTS trace_scitt_nodes (
    level           INTEGER NOT NULL,
    node_index      BIGINT NOT NULL,
    hash            TEXT NOT NULL,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (level, node_index)
);

-- Signed tree heads (checkpoints).
CREATE TABLE IF NOT EXISTS trace_scitt_tree_heads (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tree_size       BIGINT NOT NULL,
    root_hash       TEXT NOT NULL,
    signature       TEXT NOT NULL,
    signed_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- No RLS on SCITT tables: transparency log is intentionally cross-tenant.
-- The leaf data contains only submission_id and statement_hash (hash-only),
-- not trace content. tenant_id is recorded for operational routing.
```

### Integration points

- **New routes**: `POST /v1/scitt/submit`, `GET /v1/scitt/receipt/{submission_id}`,
  `GET /v1/scitt/tree-head`
- **Hook into**: post-gate acceptance path, after C2PA manifest generation

### Dependencies

No new crate dependencies. Merkle tree is implemented with `sha2`.
COSE_Sign1 envelope format uses `coset = "0.3"` if full RFC 9943
compliance is needed.

### Complexity: Medium

The Merkle tree implementation is straightforward. The main challenge is
efficient incremental tree updates in PostgreSQL and correct handling of
sparse trees (when the tree size is not a power of 2).

---

## 7. W3C DIDs + Verifiable Credentials (P1)

### Motivation

TraceCommons currently identifies contributors via Ed25519 device keypairs
and pseudonymous `principal_ref` strings derived from the device key.
This is a closed identity system -- a contributor's identity is meaningful
only within the TC ecosystem. W3C Decentralized Identifiers (DIDs) and
Verifiable Credentials (VCs) extend this to an open, interoperable
identity layer:

1. **DIDs** let contributors control their identity without depending on
   TC as the identity provider. A `did:near` DID anchored on NEAR protocol
   is self-sovereign.
2. **Verifiable Credentials** let TC issue portable reputation attestations:
   "this contributor has submitted 100+ high-quality traces" becomes a
   credential the contributor can present to any verifier, not just TC.
3. **BBS+ signatures** enable selective disclosure: the contributor can
   prove they hold a VC without revealing all attributes (e.g., prove
   they have 100+ traces without revealing their exact count or scores).

### Design

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A DID document fragment stored by TraceCommons. TC does not resolve
/// arbitrary DIDs -- it stores the DID-to-device-key binding established
/// during enrollment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributorDid {
    /// The DID string, e.g. "did:near:alice.near" or "did:key:z6Mk..."
    pub did: String,
    /// The DID method.
    pub method: DidMethod,
    /// The Ed25519 public key bytes (same as the device key).
    pub public_key_b64: String,
    /// The device_key_id this DID is bound to.
    pub device_key_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DidMethod {
    /// did:near -- NEAR protocol-anchored identity
    Near,
    /// did:key -- self-certifying key-based identity (no blockchain)
    Key,
    /// did:web -- web-hosted DID document
    Web,
}

/// A Verifiable Credential issued by TraceCommons to a contributor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceCredential {
    pub credential_id: Uuid,
    /// The contributor's DID (credential subject).
    pub subject_did: String,
    /// The issuer DID (TraceCommons instance).
    pub issuer_did: String,
    pub credential_type: TraceCredentialType,
    /// The credential claims.
    pub claims: serde_json::Value,
    /// BBS+ signature over the claims (enables selective disclosure).
    pub proof: CredentialProof,
    pub issued_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TraceCredentialType {
    /// "Contributor has submitted N+ traces that passed the quality gate."
    QualityContributor,
    /// "Contributor has submitted traces to corpus X."
    CorpusMembership,
    /// "Contributor's average quality score exceeds T."
    QualityThreshold,
    /// "Contributor has been active for N+ days."
    LongevityBadge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialProof {
    /// Proof type: "BbsBlsSignature2020" for BBS+ selective disclosure.
    pub proof_type: String,
    /// Base64-encoded BBS+ signature.
    pub signature_b64: String,
    /// The issuer's BBS+ public key for verification.
    pub verification_method: String,
}

/// Service for issuing and verifying Verifiable Credentials.
pub struct CredentialService {
    /// The TC instance's DID (issuer identity).
    pub issuer_did: String,
    // bbs_keypair: BbsKeypair,  -- from bbs crate
}

impl CredentialService {
    /// Issue a VC to a contributor. The credential claims are derived
    /// from the contributor's trace history in the DB.
    pub async fn issue_credential(
        &self,
        subject_did: &str,
        credential_type: TraceCredentialType,
        tenant_id: &str,
    ) -> anyhow::Result<TraceCredential> {
        todo!()
    }

    /// Verify a VC's BBS+ signature. Does NOT check revocation status.
    pub fn verify_credential(
        &self,
        credential: &TraceCredential,
    ) -> anyhow::Result<bool> {
        todo!()
    }

    /// Create a selective-disclosure presentation from a VC: prove
    /// specific claims without revealing others.
    pub fn create_presentation(
        &self,
        credential: &TraceCredential,
        disclosed_indices: &[usize],
    ) -> anyhow::Result<serde_json::Value> {
        todo!()
    }
}
```

### PostgreSQL migration: `V_NN__dids_and_credentials.sql`

```sql
CREATE TABLE IF NOT EXISTS trace_contributor_dids (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       TEXT NOT NULL,
    did             TEXT NOT NULL,
    method          TEXT NOT NULL CHECK (method IN ('near', 'key', 'web')),
    device_key_id   TEXT NOT NULL,
    public_key_b64  TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, did)
);

ALTER TABLE trace_contributor_dids ENABLE ROW LEVEL SECURITY;
CREATE POLICY trace_contributor_dids_tenant_isolation
    ON trace_contributor_dids
    USING (tenant_id = trace_current_tenant_id());

CREATE TABLE IF NOT EXISTS trace_verifiable_credentials (
    credential_id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       TEXT NOT NULL,
    subject_did     TEXT NOT NULL,
    issuer_did      TEXT NOT NULL,
    credential_type TEXT NOT NULL,
    claims_hash     TEXT NOT NULL,  -- sha256 of claims JSON (hash-only)
    proof_type      TEXT NOT NULL,
    issued_at       TIMESTAMPTZ NOT NULL,
    expires_at      TIMESTAMPTZ,
    revoked_at      TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE trace_verifiable_credentials ENABLE ROW LEVEL SECURITY;
CREATE POLICY trace_verifiable_credentials_tenant_isolation
    ON trace_verifiable_credentials
    USING (tenant_id = trace_current_tenant_id());

CREATE INDEX idx_credentials_subject
    ON trace_verifiable_credentials (tenant_id, subject_did, credential_type);
```

### Dependencies

```toml
# BBS+ signatures for selective disclosure
bbs = "0.6"
# DID resolution (did:key)
did-key = "0.3"
# W3C VC data model
ssi = "0.9"  # Spruce SSI library
```

### Complexity: High

DID resolution and BBS+ signature generation are well-specified but the
Rust ecosystem is still maturing. The `ssi` crate provides W3C VC
compliance but may require patches for Ed25519-to-BBS+ key derivation.

---

## 8. Private Similarity Search (P1)

### Motivation

TraceCommons' vector index (`VectorIndex` trait) computes cosine
similarity over plaintext embeddings within a single tenant. Cross-tenant
or cross-instance similarity search is not supported because it would
require sharing raw embeddings -- a leakage of the trace content they
encode.

Private similarity search enables "find similar traces across
organizations without revealing trace content" by computing similarity
on encrypted or hashed representations.

### Design: LSH-based private search

Locality-Sensitive Hashing (LSH) maps similar vectors to the same hash
bucket with high probability. By sharing only LSH hashes (not raw
embeddings), two instances can find candidate similar traces without
revealing content. The hashes leak only whether traces are "probably
similar" or "definitely not similar."

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Configuration for the private similarity search service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateSearchConfig {
    /// Number of LSH hash tables. More tables = higher recall, more storage.
    pub num_tables: usize,
    /// Number of hash functions per table. More = higher precision, lower recall.
    pub num_hashes_per_table: usize,
    /// Embedding dimension. Must match the gate-api embedder output dim.
    pub embedding_dim: usize,
    /// Random projection seed for reproducibility across instances.
    pub projection_seed: u64,
}

impl Default for PrivateSearchConfig {
    fn default() -> Self {
        Self {
            num_tables: 8,
            num_hashes_per_table: 4,
            embedding_dim: 256,  // matches MOCK_EMBEDDING_DIM
            projection_seed: 0x7472_6163_6500_0001, // "trace" + version
        }
    }
}

/// An LSH fingerprint: a set of bucket hashes, one per table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LshFingerprint {
    /// One hash per table. Two fingerprints are candidate matches if they
    /// share at least one bucket hash.
    pub bucket_hashes: Vec<u64>,
    /// Hash of the LSH configuration (seed + params) so we can detect
    /// incompatible fingerprints from different configs.
    pub config_hash: String,
}

/// The private search index. Stores LSH fingerprints (not embeddings)
/// and supports candidate-set retrieval.
pub struct PrivateSearchService {
    config: PrivateSearchConfig,
    /// Random projection matrices, one per (table, hash). Generated
    /// deterministically from the projection seed.
    projections: Vec<Vec<Vec<f32>>>,
}

impl PrivateSearchService {
    /// Compute the LSH fingerprint of an embedding. This is the ONLY
    /// data that leaves the instance for cross-instance search.
    pub fn compute_fingerprint(&self, embedding: &[f32]) -> LshFingerprint {
        let mut bucket_hashes = Vec::with_capacity(self.config.num_tables);
        for table_idx in 0..self.config.num_tables {
            let mut hash_bits: u64 = 0;
            for hash_idx in 0..self.config.num_hashes_per_table {
                let projection = &self.projections[table_idx][hash_idx];
                let dot: f32 = embedding.iter()
                    .zip(projection.iter())
                    .map(|(a, b)| a * b)
                    .sum();
                if dot >= 0.0 {
                    hash_bits |= 1u64 << hash_idx;
                }
            }
            bucket_hashes.push(hash_bits);
        }
        LshFingerprint {
            bucket_hashes,
            config_hash: self.config_hash(),
        }
    }

    /// Find candidate matches: return submission_ids whose LSH fingerprint
    /// shares at least one bucket with the query fingerprint.
    pub async fn find_candidates(
        &self,
        query: &LshFingerprint,
        tenant_id: &str,
    ) -> anyhow::Result<Vec<Uuid>> {
        todo!()
    }

    fn config_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(format!(
            "lsh:tables={},hashes={},dim={},seed={}",
            self.config.num_tables,
            self.config.num_hashes_per_table,
            self.config.embedding_dim,
            self.config.projection_seed,
        ).as_bytes());
        format!("sha256:{:x}", h.finalize())
    }
}
```

### PostgreSQL migration: `V_NN__lsh_fingerprints.sql`

```sql
CREATE TABLE IF NOT EXISTS trace_lsh_fingerprints (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       TEXT NOT NULL,
    submission_id   UUID NOT NULL,
    table_index     INTEGER NOT NULL,
    bucket_hash     BIGINT NOT NULL,
    config_hash     TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE trace_lsh_fingerprints ENABLE ROW LEVEL SECURITY;
CREATE POLICY trace_lsh_fingerprints_tenant_isolation
    ON trace_lsh_fingerprints
    USING (tenant_id = trace_current_tenant_id());

-- Index for bucket lookup: given a bucket_hash, find all submissions in
-- that bucket for candidate matching.
CREATE INDEX idx_lsh_bucket_lookup
    ON trace_lsh_fingerprints (tenant_id, table_index, bucket_hash);
CREATE INDEX idx_lsh_submission
    ON trace_lsh_fingerprints (tenant_id, submission_id);
```

### Integration points

- **Extends**: `VectorIndex` trait -- add `fn lsh_fingerprint(&self, embedding: &[f32]) -> LshFingerprint`
  as a default method
- **New route**: `POST /v1/search/private` accepts an LSH fingerprint and returns candidates
- **Hook into**: vector worker post-insert path -- compute and store LSH fingerprint
  alongside the vector entry

### Dependencies

No new crate dependencies. LSH is implemented with standard random projections
using `rand` (seeded).

### Complexity: Low-Medium

LSH itself is well-understood and simple to implement. The challenge is
tuning the parameters (num_tables, num_hashes) for the right
precision/recall tradeoff on real embedding distributions.

---

## 9. CaMeL Capabilities Model (P2)

### Motivation

TraceCommons currently uses bearer tokens with role-scoped credentials
(utility, review, retention, vector, benchmark, etc.). Each credential
is a flat bearer string that grants a fixed set of permissions. This
model lacks:

1. **Delegation**: an operator cannot delegate a subset of their
   permissions to a sub-operator without creating a new credential.
2. **Attenuation**: a consumer who receives export access cannot further
   restrict it (e.g., "export only traces newer than 30 days").
3. **Auditability**: bearer tokens are opaque -- the audit chain records
   that a token was used, but not what permissions it carried at the time.

The CaMeL capabilities model replaces bearer tokens with capability
tokens: structured, attenuable, delegatable authorization objects.

### Design

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A capability token granting specific permissions on specific resources.
/// Capabilities are structured (not opaque), attenuable (can be narrowed
/// but never widened), and auditable (the token itself describes what it
/// permits).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityToken {
    pub token_id: Uuid,
    pub issuer: String,
    /// The principal this capability is issued to. May be a DID, a
    /// device_key_id, or a service identity.
    pub subject: String,
    /// Permissions granted by this capability.
    pub permissions: Vec<Permission>,
    /// Resource constraints (tenant scope, time range, etc.).
    pub constraints: Vec<Constraint>,
    /// Parent capability this was derived from (for delegation chains).
    pub parent_token_id: Option<Uuid>,
    /// Maximum delegation depth remaining. 0 = cannot be further delegated.
    pub delegation_depth: u32,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    /// Ed25519 signature over the serialized token (sans signature field).
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// Read trace metadata (not content).
    ReadMetadata,
    /// Read trace content (requires decryption).
    ReadContent,
    /// Submit new traces.
    Submit,
    /// Score traces via the gate.
    Score,
    /// Export traces for external consumption.
    Export,
    /// Aggregate statistics queries.
    Aggregate,
    /// Revoke traces.
    Revoke,
    /// Administer tenant settings.
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Constraint {
    /// Restrict to a specific tenant.
    Tenant(String),
    /// Restrict to traces after this timestamp.
    NotBefore(DateTime<Utc>),
    /// Restrict to traces before this timestamp.
    NotAfter(DateTime<Utc>),
    /// Restrict to specific submission IDs.
    Submissions(Vec<Uuid>),
    /// Restrict to a maximum number of operations.
    MaxOperations(u32),
    /// Restrict to specific consent scopes.
    ConsentScopes(Vec<String>),
}

/// Service for creating, attenuating, and validating capability tokens.
pub struct CapabilityService {
    // signing_key: EncodingKey,  -- Ed25519 key for signing tokens
}

impl CapabilityService {
    /// Create a new capability token. The caller must hold a capability
    /// that is a superset of the requested permissions.
    pub fn create_capability(
        &self,
        subject: &str,
        permissions: Vec<Permission>,
        constraints: Vec<Constraint>,
        delegation_depth: u32,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<CapabilityToken> {
        todo!()
    }

    /// Attenuate a capability: create a derived token with strictly fewer
    /// permissions or stricter constraints. The derived token's
    /// delegation_depth is decremented.
    pub fn attenuate(
        &self,
        parent: &CapabilityToken,
        remove_permissions: &[Permission],
        add_constraints: &[Constraint],
        new_subject: &str,
    ) -> anyhow::Result<CapabilityToken> {
        // Verify parent has delegation_depth > 0
        // Verify derived permissions are a subset of parent permissions
        // Verify derived constraints are a superset of parent constraints
        // Create new token with decremented delegation_depth
        todo!()
    }

    /// Validate a capability token: signature check, expiry check,
    /// constraint evaluation against the current request context.
    pub fn validate(
        &self,
        token: &CapabilityToken,
        required_permission: &Permission,
        request_context: &RequestContext,
    ) -> Result<(), CapabilityError> {
        todo!()
    }
}

pub struct RequestContext {
    pub tenant_id: String,
    pub timestamp: DateTime<Utc>,
    pub submission_ids: Vec<Uuid>,
}

#[derive(Debug, thiserror::Error)]
pub enum CapabilityError {
    #[error("CapabilityExpired: token expired at {expired_at}")]
    Expired { expired_at: DateTime<Utc> },
    #[error("CapabilityInsufficientPermission: requires {required:?}")]
    InsufficientPermission { required: Permission },
    #[error("CapabilityConstraintViolation: {constraint}")]
    ConstraintViolation { constraint: String },
    #[error("CapabilitySignatureInvalid")]
    SignatureInvalid,
    #[error("CapabilityDelegationExhausted: delegation depth is 0")]
    DelegationExhausted,
}
```

### PostgreSQL migration: `V_NN__capability_tokens.sql`

```sql
CREATE TABLE IF NOT EXISTS trace_capability_tokens (
    token_id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       TEXT NOT NULL,
    issuer          TEXT NOT NULL,
    subject         TEXT NOT NULL,
    permissions     JSONB NOT NULL,
    constraints     JSONB NOT NULL,
    parent_token_id UUID REFERENCES trace_capability_tokens(token_id),
    delegation_depth INTEGER NOT NULL DEFAULT 0,
    issued_at       TIMESTAMPTZ NOT NULL,
    expires_at      TIMESTAMPTZ NOT NULL,
    revoked_at      TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE trace_capability_tokens ENABLE ROW LEVEL SECURITY;
CREATE POLICY trace_capability_tokens_tenant_isolation
    ON trace_capability_tokens
    USING (tenant_id = trace_current_tenant_id());

CREATE INDEX idx_capability_tokens_subject
    ON trace_capability_tokens (tenant_id, subject)
    WHERE revoked_at IS NULL;
CREATE INDEX idx_capability_tokens_parent
    ON trace_capability_tokens (parent_token_id);
```

### Integration points

- **Replaces (gradually)**: bearer token auth in route handlers. Phase 1: add
  capability validation alongside existing bearer tokens. Phase 2: deprecate
  bearer tokens.
- **Extends**: `trace_upload_claim_issuer.rs` -- claim requests can carry a
  capability token instead of a bearer credential.

### Dependencies

No new crate dependencies. Capability tokens are signed with the existing
`jsonwebtoken` + Ed25519 infrastructure.

### Complexity: Medium

The data model is straightforward. The migration challenge is gradual
rollout: every existing bearer-token-gated route needs a parallel
capability-validation path. Recommend one route category (export) as
a pilot.

---

## 10. Secure Multi-Party Computation (P2)

### Motivation

When multiple TC instances (operated by different organizations) want to
compute joint statistics -- e.g., "what is the overall quality baseline
across all instances?" -- no single instance should see another's raw
data. Secure Multi-Party Computation (MPC) lets N parties jointly compute
a function over their private inputs, such that each party learns only
the output, not any other party's input.

### Design: Secret-sharing-based MPC for aggregate statistics

For aggregate statistics (mean, count, histogram), additive secret sharing
is sufficient and dramatically simpler than general-purpose MPC.

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Configuration for an MPC computation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpcSessionConfig {
    /// Unique session identifier.
    pub session_id: Uuid,
    /// The computation to perform.
    pub computation: MpcComputation,
    /// Participating instances (by their public identity).
    pub participants: Vec<MpcParticipant>,
    /// Minimum number of participants required to reconstruct the result.
    /// For Shamir's secret sharing, this is the threshold `t`.
    pub threshold: u32,
    /// Deadline for all participants to submit shares.
    pub deadline: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MpcComputation {
    /// Compute the mean quality score across all participants.
    MeanQualityScore,
    /// Compute the total trace count across all participants.
    TotalTraceCount,
    /// Compute a quality score histogram across all participants.
    QualityHistogram { num_bins: u32 },
    /// Compute the dedup rate across all participants.
    DedupRate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpcParticipant {
    /// Instance identifier (e.g., DID or domain).
    pub instance_id: String,
    /// Public key for encrypted share delivery.
    pub public_key_b64: String,
}

/// A secret share: one participant's contribution to the MPC computation.
/// In additive secret sharing, the true value is the sum of all shares.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpcShare {
    pub session_id: Uuid,
    pub participant_id: String,
    pub share_index: u32,
    /// The share value, encrypted under the coordinator's public key.
    pub encrypted_value_b64: String,
    /// HMAC over (session_id, participant_id, share_index, value) for
    /// share integrity.
    pub integrity_tag: String,
}

/// Shamir's Secret Sharing over a prime field (used for threshold MPC).
pub struct ShamirSecretSharing {
    /// The prime modulus for the finite field.
    pub modulus: u128,
    /// Number of shares to generate.
    pub num_shares: u32,
    /// Reconstruction threshold.
    pub threshold: u32,
}

impl ShamirSecretSharing {
    /// Split a secret into `num_shares` shares such that any `threshold`
    /// shares can reconstruct the secret.
    pub fn split(&self, secret: u128) -> Vec<(u32, u128)> {
        // Generate random polynomial of degree (threshold - 1) with
        // constant term = secret. Evaluate at points 1..=num_shares.
        let mut coeffs = vec![secret];
        for _ in 1..self.threshold {
            let coeff: u128 = rand::random::<u64>() as u128 % self.modulus;
            coeffs.push(coeff);
        }
        (1..=self.num_shares)
            .map(|x| {
                let x = x as u128;
                let mut y = 0u128;
                let mut x_pow = 1u128;
                for coeff in &coeffs {
                    y = (y + coeff * x_pow % self.modulus) % self.modulus;
                    x_pow = x_pow * x % self.modulus;
                }
                (x as u32, y)
            })
            .collect()
    }

    /// Reconstruct the secret from `threshold` or more shares using
    /// Lagrange interpolation at x=0.
    pub fn reconstruct(&self, shares: &[(u32, u128)]) -> anyhow::Result<u128> {
        anyhow::ensure!(
            shares.len() >= self.threshold as usize,
            "MpcReconstructionFailed: need {} shares, got {}",
            self.threshold,
            shares.len()
        );
        let mut secret = 0u128;
        for (i, &(xi, yi)) in shares.iter().enumerate() {
            let mut num = 1u128;
            let mut den = 1u128;
            for (j, &(xj, _)) in shares.iter().enumerate() {
                if i == j { continue; }
                // Lagrange basis: product of (0 - xj) / (xi - xj)
                num = num * (self.modulus - xj as u128) % self.modulus;
                let diff = if xi > xj {
                    (xi - xj) as u128
                } else {
                    self.modulus - (xj - xi) as u128
                };
                den = den * diff % self.modulus;
            }
            let den_inv = mod_inverse(den, self.modulus);
            let lagrange = num * den_inv % self.modulus;
            secret = (secret + yi * lagrange % self.modulus) % self.modulus;
        }
        Ok(secret)
    }
}

/// Modular multiplicative inverse using extended Euclidean algorithm.
fn mod_inverse(a: u128, m: u128) -> u128 {
    let (mut old_r, mut r) = (a as i128, m as i128);
    let (mut old_s, mut s) = (1i128, 0i128);
    while r != 0 {
        let q = old_r / r;
        let tmp_r = r;
        r = old_r - q * r;
        old_r = tmp_r;
        let tmp_s = s;
        s = old_s - q * s;
        old_s = tmp_s;
    }
    ((old_s % m as i128 + m as i128) % m as i128) as u128
}
```

### PostgreSQL migration: `V_NN__mpc_sessions.sql`

```sql
CREATE TABLE IF NOT EXISTS trace_mpc_sessions (
    session_id      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       TEXT NOT NULL,
    computation     TEXT NOT NULL,
    config          JSONB NOT NULL,
    status          TEXT NOT NULL CHECK (status IN (
        'pending', 'collecting', 'computing', 'completed', 'failed'
    )) DEFAULT 'pending',
    result_hash     TEXT,  -- sha256 of the reconstructed result (hash-only)
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    deadline        TIMESTAMPTZ NOT NULL,
    completed_at    TIMESTAMPTZ
);

ALTER TABLE trace_mpc_sessions ENABLE ROW LEVEL SECURITY;
CREATE POLICY trace_mpc_sessions_tenant_isolation
    ON trace_mpc_sessions
    USING (tenant_id = trace_current_tenant_id());

CREATE TABLE IF NOT EXISTS trace_mpc_shares (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id      UUID NOT NULL REFERENCES trace_mpc_sessions(session_id),
    participant_id  TEXT NOT NULL,
    share_index     INTEGER NOT NULL,
    encrypted_value_b64 TEXT NOT NULL,
    integrity_tag   TEXT NOT NULL,
    received_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (session_id, participant_id, share_index)
);

-- MPC shares are intentionally NOT RLS-scoped: the coordinator needs to
-- read shares from all participants. Access is gated by the session's
-- bearer credential instead.
```

### Integration points

- **New routes**: `POST /v1/mpc/session`, `POST /v1/mpc/session/{id}/share`,
  `GET /v1/mpc/session/{id}/result`
- **Cross-instance**: requires an inter-instance communication channel (HTTPS
  between TC instances, authenticated by mutual TLS or DID-based auth)

### Dependencies

No new crate dependencies for basic secret sharing. For more sophisticated
MPC protocols:

```toml
# Optional, for garbled circuits or oblivious transfer:
# swanky = "0.5"  # Galois MPC library
```

### Complexity: High

The secret sharing math is straightforward. The operational complexity is
in the multi-instance coordination: session management, share collection
with deadlines, handling participant dropouts, and the inter-instance
authentication layer.

---

## Summary: Priority and Dependency Map

| # | Enhancement | Priority | Complexity | Dependencies | Phase |
|---|---|---|---|---|---|
| 1 | Differential Privacy | P0 | Medium | None | 1 |
| 2 | Zero-Knowledge Proofs | P0 | High | Existing score attestation | 1 |
| 3 | C2PA v2.3 Integration | P0 | Medium | Existing artifact store | 1 |
| 4 | EU AI Act Compliance | P0 | Medium | Existing envelope types | 1 |
| 5 | Homomorphic Encryption | P1 | Very High | Gate-API embedder trait | 2 |
| 6 | SCITT (RFC 9943) | P1 | Medium | Existing audit chain | 2 |
| 7 | W3C DIDs + VCs | P1 | High | Existing device identity | 2 |
| 8 | Private Similarity Search | P1 | Low-Medium | Gate-API VectorIndex trait | 2 |
| 9 | CaMeL Capabilities | P2 | Medium | Existing bearer auth | 3 |
| 10 | Secure Multi-Party Computation | P2 | High | Cross-instance transport | 3 |

### Recommended implementation order (within each phase)

**Phase 1 (P0):**
1. EU AI Act Compliance -- immediate regulatory pressure, uses only existing types
2. C2PA Integration -- builds on artifact store, enables provenance for compliance
3. Differential Privacy -- wraps aggregate query endpoints, independent of other work
4. Zero-Knowledge Proofs (Bulletproofs only) -- extends score attestation

**Phase 2 (P1):**
5. SCITT -- extends audit chain, independent Merkle tree implementation
6. Private Similarity Search -- simple LSH, no new crate deps
7. W3C DIDs + VCs -- depends on DID ecosystem crate maturity
8. Homomorphic Encryption -- prototype only; full integration deferred to Phase 3

**Phase 3 (P2):**
9. CaMeL Capabilities -- gradual migration from bearer tokens
10. Secure MPC -- requires cross-instance transport layer

### Crate layout

All new code goes into existing crates:

| Module | Crate | File |
|---|---|---|
| `differential_privacy` | `trace-commons-server` | `src/differential_privacy.rs` |
| `zk_attestation` | `trace-commons-server` | `src/zk_attestation.rs` |
| `c2pa_manifest` | `trace-commons-server` | `src/c2pa_manifest.rs` |
| `compliance` | `trace-commons-server` | `src/compliance.rs` |
| `homomorphic` | `trace-commons-gate-enclave` | `src/homomorphic.rs` |
| `scitt_ledger` | `trace-commons-server` | `src/scitt_ledger.rs` |
| `did_identity` | `trace-commons-protocol` | `src/did_identity.rs` |
| `credential` | `trace-commons-server` | `src/credential.rs` |
| `private_search` | `trace-commons-gate-enclave` | `src/private_search.rs` |
| `capability` | `trace-commons-server` | `src/capability.rs` |
| `mpc` | `trace-commons-server` | `src/mpc.rs` |

No new crates are introduced. Each module extends the existing trait and
type hierarchy rather than introducing parallel abstractions.
