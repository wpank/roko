//! Durable checkpoints for CLI plan execution through the Graph Engine.
//!
//! The Graph Engine records successful Activity outputs as JSONL.  This module
//! adds the small amount of run metadata needed to resume those recordings
//! safely: a schema version, a stable fingerprint of the converted graph, and
//! the run ID that scopes every record.
//!
//! # Graph fingerprints
//!
//! Checkpoints record [`plan_graph_fingerprint`], computed from the plan's
//! authored `tasks.toml`, so a roko upgrade that only adds task fields keeps
//! in-flight checkpoints resumable. Releases before it recorded
//! [`legacy_graph_execution_fingerprint`]; resume still accepts that value and
//! rewrites the checkpoint with the authored fingerprint.
//!
//! # Schema versions
//!
//! - **v2**: original manifest with plan/graph/run identity, Activity log, and
//!   cost ledger references.
//! - **v3**: adds a namespaced extension map and an idempotent receipt
//!   ledger. A v2 manifest is migrated in-memory to v3 with empty
//!   extensions/receipts and its existing cost ledger preserved.
//! - **v4** (this release): the manifest is the checkpoint's commit point and
//!   names a generation (gap-dc1d16; see [`GraphCheckpointManifest`]). A v2 or
//!   v3 manifest resumes as generation 0, and v4 is written on the next
//!   checkpoint write. Versions other than 2, 3 or 4 fail closed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use roko_fs::RokoLayout;
use roko_graph::cells::task_executor::{TaskExecutionSpec, TaskGateVerdict};
use roko_graph::convert::{PlanTaskInfo, plan_to_graph};
use roko_graph::replay::{
    LoggedRecord, RecordEntry, activity_records, complete_records_len, set_aside_activities_after,
};
use roko_graph::{
    ActivityRecorder, ActivityReplayer, AuthoredPlan, EXT_SAFETY_PROVENANCE, Graph,
    legacy_graph_execution_fingerprint, plan_graph_fingerprint,
};
use roko_learn::telemetry::report::RunRecords;
use roko_learn::telemetry::{AttemptOpenRecord, AttemptOutcome};
use serde::{Deserialize, Serialize};

use crate::runner::plan_loader::Plan;
use crate::safety_provenance::{GraphProvenanceSink, SafetyProvenanceSummary};
use crate::task_accept;
use crate::task_parser::{TaskDef, TasksFile};

/// Current host checkpoint schema version. V2 and v3 manifests resume as
/// generation 0 (see [`GraphCheckpointManifest`]); other versions fail closed.
const CHECKPOINT_SCHEMA_VERSION: u32 = 4;

/// Schema version of a manifest from before generations, once a v2 manifest
/// is migrated in memory.
const PRE_GENERATION_SCHEMA_VERSION: u32 = 3;

/// Minimum schema version we can migrate from. Anything below this fails closed.
const MIN_SUPPORTED_SCHEMA_VERSION: u32 = 2;

const COST_LEDGER_SCHEMA_VERSION: u32 = 1;
/// Runner-v2 snapshot paths that a bare `--resume-plan` (or `roko resume`)
/// may pass: the clap `default_missing_value` and the legacy executor file.
/// Both mean "resume from the canonical Graph checkpoint root".
const DEFAULT_RUNNER_RESUME_PATHS: [&str; 2] = [
    ".roko/state/state-snapshot.json",
    ".roko/state/executor.json",
];

/// Known extension namespace for workspace/attempt state (#249).
pub const WORKSPACE_ATTEMPT_EXTENSION: &str = "roko.workspace.attempt@1";

/// Known extension namespace for structured gate verdicts (#250).
pub const GATE_VERDICT_EXTENSION: &str = "roko.gate.verdict@1";

/// Known extension namespace for completion delivery state (#254).
pub const DELIVERY_EXTENSION: &str = roko_graph::delivery::DELIVERY_EXTENSION_KEY;

/// Known extension namespace for the tasks the last run did not complete.
pub const TASK_OUTCOME_EXTENSION: &str = "roko.task.outcome@1";

/// Known extension namespace for what stopped the last run before its tasks
/// settled (gap-fab2cc): `{"by": "SIGINT" | "SIGTERM" | "SIGHUP" |
/// "deadline" | "conductor"}`. Absent when nothing stopped it.
pub const STOP_EXTENSION: &str = "roko.run.stop@1";

/// Known extension namespace for the plan's delivery into its run's batch
/// branch (spec-f830c4).
pub const BATCH_EXTENSION: &str = "roko.batch@1";

/// Known extension namespace for the plan's whole-plan check (`[meta]
/// verify`, gap-60233f) when it ran in the shared working tree.
pub const PLAN_VERIFY_EXTENSION: &str = "roko.plan.verify@1";

/// Namespace of [`EXT_SAFETY_PROVENANCE`] (gap-ff95f5). This build reads
/// version 1 only, and fails closed on any other.
const SAFETY_PROVENANCE_NAMESPACE: &str = "roko.safety-provenance";

/// Known extension namespace for the tasks whose latest attempt a stop cut
/// off, as the last resume found them (gap-36f3fb).
pub const INTERRUPTED_ATTEMPT_EXTENSION: &str = "roko.attempt.interrupted@1";

/// Lifecycle state persisted beside a Graph Activity recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphCheckpointStatus {
    /// The process may have stopped before every node reached a terminal state.
    Running,
    /// Every graph node completed and every task passed its verify steps.
    Succeeded,
    /// Every graph node completed and no task failed, but some tasks ran no
    /// verify step, so the plan did not succeed.
    Unverified,
    /// At least one graph node failed.
    Failed,
    /// An operator cancelled the plan before every node finished.
    Cancelled,
    /// A stop request (a signal, a closed operator TUI, the FAST deadline or
    /// the conductor) stopped the run before every node finished; recorded
    /// Activities remain resumable. [`STOP_EXTENSION`] names the request.
    Interrupted,
}

impl GraphCheckpointStatus {
    /// Serialized name, e.g. `interrupted`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Unverified => "unverified",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Interrupted => "interrupted",
        }
    }
}

// ---------------------------------------------------------------------------
// Extension map
// ---------------------------------------------------------------------------

/// A namespaced, versioned extension stored in the checkpoint manifest.
///
/// Feature packets (#252-#255) register their own concrete extension schemas
/// without editing the `GraphCheckpointManifest` struct. The extension map key
/// is exactly `<namespace>@<schema_version>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointExtension {
    /// Dotted namespace, e.g. `roko.workspace.attempt`.
    pub namespace: String,
    /// Schema version of this extension's value.
    pub schema_version: u32,
    /// Whether this extension is required for a valid restore. Unknown
    /// required extensions fail restore; unknown optional extensions
    /// round-trip byte-for-value.
    pub required: bool,
    /// Deterministic fingerprint of the extension value, used to detect
    /// re-registration drift.
    pub fingerprint: String,
    /// Opaque JSON payload owned by the registering feature.
    pub value: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Receipt ledger
// ---------------------------------------------------------------------------

/// Ordered lifecycle states for an idempotent receipt.
///
/// State transitions only move forward: `Prepared -> Committed -> Settled`.
/// Repeating the current transition is a no-op success. Reverse or skipped
/// transitions fail closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptState {
    /// The receipt has been created but the external side-effect has not
    /// been confirmed.
    Prepared = 0,
    /// The external side-effect has been confirmed. Provider dispatch
    /// reuses the committed evidence rather than re-calling.
    Committed = 1,
    /// The receipt has been fully settled and requires no further work.
    Settled = 2,
}

impl std::fmt::Display for ReceiptState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Prepared => write!(f, "prepared"),
            Self::Committed => write!(f, "committed"),
            Self::Settled => write!(f, "settled"),
        }
    }
}

/// A single entry in the checkpoint's receipt ledger.
///
/// The `idempotency_key` is the ledger map key. Provider dispatch checks
/// this before calling externally: `Committed` reuses evidence, `Prepared`
/// invokes the owner's reconcile path, and `Settled` performs no work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptLedgerEntry {
    /// Stable key for deduplication across restarts.
    pub idempotency_key: String,
    /// Subsystem or feature that owns this receipt.
    pub owner: String,
    /// Correlation ID linking this receipt to its originating request.
    pub correlation_id: String,
    /// Current lifecycle state.
    pub state: ReceiptState,
    /// Optional reference to external evidence (commit OID, URL, etc.).
    #[serde(default)]
    pub evidence_ref: Option<String>,
    /// Unix milliseconds of the last state transition.
    pub updated_at_ms: u128,
    /// Last error message if a transition failed.
    #[serde(default)]
    pub last_error: Option<String>,
}

// ---------------------------------------------------------------------------
// Gate verdicts
// ---------------------------------------------------------------------------

/// Graph cell type that executes plan tasks.
const TASK_EXECUTOR_CELL_TYPE: &str = "task-executor";

/// A recorded Activity refused for replay on resume; its node re-runs. The
/// log keeps the record (gap-dc1d16).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvalidatedActivity {
    /// Graph node whose record was refused.
    pub node_id: String,
    /// Tick of the refused record.
    pub tick: u64,
    /// Why the record could not be trusted as a completed node.
    pub reason: String,
}

/// Value stored under [`GATE_VERDICT_EXTENSION`].
///
/// The authoritative verdict travels on each recorded output signal (see
/// [`TaskGateVerdict`]); this is the manifest-level summary, refreshed on
/// resume and on every terminal checkpoint write.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateVerdictSummary {
    /// Verdict recorded for each task node, keyed by node id.
    #[serde(default)]
    pub verdicts: BTreeMap<String, TaskGateVerdict>,
    /// Records the last resume refused because they lacked a passing
    /// verdict, and whose nodes had no other record to replay.
    #[serde(default)]
    pub invalidated_on_resume: Vec<InvalidatedActivity>,
}

/// Value stored under [`TASK_OUTCOME_EXTENSION`]: how the last run left the
/// tasks it did not complete. A resume runs every one of them again.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskOutcomeSummary {
    /// Tasks that failed.
    #[serde(default)]
    pub failed: BTreeSet<String>,
    /// Tasks skipped because a task they depend on failed, each with the
    /// failed task that blocked it.
    #[serde(default)]
    pub blocked_by: BTreeMap<String, String>,
    /// Tasks that never started for another reason, such as a spent plan
    /// budget or a fail-fast stop, each with that reason.
    #[serde(default)]
    pub not_started: BTreeMap<String, String>,
}

/// Value stored under [`INTERRUPTED_ATTEMPT_EXTENSION`]: the tasks whose
/// latest attempt a stop cut off, read on resume from the run's attempt log
/// (`.roko/runs/<run_id>/attempts.jsonl`). Anything such an attempt wrote is
/// still in the task's checkout, and no Activity records it; the task runs
/// again on top of it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterruptedAttempts {
    /// One entry per task, in task id order.
    #[serde(default)]
    pub attempts: Vec<InterruptedAttempt>,
}

/// A task's latest attempt, which a stop cut off.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterruptedAttempt {
    /// The task the attempt ran.
    pub task_id: String,
    /// The attempt's S01 key, `{chain_key}:{attempt}`.
    pub attempt_key: String,
    /// When the attempt started (Unix ms), when its open line says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at_ms: Option<i64>,
    /// The run cancelled the attempt. Otherwise it never settled: the
    /// process stopped while it ran.
    #[serde(default)]
    pub cancelled: bool,
}

/// Task nodes whose recorded outputs must carry a passing gate verdict before
/// they may be replayed: every task with authored verify steps. A task whose
/// definition cannot be decoded is treated as verify-bearing (fail closed).
fn verdict_required_nodes(graph: &Graph) -> std::collections::HashSet<String> {
    graph
        .inner
        .node_weights()
        .filter(|node| node.cell_type == TASK_EXECUTOR_CELL_TYPE)
        .filter(|node| {
            let spec = TaskExecutionSpec::from_config(&node.config);
            !spec.task_def_json.is_empty()
                && serde_json::from_str::<crate::task_parser::TaskDef>(&spec.task_def_json)
                    .map_or(true, |task| !task.verify.is_empty())
        })
        .map(|node| node.id.clone())
        .collect()
}

/// Why a recorded Activity output must not be replayed, or `None` when it may be.
fn replay_refusal(signals: &[roko_core::Signal], verify_required: bool) -> Option<String> {
    match TaskGateVerdict::from_signals(signals) {
        Some(verdict) if !verdict.is_replayable() => {
            Some(format!("recorded gate verdict is `{}`", verdict.as_str()))
        }
        // Its verify steps passed, on a tree that already held its work, or
        // failed only on tests that failed before its run.
        Some(
            TaskGateVerdict::Passed
            | TaskGateVerdict::PassedWithPreexistingFailures
            | TaskGateVerdict::AlreadySatisfied,
        ) => None,
        Some(verdict) if verify_required => Some(format!(
            "verify steps are authored but the recorded gate verdict is `{}`",
            verdict.as_str()
        )),
        None if verify_required => {
            Some("verify steps are authored but no gate verdict was recorded".to_string())
        }
        _ => None,
    }
}

/// Refuse, by line number, the recorded Activities that must not be
/// replayed: forced accepts and verify-bearing task outputs without a passing
/// verdict (for example records written before verdicts existed, when a
/// failed verify could be force-accepted). Their nodes re-execute instead of
/// resuming as successes. The log keeps every record, and a record whose line
/// is in `refused`, which an earlier resume refused, stays refused
/// (gap-dc1d16).
///
/// Returns every refused line, and the refused records whose node has no
/// other record to replay: the ones this resume runs again.
fn invalidate_unverified_activities(
    records: &[LoggedRecord],
    graph: &Graph,
    refused: &BTreeSet<u64>,
) -> (BTreeSet<u64>, Vec<InvalidatedActivity>) {
    let required = verdict_required_nodes(graph);
    let mut refused = refused.clone();
    let mut invalidated = Vec::new();
    for record in records {
        let entry = &record.entry;
        let reason = match replay_refusal(&entry.signals, required.contains(&entry.node_id)) {
            Some(reason) => reason,
            None if refused.contains(&record.line) => "an earlier resume refused it".to_string(),
            None => continue,
        };
        refused.insert(record.line);
        invalidated.push(InvalidatedActivity {
            node_id: entry.node_id.clone(),
            tick: entry.tick,
            reason,
        });
    }
    // A refused record whose node ran again and left a record to replay is
    // superseded: this resume does not run that node again.
    let replayable: BTreeSet<(&str, u64)> = records
        .iter()
        .filter(|record| !refused.contains(&record.line))
        .map(|record| (record.entry.node_id.as_str(), record.entry.tick))
        .collect();
    invalidated
        .retain(|activity| !replayable.contains(&(activity.node_id.as_str(), activity.tick)));
    (refused, invalidated)
}

/// The records of the Activity log at `path` that count for run `run_id` of
/// plan `plan_id`: those in the prefix `committed` names, less the ones a
/// resume refused, read with the parser a resume replays with (gap-dc1d16).
/// Without a committed generation, as before generations, the prefix is the
/// log's complete records. A missing log has none.
fn counted_records(
    path: &Path,
    plan_id: &str,
    run_id: &str,
    committed: Option<&CommittedGeneration>,
) -> std::io::Result<Vec<RecordEntry>> {
    let mut log = match std::fs::read(path) {
        Ok(log) => log,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let committed_len = committed.map_or_else(
        || complete_records_len(&log),
        |generation| generation.activity_bytes,
    );
    log.truncate(usize::try_from(committed_len).unwrap_or(usize::MAX));
    let mut counted = Vec::new();
    for record in activity_records(&log, plan_id, run_id) {
        let record = record?;
        if committed.is_none_or(|generation| !generation.refused_records.contains(&record.line)) {
            counted.push(record.entry);
        }
    }
    Ok(counted)
}

/// The latest gate verdict recorded for each node in `records`.
fn recorded_gate_verdicts(records: &[RecordEntry]) -> BTreeMap<String, TaskGateVerdict> {
    records
        .iter()
        .filter_map(|entry| {
            TaskGateVerdict::from_signals(&entry.signals)
                .map(|verdict| (entry.node_id.clone(), verdict))
        })
        .collect()
}

/// The nodes whose output `records` hold.
fn recorded_nodes(records: &[RecordEntry]) -> BTreeSet<String> {
    records.iter().map(|entry| entry.node_id.clone()).collect()
}

/// The tasks of plan `plan_id` whose latest attempt in `run_dir`'s attempt
/// log a stop cut off: it never settled, or the run cancelled it. A task in
/// `recorded`, whose output the Activity log holds, is left out: its attempt
/// finished, even if the process stopped before its verdict line was written.
fn interrupted_attempts(
    run_dir: &Path,
    plan_id: &str,
    recorded: &BTreeSet<String>,
) -> Vec<InterruptedAttempt> {
    let records = match RunRecords::load(run_dir) {
        Ok(records) => records,
        Err(error) => {
            tracing::warn!(
                run_dir = %run_dir.display(),
                %error,
                "resume: the run's attempt log is unreadable; interrupted attempts go unrecorded"
            );
            return Vec::new();
        }
    };
    let outcomes: BTreeMap<&str, AttemptOutcome> = records
        .verdicts
        .iter()
        .map(|verdict| {
            (
                verdict.record.identity.attempt_key.as_str(),
                verdict.record.outcome,
            )
        })
        .collect();
    let mut latest: BTreeMap<&str, &AttemptOpenRecord> = BTreeMap::new();
    for open in records.opens.iter().map(|open| &open.record) {
        let identity = &open.identity;
        if identity.plan_id != plan_id {
            continue;
        }
        let newer = latest
            .get(identity.task_id.as_str())
            .is_none_or(|seen| seen.identity.attempt < identity.attempt);
        if newer {
            latest.insert(identity.task_id.as_str(), open);
        }
    }
    latest
        .into_values()
        .filter(|open| {
            let node = open.identity.node_id.as_deref();
            !recorded.contains(node.unwrap_or(&open.identity.task_id))
        })
        .filter_map(|open| {
            let cancelled = match outcomes.get(open.identity.attempt_key.as_str()) {
                None => false,
                Some(AttemptOutcome::Cancelled) => true,
                Some(_) => return None,
            };
            Some(InterruptedAttempt {
                task_id: open.identity.task_id.clone(),
                attempt_key: open.identity.attempt_key.clone(),
                started_at_ms: open.attempt_started_at,
                cancelled,
            })
        })
        .collect()
}

/// Build the [`GATE_VERDICT_EXTENSION`] entry for `summary`.
fn gate_verdict_extension(summary: &GateVerdictSummary) -> Result<CheckpointExtension> {
    let value = serde_json::to_value(summary).context("serialize gate verdict summary")?;
    host_extension(GATE_VERDICT_EXTENSION, value)
}

/// Build the optional, host-owned extension entry for `key`
/// (`<namespace>@<schema_version>`) holding `value`.
fn host_extension(key: &str, value: serde_json::Value) -> Result<CheckpointExtension> {
    let (namespace, version) = key
        .split_once('@')
        .with_context(|| format!("extension key `{key}` has no schema version"))?;
    let bytes =
        serde_json::to_vec(&value).with_context(|| format!("serialize extension `{key}`"))?;
    Ok(CheckpointExtension {
        namespace: namespace.to_string(),
        schema_version: version
            .parse()
            .with_context(|| format!("extension `{key}` schema version"))?,
        required: false,
        fingerprint: blake3::hash(&bytes).to_hex().to_string(),
        value,
    })
}

// ---------------------------------------------------------------------------
// Graph identity
// ---------------------------------------------------------------------------

/// How a checkpoint's recorded graph fingerprint relates to the plan being run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FingerprintMatch {
    /// The checkpoint records the plan's current fingerprint.
    New,
    /// The checkpoint records the Graph fingerprint an older roko computed;
    /// resuming it rewrites the checkpoint with the current one.
    Legacy,
    /// The checkpoint records neither: the plan changed after it was written.
    #[serde(rename = "none")]
    Mismatch,
}

impl FingerprintMatch {
    /// Label shown by `plan run --dry-run`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Legacy => "legacy",
            Self::Mismatch => "none",
        }
    }
}

/// The fingerprints a checkpoint of one converted plan graph may record.
#[derive(Debug, Clone, PartialEq, Eq)]
struct GraphIdentity {
    /// Recorded by new and migrated checkpoints: the authored plan fingerprint
    /// when the plan's `tasks.toml` still describes the graph, otherwise the
    /// legacy Graph fingerprint.
    current: String,
    /// The Graph fingerprint recorded before authored plan fingerprints.
    legacy: String,
}

impl GraphIdentity {
    fn of(workdir: &Path, graph: &Graph) -> Result<Self> {
        let legacy = legacy_graph_execution_fingerprint(graph).context("fingerprint Graph")?;
        let current = match authored_plan(workdir, graph) {
            Some(authored) => {
                plan_graph_fingerprint(graph, &authored).context("fingerprint authored plan")?
            }
            None => legacy.clone(),
        };
        Ok(Self { current, legacy })
    }

    fn matching(&self, recorded: &str) -> FingerprintMatch {
        if recorded == self.current {
            FingerprintMatch::New
        } else if recorded == self.legacy {
            FingerprintMatch::Legacy
        } else {
            FingerprintMatch::Mismatch
        }
    }
}

