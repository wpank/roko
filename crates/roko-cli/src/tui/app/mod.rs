//! Interactive TUI application shell.
//!
//! Integrates the Mori-style tab system (F1-F10), modal dialogs, TuiState,
//! TuiAction dispatch, PostFX pipeline, and atmosphere animations.

mod actions;
mod channels;
mod event_loop;
mod focus;
mod modals;
#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{self, Stdout, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc as std_mpsc,
};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::cursor;
use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, KeyEvent, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode, size,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::Paragraph;

use roko_runtime::process::ProcessSupervisor;
use sysinfo::{Disks, Networks, Pid, ProcessStatus, ProcessesToUpdate, System};
use tokio::sync::{mpsc, oneshot, watch};

use super::approval_ipc::ApprovalRequest;
use super::dashboard::{DashboardData, DashboardScaffold, Theme};
use super::effects_config::EffectsConfig;
use super::event::{
    Event, EventHandler, FrameStats, RenderDirty, TickPolicyInputs, next_tick_policy,
};
use super::fs_watch::{self, FsRefresh, FsWatchHandle};
use super::git_watch::{self, GitRefresh, GitWatchHandle};
use super::input::{self, ConfirmAction, FocusZone, InputMode, TuiAction};
use super::modals::{
    self as modals_mod, Milestone, ModalState, QueueTask, TaskPickerRow, WaveInfo, WavePlanEntry,
};
use super::pages::{PageId, PageRegistry};
use super::state::{PendingApproval, PlanEntry, TaskRowStatus, TuiState};
use super::tabs::Tab;
use super::verdicts::VerdictsAggregator;
use super::views::{self, ViewState};
use super::ws_client::AgentStreamClient;

pub use event_loop::run;

/// Interactive dashboard shell backed by the existing snapshot renderer.
///
/// Supports two rendering paths:
/// - **Mori-style tabs** (F1-F10): full TuiState + views + modals + postfx
/// - **Legacy scaffold pages**: original PageId-based rendering
///
/// All expensive I/O stays off the render path. System metrics run on a
/// background thread, while filesystem and git refreshes run only on watcher
/// nudges. The render path does zero I/O -- it only reads `&self.tui_state`
/// and `&self.data` and writes to the frame buffer.
pub struct App {
    workdir: PathBuf,

    // -- Mori-style state --
    /// Full TUI state (agents, plans, navigation, modals, scroll, etc.).
    pub tui_state: TuiState,
    /// PostFX configuration.
    fx_config: EffectsConfig,
    /// Reusable post-FX scratch buffers (#366 — resized only on terminal resize).
    pfx_bufs: super::postfx_pipeline::PostFxBuffers,
    /// Toast notifications.
    notifications: VecDeque<super::modals::Notification>,
    /// Keyboard scroll acceleration state for held-key scrolling.
    scroll_accel: super::scroll::ScrollAccel,

    // -- Legacy scaffold state (kept for text-mode compatibility) --
    /// Currently selected dashboard page (legacy path).
    pub current_page: PageId,
    /// Shared dashboard data model, refreshed on tick.
    pub data: DashboardData,
    /// Static page scaffold used by the legacy renderer.
    scaffold: DashboardScaffold,
    /// Last seen dashboard data generation used to avoid redundant scaffold rebuilds.
    last_data_gen: u64,
    /// Incremental substrate reader for gate verdict trends.
    verdicts_aggregator: Option<VerdictsAggregator>,

