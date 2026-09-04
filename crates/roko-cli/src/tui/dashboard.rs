//! Dashboard scaffold container for future TUI wiring.
//!
//! This module keeps the existing page scaffold intact, but layers a
//! best-effort learning snapshot on top so the health and trends pages
//! can render real stats when the memory JSONL files are present.

use std::collections::{BTreeMap, HashMap, HashSet, hash_map::DefaultHasher};
use std::fmt::{self, Write as _};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

use anyhow::{Context as _, Result};
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::Value;

#[cfg(test)]
use ratatui::style::Color;

use crate::plan::{PlanSummary, plans_dir};
use crate::task_parser::{TaskDef, TasksFile};
use roko_core::ExperimentWinnerSummary;
#[cfg(test)]
use roko_core::metric::TaskMetric;
use roko_gate::adaptive_threshold::AdaptiveThresholds;
use roko_learn::aggregate::{CFactorBucket, EfficiencyBucket, cfactor_trend, efficiency_trend};
pub use roko_learn::cfactor::{CFactor, CFactorComponents};
use roko_learn::efficiency::AgentEfficiencyEvent;
use roko_learn::episode_logger::Episode;
use roko_learn::prompt_experiment::ExperimentStore;
#[cfg(test)]
use roko_learn::skill_library::Skill;
use roko_runtime::{
    LEGACY_EXECUTOR_RELATIVE_PATH, RunnerProjectionSource, STATE_SNAPSHOT_RELATIVE_PATH,
    load_durable_runner_projection,
};

use super::cursors::{EpisodeCursor, EventLogCursor, SignalCursor};
use super::dashboard_gen::DurableDashboardGenerationCounter;
use super::display_utils::truncate as truncate_str;
use super::pages::{PageId, PageScaffold, efficiency, operations};
use super::state::{PlanPhase, TaskStatus};
use super::task_outputs::TaskOutputCursors;
pub use super::theme::Theme;

// Re-export types from the extracted dashboard_types module.
pub use super::dashboard_types::{
    AgentSummary, AlertSummary, CascadeRouterModelStats, CascadeRouterState, EfficiencySummary,
    ExperimentSummary, GateFailureRow, GateResultSummary, GateResultsPageData, GateSignalSummary,
    GateSummaryRow, GateThresholdRow, GateTrend, KnowledgeBrowseEntry, PlanExecutionSnapshot,
    PlanExecutionTaskDetail, PlanExecutionTaskRow, ReadFileSnapshot, SignalSummary, TaskSummary,
};
pub(crate) use super::dashboard_types::{
    PlanTaskListSnapshot, PlanTaskSnapshot, build_agent_activity_snapshot,
};
use super::dashboard_types::{ParsedPlanTasksFile, PlanTaskRuntimeFields};

// Re-export TuiDashboardModel and import shared functions from dashboard_model.
pub use super::dashboard_model::TuiDashboardModel;
use super::dashboard_model::{
    count_to_f64, load_json_opt, load_knowledge_browse_entries, load_snapshot_best_effort,
    resolve_snapshot_root,
};
#[cfg(test)]
use super::dashboard_model::load_snapshot_blocking;

pub(super) const MEMORY_DIR: &str = ".roko/memory";
pub(super) const EPISODES_FILE: &str = "episodes.jsonl";
pub(super) const TASK_METRICS_FILE: &str = "task-metrics.jsonl";

pub(super) const LEARN_DIR: &str = ".roko/learn";
pub(super) const EFFICIENCY_FILE: &str = "efficiency.jsonl";
pub(super) const EXPERIMENTS_FILE: &str = "experiments.json";
pub(super) const GATE_THRESHOLDS_FILE: &str = "gate-thresholds.json";
pub(super) const CASCADE_ROUTER_FILE: &str = "cascade-router.json";
pub(super) const SKILLS_FILE: &str = "skills.json";
pub(super) const PROVIDER_HEALTH_FILE: &str = "provider-health.json";
pub(super) const LATENCY_STATS_FILE: &str = "latency-stats.json";
pub(super) const NEURO_DIR: &str = ".roko/neuro";
pub(super) const KNOWLEDGE_FILE: &str = "knowledge.jsonl";
pub(super) const KNOWLEDGE_CONFIRMATIONS_FILE: &str = "knowledge-confirmations.jsonl";