/// The authored definition of the plan `graph` was converted from, read from
/// the `tasks.toml` in its task nodes' plan directory.
///
/// `None` when the graph has no plan task nodes or the file cannot be read, and
/// when the file no longer holds exactly the tasks the graph runs (it was
/// edited after the plan was loaded), so the identity never describes tasks
/// other than the ones executing.
fn authored_plan(workdir: &Path, graph: &Graph) -> Option<AuthoredPlan> {
    let specs: Vec<TaskExecutionSpec> = graph
        .inner
        .node_weights()
        .filter(|node| node.cell_type == TASK_EXECUTOR_CELL_TYPE)
        .map(|node| TaskExecutionSpec::from_config(&node.config))
        .collect();
    let plan_dir = Path::new(&specs.first()?.plan_dir);
    if plan_dir.as_os_str().is_empty() {
        return None;
    }
    let path = [plan_dir.to_path_buf(), workdir.join(plan_dir)]
        .into_iter()
        .map(|dir| dir.join("tasks.toml"))
        .find(|path| path.is_file())?;
    let content = std::fs::read_to_string(&path).ok()?;
    let authored = authored_plan_running(&content, &specs);
    if authored.is_none() {
        tracing::warn!(
            tasks_toml = %path.display(),
            "tasks.toml no longer matches the tasks this run converted; \
             its checkpoint keeps the legacy Graph fingerprint"
        );
    }
    authored
}

/// The authored plan in `content`, when its tasks are exactly those `specs` run.
///
/// A run's tasks carry the verify steps [`task_accept`] generated for their `[task.accept]` tests
/// in front of their own; the file does not. Those steps are set aside for the comparison, and
/// the sha256 each one pinned is recorded on its `[task.accept]` entry. The identity then changes
/// when a test is re-pinned with other content, but not with the store's place on disk.
fn authored_plan_running(content: &str, specs: &[TaskExecutionSpec]) -> Option<AuthoredPlan> {
    let mut authored = AuthoredPlan::from_tasks_toml(content).ok()?;
    let loaded = TasksFile::parse_str(content)
        .ok()?
        .tasks
        .iter()
        .map(|task| Some((task.id.clone(), serde_json::to_value(task).ok()?)))
        .collect::<Option<BTreeMap<_, _>>>()?;
    let mut pinned = BTreeMap::new();
    let running = specs
        .iter()
        .map(|spec| {
            let mut task: serde_json::Value = serde_json::from_str(&spec.task_def_json).ok()?;
            let id = task.get("id")?.as_str()?.to_string();
            let hashes = set_aside_pinned_steps(&mut task)?;
            if !hashes.is_empty() {
                pinned.insert(id.clone(), hashes);
            }
            Some((id, task))
        })
        .collect::<Option<BTreeMap<_, _>>>()?;
    if running != loaded || !authored.tasks.keys().eq(loaded.keys()) {
        return None;
    }
    for (id, hashes) in &pinned {
        record_pinned_hashes(authored.tasks.get_mut(id)?, hashes)?;
    }
    Some(authored)
}

/// Remove the generated acceptance steps from a converted task's `verify`, and return the sha256
/// each one pinned, in order. `None` when a step's hash cannot be read.
fn set_aside_pinned_steps(task: &mut serde_json::Value) -> Option<Vec<String>> {
    let Some(steps) = task
        .get_mut("verify")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return Some(Vec::new());
    };
    let mut hashes = Vec::new();
    let mut readable = true;
    steps.retain(|step| {
        let command = step
            .get("command")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        if !task_accept::is_pinned_command(command) {
            return true;
        }
        match task_accept::pinned_sha256(command) {
            Some(hash) => hashes.push(hash.to_string()),
            None => readable = false,
        }
        false
    });
    readable.then_some(hashes)
}

/// Record `hashes` on an authored task's `[task.accept]` entries, one per entry in order. The
/// entries deny unknown fields, so no authored entry has a `sha256` of its own.
fn record_pinned_hashes(task: &mut serde_json::Value, hashes: &[String]) -> Option<()> {
    let entries = task.pointer_mut("/accept/files")?.as_array_mut()?;
    if entries.len() != hashes.len() {
        return None;
    }
    for (entry, hash) in entries.iter_mut().zip(hashes) {
        entry.as_object_mut()?.insert(
            "sha256".to_string(),
            serde_json::Value::String(hash.clone()),
        );
    }
    Some(())
}

// ---------------------------------------------------------------------------
// Manifest
// ---------------------------------------------------------------------------

/// What one generation of a Graph checkpoint commits (gap-dc1d16).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommittedGeneration {
    /// Length in bytes of the committed prefix of the Activity log.
    pub activity_bytes: u64,
    /// BLAKE3 of that prefix, in hex.
    pub activity_blake3: String,
    /// Actual provider spend, in millionths of one USD.
    pub spent_micro_usd: u64,
    /// Spend reserved for provider calls in flight, in millionths of one USD.
    pub reserved_micro_usd: u64,
    /// Line numbers, from 0, of the committed records a resume refused for
    /// replay. The log keeps them, and every reader of it skips them.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub refused_records: BTreeSet<u64>,
}

/// Versioned metadata that makes an Activity JSONL file safe to resume.
///
/// The manifest is the checkpoint's only commit point (gap-dc1d16). Each
/// write names the next generation: under [`Self::committed`], the length and
/// BLAKE3 of the Activity log's committed prefix and the cost at that moment.
/// A durable change appends and syncs its Activity record, or writes the cost
/// ledger, first, and writes the manifest last, so a crash between the two
/// leaves the change uncommitted. A resume uses exactly the generation the
/// manifest names: it sets aside the log's bytes past the committed prefix
/// and a cost the ledger holds beyond the committed one, and fails closed
/// when committed bytes changed. It never rewrites committed bytes: it
/// refuses a record by its line number
/// ([`CommittedGeneration::refused_records`]) instead.
///
/// Schema version 3 added `extensions` and `receipts` to the v2 fields, and
/// version 4 the generation. A v2 manifest on disk is migrated in-memory to
/// v3 with empty extension/receipt maps and its cost ledger preserved, and a
/// v3 manifest resumes as generation 0: its log up to the last complete
/// record, and the cost its ledger holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphCheckpointManifest {
    /// On-disk schema version.
    pub schema_version: u32,
    /// Plan/graph identifier.
    pub plan_id: String,
    /// BLAKE3 of the execution-relevant converted graph definition.
    pub graph_fingerprint: String,
    /// Run ID stored in every Activity record.
    pub run_id: String,
    /// Activity log filename, relative to this manifest.
    pub activity_log: String,
    /// Actual-provider-cost ledger filename, relative to this manifest.
    #[serde(default)]
    pub cost_ledger: String,
    /// Last known run state.
    pub status: GraphCheckpointStatus,
    /// Last manifest update as Unix milliseconds.
    pub updated_at_ms: u128,
    /// The generation this manifest commits; 0 before generations (schema
    /// v2 or v3).
    #[serde(default)]
    pub generation: u64,
    /// What the generation commits; `None` only before generations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub committed: Option<CommittedGeneration>,
    /// Namespaced extension map. Keys are `<namespace>@<schema_version>`.
    #[serde(default)]
    pub extensions: BTreeMap<String, CheckpointExtension>,
    /// Idempotent receipt ledger. Keys are stable idempotency keys.
    #[serde(default)]
    pub receipts: BTreeMap<String, ReceiptLedgerEntry>,
}

/// Resolved files for one plan's Graph checkpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphCheckpointPaths {
    /// Versioned JSON manifest.
    pub manifest: PathBuf,
    /// Append-only Activity output log.
    pub activities: PathBuf,
    /// Atomically replaced actual-provider-cost state.
    pub costs: PathBuf,
}

impl GraphCheckpointPaths {
    /// Gate feedback pending for the plan's next task attempts, kept beside
    /// the manifest: `retry-feedback.json` in a checkpoint directory,
    /// `<stem>.retry-feedback.json` beside a checkpoint file.
    #[must_use]
    pub fn retry_feedback(&self) -> PathBuf {
        let in_directory = self
            .activities
            .file_name()
            .is_some_and(|name| name == "activities.jsonl");
        let stem = self.manifest.file_stem().and_then(|stem| stem.to_str());
        let file_name = match stem {
            Some(stem) if !in_directory => format!("{stem}.retry-feedback.json"),
            _ => "retry-feedback.json".to_string(),
        };
        self.manifest.with_file_name(file_name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GraphCostLedgerState {
    schema_version: u32,
    plan_id: String,
    graph_fingerprint: String,
    run_id: String,
    spent_micro_usd: u64,
    reserved_micro_usd: u64,
}

/// Durable actual-provider-cost state bound to one Graph checkpoint identity.
#[derive(Debug, Clone)]
pub struct GraphCostLedgerCheckpoint {
    path: PathBuf,
    identity: GraphCostLedgerState,
    /// The checkpoint's commit point: each persist commits a generation.
    commits: Option<Arc<CheckpointCommits>>,
}

impl GraphCostLedgerCheckpoint {
    /// Restored actual spend in millionths of one USD.
    #[must_use]
    pub const fn spent_micro_usd(&self) -> u64 {
        self.identity.spent_micro_usd
    }

    /// Persist the latest actual spend atomically, then commit it as the
    /// checkpoint's next generation (gap-dc1d16).
    pub(crate) fn persist(&self, spent_micro_usd: u64, reserved_micro_usd: u64) -> Result<()> {
        let mut state = self.identity.clone();
        state.spent_micro_usd = spent_micro_usd;
        state.reserved_micro_usd = reserved_micro_usd;
        write_cost_ledger_atomic(&self.path, &state)?;
        match &self.commits {
            Some(commits) => commits.commit_cost(spent_micro_usd, reserved_micro_usd),
            None => Ok(()),
        }
    }

    /// Load the ledger at `path` for `manifest`'s checkpoint, with the cost a
    /// resume takes: what the manifest's generation committed, or the
    /// ledger's own before generations.
    fn load(
        path: PathBuf,
        manifest: &GraphCheckpointManifest,
        graph: &GraphIdentity,
    ) -> Result<GraphCostLedgerCheckpoint> {
        let bytes = std::fs::read(&path)
            .with_context(|| format!("read Graph cost ledger {}", path.display()))?;
        let mut state: GraphCostLedgerState = serde_json::from_slice(&bytes)
            .with_context(|| format!("parse Graph cost ledger {}", path.display()))?;
        if let Some(committed) = &manifest.committed {
            state.spent_micro_usd = committed.spent_micro_usd;
            state.reserved_micro_usd = committed.reserved_micro_usd;
        }
        validate_cost_ledger_state(&state, manifest, graph)
            .with_context(|| format!("validate Graph cost ledger {}", path.display()))?;
        Ok(Self {
            path,
            identity: state,
            commits: None,
        })
    }

    /// Set aside a cost the ledger file holds beyond the committed one, which
    /// was written after the last commit: the file is copied to
    /// `<ledger>.uncommitted.<unix ms>` and rewritten with the committed
    /// cost. Returns the copy, or `None` when the file holds the committed
    /// cost.
    fn set_aside_uncommitted(&self) -> Result<Option<PathBuf>> {
        let bytes = std::fs::read(&self.path)
            .with_context(|| format!("read Graph cost ledger {}", self.path.display()))?;
        let stored: GraphCostLedgerState = serde_json::from_slice(&bytes)
            .with_context(|| format!("parse Graph cost ledger {}", self.path.display()))?;
        if stored == self.identity {
            return Ok(None);
        }
        let mut aside = self.path.as_os_str().to_owned();
        aside.push(format!(".uncommitted.{}", unix_ms()));
        let aside = PathBuf::from(aside);
        std::fs::File::create_new(&aside)
            .and_then(|mut file| std::io::Write::write_all(&mut file, &bytes))
            .with_context(|| format!("set aside Graph cost ledger as {}", aside.display()))?;
        write_cost_ledger_atomic(&self.path, &self.identity)?;
        Ok(Some(aside))
    }

    /// Persist `graph_fingerprint` as the graph this ledger belongs to.
    fn rebind(&mut self, graph_fingerprint: &str) -> Result<()> {
        self.identity.graph_fingerprint = graph_fingerprint.to_string();
        write_cost_ledger_atomic(&self.path, &self.identity)
    }
}

/// The commit point of one Graph checkpoint (gap-dc1d16), shared by the
/// run's Activity recorder, its cost ledger and its own manifest writes.
///
/// Each commit writes the manifest last, naming the next generation: after
/// the recorder appended and synced a record, after the cost ledger was
/// written, or when the run changes the manifest itself.
#[derive(Debug)]
struct CheckpointCommits {
    /// The manifest file.
    path: PathBuf,
    state: parking_lot::Mutex<CommitState>,
}

#[derive(Debug)]
struct CommitState {
    /// The manifest as last committed.
    manifest: GraphCheckpointManifest,
    /// What the next commit commits.
    committed: CommittedGeneration,
    /// BLAKE3 state of the Activity log's committed prefix.
    activity_hash: blake3::Hasher,
}

/// The commit points of this process's open checkpoints, by manifest path,
/// so that a forced exit marks a running checkpoint interrupted through its
/// commit point, and a commit that lands later keeps that status (see
/// [`mark_running_checkpoint_interrupted`]).
static OPEN_COMMITS: std::sync::Mutex<BTreeMap<PathBuf, Weak<CheckpointCommits>>> =
    std::sync::Mutex::new(BTreeMap::new());

fn open_commits() -> std::sync::MutexGuard<'static, BTreeMap<PathBuf, Weak<CheckpointCommits>>> {
    OPEN_COMMITS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl CheckpointCommits {
    /// Open the commit point of the checkpoint whose manifest is `path`, as
    /// last committed in `manifest`, with `committed` and `activity_hash` for
    /// the Activity log's committed prefix.
    fn open(
        path: PathBuf,
        manifest: GraphCheckpointManifest,
        committed: CommittedGeneration,
        activity_hash: blake3::Hasher,
    ) -> Arc<Self> {
        let commits = Arc::new(Self {
            path: path.clone(),
            state: parking_lot::Mutex::new(CommitState {
                manifest,
                committed,
                activity_hash,
            }),
        });
        let mut open = open_commits();
        open.retain(|_, weak| weak.strong_count() > 0);
        open.insert(path, Arc::downgrade(&commits));
        commits
    }

    /// What the last commit committed.
    fn committed(&self) -> CommittedGeneration {
        self.state.lock().committed.clone()
    }

    /// Commit `manifest`, the run's own copy, as the next generation, and
    /// bring the copy's generation fields up to date.
    fn commit_manifest(&self, manifest: &mut GraphCheckpointManifest) -> Result<()> {
        let mut guard = self.state.lock();
        let state = &mut *guard;
        let generation = state.manifest.generation;
        state.manifest.clone_from(manifest);
        state.manifest.generation = generation;
        let written = self.write(state);
        manifest.clone_from(&state.manifest);
        written
    }

    /// Commit a record the Activity recorder appended and synced.
    fn commit_record(&self, record: &[u8]) -> Result<()> {
        let mut guard = self.state.lock();
        let state = &mut *guard;
        state.activity_hash.update(record);
        state.committed.activity_bytes += record.len() as u64;
        state.committed.activity_blake3 = state.activity_hash.finalize().to_hex().to_string();
        self.write(state)
    }

    /// Commit the cost the cost ledger now holds.
    fn commit_cost(&self, spent_micro_usd: u64, reserved_micro_usd: u64) -> Result<()> {
        let mut guard = self.state.lock();
        let state = &mut *guard;
        state.committed.spent_micro_usd = spent_micro_usd;
        state.committed.reserved_micro_usd = reserved_micro_usd;
        self.write(state)
    }

    /// Mark the checkpoint `interrupted` by the stop request `by` if it still
    /// reads `running`, as the next generation. Returns whether it was
    /// marked.
    fn mark_interrupted(&self, by: &str) -> Result<bool> {
        let mut guard = self.state.lock();
        let state = &mut *guard;
        if state.manifest.status != GraphCheckpointStatus::Running {
            return Ok(false);
        }
        state.manifest.status = GraphCheckpointStatus::Interrupted;
        set_stop_cause(&mut state.manifest, Some(by))?;
        self.write(state)?;
        Ok(true)
    }

    /// Write the manifest as the next generation.
    fn write(&self, state: &mut CommitState) -> Result<()> {
        let manifest = &mut state.manifest;
        manifest.schema_version = CHECKPOINT_SCHEMA_VERSION;
        manifest.generation += 1;
        manifest.committed = Some(state.committed.clone());
        manifest.updated_at_ms = unix_ms();
        write_manifest_atomic(&self.path, manifest)
    }
}

/// A recorder commit hook that commits each record at `commits`.
fn commit_hook(
    commits: &Arc<CheckpointCommits>,
) -> impl Fn(&[u8]) -> std::io::Result<()> + Send + Sync + 'static {
    let commits = Arc::clone(commits);
    move |record: &[u8]| {
        commits
            .commit_record(record)
            .map_err(|error| std::io::Error::other(format!("{error:#}")))
    }
}

/// Recorder/replayer pair prepared for one Graph execution.
pub struct PreparedGraphCheckpoint {
    paths: GraphCheckpointPaths,
    manifest: GraphCheckpointManifest,
    /// The checkpoint's commit point, shared with the recorder and the cost
    /// ledger (gap-dc1d16).
    commits: Arc<CheckpointCommits>,
    recorder: Option<ActivityRecorder>,
    replayer: Option<ActivityReplayer>,
    replayed_entries: usize,
    cost_ledger: Option<GraphCostLedgerCheckpoint>,
    invalidated_on_resume: Vec<InvalidatedActivity>,
    /// The run's safety provenance sink, whose summary every manifest write
    /// stores (gap-ff95f5).
    safety_provenance: Option<Arc<GraphProvenanceSink>>,
}

impl std::fmt::Debug for PreparedGraphCheckpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedGraphCheckpoint")
            .field("paths", &self.paths)
            .field("replayed_entries", &self.replayed_entries)
            .finish_non_exhaustive()
    }
}

impl PreparedGraphCheckpoint {
    /// Run ID shared by telemetry, the manifest, and JSONL records.
    #[must_use]
    pub fn run_id(&self) -> &str {
        &self.manifest.run_id
    }

    /// Number of previously completed Activities available for replay.
    #[must_use]
    pub const fn replayed_entries(&self) -> usize {
        self.replayed_entries
    }

    /// Resolved checkpoint paths.
    #[must_use]
    pub const fn paths(&self) -> &GraphCheckpointPaths {
        &self.paths
    }

    /// Move the prepared recorder into a Graph Engine.
    pub fn take_recorder(&mut self) -> ActivityRecorder {
        self.recorder
            .take()
            .expect("Graph checkpoint recorder may only be taken once")
    }

    /// Move the optional replayer into a Graph Engine.
    pub fn take_replayer(&mut self) -> Option<ActivityReplayer> {
        self.replayer.take()
    }

    /// Move the restored durable cost ledger into the task dispatcher.
    pub fn take_cost_ledger(&mut self) -> GraphCostLedgerCheckpoint {
        self.cost_ledger
            .take()
            .expect("Graph cost ledger may only be taken once")
    }

    /// Persist the terminal state after Graph execution.
    pub fn finish(&mut self, succeeded: bool) -> Result<()> {
        self.finish_with_status(if succeeded {
            GraphCheckpointStatus::Succeeded
        } else {
            GraphCheckpointStatus::Failed
        })
    }

    /// Persist an explicit terminal state, e.g. `Interrupted` after a signal.
    pub fn finish_with_status(&mut self, status: GraphCheckpointStatus) -> Result<()> {
        self.manifest.status = status;
        // Best-effort: a verdict summary failure must not block the terminal write.
        if let Err(error) = self.refresh_gate_verdicts() {
            tracing::warn!(%error, "gate verdict checkpoint summary refresh failed");
        }
        if let Err(error) = self.refresh_safety_provenance() {
            tracing::warn!(%error, "safety provenance checkpoint summary refresh failed");
        }
        self.commit_manifest()
    }

    /// Commit the manifest as the checkpoint's next generation (gap-dc1d16).
    fn commit_manifest(&mut self) -> Result<()> {
        self.commits.commit_manifest(&mut self.manifest)
    }

    /// The Activity log's records that count, as of the last commit: the
    /// committed ones, less those a resume refused.
    fn counted_records(&self) -> Result<Vec<RecordEntry>> {
        let committed = self.commits.committed();
        counted_records(
            &self.paths.activities,
            &self.manifest.plan_id,
            &self.manifest.run_id,
            Some(&committed),
        )
        .with_context(|| {
            format!(
                "read Graph Activity checkpoint {}",
                self.paths.activities.display()
            )
        })
    }

    /// Last persisted lifecycle state.
    #[must_use]
    pub const fn status(&self) -> GraphCheckpointStatus {
        self.manifest.status
    }

    /// Records refused for replay by this resume; their nodes re-run.
    #[must_use]
    pub fn invalidated_activities(&self) -> &[InvalidatedActivity] {
        &self.invalidated_on_resume
    }

    /// Record how this run left the tasks it did not complete, replacing the
    /// previous run's record. The next terminal write persists it.
    pub fn record_task_outcomes(&mut self, summary: &TaskOutcomeSummary) -> Result<()> {
        if summary == &TaskOutcomeSummary::default() {
            self.manifest.extensions.remove(TASK_OUTCOME_EXTENSION);
            return Ok(());
        }
        let value = serde_json::to_value(summary).context("serialize task outcomes")?;
        self.manifest.extensions.insert(
            TASK_OUTCOME_EXTENSION.to_string(),
            host_extension(TASK_OUTCOME_EXTENSION, value)?,
        );
        Ok(())
    }