    // -- Common --
    /// Whether the event loop should keep running.
    pub running: bool,
    /// Timestamp of the last data refresh.
    pub last_refresh: Instant,
    /// Per-page scroll position (legacy).
    pub scroll_offset: HashMap<PageId, u16>,
    /// Selected signal row on the Signals page (legacy).
    pub signal_selection: usize,
    /// Selected gate-failure row on the Verify Results page (legacy).
    pub gate_failure_selection: usize,
    // -- Background I/O channels --
    /// Background system metrics receiver (CPU/MEM collected off main thread).
    sys_rx: Option<watch::Receiver<SysSnapshot>>,
    /// One-shot agent-topology fetch receiver.
    agent_topology_rx: Option<std_mpsc::Receiver<AgentTopologyFetchResult>>,
    /// Whether a topology fetch is currently in flight.
    agent_topology_in_flight: bool,
    /// Filesystem watcher handle for debounced `.roko/` refresh events.
    fs_watch: Option<FsWatchHandle>,
    /// Cached theme instance (avoids per-frame env reads).
    theme: Theme,
    /// Debounced git watcher for repo metadata refreshes.
    git_watch: Option<GitWatchHandle>,
    /// Optional live process supervisor used for per-agent process sampling.
    process_supervisor: Option<Arc<ProcessSupervisor>>,
    /// Optional approval request receiver from the orchestrator.
    pub approval_rx: Option<mpsc::Receiver<ApprovalRequest>>,
    /// Pending response channel for the active approval modal.
    pending_approval_response: Option<oneshot::Sender<bool>>,
    /// Owning handle for the in-process or connected state hub.
    _state_hub: Option<crate::state_hub::SharedStateHub>,
    /// Whether this app should refresh the StateHub from on-disk dashboard state.
    ///
    /// Standalone `roko dashboard` uses a local hub backed by disk replay.
    /// Connected approval/dashboard sessions receive live events from their
    /// caller and must not import stale historical runs from disk.
    replay_disk_snapshots: bool,
    /// Optional signal used by an owning command to shut down the TUI.
    shutdown_rx: Option<std_mpsc::Receiver<()>>,
    /// Whether this connected TUI should exit when its observed run completes.
    exit_on_plan_completion: bool,
    /// Whether to enable terminal mouse reporting while the TUI is active.
    capture_mouse: bool,
    /// Tab transition fade: tracks a brief fade-in when switching tabs.
    /// Holds `(started_at, duration)` while the transition is active.
    tab_transition: Option<(Instant, Duration)>,
    /// Whether a plan has been observed in this connected TUI session.
    connected_plan_observed: bool,
    /// Live dashboard snapshot receiver from `StateHub` when connected.
    pub snapshot_rx: Option<tokio::sync::watch::Receiver<roko_core::DashboardSnapshot>>,
    /// Lossless live event subscription paired with `snapshot_rx`. This is
    /// the primary transcript source; disk/task tails are recovery only.
    state_events: Option<crate::state_hub::StateHubSubscription>,
    /// Last error entry surfaced from the live snapshot stream.
    last_snapshot_error_marker: Option<(String, u64)>,
    /// Number of gate verdicts already processed for toast generation.
    last_seen_gate_count: usize,
    /// Plan phase states already seen (plan_id -> phase) for completion toasts.
    last_seen_plan_phases: HashMap<String, String>,
    /// Live websocket consumers for the Agents tab.
    agent_stream_clients: HashMap<String, AgentStreamClient>,
    /// Base URL for the `roko-serve` websocket event bus.
    agent_stream_server_url: String,
    /// Optional bearer token for authenticated websocket handshakes.
    agent_stream_auth_token: Option<String>,
    /// Configured redraw/poll cadence. Connected sessions no longer force a
    /// hard-coded 60/20 FPS loop while idle.
    refresh_rate: Duration,
    /// Last known terminal size used for hit-testing.
    terminal_size: (u16, u16),
    /// Flag set by sync methods to request an async refresh on next loop
    /// iteration (verdicts open/tick are async and cannot be called from sync
    /// dispatch_action).
    pending_refresh: bool,
    /// Accumulated dirty-frame reasons since the last draw. The render loop
    /// draws only when this is non-empty after coalescing all ready inputs.
    render_dirty: RenderDirty,
    /// Per-session render telemetry (frame count, skipped ticks, latencies).
    frame_stats: FrameStats,
    /// Hash of the last applied snapshot file for incremental change detection (RC-6).
    /// When the standalone dashboard detects a filesystem change, it reads only the
    /// snapshot hash first and skips the full re-bootstrap when unchanged.
    last_snapshot_hash: Option<u64>,
    /// Byte offset into `events.jsonl` for incremental event replay (RC-6).
    /// Only new events after this offset are replayed, avoiding O(n) re-reads.
    last_events_offset: u64,
    /// Sender for in-process execution commands to the runner/graph event loop.
    exec_cmd_sender: Option<crate::execution_control::ExecutionCommandSender>,
    /// Receiver for command acknowledgements from the executor.
    exec_ack_receiver: Option<crate::execution_control::CommandAckReceiver>,
    /// Pending commands awaiting acknowledgement: command_id -> kind.
    /// Used to commit state changes (e.g. flip is_paused) on `Completed`.
    pending_exec_commands:
        std::collections::HashMap<String, crate::execution_control::ExecutionCommandKind>,
    /// Receiver for background git data collection results.
    git_bg_rx: Option<std::sync::mpsc::Receiver<(u64, GitBgData)>>,
    /// Monotonically increasing generation counter for spawned git jobs.
    git_bg_generation: u64,
    /// Highest generation that has been applied to the TUI state.
    git_applied_generation: u64,
    /// P1-40: Broadcast sender for SurfaceEvent commands produced by TUI actions.
    ///
    /// TUI actions (approve, reject, pause, cancel, etc.) emit SurfaceEvent
    /// objects through this channel so external consumers (serve, runner) can
    /// translate them into execution effects.
    surface_event_tx: tokio::sync::broadcast::Sender<roko_core::runtime_event::SurfaceEvent>,
}

/// Bundle of git data collected by the watcher-driven git refresh path.
struct GitBgData {
    /// Full git view data for the F4 Git tab.
    view_data: super::views::git_view::GitViewData,
    /// Summary lines for the dashboard sub-tab.
    summary_lines: Vec<String>,
    /// Git branch name.
    branch: String,
    /// Short commit hash.
    commit_short: String,
    /// Commit age string (e.g. "3 hours ago").
    age: String,
}

/// Combined host + process metrics snapshot emitted by the background sampler.
#[derive(Debug, Clone, Default)]
struct SysSnapshot {
    /// Host-level system metrics.
    sys: super::state::SysMetrics,
    /// Per-process point-in-time samples.
    process_metrics: Vec<ProcessMetricSample>,
}

/// One sampled process row before history is merged into `TuiState`.
#[derive(Debug, Clone)]
struct ProcessMetricSample {
    /// OS process identifier.
    pid: u32,
    /// Human-readable role or label.
    role: String,
    /// Current CPU usage percentage.
    cpu_pct: f32,
    /// Resident memory in bytes.
    mem_bytes: u64,
    /// Compact process state label.
    state: String,
    /// Process uptime in seconds.
    uptime_secs: f64,
}

enum AgentTopologyFetchResult {
    Ready(roko_core::AgentTopology),
    Unavailable,
    Error(String),
}

// -----------------------------------------------------------------------
// Free helper functions
// -----------------------------------------------------------------------

fn collect_git_bg_data(workdir: &Path) -> GitBgData {
    let view_data = super::views::git_view::collect_git_data(workdir);
    let branch = view_data.current_branch.clone();
    let commit_short = view_data
        .commits
        .first()
        .map(|commit| commit.hash_short.clone())
        .unwrap_or_default();
    let age = view_data
        .commits
        .first()
        .map(|commit| commit.age.clone())
        .unwrap_or_default();
    let summary_lines = super::views::dashboard_view::collect_git_summary(&view_data, &age);

    GitBgData {
        view_data,
        summary_lines,
        branch,
        commit_short,
        age,
    }
}

fn plan_status_label(plan: &PlanEntry) -> String {
    if !plan.phase.is_empty() {
        plan.phase.clone()
    } else if plan.status != super::state::PlanPhase::Pending {
        plan.status.label().to_string()
    } else if plan.active {
        "active".to_string()
    } else {
        "pending".to_string()
    }
}

fn task_status_label(status: TaskRowStatus) -> &'static str {
    match status {
        TaskRowStatus::Pending => "pending",
        TaskRowStatus::Active => "active",
        TaskRowStatus::Done => "done",
        TaskRowStatus::Failed => "failed",
        TaskRowStatus::AcceptedWithFailures => {
            roko_core::dashboard_snapshot::TASK_OUTCOME_ACCEPTED_WITH_FAILURES
        }
        TaskRowStatus::AlreadySatisfied => {
            roko_core::dashboard_snapshot::TASK_OUTCOME_ALREADY_SATISFIED
        }
        TaskRowStatus::Unverified => roko_core::dashboard_snapshot::TASK_OUTCOME_UNVERIFIED,
        TaskRowStatus::Skipped => "skipped",
        TaskRowStatus::Blocked => "blocked",
    }
}

