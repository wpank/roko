//! Single-file, checksummed orchestration state snapshot.
//!
//! All four mutable state groups (executor, orchestrator, run counters, gate thresholds)
//! are serialized into this struct and written atomically in one `atomic_write` call.
//! The `checksum` field is a domain-separated SHA-256 digest over
//! length-prefixed inner payloads (computed before they are embedded).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

/// Bump this constant whenever the shape of `StateSnapshot` changes in an incompatible way.
/// Resume code must reject snapshots with a different version.
pub const STATE_SNAPSHOT_VERSION: u32 = 2;

/// Maximum encoded size accepted for either the unified snapshot or its
/// read-only legacy executor fallback.
pub const MAX_DURABLE_RUNNER_PROJECTION_BYTES: u64 = 16 * 1024 * 1024;

/// Canonical unified Runner-v2 snapshot path relative to a workspace root.
pub const STATE_SNAPSHOT_RELATIVE_PATH: &str = ".roko/state/state-snapshot.json";

/// Legacy executor projection path relative to a workspace root.
pub const LEGACY_EXECUTOR_RELATIVE_PATH: &str = ".roko/state/executor.json";

/// All runtime state groups bundled for a single atomic write.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// Schema version -- compared against `STATE_SNAPSHOT_VERSION` on load.
    pub version: u32,
    /// Wall-clock timestamp of this snapshot (milliseconds since Unix epoch).
    pub timestamp_ms: u64,
    /// Executor snapshot JSON (opaque to roko-runtime; owned by the CLI runner).
    pub executor_json: String,
    /// Orchestrator snapshot JSON (opaque; includes merge queue).
    pub orchestrator_json: String,
    /// Run-state counters JSON.
    pub run_state_json: String,
    /// Gate threshold EMA state JSON.
    pub gate_thresholds_json: String,
    /// SHA-256 digest over domain-separated, length-prefixed embedded fields.
    /// Version 1 used raw concatenation and remains read-compatible.
    pub checksum: String,
}

/// Durable source selected for a Runner projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunnerProjectionSource {
    /// Checksummed Runner-v2 `state-snapshot.json`.
    StateSnapshot,
    /// Read-only compatibility with the pre-unified `executor.json` format.
    LegacyExecutor,
}

impl RunnerProjectionSource {
    /// Stable source label exposed by CLI and HTTP projection surfaces.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::StateSnapshot => "state_snapshot",
            Self::LegacyExecutor => "legacy_executor",
        }
    }
}

impl fmt::Display for RunnerProjectionSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

/// Verified durable Runner projection shared by runtime, CLI/TUI, and HTTP.
///
/// The four embedded payloads are parsed before this value is returned. A
/// present unified snapshot is authoritative: corruption never falls through
/// to the legacy executor file.
#[derive(Debug, Clone, PartialEq)]
pub struct DurableRunnerProjection {
    /// Format selected by the loader.
    pub source: RunnerProjectionSource,
    /// Exact file path that supplied the projection.
    pub source_path: PathBuf,
    /// Stable identity for the exact durable generation.
    pub generation: String,
    /// Parsed executor payload used for plan/task/agent/gate projections.
    pub executor: serde_json::Value,
    /// Canonical cross-surface view enriched from `run_state` lifecycle data.
    pub executor_projection: serde_json::Value,
    /// Parsed orchestrator payload, present for unified snapshots.
    pub orchestrator: Option<serde_json::Value>,
    /// Parsed run-state payload, present for unified snapshots.
    pub run_state: Option<serde_json::Value>,
    /// Parsed gate-threshold payload, present for unified snapshots.
    pub gate_thresholds: Option<serde_json::Value>,
    /// Verified outer snapshot, absent only for the legacy fallback.
    pub snapshot: Option<StateSnapshot>,
}

/// Canonical recovered dashboard state plus its durable Runner source.
#[derive(Debug, Clone)]
pub struct DurableDashboardProjection {
    /// Materialized plan/task/agent/gate dashboard state.
    pub dashboard: roko_core::dashboard_snapshot::DashboardSnapshot,
    /// Durable Runner input, or `None` when neither current nor legacy state exists.
    pub runner: Option<DurableRunnerProjection>,
}

impl DurableRunnerProjection {
    /// Return the exact raw embedded gate-threshold JSON when available.
    #[must_use]
    pub fn gate_thresholds_raw(&self) -> Option<&str> {
        self.snapshot
            .as_ref()
            .map(|snapshot| snapshot.gate_thresholds_json.as_str())
    }
}

impl StateSnapshot {
    /// Construct and checksum a new snapshot from its constituent serialized pieces.
    pub fn new(
        timestamp_ms: u64,
        executor_json: String,
        orchestrator_json: String,
        run_state_json: String,
        gate_thresholds_json: String,
    ) -> Self {
        let checksum = compute_checksum(
            &executor_json,
            &orchestrator_json,
            &run_state_json,
            &gate_thresholds_json,
        );
        Self {
            version: STATE_SNAPSHOT_VERSION,
            timestamp_ms,
            executor_json,
            orchestrator_json,
            run_state_json,
            gate_thresholds_json,
            checksum,
        }
    }

    /// Validate the embedded checksum. Returns `Err` with a descriptive message on mismatch.
    pub fn verify(&self) -> Result<(), String> {
        if !matches!(self.version, 1 | STATE_SNAPSHOT_VERSION) {
            return Err(format!(
                "state snapshot version mismatch: file has {}, code supports 1 and {}",
                self.version, STATE_SNAPSHOT_VERSION,
            ));
        }
        let expected = if self.version == 1 {
            compute_v1_checksum(
                &self.executor_json,
                &self.orchestrator_json,
                &self.run_state_json,
                &self.gate_thresholds_json,
            )
        } else {
            compute_checksum(
                &self.executor_json,
                &self.orchestrator_json,
                &self.run_state_json,
                &self.gate_thresholds_json,
            )
        };
        if expected != self.checksum {
            return Err(format!(
                "state snapshot checksum mismatch: stored {}, computed {expected}",
                self.checksum
            ));
        }
        Ok(())
    }
}