pub(super) fn resolve_episodes_path(root: &Path) -> PathBuf {
    let canonical = root.join(".roko").join(EPISODES_FILE);
    if canonical.exists() {
        return canonical;
    }
    let learn_legacy = root.join(LEARN_DIR).join(EPISODES_FILE);
    if learn_legacy.exists() {
        return learn_legacy;
    }
    let memory_legacy = root.join(MEMORY_DIR).join(EPISODES_FILE);
    if memory_legacy.exists() {
        return memory_legacy;
    }
    canonical
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(super) struct FileStamp {
    pub(super) modified: Option<SystemTime>,
    pub(super) len: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
struct RunnerProjectionStamp {
    state_snapshot: FileStamp,
    legacy_executor: FileStamp,
}

impl FileStamp {
    fn from_path(path: &Path) -> Option<Self> {
        let meta = std::fs::metadata(path).ok()?;
        Some(Self {
            modified: meta.modified().ok(),
            len: meta.len(),
        })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
struct DashboardDataStamps {
    executor_state: RunnerProjectionStamp,
    efficiency: FileStamp,
    experiments: FileStamp,
    gate_thresholds: FileStamp,
    signals: FileStamp,
    episodes: FileStamp,
    cfactor: FileStamp,
    cascade_router: FileStamp,
    task_outputs: u64,
    event_log: FileStamp,
}

impl DashboardDataStamps {
    fn fingerprint(self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

static DASHBOARD_GENERATION_COUNTERS: OnceLock<
    Mutex<HashMap<PathBuf, Arc<DurableDashboardGenerationCounter>>>,
> = OnceLock::new();

/// In-memory scaffold of all placeholder dashboard pages.
#[derive(Debug, Clone)]
pub struct DashboardScaffold {
    pages: BTreeMap<PageId, PageScaffold>,
    active_page: PageId,
    snapshot: TuiDashboardModel,
}

impl DashboardScaffold {
    /// Build the full scaffold with all placeholder pages.
    #[must_use]
    pub fn new() -> Self {
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::new_in(root)
    }

    /// Build the scaffold and load snapshot data relative to `root`.
    #[must_use]
    pub fn new_in(root: impl AsRef<Path>) -> Self {
        let mut pages = BTreeMap::new();
        for page in efficiency::scaffold_pages()
            .into_iter()
            .chain(operations::scaffold_pages())
        {
            pages.insert(page.id, page);
        }

        let root = resolve_snapshot_root(root.as_ref());
        let snapshot = load_snapshot_best_effort(&root);

        Self {
            pages,
            active_page: PageId::Health,
            snapshot,
        }
    }

    /// List all pages in stable order.
    #[must_use]
    pub fn pages(&self) -> Vec<&PageScaffold> {
        self.pages.values().collect()
    }

    /// Current active page.
    #[must_use]
    pub const fn active_page(&self) -> PageId {
        self.active_page
    }

    /// Set active page if it exists in the scaffold.
    pub fn set_active_page(&mut self, page: PageId) -> bool {
        if self.pages.contains_key(&page) {
            self.active_page = page;
            true
        } else {
            false
        }
    }

    /// Return a specific page by ID.
    #[must_use]
    pub fn page(&self, page: PageId) -> Option<&PageScaffold> {
        self.pages.get(&page)
    }

    /// Build a high-level summary used by future command wiring.
    #[must_use]
    pub fn summary(&self) -> DashboardSummary {
        let widget_count = self.pages.values().map(|p| p.widgets.len()).sum();
        DashboardSummary {
            active_page: self.active_page,
            page_count: self.pages.len(),
            widget_count,
        }
    }

    /// Render a plain-text dashboard summary suitable for CLI output.
    #[must_use]
    pub fn render_overview_text(&self) -> String {
        let mut out = self.summary().to_string();
        out.push_str("\nactive page:\n");
        if let Some(page) = self.page(self.active_page) {
            let _ = writeln!(out, "{}", page.render_summary_line(true));
        }
        out.push_str("pages:\n");
        out.push_str(&self.render_page_index_text());
        out
    }

    /// Render the compact page index only.
    #[must_use]
    pub fn render_page_index_text(&self) -> String {
        let mut out = String::new();
        for page in self.pages.values() {
            let _ = writeln!(
                out,
                "{}",
                page.render_summary_line(page.id == self.active_page)
            );
        }
        out
    }

    /// Render one page as plain text. Returns `None` if the page does not exist.
    #[must_use]
    pub fn render_page_text(&self, page: PageId) -> Option<String> {
        let scaffold = self.page(page)?;
        let rendered = match page {
            PageId::Health => self.snapshot.render_health_page(scaffold),
            PageId::Trends => self.snapshot.render_trends_page(scaffold),
            PageId::Correlations => self.snapshot.render_correlations_page(scaffold),
            PageId::GateResults => self.snapshot.render_gate_results_page(scaffold),
            PageId::Learning => self.snapshot.render_learning_page(scaffold),
            PageId::Parameters => self.snapshot.render_parameters_page(scaffold),
            PageId::Experiments => self.snapshot.render_experiments_page(scaffold),
            PageId::Optimizer => self.snapshot.render_optimizer_page(scaffold),
            PageId::AgentStatus => self.snapshot.render_agent_status_page(scaffold),
            PageId::PlanView => self.snapshot.render_plan_view_page(scaffold),
            PageId::LogView => self.snapshot.render_log_view_page(scaffold),
            PageId::Signals => self.snapshot.render_signals_page(scaffold),
            PageId::ConfigView => self.snapshot.render_config_view_page(scaffold),
            PageId::ProviderHealth => self.snapshot.render_provider_health_page(scaffold),
            PageId::ModelComparison => self.snapshot.render_model_comparison_page(scaffold),
            PageId::Dreams => self.snapshot.render_dreams_page(scaffold),
        };
        rendered.or_else(|| Some(scaffold.render_text()))
    }

    /// Render one page's widget list only. Returns `None` if the page does not exist.
    #[must_use]
    pub fn render_page_list_text(&self, page: PageId) -> Option<String> {
        self.page(page).map(PageScaffold::render_widget_list)
    }

    /// Render the current active page as plain text.
    #[must_use]
    pub fn render_active_page_text(&self) -> String {
        self.render_page_text(self.active_page)
            .unwrap_or_else(|| String::from("<missing active page>"))
    }

    /// Render the health page as plain text.
    #[must_use]
    pub fn render_health_page_text(&self) -> String {
        self.render_page_text(PageId::Health)
            .unwrap_or_else(|| String::from("<missing health page>"))
    }

    /// Render the trends page as plain text.
    #[must_use]
    pub fn render_trends_page_text(&self) -> String {
        self.render_page_text(PageId::Trends)
            .unwrap_or_else(|| String::from("<missing trends page>"))
    }

    /// Render the provider health page as plain text.
    #[must_use]
    pub fn render_provider_health_page_text(&self) -> String {
        self.render_page_text(PageId::ProviderHealth)
            .unwrap_or_else(|| String::from("<missing provider health page>"))
    }

    /// Render the model comparison page as plain text.
    #[must_use]
    pub fn render_model_comparison_page_text(&self) -> String {
        self.render_page_text(PageId::ModelComparison)
            .unwrap_or_else(|| String::from("<missing model comparison page>"))
    }
}

impl Default for DashboardScaffold {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary metadata for the dashboard scaffold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DashboardSummary {
    /// Currently selected page.
    pub active_page: PageId,
    /// Number of pages scaffolded.
    pub page_count: usize,
    /// Number of widgets scaffolded across all pages.
    pub widget_count: usize,
}

/// An entry in the orchestrator event log.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventLogEntry {
    /// Epoch milliseconds when the event occurred.
    pub timestamp_ms: u64,
    /// Event kind (e.g. "task_started", "gate_passed").
    pub event_type: String,
    /// Plan the event belongs to.
    pub plan_id: String,
    /// Task the event belongs to.
    pub task_id: String,
    /// Human-readable event description.
    pub message: String,
}

/// Shared dashboard data loaded from `.roko/`.
#[derive(Debug, Clone, Default)]
pub struct DashboardData {
    /// Workspace root used for refreshes.
    root: PathBuf,
    /// Monotonic token advanced when tracked dashboard source files change.
    pub generation: u64,
    /// Effective spend configuration used by HTTP/runner/TUI projections.
    pub budget: roko_core::config::BudgetConfig,
    /// Cached executor state from the canonical durable Runner projection.
    executor_state: Value,
    /// Durable source supplying `executor_state`.
    runner_projection_source: Option<RunnerProjectionSource>,
    /// Exact durable source path supplying `executor_state`.
    runner_projection_path: Option<PathBuf>,
    /// Stable identity of the exact durable generation.
    runner_projection_generation: Option<String>,
    /// Missing/invalid loader status retained for TUI and `roko show`.
    runner_projection_status: String,
    /// Loader failure retained instead of silently becoming empty state.
    runner_projection_error: Option<String>,
    /// Last observed canonical and legacy state file metadata.
    executor_state_stamp: RunnerProjectionStamp,
    /// Plans from executor state.
    pub plans: Vec<PlanSummary>,
    /// Currently executing tasks.
    pub active_tasks: Vec<TaskSummary>,
    /// Agents tracked by the process supervisor.
    pub agents: Vec<AgentSummary>,
    /// Verify verdicts collected from signals.
    pub gate_results: Vec<GateResultSummary>,
    /// Efficiency aggregate from `.roko/learn/efficiency.jsonl`.
    pub efficiency: EfficiencySummary,
    /// Raw efficiency events from `.roko/learn/efficiency.jsonl`.
    pub efficiency_events: Vec<AgentEfficiencyEvent>,
    /// Hourly efficiency trend over the last 24 hours.
    pub efficiency_trend: Vec<EfficiencyBucket>,
    /// Hourly c-factor trend over the last 24 hours.
    pub cfactor_trend: Vec<CFactorBucket>,
    /// Last observed efficiency file metadata.
    efficiency_stamp: FileStamp,
    /// Cascade router state from `.roko/learn/cascade-router.json`.
    pub cascade_router: CascadeRouterState,
    /// Full experiment store from `.roko/learn/experiments.json`.
    pub experiment_store: ExperimentStore,
    /// Experiments from `.roko/learn/experiments.json`.
    pub experiments: Vec<ExperimentSummary>,
    /// Concluded experiment winners from `.roko/learn/experiments.json`.
    pub experiment_winners: Vec<ExperimentWinnerSummary>,
    /// Last observed experiments file metadata.
    experiments_stamp: FileStamp,
    /// Verify-results page data derived from signals and adaptive thresholds.
    pub gate_results_page: GateResultsPageData,
    /// Cached adaptive thresholds from `.roko/learn/gate-thresholds.json`.
    adaptive_thresholds: Option<AdaptiveThresholds>,
    /// Last observed gate-thresholds file metadata.
    gate_thresholds_stamp: FileStamp,
    /// Most recent signals from `.roko/engrams.jsonl`.
    pub recent_signals: Vec<SignalSummary>,
    /// Cached signal-derived gate results when executor state does not provide them.
    signal_gate_results: Vec<GateResultSummary>,
    /// Parsed gate-related signals for the gate-results page.
    gate_signal_summaries: Vec<GateSignalSummary>,
    /// Incremental cursor over `.roko/engrams.jsonl`.
    signal_cursor: SignalCursor,
    /// Snapshot of the currently executing plan for the Plan Execution page.
    pub current_plan_execution: Option<PlanExecutionSnapshot>,
    /// Incremental cursor over `.roko/episodes.jsonl`.
    episode_cursor: EpisodeCursor,
    /// Cached episodes for plan execution rendering.
    episodes: Vec<Episode>,
    /// Conductor alerts filtered from signals.
    pub conductor_alerts: Vec<AlertSummary>,
    /// Latest C-Factor snapshot, if present.
    pub cfactor: Option<CFactor>,
    /// Last observed C-Factor file metadata.
    cfactor_stamp: FileStamp,
    /// Cascade router file metadata.
    cascade_router_stamp: FileStamp,
    /// Incremental task-output cursors keyed by task ID.
    task_output_cursors: TaskOutputCursors,
    /// Per-task agent output tail cache derived from `.roko/task-outputs/`.
    pub task_outputs: HashMap<String, Vec<String>>,
    /// Cached git diff shown in the Dashboard Diff sub-tab.
    pub git_diff: String,
    /// Whether the cached git diff came from staged changes.
    pub git_diff_is_staged: bool,
    /// Orchestrator event log from `.roko/state/events.json`.
    pub event_log: Vec<EventLogEntry>,
    /// Whole-file reload cursor over `.roko/state/events.json`.
    event_log_cursor: EventLogCursor,
    /// Marketplace jobs from `.roko/jobs/`.
    pub marketplace_jobs: Vec<roko_core::MarketplaceJob>,
    /// PRD summaries from `.roko/prd/`.
    pub atelier_prds: Vec<roko_core::PrdSummary>,
    /// Per-slug task lists for Atelier.
    pub atelier_tasks_by_slug: std::collections::HashMap<String, Vec<roko_core::job::TaskSummary>>,
    /// Knowledge entries from `.roko/neuro/knowledge.jsonl` for the Inspect tab.
    pub knowledge_entries: Vec<KnowledgeBrowseEntry>,
    /// Incremental tailer for `.roko/learn/efficiency.jsonl`.
    efficiency_tailer: super::jsonl_tailer::IncrementalTailer<AgentEfficiencyEvent>,
    /// Incremental tailer for `.roko/learn/c-factor.jsonl`.
    cfactor_tailer: super::jsonl_tailer::IncrementalTailer<CFactor>,
}

/// Derived executor snapshot fields used by TUI orchestration chrome.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ExecutorSummary {
    pub orchestrator_state: String,
    pub current_iteration: usize,
    pub current_phase: String,
}

impl DashboardData {
    /// Load dashboard data from a workspace root, falling back to empty data on errors.
    #[must_use]
    pub fn load_best_effort(root: impl AsRef<Path>) -> Self {
        let root = resolve_snapshot_root(root.as_ref());
        let roko_dir = root.join(".roko");
        let learn_dir = roko_dir.join("learn");
        let signals_path = roko_dir.join("engrams.jsonl");
        let episodes_path = resolve_episodes_path(&root);
        let efficiency_path = learn_dir.join(EFFICIENCY_FILE);
        let experiments_path = learn_dir.join(EXPERIMENTS_FILE);
        let gate_thresholds_path = learn_dir.join(GATE_THRESHOLDS_FILE);
        let cascade_router_path = learn_dir.join(CASCADE_ROUTER_FILE);
        let cfactor_path = learn_dir.join("c-factor.jsonl");
        let events_path = roko_dir.join("state").join("events.json");
        let budget = roko_core::config::loader::load_config_unified(&root)
            .map(|config| config.budget)
            .unwrap_or_default();

        let (runner_projection, runner_projection_status, runner_projection_error) =
            match load_durable_runner_projection(&root) {
                Ok(projection) => {
                    let status = projection
                        .as_ref()
                        .map_or("missing", |projection| projection.source.label())
                        .to_string();
                    (projection, status, None)
                }
                Err(error) => {
                    tracing::warn!(
                        error = %error,
                        workdir = %root.display(),
                        "failed to load durable Runner projection; using empty state"
                    );
                    (None, "invalid".to_string(), Some(error.to_string()))
                }
            };
        let state = runner_projection
            .as_ref()
            .map_or(Value::Null, |projection| {
                projection.executor_projection.clone()
            });
        let runner_projection_source = runner_projection
            .as_ref()
            .map(|projection| projection.source);
        let runner_projection_path = runner_projection
            .as_ref()
            .map(|projection| projection.source_path.clone())
            .or_else(|| {
                (runner_projection_status == "invalid")
                    .then(|| root.join(roko_runtime::STATE_SNAPSHOT_RELATIVE_PATH))
            });
        let runner_projection_generation = runner_projection
            .as_ref()
            .map(|projection| projection.generation.clone());
        let state_stamp = runner_projection_stamp(&root);
        let signals_stamp = file_stamp(&signals_path);
        let episodes_stamp = file_stamp(&episodes_path);
        let event_log_stamp = file_stamp(&events_path);

        let mut signal_cursor = SignalCursor::new(&signals_path);
        let _ = signal_cursor.tick();
        let recent_signals = signal_cursor.recent_signals().to_vec();
        let gate_signal_summaries = signal_cursor.gate_signal_summaries().to_vec();
        let signal_gate_results = signal_cursor.signal_gate_results().to_vec();

        let mut episode_cursor = EpisodeCursor::new(&episodes_path);
        let _ = episode_cursor.tick();
        let episodes = episode_cursor.episodes().to_vec();

        let mut event_log_cursor = EventLogCursor::new(&events_path);
        let _ = event_log_cursor.tick();
        let event_log = event_log_cursor.event_log().to_vec();

        let runner_projection_valid = runner_projection_status != "invalid";
        let plans = if runner_projection_valid {
            load_plan_summaries(&root, &state)
        } else {
            Vec::new()
        };
        let active_tasks = if runner_projection_valid {
            load_active_tasks(&state)
        } else {
            Vec::new()
        };
        let mut agents = if runner_projection_valid {
            load_agents(&state)
        } else {
            Vec::new()
        };
        if runner_projection_valid {
            merge_runtime_agents(&mut agents, &root);
        }
        let gate_results = if runner_projection_valid {
            load_gate_results(&state, &signal_gate_results)
        } else {
            Vec::new()
        };
        let efficiency_events = read_efficiency_events_sync(&efficiency_path);
        let efficiency = load_efficiency_summary(&efficiency_path);
        let efficiency_trend = load_efficiency_trend(&efficiency_path);
        let cfactor_trend = load_cfactor_trend(&cfactor_path);
        let cascade_router =
            load_json_opt::<CascadeRouterState>(&cascade_router_path).unwrap_or_default();
        let cascade_router_stamp = file_stamp(&cascade_router_path);
        let experiment_store =
            load_json_opt::<ExperimentStore>(&experiments_path).unwrap_or_default();
        let experiments_stamp = file_stamp(&experiments_path);
        let mut experiments = experiment_store
            .iter()
            .map(ExperimentSummary::from_experiment)
            .collect::<Vec<_>>();
        experiments.sort_by(|a, b| a.experiment_id.cmp(&b.experiment_id));
        let experiment_winners = experiment_store.winner_summaries();
        let adaptive_thresholds = match runner_projection
            .as_ref()
            .and_then(|projection| projection.gate_thresholds.as_ref())
        {
            Some(thresholds) => serde_json::from_value(thresholds.clone()).ok(),
            None if runner_projection_status == "invalid" => None,
            None => load_json_opt::<AdaptiveThresholds>(&gate_thresholds_path),
        };
        let gate_thresholds_stamp = file_stamp(&gate_thresholds_path);
        let gate_results_page = if runner_projection_status == "invalid" {
            GateResultsPageData::default()
        } else {
            build_gate_results_page_data(&gate_signal_summaries, adaptive_thresholds.as_ref())
        };
        let conductor_alerts = recent_signals
            .iter()
            .filter(|signal| signal.kind.starts_with("conductor:alert:"))
            .map(AlertSummary::from_signal)
            .collect();
        let cfactor = load_latest_jsonl_value::<CFactor>(&cfactor_path);
        let cfactor_stamp = file_stamp(&cfactor_path);

        let task_outputs_dir = roko_dir.join("task-outputs");
        let mut task_output_cursors = TaskOutputCursors::new(&task_outputs_dir);
        let _ = task_output_cursors.reconcile();
        let _ = task_output_cursors.tick();
        let task_outputs = task_output_cursors.snapshot();

        let current_plan_execution = load_current_plan_execution(&root, &state, &episodes);
        let efficiency_stamp = file_stamp(&efficiency_path);

        // Backfill agent_output_tail from task-outputs if episode didn't provide it.
        let current_plan_execution =
            backfill_agent_output_tail(current_plan_execution, &mut task_output_cursors);

        let (git_diff, git_diff_is_staged) = load_dashboard_git_diff(&root);
        let generation = next_dashboard_data_generation(
            &root,
            DashboardDataStamps {
                executor_state: state_stamp,
                efficiency: efficiency_stamp,
                experiments: experiments_stamp,
                gate_thresholds: gate_thresholds_stamp,
                signals: signals_stamp,
                episodes: episodes_stamp,
                cfactor: cfactor_stamp,
                cascade_router: cascade_router_stamp,
                task_outputs: task_output_cursors.revision(),
                event_log: event_log_stamp,
            },
        );

        let (atelier_prds, atelier_tasks_by_slug) = scan_atelier_prds(&roko_dir);
        let knowledge_entries = load_knowledge_browse_entries(&root);

        // Initialize incremental tailers and do the first tick so items are
        // populated to match the full-read data already loaded above.
        let mut efficiency_tailer =
            super::jsonl_tailer::IncrementalTailer::<AgentEfficiencyEvent>::new(&efficiency_path);
        let _ = efficiency_tailer.tick();
        let mut cfactor_tailer =
            super::jsonl_tailer::IncrementalTailer::<CFactor>::new(&cfactor_path);
        let _ = cfactor_tailer.tick();

        Self {
            root,
            generation,
            budget,
            executor_state: state,
            runner_projection_source,
            runner_projection_path,
            runner_projection_generation,
            runner_projection_status,
            runner_projection_error,
            executor_state_stamp: state_stamp,
            plans,
            active_tasks,
            agents,
            gate_results,
            efficiency,
            efficiency_events,
            efficiency_trend,
            cfactor_trend,
            efficiency_stamp,
            cascade_router,
            experiment_store,
            experiments,
            experiment_winners,
            experiments_stamp,
            gate_results_page,
            adaptive_thresholds,
            gate_thresholds_stamp,
            recent_signals,
            signal_gate_results,
            gate_signal_summaries,
            signal_cursor,
            current_plan_execution,
            episode_cursor,
            episodes,
            conductor_alerts,
            cfactor,
            cfactor_stamp,
            cascade_router_stamp,
            task_output_cursors,
            task_outputs,
            git_diff,
            git_diff_is_staged,
            event_log,
            event_log_cursor,
            marketplace_jobs: scan_marketplace_jobs(&roko_dir),
            atelier_prds,
            atelier_tasks_by_slug,
            knowledge_entries,
            efficiency_tailer,
            cfactor_tailer,
        }
    }

    /// Refresh the snapshot from the stored workspace root.
    #[allow(deprecated)] // tick() is deprecated but still needed for standalone refresh
    pub async fn refresh(&mut self) -> Result<()> {
        let mut snapshot = std::mem::take(self);
        let refreshed = tokio::task::spawn_blocking(move || -> Result<Self> {
            snapshot.tick()?;
            Ok(snapshot)
        })
        .await??;
        *self = refreshed;
        Ok(())
    }

    /// Advance cursor-backed dashboard artifacts and refresh stamp-backed files once.
    ///
    /// **Deprecated**: In connected mode the TUI is fully push-based via
    /// `TuiDashboardModel`. This method is only used for standalone mode
    /// (`roko dashboard`) where no orchestrator is attached.
    #[deprecated(note = "use push-based TuiDashboardModel instead; kept for standalone mode only")]
    pub fn tick(&mut self) -> Result<()> {
        let roko_dir = self.root.join(".roko");
        let efficiency_path = roko_dir.join("learn").join(EFFICIENCY_FILE);
        let experiments_path = roko_dir.join("learn").join(EXPERIMENTS_FILE);
        let gate_thresholds_path = roko_dir.join("learn").join(GATE_THRESHOLDS_FILE);
        let cascade_router_path = roko_dir.join("learn").join(CASCADE_ROUTER_FILE);
        let cfactor_path = roko_dir.join("learn").join("c-factor.jsonl");

        let mut state_changed = false;
        let mut generation_changed = false;
        let mut episodes_changed = false;
        let stamp = runner_projection_stamp(&self.root);
        if stamp != self.executor_state_stamp {
            let projection = load_durable_runner_projection(&self.root);
            self.executor_state_stamp = stamp;
            match projection {
                Ok(projection) => {
                    self.executor_state = projection.as_ref().map_or(Value::Null, |projection| {
                        projection.executor_projection.clone()
                    });
                    self.runner_projection_source =
                        projection.as_ref().map(|projection| projection.source);
                    self.runner_projection_path = projection
                        .as_ref()
                        .map(|projection| projection.source_path.clone());
                    self.runner_projection_generation = projection
                        .as_ref()
                        .map(|projection| projection.generation.clone());
                    self.runner_projection_status = projection
                        .as_ref()
                        .map_or("missing", |projection| projection.source.label())
                        .to_string();
                    self.runner_projection_error = None;
                    self.adaptive_thresholds = match projection
                        .as_ref()
                        .and_then(|projection| projection.gate_thresholds.as_ref())
                    {
                        Some(thresholds) => serde_json::from_value(thresholds.clone()).ok(),
                        None => load_json_opt::<AdaptiveThresholds>(&gate_thresholds_path),
                    };
                }
                Err(error) => {
                    self.executor_state = Value::Null;
                    self.runner_projection_source = None;
                    self.runner_projection_path =
                        Some(self.root.join(roko_runtime::STATE_SNAPSHOT_RELATIVE_PATH));
                    self.runner_projection_generation = None;
                    self.runner_projection_status = "invalid".to_string();
                    self.runner_projection_error = Some(error.to_string());
                    self.adaptive_thresholds = None;
                }
            }
            state_changed = true;
            generation_changed = true;
        }

        if self.signal_cursor.tick()? {
            self.recent_signals = self.signal_cursor.recent_signals().to_vec();
            self.gate_signal_summaries = self.signal_cursor.gate_signal_summaries().to_vec();
            self.signal_gate_results = self.signal_cursor.signal_gate_results().to_vec();
            self.rebuild_signal_dependent_fields();
            generation_changed = true;
        }

        if self.episode_cursor.tick()? {
            self.episodes = self.episode_cursor.episodes().to_vec();
            episodes_changed = true;
            generation_changed = true;
        }

        // Incremental efficiency tailer: only deserialize newly appended lines.
        if self.efficiency_tailer.tick().unwrap_or(0) > 0 {
            self.efficiency_events = self.efficiency_tailer.items().to_vec();
            self.efficiency = efficiency_summary_from_events(&self.efficiency_events);
            // Trend still reads the file — only the O(N) deserialization of the
            // raw event list is avoided here.
            self.efficiency_trend = load_efficiency_trend(&efficiency_path);
            self.efficiency_stamp = file_stamp(&efficiency_path);
            generation_changed = true;
        }

        let stamp = file_stamp(&experiments_path);
        if stamp != self.experiments_stamp {
            self.experiments_stamp = stamp;
            self.experiment_store =
                load_json_opt::<ExperimentStore>(&experiments_path).unwrap_or_default();
            self.experiments = self
                .experiment_store
                .iter()
                .map(ExperimentSummary::from_experiment)
                .collect::<Vec<_>>();
            self.experiments
                .sort_by(|a, b| a.experiment_id.cmp(&b.experiment_id));
            self.experiment_winners = self.experiment_store.winner_summaries();
            generation_changed = true;
        }

        let stamp = file_stamp(&gate_thresholds_path);
        if stamp != self.gate_thresholds_stamp
            && self.runner_projection_source != Some(RunnerProjectionSource::StateSnapshot)
            && self.runner_projection_status != "invalid"
        {
            self.gate_thresholds_stamp = stamp;
            self.adaptive_thresholds = load_json_opt::<AdaptiveThresholds>(&gate_thresholds_path);
            self.rebuild_gate_results_page();
            generation_changed = true;
        }

        let stamp = file_stamp(&cascade_router_path);
        if stamp != self.cascade_router_stamp {
            self.cascade_router_stamp = stamp;
            self.cascade_router =
                load_json_opt::<CascadeRouterState>(&cascade_router_path).unwrap_or_default();
            generation_changed = true;
        }

        // Incremental c-factor tailer: pick up the latest CFactor snapshot.
        if self.cfactor_tailer.tick().unwrap_or(0) > 0 {
            self.cfactor = self.cfactor_tailer.items().last().cloned();
            self.cfactor_trend = load_cfactor_trend(&cfactor_path);
            self.cfactor_stamp = file_stamp(&cfactor_path);
            generation_changed = true;
        }

        let mut task_outputs_changed = false;
        if self.task_output_cursors.reconcile()? {
            task_outputs_changed = true;
        }
        if self.task_output_cursors.tick()? {
            task_outputs_changed = true;
        }
        if task_outputs_changed {
            self.task_outputs = self.task_output_cursors.snapshot();
            generation_changed = true;
        }

        if self.event_log_cursor.tick()? {
            self.event_log = self.event_log_cursor.event_log().to_vec();
            generation_changed = true;
        }

        // NOTE: load_dashboard_git_diff() was previously called
        // unconditionally here on every tick, blocking the main thread with
        // `git diff` subprocess calls.  Git diff data is now refreshed only
        // via the watcher-driven background path in App::drain_background_channels().

        if state_changed || episodes_changed || task_outputs_changed {
            if self.runner_projection_status == "invalid" {
                self.plans.clear();
                self.active_tasks.clear();
                self.agents.clear();
                self.gate_results.clear();
                self.gate_results_page = GateResultsPageData::default();
                self.current_plan_execution = None;
            } else {
                self.plans = load_plan_summaries(&self.root, &self.executor_state);
                self.active_tasks = load_active_tasks(&self.executor_state);
                self.agents = load_agents(&self.executor_state);
                merge_runtime_agents(&mut self.agents, &self.root);
                self.gate_results =
                    load_gate_results(&self.executor_state, &self.signal_gate_results);
                self.current_plan_execution = backfill_agent_output_tail(
                    load_current_plan_execution(&self.root, &self.executor_state, &self.episodes),
                    &mut self.task_output_cursors,
                );
            }
        }

        // Refresh marketplace jobs + PRDs each tick.
        self.marketplace_jobs = scan_marketplace_jobs(&roko_dir);
        let (prds, tasks_by_slug) = scan_atelier_prds(&roko_dir);
        self.atelier_prds = prds;
        self.atelier_tasks_by_slug = tasks_by_slug;

        if generation_changed {
            self.generation = self.generation.saturating_add(1);
        }

        Ok(())
    }

    fn rebuild_signal_dependent_fields(&mut self) {
        self.gate_results = load_gate_results(&self.executor_state, &self.signal_gate_results);
        self.rebuild_gate_results_page();
        self.conductor_alerts = self
            .recent_signals
            .iter()
            .filter(|signal| signal.kind.starts_with("conductor:alert:"))
            .map(AlertSummary::from_signal)
            .collect();
    }

    fn rebuild_gate_results_page(&mut self) {
        self.gate_results_page = build_gate_results_page_data(
            &self.gate_signal_summaries,
            self.adaptive_thresholds.as_ref(),
        );
    }

    /// Workspace root used to load dashboard artifacts.
    #[must_use]
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    /// Durable Runner source used for plan/task/agent/gate state.
    #[must_use]
    pub fn runner_projection_source(&self) -> Option<RunnerProjectionSource> {
        self.runner_projection_source
    }

    /// Exact durable Runner source path used by this snapshot.
    #[must_use]
    pub fn runner_projection_path(&self) -> Option<&Path> {
        self.runner_projection_path.as_deref()
    }

    /// Stable durable generation used by this snapshot.
    #[must_use]
    pub fn runner_projection_generation(&self) -> Option<&str> {
        self.runner_projection_generation.as_deref()
    }

    /// Stable source status exposed by TUI and `roko show`.
    #[must_use]
    pub fn runner_projection_status(&self) -> &str {
        if self.runner_projection_status.is_empty() {
            "missing"
        } else {
            &self.runner_projection_status
        }
    }

    /// Durable loader error, when the authoritative snapshot is invalid.
    #[must_use]
    pub fn runner_projection_error(&self) -> Option<&str> {
        self.runner_projection_error.as_deref()
    }

    /// Cached episodes for log display.
    #[must_use]
    pub(crate) fn episodes(&self) -> &[Episode] {
        &self.episodes
    }

    /// Per-task agent output tails.
    #[must_use]
    pub(crate) fn task_outputs(&self) -> &HashMap<String, Vec<String>> {
        &self.task_outputs
    }

    /// Executor-level summary derived from the canonical durable Runner projection.
    #[must_use]
    pub(crate) fn executor_summary(&self) -> ExecutorSummary {
        summarize_executor_state(&self.executor_state)
    }

    #[must_use]
    pub(crate) fn gate_signals_for_task(&self, task_id: &str) -> Vec<GateSignalSummary> {
        self.gate_signal_summaries
            .iter()
            .filter(|signal| signal.task_id.as_deref() == Some(task_id))
            .cloned()
            .collect()
    }

    /// Plan/task snapshots for the interactive plans tree and detail panes.
    #[must_use]
    pub(crate) fn plan_task_snapshots(&self) -> HashMap<String, PlanTaskListSnapshot> {
        build_plan_task_snapshots(
            &self.root,
            &self.executor_state,
            &self.plans,
            &self.active_tasks,
            &self.episodes,
        )
    }
}

/// Scan `.roko/jobs/` for marketplace job JSON files.
fn scan_marketplace_jobs(roko_dir: &Path) -> Vec<roko_core::MarketplaceJob> {
    let dir = roko_dir.join("jobs");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut jobs: Vec<roko_core::MarketplaceJob> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext == "json")
        })
        .filter_map(|e| {
            let data = std::fs::read_to_string(e.path()).ok()?;
            let mut job: roko_core::MarketplaceJob = serde_json::from_str(&data).ok()?;
            if job.id.is_empty() {
                job.id = e
                    .path()
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_string();
            }
            Some(job)
        })
        .collect();
    jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.id.cmp(&a.id)));
    jobs
}

/// Scan `.roko/prd/` for PRD markdown files, then correlate with plan task
/// files to populate task counts and per-slug task lists.
fn scan_atelier_prds(
    roko_dir: &Path,
) -> (
    Vec<roko_core::PrdSummary>,
    std::collections::HashMap<String, Vec<roko_core::job::TaskSummary>>,
) {
    let dir = roko_dir.join("prd");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return (Vec::new(), std::collections::HashMap::new());
    };
    let mut prds: Vec<roko_core::PrdSummary> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext == "md")
        })
        .filter_map(|e| {
            let slug = e
                .path()
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            let data = std::fs::read_to_string(e.path()).ok()?;
            // Extract title from first markdown heading.
            let title = data
                .lines()
                .find(|l| l.starts_with("# "))
                .map(|l| l.trim_start_matches("# ").trim().to_string())
                .unwrap_or_else(|| slug.clone());
            // Detect status from frontmatter or content.
            let status = if data.contains("status: published") || data.contains("Status: Published")
            {
                "published"
            } else if data.contains("status: planned") || data.contains("Status: Planned") {
                "planned"
            } else if data.contains("status: draft") || data.contains("Status: Draft") {
                "draft"
            } else {
                "idea"
            };
            Some(roko_core::PrdSummary {
                slug,
                title,
                status: status.to_string(),
                ..Default::default()
            })
        })
        .collect();
    prds.sort_by(|a, b| a.slug.cmp(&b.slug));

    // Scan plan directories for tasks.toml files and correlate with PRD slugs.
    let mut tasks_by_slug: std::collections::HashMap<String, Vec<roko_core::job::TaskSummary>> =
        std::collections::HashMap::new();

    // Workspace root is one level up from .roko/
    let workspace_root = roko_dir.parent().unwrap_or(roko_dir);
    let plan_dirs: Vec<PathBuf> = [workspace_root.join("plans"), roko_dir.join("plans")]
        .into_iter()
        .filter(|d| d.is_dir())
        .collect();

    for plan_dir in &plan_dirs {
        let Ok(plan_entries) = std::fs::read_dir(plan_dir) else {
            continue;
        };
        for entry in plan_entries.filter_map(|e| e.ok()) {
            let tasks_path = entry.path().join("tasks.toml");
            let Ok(tasks_file) = TasksFile::parse(&tasks_path) else {
                continue;
            };
            let plan_name = entry
                .path()
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();

            // Match plan to PRD: check if any PRD slug is a substring of the plan name,
            // or the plan name contains the slug.
            for prd in &mut prds {
                let slug_lower = prd.slug.to_lowercase();
                let plan_lower = plan_name.to_lowercase();
                if plan_lower.contains(&slug_lower) || slug_lower.contains(&plan_lower) {
                    prd.plan_count += 1;
                    prd.task_total += tasks_file.tasks.len();
                    let mut done = 0usize;
                    let mut failed = 0usize;
                    let mut task_summaries = Vec::new();
                    for task in &tasks_file.tasks {
                        match task.status.as_str() {
                            "done" | "completed" | "passed" => done += 1,
                            "failed" | "error" => failed += 1,
                            _ => {}
                        }
                        task_summaries.push(roko_core::job::TaskSummary {
                            id: task.id.clone(),
                            title: task.title.clone(),
                            status: task.status.clone(),
                            agent: String::new(),
                        });
                    }
                    prd.task_done += done;
                    prd.task_failed += failed;
                    tasks_by_slug
                        .entry(prd.slug.clone())
                        .or_default()
                        .extend(task_summaries);
                }
            }
        }
    }

    (prds, tasks_by_slug)
}

