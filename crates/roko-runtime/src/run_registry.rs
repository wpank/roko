//! Plan-scoped run registry (#327).
//!
//! Tracks every execution run for a plan, supporting new/resume/rerun
//! decisions based on plan fingerprints and prior run state.
//!
//! # Types
//!
//! - [`RunRegistry`]: per-plan run history, stored as `.roko/state/runs/<plan_id>.json`.
//! - [`RunIndex`] / [`RunIndexEntry`]: chronological index of all runs across plans.
//! - [`RunIntent`]: what the caller wants (new run, resume, fresh rerun).
//! - [`RunManifest`]: metadata about a single run.
//! - [`RunScope`]: `(plan_id, run_id)` pair that scopes every downstream artifact.
//! - [`RunStartDecision`]: registry verdict on whether to start/resume/reject.
//! - [`RunStatus`]: lifecycle state of a run.
//! - [`ExecutionRunContext`]: current run state handed to engines at dispatch time.
//! - [`compute_plan_fingerprint`]: BLAKE3 hash of a `tasks.toml` file.
//! - [`compute_task_graph_fingerprint`]: BLAKE3 hash of serialised task graph bytes.
//! - [`migrate_legacy_singleton`]: upgrade single-run state to the registry format.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use roko_core::RunId;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// RunStatus
// ---------------------------------------------------------------------------

/// Lifecycle state of a plan execution run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    /// Run has been created but not yet started.
    Pending,
    /// Run is actively executing.
    Running,
    /// Run completed successfully (all tasks passed all gates).
    Completed,
    /// Run failed (at least one task failed its gates after all retries).
    Failed,
    /// Run was cancelled by the user or a control signal.
    Cancelled,
}

impl RunStatus {
    /// Returns `true` when no further state transitions should be accepted.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    /// Short label suitable for display / logging.
    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

// ---------------------------------------------------------------------------
// RunIntent
// ---------------------------------------------------------------------------

/// What the caller wants when starting a plan execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum RunIntent {
    /// Start a brand-new run.
    New,
    /// Resume the most recent non-terminal run.
    Resume,
    /// Discard all prior state and start fresh (new run ID).
    RerunFresh,
}

// ---------------------------------------------------------------------------
// RunManifest
// ---------------------------------------------------------------------------

/// Metadata about a single execution run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunManifest {
    /// Unique run identifier.
    pub run_id: RunId,
    /// BLAKE3 fingerprint of the plan at the time this run was created.
    pub plan_fingerprint: String,
    /// Current lifecycle status.
    pub status: RunStatus,
    /// Wall-clock timestamp when the run was created (ms since epoch).
    pub created_at_ms: u64,
    /// Wall-clock timestamp of the last status transition (ms since epoch).
    pub updated_at_ms: u64,
    /// Optional human-readable notes (e.g. failure reason, cancellation source).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl RunManifest {
    /// Create a new manifest in [`RunStatus::Pending`].
    pub fn new(run_id: RunId, plan_fingerprint: String) -> Self {
        let now = epoch_ms();
        Self {
            run_id,
            plan_fingerprint,
            status: RunStatus::Pending,
            created_at_ms: now,
            updated_at_ms: now,
            notes: None,
        }
    }

    /// Transition to a new status, updating the timestamp.
    pub fn transition(&mut self, status: RunStatus) {
        self.status = status;
        self.updated_at_ms = epoch_ms();
    }

    /// Transition to a new status with an optional note.
    pub fn transition_with_note(&mut self, status: RunStatus, note: impl Into<String>) {
        self.status = status;
        self.updated_at_ms = epoch_ms();
        self.notes = Some(note.into());
    }
}

// ---------------------------------------------------------------------------
// RunScope
// ---------------------------------------------------------------------------

/// `(plan_id, run_id)` pair that scopes every downstream artifact.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunScope {
    /// Plan identifier (directory name or slug).
    pub plan_id: String,
    /// Run identifier.
    pub run_id: RunId,
}

impl RunScope {
    /// Construct a new scope.
    pub fn new(plan_id: impl Into<String>, run_id: RunId) -> Self {
        Self {
            plan_id: plan_id.into(),
            run_id,
        }
    }
}

impl std::fmt::Display for RunScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.plan_id, self.run_id)
    }
}

// ---------------------------------------------------------------------------
// RunStartDecision
// ---------------------------------------------------------------------------

