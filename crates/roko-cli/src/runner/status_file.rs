//! Lightweight `.roko/state/status.json` writer and reader.
//!
//! A tiny (< 500 byte) file that lets external tools (`roko status`,
//! `roko plan status`, the evidence collector's status sampling) check a
//! run's progress without parsing the full snapshot. Graph runs keep it
//! current with a [`GraphStatusWriter`], which folds the run's StateHub
//! events into a [`GraphRunStatus`]. Writes are debounced to at most once
//! per second.
//!
//! The reader path (`read_runner_status`) provides staleness detection:
//! if `status.json` is older than 60 seconds and the writer PID is dead,
//! the file is treated as stale.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use roko_core::DashboardEvent;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::error::{RecvError, TryRecvError};
use tokio::sync::oneshot;

use crate::state_hub::StateHub;

/// Debounce interval in milliseconds.
const DEBOUNCE_MS: u64 = 1_000;

/// A Graph run's status is rewritten this often (in milliseconds) while no
/// event changes it, so `updated_at_ms` works as a heartbeat.
const HEARTBEAT_MS: u64 = 5_000;

/// A status file older than this (in milliseconds) is considered potentially
/// stale and requires PID liveness to be trusted.
const STALENESS_THRESHOLD_MS: u64 = 60_000;

/// Last successful write timestamp, in epoch milliseconds.
static LAST_WRITE_MS: AtomicU64 = AtomicU64::new(0);

/// The lightweight status payload written to `status.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunnerStatusFile {
    pub run_id: String,
    pub phase: String,
    /// Finer-grained phase label (e.g. "dispatch", "gate", "merge").
    #[serde(default)]
    pub current_phase: String,
    pub active_plans: usize,
    pub completed_plans: usize,
    pub total_plans: usize,
    pub active_agents: usize,
    pub elapsed_secs: u64,
    pub last_event: String,
    /// PID of the runner process that wrote this file.
    #[serde(default)]
    pub pid: u32,
    /// Unix epoch milliseconds when this file was last written.
    #[serde(default)]
    pub updated_at_ms: u64,
    /// Plan of the latest plan, task, agent or gate event (Graph runs).
    #[serde(default)]
    pub plan_id: String,
    /// Tasks started and not yet finished (Graph runs).
    #[serde(default)]
    pub running_tasks: usize,
    /// Tasks finished, whatever their outcome (Graph runs).
    #[serde(default)]
    pub finished_tasks: usize,
    /// Tasks in the run's plan set (Graph runs).
    #[serde(default)]
    pub total_tasks: usize,
}

/// Attempt to write `status.json` into the given `state_dir`.
///
/// Silently returns without writing if the debounce interval has not elapsed
/// since the last successful write.
pub fn write_status_debounced(state_dir: &Path, status: &RunnerStatusFile) {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let prev = LAST_WRITE_MS.load(Ordering::Relaxed);
    if now_ms.saturating_sub(prev) < DEBOUNCE_MS {
        return;
    }

    let Ok(payload) = serde_json::to_string(status) else {
        return;
    };

    let path = status_file_path(state_dir);
    if let Err(e) = atomic_write(&path, payload.as_bytes()) {
        tracing::trace!(error = %e, "failed to write status.json");
        return;
    }

    LAST_WRITE_MS.store(now_ms, Ordering::Relaxed);
}

/// Write `status.json` immediately, bypassing the periodic-write debounce.
///
/// Terminal lifecycle projections must use this path: the final snapshot can
/// be produced less than one second after the preceding gate snapshot, and a
/// skipped terminal write leaves external readers believing the run is still
/// active.
pub fn write_status_immediate(state_dir: &Path, status: &RunnerStatusFile) {
    let Ok(payload) = serde_json::to_string(status) else {
        return;
    };

    let path = status_file_path(state_dir);
    if let Err(e) = atomic_write(&path, payload.as_bytes()) {
        tracing::trace!(error = %e, "failed to write terminal status.json");
        return;
    }

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    LAST_WRITE_MS.store(now_ms, Ordering::Relaxed);
}

/// Canonical path for the lightweight status file.
pub fn status_file_path(state_dir: &Path) -> PathBuf {
    state_dir.join("status.json")
}

/// Write `data` to a temporary file then rename, avoiding partial reads.
fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Result of reading `status.json` with staleness detection.
#[derive(Debug, Clone)]
pub enum RunnerStatusRead {
    /// The file was read and the writing process appears live.
    Live(RunnerStatusFile),
    /// The file was read but the writing process is dead or the file is
    /// older than 60 seconds with a dead PID.
    Stale(RunnerStatusFile),
    /// The file does not exist or could not be parsed.
    Missing,
}

