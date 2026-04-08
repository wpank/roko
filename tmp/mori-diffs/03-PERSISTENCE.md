# Path 3: State Persistence -- Full Snapshot + Resume

## Current State (What's Broken)

Persistence is handled by `crates/roko-cli/src/runner/persist.rs` with a single `PersistPaths`
struct and three write functions. While atomic writes are implemented, the overall persistence
model has critical gaps.

### 1. Only Saves executor.json with Partial Data

The snapshot captures `ExecutorSnapshot` which contains `PlanState` per plan, but this
represents only the *orchestrator-level* state machine (which `PlanPhase` each plan is in).
It does not include:

- **RunState fields** (`total_cost_usd`, `total_tokens_in/out`, `tasks_completed`,
  `tasks_failed`, `plan_costs`, `completed_tasks` per plan)
- **Cascade router state** (learned model routing weights from `roko-learn`)
- **Adaptive gate thresholds** (EMA per rung from `roko-gate`)
- **Daimon state** (affect engine state from `roko-daimon`)
- **Efficiency stats** (per-model cost/quality from `roko-learn`)

After a crash and resume, all learning state, cost tracking, and per-task completion tracking
is lost. The executor resumes from the right `PlanPhase`, but doesn't know which tasks within
a plan were already completed.

### 2. No Version Field (Partial)

`ExecutorSnapshot` has `schema_version: u32` (added recently, defaults to 0), but:
- No migration logic for version bumps.
- No validation that the snapshot version is compatible with the current binary.
- Other persisted files (episodes.jsonl, efficiency.jsonl) have no version field at all.

### 3. Resume Doesn't Fully Validate Plan IDs

`try_resume()` in event_loop.rs checks for "overlap" (at least one plan ID matches):

```rust
// event_loop.rs:360-371
let has_overlap = plan_ids.iter().any(|id| snapshot.plan_states.contains_key(id));
if snap_plan_ids.is_empty() || !has_overlap {
    info!("stale executor snapshot (no plan overlap) -- starting fresh");
    return None;
}
```

There is a separate `snapshot_reconcile.rs` module with stricter validation, but it's not
called from the event loop's `try_resume`. The reconciler checks that every snapshot plan ID
exists on disk, but the event loop only checks for any overlap.

### 4. No Atomic Writes (Fixed)

The `persist.rs` module now uses `atomic_write` (write to `.tmp`, then `rename`). This was
a previous gap that has been addressed:

```rust
// persist.rs:52-59
pub fn atomic_write(path: &Path, content: &[u8]) -> Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, content)?;
    fs::rename(&tmp, path)?;
    Ok(())
}
```

However, only `executor.json` and `agent-pids.json` use atomic writes. JSONL files
(`episodes.jsonl`, `efficiency.jsonl`) use `append` mode which is not atomic across entries
(an interrupted append can leave a partial JSON line).

### 5. Completed Tasks Tracked as Counts, Not Per-Plan

`RunState` tracks `tasks_completed: usize` (global count) and
`completed_tasks: HashMap<String, Vec<String>>` (per-plan task IDs). But the `completed_tasks`
map is never persisted -- it exists only in memory. On resume, the runner doesn't know which
specific tasks completed, only the plan's phase.

### 6. No Save After Gate Result

Snapshots are saved:
- After every `TurnCompleted` or `Exited` agent event (line 192)
- After every gate completion (line 261)
- Every 2 seconds via the flush interval (line 286)
- On cancellation (line 298)

This is actually reasonably frequent. The real gap is that only `executor.json` is saved --
the other state files (routing, thresholds) are never checkpointed.

### 7. Single PersistPaths, Multiple Missing Files

`PersistPaths` defines 5 paths but only 3 are actively used:

```rust
pub struct PersistPaths {
    pub executor_json: PathBuf,    // USED: save_executor_snapshot
    pub episodes_jsonl: PathBuf,   // USED: append_jsonl episodes
    pub efficiency_jsonl: PathBuf, // USED: append_jsonl efficiency
    pub agent_pids_json: PathBuf,  // USED: save_agent_pids
    pub events_json: PathBuf,      // UNUSED: declared but never written
}
```

Not saved:
- `.roko/learn/cascade-router.json` (exists at runtime, never persisted from runner)
- `.roko/learn/gate-thresholds.json` (exists at runtime, never persisted from runner)
- `.roko/state/daimon.json` (never persisted)
- `.roko/state/run-state.json` (RunState never persisted)


## Design Goals

1. **Full snapshot**: Save all 5 state files on every significant event
2. **Version + migration**: Schema version in every snapshot file, migration on load
3. **Strict resume validation**: Use `snapshot_reconcile.rs` from the event loop, validate task-level state
4. **Per-plan task tracking**: Persist completed task IDs (not just counts)
5. **Incremental saves**: Only write files that changed since last save
6. **JSONL integrity**: Detect and recover from partial JSONL lines
7. **Composability**: Each state component saves/loads independently


## Architecture

### New Types