    /// Record what stopped this run (`by`, e.g. `deadline`) under
    /// [`STOP_EXTENSION`], or that nothing did, replacing the previous run's
    /// record (gap-fab2cc). The next terminal write persists it.
    pub fn record_stop_cause(&mut self, by: Option<&str>) -> Result<()> {
        set_stop_cause(&mut self.manifest, by)
    }

    /// Record how the plan was delivered into its run's batch branch
    /// (spec-f830c4): `batch` under [`BATCH_EXTENSION`], and the whole
    /// `receipt` under [`DELIVERY_EXTENSION`] so a resume can continue that
    /// delivery. The next terminal write persists them.
    pub fn record_batch_delivery(
        &mut self,
        batch: serde_json::Value,
        receipt: &roko_graph::delivery::CompletionDeliveryReceiptV1,
    ) -> Result<()> {
        let mut delivery = roko_graph::delivery::delivery_extension_value(receipt);
        delivery["receipt"] =
            serde_json::to_value(receipt).context("serialize delivery receipt")?;
        for (key, value) in [(DELIVERY_EXTENSION, delivery), (BATCH_EXTENSION, batch)] {
            self.manifest
                .extensions
                .insert(key.to_string(), host_extension(key, value)?);
        }
        Ok(())
    }

    /// Record the plan's whole-plan check (gap-60233f) under
    /// [`PLAN_VERIFY_EXTENSION`]. The next terminal write persists it.
    pub fn record_plan_verify(&mut self, value: serde_json::Value) -> Result<()> {
        self.manifest.extensions.insert(
            PLAN_VERIFY_EXTENSION.to_string(),
            host_extension(PLAN_VERIFY_EXTENSION, value)?,
        );
        Ok(())
    }

    /// The delivery receipt an earlier process of this checkpoint recorded
    /// with [`Self::record_batch_delivery`], if any.
    #[must_use]
    pub fn recorded_delivery(&self) -> Option<roko_graph::delivery::CompletionDeliveryReceiptV1> {
        let receipt = self
            .manifest
            .extensions
            .get(DELIVERY_EXTENSION)?
            .value
            .get("receipt")?;
        serde_json::from_value(receipt.clone()).ok()
    }

    /// Decode the persisted [`GATE_VERDICT_EXTENSION`] summary, if any.
    #[must_use]
    pub fn gate_verdicts(&self) -> Option<GateVerdictSummary> {
        self.manifest
            .extensions
            .get(GATE_VERDICT_EXTENSION)
            .and_then(|extension| serde_json::from_value(extension.value.clone()).ok())
    }

    /// The gate verdict of each task output recorded so far, replayed outputs
    /// included. The engine records a node's output before it reports the
    /// node complete, so a completed task's verdict is already here. A record
    /// a resume refused does not count.
    #[must_use]
    pub fn recorded_gate_verdicts(&self) -> BTreeMap<String, TaskGateVerdict> {
        match self.counted_records() {
            Ok(records) => recorded_gate_verdicts(&records),
            Err(error) => {
                tracing::warn!(
                    error = %format!("{error:#}"),
                    "recorded gate verdicts unreadable"
                );
                BTreeMap::new()
            }
        }
    }

    /// Rebuild the gate-verdict extension from the durable Activity log.
    ///
    /// The host-owned summary is replaced wholesale, so this bypasses the
    /// write-once fingerprint guard of [`Self::register_extension`].
    fn refresh_gate_verdicts(&mut self) -> Result<()> {
        let records = self.counted_records()?;
        let summary = GateVerdictSummary {
            verdicts: recorded_gate_verdicts(&records),
            invalidated_on_resume: self.invalidated_on_resume.clone(),
        };
        if summary == GateVerdictSummary::default() {
            // Nothing verified or invalidated: keep the manifest free of an
            // empty summary (and drop a stale one).
            self.manifest.extensions.remove(GATE_VERDICT_EXTENSION);
            return Ok(());
        }
        let extension = gate_verdict_extension(&summary)?;
        self.manifest
            .extensions
            .insert(GATE_VERDICT_EXTENSION.to_string(), extension);
        Ok(())
    }

    /// Record under [`INTERRUPTED_ATTEMPT_EXTENSION`] the tasks whose latest
    /// attempt of this run a stop cut off, read from the run's attempt log in
    /// `workdir`, replacing the previous resume's record (gap-36f3fb).
    fn record_interrupted_attempts(&mut self, workdir: &Path) -> Result<()> {
        let layout = RokoLayout::for_project(workdir);
        let run_dir = layout.run_dir(&self.manifest.run_id);
        let recorded = recorded_nodes(&self.counted_records()?);
        let attempts = interrupted_attempts(&run_dir, &self.manifest.plan_id, &recorded);
        if attempts.is_empty() {
            self.manifest
                .extensions
                .remove(INTERRUPTED_ATTEMPT_EXTENSION);
            return Ok(());
        }
        for attempt in &attempts {
            tracing::warn!(
                task_id = %attempt.task_id,
                attempt_key = %attempt.attempt_key,
                cancelled = attempt.cancelled,
                "resume: the task's last attempt stopped before it settled; anything it wrote \
                 is still in the task's checkout, and the task runs again on top of it"
            );
        }
        let value = serde_json::to_value(InterruptedAttempts { attempts })
            .context("serialize interrupted attempts")?;
        self.manifest.extensions.insert(
            INTERRUPTED_ATTEMPT_EXTENSION.to_string(),
            host_extension(INTERRUPTED_ATTEMPT_EXTENSION, value)?,
        );
        Ok(())
    }

    // ---- Extension registration ----

    /// Register a namespaced extension in the checkpoint manifest.
    ///
    /// The extension map key is `<namespace>@<schema_version>`. Duplicate
    /// registration with the same fingerprint is a no-op success. Duplicate
    /// registration with a different fingerprint fails closed.
    ///
    /// # Errors
    ///
    /// Returns an error if an extension with the same key but a different
    /// fingerprint is already registered.
    pub fn register_extension(&mut self, ext: CheckpointExtension) -> Result<()> {
        let key = format!("{}@{}", ext.namespace, ext.schema_version);
        if let Some(existing) = self.manifest.extensions.get(&key) {
            if existing.fingerprint != ext.fingerprint {
                bail!(
                    "extension '{key}' already registered with fingerprint '{}', \
                     cannot re-register with different fingerprint '{}'",
                    existing.fingerprint,
                    ext.fingerprint
                );
            }
            // Same fingerprint: idempotent success.
            return Ok(());
        }
        self.manifest.extensions.insert(key, ext);
        Ok(())
    }

    /// Look up a registered extension by its full key (`namespace@version`).
    #[must_use]
    pub fn extension(&self, key: &str) -> Option<&CheckpointExtension> {
        self.manifest.extensions.get(key)
    }

    /// Return a read-only view of all registered extensions.
    #[must_use]
    pub fn extensions(&self) -> &BTreeMap<String, CheckpointExtension> {
        &self.manifest.extensions
    }

    // ---- Receipt ledger ----

    /// Prepare a new receipt in the ledger.
    ///
    /// If a receipt with this idempotency key already exists and is in the
    /// same state, this is a no-op success. If it exists in a later state,
    /// the existing receipt is returned (forward-only transitions).
    ///
    /// # Errors
    ///
    /// Returns an error only if the ledger invariants are violated (which
    /// should not happen in normal operation).
    pub fn prepare_receipt(
        &mut self,
        idempotency_key: String,
        owner: String,
        correlation_id: String,
    ) -> Result<&ReceiptLedgerEntry> {
        let now = unix_ms();
        if self.manifest.receipts.contains_key(&idempotency_key) {
            // Already at or past Prepared: idempotent success.
            return Ok(self.manifest.receipts.get(&idempotency_key).unwrap());
        }
        let entry = ReceiptLedgerEntry {
            idempotency_key: idempotency_key.clone(),
            owner,
            correlation_id,
            state: ReceiptState::Prepared,
            evidence_ref: None,
            updated_at_ms: now,
            last_error: None,
        };
        self.manifest
            .receipts
            .insert(idempotency_key.clone(), entry);
        Ok(self.manifest.receipts.get(&idempotency_key).unwrap())
    }

    /// Transition a receipt from `Prepared` to `Committed`.
    ///
    /// Repeating `commit_receipt` on an already-`Committed` or `Settled`
    /// receipt is a no-op success. Calling it on a nonexistent receipt or
    /// attempting a reverse transition fails closed.
    ///
    /// # Errors
    ///
    /// Returns an error if the receipt does not exist or a reverse transition
    /// is attempted.
    pub fn commit_receipt(
        &mut self,
        idempotency_key: &str,
        evidence_ref: Option<String>,
    ) -> Result<&ReceiptLedgerEntry> {
        let entry = self
            .manifest
            .receipts
            .get_mut(idempotency_key)
            .ok_or_else(|| anyhow::anyhow!("receipt '{idempotency_key}' not found in ledger"))?;

        match entry.state {
            ReceiptState::Prepared => {
                entry.state = ReceiptState::Committed;
                entry.evidence_ref = evidence_ref;
                entry.updated_at_ms = unix_ms();
                entry.last_error = None;
            }
            ReceiptState::Committed | ReceiptState::Settled => {
                // Already at or past Committed: idempotent success.
            }
        }

        Ok(self
            .manifest
            .receipts
            .get(idempotency_key)
            .expect("entry exists"))
    }

    /// Transition a receipt from `Committed` to `Settled`.
    ///
    /// Repeating `settle_receipt` on an already-`Settled` receipt is a no-op.
    /// Calling it on a `Prepared` receipt (skipping `Committed`) fails closed.
    ///
    /// # Errors
    ///
    /// Returns an error if the receipt does not exist, is still `Prepared`
    /// (skipped transition), or the idempotency key is unknown.
    pub fn settle_receipt(&mut self, idempotency_key: &str) -> Result<&ReceiptLedgerEntry> {
        let entry = self
            .manifest
            .receipts
            .get_mut(idempotency_key)
            .ok_or_else(|| anyhow::anyhow!("receipt '{idempotency_key}' not found in ledger"))?;

        match entry.state {
            ReceiptState::Prepared => {
                bail!(
                    "cannot settle receipt '{idempotency_key}': still in Prepared state \
                     (must commit first)"
                );
            }
            ReceiptState::Committed => {
                entry.state = ReceiptState::Settled;
                entry.updated_at_ms = unix_ms();
                entry.last_error = None;
            }
            ReceiptState::Settled => {
                // Already settled: idempotent success.
            }
        }

        Ok(self
            .manifest
            .receipts
            .get(idempotency_key)
            .expect("entry exists"))
    }

    /// Look up a receipt by its idempotency key.
    #[must_use]
    pub fn receipt(&self, idempotency_key: &str) -> Option<&ReceiptLedgerEntry> {
        self.manifest.receipts.get(idempotency_key)
    }

    /// Return a read-only view of all receipts in the ledger.
    #[must_use]
    pub fn receipts(&self) -> &BTreeMap<String, ReceiptLedgerEntry> {
        &self.manifest.receipts
    }

    /// Record an error on a receipt without changing its state.
    ///
    /// This is used by the reconciliation owner when a `Prepared` receipt's
    /// external call fails: the error is stored, and the receipt remains
    /// `Prepared` for retry.
    pub fn record_receipt_error(
        &mut self,
        idempotency_key: &str,
        error: impl Into<String>,
    ) -> Result<()> {
        let entry = self
            .manifest
            .receipts
            .get_mut(idempotency_key)
            .ok_or_else(|| anyhow::anyhow!("receipt '{idempotency_key}' not found in ledger"))?;
        entry.last_error = Some(error.into());
        entry.updated_at_ms = unix_ms();
        Ok(())
    }

    /// Persist the current manifest (with extensions and receipts) atomically.
    ///
    /// Call this after registering extensions or transitioning receipts to
    /// make the change durable before the next external call.
    pub fn persist_manifest(&mut self) -> Result<()> {
        self.refresh_safety_provenance()?;
        self.commit_manifest()
    }

    /// Keep the run's safety provenance (gap-ff95f5): every later manifest
    /// write stores `sink`'s summary under [`EXT_SAFETY_PROVENANCE`].
    pub fn attach_safety_provenance(&mut self, sink: Arc<GraphProvenanceSink>) {
        self.safety_provenance = Some(sink);
    }

    /// The run's safety provenance as the checkpoint stored it under
    /// [`EXT_SAFETY_PROVENANCE`]; `None` when it stored none, as for a fresh
    /// run or a checkpoint from before gap-ff95f5.
    ///
    /// # Errors
    ///
    /// Fails closed on a version of the extension this build cannot read, or
    /// a value it cannot decode.
    pub fn stored_safety_provenance(&self) -> Result<Option<SafetyProvenanceSummary>> {
        let unknown = self.manifest.extensions.iter().find(|(key, extension)| {
            extension.namespace == SAFETY_PROVENANCE_NAMESPACE
                && (key.as_str() != EXT_SAFETY_PROVENANCE || extension.schema_version != 1)
        });
        if let Some((key, _)) = unknown {
            bail!("safety provenance: this build cannot read the checkpoint's `{key}`");
        }
        let Some(extension) = self.manifest.extensions.get(EXT_SAFETY_PROVENANCE) else {
            return Ok(None);
        };
        serde_json::from_value(extension.value.clone())
            .map(Some)
            .context("safety provenance: decode the checkpoint's summary")
    }

    /// Open the run's safety provenance sink, attach it, and store its
    /// summary (gap-ff95f5). A run whose checkpoint stored no provenance
    /// starts fresh, whatever history the workspace's logs hold. A resumed
    /// run's sink comes back from what the checkpoint stored, its own records
    /// checked against the witness and custody logs, so call this before any
    /// task runs; see [`GraphProvenanceSink::resume`].
    ///
    /// # Errors
    ///
    /// Fails closed when the stored provenance or the run's records do not
    /// check out.
    pub fn open_safety_provenance(&mut self, workdir: &Path) -> Result<Arc<GraphProvenanceSink>> {
        let sink = match self.stored_safety_provenance()? {
            Some(stored) => GraphProvenanceSink::resume(workdir, &self.manifest.run_id, &stored)?,
            None => GraphProvenanceSink::start(workdir)?,
        };
        let sink = Arc::new(sink);
        self.attach_safety_provenance(Arc::clone(&sink));
        self.persist_manifest()?;
        Ok(sink)
    }

    /// Rebuild the [`EXT_SAFETY_PROVENANCE`] extension from the attached
    /// sink, replacing what an earlier write stored. Without a sink the
    /// extension stays as it is.
    fn refresh_safety_provenance(&mut self) -> Result<()> {
        let Some(sink) = &self.safety_provenance else {
            return Ok(());
        };
        let value = serde_json::to_value(sink.summary()).context("serialize safety provenance")?;
        self.manifest.extensions.insert(
            EXT_SAFETY_PROVENANCE.to_string(),
            host_extension(EXT_SAFETY_PROVENANCE, value)?,
        );
        Ok(())
    }
}

/// Prepare a fresh or resumed checkpoint for one converted plan graph.
///
/// `requested_path` behaves as a checkpoint directory unless it has a `.json`
/// extension.  A file path is only unambiguous for a single-plan invocation.
/// The CLI's historical bare `--resume-plan` default is mapped to the canonical
/// Graph checkpoint root instead of overwriting Runner v2's executor snapshot.
pub fn prepare_graph_checkpoint(
    workdir: &Path,
    requested_path: Option<&Path>,
    plan_id: &str,
    plan_count: usize,
    graph: &Graph,
    fresh: bool,
    force_resume: bool,
) -> Result<PreparedGraphCheckpoint> {
    prepare_graph_checkpoint_for_run(
        workdir,
        requested_path,
        plan_id,
        plan_count,
        graph,
        fresh,
        force_resume,
        None,
    )
}

/// [`prepare_graph_checkpoint`] for a caller whose run already has an id: a
/// fresh checkpoint is named `run_id` instead of a minted
/// `graph-<plan>-<uuid>`, so the run keeps one id and one directory under
/// `.roko/runs` (`roko run` passes its own, bug-ccc7c4). A resumed
/// checkpoint keeps the run it recorded.
pub fn prepare_graph_checkpoint_for_run(
    workdir: &Path,
    requested_path: Option<&Path>,
    plan_id: &str,
    plan_count: usize,
    graph: &Graph,
    fresh: bool,
    force_resume: bool,
    run_id: Option<&str>,
) -> Result<PreparedGraphCheckpoint> {
    let paths = resolve_checkpoint_paths(workdir, requested_path, plan_id, plan_count)?;
    let identity = GraphIdentity::of(workdir, graph)?;

    if fresh {
        archive_checkpoint_files(&paths)?;
    }

    if !fresh && paths.manifest.exists() {
        let mut manifest = read_manifest(&paths.manifest)?;
        match checkpoint_match(&manifest, plan_id, &identity, &paths) {
            Err(reason) => {
                if !force_resume {
                    bail!(
                        "cannot resume Graph checkpoint {}: {reason}; use --fresh to archive it or --force-resume to start a new run",
                        paths.manifest.display()
                    );
                }
                archive_checkpoint_files(&paths)?;
            }
            Ok(matched) => {
                if (!paths.activities.is_file() || !paths.costs.is_file()) && !force_resume {
                    bail!(
                        "Graph checkpoint {} references missing Activity log or cost ledger ({}, {}); use --fresh or --force-resume",
                        paths.manifest.display(),
                        paths.activities.display(),
                        paths.costs.display(),
                    );
                }
                if !paths.activities.is_file() || !paths.costs.is_file() {
                    archive_checkpoint_files(&paths)?;
                    return create_fresh_checkpoint(paths, plan_id, identity.current, run_id);
                }
                let mut cost_ledger = match GraphCostLedgerCheckpoint::load(
                    paths.costs.clone(),
                    &manifest,
                    &identity,
                ) {
                    Ok(cost_ledger) => cost_ledger,
                    Err(_) if force_resume => {
                        archive_checkpoint_files(&paths)?;
                        return create_fresh_checkpoint(paths, plan_id, identity.current, run_id);
                    }
                    Err(error) => return Err(error),
                };
                // Everything a resume takes from the checkpoint is read and
                // checked before any of its files changes.
                let selected = select_generation(&paths, &manifest, &cost_ledger.identity, graph)?;
                if matched == FingerprintMatch::Legacy {
                    tracing::info!(
                        plan_id,
                        manifest = %paths.manifest.display(),
                        "resuming a Graph checkpoint recorded with the legacy graph fingerprint; \
                         recording the authored plan fingerprint"
                    );
                    // Ledger first: validation accepts either fingerprint, so
                    // a crash before the manifest write stays resumable.
                    cost_ledger.rebind(&identity.current)?;
                    manifest.graph_fingerprint.clone_from(&identity.current);
                } else {
                    tracing::debug!(
                        plan_id,
                        fingerprint_match = matched.as_str(),
                        "resuming Graph checkpoint"
                    );
                }
                return resume_checkpoint(workdir, paths, manifest, cost_ledger, selected);
            }
        }
    } else if !fresh && (paths.activities.exists() || paths.costs.exists()) {
        if !force_resume {
            bail!(
                "found Graph Activity log or cost ledger without its manifest ({}, {}); use --fresh or --force-resume",
                paths.activities.display(),
                paths.costs.display(),
            );
        }
        archive_checkpoint_files(&paths)?;
    }

    create_fresh_checkpoint(paths, plan_id, identity.current, run_id)
}

/// The generation a resume of a checkpoint selects, read without changing a
/// file (gap-dc1d16; see [`select_generation`]).
struct SelectedGeneration {
    /// What the generation committed, with this resume's refusals. A
    /// manifest from before generations is generation 0: its log up to the
    /// last complete record, and the cost its ledger holds.
    committed: CommittedGeneration,
    /// The generation's records to replay.
    replayer: ActivityReplayer,
    /// Refused records whose nodes this resume runs again.
    invalidated: Vec<InvalidatedActivity>,
    /// BLAKE3 state of the committed prefix, which the run's commits extend.
    activity_hash: blake3::Hasher,
}

/// Select the generation `manifest` names for a resume of `graph`, with
/// `ledger` as [`GraphCostLedgerCheckpoint::load`] read it. Fails closed when
/// the Activity log lost or changed committed bytes, or a committed line is
/// not a record of the run.
fn select_generation(
    paths: &GraphCheckpointPaths,
    manifest: &GraphCheckpointManifest,
    ledger: &GraphCostLedgerState,
    graph: &Graph,
) -> Result<SelectedGeneration> {
    let log = std::fs::read(&paths.activities).with_context(|| {
        format!(
            "read Graph Activity checkpoint {}",
            paths.activities.display()
        )
    })?;
    let mut committed = manifest
        .committed
        .clone()
        .unwrap_or_else(|| CommittedGeneration {
            activity_bytes: complete_records_len(&log),
            spent_micro_usd: ledger.spent_micro_usd,
            reserved_micro_usd: ledger.reserved_micro_usd,
            ..CommittedGeneration::default()
        });
    let prefix = usize::try_from(committed.activity_bytes)
        .ok()
        .and_then(|len| log.get(..len))
        .with_context(|| {
            format!(
                "Graph Activity checkpoint {} holds {} bytes, fewer than the {} its generation {} committed",
                paths.activities.display(),
                log.len(),
                committed.activity_bytes,
                manifest.generation
            )
        })?;
    let mut activity_hash = blake3::Hasher::new();
    activity_hash.update(prefix);
    let digest = activity_hash.finalize().to_hex().to_string();
    if manifest.committed.is_some() && digest != committed.activity_blake3 {
        bail!(
            "the committed records of Graph Activity checkpoint {} changed after generation {} committed them; use --fresh to archive it",
            paths.activities.display(),
            manifest.generation
        );
    }
    committed.activity_blake3 = digest;
    let load = || {
        format!(
            "load Graph Activity checkpoint {}",
            paths.activities.display()
        )
    };
    let records = activity_records(prefix, &manifest.plan_id, &manifest.run_id)
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(load)?;
    let (refused, invalidated) =
        invalidate_unverified_activities(&records, graph, &committed.refused_records);
    committed.refused_records = refused;
    let replayer =
        ActivityReplayer::from_records(records, &committed.refused_records).with_context(load)?;
    Ok(SelectedGeneration {
        committed,
        replayer,
        invalidated,
        activity_hash,
    })
}