impl RunnerStatusRead {
    /// Returns the status payload regardless of liveness.
    #[must_use]
    pub fn status(&self) -> Option<&RunnerStatusFile> {
        match self {
            Self::Live(s) | Self::Stale(s) => Some(s),
            Self::Missing => None,
        }
    }

    /// Whether the runner that wrote this file is still alive.
    #[must_use]
    pub fn is_live(&self) -> bool {
        matches!(self, Self::Live(_))
    }
}

/// Read `status.json` from `state_dir` and check staleness.
///
/// A status file is considered live when:
/// - `pid` is nonzero and the process is alive, OR
/// - `updated_at_ms` is within the staleness threshold (60s)
///
/// Legacy files without `pid`/`updated_at_ms` fields fall back to the
/// file modification time for the age check.
pub fn read_runner_status(state_dir: &Path) -> RunnerStatusRead {
    let path = status_file_path(state_dir);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => return RunnerStatusRead::Missing,
    };
    let status: RunnerStatusFile = match serde_json::from_slice(&bytes) {
        Ok(s) => s,
        Err(_) => return RunnerStatusRead::Missing,
    };

    let current_ms = now_ms();

    // Determine age from the embedded timestamp, falling back to file mtime.
    let age_ms = if status.updated_at_ms > 0 {
        current_ms.saturating_sub(status.updated_at_ms)
    } else {
        std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|mtime| mtime.elapsed().ok())
            .map(|elapsed| elapsed.as_millis() as u64)
            .unwrap_or(u64::MAX)
    };

    // If PID is available, check liveness directly.
    if status.pid > 0 {
        if process_is_alive(status.pid) {
            return RunnerStatusRead::Live(status);
        }
        return RunnerStatusRead::Stale(status);
    }

    // Legacy file without PID: trust the file if recent.
    if age_ms <= STALENESS_THRESHOLD_MS {
        RunnerStatusRead::Live(status)
    } else {
        RunnerStatusRead::Stale(status)
    }
}

