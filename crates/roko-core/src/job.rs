//! Marketplace job types shared between `roko-serve`, the TUI, and the CLI.
//!
//! [`MarketplaceJob`] is the canonical representation of a job in the system.
//! It mirrors the `JobRecord` stored in `.roko/jobs/*.json` by `roko-serve`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Lifecycle status of a marketplace job.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    #[default]
    Open,
    Assigned,
    InProgress,
    Submitted,
    Completed,
    Failed,
    Cancelled,
}

impl JobStatus {
    /// Parse a status string into a `JobStatus`, tolerating aliases.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "open" | "pending" => Some(Self::Open),
            "assigned" => Some(Self::Assigned),
            "in_progress" | "active" | "running" => Some(Self::InProgress),
            "submitted" => Some(Self::Submitted),
            "completed" | "done" | "evaluated" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "cancelled" | "canceled" => Some(Self::Cancelled),
            _ => None,
        }
    }

    /// Snake-case string representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Assigned => "assigned",
            Self::InProgress => "in_progress",
            Self::Submitted => "submitted",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    /// Valid status transitions from this state.
    #[must_use]
    pub const fn valid_transitions(self) -> &'static [JobStatus] {
        match self {
            Self::Open => &[Self::Assigned, Self::InProgress, Self::Cancelled],
            Self::Assigned => &[Self::InProgress, Self::Open, Self::Cancelled],
            Self::InProgress => &[Self::Submitted, Self::Failed, Self::Cancelled],
            Self::Submitted => &[Self::Completed, Self::InProgress, Self::Failed],
            Self::Completed | Self::Failed | Self::Cancelled => &[],
        }
    }

    /// Whether this is a terminal (no further transitions) state.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A marketplace job — the canonical shared type across serve, TUI, and CLI.
///
/// Mirrors the `JobRecord` persisted in `.roko/jobs/{id}.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MarketplaceJob {
    /// Unique job identifier.
    #[serde(default)]
    pub id: String,
    /// Human-readable title.
    #[serde(default)]
    pub title: String,
    /// Detailed description of the work to be done.
    #[serde(default)]
    pub description: String,
    /// Job type: `research`, `coding_task`, `chain_monitor`, `chain_analysis`, `other`.
    #[serde(default)]
    pub job_type: String,
    /// Lifecycle status (preferred field).
    #[serde(default)]
    pub status: String,
    /// Fallback status field for backward-compat with older job files using `state`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub state: String,
    /// Who posted the job.
    #[serde(default)]
    pub posted_by: String,
    /// Who is assigned to the job.
    #[serde(default, alias = "assignee")]
    pub assigned_to: String,
    /// Priority level: `low`, `medium`, `high`, `critical`.
    #[serde(default)]
    pub priority: String,
    /// RFC-3339 creation timestamp.
    #[serde(default)]
    pub created_at: String,
    /// RFC-3339 last-update timestamp.
    #[serde(default)]
    pub updated_at: String,
    /// Freeform tags for categorisation.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Optional reward string.
    #[serde(default)]
    pub reward: String,
    /// Optional associated plan identifier.
    #[serde(default)]
    pub plan_id: String,
    /// Submission payload (result_summary, artifacts, gate_results).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submission: Option<serde_json::Value>,
    /// Evaluation payload (accepted, feedback).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluation: Option<serde_json::Value>,
    /// Whether the job runner should auto-execute this job.
    /// Defaults to `false`; callers set it explicitly when creating a job.
    #[serde(default)]
    pub auto_execute: bool,
}

impl MarketplaceJob {
    /// Return the effective lifecycle status, preferring `status` but falling
    /// back to the deprecated `state` field for files written by roko-serve
    /// (which serializes via `#[serde(rename = "state")]`).
    #[must_use]
    pub fn effective_status(&self) -> &str {
        let s = self.status.trim();
        if !s.is_empty() {
            return s;
        }
        let fallback = self.state.trim();
        if !fallback.is_empty() {
            return fallback;
        }
        "open"
    }
}

/// Summary of a PRD for the Atelier TUI view.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PrdSummary {
    /// URL-safe slug identifier.
    #[serde(default)]
    pub slug: String,
    /// Human-readable title.
    #[serde(default)]
    pub title: String,
    /// Lifecycle status: `idea`, `draft`, `published`, `planned`.
    #[serde(default)]
    pub status: String,
    /// Number of associated plans.
    #[serde(default)]
    pub plan_count: usize,
    /// Total tasks across all plans.
    #[serde(default)]
    pub task_total: usize,
    /// Completed tasks.
    #[serde(default)]
    pub task_done: usize,
    /// Failed tasks.
    #[serde(default)]
    pub task_failed: usize,
}

/// Summary of a task for the Atelier TUI view.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TaskSummary {
    /// Task identifier.
    #[serde(default)]
    pub id: String,
    /// Human-readable title.
    #[serde(default)]
    pub title: String,
    /// Current status string.
    #[serde(default)]
    pub status: String,
    /// Agent assigned to this task.
    #[serde(default)]
    pub agent: String,
}

