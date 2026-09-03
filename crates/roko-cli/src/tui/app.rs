//! Interactive TUI application shell.
//!
//! Integrates the Mori-style tab system (F1-F7), modal dialogs, TuiState,
//! TuiAction dispatch, PostFX pipeline, and atmosphere animations.

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
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};
use ratatui::{Frame, Terminal};

use roko_runtime::process::ProcessSupervisor;
use sysinfo::{Disks, Networks, Pid, ProcessStatus, ProcessesToUpdate, System};
use tokio::sync::{mpsc, oneshot, watch};

use super::approval_ipc::ApprovalRequest;
use super::dashboard::{DashboardData, DashboardScaffold, Theme};
use super::effects_config::EffectsConfig;
use super::event::{Event, EventHandler, FrameStats, RenderDirty, TickPolicyInputs, next_tick_policy};
use super::fs_watch::{self, FsRefresh, FsWatchHandle};
use super::git_watch::{self, GitRefresh, GitWatchHandle};
use super::input::{self, ConfirmAction, FocusZone, InputMode, TuiAction};
use super::modals::{
    self, Milestone, ModalState, QueueTask, TaskPickerRow, WaveInfo, WavePlanEntry,
};
use super::pages::{PageId, PageRegistry};
use super::state::{PendingApproval, PlanEntry, TaskRowStatus, TuiState};
use super::tabs::Tab;
use super::verdicts::VerdictsAggregator;
use super::views::{self, ViewState};
use super::ws_client::{AgentStreamClient, StreamChunk};

/// Interactive dashboard shell backed by the existing snapshot renderer.
///
/// Supports two rendering paths:
/// - **Mori-style tabs** (F1-F7): full TuiState + views + modals + postfx
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
    /// Receiver for background git data collection results.
    git_bg_rx: Option<std::sync::mpsc::Receiver<(u64, GitBgData)>>,
    /// Monotonically increasing generation counter for spawned git jobs.
    git_bg_generation: u64,
    /// Highest generation that has been applied to the TUI state.
    git_applied_generation: u64,
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
        let _ = libc::signal(libc::SIGINT, handler);
        let _ = libc::signal(libc::SIGTERM, handler);
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