/// Registry verdict on whether the requested run should proceed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunStartDecision {
    /// Start a brand-new run with the given scope.
    Start(RunScope),
    /// Resume an existing run with the given scope.
    Resume(RunScope),
    /// Reject: the checkpoint/state is corrupt or unreadable.
    RejectCorrupt {
        /// Human-readable reason.
        reason: String,
    },
    /// Reject: the plan fingerprint doesn't match any prior run.
    RejectUnrelated {
        /// Human-readable reason.
        reason: String,
    },
}

impl RunStartDecision {
    /// Returns `true` if the decision allows execution to proceed.
    pub fn is_go(&self) -> bool {
        matches!(self, Self::Start(_) | Self::Resume(_))
    }

    /// Extract the run scope if the decision allows execution.
    pub fn scope(&self) -> Option<&RunScope> {
        match self {
            Self::Start(s) | Self::Resume(s) => Some(s),
            Self::RejectCorrupt { .. } | Self::RejectUnrelated { .. } => None,
        }
    }
}

// ---------------------------------------------------------------------------
// ExecutionRunContext
// ---------------------------------------------------------------------------

/// Current run state handed to engines at dispatch time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionRunContext {
    /// Active run scope.
    pub scope: RunScope,
    /// Whether this run is a resume of a prior attempt.
    pub is_resume: bool,
    /// BLAKE3 fingerprint of the plan as it was when this run started.
    pub plan_fingerprint: String,
    /// Status of the run at the time this context was created.
    pub status: RunStatus,
}

impl ExecutionRunContext {
    /// Build a context from a start decision and plan fingerprint.
    pub fn from_decision(decision: &RunStartDecision, plan_fingerprint: &str) -> Option<Self> {
        match decision {
            RunStartDecision::Start(scope) => Some(Self {
                scope: scope.clone(),
                is_resume: false,
                plan_fingerprint: plan_fingerprint.to_string(),
                status: RunStatus::Running,
            }),
            RunStartDecision::Resume(scope) => Some(Self {
                scope: scope.clone(),
                is_resume: true,
                plan_fingerprint: plan_fingerprint.to_string(),
                status: RunStatus::Running,
            }),
            RunStartDecision::RejectCorrupt { .. } | RunStartDecision::RejectUnrelated { .. } => {
                None
            }
        }
    }
}

// ---------------------------------------------------------------------------
// RunRegistryError
// ---------------------------------------------------------------------------

/// Errors from registry operations.
#[derive(Debug, thiserror::Error)]
pub enum RunRegistryError {
    /// IO error reading/writing registry state.
    #[error("registry I/O: {0}")]
    Io(#[from] std::io::Error),
    /// JSON serialization/deserialization error.
    #[error("registry serde: {0}")]
    Serde(#[from] serde_json::Error),
    /// Attempted an invalid state transition.
    #[error("invalid transition for run {run_id}: {reason}")]
    InvalidTransition {
        /// Run ID that had the invalid transition.
        run_id: String,
        /// Why the transition was rejected.
        reason: String,
    },
    /// No resumable run found.
    #[error("no resumable run for plan {plan_id}")]
    NoResumableRun {
        /// Plan ID that had no resumable run.
        plan_id: String,
    },
}

// ---------------------------------------------------------------------------
// RunRegistry
// ---------------------------------------------------------------------------

/// Per-plan run registry: tracks all runs with their state.
///
/// Persisted as `.roko/state/runs/<plan_id>.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRegistry {
    /// Plan identifier.
    pub plan_id: String,
    /// All runs for this plan, keyed by run ID string.
    pub runs: BTreeMap<String, RunManifest>,
}

impl RunRegistry {
    /// Create an empty registry for a plan.
    pub fn new(plan_id: impl Into<String>) -> Self {
        Self {
            plan_id: plan_id.into(),
            runs: BTreeMap::new(),
        }
    }

