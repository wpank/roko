# Forensic AI -- Causal Replay

> Depth file for [07-GATES.md](../../07-GATES.md) section 14.
> Source: `crates/roko-gate/src/forensic.rs`,
> `crates/roko-gate/src/artifact_store.rs`,
> `crates/roko-learn/src/episode_logger.rs`

---

## 1. What Forensic Replay Means

Forensic replay reconstructs, step by step, exactly what an agent did, why it
did it, and what verification outcomes resulted -- with cryptographic proof
(BLAKE3 content addressing) that the reconstruction is accurate. This is not
debugging. This is audit-grade reconstruction that can withstand regulatory
scrutiny.

Given a task ID, produce the complete chain from initial prompt through every
tool call, every gate verdict, every retry, to the final outcome. Every
artifact in this chain is content-addressed, so any tampering is detectable.

---

## 2. The Content-Addressed Chain

Every element in the replay chain is identified by its BLAKE3 hash:

```
TaskSpec (hash: 0xa3f...)
    |
SystemPrompt (hash: 0xb7c...)
    |
AgentTurn 1 (hash: 0xc1d...)
    +-- ToolCall: Read "src/lib.rs" (hash: 0xd2e...)
    |   +-- Result: file contents (hash: 0xe3f...)
    +-- ToolCall: Edit "src/lib.rs" (hash: 0xf4a...)
    |   +-- Result: success (hash: 0xa5b...)
    +-- Response: "I've added the new struct" (hash: 0xb6c...)
    |
GateVerdict Rung 0 (hash: 0xc7d...)
    +-- Detail: compile output (hash: 0xd8e...)
    |
AgentTurn 2 (hash: 0xe9f...) [retry after gate failure]
    ...
    |
GateVerdict Rung 0 (hash: 0xab1...)  [pass]
GateVerdict Rung 1 (hash: 0xbc2...)  [pass]
GateVerdict Rung 2 (hash: 0xcd3...)  [pass]
    |
FinalOutcome (hash: 0xde4...)
```

Each node's hash incorporates its content. Modifying any element changes its
hash, and the chain becomes inconsistent. This is the same principle Git uses
for commits and blockchains use for blocks.

---

## 3. Implementation: CausalChain and TurnRecord

The `forensic.rs` module in `roko-gate` provides the data structures:

```rust
pub struct CausalChain {
    pub task_id: String,
    pub agent_model: String,
    pub turns: Vec<TurnRecord>,
    pub verdicts: Vec<(Verdict, Option<ContentHash>)>,
    pub artifacts: Vec<ArtifactMetadata>,
    pub integrity_verified: bool,
}

pub struct TurnRecord {
    pub turn_index: usize,
    pub agent_model: String,
    pub verdicts: Vec<Verdict>,
    pub artifact_hashes: Vec<ContentHash>,
}

pub struct ArtifactMetadata {
    pub hash: ContentHash,
    pub size_bytes: usize,
    pub kind: String,
    pub integrity_verified: bool,
}
```

The `ForensicReplayBuilder` constructs a `CausalChain` from stored artifacts
and episode data:

```rust
pub struct ForensicReplayBuilder {
    pub task_id: String,
    pub artifact_store: ArtifactStore,
}
```

Construction flow:

1. Query the episode log for all turns with this `task_id`
2. For each turn, retrieve tool call inputs and outputs
3. Query the signal log for all verdicts associated with this `task_id`
4. For each verdict, retrieve the gate artifact from `ArtifactStore` by hash
5. Build the causal chain: TaskSpec -> Prompt -> Turns -> Verdicts -> Outcome
6. Verify chain integrity: recompute each BLAKE3 hash and compare

---

## 4. Data Sources

### 4.1 Episode Log

**Path**: `.roko/episodes.jsonl`

Each line records an agent turn:

```json
{
  "task_id": "plan-42-task-3",
  "turn": 1,
  "model": "claude-opus-4-6",
  "tool_calls": [
    {"tool": "Read", "args": {"path": "src/lib.rs"}, "duration_ms": 45},
    {"tool": "Edit", "args": {"path": "src/lib.rs"}, "duration_ms": 12}
  ],
  "input_tokens": 12500,
  "output_tokens": 3200,
  "timestamp": "2026-04-10T14:30:00Z"
}
```

### 4.2 Signal Log

**Path**: `.roko/engrams.jsonl`

Every signal written to the substrate, including gate verdicts:

```json
{
  "hash": "0xab1c2d3e...",
  "kind": "verdict",
  "body": {"gate": "compile:cargo", "passed": true, "duration_ms": 4200},
  "parent": "0x9f8e7d6c...",
  "timestamp": "2026-04-10T14:30:05Z"
}
```

### 4.3 Artifact Store

Content-addressed gate artifacts: compile output, test output, diff analysis
results. Each artifact is retrievable by its BLAKE3 hash. The store's
append-only semantics guarantee that artifacts referenced in the chain still
exist.

### 4.4 Efficiency Events

**Path**: `.roko/learn/efficiency.jsonl`

Per-turn efficiency data: token counts, tool call metadata, gate timing, cost
estimates.

---

## 5. Why Forensic Replay Matters

### 5.1 Regulatory Compliance

| Regulation | Requirement | How replay satisfies it |
|---|---|---|
| EU AI Act Art. 14 | Human oversight of high-risk AI | Complete action trace, gate verdicts as checkpoints |
| SEC/CFTC | Algorithmic trading audit trail | Content-addressed chain from decision to execution |
| HIPAA | Access audit for health data | Every file read/write by every agent, timestamped |
| SOX | Financial system change controls | Immutable verification artifacts for every code change |

