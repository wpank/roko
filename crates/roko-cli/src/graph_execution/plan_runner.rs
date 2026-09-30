//! Graph plan execution entry point for library-callable use.
//!
//! This module contains the primary graph plan execution function, extracted
//! from the binary-side `cmd_plan_run_engine` so that `serve_runtime` and
//! other library callers can invoke it without depending on the binary crate.

use std::collections::HashMap;
use std::io::IsTerminal as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context as _, anyhow};
use futures::StreamExt as _;
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
/// order, with each plan's dependency wave, prerequisites and conflicts.
fn plan_set_entries(
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
            let title = plan.tasks.meta.plan.trim();
            roko_core::dashboard_snapshot::PlanSetEntry {
                plan_id: plan.id.clone(),
                title: if title.is_empty() {
                    plan.id.clone()
                } else {
                    title.to_string()
                },
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

// ── Stopping a plan run: SIGINT, SIGTERM, operator ───────────────────────

/// Why a CLI plan run stopped before its plan set finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanRunInterrupt {
    /// SIGINT, or the operator closed the TUI while plans were still running.
    Interrupt,
    /// SIGTERM.
    Terminate,
}

impl PlanRunInterrupt {
    /// Conventional shell status for the signal: 128 + signal number.
    #[must_use]
    pub const fn exit_code(self) -> i32 {
        match self {
            Self::Interrupt => 130,
            Self::Terminate => 143,
        }
    }

    /// Signal name for logs and summaries.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Interrupt => "SIGINT",
            Self::Terminate => "SIGTERM",
        }
    }

    const fn code(self) -> u8 {
        match self {
            Self::Interrupt => 1,
            Self::Terminate => 2,
        }
    }

    const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Interrupt),
            2 => Some(Self::Terminate),
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
}

/// How long a stopping plan waits for its cancelled graph to settle before
/// its checkpoint is finalized without it.
const INTERRUPT_DRAIN_TIMEOUT: Duration = Duration::from_secs(3);

/// Grace period between the first signal and a forced exit.
const FORCED_EXIT_GRACE: Duration = Duration::from_secs(10);

/// Set while a CLI plan run handles SIGINT/SIGTERM itself.
static CLI_OWNS_TERMINATION_SIGNALS: AtomicBool = AtomicBool::new(false);

/// Original stderr while it is redirected to the runner log for the TUI.
static REDIRECTED_STDERR_ORIGINAL: AtomicI32 = AtomicI32::new(-1);

/// Whether a CLI plan run currently owns SIGINT/SIGTERM. The binary's
/// process-wide SIGTERM handler defers to the run while this is `true`.
#[must_use]
pub fn plan_run_owns_termination_signals() -> bool {
    CLI_OWNS_TERMINATION_SIGNALS.load(Ordering::SeqCst)
}

/// Owns SIGINT/SIGTERM for one CLI plan run; dropping it hands them back.
#[derive(Debug)]
pub struct PlanRunSignalGuard {
    listener: tokio::task::JoinHandle<()>,
}

impl Drop for PlanRunSignalGuard {
    fn drop(&mut self) {
        self.listener.abort();
        CLI_OWNS_TERMINATION_SIGNALS.store(false, Ordering::SeqCst);
    }
}

