# Checkpoint and Resume

> Depth file for [04-EXECUTION.md](../../04-EXECUTION.md) section 14.
> Preserves and updates content from v1 `01-orchestration/09-snapshot-recovery.md`.

---

## Overview

Long-running plan sessions must survive crashes, operator interruptions,
and machine restarts. The checkpoint-and-resume system ensures no completed
work is repeated. It combines durable snapshots with an Activity replay log
so that deterministic nodes are re-derived and non-deterministic nodes are
substituted from recordings.

**Source:** `crates/roko-graph/src/snapshot.rs`, `crates/roko-graph/src/replay.rs`

---

## Workflow/Activity Split

Every graph node is classified as one of two execution classes:

- **Workflow**: deterministic computation (routing, scoring, composition,
  context lookup). Re-derived from inputs on resume. Never recorded. These
  nodes always produce the same outputs given the same inputs.

- **Activity**: non-deterministic computation (LLM calls, tool use, gate
  execution). Recorded to JSONL after execution. Replayed from recording
  on resume. These nodes interact with external systems and cannot be
  reproduced deterministically.

This split is the foundation of the resume system. Workflow nodes are
cheap to re-execute (knowledge queries, playbook lookups, prompt assembly),
so recording them would waste disk I/O. Activity nodes are expensive and
non-reproducible, so their outputs must be captured.

In the production topology, the six enricher nodes (knowledge, episodes,
playbook, modulation, safety, experiment) and the compose node are
Workflow. The task executor and gate nodes are Activity. For a 10-task
plan with 110 nodes, only 20 Activity outputs are recorded.

---

## GraphSnapshotV2

The primary checkpoint format captures everything needed to resume:

```rust
pub struct GraphSnapshotV2 {
    pub schema_version:            u8,        // always 2
    pub graph_name:                String,
    pub graph_id:                  String,
    pub graph_fingerprint:         String,    // BLAKE3 fingerprint
    pub node_statuses:             HashMap<String, SerializableNodeStatus>,
    pub node_outputs:              HashMap<String, Vec<SerializableSignal>>,
    pub tick_count:                u64,
    pub budget_spent_micro_usd:    u64,
    pub budget_reserved_micro_usd: u64,
    pub last_event_seq:            u64,
    pub created_at_ms:             i64,
    pub policy:                    GraphPolicy,
    pub extension_ledger:          BTreeMap<String, serde_json::Value>,
    pub receipt_state:             Option<ReceiptState>,
}
```

### Schema versioning

- **v1** (implicit): the original unversioned snapshot. Missing fields
  default to zero/empty via `#[serde(default)]`.
- **v2** (current): adds `schema_version`, `graph_fingerprint`, budget
  tracking fields, and `last_event_seq`.

A v1 snapshot on disk deserializes into `GraphSnapshotV2` with serde
defaults. No explicit migration code is needed because all new fields
carry `#[serde(default)]` attributes.

### Graph fingerprint

The `graph_fingerprint` is a BLAKE3 hash of the execution-relevant graph
definition (node IDs, cell types, edges, policy). It ensures resume is
rejected after graph definition drift.

This catches a class of bugs where:
- `tasks.toml` is edited between crash and resume
- A new task is added to the plan
- An existing task's dependencies change

In all these cases, the fingerprint will not match and the engine returns
`GraphError::FingerprintMismatch` rather than silently executing against
a stale snapshot.

---

## Persistence Paths

| What | Path |
|---|---|
| Graph checkpoints | `.roko/state/graph/` |
| Legacy Runner-v2 snapshot | `.roko/state/state-snapshot.json` |
| Activity recordings | `.roko/state/graph/<run-id>/activities/*.jsonl` |
| Episode log | `.roko/episodes.jsonl` |
| Signal log | `.roko/engrams.jsonl` |

---

## Atomic Writes

Snapshots use the write-fsync-rename pattern to prevent corruption:

1. Write to `<path>.tmp`
2. `fsync` the temp file (ensures data reaches disk, not just OS buffer)
3. Rename `<path>.tmp` to `<path>` (atomic on POSIX filesystems)

A crash during steps 1-2 leaves the original intact. A crash during step 3
produces either the old or new snapshot, never a partial write. This
guarantee relies on POSIX `rename(2)` atomicity within a single filesystem.

---

## Resume Procedure

When `roko plan run <dir> --resume-plan` is invoked:

