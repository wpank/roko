# Graph Fingerprinting and Resume

> BLAKE3 fingerprinting of graph definitions for Activity resume, the
> GraphSnapshotV2 schema, the extension ledger, the receipt state machine,
> reconciliation callbacks, and Hot Graph checkpoint artifacts.

---

## Source Files

| File | What |
|---|---|
| `crates/roko-graph/src/fingerprint.rs` | graph_execution_fingerprint |
| `crates/roko-graph/src/snapshot.rs` | GraphSnapshotV2, extensions, receipts, reconciliation |
| `crates/roko-graph/src/hot.rs` | HotGraphCheckpointManifest, start_hot_resumable |

---

## BLAKE3 Graph Fingerprinting

`graph_execution_fingerprint()` computes a stable identity over the
execution-relevant parts of a Graph definition:

```rust
pub fn graph_execution_fingerprint(graph: &Graph) -> Result<String, serde_json::Error>
```

### Algorithm

1. **Sort nodes** by `node.id` (lexicographic).
2. **Sort edges** by `(from, to, condition)` -- condition is sorted by its
   `Debug` representation for determinism.
3. **Collect metadata labels** into a `BTreeMap` (sorted by key).
4. **Serialize** a `Fingerprint` struct containing:
   - Schema version (currently 1).
   - Graph name, description, version, sorted labels.
   - Full `GraphPolicy`.
   - Sorted nodes (including each node's `config`).
   - Sorted edges (including conditions).
5. **Hash** the JSON bytes with BLAKE3.
6. **Return** the hex-encoded hash string via `ContentHash::of(&encoded).to_hex()`.

### Invariants

- **Insertion-order independence:** Node and edge insertion order do not
  affect the fingerprint. Two structurally identical graphs always produce
  the same fingerprint, regardless of construction order.

- **Sensitivity:** Any change to node config, policy, metadata, edge
  conditions, or graph structure produces a different fingerprint. The
  `fingerprint_is_stable_and_sensitive_to_execution_config` test verifies
  this by comparing graphs with config values `1` and `2`.

### Use in Resume

The fingerprint is stored in every snapshot (`GraphSnapshotV2.graph_fingerprint`).
On resume:

1. The engine recomputes the fingerprint from the current graph definition.
2. If the recomputed fingerprint differs from the snapshot's stored
   fingerprint, the engine rejects the resume -- preventing replay after
   graph definition or policy drift.
3. If fingerprints match, Activity outputs from the snapshot are trusted
   and replayed without re-execution.

---

## GraphSnapshotV2

The snapshot captures the minimum state needed for resume:

```rust
pub struct GraphSnapshotV2 {
    pub schema_version:           u8,                    // always 2
    pub graph_name:               String,
    pub graph_id:                 String,
    pub graph_fingerprint:        String,                // BLAKE3 hex
    pub node_statuses:            HashMap<String, SerializableNodeStatus>,
    pub node_outputs:             HashMap<String, Vec<SerializableSignal>>,
    pub tick_count:               u64,
    pub budget_spent_micro_usd:   u64,                   // 1 USD = 1_000_000
    pub budget_reserved_micro_usd: u64,
    pub last_event_seq:           u64,                   // monotonic event counter
    pub created_at_ms:            i64,
    pub policy:                   GraphPolicy,
}
```

### Key Design Choices

- **Only Activity node outputs are stored.** Workflow nodes are re-derived
  from their inputs on resume (they are deterministic by definition). This
  keeps snapshots small.
- **V1 backward compatibility.** V1 snapshots on disk deserialize into V2
  via `#[serde(default)]` on all new fields -- no explicit migration code.
  Missing `graph_fingerprint` defaults to empty string, budget fields to 0.
- **`type GraphSnapshot = GraphSnapshotV2`** -- the alias ensures all
  callers use the current version.

### SerializableNodeStatus

```rust
pub enum SerializableNodeStatus {
    Pending,
    Running,          // delegated to reconciliation owner on resume
    Complete,
    Failed,
    Skipped,
    ConditionSkipped,
}
```

The `Running` status is preserved in snapshots (not reset to `Pending`)
so that registered reconciliation owners can make an informed decision.

---

## Reconciliation

When a snapshot contains `Running` Activity nodes (the process crashed during
execution), the engine delegates to the registered reconciliation owner:

```rust
pub enum ReconcileAction {
    Continue,        // stored state is valid, continue from it
    Reset,           // reset to pending, re-execute
    Fail(String),    // unrecoverable, fail with reason
}
```

The default `reconcile_running_status()` function converts `Running` to
`Reset` and everything else to `Continue`. Host layers that implement
owner-based reconciliation register their own callbacks via
`ExtensionRegistry::register_reconciler()`.

---

## Extension Ledger

Host features register namespaced extensions without editing the graph-core
schema:

```rust
pub struct CheckpointExtension {
    pub namespace:      String,           // e.g. "roko.workspace.attempt"
    pub schema_version: u32,
    pub required:       bool,
    pub fingerprint:    String,           // deterministic content hash
    pub value:          serde_json::Value, // opaque JSON payload
}
```

Extension map keys follow the format `<namespace>@<schema_version>`.

### Registration Rules

- **Idempotent:** Re-registering with the same fingerprint is a no-op.
- **Drift detection:** Re-registering with a different fingerprint returns
  `ExtensionError::FingerprintMismatch`.
- **Unknown required extensions** fail on restore; unknown optional extensions
  round-trip unchanged.

### Known Extension Namespaces (11)

| Constant | Namespace | Required | Owner |
|---|---|---|---|
| `EXT_COST` | `roko.cost@1` | Yes | graph core |
| `EXT_ACTIVITY` | `roko.activity@1` | Yes | graph core |
| `EXT_REPLAN` | `roko.replan@1` | Yes | #252 |
| `EXT_FEEDBACK` | `roko.feedback@1` | Yes | #253 |
| `EXT_DELIVERY` | `roko.delivery@1` | Yes | #254 |
| `EXT_CONTROL` | `roko.control@1` | Yes | #255 |
| `EXT_RUN_CONTEXT` | `roko.run-context@1` | Yes | #327 |
| `EXT_GATE_HISTORY` | `roko.gate-history@1` | No | #250 |
| `EXT_APPROVAL` | `roko.approval@1` | No | #255 |
| `EXT_SAFETY_PROVENANCE` | `roko.safety-provenance@1` | No | #351 |
| `EXT_EXPERIMENT` | `roko.experiment@1` | No | #359 |

---

## Receipt Ledger

The receipt state machine enforces forward-only transitions for idempotent
external side-effects:

```rust
pub enum ReceiptState {
    Prepared = 0,    // receipt created, side-effect not confirmed
    Committed = 1,   // side-effect confirmed, evidence recorded
    Settled = 2,     // fully settled, no further work needed
}
```

### Transition Rules

| From | To | Behavior |
|---|---|---|
| Prepared | Committed | Normal forward transition |
| Prepared | Settled | **Error**: `SkippedTransition` (must go through Committed) |
| Committed | Settled | Normal forward transition |
| Same state | Same state | Idempotent success (no-op) |
| Later state | Earlier state | Idempotent success (no-op, already past) |

### ReceiptLedgerEntry

```rust
pub struct ReceiptLedgerEntry {
    pub idempotency_key: String,         // stable dedup key across restarts
    pub owner:           String,         // subsystem that owns this receipt
    pub correlation_id:  String,         // links to originating request
    pub state:           ReceiptState,
    pub evidence_ref:    Option<String>, // commit OID, URL, etc.
    pub updated_at_ms:   u128,
    pub last_error:      Option<String>,
}
```

### Operations

| Function | Purpose |
|---|---|
| `prepare_receipt(...)` | Create a new Prepared receipt; idempotent if key exists |
| `commit_receipt(...)` | Transition to Committed with evidence; idempotent |
| `settle_receipt(...)` | Transition to Settled; fails if still Prepared |

---

## Hot Graph Checkpoints

Hot Graphs (tick-driven, resident) persist three artifacts after each
successful tick:

### 1. Checkpoint Manifest (`checkpoint.json`)

Contains the graph fingerprint, tick count, budget checkpoint, and per-node
tick state. On resume, the manifest is loaded and the fingerprint is verified.

### 2. Activity Log (`activities.jsonl`)

Activity node outputs appended after each tick. On resume, the log is replayed
to restore Activity outputs without re-execution.

### 3. Budget Checkpoint

`BudgetCheckpoint` with `tokens_used`, `cost_microdollars`, `elapsed_ms`,
and per-node cost breakdown. Restored via `BudgetEnforcer::restore_checkpoint()`.

### Loop Levels

Named temporal tiers for nested hot graphs:

| Level | Name | Default Interval |
|---|---|---|
| Gamma | Perception, reflex | 250 ms |
| Theta | Planning, deliberation | 10,000 ms |
| Delta | Learning, consolidation | 60,000 ms |

---

## Resume Flow

```
1. Load snapshot from .roko/state/graph/<fingerprint>.json
2. Recompute graph fingerprint from current definition
3. Reject if fingerprints differ (graph/policy drift)
4. For each node in snapshot:
   - Complete/Failed/Skipped/ConditionSkipped: preserve status
   - Running: delegate to reconciliation owner (default: Reset to Pending)
   - Pending: keep as Pending
5. Restore budget counters from snapshot
6. Replay Activity outputs (skip re-execution)
7. Re-derive Workflow outputs (deterministic)
8. Continue execution from the first Pending node
```

---

## Verification

```bash
cargo test -p roko-graph --lib fingerprint::tests
cargo test -p roko-graph --lib snapshot::tests
```

Test coverage for fingerprinting:
- Fingerprint is stable (same graph produces same hash).
- Fingerprint is sensitive to execution config changes.

Test coverage for snapshots:
- V2 serde roundtrip preserves all fields.
- V1 backward compatibility (missing new fields default).
- Type alias `GraphSnapshot = GraphSnapshotV2`.
- Reconciliation: Running returns Reset, Complete/Pending return Continue.
- Extension registration is idempotent with same fingerprint.
- Extension rejects fingerprint mismatch.
- Extension opaque value round-trips unchanged.
- Novel namespace registers without schema edit.
- Receipt lifecycle: Prepared -> Committed -> Settled.
- Receipt idempotent on duplicate prepare/commit/settle.
- Receipt settle fails if still Prepared (skipped transition).
- Receipt state ordering: Prepared < Committed < Settled.
- Registry validates required extensions; allows unknown optional.
- Reconciler callback invoked correctly.
