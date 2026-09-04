//! Atomic persistence for executor snapshots, episodes, and agent PIDs.
//!
//! All writes use write-to-tmp-then-rename for crash safety.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use crate::orchestrator::{ExecutorSnapshot, OrchestratorSnapshot, PlanRevisionRequest};
use anyhow::{Context, Result};
use roko_core::defaults::{
    DEFAULT_GATE_RETRY_COLD_START, DEFAULT_GATE_RETRY_MAX, DEFAULT_GATE_RETRY_MIN,
    DEFAULT_GATE_RETRY_MIN_OBSERVATIONS,
};
use roko_fs::RokoLayout;
use roko_runtime::StateSnapshot;
use serde::{Deserialize, Serialize};

use crate::task_parser::TaskDef;

use super::types::{RunnerEvent, RunnerLifecycleProjection};

/// Schema version for the runner-owned `run-state.json` snapshot.
///
/// Bump only when the on-disk shape of [`RunStateSnapshot`] changes in a way
/// that requires migration on resume.
pub const RUN_STATE_SCHEMA_VERSION: u32 = 1;

/// Paths for all persistent state files.
#[derive(Debug, Clone)]
pub struct PersistPaths {
    /// `.roko/state/executor.json` — executor snapshot.
    pub executor_json: PathBuf,
    /// `.roko/state/orchestrator.json` — aggregate orchestrator snapshot.
    pub orchestrator_json: PathBuf,
    /// `.roko/state/run-state.json` — runner-owned cost/token/completed-task snapshot.
    pub run_state_json: PathBuf,
    /// `.roko/episodes.jsonl` — episode log.
    pub episodes_jsonl: PathBuf,
    /// `.roko/learn/efficiency.jsonl` — efficiency events.
    pub efficiency_jsonl: PathBuf,
    /// `.roko/learn/cascade-router.json` — cascade router learning state.
    pub cascade_router_json: PathBuf,
    /// `.roko/learn/gate-thresholds.json` — adaptive gate thresholds.
    pub gate_thresholds_json: PathBuf,
    /// `.roko/state/state-snapshot.json` — unified, checksummed state snapshot.
    pub state_snapshot_json: PathBuf,
    /// `.roko/runtime/agent-pids.json` — live agent PIDs.
    pub agent_pids_json: PathBuf,
    /// `.roko/state/events.json` — event log for replay.
    pub events_json: PathBuf,
    /// `.roko/events.jsonl` — append-only runner event log consumed by TUI/server.
    pub events_jsonl: PathBuf,
    /// `.roko/state/run-ledger.jsonl` — typed run ledger (task starts, completions, gate outcomes).
    pub run_ledger_jsonl: PathBuf,
    /// `.roko/state/status.json` — lightweight runner status for fast polling.
    pub status_json: PathBuf,
}

impl PersistPaths {
    /// Derive all paths from a workdir, creating parent directories as needed.
    pub fn from_workdir(workdir: &Path) -> Result<Self> {
        let layout = RokoLayout::for_project(workdir);
        let state = layout.state_dir();
        let learn = layout.learn_dir();
        let runtime = layout.runtime_dir();

        for dir in [&state, &learn, &runtime] {
            fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }

        Ok(Self {
            executor_json: layout.executor_snapshot(),
            orchestrator_json: layout.orchestrator_snapshot(),
            run_state_json: layout.run_state_path(),
            state_snapshot_json: state.join("state-snapshot.json"),
            episodes_jsonl: layout.root_episodes_path(),
            efficiency_jsonl: layout.efficiency_path(),
            cascade_router_json: layout.cascade_router_path(),
            gate_thresholds_json: layout.gate_thresholds_path(),
            agent_pids_json: layout.agent_pids_path(),
            events_json: layout.event_log_snapshot(),
            events_jsonl: layout.events_jsonl_path(),
            run_ledger_jsonl: layout.run_ledger_path(),
            status_json: state.join("status.json"),
        })
    }
}

/// Runner-owned snapshot persisted alongside `executor.json`.
///
/// Captures the cost, token, and completed-task state the orchestrator-level
/// `ExecutorSnapshot` does not retain. This is the structure written to
/// `.roko/state/run-state.json` and consumed by [`super::resume`] when
/// validating a resume.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunStateSnapshot {
    /// On-disk schema version. See [`RUN_STATE_SCHEMA_VERSION`].
    #[serde(default)]
    pub schema_version: u32,
    /// Stable identifier for the runner invocation that wrote this snapshot.
    pub run_id: String,
    /// UTC ms when the run started.
    #[serde(default)]
    pub started_at_ms: u64,
    /// UTC ms when the snapshot was written.
    #[serde(default)]
    pub timestamp_ms: u64,
    /// Total tasks across all plans known at snapshot time.
    pub tasks_total: usize,
    /// Number of tasks completed.
    pub tasks_completed: usize,
    /// Number of tasks that failed.
    pub tasks_failed: usize,
    /// Total input tokens across the run.
    pub total_tokens_in: u64,
    /// Total output tokens across the run.
    pub total_tokens_out: u64,
    /// Total cost in USD across the run.
    pub total_cost_usd: f64,
    /// Total agent spawn count.
    pub total_agent_calls: usize,
    /// Per-plan cost accumulation.
    #[serde(default)]
    pub plan_costs: HashMap<String, f64>,
    /// Per-task usage accumulation, including failed/retried agent attempts.
    #[serde(default)]
    pub task_usage: HashMap<String, super::state::TaskUsage>,
    /// Exact attempts already attributed to `task_usage`, preventing replay duplication.
    #[serde(default)]
    pub accounted_usage_attempts: Vec<String>,
    /// Completed task IDs per plan — the durable record used to skip
    /// already-finished work on resume.
    #[serde(default)]
    pub completed_tasks: HashMap<String, Vec<String>>,
    /// Failed task IDs per plan — durable record used to reconstruct DAG
    /// blocked/skipped state on resume (SH03-T01).
    #[serde(default)]
    pub failed_tasks: HashMap<String, Vec<String>>,
    /// Explicit skipped task outcomes that are not derivable from failed-task
    /// dependency propagation.
    #[serde(default)]
    pub skipped_tasks: HashMap<String, HashMap<String, super::task_dag::SkippedReason>>,
    /// Durable lifecycle projection, including in-flight cancellation state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<RunnerLifecycleProjection>,
    /// Consecutive snapshot save failures (degradation tracking).
    #[serde(default)]
    pub snapshot_fail_streak: u32,
    /// Forensic fingerprints of every task definition known when this
    /// snapshot was written. Read by [`super::resume::prepare_resume`]
    /// to detect drift between runs.
    #[serde(default)]
    pub fingerprints: Vec<TaskDefFingerprint>,
    /// Durable gate-failure replan ledger. This prevents duplicate revision
    /// requests and preserves the configured per-plan cap across runner
    /// restarts.
    #[serde(default)]
    pub replan_ledger: ReplanLedgerSnapshot,
    /// Task definitions revised by gate-failure replan requests. The runner
    /// reapplies these to the in-memory task index on resume so the retry is
    /// driven by task data, not only an appended prompt paragraph.
    #[serde(default)]
    pub revised_tasks: Vec<TaskRevision>,
    /// CascadeRouter snapshot JSON captured at save time.
    ///
    /// `None` for old snapshots or when no router is configured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cascade_router_json: Option<String>,
    /// Adaptive gate-threshold EMA state captured at save time.
    ///
    /// Embedding inside `RunStateSnapshot` (in addition to the top-level
    /// `StateSnapshot.gate_thresholds_json`) provides redundancy for the
    /// legacy `run-state.json` fallback path where the outer checksummed
    /// snapshot is not available.
    ///
    /// `None` for old snapshots or when thresholds are at their defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_thresholds_json: Option<String>,
    /// Conductor circuit-breaker state captured at save time.
    ///
    /// On `--resume`, this is restored via
    /// [`roko_conductor::Conductor::from_circuit_breaker_state`] so
    /// tripped failure budgets survive process restarts.
    /// `None` for old snapshots or when no conductor is configured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conductor_circuit_breaker_state: Option<roko_conductor::CircuitBreakerState>,
}

