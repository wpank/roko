# Witness DAG: Cryptographic Cognitive Traces

> **v3 depth file** -- `/docs/v3/depth/12-safety/witness-dag.md`
> Canonical source: v1 `docs/v1/11-safety/12-witness-dag.md` (1,547 lines)
> Status: **Specified**. The data structures and algorithms are defined. SQLite
> persistence, ZK proof generation, and on-chain anchoring remain product work.
> The linear audit chain and content-addressed Signals provide the foundational
> infrastructure that the Witness DAG extends.

---

## 1. Motivation: Beyond Linear Audit Chains

The existing audit chain in Roko is a linear Merkle hash-chain: each decision is hashed,
each hash commits to the previous one. This chain proves that events happened in a
particular sequence. It cannot prove **why** those events happened.

A linear chain records that the agent swapped ETH for USDC at block 19,412,003. It says
nothing about the three observations that suggested a regime change, the two predictions
that confirmed it, or the Gate that approved the trade.

The Witness DAG extends the linear audit chain into a directed acyclic graph that links
every observation, prediction, decision, and outcome into a tamper-proof chain of
reasoning. Any learned knowledge in the Neuro store traces backward through the DAG to
the raw observations that justify it. The linear audit chain becomes a degenerate path
through the DAG -- backward compatibility is preserved.

---

## 2. Mathematical Foundations

### 2.1 DAG definition

A Witness DAG is a directed acyclic graph `G = (V, E)` where:

- `V` is a finite set of vertices, each representing a cognitive event (observation,
  prediction, decision, resolution, or knowledge entry).
- `E` is a set of directed edges `(u, v)` where vertex `u` causally contributes to
  vertex `v`. If `(u, v)` is in `E`, then `u` must have been created before `v`.

The DAG property (no directed cycles) is enforced structurally: every edge points from
an older vertex to a newer one, and the content-addressed hash of each vertex commits
to its parent hashes, making retroactive edge insertion detectable.

### 2.2 Content-addressed vertices

Each vertex is identified by a commitment hash:

```
commitment(v) = BLAKE3(type || content_hash || timestamp || parent_hashes || metadata)
```

Where:

- `type` is one of five vertex types (Section 3).
- `content_hash` is the BLAKE3 hash of the vertex payload.
- `timestamp` is the Unix millisecond creation time.
- `parent_hashes` is the sorted, concatenated hashes of all parent vertices.
- `metadata` is a canonical JSON encoding of vertex-type-specific fields.

The commitment hash is collision-resistant under BLAKE3 (256-bit output, conjectured
2^128 collision resistance). Modifying any field changes the commitment, making
tampering detectable by any party holding the original hash.

### 2.3 Tamper evidence

The DAG inherits tamper evidence from two properties:

1. **Hash chain integrity.** Each vertex commits to its parent hashes. Modifying a
   vertex changes its hash, which invalidates every descendant that references it. An
   attacker cannot modify a vertex without re-creating the entire sub-DAG rooted at
   that vertex.

2. **Topological ordering.** The DAG's topological sort defines a unique causal ordering
   (up to concurrent vertices). Any vertex claiming a parent that does not exist or has
   a later timestamp is detectable as forged.

Together, these properties provide Merkle-tree-level tamper evidence over an arbitrary
DAG topology, not just a linear chain.

### 2.4 Provenance paths

A provenance path `P(v)` from a vertex `v` to a root (parentless) vertex is any
directed path in the transposed graph `G^T`. The set of all provenance paths from `v`
defines the complete causal history of the cognitive event that `v` represents.

For forensic replay, the provenance query returns the transitive closure of `v`'s
ancestors:

```
ancestors(v) = { u in V : there exists a directed path from u to v }
```

This set is the minimal context needed to reconstruct the reasoning chain that produced
`v`.

---

## 3. Five Vertex Types

The Witness DAG uses five typed vertices to cover the full cognitive lifecycle:

### 3.1 Observation