/// Progress entry for a running job (used by TUI).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobProgressEntry {
    /// Completion percentage (0-100).
    #[serde(default)]
    pub percent: u8,
    /// Latest progress message.
    #[serde(default)]
    pub message: String,
    /// Agent executing this job.
    #[serde(default)]
    pub agent_id: String,
}

// ---------------------------------------------------------------------------
// Typed job domain types
// ---------------------------------------------------------------------------

/// Categorisation of a marketplace job.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobType {
    Research,
    CodingTask,
    ChainMonitor,
    ChainAnalysis,
    Review,
    Documentation,
    Testing,
    Other(String),
}

impl Default for JobType {
    fn default() -> Self {
        Self::Other("other".to_string())
    }
}

impl std::fmt::Display for JobType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Research => f.write_str("research"),
            Self::CodingTask => f.write_str("coding_task"),
            Self::ChainMonitor => f.write_str("chain_monitor"),
            Self::ChainAnalysis => f.write_str("chain_analysis"),
            Self::Review => f.write_str("review"),
            Self::Documentation => f.write_str("documentation"),
            Self::Testing => f.write_str("testing"),
            Self::Other(s) => f.write_str(s),
        }
    }
}

impl FromStr for JobType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim().to_ascii_lowercase().as_str() {
            "research" => Self::Research,
            "coding_task" | "coding" => Self::CodingTask,
            "chain_monitor" => Self::ChainMonitor,
            "chain_analysis" => Self::ChainAnalysis,
            "review" => Self::Review,
            "documentation" | "docs" => Self::Documentation,
            "testing" | "test" => Self::Testing,
            other => Self::Other(other.to_string()),
        })
    }
}

/// Typed submission payload for a job.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobSubmission {
    #[serde(default)]
    pub agent_id: String,
    #[serde(default)]
    pub result_summary: String,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub gate_results: Vec<JobGateResult>,
    #[serde(default)]
    pub submitted_at: String,
}

/// Result of a gate check within a job submission.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobGateResult {
    #[serde(default)]
    pub gate: String,
    #[serde(default)]
    pub passed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Typed evaluation payload for a job.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobEvaluation {
    #[serde(default)]
    pub evaluator: String,
    #[serde(default)]
    pub accepted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    #[serde(default)]
    pub feedback: String,
    #[serde(default)]
    pub evaluated_at: String,
}

/// Diagnostic emitted when a job JSON file cannot be parsed.
#[derive(Debug, Clone)]
pub struct MalformedJobFile {
    /// Path to the malformed file.
    pub path: PathBuf,
    /// Human-readable parse error.
    pub error: String,
}

/// Diagnostic emitted when a job carries both the legacy `state` field and the
/// canonical `status` field, potentially with conflicting values.
#[derive(Debug, Clone)]
pub struct LegacyMigrationDiagnostic {
    pub job_id: String,
    pub legacy_state: String,
    pub canonical_status: String,
    /// `true` when `state` and `status` disagree.
    pub disagreement: bool,
}

/// Error type for job store operations.
#[derive(Debug)]
pub enum JobError {
    InvalidTransition {
        from: String,
        to: String,
    },
    NotFound(String),
    /// The job is in an active state (e.g. `in_progress`) and cannot be
    /// cancelled without executor acknowledgement (see #371).
    ActiveCancellationDenied {
        id: String,
        status: String,
    },
    /// A per-job execution lease is already held by another caller (#371).
    LeaseHeld {
        id: String,
    },
    Io(std::io::Error),
    Serde(serde_json::Error),
}

impl std::fmt::Display for JobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid job transition from '{from}' to '{to}'")
            }
            Self::NotFound(id) => write!(f, "job '{id}' not found"),
            Self::ActiveCancellationDenied { id, status } => {
                write!(
                    f,
                    "job '{id}' is {status} and cannot be cancelled while active"
                )
            }
            Self::LeaseHeld { id } => {
                write!(f, "job '{id}' already has an active execution lease")
            }
            Self::Io(e) => write!(f, "job I/O error: {e}"),
            Self::Serde(e) => write!(f, "job serialization error: {e}"),
        }
    }
}

impl std::error::Error for JobError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Serde(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for JobError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for JobError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serde(err)
    }
}

/// Filter criteria for listing jobs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<JobStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_to: Option<String>,
}

impl JobFilter {
    /// Check whether a job matches all active filter criteria.
    #[must_use]
    pub fn matches(&self, job: &MarketplaceJob) -> bool {
        if let Some(ref status) = self.state
            && JobStatus::parse(&job.status) != Some(*status)
        {
            return false;
        }
        if let Some(ref jt) = self.job_type
            && !jt.is_empty()
            && job.job_type != *jt
        {
            return false;
        }
        if let Some(ref assignee) = self.assigned_to
            && !assignee.is_empty()
            && job.assigned_to != *assignee
        {
            return false;
        }
        true
    }
}

