# B1: Job type system in roko-core

## Context

**Repo:** `/Users/will/dev/nunchi/roko/roko`
**Branch:** `demo-backend`
**Language:** Rust (workspace with ~29 crates)
**Key crate paths:**
- CLI + orchestrator: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/`
- Core types: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/`
- HTTP server: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/`
- Agent dispatch: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/`

**Key files:**
- Orchestrator (20K lines): `crates/roko-cli/src/orchestrate.rs`
- CLI entry: `crates/roko-cli/src/main.rs`
- Server routes: `crates/roko-serve/src/routes/mod.rs`
- Server state: `crates/roko-serve/src/state.rs`
- Server events: `crates/roko-serve/src/events.rs`
- Server WS: `crates/roko-serve/src/routes/ws.rs`

**Architecture:**
- `roko-serve` is an axum HTTP server on port 6677 with ~85 REST routes + WebSocket
- `AppState` uses `tokio::sync::RwLock` — all lock ops are `.read().await` / `.write().await` (NOT `.unwrap()`)
- Event bus: `state.event_bus.publish(event)` — always present, no Option wrapping
- The TUI gets data two ways: (1) StateHub push via `watch<DashboardSnapshot>` channel, (2) file polling via `DashboardData::tick()` reading `.roko/` files

### Pre-commit (MANDATORY)

```bash
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

---

## What this task does

Create the complete job type system as a new module in `roko-core`. Jobs represent units of work (research tasks, coding tasks, reviews) that agents claim, execute, and submit results for. The module includes type definitions, state machine validation, file-backed persistence, and unit tests.

---

## Dependencies

- `uuid` is already in workspace deps at `/Users/will/dev/nunchi/roko/roko/Cargo.toml` line 133:
  `uuid = { version = "1", features = ["v4", "serde"] }`
- `uuid` is **not** in `roko-core`'s Cargo.toml — add it as shown below.
- `serde`, `serde_json`, `chrono`, `thiserror` are already in roko-core's Cargo.toml.
- Tests require `tempfile` — check if it is already a dev-dependency in roko-core; add it if not.

---

## Steps

- [ ] **Add `uuid` dependency to roko-core.**
  Open `/Users/will/dev/nunchi/roko/roko/crates/roko-core/Cargo.toml` and add under `[dependencies]`, after the `tracing` line:
  ```toml
  uuid = { workspace = true }
  ```

- [ ] **Create the jobs module.**
  Create `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/jobs.rs` with the full contents below.

- [ ] **Register the module in lib.rs.**
  Open `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/lib.rs`.
  Add `pub mod jobs;` after line 74 (after `pub mod hash;`).
  Add a re-export block after the `immune` re-exports:
  ```rust
  pub use jobs::{
      CreateJobRequest, FileJobStore, Job, JobEvaluation, JobFilter, JobGateResult, JobState,
      JobStats, JobSubmission, JobType,
  };
  ```

---

## Full contents of `roko-core/src/jobs.rs`

```rust
//! Job type system for the roko work marketplace.
//!
//! Jobs represent discrete units of work that agents claim, execute, and
//! submit results for. Each job follows a strict state machine:
//!
//! ```text
//! Open ──────────────────────────────────────── Cancelled
//!  │
//!  ▼
//! Assigned ──────────────────────────────────── Cancelled
//!  │
//!  ▼
//! InProgress ────────────────────────────────── Cancelled
//!  │
//!  ▼
//! Submitted
//!  │
//!  ▼
//! Evaluated
//! ```
//!
//! Terminal states (`Evaluated`, `Cancelled`) have no outgoing transitions.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during job operations.
#[derive(Debug, Error)]
pub enum JobError {
    #[error("invalid state transition from {from} to {to}")]
    InvalidTransition { from: JobState, to: JobState },