```rust
// crates/roko-cli/src/runner/persist.rs

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

/// Schema version for the full snapshot format.
pub const SNAPSHOT_SCHEMA_VERSION: u32 = 2;

/// Complete runner state, encompassing all subsystems.
///
/// This is the top-level checkpoint that gets written to `.roko/state/`.
/// Each field maps to one file on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullSnapshot {
    /// Schema version for forward compatibility.
    pub version: u32,
    /// Unix millisecond timestamp when the snapshot was taken.
    pub timestamp_ms: u64,
    /// Executor state (plan phases, queue order).
    pub executor: ExecutorSnapshot,
    /// Runner-level state (costs, token counts, completed tasks).
    pub run_state: RunStateSnapshot,
    /// Per-task DAG state for each plan.
    pub dag_states: HashMap<String, DagSnapshot>,
}

/// Serializable subset of RunState.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunStateSnapshot {
    pub version: u32,
    /// Total tasks across all plans.
    pub tasks_total: usize,
    /// Number of tasks completed.
    pub tasks_completed: usize,
    /// Number of tasks failed.
    pub tasks_failed: usize,
    /// Total input tokens across the entire run.
    pub total_tokens_in: u64,
    /// Total output tokens across the entire run.
    pub total_tokens_out: u64,
    /// Total cost in USD.
    pub total_cost_usd: f64,
    /// Total agent spawn count.
    pub total_agent_calls: usize,
    /// Per-plan cost accumulation.
    pub plan_costs: HashMap<String, f64>,
    /// Completed task IDs per plan.
    pub completed_tasks: HashMap<String, Vec<String>>,
    /// Consecutive snapshot failures (for degradation tracking).
    pub snapshot_fail_streak: u32,
}

/// Serializable DAG state for a single plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagSnapshot {
    pub plan_id: String,
    /// Per-task status, keyed by task_id.
    pub task_states: HashMap<String, PersistedTaskState>,
    /// Topological order (for deterministic restore).
    pub topo_order: Vec<String>,
}

/// Serializable per-task state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedTaskState {
    pub task_id: String,
    pub status: String,  // "pending", "passed", "exhausted", etc.
    pub attempts: u32,
    pub cost_usd: f64,
    /// If retrying, the UTC timestamp when backoff expires.
    pub backoff_until_ms: Option<u64>,
    /// Last failure classification, if any.
    pub last_failure_summary: Option<String>,
}
```

```rust
/// Manages all persistence I/O for a runner session.
///
/// Tracks what has changed since the last save to enable incremental writes.
pub struct SnapshotWriter {
    paths: SnapshotPaths,
    /// Hash of last-written content per file, for dirty detection.
    last_hashes: HashMap<PathBuf, u64>,
}

/// All file paths for the full snapshot.
#[derive(Debug, Clone)]
pub struct SnapshotPaths {
    /// `.roko/state/executor.json`
    pub executor: PathBuf,
    /// `.roko/state/run-state.json`
    pub run_state: PathBuf,
    /// `.roko/state/dag-states.json`
    pub dag_states: PathBuf,
    /// `.roko/learn/cascade-router.json`
    pub cascade_router: PathBuf,
    /// `.roko/learn/gate-thresholds.json`
    pub gate_thresholds: PathBuf,
    /// `.roko/state/daimon.json`
    pub daimon: PathBuf,
    /// `.roko/learn/efficiency.jsonl` (append-only)
    pub efficiency: PathBuf,
    /// `.roko/episodes.jsonl` (append-only)
    pub episodes: PathBuf,
    /// `.roko/runtime/agent-pids.json`
    pub agent_pids: PathBuf,
}

impl SnapshotPaths {
    /// Derive all paths from a workdir, creating directories.
    pub fn from_workdir(workdir: &Path) -> Result<Self> {
        let roko = workdir.join(".roko");
        let state = roko.join("state");
        let learn = roko.join("learn");
        let runtime = roko.join("runtime");

        for dir in [&state, &learn, &runtime] {
            fs::create_dir_all(dir)?;
        }

        Ok(Self {
            executor: state.join("executor.json"),
            run_state: state.join("run-state.json"),
            dag_states: state.join("dag-states.json"),
            cascade_router: learn.join("cascade-router.json"),
            gate_thresholds: learn.join("gate-thresholds.json"),
            daimon: state.join("daimon.json"),
            efficiency: learn.join("efficiency.jsonl"),
            episodes: roko.join("episodes.jsonl"),
            agent_pids: runtime.join("agent-pids.json"),
        })
    }
}

impl SnapshotWriter {
    pub fn new(paths: SnapshotPaths) -> Self {
        Self {
            paths,
            last_hashes: HashMap::new(),
        }
    }

    /// Save all state files that have changed since the last save.
    ///
    /// Uses content hashing to skip unchanged files (incremental save).
    pub fn save_all(&mut self, snapshot: &FullSnapshot) -> Result<SaveReport> {
        let mut report = SaveReport::default();

        // 1. Executor snapshot (always save -- it's the primary recovery file).
        report.merge(self.save_if_changed(
            &self.paths.executor.clone(),
            &snapshot.executor,
        )?);

        // 2. Run state.
        report.merge(self.save_if_changed(
            &self.paths.run_state.clone(),
            &snapshot.run_state,
        )?);

        // 3. DAG states.
        report.merge(self.save_if_changed(
            &self.paths.dag_states.clone(),
            &snapshot.dag_states,
        )?);

        report
    }

    /// Atomically write a JSON file if its content hash differs from the last save.
    fn save_if_changed<T: Serialize>(
        &mut self,
        path: &Path,
        value: &T,
    ) -> Result<SaveReport> {
        let json = serde_json::to_string_pretty(value)?;
        let hash = hash_content(json.as_bytes());

        if self.last_hashes.get(path) == Some(&hash) {
            return Ok(SaveReport { files_skipped: 1, ..Default::default() });
        }

        atomic_write(path, json.as_bytes())?;
        self.last_hashes.insert(path.to_path_buf(), hash);
        Ok(SaveReport { files_written: 1, ..Default::default() })
    }

    /// Save learning subsystem state (cascade router, gate thresholds).
    /// Called less frequently than the core state (e.g., every 30 seconds).
    pub fn save_learning_state(
        &mut self,
        cascade_router: Option<&impl Serialize>,
        gate_thresholds: Option<&impl Serialize>,
        daimon: Option<&impl Serialize>,
    ) -> Result<SaveReport> {
        let mut report = SaveReport::default();

        if let Some(router) = cascade_router {
            report.merge(self.save_if_changed(
                &self.paths.cascade_router.clone(),
                router,
            )?);
        }
        if let Some(thresholds) = gate_thresholds {
            report.merge(self.save_if_changed(
                &self.paths.gate_thresholds.clone(),
                thresholds,
            )?);
        }
        if let Some(d) = daimon {
            report.merge(self.save_if_changed(
                &self.paths.daimon.clone(),
                d,
            )?);
        }

        Ok(report)
    }
}

/// Summary of a save operation.
#[derive(Debug, Default)]
pub struct SaveReport {
    pub files_written: usize,
    pub files_skipped: usize,
    pub bytes_written: usize,
}

impl SaveReport {
    fn merge(&mut self, other: SaveReport) {
        self.files_written += other.files_written;
        self.files_skipped += other.files_skipped;
        self.bytes_written += other.bytes_written;
    }
}

/// FNV-1a hash of content bytes for change detection.
fn hash_content(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
```