/// Reopen a validated checkpoint for another run of the same plan graph, at
/// the generation `selected` names (gap-dc1d16).
fn resume_checkpoint(
    workdir: &Path,
    paths: GraphCheckpointPaths,
    manifest: GraphCheckpointManifest,
    mut cost_ledger: GraphCostLedgerCheckpoint,
    selected: SelectedGeneration,
) -> Result<PreparedGraphCheckpoint> {
    let SelectedGeneration {
        committed,
        replayer,
        invalidated,
        activity_hash,
    } = selected;
    // Bytes past the committed prefix were never committed, such as a record
    // a crash tore or one whose commit did not land: set them aside rather
    // than fail on them or replay them, so the log ends in its last committed
    // record and those nodes run again.
    let uncommitted = set_aside_activities_after(&paths.activities, committed.activity_bytes)
        .with_context(|| {
            format!(
                "set aside the uncommitted end of {}",
                paths.activities.display()
            )
        })?;
    if let Some(aside) = uncommitted {
        tracing::warn!(
            activities = %paths.activities.display(),
            set_aside = %aside.display(),
            generation = manifest.generation,
            "resume: the Activity log holds records the checkpoint never committed; \
             set them aside, and their nodes run again"
        );
    }
    // So was a cost the ledger holds beyond the committed one.
    if let Some(aside) = cost_ledger.set_aside_uncommitted()? {
        tracing::warn!(
            costs = %paths.costs.display(),
            set_aside = %aside.display(),
            spent_micro_usd = committed.spent_micro_usd,
            "resume: the cost ledger holds spend the checkpoint never committed; \
             set it aside, and the run resumes from the committed spend"
        );
    }
    for activity in &invalidated {
        tracing::warn!(
            node_id = %activity.node_id,
            tick = activity.tick,
            reason = %activity.reason,
            "resume: recorded task output is not verified; the node will re-run"
        );
    }
    let commits = CheckpointCommits::open(
        paths.manifest.clone(),
        manifest.clone(),
        committed,
        activity_hash,
    );
    cost_ledger.commits = Some(Arc::clone(&commits));
    let recorder = ActivityRecorder::create(&manifest.run_id, &paths.activities)
        .with_context(|| {
            format!(
                "open Graph Activity checkpoint {}",
                paths.activities.display()
            )
        })?
        .with_commit_hook(commit_hook(&commits));
    let replayed_entries = replayer.entry_count();
    let mut prepared = PreparedGraphCheckpoint {
        paths,
        manifest,
        commits,
        recorder: Some(recorder),
        replayer: Some(replayer),
        replayed_entries,
        cost_ledger: Some(cost_ledger),
        invalidated_on_resume: invalidated,
        safety_provenance: None,
    };
    prepared.manifest.status = GraphCheckpointStatus::Running;
    prepared.refresh_gate_verdicts()?;
    prepared.record_interrupted_attempts(workdir)?;
    prepared.commit_manifest()?;
    Ok(prepared)
}

fn create_fresh_checkpoint(
    paths: GraphCheckpointPaths,
    plan_id: &str,
    fingerprint: String,
    run_id: Option<&str>,
) -> Result<PreparedGraphCheckpoint> {
    let parent = paths
        .manifest
        .parent()
        .context("Graph checkpoint manifest has no parent directory")?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("create Graph checkpoint directory {}", parent.display()))?;
    let run_id = run_id.map_or_else(
        || format!("graph-{plan_id}-{}", uuid::Uuid::new_v4()),
        str::to_string,
    );
    let activity_log = paths
        .activities
        .file_name()
        .and_then(|name| name.to_str())
        .context("Graph Activity checkpoint filename is not valid UTF-8")?
        .to_string();
    let cost_ledger = paths
        .costs
        .file_name()
        .and_then(|name| name.to_str())
        .context("Graph cost ledger filename is not valid UTF-8")?
        .to_string();
    let manifest = GraphCheckpointManifest {
        schema_version: CHECKPOINT_SCHEMA_VERSION,
        plan_id: plan_id.to_string(),
        graph_fingerprint: fingerprint.clone(),
        run_id: run_id.clone(),
        activity_log,
        cost_ledger,
        status: GraphCheckpointStatus::Running,
        updated_at_ms: unix_ms(),
        generation: 0,
        committed: None,
        extensions: BTreeMap::new(),
        receipts: BTreeMap::new(),
    };
    let committed = CommittedGeneration {
        activity_blake3: blake3::hash(b"").to_hex().to_string(),
        ..CommittedGeneration::default()
    };
    let commits = CheckpointCommits::open(
        paths.manifest.clone(),
        manifest.clone(),
        committed,
        blake3::Hasher::new(),
    );
    let recorder = ActivityRecorder::create_fresh(&run_id, &paths.activities)
        .with_context(|| format!("create Graph Activity log {}", paths.activities.display()))?
        .with_commit_hook(commit_hook(&commits));
    let cost_ledger = GraphCostLedgerCheckpoint {
        path: paths.costs.clone(),
        identity: GraphCostLedgerState {
            schema_version: COST_LEDGER_SCHEMA_VERSION,
            plan_id: plan_id.to_string(),
            graph_fingerprint: fingerprint.clone(),
            run_id,
            spent_micro_usd: 0,
            reserved_micro_usd: 0,
        },
        commits: Some(Arc::clone(&commits)),
    };
    write_cost_ledger_atomic(&cost_ledger.path, &cost_ledger.identity)?;
    let mut prepared = PreparedGraphCheckpoint {
        paths,
        manifest,
        commits,
        recorder: Some(recorder),
        replayer: None,
        replayed_entries: 0,
        cost_ledger: Some(cost_ledger),
        invalidated_on_resume: Vec::new(),
        safety_provenance: None,
    };
    prepared.commit_manifest()?;
    Ok(prepared)
}

fn resolve_checkpoint_paths(
    workdir: &Path,
    requested_path: Option<&Path>,
    plan_id: &str,
    plan_count: usize,
) -> Result<GraphCheckpointPaths> {
    let canonical_graph_root = workdir.join(".roko/state/graph");
    let requested = requested_path.map(|path| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            workdir.join(path)
        }
    });
    let is_runner_default = |path: &Path| {
        DEFAULT_RUNNER_RESUME_PATHS
            .iter()
            .any(|default| path == workdir.join(default))
    };
    let base = match requested {
        None => canonical_graph_root,
        Some(path) if is_runner_default(&path) => canonical_graph_root,
        Some(path) => path,
    };

    let is_manifest = base
        .extension()
        .is_some_and(|extension| extension == "json")
        && !base.is_dir();
    if is_manifest {
        if plan_count != 1 {
            bail!(
                "a Graph checkpoint file can only resume one plan; pass a directory when running {plan_count} plans"
            );
        }
        let stem = base
            .file_stem()
            .and_then(|stem| stem.to_str())
            .context("Graph checkpoint filename is not valid UTF-8")?;
        let activities = base.with_file_name(format!("{stem}.activities.jsonl"));
        let costs = base.with_file_name(format!("{stem}.costs.json"));
        return Ok(GraphCheckpointPaths {
            manifest: base,
            activities,
            costs,
        });
    }

    let plan_dir = base.join(safe_plan_component(plan_id));
    Ok(GraphCheckpointPaths {
        manifest: plan_dir.join("checkpoint.json"),
        activities: plan_dir.join("activities.jsonl"),
        costs: plan_dir.join("costs.json"),
    })
}

/// Read a checkpoint manifest, migrating a v2 manifest to v3 in memory.
///
/// A manifest from before generations commits none, whatever generation
/// fields its file holds: a resume stamps it as generation 0 from the files.
/// A v4 manifest that names no committed generation fails closed.
fn read_manifest(path: &Path) -> Result<GraphCheckpointManifest> {
    let bytes =
        std::fs::read(path).with_context(|| format!("read Graph checkpoint {}", path.display()))?;
    let mut manifest: GraphCheckpointManifest = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse Graph checkpoint {}", path.display()))?;
    // In-memory v2 -> v3 migration: add empty extensions/receipts, bump
    // version. The upgraded manifest is written on the next atomic commit.
    if manifest.schema_version == 2 {
        migrate_v2_to_v3(&mut manifest);
    }
    match manifest.schema_version {
        PRE_GENERATION_SCHEMA_VERSION => {
            manifest.generation = 0;
            manifest.committed = None;
        }
        CHECKPOINT_SCHEMA_VERSION if manifest.committed.is_none() => bail!(
            "Graph checkpoint {} has schema version {CHECKPOINT_SCHEMA_VERSION} but names no committed generation",
            path.display()
        ),
        _ => {}
    }
    Ok(manifest)
}

/// Which of the graph's fingerprints `manifest` recorded, or why it cannot
/// resume this plan.
fn checkpoint_match(
    manifest: &GraphCheckpointManifest,
    plan_id: &str,
    graph: &GraphIdentity,
    paths: &GraphCheckpointPaths,
) -> Result<FingerprintMatch, String> {
    // Accept v2 and v3, which resume as generation 0, and v4.
    if manifest.schema_version < MIN_SUPPORTED_SCHEMA_VERSION
        || manifest.schema_version > CHECKPOINT_SCHEMA_VERSION
    {
        return Err(format!(
            "schema version {} is unsupported (expected {MIN_SUPPORTED_SCHEMA_VERSION}..={CHECKPOINT_SCHEMA_VERSION})",
            manifest.schema_version
        ));
    }
    if manifest.plan_id != plan_id {
        return Err(format!(
            "manifest is for plan '{}' rather than '{plan_id}'",
            manifest.plan_id
        ));
    }
    let matched = graph.matching(&manifest.graph_fingerprint);
    if matched == FingerprintMatch::Mismatch {
        return Err("the converted plan graph has changed".to_string());
    }
    if paths.activities.file_name().and_then(|name| name.to_str())
        != Some(manifest.activity_log.as_str())
    {
        return Err("manifest references a different Activity log".to_string());
    }
    if paths.costs.file_name().and_then(|name| name.to_str()) != Some(manifest.cost_ledger.as_str())
    {
        return Err("manifest references a different cost ledger".to_string());
    }
    Ok(matched)
}

/// Migrate a v2 manifest to v3 in-memory by adding empty extension and receipt
/// maps while preserving all existing fields including the cost ledger.
fn migrate_v2_to_v3(manifest: &mut GraphCheckpointManifest) {
    debug_assert_eq!(manifest.schema_version, 2);
    manifest.schema_version = PRE_GENERATION_SCHEMA_VERSION;
    // extensions and receipts default to empty BTreeMap via serde(default),
    // so a v2 deserialized manifest already has them empty. We just bump the
    // version to signal the next atomic write should use v3 format.
}

fn validate_cost_ledger_state(
    state: &GraphCostLedgerState,
    manifest: &GraphCheckpointManifest,
    graph: &GraphIdentity,
) -> Result<()> {
    if state.schema_version != COST_LEDGER_SCHEMA_VERSION {
        bail!(
            "cost ledger schema version {} is unsupported (expected {COST_LEDGER_SCHEMA_VERSION})",
            state.schema_version
        );
    }
    if state.plan_id != manifest.plan_id {
        bail!(
            "cost ledger is for plan '{}' rather than '{}'",
            state.plan_id,
            manifest.plan_id
        );
    }
    // A legacy-fingerprint resume rebinds the ledger before the manifest, so
    // either of the graph's fingerprints identifies it.
    if graph.matching(&state.graph_fingerprint) == FingerprintMatch::Mismatch {
        bail!("cost ledger graph fingerprint does not match the plan graph");
    }
    if state.run_id != manifest.run_id {
        bail!("cost ledger run ID does not match the checkpoint manifest");
    }
    // The cost a resume takes (see `GraphCostLedgerCheckpoint::load`).
    if state.reserved_micro_usd > 0 {
        bail!(
            "the checkpoint committed {} unresolved reserved micro-USD from an interrupted provider call",
            state.reserved_micro_usd
        );
    }
    Ok(())
}

fn archive_checkpoint_files(paths: &GraphCheckpointPaths) -> Result<()> {
    let timestamp = unix_ms();
    for path in [&paths.manifest, &paths.activities, &paths.costs] {
        if path.exists() {
            let extension = path
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or("state");
            let backup = path.with_extension(format!("{extension}.bak.{timestamp}"));
            std::fs::rename(path, &backup).with_context(|| {
                format!(
                    "archive Graph checkpoint {} to {}",
                    path.display(),
                    backup.display()
                )
            })?;
        }
    }
    Ok(())
}

fn write_cost_ledger_atomic(path: &Path, state: &GraphCostLedgerState) -> Result<()> {
    let parent = path
        .parent()
        .context("Graph cost ledger has no parent directory")?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("create Graph checkpoint directory {}", parent.display()))?;
    let temporary = path.with_extension(format!("json.tmp.{}", uuid::Uuid::new_v4()));
    let bytes = serde_json::to_vec_pretty(state).context("serialize Graph cost ledger")?;
    std::fs::write(&temporary, bytes)
        .with_context(|| format!("write Graph cost ledger {}", temporary.display()))?;
    std::fs::rename(&temporary, path)
        .with_context(|| format!("commit Graph cost ledger {}", path.display()))
}

fn write_manifest_atomic(path: &Path, manifest: &GraphCheckpointManifest) -> Result<()> {
    let parent = path
        .parent()
        .context("Graph checkpoint manifest has no parent directory")?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("create Graph checkpoint directory {}", parent.display()))?;
    let temporary = path.with_extension(format!("json.tmp.{}", uuid::Uuid::new_v4()));
    let bytes = serde_json::to_vec_pretty(manifest).context("serialize Graph checkpoint")?;
    std::fs::write(&temporary, bytes)
        .with_context(|| format!("write Graph checkpoint {}", temporary.display()))?;
    std::fs::rename(&temporary, path)
        .with_context(|| format!("commit Graph checkpoint {}", path.display()))
}

/// The batch branch recorded in plan `plan_id`'s checkpoint under
/// [`BATCH_EXTENSION`] (spec-f830c4), if any.
#[must_use]
pub fn recorded_batch_branch(workdir: &Path, plan_id: &str) -> Option<String> {
    let manifest = workdir
        .join(".roko/state/graph")
        .join(safe_plan_component(plan_id))
        .join("checkpoint.json");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(manifest).ok()?).ok()?;
    manifest["extensions"][BATCH_EXTENSION]["value"]["branch"]
        .as_str()
        .map(ToOwned::to_owned)
}

/// A plan's delivery into its run's batch branch, as its checkpoint recorded
/// it (see [`recorded_batch_delivery`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedBatchDelivery {
    /// The run's batch branch, `roko/batch/<run-id>`.
    pub branch: String,
    /// The batch commit that delivered the plan's work.
    pub merge_commit: String,
}

/// Where plan `plan_id`'s work went: the batch branch and commit its
/// checkpoint recorded under [`BATCH_EXTENSION`] when the plan was delivered
/// (gap-4ec59f). `None` when it was not.
#[must_use]
pub fn recorded_batch_delivery(workdir: &Path, plan_id: &str) -> Option<RecordedBatchDelivery> {
    let manifest = workdir
        .join(".roko/state/graph")
        .join(safe_plan_component(plan_id))
        .join("checkpoint.json");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(manifest).ok()?).ok()?;
    let batch = &manifest["extensions"][BATCH_EXTENSION]["value"];
    if batch["state"].as_str() != Some("delivered") {
        return None;
    }
    Some(RecordedBatchDelivery {
        branch: batch["branch"].as_str()?.to_string(),
        merge_commit: batch["merge_commit"].as_str()?.to_string(),
    })
}

/// Why plan `plan_id`'s whole-plan check failed, as its checkpoint recorded
/// it: the failed `[meta] verify` step in the shared working tree, or the
/// failed delivery into the run's batch branch, whose regression check runs
/// those steps. `None` when it passed or never ran.
#[must_use]
pub fn recorded_plan_check_failure(workdir: &Path, plan_id: &str) -> Option<String> {
    let manifest = workdir
        .join(".roko/state/graph")
        .join(safe_plan_component(plan_id))
        .join("checkpoint.json");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(manifest).ok()?).ok()?;
    let extensions = &manifest["extensions"];
    let verify = &extensions[PLAN_VERIFY_EXTENSION]["value"]["failure"];
    if let Some(command) = verify["command"].as_str() {
        let output = verify["output"].as_str().unwrap_or_default();
        return Some(format!("`{command}` failed: {output}"));
    }
    let batch = &extensions[BATCH_EXTENSION]["value"];
    match batch["state"].as_str() {
        Some("delivered") | None => None,
        Some(state) => Some(format!(
            "delivery into {} ended {state}: {}",
            batch["branch"].as_str().unwrap_or("the batch branch"),
            batch["error"].as_str().unwrap_or("no reason recorded")
        )),
    }
}

/// Last status recorded in `plan_id`'s canonical checkpoint under
/// `.roko/state/graph/`, or `None` when it has no readable checkpoint.
#[must_use]
pub fn canonical_checkpoint_status(workdir: &Path, plan_id: &str) -> Option<GraphCheckpointStatus> {
    #[derive(Deserialize)]
    struct StatusOnly {
        status: GraphCheckpointStatus,
    }

    let manifest = workdir
        .join(".roko/state/graph")
        .join(safe_plan_component(plan_id))
        .join("checkpoint.json");
    let bytes = std::fs::read(manifest).ok()?;
    serde_json::from_slice::<StatusOnly>(&bytes)
        .ok()
        .map(|manifest| manifest.status)
}

/// Task outcomes recorded by the last run in `plan_id`'s canonical
/// checkpoint under `.roko/state/graph/`, or `None` when there are none.
#[must_use]
pub fn canonical_task_outcomes(workdir: &Path, plan_id: &str) -> Option<TaskOutcomeSummary> {
    #[derive(Deserialize)]
    struct ExtensionsOnly {
        #[serde(default)]
        extensions: BTreeMap<String, CheckpointExtension>,
    }

    let manifest = workdir
        .join(".roko/state/graph")
        .join(safe_plan_component(plan_id))
        .join("checkpoint.json");
    let bytes = std::fs::read(manifest).ok()?;
    let manifest = serde_json::from_slice::<ExtensionsOnly>(&bytes).ok()?;
    serde_json::from_value(
        manifest
            .extensions
            .get(TASK_OUTCOME_EXTENSION)?
            .value
            .clone(),
    )
    .ok()
}

/// Every canonical checkpoint under `.roko/state/graph/`: the plan id its
/// manifest records, with the manifest's path. Unreadable manifests are left
/// out. `roko backlog audit` finds checkpoints whose plan is gone with this.
#[must_use]
pub fn canonical_checkpoint_plans(workdir: &Path) -> Vec<(String, PathBuf)> {
    #[derive(Deserialize)]
    struct PlanIdOnly {
        plan_id: String,
    }

    let Ok(entries) = std::fs::read_dir(workdir.join(".roko/state/graph")) else {
        return Vec::new();
    };
    let mut plans: Vec<(String, PathBuf)> = entries
        .flatten()
        .map(|entry| entry.path().join("checkpoint.json"))
        .filter_map(|manifest| {
            let bytes = std::fs::read(&manifest).ok()?;
            let plan = serde_json::from_slice::<PlanIdOnly>(&bytes).ok()?;
            Some((plan.plan_id, manifest))
        })
        .collect();
    plans.sort();
    plans
}

/// The run one checkpoint manifest under `.roko/state/graph/` names, as
/// [`recorded_checkpoint_runs`] reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedCheckpointRun {
    /// The plan's directory under `.roko/state/graph/`.
    pub plan_dir: String,
    /// The run the manifest names.
    pub run_id: String,
    /// The status the manifest records.
    pub status: GraphCheckpointStatus,
    /// Whether this is the plan's current checkpoint rather than one a later
    /// run archived when it replaced it.
    pub current: bool,
}