    #[error("job not found: {0}")]
    NotFound(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

// ---------------------------------------------------------------------------
// JobType
// ---------------------------------------------------------------------------

/// The category of work a job represents.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobType {
    /// Deep research on a topic, producing a report with citations.
    Research,
    /// Write or modify code in the repository.
    CodingTask,
    /// Review a pull request or code change.
    Review,
    /// Generate or update documentation.
    Documentation,
    /// Run a test suite and report results.
    Testing,
    /// A freeform job type with a custom label.
    Custom(String),
}

impl std::fmt::Display for JobType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Research      => f.write_str("research"),
            Self::CodingTask    => f.write_str("coding_task"),
            Self::Review        => f.write_str("review"),
            Self::Documentation => f.write_str("documentation"),
            Self::Testing       => f.write_str("testing"),
            Self::Custom(label) => write!(f, "custom:{label}"),
        }
    }
}

// ---------------------------------------------------------------------------
// JobState
// ---------------------------------------------------------------------------

/// Lifecycle state of a job.
///
/// Transitions are enforced by [`JobState::transition_to`]. Only moves listed
/// in [`JobState::valid_transitions`] are permitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    /// Posted and waiting for an agent to claim it.
    Open,
    /// Claimed by an agent; work has not yet started.
    Assigned,
    /// The agent is actively working on the job.
    InProgress,
    /// The agent has submitted results; pending evaluation.
    Submitted,
    /// Results have been evaluated (accepted or rejected). Terminal state.
    Evaluated,
    /// The job was cancelled before completion. Terminal state.
    Cancelled,
}

impl JobState {
    /// Return the set of states that are legal to transition to from `self`.
    ///
    /// # Examples
    ///
    /// ```
    /// use roko_core::jobs::JobState;
    /// assert!(JobState::Open.valid_transitions().contains(&JobState::Assigned));
    /// assert!(!JobState::Open.valid_transitions().contains(&JobState::Evaluated));
    /// ```
    #[must_use]
    pub fn valid_transitions(self) -> &'static [Self] {
        match self {
            Self::Open       => &[Self::Assigned, Self::Cancelled],
            Self::Assigned   => &[Self::InProgress, Self::Cancelled],
            Self::InProgress => &[Self::Submitted, Self::Cancelled],
            Self::Submitted  => &[Self::Evaluated],
            // Terminal states — no outgoing transitions.
            Self::Evaluated  => &[],
            Self::Cancelled  => &[],
        }
    }

    /// Return `true` if transitioning from `self` to `target` is permitted.
    ///
    /// # Examples
    ///
    /// ```
    /// use roko_core::jobs::JobState;
    /// assert!( JobState::Open.can_transition_to(JobState::Assigned));
    /// assert!(!JobState::Open.can_transition_to(JobState::InProgress));
    /// ```
    #[must_use]
    pub fn can_transition_to(self, target: Self) -> bool {
        self.valid_transitions().contains(&target)
    }

    /// Attempt the transition to `target`.
    ///
    /// Returns `Ok(target)` on success or [`JobError::InvalidTransition`]
    /// if the move is not permitted by the state machine.
    pub fn transition_to(self, target: Self) -> Result<Self, JobError> {
        if self.can_transition_to(target) {
            Ok(target)
        } else {
            Err(JobError::InvalidTransition { from: self, to: target })
        }
    }

    /// Return `true` if this is a terminal state (no further transitions).
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Evaluated | Self::Cancelled)
    }
}

impl std::fmt::Display for JobState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Open       => "open",
            Self::Assigned   => "assigned",
            Self::InProgress => "in_progress",
            Self::Submitted  => "submitted",
            Self::Evaluated  => "evaluated",
            Self::Cancelled  => "cancelled",
        })
    }
}

// ---------------------------------------------------------------------------
// Job
// ---------------------------------------------------------------------------