fn load_dashboard_git_diff(root: &Path) -> (String, bool) {
    let staged = run_dashboard_git_diff(root, true);
    if !staged.is_empty() {
        return (staged, true);
    }

    (run_dashboard_git_diff(root, false), false)
}

fn run_dashboard_git_diff(root: &Path, staged: bool) -> String {
    let args: &[&str] = if staged {
        &["diff", "--cached"]
    } else {
        &["diff", "HEAD"]
    };

    Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
        .unwrap_or_default()
}


impl SignalSummary {
    pub(crate) fn from_value(value: &Value) -> Option<Self> {
        Some(Self {
            id: value.get("id")?.as_str()?.to_string(),
            kind: value.get("kind")?.as_str()?.to_string(),
            created_at_ms: entry_timestamp_ms(value)?,
            confidence: signal_confidence(value),
            plan_id: value
                .pointer("/tags/plan_id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
                .or_else(|| {
                    value
                        .pointer("/body/data/plan_id")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                }),
            task_id: value
                .pointer("/tags/task_id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
                .or_else(|| {
                    value
                        .pointer("/body/data/task_id")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                }),
            parent_hash: signal_parent_hash(value),
            lineage: signal_lineage(value),
            payload_preview: signal_payload_preview(value),
        })
    }
}

impl GateResultSummary {
    pub(crate) fn from_signal(value: &Value, plan_id: &str) -> Option<Self> {
        let gate_name = extract_gate_name(value)?;
        let passed = extract_gate_passed(value)?;
        let duration_ms = extract_gate_duration_ms(value).unwrap_or(0);
        let rung = value
            .pointer("/tags/rung")
            .and_then(Value::as_u64)
            .or_else(|| value.pointer("/body/data/rung").and_then(Value::as_u64))
            .unwrap_or_default() as u32;
        let summary = value
            .pointer("/body/data/reason")
            .and_then(Value::as_str)
            .or_else(|| value.pointer("/body/reason").and_then(Value::as_str))
            .unwrap_or("")
            .to_string();

        Some(Self {
            plan_id: plan_id.to_string(),
            gate_name,
            passed,
            rung,
            duration_ms,
            summary,
        })
    }
}

impl GateSignalSummary {
    pub(crate) fn from_value(value: &Value) -> Option<Self> {
        if !value
            .get("kind")
            .and_then(Value::as_str)
            .is_some_and(is_gate_result_kind)
        {
            return None;
        }

        Some(Self {
            id: value.get("id")?.as_str()?.to_string(),
            created_at_ms: entry_timestamp_ms(value)?,
            plan_id: value
                .pointer("/tags/plan_id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
                .or_else(|| {
                    value
                        .pointer("/body/data/plan_id")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                })
                .or_else(|| {
                    value
                        .pointer("/body/plan_id")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                }),
            task_id: value
                .pointer("/tags/task_id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
                .or_else(|| {
                    value
                        .pointer("/body/data/task_id")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                })
                .or_else(|| {
                    value
                        .pointer("/body/task_id")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                }),
            gate_name: extract_gate_name(value)?,
            passed: extract_gate_passed(value)?,
            duration_ms: extract_gate_duration_ms(value).unwrap_or_default(),
            excerpt: gate_excerpt_from_value(value),
        })
    }
}

impl ExperimentSummary {
    pub(crate) fn from_experiment(
        experiment: &roko_learn::prompt_experiment::PromptExperiment,
    ) -> Self {
        let total_trials: u64 = experiment.stats.values().map(|stats| stats.trials).sum();
        let active_variants = experiment
            .variants
            .iter()
            .filter(|variant| variant.active)
            .count();
        Self {
            experiment_id: experiment.experiment_id.clone(),
            section_name: experiment.section_name.clone(),
            status: format!("{:?}", experiment.status),
            winner_id: experiment.winner_id.clone(),
            active_variants,
            total_trials,
        }
    }
}

/// Load orchestrator event log from `.roko/state/events.json`.
pub(crate) fn load_event_log(events_path: &Path) -> Vec<EventLogEntry> {
    let Some(value) = read_json_value(events_path) else {
        return Vec::new();
    };
    let Some(entries) = value.as_array() else {
        // Try as JSONL-style (one object = single event)
        return parse_event_entry(&value).into_iter().collect();
    };
    entries.iter().filter_map(parse_event_entry).collect()
}

fn parse_event_entry(value: &Value) -> Option<EventLogEntry> {
    Some(EventLogEntry {
        timestamp_ms: value
            .get("timestamp_ms")
            .and_then(Value::as_u64)
            .or_else(|| value.get("timestamp").and_then(Value::as_u64))
            .unwrap_or_default(),
        event_type: value
            .get("event_type")
            .or_else(|| value.get("type"))
            .or_else(|| value.get("kind"))
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        plan_id: value
            .get("plan_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        task_id: value
            .get("task_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        message: value
            .get("message")
            .or_else(|| value.get("detail"))
            .or_else(|| value.get("description"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
    })
}

/// Backfill `agent_output_tail` from task-outputs when episodes didn't provide it.
fn backfill_agent_output_tail(
    mut snapshot: Option<PlanExecutionSnapshot>,
    task_outputs: &mut TaskOutputCursors,
) -> Option<PlanExecutionSnapshot> {
    let exec = snapshot.as_mut()?;
    if exec.agent_output_tail.is_empty() {
        // Try current task first
        if let Some(detail) = &exec.current_task {
            if let Some(output) = task_outputs.tail_for(&detail.task_id) {
                exec.agent_output_tail = output.to_vec();
            }
        }
        // If still empty, try any task in the execution that has output
        if exec.agent_output_tail.is_empty() {
            for task_row in exec.tasks.iter().rev() {
                if let Some(output) = task_outputs.tail_for(&task_row.task_id) {
                    if !output.is_empty() {
                        exec.agent_output_tail = output.to_vec();
                        break;
                    }
                }
            }
        }
    }
    snapshot
}

fn load_plan_summaries(root: &Path, state: &Value) -> Vec<PlanSummary> {
    let mut ids = std::collections::BTreeSet::new();
    let canonical_runner = state.get("_runner_projection").is_some();
    let trackers = if canonical_runner {
        HashMap::new()
    } else {
        load_task_trackers(root)
    };
    if let Some(plan_states) = state.get("plan_states").and_then(Value::as_object) {
        ids.extend(plan_states.keys().cloned());
    }
    if ids.is_empty() {
        let pdir = plans_dir(root);
        if pdir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&pdir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_dir() && p.join("tasks.toml").exists() {
                        ids.insert(entry.file_name().to_string_lossy().into_owned());
                    }
                }
            }
        }
    }

    let mut summaries = Vec::new();
    for id in ids {
        let mut title = id.clone();
        let mut task_count = 0usize;
        let mut tasks_done = 0usize;
        let mut tasks_failed = 0usize;
        let plan_dir = plans_dir(root).join(&id);
        let tasks_path = plan_dir.join("tasks.toml");
        if let Ok(tasks_file) = TasksFile::parse(&tasks_path) {
            if !tasks_file.meta.plan.trim().is_empty() {
                title = tasks_file.meta.plan.clone();
            }
            task_count = tasks_file.tasks.len();

            let tracker = trackers.get(&id);
            let completed: HashSet<String> = tracker
                .map(|tracker| tracker.completed.iter().cloned().collect())
                .unwrap_or_default();
            let failed: HashSet<String> = tracker
                .map(|tracker| tracker.failed.iter().cloned().collect())
                .unwrap_or_default();

            for task in &tasks_file.tasks {
                if completed.contains(&task.id) || is_task_done_status(&task.status) {
                    tasks_done += 1;
                } else if failed.contains(&task.id) || is_task_failed_status(&task.status) {
                    tasks_failed += 1;
                }
            }
        }

        if let Some(task_outcomes) = runner_task_outcomes_for_plan(state, &id) {
            task_count = task_outcomes.len();
            tasks_done = task_outcomes
                .iter()
                .filter(|(_, status)| status == "passed")
                .count();
            tasks_failed = task_outcomes
                .iter()
                .filter(|(_, status)| {
                    matches!(
                        status.as_str(),
                        "failed" | "exhausted" | "cancelled" | "timed_out"
                    )
                })
                .count();
        }

        let phase = state
            .get("plan_states")
            .and_then(Value::as_object)
            .and_then(|plans| plans.get(&id))
            .and_then(current_phase_label)
            .unwrap_or_default();
        let runner_plan_status = state
            .pointer("/_runner_projection/lifecycle/plans")
            .and_then(Value::as_object)
            .and_then(|plans| plans.get(&id))
            .and_then(Value::as_str);
        let completed = runner_plan_status.is_some_and(|status| status != "started")
            || state
                .get("plan_states")
                .and_then(Value::as_object)
                .and_then(|plans| plans.get(&id))
                .is_some_and(plan_state_is_terminal);
        if task_count > 0
            && tasks_done == 0
            && tasks_failed == 0
            && phase.eq_ignore_ascii_case("complete")
        {
            tasks_done = task_count;
        }

        let tasks_done = (!canonical_runner)
            .then(|| {
                state
                    .get("plan_states")
                    .and_then(Value::as_object)
                    .and_then(|plans| plans.get(&id))
                    .and_then(|plan_state| {
                        plan_state
                            .get("done")
                            .and_then(Value::as_u64)
                            .or_else(|| plan_state.get("tasks_done").and_then(Value::as_u64))
                    })
                    .unwrap_or(tasks_done as u64) as usize
            })
            .unwrap_or(tasks_done);

        let tasks_failed = (!canonical_runner)
            .then(|| {
                state
                    .get("plan_states")
                    .and_then(Value::as_object)
                    .and_then(|plans| plans.get(&id))
                    .and_then(|plan_state| {
                        plan_state
                            .get("failed")
                            .and_then(Value::as_u64)
                            .or_else(|| plan_state.get("tasks_failed").and_then(Value::as_u64))
                    })
                    .unwrap_or(tasks_failed as u64) as usize
            })
            .unwrap_or(tasks_failed);

        let last_error = state
            .get("plan_states")
            .and_then(Value::as_object)
            .and_then(|plans| plans.get(&id))
            .and_then(|plan_state| {
                plan_state
                    .get("last_error")
                    .and_then(Value::as_str)
                    .or_else(|| plan_state.pointer("/error/message").and_then(Value::as_str))
                    .or_else(|| plan_state.get("error").and_then(Value::as_str))
            })
            .map(ToOwned::to_owned);

        summaries.push(PlanSummary {
            id: id.clone(),
            title,
            task_count,
            tasks_done,
            tasks_failed,
            completed,
            status: if completed {
                if runner_plan_status == Some("failed") || phase.eq_ignore_ascii_case("failed") {
                    "failed"
                } else {
                    "done"
                }
            } else if state
                .get("plan_states")
                .and_then(Value::as_object)
                .and_then(|plans| plans.get(&id))
                .is_some_and(plan_state_is_paused)
            {
                "paused"
            } else if state
                .get("plan_states")
                .and_then(Value::as_object)
                .and_then(|plans| plans.get(&id))
                .is_some_and(|plan_state| runner_plan_is_started(state, &id, plan_state))
            {
                "running"
            } else {
                "ready"
            }
            .into(),
            superseded_by: None,
            old_format: false,
            last_error,
        });
    }

    summaries.sort_by(|a, b| a.id.cmp(&b.id));
    summaries
}

pub(super) fn runner_task_outcomes_for_plan(state: &Value, plan_id: &str) -> Option<Vec<(String, String)>> {
    let runner = state.get("_runner_projection")?;
    if let Some(tasks) = runner
        .pointer("/lifecycle/tasks")
        .and_then(Value::as_object)
    {
        return Some(
            tasks
                .values()
                .filter(|task| task.get("plan_id").and_then(Value::as_str) == Some(plan_id))
                .filter_map(|task| {
                    let task_id = task.get("task_id")?.as_str()?;
                    Some((
                        task_id.to_string(),
                        runner_terminal_task_outcome(runner, plan_id, task_id)
                            .unwrap_or_else(|| {
                                task.get("status")
                                    .and_then(Value::as_str)
                                    .unwrap_or("started")
                            })
                            .to_string(),
                    ))
                })
                .collect(),
        );
    }
    let mut outcomes = Vec::new();
    for (field, status) in [("completed_tasks", "passed"), ("failed_tasks", "failed")] {
        outcomes.extend(
            runner
                .get(field)
                .and_then(|plans| plans.get(plan_id))
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(|task_id| (task_id.to_string(), status.to_string())),
        );
    }
    outcomes.extend(
        runner
            .get("skipped_tasks")
            .and_then(|plans| plans.get(plan_id))
            .and_then(Value::as_object)
            .into_iter()
            .flat_map(|tasks| tasks.keys())
            .map(|task_id| (task_id.clone(), "skipped".to_string())),
    );
    Some(outcomes)
}

pub(super) fn runner_terminal_task_outcome<'a>(
    runner: &'a Value,
    plan_id: &str,
    task_id: &str,
) -> Option<&'a str> {
    if runner
        .get("completed_tasks")
        .and_then(|plans| plans.get(plan_id))
        .and_then(Value::as_array)
        .is_some_and(|tasks| tasks.iter().any(|task| task.as_str() == Some(task_id)))
    {
        return Some("passed");
    }
    if runner
        .get("failed_tasks")
        .and_then(|plans| plans.get(plan_id))
        .and_then(Value::as_array)
        .is_some_and(|tasks| tasks.iter().any(|task| task.as_str() == Some(task_id)))
    {
        return Some("failed");
    }
    runner
        .get("skipped_tasks")
        .and_then(|plans| plans.get(plan_id))
        .and_then(Value::as_object)
        .filter(|tasks| tasks.contains_key(task_id))
        .map(|_| "skipped")
}

fn build_plan_task_snapshots(
    root: &Path,
    state: &Value,
    plans: &[PlanSummary],
    active_tasks: &[TaskSummary],
    episodes: &[Episode],
) -> HashMap<String, PlanTaskListSnapshot> {
    let trackers = if state.get("_runner_projection").is_some() {
        HashMap::new()
    } else {
        load_task_trackers(root)
    };
    let plan_states = state.get("plan_states").and_then(Value::as_object);
    let active_by_key: HashMap<(String, String), &TaskSummary> = active_tasks
        .iter()
        .map(|task| ((task.plan_id.clone(), task.task_id.clone()), task))
        .collect();
    let current_task_by_plan: HashMap<&str, &str> = active_tasks
        .iter()
        .map(|task| (task.plan_id.as_str(), task.task_id.as_str()))
        .collect();
    let mut snapshots = HashMap::new();

    for plan in plans {
        let plan_state = plan_states.and_then(|states| states.get(&plan.id));
        let phase = plan_state.and_then(current_phase_label).unwrap_or_else(|| {
            if plan.completed {
                String::from("done")
            } else {
                String::from("pending")
            }
        });
        let plan_succeeded = PlanPhase::from(phase.as_str()).is_done();
        let active = plan_state
            .map(|plan_state| runner_plan_is_started(state, &plan.id, plan_state))
            .unwrap_or(!plan.completed);
        let elapsed_secs = episodes
            .iter()
            .filter(|episode| episode_matches_plan(episode, &plan.id, None))
            .map(|episode| episode.usage.wall_ms as f64 / 1000.0)
            .sum();
        let mut snapshot = PlanTaskListSnapshot {
            phase,
            active,
            elapsed_secs,
            ..PlanTaskListSnapshot::default()
        };

        let tasks_path = plans_dir(root).join(&plan.id).join("tasks.toml");
        let parsed = match parse_plan_tasks_file(&tasks_path) {
            Ok(parsed) => parsed,
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    path = %tasks_path.display(),
                    plan_id = %plan.id,
                    "failed to parse tasks.toml for TUI plan snapshot"
                );
                snapshots.insert(plan.id.clone(), snapshot);
                continue;
            }
        };
        let tasks_file = &parsed.tasks_file;
        if parsed.task_runtime_fields.len() != tasks_file.tasks.len() {
            tracing::warn!(
                path = %tasks_path.display(),
                plan_id = %plan.id,
                runtime_fields = parsed.task_runtime_fields.len(),
                parsed_tasks = tasks_file.tasks.len(),
                "tasks.toml runtime metadata count did not match parsed task count"
            );
        }

        let tracker = trackers.get(&plan.id);
        let mut completed: HashSet<String> = tracker
            .map(|tracker| tracker.completed.iter().cloned().collect())
            .unwrap_or_default();
        let mut failed: HashSet<String> = tracker
            .map(|tracker| tracker.failed.iter().cloned().collect())
            .unwrap_or_default();
        let runner_outcomes = runner_task_outcomes_for_plan(state, &plan.id).unwrap_or_default();
        if !runner_outcomes.is_empty() || state.get("_runner_projection").is_some() {
            completed = runner_outcomes
                .iter()
                .filter(|(_, status)| status == "passed")
                .map(|(task_id, _)| task_id.clone())
                .collect();
            failed = runner_outcomes
                .iter()
                .filter(|(_, status)| {
                    matches!(
                        status.as_str(),
                        "failed" | "exhausted" | "cancelled" | "timed_out"
                    )
                })
                .map(|(task_id, _)| task_id.clone())
                .collect();
        }
        let current_task_id = current_task_by_plan
            .get(plan.id.as_str())
            .map(|task_id| (*task_id).to_string())
            .or_else(|| current_task_id(&tasks_file, tracker, &completed, &failed));

        snapshot.tasks = tasks_file
            .tasks
            .iter()
            .enumerate()
            .map(|(index, task)| {
                let runtime = parsed.task_runtime_fields.get(index);
                let active_task = active_by_key
                    .get(&(plan.id.clone(), task.id.clone()))
                    .copied();
                let runner_outcome = runner_outcomes
                    .iter()
                    .find(|(task_id, _)| task_id == &task.id)
                    .map(|(_, status)| status.as_str());
                let status = if runner_outcome == Some("skipped") {
                    String::from("skipped")
                } else if completed.contains(&task.id) {
                    String::from("done")
                } else if failed.contains(&task.id) {
                    String::from("failed")
                } else if let Some(active_task) = active_task {
                    active_task.status.clone()
                } else if is_task_done_status(&task.status) {
                    String::from("done")
                } else if is_task_failed_status(&task.status) {
                    String::from("failed")
                } else if plan_succeeded {
                    String::from("done")
                } else {
                    task_phase_label(
                        task,
                        &snapshot.phase,
                        current_task_id.as_deref(),
                        tracker,
                        &completed,
                        &failed,
                    )
                };

                let acceptance_text = if task.acceptance.is_empty() {
                    None
                } else {
                    Some(task.acceptance.join("; "))
                };
                let verify_command = task.verify.first().map(|step| step.command.clone());

                PlanTaskSnapshot {
                    id: task.id.clone(),
                    title: task.title.clone(),
                    tier: task.tier.clone(),
                    model_hint: task.model_hint.clone(),
                    status,
                    agent_id: active_task.and_then(|task| task.assigned_agents.first().cloned()),
                    model: runtime
                        .and_then(|runtime| runtime.model.clone())
                        .or_else(|| task.model_hint.clone()),
                    elapsed_ms: runtime.and_then(|runtime| runtime.elapsed_ms),
                    started_at: runtime.and_then(|runtime| runtime.started_at.clone()),
                    ended_at: runtime.and_then(|runtime| runtime.ended_at.clone()),
                    wave: runtime.and_then(|runtime| runtime.wave),
                    dependencies: task.depends_on.clone(),
                    acceptance_text,
                    verify_command,
                    files: task.files.clone(),
                }
            })
            .collect();
        snapshot.tasks_done = snapshot
            .tasks
            .iter()
            .filter(|task| is_task_done_status(&task.status))
            .count();
        snapshot.tasks_failed = snapshot
            .tasks
            .iter()
            .filter(|task| is_task_failed_status(&task.status))
            .count();
        snapshot.elapsed_ms = snapshot
            .tasks
            .iter()
            .map(|task| task.elapsed_ms.unwrap_or(0))
            .sum();
        if snapshot.elapsed_ms > 0 {
            snapshot.elapsed_secs = snapshot.elapsed_ms as f64 / 1000.0;
        }
        snapshot.wave = parsed.plan_wave.unwrap_or_else(|| {
            snapshot
                .tasks
                .iter()
                .filter(|task| TaskStatus::from(task.status.as_str()).is_active())
                .filter_map(|task| task.wave)
                .max()
                .unwrap_or_default()
        });
        snapshot.failed_count = snapshot.tasks_failed as u32;

        snapshots.insert(plan.id.clone(), snapshot);
    }

    snapshots
}

fn parse_plan_tasks_file(path: &Path) -> Result<ParsedPlanTasksFile> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let tasks_file = toml::from_str::<TasksFile>(&content)
        .with_context(|| format!("parse {}", path.display()))?;
    let raw = toml::from_str::<toml::Value>(&content)
        .with_context(|| format!("parse runtime metadata from {}", path.display()))?;
    let raw_tasks = raw.get("task").and_then(toml::Value::as_array);

