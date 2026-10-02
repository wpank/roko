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
    /// Fields that roko-acp's `AdaptiveThresholds` keeps for the rung (its
    /// pass streak, CUSUM and poisoning-defense state), carried through a
    /// load and save unchanged (bug-35c901).
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
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
            extra: serde_json::Map::new(),
        }
    }
}

/// Persisted adaptive gate thresholds loaded at runner startup.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GateThresholds {
    #[serde(default)]
    pub rungs: HashMap<u32, GateThresholdStats>,
    /// Fields that roko-acp's `AdaptiveThresholds` keeps in the same file
    /// (its CUSUM settings and SPC detectors), carried through a load and
    /// save unchanged (bug-35c901).
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Per-rung conservative EMA priors used when no observations have been
/// recorded for a rung.  Higher = gate is expected to pass easily;
/// lower = gate is expected to need more retries.
///
/// Indices correspond to the canonical `roko_gate::rung_selector::Rung` enum:
///  0 = Compile, 1 = Lint, 2 = Test, 3 = Symbol,
///  4 = GeneratedTest, 5 = PropertyTest, 6 = Integration
const RUNG_DEFAULT_EMA: [(u32, f64); 7] = [
    (0, 0.85), // Compile — code usually compiles on first attempt
    (1, 0.75), // Lint — clippy warnings are common early in a run
    (2, 0.70), // Test — tests occasionally fail; leave room to learn
    (3, 0.80), // Symbol — symbol manifests are generally stable
    (4, 0.70), // GeneratedTest — generated tests are unpredictable initially
    (5, 0.90), // PropertyTest / fact-check — conservative; rare failures
    (6, 0.70), // Integration / LLM-judge — judge results vary by model
];

impl GateThresholds {
    fn load(path: &Path) -> Result<Self> {
        let file = fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
        serde_json::from_reader(file).with_context(|| format!("parsing {}", path.display()))
    }

    /// Audit #80: Ensure all 7 canonical gate rungs have an entry so that
    /// external readers (TUI, serve, `roko learn gates`) always see a full
    /// picture rather than only the rungs observed during the most recent run.
    ///
    /// This is a **fill-in** operation: rungs that already have observations
    /// are never overwritten.  The conservative priors are used only for rungs
    /// with `total_count == 0`, matching the same guard used by
    /// `GateThresholds::apply_profile`.
    pub fn fill_default_rungs(&mut self) {
        for (rung, prior) in RUNG_DEFAULT_EMA {
            let stats = self
                .rungs
                .entry(rung)
                .or_insert_with(|| GateThresholdStats {
                    ema_pass_rate: prior,
                    ..GateThresholdStats::default()
                });
            // Only update the EMA when this rung truly has no observations.
            // Never clobber learned data.
            if stats.total_count == 0 {
                stats.ema_pass_rate = prior;
            }
        }
    }

    /// Sum of `total_count` across all observed rungs.
    pub fn total_observations(&self) -> u64 {
        self.rungs.values().map(|s| s.total_count).sum()
    }

    /// Fold one gate outcome into `rung`'s pass-rate EMA with the default
    /// `[gates] ema_alpha`. Graph runs pass the configured one
    /// ([`Self::observe_with_alpha`]).
    #[cfg(test)]
    pub(crate) fn observe(&mut self, rung: u32, passed: bool) {
        let alpha = roko_core::config::GatesConfig::default().ema_alpha;
        self.observe_with_alpha(rung, passed, alpha);
    }

    /// Fold one gate outcome into `rung`'s pass-rate EMA with smoothing
    /// factor `alpha`, `[gates] ema_alpha` on Graph runs (gap-7a3527). A
    /// rung's first observation replaces its prior.
    pub(crate) fn observe_with_alpha(&mut self, rung: u32, passed: bool, alpha: f64) {
        let stats = self.rungs.entry(rung).or_default();
        let value = if passed { 1.0 } else { 0.0 };

        if stats.total_count == 0 {
            stats.ema_pass_rate = value;
        } else {
            stats.ema_pass_rate = alpha.mul_add(value, (1.0 - alpha) * stats.ema_pass_rate);
        }

        stats.total_count += 1;
        if passed {
            stats.pass_count += 1;
        }
    }