```rust
pub struct ObservationVertex {
    pub commitment: [u8; 32],
    pub content_hash: [u8; 32],
    pub source: ObservationSource,
    pub timestamp_ms: i64,
    pub confidence: f64,
    pub taint: TaintLevel,
    pub metadata: ObservationMetadata,
}

pub enum ObservationSource {
    MarketData { venue: String, pair: String },
    WebFetch { url: String, status: u16 },
    ToolResult { tool_name: String, exit_code: i32 },
    UserInput { session_id: String },
    PluginOutput { plugin_id: String, tier: PluginTier },
    InternalComputation { component: String },
}
```

An Observation is a root or near-root vertex: it records raw data entering the system.
Observations carry taint metadata from the trust-origin lattice (see
`taint-tracking-ifc.md`), establishing the initial taint level that propagates through
all descendant vertices.

### 3.2 Prediction

```rust
pub struct PredictionVertex {
    pub commitment: [u8; 32],
    pub content_hash: [u8; 32],
    pub observation_parents: Vec<[u8; 32]>,
    pub model_fingerprint: String,
    pub prediction_type: PredictionType,
    pub confidence: f64,
    pub horizon_ms: i64,
    pub timestamp_ms: i64,
}

pub enum PredictionType {
    PriceMovement { direction: Direction, magnitude: f64 },
    GateOutcome { expected_verdict: Verdict, confidence: f64 },
    TaskCompletion { estimated_turns: u32 },
    RiskAssessment { risk_level: f64 },
    Custom { kind: String, payload: serde_json::Value },
}
```

A Prediction vertex connects Observations to Decisions. Its `observation_parents` field
lists the Observation commitments that the prediction is based on. The `model_fingerprint`
records which LLM or deterministic model produced the prediction, enabling attribution
in the forensic replay.

### 3.3 Decision

```rust
pub struct DecisionVertex {
    pub commitment: [u8; 32],
    pub content_hash: [u8; 32],
    pub prediction_parents: Vec<[u8; 32]>,
    pub observation_parents: Vec<[u8; 32]>,
    pub action: ActionDescription,
    pub gate_verdicts: Vec<GateVerdictRef>,
    pub corrigibility_decision: CorrigibilityDecision,
    pub confidence: f64,
    pub timestamp_ms: i64,
    pub principal: String,
}

pub struct ActionDescription {
    pub tool_name: String,
    pub parameters_hash: [u8; 32],
    pub estimated_cost_usd: f64,
    pub irreversibility_score: f64,
    pub sandbox_level: SandboxLevel,
}

pub struct GateVerdictRef {
    pub gate_name: String,
    pub rung: u8,
    pub verdict: Verdict,
    pub confidence: f64,
}
```

A Decision vertex is the central vertex type. It records:

- Which Predictions and Observations influenced the decision.
- What action was taken.
- Which Gates approved or rejected the action.
- The five-head corrigibility evaluation result.
- The principal (agent or user) that authorized the action.

The `parameters_hash` field stores a hash of the tool call parameters rather than
the parameters themselves, preventing secret leakage into the DAG while preserving
forensic verifiability (the auditor can re-hash the stored parameters and compare).

### 3.4 Resolution

```rust
pub struct ResolutionVertex {
    pub commitment: [u8; 32],
    pub content_hash: [u8; 32],
    pub decision_parent: [u8; 32],
    pub outcome: Outcome,
    pub actual_vs_predicted: Option<PredictionAccuracy>,
    pub timestamp_ms: i64,
}

pub enum Outcome {
    Success { result_hash: [u8; 32] },
    Failure { error_kind: String, message: String },
    Partial { completed_fraction: f64, result_hash: [u8; 32] },
    Timeout { elapsed_ms: u64 },
    Cancelled { reason: String },
}

pub struct PredictionAccuracy {
    pub prediction_commitment: [u8; 32],
    pub predicted_value: f64,
    pub actual_value: f64,
    pub error: f64,
}
```