/// The run of every checkpoint manifest under `.roko/state/graph/`: each
/// plan's current one, and the ones archived when a later run replaced them.
/// Unreadable manifests are left out. `roko doctor disk --fix` finds with
/// this which plan an attempt checkout's run belonged to (gap-f67a72).
#[must_use]
pub fn recorded_checkpoint_runs(workdir: &Path) -> Vec<RecordedCheckpointRun> {
    #[derive(Deserialize)]
    struct RunOnly {
        run_id: String,
        status: GraphCheckpointStatus,
    }

    let mut runs = Vec::new();
    let plans = std::fs::read_dir(workdir.join(".roko/state/graph"));
    for plan in plans.into_iter().flatten().flatten() {
        let plan_dir = plan.file_name().to_string_lossy().into_owned();
        let manifests = std::fs::read_dir(plan.path());
        for manifest in manifests.into_iter().flatten().flatten() {
            let name = manifest.file_name().to_string_lossy().into_owned();
            let current = name == "checkpoint.json";
            if !current && !name.starts_with("checkpoint.json.bak.") {
                continue;
            }
            let Ok(bytes) = std::fs::read(manifest.path()) else {
                continue;
            };
            let Ok(recorded) = serde_json::from_slice::<RunOnly>(&bytes) else {
                continue;
            };
            runs.push(RecordedCheckpointRun {
                plan_dir: plan_dir.clone(),
                run_id: recorded.run_id,
                status: recorded.status,
                current,
            });
        }
    }
    runs
}

// ---------------------------------------------------------------------------
// Inspection
// ---------------------------------------------------------------------------

/// A plan's canonical Graph checkpoint as last written, read without
/// validating or changing it. `roko diagnose` reports from this.
#[derive(Debug, Clone)]
pub struct CheckpointInspection {
    /// Files of the checkpoint.
    pub paths: GraphCheckpointPaths,
    /// The manifest; a v2 manifest is migrated in memory.
    pub manifest: GraphCheckpointManifest,
    /// Actual provider spend, in millionths of one USD, as the checkpoint
    /// committed it (the cost ledger's own before generations). `None` when
    /// a checkpoint from before generations has no readable ledger.
    pub spent_micro_usd: Option<u64>,
    /// Spend reserved for a provider call that never settled. Resume refuses
    /// a checkpoint that committed an unresolved reservation.
    pub reserved_micro_usd: Option<u64>,
    /// Task nodes whose output this run recorded in its Activity log, each
    /// with the gate verdict of its latest record. A record a resume refused
    /// does not count.
    pub recorded: BTreeMap<String, Option<TaskGateVerdict>>,
    /// Records the last resume refused because they lacked a passing verdict.
    pub invalidated_on_resume: Vec<InvalidatedActivity>,
    /// When the newest checkpoint this one replaced was archived, as Unix
    /// milliseconds: records older than this belong to earlier runs of the
    /// plan. `None` when no earlier checkpoint was archived.
    pub replaced_at_ms: Option<u128>,
}

/// Read `plan_id`'s canonical checkpoint under `.roko/state/graph/` for
/// inspection, or `None` when it has no manifest.
///
/// # Errors
///
/// Returns an error when the manifest exists but cannot be read or parsed.
pub fn inspect_canonical_checkpoint(
    workdir: &Path,
    plan_id: &str,
) -> Result<Option<CheckpointInspection>> {
    let paths = resolve_checkpoint_paths(workdir, None, plan_id, 1)?;
    if !paths.manifest.is_file() {
        return Ok(None);
    }
    let manifest = read_manifest(&paths.manifest)?;
    // The cost a resume takes: what the generation committed, or the ledger's
    // own before generations.
    let cost = match &manifest.committed {
        Some(committed) => Some((committed.spent_micro_usd, committed.reserved_micro_usd)),
        None => std::fs::read(&paths.costs)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<GraphCostLedgerState>(&bytes).ok())
            .map(|ledger| (ledger.spent_micro_usd, ledger.reserved_micro_usd)),
    };
    let recorded = match counted_records(
        &paths.activities,
        &manifest.plan_id,
        &manifest.run_id,
        manifest.committed.as_ref(),
    ) {
        Ok(records) => recorded_outputs(&records),
        Err(error) => {
            tracing::warn!(
                activities = %paths.activities.display(),
                %error,
                "the Activity log does not read; no task output counts as recorded"
            );
            BTreeMap::new()
        }
    };
    let invalidated_on_resume = manifest
        .extensions
        .get(GATE_VERDICT_EXTENSION)
        .and_then(|extension| {
            serde_json::from_value::<GateVerdictSummary>(extension.value.clone()).ok()
        })
        .map(|summary| summary.invalidated_on_resume)
        .unwrap_or_default();
    Ok(Some(CheckpointInspection {
        recorded,
        replaced_at_ms: latest_archive_ms(&paths),
        spent_micro_usd: cost.map(|(spent, _)| spent),
        reserved_micro_usd: cost.map(|(_, reserved)| reserved),
        invalidated_on_resume,
        manifest,
        paths,
    }))
}

/// Node ids `records` hold, each with the gate verdict of its latest record.
fn recorded_outputs(records: &[RecordEntry]) -> BTreeMap<String, Option<TaskGateVerdict>> {
    records
        .iter()
        .map(|entry| {
            let verdict = TaskGateVerdict::from_signals(&entry.signals);
            (entry.node_id.clone(), verdict)
        })
        .collect()
}

/// Newest timestamp [`archive_checkpoint_files`] gave any of `paths`' files.
fn latest_archive_ms(paths: &GraphCheckpointPaths) -> Option<u128> {
    let prefixes: Vec<String> = [&paths.manifest, &paths.activities, &paths.costs]
        .into_iter()
        .filter_map(|path| {
            path.file_name()?
                .to_str()
                .map(|name| format!("{name}.bak."))
        })
        .collect();
    std::fs::read_dir(paths.manifest.parent()?)
        .ok()?
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter_map(|name| {
            prefixes
                .iter()
                .find_map(|prefix| name.strip_prefix(prefix.as_str())?.parse::<u128>().ok())
        })
        .max()
}

/// Mark the checkpoint whose manifest is `manifest` `interrupted` by the stop
/// request `by` if it still reads `running`, as a plan run forced out before
/// it could write its own terminal status does on its way out (bug-4641e3),
/// so the checkpoint does not look alive afterwards. A checkpoint its run
/// already finalized keeps its status. Returns whether it was marked.
pub fn mark_running_checkpoint_interrupted(manifest: &Path, by: &str) -> Result<bool> {
    // A checkpoint this process has open is marked through its commit point,
    // so a commit that lands later keeps the status (gap-dc1d16).
    let open = open_commits().get(manifest).and_then(Weak::upgrade);
    if let Some(commits) = open {
        return commits.mark_interrupted(by);
    }
    let mut recorded = read_manifest(manifest)?;
    if recorded.status != GraphCheckpointStatus::Running {
        return Ok(false);
    }
    recorded.status = GraphCheckpointStatus::Interrupted;
    set_stop_cause(&mut recorded, Some(by))?;
    recorded.updated_at_ms = unix_ms();
    write_manifest_atomic(manifest, &recorded)?;
    Ok(true)
}

/// Set `manifest`'s [`STOP_EXTENSION`] to `by`, or remove it.
fn set_stop_cause(manifest: &mut GraphCheckpointManifest, by: Option<&str>) -> Result<()> {
    match by {
        Some(by) => {
            let value = serde_json::json!({ "by": by });
            manifest.extensions.insert(
                STOP_EXTENSION.to_string(),
                host_extension(STOP_EXTENSION, value)?,
            );
        }
        None => {
            manifest.extensions.remove(STOP_EXTENSION);
        }
    }
    Ok(())
}

/// What stopped the last run recorded in `plan_id`'s canonical checkpoint
/// under `.roko/state/graph/`, as [`STOP_EXTENSION`] holds it; `None` when
/// nothing did or the checkpoint cannot be read.
#[must_use]
pub fn canonical_stop_cause(workdir: &Path, plan_id: &str) -> Option<String> {
    let manifest = workdir
        .join(".roko/state/graph")
        .join(safe_plan_component(plan_id))
        .join("checkpoint.json");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(manifest).ok()?).ok()?;
    manifest["extensions"][STOP_EXTENSION]["value"]["by"]
        .as_str()
        .map(ToOwned::to_owned)
}

/// Start `plan`'s canonical checkpoint the way a default `plan run` does.
#[cfg(test)]
pub(crate) fn start_plan_checkpoint(
    workdir: &Path,
    plan: &Plan,
) -> Result<PreparedGraphCheckpoint> {
    let graph = convert_plan(plan, &plan_task_infos(plan, |task| task.max_retries))?;
    prepare_graph_checkpoint(workdir, None, &plan.id, 1, &graph, false, false)
}

// ---------------------------------------------------------------------------
// Resume preview
// ---------------------------------------------------------------------------

/// `plan run` options that decide what happens to a plan's Graph checkpoint.
#[derive(Debug, Clone, Copy, Default)]
pub struct ResumeOptions<'a> {
    /// `--resume-plan`: checkpoint root directory, or one plan's manifest.
    pub resume_plan: Option<&'a Path>,
    /// `--fresh`: archive existing checkpoints and run every task.
    pub fresh: bool,
    /// `--force-resume`: archive an unusable checkpoint instead of stopping.
    pub force_resume: bool,
    /// `--max-tasks`: concurrency override; 0 keeps `[meta] max_parallel`.
    /// It caps the run without changing its checkpoint identity
    /// (gap-7147bb), so the preview converts the plan without it.
    pub max_tasks: usize,
    /// `--max-retries`: per-task retry override.
    pub max_retries: Option<u32>,
    /// `--rich-topology`: convert through the production topology.
    pub rich_topology: bool,
}

/// What a plan run does with a plan's Graph checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResumeAction {
    /// No checkpoint exists: every task runs.
    Start,
    /// The checkpoint resumes: restored tasks replay their recorded outputs.
    Resume,
    /// The checkpoint is archived and every task runs.
    Archive,
    /// The checkpoint cannot be used and the run stops with an error.
    Refuse,
}

/// What `plan run` would do with one plan's Graph checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResumePreview {
    /// Checkpoint manifest the run would use.
    pub manifest: PathBuf,
    /// Status recorded in the manifest.
    pub status: Option<GraphCheckpointStatus>,
    /// How the recorded fingerprint relates to the plan.
    pub fingerprint_match: Option<FingerprintMatch>,
    /// What the run would do with the checkpoint.
    pub action: ResumeAction,
    /// Why the checkpoint would be migrated, archived, or refused.
    pub reason: Option<String>,
    /// Tasks whose recorded outputs would be replayed instead of run.
    pub restored_tasks: Vec<String>,
    /// Tasks that would run.
    pub tasks_to_run: Vec<String>,
}

impl ResumePreview {
    /// Lines describing the preview for `plan run --dry-run`.
    #[must_use]
    pub fn describe(&self, workdir: &Path) -> Vec<String> {
        let manifest = self
            .manifest
            .strip_prefix(workdir)
            .unwrap_or(&self.manifest)
            .display();
        let facts: Vec<String> = self
            .fingerprint_match
            .map(|matched| format!("fingerprint match: {}", matched.as_str()))
            .into_iter()
            .chain(
                self.status
                    .map(|status| format!("status: {}", status.as_str())),
            )
            .collect();
        let facts = if facts.is_empty() {
            String::new()
        } else {
            format!(" ({})", facts.join(", "))
        };
        let reason = self
            .reason
            .as_deref()
            .map(|reason| format!(": {reason}"))
            .unwrap_or_default();
        let mut lines = vec![match self.action {
            ResumeAction::Start => "checkpoint: none, every task runs".to_string(),
            ResumeAction::Resume => format!("checkpoint: resumable{facts} {manifest}{reason}"),
            ResumeAction::Archive => {
                format!("checkpoint: archived{facts}, every task runs{reason}")
            }
            ResumeAction::Refuse => {
                format!("checkpoint: not resumable{facts}, the run stops{reason}")
            }
        }];
        if self.action == ResumeAction::Resume {
            let list = |tasks: &[String]| {
                if tasks.is_empty() {
                    "-".to_string()
                } else {
                    tasks.join(", ")
                }
            };
            lines.push(format!("  restored: {}", list(&self.restored_tasks)));
            lines.push(format!("  run:      {}", list(&self.tasks_to_run)));
        }
        lines
    }

    /// The checkpoint cannot resume: `--force-resume` archives it, otherwise
    /// the run stops.
    fn unusable(mut self, force_resume: bool, reason: impl std::fmt::Display) -> Self {
        (self.action, self.reason) = if force_resume {
            (
                ResumeAction::Archive,
                Some(format!("--force-resume archives it, {reason}")),
            )
        } else {
            (
                ResumeAction::Refuse,
                Some(format!("{reason}; use --fresh or --force-resume")),
            )
        };
        self
    }

    /// The run stops even with `--force-resume`.
    fn refused(mut self, reason: impl std::fmt::Display) -> Self {
        self.action = ResumeAction::Refuse;
        self.reason = Some(format!("{reason}; use --fresh"));
        self
    }
}

/// Report what `plan run` would do with `plan`'s Graph checkpoint, without
/// changing any file.
///
/// # Errors
///
/// Returns an error when the plan cannot be converted to a Graph or its
/// checkpoint location cannot be resolved.
pub fn preview_plan_resume(
    workdir: &Path,
    plan: &Plan,
    plan_count: usize,
    options: &ResumeOptions<'_>,
) -> Result<ResumePreview> {
    if options.rich_topology {
        bail!("the checkpoint preview does not support --rich-topology");
    }
    // A run's retry budgets come from its dispatcher's gate thresholds. They
    // live in the node configs, which the plan fingerprint leaves out.
    let tasks = plan_task_infos(plan, |t| options.max_retries.unwrap_or(t.max_retries));
    let graph = convert_plan(plan, &tasks)?;
    let mut preview = preview_graph_checkpoint(
        workdir,
        options.resume_plan,
        &plan.id,
        plan_count,
        &graph,
        options.fresh,
        options.force_resume,
    )?;
    let position: BTreeMap<&str, usize> = plan
        .tasks
        .tasks
        .iter()
        .enumerate()
        .map(|(index, task)| (task.id.as_str(), index))
        .collect();
    for tasks in [&mut preview.restored_tasks, &mut preview.tasks_to_run] {
        tasks.sort_by_key(|task| position.get(task.as_str()).copied());
    }
    Ok(preview)
}

/// `plan`'s tasks as the Graph converters take them, each with the retry
/// budget `max_retries` gives it. `run_one_plan` and the resume preview both
/// build their graphs from this list, so the preview fingerprints the graph a
/// run executes (gap-be7368).
pub(crate) fn plan_task_infos(
    plan: &Plan,
    mut max_retries: impl FnMut(&TaskDef) -> u32,
) -> Vec<(String, PlanTaskInfo)> {
    plan.tasks
        .tasks
        .iter()
        .map(|task| {
            let info = PlanTaskInfo {
                title: task.title.clone(),
                description: task.description.clone(),
                role: task.role.clone(),
                tier: task.tier.clone(),
                model_hint: task.model_hint.clone(),
                files: task.files.clone(),
                depends_on: task.depends_on.clone(),
                depends_on_plan: task.depends_on_plan.clone(),
                timeout_secs: task.timeout_secs,
                max_retries: max_retries(task),
                domain: task.domain.as_ref().map(|domain| format!("{domain:?}")),
                sequence: task.sequence,
                full_config_json: serde_json::to_value(task).unwrap_or_default(),
            };
            (task.id.clone(), info)
        })
        .collect()
}

/// The concurrency `plan` converts at: its own `max_parallel`, and 1 when it
/// omits it, as before that meant "as wide as the DAG allows" (gap-272448).
/// The checkpoint identity hashes the converted concurrency, so a run applies
/// `--max-tasks` and the DAG width only after taking it (gap-7147bb).
pub(crate) fn converted_max_parallel(plan: &Plan) -> u32 {
    plan.tasks.meta.max_parallel.unwrap_or(1)
}

/// Convert `plan`'s `tasks` ([`plan_task_infos`]) with the default topology,
/// at [`converted_max_parallel`]: the graph `run_one_plan` runs, and the one
/// the resume preview fingerprints.
pub(crate) fn convert_plan(plan: &Plan, tasks: &[(String, PlanTaskInfo)]) -> Result<Graph> {
    plan_to_graph(
        &plan.id,
        &plan.dir.display().to_string(),
        tasks,
        converted_max_parallel(plan),
    )
    .with_context(|| format!("convert plan '{}' to a Graph", plan.id))
}

/// Read-only counterpart of [`prepare_graph_checkpoint`]: what it would do
/// with `plan_id`'s checkpoint for `graph`.
fn preview_graph_checkpoint(
    workdir: &Path,
    requested_path: Option<&Path>,
    plan_id: &str,
    plan_count: usize,
    graph: &Graph,
    fresh: bool,
    force_resume: bool,
) -> Result<ResumePreview> {
    let paths = resolve_checkpoint_paths(workdir, requested_path, plan_id, plan_count)?;
    let identity = GraphIdentity::of(workdir, graph)?;
    let mut task_nodes: Vec<String> = graph
        .inner
        .node_weights()
        .filter(|node| node.cell_type == TASK_EXECUTOR_CELL_TYPE)
        .map(|node| node.id.clone())
        .collect();
    task_nodes.sort();
    let mut preview = ResumePreview {
        manifest: paths.manifest.clone(),
        status: None,
        fingerprint_match: None,
        action: ResumeAction::Start,
        reason: None,
        restored_tasks: Vec::new(),
        tasks_to_run: task_nodes.clone(),
    };

    let manifest = if paths.manifest.exists() {
        match read_manifest(&paths.manifest) {
            Ok(manifest) => {
                preview.status = Some(manifest.status);
                preview.fingerprint_match = Some(identity.matching(&manifest.graph_fingerprint));
                Some(manifest)
            }
            Err(error) if !fresh => return Ok(preview.refused(format!("{error:#}"))),
            Err(_) => None,
        }
    } else {
        None
    };
    if fresh {
        if paths.manifest.exists() || paths.activities.exists() || paths.costs.exists() {
            preview.action = ResumeAction::Archive;
            preview.reason = Some("--fresh archives it".to_string());
        }
        return Ok(preview);
    }
    let Some(manifest) = manifest else {
        if paths.activities.exists() || paths.costs.exists() {
            return Ok(preview.unusable(
                force_resume,
                "found an Activity log or cost ledger without its manifest",
            ));
        }
        return Ok(preview);
    };
    let matched = match checkpoint_match(&manifest, plan_id, &identity, &paths) {
        Ok(matched) => matched,
        Err(reason) => return Ok(preview.unusable(force_resume, reason)),
    };
    if !paths.activities.is_file() || !paths.costs.is_file() {
        return Ok(preview.unusable(
            force_resume,
            "the manifest references a missing Activity log or cost ledger",
        ));
    }
    let ledger = match GraphCostLedgerCheckpoint::load(paths.costs.clone(), &manifest, &identity) {
        Ok(ledger) => ledger,
        Err(error) => return Ok(preview.unusable(force_resume, format!("{error:#}"))),
    };
    // A resume selects the generation the manifest names, so only its
    // committed records count here, less the ones the resume refuses.
    let selected = match select_generation(&paths, &manifest, &ledger.identity, graph) {
        Ok(selected) => selected,
        Err(error) => return Ok(preview.refused(format!("{error:#}"))),
    };
    let (restored, to_run) = task_nodes
        .into_iter()
        .partition(|node| selected.replayer.lookup(node, 0).is_some());
    preview.action = ResumeAction::Resume;
    preview.restored_tasks = restored;
    preview.tasks_to_run = to_run;
    if matched == FingerprintMatch::Legacy {
        preview.reason =
            Some("resuming rewrites it with the authored plan fingerprint".to_string());
    }
    Ok(preview)
}

fn safe_plan_component(plan_id: &str) -> String {
    let safe: String = plan_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect();
    if safe.is_empty() || safe == "." || safe == ".." {
        "plan".to_string()
    } else {
        safe
    }
}

