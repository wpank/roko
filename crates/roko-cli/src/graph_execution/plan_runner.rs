//! Graph plan execution entry point for library-callable use.
//!
//! This module contains the primary graph plan execution function, extracted
//! from the binary-side `cmd_plan_run_engine` so that `serve_runtime` and
//! other library callers can invoke it without depending on the binary crate.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::IsTerminal as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context as _, anyhow};
use futures::StreamExt as _;
use roko_core::config::homeostasis::HomeostasisMode;
use roko_core::config::learning::GuardedCommitMode;
use roko_fs::RokoLayout;

use super::plan_set::{BlockReason, PlanConflicts, PlanOutcome, PlanSetOrder, PlanSetScheduler};
use crate::execution_control::{
    CommandAckReceiver, CommandAckStatus, ExecutionCommandKind, ExecutionCommandSender, ack_for,
};
use crate::exit_codes::{EXIT_FAILURE, EXIT_SUCCESS};
use crate::graph_checkpoint::{GraphCheckpointStatus, TaskOutcomeSummary};

// ── Private helpers ──────────────────────────────────────────────────────

/// Inline progress telemetry sink that prints per-node lifecycle events to
/// stderr and delegates to the inner (StateHub) sink. This provides real-time
/// feedback during Graph engine execution without requiring the full TUI.
struct InlineProgressTelemetrySink {
    inner: Arc<dyn roko_core::TelemetryEventSink>,
    show_progress: bool,
}

#[async_trait::async_trait]
impl roko_core::TelemetryEventSink for InlineProgressTelemetrySink {
    async fn emit(
        &self,
        event: &roko_core::ObservableEvent,
        ancestry: &[roko_core::LensScope],
    ) -> roko_core::error::Result<Vec<roko_core::Signal>> {
        if self.show_progress {
            match event {
                roko_core::ObservableEvent::CellStarted { block, .. } => {
                    eprintln!("    \u{25b8} executing node '{block}'...");
                }
                roko_core::ObservableEvent::CellCompleted {
                    block,
                    duration_ms,
                    cost_usd,
                    ..
                } => {
                    let secs = *duration_ms as f64 / 1000.0;
                    if *cost_usd > 0.0 {
                        eprintln!(
                            "    \u{2713} node '{block}' completed ({secs:.1}s, ${cost_usd:.4})"
                        );
                    } else {
                        eprintln!("    \u{2713} node '{block}' completed ({secs:.1}s)");
                    }
                }
                roko_core::ObservableEvent::CellFailed { block, error, .. } => {
                    eprintln!("    \u{2717} node '{block}' failed: {error}");
                }
                _ => {}
            }
        }
        self.inner.emit(event, ancestry).await
    }
}

fn join_approval_tui_thread(handle: Option<std::thread::JoinHandle<anyhow::Result<()>>>) {
    let Some(handle) = handle else {
        return;
    };

    match handle.join() {
        Ok(Ok(())) => {}
        Ok(Err(err)) => {
            tracing::error!(error = %err, "approval TUI exited with error");
        }
        Err(_) => {
            tracing::error!("approval TUI thread panicked");
        }
    }
}

/// Entries for [`roko_core::DashboardEvent::PlanSetLoaded`], in execution
/// order, with each plan's group under `plans_root`, dependency wave,
/// prerequisites and conflicts.
fn plan_set_entries(
    plans_root: &Path,
    plans: &[crate::runner::plan_loader::Plan],
    order: &PlanSetOrder,
    conflicts: &PlanConflicts,
) -> Vec<roko_core::dashboard_snapshot::PlanSetEntry> {
    let dag = crate::runner::plan_dag::CrossPlanDag::compute(plans).ok();
    order
        .order
        .iter()
        .filter_map(|plan_id| plans.iter().find(|plan| &plan.id == plan_id))
        .map(|plan| {
            // A plan named for people goes by its title (gap-c2efe7).
            let title = plan_title(plan);
            roko_core::dashboard_snapshot::PlanSetEntry {
                plan_id: plan.id.clone(),
                title: if title.is_empty() {
                    plan.id.clone()
                } else {
                    title.to_string()
                },
                group: plan_set_group(plans_root, &plan.dir),
                tasks_total: plan.tasks.tasks.len(),
                wave: dag
                    .as_ref()
                    .and_then(|dag| dag.wave_for_plan(&plan.id))
                    .unwrap_or(0),
                depends_on: order
                    .dependencies
                    .get(&plan.id)
                    .into_iter()
                    .flatten()
                    .cloned()
                    .collect(),
                conflicts_with: conflicts
                    .get(&plan.id)
                    .map(|others| others.keys().cloned().collect())
                    .unwrap_or_default(),
            }
        })
        .collect()
}

/// What `plan` is called: its `[meta] title` (gap-c2efe7), else its `[meta]
/// plan` id.
fn plan_title(plan: &crate::runner::plan_loader::Plan) -> &str {
    let meta = &plan.tasks.meta;
    meta.title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| meta.plan.trim())
}

/// The plan set containing `plan_dir`: its parent directory relative to
/// `plans_root`, `/`-separated, as plan discovery names it. `None` for a plan
/// directly under the root or outside it.
fn plan_set_group(plans_root: &Path, plan_dir: &Path) -> Option<String> {
    let parent = plan_dir.parent()?;
    let relative = parent
        .strip_prefix(plans_root)
        .ok()
        .map(Path::to_path_buf)
        .or_else(|| {
            // The two may be spelled differently, e.g. one relative, one absolute.
            let parent = parent.canonicalize().ok()?;
            let root = plans_root.canonicalize().ok()?;
            parent.strip_prefix(root).ok().map(Path::to_path_buf)
        })?;
    let group = relative
        .components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    (!group.is_empty()).then_some(group)
}

// ── Stopping a plan run: SIGINT, SIGTERM, operator ───────────────────────

/// Why a CLI plan run stopped before its plan set finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanRunInterrupt {
    /// SIGINT.
    Interrupt,
    /// SIGTERM.
    Terminate,
    /// SIGHUP: the run's terminal hung up (bug-4641e3).
    Hangup,
    /// The FAST run deadline (`ROKO_FAST_PLAN_DEADLINE_SECS`) elapsed
    /// (gap-9efe8e). It exits as SIGTERM does, which `./dev.sh fast` expects.
    Deadline,
    /// A conductor watcher failed the run (gap-fab2cc); the run ends with
    /// the watcher's error.
    Conductor,
    /// A cancel request: a host cancelled the run through its cancel token,
    /// as `roko serve` does for `run_cancel` over `/mcp` and for
    /// `POST /plans/{id}/cancel` (bug-1d09e8). It exits as SIGINT does.
    Cancel,
    /// The operator closed the TUI while plans were still running. It exits
    /// as SIGINT does.
    TuiClosed,
}

impl PlanRunInterrupt {
    /// Every stop cause, e.g. to tell a stopped run's exit status apart.
    pub const ALL: [Self; 7] = [
        Self::Interrupt,
        Self::Terminate,
        Self::Hangup,
        Self::Deadline,
        Self::Conductor,
        Self::Cancel,
        Self::TuiClosed,
    ];

    /// Conventional shell status for the signal: 128 + signal number.
    #[must_use]
    pub const fn exit_code(self) -> i32 {
        match self {
            Self::Interrupt | Self::Cancel | Self::TuiClosed => 130,
            Self::Terminate | Self::Deadline | Self::Conductor => 143,
            Self::Hangup => 129,
        }
    }

    /// Signal name, `deadline`, `conductor`, `cancel` or `tui-closed`, for
    /// logs, summaries and the checkpoint's stop cause.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Interrupt => "SIGINT",
            Self::Terminate => "SIGTERM",
            Self::Hangup => "SIGHUP",
            Self::Deadline => "deadline",
            Self::Conductor => "conductor",
            Self::Cancel => "cancel",
            Self::TuiClosed => "tui-closed",
        }
    }

    /// What stopped the run, for the person reading its events: "a cancel
    /// request stopped the run" (bug-1d09e8).
    #[must_use]
    pub const fn cause(self) -> &'static str {
        match self {
            Self::Interrupt => "SIGINT",
            Self::Terminate => "SIGTERM",
            Self::Hangup => "SIGHUP",
            Self::Deadline => "the FAST deadline",
            Self::Conductor => "a conductor watcher",
            Self::Cancel => "a cancel request",
            Self::TuiClosed => "closing the TUI",
        }
    }

    const fn code(self) -> u8 {
        match self {
            Self::Interrupt => 1,
            Self::Terminate => 2,
            Self::Hangup => 3,
            Self::Deadline => 4,
            Self::Conductor => 5,
            Self::Cancel => 6,
            Self::TuiClosed => 7,
        }
    }

    const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Interrupt),
            2 => Some(Self::Terminate),
            3 => Some(Self::Hangup),
            4 => Some(Self::Deadline),
            5 => Some(Self::Conductor),
            6 => Some(Self::Cancel),
            7 => Some(Self::TuiClosed),
            _ => None,
        }
    }
}

/// Stop request shared by a signal listener and a running plan set. The
/// first request wins; later ones do not change the recorded cause.
#[derive(Debug, Clone, Default)]
pub struct PlanRunInterruptHandle {
    requested: Arc<AtomicU8>,
}

impl PlanRunInterruptHandle {
    /// Record a stop request. Returns `true` if it is the first one.
    pub fn request(&self, interrupt: PlanRunInterrupt) -> bool {
        self.requested
            .compare_exchange(0, interrupt.code(), Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    /// The first recorded stop request, if any.
    #[must_use]
    pub fn requested(&self) -> Option<PlanRunInterrupt> {
        PlanRunInterrupt::from_code(self.requested.load(Ordering::SeqCst))
    }

    /// Wait for a stop request and return it, looking every
    /// [`PLAN_WATCH_INTERVAL`] as the run's own loop does. A whole-plan check
    /// races its steps against it (bug-7b62ff).
    pub async fn stopped(&self) -> PlanRunInterrupt {
        loop {
            if let Some(interrupt) = self.requested() {
                return interrupt;
            }
            tokio::time::sleep(PLAN_WATCH_INTERVAL).await;
        }
    }
}

/// How long a stopping plan waits for its cancelled graph to settle before
/// its checkpoint is finalized without it.
const INTERRUPT_DRAIN_TIMEOUT: Duration = Duration::from_secs(3);

/// Grace period between the first signal and a forced exit.
const FORCED_EXIT_GRACE: Duration = Duration::from_secs(10);

/// How long a forced exit waits to mark the running plans' checkpoints
/// `interrupted` before it exits without them.
const FORCED_EXIT_CHECKPOINT_TIMEOUT: Duration = Duration::from_secs(2);

/// After [`INTERRUPT_DRAIN_TIMEOUT`], how long the attempts a stopping plan
/// asked to stop may take to settle with the usage they streamed before the
/// checkpoint is finalized without them (bug-2b1ddc).
const INTERRUPT_SETTLE_TIMEOUT: Duration = Duration::from_secs(2);

/// How long a plan waits for its attempts' cost and learning rows to reach
/// the disk before it returns, and the process exits.
const ROW_WRITES_TIMEOUT: Duration = Duration::from_secs(1);

/// Set while a CLI plan run handles SIGINT/SIGTERM itself.
static CLI_OWNS_TERMINATION_SIGNALS: AtomicBool = AtomicBool::new(false);

/// Set while a CLI plan run handles SIGHUP itself, which it does unless
/// SIGHUP was ignored when the run started, as under `nohup`.
static CLI_OWNS_HANGUP_SIGNAL: AtomicBool = AtomicBool::new(false);

/// Checkpoint manifests of the plans this process is running, which a forced
/// exit marks `interrupted` (bug-4641e3).
static RUNNING_PLAN_CHECKPOINTS: std::sync::Mutex<BTreeSet<PathBuf>> =
    std::sync::Mutex::new(BTreeSet::new());

/// Original stderr while it is redirected to the runner log for the TUI.
static REDIRECTED_STDERR_ORIGINAL: AtomicI32 = AtomicI32::new(-1);

/// Whether a CLI plan run currently owns SIGINT/SIGTERM. The binary's
/// process-wide SIGTERM handler defers to the run while this is `true`.
#[must_use]
pub fn plan_run_owns_termination_signals() -> bool {
    CLI_OWNS_TERMINATION_SIGNALS.load(Ordering::SeqCst)
}

/// Whether a CLI plan run currently owns SIGHUP, so its TUI must leave the
/// signal to the run.
fn plan_run_owns_hangup_signal() -> bool {
    CLI_OWNS_HANGUP_SIGNAL.load(Ordering::SeqCst)
}

/// Owns SIGINT/SIGTERM/SIGHUP for one CLI plan run; dropping it hands them
/// back.
#[derive(Debug)]
pub struct PlanRunSignalGuard {
    listener: tokio::task::JoinHandle<()>,
}

impl Drop for PlanRunSignalGuard {
    fn drop(&mut self) {
        self.listener.abort();
        CLI_OWNS_TERMINATION_SIGNALS.store(false, Ordering::SeqCst);
        CLI_OWNS_HANGUP_SIGNAL.store(false, Ordering::SeqCst);
    }
}

/// Route SIGINT/SIGTERM, and SIGHUP unless it is ignored (as under `nohup`),
/// to `interrupt` until the returned guard drops.
///
/// The first signal asks the run to stop: the running graph is cancelled,
/// its checkpoint is finalized as `interrupted`, the TUI restores the
/// terminal, and the run returns [`PlanRunInterrupt::exit_code`]. A second
/// signal, or [`FORCED_EXIT_GRACE`] after the first, forces the process out
/// with the same non-zero status, so shutdown can never hang; the forced
/// exit marks the running plans' checkpoints `interrupted` first.
///
/// Only the CLI installs this; library callers such as `roko serve` keep
/// their own signal handling.
#[cfg(unix)]
pub fn install_plan_run_signal_handlers(
    interrupt: PlanRunInterruptHandle,
) -> anyhow::Result<PlanRunSignalGuard> {
    use tokio::signal::unix::{SignalKind, signal};

    let mut sigint = signal(SignalKind::interrupt()).context("install SIGINT handler")?;
    let mut sigterm = signal(SignalKind::terminate()).context("install SIGTERM handler")?;
    // A hung-up terminal stops the run like SIGTERM (bug-4641e3), unless the
    // run was started to survive one.
    let mut sighup = if hangup_ignored() {
        None
    } else {
        Some(signal(SignalKind::hangup()).context("install SIGHUP handler")?)
    };
    CLI_OWNS_TERMINATION_SIGNALS.store(true, Ordering::SeqCst);
    CLI_OWNS_HANGUP_SIGNAL.store(sighup.is_some(), Ordering::SeqCst);
    let listener = tokio::spawn(async move {
        loop {
            let received = tokio::select! {
                Some(()) = sigint.recv() => PlanRunInterrupt::Interrupt,
                Some(()) = sigterm.recv() => PlanRunInterrupt::Terminate,
                Some(()) = next_signal(sighup.as_mut()) => PlanRunInterrupt::Hangup,
                else => return,
            };
            if interrupt.request(received) {
                tracing::warn!(
                    signal = received.label(),
                    "stopping plan run: cancelling in-flight work and finalizing checkpoints; \
                     signal again to force exit"
                );
                spawn_forced_exit_deadline(received);
            } else {
                force_exit(
                    interrupt.requested().unwrap_or(received),
                    "second signal received",
                );
            }
        }
    });
    Ok(PlanRunSignalGuard { listener })
}

/// Whether SIGHUP is ignored, as `nohup` leaves it for the command it starts.
#[cfg(unix)]
#[allow(unsafe_code)]
fn hangup_ignored() -> bool {
    // SAFETY: with a null new action, sigaction(2) only reads the current
    // disposition into `current`, a zero-initialized plain C struct.
    unsafe {
        let mut current: libc::sigaction = std::mem::zeroed();
        libc::sigaction(libc::SIGHUP, std::ptr::null(), &raw mut current) == 0
            && current.sa_sigaction == libc::SIG_IGN
    }
}

/// The next delivery of a signal the run may not have claimed; `None` at once
/// when it did not, which disables its `select!` arm.
#[cfg(unix)]
async fn next_signal(signal: Option<&mut tokio::signal::unix::Signal>) -> Option<()> {
    signal?.recv().await
}

/// No process signals to route on this platform.
#[cfg(not(unix))]
pub fn install_plan_run_signal_handlers(
    _interrupt: PlanRunInterruptHandle,
) -> anyhow::Result<PlanRunSignalGuard> {
    Ok(PlanRunSignalGuard {
        listener: tokio::spawn(async {}),
    })
}

#[cfg(unix)]
fn spawn_forced_exit_deadline(interrupt: PlanRunInterrupt) {
    // A plain thread, so the deadline fires even if the async runtime is
    // wedged by the work being stopped.
    let spawned = std::thread::Builder::new()
        .name("roko-plan-run-exit-deadline".to_string())
        .spawn(move || {
            std::thread::sleep(FORCED_EXIT_GRACE);
            force_exit(interrupt, "graceful shutdown deadline elapsed");
        });
    if let Err(error) = spawned {
        tracing::warn!(%error, "failed to arm the forced-exit deadline");
    }
}

/// Last resort: restore the terminal, kill every process this run spawned,
/// mark the running plans' checkpoints `interrupted` (bug-4641e3), and exit
/// with the signal's status.
#[cfg(unix)]
fn force_exit(interrupt: PlanRunInterrupt, reason: &str) -> ! {
    crate::tui::app::restore_terminal_for_forced_exit();
    restore_redirected_stderr();
    let killed = signal_processes(
        roko_agent::process::collect_descendants(std::process::id()),
        libc::SIGKILL,
    );
    let running: Vec<PathBuf> = running_plan_checkpoints().iter().cloned().collect();
    let by = interrupt.label();
    let interrupted = mark_checkpoints_interrupted(running, by, FORCED_EXIT_CHECKPOINT_TIMEOUT);
    tracing::error!(
        signal = interrupt.label(),
        reason,
        killed,
        checkpoints_interrupted = interrupted,
        "forced plan run exit; the running plans' checkpoints were marked `interrupted`"
    );
    eprintln!(
        "roko: forced exit after {} ({reason}); killed {killed} child process(es), marked \
         {interrupted} checkpoint(s) interrupted; resume with `roko plan run <plans-dir> \
         --resume-plan`",
        interrupt.label()
    );
    std::process::exit(interrupt.exit_code());
}

/// The checkpoint manifests in [`RUNNING_PLAN_CHECKPOINTS`].
fn running_plan_checkpoints() -> std::sync::MutexGuard<'static, BTreeSet<PathBuf>> {
    RUNNING_PLAN_CHECKPOINTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Lists a running plan's checkpoint for a forced exit until it drops, after
/// the plan has written its own terminal status.
struct RunningPlanCheckpoint(PathBuf);

impl RunningPlanCheckpoint {
    fn register(manifest: &Path) -> Self {
        running_plan_checkpoints().insert(manifest.to_path_buf());
        Self(manifest.to_path_buf())
    }
}

impl Drop for RunningPlanCheckpoint {
    fn drop(&mut self) {
        running_plan_checkpoints().remove(&self.0);
    }
}

/// Mark each checkpoint in `manifests` that still reads `running` as
/// `interrupted` by the stop request `by`, on a thread of its own so a stuck
/// disk cannot hold a forced exit past `timeout`. Returns how many were
/// marked in time.
fn mark_checkpoints_interrupted(
    manifests: Vec<PathBuf>,
    by: &'static str,
    timeout: Duration,
) -> usize {
    use crate::graph_checkpoint::mark_running_checkpoint_interrupted;

    if manifests.is_empty() {
        return 0;
    }
    let (marked_tx, marked_rx) = std::sync::mpsc::channel();
    let spawned = std::thread::Builder::new()
        .name("roko-plan-run-exit-checkpoints".to_string())
        .spawn(move || {
            for manifest in manifests {
                match mark_running_checkpoint_interrupted(&manifest, by) {
                    Ok(marked) => {
                        let _ = marked_tx.send(marked);
                    }
                    Err(error) => tracing::warn!(
                        manifest = %manifest.display(),
                        error = %format!("{error:#}"),
                        "could not mark a running plan's checkpoint interrupted"
                    ),
                }
            }
        });
    if let Err(error) = spawned {
        tracing::warn!(%error, "could not mark the running plans' checkpoints interrupted");
        return 0;
    }
    let deadline = Instant::now() + timeout;
    std::iter::from_fn(|| {
        marked_rx
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .ok()
    })
    .filter(|marked| *marked)
    .count()
}

/// This run's agent processes that are still descendants of this process,
/// plus all of their descendants. Checking descendancy avoids signalling a PID
/// that was recycled after its agent exited.
///
/// The run's agents are those registered in this thread's spawn scope: `roko
/// serve` runs each plan on a thread it scopes, so stopping one run leaves the
/// plan generation, revision and chat agents the server runs beside it alone
/// (find-65ff6b). A CLI run has no scope and owns every unscoped agent.
#[cfg(unix)]
fn live_agent_process_trees() -> Vec<u32> {
    let ours = roko_agent::process::collect_descendants(std::process::id())
        .into_iter()
        .collect::<std::collections::HashSet<_>>();
    let scope = roko_agent::process::current_spawn_scope();
    let mut targets = Vec::new();
    for pid in roko_agent::process::registered_pids_in_scope(scope) {
        if ours.contains(&pid) {
            targets.extend(roko_agent::process::collect_descendants(pid));
            targets.push(pid);
        }
    }
    targets
}

/// Send `signal` to each PID; returns how many accepted it.
#[cfg(unix)]
#[allow(unsafe_code)]
fn signal_processes(pids: impl IntoIterator<Item = u32>, signal: libc::c_int) -> usize {
    pids.into_iter()
        .filter_map(|pid| libc::pid_t::try_from(pid).ok().filter(|pid| *pid > 0))
        // SAFETY: kill(2) only delivers a signal; each PID is a positive
        // descendant PID observed just before.
        .filter(|pid| unsafe { libc::kill(*pid, signal) } == 0)
        .count()
}

/// Ask in-flight agents to stop so a cancelled graph settles promptly.
fn terminate_in_flight_agents() -> usize {
    #[cfg(unix)]
    {
        signal_processes(live_agent_process_trees(), libc::SIGTERM)
    }
    #[cfg(not(unix))]
    {
        0
    }
}

/// Kill in-flight agents that ignored [`terminate_in_flight_agents`].
fn kill_in_flight_agents() -> usize {
    #[cfg(unix)]
    {
        signal_processes(live_agent_process_trees(), libc::SIGKILL)
    }
    #[cfg(not(unix))]
    {
        0
    }
}

/// Send stderr to the runner log while the TUI owns the terminal.
fn redirect_stderr_to(log_path: &Path) {
    #[cfg(unix)]
    if let Ok(log_file) = std::fs::File::create(log_path) {
        use std::os::unix::io::AsRawFd;
        #[allow(unsafe_code)]
        // SAFETY: dup/dup2 on process-owned descriptors; the duplicate of the
        // original stderr is kept for `restore_redirected_stderr`.
        unsafe {
            let original = libc::dup(2);
            if libc::dup2(log_file.as_raw_fd(), 2) >= 0 && original >= 0 {
                let previous = REDIRECTED_STDERR_ORIGINAL.swap(original, Ordering::SeqCst);
                if previous >= 0 {
                    libc::close(previous);
                }
            } else if original >= 0 {
                libc::close(original);
            }
        }
    }
    #[cfg(not(unix))]
    let _ = log_path;
}

/// Point stderr back at the terminal so errors after the TUI are visible.
fn restore_redirected_stderr() {
    let original = REDIRECTED_STDERR_ORIGINAL.swap(-1, Ordering::SeqCst);
    #[cfg(unix)]
    if original >= 0 {
        #[allow(unsafe_code)]
        // SAFETY: `original` is the descriptor saved by `redirect_stderr_to`.
        unsafe {
            libc::dup2(original, 2);
            libc::close(original);
        }
    }
    #[cfg(not(unix))]
    let _ = original;
}

/// The connected TUI thread and the channel that asks it to exit.
///
/// Dropping the session (including on an early `?` return) stops the TUI,
/// joins it so the terminal is restored, and points stderr back at the
/// terminal before the caller reports anything.
struct TuiSession {
    shutdown: Option<std::sync::mpsc::Sender<()>>,
    handle: Option<std::thread::JoinHandle<anyhow::Result<()>>>,
}

impl TuiSession {
    /// `true` once, when the operator closed the TUI while the run still
    /// owned it. A TUI that failed rather than quit is logged and ignored.
    fn closed_by_operator(&mut self) -> bool {
        if self.shutdown.is_none()
            || !self
                .handle
                .as_ref()
                .is_some_and(std::thread::JoinHandle::is_finished)
        {
            return false;
        }
        match self.handle.take().map(std::thread::JoinHandle::join) {
            Some(Ok(Ok(()))) => true,
            Some(Ok(Err(error))) => {
                tracing::error!(%error, "Graph Engine TUI exited with error; continuing without it");
                false
            }
            Some(Err(_)) => {
                tracing::error!("Graph Engine TUI thread panicked; continuing without it");
                false
            }
            None => false,
        }
    }

    /// Ask the TUI to exit and wait until it has restored the terminal.
    fn stop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        join_approval_tui_thread(self.handle.take());
        restore_redirected_stderr();
    }
}

impl Drop for TuiSession {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The recorded stop request, or a new `TuiClosed` when the operator just
/// closed the TUI mid-run (no terminal output would remain otherwise).
fn pending_interrupt(
    interrupt: &PlanRunInterruptHandle,
    tui: &mut Option<TuiSession>,
) -> Option<PlanRunInterrupt> {
    if interrupt.requested().is_none() && tui.as_mut().is_some_and(TuiSession::closed_by_operator) {
        tracing::warn!("operator closed the TUI mid-run; stopping the plan run");
        interrupt.request(PlanRunInterrupt::TuiClosed);
    }
    interrupt.requested()
}

/// What a plan's checkpoint names as the stop of its run (gap-fab2cc): the
/// stop request, when the plan ended interrupted by it.
fn stop_cause(
    outcome: PlanOutcome,
    interrupted_by: Option<PlanRunInterrupt>,
) -> Option<&'static str> {
    interrupted_by
        .filter(|_| outcome == PlanOutcome::Interrupted)
        .map(PlanRunInterrupt::label)
}

/// Terminal checkpoint status of a plan that ran and ended with `outcome`.
const fn plan_checkpoint_status(outcome: PlanOutcome) -> GraphCheckpointStatus {
    match outcome {
        PlanOutcome::Succeeded => GraphCheckpointStatus::Succeeded,
        PlanOutcome::Unverified => GraphCheckpointStatus::Unverified,
        PlanOutcome::Interrupted => GraphCheckpointStatus::Interrupted,
        PlanOutcome::Cancelled => GraphCheckpointStatus::Cancelled,
        PlanOutcome::Failed | PlanOutcome::Blocked => GraphCheckpointStatus::Failed,
    }
}

/// Resolve the effective per-plan USD ceiling from CLI flags and config.
///
/// Priority order (highest to lowest):
/// 1. `no_budget = true` → `0.0` (unlimited — no enforcement)
/// 2. `budget_override = Some(amount)` → `amount.max(0.0)` (explicit CLI ceiling)
/// 3. `config_max_plan_usd` (from `roko.toml [budget].max_plan_usd`)
///
/// Returns `(effective_ceiling, bypass_block)` where `bypass_block` is `true`
/// only under `--no-budget`, which also lets a spent day dispatch. An explicit
/// CLI ceiling is a hard cap, as a configured one is: once the plan has spent
/// it, no further task starts (gap-d31457). `--budget-override 0` removes the
/// plan ceiling but keeps the per-task and daily ones.
pub fn resolve_budget_ceiling(
    budget_override: Option<f64>,
    no_budget: bool,
    config_max_plan_usd: f64,
) -> (f64, bool) {
    if no_budget {
        (0.0, true)
    } else if let Some(ceiling) = budget_override {
        (ceiling.max(0.0), false)
    } else {
        (config_max_plan_usd, false)
    }
}

/// Check that the configured provider for a model is reachable.
///
/// Ported from `commands::util::preflight_provider_for_model` so library
/// callers do not need to depend on the binary-only `commands` module.
fn preflight_provider_for_model(
    config: &roko_core::config::schema::RokoConfig,
    model_key: &str,
) -> anyhow::Result<()> {
    let model = config.models.get(model_key);
    if model.is_none() {
        if let Some(builtin) = roko_core::config::model_registry::builtin_model(model_key) {
            if std::env::var(builtin.api_key_env).is_ok() {
                return Ok(());
            }
            anyhow::bail!(
                "model '{}' requires {} but it is not set.\n  hint: export {}=<your-key>",
                model_key,
                builtin.api_key_env,
                builtin.api_key_env
            );
        }
        anyhow::bail!(
            "model '{}' not found in config.\n  hint: run `roko config models list` to see configured models, or add a [[models]] entry in roko.toml",
            model_key
        );
    }
    let model = model.expect("model should be Some after config lookup loop");
    let provider_name = &model.provider;
    let provider = config.providers.get(provider_name).ok_or_else(|| {
        anyhow!(
            "provider '{}' (for model '{}') not found in config",
            provider_name,
            model_key
        )
    })?;

    if let Some(ref env_var) = provider.api_key_env
        && !env_var.trim().is_empty()
    {
        match std::env::var(env_var) {
            Ok(val) if val.is_empty() => {
                anyhow::bail!(
                    "provider '{}' requires {} but it is empty.\n  hint: export {}=<your-key>",
                    provider_name,
                    env_var,
                    env_var
                );
            }
            Err(_) => {
                anyhow::bail!(
                    "provider '{}' requires {} but it is not set.\n  hint: export {}=<your-key>",
                    provider_name,
                    env_var,
                    env_var
                );
            }
            Ok(_) => {}
        }
    }

    Ok(())
}

// ── Public API ────────────────────────────────────────────────────────────

/// Filter `plans` to those listed in `only`. Returns an error for any id in
/// `only` that does not exist in `plans`. When `only` is `None`, all plans
/// are returned unchanged.
fn filter_only_plans(
    mut plans: Vec<crate::runner::plan_loader::Plan>,
    only: Option<&[String]>,
    plans_dir: &Path,
) -> anyhow::Result<Vec<crate::runner::plan_loader::Plan>> {
    let Some(only) = only else {
        return Ok(plans);
    };
    let mut filtered = Vec::with_capacity(only.len());
    for id in only {
        let pos = plans.iter().position(|p| p.id == *id).ok_or_else(|| {
            anyhow!(
                "plan '{id}' not found in {}; available: {}",
                plans_dir.display(),
                plans
                    .iter()
                    .map(|p| p.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;
        filtered.push(plans.remove(pos));
    }
    Ok(filtered)
}

/// Return the ordered list of plan ids for `plans_dir`, optionally filtered
/// to `only_plans`.
///
/// Ids in `only_plans` that are not found in the directory produce an error;
/// `None` means "all discovered plans in topological order". This is the same
/// ordering the run uses — callers may use it to build a plan-set member list
/// before the run starts.
pub fn compute_plan_run_order(
    workdir: &Path,
    plans_dir: &Path,
    only_plans: Option<&[String]>,
) -> anyhow::Result<Vec<String>> {
    let plans = crate::runner::plan_loader::load_plans(plans_dir)?;
    let plans = filter_only_plans(plans, only_plans, plans_dir)?;
    let plan_order = super::plan_set::plan_set_order(workdir, plans_dir, &plans)?;
    Ok(plan_order.order)
}

/// Parameters for running plans through the Graph Engine.
///
/// Mirrors the arguments of the binary-side `cmd_plan_run_engine`, but
/// replaces the `cli: &Cli` reference with plain boolean fields for the
/// flags the engine actually inspects.
#[derive(Debug)]
pub struct GraphPlanRunParams {
    pub plans_dir: PathBuf,
    pub workdir: PathBuf,
    /// Suppress all non-error output.
    pub quiet: bool,
    /// Emit structured JSON output instead of human-readable text.
    pub json: bool,
    pub resume_plan: Option<PathBuf>,
    pub fresh: bool,
    pub force_resume: bool,
    pub max_retries: Option<u32>,
    pub max_tasks: usize,
    pub budget_override: Option<f64>,
    pub no_budget: bool,
    pub cli_model_override: Option<String>,
    pub dangerously_skip_permissions: bool,
    pub log_file: Option<PathBuf>,
    pub worktree_per_task: bool,
    /// Whether `--worktree-per-task` asked for [`Self::worktree_per_task`],
    /// rather than `[runner] worktree_per_task` or its default (gap-4ec59f).
    /// The run says which when it turns isolation on.
    pub worktree_per_task_explicit: bool,
    pub rich_topology: bool,
    /// With `worktree_per_task`: once every plan is delivered into the run's
    /// batch branch, promote the batch into this branch and tag the run
    /// `roko/run/<run-id>` (spec-f830c4). Never pushes.
    pub promote: Option<String>,
    pub no_tui: bool,
    /// Hub that receives every dashboard event of this run (plan set,
    /// lifecycle, agent, and gate events). `None` creates a private
    /// in-process hub; `roko serve` passes its own so API/SSE clients see
    /// server-started runs.
    pub state_hub: Option<crate::state_hub::SharedStateHub>,
    /// Stop request for this run (see [`install_plan_run_signal_handlers`]).
    /// A requested run cancels the running graph, finalizes that plan's
    /// checkpoint as `interrupted`, leaves later plans unstarted, and returns
    /// [`PlanRunInterrupt::exit_code`]. `None` stops early only when the
    /// operator closes the TUI.
    pub interrupt: Option<PlanRunInterruptHandle>,
    /// How many independent plans of the selected set may run at once.
    /// `None` uses `[conductor] max_parallel_plans` (default 1, which runs
    /// the set one plan at a time in execution order). A per-run override:
    /// it is never written into the config, which every checkpoint
    /// fingerprint includes.
    pub max_parallel_plans: Option<usize>,
    /// Start no further plans after the first plan fails. Plans already
    /// running finish; the rest end blocked. `false` keeps running every
    /// plan whose prerequisites succeeded.
    pub fail_fast: bool,
    /// Restrict execution to these plan ids; `None` means run the whole
    /// discovered set. Any id listed here that is not present in `plans_dir`
    /// causes an immediate error.
    pub only_plans: Option<Vec<String>>,
    /// Which live events the dispatcher forwards to the TUI while an agent
    /// runs.  `ToolSteps` (safe default) forwards only the tool name and
    /// target; `Trusted` additionally forwards unscreened text and tool
    /// results and should only be used on loopback-bound servers or in
    /// standalone CLI runs where there is no remote attack surface.
    pub live_agent_output: crate::graph_task_dispatch::LiveAgentOutput,
    /// Start the run even when the workdir has less free disk than
    /// `[resources] min_free_disk_mb` (`plan run --force`, reg-7cf6f9).
    pub force_disk_check: bool,
    /// Reasoning effort of this run's dispatches in place of `[agent]
    /// default_effort` (`roko run --effort`, gap-9980c6); `None` keeps the
    /// config's.
    pub effort: Option<String>,
    /// Pick models without the cascade router (`roko run --no-cascade`,
    /// gap-9980c6): the routing ladder, else the default model, routes each
    /// task. The run's outcomes still teach the router.
    pub no_cascade: bool,
    /// Hold learned state fixed for this run alone (`roko plan run
    /// --frozen-learning`, decision 2218): the run's config reads
    /// `[learning] frozen = true` whatever `roko.toml` says.
    pub frozen_learning: bool,
    /// Maximize mode for this run alone (`roko plan run --no-holdout`,
    /// decision 4115): the run's config reads `[experiments] maximize =
    /// true`, so no loop is withheld and no route explores.
    pub no_holdout: bool,
    /// Registry that counts this run's verify verdicts and durations
    /// (`roko_gate_verdicts_total`, `roko_gate_duration_seconds`) beside the
    /// tracing fields: serve passes the one `/metrics` renders (gap-d8c39a).
    /// `None` keeps the tracing fields only.
    pub metrics: Option<Arc<roko_core::obs::metrics::MetricRegistry>>,
    /// The least strict policy every task of the run gets for tool calls
    /// that act on the outside world (9131), whatever its plan's `[meta]
    /// outbound` or its domain says: [`outbound_floor`] of where the run's
    /// request came from. A chat host's run stages them (decision 9107), so a
    /// plan it submits directly holds them for approval as the plan `roko
    /// run` writes for it does (gap-1a4563). `None` leaves the plan and domain
    /// to decide.
    pub outbound_floor: Option<roko_core::tool::OutboundPolicy>,
}

/// The outbound floor ([`GraphPlanRunParams::outbound_floor`]) of a run that
/// `origin` started: `stage` for a chat host's run (decision 9107), none
/// for the CLI's or the HTTP API's.
#[must_use]
pub const fn outbound_floor(
    origin: &roko_serve::runtime::RunOrigin,
) -> Option<roko_core::tool::OutboundPolicy> {
    if origin.is_chat() {
        Some(roko_core::tool::OutboundPolicy::Stage)
    } else {
        None
    }
}

/// Execute plans via the Graph Engine path.
///
/// Loads plans using the Runner v2 plan_loader, converts each to a Graph
/// via `roko_graph::convert::plan_to_graph` (default) or
/// `roko_graph::topology::ProductionPlanTopology` (when `rich_topology` is
/// true), and runs them through the GraphEngine with the default cell registry.
pub async fn run_graph_plan(params: GraphPlanRunParams) -> anyhow::Result<i32> {
    run_graph_plan_in_run(params, None).await
}

/// [`run_graph_plan`] for a caller whose run already has an id: a single
/// plan's fresh checkpoint takes `run_id`, so the run's attempt records and
/// manifest land in the caller's own `.roko/runs/<run_id>/` (`roko run`,
/// bug-ccc7c4). A resumed checkpoint keeps its recorded run, a set of several
/// plans mints one run per plan, and a `--log-file` run mints its own.
pub async fn run_graph_plan_in_run(
    params: GraphPlanRunParams,
    run_id: Option<String>,
) -> anyhow::Result<i32> {
    // `--log-file`: delegates entirely to `event_log::run_recorded`, which is
    // responsible for publishing its own terminal events.
    if params.log_file.is_some() {
        return super::event_log::run_recorded(params).await;
    }
    let observed_run_id = super::event_log::graph_run_id(run_id.as_deref());
    run_graph_plan_observed(params, run_id, observed_run_id).await
}

/// [`run_graph_plan_in_run`] without a `log_file`, whose `status.json` and
/// workspace event log name the run `observed_run_id`.
pub(crate) async fn run_graph_plan_observed(
    params: GraphPlanRunParams,
    run_id: Option<String>,
    observed_run_id: String,
) -> anyhow::Result<i32> {
    // Resolve the hub early so `DashboardEvent::RunCompleted` is published on
    // every exit path, including the early `?` returns in the body (plan load,
    // config validation, provider preflight, extension start-up, checkpoint).
    // The body, `status.json` and `RunCompleted` share it; a caller without a
    // hub gets a private one.
    let mut params = params;
    let hub = params
        .state_hub
        .get_or_insert_with(crate::state_hub::shared_state_hub)
        .clone();
    let hub_sender = hub.sender();
    // `.roko/state/status.json` shows the live run to `roko status` and the
    // evidence collector's status sampling (gap-568056).
    let status = crate::runner::status_file::GraphStatusWriter::spawn(
        &hub,
        RokoLayout::for_project(&params.workdir).state_dir(),
        observed_run_id.clone(),
    );
    // `.roko/events.jsonl` and the run's index show it to a dashboard in
    // another terminal, serve and `roko doctor` (bug-230de6).
    let completed_run_id = observed_run_id.clone();
    let events = super::event_log::WorkspaceEventLog::spawn(&hub, &params.workdir, observed_run_id);

    // Ensure a consistent interrupt handle: if the caller passed None, create
    // one now and put it back so the body and this wrapper share the same
    // Arc<AtomicU8>.  Clone *after* the insert so both ends observe the same
    // stop flag.
    if params.interrupt.is_none() {
        params.interrupt = Some(PlanRunInterruptHandle::default());
    }
    let interrupt_handle = params
        .interrupt
        .as_ref()
        .expect("interrupt handle was just set")
        .clone();
    let run_start = Instant::now();

    let result = run_graph_plan_body(params, run_id).await;

    let outcome = match &result {
        // Its plans ran no check, and none failed (bug-db89d4).
        Ok(exit) if exit.unverified => "unverified",
        Ok(exit) => graph_run_outcome(Some(exit.code), &interrupt_handle),
        Err(_) => graph_run_outcome(None, &interrupt_handle),
    };
    hub_sender.publish(roko_core::DashboardEvent::RunCompleted {
        run_id: Some(completed_run_id),
        outcome: outcome.to_string(),
        duration_ms: run_start.elapsed().as_millis() as u64,
        cleanup_degraded: false,
        surviving_agent_ids: vec![],
        surviving_agent_pids: vec![],
    });
    status.finish(outcome).await;
    if let Some(events) = events {
        events.finish().await;
    }

    result.map(|exit| exit.code)
}

/// How the body of a plan run ended: its exit code, and whether it failed
/// only because a plan ended unverified (bug-db89d4).
struct RunExit {
    code: i32,
    /// Every plan succeeded or ended unverified, at least one unverified,
    /// and nothing stopped the run.
    unverified: bool,
}

impl From<i32> for RunExit {
    fn from(code: i32) -> Self {
        Self {
            code,
            unverified: false,
        }
    }
}

/// Map the run's exit code (`None`: it ended in an error) to an outcome
/// label for `DashboardEvent::RunCompleted`.
///
/// - `"succeeded"`: exit code `EXIT_SUCCESS`.
/// - `"cancelled"`: a stop request whose exit code matches the returned code
///   (TUI cancel routes through the same handle via `PlanRunInterrupt`).
/// - `"failed"`: an error and every other non-zero exit code.
///
/// A run whose plans ended unverified is `"unverified"`, decided before this.
fn graph_run_outcome(code: Option<i32>, interrupt: &PlanRunInterruptHandle) -> &'static str {
    match code {
        Some(EXIT_SUCCESS) => "succeeded",
        Some(code) => {
            if interrupt
                .requested()
                .is_some_and(|req| req.exit_code() == code)
            {
                "cancelled"
            } else {
                "failed"
            }
        }
        None => "failed",
    }
}

/// Attach the run's tool observability to `factory` (find-f489db). Every tool
/// call roko's own tool loops make then leaves a scrubbed admit and result
/// pair in `.roko/tool_audit.jsonl` and a closed trace under `.roko/traces/`,
/// both under `workdir`; no tool metrics are written (backlog 2123). The
/// audit scrubs with the process's secret scrubber, which holds the
/// configured secrets, or the built-in patterns when none is installed. The
/// audit is observability, not a gate: when its log cannot be opened, the run
/// goes on without it.
pub(crate) async fn attach_tool_observability(
    factory: crate::dispatch::SharedAgentFactory,
    workdir: &Path,
) -> crate::dispatch::SharedAgentFactory {
    let sinks = roko_fs::FsObservabilitySinks::for_workdir(workdir);
    let factory = factory.with_observability_sinks(sinks);
    match roko_fs::tool_audit::ToolAuditLog::open(workdir).await {
        Ok(log) => {
            let scrubber = roko_core::obs::secret_scrubber()
                .unwrap_or_else(|| roko_fs::observability::RunScrubber::build(&[]));
            factory.with_tool_audit(Arc::new(roko_fs::tool_audit::ScrubAuditAdapter::new(
                Arc::new(log),
                scrubber,
            )))
        }
        Err(error) => {
            tracing::warn!(%error, "tool audit log unavailable; tool calls are not audited");
            factory
        }
    }
}

async fn run_graph_plan_body(
    params: GraphPlanRunParams,
    run_id: Option<String>,
) -> anyhow::Result<RunExit> {
    use roko_graph::cells::TaskDispatcher;

    let GraphPlanRunParams {
        plans_dir,
        workdir,
        quiet,
        json,
        resume_plan,
        fresh,
        force_resume,
        max_retries,
        max_tasks,
        budget_override,
        no_budget,
        cli_model_override,
        dangerously_skip_permissions,
        // A `--log-file` run reaches this body through
        // `event_log::run_recorded`, which records it and clears the field.
        log_file: _,
        worktree_per_task,
        worktree_per_task_explicit,
        rich_topology,
        promote,
        no_tui,
        state_hub,
        interrupt,
        max_parallel_plans,
        fail_fast,
        only_plans,
        live_agent_output,
        force_disk_check,
        effort,
        no_cascade,
        frozen_learning,
        no_holdout,
        metrics,
        outbound_floor,
    } = params;
    let interrupt = interrupt.unwrap_or_default();
    // FAST lane (`./dev.sh fast`): stop the run when its deadline elapses.
    let _fast_deadline = super::fast_lane::arm_plan_deadline(&interrupt);

    let plans_dir: &Path = &plans_dir;
    let workdir: &Path = &workdir;
    let resume_plan: Option<PathBuf> = resume_plan;
    // Each task's plan gate judges the worktree its attempt ran in, never the
    // shared working tree (bug-50caf2), so the rich topology needs them.
    if rich_topology && !worktree_per_task {
        anyhow::bail!(
            "--rich-topology needs --worktree-per-task: each task's plan gate judges the \
             worktree its attempt ran in, never the shared working tree"
        );
    }

    let run_start = std::time::Instant::now();
    let plans = crate::runner::plan_loader::load_plans(plans_dir)?;
    // Apply only_plans filter: keep exactly the named ids, in name order, and
    // fail immediately if any listed id does not exist in the directory.
    let mut plans = filter_only_plans(plans, only_plans.as_deref(), plans_dir)?;
    // Pin planner-written acceptance tests outside the working tree and run
    // them first, before anything is dispatched (gap-d14a43).
    crate::task_accept::pin_plans(&mut plans, workdir)?;
    // A plan that holds each verified task for a person's approval
    // (gap-0d64d5) needs per-task worktrees: the hold sits between a task's
    // verified attempt and its acceptance onto the plan branch. The rich
    // topology's plan gate accepts on its own, so it cannot hold.
    let held_plans: Vec<String> = plans
        .iter()
        .filter(|plan| plan.tasks.meta.holds_each_task_for_approval())
        .map(|plan| plan.id.clone())
        .collect();
    if !held_plans.is_empty() && (!worktree_per_task || rich_topology) {
        anyhow::bail!(
            "plan(s) {} hold each task for approval ([meta] approval = \"per_task\"), which \
             needs --worktree-per-task and the default topology",
            held_plans.join(", ")
        );
    }
    // Validate the complete selected set before initializing extensions or
    // launching a provider. This makes missing, incomplete, and cyclic
    // cross-plan dependencies fail closed without partially executing the
    // batch.
    let plan_order = super::plan_set::plan_set_order(workdir, plans_dir, &plans)?;
    let plan_execution_order = plan_order.order.clone();

    // Build the same provider/config/extension foundation used by runner-v2.
    // Graph tasks are Activities, but they must still share rate limits,
    // health state, prompt context, MCP/plugin handlers, and safety contracts.
    let mut roko_config = roko_core::config::loader::load_config_validated(workdir)
        .map_err(|error| anyhow!("load Graph runtime config: {error}"))?
        .into_config();
    // `--effort` sets this run's reasoning effort (gap-9980c6).
    if let Some(effort) = effort {
        roko_config.agent.default_effort = effort;
    }
    // `--frozen-learning` and `--no-holdout` apply to this run alone, before
    // the manifest, the feedback facade and the dispatcher are built, so
    // every reader sees one value.
    apply_run_switches(&mut roko_config, frozen_learning, no_holdout);
    // Every check the run starts builds with a job cap at a lower priority
    // (bug-b3849c): `[gates] cargo_jobs` and `[gates] nice`.
    let gate_limits = roko_config.gates.gate_limits();
    roko_core::child_env::set_gate_limits(gate_limits);
    tracing::info!(
        cargo_jobs = ?gate_limits.cargo_jobs,
        nice = gate_limits.nice,
        cargo_build_jobs_from_env = ?std::env::var("CARGO_BUILD_JOBS").ok(),
        "gate commands run with a job cap at a lower priority"
    );
    // ... and without the person's credentials (bug-3c4600): `[gates]
    // env_passthrough` and `strip_env`, and an empty GH_CONFIG_DIR for this
    // run, removed when it ends. The first check logs what it left out.
    let _check_run = roko_core::child_env::begin_check_run(roko_config.gates.check_env_settings());
    if roko_config.learning.frozen {
        tracing::info!(
            "learning is frozen for this run: it reads learned state and writes none \
             (decision 2218)"
        );
    }
    if roko_config.experiments.maximize {
        tracing::info!(
            "maximize mode: no learning loop is withheld and no route explores in this run, \
             though every decision is logged (decision 4115)"
        );
    }
    roko_core::config::loader::normalize_and_validate_dispatch_models(&mut roko_config)
        .context("validate model configuration before Graph dispatch")?;
    // A run refuses to start on a nearly full disk (reg-7cf6f9).
    super::disk_admission::check_free_disk(workdir, &roko_config.resources, force_disk_check)?;
    // 3231: the plan-load spec gate, which every run passes (plan run,
    // serve, ACP, roko run): score each task, prove its shell checks red on
    // the base when `[spec_quality] red_on_base` is on, and refuse the plans
    // before any dispatch when a task is blocked (decision 3201). Each
    // plan's run records the decisions before its first task starts.
    let spec_files: Vec<PathBuf> = plans
        .iter()
        .map(|plan| plan.dir.join("tasks.toml"))
        .collect();
    let spec_gate =
        match crate::spec_gate::gate_plans(&spec_files, workdir, &roko_config.spec_quality) {
            Ok(report) => report,
            Err(interrupted) => {
                tracing::error!("{interrupted}");
                return Ok(RunExit::from(128 + interrupted.signal));
            }
        };
    if spec_gate.blocks() {
        crate::spec_gate::log_blocked(&spec_gate);
        // The run's event log and SSE show the refusal too.
        if let Some(hub) = &state_hub {
            let sender = hub.sender();
            for event in crate::spec_gate::blocked_events(&spec_gate) {
                sender.publish(event);
            }
        }
        return Ok(RunExit::from(1));
    }

    // Merge CLI flag with config (same logic as runner-v2).
    let dangerously_skip_permissions =
        dangerously_skip_permissions || roko_config.runner.dangerously_skip_permissions;

    // How many independent plans may run at once: the per-run override, else
    // `[conductor] max_parallel_plans`. Never written back into the config,
    // which every checkpoint fingerprint includes.
    // Per-task worktrees run plans side by side too (backlog 3104): each
    // plan's attempts fork from its own branch, and the plans are delivered
    // into the run's batch branch one at a time, each merged into the tip
    // the last one left.
    let max_parallel_plans = max_parallel_plans
        .unwrap_or(roko_config.conductor.max_parallel_plans)
        .max(1);

    let (plan_budget_ceiling, budget_bypassed) = resolve_budget_ceiling(
        budget_override,
        no_budget,
        f64::from(roko_config.budget.max_plan_usd),
    );
    if !roko_config.agent.default_model.trim().is_empty() {
        preflight_provider_for_model(&roko_config, &roko_config.agent.default_model)?;
    }
    let graph_run_config = crate::runner::RunConfig::from_roko_config(
        workdir.to_path_buf(),
        plans_dir.to_path_buf(),
        roko_config.clone(),
    );
    crate::runner::extension_loader::initialize_extensions(
        graph_run_config.extension_chain.as_ref(),
    )
    .await?;

    let roko_config = Arc::new(roko_config);
    // S01 P0-2: the harness build and config fingerprint every checkpoint
    // run's manifest records.
    let mut run_manifests = super::run_manifest::RunManifests::capture(workdir, &roko_config);
    let prompt_cache = Arc::new(crate::dispatch::PromptCache::load(workdir));
    // `--no-cascade`: the router picks no model; the run's feedback still
    // trains it.
    let routing_cascade = if no_cascade {
        None
    } else {
        graph_run_config.cascade_router.clone()
    };
    let health_registry = roko_learn::provider_health::ProviderHealthRegistry::load_or_new(
        &RokoLayout::for_project(workdir)
            .learn_dir()
            .join("provider-health.json"),
    );
    // A run that serve hosts counts its provider failures on `/metrics`
    // (gap-a95898).
    if let Some(metrics) = &metrics {
        health_registry.attach_metrics(Arc::clone(metrics));
    }
    let shared_factory = crate::dispatch::SharedAgentFactory::new(
        Arc::clone(&roko_config),
        roko_config.agent.mcp_config.as_ref(),
        routing_cascade,
        Some(prompt_cache),
    )
    .await
    .with_health_registry(Arc::new(health_registry))
    .with_error_patterns_from_disk(workdir)
    .with_knowledge_routing(workdir);
    let mut shared_factory = attach_tool_observability(shared_factory, workdir).await;
    // Each plan registers its run's safety provenance sink here (gap-ff95f5).
    let provenance_sinks = crate::safety_provenance::ProvenanceSinks::default();
    shared_factory = shared_factory.with_provenance_sinks(provenance_sinks.clone());
    let plugin_catalog = crate::runner::extension_loader::resolve_plugin_tool_catalog(
        workdir,
        &roko_config.agent.extensions,
        &[],
    )?;
    if !plugin_catalog.plugin_tools().is_empty() {
        shared_factory = shared_factory.with_local_tool_runtime(plugin_catalog.local_runtime());
    }
    // Decision 1119 (3-A): give each API rung of the model ladder one
    // tool-use call at most once a day, and skip a rung that cannot do agent
    // work (backlog 1121). FAST and `--no-budget` runs never probe.
    let probe_rungs = roko_config.routing.ladder.probe
        && !no_budget
        && super::fast_lane::FastAttemptBounds::from_env(workdir).is_none();
    if probe_rungs && let Some(ladder) = shared_factory.dispatcher().routing_ladder().cloned() {
        let failed =
            crate::dispatch::rung_probe::probe_ladder(&roko_config, &ladder, workdir).await;
        shared_factory = shared_factory.skip_failed_rungs(&failed);
    }
    // A chat host's run holds its tasks' outbound tool calls for approval
    // (decision 9107), which only roko's own tool loop can do: its ladder
    // keeps only the rungs that run there, so no task starts or climbs onto a
    // CLI agent that would be refused at dispatch (gap-cf427e).
    if outbound_floor.is_some_and(|floor| floor != roko_core::tool::OutboundPolicy::Allow) {
        shared_factory = shared_factory.keep_tool_loop_rungs();
    }
    let shared_factory = Arc::new(shared_factory);
    if dangerously_skip_permissions {
        tracing::warn!(
            "running Graph Engine with --dangerously-skip-permissions: agents will execute tools without approval"
        );
    }

    // ── Learning/feedback subsystem wiring ─────────────────────────────
    //
    // The learning sinks and stores every settled task attempt feeds; the
    // learning wiring census inspects the same object graph.
    let graph_layout = RokoLayout::for_project(workdir);
    let graph_learn_dir = graph_layout.learn_dir();
    // Routing outcomes are journaled in the learning WAL until the run saves
    // the router at its end, so a crash keeps them (bug-dfb28f).
    let cascade_journal = graph_run_config.cascade_router.as_ref().map(|_| {
        Arc::new(
            roko_learn::model_call_feedback::ModelCallJournal::for_snapshot(
                &graph_layout.cascade_router_path(),
            ),
        )
    });
    // P21 (8138): what the run adds to the knowledge store waits in the
    // run's batch until the run's end proposes it to the store's guard; an
    // unguarded or frozen run adds straight to the store.
    let guarded = roko_config.learning.guarded_commit != GuardedCommitMode::Off;
    let knowledge_batch = (guarded && !roko_config.learning.frozen).then(|| {
        run_id
            .clone()
            .unwrap_or_else(super::batch::new_batch_run_id)
    });
    // Keep the full SharedStateHub alive so the TUI can subscribe to the
    // live event stream. Previously this path only extracted sender().
    // Every publisher of this run (plan set, TUI bridges, telemetry, the
    // TUI itself, the knowledge it writes) shares this one hub.
    let state_hub = state_hub.unwrap_or_else(crate::state_hub::shared_state_hub);
    let state_hub_sender = state_hub.sender();
    let mut graph_feedback = build_graph_feedback_context(
        workdir,
        &roko_config,
        graph_run_config.cascade_router.as_ref(),
        cascade_journal.as_ref(),
        shared_factory.error_pattern_store(),
        knowledge_batch.as_deref(),
        Some(state_hub_sender.clone()),
    );
    // CLI dispatch turns record with their run's provenance sink (gap-ca8022).
    graph_feedback.provenance_sinks = Some(provenance_sinks.clone());
    // What the run's attempts teach the section bandit, saved at its end.
    let section_outcomes = graph_feedback.section_outcomes.clone();
    // The self-model the run's verdicts teach, saved at its end (6129).
    let self_model = graph_feedback.self_model.clone();
    // M1's sink, whose open chains resolve and whose controller state is
    // saved at the run's end (8122).
    let homeostasis = graph_feedback.homeostasis.clone();
    // The facade the dispatcher takes hears each plan's completion too
    // (q-6b7cca option 2, gap-a960a1).
    let plan_feedback = graph_feedback.feedback_facade.clone();
    // Its runs' manifests name the θ M1 dispatches with (8123).
    run_manifests.set_params_digest(homeostasis.as_ref().map(|sink| sink.params_digest()));

    // ── TUI vs inline progress decision ──────────────────────────────
    //
    // Auto-enable the interactive TUI dashboard when stdout is an
    // interactive terminal, unless the user explicitly opted out with
    // --no-tui, --quiet, or --json. This mirrors the runner-v2 approval
    // TUI logic (line ~470).
    let launch_tui = !no_tui && !quiet && !json && std::io::stdout().is_terminal();

    // P21 (gap-6ff99a): a knowledge batch an earlier run left unproposed, as a
    // run that crashed leaves it, goes through the guard now instead of staying
    // hidden; this process holds the runner lock, so no other run owns one.
    super::learning_commit::recover_orphaned_knowledge(
        workdir,
        &roko_config,
        knowledge_batch.as_deref(),
        Some(&state_hub),
    );
    // Plans whose footprints overlap never share the working tree at the
    // same time. Only needed when more than one plan may run at once.
    let plan_conflicts = if max_parallel_plans > 1 && plans.len() > 1 {
        super::plan_set::plan_set_conflicts(workdir, &plans).await
    } else {
        PlanConflicts::new()
    };
    // Announce the whole selected set before any plan starts, so the TUI and
    // StateHub consumers show every plan, the whole-set task total, and one
    // run clock across plan boundaries.
    state_hub_sender.publish(roko_core::DashboardEvent::PlanSetLoaded {
        plans: plan_set_entries(
            &crate::plan::plans_dir(workdir),
            &plans,
            &plan_order,
            &plan_conflicts,
        ),
    });
    let state_hub_sink: Arc<dyn roko_core::TelemetryEventSink> = Arc::new(
        crate::runner::graph_tui_bridge::StateHubTelemetrySink::new(state_hub_sender.clone()),
    );

    // Inline progress display: print per-node start/complete/fail to stderr
    // so the user can see what the Graph engine is doing in real time.
    // Disabled when the TUI is active — events flow through the dashboard
    // instead of being printed inline.
    let show_progress = !quiet && !json && !launch_tui;
    let graph_telemetry: Arc<dyn roko_core::TelemetryEventSink> =
        Arc::new(InlineProgressTelemetrySink {
            inner: state_hub_sink,
            show_progress,
        });

    // Wire graph engine execution into the TUI dashboard event stream.
    // Create separate TUI bridges for the task dispatcher (agent output
    // streaming) and the graph lifecycle bridge (plan/node events).
    let dispatcher_tui_bridge = crate::runner::tui_bridge::TuiBridge::new(state_hub_sender.clone());
    // M1 shows its EVs and episode moves on the run's StateHub (8130).
    if let Some(sink) = &homeostasis {
        sink.publish_to(dispatcher_tui_bridge.clone());
    }
    // The agent slot pool logs its live resizes there too (8134).
    let slot_events = dispatcher_tui_bridge.clone();
    let graph_tui_bridge = crate::runner::graph_tui_bridge::GraphTuiBridge::new(
        crate::runner::tui_bridge::TuiBridge::new(state_hub_sender),
    );

    // ── T0 reflex store ───────────────────────────────────────────────
    //
    // Open the persisted reflex store so the dispatcher can check for
    // deterministic condition-action rules before invoking the LLM.
    // The store is cheap to open (reads one JSONL file); rules are
    // matched in-memory at sub-millisecond latency.
    let reflex_store_path = graph_learn_dir.join("reflexes.jsonl");
    let reflex_store = roko_learn::reflex_store::ReflexStore::open(&reflex_store_path);
    tracing::debug!(
        path = %reflex_store_path.display(),
        rules = reflex_store.len(),
        "T0 reflex store opened for graph plan run"
    );

    let mut dispatcher_builder = crate::graph_task_dispatch::GraphTaskDispatcher::new(
        Arc::clone(&shared_factory),
        Arc::clone(&roko_config),
        workdir.to_path_buf(),
    )
    .with_plan_budget(
        plan_budget_ceiling,
        f64::from(roko_config.budget.max_turn_usd),
        budget_bypassed,
    )
    // The agent slots below cap the calls in flight at `max_agents`.
    .with_concurrent_calls(roko_config.conductor.max_agents)
    .with_cli_model_override(cli_model_override)
    .with_dangerously_skip_permissions(dangerously_skip_permissions)
    // FAST lane (`./dev.sh fast`): bound each attempt (gap-4a6dcb).
    .with_fast_bounds(super::fast_lane::FastAttemptBounds::from_env(workdir))
    .with_feedback(graph_feedback)
    .with_reflex_store(reflex_store)
    .with_tui_bridge(dispatcher_tui_bridge)
    .with_live_agent_output(live_agent_output)
    .with_outbound_floor(outbound_floor)
    .with_metrics(metrics.clone());

    // ── Whole-plan checks (gap-60233f) ──
    // Each plan's `[meta] verify`, or the default for a Cargo workspace,
    // runs on the plan's integrated result once its tasks all passed.
    let rust_workspace = workdir.join("Cargo.toml").is_file();
    let cargo = if rust_workspace && plans.iter().any(|plan| plan.tasks.meta.verify.is_empty()) {
        super::plan_set::CargoWorkspace::load(workdir).await
    } else {
        None
    };
    let plan_checks: HashMap<String, Vec<crate::task_parser::VerifyStep>> = plans
        .iter()
        .map(|plan| {
            let footprint = rust_workspace
                .then(|| super::plan_set::PlanFootprint::of(plan, workdir, cargo.as_ref()));
            (
                plan.id.clone(),
                super::plan_verify::plan_verify_steps(plan, footprint.as_ref()),
            )
        })
        .collect();

    // ── Batch integration (spec-f830c4) ──
    // Under --worktree-per-task every plan whose tasks all passed is
    // delivered into one batch branch, and every plan's attempts start from
    // it. A resumed run continues the batch an earlier process recorded.
    let batch = if worktree_per_task {
        let resumed = if resume_plan.is_some() {
            let plan_ids: Vec<&str> = plans.iter().map(|plan| plan.id.as_str()).collect();
            super::batch::resumed_batch_run(workdir, &plan_ids).await
        } else {
            None
        };
        let batch_run = resumed
            .or_else(|| run_id.clone())
            .unwrap_or_else(super::batch::new_batch_run_id);
        let batch = super::batch::BatchIntegration::open(workdir, &batch_run)
            .await
            .map_err(|error| anyhow!("open the run's batch branch: {error}"))?;
        // The branch says which run it is for, as `git branch -v` and hosts
        // show it (gap-c2efe7).
        let titles = plans.iter().map(plan_title).collect::<Vec<_>>();
        batch.describe(&titles.join("; ")).await;
        if !quiet && !json {
            tracing::info!(
                branch = batch.branch(),
                "finished plans are delivered into the run's batch branch"
            );
        }
        Some(batch)
    } else {
        None
    };

    // ── Per-task worktree isolation (opt-in via --worktree-per-task) ──
    let mut workspace_provider: Option<Arc<dyn roko_graph::workspace::ExecutionWorkspaceProvider>> =
        None;
    // The attempt checkouts' manager, which each plan tells its run
    // (bug-056b40).
    let mut worktrees: Option<crate::orchestrator::worktree::WorktreeManager> = None;
    if worktree_per_task {
        use crate::orchestrator::worktree::{WorktreeConfig, WorktreeManager};
        let worktree_manager = WorktreeManager::new(WorktreeConfig {
            repo_root: workdir.to_path_buf(),
            base_branch: batch
                .as_ref()
                .map_or_else(|| "HEAD".to_string(), |batch| batch.branch().to_string()),
            worktrees_root: workdir.join(".roko").join("worktrees"),
            max_live: None,
            idle_ttl: std::time::Duration::from_hours(1),
        });
        worktrees = Some(worktree_manager.clone());
        repair_worktree_state(&worktree_manager).await;
        // Each attempt reserves its worktree's disk headroom before it
        // starts, and attempts serialise under disk pressure (reg-7cf6f9).
        let counted = worktree_manager.clone();
        let mut disk_admission =
            super::disk_admission::DiskAdmission::new(workdir, &roko_config.resources)
                .with_worktree_count(move || counted.active_count());
        if let Some(ring) = graph_run_config.conductor_ring.clone() {
            disk_admission = disk_admission.with_ring(ring);
        }
        let provider = Arc::new(
            crate::graph_execution::WorktreeExecutionWorkspaceProvider::new(worktree_manager),
        );
        if !quiet && !json {
            let asked_by = if worktree_per_task_explicit {
                "--worktree-per-task"
            } else {
                "[runner] worktree_per_task"
            };
            tracing::info!(asked_by, "per-task worktree isolation enabled");
        }
        dispatcher_builder = dispatcher_builder
            .with_workspace_provider(provider.clone())
            .with_disk_admission(disk_admission);
        workspace_provider = Some(provider);
    }
    // The same provider settles the worktrees the rich topology's executors
    // hand on to their gates.
    let cell_resources = plan_cell_resources(rich_topology, &roko_config.gates, workspace_provider);

    // ── Conductor supervision (spec-a0403b) ───────────────────────────
    //
    // The run's conductor watches each running attempt's live output, and
    // the ticker evaluates it every `SUPERVISION_INTERVAL`: a `Restart`
    // cancels that attempt, which retries; a `Fail` stops the run the way
    // SIGTERM does, and the run returns an error naming the watcher. With
    // `[conductor] supervise = false` (1210), or `silence_timeout_secs` and
    // `task_stall_secs` both 0, there is no conductor and no ticker.
    if let (Some(conductor), Some(ring)) = (
        graph_run_config.conductor.clone(),
        graph_run_config.conductor_ring.clone(),
    ) {
        // A run that serve hosts counts its evaluations on `/metrics`
        // (gap-a95898).
        if let Some(metrics) = &metrics {
            conductor.attach_metrics(Arc::clone(metrics));
        }
        dispatcher_builder = dispatcher_builder.with_conductor(conductor, ring);
    }
    let graph_task_dispatcher = Arc::new(dispatcher_builder);
    if !quiet && !json {
        graph_task_dispatcher.announce_call_reservation();
    }
    // `budget.max_daily_usd`: today's spend before this run, read once, so a
    // run whose day is already spent starts no task (bug-ae28ac).
    graph_task_dispatcher.prime_daily_budget().await;
    for plan_id in &held_plans {
        graph_task_dispatcher.hold_for_approval(plan_id);
    }
    // The accepted commit of a named one-task plan, such as a prompt run's,
    // takes the plan's title (gap-c2efe7).
    for plan in &plans {
        if let ([_], Some(title)) = (
            plan.tasks.tasks.as_slice(),
            plan.tasks.meta.title.as_deref(),
        ) {
            graph_task_dispatcher.name_commits(&plan.id, title);
        }
    }
    let conductor_stop = Arc::new(parking_lot::Mutex::new(
        None::<crate::graph_task_dispatch::ConductorStop>,
    ));
    let _conductor_ticker = graph_task_dispatcher.spawn_conductor_ticker(
        crate::graph_task_dispatch::SUPERVISION_INTERVAL,
        {
            let conductor_stop = Arc::clone(&conductor_stop);
            let interrupt = interrupt.clone();
            move |stop| {
                tracing::error!(%stop, "the conductor is stopping the plan run");
                *conductor_stop.lock() = Some(stop);
                interrupt.request(PlanRunInterrupt::Conductor);
            }
        },
    );
    // `[conductor] max_agents` caps concurrently executing tasks across
    // every plan of the run. With M1 on, θ's B5 knob sizes the pool within
    // that cap (8134); in shadow mode the pool stays at the cap.
    let mut agent_slots = super::agent_slots::AgentSlotDispatcher::new(
        graph_task_dispatcher.clone(),
        roko_config.conductor.max_agents,
    );
    let live_limit = homeostasis
        .as_ref()
        .filter(|sink| sink.mode() == HomeostasisMode::On);
    if let Some(sink) = live_limit {
        agent_slots = agent_slots.with_live_limit(sink.handle().clone(), slot_events);
    }
    let task_dispatcher: Arc<dyn TaskDispatcher> = Arc::new(agent_slots);

    // ── TUI execution command channel (P2-TUI-3) ─────────────────────
    //
    // Create the bi-directional command/ack channel before spawning the TUI
    // so we can wire both ends: sender+ack_rx go to the TUI, cmd_rx+ack_tx
    // stay in the async execution path. The run_id "graph-engine" is a
    // placeholder; per-plan run IDs are substituted when each plan begins.
    // Using tokio::sync::mpsc directly here because ExecutionCommandSender
    // wraps a bounded Tokio sender and we need the raw receiver.
    let (tui_cmd_sender, mut exec_cmd_rx, tui_ack_tx, tui_ack_rx) =
        ExecutionCommandSender::channel("graph-engine");
    let tui_ack_receiver = CommandAckReceiver::new(tui_ack_rx);

    // `roko plan pause/resume/cancel/retry` in another terminal write
    // `.roko/state/control.json`; the plan-set driver routes what they write
    // through this channel, like a TUI command (bug-8208a6). A command left
    // from before this run is dropped.
    let control_file_sender = tui_cmd_sender.clone();
    let control_state_dir = RokoLayout::for_project(workdir).state_dir();
    if let Some(stale) = crate::runner::types::ControlCommand::poll(&control_state_dir) {
        tracing::warn!(
            command = ?stale.command,
            "dropping a plan control command written before this run"
        );
    }

    // Shared pause flag: set and cleared by the Pause and Resume commands of
    // every control surface. Pause holds (decision 1206): while it is set the
    // plan-set driver starts no plan and each task's dispatch starts no
    // attempt, a retry included (`hold_while_paused`); running attempts
    // finish, and the deadline keeps running.
    let shared_pause_flag: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

    // ── Spawn interactive TUI thread ─────────────────────────────────
    //
    // Replicates the runner-v2 approval TUI pattern: spawn the App on a
    // dedicated OS thread so it owns the terminal while the async engine
    // drives execution on the current task.
    let mut tui_session: Option<TuiSession> = None;
    if launch_tui {
        // Redirect stderr to a log file so tracing output does not
        // corrupt the TUI's raw terminal display. `TuiSession::stop`
        // points it back at the terminal.
        let layout = RokoLayout::for_project(workdir);
        let stderr_log_path = layout.runner_stderr_log();
        let _ = std::fs::create_dir_all(stderr_log_path.parent().unwrap_or(workdir));
        redirect_stderr_to(&stderr_log_path);

        let (tui_shutdown_tx, tui_shutdown_rx) = std::sync::mpsc::channel();
        // When the CLI routes SIGINT/SIGTERM (and SIGHUP) to this run, keep
        // the TUI's terminal-reset handler off them so they stop the run
        // gracefully instead of killing the process.
        let host_owns_signals = plan_run_owns_termination_signals();
        let host_owns_hangup = plan_run_owns_hangup_signal();
        let state_hub_for_tui = state_hub.clone();
        let workdir_for_tui = workdir.to_path_buf();
        let handle = std::thread::Builder::new()
            .name("roko-graph-engine-tui".to_string())
            .spawn(move || {
                let app = crate::tui::App::new_connected_with_page(
                    &workdir_for_tui,
                    None, // Default page = Tab::Dashboard
                    &state_hub_for_tui,
                )
                .without_mouse_capture()
                // Close once the whole selected plan set is terminal; the
                // runner also stops it explicitly when the run ends early.
                .with_exit_on_plan_completion()
                .with_shutdown_receiver(tui_shutdown_rx);
                let app = if host_owns_signals {
                    app.with_host_termination_signals()
                } else {
                    app
                };
                let app = if host_owns_hangup {
                    app.with_host_hangup_signal()
                } else {
                    app
                };
                // P2-TUI-3: Wire the execution command sender so TUI recovery
                // keybindings (s=soft-retry, S=repair, c=reverify, F=force-advance,
                // V=reverify-plan, p=pause/resume) forward commands to this loop.
                app.with_execution_command_sender(tui_cmd_sender, tui_ack_receiver)
                    .run()
            });
        let handle = match handle {
            Ok(handle) => handle,
            Err(error) => {
                restore_redirected_stderr();
                return Err(error).context("spawn Graph Engine TUI thread");
            }
        };
        tui_session = Some(TuiSession {
            shutdown: Some(tui_shutdown_tx),
            handle: Some(handle),
        });
    }

    let total_tasks: usize = plans.iter().map(|p| p.tasks.tasks.len()).sum();
    let plan_count = plans.len();

    if !quiet && !json && !launch_tui {
        let plan_names: Vec<&str> = plan_execution_order.iter().map(String::as_str).collect();
        tracing::info!(
            plan_count,
            total_tasks,
            max_parallel_plans,
            plans = plan_names.join(", "),
            "running plans via Graph Engine"
        );
        if max_parallel_plans > 1 && plan_count > 1 {
            let ceiling = f64::from(roko_config.budget.max_plan_usd);
            for (plan_id, others) in &plan_conflicts {
                tracing::info!(
                    plan_id = %plan_id,
                    never_beside = %others.keys().cloned().collect::<Vec<_>>().join(", "),
                    "plan shares part of the working tree with other plans; they run one at a time"
                );
            }
            if ceiling > 0.0 {
                tracing::info!(
                    in_flight_ceiling_usd = ceiling * max_parallel_plans.min(plan_count) as f64,
                    "up to {} plans run at once, each under its own max_plan_usd ceiling",
                    max_parallel_plans.min(plan_count)
                );
            }
        }
    }
    for (prerequisite, evidence) in &plan_order.satisfied_outside {
        tracing::info!(
            prerequisite = %prerequisite,
            evidence = %evidence,
            "prerequisite plan outside the selected set is complete"
        );
    }

    let mut all_succeeded = true;
    let mut total_output_count = 0usize;
    let mut plan_outcomes = std::collections::BTreeMap::<String, bool>::new();
    // How the tasks of each plan that ran settled, for the run metrics.
    let mut plan_task_verdicts = std::collections::BTreeMap::<String, TaskVerdictCounts>::new();
    // The work each plan that was not delivered left that no check ran on
    // (bug-db89d4).
    let mut plan_unverified_work = std::collections::BTreeMap::<String, UnverifiedWork>::new();
    // Set once a stop request (a signal, closing the TUI, the FAST deadline
    // or the conductor) stops the run; later plans are left unstarted.
    let mut stopped_by: Option<PlanRunInterrupt> = None;

    let failure_issues = graph_run_config.github_ops.clone().map(|ops| {
        super::failure_issues::FailureIssues::new(ops, &roko_config.github.label_prefix)
    });
    let run_context = PlanRunContext {
        workdir,
        provenance_sinks: &provenance_sinks,
        resume_plan: resume_plan.as_deref(),
        plan_count,
        fresh,
        force_resume,
        max_retries,
        max_tasks,
        rich_topology,
        worktree_per_task,
        cell_resources: &cell_resources,
        batch: batch.as_ref(),
        plan_checks: &plan_checks,
        gate_env_passthrough: &roko_config.gates.env_passthrough,
        worktrees: worktrees.as_ref(),
        delete_attempt_branches: roko_config.runner.delete_attempt_branches,
        quiet,
        json,
        launch_tui,
        task_dispatcher: &task_dispatcher,
        graph_task_dispatcher: &graph_task_dispatcher,
        feedback: plan_feedback.as_ref(),
        plan_failure_policy: roko_config.conductor.plan_failure_policy,
        graph_tui_bridge: &graph_tui_bridge,
        graph_telemetry: &graph_telemetry,
        shared_pause_flag: &shared_pause_flag,
        interrupt: &interrupt,
        run_manifests: &run_manifests,
        learning: &roko_config.learning,
        caller_run_id: run_id.as_deref(),
        failure_issues: failure_issues.as_ref(),
        spec_gate: &spec_gate,
    };
    let mut scheduler = super::plan_set::PlanSetScheduler::new(
        &plan_order,
        plan_conflicts,
        max_parallel_plans,
        fail_fast,
    );
    let mut running = futures::stream::FuturesUnordered::new();
    let mut controls = std::collections::HashMap::<String, PlanControl>::new();
    // A TUI skip stops one task's running agent (gap-c002bb).
    let task_stops = graph_task_dispatcher.operator_stops();
    // `roko inject` reaches the run through a socket of its own, and what it
    // sends is routed like a TUI command (gap-f118b3).
    let operator_directives = graph_task_dispatcher.operator_directives();
    // `roko plan budget raise` raises a running plan's ceiling (backlog 2118).
    let budget_control = graph_task_dispatcher.plan_budget_control();
    let (inject_sender, mut inject_rx, inject_ack_tx, inject_acks) =
        ExecutionCommandSender::channel("graph-engine");
    let inject_target = Arc::clone(&graph_task_dispatcher);
    let inject_server = crate::inject::listen_for_inject(
        workdir,
        crate::inject::InjectLink {
            commands: inject_sender,
            acks: inject_acks,
            target: Arc::new(move |session: &str| inject_target.inject_target(session)),
            answer_timeout: crate::inject::INJECT_ANSWER_TIMEOUT,
        },
    );
    // Plans the operator asked to run again, until they start, and whether
    // any was: the run then settles by each plan's last run.
    let mut pending_reruns = std::collections::HashMap::<String, PlanRerun>::new();
    let mut reran_plans = false;
    // A checkpoint or budget-ledger error in one plan stops the run once the
    // plans still running have finished.
    let mut first_error: Option<anyhow::Error> = None;

    // ── Plan-set driver ──────────────────────────────────────────────
    //
    // Start plans as the scheduler admits them, route TUI commands to the
    // running plans, and settle each plan as it finishes. Every plan that
    // does not run to a PlanCompleted of its own (blocked, cancelled before
    // it started, unconvertible, invalid) still gets a terminal
    // PlanCompleted, so the announced plan set can complete.
    loop {
        if stopped_by.is_none()
            && let Some(reason) = pending_interrupt(&interrupt, &mut tui_session)
        {
            stopped_by = Some(reason);
            scheduler.stop();
        }
        forward_control_file(&control_state_dir, &control_file_sender);
        let mut routed = route_execution_commands(
            &mut exec_cmd_rx,
            &tui_ack_tx,
            &controls,
            &mut scheduler,
            &shared_pause_flag,
            &task_stops,
            &operator_directives,
            &budget_control,
            workdir,
        );
        routed.merge(route_execution_commands(
            &mut inject_rx,
            &inject_ack_tx,
            &controls,
            &mut scheduler,
            &shared_pause_flag,
            &task_stops,
            &operator_directives,
            &budget_control,
            workdir,
        ));
        for plan_id in routed.cancelled_before_start {
            graph_tui_bridge.log_event(
                "graph.plan_cancelled",
                &format!("plan '{plan_id}' cancelled before it started"),
            );
            graph_tui_bridge.plan_completed(&plan_id, false);
            plan_outcomes.insert(plan_id, false);
            all_succeeded = false;
        }
        for (plan_id, rerun) in routed.reruns {
            graph_tui_bridge.log_event("graph.plan_rerun", &rerun.describe(&plan_id));
            pending_reruns.insert(plan_id, rerun);
            reran_plans = true;
        }
        // The event log and the plan's run manifest say who raised a plan's
        // ceiling, and to what (backlog 2118); its costs.json keeps it.
        for raised in routed.budget_raises {
            graph_tui_bridge.log_event("graph.plan_budget_raised", &raised.describe());
            if let Some(run_id) = graph_task_dispatcher.plan_run_id(&raised.plan_id) {
                run_manifests.record_budget_raise(
                    &run_id,
                    &raised.plan_id,
                    &raised.raise,
                    &raised.requested_by,
                );
            }
        }

        // A paused run starts no plan (decision 1206).
        let paused = shared_pause_flag.load(Ordering::Acquire);
        let admission = if paused {
            super::plan_set::Admission::default()
        } else {
            scheduler.admit()
        };
        for (plan_id, reason) in admission.blocked {
            report_blocked_plan(&graph_tui_bridge, &plan_id, &reason);
            plan_outcomes.insert(plan_id, false);
            all_succeeded = false;
        }
        for (plan_id, holder, reason) in admission.waiting {
            tracing::info!(
                plan_id = %plan_id,
                waits_for = %holder,
                reason = %reason,
                "plan waits: it cannot share the working tree with a running plan"
            );
            graph_tui_bridge.log_event(
                "graph.plan_waiting",
                &format!("plan '{plan_id}' waits for '{holder}': {reason}"),
            );
        }
        for plan_id in admission.start {
            let plan = plans
                .iter()
                .find(|plan| plan.id == plan_id)
                .ok_or_else(|| {
                    anyhow!("Graph execution order references unloaded plan '{plan_id}'")
                })?;
            let control = PlanControl {
                rerun: pending_reruns.remove(&plan_id),
                ..PlanControl::default()
            };
            controls.insert(plan_id, control.clone());
            // Agents of the run's other plans hear what this one writes (gap-c09fc7).
            let footprint = super::plan_set::PlanFootprint::of(plan, workdir, cargo.as_ref());
            let writes = footprint.writes.iter().map(ToString::to_string).collect();
            graph_task_dispatcher.plan_started(&plan.id, writes);
            running.push(run_admitted_plan(&run_context, plan, control));
        }

        // A paused run with plans still to start waits for resume.
        if running.is_empty() && (!paused || scheduler.is_settled()) {
            break;
        }
        tokio::select! {
            Some((plan_id, result)) = running.next() => {
                controls.remove(&plan_id);
                let outcome = match result {
                    Ok(result) => {
                        total_output_count += result.output_count;
                        plan_task_verdicts.insert(plan_id.clone(), result.tasks);
                        match result.unverified_work {
                            Some(work) => plan_unverified_work.insert(plan_id.clone(), work),
                            None => plan_unverified_work.remove(&plan_id),
                        };
                        result.outcome
                    }
                    Err(error) => {
                        tracing::error!(
                            plan_id = %plan_id,
                            error = %error,
                            "plan run failed; starting no further plans"
                        );
                        scheduler.stop();
                        first_error.get_or_insert(error);
                        PlanOutcome::Failed
                    }
                };
                if !outcome.succeeded() {
                    all_succeeded = false;
                }
                plan_outcomes.insert(plan_id.clone(), outcome.succeeded());
                scheduler.finish(&plan_id, outcome);
                graph_task_dispatcher.plan_finished(&plan_id);
                // A pause outlives the plan that was running: the next plan
                // waits for resume.
            }
            () = tokio::time::sleep(PLAN_WATCH_INTERVAL) => {}
        }
    }
    drop(running);
    // A plan the operator ran again settles by its last run.
    if reran_plans {
        all_succeeded = plan_execution_order
            .iter()
            .all(|plan_id| plan_outcomes.get(plan_id) == Some(&true));
    }

    // Commands that arrived after the last plan finished have nothing left
    // to act on. The inject socket stops listening first.
    drop(inject_server);
    for (commands, acks) in [
        (&mut exec_cmd_rx, &tui_ack_tx),
        (&mut inject_rx, &inject_ack_tx),
    ] {
        while let Ok(cmd) = commands.try_recv() {
            let ack = ack_for(
                &cmd,
                CommandAckStatus::Rejected,
                Some("the plan run has finished".into()),
            );
            let _ = acks.try_send(ack);
        }
    }
    // A conductor `Fail` stopped the run: it fails, naming the watcher.
    if let Some(stop) = conductor_stop.lock().take() {
        return Err(anyhow!("{stop}"));
    }
    if let Some(error) = first_error {
        return Err(error);
    }

    if let Some(reason) = stopped_by {
        all_succeeded = false;
        let unstarted = plan_execution_order
            .iter()
            .filter(|plan_id| !plan_outcomes.contains_key(*plan_id))
            .count();
        tracing::warn!(
            signal = reason.label(),
            unstarted,
            "plan run stopped early; resume with --resume-plan"
        );
        let not_started = if unstarted > 0 {
            format!("; {unstarted} plan(s) not started")
        } else {
            String::new()
        };
        graph_tui_bridge.log_event(
            "graph.run_interrupted",
            &format!("{} stopped the run{not_started}", reason.cause()),
        );
    }

    // A run that delivered nothing and accepted nothing leaves no batch
    // branch behind (bug-58329f).
    let mut batch_discarded = false;
    if let Some(batch) = batch.as_ref()
        && run_accepted_nothing(batch, &plans, &graph_task_dispatcher).await
        && batch.discard_if_unused().await
    {
        tracing::info!(
            batch = batch.branch(),
            "the run left no work; removed its batch branch"
        );
        graph_tui_bridge.log_event(
            "graph.batch_discarded",
            &format!(
                "the run left no work, so its batch branch {} was removed",
                batch.branch()
            ),
        );
        batch_discarded = true;
    }

    // spec-f830c4: with --promote, a run whose plans were all delivered
    // promotes its batch into the target branch and tags it. The run summary
    // reports it (gap-415c54).
    let mut promotion = None;
    if let (Some(batch), Some(target)) = (batch.as_ref(), promote.as_deref()) {
        if all_succeeded {
            match batch.promote(target).await {
                Ok(promoted) if promoted.moved => {
                    tracing::info!(
                        batch = batch.branch(),
                        target,
                        commit = %promoted.commit,
                        tag = %promoted.tag,
                        "run promoted"
                    );
                    promotion = Some(promoted);
                }
                Ok(promoted) => {
                    tracing::warn!(
                        batch = batch.branch(),
                        target,
                        tag = %promoted.tag,
                        summary = %promoted.summary,
                        "run not promoted: its target is checked out"
                    );
                    graph_tui_bridge.log_event("graph.run_promotion_parked", &promoted.summary);
                    promotion = Some(promoted);
                }
                Err(error) => {
                    all_succeeded = false;
                    tracing::error!(batch = batch.branch(), target, %error, "run promotion failed");
                    graph_tui_bridge.error(&format!(
                        "promoting {} into {target} failed: {error}",
                        batch.branch()
                    ));
                }
            }
        } else {
            tracing::warn!(
                batch = batch.branch(),
                target,
                "not every plan was delivered, so the run was not promoted"
            );
        }
    }

    // gap-4ec59f: the run never changes the operator's checkout, so its
    // summary says how to take the batch's work, unless a promotion did.
    let merge_command = match (batch.as_ref(), promotion.as_ref()) {
        (Some(batch), None) => batch.merge_command().await,
        _ => None,
    };

    let plan_outcome_labels = plan_execution_order
        .iter()
        .map(|plan_id| {
            let label = scheduler
                .outcome(plan_id)
                .map_or("not started", PlanOutcome::label);
            (plan_id.clone(), serde_json::Value::from(label))
        })
        .collect::<serde_json::Map<_, _>>();
    let plan_budgets = plans
        .iter()
        .map(|plan| {
            let budget = graph_task_dispatcher.plan_budget_snapshot(&plan.id);
            serde_json::json!({
                "plan_id": plan.id,
                "spent_usd": budget.spent_usd,
                "reserved_usd": budget.reserved_usd,
                "ceiling_usd": budget.ceiling_usd,
                "exhausted": budget.exhausted,
                "dispatch_blocked": budget.dispatch_blocked,
            })
        })
        .collect::<Vec<_>>();
    let total_cost_usd = plans
        .iter()
        .map(|plan| {
            graph_task_dispatcher
                .plan_budget_snapshot(&plan.id)
                .spent_usd
        })
        .sum::<f64>();

    if let Some(extension_chain) = graph_run_config.extension_chain.as_ref() {
        let mut chain = extension_chain.lock().await;
        for (name, error) in chain.shutdown_all().await {
            tracing::warn!(extension = %name, %error, "extension shutdown failed");
            all_succeeded = false;
        }
    }

    // ── Run summary for the TUI ────────────────────────────────────
    // The TUI closes on its own once the whole plan set is terminal; it is
    // stopped explicitly below once post-run persistence finishes. The tally
    // names how each plan ended, and is an error only when one failed: an
    // unverified, cancelled or blocked plan did not fail (bug-db89d4).
    if !all_succeeded {
        let outcomes = plan_execution_order
            .iter()
            .map(|plan_id| scheduler.outcome(plan_id))
            .collect::<Vec<_>>();
        let message = format!(
            "Graph Engine: {plan_count} plan(s): {}",
            outcome_tally(&outcomes)
        );
        if outcomes.contains(&Some(PlanOutcome::Failed)) {
            state_hub
                .sender()
                .publish(roko_core::DashboardEvent::Error { message });
        } else {
            graph_tui_bridge.log_event("graph.run_outcome", &message);
        }
    }
    // ── Save or propose the run's learning (UX34; P21, 8138) ─────────
    //
    // Save learned routing state (confidence stats, LinUCB weights, Pareto
    // frontier) so that force_backend override outcomes and all other
    // routing observations survive across runs. Saving through the run's
    // journal truncates it once the snapshot holds its observations, so a
    // later load does not replay them again (bug-dfb28f). If the save fails,
    // the journal keeps them for that load. `[learning] guarded_commit`
    // proposes the router's merge and the run's knowledge batch to their
    // guards, whatever the run's outcome; `off` saves the router unchecked.
    // A frozen run saves and proposes nothing.
    let plan_run_ids: Vec<String> = plans
        .iter()
        .filter_map(|plan| graph_task_dispatcher.plan_run_id(&plan.id))
        .collect();
    let router = graph_run_config
        .cascade_router
        .as_deref()
        .zip(cascade_journal.as_deref());
    super::learning_commit::commit_run_learning(&super::learning_commit::RunLearning {
        workdir,
        config: &roko_config,
        batch: knowledge_batch.as_deref(),
        router,
        run_ids: &plan_run_ids,
        notices: Some(&state_hub),
    });

    // ── Persist the section bandit (S02 L9) ─────────────────────────
    //
    // Fold what the run's settled attempts taught the section bandit into
    // its file, under the file's lock, so a concurrent run's outcomes
    // survive. The attempts of failed and cancelled plans count too; a
    // frozen run has no outcomes to fold.
    if let Some(outcomes) = &section_outcomes {
        let path = workdir.join(roko_learn::section_effect::SECTION_BANDIT_PATH);
        if let Err(err) = outcomes.save(&path) {
            tracing::warn!(
                path = %path.display(),
                error = %err,
                "failed to persist the section bandit (non-fatal)"
            );
        }
    }

    // ── Persist the self-model (S04, 6129) ──────────────────────────
    //
    // Save what the run's settled verdicts taught the self-model, atomically,
    // so the next run, a resumed one included, loads it. A frozen run
    // registered no outcome sink, so it has nothing to save.
    if !roko_config.learning.frozen
        && let Some(runtime) = &self_model
        && let Err(err) = runtime.save()
    {
        tracing::warn!(error = %err, "failed to persist the self-model state (non-fatal)");
    }

    // ── M1, the ultrastable controller (S06, 8122) ───────────────────
    //
    // The chains still open resolve as they stand, and the controller's state
    // is saved for the next run.
    if let Some(sink) = &homeostasis {
        sink.finish();
    }

    // ── Persist run metrics (backlog #169) ──────────────────────────
    //
    // Collect task counts and cost from the just-completed plan loop and
    // append a structured RunMetricsRecord to `.roko/learn/run-metrics.jsonl`.
    // Each task counts under its own verdict (bug-7eb27e), not its plan's.
    // The write is one appended line, made before the run returns: a write
    // spawned onto the runtime could be dropped when the process exits.
    {
        let duration_ms = run_start.elapsed().as_millis() as u64;
        let per_plan: Vec<roko_learn::run_metrics::PlanMetrics> = plan_outcomes
            .iter()
            .map(|(id, succeeded)| {
                let plan_tasks = plans
                    .iter()
                    .find(|p| &p.id == id)
                    .map_or(0, |p| p.tasks.tasks.len());
                // A plan that never started (blocked, or cancelled before it
                // started) ran none of its tasks.
                let tasks = plan_task_verdicts
                    .get(id)
                    .copied()
                    .unwrap_or(TaskVerdictCounts::not_run(plan_tasks));
                roko_learn::run_metrics::PlanMetrics {
                    run_id: graph_task_dispatcher.plan_run_id(id),
                    ..plan_metrics(id, *succeeded, tasks)
                }
            })
            .collect();
        // One run id, not two (S01 §4.2): a single plan's metrics carry its
        // checkpoint run, as its attempt keys do.
        let run_id = match per_plan.as_slice() {
            [plan] => plan.run_id.clone(),
            _ => None,
        }
        .unwrap_or_else(|| format!("graph-run-{}", chrono::Utc::now().timestamp_millis().max(0)));
        let tasks_completed: usize = per_plan.iter().map(|p| p.tasks_completed).sum();
        let tasks_already_satisfied: usize =
            per_plan.iter().map(|p| p.tasks_already_satisfied).sum();
        let tasks_failed: usize = per_plan.iter().map(|p| p.tasks_failed).sum();
        let tasks_unverified: usize = per_plan.iter().map(|p| p.tasks_unverified).sum();
        let tasks_skipped: usize = per_plan.iter().map(|p| p.tasks_skipped).sum();
        let any_budget_exhausted = plans
            .iter()
            .any(|p| graph_task_dispatcher.plan_budget_snapshot(&p.id).exhausted);
        let (agg_tokens_in, agg_tokens_out, agg_dispatch_count) =
            graph_task_dispatcher.run_aggregate_stats();
        let record = roko_learn::run_metrics::RunMetricsRecord {
            run_id,
            timestamp: chrono::Utc::now().to_rfc3339(),
            duration_ms,
            total_tasks,
            tasks_completed,
            tasks_already_satisfied,
            tasks_failed,
            tasks_unverified,
            tasks_skipped,
            total_cost_usd,
            total_tokens_in: agg_tokens_in,
            total_tokens_out: agg_tokens_out,
            total_agent_calls: agg_dispatch_count as usize,
            budget_exhausted: any_budget_exhausted,
            plans: per_plan,
        };
        let metrics_path = graph_learn_dir.join("run-metrics.jsonl");
        if let Err(err) = roko_learn::run_metrics::append_run_metrics(&metrics_path, &record) {
            tracing::warn!(error = %err, "failed to persist run metrics (non-fatal)");
        }
        // The recent runs' cost per verified task, this one last, for a host's
        // trend (gap-de0e96).
        match roko_learn::run_metrics::read_recent(&metrics_path, RUN_COST_TREND_RUNS) {
            Ok(runs) if !runs.is_empty() => graph_tui_bridge
                .tui()
                .efficiency_trend_updated(run_cost_trend(&runs)),
            Ok(_) => {}
            Err(err) => tracing::warn!(error = %err, "the run cost trend was not published"),
        }
    }

    // Restores the terminal (and stderr) before the summary is printed.
    if let Some(mut tui) = tui_session.take() {
        tui.stop();
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "engine": "graph",
                "dry_run": false,
                "succeeded": all_succeeded,
                "plan_count": plan_count,
                "total_tasks": total_tasks,
                "total_outputs": total_output_count,
                "total_agent_calls": graph_task_dispatcher.run_aggregate_stats().2,
                "total_cost_usd": total_cost_usd,
                "plan_budgets": plan_budgets,
                "max_parallel_plans": max_parallel_plans,
                "plan_outcomes": plan_outcome_labels,
                "interrupted_by": stopped_by.map(PlanRunInterrupt::label),
                "batch": batch.as_ref().map(|batch| serde_json::json!({
                    "branch": batch.branch(),
                    "discarded": batch_discarded,
                    // Work no check ran on, which was not delivered
                    // (bug-db89d4).
                    "unverified": plan_unverified_work
                        .iter()
                        .map(|(plan_id, work)| serde_json::json!({
                            "plan_id": plan_id,
                            "branch": work.branch,
                            "commit": work.commit,
                            "base": work.base,
                            "tasks": work.tasks,
                            "review_command": work.review_command(),
                        }))
                        .collect::<Vec<_>>(),
                    "deliveries": batch
                        .receipts()
                        .iter()
                        .map(|receipt| batch.summary_record(receipt))
                        .collect::<Vec<_>>(),
                    "promotion": promotion,
                    "merge_command": merge_command,
                })),
            }))
            .unwrap_or_default()
        );
    } else if !quiet {
        // Always print a human-readable summary to stdout so `--no-tui` and
        // piped invocations produce visible output.  When the TUI was active
        // the user already saw interactive progress, but one final summary
        // line is still useful (and harmless) after the terminal is restored.
        if let Some(reason) = stopped_by {
            println!(
                "Graph Engine stopped by {}: {}/{} plan(s) attempted, {} task(s), ${:.2}; \
                 resume with --resume-plan",
                reason.cause(),
                plan_outcomes.len(),
                plan_count,
                total_tasks,
                total_cost_usd,
            );
        } else {
            println!(
                "Graph Engine complete: {} plan(s), {} task(s), ${:.2}",
                plan_count, total_tasks, total_cost_usd,
            );
        }
        // Name the plans that did not succeed, such as one whose delivery
        // into the batch branch failed its regression check (spec-f830c4),
        // with how each ended (bug-db89d4).
        let failed_plans = plan_outcomes
            .iter()
            .filter(|(_, succeeded)| !**succeeded)
            .map(|(plan_id, _)| {
                let label = scheduler
                    .outcome(plan_id)
                    .map_or("not started", PlanOutcome::label);
                format!("{plan_id} ({label})")
            })
            .collect::<Vec<_>>();
        if !failed_plans.is_empty() {
            println!("Plans that did not succeed: {}", failed_plans.join(", "));
        }
        // Work no check ran on is never delivered: say where it is
        // (bug-db89d4).
        for (plan_id, work) in &plan_unverified_work {
            println!(
                "Plan {plan_id}: no check ran on the work of {}, so it was not delivered. \
                 It is on {} at {}; to see it:\n  {}",
                work.tasks.join(", "),
                work.branch,
                short_oid(&work.commit),
                work.review_command()
            );
        }
        // Where each delivered plan's work landed (gap-415c54).
        if let Some(batch) = batch.as_ref() {
            for receipt in batch.receipts() {
                if receipt.state.is_success()
                    && let Some(merge) = &receipt.merge_commit
                {
                    // A plan that did not finish delivered its verified
                    // tasks only (bug-74164c).
                    let partial = receipt
                        .extensions
                        .get(super::batch::PARTIAL_EXTENSION)
                        .and_then(|partial| partial["tasks"].as_array())
                        .map(|tasks| {
                            let tasks = tasks
                                .iter()
                                .filter_map(serde_json::Value::as_str)
                                .collect::<Vec<_>>();
                            format!(
                                " (partial: the plan did not finish; only its verified task(s) {})",
                                tasks.join(", ")
                            )
                        })
                        .unwrap_or_default();
                    println!(
                        "Plan {} delivered into {} at {merge}{partial}",
                        receipt.request.plan_id,
                        batch.branch()
                    );
                    let kept = receipt
                        .extensions
                        .get(super::batch::ATTEMPT_CLEANUP_EXTENSION)
                        .and_then(|cleanup| cleanup["kept_branches"].as_array())
                        .map(|branches| {
                            branches
                                .iter()
                                .filter_map(serde_json::Value::as_str)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    if !kept.is_empty() {
                        println!(
                            "  {} attempt branch(es) kept: {}",
                            kept.len(),
                            kept.join(", ")
                        );
                    }
                }
            }
            if let Some(promotion) = &promotion {
                println!("{}", promotion.summary);
            }
            if let Some(command) = &merge_command {
                println!(
                    "The work is on branch {}; your checkout was not changed. To take it:\n  \
                     {command}",
                    batch.branch()
                );
            }
        }
    }

    // An unverified plan is not a failure: the run ends unverified when every
    // plan succeeded or ended unverified (bug-db89d4).
    let outcomes = plan_execution_order
        .iter()
        .map(|plan_id| scheduler.outcome(plan_id))
        .collect::<Vec<_>>();
    let unverified = stopped_by.is_none()
        && !all_succeeded
        && outcomes.contains(&Some(PlanOutcome::Unverified))
        && outcomes.iter().all(|outcome| {
            matches!(
                outcome,
                Some(PlanOutcome::Succeeded | PlanOutcome::Unverified)
            )
        });
    Ok(RunExit {
        code: match stopped_by {
            Some(reason) => reason.exit_code(),
            None if all_succeeded => EXIT_SUCCESS,
            None => EXIT_FAILURE,
        },
        unverified,
    })
}

/// Apply a run's switches to its config: `frozen_learning`
/// (`--frozen-learning`, decision 2218) freezes its learning and
/// `no_holdout` (`--no-holdout`, decision 4115) turns on maximize mode, for
/// this run alone. Without them the config's own `[learning] frozen` and
/// `[experiments] maximize` stand. Maximize mode, from either, holds nothing
/// out: it zeroes the plan-load spec gate's gate-off holdout
/// (`[spec_quality] holdout_frac`) too (gap-29fe0a).
pub fn apply_run_switches(
    config: &mut roko_core::config::schema::RokoConfig,
    frozen_learning: bool,
    no_holdout: bool,
) {
    if frozen_learning {
        config.learning.frozen = true;
    }
    if no_holdout {
        config.experiments.maximize = true;
    }
    if config.experiments.maximize {
        config.spec_quality.holdout_frac = 0.0;
    }
}

// ── Learning and feedback wiring ──────────────────────────────────────────

/// The learning and feedback wiring of a Graph plan run under `workdir`: the
/// feedback facade ([`build_graph_feedback_facade`]) and every learning store
/// a task attempt's feedback writes. `cascade_journal` is the journal the run
/// saves `cascade_router` through; `error_patterns` is the dispatch
/// factory's error-pattern store, which prompts read.
///
/// It builds the same feedback infrastructure that Runner-v2 used, so Graph
/// engine runs produce episodes, efficiency events, playbook outcomes,
/// routing observations, experiment settlements, and daimon feedback; their
/// verified passes also write playbooks (gap-89f985).
/// [`run_graph_plan`] wires its dispatcher with it, and the learning wiring
/// census (`tests/learning_wiring_census.rs`, S01 §4.8) inspects the same
/// object graph through `GraphTaskDispatcher::wiring_report`.
pub fn build_graph_feedback_context(
    workdir: &Path,
    config: &roko_core::config::schema::RokoConfig,
    cascade_router: Option<&Arc<roko_learn::cascade_router::CascadeRouter>>,
    cascade_journal: Option<&Arc<roko_learn::model_call_feedback::ModelCallJournal>>,
    error_patterns: &Arc<std::sync::RwLock<roko_learn::error_pattern_store::ErrorPatternStore>>,
    knowledge_batch: Option<&str>,
    publisher: Option<crate::state_hub::StateHubSender>,
) -> crate::graph_task_dispatch::GraphFeedbackContext {
    let graph_layout = RokoLayout::for_project(workdir);
    let graph_learn_dir = graph_layout.learn_dir();
    let _ = std::fs::create_dir_all(&graph_learn_dir);
    // A frozen run (decision 2218) sets none of the paths that only write
    // learned state: playbook outcomes (prompts read playbooks from the
    // workdir) and prompt treatments. Paths that are also read stay; their
    // writers check the flag.
    let learning = !config.learning.frozen;

    // #144: one daimon state, shared by the feedback facade (plan-completion
    // persistence) and dispatch (affect modulation).
    let shared_daimon_state = graph_daimon_state(workdir, config);

    // ── P0-04: CodingOracle ─────────────────────────────────────────────
    //
    // Persists across the plan run, accumulating build/test observations
    // for predictive gate feedback. Mirrors Runner-v2's CodingOracle.
    let coding_oracle = std::sync::Arc::new(roko_learn::oracles::coding::CodingOracle::new());
    // M3 (S04): the self-model, loaded when `[self_model] mode` is not off,
    // shadow by default (decision 2 of 2026-10-15, superseding decision
    // 6102 question 5); its outcome sink teaches it each settled verdict
    // (6129). Shadow forecasts and records every routed attempt; it never
    // changes the route (`routes()` needs `active`). A frozen run loads and
    // saves none of it, like M1's sink below.
    let self_model = learning
        .then(|| crate::graph_task_dispatch::self_model::SelfModelRuntime::load(workdir, config))
        .flatten();
    // M1 (S06, 8122): the ultrastable controller's sink, in shadow by default;
    // a frozen run writes no controller state or ledger.
    let homeostasis = if learning {
        crate::runtime_feedback::HomeostasisSink::for_workdir(workdir, config, self_model.as_ref())
            .map(Arc::new)
    } else {
        None
    };

    let feedback_facade = build_graph_feedback_facade(
        workdir,
        config,
        cascade_router,
        cascade_journal,
        shared_daimon_state.as_ref(),
        error_patterns,
        self_model.as_ref(),
        homeostasis.as_ref(),
        knowledge_batch,
        publisher,
    );
    // A test can listen to what a real run's facade hears (gap-a960a1).
    #[cfg(test)]
    let feedback_facade = tests::with_attached_sinks(workdir, feedback_facade);

    crate::graph_task_dispatch::GraphFeedbackContext {
        feedback_facade: Some(feedback_facade),
        efficiency_path: Some(graph_learn_dir.join("efficiency.jsonl")),
        costs_path: Some(graph_learn_dir.join("costs.jsonl")),
        playbook_dir: learning.then(|| graph_learn_dir.join("playbooks")),
        // Reuse the daimon state constructed above so the feedback facade
        // persistence sink and dispatch-time modulation share the same
        // mutable state (#144).
        daimon_state: shared_daimon_state,
        experiment_store_path: learning.then(|| graph_learn_dir.join("experiments.json")),
        gate_failures_path: Some(graph_layout.gate_failures_path()),
        coding_oracle: Some(coding_oracle),
        // P2-LRN-6 Loop 1: Gate threshold EMA updates after each task's
        // verify sequence. Uses the canonical workspace path so the TUI,
        // serve, and `roko learn gates` all read from the same file.
        gate_thresholds_path: Some(graph_layout.gate_thresholds_path()),
        // S01: every attempt's open line and verdict, per checkpoint run.
        runs_dir: Some(graph_layout.runs_dir()),
        // The run body attaches its runs' provenance sinks (gap-ca8022).
        provenance_sinks: None,
        // S02 L9: the section bandit's outcomes, saved when the run ends.
        section_outcomes: learning.then(Arc::default),
        self_model,
        homeostasis,
    }
}

/// Where a workspace's daimon affect state persists.
fn daimon_affect_path(workdir: &Path) -> PathBuf {
    workdir.join(".roko").join("daimon").join("affect.json")
}

/// #144: the daimon state a plan run shares between the feedback facade and
/// dispatch. `None` unless `[daimon] enabled` turns affect on (it is held,
/// dec-e70592, so off by default) and `strategy_space.dimensions` has
/// exactly 8 entries. Without it no attempt is appraised, routing gets the
/// neutral policy, and `.roko/daimon/affect.json` is neither read nor saved.
fn graph_daimon_state(
    workdir: &Path,
    config: &roko_core::config::schema::RokoConfig,
) -> Option<Arc<std::sync::Mutex<roko_daimon::DaimonState>>> {
    if !config.daimon.enabled {
        return None;
    }
    let dims_vec = &config.daimon.strategy_space.dimensions;
    if dims_vec.len() == 8 {
        // SAFETY: len == 8 is checked above, so try_into() is infallible here.
        let dims: [String; 8] = dims_vec
            .clone()
            .try_into()
            .expect("dims_vec has exactly 8 elements (checked above)");
        let def = roko_daimon::StrategySpaceDefinition {
            domain: config.daimon.strategy_space.domain.clone(),
            dimensions: dims,
        };
        let mut s = roko_daimon::DaimonState::load_or_new(&daimon_affect_path(workdir));
        let _ = s.configure_strategy_space(def);
        Some(std::sync::Arc::new(std::sync::Mutex::new(s)))
    } else {
        tracing::warn!(
            dims = dims_vec.len(),
            "daimon strategy_space.dimensions must have exactly 8 entries; skipping"
        );
        None
    }
}

/// The feedback facade of a Graph plan run: the sinks each settled task
/// attempt fans out to, in order (episodes, hindsight, verified knowledge,
/// verified playbooks, error patterns, routing when there is a cascade
/// router, and the daimon sink, the one that hears each plan's completion).
/// The routing sink journals its observations in `cascade_journal`, when
/// there is one, and the hindsight sink its retractions of them;
/// `daimon_state` is the state dispatch modulates, persisted when a plan
/// completes; `error_patterns` is the store dispatch formats into prompts;
/// `self_model` is the run's self-model, which its outcome sink teaches;
/// `homeostasis` is M1's sink (8122), registered after it.
pub fn build_graph_feedback_facade(
    workdir: &Path,
    config: &roko_core::config::schema::RokoConfig,
    cascade_router: Option<&Arc<roko_learn::cascade_router::CascadeRouter>>,
    cascade_journal: Option<&Arc<roko_learn::model_call_feedback::ModelCallJournal>>,
    daimon_state: Option<&Arc<std::sync::Mutex<roko_daimon::DaimonState>>>,
    error_patterns: &Arc<std::sync::RwLock<roko_learn::error_pattern_store::ErrorPatternStore>>,
    self_model: Option<&Arc<crate::graph_task_dispatch::self_model::SelfModelRuntime>>,
    homeostasis: Option<&Arc<crate::runtime_feedback::HomeostasisSink>>,
    knowledge_batch: Option<&str>,
    publisher: Option<crate::state_hub::StateHubSender>,
) -> Arc<crate::runtime_feedback::FeedbackFacade> {
    // A frozen run (decision 2218) registers no learning sink. Each one
    // writes learned state: episodes, hindsight, knowledge, playbooks, error
    // patterns, the router and the daimon state, so a frozen run creates no
    // playbook (gap-89f985) and saves no affect when a plan completes
    // (gap-a960a1). Its episodes go to each run's own
    // `runs/<run_id>/episodes.jsonl`, telemetry that no later run reads and
    // the bench driver's attempt evidence (gap-127263).
    if config.learning.frozen {
        let runs_dir = RokoLayout::for_project(workdir).runs_dir();
        let run_episodes = crate::runtime_feedback::EpisodeSink::per_run(runs_dir)
            .with_hdc_fingerprint(config.learning.episode_hdc_fingerprint);
        return std::sync::Arc::new(
            crate::runtime_feedback::FeedbackFacade::new()
                .with_sink(std::sync::Arc::new(run_episodes)),
        );
    }
    let graph_layout = RokoLayout::for_project(workdir);
    let graph_learn_dir = graph_layout.learn_dir();
    let graph_episodes_path = graph_layout.root_episodes_path();
    // The knowledge a guarded run admits joins its batch (8138).
    let verified_knowledge = crate::runtime_feedback::VerifiedKnowledgeSink::for_workdir(workdir);
    let verified_knowledge = match knowledge_batch {
        Some(batch) => verified_knowledge.with_commit_batch(batch),
        None => verified_knowledge,
    };
    // Gate-verified passes write L-play's playbooks, where prompts read
    // them (gap-89f985).
    let verified_playbooks =
        crate::runtime_feedback::VerifiedPlaybookSink::new(graph_learn_dir.join("playbooks"));
    // What each writes is published on the run's hub (gap-7576c7).
    let (verified_knowledge, verified_playbooks) = match publisher {
        Some(publisher) => (
            verified_knowledge.with_publisher(publisher.clone()),
            verified_playbooks.with_publisher(publisher),
        ),
        None => (verified_knowledge, verified_playbooks),
    };
    let mut facade = crate::runtime_feedback::FeedbackFacade::new()
        .with_sink(std::sync::Arc::new(
            crate::runtime_feedback::EpisodeSink::at(&graph_episodes_path)
                .with_hdc_fingerprint(config.learning.episode_hdc_fingerprint),
        ))
        // Reads back the failed episode the episode sink just wrote, so
        // it must follow it.
        .with_sink(std::sync::Arc::new(
            crate::runtime_feedback::HindsightSink::new(
                &graph_episodes_path,
                graph_learn_dir.join(roko_learn::hindsight::DEFAULT_ADJUSTMENTS_FILE),
            )
            .with_router(cascade_router.cloned())
            .with_journal(cascade_journal.cloned()),
        ))
        // Gate-verified attempts grow durable knowledge (tier
        // progression included) under `.roko/neuro/`, and playbooks under
        // `.roko/learn/playbooks/`.
        .with_sink(std::sync::Arc::new(verified_knowledge))
        .with_sink(std::sync::Arc::new(verified_playbooks))
        // A failure of the agent's work goes into the error-pattern store
        // dispatch formats into prompts, and to `learn/error-patterns.json`.
        .with_sink(std::sync::Arc::new(
            crate::runtime_feedback::ErrorPatternSink::new(
                std::sync::Arc::clone(error_patterns),
                graph_learn_dir.join("error-patterns.json"),
            ),
        ));
    if let Some(cascade) = cascade_router {
        let mut routing = crate::runtime_feedback::RoutingObservationSink::new(cascade.clone());
        if let Some(journal) = cascade_journal {
            routing = routing.with_journal(Arc::clone(journal));
        }
        facade = facade.with_sink(std::sync::Arc::new(routing));
    }
    // M3 (S04, 6129): each settled verdict teaches the self-model what it
    // forecast before routing.
    if let Some(runtime) = self_model {
        facade = facade.with_sink(std::sync::Arc::new(
            crate::runtime_feedback::SelfModelOutcomeSink::new(Arc::clone(runtime)),
        ));
    }
    // M1 (S06, 8122): each settled verdict feeds the ultrastable controller.
    if let Some(sink) = homeostasis {
        facade =
            facade.with_sink(Arc::clone(sink) as Arc<dyn crate::runtime_feedback::FeedbackSink>);
    }

    // ── #144: Daimon affect persistence on plan completion ──────────
    //
    // The one sink that hears a plan's completion (q-6b7cca option 2,
    // gap-a960a1). No dream starts when a plan completes: dreams run on
    // demand, through `roko knowledge dream run`. The parked cognitive
    // clock's theta and delta sinks are not registered either.
    if let Some(daimon) = daimon_state {
        facade = facade.with_sink(std::sync::Arc::new(
            crate::runtime_feedback::DaimonPersistenceSink::new(
                daimon_affect_path(workdir),
                std::sync::Arc::clone(daimon),
            ),
        ));
    }

    std::sync::Arc::new(facade)
}

// ── Running the plans of a set ────────────────────────────────────────────

/// How often the plan-set driver and each running plan poll for stop
/// requests and operator commands.
const PLAN_WATCH_INTERVAL: Duration = Duration::from_millis(100);

/// Run-wide state every plan of the set runs with.
struct PlanRunContext<'a> {
    workdir: &'a Path,
    /// Where each plan registers its run's safety provenance sink
    /// (gap-ff95f5).
    provenance_sinks: &'a crate::safety_provenance::ProvenanceSinks,
    resume_plan: Option<&'a Path>,
    plan_count: usize,
    fresh: bool,
    force_resume: bool,
    max_retries: Option<u32>,
    max_tasks: usize,
    rich_topology: bool,
    /// `--worktree-per-task`: each task writes its own checkout.
    worktree_per_task: bool,
    /// Services the cells of each plan's graph run with (see
    /// [`plan_cell_resources`]).
    cell_resources: &'a roko_graph::cell::CellResources,
    /// The run's batch branch, under `--worktree-per-task` (spec-f830c4).
    batch: Option<&'a super::batch::BatchIntegration>,
    /// Each plan's whole-plan check (gap-60233f), by plan id.
    plan_checks: &'a HashMap<String, Vec<crate::task_parser::VerifyStep>>,
    /// `[gates] env_passthrough`: what the whole-plan check's steps (and the
    /// delivery regression that runs them) inherit beyond the gate allowlist.
    gate_env_passthrough: &'a [String],
    /// The attempt checkouts' manager, under `--worktree-per-task`.
    worktrees: Option<&'a crate::orchestrator::worktree::WorktreeManager>,
    /// `[runner] delete_attempt_branches`: a delivered plan's attempt
    /// branches go with their checkouts (gap-415c54).
    delete_attempt_branches: bool,
    quiet: bool,
    json: bool,
    launch_tui: bool,
    task_dispatcher: &'a Arc<dyn roko_graph::cells::TaskDispatcher>,
    graph_task_dispatcher: &'a Arc<crate::graph_task_dispatch::GraphTaskDispatcher>,
    /// The run's feedback facade, which hears each plan close
    /// ([`close_plan`]).
    feedback: Option<&'a Arc<crate::runtime_feedback::FeedbackFacade>>,
    /// `[conductor] plan_failure_policy`; a plan's `[meta] failure_policy`
    /// overrides it.
    plan_failure_policy: roko_core::config::PlanFailurePolicy,
    graph_tui_bridge: &'a crate::runner::graph_tui_bridge::GraphTuiBridge,
    graph_telemetry: &'a Arc<dyn roko_core::TelemetryEventSink>,
    shared_pause_flag: &'a Arc<AtomicBool>,
    interrupt: &'a PlanRunInterruptHandle,
    /// Each checkpoint run's `manifest.json` (S01 §5.1).
    run_manifests: &'a super::run_manifest::RunManifests,
    /// `[learning]`, for the loop auditor's tick at each run's close (5126):
    /// its `[learning.audit]`, and whether learning is frozen, which skips
    /// the tick.
    learning: &'a roko_core::config::learning::LearningConfig,
    /// The run id the caller already gave this run (`roko run`); a single
    /// plan's fresh checkpoint takes it.
    caller_run_id: Option<&'a str>,
    /// Files a GitHub issue for each task a plan leaves failed, when
    /// `[github] auto_pr` is on (gap-cd51b7).
    failure_issues: Option<&'a super::failure_issues::FailureIssues>,
    /// The plan-load spec gate's decisions, which each plan's run records
    /// before its first task starts (3231).
    spec_gate: &'a crate::spec_gate::SpecGateReport,
}

/// Services the cells of a plan's graph run with (gap-6daad9). The rich
/// topology's `plan.gate` cells run the gates, with the run's `[gates]`,
/// and settle the worktree each task executor hands on through `workspaces`,
/// the provider the executors acquire them from. The default topology needs
/// neither.
fn plan_cell_resources(
    rich_topology: bool,
    gates: &roko_core::config::GatesConfig,
    workspaces: Option<Arc<dyn roko_graph::workspace::ExecutionWorkspaceProvider>>,
) -> roko_graph::cell::CellResources {
    plan_cell_resources_with(
        rich_topology,
        gates,
        workspaces,
        Arc::new(roko_gate::production_service::ProductionGateService::new()),
    )
}

/// [`plan_cell_resources`], with the gate pipeline `service` the rich
/// topology's gates run on.
fn plan_cell_resources_with(
    rich_topology: bool,
    gates: &roko_core::config::GatesConfig,
    workspaces: Option<Arc<dyn roko_graph::workspace::ExecutionWorkspaceProvider>>,
    service: Arc<dyn roko_gate::production_service::ProductionGateRunner>,
) -> roko_graph::cell::CellResources {
    if !rich_topology {
        return roko_graph::cell::CellResources::default();
    }
    roko_graph::cell::CellResources {
        gates: Some(Arc::new(
            crate::runner::gate_adapter::RunnerProductionGateAdapter::new(service)
                .with_gates_config(gates.clone()),
        )),
        workspaces,
    }
}

/// The context a plan's graph runs in: its checkpoint run, the run's shared
/// pause flag, and the cell services.
fn plan_cell_context(
    run_id: &str,
    pause_flag: &Arc<AtomicBool>,
    stop_flag: &Arc<AtomicBool>,
    resources: &roko_graph::cell::CellResources,
) -> roko_graph::cell::CellContext {
    roko_graph::cell::CellContext::new()
        .with_run_id(run_id.to_string())
        .with_pause_flag(Arc::clone(pause_flag))
        .with_cancel_flag(Arc::clone(stop_flag))
        .with_resources(resources.clone())
}

/// Close checkpoint run `run_id`'s attempt log, then record in its manifest
/// that it ended with `status` and how many attempts it opened and settled.
/// The loop auditor's tick then reads the flushed rows (5126), unless
/// learning is frozen.
fn close_run_manifest(ctx: &PlanRunContext<'_>, run_id: &str, status: GraphCheckpointStatus) {
    let writer = ctx.graph_task_dispatcher.close_run_attempts(run_id);
    ctx.run_manifests.close(run_id, status, writer);
    super::loop_audit::audit_tick(ctx.workdir, ctx.learning, run_id, ctx.graph_tui_bridge);
}

/// Close plan `plan_id`, which [`run_one_plan`] ran and which ended
/// `succeeded` with its tasks settled as `tasks`. The run's feedback facade
/// hears it first, as one [`FeedbackEvent::PlanCompleted`] (q-6b7cca option
/// 2, gap-a960a1), then the live views do; so with affect on, the daimon
/// sink has saved the affect state by the time a live view sees the plan
/// close. The event counts the tasks as the plan's run-metrics row does
/// ([`plan_metrics`]) and carries the plan's settled spend. A plan that never
/// started (blocked, or cancelled before it started) is closed by the
/// plan-set driver and sends no event; a plan the operator runs again closes
/// once per run.
///
/// [`FeedbackEvent::PlanCompleted`]: crate::runtime_feedback::FeedbackEvent::PlanCompleted
async fn close_plan(
    ctx: &PlanRunContext<'_>,
    plan_id: &str,
    succeeded: bool,
    tasks: TaskVerdictCounts,
) {
    if let Some(facade) = ctx.feedback {
        let event = crate::runtime_feedback::FeedbackEvent::PlanCompleted {
            plan_id: plan_id.to_string(),
            succeeded,
            tasks_completed: tasks.passed,
            tasks_failed: tasks.failed,
            total_cost_usd: ctx
                .graph_task_dispatcher
                .plan_budget_snapshot(plan_id)
                .spent_usd,
        };
        if let Err(error) = facade.on_event(&event).await {
            tracing::warn!(plan_id, %error, "plan completion feedback failed (non-fatal)");
        }
    }
    ctx.graph_tui_bridge.plan_completed(plan_id, succeeded);
}

/// Operator controls the plan-set driver routes to one running plan.
#[derive(Debug, Clone, Default)]
struct PlanControl {
    /// Set by a TUI cancel aimed at this plan, or at every running plan.
    cancel: Arc<AtomicBool>,
    /// Set when the operator runs the plan again after it failed or was
    /// cancelled earlier in this run (gap-c002bb).
    rerun: Option<PlanRerun>,
}

/// How the operator asked to run a plan again ([`route_execution_commands`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlanRerun {
    /// Soft retry, or repair that keeps completed work: resume the plan's
    /// checkpoint, so the tasks that passed stay done and the rest run again.
    Resume,
    /// Reset, or a clean repair: archive the plan's checkpoint, as `--fresh`
    /// does, and run every task again.
    Fresh,
}

impl PlanRerun {
    /// What the run does, for the TUI's acknowledgement and the event log.
    fn describe(self, plan_id: &str) -> String {
        match self {
            Self::Resume => format!(
                "plan '{plan_id}' runs again from its checkpoint: the tasks that passed stay done"
            ),
            Self::Fresh => {
                format!("plan '{plan_id}' runs again from scratch: its checkpoint is archived")
            }
        }
    }
}

/// What [`route_execution_commands`] changed in the plan set.
#[derive(Debug, Default)]
struct RoutedCommands {
    /// Plans cancelled before they started.
    cancelled_before_start: Vec<String>,
    /// Plans the operator runs again, each with how
    /// ([`PlanSetScheduler::retry`]).
    reruns: Vec<(String, PlanRerun)>,
    /// Running plans whose budget ceiling the operator raised.
    budget_raises: Vec<RaisedBudget>,
}

impl RoutedCommands {
    /// Add what another channel's routing changed.
    fn merge(&mut self, other: Self) {
        self.cancelled_before_start
            .extend(other.cancelled_before_start);
        self.reruns.extend(other.reruns);
        self.budget_raises.extend(other.budget_raises);
    }
}

/// A raise of a running plan's budget ceiling that
/// [`route_execution_commands`] applied (backlog 2118).
#[derive(Debug)]
struct RaisedBudget {
    plan_id: String,
    raise: crate::graph_task_dispatch::PlanBudgetRaise,
    /// Who asked: the control surface the raise came through.
    requested_by: String,
}

impl RaisedBudget {
    /// What the event log says of it.
    fn describe(&self) -> String {
        let Self {
            plan_id,
            raise,
            requested_by,
        } = self;
        format!(
            "plan '{plan_id}' budget ceiling raised from ${:.4} to ${:.4} by {requested_by}, \
             with ${:.4} spent",
            raise.from_usd, raise.to_usd, raise.spent_usd
        )
    }
}

/// How one plan's run ended.
struct PlanRunResult {
    outcome: PlanOutcome,
    output_count: usize,
    /// How the plan's tasks settled.
    tasks: TaskVerdictCounts,
    /// The work it left on its plan branch that no check ran on, when it was
    /// not delivered (bug-db89d4).
    unverified_work: Option<UnverifiedWork>,
}

impl PlanRunResult {
    /// A plan that failed before any of its `task_count` tasks ran.
    const fn failed(task_count: usize) -> Self {
        Self {
            outcome: PlanOutcome::Failed,
            output_count: 0,
            tasks: TaskVerdictCounts::not_run(task_count),
            unverified_work: None,
        }
    }
}

/// Work a plan that was not delivered left on its plan branch, which no
/// check ran on (bug-db89d4). It is never delivered; the run's summary says
/// where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
struct UnverifiedWork {
    /// The plan branch, `roko/plan/<plan>`.
    branch: String,
    /// The commit the work ends at.
    commit: String,
    /// The commit it starts from.
    base: String,
    /// The tasks whose work it holds.
    tasks: Vec<String>,
}

impl UnverifiedWork {
    /// The unverified part of `work`, run `plan_id`'s accepted work, if any.
    fn of(plan_id: &str, work: &super::batch::PlanWork) -> Option<Self> {
        let unverified = work.unverified();
        let mut tasks = Vec::new();
        for accepted in unverified {
            if !tasks.contains(&accepted.task_id) {
                tasks.push(accepted.task_id.clone());
            }
        }
        Some(Self {
            branch: crate::orchestrator::worktree::format_branch_name(plan_id),
            commit: unverified.last()?.commit.clone(),
            base: work.verified_tip().unwrap_or(&work.base).to_string(),
            tasks,
        })
    }

    /// The command that shows the work: its diff from where it starts.
    fn review_command(&self) -> String {
        format!("git diff {}..{}", short_oid(&self.base), self.branch)
    }
}

/// The first 12 characters of an object id, as a person types it.
fn short_oid(oid: &str) -> &str {
    oid.get(..12).unwrap_or(oid)
}

/// Outcome of a plan that ran. A graph that ran to completion settles by its
/// tasks' verdicts ([`TaskVerdictCounts::outcome`]); otherwise a stop request
/// outranks an operator cancel, which outranks a failure.
const fn plan_outcome(
    execution_succeeded: bool,
    tasks: TaskVerdictCounts,
    interrupted: bool,
    cancelled_by_operator: bool,
) -> PlanOutcome {
    if execution_succeeded {
        tasks.outcome()
    } else if interrupted {
        PlanOutcome::Interrupted
    } else if cancelled_by_operator {
        PlanOutcome::Cancelled
    } else {
        PlanOutcome::Failed
    }
}

/// Whether no plan of the run accepted anything onto its plan branch in its
/// run (bug-58329f), so that the run left no work for `batch` to hold.
async fn run_accepted_nothing(
    batch: &super::batch::BatchIntegration,
    plans: &[crate::runner::plan_loader::Plan],
    dispatcher: &crate::graph_task_dispatch::GraphTaskDispatcher,
) -> bool {
    for plan in plans {
        let Some(run_id) = dispatcher.plan_run_id(&plan.id) else {
            continue;
        };
        if super::batch::PlanWork::read(batch.repo(), &plan.id, &run_id)
            .await
            .is_some()
        {
            return false;
        }
    }
    true
}

/// How many plans ended each way, in a fixed order and leaving out the ways
/// none did, such as "1 succeeded, 1 unverified" (bug-db89d4). `None` is a
/// plan that never started.
fn outcome_tally(outcomes: &[Option<PlanOutcome>]) -> String {
    const LABELS: [Option<PlanOutcome>; 7] = [
        Some(PlanOutcome::Succeeded),
        Some(PlanOutcome::Unverified),
        Some(PlanOutcome::Failed),
        Some(PlanOutcome::Blocked),
        Some(PlanOutcome::Cancelled),
        Some(PlanOutcome::Interrupted),
        None,
    ];
    LABELS
        .iter()
        .filter_map(|label| {
            let count = outcomes.iter().filter(|outcome| *outcome == label).count();
            (count > 0).then(|| {
                let word = label.map_or("not started", PlanOutcome::label);
                format!("{count} {word}")
            })
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Publish that a plan will never start.
fn report_blocked_plan(
    graph_tui_bridge: &crate::runner::graph_tui_bridge::GraphTuiBridge,
    plan_id: &str,
    reason: &BlockReason,
) {
    let message = match reason {
        BlockReason::Prerequisites(prerequisites) => {
            tracing::warn!(
                plan_id,
                prerequisites = prerequisites.join(", "),
                "plan blocked: prerequisite plan(s) did not succeed"
            );
            format!(
                "plan '{plan_id}' blocked: prerequisites {}",
                prerequisites.join(", ")
            )
        }
        BlockReason::FailFast => {
            tracing::warn!(
                plan_id,
                "plan not started: an earlier plan failed (--fail-fast)"
            );
            format!("plan '{plan_id}' not started: an earlier plan failed (--fail-fast)")
        }
    };
    graph_tui_bridge.log_event("graph.plan_blocked", &message);
    graph_tui_bridge.plan_completed(plan_id, false);
}

/// Drain pending TUI commands (gap-c002bb).
///
/// - Cancel reaches the plan it names when that plan is running, drops it
///   when it has not started, and reaches every running plan when it names
///   none.
/// - Pause and resume set the pause flag every plan shares. Pause holds: no
///   plan or attempt starts until resume, and running attempts finish.
/// - Skip stops the running agent of the task it names (`task_stops`); that
///   task fails as stopped by the operator, and its plan runs on.
/// - Soft retry, repair and reset run again a plan that failed or was
///   cancelled earlier in this run ([`PlanRerun`]).
/// - Inject (`roko inject`) queues an operator directive or context for the
///   next task of the running plan it names (`directives`, gap-f118b3).
/// - Approve and reject record a reviewer's decision on the task held for
///   review that the approval id names (`<plan>/<task>`), as `roko plan
///   review` does; the held attempt in `workdir` reads it
///   ([`record_held_task_review`]).
/// - A budget raise (`roko plan budget raise`) lifts the ceiling of the
///   running plan it names for the rest of its run (`budget`, backlog 2118).
///
/// Every other command, and one these cannot carry out, is rejected with
/// the reason ([`reject_command`]): none is acknowledged and then dropped.
fn route_execution_commands(
    commands: &mut tokio::sync::mpsc::Receiver<crate::execution_control::ExecutionCommand>,
    acks: &tokio::sync::mpsc::Sender<crate::execution_control::CommandAck>,
    controls: &std::collections::HashMap<String, PlanControl>,
    scheduler: &mut PlanSetScheduler,
    pause: &AtomicBool,
    task_stops: &crate::graph_task_dispatch::OperatorStops,
    directives: &crate::graph_task_dispatch::OperatorDirectives,
    budget: &crate::graph_task_dispatch::PlanBudgetControl,
    workdir: &Path,
) -> RoutedCommands {
    let mut routed = RoutedCommands::default();
    while let Ok(cmd) = commands.try_recv() {
        let (status, note) = match &cmd.kind {
            ExecutionCommandKind::Cancel => match cmd.plan_id.as_deref() {
                Some(plan_id) => {
                    if let Some(control) = controls.get(plan_id) {
                        control.cancel.store(true, Ordering::Release);
                        (CommandAckStatus::Completed, None)
                    } else if scheduler.cancel_pending(plan_id) {
                        routed.cancelled_before_start.push(plan_id.to_string());
                        (CommandAckStatus::Completed, None)
                    } else {
                        reject_command(&cmd, &format!("plan '{plan_id}' is not running"))
                    }
                }
                None => {
                    for control in controls.values() {
                        control.cancel.store(true, Ordering::Release);
                    }
                    (CommandAckStatus::Completed, None)
                }
            },
            ExecutionCommandKind::Pause => {
                pause.store(true, Ordering::Release);
                let note = "paused: no new task starts until resume; running attempts finish";
                tracing::info!(command_id = %cmd.command_id, "{note}");
                (CommandAckStatus::Completed, Some(note.to_string()))
            }
            ExecutionCommandKind::Resume => {
                pause.store(false, Ordering::Release);
                let note = "resumed: tasks start again";
                tracing::info!(command_id = %cmd.command_id, "{note}");
                (CommandAckStatus::Completed, Some(note.to_string()))
            }
            ExecutionCommandKind::SoftRetry
            | ExecutionCommandKind::Repair { .. }
            | ExecutionCommandKind::Reset => {
                let rerun = match cmd.kind {
                    ExecutionCommandKind::Reset
                    | ExecutionCommandKind::Repair {
                        preserve_completed: false,
                    } => PlanRerun::Fresh,
                    _ => PlanRerun::Resume,
                };
                match cmd.plan_id.as_deref() {
                    Some(plan_id) if scheduler.retry(plan_id) => {
                        routed.reruns.push((plan_id.to_string(), rerun));
                        (CommandAckStatus::Accepted, Some(rerun.describe(plan_id)))
                    }
                    Some(plan_id) => reject_command(&cmd, &rerun_refusal(scheduler, plan_id)),
                    None => reject_command(&cmd, "name the plan to run again"),
                }
            }
            ExecutionCommandKind::ReverifyGates => reject_command(
                &cmd,
                "re-verifying gates is not available during a Graph run",
            ),
            ExecutionCommandKind::Skip => match (cmd.plan_id.as_deref(), cmd.task_id.as_deref()) {
                (Some(plan_id), Some(task_id)) if task_stops.stop(plan_id, task_id) => (
                    CommandAckStatus::Completed,
                    Some(format!(
                        "stopped the agent of {plan_id}/{task_id}: the task fails as stopped by \
                         the operator, and its plan runs on"
                    )),
                ),
                (Some(plan_id), Some(task_id)) => reject_command(
                    &cmd,
                    &format!("{plan_id}/{task_id} has no running agent to stop"),
                ),
                _ => reject_command(&cmd, "name the plan and task whose agent to stop"),
            },
            ExecutionCommandKind::Inject { kind, text } => match cmd.plan_id.as_deref() {
                Some(plan_id) if controls.contains_key(plan_id) => {
                    match directives.queue(plan_id, &cmd.command_id, *kind, text.as_str()) {
                        Ok(_) => (
                            CommandAckStatus::Accepted,
                            Some(format!(
                                "waits for the next task of plan '{plan_id}' to start"
                            )),
                        ),
                        Err(reason) => reject_command(&cmd, &reason),
                    }
                }
                Some(plan_id) => reject_command(&cmd, &format!("plan '{plan_id}' is not running")),
                None => reject_command(&cmd, "name the running plan to send it to"),
            },
            ExecutionCommandKind::Approve { approval_id } => {
                record_held_task_review(workdir, &cmd, approval_id, "approved", "")
            }
            ExecutionCommandKind::RejectApproval {
                approval_id,
                reason,
            } => record_held_task_review(workdir, &cmd, approval_id, "rejected", reason),
            ExecutionCommandKind::RaiseBudget {
                ceiling_micro_usd,
                requested_by,
            } => match cmd.plan_id.as_deref() {
                Some(plan_id) if controls.contains_key(plan_id) => {
                    match budget.raise(plan_id, *ceiling_micro_usd) {
                        Ok(raise) => {
                            let raised = RaisedBudget {
                                plan_id: plan_id.to_string(),
                                raise,
                                requested_by: requested_by.clone(),
                            };
                            let note = raised.describe();
                            tracing::info!(command_id = %cmd.command_id, "{note}");
                            routed.budget_raises.push(raised);
                            (CommandAckStatus::Completed, Some(note))
                        }
                        Err(reason) => reject_command(&cmd, &reason),
                    }
                }
                Some(plan_id) => reject_command(&cmd, &format!("plan '{plan_id}' is not running")),
                None => reject_command(&cmd, "name the running plan whose ceiling to raise"),
            },
        };
        if matches!(cmd.kind, ExecutionCommandKind::Cancel) {
            tracing::info!(
                command_id = %cmd.command_id,
                plan_id = ?cmd.plan_id,
                "TUI cancel"
            );
        }
        let _ = acks.try_send(ack_for(&cmd, status, note));
    }
    routed
}

/// Record a reviewer's `decision` (`approved` or `rejected`) with `note` on
/// the task held for review in `workdir` that `approval_id` names as
/// `<plan>/<task>`, as `roko plan review` does (gap-0d64d5); the held attempt
/// reads it from the review log. Rejected, with the reason, when the id names
/// no task waiting for a review.
fn record_held_task_review(
    workdir: &Path,
    cmd: &crate::execution_control::ExecutionCommand,
    approval_id: &str,
    decision: &str,
    note: &str,
) -> (CommandAckStatus, Option<String>) {
    let Some((plan_id, task_id)) = approval_id.rsplit_once('/') else {
        let reason = format!("approval `{approval_id}` names no held task: use `<plan>/<task>`");
        return reject_command(cmd, &reason);
    };
    match crate::graph_task_dispatch::record_review(workdir, plan_id, task_id, decision, note) {
        Ok(attempt_key) => {
            let note = format!("{decision} attempt {attempt_key} of {plan_id}/{task_id}");
            tracing::info!(command_id = %cmd.command_id, "{note}");
            (CommandAckStatus::Completed, Some(note))
        }
        Err(error) => reject_command(cmd, &error.to_string()),
    }
}

/// Why plan `plan_id` cannot run again now ([`PlanSetScheduler::retry`]).
fn rerun_refusal(scheduler: &PlanSetScheduler, plan_id: &str) -> String {
    if scheduler.is_running(plan_id) {
        return format!("plan '{plan_id}' is running; cancel it first");
    }
    match scheduler.outcome(plan_id) {
        Some(PlanOutcome::Succeeded | PlanOutcome::Unverified) => {
            format!("plan '{plan_id}' did not fail")
        }
        Some(PlanOutcome::Blocked) => {
            format!("plan '{plan_id}' is blocked: a plan it depends on did not succeed")
        }
        Some(PlanOutcome::Failed | PlanOutcome::Cancelled | PlanOutcome::Interrupted) => {
            "the plan run is stopping".to_string()
        }
        None => format!("plan '{plan_id}' has not run yet"),
    }
}

/// The acknowledgement of a TUI command that takes no effect: rejected with
/// `reason`, which the TUI shows as a warning.
fn reject_command(
    cmd: &crate::execution_control::ExecutionCommand,
    reason: &str,
) -> (CommandAckStatus, Option<String>) {
    tracing::info!(
        command_id = %cmd.command_id,
        kind = %cmd.kind,
        reason,
        "TUI command rejected"
    );
    (CommandAckStatus::Rejected, Some(reason.to_string()))
}

/// Forward the command `roko plan pause/resume/cancel/retry` wrote to
/// `<state_dir>/control.json`, if any, into the run's command channel, where
/// [`route_execution_commands`] routes it like a TUI command (bug-8208a6).
/// The file is consumed.
fn forward_control_file(state_dir: &Path, commands: &ExecutionCommandSender) {
    let Some(control) = crate::runner::types::ControlCommand::poll(state_dir) else {
        return;
    };
    let command = crate::execution_control::control_command_to_execution(&control, "graph-engine");
    if let Err(error) = commands.try_send(command) {
        tracing::warn!(%error, "could not route a plan control command to the run");
    }
}

/// [`run_one_plan`], tagged with the plan's ID for the plan-set driver.
async fn run_admitted_plan(
    ctx: &PlanRunContext<'_>,
    plan: &crate::runner::plan_loader::Plan,
    control: PlanControl,
) -> (String, anyhow::Result<PlanRunResult>) {
    (plan.id.clone(), run_one_plan(ctx, plan, &control).await)
}

/// Repair what a crashed run left behind before the first dispatch
/// (gap-4ec59f): an unowned repository mutation lock file, stale
/// `index.lock` files and `git worktree` metadata whose checkout is gone.
/// A repair that fails is logged, and the run goes on without it.
async fn repair_worktree_state(worktrees: &crate::orchestrator::worktree::WorktreeManager) {
    let manager = worktrees.clone();
    let cleared = tokio::task::spawn_blocking(move || {
        if let Err(error) = manager.clear_stuck_mutation_lock() {
            tracing::warn!(%error, "could not check for a stuck worktree mutation lock");
        }
        manager.clear_stale_locks()
    })
    .await;
    match cleared {
        Ok(Ok(cleared)) => {
            for lock in cleared {
                tracing::warn!(
                    path = %lock.display(),
                    "removed a stale git index.lock left by a crashed run"
                );
            }
        }
        Ok(Err(error)) => tracing::warn!(%error, "could not clear stale git index.lock files"),
        Err(error) => tracing::warn!(%error, "the stale git lock repair did not finish"),
    }
    match worktrees.prune().await {
        Ok(_) => tracing::debug!("pruned git worktree metadata whose checkout is gone"),
        Err(error) => tracing::warn!(%error, "could not prune stale git worktree metadata"),
    }
}

/// With `--worktree-per-task` each task writes its own checkout, so no two
/// tasks share a tree: drop the exclusive paths that keep tasks writing the
/// same files apart, and let them run together (gap-19e596). The paths are
/// not part of the checkpoint identity.
fn drop_exclusion_for_worktrees(graph: &mut roko_graph::Graph, worktree_per_task: bool) {
    if !worktree_per_task {
        return;
    }
    for node in graph.inner.node_weights_mut() {
        node.exclusive.clear();
    }
}

/// `cell`, reading the live `budgets` before each retry when M1 moves them
/// (B2, 8126).
fn with_live_budgets(
    cell: roko_graph::cells::TaskExecutorCell,
    budgets: Option<&Arc<dyn roko_graph::cells::RetryBudgetSource>>,
) -> roko_graph::cells::TaskExecutorCell {
    match budgets {
        Some(budgets) => cell.with_retry_budget(Arc::clone(budgets)),
        None => cell,
    }
}

/// Run one admitted plan to a terminal checkpoint.
///
/// Every plan it runs closes once ([`close_plan`]): its PlanCompleted reaches
/// the run's feedback facade and the live views, even when the plan cannot be
/// converted or validated. `Err` is reserved for checkpoint and budget-ledger
/// failures, which stop the whole run.
async fn run_one_plan(
    ctx: &PlanRunContext<'_>,
    plan: &crate::runner::plan_loader::Plan,
    control: &PlanControl,
) -> anyhow::Result<PlanRunResult> {
    use roko_graph::cells::TaskExecutorCell;
    use roko_graph::engine::GraphEngine;

    let graph_tui_bridge = ctx.graph_tui_bridge;
    if !ctx.quiet && !ctx.json && !ctx.launch_tui {
        tracing::info!(
            plan_id = %plan.id,
            task_count = plan.tasks.tasks.len(),
            "running plan via Graph Engine"
        );
    }

    // Convert Runner v2 tasks into PlanTaskInfo for the converter, with the
    // mapping the resume preview uses (gap-be7368). Each task's retry budget
    // is `--max-retries`, else what it authors, else set by the adaptive gate
    // thresholds.
    let retry_budgets = ctx.graph_task_dispatcher.task_retry_budgets(&plan.dir);
    let tasks = crate::graph_checkpoint::plan_task_infos(plan, |t| {
        ctx.max_retries
            .unwrap_or_else(|| retry_budgets.max_retries(&plan.id, t))
    });
    // M1's sink resolves a failed chain at the attempt its budget ends on.
    if let Some(sink) = ctx.graph_task_dispatcher.homeostasis_sink() {
        let limits = tasks
            .iter()
            .map(|(task_id, info)| (task_id.clone(), info.max_retries));
        sink.set_retry_limits(&plan.id, limits);
    }
    // M1 moves the budgets the tasks do not author during the run (B2,
    // 8126); `--max-retries` fixes every budget.
    let live_budgets =
        if ctx.max_retries.is_none() && ctx.graph_task_dispatcher.homeostasis_sink().is_some() {
            Some(Arc::clone(ctx.graph_task_dispatcher).live_retry_budgets(retry_budgets))
        } else {
            None
        };

    // An omitted `max_parallel` converts as 1, as it did before it meant "as
    // wide as the DAG allows" (gap-272448): the checkpoint identity hashes
    // the converted concurrency. `--max-tasks` and the width are applied once
    // the identity is taken, below, so a run resumes whatever `--max-tasks`
    // it is given (gap-7147bb).
    let max_parallel = crate::graph_checkpoint::converted_max_parallel(plan);
    let max_parallel_usize = usize::try_from(max_parallel.max(1)).unwrap_or(usize::MAX);
    let plan_dir_str = plan.dir.display().to_string();

    let (mut graph, registry) = if ctx.rich_topology {
        // ── Rich 5-node-per-task production topology ───────────────────
        if !ctx.quiet && !ctx.json {
            tracing::info!(
                "--rich-topology is active: each task runs as task-context, compose, executor, \
                 gate and success-boundary nodes"
            );
        }
        let topo =
            roko_graph::ProductionPlanTopology::new(&plan.id, &plan_dir_str, max_parallel_usize);
        // Convert PlanTaskInfo -> TopologyTaskInfo.
        // Note: info.max_retries is already resolved (max_retries.unwrap_or(t.max_retries))
        // during the PlanTaskInfo construction above, so we use it directly.
        let topo_tasks: Vec<roko_graph::TopologyTaskInfo> = tasks
            .iter()
            .map(|(id, info)| roko_graph::TopologyTaskInfo {
                task_id: id.clone(),
                title: info.title.clone(),
                description: info.description.clone(),
                role: info.role.clone(),
                tier: info.tier.clone(),
                model_hint: info.model_hint.clone(),
                files: info.files.clone(),
                depends_on: info.depends_on.clone(),
                timeout_secs: info.timeout_secs,
                max_retries: info.max_retries,
                domain: info.domain.clone(),
                sequence: info.sequence,
                full_config_json: info.full_config_json.clone(),
            })
            .collect();
        match topo.build(&topo_tasks) {
            Ok((g, _report)) => {
                let mut reg = roko_graph::default_registry();
                // Register stub passthrough cells for all topology node types.
                roko_graph::register_topology_cells(&mut reg);
                let plan_dispatcher = Arc::clone(ctx.task_dispatcher);
                reg.register("task-executor", move |config| {
                    let cell = TaskExecutorCell::live(config, Arc::clone(&plan_dispatcher));
                    Box::new(with_live_budgets(cell, live_budgets.as_ref()))
                });
                (g, reg)
            }
            Err(e) => {
                graph_tui_bridge.error(&format!(
                    "failed to build rich topology for plan '{}': {e}",
                    plan.id
                ));
                tracing::error!(plan_id = %plan.id, error = %e, "failed to build rich topology for plan");
                let result = PlanRunResult::failed(tasks.len());
                close_plan(ctx, &plan.id, false, result.tasks).await;
                return Ok(result);
            }
        }
    } else {
        // ── Simple single-Activity-per-task converter (default) ─────────
        match crate::graph_checkpoint::convert_plan(plan, &tasks) {
            Ok(g) => {
                let mut reg = roko_graph::default_registry();
                let plan_dispatcher = Arc::clone(ctx.task_dispatcher);
                reg.register("task-executor", move |config| {
                    let cell = TaskExecutorCell::live(config, Arc::clone(&plan_dispatcher));
                    Box::new(with_live_budgets(cell, live_budgets.as_ref()))
                });
                (g, reg)
            }
            Err(e) => {
                // Text, not the `&dyn Error` itself, which is not `Send` and
                // would be held across the close below.
                let e = e.root_cause().to_string();
                graph_tui_bridge.error(&format!(
                    "failed to convert plan '{}' to graph: {e}",
                    plan.id
                ));
                tracing::error!(plan_id = %plan.id, error = %e, "failed to convert plan to graph");
                let result = PlanRunResult::failed(tasks.len());
                close_plan(ctx, &plan.id, false, result.tasks).await;
                return Ok(result);
            }
        }
    };
    drop_exclusion_for_worktrees(&mut graph, ctx.worktree_per_task);
    // A plan the operator runs again resumes its checkpoint, or archives it
    // to start over, whatever the run's `--fresh` (gap-c002bb).
    let fresh_checkpoint = match control.rerun {
        Some(PlanRerun::Resume) => false,
        Some(PlanRerun::Fresh) => true,
        None => ctx.fresh,
    };
    let mut checkpoint = crate::graph_checkpoint::prepare_graph_checkpoint_for_run(
        ctx.workdir,
        ctx.resume_plan,
        &plan.id,
        ctx.plan_count,
        &graph,
        fresh_checkpoint,
        ctx.force_resume,
        ctx.caller_run_id.filter(|_| ctx.plan_count == 1),
    )?;
    // Until the plan writes its terminal status, a forced exit marks its
    // checkpoint `interrupted` (bug-4641e3).
    let _running = RunningPlanCheckpoint::register(&checkpoint.paths().manifest);
    let run_id = checkpoint.run_id().to_string();
    // The run's tool calls leave durable safety provenance, and a resumed
    // run's taint lineage comes back before any task runs (gap-ff95f5).
    let provenance = checkpoint.open_safety_provenance(ctx.workdir)?;
    let _provenance = ctx.provenance_sinks.register(&run_id, provenance);
    // A new run's manifest, or one more invocation of a resumed run; the
    // run's attempt records carry the invocation's ordinal.
    if let Some(inv) = ctx.run_manifests.open(&run_id, &plan.id) {
        ctx.graph_task_dispatcher
            .attach_run_invocation(&run_id, inv);
    }
    // The learning components the run's dispatcher has (S01 §5.8); a resume
    // rewrites them, since its build may differ.
    ctx.run_manifests
        .write_census(&run_id, &ctx.graph_task_dispatcher.wiring_report());
    // 3231: the plan's spec.quality and spec.gate records, before any of its
    // tasks starts, and one event-log line per decision for SSE.
    let run_dir = RokoLayout::for_project(ctx.workdir)
        .runs_dir()
        .join(&run_id);
    let tasks_path = plan.dir.join("tasks.toml");
    for event in
        crate::spec_gate::record_plan(ctx.spec_gate, &tasks_path, ctx.workdir, &run_dir, &run_id)
    {
        ctx.graph_tui_bridge.log_event("spec.gate", &event);
    }
    // A resumed run's attempts continue from the plan branch its earlier
    // process accepted work onto, and re-attach the checkouts it kept
    // (bug-056b40).
    if let Some(worktrees) = ctx.worktrees {
        match worktrees.begin_plan_run(&plan.id, &run_id).await {
            Ok(Some(tip)) => tracing::info!(
                plan_id = %plan.id,
                %run_id,
                %tip,
                "resumed run: the plan's attempts start from its plan branch"
            ),
            Ok(None) => {}
            Err(error) => tracing::warn!(
                plan_id = %plan.id,
                %error,
                "could not read the plan branch; the plan's attempts start from the run's base"
            ),
        }
    }
    let replayed_entries = checkpoint.replayed_entries();
    ctx.graph_task_dispatcher
        .attach_plan_budget_checkpoint(&plan.id, checkpoint.take_cost_ledger())?;
    // By default a failed task blocks only its own dependants: the tasks that
    // do not depend on it keep running, and the plan reports failure once
    // every task has settled. `fail_fast` (from `[meta] failure_policy`, else
    // `[conductor] plan_failure_policy`) starts no further task after the
    // first failure instead. This is set after the checkpoint identity is
    // taken: the failure strategy does not change what a replayed task
    // produced, and hashing it into the plan fingerprint would stop
    // checkpoints written under another policy from resuming.
    graph.policy.failure_strategy = match plan
        .tasks
        .meta
        .failure_policy
        .unwrap_or(ctx.plan_failure_policy)
    {
        roko_core::config::PlanFailurePolicy::SkipFailed => roko_graph::FailureStrategy::SkipFailed,
        roko_core::config::PlanFailurePolicy::FailFast => roko_graph::FailureStrategy::FailFast,
    };
    // `--max-tasks` caps the run. Otherwise a plan that omits `max_parallel`
    // runs as wide as its DAG allows when every task that can write declares
    // its files: the engine keeps tasks whose files overlap apart. Both are
    // set after the identity is taken, like the failure strategy, so
    // checkpoints keep resuming.
    if ctx.max_tasks > 0 {
        graph.policy.max_concurrent_nodes = ctx.max_tasks;
    } else if plan.tasks.meta.max_parallel.is_none() {
        let width = crate::plan_policy::plan_max_parallel(&plan.tasks);
        if let Some(task) = crate::plan_policy::task_with_unknown_writes(&plan.tasks) {
            tracing::info!(
                plan_id = %plan.id,
                task_id = %task.id,
                "max_parallel is omitted and this task declares no files: one task at a time"
            );
        } else {
            tracing::info!(
                plan_id = %plan.id,
                width,
                "max_parallel is omitted and every writing task declares its files; running tasks \
                 as wide as the DAG allows"
            );
        }
        graph.policy.max_concurrent_nodes = usize::try_from(width.max(1)).unwrap_or(usize::MAX);
    }
    ctx.graph_task_dispatcher.attach_retry_feedback(
        &plan.id,
        checkpoint.paths().retry_feedback(),
        &run_id,
    );
    // Whatever the failure policy, a plan whose budget is spent starts no
    // further task; tasks already running finish.
    let budget_dispatcher = Arc::clone(ctx.graph_task_dispatcher);
    let budget_plan_id = plan.id.clone();
    let mut engine = GraphEngine::new(graph, registry)
        .with_recorder(checkpoint.take_recorder())
        .with_telemetry(Arc::clone(ctx.graph_telemetry))
        .with_dispatch_stop(Arc::new(move || {
            budget_dispatcher.plan_dispatch_stop(&budget_plan_id)
        }))
        // Allow stub cells when using the rich topology. Enricher cells are
        // PassthroughCell stubs; without this the engine rejects the graph
        // at validate_for_start time.
        .with_allow_test_stubs(ctx.rich_topology);
    if let Some(replayer) = checkpoint.take_replayer() {
        engine = engine.with_replayer(replayer);
    }
    // The shared pause flag reaches each task's dispatch through its
    // CellContext: a paused run starts no attempt (decision 1206).
    // Set when a stopping plan's attempts must stop (bug-2b1ddc).
    let stop_attempts = Arc::new(AtomicBool::new(false));
    let cell_ctx = plan_cell_context(
        &run_id,
        ctx.shared_pause_flag,
        &stop_attempts,
        ctx.cell_resources,
    );

    // Validate before running.
    let issues = engine.validate();
    if !issues.is_empty() {
        graph_tui_bridge.error(&format!(
            "plan '{}' has {} validation error{}",
            plan.id,
            issues.len(),
            if issues.len() == 1 { "" } else { "s" },
        ));
        tracing::error!(plan_id = %plan.id, error_count = issues.len(), "plan has validation errors");
        for issue in &issues {
            tracing::error!(plan_id = %plan.id, issue, "validation error");
        }
        let result = PlanRunResult::failed(tasks.len());
        close_plan(ctx, &plan.id, false, result.tasks).await;
        close_run_manifest(ctx, &run_id, GraphCheckpointStatus::Failed);
        checkpoint.finish(false)?;
        return Ok(result);
    }

    if replayed_entries > 0 && !ctx.quiet && !ctx.json {
        tracing::info!(
            plan_id = %plan.id,
            replayed_entries,
            manifest = %checkpoint.paths().manifest.display(),
            "resuming plan with replayed task outputs"
        );
    }

    // ── Graph TUI bridge: emit PlanStarted + per-node TaskStarted ──
    let plan_task_count = tasks.len();
    graph_tui_bridge.plan_started(&plan.id, plan_task_count);
    graph_tui_bridge.log_event(
        "graph.plan_executing",
        &format!(
            "plan '{}': {} task{}, engine=graph",
            plan.id,
            plan_task_count,
            if plan_task_count == 1 { "" } else { "s" },
        ),
    );
    // Build the node→title lookup once so the status-polling loop can emit
    // accurate TaskStarted/TaskCompleted deltas without iterating all tasks
    // on every tick. Only IDs present in this map are real tasks; the rich
    // topology adds helper nodes that must be ignored.
    let node_titles = crate::runner::graph_tui_bridge::build_node_title_map(&tasks);
    // Tracks the status snapshot from the previous polling tick so the bridge
    // can emit a diff (started / completed) rather than a full replay.
    let mut previous_statuses: HashMap<String, roko_graph::engine::NodeStatus> = HashMap::new();

    // P2-TUI-3: Use engine.start() instead of engine.execute() so we can
    // watch for stop requests and operator cancels while the graph runs.
    let flow_handle = engine.start(cell_ctx);
    let mut was_cancelled_by_tui = false;
    let mut interrupted_by: Option<PlanRunInterrupt> = None;
    // After an interrupt, how long the cancelled graph may still settle.
    let mut drain_deadline: Option<Instant> = None;
    let mut flow_abandoned = false;

    // ── Watch loop ───────────────────────────────────────────────────
    //
    // Every 100 ms while the plan runs: honour a stop request and an
    // operator cancel the plan-set driver routed to this plan. Pause and
    // resume act through the shared pause flag, which each task's dispatch
    // checks before it starts an attempt.
    loop {
        if !flow_handle.is_running() {
            break;
        }
        // Interrupt: cancel the graph and ask in-flight agents to stop so
        // their nodes settle. After INTERRUPT_DRAIN_TIMEOUT, stop the
        // attempts still running; after a further INTERRUPT_SETTLE_TIMEOUT,
        // give up on the graph so the checkpoint is still finalized.
        if let Some(deadline) = drain_deadline {
            if Instant::now() >= deadline {
                if stop_attempts.swap(true, Ordering::AcqRel) {
                    tracing::warn!(
                        plan_id = %plan.id,
                        "stopped attempts did not settle in time; finalizing checkpoint without them"
                    );
                    flow_abandoned = true;
                    break;
                }
                // A stopped attempt drops its provider call and settles with
                // the usage it streamed (bug-2b1ddc).
                tracing::warn!(
                    plan_id = %plan.id,
                    "cancelled graph did not settle in time; stopping its attempts"
                );
                drain_deadline = Some(Instant::now() + INTERRUPT_SETTLE_TIMEOUT);
            }
        } else if let Some(reason) = ctx.interrupt.requested() {
            flow_handle.cancel();
            // An agent that exits on this SIGTERM settles as cancelled.
            ctx.graph_task_dispatcher.begin_stop();
            let signalled = terminate_in_flight_agents();
            tracing::warn!(
                plan_id = %plan.id,
                signal = reason.label(),
                signalled,
                "plan run interrupted: cancelling graph"
            );
            // Name what stops the run: an API cancel is not a SIGINT
            // (bug-1d09e8).
            graph_tui_bridge.log_event(
                "graph.run_interrupted",
                &format!(
                    "{} is stopping the run: cancelling plan '{}'",
                    reason.cause(),
                    plan.id
                ),
            );
            interrupted_by = Some(reason);
            drain_deadline = Some(Instant::now() + INTERRUPT_DRAIN_TIMEOUT);
        }
        if !was_cancelled_by_tui && control.cancel.load(Ordering::Acquire) {
            tracing::info!(plan_id = %plan.id, "TUI cancel: requesting flow cancellation");
            flow_handle.cancel();
            was_cancelled_by_tui = true;
        }
        tokio::time::sleep(PLAN_WATCH_INTERVAL).await;
        // Emit incremental TaskStarted/TaskCompleted events for any node whose
        // status changed since the last tick. Filter to real tasks only (the
        // rich topology adds helper nodes absent from `node_titles`). A
        // completed task is reported with the gate verdict of its recorded
        // output (bug-7e1b6b).
        let mut current_statuses: HashMap<String, roko_graph::engine::NodeStatus> = flow_handle
            .status()
            .node_statuses
            .into_iter()
            .filter(|(id, _)| node_titles.contains_key(id))
            .collect();
        // While the run stops, a task whose node fails is reported once its
        // error says how it ended (the final diff below), so a task the stop
        // cut off reads as cancelled, not failed (bug-1d09e8).
        if interrupted_by.is_some() {
            hold_back_failures(&previous_statuses, &mut current_statuses);
        }
        graph_tui_bridge.poll_status_changes(
            &plan.id,
            &previous_statuses,
            &current_statuses,
            &node_titles,
            || checkpoint.recorded_gate_verdicts(),
        );
        previous_statuses = current_statuses;
    }
    // Agents that ignored SIGTERM must not outlive the run. They are killed
    // once their attempts were asked to stop, so an attempt settles as
    // stopped rather than as a provider failure.
    if stop_attempts.load(Ordering::Acquire) {
        let killed = kill_in_flight_agents();
        if killed > 0 {
            tracing::warn!(plan_id = %plan.id, killed, "killed agents that ignored SIGTERM");
        }
    }

    // Collect the final result from the background task.
    let flow_result = if flow_abandoned {
        None
    } else {
        flow_handle.await_completion().await
    };
    // The process exits soon after a run returns: let its attempts' cost and
    // learning rows reach the disk first. The last attempt's rows are still
    // being written when its graph settles, interrupted or not (q-1faa0c).
    let _ = tokio::time::timeout(
        ROW_WRITES_TIMEOUT,
        crate::background_writes::settled(ctx.workdir),
    )
    .await;

    let Some(output) = flow_result else {
        // Flow was cancelled before producing a result (e.g. validation
        // failure inside start(), the task panicked, or an interrupted
        // graph did not settle in time).
        if let Some(reason) = interrupted_by {
            // The tasks the stop cut off never settled, or failed while the
            // run stopped (held back above): each ends cancelled, and the stop
            // is not reported as a failure (bug-1d09e8).
            let mut running = previous_statuses
                .iter()
                .filter(|(_, status)| **status == roko_graph::engine::NodeStatus::Running)
                .map(|(task_id, _)| task_id.as_str())
                .collect::<Vec<_>>();
            running.sort_unstable();
            for task_id in running {
                graph_tui_bridge.task_cancelled(&plan.id, task_id);
            }
            graph_tui_bridge.log_event(
                "graph.plan_stopped",
                &format!(
                    "plan '{}' stopped by {} before its running tasks settled",
                    plan.id,
                    reason.cause()
                ),
            );
            tracing::warn!(plan_id = %plan.id, "plan stopped before its graph settled");
        } else {
            let message = if was_cancelled_by_tui {
                format!("plan '{}' cancelled by user", plan.id)
            } else {
                format!("plan '{}' execution failed (no output)", plan.id)
            };
            graph_tui_bridge.error(&message);
            tracing::error!(plan_id = %plan.id, "plan execution failed: no output");
        }

        // Without the graph's result no task's verdict is known, so none
        // counts as passed.
        let task_verdicts = TaskVerdictCounts::not_run(tasks.len());
        close_plan(ctx, &plan.id, false, task_verdicts).await;
        let outcome = plan_outcome(
            false,
            task_verdicts,
            interrupted_by.is_some(),
            was_cancelled_by_tui,
        );
        checkpoint.record_task_outcomes(&TaskOutcomeSummary::default())?;
        checkpoint.record_stop_cause(stop_cause(outcome, interrupted_by))?;
        close_run_manifest(ctx, &run_id, plan_checkpoint_status(outcome));
        checkpoint.finish_with_status(plan_checkpoint_status(outcome))?;
        return Ok(PlanRunResult {
            outcome,
            output_count: 0,
            tasks: task_verdicts,
            unverified_work: None,
        });
    };

    let output_count = output
        .node_results
        .iter()
        .map(|r| r.output_count)
        .sum::<usize>();
    let budget = ctx.graph_task_dispatcher.plan_budget_snapshot(&plan.id);
    let execution_succeeded = output.success && !budget.dispatch_blocked && !was_cancelled_by_tui;
    // A plan succeeds only when every task passed its verify steps
    // (gap-29a84b); one whose tasks all completed, some without running a
    // verify step, is unverified.
    let task_verdicts = TaskVerdictCounts::of(&output);
    let outcome = plan_outcome(
        execution_succeeded,
        task_verdicts,
        interrupted_by.is_some(),
        was_cancelled_by_tui,
    );
    // A plan whose tasks all passed is checked as a whole (gap-60233f): under
    // --worktree-per-task by its delivery into the run's batch branch, whose
    // regression check runs its steps on the merge (spec-f830c4); otherwise
    // in the shared working tree. It succeeds only when that check passes.
    let plan_checks = ctx.plan_checks.get(&plan.id).map_or(&[][..], Vec::as_slice);
    // Whether the tasks all passed, before the whole-plan check judged them.
    let tasks_succeeded = outcome.succeeded();
    let outcome = match ctx.batch {
        Some(batch) if outcome.succeeded() => {
            deliver_plan_to_batch(
                batch,
                plan,
                plan_checks,
                ctx.gate_env_passthrough,
                ctx.worktrees,
                ctx.delete_attempt_branches,
                &ctx.interrupt,
                &mut checkpoint,
                graph_tui_bridge,
            )
            .await?
        }
        None if outcome.succeeded() && !plan_checks.is_empty() => {
            check_plan_in_place(
                ctx.workdir,
                plan,
                plan_checks,
                ctx.gate_env_passthrough,
                &ctx.interrupt,
                &mut checkpoint,
                graph_tui_bridge,
            )
            .await?
        }
        _ => outcome,
    };
    // A stop request that came during the whole-plan check stopped the plan
    // there: it reads as stopped, with what stopped it (bug-7b62ff).
    if outcome == PlanOutcome::Interrupted && interrupted_by.is_none() {
        interrupted_by = ctx.interrupt.requested();
    }
    // How the plan's tasks that did not complete were left.
    let task_outcomes = task_outcomes(&output, &node_titles);
    // A plan that did not succeed still delivers the verified part of what
    // this run accepted onto its plan branch, marked partial, unless its tasks
    // all passed and its whole-plan check refused them. The rest stays on the
    // plan branch: its batch record says what no check ran on, and the run's
    // summary where it is (bug-db89d4, bug-74164c).
    let unverified_work = match ctx.batch {
        Some(batch) if !outcome.succeeded() => {
            let settle = UndeliveredPlan {
                plan,
                run_id: &run_id,
                outcome,
                deliver_verified: !tasks_succeeded,
                task_outcomes: &task_outcomes,
            };
            settle_undelivered_plan(ctx, batch, &settle, &mut checkpoint).await?
        }
        _ => None,
    };

    // ── Graph TUI bridge: final status diff + PlanCompleted ──
    // Emit any transitions (Running→Complete/Failed/Skipped, or
    // Pending→Running for tasks that started and finished between ticks)
    // that the polling loop did not yet publish, then close the plan. A task
    // a stop cut off is reported as cancelled, with why, not as failed
    // (bug-1d09e8).
    let cancelled = cancelled_tasks(&output, &node_titles);
    let final_statuses: HashMap<String, roko_graph::engine::NodeStatus> = output
        .node_results
        .iter()
        .filter(|r| node_titles.contains_key(&r.node_id) && !cancelled.contains_key(&r.node_id))
        .map(|r| (r.node_id.clone(), r.status))
        .collect();
    graph_tui_bridge.poll_status_changes(
        &plan.id,
        &previous_statuses,
        &final_statuses,
        &node_titles,
        || output.gate_verdicts.clone(),
    );
    for (task_id, reason) in &cancelled {
        graph_tui_bridge.log_event(
            "graph.task_cancelled",
            &format!("plan '{}': task '{task_id}' cancelled: {reason}", plan.id),
        );
        graph_tui_bridge.task_cancelled(&plan.id, task_id);
    }
    // Say why each task that did not run was held back; a resume runs them
    // and the failed tasks again.
    // The live views list each such task as blocked, with its blocker or
    // reason (gap-f59fe9).
    for (task_id, blocker) in &task_outcomes.blocked_by {
        graph_tui_bridge.log_event(
            "graph.task_blocked",
            &format!(
                "plan '{}': task '{task_id}' blocked by failed task '{blocker}'",
                plan.id
            ),
        );
        graph_tui_bridge.task_blocked(
            &plan.id,
            task_id,
            node_titles.get(task_id).map_or("", String::as_str),
            Some(blocker.as_str()),
            &format!("blocked by failed task '{blocker}'"),
        );
    }
    for (task_id, reason) in &task_outcomes.not_started {
        graph_tui_bridge.log_event(
            "graph.task_not_started",
            &format!(
                "plan '{}': task '{task_id}' did not start: {reason}",
                plan.id
            ),
        );
        let title = node_titles.get(task_id).map_or("", String::as_str);
        graph_tui_bridge.task_blocked(&plan.id, task_id, title, None, reason);
    }
    checkpoint.record_task_outcomes(&task_outcomes)?;
    checkpoint.record_stop_cause(stop_cause(outcome, interrupted_by))?;
    // Each task the run left failed gets a GitHub issue when `[github]
    // auto_pr` is on (gap-cd51b7). A run that was interrupted or cancelled
    // files none.
    if let Some(failure_issues) = ctx.failure_issues
        && interrupted_by.is_none()
        && !was_cancelled_by_tui
    {
        failure_issues
            .file(
                &plan.id,
                &run_id,
                &output,
                &task_outcomes.failed,
                &node_titles,
            )
            .await;
    }
    // Work no check ran on is never delivered: say where it is instead
    // (bug-db89d4).
    let unverified_at = unverified_work.as_ref().map_or_else(String::new, |work| {
        format!(
            ". Its work is on {} at {} and was not delivered; see it with `{}`",
            work.branch,
            short_oid(&work.commit),
            work.review_command()
        )
    });
    if outcome == PlanOutcome::Unverified {
        graph_tui_bridge.log_event(
            "graph.plan_unverified",
            &format!(
                "plan '{}' is unverified: {} task(s) ran no verify step{unverified_at}",
                plan.id, task_verdicts.unverified
            ),
        );
    } else if let Some(work) = &unverified_work {
        graph_tui_bridge.log_event(
            "graph.plan_unverified",
            &format!(
                "plan '{}': task(s) {} ran no verify step{unverified_at}",
                plan.id,
                work.tasks.join(", ")
            ),
        );
    }
    // Why each failed task failed, in the run's events, so a host can tell
    // its person (bug-103bce): a dispatch error otherwise reached only the
    // log below, which `roko serve` does not print by default.
    for message in task_failure_errors(&plan.id, &output, &node_titles) {
        graph_tui_bridge.error(&message);
    }
    // A stopped plan says what stopped it and where its tasks stand, as a
    // notice rather than a failure (bug-1d09e8).
    if let Some(reason) = interrupted_by {
        graph_tui_bridge.log_event(
            "graph.plan_stopped",
            &plan_stopped_message(&plan.id, reason, task_verdicts, plan_task_count),
        );
    }
    close_plan(ctx, &plan.id, outcome.succeeded(), task_verdicts).await;

    if !ctx.quiet && !ctx.json {
        if was_cancelled_by_tui {
            tracing::warn!(
                plan_id = %plan.id,
                node_count = output.node_results.len(),
                "plan cancelled by user"
            );
        } else if outcome.succeeded() {
            tracing::info!(
                plan_id = %plan.id,
                node_count = output.node_results.len(),
                output_count,
                "plan completed: SUCCESS"
            );
        } else if outcome == PlanOutcome::Unverified {
            tracing::warn!(
                plan_id = %plan.id,
                node_count = output.node_results.len(),
                unverified_tasks = task_verdicts.unverified,
                "plan completed: UNVERIFIED (some tasks ran no verify step)"
            );
        } else {
            tracing::warn!(
                plan_id = %plan.id,
                node_count = output.node_results.len(),
                output_count,
                "plan completed: FAILED"
            );
        }
        // Log per-node error details so failures are not silent.
        if !execution_succeeded {
            for result in &output.node_results {
                if let Some(error) = &result.error {
                    tracing::warn!(
                        node_id = %result.node_id,
                        status = ?result.status,
                        blocked_by = result.blocked_by.as_deref().unwrap_or_default(),
                        %error,
                        "node failed"
                    );
                }
            }
        }
        if budget.exhausted {
            let ceiling = budget.ceiling_usd.unwrap_or_default();
            if budget.dispatch_blocked {
                tracing::warn!(
                    plan_id = %plan.id,
                    spent_usd = budget.spent_usd,
                    ceiling_usd = ceiling,
                    "plan budget exhausted"
                );
            } else {
                tracing::warn!(
                    plan_id = %plan.id,
                    spent_usd = budget.spent_usd,
                    ceiling_usd = ceiling,
                    "plan budget exhausted; explicit override active, continuing"
                );
            }
        }
    }
    // The plan's checkouts that hold nothing to inspect or resume go now
    // (bug-58329f): those of its accepted attempts, whose work is on its plan
    // branch, and those of attempts that changed nothing. A failed attempt's
    // checkout with work stays.
    if let Some(worktrees) = ctx.worktrees {
        release_plan_checkouts(
            worktrees,
            &plan.id,
            ctx.delete_attempt_branches,
            graph_tui_bridge,
        )
        .await;
    }
    close_run_manifest(ctx, &run_id, plan_checkpoint_status(outcome));
    checkpoint.finish_with_status(plan_checkpoint_status(outcome))?;
    Ok(PlanRunResult {
        outcome,
        output_count,
        tasks: task_verdicts,
        unverified_work,
    })
}

/// Deliver `plan`, whose tasks all passed, into the run's batch branch
/// (spec-f830c4): merge its verified plan-branch tip with the batch's tip, run
/// the regression check on the merge, move the batch to the merge only once
/// the check passed (bug-700d7c), and record the delivery in the plan's
/// checkpoint. The plan succeeds only when the delivery does. A delivered
/// plan's accepted attempt checkouts are then removed from `worktrees`, and
/// with `delete_attempt_branches` their branches too (gap-415c54). `Err`
/// only when the checkpoint cannot record it.
async fn deliver_plan_to_batch(
    batch: &super::batch::BatchIntegration,
    plan: &crate::runner::plan_loader::Plan,
    checks: &[crate::task_parser::VerifyStep],
    env_passthrough: &[String],
    worktrees: Option<&crate::orchestrator::worktree::WorktreeManager>,
    delete_attempt_branches: bool,
    stop: &PlanRunInterruptHandle,
    checkpoint: &mut crate::graph_checkpoint::PreparedGraphCheckpoint,
    graph_tui_bridge: &crate::runner::graph_tui_bridge::GraphTuiBridge,
) -> anyhow::Result<PlanOutcome> {
    let Some(verified) = super::batch::plan_branch_tip(batch.repo(), &plan.id).await else {
        tracing::info!(
            plan_id = %plan.id,
            "no attempt of the plan was accepted, so it has nothing to deliver"
        );
        return Ok(PlanOutcome::Succeeded);
    };
    // The regression check is the plan's whole-plan check (gap-60233f), with
    // the environment verify steps get. Its receipt says where the steps came
    // from (backlog 3111).
    let backend = super::delivery::GitDeliveryBackend::new(batch.repo().to_path_buf())
        .with_regression_steps(checks.iter().map(|step| step.command.clone()).collect())
        .with_regression_source(plan_check_source(plan))
        .with_env_passthrough(env_passthrough.to_vec())
        .with_check_events(plan_check_events(
            graph_tui_bridge,
            plan,
            super::plan_check_events::CheckStage::Regression,
        ))
        .with_stop(stop.clone());
    let service = super::delivery::CliCompletionDeliveryService::with_store(
        batch.store().clone(),
        Arc::new(backend),
    )
    .with_events(delivery_events(graph_tui_bridge));
    let request = batch.request(&plan.id, verified);
    let receipt = match batch
        .deliver(&service, request, checkpoint.recorded_delivery())
        .await
    {
        Ok(receipt) => receipt,
        Err(error) => {
            tracing::error!(plan_id = %plan.id, %error, "plan could not be delivered");
            graph_tui_bridge.error(&format!(
                "plan '{}' could not be delivered into {}: {error}",
                plan.id,
                batch.branch()
            ));
            return Ok(PlanOutcome::Failed);
        }
    };
    checkpoint.record_batch_delivery(batch.record(&receipt), &receipt)?;
    if receipt.state.is_success() {
        tracing::info!(
            plan_id = %plan.id,
            batch = batch.branch(),
            merge_commit = receipt.merge_commit.as_deref().unwrap_or_default(),
            "plan delivered into the run's batch branch"
        );
        release_delivered_attempts(worktrees, &plan.id, delete_attempt_branches, batch, receipt)
            .await;
        return Ok(PlanOutcome::Succeeded);
    }
    let reason = receipt.error.as_deref().unwrap_or("no reason recorded");
    // A stop request cut the delivery's regression check short: the plan
    // stopped, it did not fail (bug-7b62ff).
    if stop.requested().is_some() {
        tracing::warn!(
            plan_id = %plan.id,
            batch = batch.branch(),
            reason,
            "the run was asked to stop during the plan's delivery; it was not delivered"
        );
        return Ok(PlanOutcome::Interrupted);
    }
    tracing::error!(
        plan_id = %plan.id,
        batch = batch.branch(),
        state = ?receipt.state,
        reason,
        "plan was not delivered into the run's batch branch"
    );
    graph_tui_bridge.error(&format!(
        "plan '{}' was not delivered into {} ({:?}): {reason}",
        plan.id,
        batch.branch(),
        receipt.state
    ));
    Ok(PlanOutcome::Failed)
}

/// A plan that ended without succeeding, as [`settle_undelivered_plan`]
/// takes it (bug-74164c).
struct UndeliveredPlan<'a> {
    plan: &'a crate::runner::plan_loader::Plan,
    /// The plan's checkpoint run, which the commits on its branch name.
    run_id: &'a str,
    outcome: PlanOutcome,
    /// Whether the verified part of its work may be delivered: not when its
    /// tasks all passed and its whole-plan check refused them.
    deliver_verified: bool,
    task_outcomes: &'a TaskOutcomeSummary,
}

/// Settle `undelivered`, a plan that did not succeed under `batch`
/// (bug-db89d4, bug-74164c). The verified part of what its run accepted onto its
/// plan branch, when it may be delivered, goes into the batch as a partial
/// delivery. Its checkpoint records which tasks' work was delivered, which
/// was not and why, and the work no check ran on, which is returned. `Err`
/// only when the checkpoint cannot record it.
async fn settle_undelivered_plan(
    ctx: &PlanRunContext<'_>,
    batch: &super::batch::BatchIntegration,
    undelivered: &UndeliveredPlan<'_>,
    checkpoint: &mut crate::graph_checkpoint::PreparedGraphCheckpoint,
) -> anyhow::Result<Option<UnverifiedWork>> {
    let plan = undelivered.plan;
    let Some(work) = super::batch::PlanWork::read(batch.repo(), &plan.id, undelivered.run_id).await
    else {
        return Ok(None);
    };
    let mut delivered_tasks = Vec::new();
    if undelivered.deliver_verified
        && let Some(tip) = work.verified_tip()
    {
        let tasks = distinct_tasks(&work.commits[..work.verified_len()]);
        if deliver_partial_to_batch(ctx, batch, plan, tip.to_string(), &tasks, checkpoint).await? {
            delivered_tasks = tasks;
        }
    }
    let mut record = serde_json::json!({
        "partial": !delivered_tasks.is_empty(),
        "delivered_tasks": delivered_tasks,
        "missing_tasks": missing_tasks(undelivered, &work, &delivered_tasks),
    });
    if let Some(unverified) = work.unverified_record() {
        record["unverified"] = unverified;
    }
    checkpoint.record_batch_work(batch.plan_record(&plan.id), record)?;
    Ok(UnverifiedWork::of(&plan.id, &work))
}

/// The tasks `commits` hold, each once, in their order.
fn distinct_tasks(commits: &[super::batch::AcceptedCommit]) -> Vec<String> {
    let mut tasks: Vec<String> = Vec::new();
    for accepted in commits {
        if !tasks.contains(&accepted.task_id) {
            tasks.push(accepted.task_id.clone());
        }
    }
    tasks
}

/// Each task of `undelivered`'s plan whose work is not on the batch branch,
/// with why (bug-74164c): its work ran no check, or builds on work that did not;
/// the whole-plan check refused the plan, or the delivery failed; a stop cut
/// it off (cancelled or interrupted), or it failed; it was blocked, or never
/// started.
fn missing_tasks(
    undelivered: &UndeliveredPlan<'_>,
    work: &super::batch::PlanWork,
    delivered: &[String],
) -> Vec<serde_json::Value> {
    let outcomes = undelivered.task_outcomes;
    undelivered
        .plan
        .tasks
        .tasks
        .iter()
        .filter(|task| !delivered.contains(&task.id))
        .map(|task| {
            let accepted = work
                .commits
                .iter()
                .rev()
                .find(|accepted| accepted.task_id == task.id);
            let why = match accepted {
                Some(accepted) if !accepted.verified() => "ran no check".to_string(),
                Some(_)
                    if work
                        .unverified()
                        .iter()
                        .any(|later| later.task_id == task.id) =>
                {
                    "builds on work no check ran on".to_string()
                }
                Some(_) if !undelivered.deliver_verified => match undelivered.outcome {
                    // A stop cut the check short (bug-7b62ff).
                    PlanOutcome::Interrupted => "the plan's whole-plan check was interrupted",
                    _ => "the plan's whole-plan check failed",
                }
                .to_string(),
                Some(_) => "its delivery failed".to_string(),
                // A stop cut it off (bug-1d09e8).
                None if outcomes.cancelled.contains(&task.id) => match undelivered.outcome {
                    PlanOutcome::Interrupted => "interrupted".to_string(),
                    _ => "cancelled".to_string(),
                },
                None if outcomes.failed.contains(&task.id) => match undelivered.outcome {
                    PlanOutcome::Cancelled => "cancelled".to_string(),
                    PlanOutcome::Interrupted => "interrupted".to_string(),
                    _ => "failed".to_string(),
                },
                None => match outcomes.blocked_by.get(&task.id) {
                    Some(blocker) => format!("blocked by {blocker}"),
                    None if outcomes.not_started.contains_key(&task.id) => {
                        "not started".to_string()
                    }
                    None => "not accepted".to_string(),
                },
            };
            serde_json::json!({ "task_id": task.id, "why": why })
        })
        .collect()
}

/// Deliver `verified`, the tip of the verified part of `plan`'s work, into
/// the run's batch branch as a partial delivery (bug-74164c): a merge only, since
/// the plan did not finish and its whole-plan check does not apply. Its
/// receipt says it is partial and which `tasks` it holds, the plan's
/// checkpoint records it, and the delivered attempts' checkouts are removed
/// as a full delivery's are. Returns whether it was delivered; `Err` only
/// when the checkpoint cannot record it.
async fn deliver_partial_to_batch(
    ctx: &PlanRunContext<'_>,
    batch: &super::batch::BatchIntegration,
    plan: &crate::runner::plan_loader::Plan,
    verified: String,
    tasks: &[String],
    checkpoint: &mut crate::graph_checkpoint::PreparedGraphCheckpoint,
) -> anyhow::Result<bool> {
    let backend = super::delivery::GitDeliveryBackend::new(batch.repo().to_path_buf())
        .with_regression_steps(Vec::new());
    let service = super::delivery::CliCompletionDeliveryService::with_store(
        batch.store().clone(),
        Arc::new(backend),
    )
    .with_events(delivery_events(ctx.graph_tui_bridge));
    let request = batch.partial_request(&plan.id, verified);
    let mut receipt = match batch
        .deliver(&service, request, checkpoint.recorded_delivery())
        .await
    {
        Ok(receipt) => receipt,
        Err(error) => {
            tracing::warn!(plan_id = %plan.id, %error, "the plan's verified tasks could not be delivered");
            ctx.graph_tui_bridge.error(&format!(
                "plan '{}': its verified tasks could not be delivered into {}: {error}",
                plan.id,
                batch.branch()
            ));
            return Ok(false);
        }
    };
    receipt.extensions.insert(
        super::batch::PARTIAL_EXTENSION.to_string(),
        serde_json::json!({ "tasks": tasks }),
    );
    batch.store().update(&receipt);
    checkpoint.record_batch_delivery(batch.record(&receipt), &receipt)?;
    let Some(merge) = receipt
        .merge_commit
        .clone()
        .filter(|_| receipt.state.is_success())
    else {
        let reason = receipt.error.as_deref().unwrap_or("no reason recorded");
        tracing::warn!(plan_id = %plan.id, state = ?receipt.state, reason, "the plan's verified tasks were not delivered");
        ctx.graph_tui_bridge.error(&format!(
            "plan '{}': its verified tasks were not delivered into {} ({:?}): {reason}",
            plan.id,
            batch.branch(),
            receipt.state
        ));
        return Ok(false);
    };
    tracing::info!(plan_id = %plan.id, batch = batch.branch(), merge_commit = %merge, "delivered the plan's verified tasks");
    ctx.graph_tui_bridge.log_event(
        "graph.plan_partially_delivered",
        &format!(
            "plan '{}' did not finish: the work of its verified task(s) {} was delivered into {} at {}",
            plan.id,
            tasks.join(", "),
            batch.branch(),
            short_oid(&merge)
        ),
    );
    release_delivered_attempts(
        ctx.worktrees,
        &plan.id,
        ctx.delete_attempt_branches,
        batch,
        receipt,
    )
    .await;
    Ok(true)
}

/// Remove plan `plan_id`'s attempt checkouts that hold nothing to inspect or
/// resume once the plan ended (bug-58329f): the accepted attempts' that a delivery
/// did not already remove, whose work is on the plan branch, and the ones
/// whose attempts changed nothing, with their branches. Says what went in the
/// run's events.
async fn release_plan_checkouts(
    worktrees: &crate::orchestrator::worktree::WorktreeManager,
    plan_id: &str,
    delete_attempt_branches: bool,
    graph_tui_bridge: &crate::runner::graph_tui_bridge::GraphTuiBridge,
) {
    let mut removed = match worktrees
        .release_accepted(plan_id, delete_attempt_branches)
        .await
    {
        Ok(released) => released.removed_checkouts,
        Err(error) => {
            tracing::warn!(plan_id, %error, "kept the plan's accepted attempt checkouts");
            Vec::new()
        }
    };
    match worktrees.remove_idle_attempts(plan_id).await {
        Ok(idle) => removed.extend(idle),
        Err(error) => tracing::warn!(plan_id, %error, "kept the plan's idle attempt checkouts"),
    }
    if !removed.is_empty() {
        tracing::info!(
            plan_id,
            removed = removed.len(),
            "removed the plan's finished attempt checkouts"
        );
        graph_tui_bridge.log_event(
            "graph.attempt_checkouts_removed",
            &format!(
                "plan '{plan_id}': removed {} attempt checkout(s) that held nothing to inspect or resume",
                removed.len()
            ),
        );
    }
}

/// Remove the checkouts of the attempts plan `plan_id` accepted, once
/// `receipt` delivered their work into `batch` (gap-415c54): it is on the
/// plan and batch branches now, so the checkouts kept for review have done
/// their job. With `delete_attempt_branches` their branches go too. What
/// went and what stayed is on the receipt, for the run summary.
async fn release_delivered_attempts(
    worktrees: Option<&crate::orchestrator::worktree::WorktreeManager>,
    plan_id: &str,
    delete_attempt_branches: bool,
    batch: &super::batch::BatchIntegration,
    mut receipt: roko_graph::delivery::CompletionDeliveryReceiptV1,
) {
    let Some(worktrees) = worktrees else {
        return;
    };
    if receipt.release_policy != roko_graph::delivery::ReleasePolicy::Delete {
        return;
    }
    match worktrees
        .release_accepted(plan_id, delete_attempt_branches)
        .await
    {
        Ok(released) => {
            tracing::info!(
                plan_id,
                removed = released.removed_checkouts.len(),
                kept_branches = released.kept_branches.len(),
                deleted_branches = released.deleted_branches.len(),
                "removed the delivered plan's attempt checkouts"
            );
            if let Ok(cleanup) = serde_json::to_value(&released) {
                receipt
                    .extensions
                    .insert(super::batch::ATTEMPT_CLEANUP_EXTENSION.to_string(), cleanup);
                batch.store().update(&receipt);
            }
        }
        Err(error) => tracing::warn!(
            plan_id,
            %error,
            "kept the delivered plan's attempt checkouts"
        ),
    }
}

/// Run `plan`'s whole-plan check (gap-60233f) in the shared working tree at
/// `workdir`, which its tasks edited, and record it in the plan's
/// checkpoint. Its steps inherit what the gate policy and `env_passthrough`
/// admit. The plan succeeds only when the check passes. `Err` only when the
/// checkpoint cannot record it.
async fn check_plan_in_place(
    workdir: &Path,
    plan: &crate::runner::plan_loader::Plan,
    checks: &[crate::task_parser::VerifyStep],
    env_passthrough: &[String],
    stop: &PlanRunInterruptHandle,
    checkpoint: &mut crate::graph_checkpoint::PreparedGraphCheckpoint,
    graph_tui_bridge: &crate::runner::graph_tui_bridge::GraphTuiBridge,
) -> anyhow::Result<PlanOutcome> {
    let events = plan_check_events(
        graph_tui_bridge,
        plan,
        super::plan_check_events::CheckStage::InPlace,
    );
    let result =
        super::plan_verify::run_plan_verify(workdir, checks, env_passthrough, &events, stop).await;
    let commands: Vec<&str> = checks.iter().map(|step| step.command.as_str()).collect();
    checkpoint.record_plan_verify(serde_json::json!({
        "passed": result.is_ok(),
        "steps": commands,
        "source": plan_check_source(plan),
        "failure": result.as_ref().err(),
    }))?;
    match result {
        Ok(()) => {
            tracing::info!(plan_id = %plan.id, steps = checks.len(), "plan check passed");
            Ok(PlanOutcome::Succeeded)
        }
        // A stop request ended the check: the plan stopped (bug-7b62ff).
        Err(failure) if failure.stopped_by.is_some() => {
            tracing::warn!(plan_id = %plan.id, step = %failure.command, "plan check stopped");
            Ok(PlanOutcome::Interrupted)
        }
        Err(failure) => {
            tracing::error!(
                plan_id = %plan.id,
                step = %failure.command,
                output = %failure.output,
                "plan check failed: its tasks passed, but not together"
            );
            graph_tui_bridge.error(&format!("plan '{}': [meta] verify {failure}", plan.id));
            Ok(PlanOutcome::Failed)
        }
    }
}

/// How many plan runs the cost trend a run publishes as it ends covers
/// (gap-de0e96).
const RUN_COST_TREND_RUNS: usize = 20;

/// The trend a Graph run publishes as it ends (gap-de0e96): one
/// [`roko_core::EfficiencyBucket`] per plan run of `runs` (newest first, as
/// `run-metrics.jsonl` gives them), oldest first, each with its cost, its
/// tasks, the tasks that passed every verify step and its cost per verified
/// task.
fn run_cost_trend(
    runs: &[roko_learn::run_metrics::RunMetricsRecord],
) -> Vec<roko_core::EfficiencyBucket> {
    runs.iter()
        .rev()
        .map(|run| {
            let ended = chrono::DateTime::parse_from_rfc3339(&run.timestamp).map_or_else(
                |_| chrono::Utc::now(),
                |ended| ended.with_timezone(&chrono::Utc),
            );
            let took = i64::try_from(run.duration_ms)
                .ok()
                .and_then(chrono::TimeDelta::try_milliseconds)
                .unwrap_or_default();
            let verified = run.tasks_completed as u64;
            roko_core::EfficiencyBucket {
                start: ended - took,
                turns: run.total_agent_calls as u64,
                tokens_in: run.total_tokens_in,
                tokens_out: run.total_tokens_out,
                cost_usd_cents: (run.total_cost_usd * 100.0).round() as u64,
                latency_ms_avg: run.duration_ms as f64 / run.total_agent_calls.max(1) as f64,
                run_id: Some(run.run_id.clone()),
                cost_usd: Some(run.total_cost_usd),
                tasks: Some(run.total_tasks as u64),
                verified_tasks: Some(verified),
                cost_per_verified_usd: (verified > 0).then(|| run.total_cost_usd / verified as f64),
            }
        })
        .collect()
}

/// The events of a plan's delivery, published through the run's bridge
/// (gap-4f6cbe).
fn delivery_events(
    graph_tui_bridge: &crate::runner::graph_tui_bridge::GraphTuiBridge,
) -> super::delivery::DeliveryEvents {
    super::delivery::DeliveryEvents::new(graph_tui_bridge.tui().clone())
}

/// The hub events of `plan`'s whole-plan check at `stage` (gap-6b6ec1).
fn plan_check_events(
    graph_tui_bridge: &crate::runner::graph_tui_bridge::GraphTuiBridge,
    plan: &crate::runner::plan_loader::Plan,
    stage: super::plan_check_events::CheckStage,
) -> super::plan_check_events::PlanCheckEvents {
    super::plan_check_events::PlanCheckEvents::new(
        graph_tui_bridge.tui().clone(),
        &plan.id,
        stage,
        plan_check_source(plan),
    )
}

/// Where `plan`'s whole-plan check came from: `authored` (its `[meta]
/// verify`) or `cargo-default` (the default check of a Cargo workspace,
/// [`super::plan_verify::default_plan_verify`]).
fn plan_check_source(plan: &crate::runner::plan_loader::Plan) -> &'static str {
    if plan.tasks.meta.verify.is_empty() {
        "cargo-default"
    } else {
        "authored"
    }
}

/// The most of a failed task's error its `error` event carries (bug-103bce).
const MAX_TASK_ERROR_CHARS: usize = 1_500;

/// Why each of the plan's tasks that failed by its own error did, as the
/// `error` events a run shows a host (bug-103bce), such as a dispatch that
/// found no provider able to run the task. A task blocked by another, a task
/// a stop cut off ([`cancelled_tasks`]), a failure without an error, and the
/// helper nodes of the rich topology, which are absent from `node_titles`,
/// are left out.
fn task_failure_errors(
    plan_id: &str,
    output: &roko_graph::GraphOutput,
    node_titles: &HashMap<String, String>,
) -> Vec<String> {
    use roko_graph::engine::NodeStatus;

    output
        .node_results
        .iter()
        .filter(|result| {
            result.status == NodeStatus::Failed
                && result.blocked_by.is_none()
                && cancel_reason(result).is_none()
                && node_titles.contains_key(&result.node_id)
        })
        .filter_map(|result| {
            let error = result.error.as_deref()?.trim();
            if error.is_empty() {
                return None;
            }
            let clipped: String = error.chars().take(MAX_TASK_ERROR_CHARS).collect();
            let more = if clipped.len() < error.len() {
                "…"
            } else {
                ""
            };
            Some(format!(
                "plan '{plan_id}': task '{}' failed: {clipped}{more}",
                result.node_id
            ))
        })
        .collect()
}

/// Keep the nodes that failed since `previous` at their `previous` status in
/// `current`, or out of it when they had none, so the poll does not report
/// them yet: while a run stops, a failed node's error, which the poll does not
/// see, says whether the stop cut its task off ([`cancel_reason`]).
fn hold_back_failures(
    previous: &HashMap<String, roko_graph::engine::NodeStatus>,
    current: &mut HashMap<String, roko_graph::engine::NodeStatus>,
) {
    use roko_graph::engine::NodeStatus;

    current.retain(|node_id, status| {
        if *status != NodeStatus::Failed {
            return true;
        }
        match previous.get(node_id) {
            Some(&before) => {
                *status = before;
                true
            }
            None => false,
        }
    });
}

/// Why a stop cut off the task of node `result`, when it did: the node failed
/// with a cancellation (bug-1d09e8), because its plan run was stopping or the
/// operator stopped the task. Such a task is cancelled, not failed.
fn cancel_reason(result: &roko_graph::NodeResult) -> Option<&str> {
    if result.status != roko_graph::engine::NodeStatus::Failed {
        return None;
    }
    result
        .error
        .as_deref()
        .and_then(roko_core::RokoError::cancelled_reason)
}

/// The plan's tasks in `output` that a stop cut off ([`cancel_reason`]), each
/// with the reason. Helper nodes of the rich topology, which are absent from
/// `node_titles`, are left out.
fn cancelled_tasks<'a>(
    output: &'a roko_graph::GraphOutput,
    node_titles: &HashMap<String, String>,
) -> BTreeMap<String, &'a str> {
    output
        .node_results
        .iter()
        .filter(|result| node_titles.contains_key(&result.node_id))
        .filter_map(|result| Some((result.node_id.clone(), cancel_reason(result)?)))
        .collect()
}

/// How `output` left the plan's tasks that did not complete. Helper nodes of
/// the rich topology, which are absent from `node_titles`, are left out.
fn task_outcomes(
    output: &roko_graph::GraphOutput,
    node_titles: &HashMap<String, String>,
) -> TaskOutcomeSummary {
    use roko_graph::engine::NodeStatus;

    let mut summary = TaskOutcomeSummary::default();
    for result in output
        .node_results
        .iter()
        .filter(|result| node_titles.contains_key(&result.node_id))
    {
        match (result.status, &result.blocked_by) {
            (NodeStatus::Failed, _) if cancel_reason(result).is_some() => {
                summary.cancelled.insert(result.node_id.clone());
            }
            (NodeStatus::Failed, _) => {
                summary.failed.insert(result.node_id.clone());
            }
            (NodeStatus::Skipped, Some(blocker)) => {
                summary
                    .blocked_by
                    .insert(result.node_id.clone(), blocker.clone());
            }
            (NodeStatus::Skipped, None) => {
                summary.not_started.insert(
                    result.node_id.clone(),
                    result.error.clone().unwrap_or_default(),
                );
            }
            _ => {}
        }
    }
    summary
}

/// What a plan that `reason` stopped tells its person (bug-1d09e8): what
/// stopped it, how its `task_count` tasks stand, and that a resume keeps the
/// ones that passed.
fn plan_stopped_message(
    plan_id: &str,
    reason: PlanRunInterrupt,
    tasks: TaskVerdictCounts,
    task_count: usize,
) -> String {
    let passed = tasks.passed + tasks.already_satisfied;
    let settled = passed + tasks.unverified + tasks.failed + tasks.cancelled;
    let mut stand = vec![format!("{passed} task(s) passed")];
    for (count, word) in [
        (tasks.unverified, "unverified"),
        (tasks.failed, "failed"),
        (tasks.cancelled, "cancelled"),
        (task_count.saturating_sub(settled), "did not run"),
    ] {
        if count > 0 {
            stand.push(format!("{count} {word}"));
        }
    }
    let resume = if passed > 0 {
        "; a resume keeps the passed tasks and runs the rest"
    } else {
        ""
    };
    format!(
        "plan '{plan_id}' stopped by {}: {}{resume}",
        reason.cause(),
        stand.join(", ")
    )
}

/// Graph cell type that runs a plan task: one node per task in either
/// topology.
const TASK_EXECUTOR_CELL_TYPE: &str = "task-executor";

/// How a plan's tasks settled in one run, each counted by its node status
/// and gate verdict (epic spec-e9d7ec).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct TaskVerdictCounts {
    /// Completed with a `passed` gate verdict: every verify step passed. A
    /// `passed_with_preexisting_failures` verdict counts here too: its steps
    /// failed only on tests that failed before the run (gap-161be1), which
    /// its attempt record keeps.
    passed: usize,
    /// Completed with an `already_satisfied` gate verdict: the attempt
    /// changed nothing, and every verify step passed on the tree as it was
    /// (gap-9eb1e1). Verified, but not counted as passed.
    already_satisfied: usize,
    /// Completed without a verify step running: the task declares none, its
    /// role is disabled, or its output carries no gate verdict.
    unverified: usize,
    /// Never ran: blocked by a failed task, or not started.
    skipped: usize,
    /// Failed, or completed over a failed verify step (`forced_accept`).
    failed: usize,
    /// Cut off by a stop ([`cancel_reason`]): not a failure (bug-1d09e8).
    cancelled: usize,
}

impl TaskVerdictCounts {
    /// Count the task nodes of `output`.
    fn of(output: &roko_graph::GraphOutput) -> Self {
        use roko_graph::cells::task_executor::TaskGateVerdict;
        use roko_graph::engine::NodeStatus;

        let mut counts = Self::default();
        for result in output
            .node_results
            .iter()
            .filter(|result| result.cell_type == TASK_EXECUTOR_CELL_TYPE)
        {
            let verdict = output.gate_verdicts.get(&result.node_id).copied();
            match (result.status, verdict) {
                (
                    NodeStatus::Complete,
                    Some(TaskGateVerdict::Passed | TaskGateVerdict::PassedWithPreexistingFailures),
                ) => counts.passed += 1,
                (NodeStatus::Complete, Some(TaskGateVerdict::AlreadySatisfied)) => {
                    counts.already_satisfied += 1;
                }
                (NodeStatus::Failed, _) if cancel_reason(result).is_some() => {
                    counts.cancelled += 1;
                }
                (NodeStatus::Complete, Some(TaskGateVerdict::ForcedAccept))
                | (NodeStatus::Failed, _) => counts.failed += 1,
                (NodeStatus::Complete, _) => counts.unverified += 1,
                // Skipped, not selected by a route, or never settled.
                _ => counts.skipped += 1,
            }
        }
        counts
    }

    /// A plan of `task_count` tasks that ran none of them.
    const fn not_run(task_count: usize) -> Self {
        Self {
            passed: 0,
            already_satisfied: 0,
            unverified: 0,
            skipped: task_count,
            failed: 0,
            cancelled: 0,
        }
    }

    /// Outcome of a plan whose graph ran to completion (gap-29a84b): it
    /// succeeded only when every task passed its verify steps (an
    /// already-satisfied task did), and is unverified when the rest passed but
    /// some ran no verify step. A task the operator stopped cancels the plan
    /// unless another failed. Anything else failed.
    const fn outcome(self) -> PlanOutcome {
        if self.failed > 0 || self.skipped > 0 {
            PlanOutcome::Failed
        } else if self.cancelled > 0 {
            PlanOutcome::Cancelled
        } else if self.unverified > 0 {
            PlanOutcome::Unverified
        } else {
            PlanOutcome::Succeeded
        }
    }
}

/// The run-metrics row of one plan: whether it succeeded, and its tasks
/// counted by verdict. A task a stop cut off counts in none of them: it says
/// nothing about its model or prompt (bug-1d09e8). The caller adds the plan's
/// checkpoint run.
fn plan_metrics(
    plan_id: &str,
    succeeded: bool,
    tasks: TaskVerdictCounts,
) -> roko_learn::run_metrics::PlanMetrics {
    roko_learn::run_metrics::PlanMetrics {
        plan_id: plan_id.to_string(),
        completed: succeeded,
        tasks_completed: tasks.passed,
        tasks_already_satisfied: tasks.already_satisfied,
        tasks_failed: tasks.failed,
        tasks_unverified: tasks.unverified,
        tasks_skipped: tasks.skipped,
        run_id: None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;

    fn test_plan(id: &str, title: &str, task_count: usize) -> crate::runner::plan_loader::Plan {
        let task_blocks = (0..task_count)
            .map(|index| format!("[[task]]\nid = \"T{index}\"\ntitle = \"Task {index}\"\n"))
            .collect::<String>();
        let tasks = crate::task_parser::TasksFile::parse_str(&format!(
            "[meta]\nplan = \"{title}\"\n\n{task_blocks}"
        ))
        .expect("parse test plan");
        crate::runner::plan_loader::Plan {
            id: id.to_string(),
            dir: PathBuf::from(id),
            tasks,
        }
    }

    /// bug-8208a6: a command `roko plan cancel` writes to control.json reaches
    /// the run's command channel, as a TUI command would, and is consumed.
    /// gap-4ec59f: before the first dispatch, a worktree run clears what a
    /// crashed run left behind, here a stale `index.lock`.
    #[tokio::test]
    async fn worktree_startup_repair_clears_a_stale_index_lock() {
        let dir = tempfile::tempdir().expect("tempdir");
        let repo = std::fs::canonicalize(dir.path()).expect("canonical repo");
        for args in [
            &["init", "--quiet"][..],
            &[
                "-c",
                "user.name=Operator",
                "-c",
                "user.email=operator@example.test",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--quiet",
                "--allow-empty",
                "-m",
                "base",
            ][..],
        ] {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(&repo)
                .env_remove("GIT_DIR")
                .env_remove("GIT_WORK_TREE")
                .status()
                .expect("run git");
            assert!(status.success(), "git {args:?}");
        }
        let lock = repo.join(".git/index.lock");
        std::fs::File::create(&lock)
            .expect("create a lock")
            .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(120))
            .expect("age the lock");
        let manager = crate::orchestrator::worktree::WorktreeManager::new(
            crate::orchestrator::worktree::WorktreeConfig {
                repo_root: repo.clone(),
                base_branch: "HEAD".to_string(),
                worktrees_root: repo.join(".roko").join("worktrees"),
                max_live: None,
                idle_ttl: std::time::Duration::from_secs(3600),
            },
        );

        repair_worktree_state(&manager).await;

        assert!(!lock.exists(), "the stale index.lock was cleared");
    }

    #[test]
    fn a_control_file_command_reaches_the_run() {
        let state_dir = tempfile::tempdir().expect("tempdir");
        crate::runner::types::ControlCommand {
            command: crate::runner::types::ControlAction::Cancel,
            plan_id: Some("p1".to_string()),
            task_id: None,
            budget_usd: None,
        }
        .write(state_dir.path())
        .expect("write control.json");
        let (sender, mut commands, _acks, _ack_rx) =
            ExecutionCommandSender::channel("graph-engine");

        forward_control_file(state_dir.path(), &sender);

        let command = commands.try_recv().expect("the command is routed");
        assert_eq!(command.kind, ExecutionCommandKind::Cancel);
        assert_eq!(command.plan_id.as_deref(), Some("p1"));
        assert!(!state_dir.path().join("control.json").exists());
        forward_control_file(state_dir.path(), &sender);
        assert!(commands.try_recv().is_err(), "nothing more to route");
    }

    /// 1211: affect is held (dec-e70592), so a default plan run builds no
    /// daimon state: it appraises no attempt and shifts no routing tier.
    /// `[daimon] enabled = true` turns it on.
    #[test]
    fn default_config_builds_no_daimon_state() {
        let workdir = tempfile::tempdir().expect("tempdir");
        let mut config = roko_core::config::schema::RokoConfig::default();

        assert!(graph_daimon_state(workdir.path(), &config).is_none());
        assert!(!daimon_affect_path(workdir.path()).exists());

        config.daimon.enabled = true;
        assert!(graph_daimon_state(workdir.path(), &config).is_some());
    }

    /// The sinks a plan's completion reaches on the feedback facade a plan
    /// run under `workdir` builds for `config`, and whether the workspace's
    /// affect file is on disk once they have heard it.
    async fn plan_completion_listeners(
        workdir: &Path,
        config: &roko_core::config::schema::RokoConfig,
    ) -> (Vec<&'static str>, bool) {
        let error_patterns = Arc::new(std::sync::RwLock::new(
            roko_learn::error_pattern_store::ErrorPatternStore::load(&workdir.join("none.json")),
        ));
        let feedback =
            build_graph_feedback_context(workdir, config, None, None, &error_patterns, None, None);
        let facade = feedback.feedback_facade.expect("a plan run's facade");
        let completed = crate::runtime_feedback::FeedbackEvent::PlanCompleted {
            plan_id: "p".to_string(),
            succeeded: true,
            tasks_completed: 1,
            tasks_failed: 0,
            total_cost_usd: 0.0,
        };
        facade.on_event(&completed).await.expect("deliver it");
        let listeners = facade
            .stats()
            .per_sink
            .into_iter()
            .filter(|sink| sink.delivered > 0)
            .map(|sink| sink.name)
            .collect();
        (listeners, daimon_affect_path(workdir).exists())
    }

    /// q-6b7cca option 2 (gap-a960a1): a plan's completion reaches the
    /// daimon sink alone, which saves the affect state, and only when
    /// `[daimon] enabled` gives the run one. Both dream switches are on: the
    /// dream sink they gated started a `claude` dream after every plan, and
    /// it is gone, with the theta and delta sinks.
    #[tokio::test]
    async fn plan_completion_reaches_only_the_daimon_sink() {
        let mut config = roko_core::config::schema::RokoConfig::default();
        config.learning.dream_on_completion = true;
        config.learning.dreams.trigger_on_plan_complete = true;
        let held = tempfile::tempdir().expect("tempdir");
        assert_eq!(
            plan_completion_listeners(held.path(), &config).await,
            (vec![], false),
            "affect is held by default"
        );

        config.daimon.enabled = true;
        let on = tempfile::tempdir().expect("tempdir");
        assert_eq!(
            plan_completion_listeners(on.path(), &config).await,
            (vec!["daimon_persistence"], true)
        );
    }

    /// gap-a960a1: a frozen run (decision 2218) writes no learned state, and
    /// the plan-completion save follows it: with affect on, no sink hears a
    /// plan's completion, and no affect file is written.
    #[tokio::test]
    async fn a_frozen_run_saves_no_affect_when_a_plan_completes() {
        let mut config = roko_core::config::schema::RokoConfig::default();
        config.daimon.enabled = true;
        config.learning.frozen = true;
        let dir = tempfile::tempdir().expect("tempdir");

        assert_eq!(
            plan_completion_listeners(dir.path(), &config).await,
            (vec![], false)
        );
    }

    #[test]
    fn plan_set_entries_follow_execution_order_with_task_totals() {
        let plans = vec![
            test_plan("b-second", "Second", 1),
            test_plan("a-first", "", 3),
        ];
        let order = PlanSetOrder {
            order: vec!["a-first".to_string(), "b-second".to_string()],
            ..PlanSetOrder::default()
        };

        let entries = plan_set_entries(Path::new("plans"), &plans, &order, &PlanConflicts::new());

        let summary = entries
            .iter()
            .map(|entry| {
                (
                    entry.plan_id.as_str(),
                    entry.title.as_str(),
                    entry.tasks_total,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            summary,
            vec![("a-first", "a-first", 3), ("b-second", "Second", 1)]
        );
    }

    #[test]
    fn plan_set_entries_name_each_plans_group() {
        let root = Path::new("/workspace/plans");
        let mut nested = test_plan("01-backend", "", 1);
        nested.dir = root.join("portal-programme").join("01-backend");
        let mut top_level = test_plan("02-docs", "", 1);
        top_level.dir = root.join("02-docs");
        let mut outside = test_plan("03-scratch", "", 1);
        outside.dir = PathBuf::from("/elsewhere/plans/03-scratch");
        let order = PlanSetOrder {
            order: vec![
                "01-backend".to_string(),
                "02-docs".to_string(),
                "03-scratch".to_string(),
            ],
            ..PlanSetOrder::default()
        };

        let plans = [nested, top_level, outside];
        let entries = plan_set_entries(root, &plans, &order, &PlanConflicts::new());

        assert_eq!(entries[0].group.as_deref(), Some("portal-programme"));
        assert_eq!(entries[1].group, None);
        assert_eq!(entries[2].group, None, "outside plans/");
    }

    /// Workspace config whose only role in use is disabled, so the task
    /// fails without dispatching a provider (bug-a843d4).
    const DISABLED_ROLE_CONFIG: &str = r#"
[agent]
default_model = "claude-sonnet-4-6"

[providers.claude_cli]
kind = "claude_cli"
command = "claude"

[models.claude-sonnet-4-6]
provider = "claude_cli"
slug = "claude-sonnet-4-6"

[agent.roles.researcher]
enabled = false
"#;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn provided_state_hub_receives_the_runs_plan_events() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("roko.toml"), DISABLED_ROLE_CONFIG).expect("config");
        std::fs::write(dir.path().join("README.md"), "# hub test\n").expect("readme");
        let plan_dir = dir.path().join("plans").join("01-disabled");
        std::fs::create_dir_all(&plan_dir).expect("plan dir");
        std::fs::write(
            plan_dir.join("tasks.toml"),
            r#"[meta]
plan = "01-disabled"
max_parallel = 1
skip_enrichment = true

[[task]]
id = "T1"
title = "Disabled-role task"
description = "Fails without dispatch because its role is disabled."
role = "researcher"
status = "ready"
tier = "focused"
files = ["README.md"]
"#,
        )
        .expect("tasks.toml");
        let hub = crate::state_hub::shared_state_hub();

        let exit_code = run_graph_plan(GraphPlanRunParams {
            plans_dir: dir.path().join("plans"),
            workdir: dir.path().to_path_buf(),
            quiet: true,
            json: false,
            resume_plan: None,
            fresh: false,
            force_resume: false,
            max_retries: None,
            max_tasks: 0,
            budget_override: None,
            no_budget: true,
            cli_model_override: None,
            dangerously_skip_permissions: false,
            log_file: None,
            worktree_per_task: false,
            worktree_per_task_explicit: false,
            rich_topology: false,
            promote: None,
            no_tui: true,
            state_hub: Some(hub.clone()),
            interrupt: None,
            max_parallel_plans: None,
            fail_fast: false,
            only_plans: None,
            live_agent_output: crate::graph_task_dispatch::LiveAgentOutput::ToolSteps,
            force_disk_check: false,
            effort: None,
            no_cascade: false,
            frozen_learning: false,
            no_holdout: false,
            metrics: None,
            outbound_floor: None,
        })
        .await
        .expect("run plan set");

        let snapshot = hub.current_snapshot();
        let plan_set = snapshot
            .plan_set
            .as_ref()
            .expect("PlanSetLoaded reached the hub");
        assert_eq!(plan_set.plans.len(), 1);
        assert_eq!(plan_set.plans[0].plan_id, "01-disabled");
        // PlanStarted/PlanCompleted landed in the same hub.
        assert!(snapshot.plan_set_complete());
        // bug-a843d4: the disabled-role task failed, so the plan failed.
        assert_eq!(snapshot.plans["01-disabled"].phase, "failed");
        assert_eq!(exit_code, EXIT_FAILURE);
        assert_eq!(
            crate::graph_checkpoint::canonical_checkpoint_status(dir.path(), "01-disabled"),
            Some(GraphCheckpointStatus::Failed)
        );
    }

    /// gap-568056: a Graph run keeps `.roko/state/status.json` current and
    /// leaves its terminal status there, with this process as the writer.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_graph_run_writes_status_json() {
        let dir = disabled_role_plan_set(&[("a", "a.txt", &[]), ("b", "b.txt", &[])], "");
        let (exit_code, _, _) = run_plan_set(dir.path(), Some(1), None).await;

        let state_dir = RokoLayout::for_project(dir.path()).state_dir();
        let read = crate::runner::status_file::read_runner_status(&state_dir);
        // Finished, though this process, its writer, still runs (bug-f7f3bb).
        assert!(read.is_finished(), "{read:?}");
        let status = read.status().expect("status.json after a Graph run");
        let expected_phase = if exit_code == EXIT_SUCCESS {
            "completed"
        } else {
            "failed"
        };
        assert_eq!(status.phase, expected_phase);
        assert_eq!(status.last_event, "run_completed");
        assert_eq!(status.pid, std::process::id());
        assert!(!status.run_id.is_empty());
        assert_eq!((status.total_plans, status.completed_plans), (2, 2));
        assert_eq!((status.total_tasks, status.finished_tasks), (2, 2));
        assert_eq!((status.active_agents, status.running_tasks), (0, 0));
    }

    /// A workspace with [`DISABLED_ROLE_CONFIG`] plus `extra_config`, and one
    /// single-task plan per `(plan_id, file, depends_on_plan)`.
    fn disabled_role_plan_set(
        plans: &[(&str, &str, &[&str])],
        extra_config: &str,
    ) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("roko.toml"),
            format!("{DISABLED_ROLE_CONFIG}{extra_config}"),
        )
        .expect("config");
        std::fs::write(dir.path().join("README.md"), "# plan set test\n").expect("readme");
        for (plan_id, file, depends_on_plan) in plans {
            let plan_dir = dir.path().join("plans").join(plan_id);
            std::fs::create_dir_all(&plan_dir).expect("plan dir");
            let depends_on_plan = depends_on_plan
                .iter()
                .map(|dependency| format!("{dependency:?}"))
                .collect::<Vec<_>>()
                .join(", ");
            std::fs::write(
                plan_dir.join("tasks.toml"),
                format!(
                    r#"[meta]
plan = "{plan_id}"
max_parallel = 1
skip_enrichment = true

[[task]]
id = "T1"
title = "Disabled-role task"
description = "Fails without dispatch because its role is disabled."
role = "researcher"
status = "ready"
tier = "focused"
files = ["{file}"]
depends_on_plan = [{depends_on_plan}]
"#
                ),
            )
            .expect("tasks.toml");
        }
        dir
    }

    /// A workspace with the stand-in provider of [`fake_provider_workspace`]
    /// plus `extra_config`, and one single-task plan per `(plan_id, file,
    /// depends_on_plan)` whose task passes its verify step.
    #[cfg(unix)]
    fn verified_plan_set(plans: &[(&str, &str, &[&str])], extra_config: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        fake_provider_workspace(dir.path(), 0.0, extra_config);
        std::fs::write(dir.path().join("README.md"), "# plan set test\n").expect("readme");
        for (plan_id, file, depends_on_plan) in plans {
            write_single_task_plan(dir.path(), plan_id, file, depends_on_plan, Some("true"));
        }
        dir
    }

    /// Write plan `plan_id` with one task for the stand-in provider. The task
    /// writes `file`, waits for the plans `depends_on_plan`, and has the
    /// verify command `verify`. Without one it is a `scribe` task, since an
    /// `implementer` task must declare a verify step.
    #[cfg(unix)]
    fn write_single_task_plan(
        dir: &Path,
        plan_id: &str,
        file: &str,
        depends_on_plan: &[&str],
        verify: Option<&str>,
    ) {
        let depends_on_plan = depends_on_plan
            .iter()
            .map(|dependency| format!("{dependency:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        let role = if verify.is_some() {
            "implementer"
        } else {
            "scribe"
        };
        let verify = verify.map_or_else(String::new, |command| {
            format!("verify = [{{ phase = \"structural\", command = {command:?} }}]")
        });
        let plan_dir = dir.join("plans").join(plan_id);
        std::fs::create_dir_all(&plan_dir).expect("plan dir");
        std::fs::write(
            plan_dir.join("tasks.toml"),
            format!(
                r#"[meta]
plan = "{plan_id}"
max_parallel = 1
skip_enrichment = true

[[task]]
id = "T1"
title = "Single task"
description = "Its verify step, if it has one, decides its outcome."
role = "{role}"
status = "ready"
tier = "focused"
model_hint = "graph-model"
files = ["{file}"]
depends_on_plan = [{depends_on_plan}]
timeout_secs = 60
max_retries = 0
{verify}
"#
            ),
        )
        .expect("tasks.toml");
    }

    /// Run the workspace's plan set. Returns the exit code, the plan
    /// lifecycle events in publication order, and the run's hub.
    async fn run_plan_set(
        dir: &Path,
        max_parallel_plans: Option<usize>,
        interrupt: Option<PlanRunInterruptHandle>,
    ) -> (i32, Vec<String>, crate::state_hub::SharedStateHub) {
        run_plan_set_with(dir, max_parallel_plans, interrupt, true, false).await
    }

    /// [`run_plan_set`], enforcing the workspace's `[budget]` unless
    /// `no_budget`, and freezing learning for the run when
    /// `frozen_learning`.
    async fn run_plan_set_with(
        dir: &Path,
        max_parallel_plans: Option<usize>,
        interrupt: Option<PlanRunInterruptHandle>,
        no_budget: bool,
        frozen_learning: bool,
    ) -> (i32, Vec<String>, crate::state_hub::SharedStateHub) {
        let hub = crate::state_hub::shared_state_hub();
        let exit_code = run_graph_plan(GraphPlanRunParams {
            plans_dir: dir.join("plans"),
            workdir: dir.to_path_buf(),
            quiet: true,
            json: false,
            resume_plan: None,
            fresh: false,
            force_resume: false,
            max_retries: None,
            max_tasks: 0,
            budget_override: None,
            no_budget,
            cli_model_override: None,
            dangerously_skip_permissions: false,
            log_file: None,
            worktree_per_task: false,
            worktree_per_task_explicit: false,
            rich_topology: false,
            promote: None,
            no_tui: true,
            state_hub: Some(hub.clone()),
            interrupt,
            max_parallel_plans,
            fail_fast: false,
            only_plans: None,
            live_agent_output: crate::graph_task_dispatch::LiveAgentOutput::ToolSteps,
            force_disk_check: false,
            effort: None,
            no_cascade: false,
            frozen_learning,
            no_holdout: false,
            metrics: None,
            outbound_floor: None,
        })
        .await
        .expect("run plan set");
        let lifecycle = plan_lifecycle(&hub);
        (exit_code, lifecycle, hub)
    }

    /// The plan starts and ends `hub` saw, in order: `start <plan>` and
    /// `end <plan> <success>`.
    fn plan_lifecycle(hub: &crate::state_hub::SharedStateHub) -> Vec<String> {
        hub.subscribe_events_from(0)
            .replay
            .into_iter()
            .filter_map(|envelope| match envelope.payload {
                roko_core::DashboardEvent::PlanStarted { plan_id, .. } => {
                    Some(format!("start {plan_id}"))
                }
                roko_core::DashboardEvent::PlanCompleted { plan_id, success } => {
                    Some(format!("end {plan_id} {success}"))
                }
                _ => None,
            })
            .collect()
    }

    /// The sinks a test attached to the feedback facade of the plan runs
    /// under a workdir ([`attach_sink`]).
    static ATTACHED_SINKS: std::sync::Mutex<
        Vec<(PathBuf, Arc<dyn crate::runtime_feedback::FeedbackSink>)>,
    > = std::sync::Mutex::new(Vec::new());

    /// `facade` with the sinks a test attached for `workdir` after its own,
    /// so the test hears what a real plan run's facade delivers.
    pub(super) fn with_attached_sinks(
        workdir: &Path,
        facade: Arc<crate::runtime_feedback::FeedbackFacade>,
    ) -> Arc<crate::runtime_feedback::FeedbackFacade> {
        let attached = ATTACHED_SINKS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .filter(|(dir, _)| dir == workdir)
            .map(|(_, sink)| Arc::clone(sink))
            .collect::<Vec<_>>();
        if attached.is_empty() {
            return facade;
        }
        let facade = attached
            .into_iter()
            .fold(Arc::unwrap_or_clone(facade), |facade, sink| {
                facade.with_sink(sink)
            });
        Arc::new(facade)
    }

    /// Detaches the sinks attached for its workdir when dropped.
    #[cfg(unix)]
    struct AttachedSinks(PathBuf);

    #[cfg(unix)]
    impl Drop for AttachedSinks {
        fn drop(&mut self) {
            ATTACHED_SINKS
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .retain(|(dir, _)| *dir != self.0);
        }
    }

    /// Attach `sink` to the feedback facade of each plan run under
    /// `workdir`, after the run's own sinks, until the guard drops.
    #[cfg(unix)]
    fn attach_sink(
        workdir: &Path,
        sink: Arc<dyn crate::runtime_feedback::FeedbackSink>,
    ) -> AttachedSinks {
        ATTACHED_SINKS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((workdir.to_path_buf(), sink));
        AttachedSinks(workdir.to_path_buf())
    }

    /// One `PlanCompleted` a plan run's facade delivered, and whether the
    /// workspace's affect file was on disk by then.
    #[cfg(unix)]
    #[derive(Debug, Clone, PartialEq)]
    struct HeardCompletion {
        plan_id: String,
        succeeded: bool,
        tasks_completed: usize,
        tasks_failed: usize,
        total_cost_usd: f64,
        affect_saved: bool,
    }

    /// Hears each `PlanCompleted` a plan run's facade delivers, after the
    /// run's own sinks (gap-a960a1), and whether the affect file is on disk
    /// then. An attempt without a learning label is not appraised, so nothing
    /// autosaves affect for it: when one settles, the log removes the affect
    /// file and notes the attempt's plan, and at that plan's end the file is
    /// back only if the plan-completion save wrote it.
    #[cfg(unix)]
    #[derive(Debug)]
    struct PlanCompletionLog {
        affect_path: PathBuf,
        heard: parking_lot::Mutex<Vec<HeardCompletion>>,
        /// The plans an attempt without a learning label settled in.
        unlabelled: parking_lot::Mutex<BTreeSet<String>>,
    }

    #[cfg(unix)]
    impl PlanCompletionLog {
        fn new(workdir: &Path) -> Arc<Self> {
            Arc::new(Self {
                affect_path: daimon_affect_path(workdir),
                heard: parking_lot::Mutex::default(),
                unlabelled: parking_lot::Mutex::default(),
            })
        }

        /// The plans an attempt without a learning label settled in.
        fn unlabelled(&self) -> BTreeSet<String> {
            self.unlabelled.lock().clone()
        }

        /// What it heard, by plan id.
        fn heard(&self) -> Vec<HeardCompletion> {
            let mut heard = self.heard.lock().clone();
            heard.sort_by(|a, b| a.plan_id.cmp(&b.plan_id));
            heard
        }
    }

    #[cfg(unix)]
    #[async_trait::async_trait]
    impl crate::runtime_feedback::FeedbackSink for PlanCompletionLog {
        fn name(&self) -> &'static str {
            "plan_completion_log"
        }

        fn interested(&self, event: &crate::runtime_feedback::FeedbackEvent) -> bool {
            matches!(
                event,
                crate::runtime_feedback::FeedbackEvent::PlanCompleted { .. }
                    | crate::runtime_feedback::FeedbackEvent::AttemptSettled(_)
            )
        }

        async fn on_event(
            &self,
            event: &crate::runtime_feedback::FeedbackEvent,
        ) -> Result<(), anyhow::Error> {
            use crate::runtime_feedback::FeedbackEvent;
            match event {
                FeedbackEvent::AttemptSettled(verdict) if verdict.learning_success().is_none() => {
                    let _ = std::fs::remove_file(&self.affect_path);
                    self.unlabelled
                        .lock()
                        .insert(verdict.identity.plan_id.clone());
                }
                FeedbackEvent::PlanCompleted {
                    plan_id,
                    succeeded,
                    tasks_completed,
                    tasks_failed,
                    total_cost_usd,
                } => self.heard.lock().push(HeardCompletion {
                    plan_id: plan_id.clone(),
                    succeeded: *succeeded,
                    tasks_completed: *tasks_completed,
                    tasks_failed: *tasks_failed,
                    total_cost_usd: *total_cost_usd,
                    affect_saved: self.affect_path.exists(),
                }),
                _ => {}
            }
            Ok(())
        }
    }

    /// q-6b7cca option 2 (gap-a960a1): a plan run sends one `PlanCompleted`
    /// for each plan it runs, with how the plan ended, its tasks counted as
    /// its run-metrics row counts them, and its settled spend; `d`, which
    /// never starts, sends none. With `[daimon] enabled` the affect state is
    /// on disk as each plan closes: `c`'s unverified attempt has no learning
    /// label, so nothing autosaves affect for it, and the file the log
    /// removed when it settled is back by `c`'s end.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn graph_plan_run_emits_plan_completed_once_per_plan() {
        let dir = tempfile::tempdir().expect("tempdir");
        fake_provider_workspace(dir.path(), 0.004, "\n[daimon]\nenabled = true\n");
        // `a` passes its check, `b` fails it, `c` has none, and `d` needs `b`.
        write_single_task_plan(dir.path(), "a", "a.txt", &[], Some("true"));
        write_single_task_plan(dir.path(), "b", "b.txt", &[], Some("false"));
        write_single_task_plan(dir.path(), "c", "c.txt", &[], None);
        write_single_task_plan(dir.path(), "d", "d.txt", &["b"], Some("true"));
        let log = PlanCompletionLog::new(dir.path());
        let _attached = attach_sink(dir.path(), log.clone());

        let (exit_code, lifecycle, _) = run_plan_set(dir.path(), Some(1), None).await;

        assert_eq!(exit_code, EXIT_FAILURE);
        assert!(
            lifecycle.contains(&"end d false".to_string())
                && !lifecycle.contains(&"start d".to_string()),
            "d never starts: {lifecycle:?}"
        );
        let heard = log.heard();
        let summary = heard
            .iter()
            .map(|heard| {
                (
                    heard.plan_id.as_str(),
                    heard.succeeded,
                    heard.tasks_completed,
                    heard.tasks_failed,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            summary,
            [("a", true, 1, 0), ("b", false, 0, 1), ("c", false, 0, 0)],
            "one PlanCompleted for each plan that ran: {heard:?}"
        );
        assert!(
            log.unlabelled().contains("c"),
            "c's attempt has no learning label, so only the plan-end save puts affect back"
        );
        for completion in &heard {
            let ledger_path = dir
                .path()
                .join(".roko/state/graph")
                .join(&completion.plan_id)
                .join("costs.json");
            let ledger: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&ledger_path).expect("the plan's ledger"))
                    .expect("ledger json");
            let spent = ledger["spent_micro_usd"].as_u64().expect("spent") as f64 / 1e6;
            assert!(spent > 0.0, "{ledger}");
            assert!(
                (completion.total_cost_usd - spent).abs() < 1e-9,
                "{completion:?} carries the plan's settled spend: {ledger}"
            );
            assert!(
                completion.affect_saved,
                "affect is on disk when {} closes",
                completion.plan_id
            );
        }
    }

    /// A stand-in `claude_cli` agent that writes `greeting.txt` and prints
    /// it: two tool calls that succeed, then a finished turn at no cost.
    #[cfg(unix)]
    const GREETING_AGENT: &str = r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"assistant","message":{"id":"msg_1","model":"claude-sonnet-4-6","content":[{"type":"tool_use","id":"tu_1","name":"Write","input":{"file_path":"greeting.txt","content":"hello"}},{"type":"tool_use","id":"tu_2","name":"Bash","input":{"command":"cat greeting.txt"}}],"usage":{"input_tokens":10,"output_tokens":5}}}'
printf '%s\n' '{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"tu_1","content":"written"},{"type":"tool_result","tool_use_id":"tu_2","content":"hello"}]}}'
printf '%s\n' '{"type":"result","subtype":"success","is_error":false,"session_id":"greet","model":"claude-sonnet-4-6","total_cost_usd":0.0,"num_turns":2,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

    /// A workspace of [`fake_provider_workspace`] whose provider is
    /// [`GREETING_AGENT`], with `extra_config` after its `[gates]` table.
    #[cfg(unix)]
    fn greeting_agent_workspace(extra_config: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        fake_provider_workspace(dir.path(), 0.0, extra_config);
        std::fs::write(dir.path().join("fake-provider.sh"), GREETING_AGENT)
            .expect("the greeting agent");
        dir
    }

    /// Write plan `plan_id` with one implementer task about the greeting
    /// banner for [`GREETING_AGENT`], whose verify command is `verify`.
    #[cfg(unix)]
    fn write_greeting_plan(dir: &Path, plan_id: &str, verify: &str) {
        let plan_dir = dir.join("plans").join(plan_id);
        std::fs::create_dir_all(&plan_dir).expect("plan dir");
        std::fs::write(
            plan_dir.join("tasks.toml"),
            format!(
                r#"[meta]
plan = "{plan_id}"
max_parallel = 1
skip_enrichment = true

[[task]]
id = "T1"
title = "Write the greeting banner"
description = "Print a friendly greeting banner from greeting.txt."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "graph-model"
files = ["greeting.txt"]
timeout_secs = 60
max_retries = 0
verify = [{{ phase = "structural", command = {verify:?} }}]
"#
            ),
        )
        .expect("tasks.toml");
    }

    /// The `playbook_written` events `hub` saw, as `(plan, admission,
    /// playbook id)`.
    #[cfg(unix)]
    fn playbooks_written(hub: &crate::state_hub::SharedStateHub) -> Vec<(String, String, String)> {
        hub.subscribe_events_from(0)
            .replay
            .into_iter()
            .filter_map(|envelope| match envelope.payload {
                roko_core::DashboardEvent::PlaybookWritten {
                    plan_id,
                    admission,
                    playbook_id,
                    ..
                } => Some((plan_id, admission, playbook_id)),
                _ => None,
            })
            .collect()
    }

    /// gap-89f985: plan runs write L-play's playbooks. A task that passes its
    /// verify step leaves one playbook in `.roko/learn/playbooks/`: its
    /// agent's tool calls, keyed by the task's topic terms, which the run's
    /// hub announces (`playbook_written`). A task that fails its check, or
    /// has none, leaves none. The next run of a task on that topic retrieves
    /// the playbook into its prompt (one exposure row, included, or withheld
    /// on L-play's arm), and repeating its steps credits it once, by dispatch
    /// when the prompt included it and by the sink otherwise, instead of
    /// adding another. A frozen run writes none.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn verified_plan_task_writes_a_playbook() {
        use roko_learn::playbook::PlaybookStore;
        use roko_learn::telemetry::ExposureItemKind;
        use roko_learn::telemetry::report::RunRecords;

        let dir = greeting_agent_workspace("");
        // `greet` passes its check, `broken` fails it, and `unchecked` has none.
        write_greeting_plan(dir.path(), "greet", "true");
        write_greeting_plan(dir.path(), "broken", "false");
        write_single_task_plan(dir.path(), "unchecked", "c.txt", &[], None);
        let store = PlaybookStore::new(dir.path().join(".roko/learn/playbooks"));

        let (exit_code, _, hub) = run_plan_set(dir.path(), Some(1), None).await;

        assert_eq!(exit_code, EXIT_FAILURE, "broken and unchecked fail");
        let playbooks = store.list().await.expect("read the playbooks");
        let [playbook] = playbooks.as_slice() else {
            panic!("greet's verified pass alone writes a playbook: {playbooks:#?}");
        };
        let id = playbook.id.clone();
        assert!(id.starts_with("ep-greet-T1-"), "{playbook:#?}");
        assert_eq!(
            playbook.goal,
            "Write the greeting banner: Print a friendly greeting banner from greeting.txt."
        );
        assert_eq!(
            playbook.when_pattern.as_deref(),
            Some("banner friendly greeting print"),
            "keyed by the task's topic terms"
        );
        let steps: Vec<(&str, &str)> = playbook
            .steps
            .iter()
            .map(|step| (step.action_kind.as_str(), step.description.as_str()))
            .collect();
        assert_eq!(
            steps,
            [("Write", "greeting.txt"), ("Bash", "cat greeting.txt")]
        );
        assert_eq!((playbook.success_count, playbook.failure_count), (1, 0));
        assert_eq!(
            playbooks_written(&hub),
            [("greet".to_string(), "created".to_string(), id.clone())]
        );

        // The next run: the same task in another plan.
        for plan in ["greet", "broken", "unchecked"] {
            std::fs::remove_dir_all(dir.path().join("plans").join(plan)).expect("drop the plan");
        }
        write_greeting_plan(dir.path(), "greet-again", "true");
        let (exit_code, _, _) = run_plan_set(dir.path(), Some(1), None).await;

        assert_eq!(exit_code, EXIT_SUCCESS);
        let exposures: Vec<bool> = std::fs::read_dir(dir.path().join(".roko/runs"))
            .expect("read .roko/runs")
            .map(|entry| RunRecords::load(&entry.expect("run directory").path()))
            .flat_map(|run| run.expect("read the run's records").exposures)
            .filter(|line| {
                line.record.identity.plan_id == "greet-again"
                    && line.record.item_kind == ExposureItemKind::Playbook
                    && line.record.item_id == id
            })
            .map(|line| line.record.included)
            .collect();
        assert_eq!(
            exposures.len(),
            1,
            "one playbook exposure row: {exposures:?}"
        );
        let playbooks = store.list().await.expect("read the playbooks");
        let [playbook] = playbooks.as_slice() else {
            panic!("the repeat credits the playbook: {playbooks:#?}");
        };
        assert_eq!(playbook.id, id);
        assert_eq!((playbook.success_count, playbook.failure_count), (2, 0));

        let frozen = greeting_agent_workspace("\n[learning]\nfrozen = true\n");
        write_greeting_plan(frozen.path(), "greet", "true");
        let (exit_code, _, hub) = run_plan_set(frozen.path(), Some(1), None).await;

        assert_eq!(exit_code, EXIT_SUCCESS);
        let frozen_store = PlaybookStore::new(frozen.path().join(".roko/learn/playbooks"));
        let playbooks = frozen_store.list().await.expect("read the playbooks");
        assert!(
            playbooks.is_empty(),
            "a frozen run writes none: {playbooks:#?}"
        );
        assert!(playbooks_written(&hub).is_empty());
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn one_plan_at_a_time_keeps_the_execution_order() {
        let dir = verified_plan_set(
            &[
                ("a", "a.txt", &[]),
                ("b", "b.txt", &[]),
                ("c", "c.txt", &["a"]),
            ],
            "",
        );

        let (exit_code, lifecycle, _) = run_plan_set(dir.path(), Some(1), None).await;

        assert_eq!(exit_code, EXIT_SUCCESS);
        assert_eq!(
            lifecycle,
            [
                "start a",
                "end a true",
                "start b",
                "end b true",
                "start c",
                "end c true"
            ]
        );
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn independent_plans_run_side_by_side() {
        let dir = verified_plan_set(&[("a", "a.txt", &[]), ("b", "b.txt", &[])], "");

        let (exit_code, lifecycle, hub) = run_plan_set(dir.path(), Some(2), None).await;

        assert_eq!(exit_code, EXIT_SUCCESS);
        assert_eq!(
            lifecycle[..2],
            ["start a", "start b"],
            "both plans start before either ends: {lifecycle:?}"
        );
        assert!(lifecycle.contains(&"end a true".to_string()));
        assert!(lifecycle.contains(&"end b true".to_string()));
        let snapshot = hub.current_snapshot();
        assert!(snapshot.plan_set_complete());
        for plan_id in ["a", "b"] {
            assert_eq!(
                crate::graph_checkpoint::canonical_checkpoint_status(dir.path(), plan_id),
                Some(GraphCheckpointStatus::Succeeded)
            );
        }
    }

    /// Write a stand-in `claude_cli` provider and a `roko.toml` that routes
    /// every task to it, with the spec gate off (its fixture checks, such as
    /// `true`, can never fail), followed by `extra_config`. The provider ignores its
    /// prompt, appends a line to `provider-calls`, and reports a finished turn
    /// costing `cost_usd`, so each task's verify step alone decides its
    /// outcome.
    #[cfg(unix)]
    fn fake_provider_workspace(dir: &Path, cost_usd: f64, extra_config: &str) {
        use std::os::unix::fs::PermissionsExt as _;

        let provider = dir.join("fake-provider.sh");
        std::fs::write(
            &provider,
            format!(
                r#"#!/bin/sh
set -eu
cat >/dev/null
printf 'call\n' >> "$(dirname "$0")/provider-calls"
printf '%s\n' '{{"type":"content_block_delta","delta":{{"text":"done"}}}}'
printf '%s\n' '{{"type":"result","session_id":"fake","model":"claude-sonnet-4-6","total_cost_usd":{cost_usd},"usage":{{"input_tokens":1,"output_tokens":1}},"is_error":false}}'
"#
            ),
        )
        .expect("provider script");
        std::fs::set_permissions(&provider, std::fs::Permissions::from_mode(0o755))
            .expect("make provider executable");
        std::fs::write(
            dir.join("roko.toml"),
            format!(
                r#"
[agent]
default_model = "graph-model"
command = {provider:?}
bare_mode = false

[providers.graph-cli]
kind = "claude_cli"
command = {provider:?}

[models.graph-model]
provider = "graph-cli"
slug = "claude-sonnet-4-6"
context_window = 200000

# These runs test the runner, not the spec gate (3231), and their checks,
# such as `true`, are fixtures that can never fail (HF2).
[spec_quality]
mode = "off"

[gates]
sibling_settle_secs = 0
{extra_config}"#,
                provider = provider.display().to_string()
            ),
        )
        .expect("config");
    }

    /// Write plan `plan_id` with the extra `[meta]` lines `meta` and one task
    /// per `(id, depends_on, verify)`, where `verify` is the task's single
    /// verify command.
    fn write_verify_plan(dir: &Path, plan_id: &str, meta: &str, tasks: &[(&str, &[&str], &str)]) {
        let tasks = tasks
            .iter()
            .map(|(id, depends_on, verify)| {
                let depends_on = depends_on
                    .iter()
                    .map(|dependency| format!("{dependency:?}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    r#"
[[task]]
id = "{id}"
title = "Task {id}"
description = "Its verify step decides its outcome."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "graph-model"
files = ["{id}.txt"]
depends_on = [{depends_on}]
verify = [{{ phase = "structural", command = "{verify}", fail_msg = "{id} failed" }}]
timeout_secs = 60
max_retries = 0
"#
                )
            })
            .collect::<String>();
        let plan_dir = dir.join("plans").join(plan_id);
        std::fs::create_dir_all(&plan_dir).expect("plan dir");
        std::fs::write(
            plan_dir.join("tasks.toml"),
            format!("[meta]\nplan = \"{plan_id}\"\nskip_enrichment = true\n{meta}\n{tasks}"),
        )
        .expect("tasks.toml");
    }

    /// `T1` fails; `T2` passes once `T1` has failed; `T3` needs only `T2`;
    /// `T4` needs `T1`. Each passing task leaves a `<id>.verified` file.
    const ISOLATION_TASKS: &[(&str, &[&str], &str)] = &[
        ("T1", &[], "false"),
        ("T2", &[], "sleep 2 && touch T2.verified"),
        ("T3", &["T2"], "touch T3.verified"),
        ("T4", &["T1"], "touch T4.verified"),
    ];

    /// gap-4d835d: in a plan run a failed task blocks only its own
    /// dependants. `T3` does not depend on the failed `T1` and becomes ready
    /// only after `T1` has failed, yet it still runs; `T4` needs `T1`, is
    /// skipped, and is reported and recorded as blocked by `T1`; the plan
    /// reports failure once every task has settled.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_failed_task_blocks_only_its_dependants() {
        let dir = tempfile::tempdir().expect("tempdir");
        fake_provider_workspace(dir.path(), 0.0, "");
        write_verify_plan(dir.path(), "isolation", "max_parallel = 2", ISOLATION_TASKS);

        let (exit_code, _, hub) = run_plan_set(dir.path(), Some(1), None).await;

        assert_eq!(exit_code, EXIT_FAILURE);
        assert!(dir.path().join("T2.verified").exists(), "T2 passes");
        assert!(
            dir.path().join("T3.verified").exists(),
            "T3 does not depend on the failed T1, so it runs"
        );
        assert!(
            !dir.path().join("T4.verified").exists(),
            "T4 depends on the failed T1, so it is skipped"
        );
        assert_eq!(
            crate::graph_checkpoint::canonical_checkpoint_status(dir.path(), "isolation"),
            Some(GraphCheckpointStatus::Failed)
        );
        let outcomes = crate::graph_checkpoint::canonical_task_outcomes(dir.path(), "isolation")
            .expect("recorded task outcomes");
        assert_eq!(outcomes.failed, BTreeSet::from(["T1".to_string()]));
        assert_eq!(
            outcomes.blocked_by,
            BTreeMap::from([("T4".to_string(), "T1".to_string())])
        );
        assert!(outcomes.not_started.is_empty(), "{outcomes:?}");
        assert!(
            hub.current_snapshot().event_log.iter().any(|entry| {
                entry.event_type == "graph.task_blocked"
                    && entry
                        .message
                        .contains("task 'T4' blocked by failed task 'T1'")
            }),
            "the block is reported"
        );
        // The live task list shows T4 blocked by T1, and the plan does not
        // count it as a skipped task that is done (gap-f59fe9).
        let snapshot = hub.current_snapshot();
        let blocked = snapshot.tasks.get("isolation/T4").expect("T4 is listed");
        assert_eq!(blocked.outcome.as_deref(), Some("blocked"));
        assert_eq!(blocked.blocked_by.as_deref(), Some("T1"));
        assert_eq!(snapshot.plans["isolation"].tasks_skipped, 0);
    }

    /// `fail_fast`, from `[conductor] plan_failure_policy` or a plan's
    /// `[meta] failure_policy`, starts no further task after the first
    /// failure: `T3` never runs and is recorded as not started.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn fail_fast_policy_stops_a_plan_at_its_first_failure() {
        for (config, meta) in [
            (
                "[conductor]\nplan_failure_policy = \"fail_fast\"\n",
                "max_parallel = 2",
            ),
            ("", "max_parallel = 2\nfailure_policy = \"fail_fast\""),
        ] {
            let dir = tempfile::tempdir().expect("tempdir");
            fake_provider_workspace(dir.path(), 0.0, config);
            write_verify_plan(dir.path(), "isolation", meta, ISOLATION_TASKS);

            let (exit_code, _, _) = run_plan_set(dir.path(), Some(1), None).await;

            assert_eq!(exit_code, EXIT_FAILURE, "{config}{meta}");
            assert!(dir.path().join("T2.verified").exists(), "T2 was running");
            assert!(
                !dir.path().join("T3.verified").exists(),
                "no task starts after the failure ({config}{meta})"
            );
            let outcomes =
                crate::graph_checkpoint::canonical_task_outcomes(dir.path(), "isolation")
                    .expect("recorded task outcomes");
            assert_eq!(outcomes.failed, BTreeSet::from(["T1".to_string()]));
            assert_eq!(
                outcomes.not_started.get("T3").map(String::as_str),
                Some("aborted after graph failure")
            );
        }
    }

    /// The identity a checkpoint records for plan `plan_id` in `dir` when its
    /// graph is converted with `max_parallel`.
    #[cfg(unix)]
    fn plan_identity(dir: &Path, plan_id: &str, max_parallel: u32) -> String {
        let plan_dir = dir.join("plans").join(plan_id);
        let content = std::fs::read_to_string(plan_dir.join("tasks.toml")).expect("tasks.toml");
        let tasks_file = crate::task_parser::TasksFile::parse_str(&content).expect("parse plan");
        let tasks: Vec<(String, roko_graph::convert::PlanTaskInfo)> = tasks_file
            .tasks
            .iter()
            .map(|task| {
                let info = roko_graph::convert::PlanTaskInfo {
                    title: task.title.clone(),
                    description: None,
                    role: task.role.clone(),
                    tier: task.tier.clone(),
                    model_hint: None,
                    files: task.files.clone(),
                    depends_on: task.depends_on.clone(),
                    depends_on_plan: Vec::new(),
                    timeout_secs: task.timeout_secs,
                    max_retries: task.max_retries,
                    domain: None,
                    sequence: task.sequence,
                    full_config_json: serde_json::Value::Null,
                };
                (task.id.clone(), info)
            })
            .collect();
        let graph = roko_graph::convert::plan_to_graph(
            plan_id,
            &plan_dir.display().to_string(),
            &tasks,
            max_parallel,
        )
        .expect("convert plan");
        let authored = roko_graph::AuthoredPlan::from_tasks_toml(&content).expect("authored");
        roko_graph::plan_graph_fingerprint(&graph, &authored).expect("fingerprint")
    }

    /// gap-272448: a plan that omits `max_parallel` runs its independent
    /// tasks together when every task declares its files. Each verify step
    /// here waits until all three tasks have started, so it passes only when
    /// they run at the same time. When a task that can write declares no
    /// files, the plan runs one task at a time: a `mkdir` lock fails whenever
    /// two verify steps overlap. The checkpoint records the identity the plan
    /// had when an omitted `max_parallel` meant 1, so older checkpoints still
    /// resume.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn omitted_max_parallel_runs_disjoint_tasks_together() {
        const IDS: [&str; 3] = ["T1", "T2", "T3"];
        let no_dependencies: &[&str] = &[];
        let verified = |dir: &Path, id: &str| dir.join(format!("{id}.verified")).exists();

        let wide = tempfile::tempdir().expect("tempdir");
        fake_provider_workspace(wide.path(), 0.0, "");
        let barrier = IDS.map(|id| {
            format!(
                "touch {id}.started; for _ in $(seq 100); do test -f T1.started && \
                 test -f T2.started && test -f T3.started && touch {id}.verified && exit 0; \
                 sleep 0.1; done; exit 1"
            )
        });
        let tasks: Vec<(&str, &[&str], &str)> = IDS
            .iter()
            .zip(&barrier)
            .map(|(id, verify)| (*id, no_dependencies, verify.as_str()))
            .collect();
        write_verify_plan(wide.path(), "wide", "", &tasks);

        let (exit_code, _, _) = run_plan_set(wide.path(), Some(1), None).await;

        assert_eq!(exit_code, EXIT_SUCCESS, "the tasks ran together");
        for id in IDS {
            assert!(verified(wide.path(), id), "{id}");
        }
        let manifest = std::fs::read(wide.path().join(".roko/state/graph/wide/checkpoint.json"))
            .expect("checkpoint manifest");
        let manifest: crate::graph_checkpoint::GraphCheckpointManifest =
            serde_json::from_slice(&manifest).expect("parse checkpoint manifest");
        assert_eq!(
            manifest.graph_fingerprint,
            plan_identity(wide.path(), "wide", 1)
        );
        assert_ne!(
            manifest.graph_fingerprint,
            plan_identity(wide.path(), "wide", 3)
        );

        let narrow = tempfile::tempdir().expect("tempdir");
        fake_provider_workspace(narrow.path(), 0.0, "");
        let lock = IDS
            .map(|id| format!("mkdir lock.d && sleep 0.5 && rmdir lock.d && touch {id}.verified"));
        let tasks: Vec<(&str, &[&str], &str)> = IDS
            .iter()
            .zip(&lock)
            .map(|(id, verify)| (*id, no_dependencies, verify.as_str()))
            .collect();
        write_verify_plan(narrow.path(), "narrow", "", &tasks);
        // T3 becomes a scribe that declares no files: what it writes is unknown.
        let tasks_toml = narrow.path().join("plans/narrow/tasks.toml");
        let content = std::fs::read_to_string(&tasks_toml).expect("tasks.toml");
        let (head, task_t3) = content.split_at(content.find("id = \"T3\"").expect("T3"));
        let task_t3 = task_t3
            .replacen("role = \"implementer\"", "role = \"scribe\"", 1)
            .replace("files = [\"T3.txt\"]", "files = []");
        std::fs::write(&tasks_toml, format!("{head}{task_t3}")).expect("rewrite tasks.toml");

        let (exit_code, _, _) = run_plan_set(narrow.path(), Some(1), None).await;

        assert_eq!(exit_code, EXIT_SUCCESS, "no two verify steps overlapped");
        for id in IDS {
            assert!(verified(narrow.path(), id), "{id}");
        }
    }

    /// gap-d31457: `--budget-override` is a hard plan ceiling, as a configured
    /// one is; only `--no-budget` lets dispatch go on past a spent budget.
    #[test]
    fn a_budget_override_is_a_hard_ceiling() {
        for (budget_override, no_budget, expected) in [
            (Some(2.0), false, (2.0, false)),
            (Some(0.0), false, (0.0, false)),
            (Some(-1.0), false, (0.0, false)),
            (None, false, (25.0, false)),
            (None, true, (0.0, true)),
        ] {
            assert_eq!(
                resolve_budget_ceiling(budget_override, no_budget, 25.0),
                expected
            );
        }
    }

    /// Once a plan's settled spend reaches `[budget] max_plan_usd`, no further
    /// task starts, even under the default `skip_failed` policy: the tasks
    /// waiting on the spent one are recorded as not started, not failed.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_spent_plan_budget_starts_no_further_task() {
        let dir = tempfile::tempdir().expect("tempdir");
        fake_provider_workspace(
            dir.path(),
            0.002,
            "[budget]\nmax_plan_usd = 0.001\nmax_turn_usd = 0.001\n",
        );
        write_verify_plan(
            dir.path(),
            "budget",
            "max_parallel = 2",
            &[
                ("T1", &[], "touch T1.verified"),
                ("T2", &["T1"], "touch T2.verified"),
                ("T3", &["T1"], "touch T3.verified"),
            ],
        );

        let (exit_code, _, _) = run_plan_set_with(dir.path(), Some(1), None, false, false).await;

        assert_eq!(exit_code, EXIT_FAILURE);
        assert!(
            dir.path().join("T1.verified").exists(),
            "T1 spends the budget"
        );
        let provider_calls =
            std::fs::read_to_string(dir.path().join("provider-calls")).expect("provider call log");
        assert_eq!(provider_calls.lines().count(), 1, "only T1 was dispatched");
        let outcomes = crate::graph_checkpoint::canonical_task_outcomes(dir.path(), "budget")
            .expect("recorded task outcomes");
        assert!(outcomes.failed.is_empty(), "{outcomes:?}");
        for task_id in ["T2", "T3"] {
            let reason = outcomes.not_started.get(task_id).expect("not started");
            assert!(reason.starts_with("plan budget exhausted"), "{reason}");
        }
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn conductor_max_parallel_plans_alone_sets_the_limit() {
        let dir = verified_plan_set(
            &[("a", "a.txt", &[]), ("b", "b.txt", &[])],
            "\n[conductor]\nmax_parallel_plans = 2\n",
        );

        let (exit_code, lifecycle, _) = run_plan_set(dir.path(), None, None).await;

        assert_eq!(exit_code, EXIT_SUCCESS);
        assert_eq!(lifecycle[..2], ["start a", "start b"], "{lifecycle:?}");
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn plans_sharing_a_file_never_run_side_by_side() {
        let dir = verified_plan_set(&[("a", "README.md", &[]), ("b", "README.md", &[])], "");

        let (exit_code, lifecycle, hub) = run_plan_set(dir.path(), Some(2), None).await;

        assert_eq!(exit_code, EXIT_SUCCESS);
        assert_eq!(
            lifecycle,
            ["start a", "end a true", "start b", "end b true"]
        );
        let snapshot = hub.current_snapshot();
        let entry = &snapshot.plan_set.as_ref().expect("plan set").plans[1];
        assert_eq!(entry.conflicts_with, ["a"]);
        assert!(
            snapshot
                .event_log
                .iter()
                .any(|entry| entry.event_type == "graph.plan_waiting"
                    && entry.message.contains("both write README.md")),
            "the wait is explained"
        );
    }

    /// gap-29a84b: a plan whose only task has no verify step completes as
    /// unverified, not succeeded. The run fails, the checkpoint says so, and
    /// the plan that depends on it never starts.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn plan_with_unverified_task_does_not_succeed() {
        let dir = verified_plan_set(&[("b", "b.txt", &["a"])], "");
        write_single_task_plan(dir.path(), "a", "a.txt", &[], None);

        let (exit_code, lifecycle, hub) = run_plan_set(dir.path(), Some(1), None).await;

        assert_eq!(exit_code, EXIT_FAILURE);
        assert_eq!(lifecycle, ["start a", "end a false", "end b false"]);
        assert_eq!(
            crate::graph_checkpoint::canonical_checkpoint_status(dir.path(), "a"),
            Some(GraphCheckpointStatus::Unverified)
        );
        let provider_calls =
            std::fs::read_to_string(dir.path().join("provider-calls")).expect("provider call log");
        assert_eq!(provider_calls.lines().count(), 1, "only a's task ran");
        assert!(
            hub.current_snapshot()
                .event_log
                .iter()
                .any(|entry| entry.event_type == "graph.plan_unverified"),
            "the outcome is explained"
        );
    }

    /// S01 P0-2: a plan run writes its checkpoint run's manifest (harness
    /// build, config fingerprint, one invocation) and closes it with the
    /// run's attempt counts; resuming the run records a second invocation.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn graph_plan_run_writes_run_manifest() {
        use roko_learn::telemetry::RunProvenanceManifest;

        let dir = verified_plan_set(&[("a", "a.txt", &[])], "");
        let (exit_code, _, _) = run_plan_set(dir.path(), Some(1), None).await;
        assert_eq!(exit_code, EXIT_SUCCESS);

        let runs_dir = dir.path().join(".roko/runs");
        let run_dirs: Vec<PathBuf> = std::fs::read_dir(&runs_dir)
            .expect("read .roko/runs")
            .map(|entry| entry.expect("run directory").path())
            .collect();
        assert_eq!(run_dirs.len(), 1, "one run, one directory: {run_dirs:?}");
        let run_dir = &run_dirs[0];
        let manifest = RunProvenanceManifest::load(run_dir)
            .expect("read the manifest")
            .expect("the run wrote a manifest");
        let run_id = run_dir.file_name().and_then(|name| name.to_str());
        assert_eq!(Some(manifest.run_id.as_str()), run_id);
        assert_eq!(manifest.kind, "plan_run");
        assert_eq!(manifest.plan_ids, ["a"]);
        assert_eq!(manifest.harness.sha, env!("ROKO_GIT_HASH"));
        assert_eq!(
            manifest.harness.rustc.as_deref(),
            Some(env!("ROKO_RUSTC_VERSION"))
        );
        assert!(
            manifest.config.hash.starts_with("b3:"),
            "{}",
            manifest.config.hash
        );
        assert_eq!(manifest.config.hash.len(), 3 + 64);
        let invocation = &manifest.invocations[..];
        assert_eq!(invocation.len(), 1);
        assert_eq!((invocation[0].inv, invocation[0].resumed), (1, false));
        assert_eq!(invocation[0].pid, std::process::id());
        assert_eq!(invocation[0].harness.as_ref(), Some(&manifest.harness));
        assert_eq!(invocation[0].config.as_ref(), Some(&manifest.config));
        let closed = manifest.closed.as_ref().expect("the run closed");
        assert_eq!(closed.status, "succeeded");
        let counts = (
            closed.attempts_opened,
            closed.attempts_settled,
            closed.abandoned,
        );
        assert_eq!(counts, (1, 1, 0));

        // The second run resumes the checkpoint: same run, one more
        // invocation, and no new attempt for the replayed task.
        run_plan_set(dir.path(), Some(1), None).await;
        let resumed = RunProvenanceManifest::load(run_dir)
            .expect("read the manifest")
            .expect("the manifest is still there");
        assert_eq!(resumed.run_id, manifest.run_id);
        assert_eq!(resumed.config.hash, manifest.config.hash);
        let invocations: Vec<(u32, bool)> = resumed
            .invocations
            .iter()
            .map(|invocation| (invocation.inv, invocation.resumed))
            .collect();
        assert_eq!(invocations, [(1, false), (2, true)]);
        assert!(
            !resumed.mixed_provenance,
            "the same build resumed the run: {:?}",
            resumed.invocations
        );
        let closed = resumed.closed.as_ref().expect("the resumed run closed");
        assert_eq!(closed.attempts_opened, 1);
    }

    /// gap-9aca5d, decision 2 of 2026-10-15: `[self_model] mode` defaults to
    /// shadow, so a plan run with no `[self_model]` section forecasts its
    /// one routed attempt, writing one `predictions.jsonl` row and the
    /// self-model's state file. Shadow never changes the route (S04 §4.7):
    /// the model it dispatches is the one an explicit `mode = "off"` run
    /// dispatches too.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn default_plan_run_forecasts_in_shadow() {
        use roko_learn::telemetry::report::RunRecords;

        let shadow_dir = verified_plan_set(&[("a", "a.txt", &[])], "");
        let (shadow_exit, _, _) = run_plan_set(shadow_dir.path(), Some(1), None).await;
        assert_eq!(shadow_exit, EXIT_SUCCESS);

        let off_dir = verified_plan_set(&[("a", "a.txt", &[])], "[self_model]\nmode = \"off\"\n");
        let (off_exit, _, _) = run_plan_set(off_dir.path(), Some(1), None).await;
        assert_eq!(off_exit, EXIT_SUCCESS);

        let shadow_manifest = only_run_manifest(shadow_dir.path());
        let shadow_run_dir = shadow_dir
            .path()
            .join(".roko/runs")
            .join(&shadow_manifest.run_id);
        let off_manifest = only_run_manifest(off_dir.path());
        let off_run_dir = off_dir.path().join(".roko/runs").join(&off_manifest.run_id);

        let predictions = std::fs::read_to_string(shadow_run_dir.join("predictions.jsonl"))
            .expect("a default run forecasts and writes predictions.jsonl");
        let rows: Vec<&str> = predictions
            .lines()
            .filter(|line| !line.is_empty())
            .collect();
        assert_eq!(
            rows.len(),
            1,
            "one routed attempt, one prediction: {rows:?}"
        );

        let state_path = shadow_dir
            .path()
            .join(roko_core::config::self_model::DEFAULT_STATE_PATH);
        assert!(
            state_path.exists(),
            "a default run saves the self-model's state at {}",
            state_path.display()
        );
        assert!(
            !off_run_dir.join("predictions.jsonl").exists(),
            "mode = off forecasts nothing"
        );

        let shadow_model = RunRecords::load(&shadow_run_dir)
            .expect("read the shadow run's records")
            .verdicts[0]
            .record
            .executed
            .model_requested
            .clone();
        let off_model = RunRecords::load(&off_run_dir)
            .expect("read the off run's records")
            .verdicts[0]
            .record
            .executed
            .model_requested
            .clone();
        assert_eq!(
            shadow_model, off_model,
            "shadow forecasts; it never changes the route"
        );
    }

    /// S01 §5.8: a plan run writes its run's `census.json` from the
    /// dispatcher it built: every learning component S01 names, in census
    /// order, with the attempt log wired, stamped with this harness build.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn plan_run_writes_census_report() {
        use roko_learn::telemetry::CensusReport;

        let dir = verified_plan_set(&[("a", "a.txt", &[])], "");
        let (exit_code, _, _) = run_plan_set(dir.path(), Some(1), None).await;
        assert_eq!(exit_code, EXIT_SUCCESS);

        let runs_dir = dir.path().join(".roko/runs");
        let run_dirs: Vec<PathBuf> = std::fs::read_dir(&runs_dir)
            .expect("read .roko/runs")
            .map(|entry| entry.expect("run directory").path())
            .collect();
        assert_eq!(run_dirs.len(), 1, "one run, one directory: {run_dirs:?}");
        let census = CensusReport::load(&run_dirs[0])
            .expect("read the census")
            .expect("the run wrote a census");
        let run_id = run_dirs[0].file_name().and_then(|name| name.to_str());
        assert_eq!(Some(census.run_id.as_str()), run_id);
        assert_eq!(census.schema_version, "roko.census/1");
        assert_eq!(census.harness_sha, env!("ROKO_GIT_HASH"));
        let ids: Vec<&str> = census
            .components
            .iter()
            .map(|component| component.id.as_str())
            .collect();
        assert_eq!(
            ids,
            [
                "sink.episode",
                "sink.routing",
                "sink.knowledge_ingestion",
                "sink.playbook_outcome",
                "sink.playbook_producer",
                "sink.error_pattern",
                "sink.section_effect",
                "store.attempt_log",
                "store.prompt_experiment",
                "store.decision_writer",
                "store.exposure_writer",
                "store.record_access",
                "reader.gate_thresholds",
                "store.arm_set",
                "reader.withhold_arms",
                "store.placebo",
                "sink.router_source_credit",
            ]
        );
        let attempt_log = census.component("store.attempt_log");
        assert!(
            attempt_log.is_some_and(|component| component.wired),
            "{census:?}"
        );
        assert_eq!(
            attempt_log.map(|component| component.kind.as_str()),
            Some("store")
        );
    }

    /// The manifest of the one run in workspace `dir`.
    #[cfg(unix)]
    fn only_run_manifest(dir: &Path) -> roko_learn::telemetry::RunProvenanceManifest {
        let run_dirs: Vec<PathBuf> = std::fs::read_dir(dir.join(".roko/runs"))
            .expect("read .roko/runs")
            .map(|entry| entry.expect("run directory").path())
            .collect();
        assert_eq!(run_dirs.len(), 1, "one run, one directory: {run_dirs:?}");
        roko_learn::telemetry::RunProvenanceManifest::load(&run_dirs[0])
            .expect("read the manifest")
            .expect("the run wrote a manifest")
    }

    /// Decision 2218: `--frozen-learning` freezes a run's learning (the
    /// params' `frozen_learning`), and `[learning] frozen = true` every
    /// run's. Either reaches the run's config, so its fingerprint is a frozen
    /// one, and its manifest records `ablation_flags = ["learning_frozen"]`;
    /// neither leaves the flags empty.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn frozen_learning_switch_reaches_config_and_manifest() {
        let dir = verified_plan_set(&[("a", "a.txt", &[])], "");
        let (exit_code, _, _) = run_plan_set(dir.path(), Some(1), None).await;
        assert_eq!(exit_code, EXIT_SUCCESS);
        let live = only_run_manifest(dir.path());
        assert!(
            live.experiment.ablation_flags.is_empty(),
            "{:?}",
            live.experiment
        );

        // The run resumes with the flag: that invocation's config is frozen.
        let (exit_code, _, _) = run_plan_set_with(dir.path(), Some(1), None, true, true).await;
        assert_eq!(exit_code, EXIT_SUCCESS);
        let frozen = only_run_manifest(dir.path());
        assert_eq!(frozen.experiment.ablation_flags, ["learning_frozen"]);
        let hashes: Vec<&str> = frozen
            .invocations
            .iter()
            .filter_map(|invocation| invocation.config.as_ref())
            .map(|config| config.hash.as_str())
            .collect();
        assert_eq!(hashes.len(), 2, "{:?}", frozen.invocations);
        assert_ne!(hashes[0], hashes[1], "a frozen config is another config");
        assert!(frozen.mixed_provenance, "a live run resumed frozen");

        let configured = verified_plan_set(&[("a", "a.txt", &[])], "\n[learning]\nfrozen = true\n");
        let (exit_code, _, _) = run_plan_set(configured.path(), Some(1), None).await;
        assert_eq!(exit_code, EXIT_SUCCESS);
        let manifest = only_run_manifest(configured.path());
        assert_eq!(manifest.experiment.ablation_flags, ["learning_frozen"]);
    }

    /// The `[gates]` and later lines of the workspace of
    /// [`run_seeded_learning_plan`]: one verify run per attempt (no auto-fix
    /// re-run), T0 reflexes on, gate thresholds saved after every verify run,
    /// no model ladder, so every task runs on its hinted model, and maximize
    /// mode, so no arm withholds the seeded knowledge from a prompt.
    #[cfg(unix)]
    const LEARNED_STATE_CONFIG: &str = r#"cargo_fix_enabled = false

[learning]
t0_reflexes = true
gate_threshold_flush_interval = 1

[routing.ladder]
enabled = false

[experiments]
maximize = true
"#;

    /// The plan of [`run_seeded_learning_plan`]. T1 passes its verify step,
    /// and T2 fails it once and passes on its one retry. T3 has no verify
    /// step, so the seeded T0 reflex rule serves it and it ends unverified,
    /// and the plan with it. The seeded knowledge entry shares the words of
    /// T1's and T2's descriptions.
    #[cfg(unix)]
    const LEARNED_STATE_TASKS: &str = r#"[meta]
plan = "learned"
max_parallel = 1
# T3 ends unverified on purpose; without this, plan run refuses the plan (PLAN_037).
allow_unverified = true

[[task]]
id = "T1"
title = "Passing task"
description = "Its verify step decides its outcome."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "graph-model"
files = ["t1.txt"]
verify = [{ phase = "structural", command = "true" }]
timeout_secs = 60
max_retries = 0

[[task]]
id = "T2"
title = "Retried task"
description = "Its verify step decides its outcome, and fails once."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "graph-model"
files = ["t2.txt"]
depends_on = ["T1"]
verify = [{ phase = "structural", command = "test -f retried || { touch retried; false; }" }]
timeout_secs = 60
max_retries = 1

[[task]]
id = "T3"
title = "Reflex-served task"
description = "It has no verify step."
role = "scribe"
status = "ready"
tier = "focused"
model_hint = "graph-model"
files = ["t3.txt"]
depends_on = ["T2"]
timeout_secs = 60
max_retries = 0
"#;

    /// The files under `.roko/` a frozen run may write (decision 2218): its
    /// telemetry, and the provider circuit breaker. Every other file under
    /// `learn/`, `neuro/` and `daimon/`, and `episodes.jsonl`, is learned
    /// state. That includes the loop auditor's ledger and census: attempt
    /// open draws each chain's arms from the ledger's states (gap-addf2a),
    /// so a frozen run has no audit tick.
    #[cfg(unix)]
    const FROZEN_RUN_TELEMETRY: &[&str] = &[
        // Each attempt's and helper call's spend: `roko status`, and the
        // daily budget it is checked against.
        "learn/costs.jsonl",
        // Efficiency events and provider-call rows: `roko learn efficiency`.
        "learn/efficiency.jsonl",
        // One summary row per run: `roko show`.
        "learn/run-metrics.jsonl",
        // Gate-gaming alerts: `roko diagnose`.
        "learn/gate-gaming-alerts.jsonl",
        // The inference gateway's per-call log: serve's gateway routes.
        "learn/gateway.jsonl",
        // The run's failed verify steps: `roko diagnose`, and a revision of
        // the same plan, which reads its last run as it reads its checkpoint.
        "learn/gate-failures.jsonl",
        // Which providers are down (the circuit breaker): availability, not
        // learned behaviour, so a frozen run still stops calling a provider
        // that fails.
        "learn/provider-health.json",
    ];

    /// Whether a frozen run may write `path`, a path under `.roko/`: a file
    /// of [`FROZEN_RUN_TELEMETRY`], the advisory lock beside one, or the
    /// staging file of an atomic write to one.
    #[cfg(unix)]
    fn frozen_run_may_write(path: &str) -> bool {
        let file = path.split_once(".tmp.").map_or(path, |(file, _)| file);
        let file = file.strip_suffix(".lock").unwrap_or(file);
        FROZEN_RUN_TELEMETRY.contains(&file)
    }

    /// Seed workspace `dir` with learned state from decision 2218's list: a
    /// cascade router, gate thresholds, a playbook, a prompt-experiment
    /// store, an error pattern, the T0 reflex rule that serves T3 of
    /// [`LEARNED_STATE_TASKS`], a knowledge entry the other tasks' prompts
    /// retrieve, and an episode.
    #[cfg(unix)]
    fn seed_learned_state(dir: &Path) {
        use roko_learn::cascade_router::CascadeRouter;
        use roko_learn::error_pattern_store::ErrorPatternStore;
        use roko_learn::playbook::Playbook;
        use roko_learn::reflex_store::{
            PromotionCandidate, ReflexAction, ReflexCondition, ReflexStore,
        };

        use crate::runner::persist::GateThresholds;

        let roko = dir.join(".roko");
        let learn = roko.join("learn");
        std::fs::create_dir_all(learn.join("playbooks")).expect("create the playbook directory");
        CascadeRouter::new(vec!["claude-sonnet-4-6".to_string()])
            .save(&learn.join("cascade-router.json"))
            .expect("seed the cascade router");
        let thresholds = serde_json::to_vec(&GateThresholds::default()).expect("thresholds");
        std::fs::write(learn.join("gate-thresholds.json"), thresholds)
            .expect("seed the gate thresholds");
        let mut playbook = Playbook::new("pb-verify", "Pass the verify step");
        playbook.when_pattern = Some("verify step".to_string());
        let playbook = serde_json::to_vec_pretty(&playbook).expect("serialize the playbook");
        std::fs::write(learn.join("playbooks/pb-verify.json"), playbook)
            .expect("seed the playbook");
        roko_learn::prompt_experiment::ExperimentStore::new()
            .save(&learn.join("experiments.json"))
            .expect("seed the prompt experiments");
        let mut patterns = ErrorPatternStore::empty();
        patterns.append("verify step failed", "verify", "seed", None);
        patterns
            .save(&learn.join("error-patterns.json"))
            .expect("seed the error patterns");
        let reflexes = ReflexStore::open(learn.join("reflexes.jsonl"));
        let candidate = PromotionCandidate {
            episode_id: "episode-seed".to_string(),
            condition: ReflexCondition {
                context: Some("Reflex-served".to_string()),
                ..ReflexCondition::default()
            },
            action: ReflexAction {
                tool: "respond".to_string(),
                args: "cached reflex output".to_string(),
            },
        };
        assert!(reflexes.try_promote(&candidate, 3), "seed the reflex rule");
        let neuro = roko.join("neuro");
        std::fs::create_dir_all(&neuro).expect("create the knowledge store's directory");
        let entry = serde_json::json!({
            "id": "kn-frozen",
            "content": "A verify step decides each outcome",
            "confidence": 0.8,
            "created_at": chrono::Utc::now(),
        });
        std::fs::write(neuro.join("knowledge.jsonl"), format!("{entry}\n"))
            .expect("seed the knowledge store");
        let episode = roko_learn::episode_logger::Episode::new("seed-agent", "seed/T0");
        let episode = serde_json::to_string(&episode).expect("serialize the episode");
        std::fs::write(roko.join("episodes.jsonl"), format!("{episode}\n"))
            .expect("seed the episode log");
    }

    /// A digest of each learned-state file of workspace `dir`, by its path
    /// under `.roko/`: every file under `learn/`, `neuro/` and `daimon/`,
    /// and `episodes.jsonl`.
    #[cfg(unix)]
    fn learned_state_digests(dir: &Path) -> BTreeMap<String, blake3::Hash> {
        let roko = dir.join(".roko");
        let mut pending: Vec<PathBuf> = ["learn", "neuro", "daimon", "episodes.jsonl"]
            .iter()
            .map(|name| roko.join(name))
            .collect();
        let mut digests = BTreeMap::new();
        while let Some(path) = pending.pop() {
            if let Ok(entries) = std::fs::read_dir(&path) {
                pending.extend(entries.map(|entry| entry.expect("directory entry").path()));
            } else if let Ok(bytes) = std::fs::read(&path) {
                let name = path.strip_prefix(&roko).expect("a path under .roko");
                digests.insert(name.display().to_string(), blake3::hash(&bytes));
            }
        }
        digests
    }

    /// Run [`LEARNED_STATE_TASKS`] in a workspace seeded with learned state
    /// ([`seed_learned_state`]), with learning frozen for the run when
    /// `frozen` (`--frozen-learning`). Returns the workspace, the run's exit
    /// code, and each learned-state file the run changed, added or removed
    /// that [`frozen_run_may_write`] does not allow.
    #[cfg(unix)]
    async fn run_seeded_learning_plan(frozen: bool) -> (tempfile::TempDir, i32, Vec<String>) {
        let dir = tempfile::tempdir().expect("tempdir");
        fake_provider_workspace(dir.path(), 0.0, LEARNED_STATE_CONFIG);
        let plan_dir = dir.path().join("plans/learned");
        std::fs::create_dir_all(&plan_dir).expect("plan dir");
        std::fs::write(plan_dir.join("tasks.toml"), LEARNED_STATE_TASKS).expect("tasks.toml");
        seed_learned_state(dir.path());
        let before = learned_state_digests(dir.path());

        let (exit_code, _, _) = run_plan_set_with(dir.path(), Some(1), None, true, frozen).await;
        // The writes the run left in flight end before the state is read.
        crate::background_writes::settled(&dir.path().join(".roko")).await;
        let after = learned_state_digests(dir.path());
        let paths: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
        let changed: Vec<String> = paths
            .into_iter()
            .filter(|path| !frozen_run_may_write(path))
            .filter(|path| before.get(*path) != after.get(*path))
            .cloned()
            .collect();
        (dir, exit_code, changed)
    }

    /// Decision 2218's acceptance check (gap-644040): a frozen plan run reads
    /// the learned state it finds and writes none. In a workspace seeded with
    /// learned state, a run of a task that passes, a task that fails its
    /// verify step once and passes on its retry, and a task the seeded T0
    /// reflex rule serves leaves every learned-state file as it was, and adds
    /// none but telemetry. Its run directory holds every verdict, the route
    /// decisions and a census without a wired sink, and its manifest says it
    /// was frozen. The same run under live config learns, so the check can
    /// fail.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn frozen_learning_run_writes_no_learned_state() {
        use roko_learn::telemetry::records::AttemptOutcome;
        use roko_learn::telemetry::report::RunRecords;

        let (dir, exit_code, changed) = run_seeded_learning_plan(true).await;
        assert_eq!(exit_code, EXIT_FAILURE, "T3 ends unverified");
        assert!(
            changed.is_empty(),
            "a frozen run wrote learned state: {changed:?}"
        );
        let manifest = only_run_manifest(dir.path());
        assert_eq!(manifest.experiment.ablation_flags, ["learning_frozen"]);

        let run_dir = dir.path().join(".roko/runs").join(&manifest.run_id);
        let run = RunRecords::load(&run_dir).expect("read the run's records");
        assert!(run.invalid.is_empty(), "{:?}", run.invalid);
        let settled: Vec<&str> = run
            .verdicts
            .iter()
            .map(|line| line.record.identity.task_id.as_str())
            .collect();
        assert_eq!(settled, ["T1", "T2", "T2", "T3"], "every attempt settled");
        assert_eq!(run.verdicts[3].record.outcome, AttemptOutcome::Unverified);
        // Each attempt's episode is the run's own telemetry, beside its
        // attempt log, and not learned state (gap-127263): the workspace's
        // `episodes.jsonl` holds only the seeded episode, as checked above.
        // The reflex that served T3 made no call, so it has no episode, as
        // in a live run.
        let episodes_path = run_dir.join("episodes.jsonl");
        let episodes = roko_learn::episode_logger::EpisodeLogger::read_all(&episodes_path)
            .await
            .expect("read the run's episodes");
        let mut episode_tasks: Vec<&str> = episodes
            .iter()
            .map(|episode| episode.task_id.as_str())
            .collect();
        episode_tasks.sort_unstable();
        assert_eq!(episode_tasks, ["T1", "T2", "T2"]);
        // T1 routed once and T2 twice; the reflex that served T3 routed
        // nothing, so the frozen run read its rule.
        assert_eq!(run.decisions.len(), 3, "{:?}", run.decisions);
        // The prompts read the seeded knowledge entry, whose access count
        // the frozen run left as it was.
        let exposed = run
            .exposures
            .iter()
            .any(|line| line.record.item_id == "kn-frozen" && line.record.included);
        assert!(exposed, "the prompts read the seeded knowledge entry");
        let census = run.census.as_ref().expect("the run wrote its census");
        let wired_sinks: Vec<&str> = census
            .components
            .iter()
            .filter(|component| component.kind == "sink" && component.wired)
            .map(|component| component.id.as_str())
            .collect();
        assert!(wired_sinks.is_empty(), "{wired_sinks:?}");
        let attempt_log = census.component("store.attempt_log");
        assert!(
            attempt_log.is_some_and(|component| component.wired),
            "{census:?}"
        );

        // Under live config the same run moves the gate thresholds with its
        // verify runs, and the cascade router with its routing outcomes.
        let (_live, _, changed) = run_seeded_learning_plan(false).await;
        let learned = ["learn/cascade-router.json", "learn/gate-thresholds.json"];
        assert!(
            changed.iter().any(|path| learned.contains(&path.as_str())),
            "a live run learns: {changed:?}"
        );
    }

    /// The wiring of the dispatcher a plan run builds for `config` in
    /// `workdir`, with a cascade router and its journal, as a run builds
    /// them. A frozen config leaves the write-only learning paths unset.
    async fn production_wiring(
        workdir: &Path,
        config: &roko_core::config::schema::RokoConfig,
    ) -> crate::graph_task_dispatch::WiringReport {
        use crate::graph_task_dispatch::GraphTaskDispatcher;
        use roko_learn::cascade_router::CascadeRouter;
        use roko_learn::model_call_feedback::ModelCallJournal;

        let shared = Arc::new(config.clone());
        let router = Arc::new(CascadeRouter::new(vec!["claude-sonnet-4-6".to_string()]));
        let learn_dir = workdir.join(".roko/learn");
        let journal = Arc::new(ModelCallJournal::for_learn_dir(&learn_dir));
        let factory = crate::dispatch::SharedAgentFactory::new(
            Arc::clone(&shared),
            None,
            Some(Arc::clone(&router)),
            None,
        )
        .await;
        let feedback = build_graph_feedback_context(
            workdir,
            config,
            Some(&router),
            Some(&journal),
            factory.error_pattern_store(),
            None,
            None,
        );
        let frozen = config.learning.frozen;
        assert_eq!(feedback.playbook_dir.is_none(), frozen);
        assert_eq!(feedback.experiment_store_path.is_none(), frozen);
        assert!(feedback.runs_dir.is_some());
        assert!(feedback.gate_thresholds_path.is_some());
        let workdir = workdir.to_path_buf();
        GraphTaskDispatcher::new(Arc::new(factory), shared, workdir)
            .with_feedback(feedback)
            .wiring_report()
    }

    /// Decision 2218: a frozen run's dispatcher has no learning sink, and
    /// none of the paths that only write learned state (playbook outcomes,
    /// prompt treatments). Its telemetry, the state it also reads, and its
    /// chains' arm sets and placebo rows stay. A live run has them all. The
    /// frozen facade's one sink is the run's own episode log (gap-127263).
    #[tokio::test]
    async fn frozen_run_registers_no_learning_sinks() {
        const LEARNING: [&str; 9] = [
            "sink.episode",
            "sink.routing",
            "sink.knowledge_ingestion",
            "sink.playbook_outcome",
            "sink.playbook_producer",
            "sink.error_pattern",
            "sink.section_effect",
            "sink.router_source_credit",
            "store.prompt_experiment",
        ];
        const KEPT: [&str; 7] = [
            "store.attempt_log",
            "store.decision_writer",
            "store.exposure_writer",
            "reader.gate_thresholds",
            "store.arm_set",
            "reader.withhold_arms",
            "store.placebo",
        ];
        let temp = tempfile::tempdir().expect("tempdir");
        let mut config = roko_core::config::schema::RokoConfig::default();
        let live = production_wiring(temp.path(), &config).await;
        config.learning.frozen = true;
        let frozen = production_wiring(temp.path(), &config).await;

        let wired = |report: &crate::graph_task_dispatch::WiringReport, id: &str| {
            report
                .component(id)
                .is_some_and(|component| component.wired)
        };
        for id in LEARNING {
            assert!(wired(&live, id), "a live run has {id}");
            assert!(!wired(&frozen, id), "a frozen run has {id}");
        }
        assert_eq!(frozen.facade_sinks, ["run_episodes"]);
        for id in KEPT {
            assert!(wired(&live, id) && wired(&frozen, id), "{id}");
        }
    }

    /// bug-0ba3d9: attempt records carry the invocation ordinal the run's
    /// manifest gave their process. T1 fails on the first run and passes
    /// when the run resumes, so its two attempts come from invocations 1
    /// and 2 of one run.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn attempt_records_carry_the_invocation_ordinal() {
        let dir = tempfile::tempdir().expect("tempdir");
        // No auto-fix re-run: T1's verify step must run once per attempt.
        fake_provider_workspace(dir.path(), 0.0, "cargo_fix_enabled = false\n");
        write_verify_plan(
            dir.path(),
            "resume",
            "max_parallel = 1",
            &[("T1", &[], "test -f resumed || { touch resumed; false; }")],
        );

        let (first, _, _) = run_plan_set(dir.path(), Some(1), None).await;
        assert_eq!(first, EXIT_FAILURE, "T1 fails on the first run");
        let (second, _, _) = run_plan_set(dir.path(), Some(1), None).await;
        assert_eq!(second, EXIT_SUCCESS, "the resumed run runs T1 again");

        let run_dirs: Vec<PathBuf> = std::fs::read_dir(dir.path().join(".roko/runs"))
            .expect("read .roko/runs")
            .map(|entry| entry.expect("run directory").path())
            .collect();
        assert_eq!(
            run_dirs.len(),
            1,
            "the resume continues the run: {run_dirs:?}"
        );
        let attempts = std::fs::read_to_string(run_dirs[0].join("attempts.jsonl"))
            .expect("read attempts.jsonl");
        let records: Vec<(String, u64, Option<u64>)> = attempts
            .lines()
            .map(|line| {
                let record: serde_json::Value = serde_json::from_str(line).expect("a record");
                (
                    record["schema_version"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    record["attempt"].as_u64().unwrap_or(0),
                    record["inv"].as_u64(),
                )
            })
            .collect();
        let expected = [
            ("roko.attempt_open/1", 1, Some(1)),
            ("roko.verdict/1", 1, Some(1)),
            ("roko.attempt_open/1", 2, Some(2)),
            ("roko.verdict/1", 2, Some(2)),
        ]
        .map(|(schema, attempt, inv)| (schema.to_string(), attempt, inv));
        assert_eq!(records, expected);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_stop_requested_before_the_run_starts_no_plan() {
        let dir = disabled_role_plan_set(&[("a", "a.txt", &[]), ("b", "b.txt", &[])], "");
        let interrupt = PlanRunInterruptHandle::default();
        interrupt.request(PlanRunInterrupt::Terminate);

        let (exit_code, lifecycle, hub) = run_plan_set(dir.path(), Some(2), Some(interrupt)).await;

        assert_eq!(exit_code, PlanRunInterrupt::Terminate.exit_code());
        assert!(lifecycle.is_empty(), "{lifecycle:?}");
        assert!(!hub.current_snapshot().plan_set_complete());
    }

    /// Where [`an_interrupted_attempt_settles_the_usage_it_streamed`] tells
    /// [`interrupted_plan_run_child`] to run its plan.
    #[cfg(unix)]
    const INTERRUPTED_RUN_DIR: &str = "ROKO_INTERRUPTED_RUN_CHILD_DIR";

    /// An attempt whose agent outlives the interrupt's drain is stopped and
    /// settles with the usage it streamed (bug-2b1ddc): once the interrupted
    /// run returns, the plan's cost ledger and `costs.jsonl` hold the
    /// estimate, which was lost when the runner gave up on the graph. The
    /// interrupt signals every agent its process runs, so the run happens in
    /// a child test process, away from other tests' agents.
    #[cfg(unix)]
    #[test]
    fn an_interrupted_attempt_settles_the_usage_it_streamed() {
        let dir = tempfile::tempdir().expect("tempdir");
        fake_provider_workspace(dir.path(), 0.0, "");
        // Streams one priced message, then ignores SIGTERM until killed.
        std::fs::write(
            dir.path().join("fake-provider.sh"),
            r#"#!/bin/sh
cat >/dev/null
printf '%s\n' '{"type":"assistant","message":{"id":"msg-1","model":"claude-sonnet-4-6","content":[{"type":"text","text":"working"}],"usage":{"input_tokens":1000,"output_tokens":200,"cache_creation_input_tokens":3000,"cache_read_input_tokens":4000}}}'
trap '' TERM
printf 'call\n' >> "$(dirname "$0")/provider-calls"
exec sleep 60
"#,
        )
        .expect("provider script");
        write_verify_plan(dir.path(), "interrupted", "", &[("T1", &[], "true")]);

        let output = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "--exact",
                "graph_execution::plan_runner::tests::interrupted_plan_run_child",
                "--nocapture",
            ])
            .env(INTERRUPTED_RUN_DIR, dir.path())
            .output()
            .expect("run the child test");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{stdout}\n{stderr}");
        assert!(
            stdout.contains("1 passed"),
            "the child test did not run: {stdout}"
        );

        // Sonnet per million: $3 in, $15 out, $0.30 cache read, $6 for a cache
        // write with no TTL split (1-hour, as print mode writes, bug-acfced).
        let expected = (1_000.0 * 3.0 + 200.0 * 15.0 + 4_000.0 * 0.30 + 3_000.0 * 6.0) / 1e6;
        let ledger: serde_json::Value = serde_json::from_slice(
            &std::fs::read(dir.path().join(".roko/state/graph/interrupted/costs.json"))
                .expect("the plan's cost ledger"),
        )
        .expect("ledger json");
        let spent = ledger["spent_micro_usd"].as_u64().expect("spent") as f64 / 1e6;
        assert!((spent - expected).abs() < 1e-5, "{ledger}");
        let costs = std::fs::read_to_string(dir.path().join(".roko/learn/costs.jsonl"))
            .expect("costs.jsonl");
        let row: serde_json::Value = costs
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .find(|row: &serde_json::Value| row["task_id"] == "T1")
            .expect("T1's cost row");
        assert_eq!(row["cost_source"], "estimated", "{row}");
        assert_eq!(row["outcome"], "cancelled", "{row}");
        assert_eq!(row["learning_label"], serde_json::Value::Null, "{row}");
        assert_eq!(row["input_tokens"], 1_000);
        assert_eq!(row["output_tokens"], 200);
        let cost_usd = row["cost_usd"].as_f64().expect("cost");
        assert!((cost_usd - expected).abs() < 1e-5, "{row}");
    }

    /// The interrupted run of
    /// [`an_interrupted_attempt_settles_the_usage_it_streamed`]: it runs the
    /// plan in that test's workspace once the provider is running, then
    /// returns, as the CLI would before the process exits. Without the
    /// workspace it does nothing.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn interrupted_plan_run_child() {
        let Some(workdir) = std::env::var_os(INTERRUPTED_RUN_DIR).map(PathBuf::from) else {
            return;
        };
        let interrupt = PlanRunInterruptHandle::default();
        let run = tokio::spawn({
            let workdir = workdir.clone();
            let interrupt = interrupt.clone();
            async move { run_plan_set(&workdir, Some(1), Some(interrupt)).await }
        });
        let calls = workdir.join("provider-calls");
        for _ in 0..400 {
            if calls.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert!(calls.exists(), "the provider started");
        // Let the streamed usage reach the attempt's live output.
        tokio::time::sleep(Duration::from_millis(500)).await;

        interrupt.request(PlanRunInterrupt::Interrupt);
        let (exit_code, _, _) = run.await.expect("the plan run");

        assert_eq!(exit_code, PlanRunInterrupt::Interrupt.exit_code());
    }

    /// Where [`a_cancelled_run_reports_interrupted_tasks_as_cancelled`] tells
    /// [`cancelled_run_child`] to run its plan.
    #[cfg(unix)]
    const CANCELLED_RUN_DIR: &str = "ROKO_CANCELLED_RUN_CHILD_DIR";

    /// bug-1d09e8: a run that a cancel request stops after T1 passed and
    /// while T2 ran reads as a stop, not a failure. T2 ends `cancelled` with
    /// no failure event; the run names the cancel, not a SIGINT, says where
    /// its tasks stand, and publishes no error; the checkpoint keeps T2 apart
    /// from the failed tasks. The cancel signals every agent its process
    /// runs, so the run happens in a child test process.
    #[cfg(unix)]
    #[test]
    fn a_cancelled_run_reports_interrupted_tasks_as_cancelled() {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = tempfile::tempdir().expect("tempdir");
        // T2 runs on a provider that marks the start of its agent call (one
        // with `--print`; a probe gets a version) and then works until it is
        // stopped.
        let hanging = dir.path().join("hanging-provider.sh");
        std::fs::write(
            &hanging,
            "#!/bin/sh\n\
             case \" $* \" in *\" --print \"*) ;; *) echo '2.1.0 (Claude Code)'; exit 0 ;; esac\n\
             cat >/dev/null\n\
             touch \"$(dirname \"$0\")/hanging-started\"\n\
             exec sleep 60\n",
        )
        .expect("hanging provider");
        std::fs::set_permissions(&hanging, std::fs::Permissions::from_mode(0o755))
            .expect("make the provider executable");
        fake_provider_workspace(
            dir.path(),
            0.0,
            &format!(
                "\n[providers.hanging-cli]\nkind = \"claude_cli\"\ncommand = {:?}\n\n\
                 [models.hanging-model]\nprovider = \"hanging-cli\"\nslug = \"claude-haiku-4-5\"\n\
                 context_window = 200000\n",
                hanging.display().to_string()
            ),
        );
        write_verify_plan(
            dir.path(),
            "cancelled",
            "",
            &[("T1", &[], "touch T1.verified"), ("T2", &["T1"], "true")],
        );
        let tasks = dir.path().join("plans/cancelled/tasks.toml");
        let plan = std::fs::read_to_string(&tasks).expect("tasks.toml");
        let (first, second) = plan.split_once("id = \"T2\"").expect("T2");
        std::fs::write(
            &tasks,
            format!(
                "{first}id = \"T2\"{}",
                second.replacen("\"graph-model\"", "\"hanging-model\"", 1)
            ),
        )
        .expect("route T2 to the hanging provider");

        let output = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "--exact",
                "graph_execution::plan_runner::tests::cancelled_run_child",
                "--nocapture",
            ])
            .env(CANCELLED_RUN_DIR, dir.path())
            .output()
            .expect("run the child test");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{stdout}\n{stderr}");
        assert!(
            stdout.contains("1 passed"),
            "the child test did not run: {stdout}"
        );

        let outcomes = crate::graph_checkpoint::canonical_task_outcomes(dir.path(), "cancelled")
            .expect("the checkpoint's task outcomes");
        assert_eq!(outcomes.cancelled, BTreeSet::from(["T2".to_string()]));
        assert!(outcomes.failed.is_empty(), "{outcomes:?}");
        assert_eq!(
            crate::graph_checkpoint::canonical_stop_cause(dir.path(), "cancelled").as_deref(),
            Some("cancel")
        );
    }

    /// The cancelled run of
    /// [`a_cancelled_run_reports_interrupted_tasks_as_cancelled`]: it runs the
    /// plan in that test's workspace, cancels it once T2's provider runs, and
    /// checks what the run published. Without the workspace it does nothing.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cancelled_run_child() {
        use roko_core::DashboardEvent;

        let Some(workdir) = std::env::var_os(CANCELLED_RUN_DIR).map(PathBuf::from) else {
            return;
        };
        let interrupt = PlanRunInterruptHandle::default();
        let run = tokio::spawn({
            let workdir = workdir.clone();
            let interrupt = interrupt.clone();
            async move { run_plan_set(&workdir, Some(1), Some(interrupt)).await }
        });
        let started = workdir.join("hanging-started");
        for _ in 0..800 {
            if started.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert!(started.exists(), "T2's provider started");

        interrupt.request(PlanRunInterrupt::Cancel);
        let (exit_code, _, hub) = run.await.expect("the plan run");
        assert_eq!(exit_code, PlanRunInterrupt::Cancel.exit_code());

        let events = hub
            .subscribe_events_from(0)
            .replay
            .into_iter()
            .map(|envelope| envelope.payload)
            .collect::<Vec<_>>();
        let outcomes = |task: &str| {
            events
                .iter()
                .filter_map(|event| match event {
                    DashboardEvent::TaskCompleted {
                        task_id, outcome, ..
                    } if task_id == task => Some(outcome.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(outcomes("T1"), ["passed"]);
        assert_eq!(outcomes("T2"), ["cancelled"]);
        // A cancel raises no error, so the run's last row is not a failure:
        // its closing tally counts the stopped plan as a notice (bug-db89d4).
        let errors = events
            .iter()
            .filter_map(|event| match event {
                DashboardEvent::Error { message } => Some(message.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(errors.is_empty(), "a cancel is not a failure: {errors:?}");
        let notes = events
            .iter()
            .filter_map(|event| match event {
                DashboardEvent::EventLogEntry { message, .. } => Some(message.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        for note in [
            "a cancel request is stopping the run: cancelling plan 'cancelled'",
            "plan 'cancelled' stopped by a cancel request: 1 task(s) passed, 1 cancelled; a \
             resume keeps the passed tasks and runs the rest",
            "a cancel request stopped the run",
        ] {
            assert!(notes.contains(&note), "no {note:?} in {notes:#?}");
        }
        assert!(
            notes.iter().any(|note| note
                .starts_with("plan 'cancelled': task 'T2' cancelled: agent for cancelled/T2")),
            "{notes:#?}"
        );
        assert!(
            !notes.iter().any(|note| note.contains("SIGINT")),
            "{notes:#?}"
        );
    }

    /// Where [`interrupt_stops_running_gate_command`] tells
    /// [`interrupted_gate_run_child`] to run its plan.
    #[cfg(unix)]
    const INTERRUPTED_GATE_DIR: &str = "ROKO_INTERRUPTED_GATE_CHILD_DIR";

    /// A verify command that runs until it is signalled, marking when it
    /// starts and when it gets SIGTERM.
    #[cfg(unix)]
    const SIGNALLED_GATE: &str =
        "trap 'echo > got-term; exit 143' TERM; echo > gate-started; sleep 60 & wait $!";

    /// gap-b367bf: an interrupt stops a task's running verify command the way
    /// it stops the run's agents, with SIGTERM, which the command here traps
    /// into a marker file. The interrupt signals every registered process of
    /// its process, so the run happens in a child test process.
    #[cfg(unix)]
    #[test]
    fn interrupt_stops_running_gate_command() {
        let dir = tempfile::tempdir().expect("tempdir");
        fake_provider_workspace(dir.path(), 0.0, "");
        write_verify_plan(dir.path(), "gated", "", &[("T1", &[], SIGNALLED_GATE)]);

        let output = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "--exact",
                "graph_execution::plan_runner::tests::interrupted_gate_run_child",
                "--nocapture",
            ])
            .env(INTERRUPTED_GATE_DIR, dir.path())
            .output()
            .expect("run the child test");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{stdout}\n{stderr}");
        assert!(
            stdout.contains("1 passed"),
            "the child test did not run: {stdout}"
        );
        assert!(
            dir.path().join("got-term").exists(),
            "the interrupt never sent the verify command SIGTERM"
        );
    }

    /// The interrupted run of [`interrupt_stops_running_gate_command`]: it
    /// interrupts the plan in that test's workspace once the task's verify
    /// command runs. Without the workspace it does nothing.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn interrupted_gate_run_child() {
        let Some(workdir) = std::env::var_os(INTERRUPTED_GATE_DIR).map(PathBuf::from) else {
            return;
        };
        let interrupt = PlanRunInterruptHandle::default();
        let run = tokio::spawn({
            let workdir = workdir.clone();
            let interrupt = interrupt.clone();
            async move { run_plan_set(&workdir, Some(1), Some(interrupt)).await }
        });
        let started = workdir.join("gate-started");
        for _ in 0..400 {
            if started.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert!(started.exists(), "the verify command started");

        interrupt.request(PlanRunInterrupt::Interrupt);
        let (exit_code, _, _) = run.await.expect("the plan run");

        assert_eq!(exit_code, PlanRunInterrupt::Interrupt.exit_code());
    }

    /// One run of [`a_runs_cancel_stops_its_own_gate_command_only`]: a thread
    /// in a spawn scope of its own, as `roko serve` runs a plan, that runs
    /// [`SIGNALLED_GATE`] in `dir` and cancels its run when told to, or when
    /// the test drops its sender. Joining it gives the number of processes
    /// that cancel signalled.
    #[cfg(unix)]
    fn scoped_gate_run(
        dir: &Path,
    ) -> (
        tokio::sync::oneshot::Sender<()>,
        std::thread::JoinHandle<usize>,
    ) {
        use roko_core::Verify as _;

        std::fs::create_dir_all(dir).expect("run dir");
        let payload = roko_gate::GatePayload::in_dir(dir);
        let (cancel, cancelled) = tokio::sync::oneshot::channel::<()>();
        let run = std::thread::spawn(move || {
            let _scope =
                roko_agent::process::enter_spawn_scope(roko_agent::process::new_spawn_scope());
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("run runtime");
            runtime.block_on(async move {
                let args = vec!["-c".to_string(), SIGNALLED_GATE.to_string()];
                let gate = roko_gate::ShellGate::new("bash", args).with_timeout_ms(120_000);
                let signal = roko_core::Signal::builder(roko_core::Kind::Task)
                    .body(roko_core::Body::from_json(&payload).expect("gate payload"))
                    .build();
                let gate_ctx = roko_core::Context::now();
                let stop = async {
                    let _ = cancelled.await;
                    terminate_in_flight_agents()
                };
                let (_, signalled) = tokio::join!(gate.verify(&signal, &gate_ctx), stop);
                signalled
            })
        });
        (cancel, run)
    }

    /// q-9852b5: a gate command joins the agent PID registry in the spawn
    /// scope of the run that starts it (gap-b367bf, find-65ff6b), so a run's
    /// cancel stops its own running gate command and leaves another run's
    /// alone.
    #[cfg(unix)]
    #[test]
    fn a_runs_cancel_stops_its_own_gate_command_only() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (own, other) = (dir.path().join("own"), dir.path().join("other"));
        let (cancel_own, own_run) = scoped_gate_run(&own);
        let (cancel_other, other_run) = scoped_gate_run(&other);
        let started = |run: &Path| {
            for _ in 0..1_200 {
                if run.join("gate-started").exists() {
                    return true;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            false
        };
        assert!(started(&own) && started(&other), "both gate commands run");

        cancel_own.send(()).expect("cancel the run");
        let signalled = own_run.join().expect("the cancelled run");
        assert!(signalled > 0, "the cancel signalled its gate command");
        assert!(own.join("got-term").exists(), "it got SIGTERM");
        // A SIGTERM the cancel sent the other command would have landed by now.
        std::thread::sleep(Duration::from_millis(100));
        assert!(
            !other.join("got-term").exists(),
            "another run's gate command runs on"
        );

        cancel_other.send(()).expect("cancel the other run");
        assert!(other_run.join().expect("the other run") > 0);
        assert!(other.join("got-term").exists());
    }

    #[test]
    fn interrupt_exit_codes_follow_shell_convention() {
        assert_eq!(PlanRunInterrupt::Interrupt.exit_code(), 130);
        assert_eq!(PlanRunInterrupt::Terminate.exit_code(), 143);
        assert_eq!(PlanRunInterrupt::Hangup.exit_code(), 129);
        // A FAST deadline exits as SIGTERM does, under its own label.
        assert_eq!(PlanRunInterrupt::Deadline.exit_code(), 143);
        assert_eq!(PlanRunInterrupt::Deadline.label(), "deadline");
        assert_eq!(PlanRunInterrupt::Conductor.label(), "conductor");
        // bug-1d09e8: a cancel request and a closed TUI exit as SIGINT does,
        // each named for what it was.
        for (interrupt, label, cause) in [
            (PlanRunInterrupt::Cancel, "cancel", "a cancel request"),
            (PlanRunInterrupt::TuiClosed, "tui-closed", "closing the TUI"),
        ] {
            assert_eq!(interrupt.exit_code(), 130);
            assert_eq!(interrupt.label(), label);
            assert_eq!(interrupt.cause(), cause);
        }
        for interrupt in PlanRunInterrupt::ALL {
            assert_eq!(
                PlanRunInterrupt::from_code(interrupt.code()),
                Some(interrupt)
            );
        }
    }

    /// bug-4641e3: a forced exit marks the checkpoints of the plans still
    /// running `interrupted` and leaves finalized ones alone; a plan's
    /// checkpoint is listed for it only while the plan runs.
    #[test]
    fn a_forced_exit_marks_running_checkpoints_interrupted() {
        use crate::graph_checkpoint::{
            canonical_checkpoint_status, canonical_stop_cause, start_plan_checkpoint,
        };

        let dir = tempfile::tempdir().expect("tempdir");
        let running = start_plan_checkpoint(dir.path(), &test_plan("running", "Running", 1))
            .expect("running checkpoint");
        let mut finished = start_plan_checkpoint(dir.path(), &test_plan("finished", "Done", 1))
            .expect("finished checkpoint");
        finished
            .finish_with_status(GraphCheckpointStatus::Succeeded)
            .expect("finish");
        let manifest = running.paths().manifest.clone();
        {
            let _running = RunningPlanCheckpoint::register(&manifest);
            assert!(running_plan_checkpoints().contains(&manifest));
        }
        assert!(!running_plan_checkpoints().contains(&manifest));

        let manifests = vec![manifest, finished.paths().manifest.clone()];
        assert_eq!(
            mark_checkpoints_interrupted(manifests, "SIGTERM", Duration::from_secs(5)),
            1
        );
        assert_eq!(
            canonical_checkpoint_status(dir.path(), "running"),
            Some(GraphCheckpointStatus::Interrupted)
        );
        assert_eq!(
            canonical_stop_cause(dir.path(), "running").as_deref(),
            Some("SIGTERM")
        );
        assert_eq!(
            canonical_checkpoint_status(dir.path(), "finished"),
            Some(GraphCheckpointStatus::Succeeded)
        );
        assert_eq!(canonical_stop_cause(dir.path(), "finished"), None);
        assert_eq!(
            mark_checkpoints_interrupted(Vec::new(), "SIGTERM", Duration::ZERO),
            0
        );
    }

    /// gap-fab2cc: a plan's checkpoint names the stop request that
    /// interrupted it, and nothing when the plan ended any other way.
    #[test]
    fn an_interrupted_plan_names_its_stop() {
        let interrupted = |by| stop_cause(PlanOutcome::Interrupted, by);
        let deadline = Some(PlanRunInterrupt::Deadline);
        let conductor = Some(PlanRunInterrupt::Conductor);
        assert_eq!(interrupted(deadline), Some("deadline"));
        assert_eq!(interrupted(conductor), Some("conductor"));
        assert_eq!(interrupted(None), None);
        assert_eq!(stop_cause(PlanOutcome::Succeeded, deadline), None);
    }

    /// Set by [`a_hangup_stops_the_plan_run`] for its child test process.
    #[cfg(unix)]
    const HANGUP_CHILD: &str = "ROKO_PLAN_RUN_HANGUP_CHILD";

    /// bug-4641e3: SIGHUP reaches the plan run's stop handle, as SIGTERM
    /// does, so a hung-up terminal stops the run and its checkpoint is
    /// finalized; under `nohup` the run leaves SIGHUP ignored. The signals go
    /// to a child test process, away from the other tests.
    #[cfg(unix)]
    #[test]
    fn a_hangup_stops_the_plan_run() {
        let output = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "--exact",
                "graph_execution::plan_runner::tests::hangup_plan_run_child",
                "--nocapture",
            ])
            .env(HANGUP_CHILD, "1")
            .output()
            .expect("run the child test");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{stdout}\n{stderr}");
        assert!(
            stdout.contains("1 passed"),
            "the child test did not run: {stdout}"
        );
    }

    /// The child of [`a_hangup_stops_the_plan_run`]: with the run's signal
    /// handlers installed, a SIGHUP is recorded as a hangup stop request.
    /// Outside that test it does nothing.
    #[cfg(unix)]
    #[tokio::test]
    async fn hangup_plan_run_child() {
        if std::env::var_os(HANGUP_CHILD).is_none() {
            return;
        }
        // SAFETY: signal(2) only sets this child test process's disposition.
        #[allow(unsafe_code)]
        unsafe {
            let _ = libc::signal(libc::SIGHUP, libc::SIG_IGN);
        }
        assert!(hangup_ignored(), "as under nohup");
        // SAFETY: as above.
        #[allow(unsafe_code)]
        unsafe {
            let _ = libc::signal(libc::SIGHUP, libc::SIG_DFL);
        }
        assert!(!hangup_ignored());

        let interrupt = PlanRunInterruptHandle::default();
        let _signals = install_plan_run_signal_handlers(interrupt.clone()).expect("handlers");
        assert!(plan_run_owns_hangup_signal());
        // SAFETY: raise(3) signals this child test process, which handles it.
        #[allow(unsafe_code)]
        unsafe {
            let _ = libc::raise(libc::SIGHUP);
        }
        for _ in 0..200 {
            if interrupt.requested().is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(interrupt.requested(), Some(PlanRunInterrupt::Hangup));
    }

    #[test]
    fn first_interrupt_request_wins() {
        let interrupt = PlanRunInterruptHandle::default();
        assert_eq!(interrupt.requested(), None);
        assert!(interrupt.request(PlanRunInterrupt::Terminate));
        assert!(!interrupt.request(PlanRunInterrupt::Interrupt));
        assert_eq!(
            interrupt.clone().requested(),
            Some(PlanRunInterrupt::Terminate)
        );
    }

    #[test]
    fn checkpoint_status_is_honest_about_how_a_plan_ended() {
        let passed = TaskVerdictCounts {
            passed: 2,
            ..TaskVerdictCounts::default()
        };
        let unverified = TaskVerdictCounts {
            unverified: 1,
            ..passed
        };
        let status = |ran, tasks, interrupted, cancelled| {
            plan_checkpoint_status(plan_outcome(ran, tasks, interrupted, cancelled))
        };

        assert_eq!(
            status(true, passed, true, false),
            GraphCheckpointStatus::Succeeded
        );
        assert_eq!(
            status(true, unverified, false, false),
            GraphCheckpointStatus::Unverified
        );
        assert_eq!(
            status(false, passed, true, true),
            GraphCheckpointStatus::Interrupted
        );
        assert_eq!(
            status(false, passed, false, true),
            GraphCheckpointStatus::Cancelled
        );
        assert_eq!(
            status(false, passed, false, false),
            GraphCheckpointStatus::Failed
        );
    }

    /// gap-29a84b: a plan whose graph ran to completion succeeds only when
    /// every task passed its verify steps.
    #[test]
    fn plan_outcome_follows_task_verdicts() {
        let counts = |passed, unverified, skipped, failed| TaskVerdictCounts {
            passed,
            already_satisfied: 0,
            unverified,
            skipped,
            failed,
            cancelled: 0,
        };

        assert_eq!(counts(3, 0, 0, 0).outcome(), PlanOutcome::Succeeded);
        assert_eq!(counts(2, 1, 0, 0).outcome(), PlanOutcome::Unverified);
        assert_eq!(counts(0, 1, 0, 0).outcome(), PlanOutcome::Unverified);
        assert_eq!(counts(2, 1, 1, 0).outcome(), PlanOutcome::Failed);
        assert_eq!(counts(2, 1, 0, 1).outcome(), PlanOutcome::Failed);

        // gap-9eb1e1: a task whose work was already there was verified.
        let rerun = TaskVerdictCounts {
            already_satisfied: 2,
            ..counts(1, 0, 0, 0)
        };
        assert_eq!(rerun.outcome(), PlanOutcome::Succeeded);

        // bug-1d09e8: a task the operator stopped cancels the plan, unless
        // another failed.
        let stopped = TaskVerdictCounts {
            cancelled: 1,
            ..counts(2, 0, 0, 0)
        };
        assert_eq!(stopped.outcome(), PlanOutcome::Cancelled);
        let stopped_and_failed = TaskVerdictCounts {
            cancelled: 1,
            ..counts(2, 0, 0, 1)
        };
        assert_eq!(stopped_and_failed.outcome(), PlanOutcome::Failed);
    }

    /// bug-103bce: each task that failed by its own error says why in the
    /// run's events, clipped; blocked tasks, failures without an error and
    /// helper nodes do not.
    #[test]
    fn failed_tasks_say_why_in_the_runs_events() {
        use roko_graph::engine::NodeStatus;

        let node = |id: &str, status: NodeStatus, error: Option<&str>| roko_graph::NodeResult {
            node_id: id.to_string(),
            cell_type: TASK_EXECUTOR_CELL_TYPE.to_string(),
            status,
            duration: Duration::ZERO,
            error: error.map(str::to_string),
            output_count: 0,
            is_stub: false,
            blocked_by: None,
            timing: roko_graph::NodeTiming::default(),
        };
        let provider = "no usable provider for this task: `claude-sonnet-4-6` on `claude_cli`: \
                        provider `claude_cli` (claude_cli) runs its own tools, so it cannot \
                        honour the task's `stage` policy";
        let long = "x".repeat(MAX_TASK_ERROR_CHARS + 500);
        let output = roko_graph::GraphOutput {
            graph_name: "reasons".to_string(),
            success: false,
            node_results: vec![
                node("T1", NodeStatus::Failed, Some(provider)),
                roko_graph::NodeResult {
                    blocked_by: Some("T1".to_string()),
                    ..node("T2", NodeStatus::Failed, Some("blocked by T1"))
                },
                node("T3", NodeStatus::Complete, None),
                node("T4", NodeStatus::Failed, None),
                node("T5", NodeStatus::Failed, Some(&long)),
                // A helper node of the rich topology is not a task.
                node("task.T1.gate", NodeStatus::Failed, Some("gate failed")),
            ],
            total_duration: Duration::ZERO,
            gate_verdicts: BTreeMap::new(),
        };
        let node_titles = ["T1", "T2", "T3", "T4", "T5"]
            .into_iter()
            .map(|id| (id.to_string(), format!("task {id}")))
            .collect::<HashMap<_, _>>();

        let messages = task_failure_errors("p1", &output, &node_titles);

        assert_eq!(messages.len(), 2, "{messages:?}");
        assert_eq!(
            messages[0],
            format!("plan 'p1': task 'T1' failed: {provider}")
        );
        let clipped = format!(
            "plan 'p1': task 'T5' failed: {}…",
            "x".repeat(MAX_TASK_ERROR_CHARS)
        );
        assert_eq!(messages[1], clipped);
    }

    /// bug-1d09e8: while a run stops, the poll holds a newly failed node back
    /// until its error says whether the stop cut it off, and reports every
    /// other change.
    #[test]
    fn a_stopping_run_holds_back_failures_from_the_poll() {
        use roko_graph::engine::NodeStatus;

        let statuses = |entries: &[(&str, NodeStatus)]| {
            entries
                .iter()
                .map(|(id, status)| ((*id).to_string(), *status))
                .collect::<HashMap<_, _>>()
        };
        let previous = statuses(&[
            ("T1", NodeStatus::Running),
            ("T2", NodeStatus::Running),
            ("T4", NodeStatus::Failed),
        ]);
        let mut current = statuses(&[
            ("T1", NodeStatus::Failed),
            ("T2", NodeStatus::Complete),
            ("T3", NodeStatus::Failed),
            ("T4", NodeStatus::Failed),
        ]);
        hold_back_failures(&previous, &mut current);
        assert_eq!(
            current,
            statuses(&[
                ("T1", NodeStatus::Running),
                ("T2", NodeStatus::Complete),
                ("T4", NodeStatus::Failed),
            ])
        );
    }

    /// bug-1d09e8: a task a stop cut off, whose node failed with a
    /// cancellation, is cancelled: no failure event names it, the checkpoint
    /// and the counts keep it apart from the failed tasks, and its reason is
    /// kept.
    #[test]
    fn a_task_a_stop_cut_off_is_cancelled_not_failed() {
        use roko_graph::cells::task_executor::TaskGateVerdict;
        use roko_graph::engine::NodeStatus;

        let node = |id: &str, status: NodeStatus, error: Option<&str>| roko_graph::NodeResult {
            node_id: id.to_string(),
            cell_type: TASK_EXECUTOR_CELL_TYPE.to_string(),
            status,
            duration: Duration::ZERO,
            error: error.map(str::to_string),
            output_count: 0,
            is_stub: false,
            blocked_by: None,
            timing: roko_graph::NodeTiming::default(),
        };
        let stopped =
            roko_core::RokoError::cancelled("agent for p1/T2 stopped: its plan run is stopping");
        let output = roko_graph::GraphOutput {
            graph_name: "stopped".to_string(),
            success: false,
            node_results: vec![
                node("T1", NodeStatus::Complete, None),
                node("T2", NodeStatus::Failed, Some(&stopped.to_string())),
                node(
                    "T3",
                    NodeStatus::Failed,
                    Some("gate rejected: tests failed"),
                ),
            ],
            total_duration: Duration::ZERO,
            gate_verdicts: BTreeMap::from([("T1".to_string(), TaskGateVerdict::Passed)]),
        };
        let node_titles = ["T1", "T2", "T3"]
            .into_iter()
            .map(|id| (id.to_string(), format!("task {id}")))
            .collect::<HashMap<_, _>>();

        assert_eq!(
            task_failure_errors("p1", &output, &node_titles),
            ["plan 'p1': task 'T3' failed: gate rejected: tests failed"]
        );
        assert_eq!(
            cancelled_tasks(&output, &node_titles),
            BTreeMap::from([(
                "T2".to_string(),
                "agent for p1/T2 stopped: its plan run is stopping"
            )])
        );
        let outcomes = task_outcomes(&output, &node_titles);
        assert_eq!(outcomes.cancelled, BTreeSet::from(["T2".to_string()]));
        assert_eq!(outcomes.failed, BTreeSet::from(["T3".to_string()]));
        let counts = TaskVerdictCounts::of(&output);
        assert_eq!((counts.passed, counts.cancelled, counts.failed), (1, 1, 1));
    }

    /// bug-7eb27e: run metrics count each task under its own verdict, not
    /// under its plan's outcome.
    #[test]
    fn run_metrics_count_task_verdicts() {
        use roko_graph::cells::task_executor::TaskGateVerdict;
        use roko_graph::engine::NodeStatus;

        let node = |id: &str, cell_type: &str, status: NodeStatus| roko_graph::NodeResult {
            node_id: id.to_string(),
            cell_type: cell_type.to_string(),
            status,
            duration: Duration::ZERO,
            error: None,
            output_count: 0,
            is_stub: false,
            blocked_by: None,
            timing: roko_graph::NodeTiming::default(),
        };
        let output = roko_graph::GraphOutput {
            graph_name: "verdicts".to_string(),
            success: false,
            node_results: vec![
                node("T1", TASK_EXECUTOR_CELL_TYPE, NodeStatus::Complete),
                node("T2", TASK_EXECUTOR_CELL_TYPE, NodeStatus::Complete),
                node("T3", TASK_EXECUTOR_CELL_TYPE, NodeStatus::Failed),
                roko_graph::NodeResult {
                    blocked_by: Some("T3".to_string()),
                    ..node("T4", TASK_EXECUTOR_CELL_TYPE, NodeStatus::Skipped)
                },
                node("T5", TASK_EXECUTOR_CELL_TYPE, NodeStatus::Complete),
                // A helper node of the rich topology is not a task.
                node("task.T1.gate", "passthrough", NodeStatus::Complete),
            ],
            total_duration: Duration::ZERO,
            gate_verdicts: BTreeMap::from([
                ("T1".to_string(), TaskGateVerdict::Passed),
                ("T2".to_string(), TaskGateVerdict::Unverified),
                ("T5".to_string(), TaskGateVerdict::AlreadySatisfied),
                ("task.T1.gate".to_string(), TaskGateVerdict::Passed),
            ]),
        };

        let metrics = plan_metrics("verdicts", false, TaskVerdictCounts::of(&output));

        assert!(!metrics.completed);
        let counts = [
            metrics.tasks_completed,
            metrics.tasks_already_satisfied,
            metrics.tasks_unverified,
            metrics.tasks_skipped,
            metrics.tasks_failed,
        ];
        assert_eq!(
            counts,
            [1, 1, 1, 1, 1],
            "passed, already satisfied, unverified, skipped, failed"
        );
    }

    fn wait_until_finished(session: &TuiSession) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !session
            .handle
            .as_ref()
            .is_some_and(std::thread::JoinHandle::is_finished)
        {
            assert!(Instant::now() < deadline, "test TUI thread did not finish");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn closing_the_tui_mid_run_interrupts_the_run() {
        let (shutdown, _shutdown_rx) = std::sync::mpsc::channel();
        let mut tui = Some(TuiSession {
            shutdown: Some(shutdown),
            handle: Some(std::thread::spawn(|| Ok(()))),
        });
        wait_until_finished(tui.as_ref().expect("session"));
        let interrupt = PlanRunInterruptHandle::default();

        assert_eq!(
            pending_interrupt(&interrupt, &mut tui),
            Some(PlanRunInterrupt::TuiClosed)
        );
        assert_eq!(interrupt.requested(), Some(PlanRunInterrupt::TuiClosed));
    }

    #[test]
    fn failed_tui_does_not_interrupt_the_run() {
        let (shutdown, _shutdown_rx) = std::sync::mpsc::channel();
        let mut tui = Some(TuiSession {
            shutdown: Some(shutdown),
            handle: Some(std::thread::spawn(|| Err(anyhow!("no terminal")))),
        });
        wait_until_finished(tui.as_ref().expect("session"));
        let interrupt = PlanRunInterruptHandle::default();

        assert_eq!(pending_interrupt(&interrupt, &mut tui), None);
    }

    #[test]
    fn dropping_a_tui_session_stops_and_joins_the_tui() {
        let (shutdown, shutdown_rx) = std::sync::mpsc::channel::<()>();
        let (stopped_tx, stopped_rx) = std::sync::mpsc::channel();
        let session = TuiSession {
            shutdown: Some(shutdown),
            handle: Some(std::thread::spawn(move || {
                // Like the TUI loop: exit on the signal or a closed channel.
                let _ = shutdown_rx.recv();
                stopped_tx.send(()).expect("report stop");
                Ok(())
            })),
        };

        drop(session);

        stopped_rx
            .try_recv()
            .expect("TUI thread joined before drop returned");
    }

    /// What [`route_tui_commands`] saw.
    struct RoutedTui {
        routed: RoutedCommands,
        acks: Vec<crate::execution_control::CommandAck>,
        /// Whether `01-run` was asked to cancel.
        run_cancelled: bool,
        /// What inject commands queued.
        directives: crate::graph_task_dispatch::OperatorDirectives,
    }

    /// Route `commands`, each a kind, the plan it names and the task, through
    /// [`route_execution_commands`] while plan `01-run` runs, `02-wait` waits
    /// for it, and `03-failed` has failed. `task_stops` holds the agents
    /// that run, and no task is held for review.
    fn route_tui_commands(
        commands: Vec<(ExecutionCommandKind, Option<&str>, Option<&str>)>,
        task_stops: &crate::graph_task_dispatch::OperatorStops,
    ) -> RoutedTui {
        let workdir = tempfile::tempdir().expect("workdir");
        route_tui_commands_in(workdir.path(), commands, task_stops)
    }

    /// [`route_tui_commands`] for a run in `workdir`, whose review holds
    /// approve and reject commands can name.
    fn route_tui_commands_in(
        workdir: &Path,
        commands: Vec<(ExecutionCommandKind, Option<&str>, Option<&str>)>,
        task_stops: &crate::graph_task_dispatch::OperatorStops,
    ) -> RoutedTui {
        let budget = crate::graph_task_dispatch::PlanBudgetControl::default();
        route_tui_commands_with(workdir, commands, task_stops, &budget)
    }

    /// [`route_tui_commands_in`], with `budget` the plans' budget ledger.
    fn route_tui_commands_with(
        workdir: &Path,
        commands: Vec<(ExecutionCommandKind, Option<&str>, Option<&str>)>,
        task_stops: &crate::graph_task_dispatch::OperatorStops,
        budget: &crate::graph_task_dispatch::PlanBudgetControl,
    ) -> RoutedTui {
        let (sender, mut receiver, ack_tx, ack_rx) = ExecutionCommandSender::channel("graph");
        for (kind, plan_id, task_id) in commands {
            let command = sender.build_command(
                kind,
                plan_id.map(str::to_string),
                task_id.map(str::to_string),
                None,
            );
            sender.try_send(command).expect("queue the command");
        }
        let order = PlanSetOrder {
            order: vec![
                "01-run".to_string(),
                "02-wait".to_string(),
                "03-failed".to_string(),
            ],
            ..PlanSetOrder::default()
        };
        let mut conflicts = PlanConflicts::new();
        for (plan, other) in [("01-run", "02-wait"), ("02-wait", "01-run")] {
            conflicts
                .entry(plan.to_string())
                .or_default()
                .insert(other.to_string(), "shared tree".to_string());
        }
        let mut scheduler = PlanSetScheduler::new(&order, conflicts, 2, false);
        assert_eq!(scheduler.admit().start, ["01-run", "03-failed"]);
        scheduler.finish("03-failed", PlanOutcome::Failed);
        let running = PlanControl::default();
        let controls = HashMap::from([("01-run".to_string(), running.clone())]);
        let pause = AtomicBool::new(false);
        let directives = crate::graph_task_dispatch::OperatorDirectives::default();
        let routed = route_execution_commands(
            &mut receiver,
            &ack_tx,
            &controls,
            &mut scheduler,
            &pause,
            task_stops,
            &directives,
            budget,
            workdir,
        );
        RoutedTui {
            routed,
            acks: CommandAckReceiver::new(ack_rx).drain(),
            run_cancelled: running.cancel.load(Ordering::Acquire),
            directives,
        }
    }

    /// Run `f` under a subscriber that records every log line, at every
    /// level; return what was logged, and what `f` returned.
    fn captured_logs<T>(f: impl FnOnce() -> T) -> (String, T) {
        #[derive(Clone, Default)]
        struct Buffer(Arc<parking_lot::Mutex<Vec<u8>>>);

        impl std::io::Write for Buffer {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0.lock().extend_from_slice(bytes);
                Ok(bytes.len())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Buffer {
            type Writer = Self;

            fn make_writer(&'a self) -> Self::Writer {
                self.clone()
            }
        }

        let buffer = Buffer::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buffer.clone())
            .with_max_level(tracing::Level::TRACE)
            .with_ansi(false)
            .finish();
        let result = tracing::subscriber::with_default(subscriber, f);
        let logs = String::from_utf8_lossy(&buffer.0.lock()).into_owned();
        (logs, result)
    }

    /// gap-f118b3: an inject command queues its text for the next task of
    /// the running plan it names, acknowledged as accepted; one for a plan
    /// that is not running is rejected; no log line, and neither `Debug` nor
    /// `Display` of the command, shows the text.
    #[test]
    fn inject_commands_queue_for_the_running_plan_and_never_log_their_text() {
        let inject = |plan_id: &'static str| {
            let kind = ExecutionCommandKind::Inject {
                kind: crate::execution_control::InjectedKind::Directive,
                text: crate::execution_control::InjectedText::new("SECRET-71d0: ship it"),
            };
            (kind, Some(plan_id), None::<&str>)
        };

        let (logs, seen) = captured_logs(|| {
            route_tui_commands(
                vec![inject("01-run"), inject("02-wait")],
                &crate::graph_task_dispatch::OperatorStops::default(),
            )
        });

        let statuses: Vec<_> = seen.acks.iter().map(|ack| ack.status).collect();
        assert_eq!(
            statuses,
            [CommandAckStatus::Accepted, CommandAckStatus::Rejected]
        );
        let section = seen.directives.take_section("01-run").expect("queued");
        assert_eq!(section.matches("SECRET-71d0").count(), 1, "{section}");
        assert!(seen.directives.take_section("02-wait").is_none());
        assert!(!logs.contains("SECRET-71d0"), "{logs}");
        let (kind, _, _) = inject("01-run");
        let shown = format!("{kind:?} {kind}");
        assert!(!shown.contains("SECRET-71d0"), "{shown}");
    }

    /// backlog 2118: a budget raise lifts the ceiling of the running plan it
    /// names and is acknowledged with what changed, for the driver to record.
    /// One for a plan that is not running, one naming no plan, and one that
    /// does not raise the ceiling are rejected with the reason.
    #[test]
    fn a_budget_raise_reaches_only_a_running_plan() {
        let raise = |plan_id: Option<&'static str>, ceiling_micro_usd: u64| {
            let kind = ExecutionCommandKind::RaiseBudget {
                ceiling_micro_usd,
                requested_by: "the test".to_string(),
            };
            (kind, plan_id, None::<&str>)
        };
        let budget =
            crate::graph_task_dispatch::PlanBudgetControl::spent_for_test("01-run", 0.05, 0.05);
        let workdir = tempfile::tempdir().expect("workdir");

        let seen = route_tui_commands_with(
            workdir.path(),
            vec![
                raise(Some("02-wait"), 100_000),
                raise(None, 100_000),
                raise(Some("01-run"), 50_000),
                raise(Some("01-run"), 100_000),
            ],
            &crate::graph_task_dispatch::OperatorStops::default(),
            &budget,
        );

        let answers: Vec<_> = seen
            .acks
            .iter()
            .map(|ack| (ack.status, ack.message.clone().unwrap_or_default()))
            .collect();
        assert_eq!(answers[0].0, CommandAckStatus::Rejected);
        assert!(answers[0].1.contains("is not running"), "{answers:?}");
        assert_eq!(answers[1].0, CommandAckStatus::Rejected);
        assert_eq!(answers[2].0, CommandAckStatus::Rejected);
        assert!(answers[2].1.contains("does not raise"), "{answers:?}");
        assert_eq!(
            answers[3],
            (
                CommandAckStatus::Completed,
                "plan '01-run' budget ceiling raised from $0.0500 to $0.1000 by the test, with \
                 $0.0500 spent"
                    .to_string()
            )
        );
        assert_eq!(seen.routed.budget_raises.len(), 1);
    }

    /// gap-c002bb: a Graph run rejects, with its reason, every TUI command
    /// it cannot carry out, and never accepts one and drops it: gate
    /// re-verification, approvals naming no held task, a cancel naming a
    /// plan that is neither running nor waiting, a retry of a plan that is
    /// running, waiting or unnamed, and a skip of a task with no running
    /// agent.
    #[test]
    fn unsupported_tui_commands_are_rejected_with_a_reason() {
        let commands = vec![
            (ExecutionCommandKind::ReverifyGates, Some("01-run"), None),
            (
                ExecutionCommandKind::Approve {
                    approval_id: "ap-1".to_string(),
                },
                Some("01-run"),
                None,
            ),
            (
                ExecutionCommandKind::RejectApproval {
                    approval_id: "ap-1".to_string(),
                    reason: "not now".to_string(),
                },
                Some("01-run"),
                None,
            ),
            (ExecutionCommandKind::Cancel, Some("09-gone"), None),
            (ExecutionCommandKind::SoftRetry, Some("01-run"), None),
            (ExecutionCommandKind::Reset, Some("02-wait"), None),
            (
                ExecutionCommandKind::Repair {
                    preserve_completed: true,
                },
                None,
                None,
            ),
            (ExecutionCommandKind::Skip, Some("01-run"), Some("T9")),
            (ExecutionCommandKind::Skip, Some("01-run"), None),
        ];
        let sent = commands.len();
        let task_stops = crate::graph_task_dispatch::OperatorStops::default();
        let agent = task_stops.register("01-run", "T1");

        let seen = route_tui_commands(commands, &task_stops);

        assert!(seen.routed.cancelled_before_start.is_empty());
        assert!(seen.routed.reruns.is_empty());
        assert!(!seen.run_cancelled);
        assert!(!agent.is_stopped());
        assert_eq!(seen.acks.len(), sent);
        for ack in &seen.acks {
            assert_eq!(ack.status, CommandAckStatus::Rejected, "{ack:?}");
            assert!(
                !ack.message.as_deref().unwrap_or_default().is_empty(),
                "{ack:?}"
            );
        }
    }

    /// 1218: a Graph run takes a reviewer's decision on a held task from its
    /// own control channel, as `roko plan review` records it. Approve and
    /// RejectApproval name the task as `<plan>/<task>`; the decision goes to
    /// the review log with the hold's attempt key, a rejection with its
    /// reason, and an id naming no held task is rejected.
    #[test]
    fn approve_command_records_the_held_tasks_review() {
        let workdir = tempfile::tempdir().expect("workdir");
        let layout = RokoLayout::for_project(workdir.path());
        for (task_id, attempt_key) in [("T1", "run-1:01-run:T1:1"), ("T2", "run-1:01-run:T2:3")] {
            let hold = layout.review_hold("01-run", task_id);
            std::fs::create_dir_all(hold.parent().expect("holds dir")).expect("holds dir");
            let body = serde_json::json!({"task_id": task_id, "attempt_key": attempt_key});
            std::fs::write(&hold, body.to_string()).expect("write the hold");
        }
        let approve = |approval_id: &str| ExecutionCommandKind::Approve {
            approval_id: approval_id.to_string(),
        };
        let reject = ExecutionCommandKind::RejectApproval {
            approval_id: "01-run/T2".to_string(),
            reason: "name the file after the feature".to_string(),
        };

        let seen = route_tui_commands_in(
            workdir.path(),
            vec![
                (approve("01-run/T1"), Some("01-run"), Some("T1")),
                (reject, Some("01-run"), Some("T2")),
                (approve("01-run/T9"), Some("01-run"), Some("T9")),
                (approve("ap-1"), Some("01-run"), None),
            ],
            &crate::graph_task_dispatch::OperatorStops::default(),
        );

        let statuses: Vec<_> = seen.acks.iter().map(|ack| ack.status).collect();
        assert_eq!(
            statuses,
            [
                CommandAckStatus::Completed,
                CommandAckStatus::Completed,
                CommandAckStatus::Rejected,
                CommandAckStatus::Rejected,
            ]
        );
        let not_held = seen.acks[2].message.as_deref().unwrap_or_default();
        assert!(not_held.contains("not waiting for a review"), "{not_held}");
        let log = std::fs::read_to_string(layout.reviews_log()).expect("review log");
        let entries: Vec<serde_json::Value> = log
            .lines()
            .map(|line| serde_json::from_str(line).expect("review entry"))
            .collect();
        assert_eq!(entries.len(), 2, "{log}");
        assert_eq!(entries[0]["task_id"], "T1");
        assert_eq!(entries[0]["decision"], "approved");
        assert_eq!(entries[0]["attempt_key"], "run-1:01-run:T1:1");
        assert_eq!(entries[1]["task_id"], "T2");
        assert_eq!(entries[1]["decision"], "rejected");
        assert_eq!(entries[1]["attempt_key"], "run-1:01-run:T2:3");
        assert_eq!(entries[1]["comment"], "name the file after the feature");
    }

    /// A TUI cancel stops the running plan it names and drops a waiting one
    /// before it starts; both are acknowledged as done.
    #[test]
    fn tui_cancel_reaches_a_running_plan_and_drops_a_waiting_one() {
        let seen = route_tui_commands(
            vec![
                (ExecutionCommandKind::Cancel, Some("01-run"), None),
                (ExecutionCommandKind::Cancel, Some("02-wait"), None),
            ],
            &crate::graph_task_dispatch::OperatorStops::default(),
        );

        assert_eq!(seen.routed.cancelled_before_start, ["02-wait"]);
        assert!(seen.run_cancelled);
        let statuses: Vec<_> = seen.acks.iter().map(|ack| ack.status).collect();
        assert_eq!(statuses, vec![CommandAckStatus::Completed; 2]);
    }

    /// gap-c002bb: a TUI skip naming a running task stops that task's agent,
    /// not its sibling's, leaves the plan running, and is acknowledged as
    /// done.
    #[tokio::test]
    async fn tui_skip_command_skips_the_running_task() {
        let task_stops = crate::graph_task_dispatch::OperatorStops::default();
        let skipped = task_stops.register("01-run", "T1");
        let sibling = task_stops.register("01-run", "T2");

        let seen = route_tui_commands(
            vec![(ExecutionCommandKind::Skip, Some("01-run"), Some("T1"))],
            &task_stops,
        );

        tokio::time::timeout(std::time::Duration::from_secs(5), skipped.stopped())
            .await
            .expect("the skipped task's agent is told to stop");
        assert!(!sibling.is_stopped(), "its sibling runs on");
        assert!(!seen.run_cancelled, "the plan runs on");
        let [ack] = seen.acks.as_slice() else {
            panic!("one acknowledgement, got {:?}", seen.acks);
        };
        assert_eq!(ack.status, CommandAckStatus::Completed, "{ack:?}");
    }

    /// gap-c002bb: soft retry and reset run a plan that failed earlier in the
    /// run again, resuming its checkpoint or starting over; the run accepts
    /// both, and starts the plan once a slot is free.
    #[test]
    fn tui_retry_and_reset_run_a_failed_plan_again() {
        for (kind, rerun) in [
            (ExecutionCommandKind::SoftRetry, PlanRerun::Resume),
            (
                ExecutionCommandKind::Repair {
                    preserve_completed: true,
                },
                PlanRerun::Resume,
            ),
            (ExecutionCommandKind::Reset, PlanRerun::Fresh),
            (
                ExecutionCommandKind::Repair {
                    preserve_completed: false,
                },
                PlanRerun::Fresh,
            ),
        ] {
            let seen = route_tui_commands(
                vec![(kind.clone(), Some("03-failed"), None)],
                &crate::graph_task_dispatch::OperatorStops::default(),
            );

            assert_eq!(
                seen.routed.reruns,
                [("03-failed".to_string(), rerun)],
                "{kind}"
            );
            let [ack] = seen.acks.as_slice() else {
                panic!("one acknowledgement, got {:?}", seen.acks);
            };
            assert_eq!(ack.status, CommandAckStatus::Accepted, "{kind}");
        }
    }

    /// gap-19e596: with per-task worktrees no two tasks share a tree, so the
    /// plan graph keeps no exclusive paths, in the simple and in the rich
    /// topology. In a shared tree each task keeps the files it declares.
    #[test]
    fn worktree_per_task_clears_exclusive_paths() {
        let plan_task = |id: &str| roko_graph::convert::PlanTaskInfo {
            title: format!("Task {id}"),
            description: None,
            role: Some("implementer".to_string()),
            tier: "focused".to_string(),
            model_hint: None,
            files: vec!["src/lib.rs".to_string()],
            depends_on: Vec::new(),
            depends_on_plan: Vec::new(),
            timeout_secs: 60,
            max_retries: 0,
            domain: None,
            sequence: 0,
            full_config_json: serde_json::Value::Null,
        };
        let topology_task = |id: &str| roko_graph::TopologyTaskInfo {
            task_id: id.to_string(),
            title: format!("Task {id}"),
            description: None,
            role: Some("implementer".to_string()),
            tier: "focused".to_string(),
            model_hint: None,
            files: vec!["src/lib.rs".to_string()],
            depends_on: Vec::new(),
            timeout_secs: 60,
            max_retries: 0,
            domain: None,
            sequence: 0,
            full_config_json: serde_json::Value::Null,
        };
        let simple = roko_graph::convert::plan_to_graph(
            "p",
            "plans/p",
            &[
                ("T1".to_string(), plan_task("T1")),
                ("T2".to_string(), plan_task("T2")),
            ],
            2,
        )
        .expect("simple topology");
        let (rich, _) = roko_graph::ProductionPlanTopology::new("p", "plans/p", 2)
            .build(&[topology_task("T1"), topology_task("T2")])
            .expect("rich topology");

        let holds_paths = |graph: &roko_graph::Graph| {
            graph
                .inner
                .node_weights()
                .any(|node| !node.exclusive.is_empty())
        };
        for graph in [simple, rich] {
            let mut shared = graph.clone();
            drop_exclusion_for_worktrees(&mut shared, false);
            assert!(holds_paths(&shared));
            let mut isolated = graph;
            drop_exclusion_for_worktrees(&mut isolated, true);
            assert!(!holds_paths(&isolated));
        }
    }

    /// Stands in for a rich-topology task executor under
    /// `--worktree-per-task`: runs its attempt in a worktree from `provider`
    /// and hands that worktree on to the task's gate.
    struct HandsOnItsWorktree {
        provider: Arc<crate::graph_execution::WorktreeExecutionWorkspaceProvider>,
    }

    #[async_trait::async_trait]
    impl roko_graph::Cell for HandsOnItsWorktree {
        fn cell_id(&self) -> &'static str {
            "task-executor"
        }

        fn cell_name(&self) -> &'static str {
            "HandsOnItsWorktree"
        }

        async fn execute(
            &self,
            _input: Vec<roko_core::Signal>,
            ctx: &roko_graph::cell::CellContext,
        ) -> roko_core::error::Result<Vec<roko_core::Signal>> {
            use roko_graph::workspace::ExecutionWorkspaceProvider as _;
            let lease = self
                .provider
                .acquire(&roko_graph::workspace::WorkspaceAttemptId {
                    plan_id: "rich".to_string(),
                    task_id: "T1".to_string(),
                    attempt: 0,
                })
                .await
                .map_err(|error| roko_core::RokoError::Invalid(error.to_string()))?;
            std::fs::write(lease.path.join("feature.txt"), "feature\n")?;
            let mut output = vec![
                roko_core::Signal::builder(roko_core::Kind::AgentOutput)
                    .body(roko_core::Body::text("done"))
                    .build(),
            ];
            roko_graph::cells::TaskAttempt {
                plan_id: "rich".to_string(),
                task_id: "T1".to_string(),
                run_id: ctx.run_id.clone(),
                attempt: 1,
                workspace: Some(lease.path.clone()),
                lease: Some(lease),
                ..roko_graph::cells::TaskAttempt::default()
            }
            .stamp(&mut output);
            roko_graph::cells::task_executor::TaskGateVerdict::Passed.stamp(&mut output);
            Ok(output)
        }
    }

    fn git_in(dir: &Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .current_dir(dir)
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// gap-6daad9: a rich-topology plan runs with the gates and the run's
    /// workspace provider, so its gate cells run: the gate judges the
    /// worktree the executor handed on (here the compile rung fails, on a
    /// manifest cargo cannot parse), fails its task, and keeps that worktree
    /// for post-mortem instead of accepting it. The default topology gets
    /// neither service.
    /// Records the `[gates]` config of each gate pipeline it is asked to
    /// run, and fails it.
    #[derive(Default)]
    struct GatesConfigRecorder(parking_lot::Mutex<Vec<roko_core::config::GatesConfig>>);

    #[async_trait::async_trait]
    impl roko_gate::production_service::ProductionGateRunner for GatesConfigRecorder {
        async fn run(
            &self,
            request: roko_gate::ProductionGateRequest,
            _progress_sink: Arc<dyn roko_gate::production_service::ProgressSink>,
        ) -> roko_core::Result<roko_gate::ProductionGateVerdictV1> {
            self.0.lock().push(request.gates_config);
            Err(roko_core::RokoError::Invalid("recorded".to_string()))
        }
    }

    /// bug-4862cf: the rich topology's gates run with the run's `[gates]`,
    /// not `GatesConfig::default()`; the run's `max_rung` bounds each
    /// pipeline.
    #[tokio::test]
    async fn rich_topology_gates_use_the_runs_gates_config() {
        let mut gates = roko_core::config::GatesConfig::default();
        gates.env_passthrough = vec!["FROM_THE_RUN".to_string()];
        gates.max_rung = Some(1);
        let recorder = Arc::new(GatesConfigRecorder::default());
        let resources = plan_cell_resources_with(true, &gates, None, Arc::clone(&recorder) as _);
        let evaluator = resources.gates.expect("the rich topology runs gates");
        let request = roko_core::SharedGateRequest {
            task_id: "T1".to_string(),
            attempt_id: 1,
            rung: "compile".to_string(),
            plan_dir: "plans/p".to_string(),
            worktree_path: PathBuf::from("/wt/attempt"),
            changed_files: Vec::new(),
            context: HashMap::new(),
        };
        assert!(evaluator.verify_rung(&request).await.is_err());

        let configs = recorder.0.lock();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].env_passthrough, ["FROM_THE_RUN"]);
        assert_eq!(configs[0].max_rung, Some(1));
        assert!(
            plan_cell_resources_with(false, &gates, None, Arc::clone(&recorder) as _)
                .gates
                .is_none()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rich_topology_gates_run_with_cell_resources() {
        use roko_graph::engine::{GraphEngine, NodeStatus};

        let repo = tempfile::tempdir().expect("repo");
        git_in(repo.path(), &["init", "-b", "main"]);
        for (key, value) in [
            ("user.email", "operator@example.test"),
            ("user.name", "Operator"),
            ("commit.gpgsign", "false"),
        ] {
            git_in(repo.path(), &["config", key, value]);
        }
        // A manifest cargo cannot parse fails the compile rung at once.
        std::fs::write(repo.path().join("Cargo.toml"), "not a manifest [\n").expect("manifest");
        git_in(repo.path(), &["add", "Cargo.toml"]);
        git_in(repo.path(), &["commit", "-m", "base"]);
        let worktrees = tempfile::tempdir().expect("worktrees");
        let provider = Arc::new(
            crate::graph_execution::WorktreeExecutionWorkspaceProvider::new(
                crate::orchestrator::worktree::WorktreeManager::new(
                    crate::orchestrator::worktree::WorktreeConfig {
                        repo_root: repo.path().to_path_buf(),
                        base_branch: "HEAD".to_string(),
                        worktrees_root: worktrees.path().to_path_buf(),
                        max_live: None,
                        idle_ttl: Duration::from_secs(3600),
                    },
                ),
            ),
        );

        let gates = roko_core::config::GatesConfig::default();
        let resources = plan_cell_resources(true, &gates, Some(provider.clone()));
        assert!(resources.gates.is_some() && resources.workspaces.is_some());
        let default_topology = plan_cell_resources(false, &gates, Some(provider.clone()));
        assert!(default_topology.gates.is_none() && default_topology.workspaces.is_none());

        let task = roko_graph::TopologyTaskInfo {
            task_id: "T1".to_string(),
            title: "Add the feature".to_string(),
            description: None,
            role: Some("implementer".to_string()),
            tier: "focused".to_string(),
            model_hint: None,
            files: vec!["feature.txt".to_string()],
            depends_on: Vec::new(),
            timeout_secs: 60,
            max_retries: 0,
            domain: None,
            sequence: 0,
            full_config_json: serde_json::json!({"id": "T1"}),
        };
        let (graph, _) = roko_graph::ProductionPlanTopology::new("rich", "plans/rich", 1)
            .build(&[task])
            .expect("topology");
        let mut registry = roko_graph::default_registry();
        roko_graph::register_topology_cells(&mut registry);
        let executor_provider = provider.clone();
        registry.register("task-executor", move |_config| {
            Box::new(HandsOnItsWorktree {
                provider: executor_provider.clone(),
            })
        });
        let engine = GraphEngine::new(graph, registry).with_allow_test_stubs(true);
        let pause = Arc::new(AtomicBool::new(false));

        let output = engine
            .execute(&plan_cell_context(
                "rich-run",
                &pause,
                &Arc::new(AtomicBool::new(false)),
                &resources,
            ))
            .await
            .expect("the plan runs");

        let gate = output
            .node_results
            .iter()
            .find(|result| result.node_id == "task.T1.gate")
            .expect("the gate ran");
        let error = gate.error.clone().unwrap_or_default();
        assert_eq!(gate.status, NodeStatus::Failed, "{error}");
        assert!(error.contains("failed the plan gate"), "{error}");
        assert!(error.contains("compile"), "{error}");
        assert!(!output.success);
        // Kept, not accepted: the worktree is still there, and no plan branch
        // was made.
        let kept = provider
            .manager()
            .get_attempt("rich", "T1", 0)
            .expect("the worktree is kept");
        assert!(kept.path.join("feature.txt").is_file());
        let plan_branch = std::process::Command::new("git")
            .current_dir(repo.path())
            .args(["rev-parse", "--verify", "--quiet", "roko/plan/rich"])
            .output()
            .expect("git rev-parse");
        assert!(
            !plan_branch.status.success(),
            "a failed attempt was accepted"
        );
    }

    /// gap-0d64d5: a plan that holds its tasks for approval needs per-task
    /// worktrees and the default topology, whose acceptance the hold sits
    /// before; otherwise the run is refused before anything starts.
    #[tokio::test]
    async fn approval_needs_worktrees_and_the_default_topology() {
        let dir = tempfile::tempdir().expect("tempdir");
        let plan_dir = dir.path().join("plans").join("01-held");
        std::fs::create_dir_all(&plan_dir).expect("plan dir");
        std::fs::write(
            plan_dir.join("tasks.toml"),
            "[meta]\nplan = \"01-held\"\napproval = \"per_task\"\n\n[[task]]\nid = \"T1\"\n\
             title = \"Write held.txt\"\nfiles = [\"held.txt\"]\n\n[[task.verify]]\n\
             phase = \"structural\"\n\
             command = \"test -f held.txt\"\n",
        )
        .expect("tasks.toml");
        for (worktree_per_task, rich_topology) in [(false, false), (true, true)] {
            let error = run_graph_plan(GraphPlanRunParams {
                worktree_per_task,
                rich_topology,
                ..worktree_run_params(dir.path())
            })
            .await
            .expect_err("the run is refused");
            assert!(
                error.to_string().contains("hold each task for approval"),
                "{error}"
            );
        }
    }

    /// The rich topology's gates judge each attempt's own worktree, so a run
    /// without per-task worktrees is refused before anything starts.
    #[tokio::test]
    async fn rich_topology_needs_worktree_per_task() {
        let dir = tempfile::tempdir().expect("tempdir");
        let error = run_graph_plan(GraphPlanRunParams {
            plans_dir: dir.path().join("plans"),
            workdir: dir.path().to_path_buf(),
            quiet: true,
            json: false,
            resume_plan: None,
            fresh: false,
            force_resume: false,
            max_retries: None,
            max_tasks: 0,
            budget_override: None,
            no_budget: true,
            cli_model_override: None,
            dangerously_skip_permissions: false,
            log_file: None,
            worktree_per_task: false,
            worktree_per_task_explicit: false,
            rich_topology: true,
            promote: None,
            no_tui: true,
            state_hub: None,
            interrupt: None,
            max_parallel_plans: None,
            fail_fast: false,
            only_plans: None,
            live_agent_output: crate::graph_task_dispatch::LiveAgentOutput::ToolSteps,
            force_disk_check: false,
            effort: None,
            no_cascade: false,
            frozen_learning: false,
            no_holdout: false,
            metrics: None,
            outbound_floor: None,
        })
        .await
        .expect_err("the rich topology needs per-task worktrees");

        assert!(error.to_string().contains("--worktree-per-task"), "{error}");
    }

    /// gap-9980c6: a run's `effort` replaces `[agent] default_effort`
    /// (`medium` here) on its dispatches, as `roko run --effort low` asks.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_run_effort_reaches_the_provider() {
        let dir = tempfile::tempdir().expect("tempdir");
        fake_provider_workspace(dir.path(), 0.0, "");
        // That workspace's provider, logging the arguments of each call.
        std::fs::write(
            dir.path().join("fake-provider.sh"),
            r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' "$*" >> "$(dirname "$0")/provider-args"
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"done"}}'
printf '%s\n' '{"type":"result","session_id":"fake","model":"claude-sonnet-4-6","total_cost_usd":0,"usage":{"input_tokens":1,"output_tokens":1},"is_error":false}'
"#,
        )
        .expect("provider script");
        write_verify_plan(dir.path(), "effort", "", &[("T1", &[], "true")]);

        let exit_code = run_graph_plan(GraphPlanRunParams {
            worktree_per_task: false,
            effort: Some("low".to_string()),
            ..worktree_run_params(dir.path())
        })
        .await
        .expect("run the plan");

        assert_eq!(exit_code, EXIT_SUCCESS);
        let args = std::fs::read_to_string(dir.path().join("provider-args")).expect("calls");
        assert!(args.contains("--effort low"), "{args}");
    }

    /// gap-d8c39a: a verify step a Graph run settles counts in the run's
    /// metric registry, which serve's `/metrics` renders, and not only in
    /// the tracing fields.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn graph_verify_increments_gate_verdict_metrics() {
        let dir = tempfile::tempdir().expect("tempdir");
        fake_provider_workspace(dir.path(), 0.0, "");
        write_verify_plan(dir.path(), "metrics", "", &[("T1", &[], "true")]);
        let registry = Arc::new(roko_core::obs::metrics::MetricRegistry::new());
        roko_core::obs::metrics::register_standard_metrics(&registry);

        let exit_code = run_graph_plan(GraphPlanRunParams {
            worktree_per_task: false,
            metrics: Some(Arc::clone(&registry)),
            ..worktree_run_params(dir.path())
        })
        .await
        .expect("run the plan");

        assert_eq!(exit_code, EXIT_SUCCESS);
        let text = registry.render_prometheus();
        let passes = text
            .lines()
            .find(|line| {
                line.starts_with("roko_gate_verdicts_total{") && line.contains("verdict=\"pass\"")
            })
            .unwrap_or_else(|| panic!("no passing verdict series in:\n{text}"));
        let count: u64 = passes
            .rsplit(' ')
            .next()
            .and_then(|value| value.parse().ok())
            .expect("a count");
        assert!(count >= 1, "{passes}");
        assert!(text.contains("roko_gate_duration_seconds_count{"), "{text}");
    }

    /// A scripted provider: it writes `<name>.txt` for the "Write <name>.txt"
    /// its prompt names, in the directory it runs in.
    const WRITES_NAMED_FILE_AGENT: &str = r#"#!/bin/sh
set -eu
prompt="$(cat) $*"
name=$(printf '%s' "$prompt" | sed -n 's/.*Write \([a-z]*\)\.txt.*/\1/p' | head -n 1)
printf '%s\n' "$name" > "$name.txt"
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"wrote the file"}}'
printf '%s\n' '{"type":"result","session_id":"sess-batch","model":"claude-sonnet-4-6","total_cost_usd":0.01,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

    /// A committed repository with `WRITES_NAMED_FILE_AGENT` configured as the
    /// provider, and one plan per name whose one task writes `<name>.txt`,
    /// each with `meta_verify` as its `[meta] verify` step when given.
    fn repo_with_file_plans(names: &[&str], meta_verify: Option<&str>) -> tempfile::TempDir {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = tempfile::tempdir().expect("tempdir");
        let repo = dir.path();
        let agent = repo.join("agent.sh");
        std::fs::write(&agent, WRITES_NAMED_FILE_AGENT).expect("agent");
        std::fs::set_permissions(&agent, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        std::fs::write(
            repo.join("roko.toml"),
            format!(
                "[agent]\ndefault_model = \"fake\"\n\n[providers.fake]\nkind = \"claude_cli\"\n\
                 command = \"{}\"\n\n[models.fake]\nprovider = \"fake\"\nslug = \"claude-sonnet-4-6\"\n",
                agent.display()
            ),
        )
        .expect("config");
        std::fs::write(repo.join(".gitignore"), ".roko/\n").expect("gitignore");
        let meta_verify = meta_verify
            .map(|command| format!("\n[[meta.verify]]\ncommand = {command:?}\n"))
            .unwrap_or_default();
        for (index, name) in names.iter().enumerate() {
            let plan_id = format!("{:02}-{name}", index + 1);
            let plan_dir = repo.join("plans").join(&plan_id);
            std::fs::create_dir_all(&plan_dir).expect("plan dir");
            std::fs::write(
                plan_dir.join("tasks.toml"),
                format!(
                    "[meta]\nplan = \"{plan_id}\"\nmax_parallel = 1\nskip_enrichment = true\n{meta_verify}\n\
                     [[task]]\nid = \"T1\"\ntitle = \"Write {name}.txt\"\n\
                     description = \"Write {name}.txt.\"\nrole = \"implementer\"\n\
                     status = \"ready\"\ntier = \"focused\"\nfiles = [\"{name}.txt\"]\n\n\
                     [[task.verify]]\nphase = \"structural\"\ncommand = \"test -f {name}.txt\"\n"
                ),
            )
            .expect("tasks.toml");
        }
        git_in(repo, &["init", "-b", "main"]);
        for (key, value) in [
            ("user.email", "operator@example.test"),
            ("user.name", "Operator"),
            ("commit.gpgsign", "false"),
        ] {
            git_in(repo, &["config", key, value]);
        }
        git_in(repo, &["add", "-A"]);
        git_in(repo, &["commit", "-m", "fixture"]);
        dir
    }

    fn git_stdout(dir: &Path, args: &[&str]) -> String {
        let output = std::process::Command::new("git")
            .current_dir(dir)
            .args(args)
            .output()
            .expect("run git");
        assert!(output.status.success(), "git {args:?}");
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn worktree_run_params(repo: &Path) -> GraphPlanRunParams {
        GraphPlanRunParams {
            plans_dir: repo.join("plans"),
            workdir: repo.to_path_buf(),
            quiet: true,
            json: false,
            resume_plan: None,
            fresh: false,
            force_resume: false,
            max_retries: None,
            max_tasks: 0,
            budget_override: None,
            no_budget: true,
            cli_model_override: None,
            dangerously_skip_permissions: false,
            log_file: None,
            worktree_per_task: true,
            worktree_per_task_explicit: false,
            rich_topology: false,
            promote: None,
            no_tui: true,
            state_hub: None,
            interrupt: None,
            max_parallel_plans: None,
            fail_fast: false,
            only_plans: None,
            live_agent_output: crate::graph_task_dispatch::LiveAgentOutput::ToolSteps,
            force_disk_check: false,
            effort: None,
            no_cascade: false,
            frozen_learning: false,
            no_holdout: false,
            metrics: None,
            outbound_floor: None,
        }
    }

    /// spec-f830c4: a `--worktree-per-task` run of two plans delivers each
    /// into the run's batch branch, the second starting from the first's
    /// work, records the delivery in each plan's checkpoint, promotes the
    /// batch into `--promote`'s branch with a `roko/run/<run-id>` tag, and
    /// never changes the operator's checkout.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_worktree_run_delivers_each_plan_into_its_batch_branch() {
        let dir = repo_with_file_plans(&["alpha", "beta"], None);
        let repo = dir.path();
        let head = git_stdout(repo, &["rev-parse", "HEAD"]);
        git_in(repo, &["branch", "release", "main"]);
        let params = GraphPlanRunParams {
            promote: Some("release".to_string()),
            ..worktree_run_params(repo)
        };

        let exit_code = run_graph_plan_in_run(params, Some("run-e2e".into()))
            .await
            .expect("run the plans");

        assert_eq!(exit_code, EXIT_SUCCESS);
        let batch = "roko/batch/run-e2e";
        let files = git_stdout(repo, &["ls-tree", "--name-only", batch]);
        assert!(
            files.contains("alpha.txt") && files.contains("beta.txt"),
            "{files}"
        );
        // beta's work was built on alpha's: the batch only fast-forwarded.
        let beta_plan = git_stdout(repo, &["rev-parse", "roko/plan/02-beta"]);
        assert_eq!(git_stdout(repo, &["rev-parse", batch]), beta_plan);
        for plan_id in ["01-alpha", "02-beta"] {
            let manifest: serde_json::Value = serde_json::from_slice(
                &std::fs::read(
                    repo.join(".roko/state/graph")
                        .join(plan_id)
                        .join("checkpoint.json"),
                )
                .expect("checkpoint"),
            )
            .expect("checkpoint json");
            let record = &manifest["extensions"]["roko.batch@1"]["value"];
            assert_eq!(record["branch"], batch, "{plan_id}: {record}");
            assert_eq!(record["state"], "delivered", "{plan_id}: {record}");
        }
        // Promoted: `release` is the batch, and the run is tagged.
        assert_eq!(git_stdout(repo, &["rev-parse", "release"]), beta_plan);
        assert_eq!(
            git_stdout(repo, &["rev-parse", "roko/run/run-e2e^{commit}"]),
            beta_plan
        );
        // gap-415c54: once a plan was delivered, its attempt checkout was
        // removed and its attempt branch kept, one per plan.
        let worktrees = git_stdout(repo, &["worktree", "list", "--porcelain"]);
        assert_eq!(
            worktrees
                .lines()
                .filter(|line| line.starts_with("worktree "))
                .count(),
            1,
            "{worktrees}"
        );
        let attempt_branches = git_stdout(repo, &["for-each-ref", "refs/heads/roko/attempt/"]);
        assert_eq!(attempt_branches.lines().count(), 2, "{attempt_branches}");
        // The operator's checkout never moved.
        assert_eq!(git_stdout(repo, &["rev-parse", "HEAD"]), head);
        assert_eq!(
            git_stdout(repo, &["symbolic-ref", "--short", "HEAD"]),
            "main"
        );
        assert_eq!(git_stdout(repo, &["status", "--porcelain"]), "");
        assert!(!repo.join("alpha.txt").exists());
    }

    /// backlog 3104: with per-task worktrees and `max_parallel_plans = 2`,
    /// two independent plans start before either ends, the run succeeds,
    /// and the run's batch branch holds both plans' work. With one plan at a
    /// time they keep their order, and the second starts from the first's
    /// work, so the batch only fast-forwards.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn worktree_plans_run_side_by_side_and_both_deliver() {
        for width in [2, 1] {
            let dir = repo_with_file_plans(&["alpha", "beta"], None);
            let repo = dir.path();
            let hub = crate::state_hub::shared_state_hub();
            let params = GraphPlanRunParams {
                max_parallel_plans: Some(width),
                state_hub: Some(hub.clone()),
                ..worktree_run_params(repo)
            };
            let run_id = format!("run-width-{width}");

            let exit_code = run_graph_plan_in_run(params, Some(run_id.clone()))
                .await
                .expect("run the plans");

            assert_eq!(exit_code, EXIT_SUCCESS, "width {width}");
            let lifecycle = plan_lifecycle(&hub);
            let batch = format!("roko/batch/{run_id}");
            let files = git_stdout(repo, &["ls-tree", "--name-only", batch.as_str()]);
            assert!(
                files.contains("alpha.txt") && files.contains("beta.txt"),
                "width {width}: {files}"
            );
            if width == 2 {
                let mut first_two = lifecycle[..2].to_vec();
                first_two.sort();
                assert_eq!(
                    first_two,
                    ["start 01-alpha", "start 02-beta"],
                    "both plans start before either ends: {lifecycle:?}"
                );
                assert!(lifecycle.contains(&"end 01-alpha true".to_string()));
                assert!(lifecycle.contains(&"end 02-beta true".to_string()));
            } else {
                assert_eq!(
                    lifecycle,
                    [
                        "start 01-alpha",
                        "end 01-alpha true",
                        "start 02-beta",
                        "end 02-beta true"
                    ]
                );
                assert_eq!(
                    git_stdout(repo, &["rev-parse", batch.as_str()]),
                    git_stdout(repo, &["rev-parse", "roko/plan/02-beta"])
                );
            }
        }
    }

    /// gap-415c54: with `[runner] delete_attempt_branches = true`, a
    /// delivered plan's attempt branch goes with its checkout, and the plan
    /// and batch branches still hold its work.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_delivered_plan_deletes_its_attempt_branches_when_asked() {
        let dir = repo_with_file_plans(&["alpha"], None);
        let repo = dir.path();
        let mut config = std::fs::read_to_string(repo.join("roko.toml")).expect("config");
        config.push_str("\n[runner]\ndelete_attempt_branches = true\n");
        std::fs::write(repo.join("roko.toml"), config).expect("config");
        git_in(repo, &["commit", "-am", "delete attempt branches"]);

        let exit_code = run_graph_plan_in_run(worktree_run_params(repo), Some("run-delete".into()))
            .await
            .expect("run the plan");

        assert_eq!(exit_code, EXIT_SUCCESS);
        assert_eq!(
            git_stdout(repo, &["for-each-ref", "refs/heads/roko/attempt/"]),
            ""
        );
        let worktrees = git_stdout(repo, &["worktree", "list", "--porcelain"]);
        assert_eq!(
            worktrees
                .lines()
                .filter(|line| line.starts_with("worktree "))
                .count(),
            1,
            "{worktrees}"
        );
        let batch = "roko/batch/run-delete";
        let files = git_stdout(repo, &["ls-tree", "--name-only", batch]);
        assert!(files.contains("alpha.txt"), "{files}");
        assert_eq!(
            git_stdout(repo, &["rev-parse", "roko/plan/01-alpha"]),
            git_stdout(repo, &["rev-parse", batch])
        );
    }

    /// gap-60233f: a plan whose tasks all passed but whose `[meta] verify`
    /// fails does not succeed, whether the check runs in the shared working
    /// tree or as the regression of its delivery into the batch branch, which
    /// keeps its old tip: it never moved (bug-700d7c). The checkpoint records
    /// why, for `roko plan status`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn meta_verify_failure_fails_a_plan_whose_tasks_passed() {
        for worktree_per_task in [false, true] {
            let dir = repo_with_file_plans(&["alpha"], Some("test -f together.txt"));
            let repo = dir.path();
            let head = git_stdout(repo, &["rev-parse", "HEAD"]);
            let params = GraphPlanRunParams {
                worktree_per_task,
                ..worktree_run_params(repo)
            };

            let exit_code = run_graph_plan_in_run(params, Some("run-check".into()))
                .await
                .expect("run the plan");

            let mode = if worktree_per_task {
                "worktree"
            } else {
                "shared tree"
            };
            assert_ne!(exit_code, EXIT_SUCCESS, "{mode}");
            assert_eq!(
                crate::graph_checkpoint::canonical_checkpoint_status(repo, "01-alpha"),
                Some(GraphCheckpointStatus::Failed),
                "{mode}"
            );
            let failure = crate::graph_checkpoint::recorded_plan_check_failure(repo, "01-alpha")
                .unwrap_or_else(|| panic!("{mode}: no recorded plan check failure"));
            assert!(
                failure.contains("test -f together.txt"),
                "{mode}: {failure}"
            );
            if worktree_per_task {
                assert_eq!(
                    git_stdout(repo, &["rev-parse", "roko/batch/run-check"]),
                    head,
                    "the failed plan stayed out of the batch"
                );
                // The batch never pointed at the failed plan's merge: its
                // reflog holds its creation alone (bug-700d7c).
                assert_eq!(
                    git_stdout(
                        repo,
                        &[
                            "log",
                            "--walk-reflogs",
                            "--format=%H",
                            "roko/batch/run-check"
                        ]
                    ),
                    head,
                    "the batch moved to the failed plan's merge"
                );
            }
        }
    }

    /// bug-700d7c: with plans running side by side, a plan that starts while
    /// another plan's whole-plan check runs starts from the run's batch
    /// branch as it was, not from the merge under check. Here 01-alpha's
    /// check waits until 03-gamma's first attempt has started, which happens
    /// once 02-beta failed, and then fails. gamma neither builds on alpha's
    /// work nor brings it into the batch when it is delivered, and the batch
    /// never pointed at alpha's merge.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_plan_started_during_a_failing_check_does_not_build_on_its_merge() {
        let dir = repo_with_file_plans(&["alpha", "beta", "gamma"], None);
        let repo = dir.path();
        let scratch = tempfile::tempdir().expect("scratch");
        let checking = scratch.path().join("alpha-checking");
        let saw_gamma = scratch.path().join("alpha-saw-gamma");
        // gamma's first attempt branch exists once its checkout was made.
        let gamma_attempts = (0..=3)
            .map(|attempt| {
                format!(
                    "refs/heads/{}",
                    crate::orchestrator::worktree::format_attempt_branch_name(
                        "03-gamma", "T1", attempt
                    )
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        // alpha's whole-plan check waits (60 s at most) for gamma's attempt,
        // and fails.
        let alpha_check = format!(
            "touch '{checking}'; i=0; while [ ! -e '{saw}' ] && [ $i -lt 1200 ]; do \
             for ref in {gamma_attempts}; do \
             git rev-parse --verify --quiet $ref >/dev/null && touch '{saw}'; done; \
             sleep 0.05; i=$((i+1)); done; exit 1",
            checking = checking.display(),
            saw = saw_gamma.display(),
        );
        // beta's one task fails once alpha's check runs, which frees the slot
        // gamma starts in.
        let beta_verify = format!(
            "i=0; while [ ! -e '{checking}' ] && [ $i -lt 1200 ]; do sleep 0.05; i=$((i+1)); \
             done; exit 1",
            checking = checking.display(),
        );
        let alpha_tasks = repo.join("plans/01-alpha/tasks.toml");
        let text = std::fs::read_to_string(&alpha_tasks).expect("tasks.toml");
        let text = text.replacen(
            "skip_enrichment = true\n",
            &format!("skip_enrichment = true\n\n[[meta.verify]]\ncommand = {alpha_check:?}\n"),
            1,
        );
        std::fs::write(&alpha_tasks, text).expect("tasks.toml");
        let beta_tasks = repo.join("plans/02-beta/tasks.toml");
        let text = std::fs::read_to_string(&beta_tasks)
            .expect("tasks.toml")
            .replace(
                "files = [\"beta.txt\"]\n",
                "files = [\"beta.txt\"]\nmax_retries = 0\n",
            )
            .replace(
                "command = \"test -f beta.txt\"",
                &format!("command = {beta_verify:?}"),
            );
        std::fs::write(&beta_tasks, text).expect("tasks.toml");
        git_in(repo, &["commit", "-am", "alpha's check waits for gamma"]);
        let head = git_stdout(repo, &["rev-parse", "HEAD"]);
        let params = GraphPlanRunParams {
            max_parallel_plans: Some(2),
            ..worktree_run_params(repo)
        };

        let exit_code = run_graph_plan_in_run(params, Some("run-overlap".into()))
            .await
            .expect("run the plans");

        assert_ne!(exit_code, EXIT_SUCCESS);
        assert!(
            saw_gamma.exists(),
            "gamma's first attempt did not start while alpha's check ran"
        );
        assert_eq!(
            crate::graph_checkpoint::canonical_checkpoint_status(repo, "01-alpha"),
            Some(GraphCheckpointStatus::Failed)
        );
        let batch = "roko/batch/run-overlap";
        let delivered = crate::graph_checkpoint::recorded_batch_delivery(repo, "03-gamma")
            .expect("gamma was delivered");
        assert_eq!(
            delivered.merge_commit,
            git_stdout(repo, &["rev-parse", batch])
        );
        let files = git_stdout(repo, &["ls-tree", "--name-only", batch]);
        assert!(
            files.contains("gamma.txt") && !files.contains("alpha.txt"),
            "alpha's work reached the batch: {files}"
        );
        let alpha = git_stdout(repo, &["rev-parse", "roko/plan/01-alpha"]);
        for branch in ["roko/plan/03-gamma", batch] {
            let holds_alpha = std::process::Command::new("git")
                .current_dir(repo)
                .args(["merge-base", "--is-ancestor", alpha.as_str(), branch])
                .env_remove("GIT_DIR")
                .env_remove("GIT_WORK_TREE")
                .status()
                .expect("git merge-base");
            assert!(!holds_alpha.success(), "{branch} builds on alpha's work");
        }
        // The batch held its base, and then gamma's delivery: never alpha's
        // merge.
        let held = git_stdout(repo, &["log", "--walk-reflogs", "--format=%H", batch]);
        assert_eq!(
            held.lines().collect::<Vec<_>>(),
            [delivered.merge_commit.as_str(), head.as_str()]
        );
    }

    /// bug-7b62ff: a cancel that comes while a plan's whole-plan check runs,
    /// in the shared working tree or as its delivery's post-merge regression
    /// check, stops the check at once and kills what it started. The run
    /// ends cancelled, its checkpoint names the cancel, and it releases the
    /// workspace's runner lock, so the next run can start.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_cancel_stops_the_whole_plan_check_and_frees_the_workspace() {
        use crate::check_process_probe::{sleeping_step, wait_for_exit, wait_for_pids};

        for worktree_per_task in [false, true] {
            let mode = if worktree_per_task {
                "regression check"
            } else {
                "shared tree"
            };
            let scratch = tempfile::tempdir().expect("tempdir");
            let pids = scratch.path().join("pids");
            let dir = repo_with_file_plans(&["alpha"], Some(sleeping_step(&pids).as_str()));
            let repo = dir.path().to_path_buf();
            let head = git_stdout(&repo, &["rev-parse", "HEAD"]);
            let interrupt = PlanRunInterruptHandle::default();
            let params = GraphPlanRunParams {
                worktree_per_task,
                interrupt: Some(interrupt.clone()),
                ..worktree_run_params(&repo)
            };
            let run = tokio::spawn(run_graph_plan_in_run(params, Some("run-cancel".into())));
            let running = wait_for_pids(&pids).await;

            interrupt.request(PlanRunInterrupt::Cancel);
            let exit_code = tokio::time::timeout(Duration::from_secs(30), run)
                .await
                .unwrap_or_else(|_| panic!("{mode}: the cancelled run did not end"))
                .expect("the run task")
                .expect("the run");

            assert_eq!(exit_code, PlanRunInterrupt::Cancel.exit_code(), "{mode}");
            for pid in running {
                wait_for_exit(pid).await;
            }
            assert_eq!(
                crate::graph_checkpoint::canonical_stop_cause(&repo, "01-alpha").as_deref(),
                Some("cancel"),
                "{mode}"
            );
            let _next = crate::workspace_lock::acquire_runner_lock(&repo.join(".roko"))
                .unwrap_or_else(|error| panic!("{mode}: the run kept the workspace: {error}"));
            if worktree_per_task {
                // The delivery is cancelled, not failed, and its merge never
                // reached the batch (or the batch branch went with the run).
                let checkpoint = repo.join(".roko/state/graph/01-alpha/checkpoint.json");
                let checkpoint: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(checkpoint).expect("checkpoint"))
                        .expect("checkpoint json");
                let delivery = crate::graph_checkpoint::DELIVERY_EXTENSION;
                assert_eq!(
                    checkpoint["extensions"][delivery]["value"]["receipt"]["state"],
                    "cancelled"
                );
                let batch = std::process::Command::new("git")
                    .current_dir(&repo)
                    .args([
                        "rev-parse",
                        "--verify",
                        "--quiet",
                        "refs/heads/roko/batch/run-cancel",
                    ])
                    .output()
                    .expect("git rev-parse");
                let tip = String::from_utf8_lossy(&batch.stdout).trim().to_string();
                assert!(
                    tip.is_empty() || tip == head,
                    "the stopped plan stayed in the batch at {tip}"
                );
            }
        }
    }

    /// gap-4f6cbe: a Graph run publishes its plan's delivery into the batch
    /// branch, `delivery_started` and then `delivery_finished` with the
    /// state it ended in, and records both in the run's events. A delivered
    /// plan's ends `delivered`, and `roko serve`'s summary of the run says
    /// "delivery completed". One whose regression check a cancel cut short
    /// ends `cancelled`, says the cancel stopped it, and the summary says
    /// "delivery cancelled": it stopped, it did not fail.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_graph_run_publishes_its_delivery_end() {
        use crate::check_process_probe::{sleeping_step, wait_for_exit, wait_for_pids};

        fn deliveries(hub: &crate::state_hub::SharedStateHub) -> Vec<String> {
            hub.subscribe_events_from(0)
                .replay
                .iter()
                .filter_map(|envelope| match &envelope.payload {
                    roko_core::DashboardEvent::DeliveryStarted {
                        plan_id,
                        target,
                        branch,
                        ..
                    } => Some(format!("started {plan_id}: {branch} into {target}")),
                    roko_core::DashboardEvent::DeliveryFinished {
                        plan_id,
                        state,
                        stopped_by,
                        ..
                    } => Some(format!(
                        "finished {plan_id}: {state}, stopped by {}",
                        stopped_by.as_deref().unwrap_or("nothing")
                    )),
                    _ => None,
                })
                .collect()
        }
        async fn delivery_milestone(repo: &Path, run_id: &str) -> String {
            let deploy_backend = Arc::from(
                roko_serve::deploy::create_backend("manual", None, None, None)
                    .expect("manual backend"),
            );
            let runtime = crate::serve_runtime::RokoCliRuntime::new(
                crate::config::Config::default(),
                crate::config::RepoRegistry::default(),
            );
            let state = Arc::new(
                roko_serve::state::AppState::new(
                    repo.to_path_buf(),
                    runtime.into_arc(),
                    roko_core::config::schema::RokoConfig::default(),
                    deploy_backend,
                )
                .expect("serve state"),
            );
            let summary = roko_serve::routes::run_summary(&state, run_id)
                .await
                .unwrap_or_else(|error| panic!("{run_id}: no summary: {error:?}"));
            summary["milestones"]
                .as_array()
                .and_then(|milestones| {
                    milestones
                        .iter()
                        .find(|milestone| milestone["kind"] == "delivery")
                })
                .and_then(|milestone| milestone["text"].as_str())
                .unwrap_or_else(|| panic!("{run_id}: no delivery milestone: {summary}"))
                .to_string()
        }

        let dir = repo_with_file_plans(&["alpha"], Some("test -f alpha.txt"));
        let repo = dir.path();
        let hub = crate::state_hub::shared_state_hub();
        let params = GraphPlanRunParams {
            state_hub: Some(hub.clone()),
            ..worktree_run_params(repo)
        };
        let exit_code = run_graph_plan_in_run(params, Some("run-delivered".into()))
            .await
            .expect("run the plan");
        assert_eq!(exit_code, EXIT_SUCCESS);
        assert_eq!(
            deliveries(&hub),
            [
                "started 01-alpha: roko/plan/01-alpha into roko/batch/run-delivered",
                "finished 01-alpha: delivered, stopped by nothing",
            ]
        );
        assert_eq!(
            delivery_milestone(repo, "run-delivered").await,
            "delivery completed"
        );

        let scratch = tempfile::tempdir().expect("tempdir");
        let pids = scratch.path().join("pids");
        let dir = repo_with_file_plans(&["alpha"], Some(sleeping_step(&pids).as_str()));
        let repo = dir.path().to_path_buf();
        let hub = crate::state_hub::shared_state_hub();
        let interrupt = PlanRunInterruptHandle::default();
        let params = GraphPlanRunParams {
            state_hub: Some(hub.clone()),
            interrupt: Some(interrupt.clone()),
            ..worktree_run_params(&repo)
        };
        let run = tokio::spawn(run_graph_plan_in_run(params, Some("run-stopped".into())));
        let running = wait_for_pids(&pids).await;
        interrupt.request(PlanRunInterrupt::Cancel);
        let exit_code = tokio::time::timeout(Duration::from_secs(30), run)
            .await
            .expect("the cancelled run ends")
            .expect("the run task")
            .expect("the run");
        assert_eq!(exit_code, PlanRunInterrupt::Cancel.exit_code());
        for pid in running {
            wait_for_exit(pid).await;
        }
        assert_eq!(
            deliveries(&hub),
            [
                "started 01-alpha: roko/plan/01-alpha into roko/batch/run-stopped",
                "finished 01-alpha: cancelled, stopped by cancel",
            ]
        );
        assert_eq!(
            delivery_milestone(&repo, "run-stopped").await,
            "delivery cancelled"
        );
    }

    /// gap-de0e96: the run trend has one bucket per plan run, oldest first,
    /// with its cost and its cost per verified task; a run with no verified
    /// task has none.
    #[test]
    fn the_run_trend_gives_each_runs_cost_per_verified_task() {
        let run = |run_id: &str, timestamp: &str, cost: f64, verified: usize| {
            roko_learn::run_metrics::RunMetricsRecord {
                run_id: run_id.to_string(),
                timestamp: timestamp.to_string(),
                duration_ms: 4_000,
                total_tasks: 2,
                tasks_completed: verified,
                tasks_already_satisfied: 0,
                tasks_failed: 2 - verified,
                tasks_unverified: 0,
                tasks_skipped: 0,
                total_cost_usd: cost,
                total_tokens_in: 100,
                total_tokens_out: 10,
                total_agent_calls: 4,
                budget_exhausted: false,
                plans: Vec::new(),
            }
        };
        // Newest first, as `run-metrics.jsonl` gives them.
        let runs = [
            run("r3", "2026-10-16T10:02:00Z", 0.0, 0),
            run("r2", "2026-10-16T10:01:00Z", 0.01, 2),
            run("r1", "2026-10-16T10:00:00Z", 0.03, 1),
        ];

        let trend = run_cost_trend(&runs);

        let rows: Vec<(Option<&str>, Option<u64>, Option<f64>)> = trend
            .iter()
            .map(|bucket| {
                (
                    bucket.run_id.as_deref(),
                    bucket.verified_tasks,
                    bucket.cost_per_verified_usd,
                )
            })
            .collect();
        assert_eq!(
            rows,
            [
                (Some("r1"), Some(1), Some(0.03)),
                (Some("r2"), Some(2), Some(0.005)),
                (Some("r3"), Some(0), None),
            ]
        );
        assert_eq!(trend[0].start.to_rfc3339(), "2026-10-16T09:59:56+00:00");
        assert_eq!((trend[0].turns, trend[0].cost_usd), (4, Some(0.03)));
    }

    /// gap-de0e96: a Graph run publishes the recent runs' cost per verified
    /// task as it ends, this run's bucket last, in its own events: a trend a
    /// host can draw without reading `run-metrics.jsonl`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn run_end_publishes_cost_per_verified_task() {
        let dir = repo_with_file_plans(&["alpha"], None);
        let repo = dir.path();
        let hub = crate::state_hub::shared_state_hub();
        let params = GraphPlanRunParams {
            state_hub: Some(hub.clone()),
            ..worktree_run_params(repo)
        };

        let exit_code = run_graph_plan_in_run(params, Some("run-trend".into()))
            .await
            .expect("run the plan");

        assert_eq!(exit_code, EXIT_SUCCESS);
        let trends: Vec<Vec<roko_core::EfficiencyBucket>> = hub
            .subscribe_events_from(0)
            .replay
            .into_iter()
            .filter_map(|envelope| match envelope.payload {
                roko_core::DashboardEvent::EfficiencyTrendUpdated { buckets } => Some(buckets),
                _ => None,
            })
            .collect();
        let [trend] = trends.as_slice() else {
            panic!("one trend at the run's end: {trends:?}");
        };
        let last = trend.last().expect("this run's bucket");
        assert_eq!(last.tasks, Some(1), "{last:?}");
        assert_eq!(last.verified_tasks, Some(1), "{last:?}");
        let cost = last.cost_usd.expect("the run's cost");
        assert!(cost > 0.0, "{last:?}");
        assert_eq!(last.cost_per_verified_usd, Some(cost), "{last:?}");
    }

    /// bug-db89d4: the closing tally counts each way a plan ended, and leaves
    /// out the ways none did.
    #[test]
    fn the_closing_tally_names_each_outcome() {
        assert_eq!(
            outcome_tally(&[Some(PlanOutcome::Unverified)]),
            "1 unverified"
        );
        assert_eq!(
            outcome_tally(&[
                Some(PlanOutcome::Failed),
                None,
                Some(PlanOutcome::Succeeded),
                Some(PlanOutcome::Blocked),
                Some(PlanOutcome::Succeeded),
            ]),
            "2 succeeded, 1 failed, 1 blocked, 1 not started"
        );
    }

    /// bug-db89d4: a plan whose one task ran no verify step ends unverified.
    /// Its work is not delivered, so the batch stays at its base, but the
    /// plan's checkpoint names the commit no check ran on, the tip of its
    /// plan branch, and an event says where it is and how to see it. The
    /// run's closing tally says "unverified" and is not an error.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_unverified_run_says_where_its_unchecked_work_is() {
        let dir = repo_with_file_plans(&["alpha"], None);
        let repo = dir.path();
        let tasks = repo.join("plans/01-alpha/tasks.toml");
        let text = std::fs::read_to_string(&tasks).expect("tasks.toml");
        let (task, _verify) = text.split_once("\n[[task.verify]]").expect("a verify step");
        let task = task.replace(
            "skip_enrichment = true\n",
            "skip_enrichment = true\nallow_unverified = true\n",
        );
        std::fs::write(&tasks, format!("{task}\n")).expect("tasks.toml");
        git_in(repo, &["commit", "-am", "no verify step"]);
        let base = git_stdout(repo, &["rev-parse", "HEAD"]);
        let hub = crate::state_hub::shared_state_hub();
        let params = GraphPlanRunParams {
            state_hub: Some(hub.clone()),
            ..worktree_run_params(repo)
        };

        let exit_code = run_graph_plan_in_run(params, Some("run-unverified".into()))
            .await
            .expect("run the plan");

        assert_ne!(exit_code, EXIT_SUCCESS);
        assert_eq!(
            crate::graph_checkpoint::canonical_checkpoint_status(repo, "01-alpha"),
            Some(GraphCheckpointStatus::Unverified)
        );
        assert_eq!(
            git_stdout(repo, &["rev-parse", "roko/batch/run-unverified"]),
            base,
            "unverified work is not delivered"
        );
        let tip = git_stdout(repo, &["rev-parse", "roko/plan/01-alpha"]);
        assert!(
            git_stdout(repo, &["ls-tree", "--name-only", &tip]).contains("alpha.txt"),
            "the plan branch holds the work"
        );
        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(repo.join(".roko/state/graph/01-alpha/checkpoint.json"))
                .expect("checkpoint"),
        )
        .expect("checkpoint json");
        let record = &manifest["extensions"]["roko.batch@1"]["value"];
        assert_eq!(record["run_id"], "run-unverified", "{record}");
        assert_eq!(record["branch"], "roko/batch/run-unverified", "{record}");
        assert_eq!(record["plan_branch"], "roko/plan/01-alpha", "{record}");
        assert!(
            record.get("state").is_none(),
            "nothing was delivered: {record}"
        );
        let unverified = &record["work"]["unverified"];
        assert_eq!(unverified["commit"], tip.as_str(), "{record}");
        assert_eq!(unverified["base"], base.as_str(), "{record}");
        assert_eq!(
            unverified["tasks"],
            serde_json::json!([{ "task_id": "T1", "verdict": "unverified" }]),
            "{record}"
        );

        let events = hub.subscribe_events_from(0).replay;
        let errors = events
            .iter()
            .filter_map(|envelope| match &envelope.payload {
                roko_core::DashboardEvent::Error { message } => Some(message.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            errors
                .iter()
                .all(|message| !message.starts_with("Graph Engine")),
            "{errors:?}"
        );
        let logs = events
            .iter()
            .filter_map(|envelope| match &envelope.payload {
                roko_core::DashboardEvent::EventLogEntry {
                    event_type,
                    message,
                    ..
                } => Some((event_type.as_str(), message.as_str())),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            logs.contains(&("graph.run_outcome", "Graph Engine: 1 plan(s): 1 unverified")),
            "{logs:?}"
        );
        let (_, said) = logs
            .iter()
            .find(|(kind, _)| *kind == "graph.plan_unverified")
            .expect("the plan's unverified event");
        assert!(
            said.contains("roko/plan/01-alpha")
                && said.contains(&format!("git diff {}..roko/plan/01-alpha", &base[..12])),
            "{said}"
        );
        let ended = events
            .iter()
            .find_map(|envelope| match &envelope.payload {
                roko_core::DashboardEvent::RunCompleted { outcome, .. } => Some(outcome.as_str()),
                _ => None,
            })
            .expect("the run's end");
        assert_eq!(ended, "unverified");
    }

    /// bug-74164c: a plan whose T1 passed and whose T2 failed delivers T1's work,
    /// and only T1's, into the run's batch branch, marked partial. The plan's
    /// batch record names the delivered task and why the other was not
    /// delivered, an event says what was delivered, T1's accepted checkout is
    /// removed and T2's failed one is kept. The run still failed.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_plan_that_did_not_finish_delivers_its_verified_tasks() {
        let dir = repo_with_file_plans(&["alpha"], None);
        let repo = dir.path();
        let tasks = repo.join("plans/01-alpha/tasks.toml");
        let mut text = std::fs::read_to_string(&tasks).expect("tasks.toml");
        text.push_str(
            "\n[[task]]\nid = \"T2\"\ntitle = \"Write beta.txt\"\n\
             description = \"Write beta.txt.\"\nrole = \"implementer\"\nstatus = \"ready\"\n\
             tier = \"focused\"\nfiles = [\"beta.txt\"]\ndepends_on = [\"T1\"]\nmax_retries = 0\n\n\
             [[task.verify]]\nphase = \"structural\"\ncommand = \"test -f gamma.txt\"\n",
        );
        std::fs::write(&tasks, text).expect("tasks.toml");
        git_in(repo, &["commit", "-am", "a second task whose check fails"]);
        let hub = crate::state_hub::shared_state_hub();
        let params = GraphPlanRunParams {
            state_hub: Some(hub.clone()),
            ..worktree_run_params(repo)
        };

        let exit_code = run_graph_plan_in_run(params, Some("run-partial".into()))
            .await
            .expect("run the plan");

        assert_ne!(exit_code, EXIT_SUCCESS);
        assert_eq!(
            crate::graph_checkpoint::canonical_checkpoint_status(repo, "01-alpha"),
            Some(GraphCheckpointStatus::Failed)
        );
        let batch = "roko/batch/run-partial";
        let files = git_stdout(repo, &["ls-tree", "--name-only", batch]);
        assert!(
            files.contains("alpha.txt") && !files.contains("beta.txt"),
            "only T1's work is delivered: {files}"
        );
        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(repo.join(".roko/state/graph/01-alpha/checkpoint.json"))
                .expect("checkpoint"),
        )
        .expect("checkpoint json");
        let record = &manifest["extensions"]["roko.batch@1"]["value"];
        assert_eq!(record["state"], "delivered", "{record}");
        assert_eq!(record["partial"], true, "{record}");
        assert_eq!(
            record["work"],
            serde_json::json!({
                "partial": true,
                "delivered_tasks": ["T1"],
                "missing_tasks": [{ "task_id": "T2", "why": "failed" }],
            }),
            "{record}"
        );
        let delivery = crate::graph_checkpoint::recorded_batch_delivery(repo, "01-alpha")
            .expect("a recorded delivery");
        assert!(delivery.partial);
        assert_eq!(
            delivery.merge_commit,
            git_stdout(repo, &["rev-parse", batch])
        );
        // T1's checkout went with its delivery; T2's, holding beta.txt, stays.
        let worktrees = git_stdout(repo, &["worktree", "list", "--porcelain"]);
        let checkouts = worktrees
            .lines()
            .filter_map(|line| line.strip_prefix("worktree "))
            .collect::<Vec<_>>();
        assert_eq!(checkouts.len(), 2, "{worktrees}");
        assert!(
            Path::new(checkouts[1]).join("beta.txt").is_file(),
            "{worktrees}"
        );

        let events = hub.subscribe_events_from(0).replay;
        let logs = events
            .iter()
            .filter_map(|envelope| match &envelope.payload {
                roko_core::DashboardEvent::EventLogEntry {
                    event_type,
                    message,
                    ..
                } => Some((event_type.as_str(), message.as_str())),
                _ => None,
            })
            .collect::<Vec<_>>();
        let (_, said) = logs
            .iter()
            .find(|(kind, _)| *kind == "graph.plan_partially_delivered")
            .expect("the partial delivery's event");
        assert!(said.contains("T1") && said.contains(batch), "{said}");
        let errors = events
            .iter()
            .filter_map(|envelope| match &envelope.payload {
                roko_core::DashboardEvent::Error { message } => Some(message.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            errors.contains(&"Graph Engine: 1 plan(s): 1 failed"),
            "{errors:?}"
        );
    }

    /// A scripted provider that fails at once and changes nothing.
    const FAILS_AT_ONCE_AGENT: &str =
        "#!/bin/sh\ncat >/dev/null\necho 'provider down' >&2\nexit 1\n";

    /// bug-58329f: a run whose one task fails at once, changing nothing, leaves no
    /// attempt checkout, no attempt branch and no batch branch behind.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_run_that_did_nothing_leaves_no_checkout_or_branch() {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = repo_with_file_plans(&["alpha"], None);
        let repo = dir.path();
        std::fs::write(repo.join("agent.sh"), FAILS_AT_ONCE_AGENT).expect("agent");
        std::fs::set_permissions(
            repo.join("agent.sh"),
            std::fs::Permissions::from_mode(0o755),
        )
        .expect("chmod");
        let tasks = repo.join("plans/01-alpha/tasks.toml");
        let text = std::fs::read_to_string(&tasks).expect("tasks.toml");
        let text = text.replace(
            "files = [\"alpha.txt\"]\n",
            "files = [\"alpha.txt\"]\nmax_retries = 0\n",
        );
        std::fs::write(&tasks, text).expect("tasks.toml");
        git_in(repo, &["commit", "-am", "an agent that fails at once"]);

        let exit_code =
            run_graph_plan_in_run(worktree_run_params(repo), Some("run-nothing".into()))
                .await
                .expect("run the plan");

        assert_ne!(exit_code, EXIT_SUCCESS);
        let worktrees = git_stdout(repo, &["worktree", "list", "--porcelain"]);
        assert_eq!(
            worktrees
                .lines()
                .filter(|line| line.starts_with("worktree "))
                .count(),
            1,
            "{worktrees}"
        );
        assert_eq!(
            git_stdout(repo, &["for-each-ref", "refs/heads/roko/"]),
            "",
            "no attempt, plan or batch branch is left"
        );
    }

    /// bug-58329f: an unverified plan's accepted checkout is removed when the plan
    /// ends; its work stays on its plan branch, and its batch branch stays,
    /// since the run's summary names that work.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_unverified_plan_releases_its_accepted_checkout() {
        let dir = repo_with_file_plans(&["alpha"], None);
        let repo = dir.path();
        let tasks = repo.join("plans/01-alpha/tasks.toml");
        let text = std::fs::read_to_string(&tasks).expect("tasks.toml");
        let (task, _verify) = text.split_once("\n[[task.verify]]").expect("a verify step");
        let task = task.replace(
            "skip_enrichment = true\n",
            "skip_enrichment = true\nallow_unverified = true\n",
        );
        std::fs::write(&tasks, format!("{task}\n")).expect("tasks.toml");
        git_in(repo, &["commit", "-am", "no verify step"]);

        let exit_code =
            run_graph_plan_in_run(worktree_run_params(repo), Some("run-released".into()))
                .await
                .expect("run the plan");

        assert_ne!(exit_code, EXIT_SUCCESS);
        let worktrees = git_stdout(repo, &["worktree", "list", "--porcelain"]);
        assert_eq!(
            worktrees
                .lines()
                .filter(|line| line.starts_with("worktree "))
                .count(),
            1,
            "{worktrees}"
        );
        let tip = git_stdout(repo, &["rev-parse", "roko/plan/01-alpha"]);
        assert!(git_stdout(repo, &["ls-tree", "--name-only", &tip]).contains("alpha.txt"));
        assert!(!git_stdout(repo, &["rev-parse", "roko/batch/run-released"]).is_empty());
    }

    /// bug-7b62ff: the tasks of a plan whose whole-plan check a stop cut
    /// short are missing as interrupted, not as refused by a failed check.
    #[test]
    fn a_stopped_whole_plan_check_leaves_its_tasks_interrupted() {
        let plan = test_plan("p", "P", 1);
        let outcomes = TaskOutcomeSummary::default();
        let work = crate::graph_execution::batch::PlanWork {
            base: "b".repeat(40),
            commits: vec![crate::graph_execution::batch::AcceptedCommit {
                commit: "c".repeat(40),
                task_id: "T0".to_string(),
                verdict: "passed".to_string(),
            }],
        };
        for (outcome, why) in [
            (
                PlanOutcome::Interrupted,
                "the plan's whole-plan check was interrupted",
            ),
            (PlanOutcome::Failed, "the plan's whole-plan check failed"),
        ] {
            let undelivered = UndeliveredPlan {
                plan: &plan,
                run_id: "r",
                outcome,
                deliver_verified: false,
                task_outcomes: &outcomes,
            };
            let missing = missing_tasks(&undelivered, &work, &[]);
            assert_eq!(
                missing,
                vec![serde_json::json!({ "task_id": "T0", "why": why })]
            );
        }
    }

    /// bug-74164c, bug-1d09e8: a task a stop cut off is missing from a
    /// partial delivery as cancelled, or as interrupted when a stop request
    /// ended the plan, not as "not accepted".
    #[test]
    fn a_cut_off_task_is_missing_as_cancelled() {
        let plan = test_plan("p", "P", 2);
        let outcomes = TaskOutcomeSummary {
            cancelled: ["T1".to_string()].into_iter().collect(),
            ..TaskOutcomeSummary::default()
        };
        let work = crate::graph_execution::batch::PlanWork {
            base: "b".repeat(40),
            commits: Vec::new(),
        };
        for (outcome, why) in [
            (PlanOutcome::Cancelled, "cancelled"),
            (PlanOutcome::Interrupted, "interrupted"),
        ] {
            let undelivered = UndeliveredPlan {
                plan: &plan,
                run_id: "r",
                outcome,
                deliver_verified: true,
                task_outcomes: &outcomes,
            };
            let missing = missing_tasks(&undelivered, &work, &["T0".to_string()]);
            assert_eq!(
                missing,
                vec![serde_json::json!({ "task_id": "T1", "why": why })]
            );
        }
    }

    /// gap-c2efe7: a one-task plan named for people takes its title for its plan set
    /// entry, its accepted commit's subject and its batch branch's
    /// description; its task's own title stays for the agent.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_named_plan_titles_its_commit_branch_and_plan_set() {
        let dir = repo_with_file_plans(&["alpha"], None);
        let repo = dir.path();
        let tasks = repo.join("plans/01-alpha/tasks.toml");
        let text = std::fs::read_to_string(&tasks).expect("tasks.toml");
        let text = text.replacen(
            "plan = \"01-alpha\"\n",
            "plan = \"01-alpha\"\ntitle = \"Alpha, written for people\"\n",
            1,
        );
        std::fs::write(&tasks, text).expect("tasks.toml");
        git_in(repo, &["commit", "-am", "a titled plan"]);
        let hub = crate::state_hub::shared_state_hub();
        let params = GraphPlanRunParams {
            state_hub: Some(hub.clone()),
            ..worktree_run_params(repo)
        };

        let exit_code = run_graph_plan_in_run(params, Some("run-named".into()))
            .await
            .expect("run the plan");

        assert_eq!(exit_code, EXIT_SUCCESS);
        let subject = git_stdout(repo, &["log", "-1", "--format=%s", "roko/plan/01-alpha"]);
        assert_eq!(subject, "roko: 01-alpha/T1: Alpha, written for people");
        assert_eq!(
            git_stdout(
                repo,
                &["config", "--get", "branch.roko/batch/run-named.description"]
            ),
            "Alpha, written for people"
        );
        let titles = hub
            .subscribe_events_from(0)
            .replay
            .into_iter()
            .find_map(|envelope| match envelope.payload {
                roko_core::DashboardEvent::PlanSetLoaded { plans, .. } => {
                    Some(plans.into_iter().map(|plan| plan.title).collect::<Vec<_>>())
                }
                _ => None,
            })
            .expect("the plan set");
        assert_eq!(titles, ["Alpha, written for people"]);
    }
}