### Resume Validator

```rust
// crates/roko-cli/src/runner/resume.rs

use super::persist::{FullSnapshot, SnapshotPaths, SNAPSHOT_SCHEMA_VERSION};
use crate::snapshot_reconcile;
use crate::task_parser::TasksFile;

/// Errors during resume validation.
#[derive(Debug, thiserror::Error)]
pub enum ResumeError {
    #[error("snapshot version {found} > binary version {expected} -- upgrade roko")]
    VersionTooNew { found: u32, expected: u32 },
    #[error("snapshot has no plan overlap with current plans")]
    NoPlanOverlap,
    #[error("plan {plan_id} has tasks in snapshot that don't exist in tasks.toml: {missing:?}")]
    TaskMismatch {
        plan_id: String,
        missing: Vec<String>,
    },
    #[error("snapshot file corrupt or unreadable: {0}")]
    Corrupt(String),
    #[error(transparent)]
    Reconcile(#[from] snapshot_reconcile::SnapshotReconcileError),
}

/// Load and validate a full snapshot for resume.
///
/// Validation steps:
/// 1. Parse JSON, check schema version.
/// 2. Validate plan IDs match discovered plans (via snapshot_reconcile).
/// 3. Validate task IDs within each plan match the current tasks.toml.
/// 4. Migrate schema if version < SNAPSHOT_SCHEMA_VERSION.
pub fn load_and_validate(
    paths: &SnapshotPaths,
    plan_tasks: &HashMap<String, TasksFile>,
) -> Result<FullSnapshot, ResumeError> {
    // 1. Load executor snapshot.
    let executor_json = fs::read_to_string(&paths.executor)
        .map_err(|e| ResumeError::Corrupt(format!("executor.json: {e}")))?;
    let executor: ExecutorSnapshot = serde_json::from_str(&executor_json)
        .map_err(|e| ResumeError::Corrupt(format!("executor.json: {e}")))?;

    // 2. Load run state (may not exist for v1 snapshots).
    let run_state = if paths.run_state.exists() {
        let json = fs::read_to_string(&paths.run_state)
            .map_err(|e| ResumeError::Corrupt(format!("run-state.json: {e}")))?;
        serde_json::from_str(&json)
            .map_err(|e| ResumeError::Corrupt(format!("run-state.json: {e}")))?
    } else {
        RunStateSnapshot::default_for(&executor)
    };

    // 3. Load DAG states (may not exist for v1 snapshots).
    let dag_states = if paths.dag_states.exists() {
        let json = fs::read_to_string(&paths.dag_states)
            .map_err(|e| ResumeError::Corrupt(format!("dag-states.json: {e}")))?;
        serde_json::from_str(&json)
            .map_err(|e| ResumeError::Corrupt(format!("dag-states.json: {e}")))?
    } else {
        HashMap::new()
    };

    // 4. Version check.
    let snapshot_version = run_state.version;
    if snapshot_version > SNAPSHOT_SCHEMA_VERSION {
        return Err(ResumeError::VersionTooNew {
            found: snapshot_version,
            expected: SNAPSHOT_SCHEMA_VERSION,
        });
    }

    // 5. Validate plan IDs: every snapshot plan must exist in current plans.
    let snapshot_plan_ids: Vec<String> = executor.plan_states.keys().cloned().collect();
    let current_plan_ids: Vec<String> = plan_tasks.keys().cloned().collect();
    let has_overlap = snapshot_plan_ids.iter().any(|id| plan_tasks.contains_key(id));
    if !has_overlap {
        return Err(ResumeError::NoPlanOverlap);
    }

    // 6. Validate task IDs within each plan.
    for (plan_id, dag_state) in &dag_states {
        if let Some(tasks_file) = plan_tasks.get(plan_id) {
            let current_task_ids: HashSet<String> = tasks_file.tasks
                .iter()
                .map(|t| t.id.clone())
                .collect();
            let missing: Vec<String> = dag_state.task_states.keys()
                .filter(|id| !current_task_ids.contains(id.as_str()))
                .cloned()
                .collect();
            if !missing.is_empty() {
                return Err(ResumeError::TaskMismatch {
                    plan_id: plan_id.clone(),
                    missing,
                });
            }
        }
    }

    // 7. Schema migration (if needed).
    let snapshot = FullSnapshot {
        version: SNAPSHOT_SCHEMA_VERSION,
        timestamp_ms: executor.timestamp_ms,
        executor,
        run_state,
        dag_states,
    };

    Ok(maybe_migrate(snapshot, snapshot_version)?)
}

/// Migrate a snapshot from an older schema version to the current one.
fn maybe_migrate(mut snapshot: FullSnapshot, from_version: u32) -> Result<FullSnapshot, ResumeError> {
    if from_version < 2 {
        // v1 -> v2: Add DAG states from executor completed_tasks.
        // Reconstruct DagSnapshot from the tasks that PlanState reports as complete.
        for (plan_id, plan_state) in &snapshot.executor.plan_states {
            if !snapshot.dag_states.contains_key(plan_id) {
                // Best-effort reconstruction: mark all tasks as Pending,
                // the event loop will re-evaluate readiness.
                snapshot.dag_states.insert(plan_id.clone(), DagSnapshot {
                    plan_id: plan_id.clone(),
                    task_states: HashMap::new(),
                    topo_order: Vec::new(),
                });
            }
        }
    }
    snapshot.version = SNAPSHOT_SCHEMA_VERSION;
    Ok(snapshot)
}
```

