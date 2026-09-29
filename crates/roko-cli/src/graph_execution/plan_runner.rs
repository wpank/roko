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
use crate::graph_checkpoint::GraphCheckpointStatus;

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

/// Terminal checkpoint status for one plan.
fn plan_checkpoint_status(
    succeeded: bool,
    interrupted: bool,
    cancelled_by_operator: bool,
) -> GraphCheckpointStatus {
    if succeeded {
        GraphCheckpointStatus::Succeeded
    } else if interrupted {
        GraphCheckpointStatus::Interrupted
    } else if cancelled_by_operator {
        GraphCheckpointStatus::Cancelled
    } else {
        GraphCheckpointStatus::Failed
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
}

/// Execute plans via the Graph Engine path.
///
/// Loads plans using the Runner v2 plan_loader, converts each to a Graph
/// via `roko_graph::convert::plan_to_graph` (default) or
/// `roko_graph::topology::ProductionPlanTopology` (when `rich_topology` is
/// true), and runs them through the GraphEngine with the default cell registry.
pub async fn run_graph_plan(params: GraphPlanRunParams) -> anyhow::Result<i32> {
    // `--log-file`: delegates entirely to `event_log::run_recorded`, which is
    // responsible for publishing its own terminal events.
    if params.log_file.is_some() {
        return super::event_log::run_recorded(params).await;
    }

    // Resolve the hub early so `DashboardEvent::RunCompleted` is published on
    // every exit path, including the early `?` returns in the body (plan load,
    // config validation, provider preflight, extension start-up, checkpoint).
    let hub_sender = params
        .state_hub
        .as_ref()
        .map(|hub| hub.sender())
        .unwrap_or_else(|| crate::state_hub::shared_state_hub().sender());

    // Ensure a consistent interrupt handle: if the caller passed None, create
    // one now and put it back so the body and this wrapper share the same
    // Arc<AtomicU8>.  Clone *after* the insert so both ends observe the same
    // stop flag.
    let mut params = params;
    if params.interrupt.is_none() {
        params.interrupt = Some(PlanRunInterruptHandle::default());
    }
    let interrupt_handle = params
        .interrupt
        .as_ref()
        .expect("interrupt handle was just set")
        .clone();
    let run_start = Instant::now();

    let result = run_graph_plan_body(params).await;

    let outcome = graph_run_outcome(&result, &interrupt_handle);
    hub_sender.publish(roko_core::DashboardEvent::RunCompleted {
        outcome: outcome.to_string(),
        duration_ms: run_start.elapsed().as_millis() as u64,
        cleanup_degraded: false,
        surviving_agent_ids: vec![],
        surviving_agent_pids: vec![],
    });

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

async fn run_graph_plan_body(params: GraphPlanRunParams) -> anyhow::Result<i32> {
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
    } = params;
    let interrupt = interrupt.unwrap_or_default();
    // FAST lane (`./dev.sh fast`): stop the run when its deadline elapses.
    let _fast_deadline = super::fast_lane::arm_plan_deadline(&interrupt);

    let plans_dir: &Path = &plans_dir;
    let workdir: &Path = &workdir;
    let resume_plan: Option<PathBuf> = resume_plan;
    let log_file: Option<PathBuf> = log_file;

    let run_start = std::time::Instant::now();
    let plans = crate::runner::plan_loader::load_plans(plans_dir)?;
    // Apply only_plans filter: keep exactly the named ids, in name order, and
    // fail immediately if any listed id does not exist in the directory.
    let plans = filter_only_plans(plans, only_plans.as_deref(), plans_dir)?;
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
    // Build the same feedback infrastructure that Runner-v2 uses, so Graph
    // engine runs produce episodes, efficiency events, playbook outcomes,
    // routing observations, experiment settlements, and daimon feedback.
    let graph_layout = RokoLayout::for_project(workdir);
    let graph_learn_dir = graph_layout.learn_dir();
    let _ = std::fs::create_dir_all(&graph_learn_dir);

    // ── #144: Construct daimon state early so it can be shared between
    //    the feedback facade (for plan-completion persistence) and the
    //    GraphFeedbackContext (for dispatch-time affect modulation). ───────
    let affect_path = workdir.join(".roko").join("daimon").join("affect.json");
    let shared_daimon_state: Option<std::sync::Arc<std::sync::Mutex<roko_daimon::DaimonState>>> = {
        let dims_vec = &roko_config.daimon.strategy_space.dimensions;
        if dims_vec.len() == 8 {
            // SAFETY: len == 8 is checked above, so try_into() is infallible here.
            let dims: [String; 8] = dims_vec
                .clone()
                .try_into()
                .expect("dims_vec has exactly 8 elements (checked above)");
            let def = roko_daimon::StrategySpaceDefinition {
                domain: roko_config.daimon.strategy_space.domain.clone(),
                dimensions: dims,
            };
            let mut s = roko_daimon::DaimonState::load_or_new(&affect_path);
            let _ = s.configure_strategy_space(def);
            Some(std::sync::Arc::new(std::sync::Mutex::new(s)))
        } else {
            tracing::warn!(
                dims = dims_vec.len(),
                "daimon strategy_space.dimensions must have exactly 8 entries; skipping"
            );
            None
        }
    };

    let graph_episodes_path = graph_layout.root_episodes_path();
    let graph_feedback_facade = {
        let mut facade =
            crate::runtime_feedback::FeedbackFacade::new().with_sink(std::sync::Arc::new(
                crate::runtime_feedback::EpisodeSink::at(&graph_episodes_path),
            ));
        if let Some(cascade) = &graph_run_config.cascade_router {
            facade = facade.with_sink(std::sync::Arc::new(
                crate::runtime_feedback::RoutingObservationSink::new(cascade.clone()),
            ));
        }

        // ── #143: Dream consolidation trigger on plan completion ────────
        facade = facade.with_sink(std::sync::Arc::new(
            crate::runtime_feedback::DreamConsolidationSink::new(
                workdir.to_path_buf(),
                roko_config.learning.dream_on_completion,
                roko_config.learning.dreams.trigger_on_plan_complete,
            ),
        ));

        // ── #144: Daimon affect persistence on plan completion ──────────
        if let Some(ref daimon) = shared_daimon_state {
            facade = facade.with_sink(std::sync::Arc::new(
                crate::runtime_feedback::DaimonPersistenceSink::new(
                    affect_path.clone(),
                    std::sync::Arc::clone(daimon),
                ),
            ));
        }

        // ── Theta reflection on plan completion ─────────────────────────
        //
        // Runs a five-phase reflective cycle (gamma summary, affect update,
        // calibration check, progress assessment, meta-cognition) after each
        // plan completes. Lightweight and synchronous (no LLM calls).
        let shared_cortical =
            std::sync::Arc::new(roko_runtime::heartbeat::CorticalState::default());
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
    };

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
            model_slug: roko_config.agent.default_model.clone(),
            prompt_variant: None,
            label: "graph-shadow".to_string(),
        },
        graph_learn_dir.join("shadow-results.jsonl"),
    ));

    let graph_feedback = crate::graph_task_dispatch::GraphFeedbackContext {
        feedback_facade: Some(graph_feedback_facade),
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
        replan_on_gate_failure: roko_config.learning.replan_on_gate_failure,
        coding_oracle: Some(coding_oracle),
        gate_gaming_detector: Some(gate_gaming_detector),
        holdout_experiment: Some(holdout_experiment.clone()),
        shadow_runner: Some(shadow_runner),
        eval_generation_enabled: true,
        // P2-LRN-6 Loop 1: Gate threshold EMA updates after each task's
        // verify sequence. Uses the canonical workspace path so the TUI,
        // serve, and `roko learn gates` all read from the same file.
        gate_thresholds_path: Some(graph_layout.gate_thresholds_path()),
        // RAG-10: retrieval outcome JSONL for gate-pass correlation telemetry.
        retrieval_outcomes_path: Some(graph_learn_dir.join("retrieval-outcomes.jsonl")),
    };

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
    .with_tui_bridge(dispatcher_tui_bridge);

    // ── Per-task worktree isolation (opt-in via --worktree-per-task) ──
    if worktree_per_task {
        use crate::orchestrator::worktree::{WorktreeConfig, WorktreeManager};
        let worktree_manager = WorktreeManager::new(WorktreeConfig {
            repo_root: workdir.to_path_buf(),
            base_branch: "HEAD".to_string(),
            worktrees_root: workdir.join(".roko").join("worktrees"),
            max_live: None,
            idle_ttl: std::time::Duration::from_hours(1),
        });
        let workspace_provider = Arc::new(
            crate::graph_execution::WorktreeExecutionWorkspaceProvider::new(worktree_manager),
        );
        if !quiet && !json {
            tracing::info!("per-task worktree isolation enabled (--worktree-per-task)");
        }
        dispatcher_builder = dispatcher_builder.with_workspace_provider(workspace_provider);
    }

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
        quiet,
        json,
        launch_tui,
        task_dispatcher: &task_dispatcher,
        graph_task_dispatcher: &graph_task_dispatcher,
        graph_tui_bridge: &graph_tui_bridge,
        graph_telemetry: &graph_telemetry,
        graph_event_logger: graph_event_logger.as_ref(),
        shared_pause_flag: &shared_pause_flag,
        interrupt: &interrupt,
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
    if let Ok(exp) = holdout_experiment.try_lock() {
        if let Err(err) = exp.save() {
            tracing::warn!(error = %err, "failed to persist holdout experiment state (non-fatal)");
        }
    }

    // ── Persist run metrics (backlog #169) ──────────────────────────
    //
    // Collect task counts and cost from the just-completed plan loop and
    // append a structured RunMetricsRecord to `.roko/learn/run-metrics.jsonl`.
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
                let completed = if *succeeded { plan_tasks } else { 0 };
                roko_learn::run_metrics::PlanMetrics {
                    plan_id: id.clone(),
                    completed: *succeeded,
                    tasks_completed: completed,
                    tasks_failed: plan_tasks.saturating_sub(completed),
                }
            })
            .collect();
        let tasks_completed: usize = per_plan.iter().map(|p| p.tasks_completed).sum();
        let tasks_failed: usize = per_plan.iter().map(|p| p.tasks_failed).sum();
        let any_budget_exhausted = plans
            .iter()
            .any(|p| graph_task_dispatcher.plan_budget_snapshot(&p.id).exhausted);
        let (agg_tokens_in, agg_tokens_out, agg_dispatch_count) =
            graph_task_dispatcher.run_aggregate_stats();
        let record = roko_learn::run_metrics::RunMetricsRecord {
            run_id: format!("graph-run-{}", chrono::Utc::now().timestamp_millis().max(0)),
            timestamp: chrono::Utc::now().to_rfc3339(),
            duration_ms,
            total_tasks,
            tasks_completed,
            tasks_failed,
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
    quiet: bool,
    json: bool,
    launch_tui: bool,
    task_dispatcher: &'a Arc<dyn roko_graph::cells::TaskDispatcher>,
    graph_task_dispatcher: &'a crate::graph_task_dispatch::GraphTaskDispatcher,
    graph_tui_bridge: &'a crate::runner::graph_tui_bridge::GraphTuiBridge,
    graph_telemetry: &'a Arc<dyn roko_core::TelemetryEventSink>,
    graph_event_logger: Option<&'a Arc<dyn roko_graph::events::GraphEventSink>>,
    shared_pause_flag: &'a Arc<AtomicBool>,
    interrupt: &'a PlanRunInterruptHandle,
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
}