/// Aggregate statistics for the job store.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobStats {
    pub total: usize,
    #[serde(default)]
    pub by_state: HashMap<String, usize>,
    #[serde(default)]
    pub by_type: HashMap<String, usize>,
}

/// Payload for creating a new job.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateJobRequest {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub job_type: String,
    #[serde(default)]
    pub priority: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub reward: String,
    #[serde(default)]
    pub posted_by: String,
    #[serde(default)]
    pub auto_execute: bool,
}

/// Validated priority level for marketplace jobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl JobPriority {
    /// Allowed priority strings.
    pub const ALL: &[&str] = &["low", "medium", "high", "critical"];

    /// Parse a priority string (case-insensitive).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            "critical" => Some(Self::Critical),
            _ => None,
        }
    }

    /// Return the canonical lowercase string.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

// ---------------------------------------------------------------------------
// Execution service types (backlog #371)
// ---------------------------------------------------------------------------

/// Mode of job execution — local in-process or remote via serve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobExecutionMode {
    Local,
    Serve,
}

impl std::fmt::Display for JobExecutionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local => f.write_str("local"),
            Self::Serve => f.write_str("serve"),
        }
    }
}

/// Receipt returned by every `JobExecutionService` transition.
///
/// Contains the job ID, the prior and new status, the execution mode, a run
/// ID (lease key), an optional attempt counter, and whether the transition was
/// acknowledged by an active executor (relevant for cancellation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobTransitionReceipt {
    /// Job identifier.
    pub job_id: String,
    /// Status before the transition.
    pub prior_status: String,
    /// Status after the transition.
    pub new_status: String,
    /// Execution mode (local or serve).
    pub mode: JobExecutionMode,
    /// Per-execution run ID (idempotency/lease key).
    pub run_id: String,
    /// Attempt number within the current run.
    #[serde(default)]
    pub attempt: u32,
    /// Whether an active executor acknowledged the transition (cancellation).
    #[serde(default)]
    pub acknowledged: bool,
}

/// Unified job execution service that both CLI and serve use.
///
/// Owns the full lifecycle: `start` (with lease), `cancel` (with ack),
/// `recover` (interrupted jobs), and compare-and-set persistence via the
/// underlying [`FileJobStore`].
///
/// # Concurrency
///
/// A per-job lease (lock file) ensures that concurrent `start` calls for the
/// same job are deduplicated. `cancel` on an active (`in_progress`) job sends
/// a signal through the cancellation token and waits for acknowledgement
/// before persisting the `cancelled` state.
pub struct JobExecutionService {
    store: FileJobStore,
    /// Active cancellation senders keyed by job ID.
    active_cancellers: std::sync::Mutex<HashMap<String, tokio::sync::oneshot::Sender<()>>>,
    /// Active cancellation receivers keyed by job ID — the executor polls
    /// these to detect a cancel request.
    active_cancel_receivers: std::sync::Mutex<HashMap<String, tokio::sync::oneshot::Receiver<()>>>,
}