A Resolution vertex closes the feedback loop. It records the actual outcome of a
Decision, enabling comparison between what was predicted and what actually happened.
The `PredictionAccuracy` field explicitly links back to the Prediction vertex that
was evaluated, providing calibration data for the learning system.

### 3.5 NeuroEntry

```rust
pub struct NeuroEntryVertex {
    pub commitment: [u8; 32],
    pub content_hash: [u8; 32],
    pub resolution_parents: Vec<[u8; 32]>,
    pub decision_parents: Vec<[u8; 32]>,
    pub knowledge_type: KnowledgeType,
    pub confidence: f64,
    pub tier: KnowledgeTier,
    pub hdc_fingerprint: Option<[u8; 32]>,
    pub timestamp_ms: i64,
}

pub enum KnowledgeType {
    Heuristic { domain: String },
    Fact { source: String },
    Preference { learned_from: String },
    Playbook { when_pattern: String, then_action: String },
    Falsifier { contradicts: [u8; 32] },
}

pub enum KnowledgeTier {
    Transient,
    Working,
    Consolidated,
    Core,
}
```

A NeuroEntry vertex links durable knowledge to the Resolution and Decision vertices
that justify it. This is the critical property that the linear audit chain cannot
provide: any piece of learned knowledge traces back through the DAG to the raw
observations, predictions, and decisions that produced it. If those observations turn
out to be wrong, every knowledge entry that depends on them can be identified and
re-evaluated.

---

## 4. Commitment Hash Construction

### 4.1 The BLAKE3 commitment

Each vertex type produces a canonical byte sequence for hashing:

```rust
pub fn compute_commitment(vertex: &WitnessVertex) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();

    // 1. Type discriminant (1 byte)
    hasher.update(&[vertex.type_discriminant()]);

    // 2. Content hash (32 bytes)
    hasher.update(&vertex.content_hash());

    // 3. Timestamp (8 bytes, big-endian)
    hasher.update(&vertex.timestamp_ms().to_be_bytes());

    // 4. Parent hashes (sorted, then concatenated)
    let mut parents = vertex.parent_hashes();
    parents.sort();
    for parent in &parents {
        hasher.update(parent);
    }

    // 5. Metadata (canonical JSON, then BLAKE3)
    let metadata_bytes = serde_json::to_vec(&vertex.metadata())
        .expect("metadata serialization");
    let metadata_hash = blake3::hash(&metadata_bytes);
    hasher.update(metadata_hash.as_bytes());

    hasher.finalize().into()
}
```

### 4.2 Why BLAKE3

BLAKE3 is chosen over SHA-256 for three reasons:

1. **Performance.** BLAKE3 is 3-14x faster than SHA-256 on modern hardware, with
   SIMD-accelerated implementations available for x86-64, ARM, and WASM targets.
   For a DAG that may accumulate millions of vertices, hash computation cost matters.

2. **Parallelism.** BLAKE3's Merkle tree internal structure supports incremental and
   parallel hashing. Large vertex payloads (e.g., full context windows) can be hashed
   efficiently without buffering the entire payload.

3. **Security.** BLAKE3 provides 256-bit output with conjectured 128-bit collision
   resistance, matching SHA-256's security level for practical purposes.

