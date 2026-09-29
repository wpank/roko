# Snapshot Recovery

> Depth file for [04-EXECUTION.md](../../04-EXECUTION.md) section 12.
> Preserves and updates content from v1 `01-orchestration/09-snapshot-recovery.md`.

---

## Overview

The snapshot recovery system provides two complementary mechanisms for
surviving crashes: graph snapshots for point-in-time state capture, and
activity replay for deterministic reconstruction of non-reproducible
outputs. Together they ensure that long-running plan sessions (hours or
days) do not lose progress.

**Source:** `crates/roko-graph/src/snapshot.rs`, `crates/roko-graph/src/replay.rs`

---

## Graph Snapshots

### GraphSnapshotV2

The primary snapshot format:

```rust
pub struct GraphSnapshotV2 {
    pub schema_version:            u8,
    pub graph_name:                String,
    pub graph_id:                  String,
    pub graph_fingerprint:         String,
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

A snapshot captures everything the engine needs to resume: per-node
status, Activity node outputs, budget state, and the graph fingerprint
for drift detection.

### Schema versioning

V1 snapshots (the original unversioned format) deserialize into
`GraphSnapshotV2` via serde defaults -- no migration code needed.
All new fields have `#[serde(default)]`. This means the engine can
read snapshots from any prior version without conversion.

### Graph fingerprint

A BLAKE3 hash of the execution-relevant graph definition (node IDs, cell
types, edges, policy). Resume is rejected if the fingerprint does not
match the current graph definition. This catches cases where `tasks.toml`
changed between crash and resume, preventing execution against a stale
snapshot that no longer matches the plan.

### Atomic writes

Snapshots use write-fsync-rename:

1. Write to `<path>.tmp`.
2. `fsync` the temp file (ensures bytes reach persistent storage).
3. Rename `<path>.tmp` to `<path>` (atomic on POSIX).

A crash during steps 1-2 leaves the original intact. Step 3 is atomic:
either the old or new snapshot is visible, never a partial write. This
pattern is borrowed from database systems (PostgreSQL WAL, SQLite).

---

## Activity Replay

### Workflow/Activity split

Every node is classified as Workflow (deterministic) or Activity
(non-deterministic). Only Activity outputs are recorded. On resume,
Workflow outputs are re-derived from inputs; Activity outputs are
loaded from recordings.

This split exploits the fact that enrichment, composition, and boundary
nodes are pure functions of their inputs. Re-deriving them is cheap
(sub-millisecond). LLM calls and gate invocations are expensive and
non-reproducible, so they must be recorded.

### ActivityRecorder

```rust
pub struct ActivityRecorder {
    run_id: String,
    path: PathBuf,
    writer: BufWriter<File>,
}
```

After each Activity node execution, its output is appended to a JSONL
file as a `RecordEntry`:

```rust
pub struct RecordEntry {
    pub graph_id: String,
    pub run_id: String,
    pub node_id: String,
    pub tick: u64,
    pub signals: Vec<Signal>,
}
```

The file is flushed after every write so partial runs are recoverable.
The JSONL format ensures that each line is self-contained -- a crash
mid-write at most corrupts the last line, leaving all prior entries
intact.

### ActivityReplayer

```rust
pub struct ActivityReplayer {
    entries: HashMap<String, Vec<Signal>>,
    replayed: HashSet<String>,
}
```

On resume, the replayer loads all recorded entries and creates a lookup
map keyed by `(graph_id, node_id, tick)`. When the engine encounters an
Activity node that has a recorded output, it substitutes the recording
instead of re-executing. This avoids duplicate LLM calls.

The `replayed` set tracks which entries have been consumed. After
execution completes, any entries in the map but not in `replayed` are
orphaned recordings (from nodes that were condition-skipped on this run).

### Create vs. create_fresh

- `ActivityRecorder::create()` opens in append mode -- use when resuming
  a run and appending to an existing checkpoint.
- `ActivityRecorder::create_fresh()` truncates -- use for a new run.

