# God Structs: TuiState (127 fields), PlanRunner (80 fields), AppState (47 fields)

Three structs in this codebase have grown far beyond single-responsibility boundaries.
Each acts as a "god object" that couples every consumer to every other concern.
This document provides full decomposition designs with field-level detail, usage analysis,
migration strategy, code examples, risk analysis, and effort estimates.

---

## 1. TuiState -- 127 Fields

**File**: `crates/roko-cli/src/tui/state.rs` (4,955 lines, struct at line 986)

### 1.1 Current Definition

```rust
pub struct TuiState {
    // -- Core orchestrator state (5 fields) --
    pub orchestrator_state: String,
    pub plans: Vec<PlanEntry>,
    pub current_plan_idx: usize,
    pub current_iteration: usize,
    pub current_phase: String,

    // -- Phase pipeline (1 field) --
    pub phase_pipeline: Vec<PhaseStep>,

    // -- Execution waves (1 field) --
    pub execution_waves: Vec<Wave>,

    // -- Task checklist (1 field) --
    pub current_task_checklist: Vec<TaskRow>,

    // -- Gate results (5 fields) --
    pub gate_results: Vec<GateResultEntry>,
    pub diagnoses: Vec<DiagnosisSummary>,
    pub experiment_winners: Vec<ExperimentWinnerSummary>,
    pub gate_trends: HashMap<String, TrendBuckets>,
    pub gate_recent_failures: Vec<FailureEntry>,

    // -- Agents (6 fields) --
    pub agents: Vec<AgentRow>,
    pub agent_topology: AgentTopology,
    pub agent_topology_status: AgentTopologyStatus,
    pub route_metrics: HashMap<String, RouteMetrics>,
    pub agent_output_cache: RefCell<HashMap<String, CachedRender>>,
    pub agent_streams: HashMap<String, AgentStream>,

    // -- Navigation (10 fields) --
    pub active_tab: Tab,
    pub selected_plan_idx: usize,
    pub selected_agent: usize,
    pub selected_agent_tab: usize,
    pub config_sub_tab: usize,
    pub inspect_sub_tab: usize,
    pub marketplace_sub_tab: usize,
    pub atelier_sub_tab: usize,
    pub focus: FocusZone,
    pub atmosphere: Atmosphere,

    // -- Input (5 fields) --
    pub input_mode: InputMode,
    pub message_input: String,
    pub filter_text: String,
    pub filter_active: bool,
    pub filter: String,

    // -- Scroll positions (12 fields) --
    pub agent_scroll: Option<usize>,
    pub diff_scroll: usize,
    pub task_scroll: usize,
    pub command_output_scroll: usize,
    pub plan_detail_scroll: usize,
    pub plan_scroll_offset: usize,
    pub log_scroll: usize,
    pub agent_topology_visible: bool,
    pub agent_topology_scroll_offset: usize,
    pub log_auto_tail: bool,
    pub log_filter_levels: HashSet<LogFilterLevel>,
    pub plan_detail_tab: usize,

    // -- Approval / confirm / modal (3 fields) --
    pub pending_approval: Option<PendingApproval>,
    pub pending_confirm: Option<ConfirmAction>,
    pub active_modal: Option<ModalState>,

    // -- Git (10 fields) --
    pub git_branch: String,
    pub git_commit_short: String,
    pub git_age: String,
    pub git_branch_tree: Vec<GitBranchNode>,
    pub git_commit_graph: Vec<GitCommitEntry>,
    pub git_worktree_list: Vec<String>,
    pub git_branch_cursor: usize,
    pub git_summary_lines: Vec<String>,
    pub(crate) git_view_data: Option<GitViewData>,
    pub git_diff: String,

    // -- Pipeline control (1 field) --
    pub is_paused: bool,

    // -- Cost / tokens (11 fields) --
    pub cost_per_plan: HashMap<String, f64>,
    pub cost_per_task: HashMap<String, f64>,
    pub cumulative_input_tokens: u64,
    pub cumulative_output_tokens: u64,
    pub token_total: u64,
    pub token_history: HashMap<String, VecDeque<u64>>,
    pub token_rate: f64,
    pub cost_rate: f64,
    pub cost_dollars: f64,
    pub process_metrics: Vec<ProcessMetrics>,
    pub sys: SysMetrics,

    // -- Timing (1 field) --
    pub run_started: Option<Instant>,

    // -- Wave navigation (1 field) --
    pub selected_wave_idx: usize,

    // -- Config editor (6 fields) --
    pub config_cursor: usize,
    pub config_pending: HashMap<String, String>,
    pub config_scroll_offset: usize,
    pub config_editing: bool,
    pub config_edit_buffer: String,
    pub config_edit_key: Option<String>,

    // -- Agent pane (1 field) --
    pub agent_pane_group: usize,

    // -- Push-path state (3 fields) --
    pub event_log: Vec<DashboardEventLogEntry>,
    pub cascade_router_json: String,
    pub gate_thresholds_json: String,

    // -- View data / DashboardData migration (21 fields) --
    pub workdir: PathBuf,
    pub efficiency_summary: EfficiencySummary,
    pub efficiency_events: Vec<AgentEfficiencyEvent>,
    pub efficiency_trend: Vec<EfficiencyBucket>,
    pub cfactor_trend_buckets: Vec<CFactorBucket>,
    pub cascade_router: CascadeRouterState,
    pub recent_signals: Vec<SignalSummary>,
    pub current_plan_execution: Option<PlanExecutionSnapshot>,
    pub conductor_alerts: Vec<AlertSummary>,
    pub cfactor: Option<CFactor>,
    pub gate_results_page: GateResultsPageData,
    pub experiments: Vec<ExperimentSummary>,
    pub task_output_tails: HashMap<String, Vec<String>>,
    pub plan_summaries: Vec<PlanSummary>,
    pub agent_summaries: Vec<AgentSummary>,
    pub active_task_summaries: Vec<TaskSummary>,
    pub gate_result_summaries: Vec<GateResultSummary>,
    pub episodes_cache: Vec<Episode>,
    pub cached_unified_log: Vec<LogEntry>,
    pub knowledge_entries: Vec<KnowledgeBrowseEntry>,
    pub agents_online: usize,
    pub isfr: Option<f64>,

    // -- Marketplace / Atelier (15 fields) --
    pub marketplace_jobs: Vec<MarketplaceJob>,
    pub marketplace_selected_job: usize,
    pub atelier_prds: Vec<PrdSummary>,
    pub atelier_selected_prd: usize,
    pub atelier_tasks_by_slug: HashMap<String, Vec<TaskSummary>>,
    pub job_form_editing: bool,
    pub job_form_title: String,
    pub job_form_type: String,
    pub job_form_priority: String,
    pub job_form_description: String,
    pub job_form_focus: JobFormField,
    pub job_assign_editing: bool,
    pub job_assign_buffer: String,
    pub job_progress: HashMap<String, JobProgressEntry>,
    pub command_results: Vec<CommandResult>,

    // -- Smoothed metrics (6 private fields) --
    cpu_pct_smoothed: SmoothedValue,
    token_rate_smoothed: SmoothedValue,
    cost_rate_smoothed: SmoothedValue,
    last_rate_sample_at: Option<Instant>,
    last_token_total_sample: u64,
    last_cost_dollars_sample: f64,
}
```

**Total: 127 fields** (121 public, 6 private).

### 1.2 Usage Analysis

TuiState is referenced in **26 files** across the TUI module. Every render function takes
`&TuiState`, coupling every widget to the entire state surface.

