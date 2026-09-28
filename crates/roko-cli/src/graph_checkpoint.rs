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
//! - **v3** (this release): adds a namespaced extension map and an idempotent
//!   receipt ledger. A v2 manifest is migrated in-memory to v3 with empty
//!   extensions/receipts and its existing cost ledger preserved; v3 is written
//!   on the next atomic checkpoint. Versions other than 2 or 3 fail closed.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use roko_graph::cells::task_executor::{TaskExecutionSpec, TaskGateVerdict};
use roko_graph::convert::{PlanTaskInfo, plan_to_graph};
use roko_graph::replay::{RecordEntry, retain_recorded_activities};
use roko_graph::{
    ActivityRecorder, ActivityReplayer, AuthoredPlan, Graph, legacy_graph_execution_fingerprint,
    plan_graph_fingerprint,
};
use serde::{Deserialize, Serialize};

use crate::runner::plan_loader::Plan;
use crate::task_parser::TasksFile;

/// Current host checkpoint schema version. V2 manifests are migrated in-memory
/// to v3 with empty extensions and receipts; other versions fail closed.
const CHECKPOINT_SCHEMA_VERSION: u32 = 3;

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

/// Lifecycle state persisted beside a Graph Activity recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphCheckpointStatus {
    /// The process may have stopped before every node reached a terminal state.
    Running,
    /// Every graph node completed successfully.
    Succeeded,
    /// At least one graph node failed.
    Failed,
    /// An operator cancelled the plan before every node finished.
    Cancelled,
    /// A signal (SIGINT/SIGTERM) or a closed operator TUI stopped the run
    /// before every node finished; recorded Activities remain resumable.
    Interrupted,
}

impl GraphCheckpointStatus {
    /// Serialized name, e.g. `interrupted`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Succeeded => "succeeded",
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

/// A recorded Activity refused for replay on resume; its node re-runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvalidatedActivity {
    /// Graph node whose record was removed.
    pub node_id: String,
    /// Tick of the removed record.
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
    /// Records removed by the last resume because they lacked a passing
    /// verdict.
    #[serde(default)]
    pub invalidated_on_resume: Vec<InvalidatedActivity>,
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
        Some(TaskGateVerdict::Passed) => None,
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

/// Remove recorded Activities that must not be replayed: forced accepts and
/// verify-bearing task outputs without a passing verdict (for example records
/// written before verdicts existed, when a failed verify could be
/// force-accepted). Their nodes re-execute instead of resuming as successes.
fn invalidate_unverified_activities(
    path: &Path,
    graph: &Graph,
) -> Result<Vec<InvalidatedActivity>> {
    let required = verdict_required_nodes(graph);
    let mut invalidated = Vec::new();
    retain_recorded_activities(path, |entry| {
        match replay_refusal(&entry.signals, required.contains(&entry.node_id)) {
            Some(reason) => {
                invalidated.push(InvalidatedActivity {
                    node_id: entry.node_id.clone(),
                    tick: entry.tick,
                    reason,
                });
                false
            }
            None => true,
        }
    })
    .with_context(|| format!("screen Graph Activity checkpoint {}", path.display()))?;
    for activity in &invalidated {
        tracing::warn!(
            node_id = %activity.node_id,
            tick = activity.tick,
            reason = %activity.reason,
            "resume: recorded task output is not verified; the node will re-run"
        );
    }
    Ok(invalidated)
}

/// Read the latest recorded gate verdict for each node from an Activity log.
fn recorded_gate_verdicts(path: &Path) -> BTreeMap<String, TaskGateVerdict> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    content
        .lines()
        .filter_map(|line| serde_json::from_str::<RecordEntry>(line.trim()).ok())
        .filter_map(|entry| {
            TaskGateVerdict::from_signals(&entry.signals).map(|verdict| (entry.node_id, verdict))
        })
        .collect()
}