fn unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_graph::{GraphMetadata, Node};
    use tempfile::tempdir;

    fn graph(name: &str, config_value: i64) -> Graph {
        let mut graph = Graph::new(GraphMetadata {
            name: name.to_string(),
            ..GraphMetadata::default()
        });
        graph
            .add_node(Node {
                id: "task-1".to_string(),
                cell_type: "task-executor".to_string(),
                config: toml::Value::Table(toml::map::Map::from_iter([(
                    "value".to_string(),
                    toml::Value::Integer(config_value),
                )])),
                inputs: Vec::new(),
                outputs: Vec::new(),
                execution_class: roko_graph::ExecutionClass::Activity,
                exclusive: Vec::new(),
            })
            .expect("node");
        graph
    }

    /// A task node whose task definition declares one authored verify step.
    fn verify_graph(name: &str) -> Graph {
        let task_def = serde_json::json!({
            "id": "task-1",
            "title": "Implement the trait",
            "verify": [{ "phase": "structural", "command": "grep -q Trait src/lib.rs" }],
        });
        let mut graph = Graph::new(GraphMetadata {
            name: name.to_string(),
            ..GraphMetadata::default()
        });
        graph
            .add_node(Node {
                id: "task-1".to_string(),
                cell_type: "task-executor".to_string(),
                config: toml::Value::Table(toml::map::Map::from_iter([(
                    "task_def_json".to_string(),
                    toml::Value::String(task_def.to_string()),
                )])),
                inputs: Vec::new(),
                outputs: Vec::new(),
                execution_class: roko_graph::ExecutionClass::Activity,
                exclusive: Vec::new(),
            })
            .expect("node");
        graph
    }

    fn verdict_output(text: &str, verdict: Option<TaskGateVerdict>) -> Vec<roko_core::Signal> {
        let mut signals = vec![
            roko_core::Signal::builder(roko_core::Kind::AgentOutput)
                .body(roko_core::Body::text(text))
                .build(),
        ];
        if let Some(verdict) = verdict {
            verdict.stamp(&mut signals);
        }
        signals
    }

    #[test]
    fn only_verify_bearing_task_nodes_require_a_verdict() {
        assert!(verdict_required_nodes(&verify_graph("p")).contains("task-1"));
        assert!(verdict_required_nodes(&graph("p", 1)).is_empty());
    }

    #[test]
    fn resume_reruns_unverified_or_forced_task_records() {
        for verdict in [None, Some(TaskGateVerdict::ForcedAccept)] {
            let dir = tempdir().expect("tempdir");
            let graph = verify_graph("p");
            let mut fresh =
                prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                    .expect("fresh checkpoint");
            fresh
                .take_recorder()
                .record(
                    "p",
                    "task-1",
                    0,
                    verdict_output("BLOCK: not applied", verdict),
                )
                .expect("record");
            fresh.finish(false).expect("finish");

            let mut resumed =
                prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                    .expect("resume checkpoint");
            assert_eq!(resumed.replayed_entries(), 0, "verdict {verdict:?}");
            assert_eq!(resumed.invalidated_activities().len(), 1);
            let replayer = resumed.take_replayer().expect("replayer");
            assert!(replayer.lookup("task-1", 0).is_none());
            let summary = resumed.gate_verdicts().expect("gate verdict extension");
            assert_eq!(summary.invalidated_on_resume[0].node_id, "task-1");

            // The re-run appends a verified record; the next resume replays it
            // without tripping the duplicate-record guard.
            resumed
                .take_recorder()
                .record(
                    "p",
                    "task-1",
                    0,
                    verdict_output("implemented", Some(TaskGateVerdict::Passed)),
                )
                .expect("record re-run");
            resumed.finish(true).expect("finish");
            let again = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                .expect("second resume");
            assert_eq!(again.replayed_entries(), 1);
            assert!(again.invalidated_activities().is_empty());
        }
    }

    #[test]
    fn finish_populates_gate_verdict_extension_from_recorded_outputs() {
        let dir = tempdir().expect("tempdir");
        let graph = verify_graph("p");
        let mut fresh = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("fresh checkpoint");
        fresh
            .take_recorder()
            .record(
                "p",
                "task-1",
                0,
                verdict_output("done", Some(TaskGateVerdict::Passed)),
            )
            .expect("record");
        fresh.finish(true).expect("finish");

        let summary = fresh.gate_verdicts().expect("gate verdict extension");
        assert_eq!(
            summary.verdicts.get("task-1"),
            Some(&TaskGateVerdict::Passed)
        );
        let extension = fresh
            .extension(GATE_VERDICT_EXTENSION)
            .expect("registered under the canonical key");
        assert_eq!(
            format!("{}@{}", extension.namespace, extension.schema_version),
            GATE_VERDICT_EXTENSION
        );

        let resumed = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("resume checkpoint");
        assert_eq!(resumed.replayed_entries(), 1);
    }

    #[test]
    fn fingerprint_changes_with_execution_config() {
        let dir = tempdir().expect("tempdir");
        let first = GraphIdentity::of(dir.path(), &graph("p", 1)).expect("identity");
        // Without a plan file the identity is the legacy Graph fingerprint.
        assert_eq!(first.current, first.legacy);
        assert_ne!(
            first,
            GraphIdentity::of(dir.path(), &graph("p", 2)).expect("identity")
        );
    }

    const PLAN_TOML: &str = r#"
[meta]
plan = "p"

[[task]]
id = "T1"
title = "First"

[[task]]
id = "T2"
title = "Second"
depends_on = ["T1"]
"#;

    /// Write `content` as plan `p`'s tasks.toml under `workdir` and load it.
    fn write_plan(workdir: &Path, content: &str) -> Plan {
        let dir = workdir.join("plans").join("p");
        std::fs::create_dir_all(&dir).expect("plan dir");
        std::fs::write(dir.join("tasks.toml"), content).expect("write tasks.toml");
        Plan {
            id: "p".to_string(),
            dir,
            tasks: TasksFile::parse_str(content).expect("parse tasks.toml"),
            prd_excerpt: String::new(),
        }
    }

    fn plan_graph(plan: &Plan, options: &ResumeOptions<'_>) -> Graph {
        let tasks = plan_task_infos(plan, |t| options.max_retries.unwrap_or(t.max_retries));
        convert_plan(plan, &tasks).expect("convert plan")
    }

    /// Start a run of plan `p`, record T1's output, and stop it as failed.
    fn record_first_task(workdir: &Path, graph: &Graph) -> GraphCheckpointPaths {
        let mut checkpoint = prepare_graph_checkpoint(workdir, None, "p", 1, graph, false, false)
            .expect("fresh checkpoint");
        checkpoint
            .take_recorder()
            .record("p", "T1", 0, Vec::new())
            .expect("record");
        checkpoint.finish(false).expect("finish");
        checkpoint.paths().clone()
    }

    fn recorded_fingerprint(path: &Path) -> String {
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).expect("read")).expect("parse");
        value["graph_fingerprint"]
            .as_str()
            .expect("graph_fingerprint")
            .to_string()
    }

    fn rewrite_fingerprint(path: &Path, fingerprint: &str) {
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).expect("read")).expect("parse");
        value["graph_fingerprint"] = serde_json::Value::String(fingerprint.to_string());
        std::fs::write(path, serde_json::to_vec_pretty(&value).expect("serialize")).expect("write");
    }

    #[test]
    fn plan_checkpoint_records_the_authored_fingerprint() {
        let dir = tempdir().expect("tempdir");
        let plan = write_plan(dir.path(), PLAN_TOML);
        let graph = plan_graph(&plan, &ResumeOptions::default());
        let identity = GraphIdentity::of(dir.path(), &graph).expect("identity");
        let authored = AuthoredPlan::from_tasks_toml(PLAN_TOML).expect("authored plan");
        assert_eq!(
            identity.current,
            plan_graph_fingerprint(&graph, &authored).expect("fingerprint")
        );
        assert_ne!(identity.current, identity.legacy);

        let paths = record_first_task(dir.path(), &graph);
        assert_eq!(recorded_fingerprint(&paths.manifest), identity.current);
        assert_eq!(recorded_fingerprint(&paths.costs), identity.current);
    }

    #[test]
    fn node_config_changes_outside_the_plan_file_keep_the_checkpoint_resumable() {
        // `--max-retries` rewrites every node config, and with it the legacy
        // fingerprint, without changing what the plan's tasks are.
        let dir = tempdir().expect("tempdir");
        let plan = write_plan(dir.path(), PLAN_TOML);
        let original = plan_graph(&plan, &ResumeOptions::default());
        record_first_task(dir.path(), &original);
        let overridden = plan_graph(
            &plan,
            &ResumeOptions {
                max_retries: Some(7),
                ..ResumeOptions::default()
            },
        );
        assert_ne!(
            legacy_graph_execution_fingerprint(&original).expect("fingerprint"),
            legacy_graph_execution_fingerprint(&overridden).expect("fingerprint")
        );

        let resumed = prepare_graph_checkpoint(dir.path(), None, "p", 1, &overridden, false, false)
            .expect("resume with overridden node configs");
        assert_eq!(resumed.replayed_entries(), 1);
    }

    #[test]
    fn legacy_fingerprint_checkpoint_resumes_and_is_rewritten() {
        let dir = tempdir().expect("tempdir");
        let plan = write_plan(dir.path(), PLAN_TOML);
        let graph = plan_graph(&plan, &ResumeOptions::default());
        let identity = GraphIdentity::of(dir.path(), &graph).expect("identity");
        let paths = record_first_task(dir.path(), &graph);
        // An older roko recorded the legacy Graph fingerprint.
        rewrite_fingerprint(&paths.manifest, &identity.legacy);
        rewrite_fingerprint(&paths.costs, &identity.legacy);
        let preview = preview_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("preview");
        assert_eq!(preview.fingerprint_match, Some(FingerprintMatch::Legacy));
        assert_eq!(preview.action, ResumeAction::Resume);

        let resumed = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("resume legacy checkpoint");
        assert_eq!(resumed.replayed_entries(), 1);
        assert_eq!(recorded_fingerprint(&paths.manifest), identity.current);
        assert_eq!(recorded_fingerprint(&paths.costs), identity.current);
        drop(resumed);
        let preview = preview_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("preview");
        assert_eq!(preview.fingerprint_match, Some(FingerprintMatch::New));
    }

    #[test]
    fn migration_interrupted_between_ledger_and_manifest_stays_resumable() {
        let dir = tempdir().expect("tempdir");
        let plan = write_plan(dir.path(), PLAN_TOML);
        let graph = plan_graph(&plan, &ResumeOptions::default());
        let identity = GraphIdentity::of(dir.path(), &graph).expect("identity");
        let paths = record_first_task(dir.path(), &graph);
        // The ledger was rebound; the manifest still holds the legacy value.
        rewrite_fingerprint(&paths.manifest, &identity.legacy);

        let resumed = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("resume half-migrated checkpoint");
        assert_eq!(resumed.replayed_entries(), 1);
        assert_eq!(recorded_fingerprint(&paths.manifest), identity.current);
    }

    #[test]
    fn tasks_toml_edited_after_loading_keeps_the_legacy_identity() {
        let dir = tempdir().expect("tempdir");
        let plan = write_plan(dir.path(), PLAN_TOML);
        let graph = plan_graph(&plan, &ResumeOptions::default());
        std::fs::write(
            plan.dir.join("tasks.toml"),
            PLAN_TOML.replace("Second", "Edited"),
        )
        .expect("edit tasks.toml");

        let identity = GraphIdentity::of(dir.path(), &graph).expect("identity");
        assert_eq!(identity.current, identity.legacy);
    }

    const ACCEPT_PLAN_TOML: &str = r#"
[meta]
plan = "p"

[[task]]
id = "T1"
title = "First"

[task.accept]
files = [
    { src = "accept/t1.test.txt", dest = "src/t1.test.txt", runner = "cat {dest}", count = 1 },
]

[[task.verify]]
phase = "compile"
command = "true"