fn execution_waves_for_modal(state: &TuiState) -> Vec<WaveInfo> {
    let plans_by_id: HashMap<&str, &PlanEntry> = state
        .plans
        .iter()
        .map(|plan| (plan.id.as_str(), plan))
        .collect();

    state
        .execution_waves
        .iter()
        .map(|wave| WaveInfo {
            wave_index: wave.index,
            plans: wave
                .plans
                .iter()
                .map(|plan_id| {
                    if let Some(plan) = plans_by_id.get(plan_id.as_str()) {
                        WavePlanEntry {
                            plan_id: plan.id.clone(),
                            status: plan_status_label(plan),
                            duration_secs: Some(plan.elapsed_secs.max(0.0) as u64),
                        }
                    } else {
                        WavePlanEntry {
                            plan_id: plan_id.clone(),
                            status: "queued".to_string(),
                            duration_secs: None,
                        }
                    }
                })
                .collect(),
            total_duration_secs: Some(
                wave.plans
                    .iter()
                    .filter_map(|plan_id| plans_by_id.get(plan_id.as_str()))
                    .map(|plan| plan.elapsed_secs.max(0.0) as u64)
                    .sum(),
            ),
            eta_secs: None,
        })
        .collect()
}

fn queue_overview_milestones(state: &TuiState, workdir: &Path) -> Vec<Milestone> {
    let plans_by_id: HashMap<&str, &PlanEntry> = state
        .plans
        .iter()
        .map(|plan| (plan.id.as_str(), plan))
        .collect();

    // Try loading milestones from .roko/queue.toml first; fall back to execution waves.
    let queue_path = workdir.join(".roko").join("queue.toml");
    if let Ok(manifest) = crate::runner::queue_manifest::QueueManifest::from_file(&queue_path) {
        if !manifest.milestones.is_empty() {
            return manifest
                .milestones
                .iter()
                .map(|ms| {
                    let tasks: Vec<QueueTask> = ms
                        .plans
                        .iter()
                        .map(|plan_id| {
                            if let Some(plan) = plans_by_id.get(plan_id.as_str()) {
                                QueueTask {
                                    id: plan.id.clone(),
                                    title: if plan.name.is_empty() {
                                        plan.id.clone()
                                    } else {
                                        plan.name.clone()
                                    },
                                    status: plan_status_label(plan),
                                }
                            } else {
                                QueueTask {
                                    id: plan_id.clone(),
                                    title: plan_id.clone(),
                                    status: "queued".to_string(),
                                }
                            }
                        })
                        .collect();
                    let completed = tasks.iter().filter(|t| t.status == "done").count();
                    Milestone {
                        name: ms.name.clone(),
                        tasks,
                        completed,
                        total: ms.plans.len(),
                    }
                })
                .collect();
        }
    }

    // Fallback: derive milestones from execution waves in the TUI state.
    state
        .execution_waves
        .iter()
        .map(|wave| Milestone {
            name: format!("Wave {}", wave.index),
            tasks: wave
                .plans
                .iter()
                .map(|plan_id| {
                    if let Some(plan) = plans_by_id.get(plan_id.as_str()) {
                        QueueTask {
                            id: plan.id.clone(),
                            title: if plan.name.is_empty() {
                                plan.id.clone()
                            } else {
                                plan.name.clone()
                            },
                            status: plan_status_label(plan),
                        }
                    } else {
                        QueueTask {
                            id: plan_id.clone(),
                            title: plan_id.clone(),
                            status: "queued".to_string(),
                        }
                    }
                })
                .collect(),
            completed: wave.done,
            total: wave.total,
        })
        .collect()
}

fn task_picker_rows(state: &TuiState) -> Vec<TaskPickerRow> {
    let plan_num = state.current_plan_idx.saturating_add(1) as u32;

    state
        .current_task_checklist
        .iter()
        .map(|task| TaskPickerRow {
            plan_num,
            task_id: task.id.clone(),
            title: task.title.clone(),
            status: task_status_label(task.status).to_string(),
        })
        .collect()
}

fn convert_git_commit_graph(
    commits: &[super::views::git_view::CommitEntry],
) -> Vec<super::state::GitCommitEntry> {
    commits
        .iter()
        .map(|commit| super::state::GitCommitEntry {
            hash: commit.hash_short.clone(),
            short_hash: commit.hash_short.clone(),
            message: commit.subject.clone(),
            author: commit.author.clone(),
            timestamp_ms: 0,
            branch: None,
        })
        .collect()
}

fn convert_git_worktree_list(worktrees: &[super::views::git_view::WorktreeEntry]) -> Vec<String> {
    worktrees
        .iter()
        .map(|worktree| worktree.path.clone())
        .collect()
}

// Manual Debug impl because mpsc::Receiver does not implement Debug.
impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App")
            .field("workdir", &self.workdir)
            .field("running", &self.running)
            .field("current_page", &self.current_page)
            .finish_non_exhaustive()
    }
}

type TuiTerminal = Terminal<CrosstermBackend<Stdout>>;

const TERMINAL_RESET_SEQUENCE: &[u8] =
    b"\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?1015l\x1b[?1049l\x1b[?25h\x1b[0m";

static TERMINAL_CLEANUP_ACTIVE: AtomicBool = AtomicBool::new(false);

#[cfg(unix)]
static TERMINAL_SIGNAL_CLEANUP_INSTALLED: AtomicBool = AtomicBool::new(false);

/// Set by [`App::with_host_termination_signals`]: the host handles
/// SIGINT/SIGTERM, so the terminal handler only claims SIGHUP.
static HOST_HANDLES_TERMINATION_SIGNALS: AtomicBool = AtomicBool::new(false);

/// Best-effort terminal reset for a host that must exit while a TUI thread
/// may still own the terminal (e.g. a forced shutdown after a second signal).
pub fn restore_terminal_for_forced_exit() {
    if TERMINAL_CLEANUP_ACTIVE.load(Ordering::SeqCst) {
        App::cleanup_terminal_best_effort();
    }
}