/// Check if a process with the given PID is alive.
#[allow(unsafe_code)]
fn process_is_alive(pid: u32) -> bool {
    // Use kill(0) to check existence without sending a signal.
    // This avoids the overhead of sysinfo for a single PID check.
    #[cfg(unix)]
    {
        // SAFETY: kill(pid, 0) is a well-defined POSIX operation that checks
        // process existence without sending a signal. No memory safety concern.
        unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

// ── Graph runs ───────────────────────────────────────────────────────────

/// A Graph run's status, folded from the events the run publishes to its
/// StateHub.
///
/// The phase is `dispatch` while an agent runs, `gate` while a task's verify
/// steps run, and `idle` otherwise; once the run ends it is `completed`,
/// `failed` or `cancelled`. Graph runs publish no merge events, so there is
/// no `merge` phase.
#[derive(Debug)]
pub struct GraphRunStatus {
    run_id: String,
    started: Instant,
    total_plans: usize,
    total_tasks: usize,
    active_plans: BTreeSet<String>,
    completed_plans: BTreeSet<String>,
    /// `(plan, task)` keys.
    running_tasks: BTreeSet<(String, String)>,
    finished_tasks: BTreeSet<(String, String)>,
    /// Tasks whose verify steps started since their last agent dispatch.
    gating_tasks: BTreeSet<(String, String)>,
    active_agents: BTreeSet<String>,
    plan_id: String,
    last_event: String,
    /// Terminal phase, once the run ended.
    outcome: Option<&'static str>,
}

impl GraphRunStatus {
    /// The status of a run that has published nothing yet.
    #[must_use]
    pub fn new(run_id: impl Into<String>) -> Self {
        Self {
            run_id: run_id.into(),
            started: Instant::now(),
            total_plans: 0,
            total_tasks: 0,
            active_plans: BTreeSet::new(),
            completed_plans: BTreeSet::new(),
            running_tasks: BTreeSet::new(),
            finished_tasks: BTreeSet::new(),
            gating_tasks: BTreeSet::new(),
            active_agents: BTreeSet::new(),
            plan_id: String::new(),
            last_event: "none".to_string(),
            outcome: None,
        }
    }

    /// Fold one event into the status; returns whether it changed. Events
    /// after the run ended change nothing.
    pub fn apply(&mut self, event: &DashboardEvent) -> bool {
        if self.outcome.is_some() {
            return false;
        }
        let (kind, plan_id, task_id) = match event {
            DashboardEvent::PlanSetLoaded { plans } => {
                self.total_plans = plans.len();
                self.total_tasks = plans.iter().map(|plan| plan.tasks_total).sum();
                ("plan_set_loaded", "", "")
            }
            DashboardEvent::PlanStarted { plan_id, .. } => {
                self.active_plans.insert(plan_id.clone());
                ("plan_started", plan_id.as_str(), "")
            }
            DashboardEvent::PlanCompleted { plan_id, .. } => {
                self.active_plans.remove(plan_id);
                self.completed_plans.insert(plan_id.clone());
                ("plan_completed", plan_id.as_str(), "")
            }
            DashboardEvent::TaskStarted {
                plan_id, task_id, ..
            } => {
                let key = (plan_id.clone(), task_id.clone());
                self.finished_tasks.remove(&key);
                self.running_tasks.insert(key);
                ("task_started", plan_id.as_str(), task_id.as_str())
            }
            DashboardEvent::TaskCompleted {
                plan_id, task_id, ..
            } => {
                let key = (plan_id.clone(), task_id.clone());
                self.running_tasks.remove(&key);
                self.gating_tasks.remove(&key);
                self.finished_tasks.insert(key);
                ("task_completed", plan_id.as_str(), task_id.as_str())
            }
            DashboardEvent::AgentSpawned {
                agent_id,
                plan_id,
                task_id,
                ..
            } => {
                let key = (plan_id.clone(), task_id.clone());
                self.active_agents.insert(agent_id.clone());
                self.gating_tasks.remove(&key);
                ("agent_spawned", plan_id.as_str(), task_id.as_str())
            }
            DashboardEvent::AgentCompleted {
                agent_id,
                plan_id,
                task_id,
                ..
            } => {
                self.active_agents.remove(agent_id);
                ("agent_completed", plan_id.as_str(), task_id.as_str())
            }
            DashboardEvent::GateRungStarted {
                plan_id, task_id, ..
            } => {
                let key = (plan_id.clone(), task_id.clone());
                self.gating_tasks.insert(key);
                ("gate_rung_started", plan_id.as_str(), task_id.as_str())
            }
            DashboardEvent::GateResult {
                plan_id, task_id, ..
            } => ("gate_result", plan_id.as_str(), task_id.as_str()),
            DashboardEvent::RunCompleted { outcome, .. } => {
                self.outcome = Some(terminal_phase(outcome));
                ("run_completed", "", "")
            }
            _ => return false,
        };
        if !plan_id.is_empty() {
            self.plan_id = plan_id.to_string();
        }
        self.last_event = if task_id.is_empty() {
            kind.to_string()
        } else {
            format!("{kind}:{plan_id}/{task_id}")
        };
        true
    }

    /// End the run. Its terminal phase comes from its `RunCompleted` event
    /// when one was folded, else from `outcome` (`succeeded`, `failed` or
    /// `cancelled`; a failure when unknown).
    pub fn finish(&mut self, outcome: Option<&str>) {
        if self.outcome.is_none() {
            self.outcome = Some(terminal_phase(outcome.unwrap_or("failed")));
            self.last_event = "run_completed".to_string();
        }
    }

    /// Whether the run ended.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.outcome.is_some()
    }

    /// The current phase (see the type docs).
    #[must_use]
    pub fn phase(&self) -> &'static str {
        match self.outcome {
            Some(outcome) => outcome,
            None if !self.active_agents.is_empty() => "dispatch",
            None if !self.gating_tasks.is_empty() => "gate",
            None => "idle",
        }
    }

    /// The `status.json` payload for the status now.
    #[must_use]
    pub fn to_file(&self) -> RunnerStatusFile {
        let running = !self.is_finished();
        let phase = self.phase().to_string();
        RunnerStatusFile {
            run_id: self.run_id.clone(),
            phase: phase.clone(),
            current_phase: phase,
            active_plans: if running { self.active_plans.len() } else { 0 },
            completed_plans: self.completed_plans.len(),
            total_plans: self
                .total_plans
                .max(self.active_plans.len() + self.completed_plans.len()),
            active_agents: if running { self.active_agents.len() } else { 0 },
            elapsed_secs: self.started.elapsed().as_secs(),
            last_event: self.last_event.clone(),
            pid: std::process::id(),
            updated_at_ms: now_ms(),
            plan_id: self.plan_id.clone(),
            running_tasks: if running { self.running_tasks.len() } else { 0 },
            finished_tasks: self.finished_tasks.len(),
            total_tasks: self.total_tasks,
        }
    }
}