/// Route SIGINT/SIGTERM to `interrupt` until the returned guard drops.
///
/// The first signal asks the run to stop: the running graph is cancelled,
/// its checkpoint is finalized as `interrupted`, the TUI restores the
/// terminal, and the run returns [`PlanRunInterrupt::exit_code`]. A second
/// signal, or [`FORCED_EXIT_GRACE`] after the first, forces the process out
/// with the same non-zero status, so shutdown can never hang.
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
    CLI_OWNS_TERMINATION_SIGNALS.store(true, Ordering::SeqCst);
    let listener = tokio::spawn(async move {
        loop {
            let received = tokio::select! {
                Some(()) = sigint.recv() => PlanRunInterrupt::Interrupt,
                Some(()) = sigterm.recv() => PlanRunInterrupt::Terminate,
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
/// and exit with the signal's status.
#[cfg(unix)]
fn force_exit(interrupt: PlanRunInterrupt, reason: &str) -> ! {
    crate::tui::app::restore_terminal_for_forced_exit();
    restore_redirected_stderr();
    let killed = signal_processes(
        roko_agent::process::collect_descendants(std::process::id()),
        libc::SIGKILL,
    );
    tracing::error!(
        signal = interrupt.label(),
        reason,
        killed,
        "forced plan run exit; the interrupted plan's checkpoint may still read `running`"
    );
    eprintln!(
        "roko: forced exit after {} ({reason}); killed {killed} child process(es); \
         resume with `roko plan run <plans-dir> --resume-plan`",
        interrupt.label()
    );
    std::process::exit(interrupt.exit_code());
}

/// Agent processes that are still descendants of this process, plus all
/// of their descendants. Checking descendancy avoids signalling a PID that
/// was recycled after its agent exited.
#[cfg(unix)]
fn live_agent_process_trees() -> Vec<u32> {
    let ours = roko_agent::process::collect_descendants(std::process::id())
        .into_iter()
        .collect::<std::collections::HashSet<_>>();
    let mut targets = Vec::new();
    for pid in roko_agent::process::registered_pids() {
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

/// The recorded stop request, or a new `Interrupt` when the operator just
/// closed the TUI mid-run (no terminal output would remain otherwise).
fn pending_interrupt(
    interrupt: &PlanRunInterruptHandle,
    tui: &mut Option<TuiSession>,
) -> Option<PlanRunInterrupt> {
    if interrupt.requested().is_none() && tui.as_mut().is_some_and(TuiSession::closed_by_operator) {
        tracing::warn!("operator closed the TUI mid-run; stopping the plan run");
        interrupt.request(PlanRunInterrupt::Interrupt);
    }
    interrupt.requested()
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
/// when the caller explicitly provided a ceiling via the CLI (so the runner
/// warns on overage instead of hard-blocking).
pub fn resolve_budget_ceiling(
    budget_override: Option<f64>,
    no_budget: bool,
    config_max_plan_usd: f64,
) -> (f64, bool) {
    if no_budget {
        (0.0, true)
    } else if let Some(ceiling) = budget_override {
        (ceiling.max(0.0), true)
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
    pub rich_topology: bool,
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
        super::event_log::evidence_run_id()
            .or_else(|| run_id.clone())
            .unwrap_or_else(|| format!("graph-{}", uuid::Uuid::new_v4())),
    );

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

    let outcome = graph_run_outcome(&result, &interrupt_handle);
    hub_sender.publish(roko_core::DashboardEvent::RunCompleted {
        outcome: outcome.to_string(),
        duration_ms: run_start.elapsed().as_millis() as u64,
        cleanup_degraded: false,
        surviving_agent_ids: vec![],
        surviving_agent_pids: vec![],
    });
    status.finish(outcome).await;

    result
}

/// Map the run result to an outcome label for `DashboardEvent::RunCompleted`.
///
/// - `"succeeded"`: exit code `EXIT_SUCCESS`.
/// - `"cancelled"`: a stop request whose exit code matches the returned code
///   (TUI cancel routes through the same handle via `PlanRunInterrupt`).
/// - `"failed"`: any `Err` and every other non-zero exit code.
fn graph_run_outcome(
    result: &anyhow::Result<i32>,
    interrupt: &PlanRunInterruptHandle,
) -> &'static str {
    match result {
        Ok(code) if *code == EXIT_SUCCESS => "succeeded",
        Ok(code) => {
            if interrupt
                .requested()
                .is_some_and(|req| req.exit_code() == *code)
            {
                "cancelled"
            } else {
                "failed"
            }
        }
        Err(_) => "failed",
    }
}

async fn run_graph_plan_body(
    params: GraphPlanRunParams,
    run_id: Option<String>,
) -> anyhow::Result<i32> {
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
        log_file,
        worktree_per_task,
        rich_topology,
        no_tui,
        state_hub,
        interrupt,
        max_parallel_plans,
        fail_fast,
        only_plans,
        live_agent_output,
    } = params;
    let interrupt = interrupt.unwrap_or_default();
    // FAST lane (`./dev.sh fast`): stop the run when its deadline elapses.
    let _fast_deadline = super::fast_lane::arm_plan_deadline(&interrupt);

    let plans_dir: &Path = &plans_dir;
    let workdir: &Path = &workdir;
    let resume_plan: Option<PathBuf> = resume_plan;
    let log_file: Option<PathBuf> = log_file;
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
    roko_core::config::loader::normalize_and_validate_dispatch_models(&mut roko_config)
        .context("validate model configuration before Graph dispatch")?;

    // Merge CLI flag with config (same logic as runner-v2).
    let dangerously_skip_permissions =
        dangerously_skip_permissions || roko_config.runner.dangerously_skip_permissions;

    // How many independent plans may run at once: the per-run override, else
    // `[conductor] max_parallel_plans`. Never written back into the config,
    // which every checkpoint fingerprint includes.
    let max_parallel_plans = max_parallel_plans
        .unwrap_or(roko_config.conductor.max_parallel_plans)
        .max(1);
    if worktree_per_task && max_parallel_plans > 1 && plans.len() > 1 {
        anyhow::bail!(
            "--worktree-per-task cannot run plans in parallel (max_parallel_plans = \
             {max_parallel_plans}): per-task worktrees are never merged back, so concurrent \
             plans would not see each other's work; run with --max-parallel-plans 1"
        );
    }

    let (plan_budget_ceiling, budget_override_active) = resolve_budget_ceiling(
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
    let run_manifests = super::run_manifest::RunManifests::capture(workdir, &roko_config);
    let prompt_cache = Arc::new(crate::dispatch::PromptCache::load(workdir));
    let mut shared_factory = crate::dispatch::SharedAgentFactory::new(
        Arc::clone(&roko_config),
        roko_config.agent.mcp_config.as_ref(),
        graph_run_config.cascade_router.clone(),
        Some(prompt_cache),
    )
    .await
    .with_health_registry(Arc::new(
        roko_learn::provider_health::ProviderHealthRegistry::load_or_new(
            &RokoLayout::for_project(workdir)
                .learn_dir()
                .join("provider-health.json"),
        ),
    ))
    .with_error_patterns_from_disk(workdir);
    let plugin_catalog = crate::runner::extension_loader::resolve_plugin_tool_catalog(
        workdir,
        &roko_config.agent.extensions,
        &[],
    )?;
    if !plugin_catalog.plugin_tools().is_empty() {
        shared_factory = shared_factory.with_local_tool_runtime(plugin_catalog.local_runtime());
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
    let graph_feedback = build_graph_feedback_context(
        workdir,
        &roko_config,
        graph_run_config.cascade_router.as_ref(),
        shared_factory.error_pattern_store(),
    );
    let holdout_experiment = graph_feedback.holdout_experiment.clone();

    // ── TUI vs inline progress decision ──────────────────────────────
    //
    // Auto-enable the interactive TUI dashboard when stdout is an
    // interactive terminal, unless the user explicitly opted out with
    // --no-tui, --quiet, or --json. This mirrors the runner-v2 approval
    // TUI logic (line ~470).
    let launch_tui = !no_tui && !quiet && !json && std::io::stdout().is_terminal();

    // Keep the full SharedStateHub alive so the TUI can subscribe to the
    // live event stream. Previously this path only extracted sender().
    // Every publisher of this run (plan set, TUI bridges, telemetry, the
    // TUI itself) shares this one hub.
    let state_hub = state_hub.unwrap_or_else(crate::state_hub::shared_state_hub);
    let state_hub_sender = state_hub.sender();
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
        plans: plan_set_entries(&plans, &plan_order, &plan_conflicts),
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
        budget_override_active,
    )
    .with_cli_model_override(cli_model_override)
    .with_dangerously_skip_permissions(dangerously_skip_permissions)
    .with_feedback(graph_feedback)
    .with_reflex_store(reflex_store)
    .with_tui_bridge(dispatcher_tui_bridge)
    .with_live_agent_output(live_agent_output);

    // ── Per-task worktree isolation (opt-in via --worktree-per-task) ──
    let mut workspace_provider: Option<Arc<dyn roko_graph::workspace::ExecutionWorkspaceProvider>> =
        None;
    if worktree_per_task {
        use crate::orchestrator::worktree::{WorktreeConfig, WorktreeManager};
        let worktree_manager = WorktreeManager::new(WorktreeConfig {
            repo_root: workdir.to_path_buf(),
            base_branch: "HEAD".to_string(),
            worktrees_root: workdir.join(".roko").join("worktrees"),
            max_live: None,
            idle_ttl: std::time::Duration::from_hours(1),
        });
        let provider = Arc::new(
            crate::graph_execution::WorktreeExecutionWorkspaceProvider::new(worktree_manager),
        );
        if !quiet && !json {
            tracing::info!("per-task worktree isolation enabled (--worktree-per-task)");
        }
        dispatcher_builder = dispatcher_builder.with_workspace_provider(provider.clone());
        workspace_provider = Some(provider);
    }
    // The same provider settles the worktrees the rich topology's executors
    // hand on to their gates.
    let cell_resources = plan_cell_resources(rich_topology, workspace_provider);

    let graph_task_dispatcher = Arc::new(dispatcher_builder);
    // `[conductor] max_agents` caps concurrently executing tasks across
    // every plan of the run.
    let task_dispatcher: Arc<dyn TaskDispatcher> =
        Arc::new(super::agent_slots::AgentSlotDispatcher::new(
            graph_task_dispatcher.clone(),
            roko_config.conductor.max_agents,
        ));

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

    // Shared pause flag: set/cleared by Pause/Resume commands from the TUI.
    // Wired into each CellContext so the task executor cell can check it
    // between agent turns (cells check this flag between turns; a paused
    // cell waits until the flag is cleared).
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
        // When the CLI routes SIGINT/SIGTERM to this run, keep the TUI's
        // terminal-reset handler off them so they stop the run gracefully
        // instead of killing the process.
        let host_owns_signals = plan_run_owns_termination_signals();
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

    // ── Canonical --log-file recorder for Graph Engine (#115) ──
    let graph_event_logger: Option<Arc<dyn roko_graph::events::GraphEventSink>> =
        match log_file.as_deref() {
            Some(path) => {
                let resolved = if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    workdir.join(path)
                };
                let logger = crate::runner::structured_log::GraphEventLogger::open(&resolved)
                    .map_err(|e| anyhow!("open --log-file {}: {e}", resolved.display()))?;
                Some(Arc::new(logger))
            }
            None => None,
        };

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
    // Set once SIGINT/SIGTERM (or closing the TUI) stops the run; later
    // plans are left unstarted.
    let mut stopped_by: Option<PlanRunInterrupt> = None;

    let run_context = PlanRunContext {
        workdir,
        resume_plan: resume_plan.as_deref(),
        plan_count,
        fresh,
        force_resume,
        max_retries,
        max_tasks,
        rich_topology,
        cell_resources: &cell_resources,
        quiet,
        json,
        launch_tui,
        task_dispatcher: &task_dispatcher,
        graph_task_dispatcher: &graph_task_dispatcher,
        plan_failure_policy: roko_config.conductor.plan_failure_policy,
        graph_tui_bridge: &graph_tui_bridge,
        graph_telemetry: &graph_telemetry,
        graph_event_logger: graph_event_logger.as_ref(),
        shared_pause_flag: &shared_pause_flag,
        interrupt: &interrupt,
        run_manifests: &run_manifests,
        caller_run_id: run_id.as_deref(),
    };
    let mut scheduler = super::plan_set::PlanSetScheduler::new(
        &plan_order,
        plan_conflicts,
        max_parallel_plans,
        fail_fast,
    );
    let mut running = futures::stream::FuturesUnordered::new();
    let mut controls = std::collections::HashMap::<String, PlanControl>::new();
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
        for plan_id in route_execution_commands(
            &mut exec_cmd_rx,
            &tui_ack_tx,
            &controls,
            &mut scheduler,
            &shared_pause_flag,
        ) {
            graph_tui_bridge.log_event(
                "graph.plan_cancelled",
                &format!("plan '{plan_id}' cancelled before it started"),
            );
            graph_tui_bridge.plan_completed(&plan_id, false);
            plan_outcomes.insert(plan_id, false);
            all_succeeded = false;
        }

        let admission = scheduler.admit();
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
            let control = PlanControl::default();
            controls.insert(plan_id, control.clone());
            running.push(run_admitted_plan(&run_context, plan, control));
        }

        if running.is_empty() {
            break;
        }
        tokio::select! {
            Some((plan_id, result)) = running.next() => {
                controls.remove(&plan_id);
                let outcome = match result {
                    Ok(result) => {
                        total_output_count += result.output_count;
                        plan_task_verdicts.insert(plan_id.clone(), result.tasks);
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
                if running.is_empty() {
                    // Clear any residual pause once no plan is running.
                    shared_pause_flag.store(false, Ordering::Release);
                }
            }
            () = tokio::time::sleep(PLAN_WATCH_INTERVAL) => {}
        }
    }
    drop(running);

    // Commands that arrived after the last plan finished.
    while let Ok(cmd) = exec_cmd_rx.try_recv() {
        let ack = ack_for(
            &cmd,
            CommandAckStatus::Accepted,
            Some("plan finished — re-run to apply".into()),
        );
        let _ = tui_ack_tx.try_send(ack);
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
        graph_tui_bridge.log_event(
            "graph.run_interrupted",
            &format!(
                "{}: run stopped, {unstarted} plan(s) not started",
                reason.label()
            ),
        );
    }

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
    // stopped explicitly below once post-run persistence finishes.
    if !all_succeeded {
        state_hub
            .sender()
            .publish(roko_core::DashboardEvent::Error {
                message: format!(
                    "Graph Engine: {plan_count} plan(s), {} succeeded, {} failed",
                    plan_outcomes.values().filter(|v| **v).count(),
                    plan_outcomes.values().filter(|v| !**v).count(),
                ),
            });
    }
    // ── Persist cascade router observations (UX34) ─────────────────
    //
    // Save learned routing state (confidence stats, LinUCB weights, Pareto
    // frontier) so that force_backend override outcomes and all other
    // routing observations survive across runs. Without this, in-memory
    // learning accumulated during plan execution was lost on exit.
    if let Some(cascade) = &graph_run_config.cascade_router {
        let cascade_path = graph_layout.cascade_router_path();
        if let Err(err) = cascade.save(&cascade_path) {
            tracing::warn!(
                path = %cascade_path.display(),
                error = %err,
                "failed to persist cascade router state (non-fatal)"
            );
        }
    }

    // ── Persist holdout experiment state ────────────────────────────
    //
    // Save holdout state so overfitting detection survives across runs
    // and partition assignments remain stable. Mirrors Runner-v2 cleanup.
    if let Some(holdout) = &holdout_experiment
        && let Ok(exp) = holdout.try_lock()
        && let Err(err) = exp.save()
    {
        tracing::warn!(error = %err, "failed to persist holdout experiment state (non-fatal)");
    }

    // ── Persist run metrics (backlog #169) ──────────────────────────
    //
    // Collect task counts and cost from the just-completed plan loop and
    // append a structured RunMetricsRecord to `.roko/learn/run-metrics.jsonl`.
    // Each task counts under its own verdict (bug-7eb27e), not its plan's.
    // The write is fire-and-forget on a background task so it never blocks
    // the TUI exit path.
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
        tokio::spawn(async move {
            if let Err(err) = roko_learn::run_metrics::append_run_metrics(&metrics_path, &record) {
                tracing::warn!(error = %err, "failed to persist run metrics (non-fatal)");
            }
        });
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
                "Graph Engine interrupted by {}: {}/{} plan(s) attempted, {} task(s), ${:.2}; \
                 resume with --resume-plan",
                reason.label(),
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
    }

    Ok(match stopped_by {
        Some(reason) => reason.exit_code(),
        None if all_succeeded => EXIT_SUCCESS,
        None => EXIT_FAILURE,
    })
}

// ── Learning and feedback wiring ──────────────────────────────────────────

/// The learning and feedback wiring of a Graph plan run under `workdir`: the
/// feedback facade ([`build_graph_feedback_facade`]) and every learning store
/// a task attempt's feedback writes. `error_patterns` is the dispatch
/// factory's error-pattern store, which prompts read.
///
/// It builds the same feedback infrastructure that Runner-v2 used, so Graph
/// engine runs produce episodes, efficiency events, playbook outcomes,
/// routing observations, experiment settlements, and daimon feedback.
/// [`run_graph_plan`] wires its dispatcher with it, and the learning wiring
/// census (`tests/learning_wiring_census.rs`, S01 §4.8) inspects the same
/// object graph through `GraphTaskDispatcher::wiring_report`.
pub fn build_graph_feedback_context(
    workdir: &Path,
    config: &roko_core::config::schema::RokoConfig,
    cascade_router: Option<&Arc<roko_learn::cascade_router::CascadeRouter>>,
    error_patterns: &Arc<std::sync::RwLock<roko_learn::error_pattern_store::ErrorPatternStore>>,
) -> crate::graph_task_dispatch::GraphFeedbackContext {
    let graph_layout = RokoLayout::for_project(workdir);
    let graph_learn_dir = graph_layout.learn_dir();
    let _ = std::fs::create_dir_all(&graph_learn_dir);

    // #144: one daimon state, shared by the feedback facade (plan-completion
    // persistence) and dispatch (affect modulation).
    let shared_daimon_state = graph_daimon_state(workdir, config);

    // ── P0-04: CodingOracle ─────────────────────────────────────────────
    //
    // Persists across the plan run, accumulating build/test observations
    // for predictive gate feedback. Mirrors Runner-v2's CodingOracle.
    let coding_oracle = std::sync::Arc::new(roko_learn::oracles::coding::CodingOracle::new());

    // ── P1-01: GateGamingDetector ────────────────────────────────────
    //
    // Flags when agents game the gate system by passing gates at an
    // increasing rate while delivering lower-quality outputs. Alerts are
    // appended to a JSONL file on disk.
    let gate_gaming_detector = std::sync::Arc::new(tokio::sync::Mutex::new(
        roko_learn::GateGamingDetector::new(graph_learn_dir.join("gate-gaming-alerts.jsonl")),
    ));

    // ── P1-04: HoldoutExperiment ─────────────────────────────────────
    //
    // Deterministic 80/20 train/holdout split for detecting overfitting
    // in learned routing. Learning updates are gated behind the holdout
    // partition check.
    let holdout_experiment = std::sync::Arc::new(tokio::sync::Mutex::new(
        roko_learn::HoldoutExperiment::load_or_new(
            graph_learn_dir.join("holdout-state.json"),
        )
        .unwrap_or_else(|err| {
            tracing::warn!(error = %err, "failed to load holdout experiment state; starting fresh");
            roko_learn::HoldoutExperiment::new(
                graph_learn_dir.join("holdout-state.json"),
            )
        }),
    ));

    // ── P2-01: ShadowRunner ─────────────────────────────────────────
    //
    // Records shadow dispatch decisions (infrastructure-only; no actual
    // shadow task spawn). Uses the configured default model as the
    // shadow alternative.
    let shadow_runner = std::sync::Arc::new(roko_learn::shadow::ShadowRunner::new(
        roko_learn::shadow::ShadowConfig {
            model_slug: config.agent.default_model.clone(),
            prompt_variant: None,
            label: "graph-shadow".to_string(),
        },
        graph_learn_dir.join("shadow-results.jsonl"),
    ));

    crate::graph_task_dispatch::GraphFeedbackContext {
        feedback_facade: Some(build_graph_feedback_facade(
            workdir,
            config,
            cascade_router,
            shared_daimon_state.as_ref(),
            error_patterns,
        )),
        efficiency_path: Some(graph_learn_dir.join("efficiency.jsonl")),
        costs_path: Some(graph_learn_dir.join("costs.jsonl")),
        playbook_dir: Some(graph_learn_dir.join("playbooks")),
        // Reuse the daimon state constructed above so the feedback facade
        // persistence sink and dispatch-time modulation share the same
        // mutable state (#144).
        daimon_state: shared_daimon_state,
        experiment_store_path: Some(graph_learn_dir.join("experiments.json")),
        gate_failures_path: Some(graph_layout.gate_failures_path()),
        post_gate_reflection_path: Some(graph_learn_dir.join("post-gate-reflections.json")),
        replan_on_gate_failure: config.learning.replan_on_gate_failure,
        coding_oracle: Some(coding_oracle),
        gate_gaming_detector: Some(gate_gaming_detector),
        holdout_experiment: Some(holdout_experiment),
        shadow_runner: Some(shadow_runner),
        eval_generation_enabled: true,
        // P2-LRN-6 Loop 1: Gate threshold EMA updates after each task's
        // verify sequence. Uses the canonical workspace path so the TUI,
        // serve, and `roko learn gates` all read from the same file.
        gate_thresholds_path: Some(graph_layout.gate_thresholds_path()),
        // RAG-10: retrieval outcome JSONL for gate-pass correlation telemetry.
        retrieval_outcomes_path: Some(graph_learn_dir.join("retrieval-outcomes.jsonl")),
        // S01: every attempt's open line and verdict, per checkpoint run.
        runs_dir: Some(graph_layout.runs_dir()),
    }
}

/// Where a workspace's daimon affect state persists.
fn daimon_affect_path(workdir: &Path) -> PathBuf {
    workdir.join(".roko").join("daimon").join("affect.json")
}

/// #144: the daimon state a plan run shares between the feedback facade and
/// dispatch; `None` unless `[daimon] strategy_space.dimensions` has exactly
/// 8 entries.
fn graph_daimon_state(
    workdir: &Path,
    config: &roko_core::config::schema::RokoConfig,
) -> Option<Arc<std::sync::Mutex<roko_daimon::DaimonState>>> {
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
/// error patterns, routing when there is a cascade router, and the
/// plan-completion dream, daimon, theta and delta sinks). `daimon_state` is
/// the state dispatch modulates, persisted when a plan completes;
/// `error_patterns` is the store dispatch formats into prompts.
pub fn build_graph_feedback_facade(
    workdir: &Path,
    config: &roko_core::config::schema::RokoConfig,
    cascade_router: Option<&Arc<roko_learn::cascade_router::CascadeRouter>>,
    daimon_state: Option<&Arc<std::sync::Mutex<roko_daimon::DaimonState>>>,
    error_patterns: &Arc<std::sync::RwLock<roko_learn::error_pattern_store::ErrorPatternStore>>,
) -> Arc<crate::runtime_feedback::FeedbackFacade> {
    let graph_layout = RokoLayout::for_project(workdir);
    let graph_learn_dir = graph_layout.learn_dir();
    let graph_episodes_path = graph_layout.root_episodes_path();
    let mut facade = crate::runtime_feedback::FeedbackFacade::new()
        .with_sink(std::sync::Arc::new(
            crate::runtime_feedback::EpisodeSink::at(&graph_episodes_path),
        ))
        // Reads back the failed episode the episode sink just wrote, so
        // it must follow it.
        .with_sink(std::sync::Arc::new(
            crate::runtime_feedback::HindsightSink::new(
                &graph_episodes_path,
                graph_learn_dir.join(roko_learn::hindsight::DEFAULT_ADJUSTMENTS_FILE),
            ),
        ))
        // Gate-verified attempts grow durable knowledge (tier
        // progression included) under `.roko/neuro/`.
        .with_sink(std::sync::Arc::new(
            crate::runtime_feedback::VerifiedKnowledgeSink::for_workdir(workdir),
        ))
        // A failure of the agent's work goes into the error-pattern store
        // dispatch formats into prompts, and to `learn/error-patterns.json`.
        .with_sink(std::sync::Arc::new(
            crate::runtime_feedback::ErrorPatternSink::new(
                std::sync::Arc::clone(error_patterns),
                graph_learn_dir.join("error-patterns.json"),
            ),
        ));
    if let Some(cascade) = cascade_router {
        facade = facade.with_sink(std::sync::Arc::new(
            crate::runtime_feedback::RoutingObservationSink::new(cascade.clone()),
        ));
    }

    // ── #143: Dream consolidation trigger on plan completion ────────
    facade = facade.with_sink(std::sync::Arc::new(
        crate::runtime_feedback::DreamConsolidationSink::new(
            workdir.to_path_buf(),
            config.learning.dream_on_completion,
            config.learning.dreams.trigger_on_plan_complete,
        ),
    ));

    // ── #144: Daimon affect persistence on plan completion ──────────
    if let Some(daimon) = daimon_state {
        facade = facade.with_sink(std::sync::Arc::new(
            crate::runtime_feedback::DaimonPersistenceSink::new(
                daimon_affect_path(workdir),
                std::sync::Arc::clone(daimon),
            ),
        ));
    }

    // ── Theta reflection on plan completion ─────────────────────────
    //
    // Runs a five-phase reflective cycle (gamma summary, affect update,
    // calibration check, progress assessment, meta-cognition) after each
    // plan completes. Lightweight and synchronous (no LLM calls).
    let shared_cortical = std::sync::Arc::new(roko_runtime::heartbeat::CorticalState::default());
    let shared_theta = std::sync::Arc::new(std::sync::Mutex::new(
        roko_runtime::theta_consumer::ThetaConsumer::default(),
    ));
    facade = facade.with_sink(std::sync::Arc::new(
        crate::runtime_feedback::ThetaReflectionSink::new(
            std::sync::Arc::clone(&shared_theta),
            std::sync::Arc::clone(&shared_cortical),
        ),
    ));

    // ── Delta consolidation on plan completion ──────────────────────
    //
    // Tracks episode counts and checks trigger conditions for a dream
    // consolidation cycle (NREM replay, REM imagination, integration).
    // Bridges roko_runtime::delta_consumer into the feedback pipeline.
    let shared_delta = std::sync::Arc::new(std::sync::Mutex::new(
        roko_runtime::delta_consumer::DeltaConsumer::default(),
    ));
    facade = facade.with_sink(std::sync::Arc::new(
        crate::runtime_feedback::DeltaConsolidationSink::new(
            std::sync::Arc::clone(&shared_delta),
            std::sync::Arc::clone(&shared_cortical),
        ),
    ));

    std::sync::Arc::new(facade)
}

// ── Running the plans of a set ────────────────────────────────────────────

/// How often the plan-set driver and each running plan poll for stop
/// requests and operator commands.
const PLAN_WATCH_INTERVAL: Duration = Duration::from_millis(100);

/// Run-wide state every plan of the set runs with.
struct PlanRunContext<'a> {
    workdir: &'a Path,
    resume_plan: Option<&'a Path>,
    plan_count: usize,
    fresh: bool,
    force_resume: bool,
    max_retries: Option<u32>,
    max_tasks: usize,
    rich_topology: bool,
    /// Services the cells of each plan's graph run with (see
    /// [`plan_cell_resources`]).
    cell_resources: &'a roko_graph::cell::CellResources,
    quiet: bool,
    json: bool,
    launch_tui: bool,
    task_dispatcher: &'a Arc<dyn roko_graph::cells::TaskDispatcher>,
    graph_task_dispatcher: &'a Arc<crate::graph_task_dispatch::GraphTaskDispatcher>,
    /// `[conductor] plan_failure_policy`; a plan's `[meta] failure_policy`
    /// overrides it.
    plan_failure_policy: roko_core::config::PlanFailurePolicy,
    graph_tui_bridge: &'a crate::runner::graph_tui_bridge::GraphTuiBridge,
    graph_telemetry: &'a Arc<dyn roko_core::TelemetryEventSink>,
    graph_event_logger: Option<&'a Arc<dyn roko_graph::events::GraphEventSink>>,
    shared_pause_flag: &'a Arc<AtomicBool>,
    interrupt: &'a PlanRunInterruptHandle,
    /// Each checkpoint run's `manifest.json` (S01 §5.1).
    run_manifests: &'a super::run_manifest::RunManifests,
    /// The run id the caller already gave this run (`roko run`); a single
    /// plan's fresh checkpoint takes it.
    caller_run_id: Option<&'a str>,
}

/// Services the cells of a plan's graph run with (gap-6daad9). The rich
/// topology's `plan.gate` cells run the gates, and settle the worktree each
/// task executor hands on through `workspaces`, the provider the executors
/// acquire them from. The default topology needs neither.
fn plan_cell_resources(
    rich_topology: bool,
    workspaces: Option<Arc<dyn roko_graph::workspace::ExecutionWorkspaceProvider>>,
) -> roko_graph::cell::CellResources {
    if !rich_topology {
        return roko_graph::cell::CellResources::default();
    }
    roko_graph::cell::CellResources {
        gates: Some(Arc::new(crate::runner::gate_adapter::default_gate_adapter())),
        workspaces,
    }
}

/// The context a plan's graph runs in: its checkpoint run, the run's shared
/// pause flag, and the cell services.
fn plan_cell_context(
    run_id: &str,
    pause_flag: &Arc<AtomicBool>,
    resources: &roko_graph::cell::CellResources,
) -> roko_graph::cell::CellContext {
    roko_graph::cell::CellContext::new()
        .with_run_id(run_id.to_string())
        .with_pause_flag(Arc::clone(pause_flag))
        .with_resources(resources.clone())
}

/// Close checkpoint run `run_id`'s attempt log, then record in its manifest
/// that it ended with `status` and how many attempts it opened and settled.
fn close_run_manifest(ctx: &PlanRunContext<'_>, run_id: &str, status: GraphCheckpointStatus) {
    let writer = ctx.graph_task_dispatcher.close_run_attempts(run_id);
    ctx.run_manifests.close(run_id, status, writer);
}

/// Operator controls the plan-set driver routes to one running plan.
#[derive(Debug, Clone, Default)]
struct PlanControl {
    /// Set by a TUI cancel aimed at this plan, or at every running plan.
    cancel: Arc<AtomicBool>,
}

/// How one plan's run ended.
struct PlanRunResult {
    outcome: PlanOutcome,
    output_count: usize,
    /// How the plan's tasks settled.
    tasks: TaskVerdictCounts,
}

impl PlanRunResult {
    /// A plan that failed before any of its `task_count` tasks ran.
    const fn failed(task_count: usize) -> Self {
        Self {
            outcome: PlanOutcome::Failed,
            output_count: 0,
            tasks: TaskVerdictCounts::not_run(task_count),
        }
    }
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

/// Drain pending TUI commands.
///
/// Cancel reaches the plan it names when that plan is running, drops it
/// when it has not started, and reaches every running plan when it names
/// none. Pause and resume set the pause flag every plan shares. Other
/// commands are acknowledged and take effect only after the run. Returns
/// the plans cancelled before they started.
fn route_execution_commands(
    commands: &mut tokio::sync::mpsc::Receiver<crate::execution_control::ExecutionCommand>,
    acks: &tokio::sync::mpsc::Sender<crate::execution_control::CommandAck>,
    controls: &std::collections::HashMap<String, PlanControl>,
    scheduler: &mut PlanSetScheduler,
    pause: &AtomicBool,
) -> Vec<String> {
    let mut cancelled_before_start = Vec::new();
    while let Ok(cmd) = commands.try_recv() {
        let (status, note) = match &cmd.kind {
            ExecutionCommandKind::Cancel => match cmd.plan_id.as_deref() {
                Some(plan_id) => {
                    if let Some(control) = controls.get(plan_id) {
                        control.cancel.store(true, Ordering::Release);
                        (CommandAckStatus::Completed, None)
                    } else if scheduler.cancel_pending(plan_id) {
                        cancelled_before_start.push(plan_id.to_string());
                        (CommandAckStatus::Completed, None)
                    } else {
                        (
                            CommandAckStatus::Accepted,
                            Some(format!("plan '{plan_id}' is not running")),
                        )
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
                tracing::info!(
                    command_id = %cmd.command_id,
                    "TUI pause: execution paused after current task"
                );
                (CommandAckStatus::Completed, None)
            }
            ExecutionCommandKind::Resume => {
                pause.store(false, Ordering::Release);
                tracing::info!(command_id = %cmd.command_id, "TUI resume: execution resumed");
                (CommandAckStatus::Completed, None)
            }
            // Post-execution commands: ack as accepted; the TUI can re-send
            // after plan completion.
            ExecutionCommandKind::SoftRetry
            | ExecutionCommandKind::Repair { .. }
            | ExecutionCommandKind::ReverifyGates
            | ExecutionCommandKind::Skip
            | ExecutionCommandKind::Approve { .. }
            | ExecutionCommandKind::RejectApproval { .. }
            | ExecutionCommandKind::Reset => {
                tracing::debug!(
                    command_id = %cmd.command_id,
                    kind = %cmd.kind,
                    "TUI command queued (post-execution; plan still running)"
                );
                (CommandAckStatus::Accepted, None)
            }
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
    cancelled_before_start
}

/// [`run_one_plan`], tagged with the plan's ID for the plan-set driver.
async fn run_admitted_plan(
    ctx: &PlanRunContext<'_>,
    plan: &crate::runner::plan_loader::Plan,
    control: PlanControl,
) -> (String, anyhow::Result<PlanRunResult>) {
    (plan.id.clone(), run_one_plan(ctx, plan, &control).await)
}

/// Run one admitted plan to a terminal checkpoint.
///
/// A plan that cannot be converted or validated still gets a terminal
/// PlanCompleted. `Err` is reserved for checkpoint and budget-ledger
/// failures, which stop the whole run.
async fn run_one_plan(
    ctx: &PlanRunContext<'_>,
    plan: &crate::runner::plan_loader::Plan,
    control: &PlanControl,
) -> anyhow::Result<PlanRunResult> {
    use roko_graph::cells::TaskExecutorCell;
    use roko_graph::convert::{PlanTaskInfo, plan_to_graph};
    use roko_graph::engine::GraphEngine;

    let graph_tui_bridge = ctx.graph_tui_bridge;
    if !ctx.quiet && !ctx.json && !ctx.launch_tui {
        tracing::info!(
            plan_id = %plan.id,
            task_count = plan.tasks.tasks.len(),
            "running plan via Graph Engine"
        );
    }

    // Convert Runner v2 tasks into PlanTaskInfo for the converter. Each
    // task's retry budget is `--max-retries`, else what it authors, else set
    // by the adaptive gate thresholds.
    let retry_budgets = ctx.graph_task_dispatcher.task_retry_budgets(&plan.dir);
    let tasks: Vec<(String, PlanTaskInfo)> = plan
        .tasks
        .tasks
        .iter()
        .map(|t| {
            let info = PlanTaskInfo {
                title: t.title.clone(),
                description: t.description.clone(),
                role: t.role.clone(),
                tier: t.tier.clone(),
                model_hint: t.model_hint.clone(),
                files: t.files.clone(),
                depends_on: t.depends_on.clone(),
                depends_on_plan: t.depends_on_plan.clone(),
                timeout_secs: t.timeout_secs,
                max_retries: ctx
                    .max_retries
                    .unwrap_or_else(|| retry_budgets.max_retries(&plan.id, t)),
                domain: t.domain.as_ref().map(|d| format!("{d:?}")),
                sequence: t.sequence,
                full_config_json: serde_json::to_value(t).unwrap_or_default(),
            };
            (t.id.clone(), info)
        })
        .collect();

    let max_parallel = if ctx.max_tasks > 0 {
        u32::try_from(ctx.max_tasks).unwrap_or(u32::MAX)
    } else {
        plan.tasks.meta.max_parallel
    };
    let max_parallel_usize = usize::try_from(max_parallel.max(1)).unwrap_or(usize::MAX);
    let plan_dir_str = plan.dir.display().to_string();

    let (mut graph, registry) = if ctx.rich_topology {
        // ── Rich 11-node-per-task production topology ──────────────────
        // Warn: enricher cells are currently PassthroughCell stubs and do
        // not yet add runtime value. The richer topology is available for
        // incremental implementation of each enricher cell type.
        if !ctx.quiet && !ctx.json {
            tracing::info!(
                "--rich-topology is active; enricher cells (knowledge, \
                 episodes, playbook, modulation, safety, experiment) are \
                 currently passthrough stubs"
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
                    Box::new(TaskExecutorCell::live(config, Arc::clone(&plan_dispatcher)))
                });
                (g, reg)
            }
            Err(e) => {
                graph_tui_bridge.error(&format!(
                    "failed to build rich topology for plan '{}': {e}",
                    plan.id
                ));
                tracing::error!(plan_id = %plan.id, error = %e, "failed to build rich topology for plan");
                graph_tui_bridge.plan_completed(&plan.id, false);
                return Ok(PlanRunResult::failed(tasks.len()));
            }
        }
    } else {
        // ── Simple single-Activity-per-task converter (default) ─────────
        match plan_to_graph(&plan.id, &plan_dir_str, &tasks, max_parallel) {
            Ok(g) => {
                let mut reg = roko_graph::default_registry();
                let plan_dispatcher = Arc::clone(ctx.task_dispatcher);
                reg.register("task-executor", move |config| {
                    Box::new(TaskExecutorCell::live(config, Arc::clone(&plan_dispatcher)))
                });
                (g, reg)
            }
            Err(e) => {
                graph_tui_bridge.error(&format!(
                    "failed to convert plan '{}' to graph: {e}",
                    plan.id
                ));
                tracing::error!(plan_id = %plan.id, error = %e, "failed to convert plan to graph");
                graph_tui_bridge.plan_completed(&plan.id, false);
                return Ok(PlanRunResult::failed(tasks.len()));
            }
        }
    };
    let mut checkpoint = crate::graph_checkpoint::prepare_graph_checkpoint_for_run(
        ctx.workdir,
        ctx.resume_plan,
        &plan.id,
        ctx.plan_count,
        &graph,
        ctx.fresh,
        ctx.force_resume,
        ctx.caller_run_id.filter(|_| ctx.plan_count == 1),
    )?;
    let run_id = checkpoint.run_id().to_string();
    // A new run's manifest, or one more invocation of a resumed run; the
    // run's attempt records carry the invocation's ordinal.
    if let Some(inv) = ctx.run_manifests.open(&run_id, &plan.id) {
        ctx.graph_task_dispatcher
            .attach_run_invocation(&run_id, inv);
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
    // Wire canonical --log-file recorder (#115): attach the event sink
    // so every GraphExecutionEvent is written to JSONL.
    if let Some(sink) = ctx.graph_event_logger {
        engine = engine.with_event_sink(Arc::clone(sink));
    }
    if let Some(replayer) = checkpoint.take_replayer() {
        engine = engine.with_replayer(replayer);
    }
    // P2-TUI-3: Wire the shared pause flag into CellContext so cells can
    // check it between turns and yield when the TUI sends Pause.
    let cell_ctx = plan_cell_context(&run_id, ctx.shared_pause_flag, ctx.cell_resources);

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
        graph_tui_bridge.plan_completed(&plan.id, false);
        close_run_manifest(ctx, &run_id, GraphCheckpointStatus::Failed);
        checkpoint.finish(false)?;
        return Ok(PlanRunResult::failed(tasks.len()));
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
    // resume act through the shared pause flag, which cells check between
    // turns.
    loop {
        if !flow_handle.is_running() {
            break;
        }
        // Interrupt: cancel the graph, ask in-flight agents to stop so
        // their nodes settle, then give up on the graph after
        // INTERRUPT_DRAIN_TIMEOUT so the checkpoint is still finalized.
        if let Some(deadline) = drain_deadline {
            if Instant::now() >= deadline {
                // Agents that ignored SIGTERM must not outlive the run.
                let killed = kill_in_flight_agents();
                tracing::warn!(
                    plan_id = %plan.id,
                    killed,
                    "cancelled graph did not settle in time; finalizing checkpoint without it"
                );
                flow_abandoned = true;
                break;
            }
        } else if let Some(reason) = ctx.interrupt.requested() {
            flow_handle.cancel();
            let signalled = terminate_in_flight_agents();
            tracing::warn!(
                plan_id = %plan.id,
                signal = reason.label(),
                signalled,
                "plan run interrupted: cancelling graph"
            );
            graph_tui_bridge.log_event(
                "graph.run_interrupted",
                &format!(
                    "{}: cancelling plan '{}' and stopping the run",
                    reason.label(),
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
        // rich topology adds helper nodes absent from `node_titles`).
        let current_statuses: HashMap<String, roko_graph::engine::NodeStatus> = flow_handle
            .status()
            .node_statuses
            .into_iter()
            .filter(|(id, _)| node_titles.contains_key(id))
            .collect();
        graph_tui_bridge.poll_status_changes(
            &plan.id,
            &previous_statuses,
            &current_statuses,
            &node_titles,
        );
        previous_statuses = current_statuses;
    }

    // Collect the final result from the background task.
    let flow_result = if flow_abandoned {
        None
    } else {
        flow_handle.await_completion().await
    };

    let Some(output) = flow_result else {
        // Flow was cancelled before producing a result (e.g. validation
        // failure inside start(), the task panicked, or an interrupted
        // graph did not settle in time).
        let cancelled_msg = if let Some(reason) = interrupted_by {
            format!("plan '{}' interrupted by {}", plan.id, reason.label())
        } else if was_cancelled_by_tui {
            format!("plan '{}' cancelled by user", plan.id)
        } else {
            format!("plan '{}' execution failed (no output)", plan.id)
        };
        graph_tui_bridge.error(&cancelled_msg);
        graph_tui_bridge.plan_completed(&plan.id, false);

        tracing::error!(plan_id = %plan.id, "plan execution failed: no output");
        // Without the graph's result no task's verdict is known, so none
        // counts as passed.
        let task_verdicts = TaskVerdictCounts::not_run(tasks.len());
        let outcome = plan_outcome(
            false,
            task_verdicts,
            interrupted_by.is_some(),
            was_cancelled_by_tui,
        );
        checkpoint.record_task_outcomes(&TaskOutcomeSummary::default())?;
        close_run_manifest(ctx, &run_id, plan_checkpoint_status(outcome));
        checkpoint.finish_with_status(plan_checkpoint_status(outcome))?;
        return Ok(PlanRunResult {
            outcome,
            output_count: 0,
            tasks: task_verdicts,
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

    // ── Graph TUI bridge: final status diff + PlanCompleted ──
    // Emit any transitions (Running→Complete/Failed/Skipped, or
    // Pending→Running for tasks that started and finished between ticks)
    // that the polling loop did not yet publish, then close the plan.
    let final_statuses: HashMap<String, roko_graph::engine::NodeStatus> = output
        .node_results
        .iter()
        .filter(|r| node_titles.contains_key(&r.node_id))
        .map(|r| (r.node_id.clone(), r.status))
        .collect();
    graph_tui_bridge.poll_status_changes(
        &plan.id,
        &previous_statuses,
        &final_statuses,
        &node_titles,
    );
    // Say why each task that did not run was held back; a resume runs them
    // and the failed tasks again.
    let task_outcomes = task_outcomes(&output, &node_titles);
    for (task_id, blocker) in &task_outcomes.blocked_by {
        graph_tui_bridge.log_event(
            "graph.task_blocked",
            &format!(
                "plan '{}': task '{task_id}' blocked by failed task '{blocker}'",
                plan.id
            ),
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
    }
    checkpoint.record_task_outcomes(&task_outcomes)?;
    if outcome == PlanOutcome::Unverified {
        graph_tui_bridge.log_event(
            "graph.plan_unverified",
            &format!(
                "plan '{}': {} task(s) ran no verify step, so the plan did not succeed",
                plan.id, task_verdicts.unverified
            ),
        );
    }
    graph_tui_bridge.plan_completed(&plan.id, outcome.succeeded());

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
    close_run_manifest(ctx, &run_id, plan_checkpoint_status(outcome));
    checkpoint.finish_with_status(plan_checkpoint_status(outcome))?;
    Ok(PlanRunResult {
        outcome,
        output_count,
        tasks: task_verdicts,
    })
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

/// Graph cell type that runs a plan task: one node per task in either
/// topology.
const TASK_EXECUTOR_CELL_TYPE: &str = "task-executor";

/// How a plan's tasks settled in one run, each counted by its node status
/// and gate verdict (epic spec-e9d7ec).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct TaskVerdictCounts {
    /// Completed with a `passed` gate verdict: every verify step passed.
    passed: usize,
    /// Completed without a verify step running: the task declares none, its
    /// role is disabled, or its output carries no gate verdict.
    unverified: usize,
    /// Never ran: blocked by a failed task, or not started.
    skipped: usize,
    /// Failed, or completed over a failed verify step (`forced_accept`).
    failed: usize,
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
                (NodeStatus::Complete, Some(TaskGateVerdict::Passed)) => counts.passed += 1,
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
            unverified: 0,
            skipped: task_count,
            failed: 0,
        }
    }

    /// Outcome of a plan whose graph ran to completion (gap-29a84b): it
    /// succeeded only when every task passed its verify steps, and is
    /// unverified when the rest passed but some ran no verify step. Anything
    /// else failed.
    const fn outcome(self) -> PlanOutcome {
        if self.failed > 0 || self.skipped > 0 {
            PlanOutcome::Failed
        } else if self.unverified > 0 {
            PlanOutcome::Unverified
        } else {
            PlanOutcome::Succeeded
        }
    }
}

/// The run-metrics row of one plan: whether it succeeded, and its tasks
/// counted by verdict. The caller adds the plan's checkpoint run.
fn plan_metrics(
    plan_id: &str,
    succeeded: bool,
    tasks: TaskVerdictCounts,
) -> roko_learn::run_metrics::PlanMetrics {
    roko_learn::run_metrics::PlanMetrics {
        plan_id: plan_id.to_string(),
        completed: succeeded,
        tasks_completed: tasks.passed,
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
            prd_excerpt: String::new(),
        }
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

        let entries = plan_set_entries(&plans, &order, &PlanConflicts::new());

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

    /// Workspace config whose only role in use is disabled, so the task
    /// completes without dispatching a provider.
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
        let plan_dir = dir.path().join("plans").join("01-skipped");
        std::fs::create_dir_all(&plan_dir).expect("plan dir");
        std::fs::write(
            plan_dir.join("tasks.toml"),
            r#"[meta]
plan = "01-skipped"
max_parallel = 1
skip_enrichment = true

[[task]]
id = "T1"
title = "Disabled-role task"
description = "Completes without dispatch because its role is disabled."
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
            rich_topology: false,
            no_tui: true,
            state_hub: Some(hub.clone()),
            interrupt: None,
            max_parallel_plans: None,
            fail_fast: false,
            only_plans: None,
            live_agent_output: crate::graph_task_dispatch::LiveAgentOutput::ToolSteps,
        })
        .await
        .expect("run plan set");

        let snapshot = hub.current_snapshot();
        let plan_set = snapshot
            .plan_set
            .as_ref()
            .expect("PlanSetLoaded reached the hub");
        assert_eq!(plan_set.plans.len(), 1);
        assert_eq!(plan_set.plans[0].plan_id, "01-skipped");
        // PlanStarted/PlanCompleted landed in the same hub.
        assert!(snapshot.plan_set_complete());
        // gap-29a84b: the disabled-role task ran no verify step, so the plan
        // is unverified, not succeeded.
        assert_ne!(snapshot.plans["01-skipped"].phase, "completed");
        assert_eq!(exit_code, EXIT_FAILURE);
        assert_eq!(
            crate::graph_checkpoint::canonical_checkpoint_status(dir.path(), "01-skipped"),
            Some(GraphCheckpointStatus::Unverified)
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
        assert!(read.is_live(), "{read:?}");
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
description = "Completes without dispatch because its role is disabled."
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
        run_plan_set_with(dir, max_parallel_plans, interrupt, true).await
    }

    /// [`run_plan_set`], enforcing the workspace's `[budget]` unless
    /// `no_budget`.
    async fn run_plan_set_with(
        dir: &Path,
        max_parallel_plans: Option<usize>,
        interrupt: Option<PlanRunInterruptHandle>,
        no_budget: bool,
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
            rich_topology: false,
            no_tui: true,
            state_hub: Some(hub.clone()),
            interrupt,
            max_parallel_plans,
            fail_fast: false,
            only_plans: None,
            live_agent_output: crate::graph_task_dispatch::LiveAgentOutput::ToolSteps,
        })
        .await
        .expect("run plan set");
        let lifecycle = hub
            .subscribe_events_from(0)
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
            .collect();
        (exit_code, lifecycle, hub)
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
    /// every task to it, followed by `extra_config`. The provider ignores its
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

        let (exit_code, _, _) = run_plan_set_with(dir.path(), Some(1), None, false).await;

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

    #[test]
    fn interrupt_exit_codes_follow_shell_convention() {
        assert_eq!(PlanRunInterrupt::Interrupt.exit_code(), 130);
        assert_eq!(PlanRunInterrupt::Terminate.exit_code(), 143);
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
            unverified,
            skipped,
            failed,
        };

        assert_eq!(counts(3, 0, 0, 0).outcome(), PlanOutcome::Succeeded);
        assert_eq!(counts(2, 1, 0, 0).outcome(), PlanOutcome::Unverified);
        assert_eq!(counts(0, 1, 0, 0).outcome(), PlanOutcome::Unverified);
        assert_eq!(counts(2, 1, 1, 0).outcome(), PlanOutcome::Failed);
        assert_eq!(counts(2, 1, 0, 1).outcome(), PlanOutcome::Failed);
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
                // A helper node of the rich topology is not a task.
                node("task.T1.gate", "passthrough", NodeStatus::Complete),
            ],
            total_duration: Duration::ZERO,
            gate_verdicts: BTreeMap::from([
                ("T1".to_string(), TaskGateVerdict::Passed),
                ("T2".to_string(), TaskGateVerdict::Unverified),
                ("task.T1.gate".to_string(), TaskGateVerdict::Passed),
            ]),
        };

        let metrics = plan_metrics("verdicts", false, TaskVerdictCounts::of(&output));

        assert!(!metrics.completed);
        let counts = [
            metrics.tasks_completed,
            metrics.tasks_unverified,
            metrics.tasks_skipped,
            metrics.tasks_failed,
        ];
        assert_eq!(counts, [1, 1, 1, 1], "passed, unverified, skipped, failed");
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
            Some(PlanRunInterrupt::Interrupt)
        );
        assert_eq!(interrupt.requested(), Some(PlanRunInterrupt::Interrupt));
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

        let resources = plan_cell_resources(true, Some(provider.clone()));
        assert!(resources.gates.is_some() && resources.workspaces.is_some());
        let default_topology = plan_cell_resources(false, Some(provider.clone()));
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
            .execute(&plan_cell_context("rich-run", &pause, &resources))
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
            rich_topology: true,
            no_tui: true,
            state_hub: None,
            interrupt: None,
            max_parallel_plans: None,
            fail_fast: false,
            only_plans: None,
            live_agent_output: crate::graph_task_dispatch::LiveAgentOutput::ToolSteps,
        })
        .await
        .expect_err("the rich topology needs per-task worktrees");

        assert!(error.to_string().contains("--worktree-per-task"), "{error}");
    }
}