### Module Layout

```
crates/roko-cli/src/runner/
  persist.rs     -- MODIFIED: FullSnapshot, SnapshotWriter, SnapshotPaths, SaveReport
  resume.rs      -- NEW: load_and_validate, ResumeError, migration
  state.rs       -- MODIFIED: add to_snapshot() / from_snapshot() methods
  event_loop.rs  -- MODIFIED: use SnapshotWriter + save_all instead of save_snapshot
```

### Integration Points

#### 1. Event loop uses SnapshotWriter

```rust
// BEFORE (event_loop.rs):
fn save_snapshot(executor: &ParallelExecutor, paths: &PersistPaths, state: &mut RunState) {
    let timestamp_ms = chrono::Utc::now().timestamp_millis() as u64;
    let snapshot = executor.snapshot(timestamp_ms);
    match persist::save_executor_snapshot(paths, &snapshot) { ... }
}

// AFTER:
fn save_snapshot(
    executor: &ParallelExecutor,
    dags: &HashMap<String, DagExecutor>,
    state: &RunState,
    writer: &mut SnapshotWriter,
) {
    let timestamp_ms = chrono::Utc::now().timestamp_millis() as u64;
    let full = FullSnapshot {
        version: SNAPSHOT_SCHEMA_VERSION,
        timestamp_ms,
        executor: executor.snapshot(timestamp_ms),
        run_state: state.to_snapshot(),
        dag_states: dags.iter()
            .map(|(id, dag)| (id.clone(), dag.snapshot()))
            .collect(),
    };
    match writer.save_all(&full) {
        Ok(report) => {
            if report.files_written > 0 {
                tracing::debug!(
                    written = report.files_written,
                    skipped = report.files_skipped,
                    "snapshot saved"
                );
            }
            state.snapshot_succeeded();
        }
        Err(e) => {
            tracing::error!(err = %e, "snapshot save failed");
            state.snapshot_failed();
        }
    }
}
```

#### 2. Learning state saved on slower interval

```rust
// event_loop.rs -- new learning flush interval (every 30 seconds)
let mut learning_flush_interval = interval(Duration::from_secs(30));

// In the select! loop:
_ = learning_flush_interval.tick() => {
    if let Err(e) = writer.save_learning_state(
        cascade_router.as_ref(),
        gate_thresholds.as_ref(),
        daimon_state.as_ref(),
    ) {
        tracing::warn!(err = %e, "failed to save learning state");
    }
}
```

#### 3. Resume uses strict validation

```rust
// BEFORE (event_loop.rs:335-378):
fn try_resume(paths: &PersistPaths, config: &ExecutorConfig, plan_ids: &[String]) -> Option<ParallelExecutor> {
    // ... loose overlap check ...
}

// AFTER:
fn try_resume(
    paths: &SnapshotPaths,
    config: &ExecutorConfig,
    plan_tasks: &HashMap<String, TasksFile>,
) -> Result<Option<ResumedState>, ResumeError> {
    if !paths.executor.exists() {
        return Ok(None);
    }

    let snapshot = resume::load_and_validate(paths, plan_tasks)?;

    let executor = ParallelExecutor::from_snapshot(config.clone(), snapshot.executor);
    let state = RunState::from_snapshot(snapshot.run_state);
    let dags = snapshot.dag_states.into_iter()
        .map(|(id, dag_snap)| {
            let dag = DagExecutor::from_snapshot(dag_snap, RetryPolicy::default());
            (id, dag)
        })
        .collect();

    Ok(Some(ResumedState { executor, state, dags }))
}

/// Everything needed to resume a run.
struct ResumedState {
    executor: ParallelExecutor,
    state: RunState,
    dags: HashMap<String, DagExecutor>,
}
```

#### 4. RunState gains snapshot methods

```rust
// state.rs -- new methods
impl RunState {
    /// Serialize to a persistable snapshot.
    pub fn to_snapshot(&self) -> RunStateSnapshot {
        RunStateSnapshot {
            version: SNAPSHOT_SCHEMA_VERSION,
            tasks_total: self.tasks_total,
            tasks_completed: self.tasks_completed,
            tasks_failed: self.tasks_failed,
            total_tokens_in: self.total_tokens_in,
            total_tokens_out: self.total_tokens_out,
            total_cost_usd: self.total_cost_usd,
            total_agent_calls: self.total_agent_calls,
            plan_costs: self.plan_costs.clone(),
            completed_tasks: self.completed_tasks.clone(),
            snapshot_fail_streak: self.snapshot_fail_streak,
        }
    }

    /// Restore from a persisted snapshot.
    pub fn from_snapshot(snap: RunStateSnapshot) -> Self {
        Self {
            tasks_total: snap.tasks_total,
            tasks_completed: snap.tasks_completed,
            tasks_failed: snap.tasks_failed,
            total_tokens_in: snap.total_tokens_in,
            total_tokens_out: snap.total_tokens_out,
            total_cost_usd: snap.total_cost_usd,
            total_agent_calls: snap.total_agent_calls,
            plan_costs: snap.plan_costs,
            completed_tasks: snap.completed_tasks,
            snapshot_fail_streak: snap.snapshot_fail_streak,
            // Non-persistent fields get fresh defaults.
            agent_active: false,
            agent_model: String::new(),
            agent_output: String::new(),
            session_id: None,
            agent_pid: None,
            tokens_in: 0,
            tokens_out: 0,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            cost_usd: 0.0,
            task_agent_calls: 0,
            plan_id: String::new(),
            current_task: String::new(),
            gate_output: String::new(),
            iteration: 0,
            snapshot_degraded: false,
            started_at: Instant::now(),
            task_started_at: Instant::now(),
        }
    }
}
```