/// A single job in the work marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Unique identifier (format: `job-{uuid}`).
    pub id: String,
    /// Human-readable title.
    pub title: String,
    /// Detailed description of the work to be done.
    pub description: String,
    /// What category of work this represents.
    pub job_type: JobType,
    /// Current lifecycle state.
    pub state: JobState,
    /// Who posted the job (agent ID or `"human"`).
    pub posted_by: String,
    /// Which agent has claimed the job, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_to: Option<String>,
    /// When the job was created.
    pub created_at: DateTime<Utc>,
    /// When the job was last modified.
    pub updated_at: DateTime<Utc>,
    /// Submission payload attached when the agent finishes work.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submission: Option<JobSubmission>,
    /// Evaluation result after the submission is reviewed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluation: Option<JobEvaluation>,
    /// Freeform metadata tags.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

impl Job {
    /// Create a new job in the `Open` state.
    ///
    /// The ID is assigned automatically (`job-{uuid_v4}`).
    ///
    /// # Examples
    ///
    /// ```
    /// use roko_core::jobs::{Job, JobState, JobType};
    ///
    /// let job = Job::new("My task".into(), "Do the thing".into(), JobType::Research, "human".into());
    /// assert_eq!(job.state, JobState::Open);
    /// assert!(job.id.starts_with("job-"));
    /// ```
    #[must_use]
    pub fn new(
        title: String,
        description: String,
        job_type: JobType,
        posted_by: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: format!("job-{}", uuid::Uuid::new_v4()),
            title,
            description,
            job_type,
            state: JobState::Open,
            posted_by,
            assigned_to: None,
            created_at: now,
            updated_at: now,
            submission: None,
            evaluation: None,
            metadata: HashMap::new(),
        }
    }

    /// Transition to `target`, updating `updated_at` on success.
    ///
    /// Returns `Err` if the transition is not permitted by the state machine.
    pub fn transition(&mut self, target: JobState) -> Result<(), JobError> {
        self.state = self.state.transition_to(target)?;
        self.updated_at = Utc::now();
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// JobSubmission
// ---------------------------------------------------------------------------

/// Payload an agent attaches when finishing a job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSubmission {
    /// The agent that submitted.
    pub agent_id: String,
    /// Freeform summary of what was done.
    pub result_summary: String,
    /// Paths to artifacts produced (diffs, reports, output files).
    #[serde(default)]
    pub artifacts: Vec<String>,
    /// Results from automated gate checks.
    #[serde(default)]
    pub gate_results: Vec<JobGateResult>,
    /// When the submission was created.
    pub submitted_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// JobGateResult
// ---------------------------------------------------------------------------

/// A single automated gate check result attached to a submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobGateResult {
    /// Gate name (e.g., `"compile"`, `"test"`, `"clippy"`).
    pub gate: String,
    /// Whether the gate passed.
    pub passed: bool,
    /// Human-readable detail or error output.
    #[serde(default)]
    pub detail: String,
}

// ---------------------------------------------------------------------------
// JobEvaluation
// ---------------------------------------------------------------------------

/// Evaluation outcome for a submitted job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobEvaluation {
    /// Who performed the evaluation (agent ID or `"human"`).
    pub evaluator: String,
    /// Whether the submission was accepted.
    pub accepted: bool,
    /// Numeric quality score in `[0.0, 1.0]`, if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    /// Freeform feedback for the submitter.
    #[serde(default)]
    pub feedback: String,
    /// When the evaluation was performed.
    pub evaluated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// CreateJobRequest
// ---------------------------------------------------------------------------

/// Request payload for creating a new job via the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateJobRequest {
    /// Human-readable title (must not be blank).
    pub title: String,
    /// Detailed description (must not be blank).
    pub description: String,
    /// Job type.
    pub job_type: JobType,
    /// Who is posting. Defaults to `"human"` if empty or absent.
    #[serde(default)]
    pub posted_by: String,
    /// Optional metadata tags.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// JobFilter
// ---------------------------------------------------------------------------