    /// P1-08: Observe a prediction residual (predicted - actual pass rate).
    ///
    /// When oracles systematically overestimate task success, the absolute
    /// residual tightens the gate threshold for the corresponding rung.
    /// Uses a softer alpha (0.05) to avoid over-reacting to single
    /// observations. Graph verify runs feed it through
    /// [`Self::observe_verify_steps`].
    pub(crate) fn observe_residual(&mut self, rung: u32, residual: f64) {
        let stats = self.rungs.entry(rung).or_default();
        let abs_residual = residual.abs().clamp(0.0, 1.0);
        const RESIDUAL_ALPHA: f64 = 0.05;
        if stats.total_count > 0 {
            let adjustment = RESIDUAL_ALPHA * abs_residual;
            stats.ema_pass_rate = (stats.ema_pass_rate - adjustment).clamp(0.0, 1.0);
        }
    }

    /// Feed one Graph verify sequence, as `(phase, passed)` per step, into
    /// the thresholds: each step whose phase names a canonical rung updates
    /// that rung's EMA with smoothing factor `ema_alpha`. A test-rung step
    /// also feeds [`Self::observe_residual`] with how far the CodingOracle's
    /// test pass-rate forecast for the attempt missed, when that forecast
    /// (`(predicted, confidence)`, taken before the steps ran, so it never
    /// saw them) is confident enough to act on. Returns the `(rung,
    /// residual)` pairs observed.
    pub(crate) fn observe_verify_steps(
        &mut self,
        step_outcomes: &[(String, bool)],
        test_pass_forecast: Option<(f64, f64)>,
        ema_alpha: f64,
    ) -> Vec<(u32, f64)> {
        /// The CodingOracle's own bar for acting on its forecast.
        const MIN_FORECAST_CONFIDENCE: f64 = 0.1;
        let test_rung = roko_gate::Rung::Test.as_index();
        let forecast = test_pass_forecast
            .filter(|(predicted, confidence)| {
                predicted.is_finite() && *confidence > MIN_FORECAST_CONFIDENCE
            })
            .map(|(predicted, _)| predicted);
        let mut residuals = Vec::new();
        for (phase, passed) in step_outcomes {
            // `rung_for_gate_name` is the registry the gate pipeline uses.
            let Some(rung) = roko_gate::rung_for_gate_name(phase).map(|rung| rung.as_index())
            else {
                continue;
            };
            self.observe_with_alpha(rung, *passed, ema_alpha);
            if let Some(predicted) = forecast.filter(|_| rung == test_rung) {
                let residual = predicted - if *passed { 1.0 } else { 0.0 };
                self.observe_residual(rung, residual);
                residuals.push((rung, residual));
            }
        }
        residuals
    }

    /// P1-10: Apply a domain-specific threshold profile.
    ///
    /// Sets rung priors from the profile when the rung has no prior
    /// observations, giving domain-appropriate initial expectations. Graph
    /// verify runs apply their task's profile before observing. A rung's
    /// first observation replaces its prior ([`Self::observe_with_alpha`]).
    pub(crate) fn apply_profile(
        &mut self,
        profile: &roko_gate::adaptive_threshold::ThresholdProfile,
    ) {
        for (&rung, &prior) in &profile.rung_priors {
            let stats = self.rungs.entry(rung).or_default();
            // Only override the EMA if the rung has zero observations so
            // we don't clobber learned state.
            if stats.total_count == 0 {
                stats.ema_pass_rate = prior.clamp(0.0, 1.0);
            }
        }
    }