    /// Decide what to do given a caller intent and current plan fingerprint.
    pub fn decide(&self, intent: &RunIntent, plan_fingerprint: &str) -> RunStartDecision {
        match intent {
            RunIntent::New | RunIntent::RerunFresh => {
                let run_id = RunId::new();
                RunStartDecision::Start(RunScope::new(&self.plan_id, run_id))
            }
            RunIntent::Resume => {
                // Find the most recent non-terminal run.
                if let Some(manifest) = self.latest_active_run() {
                    if manifest.plan_fingerprint != plan_fingerprint {
                        return RunStartDecision::RejectUnrelated {
                            reason: format!(
                                "plan fingerprint changed: expected {}, got {}",
                                manifest.plan_fingerprint, plan_fingerprint
                            ),
                        };
                    }
                    RunStartDecision::Resume(RunScope::new(&self.plan_id, manifest.run_id.clone()))
                } else {
                    // No active run to resume -- start fresh.
                    let run_id = RunId::new();
                    RunStartDecision::Start(RunScope::new(&self.plan_id, run_id))
                }
            }
        }
    }

    /// Register a new run manifest.
    pub fn register(&mut self, manifest: RunManifest) {
        self.runs
            .insert(manifest.run_id.as_str().to_string(), manifest);
    }

    /// Transition an existing run to a new status.
    pub fn transition(
        &mut self,
        run_id: &RunId,
        status: RunStatus,
    ) -> Result<(), RunRegistryError> {
        let key = run_id.as_str();
        let manifest =
            self.runs
                .get_mut(key)
                .ok_or_else(|| RunRegistryError::InvalidTransition {
                    run_id: key.to_string(),
                    reason: "run not found in registry".to_string(),
                })?;

        if manifest.status.is_terminal() {
            return Err(RunRegistryError::InvalidTransition {
                run_id: key.to_string(),
                reason: format!("run is already terminal ({})", manifest.status.label()),
            });
        }

        manifest.transition(status);
        Ok(())
    }

    /// Find the most recent non-terminal run (by `updated_at_ms`).
    pub fn latest_active_run(&self) -> Option<&RunManifest> {
        self.runs
            .values()
            .filter(|m| !m.status.is_terminal())
            .max_by_key(|m| m.updated_at_ms)
    }

    /// Return all runs sorted by creation time (newest first).
    pub fn runs_newest_first(&self) -> Vec<&RunManifest> {
        let mut v: Vec<_> = self.runs.values().collect();
        v.sort_by_key(|m| std::cmp::Reverse(m.created_at_ms));
        v
    }

    /// Persist the registry to disk.
    pub fn save(&self, state_dir: &Path) -> Result<(), RunRegistryError> {
        let dir = state_dir.join("runs");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", self.plan_id));
        let json = serde_json::to_string_pretty(self)?;
        atomic_write(&path, json.as_bytes())?;
        Ok(())
    }

    /// Load a registry from disk, or return an empty one if the file doesn't exist.
    pub fn load(state_dir: &Path, plan_id: &str) -> Result<Self, RunRegistryError> {
        let path = state_dir.join("runs").join(format!("{plan_id}.json"));
        if !path.exists() {
            return Ok(Self::new(plan_id));
        }
        let data = std::fs::read_to_string(&path)?;
        let registry: Self = serde_json::from_str(&data)?;
        Ok(registry)
    }
}

// ---------------------------------------------------------------------------
// RunIndex / RunIndexEntry
// ---------------------------------------------------------------------------

/// Chronological index of all runs across all plans.
///
/// Stored as `.roko/state/run-index.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunIndex {
    /// Ordered entries (newest first after sort).
    pub entries: Vec<RunIndexEntry>,
}

/// A single entry in the cross-plan run index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunIndexEntry {
    /// Plan identifier.
    pub plan_id: String,
    /// Run identifier.
    pub run_id: RunId,
    /// Status at the time of the index write.
    pub status: RunStatus,
    /// Plan fingerprint at run creation.
    pub plan_fingerprint: String,
    /// Creation timestamp (ms since epoch).
    pub created_at_ms: u64,
    /// Last update timestamp (ms since epoch).
    pub updated_at_ms: u64,
}

impl RunIndex {
    /// Record or update an entry in the index.
    pub fn upsert(&mut self, entry: RunIndexEntry) {
        let key = entry.run_id.as_str().to_string();
        if let Some(existing) = self.entries.iter_mut().find(|e| e.run_id.as_str() == key) {
            existing.status = entry.status;
            existing.updated_at_ms = entry.updated_at_ms;
        } else {
            self.entries.push(entry);
        }
    }

    /// Sort entries newest-first by creation time.
    pub fn sort_newest_first(&mut self) {
        self.entries
            .sort_by_key(|e| std::cmp::Reverse(e.created_at_ms));
    }