/// Durable gate-failure replan ledger embedded in [`RunStateSnapshot`].
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplanLedgerSnapshot {
    /// Number of replan revisions already recorded per plan.
    #[serde(default)]
    pub replans_seen: HashMap<String, u32>,
    /// Stable failure keys already handled by a revision request.
    #[serde(default)]
    pub seen_failure_keys: Vec<String>,
    /// Revision requests issued during this run.
    #[serde(default)]
    pub revision_requests: Vec<PlanRevisionRequest>,
}

/// A task definition rewritten in response to a durable plan revision request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRevision {
    /// Plan containing the task.
    pub plan_id: String,
    /// Original task id being revised. The revised task keeps this id so the
    /// existing DAG and retry counters remain authoritative.
    pub task_id: String,
    /// Dedupe key for the gate failure that produced this revision.
    pub failure_key: String,
    /// Structured request that explains why this task was revised.
    pub revision_request: PlanRevisionRequest,
    /// Revised task data used by dispatch on the next retry and after resume.
    pub revised_task: TaskDef,
}

impl PartialEq for TaskRevision {
    fn eq(&self, other: &Self) -> bool {
        self.plan_id == other.plan_id
            && self.task_id == other.task_id
            && self.failure_key == other.failure_key
            && self.revision_request == other.revision_request
            && serde_json::to_value(&self.revised_task).ok()
                == serde_json::to_value(&other.revised_task).ok()
    }
}

/// Forensic fingerprint of a task definition used for strict resume validation.
///
/// Hash inputs are deterministic and span the fields a plan author can mutate
/// between runs (id, title, role, tier, dependencies, verify steps, gate
/// budgets). Mismatch on resume is a hard failure: see
/// [`super::resume::ResumeReport::drifted_tasks`] for the re-queue signal.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskDefFingerprint {
    /// Plan identifier.
    pub plan_id: String,
    /// Task identifier.
    pub task_id: String,
    /// FNV-1a hash (hex) of the canonical task definition payload.
    pub fingerprint: String,
}

/// Per-rung gate threshold statistics persisted in `.roko/learn/gate-thresholds.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GateThresholdStats {
    #[serde(default)]
    pub pass_count: u64,
    #[serde(default, alias = "total_observations")]
    pub total_count: u64,
    #[serde(default = "GateThresholdStats::default_ema_pass_rate")]
    pub ema_pass_rate: f64,
}

impl GateThresholdStats {
    const fn default_ema_pass_rate() -> f64 {
        0.5
    }
}

impl Default for GateThresholdStats {
    fn default() -> Self {
        Self {
            pass_count: 0,
            total_count: 0,
            ema_pass_rate: Self::default_ema_pass_rate(),
        }
    }
}

/// Persisted adaptive gate thresholds loaded at runner startup.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GateThresholds {
    #[serde(default)]
    pub rungs: HashMap<u32, GateThresholdStats>,
}

impl GateThresholds {
    fn load(path: &Path) -> Result<Self> {
        let file = fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
        serde_json::from_reader(file).with_context(|| format!("parsing {}", path.display()))
    }

    /// Sum of `total_count` across all observed rungs.
    pub fn total_observations(&self) -> u64 {
        self.rungs.values().map(|s| s.total_count).sum()
    }

    pub(crate) fn observe(&mut self, rung: u32, passed: bool) {
        let stats = self.rungs.entry(rung).or_default();
        let value = if passed { 1.0 } else { 0.0 };

        if stats.total_count == 0 {
            stats.ema_pass_rate = value;
        } else {
            stats.ema_pass_rate = 0.1_f64.mul_add(value, 0.9 * stats.ema_pass_rate);
        }

        stats.total_count += 1;
        if passed {
            stats.pass_count += 1;
        }
    }

    pub(crate) fn suggested_max_retries(&self, rung: u32) -> u32 {
        let Some(stats) = self.rungs.get(&rung) else {
            return DEFAULT_GATE_RETRY_COLD_START;
        };

        if stats.total_count < DEFAULT_GATE_RETRY_MIN_OBSERVATIONS {
            return DEFAULT_GATE_RETRY_COLD_START;
        }

        let max_f = f64::from(DEFAULT_GATE_RETRY_MAX);
        let range_f = f64::from(DEFAULT_GATE_RETRY_MAX - DEFAULT_GATE_RETRY_MIN);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let retries = stats.ema_pass_rate.mul_add(-range_f, max_f).round() as u32;

        retries.clamp(DEFAULT_GATE_RETRY_MIN, DEFAULT_GATE_RETRY_MAX)
    }