/// Filter criteria for listing jobs.
///
/// All fields are optional. Unset fields match any value.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobFilter {
    /// Restrict to jobs in this state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<JobState>,
    /// Restrict to jobs of this type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_type: Option<JobType>,
    /// Restrict to jobs assigned to this agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_to: Option<String>,
}

impl JobFilter {
    /// Return `true` if `job` matches all set criteria.
    ///
    /// # Examples
    ///
    /// ```
    /// use roko_core::jobs::{Job, JobFilter, JobState, JobType};
    ///
    /// let job = Job::new("T".into(), "d".into(), JobType::Research, "human".into());
    /// let f = JobFilter { state: Some(JobState::Open), ..Default::default() };
    /// assert!(f.matches(&job));
    /// ```
    #[must_use]
    pub fn matches(&self, job: &Job) -> bool {
        if let Some(ref state) = self.state {
            if &job.state != state {
                return false;
            }
        }
        if let Some(ref jt) = self.job_type {
            if &job.job_type != jt {
                return false;
            }
        }
        if let Some(ref assignee) = self.assigned_to {
            match &job.assigned_to {
                Some(a) if a == assignee => {}
                _ => return false,
            }
        }
        true
    }
}

// ---------------------------------------------------------------------------
// FileJobStore
// ---------------------------------------------------------------------------

/// File-backed job store.
///
/// Each job is persisted as a pretty-printed JSON file at
/// `{dir}/{job-id}.json`. Writes are atomic (write to `.tmp`, then rename).
/// Corrupt files are silently skipped during listing.
///
/// # Examples
///
/// ```no_run
/// use roko_core::jobs::{CreateJobRequest, FileJobStore, JobFilter, JobType};
/// use std::collections::HashMap;
///
/// let store = FileJobStore::new("/tmp/roko-jobs").expect("create store");
/// let job = store.create(CreateJobRequest {
///     title: "Research DeFi".into(),
///     description: "Survey AMM protocols".into(),
///     job_type: JobType::Research,
///     posted_by: "human".into(),
///     metadata: HashMap::new(),
/// }).expect("create job");
/// println!("Created: {}", job.id);
/// ```
#[derive(Debug, Clone)]
pub struct FileJobStore {
    dir: PathBuf,
}

impl FileJobStore {
    /// Create a store rooted at `dir`.
    ///
    /// Creates the directory (and any missing parents) if it does not exist.
    /// Returns an error only if directory creation fails.
    pub fn new(dir: impl Into<PathBuf>) -> Result<Self, JobError> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    /// Create a store at the default location: `{workdir}/.roko/jobs/`.
    pub fn for_workdir(workdir: &Path) -> Result<Self, JobError> {
        Self::new(workdir.join(".roko").join("jobs"))
    }