struct PanicHookRestoreGuard(Arc<dyn Fn(&std::panic::PanicHookInfo<'_>) + Send + Sync + 'static>);

impl Drop for PanicHookRestoreGuard {
    fn drop(&mut self) {
        // Restoring a panic hook from a panicking thread itself panics. Keep
        // the cleanup hook installed while unwinding so the original TUI
        // diagnostic is preserved instead of becoming a double-panic abort.
        if std::thread::panicking() {
            return;
        }
        let hook = Arc::clone(&self.0);
        std::panic::set_hook(Box::new(move |panic_info| hook(panic_info)));
    }
}

struct TerminalCleanupGuard {
    active: bool,
}

impl TerminalCleanupGuard {
    fn arm() -> Self {
        TERMINAL_CLEANUP_ACTIVE.store(true, Ordering::SeqCst);
        install_terminal_signal_cleanup();
        Self { active: true }
    }

    fn restore(&mut self) -> Result<()> {
        self.active = false;
        App::cleanup_terminal()
    }
}

impl Drop for TerminalCleanupGuard {
    fn drop(&mut self) {
        if self.active {
            App::cleanup_terminal_best_effort();
        }
    }
}

#[cfg(unix)]
fn install_terminal_signal_cleanup() {
    if TERMINAL_SIGNAL_CLEANUP_INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }

    // SAFETY: installing a process signal handler requires libc. The handler
    // only performs async-signal-safe writes and then restores the default
    // disposition before re-raising the original signal.
    #[allow(unsafe_code)]
    unsafe {
        let handler = terminal_signal_handler as *const () as libc::sighandler_t;
        if !HOST_HANDLES_TERMINATION_SIGNALS.load(Ordering::SeqCst) {
            let _ = libc::signal(libc::SIGINT, handler);
            let _ = libc::signal(libc::SIGTERM, handler);
        }
        let _ = libc::signal(libc::SIGHUP, handler);
    }
}

#[cfg(not(unix))]
fn install_terminal_signal_cleanup() {}

#[cfg(unix)]
extern "C" fn terminal_signal_handler(signal: libc::c_int) {
    if TERMINAL_CLEANUP_ACTIVE.load(Ordering::SeqCst) {
        // SAFETY: write(2) is async-signal-safe. The reset bytes are a static
        // buffer and both file descriptors are process constants.
        #[allow(unsafe_code)]
        unsafe {
            let _ = libc::write(
                libc::STDOUT_FILENO,
                TERMINAL_RESET_SEQUENCE.as_ptr().cast(),
                TERMINAL_RESET_SEQUENCE.len(),
            );
            let _ = libc::write(
                libc::STDERR_FILENO,
                TERMINAL_RESET_SEQUENCE.as_ptr().cast(),
                TERMINAL_RESET_SEQUENCE.len(),
            );
        }
    }

    // SAFETY: restore the default signal disposition and re-raise the signal
    // so process semantics remain the same after best-effort terminal reset.
    #[allow(unsafe_code)]
    unsafe {
        let _ = libc::signal(signal, libc::SIG_DFL);
        let _ = libc::raise(signal);
    }
}

fn tui_log_path(workdir: &Path) -> PathBuf {
    workdir.join(".roko").join("tui.log")
}

fn tui_log_dispatch(workdir: &Path) -> Result<tracing::Dispatch> {
    let roko_dir = workdir.join(".roko");
    std::fs::create_dir_all(&roko_dir)
        .with_context(|| format!("create TUI log directory {}", roko_dir.display()))?;

    let log_path = tui_log_path(workdir);
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .with_context(|| format!("open TUI log file {}", log_path.display()))?;

    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(Mutex::new(log_file))
        .finish();

    Ok(tracing::Dispatch::new(subscriber))
}

fn configured_tui_refresh_rate(workdir: &Path) -> Duration {
    const DEFAULT_MS: u64 = 250;
    const MIN_MS: u64 = 50;
    const MAX_MS: u64 = 5_000;

    let configured = std::fs::read_to_string(workdir.join("roko.toml"))
        .ok()
        .and_then(|contents| contents.parse::<toml::Value>().ok())
        .and_then(|value| {
            value
                .get("tui")
                .and_then(|tui| tui.get("refresh_rate_ms"))
                .and_then(toml::Value::as_integer)
        })
        .and_then(|value| u64::try_from(value).ok())
        .unwrap_or(DEFAULT_MS)
        .clamp(MIN_MS, MAX_MS);
    Duration::from_millis(configured)
}

// ---------------------------------------------------------------------------
// Config field value cycling
// ---------------------------------------------------------------------------