| File | How it accesses TuiState |
|---|---|
| `tui/app.rs` | Master event loop. Reads/writes nearly all fields. The central coordinator. |
| `tui/state.rs` | Definition, `Default` impl, `update_from_snapshot()`, helper methods. |
| `tui/dashboard.rs` | `DashboardData` feeds `update_from_snapshot()`. Bridge between disk state and TUI. |
| `views/dashboard_view.rs` | 20+ render functions. Reads: plans, agents, costs, tokens, git, efficiency, cfactor, experiments, gate trends. |
| `views/agents_view.rs` | 14 render functions. Reads: agents, agent_topology, route_metrics, agent_streams, agent_output_cache, tokens, costs. |
| `views/plans_view.rs` | 8 render functions. Reads: plans, current_plan_idx, task checklist, execution_waves, gate_results. |
| `views/learning_view.rs` | 7 render functions. Reads: cascade_router, efficiency_summary, efficiency_events, experiments. |
| `views/config_view.rs` | 4 render functions. Reads: config_cursor, config_pending, config_editing, cascade_router, experiments. |
| `views/context_view.rs` | 10 render functions. Reads: efficiency_events, cfactor, conductor_alerts, knowledge_entries, episodes_cache. |
| `views/marketplace_view.rs` | 3 render functions. Reads: marketplace_jobs, marketplace_selected_job, job_form_*, job_assign_*. |
| `views/atelier_view.rs` | 2 render functions. Reads: atelier_prds, atelier_selected_prd, atelier_tasks_by_slug. |
| `views/git_view.rs` | 2 render functions. Reads: git_branch_tree, git_commit_graph, git_worktree_list, git_view_data. |
| `views/logs_view.rs` | 3 render functions. Reads: cached_unified_log, log_scroll, log_filter_levels, recent_signals. |
| `widgets/header_bar.rs` | Reads: orchestrator_state, active_tab, plans (task counts), agents (active count), token_total, token_rate, cost_dollars, cost_rate, agents_online, isfr. |
| `widgets/status_bar.rs` | Reads: input_mode, filter, git_branch, git_commit_short, git_age. |
| `widgets/plan_tree.rs` | 7 functions. Reads: plans, selected_plan_idx, filter, gate_results. |
| `widgets/task_progress.rs` | Reads: current_task_checklist, task_scroll. |
| `widgets/phase_compact.rs` | Reads: phase_pipeline. |
| `widgets/wave_progress.rs` | Reads: execution_waves, selected_wave_idx. |
| `widgets/token_sparkline.rs` | Reads: token_history, token_total. |
| `widgets/sys_metrics.rs` | Reads: sys (SysMetrics). |
| `modals/mod.rs` | 2 render functions. Reads: active_modal, pending_approval, pending_confirm. |
| `modals/plan_detail.rs` | Reads: plans, current_plan_execution, gate_results. |
| `postfx_pipeline.rs` | Reads: atmosphere, orchestrator_state, agents, sys. |
| `tui/mod.rs` | Re-exports, type references. |
| `views/mod.rs` | Dispatches to per-tab render functions. Reads: active_tab, inspect_sub_tab, filter_active. |

**Key observation**: Each view file reads 3-15 specific fields. No view reads all 127. The
access patterns cluster strongly by tab/domain. This makes decomposition straightforward.

### 1.3 Decomposition Design

```rust
pub struct TuiState {
    pub nav: NavState,                // 11 fields -- navigation + focus
    pub input: InputState,            // 5 fields -- text input, filter
    pub scroll: ScrollState,          // 12 fields -- per-panel scroll offsets
    pub modals: ModalState,           // 3 fields -- approval, confirm, modal
    pub plans: PlanViewState,         // 9 fields -- plan list, tasks, waves, phase
    pub agents: AgentViewState,       // 8 fields -- agent roster, topology, streams
    pub gates: GateViewState,         // 5 fields -- gate results, trends, failures
    pub costs: CostState,             // 12 fields -- tokens, cost, rates, process metrics
    pub git: GitState,                // 10 fields -- branch, commits, worktrees, diff
    pub learning: LearningViewState,  // 10 fields -- efficiency, router, cfactor, experiments
    pub logs: LogViewState,           // 4 fields -- unified log, episodes, signals
    pub inspect: InspectViewState,    // 4 fields -- conductor alerts, cfactor, knowledge, event_log
    pub marketplace: MarketplaceState,// 15 fields -- jobs, PRDs, forms
    pub config_editor: ConfigEditorState, // 6 fields -- config editing state
    pub chrome: ChromeState,          // 7 fields -- orchestrator_state, iteration, phase, paused, timing, atmosphere
    pub smoothed: SmoothedMetrics,    // 6 fields -- private smoothing state

    // Shared data (read by multiple views, set by snapshot bridge)
    pub workdir: PathBuf,             // Used by config_view for file loading
}
```

#### Sub-struct definitions:

```rust
/// Tab selection, focus, sub-tab indices.
pub struct NavState {
    pub active_tab: Tab,
    pub selected_plan_idx: usize,
    pub selected_agent: usize,
    pub selected_agent_tab: usize,
    pub config_sub_tab: usize,
    pub inspect_sub_tab: usize,
    pub marketplace_sub_tab: usize,
    pub atelier_sub_tab: usize,
    pub focus: FocusZone,
    pub plan_detail_tab: usize,
    pub agent_pane_group: usize,
}

/// Text input and filter state.
pub struct InputState {
    pub input_mode: InputMode,
    pub message_input: String,
    pub filter_text: String,
    pub filter_active: bool,
    pub filter: String,
}

/// Per-panel scroll positions. Pure UI state with no domain logic.
pub struct ScrollState {
    pub agent_scroll: Option<usize>,
    pub diff_scroll: usize,
    pub task_scroll: usize,
    pub command_output_scroll: usize,
    pub plan_detail_scroll: usize,
    pub plan_scroll_offset: usize,
    pub log_scroll: usize,
    pub agent_topology_visible: bool,
    pub agent_topology_scroll_offset: usize,
    pub log_auto_tail: bool,
    pub log_filter_levels: HashSet<LogFilterLevel>,
    pub selected_wave_idx: usize,
}

/// Plan list, execution waves, task checklist.
pub struct PlanViewState {
    pub plans: Vec<PlanEntry>,
    pub current_plan_idx: usize,
    pub execution_waves: Vec<Wave>,
    pub current_task_checklist: Vec<TaskRow>,
    pub phase_pipeline: Vec<PhaseStep>,
    pub current_plan_execution: Option<PlanExecutionSnapshot>,
    pub plan_summaries: Vec<PlanSummary>,
    pub active_task_summaries: Vec<TaskSummary>,
    pub task_output_tails: HashMap<String, Vec<String>>,
}

/// Agent roster, topology, output caches, route metrics.
pub struct AgentViewState {
    pub agents: Vec<AgentRow>,
    pub agent_topology: AgentTopology,
    pub agent_topology_status: AgentTopologyStatus,
    pub route_metrics: HashMap<String, RouteMetrics>,
    pub agent_output_cache: RefCell<HashMap<String, CachedRender>>,
    pub agent_streams: HashMap<String, AgentStream>,
    pub agent_summaries: Vec<AgentSummary>,
    pub agents_online: usize,
}

/// Gate results, diagnoses, trends, failures.
pub struct GateViewState {
    pub gate_results: Vec<GateResultEntry>,
    pub diagnoses: Vec<DiagnosisSummary>,
    pub gate_trends: HashMap<String, TrendBuckets>,
    pub gate_recent_failures: Vec<FailureEntry>,
    pub gate_results_page: GateResultsPageData,
    pub gate_result_summaries: Vec<GateResultSummary>,
}

/// Token counts, cost tracking, system metrics, rates.
pub struct CostState {
    pub cost_per_plan: HashMap<String, f64>,
    pub cost_per_task: HashMap<String, f64>,
    pub cumulative_input_tokens: u64,
    pub cumulative_output_tokens: u64,
    pub token_total: u64,
    pub token_history: HashMap<String, VecDeque<u64>>,
    pub token_rate: f64,
    pub cost_rate: f64,
    pub cost_dollars: f64,
    pub process_metrics: Vec<ProcessMetrics>,
    pub sys: SysMetrics,
    pub isfr: Option<f64>,
}

/// Git branch, commits, worktrees, diff.
pub struct GitState {
    pub git_branch: String,
    pub git_commit_short: String,
    pub git_age: String,
    pub git_branch_tree: Vec<GitBranchNode>,
    pub git_commit_graph: Vec<GitCommitEntry>,
    pub git_worktree_list: Vec<String>,
    pub git_branch_cursor: usize,
    pub git_summary_lines: Vec<String>,
    pub git_view_data: Option<GitViewData>,
    pub git_diff: String,
}

/// Efficiency, cascade router, cfactor, experiments.
pub struct LearningViewState {
    pub efficiency_summary: EfficiencySummary,
    pub efficiency_events: Vec<AgentEfficiencyEvent>,
    pub efficiency_trend: Vec<EfficiencyBucket>,
    pub cfactor_trend_buckets: Vec<CFactorBucket>,
    pub cascade_router: CascadeRouterState,
    pub cascade_router_json: String,
    pub experiments: Vec<ExperimentSummary>,
    pub experiment_winners: Vec<ExperimentWinnerSummary>,
    pub gate_thresholds_json: String,
}

/// Unified log, episodes, signals.
pub struct LogViewState {
    pub cached_unified_log: Vec<LogEntry>,
    pub episodes_cache: Vec<Episode>,
    pub recent_signals: Vec<SignalSummary>,
}

/// Conductor alerts, cfactor snapshot, knowledge, event log.
pub struct InspectViewState {
    pub conductor_alerts: Vec<AlertSummary>,
    pub cfactor: Option<CFactor>,
    pub knowledge_entries: Vec<KnowledgeBrowseEntry>,
    pub event_log: Vec<DashboardEventLogEntry>,
}

/// Jobs, PRDs, forms, assign state.
pub struct MarketplaceState {
    pub marketplace_jobs: Vec<MarketplaceJob>,
    pub marketplace_selected_job: usize,
    pub atelier_prds: Vec<PrdSummary>,
    pub atelier_selected_prd: usize,
    pub atelier_tasks_by_slug: HashMap<String, Vec<TaskSummary>>,
    pub job_form_editing: bool,
    pub job_form_title: String,
    pub job_form_type: String,
    pub job_form_priority: String,
    pub job_form_description: String,
    pub job_form_focus: JobFormField,
    pub job_assign_editing: bool,
    pub job_assign_buffer: String,
    pub job_progress: HashMap<String, JobProgressEntry>,
    pub command_results: Vec<CommandResult>,
}

/// Config panel editing state.
pub struct ConfigEditorState {
    pub config_cursor: usize,
    pub config_pending: HashMap<String, String>,
    pub config_scroll_offset: usize,
    pub config_editing: bool,
    pub config_edit_buffer: String,
    pub config_edit_key: Option<String>,
}

/// Top-level orchestration chrome (status bar, phase, timing).
pub struct ChromeState {
    pub orchestrator_state: String,
    pub current_iteration: usize,
    pub current_phase: String,
    pub is_paused: bool,
    pub run_started: Option<Instant>,
    pub atmosphere: Atmosphere,
}

/// Private smoothed metric accumulators.
struct SmoothedMetrics {
    cpu_pct_smoothed: SmoothedValue,
    token_rate_smoothed: SmoothedValue,
    cost_rate_smoothed: SmoothedValue,
    last_rate_sample_at: Option<Instant>,
    last_token_total_sample: u64,
    last_cost_dollars_sample: f64,
}
```