    pub(crate) fn save(&self, path: &Path) -> Result<()> {
        let json =
            serde_json::to_string_pretty(self).context("serializing adaptive gate thresholds")?;
        atomic_write(path, json.as_bytes())
    }
}

/// Load persisted gate thresholds from disk.
pub fn load_gate_thresholds(paths: &PersistPaths) -> Result<GateThresholds> {
    GateThresholds::load(&paths.gate_thresholds_json)
}

/// Atomically write the adaptive gate thresholds to the standalone
/// `.roko/learn/gate-thresholds.json` file.
///
/// This is the canonical path read by serve, TUI, and ACP. The runner
/// embeds the thresholds inside `state-snapshot.json` for resume, but
/// the standalone file is what external readers depend on.
pub fn save_gate_thresholds(paths: &PersistPaths, thresholds: &GateThresholds) -> Result<()> {
    thresholds.save(&paths.gate_thresholds_json)
}

/// Atomically write `content` to `path` via a `.tmp` sibling.
pub fn atomic_write(path: &Path, content: &[u8]) -> Result<()> {
    roko_fs::atomic_write_bytes(path, content)
        .with_context(|| format!("atomically writing {}", path.display()))
}

/// SH03-T05: Clean stale staging files left by a prior crashed process.
///
/// Scans `state_dir` for files matching `*.tmp.<PID>.<SEQ>`. If the PID
/// no longer exists, the file is from a dead process and can be removed.
/// Files from the current process are left alone.
///
/// Returns the number of files cleaned.
pub fn clean_stale_staging_files(state_dir: &Path) -> usize {
    let current_pid = std::process::id();
    let entries = match fs::read_dir(state_dir) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };
    let mut cleaned = 0;
    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        // Match the roko_fs naming pattern: <name>.tmp.<PID>.<SEQ>
        let Some(tmp_idx) = name_str.rfind(".tmp.") else {
            continue;
        };
        let suffix = &name_str[tmp_idx + 5..]; // after ".tmp."
        let parts: Vec<&str> = suffix.splitn(2, '.').collect();
        if parts.len() != 2 {
            continue;
        }
        let Ok(pid) = parts[0].parse::<u32>() else {
            continue;
        };
        // Never clean our own staging files.
        if pid == current_pid {
            continue;
        }
        // Check if the PID is still alive by probing /proc or using
        // the existence of a send-signal-0 equivalent. On macOS/Linux,
        // checking if /proc/<pid> exists is safe but only works on Linux;
        // fall back to assuming dead if the staging file is >60s old.
        let alive = is_pid_alive(pid);
        if !alive {
            let path = entry.path();
            tracing::debug!(path = %path.display(), pid, "cleaning stale staging file");
            if fs::remove_file(&path).is_ok() {
                cleaned += 1;
            }
        }
    }
    cleaned
}

/// Check if a process ID is still alive without using unsafe.
///
/// Uses `kill -0` via Command on Unix, which is safe. On other platforms
/// conservatively returns `false` (assume dead) so stale files get cleaned.
fn is_pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // `kill -0 <pid>` exits 0 if the process exists (even if we can't
        // signal it), and non-zero otherwise.
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

/// Append a JSON line to a JSONL file.
pub fn append_jsonl(path: &Path, value: &impl Serialize) -> Result<()> {
    let line = serialized_jsonl(value)?;

    // Use the canonical E47 append/rotation boundary. This coordinates with
    // StateHub compaction and lifecycle rotation through the sibling advisory
    // lock, so an event cannot be lost during rename-and-recreate. Runtime
    // config overrides are also applied by the plan lifecycle; this per-append
    // safety valve uses the one canonical ResourcesConfig default.
    let max_mb = roko_core::config::ResourcesConfig::default().log_rotation_max_mb;
    roko_fs::log_rotation::append_jsonl_line_sync(path, &line, max_mb)
        .with_context(|| format!("appending to {}", path.display()))?;
    Ok(())
}

/// Append replayable high-frequency JSONL without an fsync per record.
/// Durable lifecycle/usage/terminal events must continue to use
/// [`append_jsonl`].
pub fn append_jsonl_relaxed(path: &Path, value: &impl Serialize) -> Result<()> {
    let line = serialized_jsonl(value)?;
    let max_mb = roko_core::config::ResourcesConfig::default().log_rotation_max_mb;
    roko_fs::log_rotation::append_jsonl_line_relaxed_sync(path, &line, max_mb)
        .with_context(|| format!("appending relaxed record to {}", path.display()))?;
    Ok(())
}

fn serialized_jsonl(value: &impl Serialize) -> Result<Vec<u8>> {
    let mut line = serde_json::to_vec(value).context("serializing JSONL value")?;
    line.push(b'\n');
    Ok(line)
}

/// Durability class used by the authoritative global runner event log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventDurability {
    /// Flush and sync the global record at the canonical rotation boundary.
    Durable,
    /// Append without an immediate data sync for replayable output deltas.
    Relaxed,
}

const MAX_RUN_INDEX_WRITERS: usize = 32;
const RUN_INDEX_BUFFER_BYTES: usize = 64 * 1024;

#[derive(Default)]
struct RunIndexWriterCache {
    writers: HashMap<PathBuf, BufWriter<File>>,
}

fn run_index_writers() -> &'static Mutex<RunIndexWriterCache> {
    static CACHE: OnceLock<Mutex<RunIndexWriterCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(RunIndexWriterCache::default()))
}

