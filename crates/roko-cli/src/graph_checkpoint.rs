//! Durable checkpoints for CLI plan execution through the Graph Engine.
//!
//! The Graph Engine records successful Activity outputs as JSONL.  This module
//! adds the small amount of run metadata needed to resume those recordings
//! safely: a schema version, a stable fingerprint of the converted graph, and
//! the run ID that scopes every record.
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
use roko_graph::{ActivityRecorder, ActivityReplayer, Graph, graph_execution_fingerprint};
use serde::{Deserialize, Serialize};

/// Current host checkpoint schema version. V2 manifests are migrated in-memory
/// to v3 with empty extensions and receipts; other versions fail closed.
const CHECKPOINT_SCHEMA_VERSION: u32 = 3;

/// Minimum schema version we can migrate from. Anything below this fails closed.
const MIN_SUPPORTED_SCHEMA_VERSION: u32 = 2;

const COST_LEDGER_SCHEMA_VERSION: u32 = 1;
const DEFAULT_RUNNER_RESUME_PATH: &str = ".roko/state/executor.json";

/// Known extension namespace for workspace/attempt state (#249).
pub const WORKSPACE_ATTEMPT_EXTENSION: &str = "roko.workspace.attempt@1";

/// Known extension namespace for structured gate verdicts (#250).
pub const GATE_VERDICT_EXTENSION: &str = "roko.gate.verdict@1";

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
    ) -> Result<GraphCostLedgerCheckpoint> {
        let bytes = std::fs::read(&path)
            .with_context(|| format!("read Graph cost ledger {}", path.display()))?;
        let state: GraphCostLedgerState = serde_json::from_slice(&bytes)
            .with_context(|| format!("parse Graph cost ledger {}", path.display()))?;
        validate_cost_ledger_state(&state, manifest)
            .with_context(|| format!("validate Graph cost ledger {}", path.display()))?;
        Ok(Self {
            path,
            identity: state,
        })
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
        self.manifest.status = if succeeded {
            GraphCheckpointStatus::Succeeded
        } else {
            GraphCheckpointStatus::Failed
        };
        self.manifest.updated_at_ms = unix_ms();
        write_manifest_atomic(&self.paths.manifest, &self.manifest)
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
        if let Some(existing) = self.manifest.receipts.get(&idempotency_key) {
            // Already at or past Prepared: idempotent success.
            return Ok(existing);
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
        self.manifest.receipts.insert(idempotency_key.clone(), entry);
        Ok(self
            .manifest
            .receipts
            .get(&idempotency_key)
            .expect("just inserted"))
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
            .ok_or_else(|| {
                anyhow::anyhow!("receipt '{idempotency_key}' not found in ledger")
            })?;

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
            .ok_or_else(|| {
                anyhow::anyhow!("receipt '{idempotency_key}' not found in ledger")
            })?;

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
            .ok_or_else(|| {
                anyhow::anyhow!("receipt '{idempotency_key}' not found in ledger")
            })?;
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
    let fingerprint = graph_execution_fingerprint(graph).context("fingerprint Graph")?;

    if fresh {
        archive_checkpoint_files(&paths)?;
    }

    if !fresh && paths.manifest.exists() {
        let bytes = std::fs::read(&paths.manifest)
            .with_context(|| format!("read Graph checkpoint {}", paths.manifest.display()))?;
        let mut manifest: GraphCheckpointManifest = serde_json::from_slice(&bytes)
            .with_context(|| format!("parse Graph checkpoint {}", paths.manifest.display()))?;

        // In-memory v2 -> v3 migration: add empty extensions/receipts, bump
        // version. The upgraded manifest is written on the next atomic commit.
        if manifest.schema_version == 2 {
            migrate_v2_to_v3(&mut manifest);
        }

        let mismatch = checkpoint_mismatch(&manifest, plan_id, &fingerprint, &paths);

        if let Some(reason) = mismatch {
            if !force_resume {
                bail!(
                    "cannot resume Graph checkpoint {}: {reason}; use --fresh to archive it or --force-resume to start a new run",
                    paths.manifest.display()
                );
            }
            archive_checkpoint_files(&paths)?;
        } else {
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
                return create_fresh_checkpoint(paths, plan_id, fingerprint);
            }
            let cost_ledger = match GraphCostLedgerCheckpoint::load(paths.costs.clone(), &manifest)
            {
                Ok(cost_ledger) => cost_ledger,
                Err(_) if force_resume => {
                    archive_checkpoint_files(&paths)?;
                    return create_fresh_checkpoint(paths, plan_id, fingerprint);
                }
                Err(error) => return Err(error),
            };
            let replayer =
                ActivityReplayer::load_scoped(&paths.activities, plan_id, &manifest.run_id)
                    .with_context(|| {
                        format!(
                            "load Graph Activity checkpoint {}",
                            paths.activities.display()
                        )
                    })?;
            let replayed_entries = replayer.entry_count();
            let recorder = ActivityRecorder::create(&manifest.run_id, &paths.activities)
                .with_context(|| {
                    format!(
                        "open Graph Activity checkpoint {}",
                        paths.activities.display()
                    )
                })?;
            manifest.status = GraphCheckpointStatus::Running;
            manifest.updated_at_ms = unix_ms();
            write_manifest_atomic(&paths.manifest, &manifest)?;
            return Ok(PreparedGraphCheckpoint {
                paths,
                manifest,
                recorder: Some(recorder),
                replayer: Some(replayer),
                replayed_entries,
                cost_ledger: Some(cost_ledger),
            });
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

    create_fresh_checkpoint(paths, plan_id, fingerprint)
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
    let runner_default = workdir.join(DEFAULT_RUNNER_RESUME_PATH);
    let base = match requested {
        None => canonical_graph_root,
        Some(path) if path == runner_default => canonical_graph_root,
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

fn checkpoint_mismatch(
    manifest: &GraphCheckpointManifest,
    plan_id: &str,
    fingerprint: &str,
    paths: &GraphCheckpointPaths,
) -> Option<String> {
    // Accept v2 (will migrate in-memory to v3) and v3.
    if manifest.schema_version < MIN_SUPPORTED_SCHEMA_VERSION
        || manifest.schema_version > CHECKPOINT_SCHEMA_VERSION
    {
        return Some(format!(
            "schema version {} is unsupported (expected {MIN_SUPPORTED_SCHEMA_VERSION}..={CHECKPOINT_SCHEMA_VERSION})",
            manifest.schema_version
        ));
    }
    if manifest.plan_id != plan_id {
        return Some(format!(
            "manifest is for plan '{}' rather than '{plan_id}'",
            manifest.plan_id
        ));
    }
    if manifest.graph_fingerprint != fingerprint {
        return Some("the converted plan graph has changed".to_string());
    }
    if paths.activities.file_name().and_then(|name| name.to_str())
        != Some(manifest.activity_log.as_str())
    {
        return Some("manifest references a different Activity log".to_string());
    }
    if paths.costs.file_name().and_then(|name| name.to_str()) != Some(manifest.cost_ledger.as_str())
    {
        return Some("manifest references a different cost ledger".to_string());
    }
    None
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
    if state.graph_fingerprint != manifest.graph_fingerprint {
        bail!("cost ledger graph fingerprint does not match the checkpoint manifest");
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

    #[test]
    fn fingerprint_changes_with_execution_config() {
        assert_ne!(
            graph_execution_fingerprint(&graph("p", 1)).expect("fingerprint"),
            graph_execution_fingerprint(&graph("p", 2)).expect("fingerprint")
        );
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

    // ─── v3 extension and receipt tests ──────────────────────────────────

    #[test]
    fn fresh_checkpoint_has_v3_schema_and_empty_extensions() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);
        let checkpoint =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
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
        checkpoint.register_extension(ext).expect("idempotent second");
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

        checkpoint
            .commit_receipt("r1", None)
            .expect("commit");
        // Repeat commit is idempotent.
        let entry = checkpoint
            .commit_receipt("r1", None)
            .expect("idempotent commit");
        assert_eq!(entry.state, ReceiptState::Committed);

        checkpoint.settle_receipt("r1").expect("settle");
        // Repeat settle is idempotent.
        let entry = checkpoint
            .settle_receipt("r1")
            .expect("idempotent settle");
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
        let mut value: serde_json::Value =
            serde_json::from_slice(&bytes).expect("parse manifest");
        value["schema_version"] = serde_json::json!(2);
        value.as_object_mut().unwrap().remove("extensions");
        value.as_object_mut().unwrap().remove("receipts");
        std::fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&value).expect("serialize v2"),
        )
        .expect("write v2 manifest");

        // Resume should succeed with in-memory migration to v3.
        let resumed =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                .expect("resume v2 manifest");
        assert_eq!(resumed.replayed_entries(), 1);
        assert!(resumed.extensions().is_empty());
        assert!(resumed.receipts().is_empty());
    }

    #[test]
    fn unsupported_schema_version_fails_closed() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);

        let checkpoint =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                .expect("fresh checkpoint");
        let manifest_path = checkpoint.paths().manifest.clone();
        drop(checkpoint);

        // Rewrite as v99 (unsupported).
        let bytes = std::fs::read(&manifest_path).expect("read manifest");
        let mut value: serde_json::Value =
            serde_json::from_slice(&bytes).expect("parse manifest");
        value["schema_version"] = serde_json::json!(99);
        std::fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&value).expect("serialize v99"),
        )
        .expect("write v99 manifest");

        let err =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                .expect_err("unsupported version must fail");
        assert!(err.to_string().contains("unsupported"));
    }

    #[test]
    fn v1_schema_version_fails_closed() {
        let dir = tempdir().expect("tempdir");
        let graph = graph("p", 1);

        let checkpoint =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
                .expect("fresh checkpoint");
        let manifest_path = checkpoint.paths().manifest.clone();
        drop(checkpoint);

        // Rewrite as v1 (below minimum).
        let bytes = std::fs::read(&manifest_path).expect("read manifest");
        let mut value: serde_json::Value =
            serde_json::from_slice(&bytes).expect("parse manifest");
        value["schema_version"] = serde_json::json!(1);
        std::fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&value).expect("serialize v1"),
        )
        .expect("write v1 manifest");

        let err =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
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
        let resumed =
            prepare_graph_checkpoint(dir.path(), None, "p", 1, &graph, false, false)
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