The crate `blake3` (O'Connor & Aumasson, 2020) is already a workspace dependency.

### 4.3 Canonical serialization

Parent hashes are sorted before concatenation to ensure that the commitment is
independent of the order in which parents are discovered. Metadata is serialized
using `serde_json::to_vec()` with default settings (no pretty-printing, deterministic
key ordering via `BTreeMap`).

This canonical form means that any two implementations producing the same vertex
from the same inputs will produce the same commitment hash. Cross-deployment
verification requires only agreement on the hash function and serialization format.

---

## 5. DAG Operations

### 5.1 Insertion

Adding a vertex to the DAG requires:

1. Compute the commitment hash.
2. Verify that all parent commitments exist in the DAG.
3. Verify that all parent timestamps are strictly earlier than the new vertex's timestamp.
4. Insert the vertex and update the vertex-type indexes.
5. Update the DAG's latest root hash (see Section 7).

```rust
pub fn insert(&mut self, vertex: WitnessVertex) -> Result<[u8; 32], DagError> {
    let commitment = compute_commitment(&vertex);

    // Parent existence check
    for parent in vertex.parent_hashes() {
        if !self.vertices.contains_key(&parent) {
            return Err(DagError::MissingParent(parent));
        }
    }

    // Temporal ordering check
    for parent in vertex.parent_hashes() {
        let parent_vertex = &self.vertices[&parent];
        if parent_vertex.timestamp_ms() >= vertex.timestamp_ms() {
            return Err(DagError::TemporalViolation {
                parent: parent,
                child: commitment,
            });
        }
    }

    // Insert and index
    self.vertices.insert(commitment, vertex);
    self.update_type_index(commitment);
    self.update_root_hash();

    Ok(commitment)
}
```

### 5.2 Provenance query

The provenance query returns all ancestors of a vertex:

```rust
pub fn provenance(&self, commitment: &[u8; 32]) -> ProvenanceResult {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut observations = Vec::new();
    let mut predictions = Vec::new();
    let mut decisions = Vec::new();
    let mut resolutions = Vec::new();
    let mut neuro_entries = Vec::new();

    queue.push_back(*commitment);

    while let Some(current) = queue.pop_front() {
        if !visited.insert(current) {
            continue;
        }
        let Some(vertex) = self.vertices.get(&current) else {
            continue;
        };

        match vertex {
            WitnessVertex::Observation(v) => observations.push(v.clone()),
            WitnessVertex::Prediction(v) => predictions.push(v.clone()),
            WitnessVertex::Decision(v) => decisions.push(v.clone()),
            WitnessVertex::Resolution(v) => resolutions.push(v.clone()),
            WitnessVertex::NeuroEntry(v) => neuro_entries.push(v.clone()),
        }

        for parent in vertex.parent_hashes() {
            if !visited.contains(&parent) {
                queue.push_back(parent);
            }
        }
    }

    ProvenanceResult {
        root: *commitment,
        observations,
        predictions,
        decisions,
        resolutions,
        neuro_entries,
        depth: compute_max_depth(&visited, commitment, &self.vertices),
    }
}
```

### 5.3 Verification

Verifying a vertex confirms that its commitment hash matches its contents:

```rust
pub fn verify(&self, commitment: &[u8; 32]) -> Result<bool, DagError> {
    let vertex = self.vertices.get(commitment)
        .ok_or(DagError::NotFound(*commitment))?;

    let recomputed = compute_commitment(vertex);
    if recomputed != *commitment {
        return Ok(false);  // Tampered
    }

    // Verify parent chain recursively (or iteratively for large DAGs)
    for parent in vertex.parent_hashes() {
        if !self.verify(&parent)? {
            return Ok(false);  // Ancestor tampered
        }
    }

    Ok(true)
}
```

Full DAG verification walks the entire graph in topological order, verifying each
vertex's commitment and parent references. For large DAGs, verification can be
parallelized by processing independent sub-DAGs concurrently.

### 5.4 Subgraph extraction

For export, replay, or cross-deployment verification, the DAG supports subgraph
extraction:

```rust
pub fn extract_subgraph(
    &self,
    roots: &[[u8; 32]],
) -> WitnessDAG {
    let mut subgraph = WitnessDAG::new();
    let mut visited = HashSet::new();
    let mut queue: VecDeque<[u8; 32]> = roots.iter().copied().collect();

    while let Some(commitment) = queue.pop_front() {
        if !visited.insert(commitment) {
            continue;
        }
        if let Some(vertex) = self.vertices.get(&commitment) {
            subgraph.vertices.insert(commitment, vertex.clone());
            for parent in vertex.parent_hashes() {
                if !visited.contains(&parent) {
                    queue.push_back(parent);
                }
            }
        }
    }

    subgraph
}
```

---

## 6. Storage Model

### 6.1 SQLite persistence

The Witness DAG uses SQLite for durable storage:

```sql
CREATE TABLE vertices (
    commitment BLOB PRIMARY KEY,     -- 32 bytes, BLAKE3 hash
    vertex_type INTEGER NOT NULL,    -- 0-4 enum discriminant
    content_hash BLOB NOT NULL,      -- 32 bytes
    timestamp_ms INTEGER NOT NULL,
    payload BLOB NOT NULL,           -- CBOR-encoded vertex data
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE edges (
    parent BLOB NOT NULL,            -- commitment of parent vertex
    child BLOB NOT NULL,             -- commitment of child vertex
    PRIMARY KEY (parent, child),
    FOREIGN KEY (parent) REFERENCES vertices(commitment),
    FOREIGN KEY (child) REFERENCES vertices(commitment)
);

CREATE INDEX idx_vertices_type ON vertices(vertex_type);
CREATE INDEX idx_vertices_timestamp ON vertices(timestamp_ms);
CREATE INDEX idx_edges_child ON edges(child);
```

### 6.2 Why SQLite

SQLite is chosen for several properties that align with the DAG's requirements:

1. **ACID transactions.** Vertex insertion with edge creation is atomic. A crash
   during insertion cannot leave orphan edges or dangling parent references.

2. **Embedded.** No separate database process. The DAG lives in a single file in
   `.roko/state/witness-dag.sqlite`, portable across deployments.

3. **Proven scale.** SQLite handles databases up to 281 TB. A Witness DAG with
   10 million vertices occupies approximately 3-5 GB, well within SQLite's
   comfortable range.

4. **WAL mode.** Write-Ahead Logging enables concurrent reads during writes,
   important for forensic queries during live execution.

### 6.3 CBOR payload encoding

Vertex payloads are encoded using CBOR (RFC 8949) rather than JSON for two reasons:

1. **Deterministic encoding.** CBOR's canonical form (RFC 8949 Section 4.2.1) ensures
   that the same vertex always produces the same byte sequence, which is required for
   commitment hash stability.

2. **Compact representation.** CBOR produces 20-40% smaller payloads than JSON for
   typical vertex data, reducing storage and network transfer costs.

### 6.4 Retention and pruning

The DAG supports configurable retention policies:

- **Time-based retention.** Vertices older than a configured threshold (default: 90
  days) can be pruned. Pruned vertices have their payloads deleted but their commitments
  and edges retained as "tombstones," preserving the DAG structure for verification.

- **Depth-based retention.** The DAG retains at least N levels of ancestors for any
  active Decision or NeuroEntry vertex, regardless of age.

- **Anchored retention.** Vertices whose commitment hashes have been anchored on-chain
  (Section 7) are never pruned, as they serve as trust anchors for verification.

---

## 7. On-Chain Anchoring

### 7.1 Root hash publication

The DAG periodically publishes its root hash to an external chain for independent
timestamp verification:

```rust
pub struct DagAnchor {
    pub root_hash: [u8; 32],          // Merkle root of all vertex commitments
    pub vertex_count: u64,
    pub latest_timestamp_ms: i64,
    pub chain_tx_hash: Option<[u8; 32]>,
    pub block_number: Option<u64>,
}
```

The root hash is computed as a Merkle tree over all vertex commitments in topological
order. Publishing this hash on-chain provides:

1. **Non-repudiable timestamp.** The chain's block timestamp proves that all vertices
   in the DAG existed before the anchor time. This survives local clock manipulation.

2. **Third-party verifiability.** Any party with the DAG data and the on-chain anchor
   can verify that the DAG has not been modified since the anchor was published.

3. **Cross-deployment coordination.** Multiple deployments can compare their DAG states
   by comparing anchored root hashes at the same block height.

### 7.2 Incremental anchoring

Publishing every vertex on-chain is prohibitively expensive. Instead, the DAG uses
incremental anchoring:

1. Accumulate vertices during a configurable window (default: 1 hour or 1000 vertices,
   whichever comes first).
2. Compute the Merkle root of the new vertices.
3. Compute the combined root as `BLAKE3(previous_anchor_root || new_merkle_root)`.
4. Publish the combined root in a single on-chain transaction.

This amortizes the on-chain cost across many vertices while preserving the ability to
verify any individual vertex's membership in the anchored set.

### 7.3 Verification against anchor

To verify that a vertex existed at the time of an anchor:

1. Retrieve the anchor's root hash from the chain.
2. Retrieve all vertices from the DAG that were included in that anchor's batch.
3. Reconstruct the Merkle tree from those vertices.
4. Verify that the computed root matches the on-chain root.
5. Verify that the target vertex is a leaf in the Merkle tree.

If all checks pass, the vertex provably existed at or before the anchor's block
timestamp.

---

## 8. Zero-Knowledge Proofs

### 8.1 Strategy privacy

An agent's decision-making strategy is often proprietary. The Witness DAG must support
auditing without revealing the strategy. Zero-knowledge proofs provide this:

**Statement:** "This decision was based on observations O1, O2, O3 and satisfied
corrigibility heads 1-5 with confidence > 0.8."

**Proof:** A ZK proof that:

1. The Decision vertex commits to parent Observation vertices with the claimed hashes.
2. The corrigibility evaluation stored in the Decision vertex satisfies the claimed
   thresholds.
3. The commitment hash is correctly computed from the vertex contents.

The verifier learns that the statement is true without learning the specific
observations, the prediction model, or the strategy parameters.

### 8.2 SNARK construction

The proof system uses Groth16 SNARKs (Groth, 2016) over the BN254 curve:

```rust
pub struct WitnessProof {
    pub statement: ProofStatement,
    pub proof: Groth16Proof,
    pub verification_key: VerificationKey,
}

pub enum ProofStatement {
    DecisionJustification {
        decision_commitment: [u8; 32],
        observation_count: usize,
        min_confidence: f64,
        corrigibility_passed: bool,
    },
    KnowledgeProvenance {
        knowledge_commitment: [u8; 32],
        observation_count: usize,
        resolution_count: usize,
        min_accuracy: f64,
    },
    TemporalOrdering {
        vertex_a: [u8; 32],
        vertex_b: [u8; 32],
        a_before_b: bool,
    },
}
```

### 8.3 Proof types

Three proof types cover the primary audit scenarios:

1. **DecisionJustification.** Proves that a decision was based on at least N
   observations with minimum confidence C, without revealing the observations.

2. **KnowledgeProvenance.** Proves that a knowledge entry traces back to at least N
   observations through at least M successful resolutions with minimum accuracy A.

3. **TemporalOrdering.** Proves that vertex A was created before vertex B, without
   revealing the actual timestamps or any other vertex content.

---

## 9. Integration with the Roko Architecture

### 9.1 Signal-to-vertex mapping

Each Signal produced by the Roko processing loop can optionally produce a Witness DAG
vertex:

| Signal Kind | Vertex Type |
|---|---|
| `Kind::Observation` | `ObservationVertex` |
| `Kind::Prediction` | `PredictionVertex` |
| `Kind::ToolInvocation` | `DecisionVertex` |
| `Kind::GateVerdict` | (metadata on `DecisionVertex`) |
| `Kind::Completion` | `ResolutionVertex` |
| `Kind::Insight` | `NeuroEntryVertex` |

Not every Signal produces a vertex. The DAG is opt-in per configuration:

```toml
[safety.witness_dag]
enabled = true
vertex_filter = ["Decision", "Resolution", "NeuroEntry"]
anchor_interval_minutes = 60
storage_path = ".roko/state/witness-dag.sqlite"
retention_days = 90
```

### 9.2 Linear audit chain compatibility

The existing linear audit chain (see `audit-chain.md`) is preserved as a degenerate
path through the DAG. Each audit chain entry maps to a Decision vertex with a single
parent edge pointing to the previous entry. Deployments that do not enable the full DAG
continue to use the linear chain with no behavioral change.

### 9.3 Gate verdict integration

Gate verdicts are embedded in Decision vertices rather than stored as separate vertices.
This design decision reflects the observation that a gate verdict has no independent
causal significance -- it only matters in the context of the decision it evaluated.
The `gate_verdicts` field on `DecisionVertex` stores the full verdict history including
rung, verdict, and confidence for each gate that evaluated the decision.

### 9.4 Neuro store integration

When the Neuro store promotes a knowledge entry from Transient to Working tier (see
the memory depth files), the `NeuroEntryVertex` is created with parent edges pointing
to the Resolution vertices that confirmed the knowledge. This creates a verifiable
chain from raw data through decision outcomes to durable knowledge.

### 9.5 Forensic replay integration

The forensic replay engine (see `forensic-ai.md`) uses the DAG's provenance query as
its primary data source. Given a Decision vertex, the replay engine reconstructs:

1. All Observation vertices that influenced the decision (direct and transitive).
2. All Prediction vertices that mediated between observations and the decision.
3. The Gate verdicts and corrigibility evaluation at decision time.
4. The Resolution vertex that records the decision's outcome.
5. Any NeuroEntry vertices that were created as a consequence.

This reconstruction is richer than what the linear audit chain provides because it
captures the causal structure, not just the temporal sequence.

---

## 10. Threat Model for the DAG

### 10.1 Attacks and mitigations

| Attack | Mitigation |
|---|---|
| Retroactive vertex modification | Commitment hash invalidation propagates to descendants |
| Vertex deletion | On-chain anchors prove prior existence |
| Fake vertex insertion | Parent existence and temporal ordering checks |
| Timestamp manipulation | On-chain anchor timestamps are independently verified |
| DAG structure manipulation | Commitment includes sorted parent hashes |
| Selective disclosure | ZK proofs reveal only claimed properties |

### 10.2 Trust assumptions

The DAG's security properties assume:

1. BLAKE3 is collision-resistant (standard cryptographic assumption).
2. The on-chain anchor contract is not compromised.
3. The local SQLite store is not tampered with between anchoring intervals.
4. The ZK proof system (Groth16) is sound under the BN254 discrete log assumption.

### 10.3 Residual risks

| Risk | Why it remains |
|---|---|
| DAG omission | An agent can choose not to record a vertex; anchoring proves what was recorded, not what was omitted |
| Timestamp coarseness | On-chain anchors are periodic (default: hourly); events between anchors rely on local timestamps |
| Proof generation cost | Groth16 proof generation requires 5-30 seconds per proof; real-time auditing is not feasible |
| Storage growth | Large DAGs require pruning policies that may reduce forensic depth for old events |

---

## 11. DAG Metrics and Health

### 11.1 Structural metrics

The DAG exposes structural metrics for monitoring:

```rust
pub struct DagMetrics {
    pub total_vertices: u64,
    pub vertices_by_type: HashMap<VertexType, u64>,
    pub total_edges: u64,
    pub max_depth: u64,
    pub avg_fan_in: f64,
    pub avg_fan_out: f64,
    pub orphan_count: u64,
    pub latest_anchor: Option<DagAnchor>,
    pub unanchored_vertices: u64,
    pub storage_bytes: u64,
}
```

### 11.2 Health checks

Periodic health checks verify DAG integrity:

1. **Commitment verification.** Random sample of vertices re-hashed and compared.
2. **Edge consistency.** Every edge references existing vertices with correct temporal
   ordering.
3. **Anchor freshness.** The latest anchor is within the configured interval.
4. **Storage budget.** Total storage is within the configured limit.

Failed health checks produce diagnostic Signals that feed into the conductor's
circuit breaker.

---

## 12. CLI Integration

The Witness DAG exposes CLI commands for operator inspection:

```bash
# Show DAG statistics
roko custody dag stats

# Show provenance chain for a specific vertex
roko custody dag provenance <commitment-hash>

# Verify a vertex and its ancestor chain
roko custody dag verify <commitment-hash>

# Export a subgraph for external review
roko custody dag export --roots <hash1>,<hash2> --format cbor

# Anchor the current DAG state on-chain
roko custody dag anchor

# Show anchor history
roko custody dag anchors --last 10

# Generate a ZK proof for a decision
roko custody dag prove --decision <hash> --type justification
```

---

## 13. Comparison with Existing Approaches

### 13.1 vs. Merkle trees

Merkle trees (Merkle, 1987) verify set membership but not causal structure. The DAG
extends Merkle trees by encoding parent-child relationships as first-class edges,
enabling provenance queries that Merkle trees cannot answer.

### 13.2 vs. blockchain audit logs

Blockchain audit logs provide non-repudiable ordering but treat each entry as
independent. The DAG's edge structure captures causal dependencies, answering
"why did this happen" in addition to "did this happen."

### 13.3 vs. W3C PROV

The W3C PROV data model (Moreau et al., 2013) defines three types: Entity, Activity,
and Agent. The Witness DAG's five types provide finer granularity specific to cognitive
agents: Observation and Prediction split PROV's Entity type, while Decision and
Resolution split PROV's Activity type.

### 13.4 vs. causal inference graphs

Causal inference graphs (Pearl, 2009) model statistical dependencies. The Witness DAG
models computational causality: vertex B was computed using vertex A's output. The
distinction matters because computational causality is deterministic and verifiable,
while statistical causality is probabilistic.

---

## 14. Performance Considerations

### 14.1 Insertion throughput

Target: 10,000 vertices per second sustained. BLAKE3 hash computation is the bottleneck
at approximately 0.5 microseconds per vertex (excluding I/O). SQLite WAL mode supports
concurrent reads during batch insertions.

### 14.2 Provenance query latency

Provenance queries traverse the ancestor set via BFS. For a typical Decision vertex
with 20-50 ancestors, query latency is under 1 millisecond. For deep provenance chains
(1000+ ancestors), the query uses iterative deepening with a configurable depth limit.

### 14.3 Memory footprint

The in-memory vertex cache uses an LRU policy with a configurable size limit (default:
10,000 vertices, approximately 50 MB). Cold vertices are read from SQLite on demand.

---

## Academic References

| Paper | Contribution |
|---|---|
| O'Connor & Aumasson (2020), "BLAKE3" | Hash function for content-addressing |
| Merkle (1987), "A Digital Signature Based on a Conventional Encryption Function" | Merkle tree tamper evidence |
| Groth (2016), "On the Size of Pairing-Based Non-interactive Arguments" | Groth16 SNARK for ZK proofs |
| Moreau et al. (2013), "PROV-DM: The PROV Data Model" | W3C provenance standard |
| Pearl (2009), "Causality: Models, Reasoning, and Inference" | Causal graph foundations |
| Saltzer & Schroeder (1975), "The Protection of Information in Computer Systems" | Complete mediation |

---

## Implementation References

| Component | Location |
|---|---|
| Signal content-addressing | `crates/roko-core/src/signal.rs` |
| Signal lineage | `crates/roko-core/src/signal.rs` `lineage` field |
| FileSubstrate (JSONL) | `crates/roko-fs/` |
| ToolDispatcher audit emissions | `crates/roko-agent/src/dispatcher/mod.rs` |
| Gate verdict persistence | `.roko/learn/gate-thresholds.json` |
| Episode logging | `.roko/episodes.jsonl` |
| Corrigibility pipeline | `crates/roko-core/src/corrigibility.rs` |
| TaintTracker | `crates/roko-agent/src/safety/taint_propagation.rs` |