[[task]]
id = "T2"
title = "Second"
depends_on = ["T1"]
"#;

    /// Write plan `p` with the pinned acceptance test `test` and pin it in the store at `store`.
    fn pinned_accept_plan(workdir: &Path, store: &Path, test: &str) -> Plan {
        let mut plan = write_plan(workdir, ACCEPT_PLAN_TOML);
        std::fs::create_dir_all(plan.dir.join("accept")).expect("accept dir");
        std::fs::write(plan.dir.join("accept").join("t1.test.txt"), test).expect("accept test");
        task_accept::pin_plans_in(
            &task_accept::AcceptStore::at(store),
            std::slice::from_mut(&mut plan),
            workdir,
        )
        .expect("pin acceptance tests");
        plan
    }

    /// bug-b0fd73: a run's tasks carry the steps pinning their `[task.accept]` tests, which the
    /// file does not. An unchanged accept plan still matches its tasks.toml, its identity covers
    /// the pinned hash but not the store's place, and an edited tasks.toml still mismatches.
    #[test]
    fn an_accept_plan_matches_its_own_tasks_toml() {
        let dir = tempdir().expect("tempdir");
        let test = "test result: ok. 1 passed\n";
        let plan = pinned_accept_plan(dir.path(), &dir.path().join("store-a"), test);
        assert!(task_accept::is_pinned_step(&plan.tasks.tasks[0].verify[0]));
        let graph = plan_graph(&plan, &ResumeOptions::default());
        let identity = GraphIdentity::of(dir.path(), &graph).expect("identity");
        assert_ne!(identity.current, identity.legacy, "matches its tasks.toml");

        let elsewhere = pinned_accept_plan(dir.path(), &dir.path().join("store-b"), test);
        let elsewhere_graph = plan_graph(&elsewhere, &ResumeOptions::default());
        let elsewhere = GraphIdentity::of(dir.path(), &elsewhere_graph).expect("identity");
        assert_eq!(
            elsewhere.current, identity.current,
            "the store's place is not identity"
        );
        assert_ne!(elsewhere.legacy, identity.legacy);

        let changed = pinned_accept_plan(
            dir.path(),
            &dir.path().join("store-c"),
            "test result: ok. 2 passed\n",
        );
        let changed_graph = plan_graph(&changed, &ResumeOptions::default());
        let changed = GraphIdentity::of(dir.path(), &changed_graph).expect("identity");
        assert_ne!(
            changed.current, identity.current,
            "the pinned hash is identity"
        );

        std::fs::write(
            plan.dir.join("tasks.toml"),
            ACCEPT_PLAN_TOML.replace("Second", "Edited"),
        )
        .expect("edit tasks.toml");
        let edited = GraphIdentity::of(dir.path(), &graph).expect("identity");
        assert_eq!(
            edited.current, edited.legacy,
            "an edited tasks.toml mismatches"
        );
    }

    #[test]
    fn inspection_reads_this_runs_records_and_the_newest_archive() {
        let dir = tempdir().expect("tempdir");
        assert!(
            inspect_canonical_checkpoint(dir.path(), "p")
                .expect("inspect")
                .is_none()
        );

        let plan = write_plan(dir.path(), PLAN_TOML);
        let paths = record_first_task(dir.path(), &plan_graph(&plan, &ResumeOptions::default()));
        // A record another run left in the log, and two archived runs.
        ActivityRecorder::create("graph-p-earlier", &paths.activities)
            .expect("recorder")
            .record("p", "T2", 0, Vec::new())
            .expect("record");
        let checkpoint_dir = paths.manifest.parent().expect("checkpoint dir");
        std::fs::write(checkpoint_dir.join("checkpoint.json.bak.100"), "{}").expect("archive");
        std::fs::write(checkpoint_dir.join("activities.jsonl.bak.200"), "").expect("archive");

        let inspection = inspect_canonical_checkpoint(dir.path(), "p")
            .expect("inspect")
            .expect("checkpoint");
        assert_eq!(inspection.manifest.status, GraphCheckpointStatus::Failed);
        assert_eq!(inspection.recorded.keys().collect::<Vec<_>>(), ["T1"]);
        assert_eq!(inspection.replaced_at_ms, Some(200));
        assert_eq!(
            (inspection.spent_micro_usd, inspection.reserved_micro_usd),
            (Some(0), Some(0))
        );
    }

    #[test]
    fn preview_reports_restored_and_pending_tasks_without_changing_files() {
        let dir = tempdir().expect("tempdir");
        let plan = write_plan(dir.path(), PLAN_TOML);
        let options = ResumeOptions::default();
        let preview = preview_plan_resume(dir.path(), &plan, 1, &options).expect("preview");
        assert_eq!(preview.action, ResumeAction::Start);
        assert_eq!(preview.tasks_to_run, ["T1", "T2"]);

        let paths = record_first_task(dir.path(), &plan_graph(&plan, &options));
        let files = || {
            [&paths.manifest, &paths.activities, &paths.costs]
                .map(|path| std::fs::read(path).expect("read checkpoint file"))
        };
        let before = files();
        let preview = preview_plan_resume(dir.path(), &plan, 1, &options).expect("preview");
        assert_eq!(preview.action, ResumeAction::Resume);
        assert_eq!(preview.fingerprint_match, Some(FingerprintMatch::New));
        assert_eq!(preview.status, Some(GraphCheckpointStatus::Failed));
        assert_eq!(preview.restored_tasks, ["T1"]);
        assert_eq!(preview.tasks_to_run, ["T2"]);
        assert_eq!(files(), before);

        let fresh = ResumeOptions {
            fresh: true,
            ..options
        };
        let preview = preview_plan_resume(dir.path(), &plan, 1, &fresh).expect("preview");
        assert_eq!(preview.action, ResumeAction::Archive);
        assert_eq!(preview.tasks_to_run, ["T1", "T2"]);
    }

    #[test]
    fn preview_refuses_an_edited_plan_unless_forced() {
        let dir = tempdir().expect("tempdir");
        let plan = write_plan(dir.path(), PLAN_TOML);
        record_first_task(dir.path(), &plan_graph(&plan, &ResumeOptions::default()));
        let edited = write_plan(dir.path(), &PLAN_TOML.replace("Second", "Edited"));

        let preview = preview_plan_resume(dir.path(), &edited, 1, &ResumeOptions::default())
            .expect("preview");
        assert_eq!(preview.fingerprint_match, Some(FingerprintMatch::Mismatch));
        assert_eq!(preview.action, ResumeAction::Refuse);
        assert!(
            preview
                .reason
                .as_deref()
                .is_some_and(|reason| reason.contains("graph has changed"))
        );
        let forced = ResumeOptions {
            force_resume: true,
            ..ResumeOptions::default()
        };
        let preview = preview_plan_resume(dir.path(), &edited, 1, &forced).expect("preview");
        assert_eq!(preview.action, ResumeAction::Archive);
        assert_eq!(preview.tasks_to_run, ["T1", "T2"]);

        let error = prepare_graph_checkpoint(
            dir.path(),
            None,
            "p",
            1,
            &plan_graph(&edited, &ResumeOptions::default()),
            false,
            false,
        )
        .expect_err("an edited plan must not resume");
        assert!(error.to_string().contains("graph has changed"));
    }

    /// gap-7147bb: `--max-tasks` caps a run's concurrency without changing
    /// its checkpoint identity, so a run started with one value resumes with
    /// another, or with none.
    #[test]
    fn resume_accepts_a_different_max_tasks() {
        let dir = tempdir().expect("tempdir");
        let plan = write_plan(dir.path(), PLAN_TOML);
        let with = |max_tasks| ResumeOptions {
            max_tasks,
            ..ResumeOptions::default()
        };
        record_first_task(dir.path(), &plan_graph(&plan, &with(2)));

        for max_tasks in [0, 5] {
            let preview =
                preview_plan_resume(dir.path(), &plan, 1, &with(max_tasks)).expect("preview");
            assert_eq!(preview.fingerprint_match, Some(FingerprintMatch::New));
            assert_eq!(
                preview.action,
                ResumeAction::Resume,
                "--max-tasks {max_tasks}"
            );
            assert_eq!(preview.restored_tasks, ["T1"]);
        }
        let resumed = prepare_graph_checkpoint(
            dir.path(),
            None,
            "p",
            1,
            &plan_graph(&plan, &with(5)),
            false,
            false,
        )
        .expect("resume with another --max-tasks");
        assert_eq!(resumed.replayed_entries(), 1);
    }

    /// gap-be7368: a run and the resume preview convert a plan with one
    /// mapping ([`plan_task_infos`], [`convert_plan`]). Only their retry
    /// budgets differ, and those are not part of the plan's identity.
    #[test]
    fn retry_budgets_leave_the_preview_identity_unchanged() {
        let dir = tempdir().expect("tempdir");
        let plan = write_plan(dir.path(), PLAN_TOML);
        let identity = |budget: u32| {
            let graph =
                convert_plan(&plan, &plan_task_infos(&plan, |_| budget)).expect("convert plan");
            GraphIdentity::of(dir.path(), &graph)
                .expect("identity")
                .current
        };
        assert_eq!(identity(0), identity(4));
    }

    #[test]
    fn fresh_checkpoint_then_resume_roundtrips_run_identity() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut fresh = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("fresh checkpoint");
        let run_id = fresh.run_id().to_string();
        fresh
            .take_recorder()
            .record("p", "task-1", 0, Vec::new())
            .expect("record");
        fresh.finish(false).expect("finish");

        let resumed = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("resume checkpoint");
        assert_eq!(resumed.run_id(), run_id);
        assert_eq!(resumed.replayed_entries(), 1);
    }

    #[test]
    fn actual_provider_cost_roundtrips_across_resume() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut fresh = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("fresh checkpoint");
        let ledger = fresh.take_cost_ledger();
        ledger.persist(375_000, 0).expect("persist cost");
        fresh.finish(false).expect("finish");

        let mut resumed = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("resume checkpoint");

        assert_eq!(resumed.take_cost_ledger().spent_micro_usd(), 375_000);
    }

    #[test]
    fn corrupt_cost_ledger_is_rejected_without_force() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let checkpoint = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("fresh checkpoint");
        let costs = checkpoint.paths().costs.clone();
        drop(checkpoint);
        std::fs::write(&costs, b"not-json").expect("corrupt cost ledger");

        let error = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .err()
            .expect("corrupt cost ledger must fail closed");

        assert!(error.to_string().contains("parse Graph cost ledger"));
    }

    #[test]
    fn mismatched_cost_ledger_identity_is_rejected_without_force() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let checkpoint = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("fresh checkpoint");
        let costs = checkpoint.paths().costs.clone();
        drop(checkpoint);
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&costs).expect("read cost ledger"))
                .expect("parse cost ledger");
        value["run_id"] = serde_json::Value::String("wrong-run".to_string());
        std::fs::write(
            &costs,
            serde_json::to_vec_pretty(&value).expect("serialize mismatch"),
        )
        .expect("write mismatched ledger");

        let error = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .err()
            .expect("mismatched cost ledger must fail closed");

        assert!(error.to_string().contains("validate Graph cost ledger"));
    }

    #[test]
    fn unresolved_crash_reservation_is_rejected_without_force() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut checkpoint =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                .expect("fresh checkpoint");
        checkpoint
            .take_cost_ledger()
            .persist(100_000, 250_000)
            .expect("persist unresolved reservation");
        drop(checkpoint);

        let error = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .err()
            .expect("unresolved reservation must fail closed");

        assert!(format!("{error:#}").contains("unresolved reserved micro-USD"));
    }

    #[test]
    fn graph_drift_is_rejected_without_force() {
        let dir = tempdir().expect("tempdir");
        let graph_v1 = graph("p", 1);
        let checkpoint =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph_v1, false, false)
                .expect("fresh checkpoint");
        drop(checkpoint);

        let error =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph("p", 2), false, false)
                .err()
                .expect("drift must fail");
        assert!(error.to_string().contains("graph has changed"));
    }

    #[test]
    fn force_resume_archives_drifted_state_and_starts_new_run() {
        let dir = tempdir().expect("tempdir");
        let graph_v1 = graph("p", 1);
        let old = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph_v1, false, false)
            .expect("fresh checkpoint");
        let old_run = old.run_id().to_string();
        drop(old);

        let replacement =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph("p", 2), false, true)
                .expect("replacement checkpoint");
        assert_ne!(replacement.run_id(), old_run);
        assert_eq!(replacement.replayed_entries(), 0);
    }

    #[test]
    fn explicit_manifest_is_rejected_for_multiple_plans() {
        let dir = tempdir().expect("tempdir");
        let error =
            resolve_checkpoint_paths(dir.path(), Some(Path::new("checkpoint.json")), "p", 2)
                .expect_err("ambiguous file must fail");
        assert!(error.to_string().contains("only resume one plan"));
    }

    #[test]
    fn bare_resume_plan_default_maps_to_canonical_root_for_multi_plan_runs() {
        let dir = tempdir().expect("tempdir");
        let canonical = dir.path().join(".roko/state/graph/p/checkpoint.json");
        // The clap `default_missing_value`, relative and as `roko resume`
        // passes it (absolute), plus the legacy executor path.
        for requested in [
            PathBuf::from(".roko/state/state-snapshot.json"),
            dir.path().join(".roko/state/state-snapshot.json"),
            PathBuf::from("./.roko/state/executor.json"),
        ] {
            for plan_count in [1, 8] {
                let paths = resolve_checkpoint_paths(dir.path(), Some(&requested), "p", plan_count)
                    .unwrap_or_else(|error| {
                        panic!("{} with {plan_count} plans: {error}", requested.display())
                    });
                assert_eq!(paths.manifest, canonical, "{}", requested.display());
            }
        }
    }

    #[test]
    fn checkpoint_records_interrupted_status() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut checkpoint =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                .expect("fresh checkpoint");
        assert_eq!(checkpoint.status(), GraphCheckpointStatus::Running);
        checkpoint
            .finish_with_status(GraphCheckpointStatus::Interrupted)
            .expect("finish");
        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(&checkpoint.paths().manifest).expect("read manifest"),
        )
        .expect("parse manifest");
        assert_eq!(manifest["status"], "interrupted");

        // An interrupted checkpoint stays resumable.
        let resumed = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("resume interrupted checkpoint");
        assert_eq!(resumed.run_id(), checkpoint.run_id());
        assert_eq!(resumed.status(), GraphCheckpointStatus::Running);
    }

    /// bug-4641e3: a forced exit marks a checkpoint that still reads
    /// `running` as `interrupted`, and leaves one its run finalized alone.
    #[test]
    fn a_running_checkpoint_is_marked_interrupted_and_a_finished_one_kept() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut checkpoint =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                .expect("fresh checkpoint");
        let manifest = checkpoint.paths().manifest.clone();
        let by = "SIGHUP";

        assert!(mark_running_checkpoint_interrupted(&manifest, by).expect("mark"));
        assert_eq!(
            canonical_checkpoint_status(dir.path(), "p"),
            Some(GraphCheckpointStatus::Interrupted)
        );
        assert_eq!(
            canonical_stop_cause(dir.path(), "p").as_deref(),
            Some("SIGHUP")
        );
        assert!(!mark_running_checkpoint_interrupted(&manifest, by).expect("mark again"));

        checkpoint
            .finish_with_status(GraphCheckpointStatus::Failed)
            .expect("finish");
        assert!(!mark_running_checkpoint_interrupted(&manifest, by).expect("mark finished"));
        assert_eq!(
            canonical_checkpoint_status(dir.path(), "p"),
            Some(GraphCheckpointStatus::Failed)
        );
    }

    /// gap-fab2cc: an interrupted run's checkpoint names the stop request
    /// that ended it, and a later run that nothing stopped clears the record.
    #[test]
    fn checkpoint_records_the_stop_cause() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut checkpoint =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                .expect("fresh checkpoint");
        checkpoint
            .record_stop_cause(Some("deadline"))
            .expect("record the stop");
        checkpoint
            .finish_with_status(GraphCheckpointStatus::Interrupted)
            .expect("finish");
        assert_eq!(
            canonical_stop_cause(dir.path(), "p").as_deref(),
            Some("deadline")
        );

        checkpoint = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("resume the interrupted checkpoint");
        checkpoint.record_stop_cause(None).expect("record no stop");
        checkpoint
            .finish_with_status(GraphCheckpointStatus::Succeeded)
            .expect("finish");
        assert_eq!(canonical_stop_cause(dir.path(), "p"), None);
    }

    // ─── v3 extension and receipt tests ──────────────────────────────────

    #[test]
    fn fresh_checkpoint_has_v3_schema_and_empty_extensions() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let checkpoint = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("fresh checkpoint");
        assert!(checkpoint.extensions().is_empty());
        assert!(checkpoint.receipts().is_empty());
    }

    #[test]
    fn extension_registration_roundtrip() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut checkpoint =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                .expect("fresh checkpoint");

        let ext = CheckpointExtension {
            namespace: "roko.workspace.attempt".into(),
            schema_version: 1,
            required: false,
            fingerprint: "abc123".into(),
            value: serde_json::json!({"lease_id": "lease-1"}),
        };
        checkpoint.register_extension(ext).expect("register");

        let retrieved = checkpoint
            .extension(WORKSPACE_ATTEMPT_EXTENSION)
            .expect("extension exists");
        assert_eq!(retrieved.fingerprint, "abc123");
        assert!(!retrieved.required);
    }

    #[test]
    fn duplicate_extension_same_fingerprint_is_idempotent() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut checkpoint =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                .expect("fresh checkpoint");

        let ext = CheckpointExtension {
            namespace: "roko.test".into(),
            schema_version: 1,
            required: false,
            fingerprint: "same".into(),
            value: serde_json::json!({}),
        };
        checkpoint.register_extension(ext.clone()).expect("first");
        checkpoint
            .register_extension(ext)
            .expect("idempotent second");
        assert_eq!(checkpoint.extensions().len(), 1);
    }

    #[test]
    fn duplicate_extension_different_fingerprint_fails() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut checkpoint =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                .expect("fresh checkpoint");

        let ext1 = CheckpointExtension {
            namespace: "roko.test".into(),
            schema_version: 1,
            required: false,
            fingerprint: "fp-a".into(),
            value: serde_json::json!({}),
        };
        let ext2 = CheckpointExtension {
            namespace: "roko.test".into(),
            schema_version: 1,
            required: false,
            fingerprint: "fp-b".into(),
            value: serde_json::json!({}),
        };
        checkpoint.register_extension(ext1).expect("first");
        let err = checkpoint
            .register_extension(ext2)
            .expect_err("different fingerprint must fail");
        assert!(err.to_string().contains("different fingerprint"));
    }

    #[test]
    fn receipt_lifecycle_prepared_committed_settled() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut checkpoint =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                .expect("fresh checkpoint");

        // Prepare
        let entry = checkpoint
            .prepare_receipt("r1".into(), "test-owner".into(), "corr-1".into())
            .expect("prepare");
        assert_eq!(entry.state, ReceiptState::Prepared);

        // Commit
        let entry = checkpoint
            .commit_receipt("r1", Some("commit-abc".into()))
            .expect("commit");
        assert_eq!(entry.state, ReceiptState::Committed);
        assert_eq!(entry.evidence_ref.as_deref(), Some("commit-abc"));

        // Settle
        let entry = checkpoint.settle_receipt("r1").expect("settle");
        assert_eq!(entry.state, ReceiptState::Settled);
    }

    #[test]
    fn receipt_idempotent_same_state_transitions() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut checkpoint =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                .expect("fresh checkpoint");

        checkpoint
            .prepare_receipt("r1".into(), "owner".into(), "corr".into())
            .expect("prepare");
        // Repeat prepare is idempotent.
        let entry = checkpoint
            .prepare_receipt("r1".into(), "owner".into(), "corr".into())
            .expect("idempotent prepare");
        assert_eq!(entry.state, ReceiptState::Prepared);

        checkpoint.commit_receipt("r1", None).expect("commit");
        // Repeat commit is idempotent.
        let entry = checkpoint
            .commit_receipt("r1", None)
            .expect("idempotent commit");
        assert_eq!(entry.state, ReceiptState::Committed);

        checkpoint.settle_receipt("r1").expect("settle");
        // Repeat settle is idempotent.
        let entry = checkpoint.settle_receipt("r1").expect("idempotent settle");
        assert_eq!(entry.state, ReceiptState::Settled);
    }

    #[test]
    fn receipt_skip_commit_fails() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut checkpoint =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                .expect("fresh checkpoint");

        checkpoint
            .prepare_receipt("r1".into(), "owner".into(), "corr".into())
            .expect("prepare");

        // Try to settle without committing first -- must fail.
        let err = checkpoint
            .settle_receipt("r1")
            .expect_err("skip commit must fail");
        assert!(err.to_string().contains("must commit first"));
    }

    #[test]
    fn receipt_commit_already_settled_is_noop() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut checkpoint =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                .expect("fresh checkpoint");

        checkpoint
            .prepare_receipt("r1".into(), "owner".into(), "corr".into())
            .expect("prepare");
        checkpoint
            .commit_receipt("r1", Some("ev".into()))
            .expect("commit");
        checkpoint.settle_receipt("r1").expect("settle");

        // Commit after settle is a no-op (already past Committed).
        let entry = checkpoint
            .commit_receipt("r1", None)
            .expect("commit after settle is noop");
        assert_eq!(entry.state, ReceiptState::Settled);
    }

    #[test]
    fn receipt_nonexistent_key_fails() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut checkpoint =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                .expect("fresh checkpoint");

        assert!(checkpoint.commit_receipt("nope", None).is_err());
        assert!(checkpoint.settle_receipt("nope").is_err());
    }

    #[test]
    fn record_receipt_error_preserves_state() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut checkpoint =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                .expect("fresh checkpoint");

        checkpoint
            .prepare_receipt("r1".into(), "owner".into(), "corr".into())
            .expect("prepare");
        checkpoint
            .record_receipt_error("r1", "timeout")
            .expect("record error");

        let entry = checkpoint.receipt("r1").expect("receipt exists");
        assert_eq!(entry.state, ReceiptState::Prepared);
        assert_eq!(entry.last_error.as_deref(), Some("timeout"));
    }

    #[test]
    fn v2_manifest_migrates_to_v3_on_resume() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);

        // Create a fresh v3 checkpoint, then manually rewrite the manifest as v2.
        let mut checkpoint =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                .expect("fresh checkpoint");
        checkpoint
            .take_recorder()
            .record("p", "task-1", 0, Vec::new())
            .expect("record");
        let manifest_path = checkpoint.paths().manifest.clone();
        checkpoint.finish(false).expect("finish");

        // Read, downgrade to v2 (remove extensions/receipts), and rewrite.
        let bytes = std::fs::read(&manifest_path).expect("read manifest");
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).expect("parse manifest");
        value["schema_version"] = serde_json::json!(2);
        value.as_object_mut().unwrap().remove("extensions");
        value.as_object_mut().unwrap().remove("receipts");
        std::fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&value).expect("serialize v2"),
        )
        .expect("write v2 manifest");

        // Resume should succeed with in-memory migration to v3.
        let resumed = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("resume v2 manifest");
        assert_eq!(resumed.replayed_entries(), 1);
        assert!(resumed.extensions().is_empty());
        assert!(resumed.receipts().is_empty());
    }

    #[test]
    fn unsupported_schema_version_fails_closed() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);

        let checkpoint = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("fresh checkpoint");
        let manifest_path = checkpoint.paths().manifest.clone();
        drop(checkpoint);

        // Rewrite as v99 (unsupported).
        let bytes = std::fs::read(&manifest_path).expect("read manifest");
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).expect("parse manifest");
        value["schema_version"] = serde_json::json!(99);
        std::fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&value).expect("serialize v99"),
        )
        .expect("write v99 manifest");

        let err = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect_err("unsupported version must fail");
        assert!(err.to_string().contains("unsupported"));
    }

    #[test]
    fn v1_schema_version_fails_closed() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);

        let checkpoint = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("fresh checkpoint");
        let manifest_path = checkpoint.paths().manifest.clone();
        drop(checkpoint);

        // Rewrite as v1 (below minimum).
        let bytes = std::fs::read(&manifest_path).expect("read manifest");
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).expect("parse manifest");
        value["schema_version"] = serde_json::json!(1);
        std::fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&value).expect("serialize v1"),
        )
        .expect("write v1 manifest");

        let err = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect_err("v1 schema must fail closed");
        assert!(err.to_string().contains("unsupported"));
    }

    #[test]
    fn extensions_persist_across_checkpoint_roundtrip() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);

        let mut checkpoint =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                .expect("fresh checkpoint");
        checkpoint
            .register_extension(CheckpointExtension {
                namespace: "roko.workspace.attempt".into(),
                schema_version: 1,
                required: false,
                fingerprint: "ws-fp".into(),
                value: serde_json::json!({"lease_id": "lease-42"}),
            })
            .expect("register extension");
        checkpoint
            .prepare_receipt("r1".into(), "test".into(), "corr".into())
            .expect("prepare receipt");
        checkpoint
            .commit_receipt("r1", Some("evidence-1".into()))
            .expect("commit receipt");
        checkpoint
            .take_recorder()
            .record("p", "task-1", 0, Vec::new())
            .expect("record");
        checkpoint.persist_manifest().expect("persist");
        checkpoint.finish(false).expect("finish");

        // Resume and verify extensions and receipts survived.
        let resumed = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("resume checkpoint");
        let ext = resumed
            .extension(WORKSPACE_ATTEMPT_EXTENSION)
            .expect("extension survived resume");
        assert_eq!(ext.fingerprint, "ws-fp");

        let receipt = resumed.receipt("r1").expect("receipt survived resume");
        assert_eq!(receipt.state, ReceiptState::Committed);
        assert_eq!(receipt.evidence_ref.as_deref(), Some("evidence-1"));
    }

    #[test]
    fn resume_records_the_attempts_a_stop_cut_off() {
        use roko_learn::telemetry::{
            AttemptIdentity, AttemptKey, AttemptVerdictRecord, TelemetryWriter,
            TelemetryWriterConfig,
        };

        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut fresh = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("fresh checkpoint");
        // task-1 recorded its output, but the process stopped before its
        // verdict line was written.
        fresh
            .take_recorder()
            .record("p", "task-1", 0, Vec::new())
            .expect("record");
        fresh.finish(false).expect("finish");
        let run_id = fresh.run_id().to_string();
        let identity = |plan: &str, task: &str, attempt: u32| {
            AttemptIdentity::new(&AttemptKey::new(run_id.as_str(), plan, task, attempt))
        };
        let open = |plan: &str, task: &str, attempt: u32| {
            AttemptOpenRecord::new(identity(plan, task, attempt), 1_000 * i64::from(attempt))
        };
        let settle = |task: &str, attempt: u32, outcome: AttemptOutcome| {
            AttemptVerdictRecord::settle(identity("p", task, attempt), outcome, true)
        };
        let run_dir = RokoLayout::for_project(dir.path()).run_dir(&run_id);
        let writer = TelemetryWriter::spawn(&run_dir, TelemetryWriterConfig::default())
            .expect("spawn writer");
        for (plan, task, attempt) in [
            ("p", "task-1", 1),
            ("p", "task-2", 1),
            ("p", "task-2", 2),
            ("p", "task-3", 1),
            ("p", "task-4", 1),
            ("q", "task-5", 1),
        ] {
            assert!(writer.submit(open(plan, task, attempt)));
        }
        assert!(writer.submit(settle("task-2", 1, AttemptOutcome::GateFailed)));
        assert!(writer.submit(settle("task-3", 1, AttemptOutcome::Cancelled)));
        assert!(writer.submit(settle("task-4", 1, AttemptOutcome::GateFailed)));
        assert_eq!(writer.close().written, 9);

        // task-2's second attempt never settled, and the run cancelled
        // task-3's; task-4's attempt failed, and task-5 is another plan's.
        let mut resumed = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("resume checkpoint");
        let value = resumed
            .extension(INTERRUPTED_ATTEMPT_EXTENSION)
            .expect("the interrupted attempts are recorded")
            .value
            .clone();
        let recorded: InterruptedAttempts = serde_json::from_value(value).expect("attempts");
        let expected = |task: &str, attempt: u32, cancelled: bool| InterruptedAttempt {
            task_id: task.to_string(),
            attempt_key: identity("p", task, attempt).attempt_key,
            started_at_ms: Some(1_000 * i64::from(attempt)),
            cancelled,
        };
        assert_eq!(
            recorded.attempts,
            [expected("task-2", 2, false), expected("task-3", 1, true)]
        );
        let persisted = read_manifest(&resumed.paths().manifest).expect("manifest");
        assert!(
            persisted
                .extensions
                .contains_key(INTERRUPTED_ATTEMPT_EXTENSION)
        );

        // Once every task's latest attempt has settled, the next resume drops
        // the record.
        resumed.finish(false).expect("finish");
        let writer = TelemetryWriter::spawn(run_dir, TelemetryWriterConfig::default())
            .expect("spawn writer");
        assert!(writer.submit(open("p", "task-3", 2)));
        assert!(writer.submit(settle("task-2", 2, AttemptOutcome::GateFailed)));
        assert!(writer.submit(settle("task-3", 2, AttemptOutcome::GateFailed)));
        assert_eq!(writer.close().written, 3);
        let resumed = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("second resume");
        assert!(resumed.extension(INTERRUPTED_ATTEMPT_EXTENSION).is_none());
    }

    #[test]
    fn resume_sets_aside_a_torn_activity_record() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut fresh = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("fresh checkpoint");
        fresh
            .take_recorder()
            .record("p", "task-1", 0, Vec::new())
            .expect("record");
        fresh.finish(false).expect("finish");
        let run_id = fresh.run_id().to_string();
        let activities = fresh.paths().activities.clone();
        let committed = std::fs::read(&activities).expect("committed log");
        // The process died while it appended a record, in the middle of a
        // multi-byte character.
        let torn: &[u8] = b"{\"graph_id\":\"p\",\"node_id\":\"task-2\",\"text\":\"\xe2\x82";
        let mut log = std::fs::OpenOptions::new()
            .append(true)
            .open(&activities)
            .expect("open log");
        std::io::Write::write_all(&mut log, torn).expect("tear the log");
        drop(log);

        // The preview changes no file, and expects what the resume does.
        let preview = preview_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("preview");
        assert_eq!(preview.action, ResumeAction::Resume);
        assert_eq!(preview.restored_tasks, ["task-1"]);

        let mut resumed = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("a torn record does not block resume");
        assert_eq!(resumed.replayed_entries(), 1);
        assert_eq!(std::fs::read(&activities).expect("log"), committed);
        let set_aside: Vec<Vec<u8>> = std::fs::read_dir(activities.parent().expect("dir"))
            .expect("checkpoint dir")
            .map(|entry| entry.expect("entry").path())
            .filter(|path| path.to_string_lossy().contains(".uncommitted."))
            .map(|path| std::fs::read(path).expect("set-aside bytes"))
            .collect();
        assert_eq!(set_aside, [torn.to_vec()]);

        // The resumed run appends after the last complete record.
        resumed
            .take_recorder()
            .record("p", "task-2", 0, Vec::new())
            .expect("record");
        let replayer = ActivityReplayer::load_scoped(&activities, "p", &run_id).expect("log");
        assert_eq!(replayer.entry_count(), 2);
    }

    /// Contents of the files a resume set aside from `path`
    /// (`<name>.uncommitted.<unix ms>`), in byte order.
    fn set_aside_from(path: &Path) -> Vec<Vec<u8>> {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("file name");
        let prefix = format!("{name}.uncommitted.");
        let mut found: Vec<Vec<u8>> = std::fs::read_dir(path.parent().expect("dir"))
            .expect("checkpoint dir")
            .map(|entry| entry.expect("entry").path())
            .filter(|candidate| {
                candidate
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(&prefix))
            })
            .map(|candidate| std::fs::read(candidate).expect("set-aside bytes"))
            .collect();
        found.sort();
        found
    }

    /// gap-dc1d16: a resume uses exactly the generation the manifest names.
    /// A record and spend that reached the files after the last commit, and
    /// a torn record, are set aside, and the committed bytes of the log stay
    /// as they were.
    #[test]
    fn resume_selects_single_immutable_generation() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut fresh = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("fresh checkpoint");
        let run_id = fresh.run_id().to_string();
        let paths = fresh.paths().clone();
        // Node A's record and the spend so far are committed.
        fresh
            .take_recorder()
            .record("p", "task-1", 0, Vec::new())
            .expect("record node A");
        fresh
            .take_cost_ledger()
            .persist(125_000, 0)
            .expect("persist the spend");
        let committed = std::fs::read(&paths.activities).expect("committed log");
        let generation = read_manifest(&paths.manifest).expect("manifest").generation;

        // Node B's record and more spend reach the files, but the process
        // dies before it commits them, while it appends another record.
        ActivityRecorder::create(&run_id, &paths.activities)
            .expect("recorder")
            .record("p", "task-2", 0, Vec::new())
            .expect("record node B");
        let torn: &[u8] = b"{\"graph_id\":\"p\",\"node_id\":\"task-3\"";
        let mut log = std::fs::OpenOptions::new()
            .append(true)
            .open(&paths.activities)
            .expect("open log");
        std::io::Write::write_all(&mut log, torn).expect("tear the log");
        drop(log);
        let written = std::fs::read(&paths.activities).expect("log");
        let uncommitted = written[committed.len()..].to_vec();
        let mut ledger: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&paths.costs).expect("read")).expect("parse");
        ledger["spent_micro_usd"] = serde_json::json!(900_000);
        std::fs::write(&paths.costs, ledger.to_string()).expect("write the ledger");
        drop(fresh);

        let mut resumed = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("resume the committed generation");
        assert_eq!(resumed.replayed_entries(), 1);
        let replayer = resumed.take_replayer().expect("replayer");
        assert!(replayer.lookup("task-1", 0).is_some());
        assert!(replayer.lookup("task-2", 0).is_none());
        assert_eq!(resumed.take_cost_ledger().spent_micro_usd(), 125_000);
        assert_eq!(std::fs::read(&paths.activities).expect("log"), committed);
        assert_eq!(set_aside_from(&paths.activities), [uncommitted]);
        let ledgers = set_aside_from(&paths.costs);
        assert_eq!(ledgers.len(), 1);
        let aside: serde_json::Value = serde_json::from_slice(&ledgers[0]).expect("parse");
        assert_eq!(aside["spent_micro_usd"], 900_000);
        let manifest = read_manifest(&paths.manifest).expect("manifest");
        assert!(manifest.generation > generation);
        let stamp = manifest.committed.expect("the committed generation");
        assert_eq!(stamp.activity_bytes, committed.len() as u64);
        assert_eq!(
            stamp.activity_blake3,
            blake3::hash(&committed).to_hex().to_string()
        );
        assert_eq!(stamp.spent_micro_usd, 125_000);
    }

    /// gap-dc1d16: resuming the same checkpoint twice derives the same state,
    /// and neither resume rewrites the log: the record a resume refuses stays
    /// in it, and the second resume refuses it again.
    #[test]
    fn resuming_twice_derives_the_same_state() {
        let dir = tempdir().expect("tempdir");
        let graph = verify_graph("p");
        let mut fresh = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("fresh checkpoint");
        let mut recorder = fresh.take_recorder();
        // task-1's verify steps never passed; task-2 has none to pass.
        recorder
            .record("p", "task-1", 0, verdict_output("BLOCK: not applied", None))
            .expect("record task-1");
        recorder
            .record(
                "p",
                "task-2",
                0,
                verdict_output("done", Some(TaskGateVerdict::Passed)),
            )
            .expect("record task-2");
        drop(recorder);
        fresh
            .take_cost_ledger()
            .persist(40_000, 0)
            .expect("persist the spend");
        fresh.finish(false).expect("finish");
        let log = std::fs::read(&fresh.paths().activities).expect("log");

        let derive = || {
            let mut resumed =
                prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                    .expect("resume");
            let replayer = resumed.take_replayer().expect("replayer");
            let restored = ["task-1", "task-2"].map(|node| replayer.lookup(node, 0).is_some());
            assert_eq!(
                std::fs::read(&resumed.paths().activities).expect("log"),
                log
            );
            (
                resumed.replayed_entries(),
                restored,
                resumed.invalidated_activities().to_vec(),
                resumed.gate_verdicts(),
                resumed.take_cost_ledger().spent_micro_usd(),
            )
        };
        let first = derive();
        assert_eq!(first.1, [false, true]);
        assert_eq!(first.2.len(), 1);
        assert_eq!(first.2[0].node_id, "task-1");
        assert_eq!(first.4, 40_000);
        assert_eq!(derive(), first);
    }

    /// gap-dc1d16: a checkpoint from before generations (schema v3) resumes
    /// as generation 0, with its log's complete records and its ledger's
    /// cost, and carries a generation from then on.
    #[test]
    fn a_v3_checkpoint_resumes_as_generation_zero() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut fresh = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("fresh checkpoint");
        fresh
            .take_recorder()
            .record("p", "task-1", 0, Vec::new())
            .expect("record");
        fresh
            .take_cost_ledger()
            .persist(70_000, 0)
            .expect("persist the spend");
        fresh.finish(false).expect("finish");
        let paths = fresh.paths().clone();
        let committed = std::fs::read(&paths.activities).expect("log");
        // An older roko wrote the manifest, and died while it appended.
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&paths.manifest).expect("read")).expect("parse");
        value["schema_version"] = serde_json::json!(3);
        let fields = value.as_object_mut().expect("manifest object");
        fields.remove("generation");
        fields.remove("committed");
        std::fs::write(&paths.manifest, value.to_string()).expect("write a v3 manifest");
        let mut log = std::fs::OpenOptions::new()
            .append(true)
            .open(&paths.activities)
            .expect("open log");
        std::io::Write::write_all(&mut log, b"{\"graph_id\":").expect("tear the log");
        drop(log);

        let mut resumed = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("resume a v3 checkpoint");
        assert_eq!(resumed.replayed_entries(), 1);
        assert_eq!(resumed.take_cost_ledger().spent_micro_usd(), 70_000);
        assert_eq!(std::fs::read(&paths.activities).expect("log"), committed);
        let manifest = read_manifest(&paths.manifest).expect("manifest");
        assert_eq!(manifest.schema_version, CHECKPOINT_SCHEMA_VERSION);
        assert_eq!(manifest.generation, 1);
        let stamp = manifest.committed.expect("the committed generation");
        assert_eq!(stamp.activity_bytes, committed.len() as u64);
        assert_eq!(stamp.spent_micro_usd, 70_000);
    }

    /// gap-dc1d16: a resume fails closed, and changes no file, when bytes a
    /// generation committed have changed.
    #[test]
    fn resume_fails_closed_when_committed_records_change() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut fresh = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("fresh checkpoint");
        fresh
            .take_recorder()
            .record("p", "task-1", 0, Vec::new())
            .expect("record");
        fresh.finish(false).expect("finish");
        let paths = fresh.paths().clone();
        let log = std::fs::read_to_string(&paths.activities).expect("log");
        let edited = log.replace("task-1", "task-9");
        std::fs::write(&paths.activities, &edited).expect("edit the log");
        let manifest = std::fs::read(&paths.manifest).expect("manifest");

        let error = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect_err("changed committed records must fail closed");
        assert!(format!("{error:#}").contains("changed after generation"));
        assert_eq!(
            std::fs::read_to_string(&paths.activities).expect("log"),
            edited
        );
        assert_eq!(std::fs::read(&paths.manifest).expect("manifest"), manifest);
    }

    /// gap-dc1d16: a forced exit marks an open checkpoint interrupted through
    /// its commit point, so a record committed afterwards keeps the status.
    #[test]
    fn a_commit_after_a_forced_exit_mark_keeps_the_interrupted_status() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut checkpoint =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                .expect("fresh checkpoint");
        let manifest = checkpoint.paths().manifest.clone();
        assert!(mark_running_checkpoint_interrupted(&manifest, "SIGHUP").expect("mark"));
        checkpoint
            .take_recorder()
            .record("p", "task-1", 0, Vec::new())
            .expect("record");
        assert_eq!(
            canonical_checkpoint_status(dir.path(), "p"),
            Some(GraphCheckpointStatus::Interrupted)
        );
        let committed = read_manifest(&manifest)
            .expect("manifest")
            .committed
            .expect("the committed generation");
        let log = std::fs::read(&checkpoint.paths().activities).expect("log");
        assert_eq!(committed.activity_bytes, log.len() as u64);
    }

    #[test]
    fn checkpoint_writes_store_the_safety_provenance_summary() {
        use roko_agent::safety::{ProvenanceCall, ProvenanceIntent, SafetyProvenanceSink};
        use roko_core::extension::CamelTaintLevel;

        use crate::safety_provenance::SafetyProvenanceSummary;

        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut checkpoint =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                .expect("fresh checkpoint");
        let stored = |checkpoint: &PreparedGraphCheckpoint| -> Option<SafetyProvenanceSummary> {
            let manifest = read_manifest(&checkpoint.paths().manifest).expect("manifest");
            let extension = manifest.extensions.get(EXT_SAFETY_PROVENANCE)?;
            Some(serde_json::from_value(extension.value.clone()).expect("summary"))
        };
        checkpoint.persist_manifest().expect("persist");
        assert_eq!(
            stored(&checkpoint),
            None,
            "without a sink there is no extension"
        );

        let sink = Arc::new(GraphProvenanceSink::open(dir.path()).expect("provenance sink"));
        checkpoint.attach_safety_provenance(Arc::clone(&sink));
        checkpoint.persist_manifest().expect("persist");
        assert_eq!(stored(&checkpoint).expect("extension").records, 0);

        // A terminal write stores the summary as it stands then.
        let intent = ProvenanceIntent {
            call: ProvenanceCall {
                run_id: checkpoint.run_id().to_string(),
                task_id: "task-1".to_string(),
                attempt_id: "1".to_string(),
                turn_id: "1".to_string(),
                call_id: "call-1".to_string(),
                tool: "read_file".to_string(),
                args_digest: roko_core::ContentHash::keyed(&sink.digest_key(), b"arguments"),
            },
            taint: CamelTaintLevel::Untrusted,
        };
        sink.record_intent(&intent).expect("record the intent");
        checkpoint.finish(false).expect("finish");
        let summary = stored(&checkpoint).expect("extension");
        assert_eq!(summary.records, 1);
        assert_eq!(summary, sink.summary());
    }

    /// Record one tool call of run `run_id` with `sink`, its turn tainted at
    /// `taint`: an intent, then its outcome. Returns the call's argument and
    /// result digests.
    fn record_tool_call(
        sink: &GraphProvenanceSink,
        run_id: &str,
        taint: roko_core::extension::CamelTaintLevel,
    ) -> (roko_core::ContentHash, roko_core::ContentHash) {
        use roko_agent::safety::{
            ProvenanceCall, ProvenanceIntent, ProvenanceOutcome, ProvenanceVerdict,
            SafetyProvenanceSink,
        };

        let key = sink.digest_key();
        let call = ProvenanceCall {
            run_id: run_id.to_string(),
            task_id: "task-1".to_string(),
            attempt_id: "1".to_string(),
            turn_id: "1".to_string(),
            call_id: "call-1".to_string(),
            tool: "fetch".to_string(),
            args_digest: roko_core::ContentHash::keyed(&key, b"arguments"),
        };
        let intent = ProvenanceIntent {
            call: call.clone(),
            taint,
        };
        let ack = sink.record_intent(&intent).expect("record the intent");
        let result = roko_core::ContentHash::keyed(&key, b"result");
        let outcome = ProvenanceOutcome {
            call: call.clone(),
            intent: Some(ack.record_id),
            verdict: ProvenanceVerdict::Succeeded,
            reason: None,
            result_digest: Some(result),
            taint,
        };
        sink.record_outcome(&outcome).expect("record the outcome");
        (call.args_digest, result)
    }

    /// Run plan `p` once in `dir`, recording one untrusted tool call, and
    /// finish it failed so that it can resume.
    fn run_with_provenance(dir: &Path, graph: &Graph) {
        let mut checkpoint = prepare_graph_checkpoint(dir, None, "p", 1, graph, false, false)
            .expect("fresh checkpoint");
        let sink = checkpoint
            .open_safety_provenance(dir)
            .expect("open provenance");
        let run_id = checkpoint.run_id().to_string();
        let untrusted = roko_core::extension::CamelTaintLevel::Untrusted;
        record_tool_call(&sink, &run_id, untrusted);
        checkpoint.finish(false).expect("finish");
    }

    /// How resuming plan `p` in `dir` fails to restore its safety provenance.
    fn restore_error(dir: &Path, graph: &Graph) -> String {
        let mut resumed = prepare_graph_checkpoint(dir, None, "p", 1, graph, false, false)
            .expect("resume checkpoint");
        let error = resumed
            .open_safety_provenance(dir)
            .expect_err("the restore must fail closed");
        format!("{error:#}")
    }

    #[test]
    fn safety_provenance_restores_taint_after_restart() {
        use roko_core::extension::CamelTaintLevel;

        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut first = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("fresh checkpoint");
        let sink = first
            .open_safety_provenance(dir.path())
            .expect("open provenance");
        let run_id = first.run_id().to_string();
        let (args, result) = record_tool_call(&sink, &run_id, CamelTaintLevel::Untrusted);
        first.finish(false).expect("finish");
        drop((first, sink));

        // Another process resumes the run, and the lineage comes back first.
        let mut resumed = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("resume checkpoint");
        let restored = resumed
            .open_safety_provenance(dir.path())
            .expect("restore provenance");
        let taint = restored.taint();
        assert_eq!(taint.get_level(&args), Some(CamelTaintLevel::Untrusted));
        assert_eq!(taint.get_level(&result), Some(CamelTaintLevel::Untrusted));
        assert_eq!(taint.derived_from(&result), [args]);
        assert_eq!(restored.summary().records, 2);
        let stored = resumed
            .stored_safety_provenance()
            .expect("stored provenance")
            .expect("a summary");
        assert_eq!(stored, restored.summary());
    }

    #[test]
    fn safety_provenance_restore_tracks_calls_after_the_last_save() {
        use roko_core::extension::CamelTaintLevel;

        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let mut first = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("fresh checkpoint");
        let sink = first
            .open_safety_provenance(dir.path())
            .expect("open provenance");
        let run_id = first.run_id().to_string();
        // The process dies after the call, before another checkpoint write.
        let (args, result) = record_tool_call(&sink, &run_id, CamelTaintLevel::External);
        drop((first, sink));

        let mut resumed = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("resume checkpoint");
        let restored = resumed
            .open_safety_provenance(dir.path())
            .expect("restore provenance");
        let taint = restored.taint();
        assert_eq!(taint.get_level(&result), Some(CamelTaintLevel::External));
        assert_eq!(taint.derived_from(&result), [args]);
    }

    #[test]
    fn safety_provenance_restore_fails_closed_on_a_tampered_custody_log() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        run_with_provenance(dir.path(), &graph);
        let log = RokoLayout::for_project(dir.path()).custody_log();
        let text = std::fs::read_to_string(&log).expect("custody log");
        let tampered = text.replacen("tool_outcome:fetch", "tool_outcome:fetch2", 1);
        assert_ne!(tampered, text);
        std::fs::write(&log, tampered).expect("tamper with the custody log");

        let error = restore_error(dir.path(), &graph);
        assert!(error.contains("custody chain"), "{error}");
    }

    #[test]
    fn safety_provenance_restore_fails_closed_on_a_missing_witness_root() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        run_with_provenance(dir.path(), &graph);
        let log = RokoLayout::for_project(dir.path()).witness_log();
        std::fs::remove_file(&log).expect("remove the witness log");

        let error = restore_error(dir.path(), &graph);
        assert!(error.contains("witness"), "{error}");
    }

    #[test]
    fn safety_provenance_restore_fails_closed_on_a_taint_downgrade() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        run_with_provenance(dir.path(), &graph);
        let path = dir.path().join(".roko/state/graph/p/checkpoint.json");
        let mut manifest = read_manifest(&path).expect("manifest");
        let extension = manifest
            .extensions
            .get_mut(EXT_SAFETY_PROVENANCE)
            .expect("the provenance extension");
        // Everything the run proved tainted now reads as trusted.
        extension.value["taint"] = roko_agent::safety::TaintTracker::new().to_json();
        write_manifest_atomic(&path, &manifest).expect("write the manifest");

        let error = restore_error(dir.path(), &graph);
        assert!(error.contains("taint index"), "{error}");
    }

    #[test]
    fn safety_provenance_restore_fails_closed_on_an_unknown_version() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        run_with_provenance(dir.path(), &graph);
        let path = dir.path().join(".roko/state/graph/p/checkpoint.json");
        let mut manifest = read_manifest(&path).expect("manifest");
        manifest.extensions.insert(
            "roko.safety-provenance@2".to_string(),
            CheckpointExtension {
                namespace: "roko.safety-provenance".into(),
                schema_version: 2,
                required: false,
                fingerprint: "from-a-later-build".into(),
                value: serde_json::json!({}),
            },
        );
        write_manifest_atomic(&path, &manifest).expect("write the manifest");

        let error = restore_error(dir.path(), &graph);
        assert!(error.contains("cannot read"), "{error}");
    }

    #[test]
    fn safety_provenance_starts_on_broken_history_and_checks_only_its_own_records() {
        use roko_agent::safety::provenance::{Custody, CustodyLogger};

        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        // Custody history no run can verify, as older builds and concurrent
        // processes leave it: a garbage line, a record edited after it was
        // sealed, and a record appended twice.
        let log = RokoLayout::for_project(dir.path()).custody_log();
        let logger = CustodyLogger::new(&log);
        let old = |action: &str, when: i64| Custody::new(action, "older-build", when, Vec::new());
        crate::custody::log_chained(&logger, old("old-a", 1)).expect("old record");
        crate::custody::log_chained(&logger, old("old-b", 2)).expect("old record");
        let text = std::fs::read_to_string(&log).expect("custody log");
        let first = text.lines().next().expect("a first record").to_string();
        let edited = text.replacen("old-b", "old-B", 1);
        std::fs::write(&log, format!("not json at all\n{edited}{first}\n")).expect("break it");
        assert!(crate::custody::cmd_custody_verify(dir.path()).is_err());

        // A fresh run still records its calls, and its resume checks only them.
        run_with_provenance(dir.path(), &graph);
        let mut resumed = prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
            .expect("resume checkpoint");
        let restored = resumed
            .open_safety_provenance(dir.path())
            .expect("restore provenance");
        assert_eq!(restored.summary().records, 2);
        drop((resumed, restored));

        // Tampering with the run's own records still fails its resume closed.
        let text = std::fs::read_to_string(&log).expect("custody log");
        let tampered = text.replacen("tool_outcome:fetch", "tool_outcome:fetch2", 1);
        std::fs::write(&log, tampered).expect("tamper with the run's records");
        let error = restore_error(dir.path(), &graph);
        assert!(error.contains("custody chain"), "{error}");
    }

    #[test]
    fn receipt_state_ordering() {
        assert!(ReceiptState::Prepared < ReceiptState::Committed);
        assert!(ReceiptState::Committed < ReceiptState::Settled);
    }

    #[test]
    fn checkpoint_extension_serde_roundtrip() {
        let ext = CheckpointExtension {
            namespace: "roko.gate.verdict".into(),
            schema_version: 1,
            required: true,
            fingerprint: "fp-gate".into(),
            value: serde_json::json!({"passed": true, "rung": 3}),
        };
        let json = serde_json::to_string(&ext).expect("serialize");
        let deser: CheckpointExtension = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(ext, deser);
    }

    #[test]
    fn receipt_ledger_entry_serde_roundtrip() {
        let entry = ReceiptLedgerEntry {
            idempotency_key: "key-1".into(),
            owner: "graph-dispatch".into(),
            correlation_id: "corr-42".into(),
            state: ReceiptState::Committed,
            evidence_ref: Some("sha256:abc".into()),
            updated_at_ms: 1_000_000,
            last_error: None,
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        let deser: ReceiptLedgerEntry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(entry, deser);
    }

    /// gap-4ec59f: `roko plan status` finds where a delivered plan's work
    /// went in its checkpoint, and nothing for a plan whose delivery failed.
    #[test]
    fn recorded_batch_delivery_reads_only_a_delivered_plan() {
        let dir = tempdir().expect("tempdir");
        for (plan, state) in [("p-delivered", "delivered"), ("p-conflict", "conflict")] {
            let checkpoint = dir.path().join(".roko/state/graph").join(plan);
            std::fs::create_dir_all(&checkpoint).expect("checkpoint dir");
            let manifest = serde_json::json!({
                "extensions": {BATCH_EXTENSION: {"value": {
                    "branch": "roko/batch/run-1",
                    "state": state,
                    "merge_commit": "a".repeat(40),
                }}}
            });
            std::fs::write(checkpoint.join("checkpoint.json"), manifest.to_string())
                .expect("checkpoint");
        }

        assert_eq!(
            recorded_batch_delivery(dir.path(), "p-delivered"),
            Some(RecordedBatchDelivery {
                branch: "roko/batch/run-1".to_string(),
                merge_commit: "a".repeat(40),
            })
        );
        assert_eq!(recorded_batch_delivery(dir.path(), "p-conflict"), None);
        assert_eq!(recorded_batch_delivery(dir.path(), "p-missing"), None);
    }

    #[test]
    fn known_extension_namespace_constants_format() {
        assert_eq!(WORKSPACE_ATTEMPT_EXTENSION, "roko.workspace.attempt@1");
        assert_eq!(GATE_VERDICT_EXTENSION, "roko.gate.verdict@1");
    }
}