/// Append one runner-owned event to the authoritative global log and its
/// derived per-run read index.
///
/// The global append retains the caller's existing durability class. The
/// derived append reuses a bounded 64 KiB buffered writer and is non-fatal, so
/// streaming agent deltas do not acquire a second rotation lock/open/flush on
/// every chunk. Callers request an index flush only at lifecycle boundaries.
pub fn append_run_scoped_event(
    paths: &PersistPaths,
    run_id: &str,
    value: &impl Serialize,
    durability: EventDurability,
    flush_index: bool,
) -> Result<()> {
    let line = serialized_jsonl(value)?;
    let max_mb = roko_core::config::ResourcesConfig::default().log_rotation_max_mb;
    match durability {
        EventDurability::Durable => {
            roko_fs::log_rotation::append_jsonl_line_sync(&paths.events_jsonl, &line, max_mb)
                .with_context(|| format!("appending to {}", paths.events_jsonl.display()))?;
        }
        EventDurability::Relaxed => {
            roko_fs::log_rotation::append_jsonl_line_relaxed_sync(
                &paths.events_jsonl,
                &line,
                max_mb,
            )
            .with_context(|| {
                format!(
                    "appending relaxed record to {}",
                    paths.events_jsonl.display()
                )
            })?;
        }
    }

    if let Err(error) = append_buffered_run_index(&paths.events_jsonl, run_id, &line, flush_index) {
        tracing::warn!(
            %run_id,
            %error,
            "failed to append buffered derived per-run event index",
        );
    }
    Ok(())
}

fn append_buffered_run_index(
    global_path: &Path,
    run_id: &str,
    line: &[u8],
    flush: bool,
) -> Result<()> {
    let run_path =
        roko_fs::run_index::run_index_path(global_path, run_id).map_err(anyhow::Error::msg)?;
    let mut cache = run_index_writers()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !cache.writers.contains_key(&run_path) && cache.writers.len() >= MAX_RUN_INDEX_WRITERS {
        for writer in cache.writers.values_mut() {
            let _ = writer.flush();
        }
        cache.writers.clear();
    }
    if !cache.writers.contains_key(&run_path) {
        let (opened_path, file) = roko_fs::run_index::open_run_index_append(global_path, run_id)
            .with_context(|| format!("opening run index {}", run_path.display()))?;
        debug_assert_eq!(opened_path, run_path);
        cache.writers.insert(
            run_path.clone(),
            BufWriter::with_capacity(RUN_INDEX_BUFFER_BYTES, file),
        );
    }
    if let Some(writer) = cache.writers.get_mut(&run_path) {
        writer
            .write_all(line)
            .with_context(|| format!("buffering run index {}", run_path.display()))?;
        if flush {
            writer
                .flush()
                .with_context(|| format!("flushing run index {}", run_path.display()))?;
        }
    }
    Ok(())
}

/// Append a normalized runner lifecycle event to the durable JSONL log.
pub fn append_runner_event(paths: &PersistPaths, event: &RunnerEvent) -> Result<()> {
    let flush_index = event.is_scheduler_milestone()
        || matches!(
            event,
            RunnerEvent::RunCompleted { .. }
                | RunnerEvent::AgentCompleted { .. }
                | RunnerEvent::TimeoutRecorded { .. }
                | RunnerEvent::RunPaused { .. }
                | RunnerEvent::BatchPause { .. }
                | RunnerEvent::PlanCancelled { .. }
        );
    append_run_scoped_event(
        paths,
        event.run_id(),
        event,
        EventDurability::Durable,
        flush_index,
    )
}

/// Save the executor snapshot atomically.
pub fn save_executor_snapshot(paths: &PersistPaths, snapshot: &ExecutorSnapshot) -> Result<()> {
    let json = serde_json::to_string_pretty(snapshot).context("serializing executor snapshot")?;
    atomic_write(&paths.executor_json, json.as_bytes())
}

/// Save the aggregate orchestrator snapshot atomically.
pub fn save_orchestrator_snapshot(
    paths: &PersistPaths,
    snapshot: &OrchestratorSnapshot,
) -> Result<()> {
    let json = snapshot
        .to_json()
        .context("serializing orchestrator snapshot")?;
    atomic_write(&paths.orchestrator_json, json.as_bytes())
}

/// Save the set of live agent PIDs.
pub fn save_agent_pids(paths: &PersistPaths, pids: &[u32]) -> Result<()> {
    let json = serde_json::to_string_pretty(&pids).context("serializing agent PIDs")?;
    atomic_write(&paths.agent_pids_json, json.as_bytes())
}

/// Atomically write the runner-owned [`RunStateSnapshot`].
pub fn save_run_state(paths: &PersistPaths, snapshot: &RunStateSnapshot) -> Result<()> {
    let json = serde_json::to_string_pretty(snapshot).context("serializing run state")?;
    atomic_write(&paths.run_state_json, json.as_bytes())
}

/// Load the runner-owned [`RunStateSnapshot`] if it exists. Returns
/// `Ok(None)` when the file is missing; `Err` only on malformed payload
/// or filesystem errors so callers can distinguish "fresh run" from
/// "broken state".
pub fn load_run_state(paths: &PersistPaths) -> Result<Option<RunStateSnapshot>> {
    match read_bounded_string(&paths.run_state_json) {
        Ok(content) => serde_json::from_str(&content)
            .map(Some)
            .with_context(|| format!("parsing {}", paths.run_state_json.display())),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err).with_context(|| format!("reading {}", paths.run_state_json.display())),
    }
}

pub(crate) fn read_bounded_string(path: &Path) -> std::io::Result<String> {
    let file = fs::File::open(path)?;
    let metadata_len = file.metadata()?.len();
    if metadata_len > roko_runtime::MAX_DURABLE_RUNNER_PROJECTION_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "{} is {metadata_len} bytes; maximum is {}",
                path.display(),
                roko_runtime::MAX_DURABLE_RUNNER_PROJECTION_BYTES
            ),
        ));
    }
    let mut bytes = Vec::with_capacity(metadata_len as usize);
    file.take(roko_runtime::MAX_DURABLE_RUNNER_PROJECTION_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > roko_runtime::MAX_DURABLE_RUNNER_PROJECTION_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "{} exceeded maximum {} while reading",
                path.display(),
                roko_runtime::MAX_DURABLE_RUNNER_PROJECTION_BYTES
            ),
        ));
    }
    String::from_utf8(bytes).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("{} is not UTF-8: {error}", path.display()),
        )
    })
}