    /// P1-12: Check whether a rung should be skipped based on its pass
    /// streak, modulated by the temperament.
    ///
    /// - Conservative: never skip.
    /// - Balanced / Exploratory: skip if consecutive passes exceed
    ///   `SKIP_STREAK_THRESHOLD` (20, matching `AdaptiveThresholds`).
    /// - Aggressive: skip at half the threshold (10).
    ///
    /// Advisory only: Graph verify runs log what it would skip and run every
    /// step.
    pub(crate) fn should_skip_rung_for_temperament(
        &self,
        _rung: u32,
        temperament: roko_core::Temperament,
    ) -> bool {
        // The persist-layer GateThresholdStats does not track
        // consecutive_passes (it only stores ema_pass_rate and counts).
        // Without a pass streak counter, we approximate using the EMA:
        // a very high EMA (>0.95) with many observations suggests the
        // rung consistently passes.
        let Some(stats) = self.rungs.get(&_rung) else {
            return false;
        };
        match temperament {
            roko_core::Temperament::Conservative => false,
            roko_core::Temperament::Balanced | roko_core::Temperament::Exploratory => {
                stats.total_count >= 20 && stats.ema_pass_rate > 0.95
            }
            roko_core::Temperament::Aggressive => {
                stats.total_count >= 10 && stats.ema_pass_rate > 0.90
            }
        }
    }

    pub(crate) fn save(&self, path: &Path) -> Result<()> {
        let json =
            serde_json::to_string_pretty(self).context("serializing adaptive gate thresholds")?;
        atomic_write(path, json.as_bytes())
    }

    /// P2-LRN-6 Loop 1: Load thresholds from disk (or return defaults if the
    /// file does not exist yet). Always calls `fill_default_rungs` so all 7
    /// canonical rungs are present regardless of prior observation coverage.
    pub(crate) fn load_or_default(path: &Path) -> Result<Self> {
        let mut thresholds = match Self::load(path) {
            Ok(t) => t,
            Err(err)
                if err.chain().any(|e| {
                    e.downcast_ref::<std::io::Error>()
                        .is_some_and(|io| io.kind() == std::io::ErrorKind::NotFound)
                }) =>
            {
                tracing::debug!(
                    path = %path.display(),
                    "gate-thresholds.json not found; starting from defaults"
                );
                Self::default()
            }
            Err(err) => return Err(err),
        };
        thresholds.fill_default_rungs();
        Ok(thresholds)
    }

    /// Update the thresholds at `path` in one read-modify-write under the
    /// file's sibling lock: load them (defaults when the file is missing,
    /// every canonical rung filled in, as [`Self::load_or_default`] does),
    /// apply `update`, and save them when it changed them. Updates that run
    /// at once, from a run's parallel tasks or another roko process in the
    /// workspace, each build on the one before, so none is lost
    /// (bug-e0f472). A file that cannot be read is an error and is left as
    /// it is. Returns the thresholds as saved and what `update` returned.
    pub(crate) fn update_locked<R>(
        path: &Path,
        update: impl FnOnce(&mut Self) -> R,
    ) -> Result<(Self, R)> {
        roko_fs::with_locked_json_transaction::<Self, _, std::io::Error, _>(path, |thresholds| {
            thresholds.fill_default_rungs();
            let result = update(thresholds);
            Ok((thresholds.clone(), result))
        })
        .with_context(|| format!("updating {}", path.display()))
    }
}