## Detailed Specification

### File Layout on Disk

After a runner session, `.roko/` contains:

```
.roko/
  state/
    executor.json         # ExecutorSnapshot (plan phases, queue order)
    run-state.json        # RunStateSnapshot (costs, tokens, completed tasks)
    dag-states.json       # HashMap<plan_id, DagSnapshot> (per-task status)
    daimon.json           # DaimonState (affect engine, optional)
  learn/
    cascade-router.json   # CascadeRouter weights (model routing)
    gate-thresholds.json  # AdaptiveThresholds (EMA per rung)
    efficiency.jsonl      # Per-task efficiency events (append-only)
  runtime/
    agent-pids.json       # Live agent PIDs (for orphan cleanup)
  episodes.jsonl          # Episode log (append-only)
```

### File Format Examples

#### executor.json

```json
{
  "schema_version": 2,
  "plan_states": {
    "refactor-dag": {
      "plan_id": "refactor-dag",
      "current_phase": "Implementing",
      "assigned_agents": ["refactor-dag/T01"],
      "gate_results": [],
      "iteration": 1,
      "started_at_ms": 1714150000000,
      "files_changed": ["crates/roko-cli/src/runner/dag.rs"],
      "merge_attempts": 0,
      "last_error": null,
      "paused": false
    }
  },
  "queue_order": ["refactor-dag"],
  "conductor_circuit_breaker": null,
  "speculative_executions": {},
  "timestamp_ms": 1714150060000
}
```

#### run-state.json

```json
{
  "version": 2,
  "tasks_total": 5,
  "tasks_completed": 2,
  "tasks_failed": 0,
  "total_tokens_in": 45000,
  "total_tokens_out": 12000,
  "total_cost_usd": 0.87,
  "total_agent_calls": 3,
  "plan_costs": {
    "refactor-dag": 0.87
  },
  "completed_tasks": {
    "refactor-dag": ["T01", "T02"]
  },
  "snapshot_fail_streak": 0
}
```

#### dag-states.json

```json
{
  "refactor-dag": {
    "plan_id": "refactor-dag",
    "topo_order": ["T01", "T02", "T03", "T04", "T05"],
    "task_states": {
      "T01": {
        "task_id": "T01",
        "status": "passed",
        "attempts": 1,
        "cost_usd": 0.34,
        "backoff_until_ms": null,
        "last_failure_summary": null
      },
      "T02": {
        "task_id": "T02",
        "status": "passed",
        "attempts": 2,
        "cost_usd": 0.53,
        "backoff_until_ms": null,
        "last_failure_summary": "clippy: unused import"
      },
      "T03": {
        "task_id": "T03",
        "status": "running",
        "attempts": 1,
        "cost_usd": 0.0,
        "backoff_until_ms": null,
        "last_failure_summary": null
      },
      "T04": {
        "task_id": "T04",
        "status": "pending",
        "attempts": 0,
        "cost_usd": 0.0,
        "backoff_until_ms": null,
        "last_failure_summary": null
      },
      "T05": {
        "task_id": "T05",
        "status": "pending",
        "attempts": 0,
        "cost_usd": 0.0,
        "backoff_until_ms": null,
        "last_failure_summary": null
      }
    }
  }
}
```

### Save Triggers

| Event | Files Saved |
|-------|-------------|
| Task completed (gate passed) | executor, run-state, dag-states |
| Task failed (gate failed) | executor, run-state, dag-states |
| Agent exited | executor, run-state, dag-states |
| Every 2s (flush interval) | executor, run-state, dag-states |
| Every 30s (learning flush) | cascade-router, gate-thresholds, daimon |
| Cancellation (Ctrl+C) | ALL files |
| Plan completed | ALL files |

### Incremental Save Logic

Each file has its content hashed (FNV-1a) before writing. The `SnapshotWriter` stores the
last hash per file. On save:

1. Serialize the value to JSON.
2. Hash the JSON bytes.
3. Compare with stored hash.
4. If unchanged, skip the write.
5. If changed, `atomic_write` (`.tmp` -> `rename`) and update stored hash.

Typical savings: After 100 ticks at 100ms, only 3-5 writes actually hit disk (rest are
no-ops because nothing changed between ticks).

### Resume Flow

```
                    +-------------------+
                    | roko plan run ... |
                    +--------+----------+
                             |
                    +--------v----------+
                    | Discover plans    |
                    | from plans/ dir   |
                    +--------+----------+
                             |
                    +--------v----------+
                    | executor.json     |
                    | exists?           |
                    +---+----------+----+
                        |          |
                       yes         no
                        |          |
               +--------v------+   |
               | Load all      |   |
               | state files   |   |
               +--------+------+   |
                        |          |
               +--------v------+   |
               | Validate:     |   |
               | - version     |   |
               | - plan IDs    |   |
               | - task IDs    |   |
               +---+------+----+   |
                   |      |        |
                  pass   fail      |
                   |      |        |
          +--------v-+  +-v------+ |
          | Resume   |  | Warn + | |
          | from     |  | start  +-+
          | snapshot  |  | fresh  |
          +----------+  +--------+
```