#### Interface between sub-structs

Sub-structs are plain data bags with no cross-references. The parent `TuiState` provides
convenience accessors that combine data from multiple sub-structs:

```rust
impl TuiState {
    /// (done, total) task counts from plans sub-state.
    pub fn task_counts(&self) -> (usize, usize) {
        self.plans.task_counts()
    }

    /// Active agent count from agents sub-state.
    pub fn active_agent_count(&self) -> usize {
        self.agents.active_agent_count()
    }

    /// Sub-tab index for a given tab -- reads from nav.
    pub fn sub_tab_for(&self, tab: Tab) -> usize {
        self.nav.sub_tab_for(tab)
    }
}
```

#### How construction changes

Before:
```rust
let mut state = TuiState::default(); // 127 field defaults
state.update_from_snapshot(&data);
```

After:
```rust
let mut state = TuiState::default(); // delegates to sub-struct defaults
state.update_from_snapshot(&data);   // same API, internals dispatch to sub-structs
```

The `Default` impl becomes trivial because each sub-struct has its own focused `Default`.

#### How passing/borrowing changes

Before:
```rust
pub fn render_agents_view(f: &mut Frame, area: Rect, tui_state: &TuiState, theme: &Theme) {
    let agent = &tui_state.agents[tui_state.selected_agent];
    // 60 lines accessing tui_state.agents, tui_state.route_metrics, tui_state.token_history
}
```

After (Phase 1 -- facade preserved):
```rust
pub fn render_agents_view(f: &mut Frame, area: Rect, tui_state: &TuiState, theme: &Theme) {
    let agent = &tui_state.agents.agents[tui_state.nav.selected_agent];
    // access through sub-structs but signature unchanged
}
```

After (Phase 2 -- sub-struct passed directly):
```rust
pub fn render_agents_view(
    f: &mut Frame, area: Rect,
    agents: &AgentViewState, nav: &NavState, costs: &CostState,
    theme: &Theme,
) {
    let agent = &agents.agents[nav.selected_agent];
}
```

#### Thread safety implications

TuiState is used single-threaded in the TUI render loop. No sub-struct needs `Arc`/`Mutex`.
The only interior mutability (`RefCell<HashMap<...>>` for `agent_output_cache`) stays in
`AgentViewState` unchanged.

### 1.4 Migration Strategy

**Phase 1: Extract sub-structs, keep facade** (estimated: 2-3 days)

1. Create each sub-struct type in `tui/state.rs`.
2. Replace flat fields with sub-struct fields in `TuiState`.
3. `update_from_snapshot()` writes to sub-struct fields (e.g., `self.plans.plans = ...`).
4. Add `Deref`-style accessors or just update all `tui_state.field` to `tui_state.sub.field`.
5. All render function signatures stay as `&TuiState`.

**Phase 2: Update consumers to use sub-structs directly** (estimated: 3-4 days)

1. Change render function signatures to take specific sub-structs:
   - `render_agents_view(&AgentViewState, &NavState, &CostState, ...)` instead of `&TuiState`.
2. Update `app.rs` event handlers to borrow specific sub-structs.
3. Test each view independently with minimal sub-struct construction.

**Phase 3: Remove facade** (estimated: 1-2 days)

1. Remove any remaining `&TuiState` parameters from render functions.
2. Remove convenience accessors that just forward to sub-structs.
3. Consider splitting `state.rs` into `state/mod.rs` + per-sub-struct files.

### 1.5 Risk Analysis

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Borrow checker conflicts when passing multiple sub-struct refs from same TuiState | Medium | Blocks compilation | Use pattern `let TuiState { plans, agents, .. } = &state;` destructuring |
| `update_from_snapshot()` accesses many sub-structs at once | Low | Messy code | Keep `update_from_snapshot` as a method on `TuiState`, not on sub-structs |
| `app.rs` event handlers need `&mut` to multiple sub-structs | Medium | Borrow conflicts | Destructure into separate `&mut` borrows: `let (plans, agents) = (&mut state.plans, &mut state.agents);` |
| Breaking test fixtures that construct full TuiState | Low | Test churn | Sub-struct defaults mean test construction is simpler, not harder |
| Performance regression from additional indirection | Very Low | Negligible | Sub-structs are inline fields, no heap allocation change |

---

## 2. PlanRunner -- 80 Fields

**File**: `crates/roko-cli/src/orchestrate.rs` (23,269 lines, struct at line 2633)

### 2.1 Current Definition