    fn job_path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    /// Atomically persist `job` to disk.
    ///
    /// Writes to a `.tmp` file first, then renames into place so that a crash
    /// during write cannot corrupt an existing file.
    pub fn save(&self, job: &Job) -> Result<(), JobError> {
        let path = self.job_path(&job.id);
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(job)?)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    /// Load a single job by ID.
    ///
    /// Returns [`JobError::NotFound`] if no file exists for `id`.
    pub fn get(&self, id: &str) -> Result<Job, JobError> {
        let path = self.job_path(id);
        if !path.exists() {
            return Err(JobError::NotFound(id.to_owned()));
        }
        let content = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&content)?)
    }

    /// List all jobs, optionally restricted by `filter`.
    ///
    /// Results are sorted descending by `created_at` (newest first).
    /// Files that are missing or contain invalid JSON are silently skipped.
    pub fn list(&self, filter: &JobFilter) -> Result<Vec<Job>, JobError> {
        let entries = match std::fs::read_dir(&self.dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };

        let mut jobs = Vec::new();
        for entry in entries {
            let path = entry?.path();

            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            // Skip non-JSON files and .tmp artifacts from atomic writes.
            if !name.ends_with(".json") || name.ends_with(".json.tmp") {
                continue;
            }

            let content = std::fs::read_to_string(&path)?;
            let job: Job = match serde_json::from_str(&content) {
                Ok(j)  => j,
                Err(_) => continue, // skip corrupt files
            };

            if filter.matches(&job) {
                jobs.push(job);
            }
        }

        jobs.sort_unstable_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(jobs)
    }

    /// Create a new job from a request and persist it.
    ///
    /// Normalizes `posted_by` to `"human"` if the field is empty.
    pub fn create(&self, req: CreateJobRequest) -> Result<Job, JobError> {
        let posted_by = if req.posted_by.is_empty() {
            "human".to_owned()
        } else {
            req.posted_by
        };
        let mut job = Job::new(req.title, req.description, req.job_type, posted_by);
        job.metadata = req.metadata;
        self.save(&job)?;
        Ok(job)
    }

    /// Overwrite a job on disk.
    ///
    /// Returns [`JobError::NotFound`] if no file exists for `job.id`.
    /// Callers should have loaded the job via [`get`][Self::get] before mutating it.
    pub fn update(&self, job: &Job) -> Result<(), JobError> {
        if !self.job_path(&job.id).exists() {
            return Err(JobError::NotFound(job.id.clone()));
        }
        self.save(job)
    }

    /// Return aggregate statistics across all stored jobs.
    pub fn stats(&self) -> Result<JobStats, JobError> {
        let jobs = self.list(&JobFilter::default())?;
        let mut by_state: HashMap<JobState, usize> = HashMap::new();
        let mut by_type: HashMap<String, usize> = HashMap::new();
        for job in &jobs {
            *by_state.entry(job.state).or_default() += 1;
            *by_type.entry(job.job_type.to_string()).or_default() += 1;
        }
        Ok(JobStats { total: jobs.len(), by_state, by_type })
    }
}

// ---------------------------------------------------------------------------
// JobStats
// ---------------------------------------------------------------------------