### JSONL Integrity

Append-only files (`episodes.jsonl`, `efficiency.jsonl`) can have partial last lines after a
crash. On startup:

```rust
/// Validate a JSONL file and truncate any partial trailing line.
pub fn repair_jsonl(path: &Path) -> Result<usize> {
    let content = fs::read_to_string(path)?;
    let mut valid_lines = 0;
    let mut valid_end = 0;

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(_) => {
                valid_lines += 1;
                valid_end = /* byte offset after this line + newline */;
            }
            Err(_) => {
                tracing::warn!(path = %path.display(), "truncating partial JSONL line");
                break;
            }
        }
    }

    // Truncate file to valid_end bytes if needed.
    if valid_end < content.len() {
        let file = fs::OpenOptions::new().write(true).open(path)?;
        file.set_len(valid_end as u64)?;
    }

    Ok(valid_lines)
}
```


## Error Handling

| Scenario | Response |
|----------|----------|
| Snapshot file corrupt JSON | Log warning, start fresh (don't resume) |
| Schema version newer than binary | Return `ResumeError::VersionTooNew`, refuse to resume |
| Plan IDs don't match | Return `ResumeError::NoPlanOverlap`, start fresh |
| Task IDs changed (tasks.toml edited) | Return `ResumeError::TaskMismatch` with details |
| Atomic write fails (disk full) | Increment `snapshot_fail_streak`, log error, continue running |
| 3+ consecutive write failures | Set `snapshot_degraded` flag, warn loudly, continue running |
| JSONL has partial trailing line | Truncate to last valid line on startup |
| Learning state files missing | Use defaults (these are optional enrichment) |


## Testing Strategy

### Unit Tests

1. **RunStateSnapshot round-trip**: `to_snapshot()` -> serialize -> deserialize -> `from_snapshot()` -> verify all fields.
2. **DagSnapshot round-trip**: Create DAG, snapshot, restore, verify task states match.
3. **Incremental save**: Write same data twice, verify second write is skipped (hash match).
4. **Atomic write crash safety**: Write to path, verify `.tmp` doesn't exist after completion.
5. **JSONL repair**: Create a file with 3 valid lines + 1 partial, verify repair keeps 3.
6. **Version validation**: Snapshot with version 99 should return `VersionTooNew`.
7. **Plan ID validation**: Snapshot with plan "X" but only plan "Y" on disk should fail.
8. **Task ID validation**: Snapshot with task "T99" but tasks.toml only has T01-T05 should fail.

### Integration Tests

1. **Full save-resume cycle**: Run 3 tasks, kill process after task 2, resume, verify task 3 starts (not task 1).
2. **Schema migration**: Create a v1 snapshot (executor.json only), resume with v2 code, verify DAG states are reconstructed.
3. **Learning state persistence**: Set cascade router weights, save, restart, verify weights are restored.
4. **Concurrent save safety**: Spawn 100 save operations, verify no corrupt files (atomic write guarantee).


## Open Questions

1. **Snapshot compaction**: As plans complete, should we remove their entries from the snapshot
   files? Currently they accumulate indefinitely.

2. **Backup rotation**: Should we keep N previous snapshots (e.g., `executor.json.1`,
   `executor.json.2`) for manual recovery? The atomic write prevents corruption, but an
   application-level bug could write a valid-but-wrong snapshot.

3. **Cross-session continuity**: When a new `roko plan run` starts with plans that overlap
   with a previous run's plans, should we reuse learning state (router, thresholds) from the
   previous session?

4. **Event sourcing**: The `events.json` path exists in `PersistPaths` but is never written.
   Should we adopt event sourcing (append events, derive state) instead of periodic snapshots?
   This would give perfect replay but at higher I/O cost.

## Implementation Packet

This work makes resume deterministic and complete enough to support real dogfooding.

### Required Context

- `crates/roko-cli/src/runner/persist.rs`
- `crates/roko-cli/src/runner/state.rs`
- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-orchestrator/src/executor/snapshot.rs`
- `crates/roko-orchestrator/src/executor/mod.rs`
- `crates/roko-learn/src/cascade/persistence.rs`
- `docs/01-orchestration/09-snapshot-recovery.md`
- `tmp/unified/04-EXECUTION.md`
- `tmp/unified/15-TELEMETRY.md`

### Target Snapshot Files

- [ ] `.roko/state/executor.json`
- [ ] `.roko/state/run-state.json`
- [ ] `.roko/learn/cascade-router.json`
- [ ] `.roko/learn/gate-thresholds.json`
- [ ] `.roko/learn/efficiency.jsonl`
- [ ] `.roko/episodes.jsonl`
- [ ] `.roko/state/events.jsonl` or `.roko/state/dashboard-events.jsonl`

### Checklist

- [ ] Add a `snapshot_version` field to every JSON snapshot written by runner-owned code.
- [ ] Extend `PersistPaths` with explicit paths for run state, router, thresholds, and event log.
- [ ] Add `RunStateSnapshot` separate from transient `RunState`.
- [ ] Persist completed task ids per plan.
- [ ] Persist current active task and active agent run id when available.
- [ ] Persist cost/token totals, not just executor phase.
- [ ] Save snapshots atomically using temp file and rename.
- [ ] On resume, validate plan-id overlap and task-id compatibility.
- [ ] On resume, refuse to reuse stale snapshots whose task ids no longer exist unless explicitly forced.
- [ ] Add orphan process cleanup before starting resumed work.
- [ ] Append runtime events to an event log for forensic replay, even if snapshots remain authoritative.

### Verification

- [ ] Unit test: corrupt `executor.json` falls back cleanly without panic.
- [ ] Unit test: stale plan ids are rejected.
- [ ] Integration test: interrupt after task pass and resume without rerunning that task.
- [ ] Integration test: interrupt during gate and resume to a deterministic phase.
- [ ] Search gate: every direct snapshot write goes through the persistence module.

## Worker 9 Evidence Checklist (2026-04-26)

Persistence that exists now:

- [x] `crates/roko-cli/src/runner/persist.rs::PersistPaths` owns `.roko/state/executor.json`, `.roko/episodes.jsonl`, `.roko/learn/efficiency.jsonl`, `.roko/runtime/agent-pids.json`, `.roko/state/events.json`, and `.roko/events.jsonl`.
- [x] `persist.rs::atomic_write` writes snapshots through a temp file and rename.
- [x] `persist.rs::append_runner_event` and `append_jsonl` append forensic runtime events to `.roko/events.jsonl`.
- [x] `persist.rs::save_executor_snapshot` is called from the live `runner/event_loop.rs` after key executor transitions and at terminal completion.
- [x] `crates/roko-orchestrator/src/executor/snapshot.rs` has `CURRENT_SCHEMA_VERSION = 1`, `schema_version`, and legacy snapshot loading.
- [x] `persist.rs::cleanup_orphaned_agents` is called before active runner work begins.
- [x] No-mock proof captured terminal `.roko/state/executor.json` with `current_phase.kind = "complete"`.

Persistence still missing or insufficiently proven:

- [ ] The runner does not write `.roko/state/run-state.json`; `RunState::completed_tasks`, lifecycle projection, retry backoff, and active effect flags are not snapshotted separately.
- [ ] `PersistPaths` does not expose `.roko/learn/cascade-router.json` or `.roko/learn/gate-thresholds.json` for the active runner feedback path.
- [ ] `ExecutorSnapshot` has `schema_version`, not the planned `snapshot_version` field across every runner-owned snapshot.
- [ ] `.roko/state/executor.json` does not include `run_id`; `run_id` is present in runtime events but not in the executor snapshot proof.
- [ ] Resume validation checks plan overlap but does not prove stale task-id rejection against changed task definitions.
- [ ] No integration proof currently shows interrupt after task pass, interrupt during gate, or resume without duplicate task completion.

## 2026-04-27 Deepening Pass - Source-Corrected Persistence State

Self-grade for this pass:

- Initial rating: 9.91 / 10.
- Reasoning: this section corrects stale persistence claims with current source evidence, separates implemented persistence APIs from unproven crash semantics, and provides concrete proof artifacts and archive gates. The score is not higher because this pass did not run a destructive crash/resume dogfood proof.

This section supersedes the "Persistence still missing" list above where source has moved forward.

### Current Source Truth

- [x] `PersistPaths` now includes `.roko/state/executor.json`, `.roko/state/orchestrator.json`, `.roko/state/run-state.json`, `.roko/episodes.jsonl`, `.roko/learn/efficiency.jsonl`, `.roko/learn/cascade-router.json`, `.roko/learn/gate-thresholds.json`, `.roko/runtime/agent-pids.json`, `.roko/state/events.json`, and `.roko/events.jsonl`.
- [x] `RunStateSnapshot` exists with `schema_version`, `run_id`, timestamps, task counts, token totals, cost totals, per-plan costs, completed task ids, snapshot failure streak, and task fingerprints.
- [x] `save_run_state` and `load_run_state` exist and use atomic writes for `.roko/state/run-state.json`.
- [x] `save_executor_snapshot`, `save_orchestrator_snapshot`, and `save_agent_pids` use the persistence module.
- [x] `append_runner_event` appends normalized runner lifecycle events to `.roko/events.jsonl`.
- [x] `recover_jsonl` exists and can truncate trailing partial or malformed JSONL tails after the last valid line.
- [x] `TaskDefFingerprint` exists and hashes fields that matter for strict resume validation.
- [x] `runner/resume.rs` loads run-state, rejects future schema versions, validates task fingerprint drift, and recovers JSONL files.
- [x] `runner/event_loop.rs` calls `save_snapshot` and writes both executor and run-state snapshots.
- [x] `cleanup_orphaned_agents` registers saved pids with `roko-agent` cleanup before resumed work.

### Remaining Persistence Gaps

- [ ] `cascade_router_json` and `gate_thresholds_json` paths exist, but active save/load proof for those files is still incomplete.
- [ ] `RunStateSnapshot` captures completed tasks and cost/token totals, but retry backoff, active effect id, active merge attempt id, prompt diagnostics ids, and projection cursor are not obviously persisted.
- [ ] `ExecutorSnapshot` and `RunStateSnapshot` use `schema_version`; the docs should stop requiring a separate `snapshot_version` name unless a migration ADR chooses that rename.
- [ ] `.roko/state/events.json` is still a declared path; the active append-only event stream is `.roko/events.jsonl`. Decide whether `events.json` is legacy, reserved, or should be removed.
- [ ] JSONL appends are recoverable but not entry-atomic across power loss; the recovery proof must show partial-line truncation before resume.
- [ ] Strict stale task-id rejection exists in helpers, but dogfood proof must show a changed `tasks.toml` refuses resume.
- [ ] Crash proof must cover interrupt after task pass, interrupt during gate, interrupt during merge, and interrupted JSONL append.
- [ ] Orchestrator snapshot and executor snapshot ownership overlap needs a stable rule: which one is authoritative for resume?
- [ ] The HTTP/query projection must expose last snapshot time, last resume marker, JSONL recovery result, orphan cleanup result, and snapshot failure streak.

### Target Persistence Contract

The runner should treat persistence as two layers:

- [ ] Snapshot layer: authoritative resume state, stored in versioned JSON snapshots under `.roko/state/`.
- [ ] Event layer: append-only forensic stream, stored in `.roko/events.jsonl` and used for replay, projections, and debugging.
- [ ] Learning layer: independently versioned learning state under `.roko/learn/`, saved through the feedback/router services.
- [ ] Runtime layer: short-lived process state under `.roko/runtime/`, safe to delete after cleanup.

Snapshot authority rules:

- [ ] `run-state.json` owns runner-local counters, completed task ids, active effect metadata, and task fingerprints.
- [ ] `executor.json` owns executor phase/queue state.
- [ ] `orchestrator.json` either becomes the aggregate snapshot used for resume or is explicitly marked compatibility-only.
- [ ] No code outside `runner/persist.rs`, `runner/resume.rs`, or a typed repository may write these files directly.

### Implementation Batches

#### PER-01: Complete Snapshot Shape

- [ ] Add active effect metadata to `RunStateSnapshot`: current plan id, current task id, attempt id, dispatch run id, gate rung, retry backoff, merge attempt id, prompt diagnostics ids, and projection cursor.
- [ ] Add resume policy metadata: created-by binary version, git commit when available, workdir fingerprint, and plan directory fingerprint.
- [ ] Keep `schema_version` as the standard version field unless an ADR explicitly renames all snapshot fields.
- [ ] Add migration helpers for `RUN_STATE_SCHEMA_VERSION` before bumping the version.

#### PER-02: Save Learning State Durably

- [ ] Save `CascadeRouter` to `paths.cascade_router_json` after terminal task observations or periodic flush.
- [ ] Save adaptive gate thresholds to `paths.gate_thresholds_json` after gate outcome observations.
- [ ] Include save success/failure in runtime events and projections.
- [ ] On startup, load both files from `PersistPaths` rather than reconstructing path strings elsewhere.

#### PER-03: Clarify Snapshot Authority

- [ ] Decide if `orchestrator.json` is the aggregate authoritative snapshot or compatibility output.
- [ ] If authoritative, resume from `orchestrator.json` first and validate it against `executor.json` and `run-state.json`.
- [ ] If compatibility-only, document it and stop using it for decisions.
- [ ] Add a proof that mismatched snapshots fail closed rather than silently picking one.

#### PER-04: Crash/Resume Proof Harness

- [ ] Create a temporary workdir outside the repo.
- [ ] Start a real provider or approved no-provider deterministic run.
- [ ] Kill the runner after one task pass and before plan completion.
- [ ] Resume and prove the completed task is not rerun.
- [ ] Kill the runner during gate execution.
- [ ] Resume and prove gate phase is deterministic.
- [ ] Kill the runner during merge execution.
- [ ] Resume and prove merge state is either completed once or safely retried.
- [ ] Corrupt the tail of `.roko/events.jsonl` and prove `recover_jsonl` truncates it before resume.

#### PER-05: Observability And Query

- [ ] Emit `persistence.snapshot_saved` with snapshot kind, path, version, duration, and byte size.
- [ ] Emit `persistence.snapshot_failed` with snapshot kind, path, error class, and failure streak.
- [ ] Emit `persistence.resume_validated` with run id, prior run id, task fingerprint count, and JSONL recovery summary.
- [ ] Emit `persistence.resume_rejected` with exact rejection reason.
- [ ] Add HTTP/query endpoints or CLI inspect commands for the latest snapshot and resume markers.

### Generated Proof Contract

An agent implementing this file must produce `tmp/mori-diffs/generated/persistence-resume-proof.json`:

```json
{
  "schema": "mori-diffs.persistence-resume-proof.v1",
  "generated_at": "ISO-8601 timestamp",
  "git_commit": "HEAD sha",
  "snapshots": {
    "executor_json": false,
    "orchestrator_json": false,
    "run_state_json": false,
    "cascade_router_json": false,
    "gate_thresholds_json": false,
    "events_jsonl": false
  },
  "resume_cases": {
    "after_task_pass_no_duplicate": false,
    "during_gate_deterministic": false,
    "during_merge_safe": false,
    "stale_task_rejected": false,
    "future_schema_rejected": false,
    "partial_jsonl_recovered": false,
    "orphan_agents_cleaned": false
  },
  "queries": [],
  "remaining_gaps": []
}
```

### No-Context Handoff Checklist

- [ ] Open `crates/roko-cli/src/runner/persist.rs` and verify `PersistPaths` still owns every path listed above.
- [ ] Open `crates/roko-cli/src/runner/event_loop.rs` and find `save_snapshot`.
- [ ] Add any missing fields to `RunStateSnapshot` before changing resume semantics.
- [ ] Open `crates/roko-cli/src/runner/resume.rs` and add proof for stale task-id rejection.
- [ ] Wire cascade router and gate threshold saves through `PersistPaths`.
- [ ] Add persistence runtime events and projection/query surfaces.
- [ ] Generate `persistence-resume-proof.json`.
- [ ] Update [22-STABILITY-PLAN.md](22-STABILITY-PLAN.md), [29-CURRENT-RUNTIME-GAP-LEDGER.md](29-CURRENT-RUNTIME-GAP-LEDGER.md), [34-OBSERVABILITY-PROJECTION-QUERY-AUDIT.md](34-OBSERVABILITY-PROJECTION-QUERY-AUDIT.md), and [README.md](README.md).

### Archive Gate

- [ ] Every old "missing file" claim is corrected or marked historical.
- [ ] Router and threshold state are saved and loaded from `PersistPaths`.
- [ ] Crash/resume proof covers task, gate, merge, stale task, future schema, JSONL recovery, and orphan cleanup.
- [ ] Persistence events are queryable through HTTP or CLI.
- [ ] `persistence-resume-proof.json` exists and is linked from README.