```rust
pub struct PlanRunner {
    // -- Workspace (5 fields) --
    workdir: PathBuf,
    config: Config,
    no_replan: bool,
    skip_validate: bool,
    cloud_execution: Option<CloudExecution>,

    // -- Execution core (8 fields) --
    executor: ParallelExecutor,
    event_log: EventLog,
    agent_calls: usize,
    gate_runs: usize,
    worktrees: WorktreeManager,
    post_merge: PostMergeRunner,
    claude_resume_session: Option<String>,
    actions_since_save: usize,

    // -- Per-plan tracking (5 fields) --
    per_plan_agents: HashMap<String, usize>,
    per_plan_gates: HashMap<String, Vec<(String, bool)>>,
    per_plan_gate_summaries: HashMap<String, GateSummaryCounts>,
    task_trackers: HashMap<String, TaskTracker>,
    gemini_plan_caches: HashMap<String, GeminiPlanCache>,

    // -- Learning (7 fields) --
    learning: LearningRuntime,
    skill_library: SkillLibrary,
    playbook: PlaybookStore,
    feedback_service: FeedbackService,
    learning_config: RuntimeLearningConfig,
    learning_event_bus: LearningEventBus,
    efficiency_events: Vec<AgentEfficiencyEvent>,

    // -- Knowledge (3 fields) --
    knowledge_store: KnowledgeStore,
    knowledge_admission: Option<KnowledgeAdmissionStore>,
    search_client: Option<PerplexitySearchClient>,

    // -- Behavioral / affect (2 fields) --
    daimon: DaimonState,
    pheromone_field: Vec<Pheromone>,
    pheromone_gate_failures: HashMap<String, u32>,

    // -- Safety (3 fields) --
    safety_layer: SafetyLayer,
    health_monitor: HealthMonitor,
    anomaly_detector: AnomalyDetector,

    // -- Conductor (4 fields) --
    conductor: Arc<Conductor>,
    conductor_signals: Vec<Engram>,
    pending_coordination_patterns: Vec<CompoundPattern>,
    stuck_detector: StuckDetector,

    // -- Meta-cognition (3 fields) --
    meta_cognition_hook: MetaCognitionHook,
    last_agent_progress_ms: i64,
    retry_conductor: ConductorBandit,

    // -- Attribution / tracking (5 fields) --
    attribution_tracker: ContextAttributionTracker,
    context_average_tracker: ContextAverageTracker,
    crate_familiarity_tracker: CrateFamiliarityTracker,
    format_bandit: ProfileBandit,
    curriculum_scheduler: CurriculumScheduler,

    // -- Cost tracking (2 fields) --
    plan_costs: HashMap<String, f64>,
    task_costs: HashMap<String, f64>,

    // -- Infrastructure / lifecycle (4 fields) --
    supervisor: Arc<ProcessSupervisor>,
    cancel: CancelToken,
    agent_pool: MultiAgentPool,
    max_retries_override: Option<u32>,
    force_model_override: Option<String>,

    // -- MCP (4 fields) --
    mcp_server_names: Vec<String>,
    mcp_state: tokio::sync::Mutex<McpServerState>,
    tool_registry: Option<Arc<DynamicToolRegistry>>,

    // -- Observability (4 fields) --
    metrics: Arc<MetricRegistry>,
    obs_sinks: FsObservabilitySinks,
    health_probes: ProbeRegistry,
    custody_logger: CustodyLogger,

    // -- Gate infrastructure (5 fields) --
    adaptive_thresholds: AdaptiveThresholds,
    gate_artifacts: GateArtifactStore,
    gate_ratchet: GateRatchet,
    verdict_publisher: Option<VerdictPublisher>,
    replan_ledger: ReplanLedger,

    // -- Routing (2 fields) --
    latency_registry: LatencyRegistry,
    router_calibration: RouterCalibration,

    // -- Event buses / IO (5 fields) --
    runtime_event_bus: RuntimeEventBus<RokoEvent>,
    runtime_event_rx: broadcast::Receiver<RuntimeEventEnvelope<RokoEvent>>,
    server_event_bus: Option<BusSender<ServerEvent>>,
    state_hub_sender: Option<StateHubSender>,
    approval_tx: Option<mpsc::Sender<ApprovalRequest>>,

    // -- Chain (2 fields) --
    chain_client: Option<Arc<dyn ChainClient>>,
    chain_wallet: Option<Arc<dyn ChainWallet>>,

    // -- Code intelligence (1 field) --
    code_index_cache: Option<(Instant, WorkspaceIndex)>,

    // -- Extension hooks (1 field) --
    extension_chain: ExtensionChain,

    // -- Caches (1 field) --
    efficiency_cache: EfficiencyCache,
}
```

**Total: 80 fields** (all private, accessed only within `orchestrate.rs` and a few CLI files).

### 2.2 Usage Analysis

PlanRunner is referenced in **6 files**, but the vast majority of logic is in `orchestrate.rs`
itself (23,269 lines, 531 functions).

| File | Usage |
|---|---|
| `orchestrate.rs` | Definition, all 531 methods, 2 constructors (`from_plans_dir`, `from_snapshot`). |
| `lib.rs` | Creates PlanRunner via `from_plans_dir()` or `from_snapshot()`, calls `run()`. |
| `run.rs` | Creates PlanRunner for single `roko run` invocations. |
| `prompt_helpers.rs` | Takes `&PlanRunner` for context enrichment helpers. |
| `gate_runner.rs` | Takes `&PlanRunner` for gate execution context. |
| `explain.rs` | References PlanRunner type (minimal). |

**Cross-field access patterns** (analyzed from `self.` usage):

| Subsystem cluster | Fields | Usage count (approx.) |
|---|---|---|
| Learning (learning, skill_library, playbook, feedback_service) | 7 | ~56 accesses |
| Daimon (daimon, pheromone_*) | 3 | ~30 accesses |
| Safety (safety_layer, anomaly_detector) | 2 | ~25 accesses |
| Knowledge (knowledge_store, knowledge_admission, search_client) | 3 | ~20 accesses |
| Conductor (conductor, conductor_signals, stuck_detector, meta_cognition_hook) | 5 | ~25 accesses |
| Cost (plan_costs, task_costs) | 2 | ~20 accesses |
| Attribution (attribution_tracker, context_average_tracker, crate_familiarity) | 3 | ~10 accesses |
| Gate infra (adaptive_thresholds, gate_artifacts, gate_ratchet, verdict_publisher, replan_ledger) | 5 | ~27 accesses |
| Executor (executor, event_log, worktrees, task_trackers) | 5 | ~30 accesses |
| IO (server_event_bus, state_hub_sender, runtime_event_bus, approval_tx) | 5 | ~40 accesses |
| MCP (mcp_state, tool_registry, mcp_server_names) | 3 | ~15 accesses |
| Observability (metrics, obs_sinks, health_probes, custody_logger) | 4 | ~15 accesses |

**Critical observation**: Many methods access 2-3 subsystems simultaneously (e.g., dispatch
touches safety + learning + daimon + conductor + cost). This means decomposition needs to
carefully avoid creating borrow conflicts where `&mut subsystem_a` and `&mut subsystem_b` are
needed simultaneously from the same parent.

### 2.3 Decomposition Design

```rust
pub struct PlanRunner {
    pub(crate) workspace: WorkspaceCtx,           // 5 fields
    pub(crate) exec: ExecutionSubsystem,           // 8 fields
    pub(crate) plan_tracking: PlanTrackingState,   // 5 fields
    pub(crate) learning: LearningSuite,            // 7 fields
    pub(crate) knowledge: KnowledgeSubsystem,      // 3 fields
    pub(crate) affect: AffectSubsystem,            // 3 fields
    pub(crate) safety: SafetySubsystem,            // 3 fields
    pub(crate) conductor_sub: ConductorSubsystem,  // 5 fields
    pub(crate) meta: MetaCognitionSubsystem,       // 3 fields
    pub(crate) attribution: AttributionSubsystem,  // 5 fields
    pub(crate) costs: CostTracker,                 // 2 fields
    pub(crate) lifecycle: LifecycleSubsystem,      // 5 fields
    pub(crate) mcp: McpSubsystem,                  // 3 fields
    pub(crate) obs: ObservabilitySubsystem,        // 4 fields
    pub(crate) gate_infra: GateInfraSubsystem,     // 5 fields
    pub(crate) routing: RoutingSubsystem,          // 2 fields
    pub(crate) io: IoSubsystem,                    // 5 fields
    pub(crate) chain: ChainSubsystem,              // 2 fields
    pub(crate) code_intel: Option<CodeIntelCache>,  // 1 field
    pub(crate) extensions: ExtensionChain,          // 1 field
    pub(crate) efficiency_cache: EfficiencyCache,   // 1 field
}
```