fn compute_v1_checksum(
    executor: &str,
    orchestrator: &str,
    run_state: &str,
    gate_thresholds: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(executor.as_bytes());
    hasher.update(orchestrator.as_bytes());
    hasher.update(run_state.as_bytes());
    hasher.update(gate_thresholds.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn compute_checksum(
    executor: &str,
    orchestrator: &str,
    run_state: &str,
    gate_thresholds: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"roko-state-snapshot-v2\0");
    for (label, payload) in [
        ("executor_json", executor),
        ("orchestrator_json", orchestrator),
        ("run_state_json", run_state),
        ("gate_thresholds_json", gate_thresholds),
    ] {
        let label = label.as_bytes();
        let payload = payload.as_bytes();
        hasher.update((label.len() as u64).to_be_bytes());
        hasher.update(label);
        hasher.update((payload.len() as u64).to_be_bytes());
        hasher.update(payload);
    }
    format!("{:x}", hasher.finalize())
}

fn legacy_generation(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"roko-legacy-executor-projection-v1\0");
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn state_snapshot_generation(snapshot: &StateSnapshot) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"roko-durable-runner-generation-v1\0");
    hasher.update(snapshot.version.to_be_bytes());
    hasher.update(snapshot.timestamp_ms.to_be_bytes());
    hasher.update((snapshot.checksum.len() as u64).to_be_bytes());
    hasher.update(snapshot.checksum.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Load the authoritative durable Runner projection for a workspace.
///
/// `state-snapshot.json` is selected whenever it exists and is validated for
/// size, schema version, checksum, and all four embedded JSON payloads. The
/// legacy `executor.json` is considered only when opening the canonical file
/// returns `NotFound`. `Ok(None)` means neither file exists.
pub fn load_durable_runner_projection(
    workdir: &Path,
) -> io::Result<Option<DurableRunnerProjection>> {
    let snapshot_path = workdir.join(STATE_SNAPSHOT_RELATIVE_PATH);
    match read_bounded(&snapshot_path) {
        Ok(bytes) => parse_state_snapshot(&snapshot_path, &bytes).map(Some),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let legacy_path = workdir.join(LEGACY_EXECUTOR_RELATIVE_PATH);
            match read_bounded(&legacy_path) {
                Ok(bytes) => {
                    let raw = parse_json(&legacy_path, "legacy executor", &bytes)?;
                    let executor = normalize_legacy_executor(&legacy_path, raw)?;
                    Ok(Some(DurableRunnerProjection {
                        source: RunnerProjectionSource::LegacyExecutor,
                        source_path: legacy_path,
                        generation: legacy_generation(&bytes),
                        executor_projection: executor.clone(),
                        executor,
                        orchestrator: None,
                        run_state: None,
                        gate_thresholds: None,
                        snapshot: None,
                    }))
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
                Err(error) => Err(error),
            }
        }
        Err(error) => Err(error),
    }
}

/// Load one verified Runner projection and materialize the core dashboard from it.
///
/// StateHub and HTTP recovery use this entry point so the source-selection and
/// corruption behavior cannot drift between surfaces.
pub fn load_durable_dashboard_projection(workdir: &Path) -> io::Result<DurableDashboardProjection> {
    let runner = load_durable_runner_projection(workdir)?;
    let empty_executor = serde_json::Value::Null;
    let executor = runner.as_ref().map_or(&empty_executor, |projection| {
        &projection.executor_projection
    });
    let gate_thresholds = runner
        .as_ref()
        .and_then(DurableRunnerProjection::gate_thresholds_raw);
    let dashboard =
        roko_core::dashboard_snapshot::DashboardSnapshot::load_from_workdir_with_runner_projection(
            workdir,
            executor,
            gate_thresholds,
        )?;
    Ok(DurableDashboardProjection { dashboard, runner })
}

fn parse_state_snapshot(path: &Path, bytes: &[u8]) -> io::Result<DurableRunnerProjection> {
    let snapshot: StateSnapshot = parse_json(path, "state snapshot", bytes)?;
    validate_state_snapshot(path, snapshot)
}

/// Validate one already-decoded authoritative snapshot and return its typed,
/// generation-bound projection. Runner resume and read-only surfaces share
/// this function so semantic acceptance cannot drift.
pub fn validate_state_snapshot(
    path: &Path,
    snapshot: StateSnapshot,
) -> io::Result<DurableRunnerProjection> {
    snapshot.verify().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("verify {}: {error}", path.display()),
        )
    })?;

    let executor = parse_and_validate_embedded::<ExecutorSchema>(
        path,
        "executor_json",
        &snapshot.executor_json,
    )?;
    validate_schema_version(path, "executor_json", &executor, 1)?;
    if executor
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0)
        == 0
        && executor.get("tasks").is_some()
        && executor.get("plan_states").is_none()
    {
        return Err(invalid_embedded(
            path,
            "executor_json",
            "legacy tasks layout is not permitted inside an authoritative unified snapshot",
        ));
    }
    let orchestrator = parse_and_validate_embedded::<OrchestratorSchema>(
        path,
        "orchestrator_json",
        &snapshot.orchestrator_json,
    )?;
    validate_schema_version(path, "orchestrator_json", &orchestrator, 1)?;
    let executor_semantics: ExecutorSchema = serde_json::from_value(executor.clone())
        .map_err(|error| invalid_embedded(path, "executor_json", error))?;
    let orchestrator_semantics: OrchestratorSchema =
        serde_json::from_value(orchestrator.clone())
            .map_err(|error| invalid_embedded(path, "orchestrator_json", error))?;
    if orchestrator.get("executor") != Some(&executor) {
        return Err(invalid_embedded(
            path,
            "orchestrator_json",
            "embedded executor does not equal executor_json",
        ));
    }
    let run_state = parse_and_validate_embedded::<RunStateSchema>(
        path,
        "run_state_json",
        &snapshot.run_state_json,
    )?;
    validate_schema_version(path, "run_state_json", &run_state, 1)?;
    let run_state_semantics: RunStateSchema = serde_json::from_value(run_state.clone())
        .map_err(|error| invalid_embedded(path, "run_state_json", error))?;
    validate_run_state_semantics(path, &run_state_semantics)?;
    validate_cross_projection_semantics(path, &executor_semantics, &run_state_semantics)?;
    let gate_thresholds = parse_and_validate_embedded::<GateThresholdsSchema>(
        path,
        "gate_thresholds_json",
        &snapshot.gate_thresholds_json,
    )?;
    let executor_projection = build_executor_projection(&executor, &run_state);
    for (field, timestamp) in [
        ("executor_json", executor_semantics.timestamp_ms),
        ("orchestrator_json", orchestrator_semantics.timestamp_ms),
        ("run_state_json", run_state_semantics.timestamp_ms),
    ] {
        if timestamp != snapshot.timestamp_ms {
            return Err(invalid_embedded(
                path,
                field,
                format!(
                    "timestamp_ms {timestamp} does not equal outer timestamp_ms {}",
                    snapshot.timestamp_ms
                ),
            ));
        }
    }
    if run_state_semantics
        .lifecycle
        .as_ref()
        .is_some_and(|lifecycle| lifecycle.run_id != run_state_semantics.run_id)
    {
        return Err(invalid_embedded(
            path,
            "run_state_json",
            "lifecycle run_id does not equal run_state run_id",
        ));
    }
    let generation = state_snapshot_generation(&snapshot);

    Ok(DurableRunnerProjection {
        source: RunnerProjectionSource::StateSnapshot,
        source_path: path.to_path_buf(),
        generation,
        executor_projection,
        executor,
        orchestrator: Some(orchestrator),
        run_state: Some(run_state),
        gate_thresholds: Some(gate_thresholds),
        snapshot: Some(snapshot),
    })
}