    /// Persist the index to disk.
    pub fn save(&self, state_dir: &Path) -> Result<(), RunRegistryError> {
        let path = state_dir.join("run-index.json");
        let json = serde_json::to_string_pretty(self)?;
        atomic_write(&path, json.as_bytes())?;
        Ok(())
    }

    /// Load from disk, or return an empty index.
    pub fn load(state_dir: &Path) -> Result<Self, RunRegistryError> {
        let path = state_dir.join("run-index.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = std::fs::read_to_string(&path)?;
        let index: Self = serde_json::from_str(&data)?;
        Ok(index)
    }
}

// ---------------------------------------------------------------------------
// Fingerprint helpers
// ---------------------------------------------------------------------------

/// Compute a BLAKE3 fingerprint of a `tasks.toml` file.
///
/// The fingerprint is deterministic for the same file contents, allowing
/// the registry to detect plan changes between runs.
pub fn compute_plan_fingerprint(tasks_toml_path: &Path) -> Result<String, std::io::Error> {
    let data = std::fs::read(tasks_toml_path)?;
    Ok(blake3::hash(&data).to_hex().to_string())
}

/// Compute a BLAKE3 fingerprint from serialised task graph bytes.
///
/// Used when the task graph is already in memory (e.g. after parsing).
pub fn compute_task_graph_fingerprint(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

// ---------------------------------------------------------------------------
// Legacy migration
// ---------------------------------------------------------------------------

/// Upgrade a single-run state directory to the registry format.
///
/// If a `.roko/state/executor.json` exists but no registry file does,
/// this creates a registry with one synthetic historical run in
/// [`RunStatus::Completed`] state.
pub fn migrate_legacy_singleton(
    state_dir: &Path,
    plan_id: &str,
) -> Result<Option<RunRegistry>, RunRegistryError> {
    let registry_path = state_dir.join("runs").join(format!("{plan_id}.json"));
    if registry_path.exists() {
        // Already migrated.
        return Ok(None);
    }

    let legacy_path = state_dir.join("executor.json");
    if !legacy_path.exists() {
        // No legacy state to migrate.
        return Ok(None);
    }

    tracing::info!(
        plan_id,
        "migrating legacy singleton executor state to run registry"
    );

    let mut registry = RunRegistry::new(plan_id);
    let run_id = RunId::from_raw(format!("legacy-{plan_id}"));
    let mut manifest = RunManifest::new(run_id, String::new());
    manifest.transition_with_note(
        RunStatus::Completed,
        "migrated from legacy singleton executor.json",
    );
    registry.register(manifest);
    registry.save(state_dir)?;

    Ok(Some(registry))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Write `data` to a temporary file then rename into `path` for crash safety.
fn atomic_write(path: &Path, data: &[u8]) -> Result<(), std::io::Error> {
    let dir = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir)?;

    // Write to a sibling temp file then rename.
    let tmp = dir.join(format!(
        ".tmp-{}",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("registry")
    ));
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_status_terminal_states() {
        assert!(!RunStatus::Pending.is_terminal());
        assert!(!RunStatus::Running.is_terminal());
        assert!(RunStatus::Completed.is_terminal());
        assert!(RunStatus::Failed.is_terminal());
        assert!(RunStatus::Cancelled.is_terminal());
    }

    #[test]
    fn run_status_labels() {
        assert_eq!(RunStatus::Pending.label(), "pending");
        assert_eq!(RunStatus::Running.label(), "running");
        assert_eq!(RunStatus::Completed.label(), "completed");
        assert_eq!(RunStatus::Failed.label(), "failed");
        assert_eq!(RunStatus::Cancelled.label(), "cancelled");
    }

    #[test]
    fn run_manifest_transitions() {
        let run_id = RunId::from_raw("test-run");
        let mut m = RunManifest::new(run_id, "fp-abc".to_string());
        assert_eq!(m.status, RunStatus::Pending);

        m.transition(RunStatus::Running);
        assert_eq!(m.status, RunStatus::Running);
        assert!(m.updated_at_ms >= m.created_at_ms);

        m.transition_with_note(RunStatus::Failed, "gate failure");
        assert_eq!(m.status, RunStatus::Failed);
        assert_eq!(m.notes.as_deref(), Some("gate failure"));
    }

    #[test]
    fn run_scope_display() {
        let scope = RunScope::new("my-plan", RunId::from_raw("run-1"));
        assert_eq!(scope.to_string(), "my-plan:run-1");
    }

    #[test]
    fn registry_decide_new() {
        let reg = RunRegistry::new("plan-a");
        let decision = reg.decide(&RunIntent::New, "fp-123");
        assert!(decision.is_go());
        assert!(matches!(decision, RunStartDecision::Start(_)));
    }

    #[test]
    fn registry_decide_resume_no_active() {
        let reg = RunRegistry::new("plan-a");
        let decision = reg.decide(&RunIntent::Resume, "fp-123");
        // No active run, so it starts fresh.
        assert!(matches!(decision, RunStartDecision::Start(_)));
    }

    #[test]
    fn registry_decide_resume_with_active() {
        let mut reg = RunRegistry::new("plan-a");
        let run_id = RunId::from_raw("run-1");
        let mut manifest = RunManifest::new(run_id.clone(), "fp-123".to_string());
        manifest.transition(RunStatus::Running);
        reg.register(manifest);

        let decision = reg.decide(&RunIntent::Resume, "fp-123");
        assert!(matches!(decision, RunStartDecision::Resume(_)));
        assert_eq!(decision.scope().unwrap().run_id, run_id);
    }

    #[test]
    fn registry_decide_resume_fingerprint_mismatch() {
        let mut reg = RunRegistry::new("plan-a");
        let run_id = RunId::from_raw("run-1");
        let mut manifest = RunManifest::new(run_id, "fp-old".to_string());
        manifest.transition(RunStatus::Running);
        reg.register(manifest);

        let decision = reg.decide(&RunIntent::Resume, "fp-new");
        assert!(matches!(decision, RunStartDecision::RejectUnrelated { .. }));
        assert!(!decision.is_go());
    }

    #[test]
    fn registry_transition_rejects_terminal() {
        let mut reg = RunRegistry::new("plan-a");
        let run_id = RunId::from_raw("run-1");
        let mut manifest = RunManifest::new(run_id.clone(), "fp-123".to_string());
        manifest.transition(RunStatus::Completed);
        reg.register(manifest);

        let result = reg.transition(&run_id, RunStatus::Running);
        assert!(result.is_err());
    }

    #[test]
    fn registry_runs_newest_first() {
        let mut reg = RunRegistry::new("plan-a");

        let mut m1 = RunManifest::new(RunId::from_raw("run-1"), "fp".to_string());
        m1.created_at_ms = 100;
        reg.register(m1);

        let mut m2 = RunManifest::new(RunId::from_raw("run-2"), "fp".to_string());
        m2.created_at_ms = 200;
        reg.register(m2);

        let runs = reg.runs_newest_first();
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].run_id.as_str(), "run-2");
        assert_eq!(runs[1].run_id.as_str(), "run-1");
    }