#### Sub-struct definitions:

```rust
/// Workspace configuration and overrides.
pub(crate) struct WorkspaceCtx {
    pub workdir: PathBuf,
    pub config: Config,
    pub no_replan: bool,
    pub skip_validate: bool,
    pub cloud_execution: Option<CloudExecution>,
}

/// DAG executor and supporting execution state.
pub(crate) struct ExecutionSubsystem {
    pub executor: ParallelExecutor,
    pub event_log: EventLog,
    pub agent_calls: usize,
    pub gate_runs: usize,
    pub worktrees: WorktreeManager,
    pub post_merge: PostMergeRunner,
    pub claude_resume_session: Option<String>,
    pub actions_since_save: usize,
}

/// Per-plan bookkeeping.
pub(crate) struct PlanTrackingState {
    pub per_plan_agents: HashMap<String, usize>,
    pub per_plan_gates: HashMap<String, Vec<(String, bool)>>,
    pub per_plan_gate_summaries: HashMap<String, GateSummaryCounts>,
    pub task_trackers: HashMap<String, TaskTracker>,
    pub gemini_plan_caches: HashMap<String, GeminiPlanCache>,
}

/// Episode logging, skill library, playbooks, experiments.
pub(crate) struct LearningSuite {
    pub runtime: LearningRuntime,
    pub skill_library: SkillLibrary,
    pub playbook: PlaybookStore,
    pub feedback_service: FeedbackService,
    pub config: RuntimeLearningConfig,
    pub event_bus: LearningEventBus,
    pub efficiency_events: Vec<AgentEfficiencyEvent>,
}

/// Durable knowledge store and search.
pub(crate) struct KnowledgeSubsystem {
    pub store: KnowledgeStore,
    pub admission: Option<KnowledgeAdmissionStore>,
    pub search_client: Option<PerplexitySearchClient>,
}

/// Daimon affect engine and pheromone field.
pub(crate) struct AffectSubsystem {
    pub daimon: DaimonState,
    pub pheromone_field: Vec<Pheromone>,
    pub pheromone_gate_failures: HashMap<String, u32>,
}

/// Safety layer, health monitor, anomaly detection.
pub(crate) struct SafetySubsystem {
    pub layer: SafetyLayer,
    pub health_monitor: HealthMonitor,
    pub anomaly_detector: AnomalyDetector,
}

/// Conductor for anomaly detection, circuit breaking.
pub(crate) struct ConductorSubsystem {
    pub conductor: Arc<Conductor>,
    pub signals: Vec<Engram>,
    pub pending_patterns: Vec<CompoundPattern>,
    pub stuck_detector: StuckDetector,
}

/// Theta-cadence meta-cognition and retry policy.
pub(crate) struct MetaCognitionSubsystem {
    pub hook: MetaCognitionHook,
    pub last_agent_progress_ms: i64,
    pub retry_conductor: ConductorBandit,
}

/// Context attribution, familiarity, format selection, curriculum.
pub(crate) struct AttributionSubsystem {
    pub tracker: ContextAttributionTracker,
    pub averages: ContextAverageTracker,
    pub crate_familiarity: CrateFamiliarityTracker,
    pub format_bandit: ProfileBandit,
    pub curriculum: CurriculumScheduler,
}

/// Cumulative cost tracking per plan and task.
pub(crate) struct CostTracker {
    pub plan_costs: HashMap<String, f64>,
    pub task_costs: HashMap<String, f64>,
}

/// Process supervision, cancellation, agent pool, overrides.
pub(crate) struct LifecycleSubsystem {
    pub supervisor: Arc<ProcessSupervisor>,
    pub cancel: CancelToken,
    pub agent_pool: MultiAgentPool,
    pub max_retries_override: Option<u32>,
    pub force_model_override: Option<String>,
}

/// MCP server state, tool registry, server names.
pub(crate) struct McpSubsystem {
    pub server_names: Vec<String>,
    pub state: tokio::sync::Mutex<McpServerState>,
    pub tool_registry: Option<Arc<DynamicToolRegistry>>,
}

/// Metrics, traces, health probes, custody logging.
pub(crate) struct ObservabilitySubsystem {
    pub metrics: Arc<MetricRegistry>,
    pub sinks: FsObservabilitySinks,
    pub health_probes: ProbeRegistry,
    pub custody_logger: CustodyLogger,
}

/// Adaptive thresholds, artifacts, ratchet, verdict publisher, replan ledger.
pub(crate) struct GateInfraSubsystem {
    pub adaptive_thresholds: AdaptiveThresholds,
    pub artifacts: GateArtifactStore,
    pub ratchet: GateRatchet,
    pub verdict_publisher: Option<VerdictPublisher>,
    pub replan_ledger: ReplanLedger,
}

/// Latency tracking and lookahead calibration.
pub(crate) struct RoutingSubsystem {
    pub latency_registry: LatencyRegistry,
    pub calibration: RouterCalibration,
}

/// Event buses, state hub, approval channel.
pub(crate) struct IoSubsystem {
    pub runtime_bus: RuntimeEventBus<RokoEvent>,
    pub runtime_rx: broadcast::Receiver<RuntimeEventEnvelope<RokoEvent>>,
    pub server_bus: Option<BusSender<ServerEvent>>,
    pub state_hub: Option<StateHubSender>,
    pub approval_tx: Option<mpsc::Sender<ApprovalRequest>>,
}

/// On-chain client and wallet.
pub(crate) struct ChainSubsystem {
    pub client: Option<Arc<dyn ChainClient>>,
    pub wallet: Option<Arc<dyn ChainWallet>>,
}
```

#### Interface between sub-structs

Sub-structs do not reference each other. They are plain field groups owned by `PlanRunner`.
Methods on `PlanRunner` continue to access multiple sub-structs as needed:

```rust
impl PlanRunner {
    async fn dispatch_task(&mut self, plan_id: &str, task: &TaskDef) -> Result<AgentResult> {
        // Accesses workspace, safety, learning, affect, conductor, costs, io
        let affect = self.affect.daimon.query();
        if let Err(v) = self.safety.layer.pre_dispatch_check(...) { ... }
        let skill = self.learning.skill_library.select(...);
        // ...
    }
}
```

This works because Rust allows simultaneous `&mut` borrows of different fields of the same struct.

#### How construction changes

Before (from `from_plans_dir`):
```rust
Ok(Self {
    workdir: workdir.to_path_buf(),
    config,
    no_replan,
    skip_validate: false,
    executor,
    event_log: EventLog::default(),
    // ... 77 more fields ...
})
```

After:
```rust
Ok(Self {
    workspace: WorkspaceCtx { workdir: workdir.to_path_buf(), config, no_replan, skip_validate: false, cloud_execution: None },
    exec: ExecutionSubsystem { executor, event_log: EventLog::default(), agent_calls: 0, gate_runs: 0, worktrees, post_merge: PostMergeRunner::new(), claude_resume_session: None, actions_since_save: 0 },
    learning: LearningSuite { runtime: learning, skill_library, playbook, feedback_service, config: learning_config, event_bus: LearningEventBus::new(256), efficiency_events: Vec::new() },
    // ... grouped by subsystem (same data, better organization) ...
})
```

#### Thread safety implications

- `supervisor: Arc<ProcessSupervisor>` stays in `LifecycleSubsystem` (Arc-shared).
- `conductor: Arc<Conductor>` stays in `ConductorSubsystem` (Arc-shared).
- `metrics: Arc<MetricRegistry>` stays in `ObservabilitySubsystem` (Arc-shared).
- `mcp_state: tokio::sync::Mutex<McpServerState>` stays in `McpSubsystem`.
- No new `Arc`/`Mutex` needed. All existing synchronization patterns preserved.

### 2.4 Migration Strategy

**Phase 1: Extract sub-structs into orchestrate.rs** (estimated: 3-5 days)

1. Define all sub-structs at the top of `orchestrate.rs`.
2. Replace flat fields in `PlanRunner` with sub-struct fields.
3. Mechanical find-and-replace: `self.field` to `self.subsystem.field`.
   - `self.workdir` -> `self.workspace.workdir`
   - `self.executor` -> `self.exec.executor`
   - `self.daimon` -> `self.affect.daimon`
   - etc.