/// The terminal phase for a `RunCompleted` outcome label, in the vocabulary
/// the Runner-v2 status file used.
fn terminal_phase(outcome: &str) -> &'static str {
    match outcome {
        "succeeded" | "completed" => "completed",
        "cancelled" | "canceled" => "cancelled",
        _ => "failed",
    }
}

/// Keeps `<state_dir>/status.json` current for one Graph run.
///
/// [`spawn`](Self::spawn) subscribes to the run's hub and writes the starting
/// status at once. While the run goes on, a status an event changed is
/// written within a second, an unchanged one is rewritten every five seconds
/// (so `updated_at_ms` is a heartbeat), and the terminal status is written as
/// soon as the run's `RunCompleted` event arrives. [`finish`](Self::finish)
/// writes the final status and leaves the file in place with its terminal
/// phase. Dropping an unfinished writer stops it without a final write.
pub struct GraphStatusWriter {
    stop: Option<oneshot::Sender<String>>,
    task: Option<tokio::task::JoinHandle<()>>,
}

impl GraphStatusWriter {
    /// Subscribe to `hub`, write run `run_id`'s starting status, and keep
    /// the file current until [`finish`](Self::finish).
    pub fn spawn(hub: &StateHub, state_dir: PathBuf, run_id: String) -> Self {
        let mut events = hub.subscribe_events();
        let mut status = GraphRunStatus::new(run_id);
        write_status_immediate(&state_dir, &status.to_file());
        let (stop, mut stopped) = oneshot::channel::<String>();
        let task = tokio::spawn(async move {
            let debounce = Duration::from_millis(DEBOUNCE_MS);
            let heartbeat = Duration::from_millis(HEARTBEAT_MS);
            let mut ticks = tokio::time::interval(debounce);
            ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            let mut written_at = Instant::now();
            let mut dirty = false;
            let mut open = true;
            let outcome = loop {
                // Ticks come before events so a busy stream of agent output
                // cannot starve the debounced writes.
                tokio::select! {
                    biased;
                    outcome = &mut stopped => break outcome.ok(),
                    _ = ticks.tick() => {
                        if !dirty && written_at.elapsed() < heartbeat {
                            continue;
                        }
                    }
                    received = events.recv(), if open => {
                        match received {
                            Ok(envelope) => dirty |= status.apply(&envelope.payload),
                            // A missed event may have changed the status.
                            Err(RecvError::Lagged(_)) => dirty = true,
                            Err(RecvError::Closed) => open = false,
                        }
                        if !(dirty && status.is_finished()) {
                            continue;
                        }
                    }
                }
                write_status_immediate(&state_dir, &status.to_file());
                written_at = Instant::now();
                dirty = false;
            };
            // Fold what was published before the stop request.
            loop {
                match events.try_recv() {
                    Ok(envelope) => {
                        status.apply(&envelope.payload);
                    }
                    Err(TryRecvError::Lagged(_)) => {}
                    Err(TryRecvError::Empty | TryRecvError::Closed) => break,
                }
            }
            status.finish(outcome.as_deref());
            write_status_immediate(&state_dir, &status.to_file());
        });
        Self {
            stop: Some(stop),
            task: Some(task),
        }
    }

    /// Fold the events published so far, write the final status and stop.
    /// `outcome` is the run's `RunCompleted` outcome label, used when that
    /// event did not reach the writer.
    pub async fn finish(mut self, outcome: &str) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(outcome.to_string());
        }
        if let Some(task) = self.task.take()
            && let Err(error) = task.await
        {
            tracing::warn!(%error, "status.json writer failed");
        }
    }
}

impl Drop for GraphStatusWriter {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use roko_core::dashboard_snapshot::PlanSetEntry;

    use super::*;

    fn test_status() -> RunnerStatusFile {
        RunnerStatusFile {
            run_id: "run-1".to_string(),
            phase: "gate".to_string(),
            current_phase: "gate".to_string(),
            active_plans: 1,
            completed_plans: 0,
            total_plans: 1,
            active_agents: 0,
            elapsed_secs: 4,
            last_event: "task:plan-verify".to_string(),
            pid: std::process::id(),
            updated_at_ms: now_ms(),
            ..RunnerStatusFile::default()
        }
    }