    #[test]
    fn run_index_upsert_and_sort() {
        let mut index = RunIndex::default();
        index.upsert(RunIndexEntry {
            plan_id: "plan-a".to_string(),
            run_id: RunId::from_raw("run-1"),
            status: RunStatus::Running,
            plan_fingerprint: "fp-1".to_string(),
            created_at_ms: 100,
            updated_at_ms: 100,
        });
        index.upsert(RunIndexEntry {
            plan_id: "plan-a".to_string(),
            run_id: RunId::from_raw("run-2"),
            status: RunStatus::Pending,
            plan_fingerprint: "fp-1".to_string(),
            created_at_ms: 200,
            updated_at_ms: 200,
        });

        assert_eq!(index.entries.len(), 2);

        // Update existing entry.
        index.upsert(RunIndexEntry {
            plan_id: "plan-a".to_string(),
            run_id: RunId::from_raw("run-1"),
            status: RunStatus::Completed,
            plan_fingerprint: "fp-1".to_string(),
            created_at_ms: 100,
            updated_at_ms: 300,
        });
        assert_eq!(index.entries.len(), 2);
        assert_eq!(index.entries[0].status, RunStatus::Completed);

        index.sort_newest_first();
        assert_eq!(index.entries[0].run_id.as_str(), "run-2");
    }

    #[test]
    fn execution_run_context_from_start_decision() {
        let scope = RunScope::new("plan-a", RunId::from_raw("run-1"));
        let decision = RunStartDecision::Start(scope);
        let ctx = ExecutionRunContext::from_decision(&decision, "fp-abc").unwrap();
        assert!(!ctx.is_resume);
        assert_eq!(ctx.plan_fingerprint, "fp-abc");
        assert_eq!(ctx.status, RunStatus::Running);
    }