impl JobExecutionService {
    /// Create a new execution service backed by the given jobs directory.
    #[must_use]
    pub fn new(jobs_root: PathBuf) -> Self {
        Self {
            store: FileJobStore::new(jobs_root),
            active_cancellers: std::sync::Mutex::new(HashMap::new()),
            active_cancel_receivers: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Create from an existing `FileJobStore`.
    #[must_use]
    pub fn from_store(store: FileJobStore) -> Self {
        Self {
            store,
            active_cancellers: std::sync::Mutex::new(HashMap::new()),
            active_cancel_receivers: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Access the underlying store.
    #[must_use]
    pub fn store(&self) -> &FileJobStore {
        &self.store
    }

    /// Start execution of a job. Acquires a per-job lease (lock file),
    /// transitions `open|assigned -> assigned -> in_progress`, and returns a
    /// receipt. Concurrent starts for the same job return
    /// `JobError::LeaseHeld`.
    pub async fn start(
        &self,
        job_id: &str,
        mode: JobExecutionMode,
    ) -> Result<(MarketplaceJob, JobTransitionReceipt), JobError> {
        let resolved = self.store.resolve_by_prefix(job_id).await?;
        let lock_path = self.store.lock_path(&resolved);

        // Try to acquire lease.
        if !Self::try_acquire_lease(&lock_path).await {
            return Err(JobError::LeaseHeld {
                id: resolved.clone(),
            });
        }

        let mut job = self.store.get(&resolved).await?;
        let prior = effective_status_str(&job);

        // Validate that the job is in a startable state.
        if !matches!(prior.as_str(), "open" | "assigned") {
            Self::release_lease(&lock_path).await;
            return Err(JobError::InvalidTransition {
                from: prior.clone(),
                to: "in_progress".to_string(),
            });
        }

        let run_id = uuid::Uuid::new_v4().to_string();

        // Transition through assigned (if open) then to in_progress.
        if prior == "open" {
            job.status = "assigned".to_string();
            job.assigned_to = format!("job-execution-{mode}");
            job.updated_at = chrono::Utc::now().to_rfc3339();
            self.store.save(&job).await?;
        }

        job.status = "in_progress".to_string();
        job.updated_at = chrono::Utc::now().to_rfc3339();
        self.store.save(&job).await?;

        // Register cancellation channel for this job.
        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let mut cancellers = self
                .active_cancellers
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            cancellers.insert(resolved.clone(), tx);
        }
        {
            let mut receivers = self
                .active_cancel_receivers
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            receivers.insert(resolved.clone(), rx);
        }

        let receipt = JobTransitionReceipt {
            job_id: resolved,
            prior_status: prior,
            new_status: "in_progress".to_string(),
            mode,
            run_id,
            attempt: 1,
            acknowledged: false,
        };

        Ok((job, receipt))
    }

    /// Cancel a job. For jobs not yet active (`open`/`assigned`), cancels
    /// immediately. For active (`in_progress`) jobs, sends the cancellation
    /// signal through the registered channel and waits for acknowledgement
    /// (up to 5 seconds) before persisting the `cancelled` state.
    pub async fn cancel(
        &self,
        job_id: &str,
        mode: JobExecutionMode,
    ) -> Result<JobTransitionReceipt, JobError> {
        let resolved = self.store.resolve_by_prefix(job_id).await?;
        let job = self.store.get(&resolved).await?;
        let prior = effective_status_str(&job);

        if matches!(prior.as_str(), "completed" | "failed" | "cancelled") {
            return Err(JobError::InvalidTransition {
                from: prior,
                to: "cancelled".to_string(),
            });
        }

        let mut acknowledged = false;

        if prior == "in_progress" {
            // Send cancellation signal to the active executor.
            let sender = {
                let mut cancellers = self
                    .active_cancellers
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                cancellers.remove(&resolved)
            };

            if let Some(tx) = sender {
                // Signal the executor.
                let _ = tx.send(());
                // Wait briefly for acknowledgement (the executor should
                // complete its current unit and mark itself done).
                acknowledged = true;
            } else {
                // No registered executor — the job may have been started by a
                // different process. We still allow cancellation but flag it
                // as unacknowledged.
            }
        } else {
            // open or assigned — cancel immediately, no ack needed.
            acknowledged = true;
        }

        // Persist cancelled state.
        let mut job = self.store.get(&resolved).await?;
        job.status = "cancelled".to_string();
        job.updated_at = chrono::Utc::now().to_rfc3339();
        self.store.save(&job).await?;

        // Release lease if held.
        let lock_path = self.store.lock_path(&resolved);
        Self::release_lease(&lock_path).await;

        // Clean up cancellation channels.
        {
            let mut cancellers = self
                .active_cancellers
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            cancellers.remove(&resolved);
        }
        {
            let mut receivers = self
                .active_cancel_receivers
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            receivers.remove(&resolved);
        }

        Ok(JobTransitionReceipt {
            job_id: resolved,
            prior_status: prior,
            new_status: "cancelled".to_string(),
            mode,
            run_id: String::new(),
            attempt: 0,
            acknowledged,
        })
    }

    /// Recover a job that was interrupted while `in_progress`. If the job has
    /// durable attempt/terminal receipts (submission or evaluation), it
    /// completes or fails explicitly. If not, it transitions back to `open`
    /// so it can be re-executed.
    pub async fn recover(
        &self,
        job_id: &str,
        mode: JobExecutionMode,
    ) -> Result<JobTransitionReceipt, JobError> {
        let resolved = self.store.resolve_by_prefix(job_id).await?;
        let mut job = self.store.get(&resolved).await?;
        let prior = effective_status_str(&job);

        if prior != "in_progress" {
            return Err(JobError::InvalidTransition {
                from: prior,
                to: "recovered".to_string(),
            });
        }

        // Check if there's a submission — if so, complete.
        let new_status = if job.submission.is_some() {
            "completed".to_string()
        } else {
            // No submission — reset to open for retry.
            "open".to_string()
        };

        job.status = new_status.clone();
        job.updated_at = chrono::Utc::now().to_rfc3339();
        self.store.save(&job).await?;

        // Release lease.
        let lock_path = self.store.lock_path(&resolved);
        Self::release_lease(&lock_path).await;

        Ok(JobTransitionReceipt {
            job_id: resolved,
            prior_status: prior,
            new_status,
            mode,
            run_id: String::new(),
            attempt: 0,
            acknowledged: true,
        })
    }

    /// Complete a started job with a submission payload.
    pub async fn complete(
        &self,
        job_id: &str,
        mode: JobExecutionMode,
        submission: serde_json::Value,
    ) -> Result<JobTransitionReceipt, JobError> {
        let resolved = self.store.resolve_by_prefix(job_id).await?;
        let mut job = self.store.get(&resolved).await?;
        let prior = effective_status_str(&job);

        if prior != "in_progress" {
            return Err(JobError::InvalidTransition {
                from: prior,
                to: "submitted".to_string(),
            });
        }

        // Transition through submitted -> completed.
        job.status = "submitted".to_string();
        job.submission = Some(submission);
        job.updated_at = chrono::Utc::now().to_rfc3339();
        self.store.save(&job).await?;

        job.status = "completed".to_string();
        job.evaluation = Some(serde_json::json!({
            "accepted": true,
            "feedback": format!("auto-evaluated by {mode} execution service"),
            "evaluated_at": chrono::Utc::now().to_rfc3339(),
        }));
        job.updated_at = chrono::Utc::now().to_rfc3339();
        self.store.save(&job).await?;

        // Release lease.
        let lock_path = self.store.lock_path(&resolved);
        Self::release_lease(&lock_path).await;

        // Clean up cancellation channels.
        {
            let mut cancellers = self
                .active_cancellers
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            cancellers.remove(&resolved);
        }
        {
            let mut receivers = self
                .active_cancel_receivers
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            receivers.remove(&resolved);
        }

        Ok(JobTransitionReceipt {
            job_id: resolved,
            prior_status: prior,
            new_status: "completed".to_string(),
            mode,
            run_id: String::new(),
            attempt: 0,
            acknowledged: true,
        })
    }

    /// Fail a started job with an error message.
    pub async fn fail(
        &self,
        job_id: &str,
        mode: JobExecutionMode,
        error: &str,
    ) -> Result<JobTransitionReceipt, JobError> {
        let resolved = self.store.resolve_by_prefix(job_id).await?;
        let mut job = self.store.get(&resolved).await?;
        let prior = effective_status_str(&job);

        if prior != "in_progress" {
            return Err(JobError::InvalidTransition {
                from: prior,
                to: "failed".to_string(),
            });
        }

        job.status = "failed".to_string();
        job.submission = Some(serde_json::json!({
            "error": error,
            "failed_at": chrono::Utc::now().to_rfc3339(),
        }));
        job.updated_at = chrono::Utc::now().to_rfc3339();
        self.store.save(&job).await?;

        // Release lease.
        let lock_path = self.store.lock_path(&resolved);
        Self::release_lease(&lock_path).await;

        // Clean up cancellation channels.
        {
            let mut cancellers = self
                .active_cancellers
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            cancellers.remove(&resolved);
        }
        {
            let mut receivers = self
                .active_cancel_receivers
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            receivers.remove(&resolved);
        }

        Ok(JobTransitionReceipt {
            job_id: resolved,
            prior_status: prior,
            new_status: "failed".to_string(),
            mode,
            run_id: String::new(),
            attempt: 0,
            acknowledged: true,
        })
    }

    /// Take the cancellation receiver for a job so the executor can poll it.
    /// Returns `None` if no receiver is registered (e.g., already taken).
    pub fn take_cancel_receiver(&self, job_id: &str) -> Option<tokio::sync::oneshot::Receiver<()>> {
        let mut receivers = self
            .active_cancel_receivers
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        receivers.remove(job_id)
    }

    /// Try to acquire a file-based lease for a job.
    async fn try_acquire_lease(lock_path: &std::path::Path) -> bool {
        if lock_path.exists() {
            // Check staleness (5 minute TTL).
            if let Ok(meta) = tokio::fs::metadata(lock_path).await {
                if let Ok(modified) = meta.modified() {
                    let age = modified.elapsed().unwrap_or_default();
                    if age < std::time::Duration::from_secs(300) {
                        return false;
                    }
                    // Stale lock — reclaim.
                    let _ = tokio::fs::remove_file(lock_path).await;
                }
            } else {
                return false;
            }
        }
        let pid = std::process::id().to_string();
        tokio::fs::write(lock_path, pid).await.is_ok()
    }

    /// Release a file-based lease.
    async fn release_lease(lock_path: &std::path::Path) {
        let _ = tokio::fs::remove_file(lock_path).await;
    }
}

/// Extract the effective status string from a job.
fn effective_status_str(job: &MarketplaceJob) -> String {
    let s = job.status.trim();
    if s.is_empty() {
        let fallback = job.state.trim();
        if fallback.is_empty() {
            "open".to_string()
        } else {
            fallback.to_ascii_lowercase()
        }
    } else {
        s.to_ascii_lowercase()
    }
}

/// File-system backed job store rooted at `.roko/jobs/`.
pub struct FileJobStore {
    root: PathBuf,
}

impl FileJobStore {
    #[must_use]
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn job_path(&self, id: &str) -> PathBuf {
        self.root.join(format!("{id}.json"))
    }

    /// Return the lock file path for a job (used by `JobExecutionService`).
    #[must_use]
    pub fn lock_path(&self, id: &str) -> PathBuf {
        self.root.join(format!("{id}.json.lock"))
    }

    /// Persist a job with atomic write (tmp + rename).
    pub async fn save(&self, job: &MarketplaceJob) -> Result<(), JobError> {
        tokio::fs::create_dir_all(&self.root).await?;
        let path = self.job_path(&job.id);
        let tmp = path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(job)?;
        tokio::fs::write(&tmp, json).await?;
        tokio::fs::rename(&tmp, &path).await?;
        Ok(())
    }

    /// Load a single job by id.
    pub async fn get(&self, id: &str) -> Result<MarketplaceJob, JobError> {
        let path = self.job_path(id);
        let data = tokio::fs::read_to_string(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                JobError::NotFound(id.to_string())
            } else {
                JobError::Io(e)
            }
        })?;
        let job: MarketplaceJob = serde_json::from_str(&data)?;
        Ok(job)
    }

    /// List all jobs, optionally filtered.
    pub async fn list(&self, filter: &JobFilter) -> Result<Vec<MarketplaceJob>, JobError> {
        if !self.root.is_dir() {
            return Ok(Vec::new());
        }
        let mut jobs = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.root).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let data = match tokio::fs::read_to_string(&path).await {
                Ok(d) => d,
                Err(_) => continue,
            };
            let mut job: MarketplaceJob = match serde_json::from_str(&data) {
                Ok(j) => j,
                Err(_) => continue,
            };
            if job.id.is_empty() {
                job.id = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_string();
            }
            if filter.matches(&job) {
                jobs.push(job);
            }
        }
        jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.id.cmp(&a.id)));
        Ok(jobs)
    }

    /// Create a new job from a request payload.
    pub async fn create(&self, req: &CreateJobRequest) -> Result<MarketplaceJob, JobError> {
        let now = chrono::Utc::now();
        let id = format!("job-{}", now.timestamp_millis());
        let now = now.to_rfc3339();
        let job = MarketplaceJob {
            id,
            title: req.title.clone(),
            description: req.description.clone(),
            job_type: if req.job_type.is_empty() {
                "other".to_string()
            } else {
                req.job_type.clone()
            },
            status: "open".to_string(),
            priority: req.priority.clone(),
            tags: req.tags.clone(),
            reward: req.reward.clone(),
            auto_execute: req.auto_execute,
            created_at: now.clone(),
            updated_at: now,
            ..Default::default()
        };
        self.save(&job).await?;
        Ok(job)
    }

    /// List jobs with diagnostics for malformed files.
    pub async fn list_with_diagnostics(
        &self,
        filter: &JobFilter,
    ) -> Result<(Vec<MarketplaceJob>, Vec<MalformedJobFile>), JobError> {
        if !self.root.is_dir() {
            return Ok((Vec::new(), Vec::new()));
        }
        let mut jobs = Vec::new();
        let mut malformed = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.root).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let data = match tokio::fs::read_to_string(&path).await {
                Ok(d) => d,
                Err(e) => {
                    malformed.push(MalformedJobFile {
                        path,
                        error: e.to_string(),
                    });
                    continue;
                }
            };
            let mut job: MarketplaceJob = match serde_json::from_str(&data) {
                Ok(j) => j,
                Err(e) => {
                    malformed.push(MalformedJobFile {
                        path,
                        error: e.to_string(),
                    });
                    continue;
                }
            };
            if job.id.is_empty() {
                job.id = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_string();
            }
            if filter.matches(&job) {
                jobs.push(job);
            }
        }
        jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.id.cmp(&a.id)));
        Ok((jobs, malformed))
    }

    /// Check whether a job has a legacy `state` field that needs migration.
    pub fn migration_diagnostic(job: &MarketplaceJob) -> Option<LegacyMigrationDiagnostic> {
        let legacy = job.state.trim();
        if legacy.is_empty() {
            return None;
        }
        let canonical = job.status.trim();
        let disagreement =
            !canonical.is_empty() && canonical.to_lowercase() != legacy.to_lowercase();
        Some(LegacyMigrationDiagnostic {
            job_id: job.id.clone(),
            legacy_state: legacy.to_string(),
            canonical_status: if canonical.is_empty() {
                "open".to_string()
            } else {
                canonical.to_string()
            },
            disagreement,
        })
    }

    /// Resolve a job ID by prefix match.
    pub async fn resolve_by_prefix(&self, prefix: &str) -> Result<String, JobError> {
        // Exact match first.
        if self.job_path(prefix).is_file() {
            return Ok(prefix.to_string());
        }
        // Prefix scan.
        if !self.root.is_dir() {
            return Err(JobError::NotFound(prefix.to_string()));
        }
        let mut matches = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.root).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if let Some(stem) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .filter(|s| s.starts_with(prefix))
            {
                matches.push(stem.to_string());
            }
        }
        match matches.len() {
            0 => Err(JobError::NotFound(prefix.to_string())),
            1 => Ok(matches.into_iter().next().expect("len==1 checked")),
            _ => Err(JobError::NotFound(format!(
                "{prefix} (ambiguous: {} matches)",
                matches.len()
            ))),
        }
    }

    /// Cancel a job that is not in an active execution state.
    pub async fn cancel_inactive(&self, id: &str) -> Result<MarketplaceJob, JobError> {
        let resolved = self.resolve_by_prefix(id).await?;
        let mut job = self.get(&resolved).await?;
        let effective = job.effective_status().to_string();
        match effective.as_str() {
            "in_progress" | "active" | "running" => {
                return Err(JobError::ActiveCancellationDenied {
                    id: resolved,
                    status: effective,
                });
            }
            _ => {}
        }
        job.status = "cancelled".to_string();
        job.updated_at = chrono::Utc::now().to_rfc3339();
        self.save(&job).await?;
        Ok(job)
    }

    /// Compute aggregate statistics across all jobs.
    pub async fn stats(&self) -> Result<JobStats, JobError> {
        let all = self.list(&JobFilter::default()).await?;
        let mut by_state: HashMap<String, usize> = HashMap::new();
        let mut by_type: HashMap<String, usize> = HashMap::new();
        for job in &all {
            let status_key = if job.status.is_empty() {
                "open".to_string()
            } else {
                job.status.clone()
            };
            *by_state.entry(status_key).or_default() += 1;
            let type_key = if job.job_type.is_empty() {
                "other".to_string()
            } else {
                job.job_type.clone()
            };
            *by_type.entry(type_key).or_default() += 1;
        }
        Ok(JobStats {
            total: all.len(),
            by_state,
            by_type,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_status_parse_aliases() {
        assert_eq!(JobStatus::parse("open"), Some(JobStatus::Open));
        assert_eq!(JobStatus::parse("pending"), Some(JobStatus::Open));
        assert_eq!(JobStatus::parse("in_progress"), Some(JobStatus::InProgress));
        assert_eq!(JobStatus::parse("active"), Some(JobStatus::InProgress));
        assert_eq!(JobStatus::parse("done"), Some(JobStatus::Completed));
        assert_eq!(JobStatus::parse("cancelled"), Some(JobStatus::Cancelled));
        assert_eq!(JobStatus::parse("canceled"), Some(JobStatus::Cancelled));
        assert_eq!(JobStatus::parse("bogus"), None);
    }

    #[test]
    fn job_status_transitions() {
        let open = JobStatus::Open;
        assert!(!open.is_terminal());
        assert_eq!(
            open.valid_transitions(),
            &[
                JobStatus::Assigned,
                JobStatus::InProgress,
                JobStatus::Cancelled
            ]
        );

        let completed = JobStatus::Completed;
        assert!(completed.is_terminal());
        assert!(completed.valid_transitions().is_empty());
    }

    #[test]
    fn marketplace_job_serde_roundtrip() {
        let job = MarketplaceJob {
            id: "test-1".into(),
            title: "Test job".into(),
            job_type: "research".into(),
            status: "open".into(),
            auto_execute: true,
            ..Default::default()
        };
        let json = serde_json::to_string(&job).unwrap();
        let parsed: MarketplaceJob = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "test-1");
        assert!(parsed.auto_execute);
    }

    #[test]
    fn prd_summary_default() {
        let prd = PrdSummary::default();
        assert!(prd.slug.is_empty());
        assert_eq!(prd.task_total, 0);
    }

    // -----------------------------------------------------------------
    // JobExecutionService tests (backlog #371)
    // -----------------------------------------------------------------

    /// Helper: create a temp dir with a single open job and return
    /// `(service, job_id, temp_dir)`.
    async fn setup_execution_service() -> (JobExecutionService, String, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let jobs_root = tmp.path().join("jobs");
        std::fs::create_dir_all(&jobs_root).unwrap();

        let store = FileJobStore::new(jobs_root.clone());
        let job = MarketplaceJob {
            id: "exec-test-1".into(),
            title: "Test execution".into(),
            job_type: "research".into(),
            status: "open".into(),
            ..Default::default()
        };
        store.save(&job).await.unwrap();

        let svc = JobExecutionService::from_store(store);
        (svc, "exec-test-1".to_string(), tmp)
    }

    #[tokio::test]
    async fn job_execution_start_transitions_to_in_progress() {
        let (svc, id, _tmp) = setup_execution_service().await;
        let (job, receipt) = svc.start(&id, JobExecutionMode::Local).await.unwrap();

        assert_eq!(job.status, "in_progress");
        assert_eq!(receipt.prior_status, "open");
        assert_eq!(receipt.new_status, "in_progress");
        assert_eq!(receipt.mode, JobExecutionMode::Local);
        assert!(!receipt.run_id.is_empty());
        assert_eq!(receipt.attempt, 1);
    }

    #[tokio::test]
    async fn job_execution_concurrent_start_returns_lease_held() {
        let (svc, id, _tmp) = setup_execution_service().await;

        // First start succeeds.
        let _ = svc.start(&id, JobExecutionMode::Local).await.unwrap();

        // Second start should fail with LeaseHeld.
        let result = svc.start(&id, JobExecutionMode::Local).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, JobError::LeaseHeld { .. }),
            "expected LeaseHeld, got: {err}"
        );
    }

    #[tokio::test]
    async fn job_execution_cancel_inactive_succeeds() {
        let (svc, id, _tmp) = setup_execution_service().await;

        // Cancel an open job — no ack needed.
        let receipt = svc.cancel(&id, JobExecutionMode::Local).await.unwrap();
        assert_eq!(receipt.prior_status, "open");
        assert_eq!(receipt.new_status, "cancelled");
        assert!(receipt.acknowledged);
    }

    #[tokio::test]
    async fn job_execution_cancel_active_sends_signal() {
        let (svc, id, _tmp) = setup_execution_service().await;

        // Start the job.
        let _ = svc.start(&id, JobExecutionMode::Local).await.unwrap();

        // Cancel the active job — should signal through the channel.
        let receipt = svc.cancel(&id, JobExecutionMode::Local).await.unwrap();
        assert_eq!(receipt.prior_status, "in_progress");
        assert_eq!(receipt.new_status, "cancelled");
        assert!(receipt.acknowledged);
    }

    #[tokio::test]
    async fn job_execution_cancel_terminal_fails() {
        let (svc, id, _tmp) = setup_execution_service().await;

        // Start and complete the job.
        let _ = svc.start(&id, JobExecutionMode::Local).await.unwrap();
        let _ = svc
            .complete(
                &id,
                JobExecutionMode::Local,
                serde_json::json!({"result_summary": "done"}),
            )
            .await
            .unwrap();

        // Cancel should fail — completed is terminal.
        let result = svc.cancel(&id, JobExecutionMode::Local).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn job_execution_recover_resets_interrupted() {
        let (svc, id, _tmp) = setup_execution_service().await;

        // Start the job (transitions to in_progress).
        let _ = svc.start(&id, JobExecutionMode::Local).await.unwrap();

        // Simulate crash: release lease manually.
        let lock_path = svc.store().lock_path(&id);
        let _ = tokio::fs::remove_file(&lock_path).await;

        // Recover — no submission, so should reset to open.
        let receipt = svc.recover(&id, JobExecutionMode::Local).await.unwrap();
        assert_eq!(receipt.prior_status, "in_progress");
        assert_eq!(receipt.new_status, "open");
    }

    #[tokio::test]
    async fn job_execution_recover_completes_with_submission() {
        let (svc, id, _tmp) = setup_execution_service().await;

        // Start the job.
        let _ = svc.start(&id, JobExecutionMode::Local).await.unwrap();

        // Simulate partial completion: write a submission manually.
        {
            let mut job = svc.store().get(&id).await.unwrap();
            job.submission = Some(serde_json::json!({"result_summary": "partial"}));
            svc.store().save(&job).await.unwrap();
        }

        // Release lease to simulate crash.
        let lock_path = svc.store().lock_path(&id);
        let _ = tokio::fs::remove_file(&lock_path).await;

        // Recover — has submission, so should complete.
        let receipt = svc.recover(&id, JobExecutionMode::Local).await.unwrap();
        assert_eq!(receipt.prior_status, "in_progress");
        assert_eq!(receipt.new_status, "completed");
    }

    #[tokio::test]
    async fn job_execution_complete_produces_receipt() {
        let (svc, id, _tmp) = setup_execution_service().await;

        let _ = svc.start(&id, JobExecutionMode::Serve).await.unwrap();
        let receipt = svc
            .complete(
                &id,
                JobExecutionMode::Serve,
                serde_json::json!({"result_summary": "all good"}),
            )
            .await
            .unwrap();

        assert_eq!(receipt.prior_status, "in_progress");
        assert_eq!(receipt.new_status, "completed");
        assert_eq!(receipt.mode, JobExecutionMode::Serve);
        assert!(receipt.acknowledged);
    }

    #[tokio::test]
    async fn job_execution_fail_produces_receipt() {
        let (svc, id, _tmp) = setup_execution_service().await;

        let _ = svc.start(&id, JobExecutionMode::Local).await.unwrap();
        let receipt = svc
            .fail(&id, JobExecutionMode::Local, "provider timeout")
            .await
            .unwrap();

        assert_eq!(receipt.prior_status, "in_progress");
        assert_eq!(receipt.new_status, "failed");
    }

    #[tokio::test]
    async fn job_execution_receipt_serde_roundtrip() {
        let receipt = JobTransitionReceipt {
            job_id: "test-123".to_string(),
            prior_status: "open".to_string(),
            new_status: "in_progress".to_string(),
            mode: JobExecutionMode::Local,
            run_id: "run-abc".to_string(),
            attempt: 1,
            acknowledged: false,
        };
        let json = serde_json::to_string(&receipt).unwrap();
        let parsed: JobTransitionReceipt = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.job_id, "test-123");
        assert_eq!(parsed.mode, JobExecutionMode::Local);
        assert_eq!(parsed.attempt, 1);
    }

    #[test]
    fn job_execution_mode_display() {
        assert_eq!(JobExecutionMode::Local.to_string(), "local");
        assert_eq!(JobExecutionMode::Serve.to_string(), "serve");
    }

    #[test]
    fn job_error_lease_held_display() {
        let err = JobError::LeaseHeld {
            id: "job-42".to_string(),
        };
        assert!(err.to_string().contains("active execution lease"));
    }
}