/// Serialize and atomically write a [`StateSnapshot`] to disk.
///
/// Before overwriting, the existing snapshot is renamed to a `.bak` sibling
/// so a subsequent corrupt-load can fall back to the previous checkpoint.
pub fn save_state_snapshot(paths: &PersistPaths, snapshot: &StateSnapshot) -> Result<()> {
    let json = serde_json::to_vec_pretty(snapshot).context("serializing state snapshot")?;
    if json.len() as u64 > roko_runtime::MAX_DURABLE_RUNNER_PROJECTION_BYTES {
        anyhow::bail!(
            "state snapshot is {} bytes; maximum is {}",
            json.len(),
            roko_runtime::MAX_DURABLE_RUNNER_PROJECTION_BYTES
        );
    }
    // Best-effort backup: rename existing snapshot before overwriting.
    let backup_path = paths.state_snapshot_json.with_extension("json.bak");
    if paths.state_snapshot_json.exists() {
        let _ = std::fs::rename(&paths.state_snapshot_json, &backup_path);
    }
    atomic_write(&paths.state_snapshot_json, &json)
}

/// Load the previous [`StateSnapshot`] from the `.bak` sibling, if present.
///
/// Returns `Ok(None)` when no backup exists. Used as a fallback when the
/// primary snapshot is corrupt.
pub fn load_state_snapshot_backup(paths: &PersistPaths) -> Result<Option<StateSnapshot>> {
    let backup = paths.state_snapshot_json.with_extension("json.bak");
    if !backup.exists() {
        return Ok(None);
    }
    let file = fs::File::open(&backup).with_context(|| format!("opening {}", backup.display()))?;
    let metadata_len = file
        .metadata()
        .with_context(|| format!("stat {}", backup.display()))?
        .len();
    if metadata_len > roko_runtime::MAX_DURABLE_RUNNER_PROJECTION_BYTES {
        anyhow::bail!(
            "backup snapshot {} is {metadata_len} bytes; maximum is {}",
            backup.display(),
            roko_runtime::MAX_DURABLE_RUNNER_PROJECTION_BYTES
        );
    }
    let mut json = Vec::with_capacity(metadata_len as usize);
    file.take(roko_runtime::MAX_DURABLE_RUNNER_PROJECTION_BYTES + 1)
        .read_to_end(&mut json)
        .with_context(|| format!("reading {}", backup.display()))?;
    if json.len() as u64 > roko_runtime::MAX_DURABLE_RUNNER_PROJECTION_BYTES {
        anyhow::bail!(
            "backup snapshot {} exceeded maximum {} while reading",
            backup.display(),
            roko_runtime::MAX_DURABLE_RUNNER_PROJECTION_BYTES
        );
    }
    let snapshot: StateSnapshot =
        serde_json::from_slice(&json).with_context(|| format!("parsing {}", backup.display()))?;
    roko_runtime::validate_state_snapshot(&backup, snapshot.clone())
        .with_context(|| format!("validate backup snapshot {}", backup.display()))?;
    Ok(Some(snapshot))
}

/// Load a [`StateSnapshot`] from disk and validate its checksum.
/// Returns `None` if the file does not exist.
/// Returns `Err` if the file exists but is corrupt or the checksum fails.
pub fn load_state_snapshot(paths: &PersistPaths) -> Result<Option<StateSnapshot>> {
    let path = &paths.state_snapshot_json;
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("opening {}", path.display())),
    };
    let metadata_len = file
        .metadata()
        .with_context(|| format!("stat {}", path.display()))?
        .len();
    if metadata_len > roko_runtime::MAX_DURABLE_RUNNER_PROJECTION_BYTES {
        anyhow::bail!(
            "state snapshot {} is {metadata_len} bytes; maximum is {}",
            path.display(),
            roko_runtime::MAX_DURABLE_RUNNER_PROJECTION_BYTES
        );
    }
    let mut json = Vec::with_capacity(metadata_len as usize);
    file.take(roko_runtime::MAX_DURABLE_RUNNER_PROJECTION_BYTES + 1)
        .read_to_end(&mut json)
        .with_context(|| format!("reading {}", path.display()))?;
    if json.len() as u64 > roko_runtime::MAX_DURABLE_RUNNER_PROJECTION_BYTES {
        anyhow::bail!(
            "state snapshot {} exceeded maximum {} while reading",
            path.display(),
            roko_runtime::MAX_DURABLE_RUNNER_PROJECTION_BYTES
        );
    }
    let snapshot: StateSnapshot =
        serde_json::from_slice(&json).with_context(|| format!("parsing {}", path.display()))?;
    roko_runtime::validate_state_snapshot(path, snapshot.clone())
        .with_context(|| format!("validate authoritative snapshot {}", path.display()))?;
    Ok(Some(snapshot))
}

/// Outcome of a JSONL recovery scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonlRecovery {
    /// File is fully consistent — every line parsed.
    Clean { lines: usize },
    /// File ended with an incomplete line; recovered by truncating after
    /// the last newline. `valid_lines` is what survives.
    TruncatedTrailing {
        valid_lines: usize,
        truncated_bytes: u64,
    },
    /// File ended with one or more malformed JSON lines that did parse as
    /// strings (have terminating `\n`) but failed serde validation.
    /// Recovered by truncating to the last valid line.
    DroppedInvalid {
        valid_lines: usize,
        dropped_lines: usize,
    },
}