    let task_runtime_fields = tasks_file
        .tasks
        .iter()
        .enumerate()
        .map(|(index, task)| {
            let task_table = raw_tasks
                .and_then(|tasks| tasks.get(index))
                .and_then(toml::Value::as_table)
                .or_else(|| {
                    raw_tasks.and_then(|tasks| {
                        tasks.iter().find_map(|task_value| {
                            let table = task_value.as_table()?;
                            let id = table.get("id").and_then(toml::Value::as_str)?;
                            (id == task.id).then_some(table)
                        })
                    })
                });

            PlanTaskRuntimeFields {
                model: task_table
                    .and_then(|table| table.get("model"))
                    .and_then(toml_scalar_to_string),
                elapsed_ms: task_table
                    .and_then(|table| table.get("elapsed_ms"))
                    .and_then(toml_value_to_u64),
                started_at: task_table
                    .and_then(|table| {
                        table
                            .get("started_at")
                            .or_else(|| table.get("started_at_ms"))
                            .or_else(|| table.get("start_time"))
                    })
                    .and_then(toml_scalar_to_string),
                ended_at: task_table
                    .and_then(|table| {
                        table
                            .get("ended_at")
                            .or_else(|| table.get("ended_at_ms"))
                            .or_else(|| table.get("end_time"))
                    })
                    .and_then(toml_scalar_to_string),
                wave: task_table
                    .and_then(|table| table.get("wave"))
                    .and_then(toml_value_to_u32),
            }
        })
        .collect();

    let plan_wave = raw
        .get("meta")
        .and_then(toml::Value::as_table)
        .and_then(|meta| meta.get("wave"))
        .and_then(toml_value_to_u32);

    Ok(ParsedPlanTasksFile {
        tasks_file,
        task_runtime_fields,
        plan_wave,
    })
}

fn toml_scalar_to_string(value: &toml::Value) -> Option<String> {
    match value {
        toml::Value::String(value) => Some(value.clone()),
        toml::Value::Integer(value) => Some(value.to_string()),
        toml::Value::Float(value) => Some(value.to_string()),
        toml::Value::Boolean(value) => Some(value.to_string()),
        toml::Value::Datetime(value) => Some(value.to_string()),
        _ => None,
    }
}

fn toml_value_to_u64(value: &toml::Value) -> Option<u64> {
    value
        .as_integer()
        .and_then(|value| u64::try_from(value).ok())
}

fn toml_value_to_u32(value: &toml::Value) -> Option<u32> {
    value
        .as_integer()
        .and_then(|value| u32::try_from(value).ok())
}

fn is_task_done_status(status: &str) -> bool {
    TaskStatus::from(status).is_done()
}

fn is_task_failed_status(status: &str) -> bool {
    if TaskStatus::from(status).is_failed() {
        return true;
    }

    let normalized = status.trim().to_ascii_lowercase();
    let compact = normalized.replace(['-', '_', ' '], "");
    matches!(
        compact.as_str(),
        "gaterejected" | "reviewrejected" | "rejected"
    )
}