/// Run the interactive dashboard event loop (async variant).
///
/// Uses the same adaptive tick policy and `RenderDirty` reason accounting
/// as the sync `main_loop()`. Draws only when dirty state has accumulated
/// after coalescing all ready inputs.
pub async fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
    // Populate verdicts aggregator on first async entry.
    app.reseed_verdicts_aggregator().await;
    app.refresh_verdicts_from_aggregator().await;

    // Initial draw
    terminal.draw(|f| app.draw(f))?;

    loop {
        // -- Adaptive tick: select policy duration --
        let tick_duration = app.current_tick_duration();

        // -- Poll terminal events with the adaptive timeout --
        if crossterm::event::poll(tick_duration)? {
            match crossterm::event::read()? {
                crossterm::event::Event::Key(key) => {
                    app.frame_stats.record_input();
                    app.render_dirty.insert(RenderDirty::INPUT);
                    app.handle_key(key);
                }
                crossterm::event::Event::Mouse(mouse) => {
                    app.frame_stats.record_input();
                    app.render_dirty.insert(RenderDirty::INPUT);
                    app.handle_mouse(mouse);
                }
                crossterm::event::Event::Resize(width, height) => {
                    app.terminal_size = (width, height);
                    app.render_dirty.insert(RenderDirty::RESIZE);
                }
                _ => {}
            }
        } else {
            // Tick (timeout expired with no input): check for animation.
            let animated = app.tui_state.agents.iter().any(|a| a.active)
                || app.tui_state.plans.iter().any(|p| p.active)
                || app.has_modal()
                || !app.notifications.is_empty();
            if animated {
                app.tui_state.atmosphere.tick();
                app.render_dirty.insert(RenderDirty::ANIMATION);
            }
        }

        // -- Drain pending async refresh requests from sync dispatch_action --
        if app.pending_refresh {
            app.pending_refresh = false;
            app.refresh_snapshot_async().await;
        }

        // -- Coalesce: drain all channels --
        app.drain_snapshot_channel();
        app.drain_approval_requests();

        if !app.running {
            break;
        }

        // -- Draw only when dirty; record stats --
        if app.render_dirty.is_dirty() {
            let draw_reasons = app.render_dirty;
            let input_pending =
                app.render_dirty.contains(RenderDirty::INPUT) && app.frame_stats.last_input_at.is_some();
            let draw_start = Instant::now();
            terminal.draw(|f| app.draw(f))?;
            let draw_elapsed = draw_start.elapsed();
            app.frame_stats.record_frame(draw_elapsed, draw_reasons);
            if input_pending {
                if let Some(input_at) = app.frame_stats.last_input_at {
                    app.frame_stats.record_input_to_draw(input_at.elapsed());
                }
            }
            app.render_dirty.remove(draw_reasons);
        } else {
            app.frame_stats.record_skip();
        }
    }
    Ok(())
}

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
            capture_mouse: false,
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
            git_bg_rx: None,
            git_bg_generation: 0,
            git_applied_generation: 0,
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
    pub(super) fn prepare_headless_capture(&mut self) {
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

    /// Attach a sender for in-process TUI commands to the runner event loop.
    ///
    /// **Deprecated**: prefer [`with_execution_command_sender`] for new code.
    #[must_use]
    #[allow(deprecated)]
    pub fn with_tui_command_tx(
        mut self,
        tx: tokio::sync::mpsc::Sender<crate::runner::TuiCommand>,
    ) -> Self {
        // Legacy path: wrap the raw mpsc sender in the new transport.
        // Callers that still pass a raw TuiCommand sender get the same
        // behavior, but the TUI internally uses ExecutionCommandSender.
        let _ = tx;
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
                terminal.draw(|frame| self.draw(frame)).expect("draw tab");
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
            })
            .collect();
        self.tui_state.active_tab = previous_tab;
        rendered
    }

    /// Install a live process supervisor used for per-agent process metrics.
    pub fn set_process_supervisor(&mut self, supervisor: Arc<ProcessSupervisor>) {
        self.process_supervisor = Some(supervisor);
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

    /// Run the terminal UI until the user quits.
    pub fn run(mut self) -> Result<()> {
        let log_path = tui_log_path(&self.workdir);
        let log_dispatch =
            tui_log_dispatch(&self.workdir).context("initialize TUI file logging")?;
        let _log_guard = tracing::dispatcher::set_default(&log_dispatch);
        tracing::info!(path = %log_path.display(), "TUI file logging enabled");
        tracing::info!(
            connected = self._state_hub.is_some(),
            exit_on_plan_completion = self.exit_on_plan_completion,
            "TUI session started"
        );

        let previous_hook: Arc<dyn Fn(&std::panic::PanicHookInfo<'_>) + Send + Sync + 'static> =
            Arc::from(std::panic::take_hook());
        let panic_hook = Arc::clone(&previous_hook);
        let _restore_hook = PanicHookRestoreGuard(previous_hook);

        std::panic::set_hook(Box::new(move |panic_info| {
            Self::cleanup_terminal_best_effort();
            panic_hook(panic_info);
        }));

        let mut terminal_guard = TerminalCleanupGuard::arm();
        let mut terminal = self.enter_terminal()?;
        let result = self.main_loop(&mut terminal);
        if let Err(e) = &result {
            tracing::info!(error = %e, "TUI exiting: error");
        }
        let cleanup = terminal_guard.restore();

        match (result, cleanup) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(err), Ok(())) => Err(err),
            (Ok(()), Err(err)) => Err(err),
            (Err(err), Err(_cleanup_err)) => Err(err),
        }
    }

    fn main_loop(&mut self, terminal: &mut TuiTerminal) -> Result<()> {
        let mut events = EventHandler::new(self.refresh_rate);
        let log_dispatch = tracing::dispatcher::get_default(|dispatch| dispatch.clone());

        // ---------------------------------------------------------------
        // Spawn background sys metrics collector thread
        // ---------------------------------------------------------------
        let (sys_tx, sys_rx) = watch::channel(SysSnapshot::default());
        let sys_log_dispatch = log_dispatch.clone();
        let process_supervisor = self.process_supervisor.clone();
        std::thread::Builder::new()
            .name("tui-sys-metrics".into())
            .spawn(move || {
                let _log_guard = tracing::dispatcher::set_default(&sys_log_dispatch);
                collect_sys_metrics_bg(sys_tx, process_supervisor);
            })
            .inspect_err(|err| {
                tracing::warn!(
                    error = %err,
                    thread = "tui-sys-metrics",
                    "failed to spawn background thread"
                );
            })
            .ok(); // graceful: TUI works without background thread
        self.sys_rx = Some(sys_rx);

        // ---------------------------------------------------------------
        // Start debounced `.roko/` watcher with polling fallback
        // ---------------------------------------------------------------
        if self._state_hub.is_none() || self.replay_disk_snapshots {
            self.fs_watch = Some(fs_watch::watch_roko_dir_with_fallback(&self.workdir));
        }

        // ---------------------------------------------------------------
        // Prime git data once via background thread, then refresh only
        // when git metadata changes.  The first drain_background_channels()
        // call will pick up the result.
        // ---------------------------------------------------------------
        {
            self.git_bg_generation += 1;
            let generation = self.git_bg_generation;
            let workdir = self.workdir.clone();
            let (tx, rx) = std::sync::mpsc::sync_channel(1);
            self.git_bg_rx = Some(rx);
            std::thread::Builder::new()
                .name("tui-git-collect".into())
                .spawn(move || {
                    let data = collect_git_bg_data(&workdir);
                    let _ = tx.send((generation, data));
                })
                .ok();
        }
        self.git_watch = Some(git_watch::watch_git_repo_with_fallback(&self.workdir));

        // ---------------------------------------------------------------
        // Populate verdicts aggregator (sync path — no Tokio runtime)
        // ---------------------------------------------------------------
        self.reseed_verdicts_aggregator_blocking();
        self.refresh_verdicts_from_aggregator_blocking();

        // ---------------------------------------------------------------
        // Initial draw
        // ---------------------------------------------------------------
        terminal
            .draw(|frame| self.draw(frame))
            .context("initial TUI draw")?;
        let mut last_draw = Instant::now();

        // ---------------------------------------------------------------
        // Event loop — adaptive tick policy with dirty-flag rendering
        // ---------------------------------------------------------------
        while self.running {
            // -- 1. Shutdown check --
            self.drain_shutdown_signal();
            if !self.running {
                break;
            }

            // -- 2. Adaptive tick rate: select policy, update EventHandler --
            let policy = next_tick_policy(&self.tick_policy_inputs());
            events.set_tick_rate(policy.duration());

            // -- 3. Wait for next event (blocks up to the policy duration) --
            match events.next().context("poll TUI event")? {
                Event::Key(key) => {
                    self.frame_stats.record_input();
                    self.render_dirty.insert(RenderDirty::INPUT);
                    self.handle_key(key);
                    // Handle deferred refresh requests from dispatch_action.
                    if self.pending_refresh {
                        self.pending_refresh = false;
                        self.refresh_snapshot();
                    }
                }
                Event::Mouse(mouse) => {
                    self.frame_stats.record_input();
                    self.render_dirty.insert(RenderDirty::INPUT);
                    self.handle_mouse(mouse);
                }
                Event::Resize(width, height) => {
                    self.terminal_size = (width, height);
                    self.render_dirty.insert(RenderDirty::RESIZE);
                }
                Event::Tick => {
                    // Tick: check for animation state that warrants a redraw.
                    let animated = self.tui_state.agents.iter().any(|a| a.active)
                        || self.tui_state.plans.iter().any(|p| p.active)
                        || self.has_modal()
                        || !self.notifications.is_empty();
                    if animated {
                        self.tui_state.atmosphere.tick();
                        self.render_dirty.insert(RenderDirty::ANIMATION);
                    }
                    // Handle deferred refresh requests from dispatch_action.
                    if self.pending_refresh {
                        self.pending_refresh = false;
                        self.refresh_snapshot();
                    }
                }
            }

            // -- 4. Coalesce: drain all background channels --
            self.drain_approval_requests();
            self.drain_background_channels();

            // -- 5. Notification expiry --
            let notification_count = self.notifications.len();
            self.expire_notifications();
            if notification_count != self.notifications.len() {
                self.render_dirty.insert(RenderDirty::NOTIFICATION);
            }

            // -- 6. Health fallback: disk-backed dashboards without a
            //    StateHub should still refresh periodically. --
            if self._state_hub.is_none() && last_draw.elapsed() >= Duration::from_secs(1) {
                self.render_dirty.insert(RenderDirty::FORCED_HEALTH);
            }

            // -- 7. Draw only when dirty; record stats --
            if !self.running {
                break;
            }
            if self.render_dirty.is_dirty() {
                let draw_reasons = self.render_dirty;
                let input_pending =
                    self.render_dirty.contains(RenderDirty::INPUT) && self.frame_stats.last_input_at.is_some();
                let draw_start = Instant::now();
                terminal
                    .draw(|frame| self.draw(frame))
                    .context("TUI redraw")?;
                let draw_elapsed = draw_start.elapsed();
                self.frame_stats.record_frame(draw_elapsed, draw_reasons);
                if input_pending {
                    if let Some(input_at) = self.frame_stats.last_input_at {
                        self.frame_stats.record_input_to_draw(input_at.elapsed());
                    }
                }
                // Clear only the reasons that were included in this draw.
                self.render_dirty.remove(draw_reasons);
                last_draw = Instant::now();
            } else {
                self.frame_stats.record_skip();
            }
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame<'_>) {
        let theme = self.theme;
        let full_area = frame.area();

        // Establish a real canvas every frame.  Relying on terminal defaults
        // leaves stale cells and makes post-processing dependent on the host
        // theme; Mori's visual hierarchy starts from an explicit black field.
        frame.render_widget(Block::default().style(Theme::block_style()), full_area);

        // Guard: if the terminal is too small for useful rendering, show a
        // short message instead of attempting layout that would panic or clip.
        if super::layout::is_terminal_too_small(full_area) {
            let msg = format!(
                "{}x{} -- resize to 60x10+",
                full_area.width, full_area.height
            );
            frame.render_widget(
                Paragraph::new(msg)
                    .style(Style::default().fg(Theme::WARNING))
                    .alignment(Alignment::Center),
                full_area,
            );
            return;
        }

        // Responsive outer margin on large terminals
        let content_area = super::layout::responsive_outer_margin(full_area);

        // Main layout: header + warning + wave + optional sub-view bar +
        // content + footer. Dashboard and Agents already render their own
        // purpose-built internal navigation bars.
        let has_waves = !self.tui_state.execution_waves.is_empty();
        let wave_row_height = if has_waves { 1 } else { 0 };
        let warning_height = super::widgets::header_bar::warning_bar_height(&self.tui_state);
        // Show the sub-view bar when the tab has more than one sub-view.
        let sub_views = views::SubView::for_tab(self.tui_state.active_tab);
        let subview_height = u16::from(sub_views.len() > 1);
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),               // [0] Mori-style header bar
                Constraint::Length(warning_height),  // [1] Warning bar (0 when no warnings)
                Constraint::Length(wave_row_height), // [2] Wave indicator row (hidden when idle)
                Constraint::Length(1),               // [3] Breadcrumb trail
                Constraint::Length(subview_height),  // [4] Reachable Alt+number sub-views
                Constraint::Min(0),                  // [5] Content area
                Constraint::Length(1),               // [6] Status footer
            ])
            .split(content_area);

        // Header: Mori header bar
        self.render_tab_header(frame, main_layout[0], &theme);

        // Warning bar (only when warnings are active)
        if warning_height > 0 {
            super::widgets::header_bar::render_warning_bar(frame, main_layout[1], &self.tui_state);
        }

        // Wave indicator row (only when waves exist)
        if has_waves {
            super::widgets::wave_progress::render_wave_progress(
                frame,
                main_layout[2],
                &self.tui_state,
            );
        }

        // Breadcrumb trail: Tab > SubView > Focus
        super::widgets::header_bar::render_breadcrumb_bar(
            frame,
            main_layout[3],
            &self.tui_state,
        );

        if subview_height > 0 {
            self.render_subview_bar(frame, main_layout[4], &theme);
        }

        // Content: dispatch to active tab view
        // Layout: [0]=header [1]=warning [2]=wave [3]=breadcrumb [4]=subview
        //         [5]=content [6]=footer
        let content_idx = 5;
        let footer_idx = 6;
        let (content_area, input_area) = self.split_content_area(main_layout[content_idx]);

        self.clamp_scroll_state_to_view();
        // Honor the config cache TTL per draw so the F6 tab stays fresh even
        // when no refresh tick fires (also covers headless --snapshot draws).
        if self.tui_state.active_tab == Tab::Config && self.tui_state.config_needs_refresh() {
            self.tui_state.invalidate_config_cache();
        }
        let view_state = self.current_view_state();
        views::render_tab_content(
            frame,
            content_area,
            self.tui_state.active_tab,
            &self.data,
            &self.tui_state,
            &view_state,
            &theme,
        );

        // Footer: status line
        self.render_status_footer(frame, main_layout[footer_idx], &theme);

        if let Some(input_area) = input_area {
            self.render_input_bar(frame, input_area, &theme);
        }

        // Visual effects are part of the scene, never a layer above controls.
        // Apply them after ordinary widgets (so they can respect occupied
        // cells) but before modal dimming and modal content.
        if self.fx_config.screen_postfx {
            let buf = frame.buffer_mut();
            super::postfx_pipeline::apply_pipeline(
                self.tui_state.active_tab as usize,
                content_area,
                buf,
                self.tui_state.atmosphere.elapsed,
                self.tui_state.atmosphere.frame_count,
                &self.fx_config,
                &self.tui_state,
                &mut self.pfx_bufs,
            );
        }

        // Tab transition: brief fade-in when switching views.
        if let Some((started, duration)) = self.tab_transition {
            let progress = started.elapsed().as_secs_f64() / duration.as_secs_f64();
            if progress >= 1.0 {
                self.tab_transition = None;
            } else {
                // Ease-out cubic: fast start, smooth deceleration.
                let t = 1.0 - (1.0 - progress).powi(3);
                super::postfx::fade_overlay(content_area, frame.buffer_mut(), t);
            }
        }

        // Dim overlay before modals
        if self.tui_state.active_modal.is_some() {
            let buf = frame.buffer_mut();
            super::postfx::dim_overlay(content_area, buf, 0.45);
        }

        // Modal rendering
        modals::render_modals(
            frame,
            full_area,
            self.tui_state.active_modal.as_ref(),
            &self.tui_state,
            &self.data,
            &self.notifications,
            &theme,
            self.fx_config.screen_postfx,
        );
    }

    // -----------------------------------------------------------------------
    // Key handling
    // -----------------------------------------------------------------------

    fn handle_key(&mut self, key: KeyEvent) {
        if key.code == crossterm::event::KeyCode::Esc {
            self.scroll_accel.reset();
        }

        // Route through the full TuiAction dispatch
        let action = input::handle_key(
            key,
            self.tui_state.input_mode,
            self.tui_state.active_tab,
            self.tui_state.focus,
            &input::ModalVisibility::from_active_modal(self.tui_state.active_modal.as_ref()),
        );

        self.dispatch_action(action);
    }

    fn dispatch_action(&mut self, action: TuiAction) {
        match action {
            TuiAction::Quit => {
                if self.has_modal() {
                    self.dismiss_all_modals();
                } else {
                    self.tui_state.input_mode = InputMode::Confirm;
                    self.tui_state.active_modal = Some(ModalState::Quit);
                }
            }
            TuiAction::QuitConfirmed => {
                tracing::info!("TUI exiting: user quit");
                self.running = false;
            }
            TuiAction::SwitchTab(tab) => {
                let previous_tab = self.tui_state.active_tab;
                self.tui_state.active_tab = tab;
                self.tui_state.focus = match tab {
                    Tab::Dashboard | Tab::Plans => FocusZone::PlanTree,
                    Tab::Agents => FocusZone::AgentOutput,
                    Tab::Git => FocusZone::GitBranches,
                    Tab::Logs => FocusZone::LogList,
                    Tab::Config => FocusZone::ConfigKeys,
                    Tab::Inspect => FocusZone::InspectTree,
                    Tab::Marketplace => FocusZone::MarketList,
                    Tab::Atelier => FocusZone::AtelierList,
                    Tab::Learning => FocusZone::LearningMetrics,
                };
                // Sync legacy page
                if let Some(page_id) = tab_to_page(tab) {
                    self.current_page = page_id;
                    let _ = self.scaffold.set_active_page(page_id);
                }
                // Warm the config editor cache on tab entry so the first F6
                // render shows config fields instead of only Runtime sections.
                if matches!(tab, Tab::Config) && self.tui_state.config_needs_refresh() {
                    self.tui_state.invalidate_config_cache();
                }
                if matches!(tab, Tab::Agents) && !matches!(previous_tab, Tab::Agents) {
                    self.request_agent_topology_refresh();
                } else if !matches!(tab, Tab::Agents) {
                    self.tui_state.close_agent_topology();
                }
                // Start a subtle fade-in transition when switching tabs.
                if previous_tab != tab && self.fx_config.screen_postfx {
                    self.tab_transition =
                        Some((Instant::now(), Duration::from_millis(200)));
                }
            }
            TuiAction::FocusNext => {
                self.tui_state.focus = self.tui_state.focus.next(self.tui_state.active_tab);
            }
            TuiAction::FocusPrev => {
                self.tui_state.focus = self.tui_state.focus.prev(self.tui_state.active_tab);
            }
            TuiAction::SelectPlanUp => {
                if matches!(self.tui_state.active_tab, Tab::Agents)
                    && !matches!(
                        self.tui_state.focus,
                        FocusZone::AgentOutput | FocusZone::RightPanel
                    )
                {
                    self.tui_state.selected_agent = self.tui_state.selected_agent.saturating_sub(1);
                } else {
                    self.move_selected_plan(-1);
                }
            }
            TuiAction::SelectPlanDown => {
                if matches!(self.tui_state.active_tab, Tab::Agents)
                    && !matches!(
                        self.tui_state.focus,
                        FocusZone::AgentOutput | FocusZone::RightPanel
                    )
                {
                    let max = self.tui_state.agents.len().saturating_sub(1);
                    if self.tui_state.selected_agent < max {
                        self.tui_state.selected_agent += 1;
                    }
                } else {
                    self.move_selected_plan(1);
                }
            }
            TuiAction::SelectPlanByIndex(index) => {
                let visible = self.visible_plan_indices();
                if let Some(&plan_idx) = visible.get(index) {
                    self.tui_state.selected_plan_idx = plan_idx;
                }
            }
            TuiAction::TaskPickerUp => {
                if let Some(ModalState::TaskPicker {
                    selected_index,
                    scroll_offset,
                    ..
                }) = self.tui_state.active_modal.as_mut()
                {
                    *selected_index = selected_index.saturating_sub(1);
                    *scroll_offset = (*selected_index).min(u16::MAX as usize) as u16;
                }
            }
            TuiAction::TaskPickerDown => {
                if let Some(ModalState::TaskPicker {
                    selected_index,
                    scroll_offset,
                    tasks,
                    ..
                }) = self.tui_state.active_modal.as_mut()
                {
                    let max = tasks.len().saturating_sub(1);
                    *selected_index = selected_index.saturating_add(1).min(max);
                    *scroll_offset = (*selected_index).min(u16::MAX as usize) as u16;
                }
            }
            TuiAction::ScrollFocusedUp
                if matches!(self.tui_state.active_modal, Some(ModalState::Help)) =>
            {
                self.tui_state.help_scroll = self.tui_state.help_scroll.saturating_sub(1);
            }
            TuiAction::ScrollFocusedDown
                if matches!(self.tui_state.active_modal, Some(ModalState::Help)) =>
            {
                self.tui_state.help_scroll = self.tui_state.help_scroll.saturating_add(1);
            }
            TuiAction::ScrollPageUp
                if matches!(self.tui_state.active_modal, Some(ModalState::Help)) =>
            {
                self.tui_state.help_scroll = self.tui_state.help_scroll.saturating_sub(10);
            }
            TuiAction::ScrollPageDown
                if matches!(self.tui_state.active_modal, Some(ModalState::Help)) =>
            {
                self.tui_state.help_scroll = self.tui_state.help_scroll.saturating_add(10);
            }
            TuiAction::ScrollFocusedHome
                if matches!(self.tui_state.active_modal, Some(ModalState::Help)) =>
            {
                self.tui_state.help_scroll = 0;
            }
            TuiAction::ScrollFocusedEnd
                if matches!(self.tui_state.active_modal, Some(ModalState::Help)) =>
            {
                // Sentinel; render will clamp.
                self.tui_state.help_scroll = usize::MAX;
            }
            TuiAction::ScrollFocusedUp => {
                let delta = i32::from(self.scroll_accel.tick(-1));
                self.scroll_focused(delta);
            }
            TuiAction::ScrollFocusedDown => {
                let delta = i32::from(self.scroll_accel.tick(1));
                self.scroll_focused(delta);
            }
            TuiAction::ScrollPageUp => self.scroll_focused(-self.page_scroll_lines()),
            TuiAction::ScrollPageDown => self.scroll_focused(self.page_scroll_lines()),
            TuiAction::ScrollFocusedHome => self.set_focused_scroll(0),
            TuiAction::ScrollFocusedEnd => self.set_focused_scroll(usize::MAX),
            TuiAction::ScrollLogUp => {
                let delta = i32::from(self.scroll_accel.tick(-1));
                self.scroll_logs_by(delta);
            }
            TuiAction::ScrollLogDown => {
                let delta = i32::from(self.scroll_accel.tick(1));
                self.scroll_logs_by(delta);
            }
            TuiAction::ScrollLogEnd => {
                self.tui_state.log_auto_tail = true;
                self.tui_state.log_scroll = 0;
            }
            TuiAction::ToggleLogFilter(level) => {
                self.tui_state.toggle_log_filter_level(level);
                self.refresh_log_search_matches();
            }
            TuiAction::ShowAllLogFilters => {
                self.tui_state.show_all_log_filter_levels();
                self.refresh_log_search_matches();
            }
            TuiAction::ScrollAgentUp => {
                let delta = i32::from(self.scroll_accel.tick(-1));
                self.scroll_agent_output_by(delta);
            }
            TuiAction::ScrollAgentDown => {
                let delta = i32::from(self.scroll_accel.tick(1));
                self.scroll_agent_output_by(delta);
            }
            TuiAction::ScrollAgentEnd => {
                if self.tui_state.agent_topology_visible {
                    self.tui_state.agent_topology_scroll_offset =
                        self.current_agent_topology_max_scroll();
                } else {
                    self.tui_state.agent_scroll = None; // Resume auto-tail
                }
            }
            TuiAction::ScrollDiffUp => {
                let delta = self.scroll_accel.tick(-1);
                self.tui_state.diff_scroll =
                    Self::apply_signed_scroll(self.tui_state.diff_scroll, delta);
            }
            TuiAction::ScrollDiffDown => {
                let delta = self.scroll_accel.tick(1);
                self.tui_state.diff_scroll =
                    Self::apply_signed_scroll(self.tui_state.diff_scroll, delta);
            }
            TuiAction::ScrollDetailUp => {
                if matches!(
                    self.tui_state.active_modal,
                    Some(ModalState::PlanDetail { .. })
                ) {
                    self.tui_state.plan_detail_scroll =
                        self.tui_state.plan_detail_scroll.saturating_sub(1);
                } else if let Some(ModalState::TaskDetail { scroll_offset, .. }) =
                    self.tui_state.active_modal.as_mut()
                {
                    *scroll_offset = scroll_offset.saturating_sub(1);
                } else {
                    self.tui_state.plan_detail_scroll =
                        self.tui_state.plan_detail_scroll.saturating_sub(1);
                }
            }
            TuiAction::ScrollDetailDown => {
                if matches!(
                    self.tui_state.active_modal,
                    Some(ModalState::PlanDetail { .. })
                ) {
                    self.tui_state.plan_detail_scroll =
                        self.tui_state.plan_detail_scroll.saturating_add(1);
                } else if let Some(ModalState::TaskDetail { scroll_offset, .. }) =
                    self.tui_state.active_modal.as_mut()
                {
                    *scroll_offset = scroll_offset.saturating_add(1);
                } else {
                    self.tui_state.plan_detail_scroll =
                        self.tui_state.plan_detail_scroll.saturating_add(1);
                }
            }
            TuiAction::ModalScrollUp => {
                if let Some(modal) = self.tui_state.active_modal.as_mut() {
                    match modal {
                        ModalState::WaveOverview { scroll_offset, .. }
                        | ModalState::AgentPool { scroll_offset, .. }
                        | ModalState::BatchReview { scroll_offset, .. } => {
                            *scroll_offset = scroll_offset.saturating_sub(1);
                        }
                        ModalState::NotificationHistory {
                            scroll_offset,
                            selected_index,
                            ..
                        } => {
                            *selected_index = selected_index.saturating_sub(1);
                            *scroll_offset = scroll_offset.saturating_sub(1);
                        }
                        ModalState::QueueOverview {
                            selected_index,
                            ..
                        } => {
                            *selected_index = selected_index.saturating_sub(1);
                        }
                        ModalState::TaskPicker {
                            selected_index,
                            ..
                        } => {
                            *selected_index = selected_index.saturating_sub(1);
                        }
                        _ => {}
                    }
                }
            }
            TuiAction::ModalScrollDown => {
                if let Some(modal) = self.tui_state.active_modal.as_mut() {
                    match modal {
                        ModalState::WaveOverview { scroll_offset, .. }
                        | ModalState::AgentPool { scroll_offset, .. }
                        | ModalState::BatchReview { scroll_offset, .. } => {
                            *scroll_offset = scroll_offset.saturating_add(1);
                        }
                        ModalState::NotificationHistory {
                            scroll_offset,
                            selected_index,
                            ..
                        } => {
                            *selected_index = selected_index.saturating_add(1);
                            *scroll_offset = scroll_offset.saturating_add(1);
                        }
                        ModalState::QueueOverview {
                            selected_index,
                            ..
                        } => {
                            *selected_index = selected_index.saturating_add(1);
                        }
                        ModalState::TaskPicker {
                            selected_index,
                            ..
                        } => {
                            *selected_index = selected_index.saturating_add(1);
                        }
                        _ => {}
                    }
                }
            }
            TuiAction::QueueOverviewUp => {
                if let Some(ModalState::QueueOverview {
                    selected_index,
                    scroll_offset,
                    ..
                }) = self.tui_state.active_modal.as_mut()
                {
                    *selected_index = selected_index.saturating_sub(1);
                    *scroll_offset = (*selected_index).min(u16::MAX as usize) as u16;
                }
            }
            TuiAction::QueueOverviewDown => {
                if let Some(ModalState::QueueOverview {
                    selected_index,
                    scroll_offset,
                    milestones,
                }) = self.tui_state.active_modal.as_mut()
                {
                    let max = milestones.len().saturating_sub(1);
                    *selected_index = selected_index.saturating_add(1).min(max);
                    *scroll_offset = (*selected_index).min(u16::MAX as usize) as u16;
                }
            }
            TuiAction::CloseModal => {
                if self.has_modal() {
                    self.dismiss_all_modals();
                }
            }
            TuiAction::WelcomeInit => {
                // Initialize workspace: create .roko/ and default roko.toml
                let roko_dir = self.workdir.join(".roko");
                let roko_toml = self.workdir.join("roko.toml");
                if let Err(err) = std::fs::create_dir_all(&roko_dir) {
                    tracing::warn!(error = %err, "failed to create .roko/");
                }
                // Create subdirectories matching `roko init`
                for sub in &[
                    "state", "learn", "jobs", "prd", "prd/published", "prd/drafts",
                    "task-outputs", "research", "subscriptions", "templates",
                ] {
                    let _ = std::fs::create_dir_all(roko_dir.join(sub));
                }
                // Create default roko.toml if absent
                if !roko_toml.exists() {
                    let default_toml = "[agent]\neffort = \"standard\"\n\n[learning]\nenabled = true\n";
                    if let Err(err) = std::fs::write(&roko_toml, default_toml) {
                        tracing::warn!(error = %err, "failed to write roko.toml");
                    }
                }
                // Update workspace state
                self.tui_state.workdir = self.workdir.clone();
                self.tui_state.refresh_mcp_config_view();
                // Transition to the confirmation screen
                self.tui_state.active_modal =
                    Some(ModalState::Welcome { initialized: true });
                tracing::info!(workdir = %self.workdir.display(), "workspace initialized from TUI welcome modal");
            }
            TuiAction::WelcomeDismiss => {
                self.dismiss_all_modals();
            }
            TuiAction::ShowHelp => {
                self.tui_state.active_modal =
                    if matches!(self.tui_state.active_modal, Some(ModalState::Help)) {
                        None
                    } else {
                        self.tui_state.help_scroll = 0;
                        Some(ModalState::Help)
                    };
            }
            TuiAction::ToggleScreenPostFx => {
                self.fx_config.screen_postfx = !self.fx_config.screen_postfx;
                let state = if self.fx_config.screen_postfx {
                    "enabled"
                } else {
                    "disabled"
                };
                self.notifications
                    .push_back(super::modals::Notification::info(&format!(
                        "Screen postfx {state}"
                    )));
            }
            TuiAction::CycleEffectsPreset => {
                let preset = self.fx_config.cycle_preset();
                match self.fx_config.save_preset(&self.workdir) {
                    Ok(()) => {
                        self.notifications
                            .push_back(super::modals::Notification::info(&format!(
                                "Effects: {}",
                                preset.label()
                            )));
                    }
                    Err(error) => {
                        self.notifications
                            .push_back(super::modals::Notification::error(&format!(
                                "Effects preset save failed: {error}"
                            )));
                    }
                }
            }
            TuiAction::ShowPlanDetail => {
                let plan_id = self
                    .tui_state
                    .plans
                    .get(self.tui_state.selected_plan_idx)
                    .map(|plan| plan.id.clone());
                let is_same_plan_open = matches!(
                    self.tui_state.active_modal.as_ref(),
                    Some(ModalState::PlanDetail {
                        plan_id: active_plan_id
                    }) if plan_id.as_ref().is_some_and(|plan_id| active_plan_id == plan_id)
                );

                self.tui_state.active_modal = if is_same_plan_open {
                    None
                } else {
                    plan_id.map(|plan_id| {
                        self.tui_state.plan_detail_scroll = 0;
                        ModalState::PlanDetail { plan_id }
                    })
                };
            }
            TuiAction::ClosePlanDetail => {
                if matches!(
                    self.tui_state.active_modal,
                    Some(ModalState::PlanDetail { .. })
                ) {
                    self.tui_state.active_modal = None;
                }
            }
            TuiAction::ShowTaskDetail => {
                let task_count = self.tui_state.current_task_checklist.len();
                if task_count > 0 {
                    let task_idx = self.tui_state.task_scroll.min(task_count.saturating_sub(1));
                    self.tui_state.active_modal = Some(ModalState::TaskDetail {
                        task_idx,
                        scroll_offset: 0,
                    });
                }
            }
            TuiAction::CloseTaskDetail => {
                if matches!(
                    self.tui_state.active_modal,
                    Some(ModalState::TaskDetail { .. })
                ) {
                    self.tui_state.active_modal = None;
                }
            }
            TuiAction::ShowWaveOverview => {
                if matches!(
                    self.tui_state.active_modal,
                    Some(ModalState::WaveOverview { .. })
                ) {
                    self.tui_state.active_modal = None;
                } else {
                    self.tui_state.active_modal = Some(ModalState::WaveOverview {
                        waves: execution_waves_for_modal(&self.tui_state),
                        scroll_offset: 0,
                    });
                }
            }
            TuiAction::ShowQueueOverview => {
                if matches!(
                    self.tui_state.active_modal,
                    Some(ModalState::QueueOverview { .. })
                ) {
                    self.tui_state.active_modal = None;
                } else {
                    let milestones = queue_overview_milestones(&self.tui_state, &self.workdir);
                    self.tui_state.active_modal = Some(ModalState::QueueOverview {
                        selected_index: self
                            .tui_state
                            .current_wave()
                            .min(milestones.len().saturating_sub(1)),
                        scroll_offset: self.tui_state.current_wave() as u16,
                        milestones,
                    });
                }
            }
            TuiAction::OpenTaskPicker => {
                let tasks = task_picker_rows(&self.tui_state);
                let selected_index = self
                    .tui_state
                    .task_scroll
                    .min(tasks.len().saturating_sub(1));
                self.tui_state.active_modal = Some(ModalState::TaskPicker {
                    tasks,
                    selected_index,
                    scroll_offset: selected_index as u16,
                });
            }
            TuiAction::ToggleAgentTopology => {
                let was_visible = self.tui_state.agent_topology_visible;
                self.tui_state.active_tab = Tab::Agents;
                self.tui_state.focus = FocusZone::AgentOutput;
                if let Some(page_id) = tab_to_page(Tab::Agents) {
                    self.current_page = page_id;
                    let _ = self.scaffold.set_active_page(page_id);
                }
                self.tui_state.toggle_agent_topology();
                if !was_visible && self.tui_state.agent_topology_visible {
                    self.request_agent_topology_refresh();
                }
            }
            TuiAction::CloseTaskPicker => {
                if matches!(
                    self.tui_state.active_modal,
                    Some(ModalState::TaskPicker { .. })
                ) {
                    self.tui_state.active_modal = None;
                }
            }
            TuiAction::ExpandCollapse => {
                if let Some(plan) = self
                    .tui_state
                    .plans
                    .get_mut(self.tui_state.selected_plan_idx)
                {
                    plan.expanded = !plan.expanded;
                }
            }
            TuiAction::TogglePause => {
                if let Some(sender) = &self.exec_cmd_sender {
                    let requested_pause = !self.tui_state.is_paused;
                    let kind = if requested_pause {
                        crate::execution_control::ExecutionCommandKind::Pause
                    } else {
                        crate::execution_control::ExecutionCommandKind::Resume
                    };
                    let cmd = sender.build_command(kind, None, None, None);
                    match sender.try_send(cmd) {
                        Ok(()) => {
                            // State changes on Completed ack; show pending.
                            self.notifications.push_back(super::modals::Notification::info(
                                if requested_pause { "Pause requested" } else { "Resume requested" },
                            ));
                        }
                        Err(crate::execution_control::CommandSendError::Full(_)) => {
                            self.notifications.push_back(super::modals::Notification::warn(
                                "command queue full",
                            ));
                        }
                        Err(crate::execution_control::CommandSendError::Disconnected(_)) => {
                            self.notifications.push_back(super::modals::Notification::warn(
                                "executor disconnected",
                            ));
                        }
                    }
                } else {
                    self.notifications.push_back(super::modals::Notification::warn(
                        "Pause is available only during a connected plan run",
                    ));
                }
            }
            TuiAction::SwitchAgentTab(idx) => {
                if idx == usize::MAX {
                    let agent_count = 7;
                    self.tui_state.selected_agent_tab =
                        (self.tui_state.selected_agent_tab + 1) % agent_count;
                } else {
                    let max_idx = self.tui_state.agents.len().saturating_sub(1).max(6);
                    self.tui_state.selected_agent_tab = idx.min(max_idx);
                }

                // P1.4: Switch selected agent to the first one matching the
                // newly selected role tab so the output panel updates.
                use crate::tui::views::agents_view::ROLE_TABS;
                if let Some(&(role, _)) = ROLE_TABS.get(self.tui_state.selected_agent_tab) {
                    // Check agent_summaries first (dashboard data), then agents (snapshot data).
                    let matching_idx = self
                        .tui_state
                        .agent_summaries
                        .iter()
                        .position(|a| a.label == role)
                        .or_else(|| self.tui_state.agents.iter().position(|a| a.role == role));
                    if let Some(agent_idx) = matching_idx {
                        self.tui_state.selected_agent = agent_idx;
                    }
                }
            }
            TuiAction::SwitchDetailTab(idx) => {
                self.tui_state.plan_detail_tab = idx;
            }
            TuiAction::ApproveCommand => {
                if !self.resolve_active_approval(true) {
                    self.tui_state.pending_approval = None;
                }
            }
            TuiAction::ApproveAll => {
                if !self.resolve_active_approval(true) {
                    self.tui_state.pending_approval = None;
                }
            }
            TuiAction::RejectCommand => {
                if !self.resolve_active_approval(false) {
                    self.tui_state.pending_approval = None;
                }
            }
            TuiAction::StartInject => {
                self.tui_state.input_mode = InputMode::Inject;
                self.tui_state.message_input.clear();
            }
            TuiAction::SubmitInject => {
                let msg = self.tui_state.message_input.clone();
                self.tui_state.input_mode = InputMode::Normal;
                self.tui_state.message_input.clear();
                if !msg.is_empty() {
                    // Write inject signal to .roko/engrams.jsonl for the orchestrator
                    let signal_path = self.workdir.join(".roko").join("engrams.jsonl");
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis();
                    let entry = serde_json::json!({
                        "id": format!("inject-{ts}"),
                        "kind": "roko.inject.directive",
                        "created_at_ms": ts,
                        "payload": { "message": msg },
                    });
                    std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&signal_path)
                        .inspect_err(|err| {
                            tracing::warn!(
                                error = %err,
                                path = %signal_path.display(),
                                "failed to open signal file for inject"
                            );
                        })
                        .ok()
                        .and_then(|mut f| {
                            use std::io::Write;
                            writeln!(f, "{}", entry)
                                .inspect_err(|err| {
                                    tracing::warn!(
                                        error = %err,
                                        path = %signal_path.display(),
                                        "failed to append inject signal"
                                    );
                                })
                                .ok()
                        });
                    self.notifications
                        .push_back(super::modals::Notification::info(format!(
                            "Injected: {}",
                            truncate_str(&msg, 40)
                        )));
                }
            }
            TuiAction::CancelInject => {
                self.tui_state.input_mode = InputMode::Normal;
                self.tui_state.message_input.clear();
            }
            TuiAction::InputChar(c) => {
                if self.tui_state.input_mode == InputMode::ConfigEdit {
                    self.tui_state.config_edit_buffer.push(c);
                } else if self.tui_state.input_mode == InputMode::Inject {
                    self.tui_state.message_input.push(c);
                } else if self.tui_state.input_mode == InputMode::Filter {
                    self.tui_state.filter_text.push(c);
                    self.tui_state.filter = self.tui_state.filter_text.clone();
                    self.tui_state.filter_active = !self.tui_state.filter.is_empty();
                } else if self.tui_state.input_mode == InputMode::LogSearch {
                    self.tui_state.log_search.pattern.push(c);
                    self.tui_state.log_search.recompile();
                    self.refresh_log_search_matches();
                } else if self.tui_state.input_mode == InputMode::PlanFilter {
                    self.tui_state.plan_tree_filter.pattern.push(c);
                    self.tui_state.plan_tree_filter.reparse();
                    self.normalize_selected_plan_for_filter();
                }
            }
            TuiAction::InputBackspace => {
                if self.tui_state.input_mode == InputMode::ConfigEdit {
                    self.tui_state.config_edit_buffer.pop();
                } else if self.tui_state.input_mode == InputMode::Inject {
                    self.tui_state.message_input.pop();
                } else if self.tui_state.input_mode == InputMode::Filter {
                    self.tui_state.filter_text.pop();
                    self.tui_state.filter = self.tui_state.filter_text.clone();
                    self.tui_state.filter_active = !self.tui_state.filter.is_empty();
                } else if self.tui_state.input_mode == InputMode::LogSearch {
                    self.tui_state.log_search.pattern.pop();
                    self.tui_state.log_search.recompile();
                    self.refresh_log_search_matches();
                } else if self.tui_state.input_mode == InputMode::PlanFilter {
                    self.tui_state.plan_tree_filter.pattern.pop();
                    self.tui_state.plan_tree_filter.reparse();
                    self.normalize_selected_plan_for_filter();
                }
            }
            TuiAction::StartFilter => {
                self.tui_state.input_mode = InputMode::Filter;
                self.tui_state.filter_text.clear();
                self.tui_state.filter.clear();
                self.tui_state.filter_active = false;
            }
            TuiAction::AcceptFilter => {
                self.tui_state.filter = self.tui_state.filter_text.clone();
                self.tui_state.input_mode = InputMode::Normal;
                self.tui_state.filter_active = !self.tui_state.filter_text.is_empty();
            }
            TuiAction::CancelFilter => {
                self.tui_state.input_mode = InputMode::Normal;
                self.tui_state.filter_text.clear();
                self.tui_state.filter.clear();
                self.tui_state.filter_active = false;
            }
            TuiAction::RequestConfirm(action) => {
                self.open_confirm_modal(self.resolve_confirm_action(action));
            }
            TuiAction::ConfirmYes => {
                if self.resolve_active_approval(true) {
                    return;
                }
                self.tui_state.input_mode = InputMode::Normal;
                if matches!(self.tui_state.active_modal, Some(ModalState::Quit)) {
                    self.dismiss_all_modals();
                    self.dispatch_action(TuiAction::QuitConfirmed);
                    return;
                }
                // Execute the confirmed action by writing a signal
                if let Some(action) = &self.tui_state.pending_confirm {
                    let action_str = action.to_string();
                    let signal_path = self.workdir.join(".roko").join("engrams.jsonl");
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis();
                    let entry = serde_json::json!({
                        "id": format!("confirm-{ts}"),
                        "kind": "roko.tui.confirm",
                        "created_at_ms": ts,
                        "payload": { "action": action_str },
                    });
                    std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&signal_path)
                        .inspect_err(|err| {
                            tracing::warn!(
                                error = %err,
                                path = %signal_path.display(),
                                "failed to open signal file for confirm"
                            );
                        })
                        .ok()
                        .and_then(|mut f| {
                            use std::io::Write;
                            writeln!(f, "{}", entry)
                                .inspect_err(|err| {
                                    tracing::warn!(
                                        error = %err,
                                        path = %signal_path.display(),
                                        "failed to append confirm signal"
                                    );
                                })
                                .ok()
                        });
                    self.notifications
                        .push_back(super::modals::Notification::info(format!(
                            "Confirmed: {}",
                            truncate_str(&action_str, 40)
                        )));
                    // Also send the corresponding ExecutionCommand to the executor.
                    self.send_tui_command_for_confirm(action);
                }
                self.tui_state.pending_confirm = None;
                self.tui_state.active_modal = None;
            }
            TuiAction::ConfirmNo => {
                if !self.resolve_active_approval(false) {
                    self.dismiss_all_modals();
                }
            }
            TuiAction::DismissNotification => {
                if let Some(dismissed) = self.notifications.pop_front() {
                    self.push_notification_history(dismissed);
                }
                // Dismiss current warnings individually so new warnings still appear.
                let keys = self.tui_state.active_warning_keys();
                if keys.is_empty() {
                    self.tui_state.warnings_dismissed = true;
                } else {
                    for key in keys {
                        self.tui_state.dismissed_warning_keys.insert(key);
                    }
                }
            }
            TuiAction::ShowNotificationHistory => {
                if matches!(
                    self.tui_state.active_modal,
                    Some(ModalState::NotificationHistory { .. })
                ) {
                    self.tui_state.active_modal = None;
                } else {
                    self.tui_state.active_modal =
                        Some(ModalState::NotificationHistory {
                            scroll_offset: 0,
                            selected_index: 0,
                            filter: super::modals::LevelFilter::default(),
                        });
                }
            }
            TuiAction::NotifFilterToggle(key) => {
                if let Some(ModalState::NotificationHistory { filter, selected_index, .. }) =
                    self.tui_state.active_modal.as_mut()
                {
                    filter.toggle(key);
                    // Reset selection when filters change.
                    *selected_index = 0;
                }
            }
            TuiAction::NotifPageUp => {
                if let Some(ModalState::NotificationHistory {
                    scroll_offset,
                    selected_index,
                    ..
                }) = self.tui_state.active_modal.as_mut()
                {
                    let page = 10u16;
                    *scroll_offset = scroll_offset.saturating_sub(page);
                    *selected_index = selected_index.saturating_sub(page as usize);
                }
            }
            TuiAction::NotifPageDown => {
                if let Some(ModalState::NotificationHistory {
                    scroll_offset,
                    selected_index,
                    ..
                }) = self.tui_state.active_modal.as_mut()
                {
                    let page = 10u16;
                    *scroll_offset = scroll_offset.saturating_add(page);
                    *selected_index = selected_index.saturating_add(page as usize);
                }
            }
            TuiAction::NotifHome => {
                if let Some(ModalState::NotificationHistory {
                    scroll_offset,
                    selected_index,
                    ..
                }) = self.tui_state.active_modal.as_mut()
                {
                    *scroll_offset = 0;
                    *selected_index = 0;
                }
            }
            TuiAction::NotifEnd => {
                // Compute count first to avoid double-borrow of tui_state.
                let count = if let Some(ModalState::NotificationHistory { filter, .. }) =
                    &self.tui_state.active_modal
                {
                    Some(
                        self.tui_state
                            .notification_history
                            .iter()
                            .filter(|e| filter.accepts(e.level))
                            .count(),
                    )
                } else {
                    None
                };
                if let (
                    Some(count),
                    Some(ModalState::NotificationHistory {
                        scroll_offset,
                        selected_index,
                        ..
                    }),
                ) = (count, self.tui_state.active_modal.as_mut())
                {
                    *selected_index = count.saturating_sub(1);
                    *scroll_offset = count.saturating_sub(1) as u16;
                }
            }
            TuiAction::NotifJumpToRelated => {
                // Extract the related target from the selected notification to
                // avoid holding a borrow on active_modal while mutating it.
                let jump_target = if let Some(ModalState::NotificationHistory {
                    selected_index,
                    filter,
                    ..
                }) = &self.tui_state.active_modal
                {
                    let filtered: Vec<&super::modals::NotificationRecord> = self
                        .tui_state
                        .notification_history
                        .iter()
                        .rev()
                        .filter(|e| filter.accepts(e.level))
                        .collect();
                    filtered.get(*selected_index).map(|entry| {
                        (entry.related_task.clone(), entry.related_run.clone())
                    })
                } else {
                    None
                };
                if let Some((related_task, related_run)) = jump_target {
                    if let Some(task_id) = related_task {
                        if let Some(idx) = self
                            .tui_state
                            .current_task_checklist
                            .iter()
                            .position(|t| t.id == task_id)
                        {
                            self.tui_state.active_modal = Some(ModalState::TaskDetail {
                                task_idx: idx,
                                scroll_offset: 0,
                            });
                        } else {
                            self.notifications.push_back(super::modals::Notification::warn(
                                format!("Task {task_id} not found (may be stale)"),
                            ));
                        }
                    } else if let Some(run_id) = related_run {
                        if let Some(idx) = self
                            .tui_state
                            .plans
                            .iter()
                            .position(|p| p.id == run_id)
                        {
                            self.tui_state.active_modal = Some(ModalState::PlanDetail {
                                plan_id: self.tui_state.plans[idx].id.clone(),
                            });
                        } else {
                            self.notifications.push_back(super::modals::Notification::warn(
                                format!("Run {run_id} not found (may be stale)"),
                            ));
                        }
                    }
                }
            }
            TuiAction::ToggleAgentPaneGroup => {
                self.tui_state.agent_pane_group = (self.tui_state.agent_pane_group + 1) % 2;
            }
            TuiAction::DrillIn => match self.tui_state.active_tab {
                Tab::Dashboard | Tab::Plans => {
                    if let Some(plan) = self
                        .tui_state
                        .plans
                        .get_mut(self.tui_state.selected_plan_idx)
                    {
                        plan.expanded = true;
                    }
                }
                Tab::Git => {
                    let max = self.git_branch_count().saturating_sub(1);
                    self.tui_state.git_branch_cursor =
                        (self.tui_state.git_branch_cursor + 1).min(max);
                }
                Tab::Inspect | Tab::Marketplace | Tab::Atelier | Tab::Learning => {}
                Tab::Agents | Tab::Logs | Tab::Config => {}
            },
            TuiAction::DrillOut => match self.tui_state.active_tab {
                Tab::Dashboard | Tab::Plans => {
                    if let Some(plan) = self
                        .tui_state
                        .plans
                        .get_mut(self.tui_state.selected_plan_idx)
                    {
                        plan.expanded = false;
                    }
                }
                Tab::Git => {
                    self.tui_state.git_branch_cursor =
                        self.tui_state.git_branch_cursor.saturating_sub(1);
                }
                Tab::Inspect | Tab::Marketplace | Tab::Atelier | Tab::Learning => {}
                Tab::Agents | Tab::Logs | Tab::Config => {}
            },
            TuiAction::WaveNext => {
                let max = self.tui_state.execution_waves.len().max(1);
                self.tui_state.selected_wave_idx = (self.tui_state.selected_wave_idx + 1) % max;
            }
            TuiAction::WavePrev => {
                let max = self.tui_state.execution_waves.len().max(1);
                self.tui_state.selected_wave_idx = self
                    .tui_state
                    .selected_wave_idx
                    .checked_sub(1)
                    .unwrap_or(max - 1);
            }
            TuiAction::RestartPhase => {
                self.tui_state.input_mode = InputMode::Confirm;
                self.tui_state.pending_confirm = Some(ConfirmAction::RestartPhase);
                let modal_action = modals::ConfirmAction::Custom {
                    message: "Restart current phase?".to_string(),
                };
                self.tui_state.active_modal = Some(ModalState::Confirm {
                    action: modal_action,
                });
            }
            TuiAction::RestartPlan => {
                if let Some(plan) = self.tui_state.plans.get(self.tui_state.selected_plan_idx) {
                    let plan_id = plan.id.clone();
                    self.tui_state.input_mode = InputMode::Confirm;
                    self.tui_state.pending_confirm =
                        Some(ConfirmAction::ResetSelectedPlan(plan_id.clone()));
                    let modal_action = modals::ConfirmAction::Custom {
                        message: format!("Restart plan '{plan_id}'?"),
                    };
                    self.tui_state.active_modal = Some(ModalState::Confirm {
                        action: modal_action,
                    });
                }
            }
            TuiAction::ForceAdvance => {
                if let Some(plan) = self.tui_state.plans.get(self.tui_state.selected_plan_idx) {
                    let plan_id = plan.id.clone();
                    self.tui_state.input_mode = InputMode::Confirm;
                    self.tui_state.pending_confirm =
                        Some(ConfirmAction::ForceAdvance(plan_id.clone()));
                    let modal_action = modals::ConfirmAction::Custom {
                        message: format!("Force-advance plan '{plan_id}'?"),
                    };
                    self.tui_state.active_modal = Some(ModalState::Confirm {
                        action: modal_action,
                    });
                }
            }
            TuiAction::ResetPlanState => {
                if let Some(plan) = self.tui_state.plans.get(self.tui_state.selected_plan_idx) {
                    let plan_id = plan.id.clone();
                    self.tui_state.input_mode = InputMode::Confirm;
                    self.tui_state.pending_confirm =
                        Some(ConfirmAction::ResetSelectedPlan(plan_id.clone()));
                    let modal_action = modals::ConfirmAction::Custom {
                        message: format!("Reset state for plan '{plan_id}'?"),
                    };
                    self.tui_state.active_modal = Some(ModalState::Confirm {
                        action: modal_action,
                    });
                }
            }
            TuiAction::ReverifyPlan => {
                if let Some(plan) = self.tui_state.plans.get(self.tui_state.selected_plan_idx) {
                    let plan_id = plan.id.clone();
                    self.tui_state.input_mode = InputMode::Confirm;
                    self.tui_state.pending_confirm =
                        Some(ConfirmAction::ReverifyPlan(plan_id.clone()));
                    let modal_action = modals::ConfirmAction::Custom {
                        message: format!("Re-verify plan '{plan_id}'?"),
                    };
                    self.tui_state.active_modal = Some(ModalState::Confirm {
                        action: modal_action,
                    });
                }
            }
            TuiAction::ConfigUp => {
                self.tui_state.config_cursor = self.tui_state.config_cursor.saturating_sub(1);
                // Skip headers when navigating up
                let items = self.tui_state.config_items().to_vec();
                while self.tui_state.config_cursor > 0 {
                    if let Some(super::config_meta::ConfigItem::Header(_)) =
                        items.get(self.tui_state.config_cursor)
                    {
                        self.tui_state.config_cursor =
                            self.tui_state.config_cursor.saturating_sub(1);
                    } else {
                        break;
                    }
                }
            }
            TuiAction::ConfigDown => {
                let items = self.tui_state.config_items().to_vec();
                let max_idx = items.len().saturating_sub(1);
                self.tui_state.config_cursor = (self.tui_state.config_cursor + 1).min(max_idx);
                // Skip headers when navigating down
                while self.tui_state.config_cursor < max_idx {
                    if let Some(super::config_meta::ConfigItem::Header(_)) =
                        items.get(self.tui_state.config_cursor)
                    {
                        self.tui_state.config_cursor += 1;
                    } else {
                        break;
                    }
                }
            }
            TuiAction::ConfigToggle => {
                let items = self.tui_state.config_items().to_vec();
                if let Some(item) = items.get(self.tui_state.config_cursor) {
                    match item {
                        super::config_meta::ConfigItem::Field {
                            meta,
                            value,
                            source,
                        } => {
                            match &meta.kind {
                                super::config_meta::ConfigFieldKind::Bool => {
                                    let new_val = if value == "true" { "false" } else { "true" };
                                    self.tui_state
                                        .config_pending
                                        .insert(meta.key.to_string(), new_val.to_string());
                                }
                                super::config_meta::ConfigFieldKind::ReadOnly => {}
                                super::config_meta::ConfigFieldKind::Enum(_)
                                | super::config_meta::ConfigFieldKind::Int { .. } => {
                                    // For enums/presets, Enter cycles right
                                    if *source != super::config_meta::ConfigSource::Env {
                                        if let Some(new_val) = cycle_field_value(meta, value, true)
                                        {
                                            self.tui_state
                                                .config_pending
                                                .insert(meta.key.to_string(), new_val);
                                        }
                                    }
                                }
                                _ => {
                                    // Start text edit for free-form fields
                                    if *source != super::config_meta::ConfigSource::Env {
                                        self.tui_state.config_editing = true;
                                        self.tui_state.config_edit_buffer = value.clone();
                                        self.tui_state.config_edit_key = Some(meta.key.to_string());
                                        self.tui_state.input_mode = InputMode::ConfigEdit;
                                    }
                                }
                            }
                        }
                        super::config_meta::ConfigItem::SaveButton => {
                            self.save_config_changes();
                        }
                        super::config_meta::ConfigItem::Header(_) => {}
                    }
                }
            }
            TuiAction::ConfigCycleLeft | TuiAction::ConfigCycleRight => {
                let items = self.tui_state.config_items().to_vec();
                if let Some(super::config_meta::ConfigItem::Field {
                    meta,
                    value,
                    source,
                }) = items.get(self.tui_state.config_cursor)
                {
                    if *source == super::config_meta::ConfigSource::Env {
                        // Env-overridden: not editable
                    } else {
                        let direction = matches!(action, TuiAction::ConfigCycleRight);
                        if let Some(new_val) = cycle_field_value(meta, value, direction) {
                            self.tui_state
                                .config_pending
                                .insert(meta.key.to_string(), new_val);
                        }
                    }
                }
            }
            TuiAction::ConfigCommitEdit => {
                if self.tui_state.config_editing {
                    if let Some(key) = self.tui_state.config_edit_key.take() {
                        let val = self.tui_state.config_edit_buffer.clone();
                        self.tui_state.config_pending.insert(key, val);
                    }
                    self.tui_state.config_editing = false;
                    self.tui_state.config_edit_buffer.clear();
                    self.tui_state.input_mode = InputMode::Normal;
                }
            }
            TuiAction::ConfigCancelEdit => {
                self.tui_state.config_editing = false;
                self.tui_state.config_edit_buffer.clear();
                self.tui_state.config_edit_key = None;
                self.tui_state.input_mode = InputMode::Normal;
            }
            TuiAction::ConfigSave => {
                self.save_config_changes();
            }
            TuiAction::ConfigReload => {
                self.tui_state.invalidate_config_cache();
            }
            TuiAction::MouseClick { x, y } => {
                // Use hit_test to determine zone
                let zones = super::hit_test::HitZones::compute(
                    super::layout::responsive_outer_margin(Rect::new(
                        0,
                        0,
                        self.terminal_size.0,
                        self.terminal_size.1,
                    )),
                    self.tui_state.active_tab as usize,
                    Tab::ALL.len(),
                );
                if let Some(zone) = zones.zone_at(x, y) {
                    match zone {
                        super::hit_test::FocusZone::HeaderTab(idx) => {
                            if let Some(&tab) = Tab::ALL.get(idx) {
                                self.dispatch_action(TuiAction::SwitchTab(tab));
                            }
                        }
                        super::hit_test::FocusZone::DetailTab(idx) => {
                            self.dispatch_action(TuiAction::SwitchSubView(idx));
                        }
                        other => {
                            let mapped = self.map_hit_zone(other);
                            self.tui_state.focus = mapped;
                        }
                    }
                }
            }
            TuiAction::MouseScrollUp { x, y } => self.scroll_at(x, y, -3),
            TuiAction::MouseScrollDown { x, y } => self.scroll_at(x, y, 3),
            TuiAction::Refresh => self.pending_refresh = true,
            TuiAction::SwitchSubView(idx) => {
                // UI-04: switch sub-view within the current tab region.
                // Map the sub-view index to the appropriate TuiState field
                // based on which tab is active. The sub_tab in ViewState
                // is derived from these fields via current_view_state().
                let tab = self.tui_state.active_tab;
                // Dashboard owns eight purpose-built detail panels.  It does
                // not use the older four-item generic SubView list.
                let max = if tab == Tab::Dashboard {
                    8
                } else {
                    views::SubView::for_tab(tab).len()
                };
                if idx < max {
                    self.tui_state.set_sub_tab_for(tab, idx);
                    if tab == Tab::Logs {
                        self.refresh_log_search_matches();
                    }
                }
            }
            TuiAction::SubmitJob => {
                self.submit_marketplace_job();
            }

            // -- Log search (#217) --
            TuiAction::StartLogSearch => {
                self.tui_state.input_mode = InputMode::LogSearch;
                self.tui_state.log_search.active = true;
                self.tui_state.log_search.pattern.clear();
                self.tui_state.log_search.recompile();
            }
            TuiAction::AcceptLogSearch => {
                self.tui_state.input_mode = InputMode::Normal;
                // Keep search active with the current pattern for n/N navigation
            }
            TuiAction::CancelLogSearch => {
                self.tui_state.input_mode = InputMode::Normal;
                self.tui_state.log_search.clear();
            }
            TuiAction::NextLogMatch => {
                self.tui_state.log_search.next_match();
                if let Some(line_idx) = self.current_log_match_display_index() {
                    self.tui_state.log_scroll = line_idx;
                    self.tui_state.log_auto_tail = false;
                }
            }
            TuiAction::PrevLogMatch => {
                self.tui_state.log_search.prev_match();
                if let Some(line_idx) = self.current_log_match_display_index() {
                    self.tui_state.log_scroll = line_idx;
                    self.tui_state.log_auto_tail = false;
                }
            }
            TuiAction::ToggleLogFilterMode => {
                use super::state::SearchMode;
                self.tui_state.log_search.mode = match self.tui_state.log_search.mode {
                    SearchMode::Highlight => SearchMode::Filter,
                    SearchMode::Filter => SearchMode::Highlight,
                };
                self.refresh_log_search_matches();
            }
            TuiAction::YankLogEntry => {
                let entries = self.tui_state.unified_log_entries().to_vec();
                let idx = if self.tui_state.log_auto_tail {
                    entries.len().saturating_sub(1)
                } else {
                    (self.tui_state.log_scroll).min(entries.len().saturating_sub(1))
                };
                if let Some(entry) = entries.get(idx) {
                    let text = format!("[{}] {} {}", entry.timestamp, entry.source, entry.message);
                    // TODO: clipboard integration (arboard/copypasta)
                    let _ = text; // Suppress unused warning until clipboard is wired
                }
            }

            // -- Plan tree filter (#219) --
            TuiAction::StartPlanFilter => {
                self.tui_state.input_mode = InputMode::PlanFilter;
                self.tui_state.plan_tree_filter.active = true;
                self.tui_state.plan_tree_filter.pattern.clear();
                self.tui_state.plan_tree_filter.reparse();
                self.normalize_selected_plan_for_filter();
            }
            TuiAction::AcceptPlanFilter => {
                self.tui_state.input_mode = InputMode::Normal;
                // Keep filter active
            }
            TuiAction::CancelPlanFilter => {
                self.tui_state.input_mode = InputMode::Normal;
                self.tui_state.plan_tree_filter.clear();
                self.normalize_selected_plan_for_filter();
            }

            // -- Recovery keybindings (#119) --
            TuiAction::SoftRetry => {
                if let Some(plan) = self.tui_state.plans.get(self.tui_state.selected_plan_idx) {
                    if plan.status.is_failed() || plan.tasks_failed > 0 {
                        let plan_id = plan.id.clone();
                        self.open_confirm_modal(ConfirmAction::SoftRetryPlan(plan_id));
                    } else {
                        self.notifications.push_back(super::modals::Notification::info(
                            "No failed tasks to retry",
                        ));
                    }
                }
            }
            TuiAction::DiagnoseSelected => {
                if let Some(plan) = self.tui_state.plans.get(self.tui_state.selected_plan_idx) {
                    let plan_id = plan.id.clone();
                    // Show a diagnose detail modal with plan error context
                    let diag_lines: Vec<String> = plan
                        .tasks
                        .iter()
                        .filter(|t| t.status.is_failed())
                        .map(|t| format!("FAILED: {} ({})", t.name, t.id))
                        .collect();
                    let message = if diag_lines.is_empty() {
                        format!("Plan '{plan_id}' -- no failed tasks found.")
                    } else {
                        format!("Plan '{plan_id}' diagnostics:\n{}", diag_lines.join("\n"))
                    };
                    self.tui_state.active_modal = Some(ModalState::PlanDetail { plan_id });
                    self.notifications
                        .push_back(super::modals::Notification::info(truncate_str(
                            &message, 80,
                        )));
                }
            }
            TuiAction::RepairWithContext => {
                if let Some(plan) = self.tui_state.plans.get(self.tui_state.selected_plan_idx) {
                    let plan_id = plan.id.clone();
                    self.open_confirm_modal(ConfirmAction::RepairPlanPreserve(plan_id));
                }
            }
            TuiAction::ReverifyGatesOnly => {
                if let Some(plan) = self.tui_state.plans.get(self.tui_state.selected_plan_idx) {
                    let plan_id = plan.id.clone();
                    self.open_confirm_modal(ConfirmAction::ReverifyPlan(plan_id));
                }
            }

            TuiAction::CycleCostSort => {
                self.tui_state.cost_sort_mode = self.tui_state.cost_sort_mode.next();
            }

            TuiAction::None => {}
        }

        self.clamp_scroll_state_to_view();
    }

    fn open_confirm_modal(&mut self, action: ConfirmAction) {
        self.tui_state.input_mode = InputMode::Confirm;
        self.tui_state.pending_confirm = Some(action.clone());
        let modal_action = modals::ConfirmAction::Custom {
            message: action.to_string(),
        };
        self.tui_state.active_modal = Some(ModalState::Confirm {
            action: modal_action,
        });
    }

    fn resolve_active_approval(&mut self, approved: bool) -> bool {
        if !matches!(
            self.tui_state.active_modal,
            Some(ModalState::Approval { .. })
        ) {
            return false;
        }

        if let Some(response_tx) = self.pending_approval_response.take() {
            let _ = response_tx.send(approved);
        }

        self.tui_state.pending_approval = None;
        self.tui_state.active_modal = None;
        if self.tui_state.input_mode == InputMode::Confirm {
            self.tui_state.input_mode = InputMode::Normal;
        }
        true
    }

    fn accept_approval_request(&mut self, request: ApprovalRequest) {
        let ApprovalRequest {
            role,
            command,
            approval_id,
            response_tx,
        } = request;

        if self.pending_approval_response.is_some() {
            let _ = response_tx.send(false);
            return;
        }

        self.tui_state.pending_approval = Some(PendingApproval {
            agent_id: role.clone(),
            description: approval_id,
            command: command.clone(),
        });
        self.pending_approval_response = Some(response_tx);
        self.tui_state.input_mode = InputMode::Confirm;
        self.tui_state.active_modal = Some(ModalState::Approval { role, command });
    }

    fn drain_approval_requests(&mut self) {
        let Some(mut rx) = self.approval_rx.take() else {
            return;
        };

        let mut disconnected = false;
        let mut got_request = false;
        loop {
            match rx.try_recv() {
                Ok(request) => {
                    self.accept_approval_request(request);
                    got_request = true;
                }
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }

        if got_request {
            self.render_dirty.insert(RenderDirty::MODAL);
        }

        if !disconnected {
            self.approval_rx = Some(rx);
        }
    }

    fn resolve_confirm_action(&self, action: ConfirmAction) -> ConfirmAction {
        match action {
            ConfirmAction::DiagnosePlan(plan_id) if plan_id.is_empty() => {
                ConfirmAction::DiagnosePlan(self.selected_plan_id().unwrap_or_default())
            }
            ConfirmAction::MergePlan { plan_id, branch }
                if plan_id.is_empty() || branch.is_empty() =>
            {
                ConfirmAction::MergePlan {
                    plan_id: if plan_id.is_empty() {
                        self.selected_plan_id().unwrap_or_default()
                    } else {
                        plan_id
                    },
                    branch: if branch.is_empty() {
                        self.current_git_branch()
                    } else {
                        branch
                    },
                }
            }
            ConfirmAction::MergeAllDone { branches } if branches.is_empty() => {
                ConfirmAction::MergeAllDone {
                    branches: self.completed_plan_branches(),
                }
            }
            ConfirmAction::ResetSelectedPlan(plan_id) if plan_id.is_empty() => {
                ConfirmAction::ResetSelectedPlan(self.selected_plan_id().unwrap_or_default())
            }
            other => other,
        }
    }

    /// Map a confirmed `ConfirmAction` to the corresponding
    /// `ExecutionCommand` and send it through the in-process channel (if
    /// connected to an executor).
    fn send_tui_command_for_confirm(&self, action: &ConfirmAction) {
        use crate::execution_control::ExecutionCommandKind;

        let (kind, plan_id, task_id) = match action {
            ConfirmAction::SoftRetryPlan(plan_id) => {
                (ExecutionCommandKind::SoftRetry, Some(plan_id.clone()), None)
            }
            ConfirmAction::RepairPlanPreserve(plan_id) => (
                ExecutionCommandKind::Repair {
                    preserve_completed: true,
                },
                Some(plan_id.clone()),
                None,
            ),
            ConfirmAction::RepairPlanClean(plan_id) => (
                ExecutionCommandKind::Repair {
                    preserve_completed: false,
                },
                Some(plan_id.clone()),
                None,
            ),
            ConfirmAction::ReverifyPlan(plan_id) => {
                (ExecutionCommandKind::ReverifyGates, Some(plan_id.clone()), None)
            }
            ConfirmAction::ForceAdvance(plan_id) => {
                let task_id = self
                    .tui_state
                    .plans
                    .get(self.tui_state.selected_plan_idx)
                    .and_then(|p| {
                        p.tasks
                            .iter()
                            .find(|t| t.status == TaskRowStatus::Failed)
                            .map(|t| t.id.clone())
                    })
                    .unwrap_or_default();
                (ExecutionCommandKind::Skip, Some(plan_id.clone()), Some(task_id))
            }
            ConfirmAction::ResetSelectedPlan(plan_id) => {
                (ExecutionCommandKind::Cancel, Some(plan_id.clone()), None)
            }
            // Other confirm actions don't map to executor commands.
            _ => return,
        };
        if let Some(sender) = &self.exec_cmd_sender {
            let cmd = sender.build_command(kind, plan_id, task_id, None);
            let _ = sender.try_send(cmd);
        }
    }

    fn selected_plan_id(&self) -> Option<String> {
        self.tui_state
            .plans
            .get(self.tui_state.selected_plan_idx)
            .map(|plan| plan.id.clone())
    }

    fn visible_plan_indices(&self) -> Vec<usize> {
        let filter = &self.tui_state.plan_tree_filter;
        let filtering = filter.active && !filter.pattern.is_empty();
        self.tui_state
            .plans
            .iter()
            .enumerate()
            .filter_map(|(index, plan)| {
                (!filtering || filter.matches_plan_or_tasks(plan)).then_some(index)
            })
            .collect()
    }

    fn normalize_selected_plan_for_filter(&mut self) {
        let visible = self.visible_plan_indices();
        if visible.is_empty() {
            return;
        }
        if !visible.contains(&self.tui_state.selected_plan_idx) {
            self.tui_state.selected_plan_idx = visible[0];
            self.tui_state.plan_scroll_offset = 0;
        }
    }

    fn move_selected_plan(&mut self, direction: i8) {
        let visible = self.visible_plan_indices();
        if visible.is_empty() {
            return;
        }
        let current = visible
            .iter()
            .position(|index| *index == self.tui_state.selected_plan_idx)
            .unwrap_or(0);
        let next = if direction < 0 {
            current.saturating_sub(1)
        } else {
            current.saturating_add(1).min(visible.len() - 1)
        };
        self.tui_state.selected_plan_idx = visible[next];
    }

    fn refresh_log_search_matches(&mut self) {
        let signals_only = self.tui_state.sub_tab_for(Tab::Logs) == 1;
        let entries = self
            .tui_state
            .unified_log_entries()
            .iter()
            .filter(|entry| self.tui_state.log_level_visible(entry.level.filter_level()))
            .filter(|entry| {
                !signals_only
                    || entry.source.starts_with("signal:")
                    || entry.source.starts_with("episode:")
            })
            .cloned()
            .collect::<Vec<_>>();
        self.tui_state.log_search.update_matches(&entries);
    }

    fn current_log_match_display_index(&self) -> Option<usize> {
        let search = &self.tui_state.log_search;
        if search.mode == super::state::SearchMode::Filter {
            (search.match_count > 0).then_some(search.current_match)
        } else {
            search.match_indices.get(search.current_match).copied()
        }
    }

    fn current_git_branch(&self) -> String {
        if !self.tui_state.git_branch.is_empty() {
            return self.tui_state.git_branch.clone();
        }

        self.tui_state
            .git_view_data
            .as_ref()
            .map(|git| git.current_branch.clone())
            .filter(|branch| !branch.is_empty())
            .unwrap_or_default()
    }

    fn completed_plan_branches(&self) -> Vec<String> {
        self.tui_state
            .plans
            .iter()
            .filter(|plan| !plan.active && !plan.status.is_failed())
            .map(|plan| plan.id.clone())
            .collect()
    }

    /// Map a hit_test::FocusZone to the input::FocusZone used by keyboard/scroll routing.
    fn map_hit_zone(&self, zone: super::hit_test::FocusZone) -> FocusZone {
        match zone {
            super::hit_test::FocusZone::PlanTree => FocusZone::PlanTree,
            super::hit_test::FocusZone::TaskProgress => FocusZone::TaskProgress,
            super::hit_test::FocusZone::AgentOutput => FocusZone::AgentOutput,
            super::hit_test::FocusZone::CommandOutput => FocusZone::CommandOutput,
            super::hit_test::FocusZone::RightContent => FocusZone::RightPanel,
            super::hit_test::FocusZone::HeaderTab(_)
            | super::hit_test::FocusZone::DetailTab(_) => FocusZone::RightPanel,
            super::hit_test::FocusZone::LeftPane => match self.tui_state.active_tab {
                Tab::Git => FocusZone::GitBranches,
                Tab::Logs => FocusZone::LogList,
                Tab::Config => FocusZone::ConfigKeys,
                Tab::Inspect => FocusZone::InspectTree,
                Tab::Marketplace => FocusZone::MarketList,
                Tab::Atelier => FocusZone::AtelierList,
                Tab::Learning => FocusZone::LearningMetrics,
                _ => FocusZone::PlanTree,
            },
            super::hit_test::FocusZone::RightPane => match self.tui_state.active_tab {
                Tab::Git => FocusZone::GitDetail,
                Tab::Logs => FocusZone::LogDetail,
                Tab::Config => FocusZone::ConfigValues,
                Tab::Inspect => FocusZone::InspectDetail,
                Tab::Marketplace => FocusZone::MarketDetail,
                Tab::Atelier => FocusZone::AtelierDetail,
                Tab::Learning => FocusZone::LearningDetail,
                _ => FocusZone::RightPanel,
            },
        }
    }

    /// Scroll the panel under the mouse cursor at (x, y) by delta lines.
    /// Falls back to scroll_focused when the cursor is outside any known zone.
    fn scroll_at(&mut self, x: u16, y: u16, delta: i32) {
        let zones = super::hit_test::HitZones::compute(
            super::layout::responsive_outer_margin(Rect::new(
                0,
                0,
                self.terminal_size.0,
                self.terminal_size.1,
            )),
            self.tui_state.active_tab as usize,
            Tab::ALL.len(),
        );
        if let Some(zone) = zones.zone_at(x, y) {
            match zone {
                super::hit_test::FocusZone::HeaderTab(_)
                | super::hit_test::FocusZone::DetailTab(_) => {
                    // Header/detail tabs are not scrollable panels.
                    return;
                }
                other => {
                    let mapped = self.map_hit_zone(other);
                    // Temporarily set focus to the zone under the cursor, scroll, restore.
                    let saved = self.tui_state.focus;
                    self.tui_state.focus = mapped;
                    self.scroll_focused(delta);
                    self.tui_state.focus = saved;
                }
            }
        } else {
            // Cursor outside any panel -- fall back to keyboard-focused panel.
            self.scroll_focused(delta);
        }
    }

    fn scroll_focused(&mut self, delta: i32) {
        match (self.tui_state.active_tab, self.tui_state.focus) {
            (Tab::Logs, _) => self.scroll_logs_by(delta),
            (Tab::Agents, FocusZone::PlanTree) => {
                let max = self.tui_state.agents.len().saturating_sub(1);
                let next = (self.tui_state.selected_agent as i32 + delta).clamp(0, max as i32);
                self.tui_state.selected_agent = next as usize;
            }
            (Tab::Agents, FocusZone::AgentOutput) => self.scroll_agent_output_by(delta),
            (Tab::Marketplace, _) => {
                if !self.tui_state.marketplace_jobs.is_empty() {
                    let max = self.tui_state.marketplace_jobs.len().saturating_sub(1);
                    let next = (self.tui_state.marketplace_selected_job as i32 + delta)
                        .clamp(0, max as i32);
                    self.tui_state.marketplace_selected_job = next as usize;
                }
            }
            (Tab::Atelier, _) => {
                if !self.tui_state.atelier_prds.is_empty() {
                    let max = self.tui_state.atelier_prds.len().saturating_sub(1);
                    let next =
                        (self.tui_state.atelier_selected_prd as i32 + delta).clamp(0, max as i32);
                    self.tui_state.atelier_selected_prd = next as usize;
                }
            }
            (_, FocusZone::PlanTree) => {
                let current = self.tui_state.plan_scroll_offset as i32;
                self.tui_state.plan_scroll_offset = (current + delta).max(0) as usize;
            }
            (_, FocusZone::TaskProgress) => {
                let current = self.tui_state.task_scroll as i32;
                self.tui_state.task_scroll = (current + delta).max(0) as usize;
            }
            (_, FocusZone::AgentOutput) => self.scroll_agent_output_by(delta),
            (_, FocusZone::CommandOutput) => {
                let current = self.tui_state.command_output_scroll as i32;
                self.tui_state.command_output_scroll = (current + delta).max(0) as usize;
            }
            // Dashboard RightPanel: route to procs_scroll when on the Procs sub-tab.
            (Tab::Dashboard, FocusZone::RightPanel) if self.tui_state.plan_detail_tab == 7 => {
                let current = self.tui_state.procs_scroll as i32;
                self.tui_state.procs_scroll = (current + delta).max(0) as usize;
            }
            (_, FocusZone::RightPanel) => {
                let current = self.tui_state.diff_scroll as i32;
                self.tui_state.diff_scroll = (current + delta).max(0) as usize;
            }
            // Per-tab detail zones: each routes to its own dedicated scroll field.
            // NOTE: Logs/Marketplace/Atelier handled by wildcard arms above.
            (Tab::Git, FocusZone::GitDetail) => {
                let current = self.tui_state.git_detail_scroll as i32;
                self.tui_state.git_detail_scroll = (current + delta).max(0) as usize;
            }
            (Tab::Config, FocusZone::ConfigValues) => {
                let current = self.tui_state.config_values_scroll as i32;
                self.tui_state.config_values_scroll = (current + delta).max(0) as usize;
            }
            (Tab::Inspect, FocusZone::InspectDetail) => {
                let current = self.tui_state.inspect_detail_scroll as i32;
                self.tui_state.inspect_detail_scroll = (current + delta).max(0) as usize;
            }
            (Tab::Learning, FocusZone::LearningDetail) => {
                let current = self.tui_state.learning_detail_scroll as i32;
                self.tui_state.learning_detail_scroll = (current + delta).max(0) as usize;
            }
            // Config left-pane list scrolling.
            (Tab::Config, FocusZone::ConfigKeys) => {
                let current = self.tui_state.config_scroll_offset as i32;
                self.tui_state.config_scroll_offset = (current + delta).max(0) as usize;
            }
            _ => {
                // Fallback: still use diff_scroll for any unmatched zones.
                let current = self.tui_state.diff_scroll as i32;
                self.tui_state.diff_scroll = (current + delta).max(0) as usize;
            }
        }
    }

    fn set_focused_scroll(&mut self, offset: usize) {
        match (self.tui_state.active_tab, self.tui_state.focus) {
            (Tab::Agents, FocusZone::PlanTree) => {
                let max = self.tui_state.agents.len().saturating_sub(1);
                self.tui_state.selected_agent = if offset == usize::MAX {
                    max
                } else {
                    offset.min(max)
                };
            }
            (Tab::Marketplace, _) => {
                if !self.tui_state.marketplace_jobs.is_empty() {
                    let max = self.tui_state.marketplace_jobs.len().saturating_sub(1);
                    self.tui_state.marketplace_selected_job = if offset == usize::MAX {
                        max
                    } else {
                        offset.min(max)
                    };
                }
            }
            (Tab::Atelier, _) => {
                if !self.tui_state.atelier_prds.is_empty() {
                    let max = self.tui_state.atelier_prds.len().saturating_sub(1);
                    self.tui_state.atelier_selected_prd = if offset == usize::MAX {
                        max
                    } else {
                        offset.min(max)
                    };
                }
            }
            (Tab::Agents, FocusZone::AgentOutput) => {
                if self.tui_state.agent_topology_visible {
                    let max = self.current_agent_topology_max_scroll();
                    self.tui_state.agent_topology_scroll_offset = if offset == usize::MAX {
                        max
                    } else {
                        offset.min(max)
                    };
                } else if offset == usize::MAX {
                    self.tui_state.agent_scroll = None;
                } else {
                    self.tui_state.agent_scroll = Some(offset);
                }
            }
            (_, FocusZone::PlanTree) => {
                self.tui_state.plan_scroll_offset = offset;
            }
            (_, FocusZone::TaskProgress) => {
                self.tui_state.task_scroll = offset;
            }
            (Tab::Logs, _) => {
                if offset == usize::MAX {
                    self.tui_state.log_auto_tail = true;
                    self.tui_state.log_scroll = 0;
                } else {
                    self.tui_state.log_auto_tail = false;
                    self.tui_state.log_scroll = offset;
                }
            }
            (_, FocusZone::AgentOutput) => {
                if self.tui_state.agent_topology_visible {
                    let max = self.current_agent_topology_max_scroll();
                    self.tui_state.agent_topology_scroll_offset = if offset == usize::MAX {
                        max
                    } else {
                        offset.min(max)
                    };
                } else if offset == usize::MAX {
                    self.tui_state.agent_scroll = None;
                } else {
                    self.tui_state.agent_scroll = Some(offset);
                }
            }
            (_, FocusZone::CommandOutput) => {
                self.tui_state.command_output_scroll = offset;
            }
            // Dashboard RightPanel: route to procs_scroll when on the Procs sub-tab.
            (Tab::Dashboard, FocusZone::RightPanel) if self.tui_state.plan_detail_tab == 7 => {
                self.tui_state.procs_scroll = offset;
            }
            (_, FocusZone::RightPanel) => {
                self.tui_state.diff_scroll = offset;
            }
            // Per-tab detail zones: each routes to its own dedicated scroll field.
            // NOTE: Logs/Marketplace/Atelier handled by wildcard arms above.
            (Tab::Git, FocusZone::GitDetail) => {
                self.tui_state.git_detail_scroll = offset;
            }
            (Tab::Config, FocusZone::ConfigValues) => {
                self.tui_state.config_values_scroll = offset;
            }
            (Tab::Config, FocusZone::ConfigKeys) => {
                self.tui_state.config_scroll_offset = offset;
            }
            (Tab::Inspect, FocusZone::InspectDetail) => {
                self.tui_state.inspect_detail_scroll = offset;
            }
            (Tab::Learning, FocusZone::LearningDetail) => {
                self.tui_state.learning_detail_scroll = offset;
            }
            _ => {
                self.tui_state.diff_scroll = offset;
            }
        }
    }

    fn apply_signed_scroll(current: usize, delta: i16) -> usize {
        if delta < 0 {
            current.saturating_sub(delta.saturating_abs() as usize)
        } else {
            current.saturating_add(delta as usize)
        }
    }

    fn page_scroll_lines(&self) -> i32 {
        i32::from(self.terminal_size.1.saturating_sub(4).max(1))
    }

    fn merge_process_metrics(&mut self, samples: Vec<ProcessMetricSample>) {
        const PROCESS_HISTORY_LIMIT: usize = 60;

        let mut existing = std::mem::take(&mut self.tui_state.process_metrics);
        let mut merged = Vec::with_capacity(samples.len());

        for sample in samples {
            let mut metric =
                if let Some(index) = existing.iter().position(|entry| entry.pid == sample.pid) {
                    existing.swap_remove(index)
                } else {
                    super::state::ProcessMetrics {
                        pid: sample.pid,
                        role: sample.role.clone(),
                        ..Default::default()
                    }
                };

            metric.pid = sample.pid;
            metric.role = sample.role;
            metric.cpu_pct = sample.cpu_pct;
            metric.mem_bytes = sample.mem_bytes;
            metric.state = sample.state;
            metric.uptime_secs = sample.uptime_secs;
            push_bounded_history(
                &mut metric.cpu_history,
                sample.cpu_pct,
                PROCESS_HISTORY_LIMIT,
            );
            push_bounded_history(
                &mut metric.mem_history,
                sample.mem_bytes,
                PROCESS_HISTORY_LIMIT,
            );
            merged.push(metric);
        }

        self.tui_state.process_metrics = merged;
    }

    fn current_agent_scroll_offset(&self) -> usize {
        self.tui_state
            .agent_scroll
            .unwrap_or_else(|| self.current_agent_max_scroll())
    }

    fn current_agent_topology_max_scroll(&self) -> usize {
        views::agents_view::agent_topology_lines(&self.tui_state)
            .len()
            .saturating_sub(self.current_agent_topology_viewport_height())
            .min(u16::MAX as usize)
    }

    fn current_agent_max_scroll(&self) -> usize {
        self.current_agent_output_line_count()
            .saturating_sub(self.current_agent_output_viewport_height())
            .min(u16::MAX as usize)
    }

    fn current_log_max_scroll(&self) -> usize {
        let content_area = self.current_content_area();
        let sections =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(content_area);
        let viewport_height = sections[1].height.saturating_sub(2) as usize;
        super::views::logs_view::filtered_entry_count(&self.data, &self.tui_state)
            .saturating_sub(viewport_height)
            .min(u16::MAX as usize)
    }

    fn current_git_max_scroll(&self) -> usize {
        let content_area = self.current_content_area();
        let panels = Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)])
            .split(content_area);
        let sections = Layout::vertical([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(panels[1]);
        let viewport_height = sections[0].height.saturating_sub(2) as usize;
        self.tui_state
            .git_view_data
            .as_ref()
            .map_or(0, |git| git.commits.len().saturating_sub(viewport_height))
            .min(u16::MAX as usize)
    }

    fn scroll_agent_output_by(&mut self, delta: i32) {
        if self.tui_state.agent_topology_visible {
            let max_scroll = self.current_agent_topology_max_scroll();
            let current = self.tui_state.agent_topology_scroll_offset.min(max_scroll);
            self.tui_state.agent_topology_scroll_offset = if delta < 0 {
                current.saturating_sub(delta.unsigned_abs() as usize)
            } else {
                current.saturating_add(delta as usize).min(max_scroll)
            };
            self.tui_state.clamp_agent_topology_scroll(max_scroll);
            return;
        }

        let max_scroll = self.current_agent_max_scroll();
        let current = self.current_agent_scroll_offset().min(max_scroll);

        if delta < 0 {
            let next = current.saturating_sub(delta.unsigned_abs() as usize);
            self.tui_state.agent_scroll = Some(next);
        } else {
            let next = current.saturating_add(delta as usize).min(max_scroll);
            if next >= max_scroll {
                self.tui_state.agent_scroll = None;
            } else {
                self.tui_state.agent_scroll = Some(next);
            }
        }

        self.tui_state.clamp_agent_scroll(max_scroll);
    }

    fn scroll_logs_by(&mut self, delta: i32) {
        let max_scroll = self.current_log_max_scroll();
        let current = if self.tui_state.log_auto_tail {
            max_scroll
        } else {
            self.tui_state.log_scroll.min(max_scroll)
        };

        if delta < 0 {
            self.tui_state.log_auto_tail = false;
            self.tui_state.log_scroll = current.saturating_sub(delta.unsigned_abs() as usize);
        } else {
            let next = current.saturating_add(delta as usize).min(max_scroll);
            if next >= max_scroll {
                self.tui_state.log_auto_tail = true;
                self.tui_state.log_scroll = 0;
            } else {
                self.tui_state.log_auto_tail = false;
                self.tui_state.log_scroll = next;
            }
        }

        self.tui_state.clamp_log_scroll(max_scroll);
    }

    fn current_content_area(&self) -> Rect {
        let full_area = Rect::new(0, 0, self.terminal_size.0, self.terminal_size.1);
        let content_area = super::layout::responsive_outer_margin(full_area);
        let has_waves = !self.tui_state.execution_waves.is_empty();
        let wave_row_height = if has_waves { 1 } else { 0 };
        let warning_height = super::widgets::header_bar::warning_bar_height(&self.tui_state);
        let sub_views = views::SubView::for_tab(self.tui_state.active_tab);
        let subview_height = u16::from(sub_views.len() > 1);
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),               // header
                Constraint::Length(warning_height),  // warning
                Constraint::Length(wave_row_height), // wave
                Constraint::Length(1),               // breadcrumb
                Constraint::Length(subview_height),  // sub-views
                Constraint::Min(0),                  // content
                Constraint::Length(1),               // footer
            ])
            .split(content_area);
        self.split_content_area(main_layout[5]).0
    }

    fn clamp_scroll_state_to_view(&mut self) {
        match self.tui_state.active_tab {
            Tab::Dashboard | Tab::Agents => {
                let max_scroll = self.current_agent_max_scroll();
                self.tui_state.clamp_agent_scroll(max_scroll);
                self.tui_state
                    .clamp_agent_topology_scroll(self.current_agent_topology_max_scroll());
            }
            Tab::Git => {
                let max = self.current_git_max_scroll();
                self.tui_state.git_detail_scroll = self.tui_state.git_detail_scroll.min(max);
            }
            Tab::Logs => {
                self.tui_state
                    .clamp_log_scroll(self.current_log_max_scroll());
            }
            // Remaining tabs: clamp selection indices to their list lengths.
            Tab::Plans => {
                let plan_count = self.tui_state.plans.len();
                if plan_count > 0 {
                    self.tui_state.selected_plan_idx =
                        self.tui_state.selected_plan_idx.min(plan_count.saturating_sub(1));
                }
            }
            Tab::Config => {
                // Config key list length varies; no dynamic content to clamp against
                // without accessing the config renderer, so leave as-is.
            }
            Tab::Marketplace => {
                if !self.tui_state.marketplace_jobs.is_empty() {
                    let max = self.tui_state.marketplace_jobs.len().saturating_sub(1);
                    self.tui_state.marketplace_selected_job =
                        self.tui_state.marketplace_selected_job.min(max);
                }
            }
            Tab::Atelier => {
                if !self.tui_state.atelier_prds.is_empty() {
                    let max = self.tui_state.atelier_prds.len().saturating_sub(1);
                    self.tui_state.atelier_selected_prd =
                        self.tui_state.atelier_selected_prd.min(max);
                }
            }
            Tab::Inspect | Tab::Learning => {}
        }
    }

    fn current_agent_output_line_count(&self) -> usize {
        match self.tui_state.active_tab {
            Tab::Agents => views::agents_view::collect_agent_output_lines(
                &self.tui_state,
                self.current_view_state().selected,
            )
            .len(),
            Tab::Dashboard if self.tui_state.plan_detail_tab == 1 => {
                let collected: Vec<String> = self
                    .data
                    .current_plan_execution
                    .as_ref()
                    .map(|exec| exec.agent_output_tail.clone())
                    .unwrap_or_default();

                if !collected.is_empty() {
                    return collected.len();
                }

                if let Some(agent) = self.tui_state.agents.get(
                    self.tui_state
                        .selected_agent
                        .min(self.tui_state.agents.len().saturating_sub(1)),
                ) {
                    if !agent.output_lines.is_empty() {
                        return agent.output_lines.len();
                    }
                }

                self.data
                    .task_outputs
                    .values()
                    .max_by_key(|lines| lines.len())
                    .map_or(0, Vec::len)
            }
            _ => 0,
        }
    }

    fn current_agent_output_viewport_height(&self) -> usize {
        let content_area = self.current_content_area();

        match self.tui_state.active_tab {
            Tab::Agents => {
                let panels = Layout::horizontal([
                    Constraint::Percentage(32),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .split(content_area);
                let sections =
                    Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(panels[2]);
                sections[1].height.saturating_sub(2) as usize
            }
            Tab::Dashboard if self.tui_state.plan_detail_tab == 1 => {
                let right = if self.tui_state.plans.iter().any(|plan| plan.active) {
                    let main = Layout::horizontal([
                        Constraint::Percentage(38),
                        Constraint::Length(1),
                        Constraint::Min(0),
                    ])
                    .split(content_area);
                    main[2]
                } else {
                    content_area
                };
                let sections =
                    Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(right);
                sections[1].height.saturating_sub(2) as usize
            }
            _ => 0,
        }
    }

    fn current_agent_topology_viewport_height(&self) -> usize {
        let content_area = self.current_content_area();

        match self.tui_state.active_tab {
            Tab::Agents => {
                let panels = Layout::horizontal([
                    Constraint::Percentage(32),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .split(content_area);
                let sections =
                    Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(panels[2]);
                sections[1].height.saturating_sub(3) as usize
            }
            _ => 0,
        }
    }

    fn split_content_area(&self, area: Rect) -> (Rect, Option<Rect>) {
        if self.tui_state.is_text_input() && area.height > 0 {
            let sections =
                Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
            (sections[0], Some(sections[1]))
        } else {
            (area, None)
        }
    }

    fn render_input_bar(&self, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let label = self.tui_state.input_mode_label();
        if label.is_empty() || area.width == 0 {
            return;
        }

        let buffer = match self.tui_state.input_mode {
            InputMode::Inject => self.tui_state.message_input.as_str(),
            InputMode::Filter => self.tui_state.filter_text.as_str(),
            InputMode::LogSearch => self.tui_state.log_search.pattern.as_str(),
            InputMode::PlanFilter => self.tui_state.plan_tree_filter.pattern.as_str(),
            _ => return,
        };

        // Build suffix for search mode (match count + filter mode indicator)
        let suffix = match self.tui_state.input_mode {
            InputMode::LogSearch if !self.tui_state.log_search.pattern.is_empty() => {
                let mode_label = match self.tui_state.log_search.mode {
                    super::state::SearchMode::Highlight => "highlight",
                    super::state::SearchMode::Filter => "filter",
                };
                if self.tui_state.log_search.pattern_error {
                    " [invalid regex]".to_string()
                } else {
                    format!(
                        " [{}/{} {}]",
                        self.tui_state.log_search.current_match + 1,
                        self.tui_state.log_search.match_count,
                        mode_label,
                    )
                }
            }
            _ => String::new(),
        };

        let prefix = format!("[{label}]");
        let horizontal_scroll =
            (prefix.chars().count() + 3 + buffer.chars().count() + suffix.chars().count() + 1)
                .saturating_sub(area.width as usize) as u16;
        let mut spans = vec![
            Span::styled(prefix, theme.accent_bold()),
            Span::styled(" > ", theme.muted()),
            Span::styled(buffer, theme.text()),
            Span::styled("│", theme.selection()),
        ];
        if !suffix.is_empty() {
            spans.push(Span::styled(suffix, theme.muted()));
        }
        let input = Paragraph::new(Line::from(spans))
            .style(theme.text().bg(Theme::BG_SECONDARY))
            .scroll((0, horizontal_scroll));

        frame.render_widget(Clear, area);
        frame.render_widget(input, area);
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        // When a modal is open, route scroll to the modal and consume clicks.
        if self.tui_state.active_modal.is_some() {
            let action = match mouse.kind {
                MouseEventKind::ScrollUp => TuiAction::ModalScrollUp,
                MouseEventKind::ScrollDown => TuiAction::ModalScrollDown,
                _ => TuiAction::None,
            };
            self.dispatch_action(action);
            return;
        }

        let action = match mouse.kind {
            MouseEventKind::Down(crossterm::event::MouseButton::Left) => TuiAction::MouseClick {
                x: mouse.column,
                y: mouse.row,
            },
            MouseEventKind::ScrollUp => TuiAction::MouseScrollUp {
                x: mouse.column,
                y: mouse.row,
            },
            MouseEventKind::ScrollDown => TuiAction::MouseScrollDown {
                x: mouse.column,
                y: mouse.row,
            },
            _ => TuiAction::None,
        };
        self.dispatch_action(action);
    }

    // -----------------------------------------------------------------------
    // Rendering helpers
    // -----------------------------------------------------------------------

    fn render_tab_header(&self, frame: &mut Frame<'_>, area: Rect, _theme: &Theme) {
        // Use the Mori-ported header_bar widget with full progress/ETA/tokens
        super::widgets::header_bar::render_header_bar(frame, area, &self.tui_state);
    }

    fn render_subview_bar(&self, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let tab = self.tui_state.active_tab;
        let active = self.tui_state.sub_tab_for(tab);
        let mut spans = vec![Span::styled(" ", Theme::block_style())];
        for (index, subview) in views::SubView::for_tab(tab).iter().enumerate() {
            let style = if index == active {
                theme
                    .selection()
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                theme.muted()
            };
            spans.push(Span::styled(
                format!(" Alt+{}:{} ", index + 1, subview.label()),
                style,
            ));
        }
        frame.render_widget(
            Paragraph::new(Line::from(spans)).style(Theme::block_style()),
            area,
        );
    }

    fn render_status_footer(&self, frame: &mut Frame<'_>, area: Rect, _theme: &Theme) {
        // Use the Mori-ported status_bar widget with context-sensitive hints
        super::widgets::status_bar::render_status_bar(frame, area, &self.tui_state);
    }

    fn expire_notifications(&mut self) {
        // Move expired notifications to history before removing them.
        let mut i = 0;
        while i < self.notifications.len() {
            if self.notifications[i].is_expired() {
                if let Some(expired) = self.notifications.remove(i) {
                    self.push_notification_history(expired);
                }
            } else {
                i += 1;
            }
        }
        // Hard cap at 20 entries to prevent unbounded memory growth.
        const MAX_NOTIFICATIONS: usize = 20;
        while self.notifications.len() > MAX_NOTIFICATIONS {
            if let Some(overflow) = self.notifications.pop_front() {
                self.push_notification_history(overflow);
            }
        }
    }

    /// Push a notification into the retained history ring buffer, evicting
    /// the oldest entry when the 200-entry cap is reached.
    fn push_notification_history(&mut self, notif: super::modals::Notification) {
        let id = self.tui_state.notification_next_id;
        self.tui_state.notification_next_id += 1;
        let record = super::modals::NotificationRecord {
            id,
            created_at: notif.created,
            level: notif.level,
            source: String::new(),
            message: super::modals::redact_message(&notif.message),
            related_run: None,
            related_task: None,
            dismissed_at: None,
        };
        self.tui_state.notification_history.push_back(record);
        while self.tui_state.notification_history.len() > super::modals::MAX_HISTORY {
            self.tui_state.notification_history.pop_front();
            self.tui_state.notification_evicted_count += 1;
        }
    }

    fn has_modal(&self) -> bool {
        self.tui_state.active_modal.is_some()
    }

    /// Build the tick-policy inputs from current `App` state.
    ///
    /// This keeps the pure `next_tick_policy()` function free of `App`
    /// coupling while letting both event loops share identical policy.
    fn tick_policy_inputs(&self) -> TickPolicyInputs {
        TickPolicyInputs {
            has_active_agents: self.tui_state.agents.iter().any(|a| a.active),
            has_active_plans: self.tui_state.plans.iter().any(|p| p.active),
            has_modal: self.has_modal(),
            has_notifications: !self.notifications.is_empty(),
            has_tab_transition: self.tab_transition.is_some(),
            has_postfx: self.fx_config.screen_postfx,
            since_last_input: self.frame_stats.since_last_input(),
        }
    }

    /// Select the current tick policy and return its duration.
    fn current_tick_duration(&self) -> std::time::Duration {
        next_tick_policy(&self.tick_policy_inputs()).duration()
    }

    fn dismiss_all_modals(&mut self) {
        if matches!(
            self.tui_state.active_modal,
            Some(ModalState::Approval { .. })
        ) {
            let _ = self.resolve_active_approval(false);
        }
        self.tui_state.active_modal = None;
        self.tui_state.pending_confirm = None;
        if self.tui_state.input_mode == InputMode::Confirm {
            self.tui_state.input_mode = InputMode::Normal;
        }
    }

    /// Full refresh — async version for the connected `run()` path.
    async fn refresh_snapshot_async(&mut self) {
        if self.replay_disk_snapshots || self._state_hub.is_none() {
            self.data = DashboardData::load_best_effort(&self.workdir);
            self.scaffold = DashboardScaffold::new_in(&self.workdir);
            self.last_data_gen = self.data.generation;
            self.tui_state.update_from_snapshot(&self.data);
            if let Some(state_hub) = &self._state_hub {
                let _ = state_hub.bootstrap_from_workdir(&self.workdir);
                let events_path = self.workdir.join(".roko").join("events.jsonl");
                state_hub.replay_log_into_snapshot(&events_path);
            }
        }
        self.reseed_verdicts_aggregator().await;
        self.refresh_verdicts_from_aggregator().await;
        self.fx_config = EffectsConfig::load_from_root(&self.workdir);
        // Refresh cached inspect data on the 5-second cadence (P3.3).
        if self.tui_state.inspect_needs_refresh() {
            self.tui_state.refresh_inspect_data();
        }
        // Refresh cached config items on the 5-second cadence (P3.2).
        if self.tui_state.config_needs_refresh() {
            self.tui_state.invalidate_config_cache();
        }
        if self.tui_state.mcp_config_needs_refresh() {
            self.tui_state.refresh_mcp_config_view();
        }
        if self.tui_state.conductor_snapshot_needs_refresh() {
            self.tui_state.refresh_conductor_snapshot();
        }
        self.last_refresh = Instant::now();
        self.clamp_signal_selection();
        self.clamp_gate_failure_selection();
        if self.pages().scaffold(self.current_page).is_none() {
            self.current_page = self.scaffold.active_page();
        }
    }

    /// Full refresh — sync version for the standalone `main_loop` path.
    fn refresh_snapshot(&mut self) {
        if self.replay_disk_snapshots || self._state_hub.is_none() {
            self.data = DashboardData::load_best_effort(&self.workdir);
            self.scaffold = DashboardScaffold::new_in(&self.workdir);
            self.last_data_gen = self.data.generation;
            self.tui_state.update_from_snapshot(&self.data);
            if let Some(state_hub) = &self._state_hub {
                let _ = state_hub.bootstrap_from_workdir(&self.workdir);
                let events_path = self.workdir.join(".roko").join("events.jsonl");
                state_hub.replay_log_into_snapshot(&events_path);
            }
        }
        self.reseed_verdicts_aggregator_blocking();
        self.refresh_verdicts_from_aggregator_blocking();
        self.fx_config = EffectsConfig::load_from_root(&self.workdir);
        // Refresh cached inspect data on the 5-second cadence (P3.3).
        if self.tui_state.inspect_needs_refresh() {
            self.tui_state.refresh_inspect_data();
        }
        // Refresh cached config items on the 5-second cadence (P3.2).
        if self.tui_state.config_needs_refresh() {
            self.tui_state.invalidate_config_cache();
        }
        if self.tui_state.mcp_config_needs_refresh() {
            self.tui_state.refresh_mcp_config_view();
        }
        if self.tui_state.conductor_snapshot_needs_refresh() {
            self.tui_state.refresh_conductor_snapshot();
        }
        self.last_refresh = Instant::now();
        self.clamp_signal_selection();
        self.clamp_gate_failure_selection();
        if self.pages().scaffold(self.current_page).is_none() {
            self.current_page = self.scaffold.active_page();
        }
    }

    #[allow(deprecated)] // tick() is deprecated but still needed for standalone mode
    fn tick_snapshot(&mut self) {
        if let Err(error) = self.data.tick() {
            tracing::warn!(
                error = %error,
                "dashboard incremental tick failed; falling back to full reload"
            );
            self.refresh_snapshot();
            return;
        }

        self.last_data_gen = self.data.generation;
        self.tui_state.update_from_snapshot(&self.data);
        self.refresh_verdicts_from_aggregator_blocking();
        self.last_refresh = Instant::now();
        self.clamp_signal_selection();
        self.clamp_gate_failure_selection();
    }

    async fn reseed_verdicts_aggregator(&mut self) {
        self.verdicts_aggregator = VerdictsAggregator::open(&self.workdir).await.ok();
    }

    async fn refresh_verdicts_from_aggregator(&mut self) {
        let Some(aggregator) = self.verdicts_aggregator.as_mut() else {
            self.tui_state.gate_trends.clear();
            self.tui_state.gate_recent_failures.clear();
            return;
        };

        if let Err(error) = aggregator.tick().await {
            tracing::warn!(
                error = %error,
                "verdicts aggregation tick failed"
            );
            return;
        }

        self.tui_state.gate_trends = aggregator.gate_trends();
        self.tui_state.gate_recent_failures = aggregator.recent_failures();

        if let Some(state_hub) = &self._state_hub {
            state_hub.update_snapshot(|snapshot| {
                snapshot.gate_trends = self.tui_state.gate_trends.clone();
                snapshot.gate_recent_failures = self.tui_state.gate_recent_failures.clone();
            });
        }
    }

    /// Sync variant of [`Self::reseed_verdicts_aggregator`] for the
    /// standalone `main_loop` path (no Tokio runtime active).
    fn reseed_verdicts_aggregator_blocking(&mut self) {
        self.verdicts_aggregator = VerdictsAggregator::open_blocking(&self.workdir).ok();
    }

    /// Sync variant of [`Self::refresh_verdicts_from_aggregator`] for the
    /// standalone `main_loop` path (no Tokio runtime active).
    fn refresh_verdicts_from_aggregator_blocking(&mut self) {
        let Some(aggregator) = self.verdicts_aggregator.as_mut() else {
            self.tui_state.gate_trends.clear();
            self.tui_state.gate_recent_failures.clear();
            return;
        };

        if let Err(error) = aggregator.tick_blocking() {
            tracing::warn!(
                error = %error,
                "verdicts aggregation tick failed"
            );
            return;
        }

        self.tui_state.gate_trends = aggregator.gate_trends();
        self.tui_state.gate_recent_failures = aggregator.recent_failures();

        if let Some(state_hub) = &self._state_hub {
            state_hub.update_snapshot(|snapshot| {
                snapshot.gate_trends = self.tui_state.gate_trends.clone();
                snapshot.gate_recent_failures = self.tui_state.gate_recent_failures.clone();
            });
        }
    }

    fn save_config_changes(&mut self) {
        if self.tui_state.config_pending.is_empty() {
            self.notifications.push_back(super::modals::Notification::info(
                "No pending changes to save",
            ));
            return;
        }

        match super::config_meta::save_pending_edits(&self.workdir, &self.tui_state.config_pending)
        {
            Ok(()) => {
                self.tui_state.config_pending.clear();
                self.tui_state.invalidate_config_cache();
                self.fx_config = EffectsConfig::load_from_root(&self.workdir);
                self.pending_refresh = true;
                self.notifications.push_back(super::modals::Notification::info(
                    "Config saved and reloaded",
                ));
            }
            Err(error) => {
                self.notifications
                    .push_back(super::modals::Notification::error(&format!(
                        "Save failed: {error}"
                    )));
            }
        }
    }

    /// Submit the CreateJob form: write a JSON file to `.roko/jobs/` so the
    /// file-watcher and job_runner pick it up automatically.
    fn submit_marketplace_job(&mut self) {
        let title = self.tui_state.job_form_title.trim().to_string();
        if title.is_empty() {
            self.notifications
                .push_back(super::modals::Notification::warn("Job title is required"));
            return;
        }

        let job_type = {
            let t = self.tui_state.job_form_type.trim().to_string();
            if t.is_empty() {
                "coding_task".to_string()
            } else {
                t
            }
        };
        let priority = {
            let p = self.tui_state.job_form_priority.trim().to_string();
            if p.is_empty() {
                "medium".to_string()
            } else {
                p
            }
        };
        let description = self.tui_state.job_form_description.trim().to_string();

        let now = chrono::Utc::now().to_rfc3339();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        let id = format!(
            "job-{}-{:04x}",
            now.replace([':', '-', 'T', '+'], "")
                .get(..14)
                .unwrap_or("0"),
            nanos & 0xFFFF
        );

        let job = roko_core::MarketplaceJob {
            id: id.clone(),
            title: title.clone(),
            description,
            job_type,
            status: "pending".to_string(),
            priority,
            posted_by: "tui".to_string(),
            created_at: now.clone(),
            updated_at: now,
            ..Default::default()
        };

        let jobs_dir = self.workdir.join(".roko").join("jobs");
        if let Err(e) = std::fs::create_dir_all(&jobs_dir) {
            self.notifications
                .push_back(super::modals::Notification::error(&format!(
                    "Failed to create jobs directory: {e}"
                )));
            return;
        }

        let path = jobs_dir.join(format!("{id}.json"));
        match serde_json::to_string_pretty(&job) {
            Ok(json) => match std::fs::write(&path, json) {
                Ok(()) => {
                    self.tui_state
                        .command_results
                        .push(super::state::CommandResult {
                            ok: true,
                            label: "create-job".to_string(),
                            message: format!("Created job '{title}' ({id})"),
                        });
                    // Reset form fields.
                    self.tui_state.job_form_title.clear();
                    self.tui_state.job_form_type.clear();
                    self.tui_state.job_form_priority.clear();
                    self.tui_state.job_form_description.clear();
                    self.tui_state.job_form_editing = false;

                    self.pending_refresh = true;
                    self.notifications
                        .push_back(super::modals::Notification::info(format!(
                            "Job '{title}' created"
                        )));
                }
                Err(e) => {
                    self.notifications
                        .push_back(super::modals::Notification::error(&format!(
                            "Failed to write job file: {e}"
                        )));
                }
            },
            Err(e) => {
                self.notifications
                    .push_back(super::modals::Notification::error(&format!(
                        "Failed to serialize job: {e}"
                    )));
            }
        }
    }

    fn current_view_state(&self) -> ViewState {
        match self.tui_state.active_tab {
            Tab::Dashboard => ViewState {
                scroll: self.tui_state.agent_scroll.unwrap_or(0) as u16,
                selected: self.tui_state.selected_plan_idx,
                sub_tab: self.tui_state.sub_tab_for(Tab::Dashboard),
                secondary_selected: 0,
                auto_tail: self.tui_state.agent_scroll.is_none(),
                search_query: self.tui_state.filter.clone(),
            },
            Tab::Plans => ViewState {
                scroll: self.tui_state.plan_scroll_offset as u16,
                selected: self.tui_state.selected_plan_idx,
                sub_tab: self.tui_state.plan_detail_tab,
                secondary_selected: 0,
                auto_tail: false,
                search_query: self.tui_state.filter.clone(),
            },
            Tab::Agents => ViewState {
                scroll: self.tui_state.agent_scroll.unwrap_or(0) as u16,
                selected: self.tui_state.selected_agent,
                sub_tab: self.tui_state.selected_agent_tab,
                secondary_selected: 0,
                auto_tail: self.tui_state.agent_scroll.is_none(),
                search_query: self.tui_state.filter.clone(),
            },
            Tab::Git => ViewState {
                scroll: self.tui_state.git_detail_scroll.min(u16::MAX as usize) as u16,
                selected: self.tui_state.git_branch_cursor,
                sub_tab: self.tui_state.sub_tab_for(Tab::Git),
                secondary_selected: 0,
                auto_tail: false,
                search_query: self.tui_state.filter.clone(),
            },
            Tab::Logs => ViewState {
                scroll: self.tui_state.log_scroll.min(u16::MAX as usize) as u16,
                selected: 0,
                sub_tab: self.tui_state.sub_tab_for(Tab::Logs),
                secondary_selected: 0,
                auto_tail: self.tui_state.log_auto_tail,
                search_query: self.tui_state.filter.clone(),
            },
            Tab::Config => ViewState {
                scroll: self.tui_state.config_scroll_offset.min(u16::MAX as usize) as u16,
                selected: self.tui_state.config_cursor,
                sub_tab: self.tui_state.sub_tab_for(Tab::Config),
                secondary_selected: 0,
                auto_tail: false,
                search_query: self.tui_state.filter.clone(),
            },
            Tab::Inspect => ViewState {
                scroll: self.tui_state.inspect_detail_scroll.min(u16::MAX as usize) as u16,
                selected: 0,
                sub_tab: self.tui_state.sub_tab_for(Tab::Inspect),
                secondary_selected: 0,
                auto_tail: false,
                search_query: self.tui_state.filter.clone(),
            },
            Tab::Marketplace => ViewState {
                scroll: 0,
                selected: self.tui_state.marketplace_selected_job,
                sub_tab: self.tui_state.sub_tab_for(Tab::Marketplace),
                secondary_selected: 0,
                auto_tail: false,
                search_query: self.tui_state.filter.clone(),
            },
            Tab::Atelier => ViewState {
                scroll: 0,
                selected: self.tui_state.atelier_selected_prd,
                sub_tab: self.tui_state.sub_tab_for(Tab::Atelier),
                secondary_selected: 0,
                auto_tail: false,
                search_query: self.tui_state.filter.clone(),
            },
            Tab::Learning => ViewState {
                scroll: 0,
                selected: 0,
                sub_tab: self.tui_state.sub_tab_for(Tab::Learning),
                secondary_selected: 0,
                auto_tail: false,
                search_query: String::new(),
            },
        }
    }

    fn git_branch_count(&self) -> usize {
        self.tui_state
            .git_view_data
            .as_ref()
            .map_or(self.tui_state.git_branch_tree.len(), |data| {
                data.branches.len()
            })
    }

    // `update_sys_metrics` removed -- see `collect_sys_metrics_bg()` standalone
    // function below, called from the background thread.

    /// Drain all background channels (sys metrics, data refresh, git,
    /// command acks) without blocking.  Called on every tick and after every
    /// keypress so the UI reflects the latest data produced by background
    /// threads and the executor.
    fn drain_background_channels(&mut self) {
        const MAX_MESSAGES_PER_DRAIN: usize = 20;

        self.drain_snapshot_channel();
        self.drain_state_events();
        self.drain_execution_acks();
        self.drain_agent_topology_fetch();
        self.sync_agent_stream_clients();
        self.drain_agent_stream_clients();

        // -- sys metrics (merge, don't replace — keep history) --
        if let Some(rx) = &mut self.sys_rx {
            if rx.has_changed().unwrap_or(false) {
                let snap = rx.borrow_and_update().clone();
                // CPU
                let cpu_pct = self.tui_state.update_cpu_pct(snap.sys.cpu_pct);
                let sys = &mut self.tui_state.sys;
                sys.cpu_history.push_back(cpu_pct);
                while sys.cpu_history.len() > super::state::MAX_METRIC_HISTORY {
                    sys.cpu_history.pop_front();
                }

                // Memory
                sys.mem_used_bytes = snap.sys.mem_used_bytes;
                sys.mem_total_bytes = snap.sys.mem_total_bytes;
                let mem_frac = if snap.sys.mem_total_bytes > 0 {
                    snap.sys.mem_used_bytes as f32 / snap.sys.mem_total_bytes as f32
                } else {
                    0.0
                };
                sys.mem_history.push_back(mem_frac);
                while sys.mem_history.len() > super::state::MAX_METRIC_HISTORY {
                    sys.mem_history.pop_front();
                }

                // Network + Disk: collector already computes bytes/sec rates.
                sys.net_down_bytes_sec = snap.sys.net_down_bytes_sec;
                sys.net_up_bytes_sec = snap.sys.net_up_bytes_sec;
                sys.disk_read_bytes_sec = snap.sys.disk_read_bytes_sec;
                sys.disk_write_bytes_sec = snap.sys.disk_write_bytes_sec;
                sys.disk_free_bytes = snap.sys.disk_free_bytes;
                sys.disk_total_bytes = snap.sys.disk_total_bytes;
                self.merge_process_metrics(snap.process_metrics);
                self.render_dirty.insert(RenderDirty::METRICS);
            }
        }

        // -- debounced filesystem refresh --
        if let Some(fs_watch) = &self.fs_watch {
            let mut got_refresh = false;
            let mut count = 0;
            while let Ok(FsRefresh::Coalesced) = fs_watch.try_recv() {
                got_refresh = true;
                count += 1;
                if count >= MAX_MESSAGES_PER_DRAIN {
                    break;
                }
            }
            if got_refresh {
                self.render_dirty.insert(RenderDirty::SNAPSHOT);
                // Incremental refresh (RC-6): avoid full re-bootstrap by
                // checking whether the snapshot file actually changed.  We
                // use the file size as a cheap proxy for content changes
                // (avoids hashing on every filesystem event).  Only new
                // events.jsonl lines past `last_events_offset` are replayed.
                if let Some(state_hub) = &self._state_hub {
                    if self.replay_disk_snapshots {
                        let snap_path = self
                            .workdir
                            .join(".roko")
                            .join("state")
                            .join("state-snapshot.json");
                        let snap_size = std::fs::metadata(&snap_path).ok().map(|m| m.len());
                        let snap_changed = snap_size != self.last_snapshot_hash;
                        if snap_changed {
                            let _ = state_hub.bootstrap_from_workdir(&self.workdir);
                            self.last_snapshot_hash = snap_size;
                        }
                        // Incremental event replay: only read bytes past the
                        // last offset instead of replaying the entire log.
                        let events_path = self.workdir.join(".roko").join("events.jsonl");
                        if let Ok(meta) = std::fs::metadata(&events_path) {
                            let file_len = meta.len();
                            if file_len > self.last_events_offset {
                                if let Ok(file) = std::fs::File::open(&events_path) {
                                    use std::io::{Seek, SeekFrom};
                                    let mut reader = std::io::BufReader::new(file);
                                    if reader
                                        .seek(SeekFrom::Start(self.last_events_offset))
                                        .is_ok()
                                    {
                                        state_hub.replay_events_from_reader(&mut reader);
                                    }
                                }
                                self.last_events_offset = file_len;
                            }
                        }
                    }
                } else if self.snapshot_rx.is_none() {
                    // Legacy fallback: no StateHub and no snapshot_rx.
                    self.tick_snapshot();
                }
            }
        }

        // -- git data: spawn background collection when the watcher fires --
        if let Some(git_watch) = &self.git_watch {
            let mut got_refresh = false;
            let mut count = 0;
            while let Ok(GitRefresh::Coalesced) = git_watch.try_recv() {
                got_refresh = true;
                count += 1;
                if count >= MAX_MESSAGES_PER_DRAIN {
                    break;
                }
            }
            // Only spawn a new job when the watcher fired AND no job is
            // currently in flight. This bounds concurrency to one thread.
            if got_refresh && self.git_bg_rx.is_none() {
                self.git_bg_generation += 1;
                let generation = self.git_bg_generation;
                let workdir = self.workdir.clone();
                let (tx, rx) = std::sync::mpsc::sync_channel(1);
                self.git_bg_rx = Some(rx);
                std::thread::Builder::new()
                    .name("tui-git-collect".into())
                    .spawn(move || {
                        let data = collect_git_bg_data(&workdir);
                        let _ = tx.send((generation, data));
                    })
                    .ok();
            }
        }

        // -- git data: drain completed background result --
        if let Some(rx) = &self.git_bg_rx {
            if let Ok((completed_gen, data)) = rx.try_recv() {
                if completed_gen >= self.git_applied_generation {
                    self.git_applied_generation = completed_gen;
                    self.apply_git_bg_data(data);
                    self.render_dirty.insert(RenderDirty::SNAPSHOT);
                }
                self.git_bg_rx = None; // channel consumed, allow new requests
            }
        }
    }

    fn apply_git_bg_data(&mut self, bg: GitBgData) {
        // Derive commit/worktree views before moving view_data.
        self.tui_state.git_commit_graph = convert_git_commit_graph(&bg.view_data.commits);
        self.tui_state.git_worktree_list = convert_git_worktree_list(&bg.view_data.worktrees);
        // git_branch_tree accessed via git_view_data when present; skip clone.
        self.tui_state.git_view_data = Some(bg.view_data);
        self.tui_state.git_summary_lines = bg.summary_lines;
        self.tui_state.git_branch = bg.branch;
        self.tui_state.git_commit_short = bg.commit_short;
        self.tui_state.git_age = bg.age;
    }

    fn drain_snapshot_channel(&mut self) {
        let Some(rx) = self.snapshot_rx.as_mut() else {
            return;
        };

        if !rx.has_changed().unwrap_or(false) {
            return;
        }

        // The watch::Ref holds a read lock that must be dropped before we can
        // mutate self. Clone is unavoidable here (the sender owns the value),
        // but the downstream apply path now uses revision-based caching (#366)
        // so the expensive unified log rebuild is skipped when inputs are unchanged.
        let snapshot = rx.borrow_and_update().clone();
        apply_dashboard_snapshot(
            &mut self.tui_state,
            &mut self.notifications,
            &mut self.last_snapshot_error_marker,
            &mut self.last_seen_gate_count,
            &mut self.last_seen_plan_phases,
            &snapshot,
        );
        self.update_plan_completion_exit(&snapshot);
        self.render_dirty.insert(RenderDirty::SNAPSHOT);
    }

    /// Drain pending command acknowledgements from the executor and update
    /// TUI state accordingly.
    fn drain_execution_acks(&mut self) {
        let Some(ack_rx) = self.exec_ack_receiver.as_mut() else {
            return;
        };
        for ack in ack_rx.drain() {
            use crate::execution_control::CommandAckStatus;
            match ack.status {
                CommandAckStatus::Accepted => {
                    let msg = ack.message.unwrap_or_else(|| "command accepted".into());
                    self.notifications.push_back(super::modals::Notification::info(msg));
                }
                CommandAckStatus::Completed => {
                    let msg = ack.message.unwrap_or_else(|| "command completed".into());
                    self.notifications.push_back(super::modals::Notification::info(msg));
                    self.render_dirty.insert(RenderDirty::SNAPSHOT);
                }
                CommandAckStatus::Rejected => {
                    let msg = ack.message.unwrap_or_else(|| "command rejected".into());
                    self.notifications.push_back(super::modals::Notification::warn(msg));
                }
                CommandAckStatus::Failed => {
                    let msg = ack.message.unwrap_or_else(|| "command failed".into());
                    self.notifications.push_back(super::modals::Notification::error(msg));
                }
            }
        }
    }

    fn drain_state_events(&mut self) {
        const MAX_EVENTS: usize = 256;
        let Some(subscription) = self.state_events.as_mut() else {
            return;
        };
        let mut events = Vec::new();
        for envelope in subscription.replay.drain(..).take(MAX_EVENTS) {
            events.push(envelope.payload);
        }
        while events.len() < MAX_EVENTS {
            match subscription.live.try_recv() {
                Ok(envelope) => events.push(envelope.payload),
                Err(tokio::sync::broadcast::error::TryRecvError::Empty) => break,
                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(dropped)) => {
                    self.tui_state.push_agent_chunk(
                        "system",
                        format!("[stream lagged: {dropped} StateHub events; snapshot resynced]"),
                    );
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Closed) => break,
            }
        }
        for event in events {
            let roko_core::DashboardEvent::AgentOutput {
                agent_id, content, ..
            } = event
            else {
                continue;
            };
            let Some(record) =
                content.strip_prefix(crate::runner::tui_bridge::STREAM_RECORD_PREFIX)
            else {
                continue;
            };
            let Ok(record) = serde_json::from_str::<serde_json::Value>(record) else {
                continue;
            };
            let kind = record
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("text");
            let payload = record.get("payload").cloned().unwrap_or_default();
            match kind {
                "text" => {
                    if let Some(text) = payload.get("text").and_then(serde_json::Value::as_str) {
                        self.tui_state.push_agent_chunk(&agent_id, text.to_string());
                    }
                }
                "reasoning" => {
                    if let Some(text) = payload.get("text").and_then(serde_json::Value::as_str) {
                        self.tui_state
                            .push_agent_chunk(&agent_id, format!("[thinking] {text}"));
                    }
                }
                "tool_start" => {
                    let tool = payload
                        .get("tool")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("tool");
                    let id = payload
                        .get("tool_id")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("");
                    self.tui_state
                        .push_agent_chunk(&agent_id, format!("[tool ⏵ {tool} {id}]"));
                }
                "tool_result" => {
                    let output = payload
                        .get("output")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("");
                    let id = payload
                        .get("tool_id")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("");
                    self.tui_state
                        .push_agent_chunk(&agent_id, format!("[tool ✓ {id}]\n{output}"));
                }
                _ => {}
            }
        }
    }

    fn drain_shutdown_signal(&mut self) {
        let Some(rx) = self.shutdown_rx.as_ref() else {
            return;
        };

        match rx.try_recv() {
            Ok(()) | Err(std_mpsc::TryRecvError::Disconnected) => {
                tracing::info!("TUI exiting: shutdown signal received");
                self.running = false;
            }
            Err(std_mpsc::TryRecvError::Empty) => {}
        }
    }

    fn update_plan_completion_exit(&mut self, snapshot: &roko_core::DashboardSnapshot) {
        if !self.exit_on_plan_completion {
            return;
        }

        let has_active_plan =
            snapshot.stats.plans_active > 0 || snapshot.plans.values().any(|plan| plan.active);
        let has_finished_plan = snapshot.stats.plans_completed > 0
            || snapshot.stats.plans_failed > 0
            || snapshot
                .plans
                .values()
                .any(|plan| !plan.active && (plan.phase == "completed" || plan.phase == "failed"));

        self.connected_plan_observed |= has_active_plan || has_finished_plan;

        if self.connected_plan_observed && !has_active_plan {
            tracing::info!("TUI exiting: all plans completed");
            self.running = false;
        }
    }

    fn request_agent_topology_refresh(&mut self) {
        if self.agent_topology_in_flight {
            return;
        }

        let (tx, rx) = std_mpsc::channel();
        let base_url = self.agent_stream_server_url.clone();
        self.agent_topology_rx = Some(rx);
        self.agent_topology_in_flight = true;
        self.tui_state.set_agent_topology_loading();

        match std::thread::Builder::new()
            .name("tui-agent-topology".into())
            .spawn(move || {
                let result = fetch_agent_topology(&base_url);
                let _ = tx.send(result);
            }) {
            Ok(_) => {}
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    thread = "tui-agent-topology",
                    "failed to spawn topology fetch thread"
                );
                self.agent_topology_in_flight = false;
                self.agent_topology_rx = None;
                self.tui_state
                    .set_agent_topology_error("topology fetch thread failed");
            }
        }
    }

    fn drain_agent_topology_fetch(&mut self) {
        let Some(rx) = &self.agent_topology_rx else {
            return;
        };

        let Ok(result) = rx.try_recv() else {
            return;
        };

        self.agent_topology_in_flight = false;
        self.agent_topology_rx = None;

        match result {
            AgentTopologyFetchResult::Ready(topology) => {
                self.tui_state.set_agent_topology(topology.clone());
                self.apply_state_hub_agent_topology(topology);
            }
            AgentTopologyFetchResult::Unavailable => {
                self.tui_state.set_agent_topology_unavailable();
                self.apply_state_hub_agent_topology(roko_core::AgentTopology::default());
            }
            AgentTopologyFetchResult::Error(message) => {
                self.tui_state.set_agent_topology_error(message);
            }
        }
    }

    fn apply_state_hub_agent_topology(&self, topology: roko_core::AgentTopology) {
        let Some(state_hub) = &self._state_hub else {
            return;
        };

        state_hub.update_snapshot(|snapshot| snapshot.agent_topology = topology);
    }

    fn sync_agent_stream_clients(&mut self) {
        if !matches!(self.tui_state.active_tab, Tab::Agents) {
            self.clear_agent_stream_clients();
            return;
        }

        let mut desired_ids = self
            .data
            .agents
            .iter()
            .map(|agent| agent.id.clone())
            .collect::<HashSet<_>>();
        desired_ids.extend(self.tui_state.agents.iter().map(|agent| agent.id.clone()));

        let stale_ids = self
            .agent_stream_clients
            .keys()
            .filter(|agent_id| !desired_ids.contains(*agent_id))
            .cloned()
            .collect::<Vec<_>>();
        for agent_id in stale_ids {
            self.agent_stream_clients.remove(&agent_id);
            self.tui_state.mark_agent_stream_disconnected(&agent_id);
        }

        let server_url = self.agent_stream_server_url.clone();
        let auth_token = self.agent_stream_auth_token.clone();
        for agent_id in desired_ids {
            if self.agent_stream_clients.contains_key(&agent_id) {
                continue;
            }
            if let Some(client) =
                AgentStreamClient::connect(&agent_id, &server_url, auth_token.clone())
            {
                self.agent_stream_clients.insert(agent_id, client);
            }
        }
    }

    fn clear_agent_stream_clients(&mut self) {
        if self.agent_stream_clients.is_empty() {
            return;
        }

        let active_ids = self
            .agent_stream_clients
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        self.agent_stream_clients.clear();
        for agent_id in active_ids {
            self.tui_state.mark_agent_stream_disconnected(&agent_id);
        }
    }

    fn drain_agent_stream_clients(&mut self) {
        let agent_ids = self
            .agent_stream_clients
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for agent_id in agent_ids {
            let Some(client) = self.agent_stream_clients.get_mut(&agent_id) else {
                continue;
            };

            loop {
                match client.try_recv() {
                    Ok(StreamChunk::Connected) => {
                        self.tui_state.mark_agent_stream_connected(&agent_id);
                    }
                    Ok(StreamChunk::Text(text)) => {
                        self.tui_state.push_agent_chunk(&agent_id, text);
                    }
                    Ok(StreamChunk::Reasoning(text)) => {
                        self.tui_state
                            .push_agent_chunk(&agent_id, format!("[reasoning] {text}"));
                    }
                    Ok(StreamChunk::ToolCall(tool_call)) => {
                        if let Ok(text) = serde_json::to_string(&tool_call) {
                            self.tui_state
                                .push_agent_chunk(&agent_id, format!("[tool_call] {text}"));
                        }
                    }
                    Ok(StreamChunk::Usage(usage)) => {
                        if let Ok(text) = serde_json::to_string(&usage) {
                            self.tui_state
                                .push_agent_chunk(&agent_id, format!("[usage] {text}"));
                        }
                    }
                    Ok(StreamChunk::Error(error)) => {
                        self.tui_state
                            .push_agent_chunk(&agent_id, format!("[error] {error}"));
                    }
                    Ok(StreamChunk::Done { session }) => {
                        if let Some(session_id) = session {
                            self.tui_state.push_agent_chunk(
                                &agent_id,
                                format!("[done] session {session_id}"),
                            );
                        }
                        self.tui_state.mark_agent_stream_done(&agent_id);
                    }
                    Ok(StreamChunk::Disconnected) => {
                        self.tui_state.mark_agent_stream_disconnected(&agent_id);
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                        self.tui_state.mark_agent_stream_disconnected(&agent_id);
                        break;
                    }
                }
            }
        }
    }

    fn pages(&self) -> PageRegistry {
        PageRegistry::from_dashboard(&self.scaffold)
    }

    #[allow(dead_code)]
    fn scroll_for(&self, page: PageId) -> u16 {
        self.scroll_offset.get(&page).copied().unwrap_or(0)
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

    #[allow(dead_code)]
    fn leave_terminal() -> Result<()> {
        Self::cleanup_terminal()
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
        let (net_rx_delta, net_tx_delta) =
            networks.iter().fold((0u64, 0u64), |(rx, tx), nic| {
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
                (
                    du.read_bytes / SAMPLE_SECS,
                    du.written_bytes / SAMPLE_SECS,
                )
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
        Tab::Git | Tab::Inspect | Tab::Marketplace | Tab::Atelier | Tab::Learning => None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use roko_core::config::RokoConfig;
    use tempfile::tempdir;

    fn rendered_text(terminal: &Terminal<TestBackend>) -> String {
        let buffer = terminal.backend().buffer();
        let width = buffer.area.width as usize;
        buffer
            .content
            .chunks(width)
            .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn app_starts_on_requested_page() {
        let dir = tempdir().unwrap();
        let app = App::new_with_page(dir.path(), Some(PageId::PlanView));
        assert_eq!(app.current_page(), PageId::PlanView);
    }

    #[test]
    fn app_has_tui_state() {
        let dir = tempdir().unwrap();
        let app = App::new(dir.path());
        assert_eq!(app.tui_state.active_tab, Tab::Dashboard);
        assert_eq!(app.tui_state.input_mode, InputMode::Normal);
    }

    #[test]
    fn tui_command_pause_toggle_sends_execution_commands() {
        let dir = tempdir().unwrap();
        let (sender, mut cmd_rx, _ack_tx, ack_rx) =
            crate::execution_control::ExecutionCommandSender::channel("test-run");
        let ack_receiver = crate::execution_control::CommandAckReceiver::new(ack_rx);
        let mut app = App::new(dir.path())
            .with_execution_command_sender(sender, ack_receiver);

        // First toggle: should send Pause
        app.dispatch_action(TuiAction::TogglePause);
        let received = cmd_rx.try_recv().unwrap();
        assert_eq!(
            received.kind,
            crate::execution_control::ExecutionCommandKind::Pause
        );
        assert_eq!(received.run_id, "test-run");
        // Pause request notification shown
        assert!(
            app.notifications.iter().any(|n| n.message.contains("Pause requested"))
        );

        // Second toggle: should send Resume
        app.dispatch_action(TuiAction::TogglePause);
        let received = cmd_rx.try_recv().unwrap();
        assert_eq!(
            received.kind,
            crate::execution_control::ExecutionCommandKind::Resume
        );
    }

    #[test]
    fn tui_standalone_pause_toggle_shows_notification() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());

        app.dispatch_action(TuiAction::TogglePause);

        assert!(!app.tui_state.is_paused);
        assert!(
            app.notifications
                .iter()
                .any(|notification| notification.message.contains("connected plan run"))
        );
    }

    #[test]
    fn app_new_connected_installs_snapshot_receiver() {
        let dir = tempdir().unwrap();
        let hub = crate::state_hub::shared_state_hub();
        let app = App::new_connected(dir.path(), &hub);
        assert!(app.snapshot_rx.is_some());
    }

    #[test]
    fn terminal_reset_sequence_disables_mouse_and_alternate_screen() {
        let sequence = std::str::from_utf8(TERMINAL_RESET_SEQUENCE).unwrap();
        assert!(sequence.contains("\x1b[?1000l"));
        assert!(sequence.contains("\x1b[?1002l"));
        assert!(sequence.contains("\x1b[?1003l"));
        assert!(sequence.contains("\x1b[?1006l"));
        assert!(sequence.contains("\x1b[?1049l"));
        assert!(sequence.contains("\x1b[?25h"));
    }

    #[test]
    fn app_defaults_to_keyboard_only_terminal_mode() {
        let dir = tempdir().unwrap();
        let app = App::new(dir.path());
        assert!(!app.capture_mouse);
    }

    #[test]
    fn approval_tui_can_disable_mouse_capture() {
        let dir = tempdir().unwrap();
        let app = App::new(dir.path()).without_mouse_capture();
        assert!(!app.capture_mouse);
    }

    #[test]
    fn shutdown_signal_stops_app() {
        let dir = tempdir().unwrap();
        let (shutdown_tx, shutdown_rx) = std_mpsc::channel();
        let mut app = App::new(dir.path()).with_shutdown_receiver(shutdown_rx);

        shutdown_tx.send(()).unwrap();
        app.drain_shutdown_signal();

        assert!(!app.running);
    }

    #[test]
    fn connected_app_exits_after_observed_plan_completion() {
        let dir = tempdir().unwrap();
        let hub = crate::state_hub::shared_state_hub();
        let mut app = App::new_connected(dir.path(), &hub).with_exit_on_plan_completion();

        hub.publish(roko_core::DashboardEvent::PlanStarted {
            plan_id: "live-plan".to_string(),
            tasks_total: 0,
        });
        app.drain_snapshot_channel();
        assert!(app.running);

        hub.publish(roko_core::DashboardEvent::PlanCompleted {
            plan_id: "live-plan".to_string(),
            success: true,
        });
        app.drain_snapshot_channel();
        assert!(!app.running);
    }

    #[test]
    fn connected_app_stays_open_after_plan_completion_by_default() {
        let dir = tempdir().unwrap();
        let hub = crate::state_hub::shared_state_hub();
        let mut app = App::new_connected(dir.path(), &hub);

        hub.publish(roko_core::DashboardEvent::PlanStarted {
            plan_id: "live-plan".to_string(),
            tasks_total: 0,
        });
        app.drain_snapshot_channel();
        hub.publish(roko_core::DashboardEvent::PlanCompleted {
            plan_id: "live-plan".to_string(),
            success: true,
        });
        app.drain_snapshot_channel();

        assert!(app.running);
    }

    #[test]
    fn connected_app_refresh_does_not_replay_disk_state() {
        let dir = tempdir().unwrap();
        let roko_dir = dir.path().join(".roko");
        std::fs::create_dir_all(&roko_dir).expect("roko dir");
        let stale_event = serde_json::to_string(&roko_core::DashboardEvent::PlanStarted {
            plan_id: "old-plan".to_string(),
            tasks_total: 0,
        })
        .expect("event json");
        std::fs::write(roko_dir.join("events.jsonl"), format!("{stale_event}\n"))
            .expect("events log");

        let hub = crate::state_hub::shared_state_hub();
        let mut app = App::new_connected(dir.path(), &hub);

        app.refresh_snapshot();
        app.drain_snapshot_channel();
        assert!(
            app.tui_state.plans.iter().all(|plan| plan.id != "old-plan"),
            "connected app imported stale disk plan"
        );

        hub.publish(roko_core::DashboardEvent::PlanStarted {
            plan_id: "live-plan".to_string(),
            tasks_total: 0,
        });
        app.drain_snapshot_channel();
        assert!(
            app.tui_state
                .plans
                .iter()
                .any(|plan| plan.id == "live-plan"),
            "connected app should still receive live StateHub updates"
        );
    }

    #[test]
    fn connected_topology_update_preserves_published_plan_state() {
        let dir = tempdir().unwrap();
        let hub = crate::state_hub::shared_state_hub();
        let app = App::new_connected(dir.path(), &hub);

        hub.publish(roko_core::DashboardEvent::PlanStarted {
            plan_id: "live-plan".to_string(),
            tasks_total: 0,
        });
        app.apply_state_hub_agent_topology(roko_core::AgentTopology::default());

        assert!(
            hub.current_snapshot().plans.contains_key("live-plan"),
            "a connected TUI snapshot mutation overwrote runner-published state"
        );
    }

    #[test]
    fn app_new_standalone_installs_snapshot_receiver() {
        let dir = tempdir().unwrap();
        let app = App::new(dir.path());
        assert!(app._state_hub.is_some());
        assert!(app.snapshot_rx.is_some());
    }

    #[test]
    fn approval_request_opens_modal_and_resolves_response() {
        use super::super::approval_ipc::ApprovalChannel;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());
        let channel = ApprovalChannel::new(1);
        let ApprovalChannel { tx, rx } = channel;
        let (response_tx, response_rx) = oneshot::channel();

        app.approval_rx = Some(rx);
        tx.try_send(ApprovalRequest {
            role: "reviewer".to_string(),
            command: "echo hello".to_string(),
            approval_id: "approval-42".to_string(),
            response_tx,
        })
        .unwrap();

        app.drain_approval_requests();

        assert!(matches!(
            app.tui_state.active_modal,
            Some(ModalState::Approval { ref role, ref command })
                if role == "reviewer" && command == "echo hello"
        ));
        assert_eq!(
            app.tui_state.pending_approval.as_ref().map(|pending| (
                pending.agent_id.as_str(),
                pending.description.as_str(),
                pending.command.as_str(),
            )),
            Some(("reviewer", "approval-42", "echo hello"))
        );
        assert_eq!(app.tui_state.input_mode, InputMode::Confirm);

        app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let approved = rt.block_on(async move { response_rx.await.unwrap() });

        assert!(approved);
        assert!(app.tui_state.active_modal.is_none());
        assert!(app.pending_approval_response.is_none());
        assert!(app.tui_state.pending_approval.is_none());
        assert_eq!(app.tui_state.input_mode, InputMode::Normal);
    }

    #[test]
    fn dashboard_snapshot_updates_preserve_navigation_state() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());
        app.tui_state.plans = vec![
            super::super::state::PlanEntry {
                id: "plan-a".to_string(),
                expanded: true,
                ..Default::default()
            },
            super::super::state::PlanEntry {
                id: "plan-b".to_string(),
                ..Default::default()
            },
        ];
        app.tui_state.agents = vec![
            super::super::state::AgentRow {
                id: "agent-a".to_string(),
                ..Default::default()
            },
            super::super::state::AgentRow {
                id: "agent-b".to_string(),
                ..Default::default()
            },
        ];
        app.tui_state.selected_plan_idx = 0;
        app.tui_state.current_plan_idx = 1;
        app.tui_state.selected_agent = 1;
        app.tui_state.active_tab = Tab::Agents;
        app.tui_state.plan_scroll_offset = 17;
        app.tui_state.agent_scroll = Some(9);

        let snapshot = roko_core::DashboardSnapshot {
            plans: [
                (
                    "plan-b".to_string(),
                    roko_core::dashboard_snapshot::PlanState {
                        plan_id: "plan-b".to_string(),
                        phase: "done".to_string(),
                        tasks_total: 2,
                        tasks_done: 2,
                        tasks_failed: 0,
                        active: false,
                    },
                ),
                (
                    "plan-c".to_string(),
                    roko_core::dashboard_snapshot::PlanState {
                        plan_id: "plan-c".to_string(),
                        phase: "active".to_string(),
                        tasks_total: 1,
                        tasks_done: 0,
                        tasks_failed: 0,
                        active: true,
                    },
                ),
            ]
            .into_iter()
            .collect(),
            agents: [
                (
                    "agent-b".to_string(),
                    roko_core::dashboard_snapshot::AgentState {
                        agent_id: "agent-b".to_string(),
                        role: "reviewer".to_string(),
                        active: true,
                        output_bytes: 0,
                        model: String::new(),
                        provider: String::new(),
                        input_tokens: 0,
                        output_tokens: 0,
                        cache_read_tokens: 0,
                        cache_write_tokens: 0,
                        cost_usd: 0.0,
                        current_task: String::new(),
                        current_plan: String::new(),
                        attempt: 0,
                        spawned_at_ms: 0,
                        last_event_at_ms: 0,
                        elapsed_ms: 0,
                    },
                ),
                (
                    "agent-c".to_string(),
                    roko_core::dashboard_snapshot::AgentState {
                        agent_id: "agent-c".to_string(),
                        role: "planner".to_string(),
                        active: false,
                        output_bytes: 0,
                        model: String::new(),
                        provider: String::new(),
                        input_tokens: 0,
                        output_tokens: 0,
                        cache_read_tokens: 0,
                        cache_write_tokens: 0,
                        cost_usd: 0.0,
                        current_task: String::new(),
                        current_plan: String::new(),
                        attempt: 0,
                        spawned_at_ms: 0,
                        last_event_at_ms: 0,
                        elapsed_ms: 0,
                    },
                ),
            ]
            .into_iter()
            .collect(),
            gates: vec![roko_core::dashboard_snapshot::GateVerdictView {
                plan_id: "plan-b".to_string(),
                task_id: "task-1".to_string(),
                gate: "compile".to_string(),
                passed: true,
                ts_millis: 42,
            }],
            errors: vec![roko_core::dashboard_snapshot::ErrorEntry {
                message: "boom".to_string(),
                ts_millis: 7,
            }],
            ..Default::default()
        };

        apply_dashboard_snapshot(
            &mut app.tui_state,
            &mut app.notifications,
            &mut app.last_snapshot_error_marker,
            &mut app.last_seen_gate_count,
            &mut app.last_seen_plan_phases,
            &snapshot,
        );

        assert_eq!(app.tui_state.active_tab, Tab::Agents);
        assert_eq!(app.tui_state.plan_scroll_offset, 17);
        assert_eq!(app.tui_state.agent_scroll, Some(9));
        assert_eq!(app.tui_state.plans[0].id, "plan-b");
        assert_eq!(
            app.tui_state.plans[0].status,
            super::super::state::PlanPhase::Done
        );
        assert_eq!(app.tui_state.plans[1].id, "plan-c");
        assert!(!app.tui_state.plans[0].expanded);
        assert_eq!(app.tui_state.selected_plan_idx, 0);
        assert_eq!(app.tui_state.current_plan_idx, 0);
        assert_eq!(app.tui_state.agents[0].id, "agent-b");
        assert!(app.tui_state.agents[0].active);
        assert_eq!(app.tui_state.selected_agent, 0);
        assert_eq!(app.tui_state.gate_results.len(), 1);
        assert_eq!(app.tui_state.gate_results[0].output, "task task-1");
        assert!(
            app.notifications
                .iter()
                .any(|notification| notification.message == "boom")
        );
    }

    #[test]
    fn gate_verdict_generates_toast() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());
        let snapshot = roko_core::DashboardSnapshot {
            gates: vec![roko_core::dashboard_snapshot::GateVerdictView {
                plan_id: "plan-a".into(),
                task_id: "task-1".into(),
                gate: "compile".into(),
                passed: true,
                ts_millis: 100,
            }],
            ..Default::default()
        };
        apply_dashboard_snapshot(
            &mut app.tui_state,
            &mut app.notifications,
            &mut app.last_snapshot_error_marker,
            &mut app.last_seen_gate_count,
            &mut app.last_seen_plan_phases,
            &snapshot,
        );
        assert!(
            app.notifications
                .iter()
                .any(|n| n.message.contains("compile PASS"))
        );
        assert_eq!(app.last_seen_gate_count, 1);
    }

    #[test]
    fn materialized_headless_snapshot_does_not_replay_historical_toasts() {
        let dir = tempdir().unwrap();
        let snapshot = roko_core::DashboardSnapshot {
            gates: vec![roko_core::dashboard_snapshot::GateVerdictView {
                plan_id: "plan-a".into(),
                task_id: "task-1".into(),
                gate: "compile".into(),
                passed: false,
                ts_millis: 100,
            }],
            errors: vec![roko_core::dashboard_snapshot::ErrorEntry {
                message: "historical failure".into(),
                ts_millis: 101,
            }],
            ..Default::default()
        };

        let app = App::new_with_dashboard_snapshot(dir.path(), &snapshot);
        assert!(app.notifications.is_empty());
        assert_eq!(app.tui_state.gate_results.len(), 1);
    }

    #[test]
    fn gate_verdict_fail_generates_error_toast() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());
        let snapshot = roko_core::DashboardSnapshot {
            gates: vec![roko_core::dashboard_snapshot::GateVerdictView {
                plan_id: "plan-a".into(),
                task_id: "task-1".into(),
                gate: "test".into(),
                passed: false,
                ts_millis: 100,
            }],
            ..Default::default()
        };
        apply_dashboard_snapshot(
            &mut app.tui_state,
            &mut app.notifications,
            &mut app.last_snapshot_error_marker,
            &mut app.last_seen_gate_count,
            &mut app.last_seen_plan_phases,
            &snapshot,
        );
        let fail_toast = app
            .notifications
            .iter()
            .find(|n| n.message.contains("test FAIL"));
        assert!(fail_toast.is_some());
        assert_eq!(
            fail_toast.unwrap().level,
            super::modals::NotificationLevel::Error
        );
    }

    #[test]
    fn plan_completion_generates_toast() {
        use roko_core::dashboard_snapshot::PlanState;

        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());
        let mut plans = std::collections::HashMap::new();
        plans.insert(
            "plan-x".to_string(),
            PlanState {
                plan_id: "plan-x".into(),
                phase: "completed".into(),
                active: false,
                ..Default::default()
            },
        );
        let snapshot = roko_core::DashboardSnapshot {
            plans,
            ..Default::default()
        };
        apply_dashboard_snapshot(
            &mut app.tui_state,
            &mut app.notifications,
            &mut app.last_snapshot_error_marker,
            &mut app.last_seen_gate_count,
            &mut app.last_seen_plan_phases,
            &snapshot,
        );
        assert!(
            app.notifications
                .iter()
                .any(|n| n.message.contains("Plan plan-x completed"))
        );
    }

    #[test]
    fn dedup_suppresses_duplicate_within_2s() {
        let mut notifications = std::collections::VecDeque::new();
        push_deduped_notification(
            &mut notifications,
            super::modals::Notification::info("same message"),
        );
        push_deduped_notification(
            &mut notifications,
            super::modals::Notification::info("same message"),
        );
        assert_eq!(notifications.len(), 1);
    }

    #[test]
    fn notification_cap_at_20() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());
        for i in 0..25 {
            app.notifications
                .push_back(super::modals::Notification::info(format!("msg {i}")));
        }
        app.expire_notifications();
        assert!(app.notifications.len() <= 20);
    }

    #[test]
    fn full_frame_render_no_panic() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());
        let backend = TestBackend::new(160, 50);
        let mut terminal = Terminal::new(backend).unwrap();
        // The real test: does a full frame render without panicking?
        terminal.draw(|frame| app.draw(frame)).unwrap();
    }

    #[test]
    fn full_effects_dashboard_preserves_operational_text() {
        use super::super::effects_config::EffectsPreset;

        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".roko")).unwrap();
        let mut app = App::new(dir.path());
        app.fx_config = EffectsConfig::from_preset(EffectsPreset::Full);
        app.tui_state.agents.push(super::super::state::AgentRow {
            id: "doctor-network/T1".to_string(),
            active: true,
            status: super::super::state::AgentStatus::Active,
            role: "implementer".to_string(),
            model: "gpt-5.6-sol".to_string(),
            current_plan: "doctor-network-v2".to_string(),
            current_task: "T1".to_string(),
            output_lines: vec!["agent output remains readable".to_string()],
            ..Default::default()
        });

        let backend = TestBackend::new(180, 55);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| app.draw(frame)).unwrap();

        let rendered = rendered_text(&terminal);
        assert!(rendered.contains("Agents (1 active)"));
        assert!(rendered.contains("agent output remains readable"));
        let braille_cells = rendered
            .chars()
            .filter(|ch| ('\u{2800}'..='\u{28ff}').contains(ch))
            .count();
        assert!(
            braille_cells <= 32,
            "refined full effects rendered {braille_cells} braille cells"
        );
    }

    #[test]
    fn all_tabs_render_without_panic() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());

        for tab in Tab::ALL {
            app.tui_state.active_tab = tab;
            let backend = TestBackend::new(160, 50);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|frame| app.draw(frame))
                .unwrap_or_else(|e| panic!("Tab {:?} failed to render: {e}", tab));
        }
    }

    #[test]
    fn dashboard_subtab_keybindings_include_learning_and_procs() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".roko")).unwrap();
        let mut app = App::new(dir.path());
        assert_eq!(app.tui_state.plan_detail_tab, 0); // starts on Agents

        app.handle_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
        assert_eq!(app.tui_state.plan_detail_tab, 1); // switched to Output

        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert_eq!(app.tui_state.plan_detail_tab, 2); // switched to Diff

        app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
        assert_eq!(app.tui_state.plan_detail_tab, 3); // switched to Errors

        app.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
        assert_eq!(app.tui_state.plan_detail_tab, 4); // switched to Git

        app.handle_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
        assert_eq!(app.tui_state.plan_detail_tab, 5); // switched to MCP

        app.handle_key(KeyEvent::new(KeyCode::Char('L'), KeyModifiers::SHIFT));
        assert_eq!(app.tui_state.plan_detail_tab, 6); // switched to Learning

        app.handle_key(KeyEvent::new(KeyCode::Char('P'), KeyModifiers::SHIFT));
        assert_eq!(app.tui_state.plan_detail_tab, 7); // switched to Procs

        app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        assert_eq!(app.tui_state.plan_detail_tab, 0); // back to Agents
    }

    #[test]
    fn keybinding_f_keys_switch_tabs() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());
        assert_eq!(app.tui_state.active_tab, Tab::Dashboard);

        app.handle_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
        assert_eq!(app.tui_state.active_tab, Tab::Plans);

        app.handle_key(KeyEvent::new(KeyCode::F(3), KeyModifiers::NONE));
        assert_eq!(app.tui_state.active_tab, Tab::Agents);

        app.handle_key(KeyEvent::new(KeyCode::F(4), KeyModifiers::NONE));
        assert_eq!(app.tui_state.active_tab, Tab::Git);

        app.handle_key(KeyEvent::new(KeyCode::F(5), KeyModifiers::NONE));
        assert_eq!(app.tui_state.active_tab, Tab::Logs);

        app.handle_key(KeyEvent::new(KeyCode::F(6), KeyModifiers::NONE));
        assert_eq!(app.tui_state.active_tab, Tab::Config);

        app.handle_key(KeyEvent::new(KeyCode::F(7), KeyModifiers::NONE));
        assert_eq!(app.tui_state.active_tab, Tab::Inspect);

        app.handle_key(KeyEvent::new(KeyCode::F(8), KeyModifiers::NONE));
        assert_eq!(app.tui_state.active_tab, Tab::Marketplace);

        app.handle_key(KeyEvent::new(KeyCode::F(9), KeyModifiers::NONE));
        assert_eq!(app.tui_state.active_tab, Tab::Atelier);

        app.handle_key(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE));
        assert_eq!(app.tui_state.active_tab, Tab::Dashboard);
    }

    #[test]
    fn keybinding_help_toggle() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".roko")).unwrap();
        let mut app = App::new(dir.path());
        assert!(!matches!(
            app.tui_state.active_modal,
            Some(ModalState::Help)
        ));

        app.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
        assert!(matches!(app.tui_state.active_modal, Some(ModalState::Help)));

        app.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
        assert!(!matches!(
            app.tui_state.active_modal,
            Some(ModalState::Help)
        ));
    }

    #[test]
    fn show_plan_detail_opens_selected_plan_and_resets_scroll() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());
        app.tui_state.plans = vec![super::super::state::PlanEntry {
            id: "plan-1".to_string(),
            name: "Plan One".to_string(),
            ..Default::default()
        }];
        app.tui_state.selected_plan_idx = 0;
        app.tui_state.plan_detail_scroll = 7;

        app.dispatch_action(TuiAction::ShowPlanDetail);

        assert!(matches!(
            app.tui_state.active_modal,
            Some(ModalState::PlanDetail { ref plan_id }) if plan_id == "plan-1"
        ));
        assert_eq!(app.tui_state.plan_detail_scroll, 0);

        app.dispatch_action(TuiAction::ShowPlanDetail);
        assert!(app.tui_state.active_modal.is_none());
    }

    #[test]
    fn modal_scroll_actions_update_modal_snapshot_only() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());
        app.tui_state.plan_scroll_offset = 9;
        app.tui_state.active_modal = Some(ModalState::WaveOverview {
            waves: Vec::new(),
            scroll_offset: 2,
        });

        app.dispatch_action(TuiAction::ModalScrollDown);

        assert!(matches!(
            app.tui_state.active_modal,
            Some(ModalState::WaveOverview {
                scroll_offset: 3,
                ..
            })
        ));
        assert_eq!(app.tui_state.plan_scroll_offset, 9);

        app.tui_state.active_modal = Some(ModalState::AgentPool {
            agents: Vec::new(),
            scroll_offset: 4,
        });

        app.dispatch_action(TuiAction::ModalScrollUp);

        assert!(matches!(
            app.tui_state.active_modal,
            Some(ModalState::AgentPool {
                scroll_offset: 3,
                ..
            })
        ));
    }

    #[test]
    fn queue_overview_actions_update_modal_state() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());
        app.tui_state.plan_scroll_offset = 5;
        app.tui_state.active_modal = Some(ModalState::QueueOverview {
            milestones: vec![
                Milestone {
                    name: "Wave 0".to_string(),
                    tasks: Vec::new(),
                    completed: 0,
                    total: 1,
                },
                Milestone {
                    name: "Wave 1".to_string(),
                    tasks: Vec::new(),
                    completed: 0,
                    total: 1,
                },
            ],
            selected_index: 0,
            scroll_offset: 0,
        });

        app.dispatch_action(TuiAction::QueueOverviewDown);

        assert!(matches!(
            app.tui_state.active_modal,
            Some(ModalState::QueueOverview {
                selected_index: 1,
                scroll_offset: 1,
                ..
            })
        ));
        assert_eq!(app.tui_state.plan_scroll_offset, 5);
    }

    #[test]
    fn quit_opens_confirmation_modal_instead_of_exiting() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".roko")).unwrap();
        let mut app = App::new(dir.path());
        assert!(app.running);
        assert!(app.tui_state.active_modal.is_none());

        app.dispatch_action(TuiAction::Quit);

        assert!(app.running);
        assert!(matches!(app.tui_state.active_modal, Some(ModalState::Quit)));
        assert_eq!(app.tui_state.input_mode, InputMode::Confirm);
    }

    #[test]
    fn confirming_quit_exits() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());
        app.tui_state.active_modal = Some(ModalState::Quit);
        app.tui_state.input_mode = InputMode::Confirm;

        app.dispatch_action(TuiAction::ConfirmYes);

        assert!(!app.running);
        assert!(app.tui_state.active_modal.is_none());
        assert_eq!(app.tui_state.input_mode, InputMode::Normal);
        assert!(app.tui_state.pending_confirm.is_none());
    }

    #[test]
    fn config_save_reloads_config_immediately() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("roko.toml"),
            RokoConfig::default().to_toml().unwrap(),
        )
        .unwrap();

        let mut app = App::new(dir.path());
        app.tui_state.config_pending.insert(
            "agent.default_model".to_string(),
            "claude-opus-4-6".to_string(),
        );

        app.dispatch_action(TuiAction::ConfigSave);

        let reloaded = roko_core::config::loader::load_config_unified(dir.path()).unwrap();

        assert!(app.tui_state.config_pending.is_empty());
        assert_eq!(reloaded.agent.default_model, "claude-opus-4-6");
        assert!(
            app.notifications
                .iter()
                .any(|notification| notification.message == "Config saved and reloaded")
        );
    }

    #[test]
    fn config_save_reloads_screen_postfx_immediately() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("roko.toml"),
            RokoConfig::default().to_toml().unwrap(),
        )
        .unwrap();

        let mut app = App::new(dir.path());
        app.tui_state
            .config_pending
            .insert("tui.effects.screen_postfx".to_string(), "true".to_string());

        app.dispatch_action(TuiAction::ConfigSave);

        assert!(app.fx_config.screen_postfx);
        let saved = std::fs::read_to_string(dir.path().join("roko.toml")).unwrap();
        assert!(saved.contains("[tui.effects]"));
        assert!(saved.contains("screen_postfx = true"));
    }

    #[test]
    fn f6_switch_to_config_tab_populates_cache_and_renders_fields() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("roko.toml"),
            RokoConfig::default().to_toml().unwrap(),
        )
        .unwrap();

        let mut app = App::new(dir.path());
        // Deterministic text assertions: no post-processing over the content.
        app.fx_config.screen_postfx = false;
        // Constructor warms the cache (covers the headless --snapshot path).
        assert!(
            !app.tui_state.config_items_cache.is_empty(),
            "App::new must warm the config items cache"
        );

        // Simulate the cold-cache regression: fresh session state where the
        // cache was never populated.
        app.tui_state.config_items_cache.clear();
        app.tui_state.config_items_refreshed_at = None;

        app.dispatch_action(TuiAction::SwitchTab(Tab::Config));

        assert!(
            !app.tui_state.config_items_cache.is_empty(),
            "F6 must warm the config items cache"
        );

        let rendered = app.render_tabs_to_text(100, 40, &[Tab::Config]);
        let text = &rendered[0].1;
        // Real config editor content: a group header and a field label, not
        // only the always-rendered `Runtime:` sections.
        assert!(text.contains("Agent"), "expected group header in:\n{text}");
        assert!(
            text.contains("Default Model"),
            "expected config field label in:\n{text}"
        );
    }

    #[test]
    fn config_r_key_reloads_items_cache() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("roko.toml"),
            RokoConfig::default().to_toml().unwrap(),
        )
        .unwrap();

        let mut app = App::new(dir.path());
        app.dispatch_action(TuiAction::SwitchTab(Tab::Config));

        // Externally modify roko.toml while the cache is still within its TTL.
        let mut cfg = RokoConfig::default();
        cfg.agent.default_model = "claude-haiku-4-5".to_string();
        std::fs::write(dir.path().join("roko.toml"), cfg.to_toml().unwrap()).unwrap();

        app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));

        let value = app
            .tui_state
            .config_items_cache
            .iter()
            .find_map(|item| match item {
                crate::tui::config_meta::ConfigItem::Field { meta, value, .. }
                    if meta.key == "agent.default_model" =>
                {
                    Some(value.clone())
                }
                _ => None,
            })
            .expect("agent.default_model field present");
        assert!(
            value.contains("claude-haiku-4-5"),
            "r:reload must re-parse roko.toml immediately, got: {value}"
        );
    }

    #[test]
    fn ctrl_e_toggles_screen_postfx() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".roko")).unwrap();
        let mut app = App::new(dir.path());
        assert!(app.fx_config.screen_postfx);

        app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL));
        assert!(!app.fx_config.screen_postfx);

        app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL));
        assert!(app.fx_config.screen_postfx);
    }

    #[test]
    fn effects_action_cycles_presets_and_keeps_master_switch_honest() {
        use super::super::effects_config::EffectsPreset;

        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("roko.toml"),
            RokoConfig::default().to_toml().unwrap(),
        )
        .unwrap();

        let mut app = App::new(dir.path());
        app.fx_config.set_preset(EffectsPreset::Off);
        app.fx_config.screen_postfx = true;

        app.dispatch_action(TuiAction::CycleEffectsPreset);
        assert_eq!(app.fx_config.preset, EffectsPreset::Minimal);
        assert!(app.fx_config.screen_postfx);
        assert!(!app.fx_config.nerv_viz);
        assert!(!app.fx_config.particles);

        app.dispatch_action(TuiAction::CycleEffectsPreset);
        assert_eq!(app.fx_config.preset, EffectsPreset::Full);
        assert!(app.fx_config.screen_postfx);
        assert!(app.fx_config.nerv_viz);
        assert!(app.fx_config.particles);

        app.dispatch_action(TuiAction::CycleEffectsPreset);
        assert_eq!(app.fx_config.preset, EffectsPreset::Off);
        assert!(!app.fx_config.screen_postfx);
        assert!(!app.fx_config.nerv_viz);
        assert!(!app.fx_config.particles);

        app.dispatch_action(TuiAction::CycleEffectsPreset);
        assert_eq!(app.fx_config.preset, EffectsPreset::Minimal);
        assert!(app.fx_config.screen_postfx);
        assert!(!app.fx_config.nerv_viz);
        assert!(!app.fx_config.particles);

        let saved = std::fs::read_to_string(dir.path().join("roko.toml")).unwrap();
        assert!(saved.contains("preset = \"minimal\""));
    }

    #[test]
    fn drill_actions_on_git_use_git_cursor_not_plan_expansion() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());
        app.tui_state.active_tab = Tab::Git;
        app.tui_state.plans = vec![super::super::state::PlanEntry::default()];
        app.tui_state.git_view_data = Some(super::views::git_view::GitViewData {
            branches: vec![
                super::views::git_view::GitBranchNode {
                    name: "main".to_string(),
                    is_current: true,
                    tracking: None,
                    ahead: 0,
                    behind: 0,
                    depth: 0,
                    children: Vec::new(),
                },
                super::views::git_view::GitBranchNode {
                    name: "feature/test".to_string(),
                    is_current: false,
                    tracking: None,
                    ahead: 0,
                    behind: 0,
                    depth: 1,
                    children: Vec::new(),
                },
            ],
            ..Default::default()
        });

        app.dispatch_action(TuiAction::DrillIn);
        assert_eq!(app.tui_state.git_branch_cursor, 1);
        assert!(!app.tui_state.plans[0].expanded);
        assert_eq!(app.current_view_state().selected, 1);

        app.dispatch_action(TuiAction::DrillOut);
        assert_eq!(app.tui_state.git_branch_cursor, 0);
        assert!(!app.tui_state.plans[0].expanded);
    }

    #[test]
    fn request_confirm_resolves_plan_and_git_context() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());
        app.tui_state.plans = vec![super::super::state::PlanEntry {
            id: "plan-7".to_string(),
            phase: "done".to_string(),
            status: super::super::state::PlanPhase::Done,
            active: false,
            ..Default::default()
        }];
        app.tui_state.git_branch = "feature/plan-7".to_string();

        app.dispatch_action(TuiAction::RequestConfirm(ConfirmAction::DiagnosePlan(
            String::new(),
        )));
        assert_eq!(app.tui_state.input_mode, InputMode::Confirm);
        assert_eq!(
            app.tui_state.pending_confirm,
            Some(ConfirmAction::DiagnosePlan("plan-7".to_string()))
        );
        assert!(matches!(
            app.tui_state.active_modal,
            Some(ModalState::Confirm {
                action: modals::ConfirmAction::Custom { .. }
            })
        ));

        app.dispatch_action(TuiAction::RequestConfirm(ConfirmAction::MergePlan {
            plan_id: String::new(),
            branch: String::new(),
        }));
        assert_eq!(
            app.tui_state.pending_confirm,
            Some(ConfirmAction::MergePlan {
                plan_id: "plan-7".to_string(),
                branch: "feature/plan-7".to_string(),
            })
        );

        app.dispatch_action(TuiAction::RequestConfirm(ConfirmAction::MergeAllDone {
            branches: Vec::new(),
        }));
        assert_eq!(
            app.tui_state.pending_confirm,
            Some(ConfirmAction::MergeAllDone {
                branches: vec!["plan-7".to_string()],
            })
        );
    }

    #[test]
    fn page_scroll_moves_focused_panel_by_terminal_height_minus_chrome() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());
        app.terminal_size = (120, 50);
        app.tui_state.focus = FocusZone::PlanTree;
        app.tui_state.plan_scroll_offset = 40;

        app.dispatch_action(TuiAction::ScrollPageUp);
        assert_eq!(app.tui_state.plan_scroll_offset, 0);

        app.dispatch_action(TuiAction::ScrollPageDown);
        assert_eq!(app.tui_state.plan_scroll_offset, 46);
    }

    #[test]
    fn focused_home_end_jump_to_bounds() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());
        app.tui_state.focus = FocusZone::RightPanel;
        app.tui_state.diff_scroll = 12;

        app.dispatch_action(TuiAction::ScrollFocusedHome);
        assert_eq!(app.tui_state.diff_scroll, 0);

        app.dispatch_action(TuiAction::ScrollFocusedEnd);
        assert_eq!(app.tui_state.diff_scroll, usize::MAX);
    }

    #[test]
    fn current_view_state_is_tab_specific() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());
        app.tui_state.selected_plan_idx = 2;
        app.tui_state.selected_agent = 3;
        app.tui_state.selected_agent_tab = 4;
        app.tui_state.plan_scroll_offset = 8;
        app.tui_state.agent_scroll = Some(5);
        app.tui_state.log_scroll = 7;
        app.tui_state.log_auto_tail = false;

        app.tui_state.active_tab = Tab::Dashboard;
        let view = app.current_view_state();
        assert_eq!(view.scroll, 5);
        assert_eq!(view.selected, 2);
        assert_eq!(view.sub_tab, 0);
        assert!(!view.auto_tail);

        app.tui_state.active_tab = Tab::Plans;
        let view = app.current_view_state();
        assert_eq!(view.scroll, 8);
        assert_eq!(view.selected, 2);
        assert_eq!(view.sub_tab, 0);
        assert!(!view.auto_tail);

        app.tui_state.active_tab = Tab::Agents;
        let view = app.current_view_state();
        assert_eq!(view.scroll, 5);
        assert_eq!(view.selected, 3);
        assert_eq!(view.sub_tab, 4);
        assert!(!view.auto_tail);

        app.tui_state.active_tab = Tab::Logs;
        let view = app.current_view_state();
        assert_eq!(view.scroll, 7);
        assert_eq!(view.selected, 0);
        assert!(!view.auto_tail);

        app.tui_state.active_tab = Tab::Git;
        app.tui_state.git_detail_scroll = 11;
        let view = app.current_view_state();
        assert_eq!(view.scroll, 11);
        assert_eq!(view.selected, app.tui_state.git_branch_cursor);
        assert!(!view.auto_tail);

        app.tui_state.active_tab = Tab::Config;
        app.tui_state.config_sub_tab = 2;
        let view = app.current_view_state();
        assert_eq!(view.sub_tab, 2);

        app.tui_state.active_tab = Tab::Inspect;
        app.tui_state.inspect_sub_tab = 3;
        let view = app.current_view_state();
        assert_eq!(view.sub_tab, 3);
    }

    #[test]
    fn switch_subview_updates_active_tab_slot_only() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());

        app.tui_state.active_tab = Tab::Config;
        app.dispatch_action(TuiAction::SwitchSubView(2));
        assert_eq!(app.tui_state.config_sub_tab, 2);
        assert_eq!(app.tui_state.plan_detail_tab, 0);

        app.tui_state.active_tab = Tab::Inspect;
        app.dispatch_action(TuiAction::SwitchSubView(3));
        assert_eq!(app.tui_state.inspect_sub_tab, 3);
        assert_eq!(app.tui_state.config_sub_tab, 2);

        app.tui_state.active_tab = Tab::Marketplace;
        app.dispatch_action(TuiAction::SwitchSubView(1));
        assert_eq!(app.tui_state.marketplace_sub_tab, 1);
        assert_eq!(app.tui_state.inspect_sub_tab, 3);

        app.tui_state.active_tab = Tab::Logs;
        app.dispatch_action(TuiAction::SwitchSubView(2));
        assert_eq!(app.tui_state.logs_sub_tab, 2);

        app.tui_state.active_tab = Tab::Learning;
        app.dispatch_action(TuiAction::SwitchSubView(1));
        assert_eq!(app.tui_state.learning_sub_tab, 1);

        app.tui_state.active_tab = Tab::Dashboard;
        app.dispatch_action(TuiAction::SwitchSubView(7));
        assert_eq!(app.tui_state.dashboard_sub_tab, 7);
        assert_eq!(app.tui_state.plan_detail_tab, 0);
    }

    #[test]
    fn agents_tab_selection_moves_agent_roster() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());
        app.tui_state.active_tab = Tab::Agents;
        app.tui_state.selected_agent = 1;
        app.tui_state.focus = FocusZone::PlanTree;
        app.tui_state.agents = vec![
            super::super::state::AgentRow::default(),
            super::super::state::AgentRow::default(),
            super::super::state::AgentRow::default(),
        ];

        app.dispatch_action(TuiAction::ScrollFocusedUp);
        assert_eq!(app.tui_state.selected_agent, 0);

        app.dispatch_action(TuiAction::ScrollFocusedDown);
        assert_eq!(app.tui_state.selected_agent, 1);

        app.dispatch_action(TuiAction::ScrollFocusedEnd);
        assert_eq!(app.tui_state.selected_agent, 2);
    }

    #[test]
    fn log_end_action_resumes_tail_mode() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());
        app.tui_state.active_tab = Tab::Logs;
        app.tui_state.log_auto_tail = false;
        app.tui_state.log_scroll = 9;

        app.dispatch_action(TuiAction::ScrollLogEnd);
        assert!(app.tui_state.log_auto_tail);
        assert_eq!(app.tui_state.log_scroll, 0);
    }

    #[test]
    fn filter_input_stays_in_sync_and_escape_clears_state() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());

        app.dispatch_action(TuiAction::StartFilter);
        assert_eq!(app.tui_state.input_mode, InputMode::Filter);
        assert!(app.tui_state.filter_text.is_empty());
        assert!(app.tui_state.filter.is_empty());
        assert!(!app.tui_state.filter_active);

        app.dispatch_action(TuiAction::InputChar('a'));
        app.dispatch_action(TuiAction::InputChar('b'));
        assert_eq!(app.tui_state.filter_text, "ab");
        assert_eq!(app.tui_state.filter, "ab");
        assert!(app.tui_state.filter_active);

        app.dispatch_action(TuiAction::InputBackspace);
        assert_eq!(app.tui_state.filter_text, "a");
        assert_eq!(app.tui_state.filter, "a");
        assert!(app.tui_state.filter_active);

        app.dispatch_action(TuiAction::AcceptFilter);
        assert_eq!(app.tui_state.input_mode, InputMode::Normal);
        assert_eq!(app.tui_state.filter, "a");
        assert!(app.tui_state.filter_active);

        app.dispatch_action(TuiAction::StartFilter);
        app.dispatch_action(TuiAction::InputChar('z'));
        app.dispatch_action(TuiAction::CancelFilter);
        assert_eq!(app.tui_state.input_mode, InputMode::Normal);
        assert!(app.tui_state.filter_text.is_empty());
        assert!(app.tui_state.filter.is_empty());
        assert!(!app.tui_state.filter_active);
    }

    #[test]
    fn plan_filter_navigation_keeps_actual_plan_identity() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());
        app.tui_state.active_tab = Tab::Plans;
        app.tui_state.plans = vec![
            PlanEntry {
                id: "hidden-a".to_string(),
                name: "Hidden A".to_string(),
                ..PlanEntry::default()
            },
            PlanEntry {
                id: "visible-one".to_string(),
                name: "Visible One".to_string(),
                ..PlanEntry::default()
            },
            PlanEntry {
                id: "hidden-b".to_string(),
                name: "Hidden B".to_string(),
                ..PlanEntry::default()
            },
            PlanEntry {
                id: "visible-two".to_string(),
                name: "Visible Two".to_string(),
                ..PlanEntry::default()
            },
        ];

        app.dispatch_action(TuiAction::StartPlanFilter);
        for c in "visible".chars() {
            app.dispatch_action(TuiAction::InputChar(c));
        }
        assert_eq!(app.tui_state.selected_plan_idx, 1);

        app.dispatch_action(TuiAction::SelectPlanDown);
        assert_eq!(app.tui_state.selected_plan_idx, 3);
        app.dispatch_action(TuiAction::ShowPlanDetail);
        assert!(matches!(
            app.tui_state.active_modal,
            Some(ModalState::PlanDetail { ref plan_id }) if plan_id == "visible-two"
        ));
    }

    #[test]
    fn text_input_bar_renders_only_for_text_modes() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|frame| app.draw(frame)).unwrap();
        let normal = rendered_text(&terminal);
        assert!(!normal.contains("[INJECT] > "));
        assert!(!normal.contains("[FILTER] > "));

        app.tui_state.input_mode = InputMode::Inject;
        app.tui_state.message_input = "ship it".to_string();
        terminal.draw(|frame| app.draw(frame)).unwrap();
        let inject = rendered_text(&terminal);
        assert!(inject.contains("[INJECT] > ship it│"));

        app.tui_state.input_mode = InputMode::Filter;
        app.tui_state.filter_text = "plan".to_string();
        terminal.draw(|frame| app.draw(frame)).unwrap();
        let filter = rendered_text(&terminal);
        assert!(filter.contains("[FILTER] > plan│"));
    }

    // -----------------------------------------------------------------
    // Adaptive tick policy + dirty-flag render loop tests
    // -----------------------------------------------------------------

    #[test]
    fn tui_event_loop_render_dirty_starts_clean() {
        let dir = tempdir().unwrap();
        let app = App::new(dir.path());
        assert!(
            app.render_dirty.is_empty(),
            "fresh App should start with no dirty bits"
        );
    }

    #[test]
    fn tui_event_loop_frame_stats_start_zeroed() {
        let dir = tempdir().unwrap();
        let app = App::new(dir.path());
        assert_eq!(app.frame_stats.frames_drawn, 0);
        assert_eq!(app.frame_stats.skipped_identical, 0);
        assert!(app.frame_stats.last_input_at.is_none());
    }

    #[test]
    fn tui_event_loop_tick_policy_dormant_on_idle_app() {
        let dir = tempdir().unwrap();
        let app = App::new(dir.path());
        let inputs = app.tick_policy_inputs();
        let policy = next_tick_policy(&inputs);
        assert_eq!(
            policy,
            super::super::event::TickPolicy::Dormant,
            "idle app with no active work should select Dormant policy"
        );
    }

    #[test]
    fn tui_event_loop_tick_policy_active_after_input() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());
        // Simulate recent input
        app.frame_stats.record_input();
        let inputs = app.tick_policy_inputs();
        let policy = next_tick_policy(&inputs);
        assert_eq!(
            policy,
            super::super::event::TickPolicy::Active,
            "app with recent input should select Active policy"
        );
    }

    #[test]
    fn tui_event_loop_tick_policy_idle_with_active_plan() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());
        // Add an active plan
        app.tui_state.plans.push(PlanEntry {
            id: "test-plan".to_string(),
            active: true,
            ..Default::default()
        });
        let inputs = app.tick_policy_inputs();
        let policy = next_tick_policy(&inputs);
        assert_eq!(
            policy,
            super::super::event::TickPolicy::Idle,
            "app with active plan should select Idle policy"
        );
    }

    #[test]
    fn tui_event_loop_snapshot_drain_sets_dirty() {
        let dir = tempdir().unwrap();
        let hub = crate::state_hub::shared_state_hub();
        let mut app = App::new_connected(dir.path(), &hub);

        // Initially clean
        app.render_dirty = RenderDirty::NONE;

        // Publish an event to trigger snapshot change
        hub.publish(roko_core::DashboardEvent::PlanStarted {
            plan_id: "dirty-test".to_string(),
            tasks_total: 1,
        });
        app.drain_snapshot_channel();

        assert!(
            app.render_dirty.contains(RenderDirty::SNAPSHOT),
            "drain_snapshot_channel should set SNAPSHOT dirty bit"
        );
    }

    #[test]
    fn tui_event_loop_clean_state_skips_draw() {
        // Verify that when render_dirty is empty, we would skip a draw.
        // (This tests the flag logic, not the actual event loop.)
        let dir = tempdir().unwrap();
        let app = App::new(dir.path());
        assert!(
            app.render_dirty.is_empty(),
            "clean state should not trigger a draw"
        );
    }

    #[test]
    fn tui_event_loop_drawn_reasons_cleared() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());

        // Set multiple dirty bits
        app.render_dirty
            .insert(RenderDirty::INPUT | RenderDirty::SNAPSHOT);

        // Simulate drawing: capture reasons, then clear
        let drawn = app.render_dirty;
        app.render_dirty.remove(drawn);

        assert!(
            app.render_dirty.is_empty(),
            "all drawn reasons should be cleared after draw"
        );
    }

    #[test]
    fn tui_event_loop_late_arrival_survives_clear() {
        let dir = tempdir().unwrap();
        let mut app = App::new(dir.path());

        // Initial dirty reason
        app.render_dirty.insert(RenderDirty::INPUT);
        let drawn = app.render_dirty;

        // Late arrival before clear
        app.render_dirty.insert(RenderDirty::METRICS);

        // Clear only drawn reasons
        app.render_dirty.remove(drawn);

        assert!(
            !app.render_dirty.contains(RenderDirty::INPUT),
            "INPUT should be cleared"
        );
        assert!(
            app.render_dirty.contains(RenderDirty::METRICS),
            "METRICS arrived late and should survive"
        );
    }

    #[test]
    fn tui_event_loop_current_tick_duration_matches_policy() {
        let dir = tempdir().unwrap();
        let app = App::new(dir.path());
        // Idle app with no active work: should be 250ms (Dormant)
        let dur = app.current_tick_duration();
        assert_eq!(
            dur,
            std::time::Duration::from_millis(250),
            "dormant app should use 250ms tick"
        );
    }
}