    #[test]
    fn immediate_write_replaces_a_nonterminal_status() {
        let dir = tempfile::tempdir().expect("temporary status directory");
        let mut status = test_status();
        write_status_immediate(dir.path(), &status);

        status.phase = "completed".to_string();
        status.current_phase = "completed".to_string();
        status.active_plans = 0;
        status.completed_plans = 1;
        status.last_event = "run.completed".to_string();
        write_status_immediate(dir.path(), &status);

        let persisted: serde_json::Value = serde_json::from_slice(
            &std::fs::read(status_file_path(dir.path())).expect("read terminal status"),
        )
        .expect("parse terminal status");
        assert_eq!(persisted["phase"], "completed");
        assert_eq!(persisted["active_plans"], 0);
        assert_eq!(persisted["completed_plans"], 1);
        assert_eq!(persisted["last_event"], "run.completed");
    }

    #[test]
    fn read_runner_status_live_with_current_pid() {
        let dir = tempfile::tempdir().unwrap();
        let status = test_status();
        write_status_immediate(dir.path(), &status);

        let result = read_runner_status(dir.path());
        assert!(result.is_live());
        let read = result.status().unwrap();
        assert_eq!(read.run_id, "run-1");
        assert_eq!(read.pid, std::process::id());
    }

    #[test]
    fn read_runner_status_stale_with_dead_pid() {
        let dir = tempfile::tempdir().unwrap();
        let mut status = test_status();
        // Use an implausible PID that is almost certainly dead.
        status.pid = 4_000_000;
        write_status_immediate(dir.path(), &status);

        let result = read_runner_status(dir.path());
        assert!(!result.is_live());
        assert!(result.status().is_some());
    }

    #[test]
    fn read_runner_status_missing_returns_missing() {
        let dir = tempfile::tempdir().unwrap();
        let result = read_runner_status(dir.path());
        assert!(matches!(result, RunnerStatusRead::Missing));
    }

    #[test]
    fn deserialize_legacy_status_without_new_fields() {
        let legacy = r#"{"run_id":"r","phase":"idle","active_plans":0,"completed_plans":0,"total_plans":0,"active_agents":0,"elapsed_secs":0,"last_event":"none"}"#;
        let status: RunnerStatusFile = serde_json::from_str(legacy).unwrap();
        assert_eq!(status.pid, 0);
        assert_eq!(status.updated_at_ms, 0);
        assert!(status.current_phase.is_empty());
        assert!(status.plan_id.is_empty());
        assert_eq!((status.running_tasks, status.finished_tasks), (0, 0));
    }

    fn agent_spawned(plan_id: &str, task_id: &str) -> DashboardEvent {
        DashboardEvent::AgentSpawned {
            agent_id: format!("{plan_id}/{task_id}"),
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            attempt: 0,
            role: "implementer".to_string(),
            model: String::new(),
            provider: String::new(),
        }
    }

    fn run_completed(outcome: &str) -> DashboardEvent {
        DashboardEvent::RunCompleted {
            outcome: outcome.to_string(),
            duration_ms: 9,
            cleanup_degraded: false,
            surviving_agent_ids: Vec::new(),
            surviving_agent_pids: Vec::new(),
        }
    }