/// Inspect a JSONL file for partial-append corruption and recover by
/// truncating at the last successfully-parsed line.
///
/// Strategy: read the file as bytes, try to parse each line through
/// `validator`. If any tail line fails (or the file ends mid-line),
/// the file is rewritten atomically with everything up through the last
/// validated line.
pub fn recover_jsonl<T, F>(path: &Path, validator: F) -> Result<JsonlRecovery>
where
    T: for<'de> Deserialize<'de>,
    F: Fn(&str) -> std::result::Result<T, serde_json::Error>,
{
    let original = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(JsonlRecovery::Clean { lines: 0 });
        }
        Err(err) => {
            return Err(err).with_context(|| format!("reading {}", path.display()));
        }
    };
    if original.is_empty() {
        return Ok(JsonlRecovery::Clean { lines: 0 });
    }

    let text = match std::str::from_utf8(&original) {
        Ok(text) => text,
        Err(_) => {
            // Non-utf8 — refuse to silently destroy it.
            anyhow::bail!("{} is not valid UTF-8; refusing to recover", path.display());
        }
    };

    let trailing_partial = !text.ends_with('\n');
    let mut last_good_byte = 0_u64;
    let mut valid_lines = 0_usize;
    let mut dropped_lines = 0_usize;

    let mut byte_offset = 0_u64;
    for raw_line in text.split_inclusive('\n') {
        let trimmed = raw_line.strip_suffix('\n').unwrap_or(raw_line);
        let is_complete = raw_line.ends_with('\n');
        if !is_complete {
            // Trailing partial line — stop here without counting it as
            // dropped.
            break;
        }
        if trimmed.trim().is_empty() {
            byte_offset += raw_line.len() as u64;
            last_good_byte = byte_offset;
            continue;
        }
        match validator(trimmed) {
            Ok(_) => {
                byte_offset += raw_line.len() as u64;
                last_good_byte = byte_offset;
                valid_lines += 1;
            }
            Err(_) => {
                dropped_lines += 1;
                // Stop on first malformed entry — don't trust the tail.
                break;
            }
        }
    }

    let truncated_bytes = original.len() as u64 - last_good_byte;
    if truncated_bytes == 0 && !trailing_partial && dropped_lines == 0 {
        return Ok(JsonlRecovery::Clean { lines: valid_lines });
    }

    // Truncate to the last validated line. An entirely-invalid file must also
    // be replaced: leaving it in place means every later valid append remains
    // hidden behind corruption and every startup repeats the same diagnosis.
    if last_good_byte == 0 {
        if dropped_lines > 0 {
            atomic_write(path, b"")?;
            return Ok(JsonlRecovery::DroppedInvalid {
                valid_lines: 0,
                dropped_lines,
            });
        }
        atomic_write(path, b"")?;
        return Ok(JsonlRecovery::TruncatedTrailing {
            valid_lines: 0,
            truncated_bytes,
        });
    }

    let kept = &original[..last_good_byte as usize];
    atomic_write(path, kept)?;

    if dropped_lines > 0 {
        Ok(JsonlRecovery::DroppedInvalid {
            valid_lines,
            dropped_lines,
        })
    } else {
        Ok(JsonlRecovery::TruncatedTrailing {
            valid_lines,
            truncated_bytes,
        })
    }
}

impl TaskDefFingerprint {
    /// Compute a forensic fingerprint for `task` in `plan_id`.
    ///
    /// The hash spans the fields a plan author can change between runs;
    /// downstream resume validation rejects mismatches as a hard
    /// failure.
    #[must_use]
    pub fn from_task(task: &crate::task_parser::TaskDef, plan_id: &str) -> Self {
        let canonical = canonical_task_payload(task);
        Self {
            plan_id: plan_id.to_string(),
            task_id: task.id.clone(),
            fingerprint: fnv1a_hex(&canonical),
        }
    }
}

fn canonical_task_payload(task: &crate::task_parser::TaskDef) -> String {
    let depends_on = task.depends_on.join(",");
    let depends_on_plan = task.depends_on_plan.join(",");
    let verify = task
        .verify
        .iter()
        .map(|step| format!("{}:{}:{}", step.phase, step.command, step.timeout_ms))
        .collect::<Vec<_>>()
        .join("|");
    let acceptance = task.acceptance.join("|");
    let role = task.role.clone().unwrap_or_default();
    let domain = task
        .domain
        .as_ref()
        .map(|d| d.label().to_string())
        .unwrap_or_default();
    let max_loc = task.max_loc.map(|n| n.to_string()).unwrap_or_default();
    format!(
        "id={};title={};role={};tier={};domain={};depends_on={};depends_on_plan={};verify={};acceptance={};max_loc={};max_retries={};timeout_secs={}",
        task.id,
        task.title,
        role,
        task.tier,
        domain,
        depends_on,
        depends_on_plan,
        verify,
        acceptance,
        max_loc,
        task.max_retries,
        task.timeout_secs,
    )
}

fn fnv1a_hex(payload: &str) -> String {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = FNV_OFFSET;
    for byte in payload.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:016x}")
}

fn fnv1a_hex_bytes(payload: &[u8]) -> String {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = FNV_OFFSET;
    for byte in payload {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:016x}")
}

/// Write a checkpoint manifest to `<state_dir>/checkpoint.txt`.
///
/// Each entry is written as `name:hash` where `hash` is the FNV-1a hex
/// fingerprint of the file contents supplied in `files`. The manifest is
/// written atomically via [`atomic_write`] so a crash mid-write leaves the
/// previous checkpoint intact.
pub fn write_checkpoint(state_dir: &Path, files: &[(&str, &[u8])]) -> Result<()> {
    let mut lines = String::new();
    for (name, content) in files {
        let hash = fnv1a_hex_bytes(content);
        lines.push_str(name);
        lines.push(':');
        lines.push_str(&hash);
        lines.push('\n');
    }
    let checkpoint_path = state_dir.join("checkpoint.txt");
    atomic_write(&checkpoint_path, lines.as_bytes())
}