/// Aggregate statistics for the job store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStats {
    /// Total number of jobs across all states.
    pub total: usize,
    /// Count per state.
    pub by_state: HashMap<JobState, usize>,
    /// Count per job type (key is the `Display` string).
    pub by_type: HashMap<String, usize>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // -- State machine --

    #[test]
    fn valid_transitions_are_accepted() {
        assert!(JobState::Open.can_transition_to(JobState::Assigned));
        assert!(JobState::Open.can_transition_to(JobState::Cancelled));
        assert!(JobState::Assigned.can_transition_to(JobState::InProgress));
        assert!(JobState::Assigned.can_transition_to(JobState::Cancelled));
        assert!(JobState::InProgress.can_transition_to(JobState::Submitted));
        assert!(JobState::InProgress.can_transition_to(JobState::Cancelled));
        assert!(JobState::Submitted.can_transition_to(JobState::Evaluated));
    }

    #[test]
    fn invalid_transitions_are_rejected() {
        // Can't skip states forward.
        assert!(!JobState::Open.can_transition_to(JobState::InProgress));
        assert!(!JobState::Open.can_transition_to(JobState::Submitted));
        assert!(!JobState::Open.can_transition_to(JobState::Evaluated));
        assert!(!JobState::Assigned.can_transition_to(JobState::Submitted));
        assert!(!JobState::Assigned.can_transition_to(JobState::Evaluated));
        assert!(!JobState::InProgress.can_transition_to(JobState::Assigned));

        // Terminal states have no outgoing transitions.
        assert!(!JobState::Evaluated.can_transition_to(JobState::Open));
        assert!(!JobState::Evaluated.can_transition_to(JobState::Cancelled));
        assert!(!JobState::Cancelled.can_transition_to(JobState::Open));
        assert!(!JobState::Cancelled.can_transition_to(JobState::Assigned));

        // Can't cancel an already-submitted job.
        assert!(!JobState::Submitted.can_transition_to(JobState::Cancelled));
    }

    #[test]
    fn transition_to_returns_error_message() {
        let err = JobState::Open.transition_to(JobState::Evaluated).unwrap_err();
        assert!(err.to_string().contains("invalid state transition"));
        assert!(err.to_string().contains("open"));
        assert!(err.to_string().contains("evaluated"));
    }

    #[test]
    fn terminal_states_are_identified() {
        assert!(!JobState::Open.is_terminal());
        assert!(!JobState::Assigned.is_terminal());
        assert!(!JobState::InProgress.is_terminal());
        assert!(!JobState::Submitted.is_terminal());
        assert!(JobState::Evaluated.is_terminal());
        assert!(JobState::Cancelled.is_terminal());
    }

    // -- Job struct --

    #[test]
    fn new_job_starts_open() {
        let job = Job::new("T".into(), "D".into(), JobType::Research, "human".into());
        assert_eq!(job.state, JobState::Open);
        assert!(job.id.starts_with("job-"));
        assert_eq!(job.assigned_to, None);
        assert!(job.submission.is_none());
        assert!(job.evaluation.is_none());
    }

    #[test]
    fn job_transition_updates_timestamp() {
        let mut job = Job::new("T".into(), "D".into(), JobType::CodingTask, "human".into());
        let before = job.updated_at;
        // Sleep is too slow for unit tests; instead we just verify the state changed.
        job.transition(JobState::Assigned).unwrap();
        assert_eq!(job.state, JobState::Assigned);
        // updated_at should be >= the original (may be equal if clock resolution is low).
        assert!(job.updated_at >= before);
    }

    #[test]
    fn job_full_lifecycle() {
        let mut job = Job::new("Research DeFi".into(), "Survey protocols".into(), JobType::Research, "human".into());

        job.transition(JobState::Assigned).unwrap();
        job.assigned_to = Some("agent-1".into());
        assert_eq!(job.state, JobState::Assigned);

        job.transition(JobState::InProgress).unwrap();
        assert_eq!(job.state, JobState::InProgress);

        job.transition(JobState::Submitted).unwrap();
        job.submission = Some(JobSubmission {
            agent_id: "agent-1".into(),
            result_summary: "Found 5 protocols".into(),
            artifacts: vec!["report.md".into()],
            gate_results: vec![JobGateResult { gate: "format".into(), passed: true, detail: String::new() }],
            submitted_at: Utc::now(),
        });
        assert_eq!(job.state, JobState::Submitted);

        job.transition(JobState::Evaluated).unwrap();
        job.evaluation = Some(JobEvaluation {
            evaluator: "human".into(),
            accepted: true,
            score: Some(0.9),
            feedback: "Good work".into(),
            evaluated_at: Utc::now(),
        });
        assert_eq!(job.state, JobState::Evaluated);
        assert!(job.state.is_terminal());
    }

    #[test]
    fn job_cancel_path() {
        let mut job = Job::new("T".into(), "D".into(), JobType::Review, "human".into());
        job.transition(JobState::Assigned).unwrap();
        job.transition(JobState::Cancelled).unwrap();
        assert_eq!(job.state, JobState::Cancelled);
        assert!(job.state.is_terminal());
        // Cannot re-open a cancelled job.
        assert!(job.transition(JobState::Open).is_err());
    }

    // -- Serde --

    #[test]
    fn job_serializes_roundtrip() {
        let job = Job::new("RT".into(), "test".into(), JobType::Custom("audit".into()), "human".into());
        let json = serde_json::to_string(&job).unwrap();
        let loaded: Job = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.id, job.id);
        assert_eq!(loaded.job_type, JobType::Custom("audit".into()));
        assert_eq!(loaded.state, JobState::Open);
    }

    #[test]
    fn job_state_serializes_as_snake_case() {
        assert_eq!(serde_json::to_string(&JobState::InProgress).unwrap(), "\"in_progress\"");
        assert_eq!(serde_json::to_string(&JobState::Open).unwrap(), "\"open\"");
    }

    // -- JobFilter --

    #[test]
    fn filter_matches_state() {
        let job = Job::new("T".into(), "D".into(), JobType::Research, "human".into());
        let f = JobFilter { state: Some(JobState::Open), ..Default::default() };
        assert!(f.matches(&job));

        let f2 = JobFilter { state: Some(JobState::Assigned), ..Default::default() };
        assert!(!f2.matches(&job));
    }

    #[test]
    fn filter_matches_job_type() {
        let job = Job::new("T".into(), "D".into(), JobType::Review, "human".into());
        assert!(JobFilter { job_type: Some(JobType::Review), ..Default::default() }.matches(&job));
        assert!(!JobFilter { job_type: Some(JobType::Research), ..Default::default() }.matches(&job));
    }

    #[test]
    fn filter_matches_assigned_to() {
        let mut job = Job::new("T".into(), "D".into(), JobType::Review, "human".into());
        job.assigned_to = Some("agent-x".into());

        let f_match = JobFilter { assigned_to: Some("agent-x".into()), ..Default::default() };
        assert!(f_match.matches(&job));

        let f_miss = JobFilter { assigned_to: Some("agent-y".into()), ..Default::default() };
        assert!(!f_miss.matches(&job));

        // Unassigned job never matches an assignee filter.
        let mut unassigned = Job::new("T2".into(), "D".into(), JobType::Review, "human".into());
        unassigned.assigned_to = None;
        assert!(!f_match.matches(&unassigned));
    }

    #[test]
    fn filter_default_matches_everything() {
        let job = Job::new("T".into(), "D".into(), JobType::Research, "human".into());
        assert!(JobFilter::default().matches(&job));
    }

    // -- FileJobStore --

    #[test]
    fn store_new_creates_directory() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sub").join("jobs");
        assert!(!path.exists());
        FileJobStore::new(&path).expect("create store");
        assert!(path.is_dir());
    }

    #[test]
    fn store_crud() {
        let dir = tempdir().unwrap();
        let store = FileJobStore::new(dir.path().join("jobs")).unwrap();

        let job = store.create(CreateJobRequest {
            title: "Test".into(),
            description: "Do it".into(),
            job_type: JobType::CodingTask,
            posted_by: "human".into(),
            metadata: HashMap::new(),
        }).unwrap();

        // Get
        let loaded = store.get(&job.id).unwrap();
        assert_eq!(loaded.title, "Test");
        assert_eq!(loaded.state, JobState::Open);

        // List
        let all = store.list(&JobFilter::default()).unwrap();
        assert_eq!(all.len(), 1);

        // Update
        let mut updated = loaded;
        updated.transition(JobState::Assigned).unwrap();
        store.update(&updated).unwrap();
        let reloaded = store.get(&updated.id).unwrap();
        assert_eq!(reloaded.state, JobState::Assigned);
    }

    #[test]
    fn store_get_returns_not_found() {
        let dir = tempdir().unwrap();
        let store = FileJobStore::new(dir.path().join("jobs")).unwrap();
        let err = store.get("job-nonexistent").unwrap_err();
        assert!(matches!(err, JobError::NotFound(_)));
    }

    #[test]
    fn store_update_returns_not_found_for_unknown_id() {
        let dir = tempdir().unwrap();
        let store = FileJobStore::new(dir.path().join("jobs")).unwrap();
        let fake = Job::new("T".into(), "D".into(), JobType::Research, "human".into());
        let err = store.update(&fake).unwrap_err();
        assert!(matches!(err, JobError::NotFound(_)));
    }

    #[test]
    fn store_list_empty_when_no_files() {
        let dir = tempdir().unwrap();
        let store = FileJobStore::new(dir.path().join("jobs")).unwrap();
        let jobs = store.list(&JobFilter::default()).unwrap();
        assert!(jobs.is_empty());
    }

    #[test]
    fn store_list_skips_corrupt_files() {
        let dir = tempdir().unwrap();
        let jobs_dir = dir.path().join("jobs");
        FileJobStore::new(&jobs_dir).unwrap();

        // Write a corrupt file.
        std::fs::write(jobs_dir.join("job-bad.json"), b"NOT JSON").unwrap();

        let store = FileJobStore::new(&jobs_dir).unwrap();
        let list = store.list(&JobFilter::default()).unwrap();
        assert!(list.is_empty(), "corrupt file should be silently skipped");
    }

    #[test]
    fn store_filter_by_state() {
        let dir = tempdir().unwrap();
        let store = FileJobStore::new(dir.path().join("jobs")).unwrap();

        store.create(CreateJobRequest {
            title: "A".into(), description: "a".into(),
            job_type: JobType::Research, posted_by: "human".into(),
            metadata: HashMap::new(),
        }).unwrap();

        let mut job_b = store.create(CreateJobRequest {
            title: "B".into(), description: "b".into(),
            job_type: JobType::Research, posted_by: "human".into(),
            metadata: HashMap::new(),
        }).unwrap();
        job_b.transition(JobState::Assigned).unwrap();
        job_b.assigned_to = Some("agent-1".into());
        store.update(&job_b).unwrap();

        let open = store.list(&JobFilter { state: Some(JobState::Open), ..Default::default() }).unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].title, "A");

        let assigned = store.list(&JobFilter { state: Some(JobState::Assigned), ..Default::default() }).unwrap();
        assert_eq!(assigned.len(), 1);
        assert_eq!(assigned[0].title, "B");
    }

    #[test]
    fn store_stats() {
        let dir = tempdir().unwrap();
        let store = FileJobStore::new(dir.path().join("jobs")).unwrap();

        store.create(CreateJobRequest {
            title: "R1".into(), description: "research".into(),
            job_type: JobType::Research, posted_by: "human".into(),
            metadata: HashMap::new(),
        }).unwrap();
        store.create(CreateJobRequest {
            title: "C1".into(), description: "code".into(),
            job_type: JobType::CodingTask, posted_by: "human".into(),
            metadata: HashMap::new(),
        }).unwrap();

        let stats = store.stats().unwrap();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.by_state.get(&JobState::Open), Some(&2));
        assert_eq!(stats.by_type.get("research"), Some(&1));
        assert_eq!(stats.by_type.get("coding_task"), Some(&1));
    }

    #[test]
    fn store_posted_by_defaults_to_human() {
        let dir = tempdir().unwrap();
        let store = FileJobStore::new(dir.path().join("jobs")).unwrap();
        let job = store.create(CreateJobRequest {
            title: "T".into(), description: "D".into(),
            job_type: JobType::Testing, posted_by: String::new(),
            metadata: HashMap::new(),
        }).unwrap();
        assert_eq!(job.posted_by, "human");
    }

    #[test]
    fn store_save_is_atomic() {
        // Verify that .tmp files are cleaned up after a successful write.
        let dir = tempdir().unwrap();
        let store = FileJobStore::new(dir.path().join("jobs")).unwrap();
        let job = store.create(CreateJobRequest {
            title: "T".into(), description: "D".into(),
            job_type: JobType::Review, posted_by: "human".into(),
            metadata: HashMap::new(),
        }).unwrap();

        let tmp_path = store.job_path(&job.id).with_extension("json.tmp");
        assert!(!tmp_path.exists(), ".tmp file must not remain after successful write");
    }
}
```

---

## Verification

```bash
cd /Users/will/dev/nunchi/roko/roko

# Check the module compiles
cargo check -p roko-core 2>&1 | head -20

# Run only the jobs tests
cargo test -p roko-core -- jobs:: --nocapture

# Confirm no clippy warnings
cargo clippy -p roko-core --no-deps -- -D warnings 2>&1 | head -20

# Formatting
cargo +nightly fmt --all -- --check
```

Expected: all tests pass, no clippy warnings, format check clean.