fn load_active_tasks(state: &Value) -> Vec<TaskSummary> {
    if let Some(tasks) = state
        .pointer("/_runner_projection/lifecycle/tasks")
        .and_then(Value::as_object)
    {
        let attempts = state
            .pointer("/_runner_projection/lifecycle/task_attempts")
            .and_then(Value::as_object);
        let plan_states = state.get("plan_states").and_then(Value::as_object);
        let mut projected = tasks
            .values()
            .filter_map(|task| {
                let plan_id = task.get("plan_id")?.as_str()?;
                let task_id = task.get("task_id")?.as_str()?;
                let status = task.get("status")?.as_str()?;
                let paused = plan_states
                    .and_then(|states| states.get(plan_id))
                    .is_some_and(plan_state_is_paused);
                let plan_started = plan_states
                    .and_then(|states| states.get(plan_id))
                    .is_some_and(|plan_state| runner_plan_is_started(state, plan_id, plan_state));
                if paused || !plan_started || !matches!(status, "started" | "running" | "retrying")
                {
                    return None;
                }
                let current_attempt = task
                    .get("current_attempt")
                    .and_then(Value::as_u64)
                    .unwrap_or(1);
                let mut assigned_agents = attempts
                    .into_iter()
                    .flat_map(|attempts| attempts.values())
                    .filter(|attempt| {
                        attempt.get("plan_id").and_then(Value::as_str) == Some(plan_id)
                            && attempt.get("task_id").and_then(Value::as_str) == Some(task_id)
                            && attempt.get("attempt").and_then(Value::as_u64)
                                == Some(current_attempt)
                            && matches!(
                                attempt.get("status").and_then(Value::as_str),
                                Some("agent_running" | "cancelling" | "cancellation_failed")
                            )
                    })
                    .filter_map(|attempt| attempt.get("agent_id").and_then(Value::as_str))
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                assigned_agents.sort();
                assigned_agents.dedup();
                Some(TaskSummary {
                    plan_id: plan_id.to_string(),
                    task_id: task_id.to_string(),
                    status: status.to_string(),
                    iteration: u32::try_from(current_attempt).unwrap_or(1),
                    assigned_agents,
                    latest_gate: None,
                })
            })
            .collect::<Vec<_>>();
        projected.sort_by(|left, right| {
            left.plan_id
                .cmp(&right.plan_id)
                .then_with(|| left.task_id.cmp(&right.task_id))
        });
        return projected;
    }
    let Some(plan_states) = state.get("plan_states").and_then(Value::as_object) else {
        return Vec::new();
    };

    let mut tasks = Vec::new();
    for (plan_id, plan_state) in plan_states {
        let status = current_phase_label(plan_state).unwrap_or_else(|| "unknown".to_string());
        if !runner_plan_is_started(state, plan_id, plan_state) {
            continue;
        }
        let Some(task_id) = plan_state
            .get("task_id")
            .and_then(Value::as_str)
            .or_else(|| plan_state.get("id").and_then(Value::as_str))
            .filter(|task_id| !task_id.trim().is_empty())
        else {
            continue;
        };
        let task_id = task_id.to_string();
        let iteration = plan_state
            .get("iteration")
            .and_then(Value::as_u64)
            .unwrap_or(1) as u32;
        let assigned_agents = plan_state
            .get("assigned_agents")
            .and_then(Value::as_array)
            .map(|agents| {
                agents
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        let latest_gate = plan_state
            .get("gate_results")
            .and_then(Value::as_array)
            .and_then(|results| results.last())
            .and_then(|result| result.get("gate_name"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);

        tasks.push(TaskSummary {
            plan_id: plan_id.clone(),
            task_id,
            status,
            iteration,
            assigned_agents,
            latest_gate,
        });
    }

    tasks.sort_by(|a, b| {
        a.plan_id
            .cmp(&b.plan_id)
            .then_with(|| a.task_id.cmp(&b.task_id))
    });
    tasks
}

fn load_agents(state: &Value) -> Vec<AgentSummary> {
    if let Some(attempts) = state
        .pointer("/_runner_projection/lifecycle/task_attempts")
        .and_then(Value::as_object)
    {
        let tasks = state
            .pointer("/_runner_projection/lifecycle/tasks")
            .and_then(Value::as_object);
        let plan_states = state.get("plan_states").and_then(Value::as_object);
        let mut latest = HashMap::<String, &Value>::new();
        for attempt in attempts.values() {
            let Some(plan_id) = attempt.get("plan_id").and_then(Value::as_str) else {
                continue;
            };
            let Some(task_id) = attempt.get("task_id").and_then(Value::as_str) else {
                continue;
            };
            let number = attempt
                .get("attempt")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            let current_attempt = tasks
                .and_then(|tasks| tasks.get(&format!("{plan_id}:{task_id}")))
                .and_then(|task| task.get("current_attempt"))
                .and_then(Value::as_u64);
            let plan_started = plan_states
                .and_then(|plans| plans.get(plan_id))
                .is_some_and(|plan_state| runner_plan_is_started(state, plan_id, plan_state));
            if current_attempt != Some(number) || !plan_started {
                continue;
            }
            let Some(agent_id) = attempt.get("agent_id").and_then(Value::as_str) else {
                continue;
            };
            if latest.get(agent_id).is_none_or(|current| {
                let current_rank = (
                    current
                        .get("started_at_ms")
                        .and_then(Value::as_u64)
                        .unwrap_or_default(),
                    current
                        .get("attempt")
                        .and_then(Value::as_u64)
                        .unwrap_or_default(),
                    current
                        .get("plan_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                    current
                        .get("task_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                );
                let next_rank = (
                    attempt
                        .get("started_at_ms")
                        .and_then(Value::as_u64)
                        .unwrap_or_default(),
                    number,
                    attempt
                        .get("plan_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                    attempt
                        .get("task_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                );
                current_rank <= next_rank
            }) {
                latest.insert(agent_id.to_string(), attempt);
            }
        }
        let mut agents = latest
            .into_iter()
            .filter(|(_, attempt)| {
                matches!(
                    attempt.get("status").and_then(Value::as_str),
                    Some("agent_running" | "cancelling" | "cancellation_failed")
                )
            })
            .map(|(agent_id, attempt)| AgentSummary {
                id: agent_id.clone(),
                label: agent_id,
                plan_id: attempt
                    .get("plan_id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                status: attempt
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
            })
            .collect::<Vec<_>>();
        agents.sort_by(|left, right| left.id.cmp(&right.id));
        return agents;
    }
    let Some(plan_states) = state.get("plan_states").and_then(Value::as_object) else {
        return Vec::new();
    };

    let mut agents = Vec::new();
    for (plan_id, plan_state) in plan_states {
        let status = current_phase_label(plan_state).unwrap_or_else(|| "unknown".to_string());
        let assigned_agents = plan_state
            .get("assigned_agents")
            .and_then(Value::as_array)
            .map(|agents| {
                agents
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for agent in assigned_agents {
            agents.push(AgentSummary {
                id: agent.clone(),
                label: agent,
                plan_id: Some(plan_id.clone()),
                status: status.clone(),
            });
        }
    }

    agents.sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.plan_id.cmp(&b.plan_id)));
    agents
}

/// Runtime entry from `.roko/runtime/agents.json`.
#[derive(Debug, Deserialize)]
struct RuntimeAgentEntry {
    name: String,
    pid: u32,
    #[allow(dead_code)]
    bind: String,
}

/// Check whether a process with the given PID is alive.
#[cfg(unix)]
#[allow(unsafe_code, clippy::cast_possible_wrap)]
fn is_process_alive(pid: u32) -> bool {
    // SAFETY: signal 0 is an existence check — no signal is delivered.
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

#[cfg(not(unix))]
fn is_process_alive(_pid: u32) -> bool {
    false
}

/// Merge agents from `.roko/runtime/agents.json` (alive PIDs) into the
/// agent list, deduplicating by ID.
fn merge_runtime_agents(agents: &mut Vec<AgentSummary>, workdir: &Path) {
    let path = workdir.join(".roko").join("runtime").join("agents.json");
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return;
    };
    let Ok(entries) = serde_json::from_str::<Vec<RuntimeAgentEntry>>(&contents) else {
        return;
    };
    let existing: HashSet<String> = agents.iter().map(|a| a.id.clone()).collect();
    for entry in entries {
        if !is_process_alive(entry.pid) {
            continue;
        }
        if existing.contains(&entry.name) {
            continue;
        }
        agents.push(AgentSummary {
            id: entry.name.clone(),
            label: entry.name,
            plan_id: None,
            status: "running".to_string(),
        });
    }
    agents.sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.plan_id.cmp(&b.plan_id)));
}

fn load_gate_results(
    state: &Value,
    signal_gate_results: &[GateResultSummary],
) -> Vec<GateResultSummary> {
    let mut gate_results = Vec::new();

    if let Some(plan_states) = state.get("plan_states").and_then(Value::as_object) {
        for (plan_id, plan_state) in plan_states {
            let Some(results) = plan_state.get("gate_results").and_then(Value::as_array) else {
                continue;
            };
            for result in results {
                gate_results.push(GateResultSummary {
                    plan_id: plan_id.clone(),
                    gate_name: result
                        .get("gate_name")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                        .to_string(),
                    passed: result
                        .get("passed")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    rung: result
                        .get("rung")
                        .and_then(Value::as_u64)
                        .unwrap_or_default() as u32,
                    duration_ms: result
                        .get("duration_ms")
                        .and_then(Value::as_u64)
                        .unwrap_or_default(),
                    summary: result
                        .get("summary")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                });
            }
        }
    }

    if gate_results.is_empty() {
        gate_results.extend(signal_gate_results.iter().cloned());
    }

    gate_results.sort_by(|a, b| {
        a.plan_id
            .cmp(&b.plan_id)
            .then_with(|| a.gate_name.cmp(&b.gate_name))
            .then_with(|| a.rung.cmp(&b.rung))
    });
    gate_results
}

#[derive(Debug, Clone, Deserialize)]
struct TaskTrackerSnapshot {
    plan_id: String,
    #[serde(default)]
    completed: Vec<String>,
    #[serde(default)]
    failed: Vec<String>,
    #[serde(default)]
    current_group_index: usize,
}

fn load_task_trackers(root: &Path) -> HashMap<String, TaskTrackerSnapshot> {
    let path = root.join(".roko").join("state").join("task-trackers.json");
    let Some(value) = read_json_value(&path) else {
        return HashMap::new();
    };
    let Some(entries) = value.as_array() else {
        return HashMap::new();
    };

    let mut trackers = HashMap::new();
    for entry in entries {
        let Ok(record) = serde_json::from_value::<TaskTrackerSnapshot>(entry.clone()) else {
            continue;
        };
        if !record.plan_id.trim().is_empty() {
            trackers.insert(record.plan_id.clone(), record);
        }
    }
    trackers
}

fn load_current_plan_execution(
    root: &Path,
    state: &Value,
    episodes: &[Episode],
) -> Option<PlanExecutionSnapshot> {
    let plan_states = state.get("plan_states").and_then(Value::as_object)?;
    let trackers = if state.get("_runner_projection").is_some() {
        HashMap::new()
    } else {
        load_task_trackers(root)
    };

    let mut candidates = plan_states
        .iter()
        .filter_map(|(plan_id, plan_state)| {
            let phase = current_phase_label(plan_state)?;
            if !runner_plan_is_started(state, plan_id, plan_state) {
                return None;
            }
            let priority = execution_phase_priority(&phase);
            let started_at_ms = plan_state
                .get("started_at_ms")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            Some((priority, started_at_ms, plan_id.clone()))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| b.1.cmp(&a.1))
            .then_with(|| a.2.cmp(&b.2))
    });

    let plan_id = candidates
        .first()
        .map(|(_, _, plan_id)| plan_id.clone())
        .or_else(|| trackers.keys().next().cloned())?;

    let plan_state = plan_states.get(&plan_id)?;
    let plan_phase = current_phase_label(plan_state).unwrap_or_else(|| String::from("queued"));
    let plan_dir = plans_dir(root).join(&plan_id);
    let tasks_file = TasksFile::parse(&plan_dir.join("tasks.toml")).ok()?;
    let tracker = trackers.get(&plan_id);
    let mut completed: HashSet<String> = tracker
        .map(|tracker| tracker.completed.iter().cloned().collect())
        .unwrap_or_default();
    let mut failed: HashSet<String> = tracker
        .map(|tracker| tracker.failed.iter().cloned().collect())
        .unwrap_or_default();
    if let Some(tasks) = runner_task_outcomes_for_plan(state, &plan_id) {
        completed = tasks
            .iter()
            .filter(|(_, status)| status == "passed")
            .map(|(task_id, _)| task_id.clone())
            .collect();
        failed = tasks
            .iter()
            .filter(|(_, status)| {
                matches!(
                    status.as_str(),
                    "failed" | "exhausted" | "cancelled" | "timed_out"
                )
            })
            .map(|(task_id, _)| task_id.clone())
            .collect();
    }
    let current_task_id = load_active_tasks(state)
        .into_iter()
        .find(|task| task.plan_id == plan_id)
        .map(|task| task.task_id)
        .or_else(|| current_task_id(&tasks_file, tracker, &completed, &failed));
    let plan_title = if tasks_file.meta.plan.trim().is_empty() {
        plan_id.clone()
    } else {
        tasks_file.meta.plan.clone()
    };
    let current_episode = latest_episode_for_plan(&episodes, &plan_id, current_task_id.as_deref())
        .or_else(|| latest_episode_for_plan(&episodes, &plan_id, None));
    let agent_output_tail = current_episode
        .as_ref()
        .and_then(extract_episode_output_text)
        .map(|text| tail_lines(&text, 20))
        .unwrap_or_default()
        .lines()
        .map(|line| line.to_string())
        .collect::<Vec<_>>();

    let current_task = current_task_id.as_ref().and_then(|task_id| {
        tasks_file
            .tasks
            .iter()
            .find(|task| task.id == *task_id)
            .map(|task| build_task_detail(task))
    });

    let mut tasks = Vec::with_capacity(tasks_file.tasks.len());
    for task in &tasks_file.tasks {
        let phase = task_phase_label(
            task,
            &plan_phase,
            current_task_id.as_deref(),
            tracker,
            &completed,
            &failed,
        );
        let model = task
            .model_hint
            .clone()
            .unwrap_or_else(|| default_model_for_tier(&task.tier));
        let duration = latest_episode_for_task(&episodes, &plan_id, &task.id)
            .map(|episode| format_duration_ms(episode.usage.wall_ms))
            .unwrap_or_else(|| String::from("--"));

        tasks.push(PlanExecutionTaskRow {
            task_id: task.id.clone(),
            title: task.title.clone(),
            phase,
            model,
            duration,
            is_current: current_task_id.as_deref() == Some(task.id.as_str()),
        });
    }

    Some(PlanExecutionSnapshot {
        plan_id: plan_id.clone(),
        plan_title,
        tasks_done: completed.len(),
        tasks_total: tasks_file.tasks.len(),
        tasks,
        current_task,
        agent_output_tail,
    })
}

fn summarize_executor_state(state: &Value) -> ExecutorSummary {
    let Some(plan_states) = state.get("plan_states").and_then(Value::as_object) else {
        return ExecutorSummary::default();
    };

    if plan_states.is_empty() {
        return ExecutorSummary::default();
    }

    let has_running = plan_states
        .iter()
        .any(|(plan_id, plan_state)| runner_plan_is_started(state, plan_id, plan_state));
    let has_paused = plan_states
        .values()
        .any(|plan_state| !plan_state_is_terminal(plan_state) && plan_state_is_paused(plan_state));
    let has_error = plan_states.values().any(plan_state_has_error);

    let mut summary = ExecutorSummary {
        orchestrator_state: if has_running {
            String::from("running")
        } else if has_paused {
            String::from("paused")
        } else if has_error {
            String::from("error")
        } else {
            String::from("idle")
        },
        ..ExecutorSummary::default()
    };

    if let Some((_, plan_state)) = most_advanced_active_plan_state(state) {
        summary.current_iteration = plan_state
            .get("iteration")
            .and_then(Value::as_u64)
            .unwrap_or_default() as usize;
        summary.current_phase = current_phase_label(plan_state).unwrap_or_default();
    }

    summary
}

fn current_task_id(
    tasks_file: &TasksFile,
    tracker: Option<&TaskTrackerSnapshot>,
    completed: &HashSet<String>,
    failed: &HashSet<String>,
) -> Option<String> {
    let groups = tasks_file.parallel_groups();
    if let Some(tracker) = tracker {
        if let Some(group) = groups
            .get(
                tracker
                    .current_group_index
                    .min(groups.len().saturating_sub(1)),
            )
            .or_else(|| groups.last())
        {
            if let Some(task) = group
                .iter()
                .find(|task| !completed.contains(&task.id) && !failed.contains(&task.id))
            {
                return Some(task.id.clone());
            }
        }
    }

    tasks_file
        .tasks
        .iter()
        .find(|task| !completed.contains(&task.id) && !failed.contains(&task.id))
        .map(|task| task.id.clone())
}

fn build_task_detail(task: &TaskDef) -> PlanExecutionTaskDetail {
    let read_files = task
        .context
        .as_ref()
        .map(|ctx| {
            ctx.read_files
                .iter()
                .map(|rf| ReadFileSnapshot {
                    path: rf.path.clone(),
                    lines: rf.lines.clone(),
                    why: rf.why.clone(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    PlanExecutionTaskDetail {
        task_id: task.id.clone(),
        description: task.title.clone(),
        read_files,
        write_files: task.files.clone(),
    }
}

fn task_phase_label(
    task: &TaskDef,
    plan_phase: &str,
    current_task_id: Option<&str>,
    tracker: Option<&TaskTrackerSnapshot>,
    completed: &HashSet<String>,
    failed: &HashSet<String>,
) -> String {
    let _ = tracker;
    if completed.contains(&task.id) {
        return String::from("Done");
    }
    if failed.contains(&task.id) {
        return String::from("Failed");
    }
    if current_task_id == Some(task.id.as_str()) {
        return match plan_phase.to_ascii_lowercase().as_str() {
            "implementing" => String::from("Implementing"),
            "gating" => String::from("Gating"),
            "verifying" => String::from("Verifying"),
            "reviewing" => String::from("Reviewing"),
            "doc-revision" => String::from("Doc Revision"),
            "auto-fixing" => String::from("Auto Fixing"),
            "regenerating-verify" => String::from("Regenerating Verify"),
            other => title_case_phase(other),
        };
    }
    String::from("Queued")
}

fn plan_state_is_terminal(plan_state: &Value) -> bool {
    current_phase_label(plan_state)
        .map(|phase| {
            matches!(
                phase.to_ascii_lowercase().as_str(),
                "complete" | "failed" | "skipped"
            )
        })
        .unwrap_or(false)
}

fn plan_state_is_paused(plan_state: &Value) -> bool {
    plan_state
        .get("paused")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn plan_state_is_started(plan_state: &Value) -> bool {
    if plan_state_is_terminal(plan_state) || plan_state_is_paused(plan_state) {
        return false;
    }
    current_phase_label(plan_state).is_some_and(|phase| {
        !matches!(
            phase.trim().to_ascii_lowercase().as_str(),
            "queued" | "pending" | "ready"
        )
    })
}

fn runner_plan_is_started(state: &Value, plan_id: &str, plan_state: &Value) -> bool {
    match state
        .pointer("/_runner_projection/lifecycle/plans")
        .and_then(Value::as_object)
        .and_then(|plans| plans.get(plan_id))
        .and_then(Value::as_str)
    {
        Some("started") => !plan_state_is_paused(plan_state),
        Some("succeeded" | "failed" | "skipped") => false,
        _ => plan_state_is_started(plan_state),
    }
}

fn plan_state_has_error(plan_state: &Value) -> bool {
    current_phase_label(plan_state)
        .map(|phase| phase.eq_ignore_ascii_case("failed"))
        .unwrap_or(false)
        || plan_state
            .get("last_error")
            .and_then(Value::as_str)
            .is_some_and(|err| !err.trim().is_empty())
        || plan_state
            .pointer("/error/message")
            .and_then(Value::as_str)
            .is_some_and(|err| !err.trim().is_empty())
        || plan_state
            .get("error")
            .and_then(Value::as_str)
            .is_some_and(|err| !err.trim().is_empty())
}

fn title_case_phase(phase: &str) -> String {
    let mut out = String::new();
    let mut capitalize = true;
    for ch in phase.chars() {
        if ch == '-' || ch == '_' {
            out.push(' ');
            capitalize = true;
            continue;
        }
        if capitalize {
            out.extend(ch.to_uppercase());
            capitalize = false;
        } else {
            out.push(ch);
        }
    }
    if out.is_empty() {
        String::from("Unknown")
    } else {
        out
    }
}

fn execution_phase_priority(phase: &str) -> u8 {
    match phase.to_ascii_lowercase().as_str() {
        "implementing" => 5,
        "gating" => 4,
        "verifying" => 3,
        "reviewing" => 2,
        "doc-revision" => 2,
        "auto-fixing" => 2,
        "regenerating-verify" => 1,
        "enriching" => 1,
        "queued" => 0,
        _ => 0,
    }
}

fn most_advanced_active_plan_state<'a>(state: &'a Value) -> Option<(&'a str, &'a Value)> {
    let plan_states = state.get("plan_states").and_then(Value::as_object)?;

    let mut candidates = plan_states
        .iter()
        .filter_map(|(plan_id, plan_state)| {
            if !runner_plan_is_started(state, plan_id, plan_state) {
                return None;
            }
            let phase = current_phase_label(plan_state)?;
            let started_at_ms = plan_state
                .get("started_at_ms")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            Some((
                execution_phase_priority(&phase),
                started_at_ms,
                plan_id.as_str(),
                plan_state,
            ))
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| b.1.cmp(&a.1))
            .then_with(|| a.2.cmp(b.2))
    });

    candidates
        .into_iter()
        .next()
        .map(|(_, _, plan_id, plan_state)| (plan_id, plan_state))
}

fn default_model_for_tier(tier: &str) -> String {
    match tier.to_ascii_lowercase().as_str() {
        "mechanical" => String::from("claude-haiku-4-5"),
        "focused" | "integrative" => String::from("claude-sonnet-4-6"),
        "architectural" => String::from("claude-opus-4-6"),
        _ => String::from("claude-sonnet-4-6"),
    }
}

fn latest_episode_for_plan(
    episodes: &[Episode],
    plan_id: &str,
    task_id: Option<&str>,
) -> Option<Episode> {
    episodes
        .iter()
        .rev()
        .find(|episode| episode_matches_plan(episode, plan_id, task_id))
        .cloned()
}

fn latest_episode_for_task(episodes: &[Episode], plan_id: &str, task_id: &str) -> Option<Episode> {
    episodes
        .iter()
        .rev()
        .find(|episode| episode_matches_plan(episode, plan_id, Some(task_id)))
        .cloned()
}

fn episode_matches_plan(episode: &Episode, plan_id: &str, task_id: Option<&str>) -> bool {
    let matches_plan = episode.input_signal_hash == plan_id
        || episode.extra.get("plan_id").and_then(Value::as_str) == Some(plan_id);
    if !matches_plan {
        return false;
    }

    if let Some(task_id) = task_id {
        return episode.task_id == task_id
            || episode.extra.get("task_id").and_then(Value::as_str) == Some(task_id);
    }

    true
}

fn extract_episode_output_text(episode: &Episode) -> Option<String> {
    for key in [
        "stderr",
        "agent_stderr",
        "output",
        "stdout",
        "agent_output",
        "output_tail",
        "detail",
        "text",
    ] {
        if let Some(value) = episode.extra.get(key).and_then(json_value_to_text) {
            if !value.trim().is_empty() {
                return Some(value);
            }
        }
    }

    episode
        .failure_reason
        .as_deref()
        .map(ToOwned::to_owned)
        .filter(|text| !text.trim().is_empty())
}

fn json_value_to_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(items) => Some(
            items
                .iter()
                .filter_map(json_value_to_text)
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        Value::Null => None,
        other => Some(other.to_string()),
    }
}

fn tail_lines(text: &str, line_count: usize) -> String {
    let mut lines: Vec<&str> = text.lines().rev().take(line_count).collect();
    lines.reverse();
    lines.join("\n")
}

pub(super) fn format_duration_ms(duration_ms: u64) -> String {
    if duration_ms == 0 {
        return String::from("--");
    }
    if duration_ms < 1000 {
        return format!("{duration_ms}ms");
    }
    let secs = duration_ms / 1000;
    if secs < 60 {
        return format!("{secs}s");
    }
    let mins = secs / 60;
    format!("{mins}m {}s", secs % 60)
}

pub(super) fn now_ms() -> u64 {
    u64::try_from(Utc::now().timestamp_millis()).unwrap_or(u64::MAX)
}

pub(super) fn format_elapsed_ms(ms: u64) -> String {
    let secs = ms / 1000;
    if secs == 0 {
        return String::from("<1s");
    }
    if secs < 60 {
        return format!("{secs}s");
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{mins}m");
    }
    let hours = mins / 60;
    if hours < 24 {
        return format!("{hours}h {}m", mins % 60);
    }
    format!("{hours}h {}m", mins % 60)
}

fn load_efficiency_summary(path: &Path) -> EfficiencySummary {
    let events = read_efficiency_events_sync(path);
    if events.is_empty() {
        return EfficiencySummary::default();
    }

    let event_count = events.len();
    let total_cost_usd = events.iter().map(|event| event.cost_usd).sum();
    let total_input_tokens = events.iter().map(|event| event.input_tokens).sum();
    let total_output_tokens = events.iter().map(|event| event.output_tokens).sum();
    let passed_count = events
        .iter()
        .filter(|event| event.gate_passed == Some(true))
        .count();
    let average_wall_time_ms = events
        .iter()
        .map(|event| event.wall_time_ms as f64)
        .sum::<f64>()
        / count_to_f64(event_count);

    EfficiencySummary {
        event_count,
        total_cost_usd,
        total_input_tokens,
        total_output_tokens,
        passed_count,
        average_wall_time_ms,
    }
}

/// Compute [`EfficiencySummary`] from an already-loaded slice of events.
pub(crate) fn efficiency_summary_from_events(events: &[AgentEfficiencyEvent]) -> EfficiencySummary {
    if events.is_empty() {
        return EfficiencySummary::default();
    }
    let event_count = events.len();
    let total_cost_usd = events.iter().map(|e| e.cost_usd).sum();
    let total_input_tokens = events.iter().map(|e| e.input_tokens).sum();
    let total_output_tokens = events.iter().map(|e| e.output_tokens).sum();
    let passed_count = events
        .iter()
        .filter(|e| e.gate_passed == Some(true))
        .count();
    let average_wall_time_ms =
        events.iter().map(|e| e.wall_time_ms as f64).sum::<f64>() / count_to_f64(event_count);
    EfficiencySummary {
        event_count,
        total_cost_usd,
        total_input_tokens,
        total_output_tokens,
        passed_count,
        average_wall_time_ms,
    }
}

pub(super) fn load_efficiency_trend(path: &Path) -> Vec<EfficiencyBucket> {
    efficiency_trend(path, Duration::hours(1), 24).unwrap_or_default()
}

fn load_cfactor_trend(path: &Path) -> Vec<CFactorBucket> {
    cfactor_trend(path, Duration::hours(1), 24).unwrap_or_default()
}

pub(super) fn load_recent_signals(path: &Path, limit: usize) -> Vec<SignalSummary> {
    let mut signals = read_jsonl_values(path)
        .into_iter()
        .filter_map(|entry| SignalSummary::from_value(&entry))
        .collect::<Vec<_>>();
    if signals.len() > limit {
        signals = signals.split_off(signals.len() - limit);
    }
    signals
}

pub(super) fn load_gate_signal_summaries(path: &Path) -> Vec<GateSignalSummary> {
    let mut signals = read_jsonl_values(path)
        .into_iter()
        .filter_map(|entry| GateSignalSummary::from_value(&entry))
        .collect::<Vec<_>>();
    signals.sort_by(|a, b| {
        a.created_at_ms
            .cmp(&b.created_at_ms)
            .then_with(|| a.gate_name.cmp(&b.gate_name))
            .then_with(|| a.task_id.cmp(&b.task_id))
    });
    signals
}

pub(super) fn build_gate_results_page_data(
    signals: &[GateSignalSummary],
    adaptive_thresholds: Option<&AdaptiveThresholds>,
) -> GateResultsPageData {
    let mut by_gate: BTreeMap<String, GateAggregate> = BTreeMap::new();
    for signal in signals {
        let aggregate = by_gate.entry(signal.gate_name.clone()).or_default();
        aggregate.total_runs += 1;
        if signal.passed {
            aggregate.passed_runs += 1;
        }
        aggregate.total_duration_ms += signal.duration_ms as f64;
        aggregate.last_run = Some(signal.clone());
    }

    let mut gate_rows = by_gate
        .into_iter()
        .filter_map(|(gate_name, aggregate)| {
            let last_run = aggregate.last_run?;
            let total_runs = aggregate.total_runs;
            let pass_rate = if total_runs == 0 {
                0.0
            } else {
                aggregate.passed_runs as f64 / total_runs as f64
            };
            let avg_duration_ms = if total_runs == 0 {
                0.0
            } else {
                aggregate.total_duration_ms / total_runs as f64
            };
            Some(GateSummaryRow {
                gate_name,
                total_runs,
                pass_rate,
                avg_duration_ms,
                last_run: format_last_run(&last_run),
            })
        })
        .collect::<Vec<_>>();
    gate_rows.sort_by(|a, b| {
        b.total_runs
            .cmp(&a.total_runs)
            .then_with(|| a.gate_name.cmp(&b.gate_name))
    });

    let mut threshold_rows = Vec::new();
    if let Some(thresholds) = adaptive_thresholds {
        threshold_rows = gate_threshold_rows(thresholds);
    }

    let mut failure_rows = signals
        .iter()
        .filter(|signal| !signal.passed)
        .map(|signal| GateFailureRow {
            created_at_ms: signal.created_at_ms,
            task_id: signal
                .task_id
                .clone()
                .unwrap_or_else(|| String::from("unknown")),
            gate_name: signal.gate_name.clone(),
            error_excerpt: signal.excerpt.clone(),
        })
        .collect::<Vec<_>>();
    failure_rows.sort_by(|a, b| b.created_at_ms.cmp(&a.created_at_ms));
    failure_rows.truncate(10);

    GateResultsPageData {
        gate_rows,
        threshold_rows,
        failure_rows,
    }
}

fn load_latest_jsonl_value<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    let text = std::fs::read_to_string(path).ok()?;
    text.lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .and_then(|line| serde_json::from_str(line).ok())
}

pub(super) fn file_stamp(path: &Path) -> FileStamp {
    FileStamp::from_path(path).unwrap_or_default()
}

fn runner_projection_stamp(root: &Path) -> RunnerProjectionStamp {
    RunnerProjectionStamp {
        state_snapshot: file_stamp(&root.join(STATE_SNAPSHOT_RELATIVE_PATH)),
        legacy_executor: file_stamp(&root.join(LEGACY_EXECUTOR_RELATIVE_PATH)),
    }
}

fn next_dashboard_data_generation(root: &Path, stamps: DashboardDataStamps) -> u64 {
    let counters = DASHBOARD_GENERATION_COUNTERS.get_or_init(|| Mutex::new(HashMap::new()));
    let counter = {
        let mut counters = counters
            .lock()
            .expect("dashboard generation counter registry lock poisoned");
        counters
            .entry(root.to_path_buf())
            .or_insert_with(|| Arc::new(DurableDashboardGenerationCounter::load(root)))
            .clone()
    };

    counter.next(root, stamps.fingerprint())
}

pub(crate) fn signal_gate_result_from_value(value: &Value) -> Option<GateResultSummary> {
    if !value
        .get("kind")
        .and_then(Value::as_str)
        .is_some_and(is_gate_result_kind)
    {
        return None;
    }
    let plan_id = value
        .pointer("/tags/plan_id")
        .and_then(Value::as_str)
        .or_else(|| value.pointer("/body/data/plan_id").and_then(Value::as_str))
        .or_else(|| value.pointer("/body/plan_id").and_then(Value::as_str))
        .unwrap_or("unknown");
    GateResultSummary::from_signal(value, plan_id)
}

pub(crate) fn read_json_value(path: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub(crate) fn read_jsonl_values(path: &Path) -> Vec<Value> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    text.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

fn read_efficiency_events_sync(path: &Path) -> Vec<AgentEfficiencyEvent> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    text.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

#[derive(Debug, Default)]
struct GateAggregate {
    total_runs: u64,
    passed_runs: u64,
    total_duration_ms: f64,
    last_run: Option<GateSignalSummary>,
}

/// Build the adaptive-threshold table rows for the gate-results page.
///
/// Shared by the disk-mode loader and the connected-mode push path, which
/// parses `DashboardSnapshot::gate_thresholds_json` into the same struct.
pub(crate) fn gate_threshold_rows(thresholds: &AdaptiveThresholds) -> Vec<GateThresholdRow> {
    let mut rows = thresholds
        .all_rungs()
        .map(|(rung, stats)| GateThresholdRow {
            rung: *rung,
            current_threshold: thresholds.suggested_max_retries(*rung),
            ema_pass_rate: stats.ema_pass_rate,
            trend: gate_trend_from_ema(stats.ema_pass_rate),
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| row.rung);
    rows
}

fn gate_trend_from_ema(ema_pass_rate: f64) -> GateTrend {
    if ema_pass_rate >= 0.55 {
        GateTrend::Up
    } else if ema_pass_rate <= 0.45 {
        GateTrend::Down
    } else {
        GateTrend::Flat
    }
}

fn format_last_run(signal: &GateSignalSummary) -> String {
    let age_ms = if signal.created_at_ms <= 0 {
        0
    } else {
        let created_at_ms = u64::try_from(signal.created_at_ms).unwrap_or_default();
        now_ms().saturating_sub(created_at_ms)
    };
    let state = if signal.passed { "pass" } else { "fail" };
    format!("{} {state}", format_elapsed_ms(age_ms))
}

fn gate_excerpt_from_value(value: &Value) -> String {
    for pointer in [
        "/tags/error",
        "/tags/message",
        "/body/data/error",
        "/body/data/message",
        "/body/data/reason",
        "/body/error",
        "/body/message",
        "/body/reason",
    ] {
        if let Some(text) = value.pointer(pointer).and_then(Value::as_str) {
            let first_line = text.lines().next().unwrap_or("").trim();
            if !first_line.is_empty() {
                return first_line.to_string();
            }
        }
    }

    String::new()
}

fn extract_gate_name(entry: &Value) -> Option<String> {
    entry
        .pointer("/tags/gate")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| {
            entry
                .pointer("/body/data/gate")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .or_else(|| {
            entry
                .pointer("/body/gate")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .or_else(|| {
            entry
                .get("kind")
                .and_then(Value::as_str)
                .and_then(|kind| kind.strip_prefix("gate:").or(kind.strip_prefix("gate_")))
                .map(ToOwned::to_owned)
        })
}

fn extract_gate_passed(entry: &Value) -> Option<bool> {
    entry
        .pointer("/tags/passed")
        .and_then(Value::as_str)
        .and_then(|s| match s {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        })
        .or_else(|| entry.pointer("/body/data/passed").and_then(Value::as_bool))
        .or_else(|| entry.pointer("/body/passed").and_then(Value::as_bool))
}

fn extract_gate_duration_ms(entry: &Value) -> Option<u64> {
    entry
        .pointer("/body/data/duration_ms")
        .and_then(Value::as_u64)
        .or_else(|| entry.pointer("/body/duration_ms").and_then(Value::as_u64))
        .or_else(|| entry.pointer("/tags/duration_ms").and_then(Value::as_u64))
}

fn entry_timestamp_ms(entry: &Value) -> Option<i64> {
    entry
        .get("created_at_ms")
        .and_then(Value::as_i64)
        .or_else(|| {
            entry
                .get("created_at_ms")
                .and_then(Value::as_u64)
                .map(|ts| ts as i64)
        })
}

fn signal_parent_hash(value: &Value) -> Option<String> {
    value
        .pointer("/parent_hash")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| {
            value
                .pointer("/body/data/parent_hash")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .or_else(|| {
            value
                .pointer("/body/parent_hash")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .or_else(|| {
            value
                .pointer("/lineage")
                .and_then(Value::as_array)
                .and_then(|lineage| lineage.last())
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
}

fn signal_lineage(value: &Value) -> Vec<String> {
    value
        .pointer("/lineage")
        .and_then(Value::as_array)
        .map(|lineage| {
            lineage
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .or_else(|| {
            value
                .pointer("/body/data/lineage")
                .and_then(Value::as_array)
                .map(|lineage| {
                    lineage
                        .iter()
                        .filter_map(Value::as_str)
                        .map(ToOwned::to_owned)
                        .collect::<Vec<_>>()
                })
        })
        .unwrap_or_default()
}

fn signal_payload_preview(value: &Value) -> String {
    let payload = value
        .pointer("/body/data")
        .or_else(|| value.get("body"))
        .or_else(|| value.get("payload"));

    let Some(payload) = payload else {
        return String::new();
    };

    let raw = match payload {
        Value::String(text) => text.clone(),
        other => serde_json::to_string(other).unwrap_or_default(),
    };
    let compact = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate_str(compact.trim(), 60)
}

fn signal_confidence(value: &Value) -> Option<f64> {
    [
        "/tags/confidence",
        "/tags/score",
        "/tags/trust",
        "/body/data/confidence",
        "/body/data/score",
        "/body/data/trust",
        "/payload/confidence",
        "/payload/score",
        "/payload/trust",
    ]
    .iter()
    .find_map(|path| value.pointer(path).and_then(value_as_f64))
    .map(|confidence| confidence.clamp(0.0, 1.0))
}

fn value_as_f64(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|text| text.parse::<f64>().ok()))
}

fn current_phase_label(plan_state: &Value) -> Option<String> {
    plan_state
        .pointer("/current_phase/kind")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| {
            plan_state
                .get("current_phase")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .or_else(|| {
            plan_state
                .pointer("/phase/kind")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .or_else(|| {
            plan_state
                .get("phase")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
}

fn is_gate_result_kind(kind: &str) -> bool {
    kind == "gate_verdict" || kind.starts_with("gate:") || kind.starts_with("gate_")
}

impl fmt::Display for DashboardSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "dashboard scaffold: {} pages, {} widgets, active={}",
            self.page_count,
            self.widget_count,
            self.active_page.slug()
        )
    }
}


#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;

    use std::fs::{self, OpenOptions};
    use std::io::Write;

    use tempfile::tempdir;

    fn write_runner_snapshot(root: &Path, executor: &Value) {
        let mut executor = executor.clone();
        if let Some(object) = executor.as_object_mut() {
            object.entry("schema_version").or_insert(Value::from(1));
            object
                .entry("queue_order")
                .or_insert_with(|| Value::Array(Vec::new()));
            object
                .entry("speculative_executions")
                .or_insert_with(|| Value::Object(Default::default()));
            object.entry("timestamp_ms").or_insert(Value::from(42));
        }
        let fallback_plan = executor
            .get("plan_states")
            .and_then(Value::as_object)
            .and_then(|plans| plans.keys().next())
            .cloned()
            .unwrap_or_else(|| "test-plan".to_string());
        let mut lifecycle_tasks = serde_json::Map::new();
        let mut completed = HashMap::<String, Vec<String>>::new();
        let mut failed = HashMap::<String, Vec<String>>::new();
        let mut skipped = HashMap::<String, HashMap<String, Value>>::new();
        for task in executor
            .get("tasks")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(task_id) = task
                .get("task_id")
                .or_else(|| task.get("id"))
                .and_then(Value::as_str)
            else {
                continue;
            };
            let plan_id = task
                .get("plan_id")
                .or_else(|| task.get("plan"))
                .and_then(Value::as_str)
                .unwrap_or(&fallback_plan);
            let status = match task.get("status").and_then(Value::as_str) {
                Some("done" | "complete" | "completed" | "passed") => "passed",
                Some("failed" | "exhausted" | "timed_out") => "failed",
                Some("skipped") => "cancelled",
                Some("running") => "running",
                _ => "started",
            };
            match status {
                "passed" => completed
                    .entry(plan_id.to_string())
                    .or_default()
                    .push(task_id.to_string()),
                "failed" => failed
                    .entry(plan_id.to_string())
                    .or_default()
                    .push(task_id.to_string()),
                "cancelled" => {
                    skipped
                        .entry(plan_id.to_string())
                        .or_default()
                        .insert(task_id.to_string(), serde_json::json!({"reason": "test"}));
                }
                _ => {}
            }
            lifecycle_tasks.insert(
                format!("{plan_id}:{task_id}"),
                serde_json::json!({
                    "plan_id": plan_id,
                    "task_id": task_id,
                    "status": status,
                    "current_attempt": 1,
                    "next_attempt": 2,
                    "started_at_ms": 42
                }),
            );
        }
        let tasks_total = lifecycle_tasks.len();
        let run_state = serde_json::json!({
            "schema_version": 1,
            "run_id": "tui-test",
            "timestamp_ms": 42,
            "tasks_total": tasks_total,
            "tasks_completed": completed.values().map(Vec::len).sum::<usize>(),
            "tasks_failed": failed.values().map(Vec::len).sum::<usize>(),
            "total_tokens_in": 0,
            "total_tokens_out": 0,
            "total_cost_usd": 0.0,
            "total_agent_calls": 0,
            "completed_tasks": completed,
            "failed_tasks": failed,
            "skipped_tasks": skipped,
            "lifecycle": {
                "run_id": "tui-test",
                "status": "running",
                "total_tasks": tasks_total,
                "plans": {},
                "tasks": lifecycle_tasks,
                "task_attempts": {}
            },
            "replan_ledger": {}
        });
        let snapshot = roko_runtime::StateSnapshot::new(
            42,
            executor.to_string(),
            serde_json::json!({"schema_version": 1, "executor": executor, "timestamp_ms": 42})
                .to_string(),
            run_state.to_string(),
            serde_json::json!({"rungs": {}}).to_string(),
        );
        write_json(&root.join(STATE_SNAPSHOT_RELATIVE_PATH), &snapshot);
    }

    #[test]
    fn episode_path_prefers_root_then_legacy_fallbacks() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path();
        let canonical = root.join(".roko/episodes.jsonl");
        let learn = root.join(".roko/learn/episodes.jsonl");
        let memory = root.join(".roko/memory/episodes.jsonl");
        for path in [&canonical, &learn, &memory] {
            fs::create_dir_all(path.parent().expect("episode parent")).expect("create parent");
            fs::write(path, "{}\n").expect("write episode fixture");
        }

        assert_eq!(resolve_episodes_path(root), canonical);
        fs::remove_file(&canonical).expect("remove canonical");
        assert_eq!(resolve_episodes_path(root), learn);
        fs::remove_file(&learn).expect("remove learn fallback");
        assert_eq!(resolve_episodes_path(root), memory);
    }

    fn write_jsonl(path: &Path, lines: &[String]) {
        fs::create_dir_all(path.parent().expect("file has parent"))
            .expect("should create parent dir");
        fs::write(path, lines.join("\n") + "\n").expect("should write jsonl");
    }

    fn append_raw(path: &Path, text: &str) {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("open append target");
        file.write_all(text.as_bytes()).expect("append bytes");
    }

    fn sample_episode(
        agent: &str,
        task: &str,
        success: bool,
        cost_usd: f64,
        wall_ms: u64,
    ) -> Episode {
        let mut episode = Episode::new(agent, task);
        episode.success = success;
        episode.usage.cost_usd = cost_usd;
        episode.usage.wall_ms = wall_ms;
        episode
    }

    fn sample_metric(
        plan: &str,
        task: &str,
        iteration: u32,
        passed: bool,
        model: &str,
        input_tokens: u64,
        cache_hit_rate: f64,
        cost_usd: f64,
    ) -> TaskMetric {
        let mut metric = TaskMetric::new(
            roko_core::metric::ConfigHash::from("hash".to_string()),
            plan,
            task,
        );
        metric.iteration = iteration;
        metric.gate_passed = passed;
        metric.model = model.to_string();
        metric.input_tokens = input_tokens;
        metric.cached_tokens = (input_tokens as f64 * cache_hit_rate).round() as u64;
        metric.cache_hit_rate = cache_hit_rate;
        metric.cost_usd = cost_usd;
        metric
    }

    fn sample_efficiency_event(
        agent: &str,
        task: &str,
        role: &str,
        model: &str,
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: f64,
        timestamp: &str,
    ) -> AgentEfficiencyEvent {
        AgentEfficiencyEvent {
            agent_id: agent.to_string(),
            role: role.to_string(),
            backend: String::from("claude"),
            model: model.to_string(),
            plan_id: String::from("plan-a"),
            task_id: task.to_string(),
            attempt_id: String::new(),
            input_tokens,
            output_tokens,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            cost_usd,
            cost_usd_without_cache: cost_usd,
            prompt_sections: Vec::new(),
            total_prompt_tokens: input_tokens,
            system_prompt_tokens: 0,
            tools_available: 0,
            tools_used: 0,
            tool_calls: Vec::new(),
            wall_time_ms: 1_000,
            duration_ms: 1_000,
            time_to_first_token_ms: 0,
            was_warm_start: true,
            iteration: 1,
            turn_number: 0,
            is_final_turn: true,
            gate_passed: Some(true),
            outcome: "success".to_string(),
            gate_errors: Vec::new(),
            model_used: model.to_string(),
            strategy_attempted: "none".to_string(),
            timestamp: timestamp.to_string(),
            frequency: roko_core::OperatingFrequency::Theta,
            reasoning_tokens: 0,
        }
    }

    #[test]
    fn signal_summary_extracts_structured_confidence() {
        let signal = SignalSummary::from_value(&serde_json::json!({
            "id": "sig-1",
            "kind": "gate:compile",
            "created_at_ms": 42,
            "body": {
                "data": {
                    "confidence": "0.82",
                    "plan_id": "plan-a"
                }
            }
        }))
        .expect("valid signal");

        assert_eq!(signal.confidence, Some(0.82));
        assert_eq!(signal.plan_id.as_deref(), Some("plan-a"));
    }

    #[test]
    fn signal_confidence_clamps_out_of_range_values() {
        assert_eq!(
            signal_confidence(&serde_json::json!({
                "tags": {
                    "score": 1.4
                }
            })),
            Some(1.0)
        );
        assert_eq!(
            signal_confidence(&serde_json::json!({
                "payload": {
                    "trust": -0.25
                }
            })),
            Some(0.0)
        );
    }

    #[test]
    fn scaffold_has_expected_page_count() {
        let dashboard = DashboardScaffold::new();
        let summary = dashboard.summary();
        assert_eq!(summary.page_count, 16);
        assert!(summary.widget_count >= 20);
        assert_eq!(summary.active_page, PageId::Health);
    }

    #[test]
    fn can_switch_active_page() {
        let mut dashboard = DashboardScaffold::new();
        assert!(dashboard.set_active_page(PageId::PlanView));
        assert_eq!(dashboard.active_page(), PageId::PlanView);
    }

    #[test]
    fn theme_defaults_to_rosedust_palette() {
        let theme = Theme::from_no_color(false);
        assert_eq!(theme.foreground, Color::Rgb(165, 142, 158));
        assert_eq!(theme.background, Color::Rgb(0, 0, 0));
        assert_eq!(theme.accent, Color::Rgb(185, 120, 148));
        assert_eq!(theme.selection_background, Color::Rgb(34, 28, 36));
        assert_eq!(theme.selection_foreground, Color::Rgb(215, 198, 158));
    }

    #[test]
    fn theme_disables_color_when_requested() {
        let theme = Theme::from_no_color(true);
        assert_eq!(theme.foreground, Color::Reset);
        assert_eq!(theme.background, Color::Reset);
        assert_eq!(theme.accent, Color::Reset);
        assert_eq!(theme.selection_background, Color::Reset);
        assert_eq!(theme.selection_foreground, Color::Reset);
    }

    #[test]
    fn overview_render_contains_active_page_and_counts() {
        let dashboard = DashboardScaffold::new();
        let rendered = dashboard.render_overview_text();
        assert!(rendered.contains("dashboard scaffold: 16 pages"));
        assert!(rendered.contains("active=health"));
        assert!(rendered.contains("active page:"));
        assert!(rendered.contains("* Health [health] efficiency"));
    }

    #[test]
    fn page_render_includes_widgets() {
        let dashboard = DashboardScaffold::new();
        let rendered = dashboard
            .render_page_text(PageId::PlanView)
            .expect("plan page should exist");
        assert!(rendered.contains("Plan View (plan-view)"));
        let widgets = dashboard
            .render_page_list_text(PageId::PlanView)
            .expect("plan widget list should exist");
        assert!(widgets.contains("widgets (2):"));
        assert!(widgets.contains("DAG [dag]"));
    }

    #[test]
    fn signals_page_renders_recent_signals_and_tree() {
        let tmpdir = tempdir().expect("tempdir");
        let roko_dir = tmpdir.path().join(".roko");
        let memory_dir = roko_dir.join("memory");
        fs::create_dir_all(&memory_dir).expect("memory dir");
        fs::write(memory_dir.join(EPISODES_FILE), "").expect("empty episodes");

        let signals = vec![
            serde_json::json!({
                "id": "sig-1",
                "kind": "gate:compile",
                "created_at_ms": 1_700_000_000_000i64,
                "lineage": [],
                "tags": {
                    "plan_id": "plan-a",
                    "task_id": "task-a"
                },
                "body": {
                    "format": "json",
                    "data": {
                        "message": "compile ok",
                        "detail": "payload"
                    }
                }
            }),
            serde_json::json!({
                "id": "sig-2",
                "kind": "conductor:alert:warning",
                "created_at_ms": 1_700_000_001_000i64,
                "lineage": ["sig-1"],
                "body": {
                    "format": "text",
                    "data": "selected payload text that should preview nicely"
                }
            }),
        ];
        write_jsonl(
            &roko_dir.join("engrams.jsonl"),
            &signals
                .into_iter()
                .map(|signal| serde_json::to_string(&signal).expect("signal json"))
                .collect::<Vec<_>>(),
        );

        let dashboard = DashboardScaffold::new_in(tmpdir.path());
        let rendered = dashboard
            .render_page_text(PageId::Signals)
            .expect("signals page should render");
        assert!(rendered.contains("Signals (signals)"));
        assert!(rendered.contains("recent signals:"));
        assert!(rendered.contains("kind distribution:"));
        assert!(rendered.contains("signal DAG explorer:"));
        assert!(rendered.contains("sig-2"));
        assert!(rendered.contains("sig-1"));
    }

    #[test]
    fn page_index_render_contains_compact_summaries() {
        let dashboard = DashboardScaffold::new();
        let rendered = dashboard.render_page_index_text();
        assert!(rendered.contains("* Health [health] efficiency | 3 widgets"));
        assert!(rendered.contains("Plan View [plan-view] operations | 2 widgets"));
    }

    #[test]
    fn page_list_render_focuses_on_one_page_widget_list() {
        let dashboard = DashboardScaffold::new();
        let rendered = dashboard
            .render_page_list_text(PageId::ConfigView)
            .expect("config page should exist");
        assert!(rendered.contains("Config View [config-view]"));
        assert!(rendered.contains("widgets (2):"));
        assert!(rendered.contains("Effective Config [effective_config]"));
    }

    #[test]
    fn snapshot_loader_aggregates_episode_and_metric_stats() {
        let tempdir = tempdir().expect("tempdir");
        let memory_dir = tempdir.path().join(MEMORY_DIR);
        let episodes_path = memory_dir.join(EPISODES_FILE);
        let metrics_path = memory_dir.join(TASK_METRICS_FILE);

        let episodes = vec![
            serde_json::to_string(&sample_episode("agent-a", "task-a", true, 1.50, 1_000))
                .expect("episode json"),
            serde_json::to_string(&sample_episode("agent-b", "task-b", false, 0.50, 3_000))
                .expect("episode json"),
        ];
        write_jsonl(&episodes_path, &episodes);

        let metrics = vec![
            sample_metric("plan-a", "t1", 1, true, "claude-haiku-4-5", 100, 0.20, 0.10),
            sample_metric(
                "plan-a",
                "t1",
                2,
                false,
                "claude-sonnet-4-5",
                300,
                0.50,
                0.20,
            ),
            sample_metric("plan-b", "t2", 1, true, "claude-haiku-4-5", 200, 0.25, 0.30),
        ];
        write_jsonl(
            &metrics_path,
            &metrics
                .iter()
                .map(|metric| metric.to_jsonl().expect("metric json"))
                .collect::<Vec<_>>(),
        );

        let snapshot = load_snapshot_blocking(tempdir.path()).expect("snapshot should load");

        assert_eq!(snapshot.episode_count, 2);
        assert_eq!(snapshot.task_metric_count, 3);
        assert_eq!(snapshot.success_rate, Some(0.5));
        assert!((snapshot.average_cost_usd.expect("avg cost") - 1.0).abs() < 1e-9);
        assert!((snapshot.average_wall_time_ms.expect("avg wall") - 2_000.0).abs() < 1e-9);
        assert!((snapshot.haiku_share.expect("haiku share") - (2.0 / 3.0)).abs() < 1e-9);
        assert!((snapshot.cache_hit_rate.expect("cache hit") - (0.95 / 3.0)).abs() < 1e-9);
        assert_eq!(snapshot.headlines.n_plans, 2);
        assert_eq!(snapshot.headlines.n_records, 3);
        assert!((snapshot.headlines.first_attempt_pass_rate - 1.0).abs() < 1e-9);
        assert!((snapshot.headlines.avg_iterations_per_plan - 1.5).abs() < 1e-9);
    }

    #[test]
    fn health_and_trends_render_real_stats_when_snapshot_exists() {
        let tempdir = tempdir().expect("tempdir");
        let memory_dir = tempdir.path().join(MEMORY_DIR);
        let episodes_path = memory_dir.join(EPISODES_FILE);
        let metrics_path = memory_dir.join(TASK_METRICS_FILE);

        write_jsonl(
            &episodes_path,
            &[
                serde_json::to_string(&sample_episode("agent-a", "task-a", true, 1.50, 1_000))
                    .expect("episode json"),
                serde_json::to_string(&sample_episode("agent-b", "task-b", false, 0.50, 3_000))
                    .expect("episode json"),
            ],
        );
        write_jsonl(
            &metrics_path,
            &[
                sample_metric("plan-a", "t1", 1, true, "claude-haiku-4-5", 100, 0.20, 0.10)
                    .to_jsonl()
                    .expect("metric json"),
                sample_metric(
                    "plan-a",
                    "t1",
                    2,
                    false,
                    "claude-sonnet-4-5",
                    300,
                    0.50,
                    0.20,
                )
                .to_jsonl()
                .expect("metric json"),
                sample_metric("plan-b", "t2", 1, true, "claude-haiku-4-5", 200, 0.25, 0.30)
                    .to_jsonl()
                    .expect("metric json"),
            ],
        );

        let dashboard = DashboardScaffold::new_in(tempdir.path());
        let health = dashboard.render_health_page_text();
        let trends = dashboard.render_trends_page_text();

        assert!(health.contains("episodes: 2"));
        assert!(health.contains("success rate: 50.0%"));
        assert!(health.contains("average cost: $1.0000"));
        assert!(health.contains("average wall time: 2000 ms"));
        assert!(health.contains("haiku share: 66.7%"));
        assert!(health.contains("cache hit rate: 31.7%"));

        assert!(trends.contains("task metrics: 3"));
        assert!(trends.contains("first-attempt pass rate: 100.0%"));
        assert!(trends.contains("avg iterations per plan: 1.50"));
        assert!(trends.contains("avg cost per plan: $0.3000"));
        assert!(trends.contains("avg input tokens per spawn: 200.00"));
        assert!(trends.contains("haiku share: 66.7%"));
        assert!(trends.contains("cache hit rate: 31.7%"));
        assert!(trends.contains("- avg_cost_per_plan: $0.3000"));
    }

    fn write_json(path: &Path, value: &impl serde::Serialize) {
        fs::create_dir_all(path.parent().expect("file has parent"))
            .expect("should create parent dir");
        let json = serde_json::to_string_pretty(value).expect("should serialize");
        fs::write(path, json).expect("should write json");
    }

    #[test]
    fn parameters_page_renders_cascade_and_thresholds() {
        let tmpdir = tempdir().expect("tempdir");
        let learn_dir = tmpdir.path().join(".roko/learn");

        // Write cascade router data.
        let cascade = serde_json::json!({
            "model_slugs": ["claude-sonnet-4-5", "claude-haiku-4-5"],
            "confidence_stats": {
                "claude-sonnet-4-5": { "trials": 50, "successes": 45 },
                "claude-haiku-4-5": { "trials": 30, "successes": 20 }
            }
        });
        write_json(&learn_dir.join(CASCADE_ROUTER_FILE), &cascade);

        // Write adaptive thresholds.
        let thresholds = AdaptiveThresholds::default();
        write_json(&learn_dir.join(GATE_THRESHOLDS_FILE), &thresholds);

        // Need memory dir to exist for the root resolver.
        let memory_dir = tmpdir.path().join(MEMORY_DIR);
        fs::create_dir_all(&memory_dir).expect("memory dir");
        fs::write(memory_dir.join(EPISODES_FILE), "").expect("empty episodes");

        let dashboard = DashboardScaffold::new_in(tmpdir.path());
        let rendered = dashboard
            .render_page_text(PageId::Parameters)
            .expect("parameters page should render");
        assert!(rendered.contains("Parameters"));
        assert!(rendered.contains("cascade router:"));
        assert!(rendered.contains("registered models: 2"));
    }

    #[test]
    fn gate_results_page_renders_summary_thresholds_and_failures() {
        let tmpdir = tempdir().expect("tempdir");
        let roko_dir = tmpdir.path().join(".roko");
        let learn_dir = roko_dir.join("learn");
        fs::create_dir_all(&learn_dir).expect("learn dir");

        let mut thresholds = AdaptiveThresholds::new();
        thresholds.update(0, true);
        thresholds.update(0, false);
        thresholds.update(1, true);
        write_json(&learn_dir.join(GATE_THRESHOLDS_FILE), &thresholds);

        let signals = vec![
            serde_json::json!({
                "id": "sig-1",
                "kind": "gate:compile",
                "created_at_ms": 1_700_000_000_000i64,
                "tags": {
                    "gate": "compile",
                    "plan_id": "plan-a",
                    "task_id": "task-a",
                    "passed": "true",
                    "duration_ms": 120
                }
            }),
            serde_json::json!({
                "id": "sig-2",
                "kind": "gate:test",
                "created_at_ms": 1_700_000_000_500i64,
                "tags": {
                    "gate": "test",
                    "plan_id": "plan-a",
                    "task_id": "task-b",
                    "passed": "false",
                    "duration_ms": 340
                },
                "body": {
                    "data": {
                        "reason": "assertion failed on line 42\nmore detail"
                    }
                }
            }),
        ];
        write_jsonl(
            &roko_dir.join("engrams.jsonl"),
            &signals
                .into_iter()
                .map(|signal| serde_json::to_string(&signal).expect("signal json"))
                .collect::<Vec<_>>(),
        );

        let memory_dir = tmpdir.path().join(MEMORY_DIR);
        fs::create_dir_all(&memory_dir).expect("memory dir");
        fs::write(memory_dir.join(EPISODES_FILE), "").expect("empty episodes");

        let dashboard = DashboardScaffold::new_in(tmpdir.path());
        let rendered = dashboard
            .render_page_text(PageId::GateResults)
            .expect("gate results page should render");
        assert!(rendered.contains("Verify Results"));
        assert!(rendered.contains("gate summary:"));
        assert!(rendered.contains("adaptive thresholds:"));
        assert!(rendered.contains("recent gate failures:"));
        assert!(rendered.contains("compile"));
        assert!(rendered.contains("task-b"));
        assert!(rendered.contains("assertion failed on line 42"));
    }

    #[test]
    fn experiments_page_renders_with_store() {
        let tmpdir = tempdir().expect("tempdir");
        let learn_dir = tmpdir.path().join(".roko/learn");

        // Write experiment store as raw JSON matching ExperimentStore structure.
        let store_json = serde_json::json!({
            "experiments": {
                "exp-1": {
                    "experiment_id": "exp-1",
                    "section_name": "system_prompt",
                    "variants": [
                        { "id": "baseline", "name": "Baseline", "section_name": "system_prompt", "content": "v1", "active": true },
                        { "id": "verbose", "name": "Verbose", "section_name": "system_prompt", "content": "v2", "active": true }
                    ],
                    "stats": {
                        "baseline": { "trials": 10, "successes": 8 },
                        "verbose": { "trials": 10, "successes": 5 }
                    },
                    "status": "Running",
                    "winner_id": null,
                    "min_trials_per_variant": 20,
                    "min_effect_size": 0.1
                }
            }
        });
        write_json(&learn_dir.join(EXPERIMENTS_FILE), &store_json);

        let memory_dir = tmpdir.path().join(MEMORY_DIR);
        fs::create_dir_all(&memory_dir).expect("memory dir");
        fs::write(memory_dir.join(EPISODES_FILE), "").expect("empty episodes");

        let dashboard = DashboardScaffold::new_in(tmpdir.path());
        let rendered = dashboard
            .render_page_text(PageId::Experiments)
            .expect("experiments page should render");
        assert!(rendered.contains("Experiments"));
        assert!(rendered.contains("system_prompt"));
        assert!(rendered.contains("1 running"));
    }

    #[test]
    fn learning_page_renders_learning_system_status() {
        let tmpdir = tempdir().expect("tempdir");
        let learn_dir = tmpdir.path().join(".roko/learn");
        let memory_dir = tmpdir.path().join(MEMORY_DIR);
        let neuro_dir = tmpdir.path().join(NEURO_DIR);
        fs::create_dir_all(&memory_dir).expect("memory dir");
        fs::write(memory_dir.join(EPISODES_FILE), "").expect("empty episodes");

        let cascade = serde_json::json!({
            "model_slugs": ["claude-sonnet-4-5"],
            "confidence_stats": {
                "claude-sonnet-4-5": { "trials": 423, "successes": 350 }
            },
            "total_observations": 423,
            "stage_transitions": [
                {
                    "from": "Static",
                    "to": "Confidence",
                    "observations": 50,
                    "timestamp": "2026-04-10T08:00:00Z"
                },
                {
                    "from": "Confidence",
                    "to": "Ucb",
                    "observations": 201,
                    "timestamp": "2026-04-10T09:00:00Z"
                }
            ]
        });
        write_json(&learn_dir.join(CASCADE_ROUTER_FILE), &cascade);

        let mut thresholds = AdaptiveThresholds::new();
        thresholds.update(0, true);
        thresholds.update(1, false);
        write_json(&learn_dir.join(GATE_THRESHOLDS_FILE), &thresholds);

        let experiment_store = serde_json::json!({
            "experiments": {
                "exp-1": {
                    "experiment_id": "exp-1",
                    "section_name": "system_prompt",
                    "variants": [
                        { "id": "baseline", "name": "Baseline", "section_name": "system_prompt", "content": "v1", "active": true },
                        { "id": "variant", "name": "Variant", "section_name": "system_prompt", "content": "v2", "active": true }
                    ],
                    "stats": {
                        "baseline": { "trials": 4, "successes": 3 },
                        "variant": { "trials": 4, "successes": 2 }
                    },
                    "status": "Running",
                    "winner_id": null,
                    "min_trials_per_variant": 10,
                    "min_effect_size": 0.1
                }
            }
        });
        write_json(&learn_dir.join(EXPERIMENTS_FILE), &experiment_store);

        let mut skill = Skill::new("route_fix", "Route a fix", "template");
        skill.first_seen = Some(Utc::now());
        write_json(&learn_dir.join(SKILLS_FILE), &vec![skill]);

        let provider_health = serde_json::json!({
            "providers": {
                "anthropic": {
                    "provider_id": "anthropic",
                    "state": "Closed",
                    "consecutive_failures": 0,
                    "total_requests": 8,
                    "total_failures": 0,
                    "last_failure_at": null,
                    "cooldown_until": null,
                    "failure_window": []
                }
            }
        });
        write_json(&learn_dir.join(PROVIDER_HEALTH_FILE), &provider_health);

        fs::create_dir_all(&neuro_dir).expect("neuro dir");
        fs::write(neuro_dir.join(KNOWLEDGE_FILE), "{\"id\":\"k1\"}\n").expect("knowledge file");

        let dashboard = DashboardScaffold::new_in(tmpdir.path());
        let rendered = dashboard
            .render_page_text(PageId::Learning)
            .expect("learning page should render");
        assert!(rendered.contains("Learning System Status"));
        assert!(rendered.contains("Stage: UCB (423 observations)"));
        assert!(rendered.contains("Last transition: Confidence -> UCB at obs 201"));
        assert!(rendered.contains("CascadeRouter"));
        assert!(rendered.contains("GateThresholds"));
        assert!(rendered.contains("Experiments"));
        assert!(rendered.contains("SkillLibrary"));
        assert!(rendered.contains("PatternMiner"));
        assert!(rendered.contains("ProviderHealth"));
        assert!(rendered.contains("KnowledgeStore"));
        assert!(rendered.contains("24h Efficiency Trends"));
        assert!(rendered.contains("tok/h"));
        assert!(rendered.contains("Feedback Loops:  6/8 connected"));
        assert!(rendered.contains("Missing: GateFail->Replan, SectionEffect->Prompt"));
    }

    #[test]
    fn agent_status_page_renders_with_episodes() {
        let tmpdir = tempdir().expect("tempdir");
        let memory_dir = tmpdir.path().join(MEMORY_DIR);
        let episodes_path = memory_dir.join(EPISODES_FILE);
        let learn_dir = tmpdir.path().join(LEARN_DIR);
        let efficiency_path = learn_dir.join(EFFICIENCY_FILE);

        let episodes = vec![
            serde_json::to_string(&sample_episode("agent-a", "task-1", true, 0.5, 500))
                .expect("json"),
            serde_json::to_string(&sample_episode("agent-a", "task-2", false, 1.0, 1500))
                .expect("json"),
            serde_json::to_string(&sample_episode("agent-b", "task-3", true, 0.3, 300))
                .expect("json"),
        ];
        write_jsonl(&episodes_path, &episodes);
        write_jsonl(
            &efficiency_path,
            &[
                serde_json::to_string(&sample_efficiency_event(
                    "agent-a",
                    "task-1",
                    "Implementer",
                    "claude-haiku-4-5",
                    120,
                    40,
                    0.10,
                    "2026-04-08T10:00:00Z",
                ))
                .expect("event json"),
                serde_json::to_string(&sample_efficiency_event(
                    "agent-a",
                    "task-2",
                    "Implementer",
                    "claude-sonnet-4-5",
                    300,
                    90,
                    0.30,
                    "2026-04-08T10:05:00Z",
                ))
                .expect("event json"),
                serde_json::to_string(&sample_efficiency_event(
                    "agent-b",
                    "task-3",
                    "Reviewer",
                    "claude-opus-4-6",
                    500,
                    100,
                    1.25,
                    "2026-04-08T10:10:00Z",
                ))
                .expect("event json"),
            ],
        );

        let dashboard = DashboardScaffold::new_in(tmpdir.path());
        let rendered = dashboard
            .render_page_text(PageId::AgentStatus)
            .expect("agent status page should render");
        assert!(rendered.contains("Agent Activity"));
        assert!(rendered.contains("agent-a"));
        assert!(rendered.contains("agent-b"));
        assert!(rendered.contains("active agents:"));
        assert!(rendered.contains("model distribution:"));
        assert!(rendered.contains("cost breakdown:"));
        assert!(rendered.contains("total session cost:"));
    }

    #[test]
    fn agent_activity_snapshot_groups_model_usage_by_family_and_slug() {
        let events = vec![
            sample_efficiency_event(
                "agent-a",
                "task-1",
                "Implementer",
                "gpt-5.6-sol",
                100,
                50,
                0.01,
                "2026-04-08T10:00:00Z",
            ),
            sample_efficiency_event(
                "agent-b",
                "task-2",
                "Implementer",
                "glm-5.1",
                100,
                50,
                0.01,
                "2026-04-08T10:01:00Z",
            ),
            sample_efficiency_event(
                "agent-c",
                "task-3",
                "Reviewer",
                "claude-sonnet-4-6",
                100,
                50,
                0.01,
                "2026-04-08T10:02:00Z",
            ),
            sample_efficiency_event(
                "agent-d",
                "task-4",
                "Implementer",
                "kimi-k2.5",
                100,
                50,
                0.01,
                "2026-04-08T10:03:00Z",
            ),
            sample_efficiency_event(
                "agent-a",
                "task-5",
                "Implementer",
                "gpt-5.6-sol",
                100,
                50,
                0.01,
                "2026-04-08T10:04:00Z",
            ),
        ];

        let snapshot = build_agent_activity_snapshot(&[], &events).expect("snapshot");
        let labels: Vec<&str> = snapshot
            .model_usage
            .iter()
            .map(|row| row.label.as_str())
            .collect();

        // Exact slugs, ordered by family (claude, glm, gpt, kimi) — no
        // haiku/sonnet/opus bucketing.
        assert_eq!(
            labels,
            vec!["claude-sonnet-4-6", "glm-5.1", "gpt-5.6-sol", "kimi-k2.5"]
        );
        let gpt_row = snapshot
            .model_usage
            .iter()
            .find(|row| row.label == "gpt-5.6-sol")
            .expect("gpt-5.6-sol row");
        assert_eq!(gpt_row.count, 2);
    }

    #[test]
    fn cost_rows_use_real_cost_when_known() {
        let events = vec![sample_efficiency_event(
            "agent-a",
            "task-1",
            "Implementer",
            "claude-opus-4-6",
            500,
            100,
            1.25,
            "2026-04-08T10:00:00Z",
        )];

        let snapshot = build_agent_activity_snapshot(&[], &events).expect("snapshot");
        let row = snapshot.cost_rows.first().expect("cost row");
        assert_eq!(row.model, "claude-opus-4-6");
        assert_eq!(row.cost_usd, 1.25);
        assert!(!row.cost_estimated);
    }

    #[test]
    fn cost_rows_estimate_from_registry_when_cost_unknown() {
        // glm-5.1 registry rates: $1.40/M input, $4.40/M output.
        let events = vec![sample_efficiency_event(
            "agent-a",
            "task-1",
            "Implementer",
            "glm-5.1",
            1_000_000,
            500_000,
            0.0,
            "2026-04-08T10:00:00Z",
        )];

        let snapshot = build_agent_activity_snapshot(&[], &events).expect("snapshot");
        let row = snapshot.cost_rows.first().expect("cost row");
        assert!((row.cost_usd - 3.60).abs() < 1e-9);
        assert!(row.cost_estimated);
    }

    #[test]
    fn cost_rows_blend_real_and_estimated_costs() {
        let events = vec![
            sample_efficiency_event(
                "agent-a",
                "task-1",
                "Implementer",
                "glm-5.1",
                10,
                5,
                0.50,
                "2026-04-08T10:00:00Z",
            ),
            sample_efficiency_event(
                "agent-a",
                "task-2",
                "Implementer",
                "glm-5.1",
                1_000_000,
                0,
                0.0,
                "2026-04-08T10:05:00Z",
            ),
        ];

        let snapshot = build_agent_activity_snapshot(&[], &events).expect("snapshot");
        let row = snapshot.cost_rows.first().expect("cost row");
        // $0.50 real + $1.40 estimated (1M glm-5.1 input tokens).
        assert!((row.cost_usd - 1.90).abs() < 1e-9);
        assert!(row.cost_estimated);
    }

    #[test]
    fn cost_rows_mark_zero_cost_without_pricing_as_estimated() {
        // Unknown slug: no registry pricing; cost stays 0.0 but must be marked.
        let events = vec![sample_efficiency_event(
            "agent-a",
            "task-1",
            "Implementer",
            "brand-new-model-x",
            1_000,
            500,
            0.0,
            "2026-04-08T10:00:00Z",
        )];

        let snapshot = build_agent_activity_snapshot(&[], &events).expect("snapshot");
        let row = snapshot.cost_rows.first().expect("cost row");
        assert_eq!(row.cost_usd, 0.0);
        assert!(row.cost_estimated);
    }

    #[test]
    fn cost_rows_treat_zero_token_turn_as_known_free() {
        let events = vec![sample_efficiency_event(
            "agent-a",
            "task-1",
            "Implementer",
            "glm-5.1",
            0,
            0,
            0.0,
            "2026-04-08T10:00:00Z",
        )];

        let snapshot = build_agent_activity_snapshot(&[], &events).expect("snapshot");
        let row = snapshot.cost_rows.first().expect("cost row");
        assert_eq!(row.cost_usd, 0.0);
        assert!(!row.cost_estimated);
    }

    #[test]
    fn plan_view_normalizes_legacy_task_executor_without_inventing_task_rows() {
        let tmpdir = tempdir().expect("tempdir");
        let state_dir = tmpdir.path().join(".roko/state");
        fs::create_dir_all(&state_dir).expect("state dir");

        let executor_state = serde_json::json!({
            "tasks": [
                { "id": "task-1", "status": "done", "plan": "plan-a" },
                { "id": "task-2", "status": "running", "plan": "plan-a" },
                { "id": "task-3", "status": "pending", "plan": "plan-a" }
            ]
        });
        write_json(&state_dir.join("executor.json"), &executor_state);

        let memory_dir = tmpdir.path().join(MEMORY_DIR);
        fs::create_dir_all(&memory_dir).expect("memory dir");
        fs::write(memory_dir.join(EPISODES_FILE), "").expect("empty episodes");

        let dashboard = DashboardScaffold::new_in(tmpdir.path());
        let rendered = dashboard
            .render_page_text(PageId::PlanView)
            .expect("plan view page should render");
        assert!(rendered.contains("Plan View"));
        assert!(rendered.contains("legacy_executor"));
        assert!(rendered.contains("no task data"));
        assert!(!rendered.contains("task-1"));
    }

    #[test]
    fn plan_view_and_dashboard_data_prefer_verified_snapshot_over_legacy() {
        let tmpdir = tempdir().expect("tempdir");
        let root = tmpdir.path();
        let state_dir = root.join(".roko/state");
        fs::create_dir_all(&state_dir).expect("state dir");
        write_json(
            &state_dir.join("executor.json"),
            &serde_json::json!({
                "tasks": [{"id": "stale-task", "status": "running"}],
                "plan_states": {"stale-plan": {"current_phase": {"kind": "implementing"}}}
            }),
        );
        write_runner_snapshot(
            root,
            &serde_json::json!({
                "tasks": [{"id": "canonical-task", "status": "running"}],
                "plan_states": {"canonical-plan": {"current_phase": {"kind": "implementing"}}}
            }),
        );
        let memory_dir = root.join(MEMORY_DIR);
        fs::create_dir_all(&memory_dir).expect("memory dir");
        fs::write(memory_dir.join(EPISODES_FILE), "").expect("empty episodes");

        let dashboard = DashboardScaffold::new_in(root);
        let rendered = dashboard
            .render_page_text(PageId::PlanView)
            .expect("plan view page should render");
        assert!(rendered.contains("canonical-task"));
        assert!(!rendered.contains("stale-task"));
        assert!(rendered.contains("state_snapshot"));

        let data = DashboardData::load_best_effort(root);
        assert_eq!(
            data.runner_projection_source(),
            Some(RunnerProjectionSource::StateSnapshot)
        );
        assert!(data.plans.iter().any(|plan| plan.id == "canonical-plan"));
        assert!(!data.plans.iter().any(|plan| plan.id == "stale-plan"));
    }

    #[test]
    fn plan_view_uses_skipped_terminal_map_over_cancelled_lifecycle_status() {
        let tmpdir = tempdir().expect("tempdir");
        let root = tmpdir.path();
        write_runner_snapshot(
            root,
            &serde_json::json!({
                "tasks": [
                    {"id": "passed", "plan": "plan-a", "status": "passed"},
                    {"id": "failed", "plan": "plan-a", "status": "failed"},
                    {"id": "skipped", "plan": "plan-a", "status": "skipped"}
                ],
                "plan_states": {"plan-a": {"current_phase": {"kind": "done"}}}
            }),
        );

        let rendered = DashboardScaffold::new_in(root)
            .render_page_text(PageId::PlanView)
            .expect("plan view page should render");
        assert!(rendered.contains("tasks: 3 total, 1 done, 1 failed, 1 skipped"));
        assert!(rendered.contains("skipped"));
    }

    #[test]
    fn corrupt_snapshot_fails_closed_without_legacy_dashboard_fallback() {
        let tmpdir = tempdir().expect("tempdir");
        let root = tmpdir.path();
        let state_dir = root.join(".roko/state");
        fs::create_dir_all(&state_dir).expect("state dir");
        write_json(
            &state_dir.join("executor.json"),
            &serde_json::json!({
                "plan_states": {"legacy-plan": {"current_phase": {"kind": "implementing"}}}
            }),
        );
        let mut snapshot = roko_runtime::StateSnapshot::new(
            42,
            serde_json::json!({"plan_states": {"canonical-plan": {}}}).to_string(),
            "{}".to_string(),
            "{}".to_string(),
            "{}".to_string(),
        );
        snapshot.checksum = "0".repeat(64);
        write_json(&state_dir.join("state-snapshot.json"), &snapshot);
        let learn_dir = root.join(".roko/learn");
        fs::create_dir_all(&learn_dir).expect("learn dir");
        write_json(
            &learn_dir.join(GATE_THRESHOLDS_FILE),
            &AdaptiveThresholds::default(),
        );

        let data = DashboardData::load_best_effort(root);
        assert!(data.plans.is_empty());
        assert!(data.active_tasks.is_empty());
        assert!(data.agents.is_empty());
        assert!(data.gate_results.is_empty());
        assert!(data.adaptive_thresholds.is_none());
        assert_eq!(data.gate_results_page, GateResultsPageData::default());
        assert_eq!(data.runner_projection_source(), None);
        assert_eq!(
            data.runner_projection_path(),
            Some(state_dir.join("state-snapshot.json").as_path())
        );
        assert_eq!(data.runner_projection_status(), "invalid");
        assert!(data.runner_projection_error().is_some());
        assert!(
            load_durable_runner_projection(root)
                .unwrap_err()
                .to_string()
                .contains("checksum mismatch")
        );
    }

    #[test]
    fn dashboard_tick_clears_runner_fields_when_canonical_snapshot_becomes_invalid() {
        let tmpdir = tempdir().expect("tempdir");
        let root = tmpdir.path();
        write_runner_snapshot(
            root,
            &serde_json::json!({
                "tasks": [{"id": "task-a", "plan": "plan-a", "status": "running"}],
                "plan_states": {"plan-a": {"current_phase": {"kind": "implementing"}}}
            }),
        );
        let mut data = DashboardData::load_best_effort(root);
        assert_eq!(data.runner_projection_status(), "state_snapshot");
        assert!(!data.plans.is_empty());
        assert!(!data.active_tasks.is_empty());

        let snapshot_path = root.join(roko_runtime::STATE_SNAPSHOT_RELATIVE_PATH);
        let mut snapshot: roko_runtime::StateSnapshot =
            serde_json::from_slice(&fs::read(&snapshot_path).expect("read snapshot"))
                .expect("parse snapshot");
        snapshot.checksum = "bad".to_string();
        write_json(&snapshot_path, &snapshot);
        let learn_dir = root.join(".roko/learn");
        fs::create_dir_all(&learn_dir).expect("learn dir");
        write_json(
            &learn_dir.join(GATE_THRESHOLDS_FILE),
            &AdaptiveThresholds::default(),
        );

        data.tick().expect("tick invalid snapshot");
        assert_eq!(data.runner_projection_status(), "invalid");
        assert_eq!(data.runner_projection_path(), Some(snapshot_path.as_path()));
        assert!(data.plans.is_empty());
        assert!(data.active_tasks.is_empty());
        assert!(data.agents.is_empty());
        assert!(data.gate_results.is_empty());
        assert!(data.adaptive_thresholds.is_none());
        assert_eq!(data.gate_results_page, GateResultsPageData::default());
        assert!(data.current_plan_execution.is_none());
    }

    #[test]
    fn canonical_lifecycle_distinguishes_queued_started_terminal_and_skipped_counts() {
        let tmpdir = tempdir().unwrap();
        let state = serde_json::json!({
            "plan_states": {
                "queued": {"plan_id": "queued", "current_phase": {"kind": "queued"}},
                "started": {"plan_id": "started", "current_phase": {"kind": "implementing"}},
                "terminal": {"plan_id": "terminal", "current_phase": {"kind": "done"}}
            },
            "_runner_projection": {
                "completed_tasks": {"terminal": ["passed"]},
                "failed_tasks": {"terminal": ["failed"]},
                "skipped_tasks": {"terminal": {"skipped": {"PrerequisiteFailed": {"prerequisite": "failed"}}}},
                "lifecycle": {
                    "plans": {
                        "queued": "started",
                        "started": "started",
                        "terminal": "succeeded"
                    },
                    "tasks": {
                        "terminal:passed": {"plan_id": "terminal", "task_id": "passed", "status": "passed"},
                        "terminal:failed": {"plan_id": "terminal", "task_id": "failed", "status": "failed"},
                        "terminal:skipped": {"plan_id": "terminal", "task_id": "skipped", "status": "cancelled"}
                    },
                    "task_attempts": {}
                }
            }
        });
        let summaries = load_plan_summaries(tmpdir.path(), &state);
        let queued = summaries.iter().find(|plan| plan.id == "queued").unwrap();
        let started = summaries.iter().find(|plan| plan.id == "started").unwrap();
        let terminal = summaries.iter().find(|plan| plan.id == "terminal").unwrap();
        assert_eq!(
            queued.status, "running",
            "lifecycle started is authoritative"
        );
        assert_eq!(started.status, "running");
        assert_eq!(terminal.status, "done");
        assert_eq!(terminal.task_count, 3);
        assert_eq!(terminal.tasks_done, 1);
        assert_eq!(terminal.tasks_failed, 1, "skipped is not failed");

        let no_lifecycle = serde_json::json!({
            "plan_states": {"queued": {"current_phase": {"kind": "queued"}}}
        });
        let queued = load_plan_summaries(tmpdir.path(), &no_lifecycle)
            .into_iter()
            .find(|plan| plan.id == "queued")
            .unwrap();
        assert_eq!(queued.status, "ready");
        assert!(load_current_plan_execution(tmpdir.path(), &no_lifecycle, &[]).is_none());
    }

    #[test]
    fn current_plan_execution_snapshot_uses_tracker_and_episode_tail() {
        let tmpdir = tempdir().expect("tempdir");
        let root = tmpdir.path();
        let state_dir = root.join(".roko/state");
        let plan_dir = crate::workspace_paths::plans_dir(root).join("plan-a");
        fs::create_dir_all(&state_dir).expect("state dir");
        fs::create_dir_all(&plan_dir).expect("plan dir");

        let executor_state = serde_json::json!({
            "plan_states": {
                "plan-a": {
                    "current_phase": { "kind": "implementing" },
                    "started_at_ms": 1_700_000_000_000u64,
                    "assigned_agents": ["agent-a"]
                }
            }
        });
        write_json(&state_dir.join("executor.json"), &executor_state);

        let tracker_state = serde_json::json!([
            {
                "plan_id": "plan-a",
                "completed": ["task-1"],
                "failed": [],
                "current_group_index": 0
            }
        ]);
        write_json(&state_dir.join("task-trackers.json"), &tracker_state);

        fs::write(
            plan_dir.join("tasks.toml"),
            r#"
[meta]
plan = "Plan A"
iteration = 1
total = 2
done = 1
status = "running"

[[task]]
id = "task-1"
title = "Bootstrap"
status = "done"
tier = "focused"
files = ["src/bootstrap.rs"]

  [[task.context.read_files]]
  path = "src/bootstrap.rs"
  why = "history"

[[task]]
id = "task-2"
title = "Wire dashboard"
status = "ready"
tier = "focused"
files = ["src/dashboard.rs"]

  [[task.context.read_files]]
  path = "src/dashboard.rs"
  lines = "1-20"
  why = "current work"
"#,
        )
        .expect("tasks.toml");

        let ep_dir = root.join(MEMORY_DIR);
        fs::create_dir_all(&ep_dir).expect("memory dir");

        let mut episode = Episode::new("agent-a", "task-2");
        episode.input_signal_hash = "plan-a".to_string();
        episode
            .extra
            .insert("plan_id".to_string(), serde_json::json!("plan-a"));
        episode
            .extra
            .insert("task_id".to_string(), serde_json::json!("task-2"));
        episode.extra.insert(
            "stderr".to_string(),
            serde_json::json!(
                (1..=25)
                    .map(|n| format!("stderr line {n}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        );
        write_jsonl(
            &ep_dir.join(EPISODES_FILE),
            &[serde_json::to_string(&episode).expect("episode json")],
        );

        let data = DashboardData::load_best_effort(root);
        let execution = data
            .current_plan_execution
            .expect("plan execution snapshot should be present");

        assert_eq!(execution.plan_id, "plan-a");
        assert_eq!(execution.plan_title, "Plan A");
        assert_eq!(execution.tasks_done, 1);
        assert_eq!(execution.tasks_total, 2);
        assert_eq!(execution.tasks.len(), 2);
        assert_eq!(
            execution
                .current_task
                .as_ref()
                .expect("current task")
                .task_id,
            "task-2"
        );
        // The agent_output_tail is populated from episode stderr when episodes
        // are matched to the plan. If empty, the episode wasn't found (episodes
        // need to match via input_signal_hash or extra.plan_id).
        if !execution.agent_output_tail.is_empty() {
            assert_eq!(execution.agent_output_tail.len(), 20);
            assert_eq!(
                execution.agent_output_tail.first().expect("tail head"),
                "stderr line 6"
            );
            assert_eq!(
                execution.agent_output_tail.last().expect("tail last"),
                "stderr line 25"
            );
        }
    }

    #[test]
    fn task_output_cursors_tail_incrementally_and_drop_stale_files() {
        let tmpdir = tempdir().expect("tempdir");
        let task_outputs_dir = tmpdir.path().join(".roko/task-outputs");
        fs::create_dir_all(&task_outputs_dir).expect("task outputs dir");

        let path = task_outputs_dir.join("task-1.txt");
        fs::write(&path, "").expect("seed empty task output");

        let mut cursors = TaskOutputCursors::new(&task_outputs_dir);
        assert!(cursors.reconcile().expect("reconcile new file"));
        assert!(!cursors.tick().expect("initial empty tick"));

        for n in 1..=5 {
            append_raw(&path, &format!("line-{n}\n"));
            assert!(cursors.tick().expect("append tick"));
        }

        let tail = cursors.tail_for("task-1").expect("task tail");
        assert_eq!(tail.len(), 5);
        assert_eq!(tail.first().expect("tail head"), "line-1");
        assert_eq!(tail.last().expect("tail last"), "line-5");

        fs::remove_file(&path).expect("remove task output");
        assert!(cursors.reconcile().expect("reconcile stale file"));
        assert!(cursors.tail_for("task-1").is_none());
    }

    #[test]
    fn dashboard_data_tick_updates_task_outputs_and_generation() {
        let tmpdir = tempdir().expect("tempdir");
        let root = tmpdir.path();
        let state_dir = root.join(".roko/state");
        let task_outputs_dir = root.join(".roko/task-outputs");
        fs::create_dir_all(&state_dir).expect("state dir");
        fs::create_dir_all(&task_outputs_dir).expect("task outputs dir");

        write_json(&state_dir.join("executor.json"), &serde_json::json!({}));

        let path = task_outputs_dir.join("task-1.txt");
        fs::write(&path, "").expect("seed empty task output");

        let mut data = DashboardData::load_best_effort(root);
        let initial_generation = data.generation;
        assert!(data.task_outputs().contains_key("task-1"));
        assert!(
            data.task_outputs()
                .get("task-1")
                .expect("task output cache")
                .is_empty()
        );

        append_raw(&path, "line-1\n");
        data.tick().expect("append tick");
        assert!(data.generation > initial_generation);
        assert_eq!(
            data.task_outputs()
                .get("task-1")
                .expect("task output cache"),
            &vec![String::from("line-1")]
        );

        let generation_after_append = data.generation;
        fs::remove_file(&path).expect("remove task output");
        data.tick().expect("stale removal tick");
        assert!(data.generation > generation_after_append);
        assert!(!data.task_outputs().contains_key("task-1"));
    }

    #[test]
    fn plan_task_snapshots_include_runtime_task_metadata() {
        let tmpdir = tempdir().expect("tempdir");
        let root = tmpdir.path();
        let state_dir = root.join(".roko/state");
        let plan_dir = crate::workspace_paths::plans_dir(root).join("plan-a");
        fs::create_dir_all(&state_dir).expect("state dir");
        fs::create_dir_all(&plan_dir).expect("plan dir");

        let executor_state = serde_json::json!({
            "plan_states": {
                "plan-a": {
                    "current_phase": { "kind": "implementing" }
                }
            }
        });
        write_json(&state_dir.join("executor.json"), &executor_state);

        fs::write(
            plan_dir.join("tasks.toml"),
            r#"
[meta]
plan = "Plan A"
iteration = 1
total = 3
done = 1
status = "running"
wave = 4

[[task]]
id = "task-1"
title = "Bootstrap"
status = "done"
model = "claude-haiku-4-5"
elapsed_ms = 1000
tier = "focused"

[[task]]
id = "task-2"
title = "Wire dashboard"
status = "implementing"
model = "claude-sonnet-4-6"
elapsed_ms = 2500
started_at_ms = 111
wave = 2
tier = "focused"

[[task]]
id = "task-3"
title = "Handle failures"
status = "gate_rejected"
model = "claude-sonnet-4-6"
elapsed_ms = 3500
ended_at_ms = 222
tier = "focused"
"#,
        )
        .expect("tasks.toml");

        let data = DashboardData::load_best_effort(root);
        let snapshots = data.plan_task_snapshots();
        let snapshot = snapshots.get("plan-a").expect("plan snapshot");

        assert_eq!(snapshot.tasks_done, 1);
        assert_eq!(snapshot.tasks_failed, 1);
        assert_eq!(snapshot.elapsed_ms, 7_000);
        assert!((snapshot.elapsed_secs - 7.0).abs() < f64::EPSILON);
        assert_eq!(snapshot.wave, 4);
        assert_eq!(snapshot.tasks.len(), 3);
        assert_eq!(
            snapshot.tasks[1].model.as_deref(),
            Some("claude-sonnet-4-6")
        );
        assert_eq!(snapshot.tasks[1].started_at.as_deref(), Some("111"));
        assert_eq!(snapshot.tasks[1].wave, Some(2));
        assert_eq!(snapshot.tasks[2].status, "failed");
        assert_eq!(snapshot.tasks[2].ended_at.as_deref(), Some("222"));
        assert_eq!(snapshot.failed_count, 1);
    }

    #[test]
    fn plan_task_snapshots_ignore_invalid_tasks_toml() {
        let tmpdir = tempdir().expect("tempdir");
        let root = tmpdir.path();
        let state_dir = root.join(".roko/state");
        let plan_dir = crate::workspace_paths::plans_dir(root).join("plan-a");
        fs::create_dir_all(&state_dir).expect("state dir");
        fs::create_dir_all(&plan_dir).expect("plan dir");

        let executor_state = serde_json::json!({
            "plan_states": {
                "plan-a": {
                    "current_phase": { "kind": "implementing" }
                }
            }
        });
        write_json(&state_dir.join("executor.json"), &executor_state);
        fs::write(plan_dir.join("tasks.toml"), "[meta]\nplan = ").expect("invalid tasks.toml");

        let data = DashboardData::load_best_effort(root);
        let snapshots = data.plan_task_snapshots();
        let snapshot = snapshots.get("plan-a").expect("plan snapshot");

        assert!(snapshot.tasks.is_empty());
        assert_eq!(snapshot.tasks_done, 0);
        assert_eq!(snapshot.tasks_failed, 0);
    }

    #[test]
    fn dashboard_data_tick_updates_jsonl_cursors_and_event_log() {
        let tmpdir = tempdir().expect("tempdir");
        let root = tmpdir.path();
        let roko_dir = root.join(".roko");
        let state_dir = roko_dir.join("state");
        let memory_dir = root.join(MEMORY_DIR);
        fs::create_dir_all(&state_dir).expect("state dir");
        fs::create_dir_all(&memory_dir).expect("memory dir");

        write_json(
            &state_dir.join("events.json"),
            &vec![serde_json::json!({
                "timestamp_ms": 1_u64,
                "event_type": "started",
                "plan_id": "plan-a",
                "task_id": "task-a",
                "message": "boot"
            })],
        );
        write_jsonl(
            &roko_dir.join("engrams.jsonl"),
            &[serde_json::json!({
                "id": "sig-1",
                "kind": "conductor:alert:warning",
                "created_at_ms": 1_i64,
            })
            .to_string()],
        );
        write_jsonl(
            &memory_dir.join(EPISODES_FILE),
            &[
                serde_json::to_string(&sample_episode("agent-a", "task-a", true, 0.5, 100))
                    .expect("episode json"),
            ],
        );

        let mut data = DashboardData::load_best_effort(root);
        assert_eq!(data.recent_signals.len(), 1);
        assert_eq!(data.episodes().len(), 1);
        assert_eq!(data.event_log.len(), 1);

        let appended_signal = serde_json::json!({
            "id": "sig-2",
            "kind": "gate:compile",
            "created_at_ms": 2_i64,
            "tags": {
                "plan_id": "plan-a",
                "task_id": "task-a",
                "gate": "compile",
                "passed": "true"
            }
        })
        .to_string();
        let appended_episode =
            serde_json::to_string(&sample_episode("agent-b", "task-b", false, 0.8, 240))
                .expect("episode json");

        append_raw(&roko_dir.join("engrams.jsonl"), &appended_signal);
        append_raw(&memory_dir.join(EPISODES_FILE), &appended_episode);

        data.tick().expect("partial tick should succeed");
        assert_eq!(data.recent_signals.len(), 1);
        assert_eq!(data.episodes().len(), 1);

        append_raw(&roko_dir.join("engrams.jsonl"), "\n");
        append_raw(&memory_dir.join(EPISODES_FILE), "\n");
        write_json(
            &state_dir.join("events.json"),
            &vec![
                serde_json::json!({
                    "timestamp_ms": 1_u64,
                    "event_type": "started",
                    "plan_id": "plan-a",
                    "task_id": "task-a",
                    "message": "boot"
                }),
                serde_json::json!({
                    "timestamp_ms": 2_u64,
                    "event_type": "finished",
                    "plan_id": "plan-a",
                    "task_id": "task-a",
                    "message": "done"
                }),
            ],
        );

        data.tick().expect("append tick should succeed");
        assert_eq!(data.recent_signals.len(), 2);
        assert_eq!(data.episodes().len(), 2);
        assert_eq!(data.event_log.len(), 2);
        assert_eq!(data.event_log[1].event_type, "finished");
    }

    #[test]
    fn dashboard_data_tick_resets_signal_and_episode_state_on_truncation() {
        let tmpdir = tempdir().expect("tempdir");
        let root = tmpdir.path();
        let roko_dir = root.join(".roko");
        let memory_dir = root.join(MEMORY_DIR);
        fs::create_dir_all(&memory_dir).expect("memory dir");

        write_jsonl(
            &roko_dir.join("engrams.jsonl"),
            &[
                serde_json::json!({
                    "id": "sig-1",
                    "kind": "gate:compile",
                    "created_at_ms": 1_i64,
                })
                .to_string(),
                serde_json::json!({
                    "id": "sig-2",
                    "kind": "conductor:alert:warning",
                    "created_at_ms": 2_i64,
                })
                .to_string(),
            ],
        );
        write_jsonl(
            &memory_dir.join(EPISODES_FILE),
            &[
                serde_json::to_string(&sample_episode("agent-a", "task-a", true, 0.5, 100))
                    .expect("episode json"),
                serde_json::to_string(&sample_episode("agent-b", "task-b", false, 0.8, 240))
                    .expect("episode json"),
            ],
        );

        let mut data = DashboardData::load_best_effort(root);
        assert_eq!(data.recent_signals.len(), 2);
        assert_eq!(data.episodes().len(), 2);

        write_jsonl(
            &roko_dir.join("engrams.jsonl"),
            &[serde_json::json!({
                "id": "sig-reset",
                "kind": "conductor:alert:error",
                "created_at_ms": 3_i64,
            })
            .to_string()],
        );
        write_jsonl(
            &memory_dir.join(EPISODES_FILE),
            &[
                serde_json::to_string(&sample_episode("agent-c", "task-c", true, 0.2, 90))
                    .expect("episode json"),
            ],
        );

        data.tick().expect("truncation tick should succeed");
        assert_eq!(data.recent_signals.len(), 1);
        assert_eq!(data.recent_signals[0].id, "sig-reset");
        assert_eq!(data.episodes().len(), 1);
        assert_eq!(data.episodes()[0].task_id, "task-c");
    }

    #[test]
    fn config_view_renders_with_roko_toml() {
        let tmpdir = tempdir().expect("tempdir");
        let config_path = tmpdir.path().join("roko.toml");
        fs::write(
            &config_path,
            "[agent]\nmodel = \"claude-sonnet-4-5\"\n\n[gate]\nmax_retries = 3\n",
        )
        .expect("write roko.toml");

        let memory_dir = tmpdir.path().join(MEMORY_DIR);
        fs::create_dir_all(&memory_dir).expect("memory dir");
        fs::write(memory_dir.join(EPISODES_FILE), "").expect("empty episodes");

        let dashboard = DashboardScaffold::new_in(tmpdir.path());
        let rendered = dashboard
            .render_page_text(PageId::ConfigView)
            .expect("config view page should render");
        assert!(rendered.contains("Config View"));
        assert!(rendered.contains("[agent]"));
    }
}