/// Verify the checkpoint manifest at `<state_dir>/checkpoint.txt`.
///
/// Re-reads each file listed in the manifest, re-hashes it, and compares
/// against the recorded hash. Returns `Ok(true)` when all hashes match,
/// `Ok(false)` on any mismatch or missing file, and `Err` only on
/// I/O errors reading the manifest itself.
pub fn verify_checkpoint(state_dir: &Path) -> Result<bool> {
    let checkpoint_path = state_dir.join("checkpoint.txt");
    let manifest = match fs::read_to_string(&checkpoint_path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(true); // no checkpoint written yet — treat as passing
        }
        Err(err) => {
            return Err(err).with_context(|| format!("reading {}", checkpoint_path.display()));
        }
    };

    for line in manifest.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((name, expected_hash)) = line.split_once(':') else {
            // Malformed entry — treat as mismatch.
            return Ok(false);
        };
        let file_path = state_dir.join(name);
        let content = match fs::read(&file_path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(false);
            }
            Err(err) => {
                return Err(err).with_context(|| format!("reading {}", file_path.display()));
            }
        };
        let actual_hash = fnv1a_hex_bytes(&content);
        if actual_hash != expected_hash {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Path to the section outcomes JSONL store within the learn directory.
pub fn section_outcomes_path(workdir: &Path) -> PathBuf {
    RokoLayout::for_project(workdir)
        .learn_dir()
        .join("section-outcomes.jsonl")
}

/// Read previously-saved agent PIDs and kill any that are still alive.
pub fn cleanup_orphaned_agents(paths: &PersistPaths) {
    let Ok(content) = fs::read_to_string(&paths.agent_pids_json) else {
        return;
    };
    let pids = match serde_json::from_str::<Vec<u32>>(&content) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(
                path = %paths.agent_pids_json.display(),
                err = %e,
                "malformed agent PID file — removing"
            );
            let _ = fs::remove_file(&paths.agent_pids_json);
            return;
        }
    };

    for pid in pids {
        // Delegate to roko-agent's registry-based cleanup.
        roko_agent::process::register_spawned_pid(pid);
    }
    roko_agent::process::cleanup_orphaned_agents();

    // Clean up the PID file.
    let _ = fs::remove_file(&paths.agent_pids_json);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persist_paths_creates_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = PersistPaths::from_workdir(tmp.path()).unwrap();
        assert!(paths.executor_json.parent().unwrap().is_dir());
        assert!(paths.efficiency_jsonl.parent().unwrap().is_dir());
        assert!(paths.agent_pids_json.parent().unwrap().is_dir());
    }

    #[test]
    fn atomic_write_creates_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.json");
        atomic_write(&path, b"hello").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn append_jsonl_multiple_values() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("log.jsonl");
        append_jsonl(&path, &serde_json::json!({"a": 1})).unwrap();
        append_jsonl(&path, &serde_json::json!({"b": 2})).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn recover_jsonl_replaces_entirely_invalid_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("log.jsonl");
        fs::write(&path, b"not-json\n").unwrap();

        let outcome =
            recover_jsonl::<serde_json::Value, _>(&path, |line| serde_json::from_str(line))
                .unwrap();

        assert_eq!(
            outcome,
            JsonlRecovery::DroppedInvalid {
                valid_lines: 0,
                dropped_lines: 1,
            }
        );
        assert_eq!(fs::read(&path).unwrap(), b"");

        append_jsonl(&path, &serde_json::json!({"recovered": true})).unwrap();
        let recovered: serde_json::Value =
            serde_json::from_str(fs::read_to_string(path).unwrap().trim()).unwrap();
        assert_eq!(recovered, serde_json::json!({"recovered": true}));
    }

    #[test]
    fn recover_jsonl_removes_trailing_partial_without_valid_lines() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("log.jsonl");
        fs::write(&path, b"{\"incomplete\"").unwrap();

        let outcome =
            recover_jsonl::<serde_json::Value, _>(&path, |line| serde_json::from_str(line))
                .unwrap();

        assert_eq!(
            outcome,
            JsonlRecovery::TruncatedTrailing {
                valid_lines: 0,
                truncated_bytes: 13,
            }
        );
        assert_eq!(fs::read(&path).unwrap(), b"");
    }

    #[test]
    fn save_agent_pids_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = PersistPaths::from_workdir(tmp.path()).unwrap();
        save_agent_pids(&paths, &[1234, 5678]).unwrap();

        let content = fs::read_to_string(&paths.agent_pids_json).unwrap();
        let pids: Vec<u32> = serde_json::from_str(&content).unwrap();
        assert_eq!(pids, vec![1234, 5678]);
    }

    #[test]
    fn load_run_state_defaults_missing_cascade_router_json() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = PersistPaths::from_workdir(tmp.path()).unwrap();
        let payload = serde_json::json!({
            "schema_version": RUN_STATE_SCHEMA_VERSION,
            "run_id": "run-1",
            "started_at_ms": 1,
            "timestamp_ms": 2,
            "tasks_total": 3,
            "tasks_completed": 1,
            "tasks_failed": 0,
            "total_tokens_in": 10,
            "total_tokens_out": 20,
            "total_cost_usd": 0.25,
            "total_agent_calls": 2,
            "plan_costs": {},
            "completed_tasks": {},
            "snapshot_fail_streak": 0,
            "fingerprints": []
        });
        atomic_write(
            &paths.run_state_json,
            serde_json::to_string(&payload).unwrap().as_bytes(),
        )
        .unwrap();

        let snapshot = load_run_state(&paths).unwrap().unwrap();
        assert!(snapshot.cascade_router_json.is_none());
    }

    #[test]
    fn section_outcomes_path_lives_in_learn_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let path = section_outcomes_path(tmp.path());
        assert!(path.ends_with("learn/section-outcomes.jsonl"));
        assert!(path.starts_with(tmp.path()));
    }

    #[test]
    fn total_observations_sums_all_rungs() {
        let mut gt = GateThresholds::default();
        gt.observe(1, true);
        gt.observe(1, false);
        gt.observe(2, true);
        assert_eq!(gt.total_observations(), 3);

        gt.observe(3, false);
        gt.observe(3, true);
        assert_eq!(gt.total_observations(), 5);
    }

    /// E07-T10: Prove incremental gate-threshold flush writes to disk
    /// periodically (every N observations) and that data survives a
    /// re-read.
    #[test]
    fn incremental_gate_threshold_flush() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = PersistPaths::from_workdir(tmp.path()).unwrap();
        let config = roko_core::config::RokoConfig::from_toml(
            "[learning]\ngate_threshold_flush_interval = 3\n",
        )
        .expect("parse custom learning cadence");
        let flush_interval = config.learning.effective_gate_threshold_flush_interval();
        assert_eq!(flush_interval, 3);

        let mut thresholds = GateThresholds::default();
        let mut obs_since_flush: u64 = 0;

        // Accumulate observations below the flush interval -- no file yet.
        for i in 0..(flush_interval - 1) {
            thresholds.observe(1, i % 2 == 0);
            obs_since_flush += 1;
            super::super::event_loop::maybe_flush_gate_thresholds(
                &thresholds,
                &mut obs_since_flush,
                &paths,
                flush_interval,
            );
        }
        assert!(
            !paths.gate_thresholds_json.exists(),
            "file must not exist before flush interval reached"
        );
        assert_eq!(obs_since_flush, flush_interval - 1);

        // One more observation crosses the interval -- file must appear.
        thresholds.observe(1, true);
        obs_since_flush += 1;
        super::super::event_loop::maybe_flush_gate_thresholds(
            &thresholds,
            &mut obs_since_flush,
            &paths,
            flush_interval,
        );
        assert!(
            paths.gate_thresholds_json.exists(),
            "file must exist after flush interval reached"
        );
        assert_eq!(obs_since_flush, 0, "counter must reset after flush");

        // Verify the persisted content round-trips.
        let loaded = GateThresholds::load(&paths.gate_thresholds_json).unwrap();
        assert_eq!(loaded, thresholds);

        // Another full interval of observations produces a second flush
        // with updated data.
        for _ in 0..flush_interval {
            thresholds.observe(2, false);
            obs_since_flush += 1;
        }
        super::super::event_loop::maybe_flush_gate_thresholds(
            &thresholds,
            &mut obs_since_flush,
            &paths,
            flush_interval,
        );
        let reloaded = GateThresholds::load(&paths.gate_thresholds_json).unwrap();
        assert_eq!(reloaded, thresholds, "second flush must write updated data");
        assert!(
            reloaded.rungs.contains_key(&2),
            "rung 2 must be present after second flush"
        );
    }

    #[test]
    fn save_creates_backup_of_previous_snapshot() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = PersistPaths::from_workdir(tmp.path()).unwrap();

        // Write a first snapshot (raw JSON, no validation needed for this test).
        let first_content = b"first-snapshot-content";
        atomic_write(&paths.state_snapshot_json, first_content).unwrap();

        // save_state_snapshot renames the existing file to .bak before atomic_write.
        // Construct a minimal valid StateSnapshot for the save call.
        let snap = roko_runtime::StateSnapshot::new(
            1,
            "{}".to_string(),
            "{}".to_string(),
            "{}".to_string(),
            "{}".to_string(),
        );
        save_state_snapshot(&paths, &snap).unwrap();

        let backup_path = paths.state_snapshot_json.with_extension("json.bak");
        assert!(
            backup_path.exists(),
            ".bak file must exist after second save"
        );
        let backup_content = fs::read(&backup_path).unwrap();
        assert_eq!(
            backup_content, first_content,
            ".bak must contain the first snapshot"
        );
    }

    #[test]
    fn load_backup_returns_none_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = PersistPaths::from_workdir(tmp.path()).unwrap();
        let result = load_state_snapshot_backup(&paths).unwrap();
        assert!(result.is_none(), "no backup file means None");
    }

    #[test]
    fn run_state_snapshot_gate_thresholds_json_roundtrip() {
        let mut thresholds = GateThresholds::default();
        thresholds.observe(1, true);
        thresholds.observe(1, false);
        thresholds.observe(2, true);

        let gt_json = serde_json::to_string(&thresholds).unwrap();
        let snapshot = RunStateSnapshot {
            schema_version: RUN_STATE_SCHEMA_VERSION,
            run_id: "roundtrip-test".into(),
            started_at_ms: 0,
            timestamp_ms: 100,
            tasks_total: 3,
            tasks_completed: 1,
            tasks_failed: 0,
            total_tokens_in: 50,
            total_tokens_out: 25,
            total_cost_usd: 0.01,
            total_agent_calls: 1,
            plan_costs: HashMap::new(),
            task_usage: HashMap::new(),
            accounted_usage_attempts: Vec::new(),
            completed_tasks: HashMap::new(),
            failed_tasks: HashMap::new(),
            skipped_tasks: HashMap::new(),
            lifecycle: None,
            snapshot_fail_streak: 0,
            fingerprints: Vec::new(),
            replan_ledger: ReplanLedgerSnapshot::default(),
            revised_tasks: Vec::new(),
            cascade_router_json: None,
            gate_thresholds_json: Some(gt_json.clone()),
            conductor_circuit_breaker_state: None,
        };

        let serialized = serde_json::to_string(&snapshot).unwrap();
        let deserialized: RunStateSnapshot = serde_json::from_str(&serialized).unwrap();
        assert_eq!(
            deserialized.gate_thresholds_json.as_deref(),
            Some(gt_json.as_str())
        );

        // Verify the embedded JSON actually round-trips back to GateThresholds.
        let restored: GateThresholds =
            serde_json::from_str(deserialized.gate_thresholds_json.as_ref().unwrap()).unwrap();
        assert_eq!(restored, thresholds);
    }

    #[test]
    fn run_state_snapshot_with_both_router_and_thresholds_roundtrips() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = PersistPaths::from_workdir(tmp.path()).unwrap();

        let mut thresholds = GateThresholds::default();
        thresholds.observe(3, true);
        thresholds.observe(3, false);

        let snapshot = RunStateSnapshot {
            schema_version: RUN_STATE_SCHEMA_VERSION,
            run_id: "full-roundtrip".into(),
            started_at_ms: 0,
            timestamp_ms: 200,
            tasks_total: 5,
            tasks_completed: 3,
            tasks_failed: 0,
            total_tokens_in: 100,
            total_tokens_out: 50,
            total_cost_usd: 0.05,
            total_agent_calls: 3,
            plan_costs: HashMap::new(),
            task_usage: HashMap::new(),
            accounted_usage_attempts: Vec::new(),
            completed_tasks: HashMap::new(),
            failed_tasks: HashMap::new(),
            skipped_tasks: HashMap::new(),
            lifecycle: None,
            snapshot_fail_streak: 0,
            fingerprints: Vec::new(),
            replan_ledger: ReplanLedgerSnapshot::default(),
            revised_tasks: Vec::new(),
            cascade_router_json: Some(r#"{"model_slugs":["a","b"]}"#.to_string()),
            gate_thresholds_json: Some(serde_json::to_string(&thresholds).unwrap()),
            conductor_circuit_breaker_state: None,
        };

        save_run_state(&paths, &snapshot).unwrap();
        let loaded = load_run_state(&paths).unwrap().expect("must load");
        assert_eq!(loaded, snapshot);
        assert!(loaded.cascade_router_json.is_some());
        assert!(loaded.gate_thresholds_json.is_some());
    }
}