    #[test]
    fn execution_run_context_from_resume_decision() {
        let scope = RunScope::new("plan-a", RunId::from_raw("run-1"));
        let decision = RunStartDecision::Resume(scope);
        let ctx = ExecutionRunContext::from_decision(&decision, "fp-abc").unwrap();
        assert!(ctx.is_resume);
    }

    #[test]
    fn execution_run_context_from_reject_returns_none() {
        let decision = RunStartDecision::RejectCorrupt {
            reason: "bad".to_string(),
        };
        assert!(ExecutionRunContext::from_decision(&decision, "fp").is_none());
    }

    #[test]
    fn compute_task_graph_fingerprint_deterministic() {
        let data = b"[task.build]\ntitle = \"Build the thing\"";
        let a = compute_task_graph_fingerprint(data);
        let b = compute_task_graph_fingerprint(data);
        assert_eq!(a, b);
        assert!(!a.is_empty());
    }

    #[test]
    fn run_intent_serde_roundtrip() {
        for intent in [RunIntent::New, RunIntent::Resume, RunIntent::RerunFresh] {
            let json = serde_json::to_string(&intent).unwrap();
            let back: RunIntent = serde_json::from_str(&json).unwrap();
            assert_eq!(back, intent);
        }
    }

    #[test]
    fn run_manifest_serde_roundtrip() {
        let m = RunManifest::new(RunId::from_raw("run-1"), "fp-abc".to_string());
        let json = serde_json::to_string(&m).unwrap();
        let back: RunManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.run_id, m.run_id);
        assert_eq!(back.plan_fingerprint, "fp-abc");
        assert_eq!(back.status, RunStatus::Pending);
    }

    #[test]
    fn registry_save_load_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let state_dir = tmp.path();

        let mut reg = RunRegistry::new("plan-test");
        reg.register(RunManifest::new(
            RunId::from_raw("run-1"),
            "fp-100".to_string(),
        ));
        reg.save(state_dir).unwrap();

        let loaded = RunRegistry::load(state_dir, "plan-test").unwrap();
        assert_eq!(loaded.plan_id, "plan-test");
        assert_eq!(loaded.runs.len(), 1);
        assert!(loaded.runs.contains_key("run-1"));
    }

    #[test]
    fn registry_load_nonexistent_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let reg = RunRegistry::load(tmp.path(), "no-such-plan").unwrap();
        assert!(reg.runs.is_empty());
    }

    #[test]
    fn index_save_load_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let state_dir = tmp.path();

        let mut index = RunIndex::default();
        index.upsert(RunIndexEntry {
            plan_id: "plan-a".to_string(),
            run_id: RunId::from_raw("run-1"),
            status: RunStatus::Completed,
            plan_fingerprint: "fp".to_string(),
            created_at_ms: 100,
            updated_at_ms: 200,
        });
        index.save(state_dir).unwrap();

        let loaded = RunIndex::load(state_dir).unwrap();
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].plan_id, "plan-a");
    }

    #[test]
    fn migrate_legacy_no_legacy_state() {
        let tmp = tempfile::tempdir().unwrap();
        let result = migrate_legacy_singleton(tmp.path(), "plan-x").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn migrate_legacy_creates_registry() {
        let tmp = tempfile::tempdir().unwrap();
        // Create a fake legacy executor.json.
        std::fs::write(tmp.path().join("executor.json"), "{}").unwrap();

        let result = migrate_legacy_singleton(tmp.path(), "plan-x").unwrap();
        assert!(result.is_some());
        let reg = result.unwrap();
        assert_eq!(reg.runs.len(), 1);
        let manifest = reg.runs.values().next().unwrap();
        assert_eq!(manifest.status, RunStatus::Completed);

        // Second call should find the registry already exists.
        let result2 = migrate_legacy_singleton(tmp.path(), "plan-x").unwrap();
        assert!(result2.is_none());
    }

    #[test]
    fn compute_plan_fingerprint_works_on_real_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("tasks.toml");
        std::fs::write(&path, "[task.build]\ntitle = \"Build\"").unwrap();
        let fp = compute_plan_fingerprint(&path).unwrap();
        assert!(!fp.is_empty());
        // Deterministic.
        let fp2 = compute_plan_fingerprint(&path).unwrap();
        assert_eq!(fp, fp2);
    }
}