/// Cycle an enum/preset field value left (false) or right (true).
fn cycle_field_value(
    meta: &super::config_meta::ConfigFieldMeta,
    current: &str,
    forward: bool,
) -> Option<String> {
    match &meta.kind {
        super::config_meta::ConfigFieldKind::Enum(opts) => {
            let idx = opts.iter().position(|&o| o == current).unwrap_or(0);
            let new_idx = if forward {
                (idx + 1) % opts.len()
            } else {
                (idx + opts.len() - 1) % opts.len()
            };
            Some(opts[new_idx].to_string())
        }
        super::config_meta::ConfigFieldKind::Int { presets, .. } if !presets.is_empty() => {
            let cur: i64 = current.parse().unwrap_or(0);
            let idx = presets.iter().position(|&p| p == cur).unwrap_or(0);
            let new_idx = if forward {
                (idx + 1) % presets.len()
            } else {
                (idx + presets.len() - 1) % presets.len()
            };
            Some(presets[new_idx].to_string())
        }
        super::config_meta::ConfigFieldKind::Bool => {
            Some(if current == "true" { "false" } else { "true" }.to_string())
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Background sys metrics collection (runs on a dedicated thread)
// ---------------------------------------------------------------------------

/// Collect system metrics on a background thread using `sysinfo`.
fn collect_sys_metrics_bg(
    tx: watch::Sender<SysSnapshot>,
    process_supervisor: Option<Arc<ProcessSupervisor>>,
) {
    let mut sys = System::new_all();
    let mut networks = Networks::new_with_refreshed_list();
    let mut disks = Disks::new_with_refreshed_list();
    let own_pid = Pid::from_u32(std::process::id());

    const SAMPLE_SECS: u64 = 2;

    loop {
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        networks.refresh(true);
        disks.refresh(false);

        // Network: sum bytes-since-last-refresh across all interfaces,
        // then divide by sample interval to get bytes/sec.
        let (net_rx_delta, net_tx_delta) = networks.iter().fold((0u64, 0u64), |(rx, tx), nic| {
            (rx + nic.1.received(), tx + nic.1.transmitted())
        });
        let net_down_bps = net_rx_delta / SAMPLE_SECS;
        let net_up_bps = net_tx_delta / SAMPLE_SECS;

        // Disk capacity: sum free/total across all mount points.
        let (disk_free, disk_total) = disks.iter().fold((0u64, 0u64), |(free, total), d| {
            (free + d.available_space(), total + d.total_space())
        });

        // Disk I/O: use process-level disk_usage for our own PID.
        // This works on both macOS and Linux.
        sys.refresh_processes(ProcessesToUpdate::Some(&[own_pid]), true);
        let (disk_read_bps, disk_write_bps) = sys
            .process(own_pid)
            .map(|p| {
                let du = p.disk_usage();
                (du.read_bytes / SAMPLE_SECS, du.written_bytes / SAMPLE_SECS)
            })
            .unwrap_or((0, 0));

        let snapshot = SysSnapshot {
            sys: super::state::SysMetrics {
                cpu_pct: sys.global_cpu_usage(),
                mem_used_bytes: sys.used_memory(),
                mem_total_bytes: sys.total_memory(),
                net_down_bytes_sec: net_down_bps,
                net_up_bytes_sec: net_up_bps,
                disk_read_bytes_sec: disk_read_bps,
                disk_write_bytes_sec: disk_write_bps,
                disk_free_bytes: disk_free,
                disk_total_bytes: disk_total,
                ..Default::default()
            },
            process_metrics: collect_process_metrics(&mut sys, process_supervisor.as_deref()),
        };

        if tx.send(snapshot).is_err() {
            break;
        }

        std::thread::sleep(Duration::from_secs(SAMPLE_SECS));
    }
}

fn collect_process_metrics(
    sys: &mut System,
    process_supervisor: Option<&ProcessSupervisor>,
) -> Vec<ProcessMetricSample> {
    let Some(process_supervisor) = process_supervisor else {
        return Vec::new();
    };

    // `active_pids()` is async (parking_lot::Mutex only), safe to call via
    // a current-thread runtime on this dedicated background thread.
    let Ok(rt) = tokio::runtime::Builder::new_current_thread().build() else {
        return Vec::new();
    };
    let active_pids = rt.block_on(process_supervisor.active_pids());
    if active_pids.is_empty() {
        return Vec::new();
    }

    let pids: Vec<Pid> = active_pids
        .iter()
        .map(|(pid, _)| Pid::from_u32(*pid))
        .collect();
    sys.refresh_processes(ProcessesToUpdate::Some(&pids), true);

    active_pids
        .into_iter()
        .filter_map(|(pid, role)| {
            let proc = sys.process(Pid::from_u32(pid))?;
            Some(ProcessMetricSample {
                pid,
                role,
                cpu_pct: proc.cpu_usage(),
                mem_bytes: proc.memory(),
                state: process_state_label(proc.status()).to_string(),
                uptime_secs: proc.run_time() as f64,
            })
        })
        .collect()
}

fn process_state_label(status: ProcessStatus) -> &'static str {
    match status {
        ProcessStatus::Run => "running",
        ProcessStatus::Sleep
        | ProcessStatus::Idle
        | ProcessStatus::Waking
        | ProcessStatus::Parked => "sleeping",
        ProcessStatus::Stop
        | ProcessStatus::Tracing
        | ProcessStatus::Dead
        | ProcessStatus::Wakekill
        | ProcessStatus::LockBlocked
        | ProcessStatus::UninterruptibleDiskSleep
        | ProcessStatus::Zombie => "stopped",
        ProcessStatus::Unknown(_) => "unknown",
    }
}

fn push_bounded_history<T>(history: &mut VecDeque<T>, value: T, max_len: usize) {
    if history.len() >= max_len {
        history.pop_front();
    }
    history.push_back(value);
}

fn resolve_agent_stream_server_url() -> String {
    std::env::var("ROKO_SERVE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("ROKO_SERVER_URL")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(|| roko_cli::DEFAULT_SERVE_URL.to_string())
}

fn resolve_agent_stream_auth_token() -> Option<String> {
    std::env::var("ROKO_SERVER_AUTH_TOKEN")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

/// Map a Mori-style Tab to a legacy PageId (best effort).
fn tab_to_page(tab: Tab) -> Option<PageId> {
    match tab {
        Tab::Dashboard => Some(PageId::Health),
        Tab::Plans => Some(PageId::PlanView),
        Tab::Agents => Some(PageId::AgentStatus),
        Tab::Logs => Some(PageId::LogView),
        Tab::Config => Some(PageId::ConfigView),
        Tab::Git
        | Tab::Inspect
        | Tab::Marketplace
        | Tab::Atelier
        | Tab::Learning
        | Tab::Providers => None,
    }
}

use crate::tui::display_utils::truncate as truncate_str;

fn apply_dashboard_snapshot(
    tui_state: &mut TuiState,
    notifications: &mut VecDeque<super::modals::Notification>,
    last_snapshot_error_marker: &mut Option<(String, u64)>,
    last_seen_gate_count: &mut usize,
    last_seen_plan_phases: &mut HashMap<String, String>,
    snapshot: &roko_core::DashboardSnapshot,
) {
    tui_state.update_from_dashboard_snapshot(snapshot);

    // -- Gate verdict toasts (new verdicts since last snapshot) ---------
    if snapshot.gates.len() > *last_seen_gate_count {
        for gate in snapshot.gates.iter().skip(*last_seen_gate_count) {
            let notif = if gate.passed {
                super::modals::Notification::info(format!(
                    "{}/{}: {} PASS",
                    gate.plan_id, gate.task_id, gate.gate
                ))
            } else {
                super::modals::Notification::new(
                    format!("{}/{}: {} FAIL", gate.plan_id, gate.task_id, gate.gate),
                    super::modals::NotificationLevel::Error,
                    10,
                )
            };
            push_deduped_notification(notifications, notif);
        }
    }
    *last_seen_gate_count = snapshot.gates.len();

    // -- Plan completion toasts ----------------------------------------
    for (plan_id, plan_state) in &snapshot.plans {
        let prev_phase = last_seen_plan_phases.get(plan_id).map(String::as_str);
        let cur_phase = plan_state.phase.as_str();

        // Only fire a toast when phase transitions to completed/failed.
        if prev_phase != Some(cur_phase) {
            match cur_phase {
                "completed" => {
                    push_deduped_notification(
                        notifications,
                        super::modals::Notification::new(
                            format!("Plan {plan_id} completed successfully"),
                            super::modals::NotificationLevel::Info,
                            8,
                        ),
                    );
                }
                "failed" => {
                    push_deduped_notification(
                        notifications,
                        super::modals::Notification::new(
                            format!("Plan {plan_id} failed"),
                            super::modals::NotificationLevel::Error,
                            10,
                        ),
                    );
                }
                p if p.contains("stall") => {
                    push_deduped_notification(
                        notifications,
                        super::modals::Notification::warn(format!(
                            "Plan {plan_id}: agent stall detected"
                        )),
                    );
                }
                _ => {}
            }
        }
        last_seen_plan_phases.insert(plan_id.clone(), cur_phase.to_string());
    }

    // -- Error toasts (existing behavior) ------------------------------
    if !snapshot.errors.is_empty() {
        let start_idx = last_snapshot_error_marker
            .as_ref()
            .and_then(|marker| {
                snapshot
                    .errors
                    .iter()
                    .position(|error| error.message == marker.0 && error.ts_millis == marker.1)
                    .map(|idx| idx + 1)
            })
            .unwrap_or(0);

        for error in snapshot.errors.iter().skip(start_idx) {
            push_deduped_notification(
                notifications,
                super::modals::Notification::error(error.message.clone()),
            );
        }

        if let Some(last_error) = snapshot.errors.last() {
            *last_snapshot_error_marker = Some((last_error.message.clone(), last_error.ts_millis));
        }
    }
}

/// Push a notification unless a duplicate (same message within 2 seconds)
/// already exists in the stack.
fn push_deduped_notification(
    notifications: &mut VecDeque<super::modals::Notification>,
    notification: super::modals::Notification,
) {
    let dominated = notifications.iter().any(|existing| {
        existing.message == notification.message && existing.created.elapsed().as_secs() < 2
    });
    if !dominated {
        notifications.push_back(notification);
    }
}

fn fetch_agent_topology(base_url: &str) -> AgentTopologyFetchResult {
    let endpoint = format!("{}/api/agents/topology", base_url.trim_end_matches('/'));
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .build();

    let Ok(client) = client else {
        return AgentTopologyFetchResult::Error("topology client init failed".to_string());
    };

    let response = match client.get(endpoint).send() {
        Ok(response) => response,
        Err(error) => {
            return AgentTopologyFetchResult::Error(format!("topology fetch failed: {error}"));
        }
    };

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return AgentTopologyFetchResult::Unavailable;
    }

    if !response.status().is_success() {
        return AgentTopologyFetchResult::Error(format!(
            "topology fetch returned {}",
            response.status()
        ));
    }

    match response.json::<roko_core::AgentTopology>() {
        Ok(topology) => AgentTopologyFetchResult::Ready(topology),
        Err(error) => AgentTopologyFetchResult::Error(format!("invalid topology payload: {error}")),
    }
}

fn snapshot_has_content(snapshot: &roko_core::DashboardSnapshot) -> bool {
    !snapshot.plans.is_empty()
        || !snapshot.tasks.is_empty()
        || !snapshot.agents.is_empty()
        || !snapshot.gates.is_empty()
        || !snapshot.agent_topology.is_empty()
        || !snapshot.experiment_winners.is_empty()
        || !snapshot.errors.is_empty()
}

// -----------------------------------------------------------------------
// App constructors and public API
// -----------------------------------------------------------------------

impl App {
    /// Build a new app from a workspace root.
    #[must_use]
    pub fn new(root: impl AsRef<Path>) -> Self {
        let state_hub = crate::state_hub::SharedStateHub::new_in_process();
        let _ = state_hub.bootstrap_from_workdir(root.as_ref());
        // Replay events.jsonl to pick up events from roko run / roko serve.
        let events_path = root.as_ref().join(".roko").join("events.jsonl");
        let count = state_hub.replay_log_into_snapshot(&events_path);
        if count > 0 {
            tracing::info!(count, path = %events_path.display(), "replayed events from log");
        }
        Self::new_connected_with_state_hub(root, None, state_hub, true)
    }

    /// Build a new app from a workspace root with an initial page selection.
    #[must_use]
    pub fn new_with_page(root: impl AsRef<Path>, initial_page: Option<PageId>) -> Self {
        let state_hub = crate::state_hub::SharedStateHub::new_in_process();
        let _ = state_hub.bootstrap_from_workdir(root.as_ref());
        // Replay events.jsonl to pick up events from roko run / roko serve.
        let events_path = root.as_ref().join(".roko").join("events.jsonl");
        let count = state_hub.replay_log_into_snapshot(&events_path);
        if count > 0 {
            tracing::info!(count, path = %events_path.display(), "replayed events from log");
        }
        Self::new_connected_with_state_hub(root, initial_page, state_hub, true)
    }

    fn new_with_page_inner(
        root: impl AsRef<Path>,
        initial_page: Option<PageId>,
        state_hub: Option<crate::state_hub::SharedStateHub>,
        replay_disk_snapshots: bool,
    ) -> Self {
        let workdir = root.as_ref().to_path_buf();
        let refresh_rate = configured_tui_refresh_rate(&workdir);
        let terminal_size = size().unwrap_or((80, 24));
        let mut scaffold = DashboardScaffold::new_in(&workdir);
        if let Some(page) = initial_page {
            let _ = scaffold.set_active_page(page);
        }
        // When connected to a live StateHub, skip disk loading — the hub is
        // the source of truth and will populate state via snapshot events.
        // Only fall back to disk for the standalone `roko dashboard` command.
        let data = if state_hub.is_some() {
            DashboardData::default()
        } else {
            DashboardData::load_best_effort(&workdir)
        };
        let last_data_gen = data.generation;
        let mut tui_state = TuiState::new();
        tui_state.update_from_snapshot(&data);
        tui_state.workdir = workdir.clone();
        // Warm the config editor cache so the first F6 press (and headless
        // --snapshot draws) renders config fields, not an empty editor.
        tui_state.invalidate_config_cache();
        tui_state.run_started = Some(Instant::now());
        tui_state.refresh_mcp_config_view();
        tui_state.refresh_conductor_snapshot();

        let mut app = Self {
            workdir,
            tui_state,
            fx_config: EffectsConfig::default(),
            pfx_bufs: super::postfx_pipeline::PostFxBuffers::default(),
            notifications: VecDeque::new(),
            scroll_accel: super::scroll::ScrollAccel::new(),
            current_page: scaffold.active_page(),
            data,
            scaffold,
            last_data_gen,
            verdicts_aggregator: None,
            running: true,
            last_refresh: Instant::now(),
            scroll_offset: HashMap::new(),
            signal_selection: 0,
            gate_failure_selection: 0,
            sys_rx: None,
            agent_topology_rx: None,
            agent_topology_in_flight: false,
            fs_watch: None,
            theme: Theme::from_env(),
            git_watch: None,
            process_supervisor: None,
            approval_rx: None,
            pending_approval_response: None,
            _state_hub: state_hub,
            replay_disk_snapshots,
            shutdown_rx: None,
            exit_on_plan_completion: false,
            capture_mouse: true,
            tab_transition: None,
            connected_plan_observed: false,
            snapshot_rx: None,
            state_events: None,
            last_snapshot_error_marker: None,
            last_seen_gate_count: 0,
            last_seen_plan_phases: HashMap::new(),
            agent_stream_clients: HashMap::new(),
            agent_stream_server_url: resolve_agent_stream_server_url(),
            agent_stream_auth_token: resolve_agent_stream_auth_token(),
            refresh_rate,
            terminal_size,
            pending_refresh: false,
            render_dirty: RenderDirty::NONE,
            frame_stats: FrameStats::default(),
            last_snapshot_hash: None,
            last_events_offset: 0,
            exec_cmd_sender: None,
            exec_ack_receiver: None,
            pending_exec_commands: std::collections::HashMap::new(),
            git_bg_rx: None,
            git_bg_generation: 0,
            git_applied_generation: 0,
            surface_event_tx: tokio::sync::broadcast::channel(64).0,
        };
        app.fx_config = EffectsConfig::load_from_root(&app.workdir);
        // Verdicts aggregator starts as None and is populated on the first
        // async tick (open/tick are now async — cannot call from sync ctor).

        // First-run detection: show welcome modal if roko.toml is absent
        // and .roko/ directory doesn't exist.
        let roko_toml = app.workdir.join("roko.toml");
        let roko_dir = app.workdir.join(".roko");
        if !roko_toml.exists() && !roko_dir.exists() {
            app.tui_state.active_modal = Some(ModalState::Welcome { initialized: false });
        }

        app
    }

    /// Build a new app connected to a shared `StateHub`.
    #[must_use]
    pub fn new_connected(
        root: impl AsRef<Path>,
        state_hub: &crate::state_hub::SharedStateHub,
    ) -> Self {
        Self::new_connected_with_page(root, None, state_hub)
    }

    /// Build a new connected app with an optional initial page selection.
    #[must_use]
    pub fn new_connected_with_page(
        root: impl AsRef<Path>,
        initial_page: Option<PageId>,
        state_hub: &crate::state_hub::SharedStateHub,
    ) -> Self {
        Self::new_connected_with_state_hub(root, initial_page, state_hub.clone(), false)
    }

    fn new_connected_with_state_hub(
        root: impl AsRef<Path>,
        initial_page: Option<PageId>,
        state_hub: crate::state_hub::SharedStateHub,
        replay_disk_snapshots: bool,
    ) -> Self {
        let mut app = Self::new_with_page_inner(
            root,
            initial_page,
            Some(state_hub.clone()),
            replay_disk_snapshots,
        );
        let snapshot_rx = state_hub.snapshot();
        // Verdicts aggregator starts as None; populated on first async tick.
        if snapshot_has_content(&snapshot_rx.borrow()) {
            let snapshot = snapshot_rx.borrow();
            apply_dashboard_snapshot(
                &mut app.tui_state,
                &mut app.notifications,
                &mut app.last_snapshot_error_marker,
                &mut app.last_seen_gate_count,
                &mut app.last_seen_plan_phases,
                &snapshot,
            );
        }
        app.snapshot_rx = Some(snapshot_rx);
        // Subscribe after capturing the initial snapshot so replay cannot
        // duplicate already-materialized output. The live event bus is the
        // authoritative stream for text and tool records.
        app.state_events =
            Some(state_hub.subscribe_events_from(state_hub.cursor_snapshot().next_seq));
        app
    }

    /// Build a headless app from one materialized dashboard snapshot.
    ///
    /// Screenshot and evidence tooling uses this constructor so it exercises
    /// the exact same `App::draw` path as the interactive TUI without needing
    /// a long-lived watch receiver.
    #[must_use]
    pub fn new_with_dashboard_snapshot(
        root: impl AsRef<Path>,
        snapshot: &roko_core::DashboardSnapshot,
    ) -> Self {
        let mut app = Self::new_with_page_inner(root, None, None, false);
        apply_dashboard_snapshot(
            &mut app.tui_state,
            &mut app.notifications,
            &mut app.last_snapshot_error_marker,
            &mut app.last_seen_gate_count,
            &mut app.last_seen_plan_phases,
            snapshot,
        );
        // A materialized capture has no previous frame, so historical gate,
        // plan, and error records are state rather than new toast events.
        // Recreating them on every continuous capture would obscure the very
        // content the evidence tooling is meant to inspect.
        app.notifications.clear();
        app
    }

    /// Remove transient overlays before a one-shot headless capture.
    ///
    /// Standalone construction replays persisted events to build the current
    /// state. Those events should remain in their panels, but must not be
    /// presented as freshly-arrived notifications in a static screenshot.
    pub(in crate::tui) fn prepare_headless_capture(&mut self) {
        self.notifications.clear();
    }

    /// Configure this app to exit when the provided shutdown signal arrives.
    #[must_use]
    pub fn with_shutdown_receiver(mut self, shutdown_rx: std_mpsc::Receiver<()>) -> Self {
        self.shutdown_rx = Some(shutdown_rx);
        self
    }

    /// Configure this app to close automatically after its connected run ends.
    #[must_use]
    pub const fn with_exit_on_plan_completion(mut self) -> Self {
        self.exit_on_plan_completion = true;
        self
    }

    /// Leave SIGINT/SIGTERM to the embedding host instead of the
    /// reset-and-reraise terminal handler. The host (e.g. `roko plan run`)
    /// cancels its work, stops this TUI through the shutdown receiver, and
    /// exits with the signal's status; SIGHUP keeps the terminal handler.
    #[must_use]
    pub fn with_host_termination_signals(self) -> Self {
        HOST_HANDLES_TERMINATION_SIGNALS.store(true, Ordering::SeqCst);
        self
    }

    /// Attach the executor-neutral command sender and acknowledgement
    /// receiver (#233). This replaces the legacy `with_tui_command_tx`.
    #[must_use]
    pub fn with_execution_command_sender(
        mut self,
        sender: crate::execution_control::ExecutionCommandSender,
        ack_rx: crate::execution_control::CommandAckReceiver,
    ) -> Self {
        self.exec_cmd_sender = Some(sender);
        self.exec_ack_receiver = Some(ack_rx);
        self
    }

    /// Configure this app to run without terminal mouse reporting.
    ///
    /// This is useful for embedded approval UIs owned by long-running commands:
    /// keyboard controls still work, and abnormal process termination cannot
    /// leave the caller's shell printing mouse escape sequences.
    #[must_use]
    pub const fn without_mouse_capture(mut self) -> Self {
        self.capture_mouse = false;
        self
    }

    /// Render every tab headlessly into a `Vec<(Tab, String)>`.
    ///
    /// Creates a [`TestBackend`] of the given dimensions, renders each tab
    /// in sequence, and extracts the buffer contents as plain text.
    pub fn render_all_tabs_to_text(&mut self, width: u16, height: u16) -> Vec<(Tab, String)> {
        self.render_tabs_to_text(width, height, &Tab::ALL)
    }

    /// Render the requested tabs through the complete application frame.
    ///
    /// This includes the global header, warning bar, footer, layout chrome,
    /// active effects, and view contents. Callers must validate non-zero
    /// dimensions before invoking this helper.
    pub fn render_tabs_to_text(
        &mut self,
        width: u16,
        height: u16,
        tabs: &[Tab],
    ) -> Vec<(Tab, String)> {
        use ratatui::backend::TestBackend;

        if width == 0 || height == 0 {
            return Vec::new();
        }

        let previous_tab = self.tui_state.active_tab;
        let rendered = tabs
            .iter()
            .map(|&tab| {
                self.tui_state.active_tab = tab;
                let backend = TestBackend::new(width, height);
                let mut terminal = Terminal::new(backend).expect("TestBackend terminal");
                match terminal.draw(|frame| self.draw(frame)) {
                    Ok(_) => {
                        let buffer = terminal.backend().buffer();
                        let w = buffer.area.width as usize;
                        let text = buffer
                            .content
                            .chunks(w)
                            .map(|row| {
                                row.iter()
                                    .map(|cell| cell.symbol())
                                    .collect::<String>()
                                    .trim_end()
                                    .to_string()
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        (tab, text)
                    }
                    Err(e) => {
                        tracing::error!(?tab, error = %e, "tab render failed");
                        // Re-render with a fallback error message so the
                        // snapshot still contains something useful instead of
                        // crashing the whole process.
                        let fallback = TestBackend::new(width, height);
                        let mut term2 = Terminal::new(fallback).expect("TestBackend terminal");
                        let msg = format!("Render error: {e}");
                        let _ = term2.draw(|frame| {
                            frame.render_widget(
                                Paragraph::new(msg.clone())
                                    .style(Style::default().fg(Theme::WARNING))
                                    .alignment(Alignment::Center),
                                frame.area(),
                            );
                        });
                        let buffer = term2.backend().buffer();
                        let w = buffer.area.width as usize;
                        let text = buffer
                            .content
                            .chunks(w)
                            .map(|row| {
                                row.iter()
                                    .map(|cell| cell.symbol())
                                    .collect::<String>()
                                    .trim_end()
                                    .to_string()
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        (tab, text)
                    }
                }
            })
            .collect();
        self.tui_state.active_tab = previous_tab;
        rendered
    }

    /// Install a live process supervisor used for per-agent process metrics.
    pub fn set_process_supervisor(&mut self, supervisor: Arc<ProcessSupervisor>) {
        self.process_supervisor = Some(supervisor);
    }

    /// Subscribe to the SurfaceEvent broadcast channel (P1-40).
    ///
    /// Returns a receiver that yields every `SurfaceEvent` produced by TUI
    /// actions (approve, reject, pause, cancel, etc.).
    pub fn subscribe_surface_events(
        &self,
    ) -> tokio::sync::broadcast::Receiver<roko_core::runtime_event::SurfaceEvent> {
        self.surface_event_tx.subscribe()
    }

    /// Emit a `SurfaceEvent` from a TUI action (P1-40).
    ///
    /// Best-effort: drops the event silently if no receivers are subscribed.
    fn emit_surface_event(&self, event: roko_core::runtime_event::SurfaceEvent) {
        let _ = self.surface_event_tx.send(event);
    }

    /// Return the active page (legacy).
    #[must_use]
    pub const fn current_page(&self) -> PageId {
        self.current_page
    }

    /// Return the active page (legacy).
    #[must_use]
    pub const fn active_page(&self) -> PageId {
        self.current_page
    }

    fn pages(&self) -> PageRegistry {
        PageRegistry::from_dashboard(&self.scaffold)
    }

    fn clamp_signal_selection(&mut self) {
        let len = self.tui_state.recent_signals.len();
        if len == 0 {
            self.signal_selection = 0;
        } else if self.signal_selection >= len {
            self.signal_selection = len - 1;
        }
    }

    fn clamp_gate_failure_selection(&mut self) {
        let len = self.tui_state.gate_results_page.failure_rows.len();
        if len == 0 {
            self.gate_failure_selection = 0;
        } else if self.gate_failure_selection >= len {
            self.gate_failure_selection = len - 1;
        }
    }

    fn enter_terminal(&self) -> Result<TuiTerminal> {
        enable_raw_mode().context("enable raw mode")?;
        let mut stdout = io::stdout();
        if self.capture_mouse {
            execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
                .context("enter alternate screen")?;
        } else {
            execute!(stdout, EnterAlternateScreen).context("enter alternate screen")?;
        }
        Terminal::new(CrosstermBackend::new(stdout)).context("create terminal")
    }

    fn cleanup_terminal() -> Result<()> {
        Self::cleanup_terminal_best_effort();
        Ok(())
    }

    fn cleanup_terminal_best_effort() {
        TERMINAL_CLEANUP_ACTIVE.store(false, Ordering::SeqCst);

        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(
            stdout,
            DisableMouseCapture,
            LeaveAlternateScreen,
            cursor::Show
        );
        let _ = stdout.write_all(TERMINAL_RESET_SEQUENCE);
        let _ = stdout.flush();
    }
}