### 5.2 Debugging Complex Failures

When an agent produces a subtle bug that passes all gates:

- Trace back through reasoning to find where the agent went wrong
- Identify which gate should have caught the issue (gap analysis)
- Determine whether tool calls were productive or wasteful

### 5.3 Learning System Validation

The learning system (skills, routing, experiments) makes decisions based on
historical data. Forensic replay verifies that those decisions were based on
accurate data:

- Did the gate verdicts that trained the router correspond to correct outcomes?
- Did the skill extraction process correctly identify the tool call patterns
  that led to success?

---

## 6. Causal Analysis

### 6.1 Root Cause Analysis

When a task fails, trace backward through the causal chain:

```
1. Which gate failed?     -> Rung 2: Test
2. What was the failure?  -> "assertion failed: expected 200, got 404"
3. Which edit caused it?  -> Turn 3, Edit to routes.rs
4. What was the reasoning? -> "I moved the route handler to a new module"
5. Was reasoning correct? -> "Yes, but forgot to update route registration"
```

This chain from verdict -> edit -> reasoning -> root cause is what "forensic"
means. Not just "what happened" but "why it happened."

### 6.2 What-If Analysis

Replay the task with the same inputs but a different model. Compare verdicts
and outcomes. This powers the shadow testing loop (Loop 12 in the evaluation
lifecycle).

### 6.3 Gap Analysis

When a bug escapes all gates:

```
Bug: off-by-one error in pagination
Escaped gates: Compile (expected), Lint (expected), Test (gap!)

Analysis:
  - Existing tests do not cover pagination edge cases
  - Generated tests (Rung 4) would have caught this if prompted
    with "test boundary conditions for pagination"

Recommendation: Add pagination boundary test to the
  GeneratedTestGate's standard generation templates
```

---

## 7. Immutability Guarantees

Four layers of immutability protect the forensic chain:

| Layer | Mechanism |
|---|---|
| Content-addressed artifacts | Changing any byte changes the BLAKE3 hash |
| Append-only logs | Episodes and signals are appended, never modified |
| ArtifactStore immutability | No delete/update operations in public API |
| Hash chain (future) | Each signal hash incorporates parent hash |

Together, these make the chain tamper-evident. Inserting, removing, or
reordering elements breaks verifiable invariants.

---

## 8. Performance and Storage

### Per-Execution Overhead

| Operation | Overhead | When |
|---|---|---|
| BLAKE3 hashing | < 1ms per artifact | Every gate run |
| Episode logging | < 1ms per turn | Every agent turn |
| Signal logging | < 1ms per signal | Every signal write |
| Artifact storage | O(artifact_size) | Every gate run |
| Chain verification | O(chain_length) | On-demand (replay) |

Total per-execution overhead: < 5ms.

### Storage Cost

For a typical plan execution (10 tasks, 3 attempts each, 5 gate runs per
attempt):

| Component | Size |
|---|---|
| Episodes | ~150 entries, ~500 KB |
| Signals | ~150 entries, ~300 KB |
| Artifacts | ~150 artifacts, ~5 MB |
| **Total** | **~6 MB per plan** |

At continuous operation, approximately 2 GB per year -- easily manageable with
periodic GC of old artifacts via `roko cache prune`.

---

## 9. Pre-Certified Agent Templates

A practical application: pre-certified templates for regulated industries.
A template is a set of:

1. System prompt sections (versioned, hashed)
2. Gate pipeline configuration (which rungs, which gates)
3. Verification criteria (generated test templates)
4. Audit trail requirements (which data must be logged)

Organizations deploy these knowing that every action will be logged,
every verification outcome is content-addressed, and the complete chain
from input to output is reconstructable. Regulatory auditors can
independently verify the chain.

---

## 10. The roko replay Command

The `roko replay <hash>` CLI command walks the signal DAG from a given hash,
reconstructing the full verification history. Combined with `roko diagnose
<plan-id>`, operators can trace any past execution to its root cause:

```bash
# Replay a specific signal chain
cargo run -p roko-cli -- replay 0xab1c2d3e

# Diagnose a failed plan execution
cargo run -p roko-cli -- diagnose plan-42
```

---

## 11. Relationship to Other Components

| Component | Relationship |
|---|---|
| ArtifactStore | Stores immutable gate artifacts (BLAKE3 addressed) |
| Episode Logger | Records agent turns with tool call metadata |
| Signal Log | Records engrams and verdict signals |
| GateRatchet | Ratchet state at each point in time |
| AdaptiveThresholds | Threshold state at each point in time |
| Efficiency Events | Per-turn cost and timing data |

---

## 12. Test criteria

| Test | Property |
|---|---|
| `causal_chain_builds_from_turns` | TurnRecords reconstructed in order |
| `artifact_metadata_integrity_check` | Hash recomputation matches stored hash |
| `chain_detects_tampered_artifact` | Modified content -> integrity_verified = false |
| `empty_task_produces_empty_chain` | No turns -> empty chain, still valid |
| `verdict_artifact_cross_reference` | Each verdict traces to an artifact hash |
| `builder_handles_missing_artifacts` | Missing artifact -> chain builds with gap note |
| `chain_serialization_roundtrip` | CausalChain survives serde JSON roundtrip |