4. Update both constructors (`from_plans_dir` and `from_snapshot`).
5. Run `cargo build`, fix any borrow issues from simultaneous sub-struct access.

**Phase 2: Move sub-structs to separate modules** (estimated: 4-6 days)

1. Create `crates/roko-cli/src/orchestrate/` directory.
2. Move each sub-struct + its methods to a dedicated file:
   - `orchestrate/workspace.rs`
   - `orchestrate/execution.rs`
   - `orchestrate/learning.rs`
   - `orchestrate/knowledge.rs`
   - `orchestrate/affect.rs`
   - `orchestrate/safety.rs`
   - `orchestrate/conductor.rs`
   - etc.
3. `orchestrate/mod.rs` re-exports `PlanRunner` and wires the event loop.
4. Extract methods that only touch one subsystem into `impl` blocks on the sub-struct.

**Phase 3: Trait-based interfaces** (estimated: 3-4 days)

1. Define traits for subsystem boundaries (e.g., `trait SafetyCheck`, `trait CostAccounting`).
2. Change cross-subsystem calls to use trait methods.
3. Enable unit testing of subsystems in isolation with mock implementations.

### 2.5 Before/After Code Examples

**Before** (dispatch with `&mut self` accessing 6 subsystems):

```rust
async fn dispatch_agent_with(&mut self, plan_id: &str, task_def: &TaskDef) -> Result<AgentResult> {
    let affect_confidence = self.daimon.query().confidence;
    if let Err(v) = self.safety_layer.pre_dispatch_check(&prompt, &role) {
        return Err(anyhow!("safety violation: {v}"));
    }
    let skills = self.skill_library.select(&task_text, 5);
    let entries = self.knowledge_store.query(&task_text, 10)?;
    *self.plan_costs.entry(plan_id.to_string()).or_insert(0.0) += cost;
    self.conductor_signals.push(signal);
    // ...
}
```

**After** (same logic, sub-struct field paths):

```rust
async fn dispatch_agent_with(&mut self, plan_id: &str, task_def: &TaskDef) -> Result<AgentResult> {
    let affect_confidence = self.affect.daimon.query().confidence;
    if let Err(v) = self.safety.layer.pre_dispatch_check(&prompt, &role) {
        return Err(anyhow!("safety violation: {v}"));
    }
    let skills = self.learning.skill_library.select(&task_text, 5);
    let entries = self.knowledge.store.query(&task_text, 10)?;
    *self.costs.plan_costs.entry(plan_id.to_string()).or_insert(0.0) += cost;
    self.conductor_sub.signals.push(signal);
    // ...
}
```

### 2.6 Risk Analysis

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Borrow checker rejects simultaneous `&mut subsystem_a` and `&mut subsystem_b` | Low | Build failure | Rust allows disjoint field borrows. Only nested borrows through the same sub-struct path conflict. |
| Methods that touch 5+ subsystems become awkward | Medium | Code clarity | Keep these as methods on `PlanRunner` (not on sub-structs). The parent struct is the coordinator. |
| `from_snapshot()` constructor diverges from `from_plans_dir()` | Medium | Subtle bugs | Both constructors share initialization via helper functions per subsystem. |
| Breaking `prompt_helpers.rs` and `gate_runner.rs` | Low | Compile errors | These take `&PlanRunner` and only access a few fields. Update field paths mechanically. |
| 23K-line file remains large even after grouping | High | Still hard to navigate | Phase 2 (module extraction) addresses this. Phase 1 alone is insufficient. |
| `TaskTracker` (32 fields, nested in `HashMap`) is also a mini god struct | Medium | Missed decomposition | Consider splitting `TaskTracker` into `TaskProgress` + `TaskGateHistory` + `TaskDispatchHistory` as a follow-up. |

---

## 3. AppState -- 47 Fields

**File**: `crates/roko-serve/src/state.rs` (1,317 lines, struct at line 344)

### 3.1 Current Definition

```rust
pub struct AppState {
    // -- Core workspace (3 fields) --
    pub workdir: PathBuf,
    pub layout: RokoLayout,
    pub signal_store: SignalStore,

    // -- Lifecycle (4 fields) --
    pub cancel: CancelToken,
    pub started_at: Instant,
    pub metrics: Arc<MetricRegistry>,
    pub supervisor: Arc<ProcessSupervisor>,

    // -- Affect (1 field) --
    pub affect_engine: Mutex<DaimonState>,

    // -- Event / streaming (5 fields) --
    pub event_bus: EventBus<ServerEvent>,
    pub state_hub: SharedStateHub,
    pub sse_adapter: Arc<SseAdapter>,
    pub runtime_event_logger: Arc<JsonlLogger>,
    pub subscriptions: SubscriptionRegistry,

    // -- Runtime bridge (1 field) --
    pub runtime: Arc<dyn CliRuntime>,

    // -- Model / inference (4 fields) --
    pub model_call_service: Arc<ModelCallService>,
    pub roko_config: ArcSwap<RokoConfig>,
    pub provider_health: ProviderHealthTracker,
    pub latency_registry: LatencyRegistry,

    // -- Active operations (6 RwLock fields) --
    pub active_runs: RwLock<HashMap<String, RunHandle>>,
    pub active_plans: RwLock<HashMap<String, PlanHandle>>,
    pub operations: RwLock<HashMap<String, OperationHandle>>,
    pub templates: RwLock<TemplateRegistry>,
    pub template_runs: RwLock<HashMap<String, Vec<TemplateRunRecord>>>,
    pub deployments: RwLock<HashMap<String, Deployment>>,

    // -- Deploy (1 field) --
    pub deploy_backend: Arc<dyn DeployBackend>,

    // -- Security / scrubbing (2 fields) --
    pub scrubber: Arc<LogScrubber>,
    pub jwks_cache: Arc<JwksCache>,

    // -- HTTP / networking (1 field) --
    pub http_client: reqwest::Client,

    // -- Discovery / aggregation (3 fields) --
    pub discovered_agents: RwLock<HashMap<String, DiscoveredAgent>>,
    pub aggregator_cache: RwLock<HashMap<String, CachedJsonValue>>,
    pub heartbeats: RwLock<VecDeque<HeartbeatPayload>>,

    // -- Chain (2 fields) --
    pub chain_client: Option<Arc<AlloyChainClient>>,
    pub chain_wallet: Option<Arc<AlloyChainWallet>>,

    // -- Relay (2 fields) --
    pub agent_count: Arc<AtomicU32>,
    pub relay_health: Arc<parking_lot::RwLock<RelayHealth>>,

    // -- Integrations (2 fields) --
    pub connectors: RwLock<ConnectorRegistry>,
    pub feeds: RwLock<FeedRegistry>,

    // -- Gateway (3 fields) --
    pub cascade_router: RwLock<Option<CascadeRouter>>,
    pub gateway_model_counters: RwLock<HashMap<String, Arc<GatewayModelCounters>>>,
    pub batch_progress: RwLock<HashMap<String, Arc<BatchProgress>>>,

    // -- Terminal (1 field) --
    pub terminal_sessions: SessionManager,

    // -- Bench (2 fields) --
    pub active_bench_runs: RwLock<HashMap<String, BenchRunHandle>>,
    pub active_matrix_runs: RwLock<HashMap<String, MatrixRunHandle>>,

    // -- Workspaces (1 field) --
    pub ephemeral_workspaces: RwLock<HashMap<String, WorkspaceInfo>>,

    // -- Proxy (2 fields) --
    pub mirage_url: Option<String>,
    pub agent_relay_url: Option<String>,
}
```

**Total: 47 fields** (all public).

### 3.2 Usage Analysis

AppState is referenced in **69 files** (53 route handlers + 16 service/infrastructure files).
Every route handler receives `State<Arc<AppState>>` (axum shared state) and can access
any field.

**Access pattern by route domain:**