---

## Recovery Procedure

### From snapshot

```
1. Load GraphSnapshotV2 from .roko/state/graph/<run-id>/
2. Validate graph_fingerprint
3. Restore node statuses
4. Load ActivityReplayer from the run-scoped JSONL file
5. Resume execution from first non-complete node
```

### Status restoration

| Stored status | Restored as | Rationale |
|---|---|---|
| `Complete` | `Complete` | Output loaded from recording |
| `Running` | Delegated to `ReconcileAction` | Was interrupted mid-execution |
| `Pending` | `Pending` | Not yet started |
| `Failed` | `Failed` | Terminal |
| `Skipped` | `Skipped` | Terminal |
| `ConditionSkipped` | `ConditionSkipped` | Terminal no-op |

For `Running` nodes, the engine delegates to the registered
reconciliation owner via `ReconcileAction`:

```rust
pub enum ReconcileAction {
    ResetToPending,   // re-execute the node
    MarkFailed,       // treat as failed
    MarkComplete,     // treat as complete (with provided outputs)
}
```

The default for unowned nodes is `ResetToPending`. This is safe because
Activity outputs are recorded after successful completion -- a `Running`
Activity with no recording must be re-executed.

### Budget restoration

Budget state is restored from the snapshot's `budget_spent_micro_usd`
and `budget_reserved_micro_usd` fields. The `BudgetTracker` atomics
are loaded from these values before execution resumes. Missing or
corrupt cost state fails closed -- the engine refuses to resume.

---

## Hot Graph Checkpoints

For Hot Graphs (persistent cognitive loops), the
`HotGraphCheckpointManifest` provides more frequent checkpointing:

```rust
pub struct HotGraphCheckpointManifest {
    pub tick: u64,
    pub activity_log_path: PathBuf,
    pub budget_checkpoint: BudgetCheckpoint,
    pub node_outputs: HashMap<String, Vec<SerializableSignal>>,
}
```

After each successful tick:
1. Activity outputs are appended to the run-scoped JSONL.
2. The manifest is written atomically.
3. Cumulative budget is persisted.

A crash between Activity write and manifest commit replays that Activity
from the log. The manifest provides the authoritative tick count.

---

## Extension Ledger

Host features register namespaced data in the snapshot:

```rust
pub extension_ledger: BTreeMap<String, serde_json::Value>
```

Keys follow `<namespace>@<schema_version>`. Unknown required extensions
fail on restore. Unknown optional extensions round-trip unchanged as
opaque JSON values. This allows the snapshot format to be extended by
host layers without modifying the graph-core schema.

---

## Persistence Paths

| What | Path |
|---|---|
| Graph checkpoints | `.roko/state/graph/` |
| Activity recordings | `.roko/state/graph/<run-id>/activities/*.jsonl` |
| Legacy Runner-v2 snapshot | `.roko/state/state-snapshot.json` |
| Episode log | `.roko/episodes.jsonl` |
| Signal log | `.roko/engrams.jsonl` |

---

## Worst-Case Analysis

The worst case after a crash is a single duplicate LLM call. This happens
when:

1. An Activity node completes and produces output.
2. The recording is not flushed before the crash.
3. On resume, the node has `Running` status and no recorded output.
4. The node re-executes, producing a (possibly different) output.

The system prefers availability (resume and re-execute) over exactly-once
semantics (refuse to resume without perfect recording). In practice, the
immediate flush after each write makes this scenario unlikely -- it
requires a crash between the cell returning and the writer flushing.

---

## CLI Usage

```bash
# Resume from last checkpoint
cargo run -p roko-cli -- plan run plans/<dir> --resume-plan

# Resume a specific run ID
cargo run -p roko-cli -- resume <run-id>

# Inspect snapshot
jq '.' .roko/state/graph/<run-id>/snapshot.json

# Inspect activity log
jq -c '.' .roko/state/graph/<run-id>/activities/main.jsonl | head -5
```