impl PlanRunResult {
    const fn failed() -> Self {
        Self {
            outcome: PlanOutcome::Failed,
            output_count: 0,
        }
    }
}

/// Outcome of a plan that ran, by the same precedence as
/// [`plan_checkpoint_status`].
const fn plan_outcome(
    succeeded: bool,
    interrupted: bool,
    cancelled_by_operator: bool,
) -> PlanOutcome {
    if succeeded {
        PlanOutcome::Succeeded
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
    use roko_graph::cell::CellContext;
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

    // Convert Runner v2 tasks into PlanTaskInfo for the converter.
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
                max_retries: ctx.max_retries.unwrap_or(t.max_retries),
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

    let (graph, registry) = if ctx.rich_topology {
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
                return Ok(PlanRunResult::failed());
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
                return Ok(PlanRunResult::failed());
            }
        }
    };
    let mut checkpoint = crate::graph_checkpoint::prepare_graph_checkpoint(
        ctx.workdir,
        ctx.resume_plan,
        &plan.id,
        ctx.plan_count,
        &graph,
        ctx.fresh,
        ctx.force_resume,
    )?;
    let run_id = checkpoint.run_id().to_string();
    let replayed_entries = checkpoint.replayed_entries();
    ctx.graph_task_dispatcher
        .attach_plan_budget_checkpoint(&plan.id, checkpoint.take_cost_ledger())?;
    let mut engine = GraphEngine::new(graph, registry)
        .with_recorder(checkpoint.take_recorder())
        .with_telemetry(Arc::clone(ctx.graph_telemetry))
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
    let cell_ctx = CellContext::new()
        .with_run_id(run_id.clone())
        .with_pause_flag(Arc::clone(ctx.shared_pause_flag));

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
        checkpoint.finish(false)?;
        return Ok(PlanRunResult::failed());
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
        checkpoint.finish_with_status(plan_checkpoint_status(
            false,
            interrupted_by.is_some(),
            was_cancelled_by_tui,
        ))?;
        return Ok(PlanRunResult {
            outcome: plan_outcome(false, interrupted_by.is_some(), was_cancelled_by_tui),
            output_count: 0,
        });
    };

    let output_count = output
        .node_results
        .iter()
        .map(|r| r.output_count)
        .sum::<usize>();
    let budget = ctx.graph_task_dispatcher.plan_budget_snapshot(&plan.id);
    let execution_succeeded = output.success && !budget.dispatch_blocked && !was_cancelled_by_tui;

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
    graph_tui_bridge.plan_completed(&plan.id, execution_succeeded);

    if !ctx.quiet && !ctx.json {
        if was_cancelled_by_tui {
            tracing::warn!(
                plan_id = %plan.id,
                node_count = output.node_results.len(),
                "plan cancelled by user"
            );
        } else if execution_succeeded {
            tracing::info!(
                plan_id = %plan.id,
                node_count = output.node_results.len(),
                output_count,
                "plan completed: SUCCESS"
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
    checkpoint.finish_with_status(plan_checkpoint_status(
        execution_succeeded,
        interrupted_by.is_some(),
        was_cancelled_by_tui,
    ))?;
    Ok(PlanRunResult {
        outcome: plan_outcome(
            execution_succeeded,
            interrupted_by.is_some(),
            was_cancelled_by_tui,
        ),
        output_count,
    })
}

#[cfg(test)]
mod tests {
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
        assert_eq!(snapshot.plans["01-skipped"].phase, "completed");
        assert!(snapshot.plan_set_complete());
        assert_eq!(exit_code, EXIT_SUCCESS);
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

    /// Run the workspace's plan set. Returns the exit code, the plan
    /// lifecycle events in publication order, and the run's hub.
    async fn run_plan_set(
        dir: &Path,
        max_parallel_plans: Option<usize>,
        interrupt: Option<PlanRunInterruptHandle>,
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
            no_budget: true,
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn one_plan_at_a_time_keeps_the_execution_order() {
        let dir = disabled_role_plan_set(
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn independent_plans_run_side_by_side() {
        let dir = disabled_role_plan_set(&[("a", "a.txt", &[]), ("b", "b.txt", &[])], "");

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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn conductor_max_parallel_plans_alone_sets_the_limit() {
        let dir = disabled_role_plan_set(
            &[("a", "a.txt", &[]), ("b", "b.txt", &[])],
            "\n[conductor]\nmax_parallel_plans = 2\n",
        );

        let (exit_code, lifecycle, _) = run_plan_set(dir.path(), None, None).await;

        assert_eq!(exit_code, EXIT_SUCCESS);
        assert_eq!(lifecycle[..2], ["start a", "start b"], "{lifecycle:?}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn plans_sharing_a_file_never_run_side_by_side() {
        let dir = disabled_role_plan_set(&[("a", "README.md", &[]), ("b", "README.md", &[])], "");

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
        assert_eq!(
            plan_checkpoint_status(true, true, false),
            GraphCheckpointStatus::Succeeded
        );
        assert_eq!(
            plan_checkpoint_status(false, true, true),
            GraphCheckpointStatus::Interrupted
        );
        assert_eq!(
            plan_checkpoint_status(false, false, true),
            GraphCheckpointStatus::Cancelled
        );
        assert_eq!(
            plan_checkpoint_status(false, false, false),
            GraphCheckpointStatus::Failed
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
}