| Route domain | Files | Fields accessed |
|---|---|---|
| Plans / runs | `plans.rs`, `run.rs`, `runs.rs`, `shared_runs.rs` | workdir, layout, runtime, roko_config, supervisor, active_plans, active_runs, cancel, metrics, event_bus |
| Agents | `agents.rs` | runtime, supervisor, discovered_agents, active_plans, agent_count |
| Gateway / inference | `gateway.rs` | model_call_service, roko_config, cascade_router, gateway_model_counters, batch_progress, provider_health, latency_registry |
| Deployments | `deployments.rs` | deploy_backend, deployments, active_runs, active_plans |
| Templates | `templates.rs` | templates, template_runs, active_runs |
| Config / secrets | `config.rs`, `secrets.rs` | roko_config, layout, workdir |
| Status / health | `health.rs`, `metrics.rs`, `dashboard.rs`, `gates.rs`, `episodes.rs` | metrics, started_at, state_hub, layout, runtime, roko_config, supervisor, active_plans |
| SSE / WS | `sse.rs`, `ws.rs` | event_bus, state_hub, sse_adapter |
| Bench | `bench.rs` | workdir, runtime, roko_config, cancel, supervisor, metrics, active_bench_runs, active_matrix_runs |
| Research | `research.rs` | workdir, layout, runtime, roko_config |
| PRDs | `prds.rs` | workdir, layout, runtime, roko_config, active_runs, active_plans |
| Jobs | `jobs.rs` | workdir, layout, runtime, supervisor, cancel, metrics |
| Heartbeats | `heartbeats.rs` | heartbeats, agent_count |
| Aggregator | `aggregator.rs` | http_client, aggregator_cache, roko_config |
| Discovery | `agents.rs` | discovered_agents |
| Connectors / feeds | `connectors.rs`, `feeds.rs` | connectors, feeds |
| Learning | `learning/*.rs` | layout, roko_config, cascade_router |
| Proxies | `rpc_proxy.rs`, `relay_proxy.rs` | mirage_url, agent_relay_url, http_client |
| Terminal | `terminal.rs` | terminal_sessions |
| Auth / middleware | `auth.rs`, `middleware.rs` | jwks_cache, scrubber, roko_config |
| Dreams | `dreams.rs` | workdir, layout, runtime, affect_engine |
| Workflows | `workflows.rs` | workdir, layout, runtime, roko_config, supervisor |
| Event ingest | `event_ingest.rs` | runtime_event_logger, sse_adapter |
| Infrastructure | `lib.rs`, `dispatch.rs`, `job_runner.rs`, `feedback.rs`, `scheduler.rs` | Nearly all fields (these are internal wiring, not routes) |

**Key observation**: Route handlers typically touch 3-8 fields each. The gateway routes
never touch deployment fields. The deployment routes never touch gateway fields. But
infrastructure code (`dispatch.rs`, `job_runner.rs`) reaches across boundaries.

The lock acquisition order comment (lines 381-392) documents 17 RwLock fields, showing the
team is already aware of concurrency complexity from the flat structure.

### 3.3 Decomposition Design

```rust
pub struct AppState {
    pub core: CoreState,                  // 5 fields -- workspace, config, cancel, started
    pub runtime: Arc<dyn CliRuntime>,     // 1 field  -- CLI bridge
    pub events: EventSubsystem,           // 5 fields -- event bus, SSE, state hub, subscriptions
    pub inference: InferenceSubsystem,    // 7 fields -- model service, gateway, providers
    pub operations: OperationsSubsystem,  // 6 fields -- active runs/plans, templates, operations
    pub deploy: DeploySubsystem,          // 2 fields -- backend, deployments
    pub lifecycle: LifecycleServices,     // 3 fields -- supervisor, metrics, cancel
    pub security: SecuritySubsystem,      // 2 fields -- scrubber, JWKS
    pub discovery: DiscoverySubsystem,    // 3 fields -- agents, aggregator cache, heartbeats
    pub integrations: IntegrationsSubsystem, // 2 fields -- connectors, feeds
    pub chain: ChainSubsystem,            // 2 fields -- client, wallet
    pub relay: RelaySubsystem,            // 2 fields -- agent count, health
    pub bench: BenchSubsystem,            // 2 fields -- bench runs, matrix runs
    pub terminal: SessionManager,         // 1 field
    pub workspaces: RwLock<HashMap<String, WorkspaceInfo>>,  // 1 field
    pub proxy: ProxyConfig,               // 2 fields -- mirage_url, agent_relay_url
    pub affect: Mutex<DaimonState>,       // 1 field
}
```

#### Sub-struct definitions:

```rust
/// Workspace path, layout, config, signal store, timing.
pub struct CoreState {
    pub workdir: PathBuf,
    pub layout: RokoLayout,
    pub signal_store: SignalStore,
    pub roko_config: ArcSwap<RokoConfig>,
    pub started_at: Instant,
}

/// Event bus, SSE adapter, runtime event logger, state hub, subscriptions.
pub struct EventSubsystem {
    pub bus: EventBus<ServerEvent>,
    pub state_hub: SharedStateHub,
    pub sse_adapter: Arc<SseAdapter>,
    pub runtime_logger: Arc<JsonlLogger>,
    pub subscriptions: SubscriptionRegistry,
}

/// Model calls, cascade router, gateway counters, provider health, latency.
pub struct InferenceSubsystem {
    pub model_call_service: Arc<ModelCallService>,
    pub provider_health: ProviderHealthTracker,
    pub latency_registry: LatencyRegistry,
    pub cascade_router: RwLock<Option<CascadeRouter>>,
    pub gateway_counters: RwLock<HashMap<String, Arc<GatewayModelCounters>>>,
    pub batch_progress: RwLock<HashMap<String, Arc<BatchProgress>>>,
    pub http_client: reqwest::Client,
}

/// Active runs, plans, operations, templates, template runs.
pub struct OperationsSubsystem {
    pub active_runs: RwLock<HashMap<String, RunHandle>>,
    pub active_plans: RwLock<HashMap<String, PlanHandle>>,
    pub operations: RwLock<HashMap<String, OperationHandle>>,
    pub templates: RwLock<TemplateRegistry>,
    pub template_runs: RwLock<HashMap<String, Vec<TemplateRunRecord>>>,
    pub deployments: RwLock<HashMap<String, Deployment>>,
}

/// Deploy backend and deployment registry.
pub struct DeploySubsystem {
    pub backend: Arc<dyn DeployBackend>,
    pub deployments: RwLock<HashMap<String, Deployment>>,
}

/// Process supervisor, metrics, cancellation.
pub struct LifecycleServices {
    pub supervisor: Arc<ProcessSupervisor>,
    pub metrics: Arc<MetricRegistry>,
    pub cancel: CancelToken,
}

/// Log scrubber, JWKS cache.
pub struct SecuritySubsystem {
    pub scrubber: Arc<LogScrubber>,
    pub jwks_cache: Arc<JwksCache>,
}

/// Agent discovery, aggregator cache, heartbeats.
pub struct DiscoverySubsystem {
    pub discovered_agents: RwLock<HashMap<String, DiscoveredAgent>>,
    pub aggregator_cache: RwLock<HashMap<String, CachedJsonValue>>,
    pub heartbeats: RwLock<VecDeque<HeartbeatPayload>>,
}

/// Connectors and feeds.
pub struct IntegrationsSubsystem {
    pub connectors: RwLock<ConnectorRegistry>,
    pub feeds: RwLock<FeedRegistry>,
}

/// On-chain client and wallet.
pub struct ChainSubsystem {
    pub client: Option<Arc<AlloyChainClient>>,
    pub wallet: Option<Arc<AlloyChainWallet>>,
}

/// Relay health and agent count.
pub struct RelaySubsystem {
    pub agent_count: Arc<AtomicU32>,
    pub health: Arc<parking_lot::RwLock<RelayHealth>>,
}

/// Active bench runs and matrix runs.
pub struct BenchSubsystem {
    pub active_runs: RwLock<HashMap<String, BenchRunHandle>>,
    pub active_matrix: RwLock<HashMap<String, MatrixRunHandle>>,
}

/// Proxy endpoint URLs.
pub struct ProxyConfig {
    pub mirage_url: Option<String>,
    pub agent_relay_url: Option<String>,
}
```