/// Load persisted gate thresholds from disk, or create a fresh default set.
///
/// Audit #80: always calls [`GateThresholds::fill_default_rungs`] after
/// loading so that all 7 canonical rungs are present regardless of which
/// rungs have been exercised in past runs.  If the file does not exist yet
/// (fresh workspace), a fully defaulted set is returned.
pub fn load_gate_thresholds(paths: &PersistPaths) -> Result<GateThresholds> {
    GateThresholds::load_or_default(&paths.gate_thresholds_json)
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

/// Append one event to the derived per-run index only, for an event the
/// global log gets from another writer: a state hub that persists its own
/// events (`StateHub::persists_events`).
pub fn append_run_index_event(
    paths: &PersistPaths,
    run_id: &str,
    value: &impl Serialize,
    flush_index: bool,
) -> Result<()> {
    let line = serialized_jsonl(value)?;
    append_buffered_run_index(&paths.events_jsonl, run_id, &line, flush_index)
}

/// Flush what is buffered of the run `run_id`'s derived index, as a run's
/// writer does when the run ends.
pub fn flush_run_index(paths: &PersistPaths, run_id: &str) -> Result<()> {
    let run_path = roko_fs::run_index::run_index_path(&paths.events_jsonl, run_id)
        .map_err(anyhow::Error::msg)?;
    let mut cache = run_index_writers()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(writer) = cache.writers.get_mut(&run_path) {
        writer
            .flush()
            .with_context(|| format!("flushing run index {}", run_path.display()))?;
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
        // The per-run index is a second copy of the event log, so it gets the
        // redaction the global log's writer applies (roko_fs::log_rotation):
        // agent output can quote a provider key.
        let scrubbed = match std::str::from_utf8(line).map(roko_core::obs::scrub_secrets_in_jsonl) {
            Ok(std::borrow::Cow::Owned(text)) => std::borrow::Cow::Owned(text.into_bytes()),
            _ => std::borrow::Cow::Borrowed(line),
        };
        writer
            .write_all(&scrubbed)
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

    /// Audit #80: `fill_default_rungs` must populate all 7 canonical rung
    /// indices with conservative EMA priors for rungs that have no
    /// observations, without overwriting rungs that already have data.
    #[test]
    fn fill_default_rungs_populates_all_seven_rungs() {
        let mut gt = GateThresholds::default();
        assert!(
            gt.rungs.is_empty(),
            "fresh GateThresholds should have no rungs"
        );

        gt.fill_default_rungs();

        // All 7 canonical rungs must be present.
        for rung_idx in 0u32..7 {
            assert!(
                gt.rungs.contains_key(&rung_idx),
                "rung {rung_idx} must be present after fill_default_rungs"
            );
            let stats = &gt.rungs[&rung_idx];
            assert_eq!(
                stats.total_count, 0,
                "default rung {rung_idx} must have 0 observations"
            );
            assert!(
                stats.ema_pass_rate > 0.0 && stats.ema_pass_rate <= 1.0,
                "default rung {rung_idx} EMA must be in (0, 1]"
            );
        }
    }

    /// Audit #80: `fill_default_rungs` must not clobber rungs that already
    /// have observed data.
    #[test]
    fn fill_default_rungs_preserves_existing_observations() {
        let mut gt = GateThresholds::default();
        // Record 5 successes on rung 0 (Compile).
        for _ in 0..5 {
            gt.observe(0, true);
        }
        let original_stats = gt.rungs[&0].clone();

        gt.fill_default_rungs();

        // Rung 0 must keep its learned EMA and count.
        let after_stats = &gt.rungs[&0];
        assert_eq!(after_stats.total_count, original_stats.total_count);
        assert_eq!(after_stats.pass_count, original_stats.pass_count);
        assert_eq!(after_stats.ema_pass_rate, original_stats.ema_pass_rate);

        // All other rungs must have been filled with defaults.
        for rung_idx in 1u32..7 {
            assert!(gt.rungs.contains_key(&rung_idx));
            assert_eq!(gt.rungs[&rung_idx].total_count, 0);
        }
    }

    /// bug-e0f472: verify runs that update gate-thresholds.json at once each
    /// read what the others saved, so every observation reaches the EMA.
    #[test]
    fn concurrent_verify_updates_keep_every_gate_observation() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("learn").join("gate-thresholds.json");
        let updates: Vec<_> = (0..16_u32)
            .map(|index| {
                let path = path.clone();
                std::thread::spawn(move || {
                    let steps = [("test".to_string(), index % 2 == 0)];
                    GateThresholds::update_locked(&path, |thresholds| {
                        thresholds.observe_verify_steps(&steps, None, 0.1)
                    })
                    .expect("locked update");
                })
            })
            .collect();
        for update in updates {
            update.join().expect("update thread");
        }

        let thresholds = GateThresholds::load_or_default(&path).expect("load thresholds");
        let test = &thresholds.rungs[&2];
        assert_eq!(test.total_count, 16, "{thresholds:?}");
        assert_eq!(test.pass_count, 8, "{thresholds:?}");
        assert_eq!(
            thresholds.rungs.len(),
            7,
            "every canonical rung is filled in"
        );
    }

    /// bug-35c901: a gate-thresholds.json roko-acp wrote keeps the fields
    /// only `AdaptiveThresholds` knows (pass streaks, CUSUM state, SPC
    /// detectors) when a Graph run updates it, and roko-acp reads them back.
    #[test]
    fn gate_thresholds_schema_keeps_acp_fields() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("gate-thresholds.json");
        let mut acp = roko_gate::AdaptiveThresholds::default();
        for _ in 0..3 {
            acp.observe(0, true);
        }
        acp.save(&path).expect("acp thresholds");
        let read = |path: &Path| -> serde_json::Value {
            serde_json::from_str(&fs::read_to_string(path).expect("thresholds")).expect("json")
        };
        let before = read(&path);

        GateThresholds::update_locked(&path, |thresholds| thresholds.observe(0, false))
            .expect("graph update");

        let after = read(&path);
        for key in before.as_object().expect("thresholds").keys() {
            assert!(after.get(key).is_some(), "{key} was dropped: {after}");
        }
        // The Graph path names the count `total_count`, which roko-acp reads.
        let rung_keys = before["rungs"]["0"].as_object().expect("rung 0").keys();
        for key in rung_keys.filter(|key| *key != "total_observations") {
            assert!(
                after["rungs"]["0"].get(key).is_some(),
                "rung field {key} was dropped: {after}"
            );
        }
        let acp = roko_gate::AdaptiveThresholds::load(&path).expect("roko-acp reads it");
        let rung = acp.rung_stats(0).expect("rung 0");
        assert_eq!(rung.total_observations, 4);
        assert_eq!(rung.consecutive_passes, 3);
    }

    /// Audit #80: `load_gate_thresholds` on a fresh workspace (no file)
    /// must not error and must return a set that covers all 7 canonical rungs.
    #[test]
    fn load_gate_thresholds_missing_file_returns_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = PersistPaths::from_workdir(tmp.path()).unwrap();

        // File does not exist — must succeed.
        let thresholds = load_gate_thresholds(&paths)
            .expect("load_gate_thresholds must not error on missing file");

        assert_eq!(
            thresholds.rungs.len(),
            7,
            "missing file must produce 7 default rungs"
        );
        for rung_idx in 0u32..7 {
            assert!(thresholds.rungs.contains_key(&rung_idx));
        }
    }

    /// Audit #80: `load_gate_thresholds` on a file with only 3 rungs
    /// must fill in the remaining 4 without clobbering the existing data.
    #[test]
    fn load_gate_thresholds_partial_file_fills_missing_rungs() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = PersistPaths::from_workdir(tmp.path()).unwrap();

        // Write a partial file covering only rungs 0, 1, 2.
        let mut partial = GateThresholds::default();
        partial.observe(0, true);
        partial.observe(1, false);
        partial.observe(2, true);
        save_gate_thresholds(&paths, &partial).unwrap();

        let loaded = load_gate_thresholds(&paths).unwrap();
        assert_eq!(loaded.rungs.len(), 7, "must have 7 rungs after fill");

        // Rungs 0, 1, 2 must keep their observation counts.
        assert_eq!(loaded.rungs[&0].total_count, 1);
        assert_eq!(loaded.rungs[&1].total_count, 1);
        assert_eq!(loaded.rungs[&2].total_count, 1);

        // Rungs 3-6 must be defaults.
        for rung_idx in 3u32..7 {
            assert_eq!(
                loaded.rungs[&rung_idx].total_count, 0,
                "rung {rung_idx} must be a default with 0 observations"
            );
        }
    }

    /// P2-LRN-6 Loop 1: `load_or_default` returns defaults when the file is
    /// missing, and correctly restores existing observations after a round-trip.
    #[test]
    fn p2_lrn6_loop1_load_or_default_and_ema_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let gt_path = tmp.path().join("gate-thresholds.json");

        // Missing file → defaults with all 7 rungs filled.
        let fresh = GateThresholds::load_or_default(&gt_path).unwrap();
        assert_eq!(fresh.rungs.len(), 7, "defaults must cover all 7 rungs");
        for rung_idx in 0u32..7 {
            assert_eq!(
                fresh.rungs[&rung_idx].total_count, 0,
                "fresh defaults must have 0 observations for rung {rung_idx}"
            );
        }

        // Observe compile (rung 0) passing and lint (rung 1) failing.
        let mut gt = GateThresholds::load_or_default(&gt_path).unwrap();
        gt.observe(0, true); // compile pass
        gt.observe(1, false); // lint fail
        gt.save(&gt_path).unwrap();

        // Reload confirms the EMA was updated and persisted.
        let reloaded = GateThresholds::load_or_default(&gt_path).unwrap();
        assert_eq!(
            reloaded.rungs[&0].total_count, 1,
            "compile rung must have 1 observation"
        );
        assert_eq!(
            reloaded.rungs[&0].pass_count, 1,
            "compile rung pass_count must be 1"
        );
        assert_eq!(
            reloaded.rungs[&1].total_count, 1,
            "lint rung must have 1 observation"
        );
        assert_eq!(
            reloaded.rungs[&1].pass_count, 0,
            "lint rung pass_count must be 0 (it failed)"
        );
        // The compile rung EMA should be 1.0 (first observation is the value).
        assert!(
            (reloaded.rungs[&0].ema_pass_rate - 1.0).abs() < 1e-9,
            "compile EMA should be 1.0 after one passing observation"
        );
        // The lint rung EMA should be 0.0 (first observation is the value).
        assert!(
            reloaded.rungs[&1].ema_pass_rate.abs() < 1e-9,
            "lint EMA should be 0.0 after one failing observation"
        );
    }

    #[test]
    fn verify_steps_feed_rung_emas_and_test_rung_forecast_residuals() {
        let mut thresholds = GateThresholds::default();
        for _ in 0..10 {
            thresholds.observe(2, true);
        }
        let steps = [
            ("compile".to_string(), true),
            ("structural".to_string(), true),
            ("test".to_string(), false),
        ];

        // The oracle forecast a 0.8 pass rate; the test step failed.
        let residuals = thresholds.observe_verify_steps(&steps, Some((0.8, 0.5)), 0.1);
        assert_eq!(residuals.len(), 1);
        assert_eq!(residuals[0].0, 2);
        assert!((residuals[0].1 - 0.8).abs() < 1e-9);
        assert_eq!(thresholds.rungs[&0].total_count, 1);
        assert!(
            !thresholds.rungs.contains_key(&5),
            "a phase outside the canonical rungs updates nothing"
        );
        let test = &thresholds.rungs[&2];
        assert_eq!((test.pass_count, test.total_count), (10, 11));
        // observe: 0.9 * 1.0 = 0.9; residual: 0.9 - 0.05 * 0.8 = 0.86.
        assert!((test.ema_pass_rate - 0.86).abs() < 1e-9, "{test:?}");

        // Without a confident forecast only the EMA moves.
        let mut unforecast = GateThresholds::default();
        assert!(
            unforecast
                .observe_verify_steps(&steps, Some((0.8, 0.1)), 0.1)
                .is_empty()
        );
        assert!(
            unforecast
                .observe_verify_steps(&steps, None, 0.1)
                .is_empty()
        );
        assert_eq!(unforecast.rungs[&2].total_count, 2);
    }
}