```
1. Load GraphSnapshotV2 from .roko/state/graph/

2. Validate graph_fingerprint matches the current graph definition.
   Mismatch -> reject resume with GraphError::FingerprintMismatch.

3. Restore node statuses:
   - Complete   -> skip (Activity output loaded from recording)
   - Running    -> handled by ReconcileAction (default: reset to Pending)
   - Pending    -> execute normally
   - Failed     -> skip (terminal)
   - Skipped    -> skip (terminal)

4. Restore budget state:
   - spent microdollars (AtomicU64)
   - reserved microdollars (AtomicU64)

5. Load ActivityReplayer from the run-scoped JSONL file.

6. Resume execution from the first non-complete node in topological order.
```

### ReconcileAction

The `ReconcileAction` type lets extension owners decide how to handle
`Running` status from a restored snapshot. A node in `Running` status
means it was mid-execution when the crash occurred.

```rust
pub enum ReconcileAction {
    ResetToPending,  // re-execute the node
    MarkFailed,      // treat as failed
    MarkComplete,    // treat as complete (with provided outputs)
}
```

The engine delegates to the registered owner rather than blindly resetting
to `Pending`. For unowned nodes, the default is `ResetToPending`, which
means the node will re-execute. This is safe because Activity outputs are
recorded after completion -- a `Running` Activity has no recorded output
and will simply re-run.

### Worst case after crash

The worst case is a single duplicate LLM call. This happens when:

1. An Activity node completes and produces output.
2. The output is not flushed to the recording before the crash.
3. On resume, the node has `Running` status and no recorded output.
4. The node re-executes, producing a (possibly different) output.

The system prefers availability over exactly-once semantics: it resumes
and re-executes rather than refusing to continue when the recording is
incomplete.

---

## Hot Graph Checkpoints

For Hot Graphs (persistent cognitive loops that tick repeatedly), the
`HotGraphCheckpointManifest` commits state after each successful tick:

```rust
pub struct HotGraphCheckpointManifest {
    pub tick: u64,
    pub activity_log_path: PathBuf,
    pub budget_checkpoint: BudgetCheckpoint,
    pub node_outputs: HashMap<String, Vec<SerializableSignal>>,
}
```

After each tick:
1. Activity outputs are appended to the run-scoped JSONL log.
2. The manifest is written atomically (write-fsync-rename).
3. Cumulative budget is persisted.

A crash between Activity write and manifest commit replays that Activity
from the run-scoped log on next startup. The manifest provides the
authoritative tick count; Activities beyond the manifest tick are replayed
rather than re-executed.

---

## Extension Ledger

Host features can register namespaced extensions in the snapshot without
editing the graph-core schema:

```rust
pub extension_ledger: BTreeMap<String, serde_json::Value>
```

Extension map keys follow the format `<namespace>@<schema_version>`.
Examples: `"cost-sidecar@1"`, `"delivery-state@2"`.

On restore:
- Unknown **required** extensions fail the restore. This prevents resuming
  with missing host features that the snapshot depends on.
- Unknown **optional** extensions round-trip unchanged as opaque JSON.
  This allows forward compatibility when new features are added.

---

## Receipt State Machine

The `ReceiptState` tracks the forward-only progress of terminal receipt
emission:

```rust
pub enum ReceiptState {
    Prepared,   // receipt constructed but not committed
    Committed,  // receipt committed to durable storage
    Settled,    // receipt delivered to all consumers
}
```

Transitions are forward-only: `Prepared -> Committed -> Settled`.
Repeating the current transition is idempotent success. Reverse or skipped
transitions fail closed. This state machine is persisted in the snapshot
so that receipt delivery can resume after a crash during the settlement
phase.

---

## Budget State Restoration

Budget state restoration is strict. On resume:

1. `budget_spent_micro_usd` is loaded into the `BudgetTracker`'s atomic
   counter.
2. `budget_reserved_micro_usd` is added back as reserved (uncommitted)
   cost.
3. If the cost sidecar file is missing, corrupt, or has a mismatched run
   ID, the engine **fails closed** -- it refuses to resume rather than
   risking over-spend.

This strict behavior prevents a scenario where a crash during cost
recording causes the resumed run to believe it has more budget remaining
than it actually does.

---

## CLI Usage

```bash
# Execute a plan
cargo run -p roko-cli -- plan run plans/<dir>

# Resume a crashed/interrupted plan
cargo run -p roko-cli -- plan run plans/<dir> --resume-plan

# Resume a specific run by ID
cargo run -p roko-cli -- resume [run-id]

# Inspect snapshot state
jq '.node_statuses | to_entries[] | {node: .key, status: .value}' \
  .roko/state/graph/<run-id>/snapshot.json
```