#### How construction changes

Before:
```rust
Ok(Self {
    workdir,
    layout,
    signal_store: SignalStore::new(signal_root),
    cancel,
    // ... 43 more flat fields ...
})
```

After:
```rust
Ok(Self {
    core: CoreState { workdir, layout, signal_store: SignalStore::new(signal_root), roko_config: ArcSwap::from_pointee(roko_config), started_at: Instant::now() },
    runtime,
    events: EventSubsystem { bus: EventBus::new(16_384), state_hub, sse_adapter: Arc::new(SseAdapter::new(256)), runtime_logger, subscriptions },
    inference: InferenceSubsystem { model_call_service, provider_health: ProviderHealthTracker::new(), latency_registry: LatencyRegistry::new(), cascade_router: RwLock::new(None), gateway_counters: RwLock::new(HashMap::new()), batch_progress: RwLock::new(HashMap::new()), http_client },
    // ... grouped by subsystem ...
})
```

#### How passing changes

Before:
```rust
async fn list_plans(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let workdir = &state.workdir;
    let layout = &state.layout;
    let config = state.roko_config.load();
    // ...
}
```

After (Phase 1 -- facade preserved):
```rust
async fn list_plans(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let workdir = &state.core.workdir;
    let layout = &state.core.layout;
    let config = state.core.roko_config.load();
    // ...
}
```

After (Phase 2 -- extractor-based):
```rust
// Optional: axum `FromRequestParts` extractor per subsystem
async fn list_plans(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let core = &state.core;
    let ops = &state.operations;
    // Only access what you need
}
```

#### Thread safety implications

- All existing `RwLock` fields stay within their subsystem.
- Lock acquisition order must still be maintained across subsystems.
- The documented lock order (lines 381-392) maps cleanly to subsystem boundaries:
  - `operations` locks (1-6) are all in `OperationsSubsystem`.
  - `discovery` locks (7-8) are in `DiscoverySubsystem`.
  - `integrations` locks (10-11) are in `IntegrationsSubsystem`.
  - `inference` locks (13-15) are in `InferenceSubsystem`.
  - `bench` locks (16-17) are in `BenchSubsystem`.
- No new synchronization primitives needed.

### 3.4 Migration Strategy

**Phase 1: Extract sub-structs, keep facade** (estimated: 2-3 days)

1. Define sub-structs in `state.rs`.
2. Replace flat fields with sub-struct fields.
3. Update all 69 files: `state.field` -> `state.subsystem.field`.
4. Update lock order comment to reference subsystem names.
5. All constructor variants updated.

**Phase 2: Update route handlers** (estimated: 3-4 days)

1. Audit each route file to confirm it only accesses its declared subsystems.
2. Consider axum `FromRequestParts` extractors for common patterns.
3. Add integration test that validates subsystem isolation.

**Phase 3: Subsystem-specific `impl` blocks** (estimated: 2-3 days)

1. Move helper methods (e.g., `gateway_counters_for()`) to subsystem impls.
2. Consider splitting `state.rs` into `state/mod.rs` + per-subsystem files.

### 3.5 Risk Analysis

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| 69 files need field path updates | Certain | 1-2 days of mechanical changes | Use find-and-replace with review. Most are simple `state.x` -> `state.sub.x`. |
| Lock order violations become harder to spot | Low | Potential deadlocks | Update lock order docs per subsystem. Consider a `#[must_acquire_before]` lint. |
| Test helpers that construct full AppState | Medium | Test churn | Sub-struct defaults simplify construction; tests only need to populate their subsystem. |
| Infrastructure code (dispatch.rs, job_runner.rs) accesses many subsystems | Medium | Verbose code | Keep these as methods on `AppState` that coordinate across subsystems. |
| Performance from additional struct nesting | Very Low | Negligible | Structs are inline; no heap allocation change. |

---

## 4. TaskTracker -- 32 Fields (Secondary God Struct)

**File**: `crates/roko-cli/src/orchestrate.rs` (line 2797, nested in PlanRunner)

TaskTracker is not as large as the other three but at 32 fields it tracks three distinct
concerns in one type: task completion progress, gate/verification history, and dispatch metadata.

### 4.1 Current Definition (abbreviated)

```rust
struct TaskTracker {
    // Progress (7 fields)
    tasks_file: TasksFile,
    completed: Vec<String>,
    failed: Vec<String>,
    failure_reasons: HashMap<String, String>,
    skipped: Vec<String>,
    current_group_index: usize,
    ready_since_ms: HashMap<String, u64>,

    // Gate history (8 fields)
    _plan_dir: PathBuf,
    last_gate_failure: Option<String>,
    last_gate_failure_phase: Option<String>,
    last_gate_failure_rung: Option<u32>,
    last_gate_verdicts: Vec<GateVerdict>,
    last_gate_verdict_summaries: Vec<GateVerdictSummary>,
    last_review_verdict: Option<ReviewVerdictEvidence>,
    review_feedback: Option<String>,

    // Dispatch metadata (10+ fields)
    last_impl_task_id: Option<String>,
    last_impl_model_slug: Option<String>,
    last_dispatch_role_label: Option<String>,
    last_impl_output_hash: Option<ContentHash>,
    artifact_valid: Option<bool>,
    last_context_knowledge_ids: Vec<String>,
    impl_round: u32,
    // ... more fields
}
```

### 4.2 Proposed Split

```rust
struct TaskTracker {
    progress: TaskProgress,
    gate_history: TaskGateHistory,
    dispatch_meta: TaskDispatchMeta,
}
```

This is a lower-priority refactor but should be done as part of PlanRunner Phase 2.

---

## 5. DashboardData -- 42 Fields (Data-Fetch God Struct)

**File**: `crates/roko-cli/src/tui/dashboard.rs` (line 316)

DashboardData is the disk-read side of TuiState. It reads `.roko/` files and feeds
`TuiState::update_from_snapshot()`. At 42 fields, it mirrors much of TuiState's domain data.

Since DashboardData is internal to the TUI module and already serves as a data transfer object,
decomposing it would follow the same domain boundaries as TuiState. The sub-structs should
match: `PlanData`, `AgentData`, `GateData`, `LearningData`, etc.

This should be done **in parallel with** TuiState Phase 1 so the snapshot bridge
(`update_from_snapshot`) maps cleanly between matching sub-structs.

---

## 6. Consolidated Effort Estimate

| Struct | Phase 1 (sub-structs, facade) | Phase 2 (consumers) | Phase 3 (remove facade / traits) | Total |
|---|---|---|---|---|
| TuiState (127 fields, 26 consumers) | 2-3 days | 3-4 days | 1-2 days | **6-9 days** |
| PlanRunner (80 fields, 6 consumers, 23K LOC) | 3-5 days | 4-6 days | 3-4 days | **10-15 days** |
| AppState (47 fields, 69 consumers) | 2-3 days | 3-4 days | 2-3 days | **7-10 days** |
| TaskTracker (32 fields) | 1 day | 1 day | -- | **2 days** |
| DashboardData (42 fields) | 1-2 days (parallel with TuiState) | -- | -- | **1-2 days** |
| **Total** | **9-14 days** | **11-15 days** | **6-9 days** | **26-38 days** |

### Recommended Execution Order

1. **TuiState** first -- smallest blast radius (26 files, all in one module), easiest to test
   visually, highest return on cognitive load reduction per day invested.
2. **AppState** second -- 69 files but all mechanical `state.x` -> `state.sub.x` changes,
   well-documented lock ordering already exists.
3. **PlanRunner** last -- largest, most complex, requires module extraction to be worthwhile.
   Phase 1 alone provides limited benefit because the 23K LOC file remains.
4. **TaskTracker** and **DashboardData** in parallel with their parent struct decompositions.

### Dependencies Between Decompositions

- TuiState decomposition has **no dependencies** on the others.
- AppState decomposition has **no dependencies** on the others.
- PlanRunner decomposition is **independent** but any new fields added during the other
  refactors should follow the sub-struct pattern.
- DashboardData should be decomposed **together with** TuiState to keep the snapshot bridge clean.