fn normalize_legacy_executor(
    path: &Path,
    value: serde_json::Value,
) -> io::Result<serde_json::Value> {
    let object = value.as_object().ok_or_else(|| {
        invalid_embedded(path, "legacy executor", "top-level value must be an object")
    })?;
    validate_schema_version(path, "legacy executor", &value, 1)?;
    let normalized = if object.contains_key("plan_states") {
        let mut normalized = value.clone();
        if object
            .get("schema_version")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            == 0
        {
            normalize_legacy_plan_gate_results(&mut normalized);
        }
        serde_json::from_value::<ExecutorSchema>(normalized.clone())
            .map_err(|error| invalid_embedded(path, "legacy executor", error))?;
        normalized
    } else if let Some(tasks) = object.get("tasks") {
        let tasks = tasks
            .as_array()
            .ok_or_else(|| invalid_embedded(path, "legacy executor", "tasks must be an array"))?;
        let mut progress = std::collections::BTreeMap::<String, (usize, usize)>::new();
        for task in tasks {
            let task = task.as_object().ok_or_else(|| {
                invalid_embedded(path, "legacy executor", "each task must be an object")
            })?;
            let plan_id = task
                .get("plan")
                .or_else(|| task.get("plan_id"))
                .and_then(serde_json::Value::as_str)
                .filter(|plan_id| !plan_id.is_empty())
                .ok_or_else(|| {
                    invalid_embedded(path, "legacy executor", "task is missing plan identity")
                })?;
            let status = task
                .get("status")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let entry = progress.entry(plan_id.to_string()).or_default();
            entry.0 += 1;
            if matches!(
                status.to_ascii_lowercase().as_str(),
                "done" | "complete" | "completed"
            ) {
                entry.1 += 1;
            }
        }
        let plan_states = progress
            .iter()
            .map(|(plan_id, (total, done))| {
                let phase = if *total > 0 && total == done {
                    "complete"
                } else {
                    "implementing"
                };
                (
                    plan_id.clone(),
                    serde_json::json!({
                        "plan_id": plan_id,
                        "current_phase": { "kind": phase },
                        "assigned_agents": []
                    }),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        let queue_order = object.get("queue_order").cloned().unwrap_or_else(|| {
            serde_json::Value::Array(progress.keys().cloned().map(Into::into).collect())
        });
        serde_json::json!({
            "schema_version": 1,
            "plan_states": plan_states,
            "queue_order": queue_order,
            "speculative_executions": {},
            "timestamp_ms": object.get("timestamp_ms").and_then(serde_json::Value::as_u64).unwrap_or(0)
        })
    } else {
        return Err(invalid_embedded(
            path,
            "legacy executor",
            "missing plan_states or supported tasks layout",
        ));
    };
    Ok(normalized)
}

fn normalize_legacy_plan_gate_results(executor: &mut serde_json::Value) {
    let Some(plans) = executor
        .get_mut("plan_states")
        .and_then(serde_json::Value::as_object_mut)
    else {
        return;
    };
    for plan in plans.values_mut() {
        let Some(results) = plan
            .get_mut("gate_results")
            .and_then(serde_json::Value::as_array_mut)
        else {
            continue;
        };
        for result in results {
            let Some(result) = result.as_object_mut() else {
                continue;
            };
            result.entry("rung").or_insert_with(|| 0.into());
            result
                .entry("summary")
                .or_insert_with(|| String::new().into());
        }
    }
}

#[derive(Deserialize, Serialize)]
#[allow(dead_code)]
struct ExecutorSchema {
    #[serde(default)]
    schema_version: u32,
    #[serde(default)]
    plan_states: HashMap<String, ExecutorPlanSchema>,
    #[serde(default)]
    queue_order: Vec<String>,
    #[serde(default)]
    conductor_circuit_breaker: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(default)]
    speculative_executions: HashMap<String, serde_json::Map<String, serde_json::Value>>,
    #[serde(default)]
    timestamp_ms: u64,
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
#[allow(dead_code)]
struct ExecutorPlanSchema {
    plan_id: String,
    current_phase: roko_core::PlanPhase,
    assigned_agents: Vec<String>,
    gate_results: Vec<ExecutorGateResultSchema>,
    iteration: u32,
    started_at_ms: u64,
    files_changed: Vec<String>,
    merge_attempts: u32,
    last_error: Option<String>,
    paused: bool,
    priority: u32,
}

impl Default for ExecutorPlanSchema {
    fn default() -> Self {
        Self {
            plan_id: String::new(),
            current_phase: roko_core::PlanPhase::Queued,
            assigned_agents: Vec::new(),
            gate_results: Vec::new(),
            iteration: 1,
            started_at_ms: 0,
            files_changed: Vec::new(),
            merge_attempts: 0,
            last_error: None,
            paused: false,
            priority: 0,
        }
    }
}

#[derive(Deserialize, Serialize)]
#[allow(dead_code)]
struct ExecutorGateResultSchema {
    gate_name: String,
    rung: u32,
    passed: bool,
    summary: String,
    duration_ms: u64,
    #[serde(default)]
    test_count: Option<roko_core::TestCount>,
}

#[derive(Deserialize, Serialize)]
#[allow(dead_code)]
struct OrchestratorSchema {
    #[serde(default = "default_schema_version")]
    schema_version: u32,
    executor: ExecutorSchema,
    #[serde(default)]
    merge_queue: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(default)]
    worktrees: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(default)]
    event_log: Option<serde_json::Map<String, serde_json::Value>>,
    timestamp_ms: u64,
}

const fn default_schema_version() -> u32 {
    1
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct RunStateSchema {
    #[serde(default)]
    schema_version: u32,
    run_id: String,
    #[serde(default)]
    started_at_ms: u64,
    #[serde(default)]
    timestamp_ms: u64,
    tasks_total: usize,
    tasks_completed: usize,
    tasks_failed: usize,
    total_tokens_in: u64,
    total_tokens_out: u64,
    total_cost_usd: f64,
    total_agent_calls: usize,
    #[serde(default)]
    plan_costs: HashMap<String, f64>,
    #[serde(default)]
    task_usage: HashMap<String, TaskUsageSchema>,
    #[serde(default)]
    accounted_usage_attempts: Vec<String>,
    #[serde(default)]
    completed_tasks: HashMap<String, Vec<String>>,
    #[serde(default)]
    failed_tasks: HashMap<String, Vec<String>>,
    #[serde(default)]
    skipped_tasks: HashMap<String, HashMap<String, serde_json::Value>>,
    #[serde(default)]
    lifecycle: Option<RunnerLifecycleSchema>,
    #[serde(default)]
    snapshot_fail_streak: u32,
    #[serde(default)]
    fingerprints: Vec<serde_json::Map<String, serde_json::Value>>,
    #[serde(default)]
    replan_ledger: ReplanLedgerSchema,
    #[serde(default)]
    revised_tasks: Vec<serde_json::Map<String, serde_json::Value>>,
    #[serde(default)]
    cascade_router_json: Option<String>,
    #[serde(default)]
    conductor_circuit_breaker_state: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct TaskUsageSchema {
    model: String,
    provider: String,
    tokens_in: u64,
    tokens_out: u64,
    cost_usd: f64,
    budget_usd: f64,
    agent_calls: u32,
}

#[derive(Default, Deserialize)]
#[allow(dead_code)]
struct ReplanLedgerSchema {
    #[serde(default)]
    replans_seen: HashMap<String, u32>,
    #[serde(default)]
    seen_failure_keys: Vec<String>,
    #[serde(default)]
    revision_requests: Vec<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct RunnerLifecycleSchema {
    run_id: String,
    status: RunnerRunStatusSchema,
    total_tasks: usize,
    #[serde(default)]
    resumed: bool,
    #[serde(default)]
    plans: HashMap<String, PlanLifecycleStatusSchema>,
    #[serde(default)]
    tasks: HashMap<String, TaskLifecycleSchema>,
    #[serde(default)]
    task_attempts: HashMap<String, TaskAttemptLifecycleSchema>,
    #[serde(default)]
    last_resume_marker: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(default)]
    global_timeout: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(default)]
    events_seen: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
enum RunnerRunStatusSchema {
    Initialized,
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
enum PlanLifecycleStatusSchema {
    Started,
    Succeeded,
    Failed,
    Skipped,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct TaskLifecycleSchema {
    plan_id: String,
    task_id: String,
    status: TaskLifecycleStatusSchema,
    current_attempt: u32,
    next_attempt: u32,
    started_at_ms: u64,
    #[serde(default)]
    completed_at_ms: Option<u64>,
    #[serde(default)]
    latest_failure_kind: Option<RunnerFailureKindSchema>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct TaskAttemptLifecycleSchema {
    plan_id: String,
    task_id: String,
    attempt: u32,
    status: TaskAttemptStatusSchema,
    started_at_ms: u64,
    #[serde(default)]
    completed_at_ms: Option<u64>,
    #[serde(default)]
    agent_id: Option<String>,
    #[serde(default)]
    failure_kind: Option<RunnerFailureKindSchema>,
    #[serde(default)]
    retry_action: Option<RetryActionSchema>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
enum TaskLifecycleStatusSchema {
    Started,
    Running,
    Retrying,
    Passed,
    Failed,
    Exhausted,
    Cancelled,
    TimedOut,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
enum TaskAttemptStatusSchema {
    Started,
    DispatchingAgent,
    AgentRunning,
    AgentCompleted,
    Gating,
    GateFailed,
    Retrying,
    Cancelling,
    CancellationFailed,
    Passed,
    Failed,
    Exhausted,
    Cancelled,
    TimedOut,
    Superseded,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
enum RunnerFailureKindSchema {
    Transient,
    Permanent,
    Resource,
    Structural,
    Unknown,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
enum RetryActionSchema {
    RetryAfterBackoff,
    Exhausted,
    NotRetryable,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct GateThresholdsSchema {
    #[serde(default)]
    rungs: HashMap<u32, GateThresholdStatsSchema>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct GateThresholdStatsSchema {
    #[serde(default)]
    pass_count: u64,
    #[serde(default, alias = "total_observations")]
    total_count: u64,
    #[serde(default = "default_ema_pass_rate")]
    ema_pass_rate: f64,
}

const fn default_ema_pass_rate() -> f64 {
    0.5
}

fn parse_and_validate_embedded<T: serde::de::DeserializeOwned>(
    path: &Path,
    field: &str,
    encoded: &str,
) -> io::Result<serde_json::Value> {
    let value = parse_embedded_json(path, field, encoded)?;
    serde_json::from_value::<T>(value.clone())
        .map_err(|error| invalid_embedded(path, field, error))?;
    Ok(value)
}

fn invalid_embedded(path: &Path, field: &str, error: impl fmt::Display) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("validate {field} embedded in {}: {error}", path.display()),
    )
}

fn validate_schema_version(
    path: &Path,
    field: &str,
    value: &serde_json::Value,
    current: u64,
) -> io::Result<()> {
    let version = value
        .get("schema_version")
        .map_or(Some(0), serde_json::Value::as_u64)
        .ok_or_else(|| invalid_embedded(path, field, "schema_version must be unsigned"))?;
    if version > current {
        return Err(invalid_embedded(
            path,
            field,
            format!("schema version {version} is newer than supported version {current}"),
        ));
    }
    Ok(())
}

fn validate_run_state_semantics(path: &Path, run_state: &RunStateSchema) -> io::Result<()> {
    let Some(lifecycle) = run_state.lifecycle.as_ref() else {
        return validate_terminal_maps(path, run_state, None);
    };
    if lifecycle.run_id != run_state.run_id {
        return Err(invalid_embedded(
            path,
            "run_state_json",
            "lifecycle run_id does not equal run_state run_id",
        ));
    }
    if lifecycle.total_tasks != run_state.tasks_total {
        return Err(invalid_embedded(
            path,
            "run_state_json",
            format!(
                "lifecycle total_tasks {} does not equal run_state tasks_total {}",
                lifecycle.total_tasks, run_state.tasks_total
            ),
        ));
    }

    let mut identities = std::collections::HashSet::new();
    for (key, task) in &lifecycle.tasks {
        let expected = format!("{}:{}", task.plan_id, task.task_id);
        if key != &expected || !identities.insert(expected.clone()) {
            return Err(invalid_embedded(
                path,
                "run_state_json",
                format!("lifecycle task key {key:?} does not uniquely bind body {expected:?}"),
            ));
        }
        if !lifecycle.plans.is_empty() && !lifecycle.plans.contains_key(&task.plan_id) {
            return Err(invalid_embedded(
                path,
                "run_state_json",
                format!(
                    "lifecycle task {key:?} refers to missing plan {:?}",
                    task.plan_id
                ),
            ));
        }
    }

    let mut attempt_identities = std::collections::HashSet::new();
    for (key, attempt) in &lifecycle.task_attempts {
        let expected = format!(
            "{}:{}:{}",
            attempt.plan_id, attempt.task_id, attempt.attempt
        );
        if key != &expected || !attempt_identities.insert(expected.clone()) {
            return Err(invalid_embedded(
                path,
                "run_state_json",
                format!("lifecycle attempt key {key:?} does not uniquely bind body {expected:?}"),
            ));
        }
        let task_key = format!("{}:{}", attempt.plan_id, attempt.task_id);
        if !lifecycle.tasks.contains_key(&task_key) {
            return Err(invalid_embedded(
                path,
                "run_state_json",
                format!("lifecycle attempt {key:?} refers to missing task {task_key:?}"),
            ));
        }
    }
    if lifecycle.tasks.len() > lifecycle.total_tasks {
        return Err(invalid_embedded(
            path,
            "run_state_json",
            "lifecycle contains more tasks than total_tasks",
        ));
    }
    validate_terminal_maps(path, run_state, Some(lifecycle))
}

fn validate_terminal_maps(
    path: &Path,
    run_state: &RunStateSchema,
    lifecycle: Option<&RunnerLifecycleSchema>,
) -> io::Result<()> {
    let mut terminal = std::collections::HashMap::<String, &'static str>::new();
    for (plan_id, task_ids, label) in run_state
        .completed_tasks
        .iter()
        .map(|(plan, tasks)| (plan, tasks, "completed"))
        .chain(
            run_state
                .failed_tasks
                .iter()
                .map(|(plan, tasks)| (plan, tasks, "failed")),
        )
    {
        for task_id in task_ids {
            let identity = format!("{plan_id}:{task_id}");
            if let Some(previous) = terminal.insert(identity.clone(), label) {
                return Err(invalid_embedded(
                    path,
                    "run_state_json",
                    format!("task {identity:?} appears in both {previous} and {label} maps"),
                ));
            }
        }
    }
    for (plan_id, tasks) in &run_state.skipped_tasks {
        for task_id in tasks.keys() {
            let identity = format!("{plan_id}:{task_id}");
            if let Some(previous) = terminal.insert(identity.clone(), "skipped") {
                return Err(invalid_embedded(
                    path,
                    "run_state_json",
                    format!("task {identity:?} appears in both {previous} and skipped maps"),
                ));
            }
        }
    }
    if run_state
        .tasks_completed
        .saturating_add(run_state.tasks_failed)
        > run_state.tasks_total
        || terminal.len() > run_state.tasks_total
    {
        return Err(invalid_embedded(
            path,
            "run_state_json",
            "task aggregate or durable terminal-map counts exceed tasks_total",
        ));
    }
    if let Some(lifecycle) = lifecycle {
        for (identity, label) in terminal {
            let Some(task) = lifecycle.tasks.get(&identity) else {
                // Older writers did not create lifecycle rows for tasks that
                // were skipped before dispatch. The terminal map remains the
                // authoritative compatible record for those identities.
                continue;
            };
            let status_matches = match label {
                "completed" => matches!(task.status, TaskLifecycleStatusSchema::Passed),
                "failed" => matches!(
                    task.status,
                    TaskLifecycleStatusSchema::Failed
                        | TaskLifecycleStatusSchema::Exhausted
                        | TaskLifecycleStatusSchema::TimedOut
                ),
                "skipped" => matches!(
                    task.status,
                    TaskLifecycleStatusSchema::Cancelled | TaskLifecycleStatusSchema::TimedOut
                ),
                _ => false,
            };
            if !status_matches {
                return Err(invalid_embedded(
                    path,
                    "run_state_json",
                    format!("{label} task {identity:?} conflicts with lifecycle status"),
                ));
            }
        }
    }
    Ok(())
}

fn validate_cross_projection_semantics(
    path: &Path,
    executor: &ExecutorSchema,
    run_state: &RunStateSchema,
) -> io::Result<()> {
    for (plan_id, plan) in &executor.plan_states {
        if !plan.plan_id.is_empty() && plan.plan_id != *plan_id {
            return Err(invalid_embedded(
                path,
                "executor_json",
                format!(
                    "plan_states key {plan_id:?} does not equal body plan_id {:?}",
                    plan.plan_id
                ),
            ));
        }
    }
    for plan_id in run_state
        .completed_tasks
        .keys()
        .chain(run_state.failed_tasks.keys())
        .chain(run_state.skipped_tasks.keys())
    {
        if !executor.plan_states.contains_key(plan_id) {
            return Err(invalid_embedded(
                path,
                "run_state_json",
                format!("terminal task map refers to missing executor plan {plan_id:?}"),
            ));
        }
    }
    if let Some(lifecycle) = run_state.lifecycle.as_ref() {
        for plan_id in lifecycle
            .plans
            .keys()
            .chain(lifecycle.tasks.values().map(|task| &task.plan_id))
        {
            if !executor.plan_states.contains_key(plan_id) {
                return Err(invalid_embedded(
                    path,
                    "run_state_json",
                    format!("lifecycle plan {plan_id:?} is absent from executor plan_states"),
                ));
            }
        }
    }
    Ok(())
}

fn build_executor_projection(
    executor: &serde_json::Value,
    run_state: &serde_json::Value,
) -> serde_json::Value {
    let mut projection = executor.clone();
    if let Some(object) = projection.as_object_mut() {
        object.insert(
            "_runner_projection".to_string(),
            serde_json::json!({
                "completed_tasks": run_state.get("completed_tasks").cloned().unwrap_or_default(),
                "failed_tasks": run_state.get("failed_tasks").cloned().unwrap_or_default(),
                "skipped_tasks": run_state.get("skipped_tasks").cloned().unwrap_or_default(),
                "lifecycle": run_state.get("lifecycle").cloned().unwrap_or_default(),
            }),
        );
    }
    projection
}

fn parse_json<T: serde::de::DeserializeOwned>(
    path: &Path,
    description: &str,
    bytes: &[u8],
) -> io::Result<T> {
    serde_json::from_slice(bytes).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("parse {description} {}: {error}", path.display()),
        )
    })
}

fn parse_embedded_json(path: &Path, field: &str, encoded: &str) -> io::Result<serde_json::Value> {
    serde_json::from_str(encoded).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("parse {field} embedded in {}: {error}", path.display()),
        )
    })
}

fn read_bounded(path: &Path) -> io::Result<Vec<u8>> {
    let file = File::open(path)?;
    let metadata_len = file.metadata()?.len();
    if metadata_len > MAX_DURABLE_RUNNER_PROJECTION_BYTES {
        return Err(projection_too_large(path, metadata_len));
    }

    // The bounded reader is authoritative even if the file changes after the
    // metadata check. Atomic rename writers yield either the old or new open
    // file generation; in-place growth cannot exceed this allocation bound.
    let mut bytes = Vec::with_capacity(metadata_len as usize);
    file.take(MAX_DURABLE_RUNNER_PROJECTION_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_DURABLE_RUNNER_PROJECTION_BYTES {
        return Err(projection_too_large(path, bytes.len() as u64));
    }
    Ok(bytes)
}

pub(crate) fn read_bounded_text(path: &Path) -> io::Result<String> {
    String::from_utf8(read_bounded(path)?).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} is not UTF-8: {error}", path.display()),
        )
    })
}

fn projection_too_large(path: &Path, bytes: u64) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "durable Runner projection {} is {bytes} bytes; maximum is {}",
            path.display(),
            MAX_DURABLE_RUNNER_PROJECTION_BYTES
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_snapshot(workdir: &Path, snapshot: &StateSnapshot) {
        let path = workdir.join(STATE_SNAPSHOT_RELATIVE_PATH);
        fs::create_dir_all(path.parent().expect("snapshot parent")).unwrap();
        fs::write(path, serde_json::to_vec(snapshot).unwrap()).unwrap();
    }

    fn fixture_snapshot(plan_id: &str) -> StateSnapshot {
        let executor = serde_json::json!({
            "schema_version": 1,
            "plan_states": {
                (plan_id): {
                    "plan_id": plan_id,
                    "current_phase": { "kind": "implementing" },
                    "assigned_agents": []
                }
            },
            "queue_order": [plan_id],
            "speculative_executions": {},
            "timestamp_ms": 42
        });
        StateSnapshot::new(
            42,
            executor.to_string(),
            serde_json::json!({
                "schema_version": 1,
                "executor": executor,
                "timestamp_ms": 42
            })
            .to_string(),
            serde_json::json!({
                "schema_version": 1,
                "run_id": format!("run-{plan_id}"),
                "timestamp_ms": 42,
                "tasks_total": 1,
                "tasks_completed": 0,
                "tasks_failed": 0,
                "total_tokens_in": 0,
                "total_tokens_out": 0,
                "total_cost_usd": 0.0,
                "total_agent_calls": 1,
                "lifecycle": {
                    "run_id": format!("run-{plan_id}"),
                    "status": "running",
                    "total_tasks": 1,
                    "tasks": {
                        (format!("{plan_id}:task-1")): {
                            "plan_id": plan_id,
                            "task_id": "task-1",
                            "status": "running",
                            "current_attempt": 1,
                            "next_attempt": 2,
                            "started_at_ms": 42
                        }
                    },
                    "task_attempts": {
                        (format!("{plan_id}:task-1:1")): {
                            "plan_id": plan_id,
                            "task_id": "task-1",
                            "attempt": 1,
                            "status": "agent_running",
                            "started_at_ms": 42,
                            "agent_id": "agent-1"
                        }
                    },
                    "events_seen": 2
                },
                "replan_ledger": {}
            })
            .to_string(),
            serde_json::json!({"rungs": {}}).to_string(),
        )
    }

    #[test]
    fn new_snapshot_verifies_cleanly() {
        let snap = StateSnapshot::new(
            1_000_000,
            r#"{"tasks":[]}"#.to_string(),
            r#"{"merge_queue":[]}"#.to_string(),
            r#"{"run_id":"run-1"}"#.to_string(),
            r#"{"rungs":{}}"#.to_string(),
        );
        assert_eq!(snap.version, STATE_SNAPSHOT_VERSION);
        assert!(snap.verify().is_ok());
    }

    #[test]
    fn corrupted_checksum_fails_verification() {
        let mut snap = StateSnapshot::new(
            1_000_000,
            r#"{"tasks":[]}"#.to_string(),
            r#"{"merge_queue":[]}"#.to_string(),
            r#"{"run_id":"run-1"}"#.to_string(),
            r#"{"rungs":{}}"#.to_string(),
        );
        snap.checksum =
            "0000000000000000000000000000000000000000000000000000000000000000".to_string();
        let err = snap.verify().unwrap_err();
        assert!(err.contains("checksum mismatch"), "got: {err}");
    }

    #[test]
    fn mutated_payload_fails_verification() {
        let mut snap = StateSnapshot::new(
            1_000_000,
            r#"{"tasks":[]}"#.to_string(),
            r#"{"merge_queue":[]}"#.to_string(),
            r#"{"run_id":"run-1"}"#.to_string(),
            r#"{"rungs":{}}"#.to_string(),
        );
        // Mutate one of the inner payloads after construction.
        snap.executor_json = r#"{"tasks":["tampered"]}"#.to_string();
        let err = snap.verify().unwrap_err();
        assert!(err.contains("checksum mismatch"), "got: {err}");
    }

    #[test]
    fn wrong_version_fails_verification() {
        let mut snap = StateSnapshot::new(
            1_000_000,
            r#"{"tasks":[]}"#.to_string(),
            r#"{"merge_queue":[]}"#.to_string(),
            r#"{"run_id":"run-1"}"#.to_string(),
            r#"{"rungs":{}}"#.to_string(),
        );
        snap.version = STATE_SNAPSHOT_VERSION + 1;
        let err = snap.verify().unwrap_err();
        assert!(err.contains("version mismatch"), "got: {err}");
    }

    #[test]
    fn checksum_is_64_char_hex() {
        let snap = StateSnapshot::new(
            1_000_000,
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
        );
        assert_eq!(snap.checksum.len(), 64);
        assert!(snap.checksum.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn v2_checksum_partitions_fields_that_collided_in_v1() {
        let left = StateSnapshot::new(1, "ab".into(), "c".into(), "".into(), "".into());
        let right = StateSnapshot::new(1, "a".into(), "bc".into(), "".into(), "".into());
        assert_eq!(
            compute_v1_checksum("ab", "c", "", ""),
            compute_v1_checksum("a", "bc", "", "")
        );
        assert_ne!(left.checksum, right.checksum);
    }

    #[test]
    fn v1_snapshot_remains_readable_and_gets_v2_generation_identity() {
        let dir = tempfile::tempdir().unwrap();
        let mut snapshot = fixture_snapshot("v1");
        snapshot.version = 1;
        snapshot.checksum = compute_v1_checksum(
            &snapshot.executor_json,
            &snapshot.orchestrator_json,
            &snapshot.run_state_json,
            &snapshot.gate_thresholds_json,
        );
        write_snapshot(dir.path(), &snapshot);
        let loaded = load_durable_runner_projection(dir.path())
            .unwrap()
            .expect("v1 projection");
        assert_eq!(loaded.snapshot.as_ref().unwrap().version, 1);
        assert_ne!(loaded.generation, snapshot.checksum);
        assert_eq!(
            StateSnapshot::new(1, "a".into(), "b".into(), "c".into(), "d".into()).version,
            2
        );
    }

    #[test]
    fn generation_binds_outer_version_and_timestamp() {
        let snapshot = fixture_snapshot("generation");
        let first = state_snapshot_generation(&snapshot);
        let mut changed = snapshot.clone();
        changed.timestamp_ms += 1;
        assert_ne!(first, state_snapshot_generation(&changed));
        changed = snapshot.clone();
        changed.version = 1;
        assert_ne!(first, state_snapshot_generation(&changed));
    }

    #[test]
    fn embedded_timestamp_mismatch_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        let mut snapshot = fixture_snapshot("timestamp");
        snapshot.timestamp_ms += 1;
        snapshot.checksum = compute_checksum(
            &snapshot.executor_json,
            &snapshot.orchestrator_json,
            &snapshot.run_state_json,
            &snapshot.gate_thresholds_json,
        );
        write_snapshot(dir.path(), &snapshot);
        assert!(
            load_durable_runner_projection(dir.path())
                .unwrap_err()
                .to_string()
                .contains("does not equal outer timestamp_ms")
        );
    }

    #[test]
    fn embedded_semantic_schema_is_strict_but_v1_outer_extensions_remain_compatible() {
        let dir = tempfile::tempdir().unwrap();
        let mut snapshot = fixture_snapshot("schema");
        snapshot.run_state_json =
            serde_json::json!({"run_id": "missing-required-fields"}).to_string();
        snapshot.checksum = compute_checksum(
            &snapshot.executor_json,
            &snapshot.orchestrator_json,
            &snapshot.run_state_json,
            &snapshot.gate_thresholds_json,
        );
        write_snapshot(dir.path(), &snapshot);
        assert!(
            load_durable_runner_projection(dir.path())
                .unwrap_err()
                .to_string()
                .contains("run_state_json")
        );

        let mut outer = serde_json::to_value(fixture_snapshot("outer")).unwrap();
        outer["unexpected"] = serde_json::json!(true);
        let path = dir.path().join(STATE_SNAPSHOT_RELATIVE_PATH);
        fs::write(&path, serde_json::to_vec(&outer).unwrap()).unwrap();
        assert!(load_durable_runner_projection(dir.path()).is_ok());
    }

    #[test]
    fn raw_executor_mismatch_future_schema_and_legacy_tasks_fail_closed() {
        let dir = tempfile::tempdir().unwrap();
        let mut snapshot = fixture_snapshot("raw-mismatch");
        let mut executor: serde_json::Value =
            serde_json::from_str(&snapshot.executor_json).unwrap();
        executor["writer_extension"] = serde_json::json!({"generation": 7});
        snapshot.executor_json = executor.to_string();
        snapshot.checksum = compute_checksum(
            &snapshot.executor_json,
            &snapshot.orchestrator_json,
            &snapshot.run_state_json,
            &snapshot.gate_thresholds_json,
        );
        write_snapshot(dir.path(), &snapshot);
        assert!(
            load_durable_runner_projection(dir.path())
                .unwrap_err()
                .to_string()
                .contains("does not equal")
        );

        let mut snapshot = fixture_snapshot("future-schema");
        let mut executor: serde_json::Value =
            serde_json::from_str(&snapshot.executor_json).unwrap();
        executor["schema_version"] = serde_json::json!(2);
        let mut orchestrator: serde_json::Value =
            serde_json::from_str(&snapshot.orchestrator_json).unwrap();
        orchestrator["executor"] = executor.clone();
        snapshot.executor_json = executor.to_string();
        snapshot.orchestrator_json = orchestrator.to_string();
        snapshot.checksum = compute_checksum(
            &snapshot.executor_json,
            &snapshot.orchestrator_json,
            &snapshot.run_state_json,
            &snapshot.gate_thresholds_json,
        );
        write_snapshot(dir.path(), &snapshot);
        assert!(
            load_durable_runner_projection(dir.path())
                .unwrap_err()
                .to_string()
                .contains("newer than supported")
        );

        let mut snapshot = fixture_snapshot("legacy-tasks");
        let executor = serde_json::json!({
            "tasks": [{"id": "T1", "plan": "legacy-tasks", "status": "done"}],
            "timestamp_ms": 42
        });
        let mut orchestrator: serde_json::Value =
            serde_json::from_str(&snapshot.orchestrator_json).unwrap();
        orchestrator["executor"] = executor.clone();
        snapshot.executor_json = executor.to_string();
        snapshot.orchestrator_json = orchestrator.to_string();
        snapshot.checksum = compute_checksum(
            &snapshot.executor_json,
            &snapshot.orchestrator_json,
            &snapshot.run_state_json,
            &snapshot.gate_thresholds_json,
        );
        write_snapshot(dir.path(), &snapshot);
        assert!(
            load_durable_runner_projection(dir.path())
                .unwrap_err()
                .to_string()
                .contains("legacy tasks layout")
        );
    }

    #[test]
    fn lifecycle_identity_conflicts_reject_but_writer_timeout_is_valid() {
        let dir = tempfile::tempdir().unwrap();
        let mut snapshot = fixture_snapshot("identity");
        let mut run_state: serde_json::Value =
            serde_json::from_str(&snapshot.run_state_json).unwrap();
        let task = run_state["lifecycle"]["tasks"]
            .as_object_mut()
            .unwrap()
            .remove("identity:task-1")
            .unwrap();
        run_state["lifecycle"]["tasks"]["wrong:key"] = task;
        snapshot.run_state_json = run_state.to_string();
        snapshot.checksum = compute_checksum(
            &snapshot.executor_json,
            &snapshot.orchestrator_json,
            &snapshot.run_state_json,
            &snapshot.gate_thresholds_json,
        );
        write_snapshot(dir.path(), &snapshot);
        assert!(
            load_durable_runner_projection(dir.path())
                .unwrap_err()
                .to_string()
                .contains("does not uniquely bind")
        );

        let mut snapshot = fixture_snapshot("timeout");
        let mut run_state: serde_json::Value =
            serde_json::from_str(&snapshot.run_state_json).unwrap();
        run_state["tasks_failed"] = serde_json::json!(1);
        run_state["failed_tasks"] = serde_json::json!({"timeout": ["task-1"]});
        run_state["lifecycle"]["tasks"]["timeout:task-1"]["status"] =
            serde_json::json!("timed_out");
        run_state["lifecycle"]["task_attempts"]["timeout:task-1:1"]["status"] =
            serde_json::json!("timed_out");
        snapshot.run_state_json = run_state.to_string();
        snapshot.checksum = compute_checksum(
            &snapshot.executor_json,
            &snapshot.orchestrator_json,
            &snapshot.run_state_json,
            &snapshot.gate_thresholds_json,
        );
        write_snapshot(dir.path(), &snapshot);
        assert!(load_durable_runner_projection(dir.path()).is_ok());
    }

    #[test]
    fn legacy_executor_compatibility_is_bounded_typed_and_normalized() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LEGACY_EXECUTOR_RELATIVE_PATH);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            serde_json::json!({
                "tasks": [
                    {"id": "T1", "plan": "legacy", "status": "done"},
                    {"id": "T2", "plan": "legacy", "status": "running"}
                ],
                "timestamp_ms": 9
            })
            .to_string(),
        )
        .unwrap();
        let projection = load_durable_runner_projection(dir.path())
            .unwrap()
            .expect("legacy projection");
        assert_eq!(
            projection.executor["plan_states"]["legacy"]["current_phase"]["kind"],
            "implementing"
        );
        assert!(projection.executor.get("tasks").is_none());

        fs::write(&path, "[]").unwrap();
        assert_eq!(
            load_durable_runner_projection(dir.path())
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        fs::write(&path, r#"{"schema_version":2,"plan_states":{}}"#).unwrap();
        assert!(
            load_durable_runner_projection(dir.path())
                .unwrap_err()
                .to_string()
                .contains("newer than supported")
        );
    }

    #[test]
    fn malformed_auxiliary_sources_do_not_poison_valid_runner_projection() {
        let dir = tempfile::tempdir().unwrap();
        write_snapshot(dir.path(), &fixture_snapshot("canonical"));
        fs::write(dir.path().join(".roko/engrams.jsonl"), "{malformed\n").unwrap();
        fs::create_dir_all(dir.path().join(".roko/learn")).unwrap();
        fs::write(dir.path().join(".roko/learn/experiments.json"), "not-json").unwrap();

        let projection = load_durable_dashboard_projection(dir.path()).unwrap();
        assert_eq!(
            projection.runner.as_ref().map(|runner| runner.source),
            Some(RunnerProjectionSource::StateSnapshot)
        );
        assert!(projection.dashboard.plans.contains_key("canonical"));
    }

    #[test]
    fn round_trip_through_json() {
        let snap = StateSnapshot::new(
            42,
            r#"{"x":1}"#.to_string(),
            r#"{"y":2}"#.to_string(),
            r#"{"z":3}"#.to_string(),
            r#"{"w":4}"#.to_string(),
        );
        let json = serde_json::to_string_pretty(&snap).unwrap();
        let loaded: StateSnapshot = serde_json::from_str(&json).unwrap();
        assert!(loaded.verify().is_ok());
        assert_eq!(loaded.checksum, snap.checksum);
        assert_eq!(loaded.timestamp_ms, 42);
    }

    #[test]
    fn canonical_snapshot_wins_over_conflicting_legacy_executor() {
        let dir = tempfile::tempdir().unwrap();
        write_snapshot(dir.path(), &fixture_snapshot("canonical"));
        let legacy_path = dir.path().join(LEGACY_EXECUTOR_RELATIVE_PATH);
        fs::write(
            legacy_path,
            serde_json::json!({"plan_states": {"stale": {}}}).to_string(),
        )
        .unwrap();

        let projection = load_durable_runner_projection(dir.path())
            .unwrap()
            .expect("projection");
        assert_eq!(projection.source, RunnerProjectionSource::StateSnapshot);
        assert!(
            projection.executor["plan_states"]
                .get("canonical")
                .is_some()
        );
        assert!(projection.executor["plan_states"].get("stale").is_none());
    }

    #[test]
    fn corrupt_authoritative_snapshot_never_falls_back_to_legacy() {
        let dir = tempfile::tempdir().unwrap();
        let mut snapshot = fixture_snapshot("canonical");
        snapshot.checksum = "0".repeat(64);
        write_snapshot(dir.path(), &snapshot);
        fs::write(
            dir.path().join(LEGACY_EXECUTOR_RELATIVE_PATH),
            serde_json::json!({"plan_states": {"legacy": {}}}).to_string(),
        )
        .unwrap();

        let error = load_durable_runner_projection(dir.path()).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("checksum mismatch"));
    }

    #[test]
    fn malformed_embedded_json_is_rejected_after_checksum_verification() {
        let dir = tempfile::tempdir().unwrap();
        let snapshot = StateSnapshot::new(
            42,
            "{not-json".to_string(),
            "{}".to_string(),
            "{}".to_string(),
            "{}".to_string(),
        );
        write_snapshot(dir.path(), &snapshot);

        let error = load_durable_runner_projection(dir.path()).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("executor_json"));
    }

    #[test]
    fn legacy_executor_is_used_only_when_snapshot_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let legacy_path = dir.path().join(LEGACY_EXECUTOR_RELATIVE_PATH);
        fs::create_dir_all(legacy_path.parent().unwrap()).unwrap();
        fs::write(
            &legacy_path,
            serde_json::json!({"plan_states": {"legacy": {}}}).to_string(),
        )
        .unwrap();

        let projection = load_durable_runner_projection(dir.path())
            .unwrap()
            .expect("legacy projection");
        assert_eq!(projection.source, RunnerProjectionSource::LegacyExecutor);
        assert_eq!(projection.source_path, legacy_path);
        assert!(projection.executor["plan_states"].get("legacy").is_some());
    }

    #[test]
    fn restart_reload_is_identical() {
        let dir = tempfile::tempdir().unwrap();
        write_snapshot(dir.path(), &fixture_snapshot("restart"));

        let first = load_durable_runner_projection(dir.path()).unwrap();
        let second = load_durable_runner_projection(dir.path()).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn atomic_replacement_reads_complete_generations() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(STATE_SNAPSHOT_RELATIVE_PATH);
        write_snapshot(dir.path(), &fixture_snapshot("old"));

        for generation in 0..32 {
            let plan_id = if generation % 2 == 0 { "new" } else { "old" };
            let replacement = path.with_extension(format!("replacement-{generation}"));
            fs::write(
                &replacement,
                serde_json::to_vec(&fixture_snapshot(plan_id)).unwrap(),
            )
            .unwrap();
            fs::rename(&replacement, &path).unwrap();

            let projection = load_durable_runner_projection(dir.path())
                .unwrap()
                .expect("projection after atomic replace");
            let plan_states = projection.executor["plan_states"].as_object().unwrap();
            assert_eq!(plan_states.len(), 1);
            assert!(plan_states.contains_key(plan_id));
        }
    }

    #[test]
    fn projection_size_is_bounded() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(STATE_SNAPSHOT_RELATIVE_PATH);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            path,
            vec![b' '; MAX_DURABLE_RUNNER_PROJECTION_BYTES as usize + 1],
        )
        .unwrap();

        let error = load_durable_runner_projection(dir.path()).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("maximum"));
    }
}