/// Build the [`GATE_VERDICT_EXTENSION`] entry for `summary`.
fn gate_verdict_extension(summary: &GateVerdictSummary) -> Result<CheckpointExtension> {
    let (namespace, version) = GATE_VERDICT_EXTENSION
        .split_once('@')
        .context("gate verdict extension key has no schema version")?;
    let value = serde_json::to_value(summary).context("serialize gate verdict summary")?;
    let bytes = serde_json::to_vec(&value).context("serialize gate verdict summary")?;
    Ok(CheckpointExtension {
        namespace: namespace.to_string(),
        schema_version: version
            .parse()
            .context("gate verdict extension schema version")?,
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
fn authored_plan_running(content: &str, specs: &[TaskExecutionSpec]) -> Option<AuthoredPlan> {
    let authored = AuthoredPlan::from_tasks_toml(content).ok()?;
    let loaded = TasksFile::parse_str(content)
        .ok()?
        .tasks
        .iter()
        .map(|task| Some((task.id.clone(), serde_json::to_value(task).ok()?)))
        .collect::<Option<BTreeMap<_, _>>>()?;
    let running = specs
        .iter()
        .map(|spec| {
            let task: serde_json::Value = serde_json::from_str(&spec.task_def_json).ok()?;
            Some((task.get("id")?.as_str()?.to_string(), task))
        })
        .collect::<Option<BTreeMap<_, _>>>()?;
    (running == loaded && authored.tasks.keys().eq(loaded.keys())).then_some(authored)
}

// ---------------------------------------------------------------------------
// Manifest
// ---------------------------------------------------------------------------

/// Versioned metadata that makes an Activity JSONL file safe to resume.
///
/// Schema version 3 adds `extensions` and `receipts` on top of the original
/// v2 fields. A v2 manifest on disk is migrated in-memory to v3 with empty
/// extension/receipt maps and its cost ledger preserved.
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
}

impl GraphCostLedgerCheckpoint {
    /// Restored actual spend in millionths of one USD.
    #[must_use]
    pub const fn spent_micro_usd(&self) -> u64 {
        self.identity.spent_micro_usd
    }

    /// Persist the latest actual spend atomically.
    pub(crate) fn persist(&self, spent_micro_usd: u64, reserved_micro_usd: u64) -> Result<()> {
        let mut state = self.identity.clone();
        state.spent_micro_usd = spent_micro_usd;
        state.reserved_micro_usd = reserved_micro_usd;
        write_cost_ledger_atomic(&self.path, &state)
    }

    fn load(
        path: PathBuf,
        manifest: &GraphCheckpointManifest,
        graph: &GraphIdentity,
    ) -> Result<GraphCostLedgerCheckpoint> {
        let bytes = std::fs::read(&path)
            .with_context(|| format!("read Graph cost ledger {}", path.display()))?;
        let state: GraphCostLedgerState = serde_json::from_slice(&bytes)
            .with_context(|| format!("parse Graph cost ledger {}", path.display()))?;
        validate_cost_ledger_state(&state, manifest, graph)
            .with_context(|| format!("validate Graph cost ledger {}", path.display()))?;
        Ok(Self {
            path,
            identity: state,
        })
    }

    /// Persist `graph_fingerprint` as the graph this ledger belongs to.
    fn rebind(&mut self, graph_fingerprint: &str) -> Result<()> {
        self.identity.graph_fingerprint = graph_fingerprint.to_string();
        write_cost_ledger_atomic(&self.path, &self.identity)
    }
}

/// Recorder/replayer pair prepared for one Graph execution.
pub struct PreparedGraphCheckpoint {
    paths: GraphCheckpointPaths,
    manifest: GraphCheckpointManifest,
    recorder: Option<ActivityRecorder>,
    replayer: Option<ActivityReplayer>,
    replayed_entries: usize,
    cost_ledger: Option<GraphCostLedgerCheckpoint>,
    invalidated_on_resume: Vec<InvalidatedActivity>,
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
        self.manifest.updated_at_ms = unix_ms();
        write_manifest_atomic(&self.paths.manifest, &self.manifest)
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

    /// Decode the persisted [`GATE_VERDICT_EXTENSION`] summary, if any.
    #[must_use]
    pub fn gate_verdicts(&self) -> Option<GateVerdictSummary> {
        self.manifest
            .extensions
            .get(GATE_VERDICT_EXTENSION)
            .and_then(|extension| serde_json::from_value(extension.value.clone()).ok())
    }

    /// Rebuild the gate-verdict extension from the durable Activity log.
    ///
    /// The host-owned summary is replaced wholesale, so this bypasses the
    /// write-once fingerprint guard of [`Self::register_extension`].
    fn refresh_gate_verdicts(&mut self) -> Result<()> {
        let summary = GateVerdictSummary {
            verdicts: recorded_gate_verdicts(&self.paths.activities),
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
        self.manifest.updated_at_ms = unix_ms();
        write_manifest_atomic(&self.paths.manifest, &self.manifest)
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
                    return create_fresh_checkpoint(paths, plan_id, identity.current);
                }
                let mut cost_ledger = match GraphCostLedgerCheckpoint::load(
                    paths.costs.clone(),
                    &manifest,
                    &identity,
                ) {
                    Ok(cost_ledger) => cost_ledger,
                    Err(_) if force_resume => {
                        archive_checkpoint_files(&paths)?;
                        return create_fresh_checkpoint(paths, plan_id, identity.current);
                    }
                    Err(error) => return Err(error),
                };
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
                return resume_checkpoint(paths, manifest, cost_ledger, plan_id, graph);
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

    create_fresh_checkpoint(paths, plan_id, identity.current)
}

/// Reopen a validated checkpoint for another run of the same plan graph.
fn resume_checkpoint(
    paths: GraphCheckpointPaths,
    mut manifest: GraphCheckpointManifest,
    cost_ledger: GraphCostLedgerCheckpoint,
    plan_id: &str,
    graph: &Graph,
) -> Result<PreparedGraphCheckpoint> {
    // Never resume a task whose recorded output was not verified: drop those
    // records so the nodes re-run.
    let invalidated_on_resume = invalidate_unverified_activities(&paths.activities, graph)?;
    let replayer = ActivityReplayer::load_scoped(&paths.activities, plan_id, &manifest.run_id)
        .with_context(|| {
            format!(
                "load Graph Activity checkpoint {}",
                paths.activities.display()
            )
        })?;
    let replayed_entries = replayer.entry_count();
    let recorder =
        ActivityRecorder::create(&manifest.run_id, &paths.activities).with_context(|| {
            format!(
                "open Graph Activity checkpoint {}",
                paths.activities.display()
            )
        })?;
    manifest.status = GraphCheckpointStatus::Running;
    let mut prepared = PreparedGraphCheckpoint {
        paths,
        manifest,
        recorder: Some(recorder),
        replayer: Some(replayer),
        replayed_entries,
        cost_ledger: Some(cost_ledger),
        invalidated_on_resume,
    };
    prepared.refresh_gate_verdicts()?;
    prepared.manifest.updated_at_ms = unix_ms();
    write_manifest_atomic(&prepared.paths.manifest, &prepared.manifest)?;
    Ok(prepared)
}

fn create_fresh_checkpoint(
    paths: GraphCheckpointPaths,
    plan_id: &str,
    fingerprint: String,
) -> Result<PreparedGraphCheckpoint> {
    let parent = paths
        .manifest
        .parent()
        .context("Graph checkpoint manifest has no parent directory")?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("create Graph checkpoint directory {}", parent.display()))?;
    let run_id = format!("graph-{plan_id}-{}", uuid::Uuid::new_v4());
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
        extensions: BTreeMap::new(),
        receipts: BTreeMap::new(),
    };
    let recorder = ActivityRecorder::create_fresh(&run_id, &paths.activities)
        .with_context(|| format!("create Graph Activity log {}", paths.activities.display()))?;
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
    };
    cost_ledger.persist(0, 0)?;
    write_manifest_atomic(&paths.manifest, &manifest)?;
    Ok(PreparedGraphCheckpoint {
        paths,
        manifest,
        recorder: Some(recorder),
        replayer: None,
        replayed_entries: 0,
        cost_ledger: Some(cost_ledger),
        invalidated_on_resume: Vec::new(),
    })
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
    // Accept v2 (will migrate in-memory to v3) and v3.
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
    manifest.schema_version = CHECKPOINT_SCHEMA_VERSION;
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
    if state.reserved_micro_usd > 0 {
        bail!(
            "cost ledger contains {} unresolved reserved micro-USD from an interrupted provider call",
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
    let graph = convert_plan(plan, options)?;
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

/// Convert `plan` as `graph_execution::plan_runner::run_one_plan` does with
/// the default topology, so the preview fingerprints the graph a run executes.
fn convert_plan(plan: &Plan, options: &ResumeOptions<'_>) -> Result<Graph> {
    let tasks: Vec<(String, PlanTaskInfo)> = plan
        .tasks
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
                max_retries: options.max_retries.unwrap_or(task.max_retries),
                domain: task.domain.as_ref().map(|domain| format!("{domain:?}")),
                sequence: task.sequence,
                full_config_json: serde_json::to_value(task).unwrap_or_default(),
            };
            (task.id.clone(), info)
        })
        .collect();
    let max_parallel = if options.max_tasks > 0 {
        u32::try_from(options.max_tasks).unwrap_or(u32::MAX)
    } else {
        plan.tasks.meta.max_parallel
    };
    plan_to_graph(
        &plan.id,
        &plan.dir.display().to_string(),
        &tasks,
        max_parallel,
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
    if let Err(error) = GraphCostLedgerCheckpoint::load(paths.costs.clone(), &manifest, &identity) {
        return Ok(preview.unusable(force_resume, format!("{error:#}")));
    }
    let replayer = match ActivityReplayer::load_scoped(&paths.activities, plan_id, &manifest.run_id)
    {
        Ok(replayer) => replayer,
        Err(error) => {
            return Ok(preview.refused(format!(
                "load Graph Activity checkpoint {}: {error}",
                paths.activities.display()
            )));
        }
    };
    let required = verdict_required_nodes(graph);
    let (restored, to_run) = task_nodes.into_iter().partition(|node| {
        replayer
            .lookup(node, 0)
            .is_some_and(|signals| replay_refusal(signals, required.contains(node)).is_none())
    });
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
        convert_plan(plan, options).expect("convert plan")
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

    #[test]
    fn known_extension_namespace_constants_format() {
        assert_eq!(WORKSPACE_ATTEMPT_EXTENSION, "roko.workspace.attempt@1");
        assert_eq!(GATE_VERDICT_EXTENSION, "roko.gate.verdict@1");
    }
}