    /// The status file once its phase is `phase`, waiting at most five
    /// seconds.
    async fn wait_for_phase(state_dir: &Path, phase: &str) -> RunnerStatusFile {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let read = read_runner_status(state_dir);
            if let Some(status) = read.status().filter(|status| status.phase == phase) {
                return status.clone();
            }
            assert!(Instant::now() < deadline, "status.json = {read:?}");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    #[test]
    fn graph_status_follows_a_run_through_its_phases() {
        let mut status = GraphRunStatus::new("run-1");
        let file = status.to_file();
        assert_eq!(file.run_id, "run-1");
        assert_eq!(file.phase, "idle");
        assert_eq!(file.pid, std::process::id());

        let plans = vec![
            PlanSetEntry {
                plan_id: "p1".to_string(),
                tasks_total: 2,
                ..PlanSetEntry::default()
            },
            PlanSetEntry {
                plan_id: "p2".to_string(),
                tasks_total: 1,
                ..PlanSetEntry::default()
            },
        ];
        assert!(status.apply(&DashboardEvent::PlanSetLoaded { plans }));
        status.apply(&DashboardEvent::PlanStarted {
            plan_id: "p1".to_string(),
            tasks_total: 2,
        });
        status.apply(&DashboardEvent::TaskStarted {
            plan_id: "p1".to_string(),
            task_id: "T1".to_string(),
            title: String::new(),
            phase: "graph-executing".to_string(),
        });
        status.apply(&agent_spawned("p1", "T1"));
        assert_eq!(status.phase(), "dispatch");
        let file = status.to_file();
        assert_eq!(
            (file.active_plans, file.total_plans, file.total_tasks),
            (1, 2, 3)
        );
        assert_eq!((file.running_tasks, file.active_agents), (1, 1));
        assert_eq!(file.plan_id, "p1");
        assert_eq!(file.last_event, "agent_spawned:p1/T1");

        // Streamed output changes nothing the file shows.
        let output = DashboardEvent::AgentOutput {
            agent_id: "p1/T1".to_string(),
            plan_id: "p1".to_string(),
            task_id: "T1".to_string(),
            attempt: 0,
            content: "text".to_string(),
        };
        assert!(!status.apply(&output));
        status.apply(&DashboardEvent::AgentCompleted {
            agent_id: "p1/T1".to_string(),
            plan_id: "p1".to_string(),
            task_id: "T1".to_string(),
            attempt: 0,
        });
        assert_eq!(status.phase(), "idle");
        status.apply(&DashboardEvent::GateRungStarted {
            plan_id: "p1".to_string(),
            task_id: "T1".to_string(),
            rung_name: "verify[0]".to_string(),
        });
        assert_eq!(status.phase(), "gate");
        status.apply(&DashboardEvent::TaskCompleted {
            plan_id: "p1".to_string(),
            task_id: "T1".to_string(),
            outcome: "passed".to_string(),
        });
        assert_eq!(status.phase(), "idle");
        status.apply(&DashboardEvent::PlanCompleted {
            plan_id: "p1".to_string(),
            success: true,
        });
        let file = status.to_file();
        assert_eq!((file.running_tasks, file.finished_tasks), (0, 1));
        assert_eq!((file.active_plans, file.completed_plans), (0, 1));

        assert!(status.apply(&run_completed("failed")));
        assert!(status.is_finished());
        // Neither later events nor the caller's outcome change a finished run.
        assert!(!status.apply(&agent_spawned("p2", "T1")));
        status.finish(Some("succeeded"));
        let file = status.to_file();
        assert_eq!(file.phase, "failed");
        assert_eq!(file.current_phase, "failed");
        assert_eq!(file.last_event, "run_completed");
        assert_eq!(file.active_agents, 0);
    }

    #[tokio::test]
    async fn graph_status_writer_writes_the_start_each_change_and_the_end() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state_dir = dir.path().join("state");
        let hub = crate::state_hub::shared_state_hub();
        let writer = GraphStatusWriter::spawn(&hub, state_dir.clone(), "run-7".to_string());

        // The starting status is on disk before the run publishes anything.
        let read = read_runner_status(&state_dir);
        assert!(read.is_live(), "{read:?}");
        let start = read.status().expect("starting status");
        assert_eq!(start.run_id, "run-7");
        assert_eq!(start.phase, "idle");

        let sender = hub.sender();
        sender.publish(DashboardEvent::PlanStarted {
            plan_id: "p1".to_string(),
            tasks_total: 2,
        });
        sender.publish(agent_spawned("p1", "T1"));
        let dispatching = wait_for_phase(&state_dir, "dispatch").await;
        assert_eq!(dispatching.plan_id, "p1");
        assert_eq!(dispatching.active_agents, 1);

        // No RunCompleted reached the writer, so the caller's outcome ends it.
        writer.finish("cancelled").await;
        let end = read_runner_status(&state_dir);
        let end = end.status().expect("final status");
        assert_eq!(end.phase, "cancelled");
        assert_eq!(end.last_event, "run_completed");
        assert_eq!((end.active_agents, end.active_plans), (0, 0));
        assert_eq!(end.total_plans, 1);
    }

    #[tokio::test]
    async fn graph_status_writer_writes_the_terminal_status_at_once() {
        let dir = tempfile::tempdir().expect("tempdir");
        let hub = crate::state_hub::shared_state_hub();
        let writer = GraphStatusWriter::spawn(&hub, dir.path().to_path_buf(), "run-8".to_string());

        hub.sender().publish(run_completed("succeeded"));
        let end = wait_for_phase(dir.path(), "completed").await;
        assert_eq!(end.run_id, "run-8");
        // The run's own outcome wins over the one `finish` is given.
        writer.finish("failed").await;
        let read = read_runner_status(dir.path());
        assert_eq!(read.status().expect("final status").phase, "completed");
    }
}
