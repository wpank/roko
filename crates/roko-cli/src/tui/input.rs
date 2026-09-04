//! Input mode state machine and key dispatch for the interactive TUI.
//!
//! Provides the full `TuiAction` enum (matching Mori's action vocabulary),
//! `InputMode` state machine, `FocusZone` for panel focus, and the
//! `handle_key` dispatch function with modal intercept priority.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::modals::ModalState;
use super::tabs::Tab;

// ---------------------------------------------------------------------------
// InputMode
// ---------------------------------------------------------------------------

/// Modal input state. Determines how keystrokes are interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InputMode {
    /// Standard navigation: arrow keys, tab switching, selection.
    #[default]
    Normal,
    /// Free-text injection: typing a message to send to an agent.
    Inject,
    /// Filter mode: typing a filter string for logs/signals.
    Filter,
    /// Confirmation dialog: yes/no prompt for destructive actions.
    Confirm,
    /// Config text-edit mode: typing a value for a config field.
    ConfigEdit,
    /// Log search mode: typing a regex pattern for log search/filter (#217).
    LogSearch,
    /// Plan tree filter mode: typing a filter string for plan tree (#219).
    PlanFilter,
    /// Agent output search mode: typing a regex for agent output (#367).
    AgentOutputSearch,
}

impl InputMode {
    /// Short badge label shown in the status bar when a non-normal mode is active.
    /// Returns `None` for `Normal` and `Confirm` (which has its own modal).
    #[must_use]
    pub const fn badge_label(self) -> Option<&'static str> {
        match self {
            Self::Normal | Self::Confirm => None,
            Self::Inject => Some("INJECT"),
            Self::Filter => Some("FILTER"),
            Self::ConfigEdit => Some("EDIT"),
            Self::LogSearch => Some("SEARCH"),
            Self::PlanFilter => Some("FILTER"),
            Self::AgentOutputSearch => Some("SEARCH"),
        }
    }
}

// ---------------------------------------------------------------------------
// FocusZone
// ---------------------------------------------------------------------------

/// Which panel currently has keyboard focus in split-pane views.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FocusZone {
    /// Plan tree / plan list (left panel).
    #[default]
    PlanTree,
    /// Task progress list.
    TaskProgress,
    /// Agent output / log pane.
    AgentOutput,
    /// Command output / bottom pane.
    CommandOutput,
    /// Right detail panel.
    RightPanel,
    // -- per-tab zones for remaining tabs --
    /// Git tab: branch/commit list (left).
    GitBranches,
    /// Git tab: diff/detail pane (right).
    GitDetail,
    /// Logs tab: log list (main).
    LogList,
    /// Logs tab: log detail pane.
    LogDetail,
    /// Config tab: key list (left).
    ConfigKeys,
    /// Config tab: value editor (right).
    ConfigValues,
    /// Inspect tab: signal tree (left).
    InspectTree,
    /// Inspect tab: detail pane (right).
    InspectDetail,
    /// Marketplace tab: job list.
    MarketList,
    /// Marketplace tab: job detail.
    MarketDetail,
    /// Atelier tab: artifact list.
    AtelierList,
    /// Atelier tab: artifact detail.
    AtelierDetail,
    /// Learning tab: metric list.
    LearningMetrics,
    /// Learning tab: chart/detail pane.
    LearningDetail,
}

impl FocusZone {
    /// Short human-readable label for the breadcrumb trail.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::PlanTree => "Plans",
            Self::TaskProgress => "Tasks",
            Self::AgentOutput => "Output",
            Self::CommandOutput => "Commands",
            Self::RightPanel => "Detail",
            Self::GitBranches => "Branches",
            Self::GitDetail => "Detail",
            Self::LogList => "Log",
            Self::LogDetail => "Detail",
            Self::ConfigKeys => "Keys",
            Self::ConfigValues => "Values",
            Self::InspectTree => "Tree",
            Self::InspectDetail => "Detail",
            Self::MarketList => "Jobs",
            Self::MarketDetail => "Detail",
            Self::AtelierList => "PRDs",
            Self::AtelierDetail => "Detail",
            Self::LearningMetrics => "Metrics",
            Self::LearningDetail => "Detail",
        }
    }

    /// Cycle to the next focus zone.
    #[must_use]
    pub const fn next(self, tab: Tab) -> Self {
        match tab {
            Tab::Dashboard => match self {
                Self::PlanTree => Self::TaskProgress,
                Self::TaskProgress => Self::AgentOutput,
                Self::AgentOutput => Self::CommandOutput,
                Self::CommandOutput => Self::RightPanel,
                _ => Self::PlanTree,
            },
            Tab::Plans => match self {
                Self::PlanTree => Self::TaskProgress,
                Self::TaskProgress => Self::RightPanel,
                _ => Self::PlanTree,
            },
            Tab::Agents => match self {
                Self::AgentOutput => Self::RightPanel,
                _ => Self::AgentOutput,
            },
            Tab::Git => match self {
                Self::GitBranches => Self::GitDetail,
                _ => Self::GitBranches,
            },
            Tab::Logs => match self {
                Self::LogList => Self::LogDetail,
                _ => Self::LogList,
            },
            Tab::Config => match self {
                Self::ConfigKeys => Self::ConfigValues,
                _ => Self::ConfigKeys,
            },
            Tab::Inspect => match self {
                Self::InspectTree => Self::InspectDetail,
                Self::InspectDetail => Self::RightPanel,
                _ => Self::InspectTree,
            },
            Tab::Marketplace => match self {
                Self::MarketList => Self::MarketDetail,
                _ => Self::MarketList,
            },
            Tab::Atelier => match self {
                Self::AtelierList => Self::AtelierDetail,
                _ => Self::AtelierList,
            },
            Tab::Learning => match self {
                Self::LearningMetrics => Self::LearningDetail,
                _ => Self::LearningMetrics,
            },
        }
    }

    /// Cycle to the previous focus zone.
    #[must_use]
    pub const fn prev(self, tab: Tab) -> Self {
        match tab {
            Tab::Dashboard => match self {
                Self::PlanTree => Self::RightPanel,
                Self::TaskProgress => Self::PlanTree,
                Self::AgentOutput => Self::TaskProgress,
                Self::CommandOutput => Self::AgentOutput,
                _ => Self::CommandOutput,
            },
            Tab::Plans => match self {
                Self::PlanTree => Self::RightPanel,
                Self::TaskProgress => Self::PlanTree,
                _ => Self::TaskProgress,
            },
            Tab::Agents => match self {
                Self::AgentOutput => Self::RightPanel,
                _ => Self::AgentOutput,
            },
            Tab::Git => match self {
                Self::GitDetail => Self::GitBranches,
                _ => Self::GitDetail,
            },
            Tab::Logs => match self {
                Self::LogDetail => Self::LogList,
                _ => Self::LogDetail,
            },
            Tab::Config => match self {
                Self::ConfigValues => Self::ConfigKeys,
                _ => Self::ConfigValues,
            },
            Tab::Inspect => match self {
                Self::InspectTree => Self::RightPanel,
                Self::InspectDetail => Self::InspectTree,
                _ => Self::InspectDetail,
            },
            Tab::Marketplace => match self {
                Self::MarketDetail => Self::MarketList,
                _ => Self::MarketList,
            },
            Tab::Atelier => match self {
                Self::AtelierDetail => Self::AtelierList,
                _ => Self::AtelierList,
            },
            Tab::Learning => match self {
                Self::LearningDetail => Self::LearningMetrics,
                _ => Self::LearningMetrics,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// ConfirmAction
// ---------------------------------------------------------------------------

/// Destructive or significant actions that require user confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmAction {
    RestartAllPlans,
    RestartPhase,
    ResetSelectedPlan(String),
    ForceAdvance(String),
    ReverifyPlan(String),
    DiagnosePlan(String),
    RepairPlanPreserve(String),
    RepairPlanClean(String),
    SoftRetryPlan(String),
    GitReconcile,
    IngestTask { plan_num: usize, task_id: String },
    MergeBatchToMain { plan_id: String, branch: String },
    MergePlan { plan_id: String, branch: String },
    MergeAllDone { branches: Vec<String> },
}

impl std::fmt::Display for ConfirmAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RestartAllPlans => write!(f, "Restart all plans?"),
            Self::RestartPhase => write!(f, "Restart current phase?"),
            Self::ResetSelectedPlan(id) => write!(f, "Reset plan {id}?"),
            Self::ForceAdvance(id) => write!(f, "Force-advance plan {id}?"),
            Self::ReverifyPlan(id) => write!(f, "Re-verify plan {id}?"),
            Self::DiagnosePlan(id) => write!(f, "Diagnose plan {id}?"),
            Self::RepairPlanPreserve(id) => write!(f, "Repair plan {id} (preserve)?"),
            Self::RepairPlanClean(id) => write!(f, "Repair plan {id} (clean)?"),
            Self::SoftRetryPlan(id) => write!(f, "Soft-retry plan {id}?"),
            Self::GitReconcile => write!(f, "Reconcile git state?"),
            Self::IngestTask { plan_num, task_id } => {
                write!(f, "Ingest task {task_id} into plan {plan_num}?")
            }
            Self::MergeBatchToMain { plan_id, branch } => {
                write!(f, "Merge {branch} (plan {plan_id}) to main?")
            }
            Self::MergePlan { plan_id, branch } => {
                write!(f, "Merge plan {plan_id} branch {branch}?")
            }
            Self::MergeAllDone { branches } => {
                write!(f, "Merge {} completed branches to main?", branches.len())
            }
        }
    }
}

/// Logs tab filter levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogFilterLevel {
    Info,
    Warn,
    Error,
    Debug,
}

impl LogFilterLevel {
    #[must_use]
    pub const fn all() -> [Self; 4] {
        [Self::Info, Self::Warn, Self::Error, Self::Debug]
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Info => "INF",
            Self::Warn => "WRN",
            Self::Error => "ERR",
            Self::Debug => "DBG",
        }
    }
}

// ---------------------------------------------------------------------------
// TuiAction
// ---------------------------------------------------------------------------

/// Every possible user action the TUI can dispatch.
///
/// Matches Mori's action vocabulary so that key bindings, mouse events, and
/// programmatic triggers all flow through a single enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiAction {
    // -- lifecycle --
    Quit,
    QuitConfirmed,

    // -- tab navigation --
    SwitchTab(Tab),
    /// Switch to a sub-view within the current tab region (UI-04).
    /// The index is 0-based (number key `1` -> index 0, etc.).
    SwitchSubView(usize),

    // -- plan list navigation --
    SelectPlanUp,
    SelectPlanDown,
    /// Jump directly to the plan at the given 0-based index (number keys on Plans tab).
    SelectPlanByIndex(usize),
    TaskPickerUp,
    TaskPickerDown,

    // -- log scrolling --
    ScrollLogUp,
    ScrollLogDown,
    ScrollLogEnd,
    ToggleLogFilter(LogFilterLevel),
    ShowAllLogFilters,

    // -- agent tab --
    SwitchAgentTab(usize),
    ToggleAgentTopology,

    // -- approval --
    ApproveCommand,
    ApproveAll,
    RejectCommand,

    // -- inject mode --
    StartInject,
    SubmitInject,
    CancelInject,
    InputChar(char),
    InputBackspace,

    // -- help --
    ShowHelp,
    /// Cycle the visual-effects preset.
    CycleEffectsPreset,
    ToggleScreenPostFx,

    // -- focus --
    FocusNext,
    FocusPrev,
    ScrollFocusedUp,
    ScrollFocusedDown,
    ScrollPageUp,
    ScrollPageDown,
    ScrollFocusedHome,
    ScrollFocusedEnd,

    // -- expand / collapse --
    ExpandCollapse,

    // -- plan detail modal --
    ShowPlanDetail,
    ClosePlanDetail,
    ScrollDetailUp,
    ScrollDetailDown,
    ModalScrollUp,
    ModalScrollDown,
    QueueOverviewUp,
    QueueOverviewDown,
    CloseModal,

    // -- agent scrolling --
    ScrollAgentUp,
    ScrollAgentDown,
    ScrollAgentEnd,

    // -- diff scrolling --
    ScrollDiffUp,
    ScrollDiffDown,

    // -- plan operations --
    RestartPhase,
    RestartPlan,

    // -- detail tabs --
    SwitchDetailTab(usize),

    // -- agent pane --
    ToggleAgentPaneGroup,

    // -- notifications --
    DismissNotification,
    ShowNotificationHistory,
    /// Toggle a notification history level filter (1=info, 2=warn, 3=error, 4=debug).
    NotifFilterToggle(u8),
    /// Jump to the related run/task of the selected notification.
    NotifJumpToRelated,
    /// Page up in the notification history modal.
    NotifPageUp,
    /// Page down in the notification history modal.
    NotifPageDown,
    /// Jump to the top of the notification history.
    NotifHome,
    /// Jump to the bottom of the notification history.
    NotifEnd,

    // -- config editor --
    ConfigUp,
    ConfigDown,
    ConfigToggle,
    ConfigCycleLeft,
    ConfigCycleRight,
    ConfigCommitEdit,
    ConfigCancelEdit,
    ConfigSave,
    /// Re-parse `roko.toml` into the config editor cache immediately.
    ConfigReload,

    // -- force / reset --
    ForceAdvance,
    ResetPlanState,
    ReverifyPlan,

    // -- confirm dialog --
    RequestConfirm(ConfirmAction),
    ConfirmYes,
    ConfirmNo,

    // -- pause --
    TogglePause,

    // -- wave / queue overviews --
    ShowWaveOverview,
    ShowQueueOverview,

    // -- filter mode --
    StartFilter,
    AcceptFilter,
    CancelFilter,

    // -- log search (#217) --
    /// Enter log search mode (displays search input bar).
    StartLogSearch,
    /// Accept current search pattern and stay in highlight/filter mode.
    AcceptLogSearch,
    /// Cancel search and clear pattern.
    CancelLogSearch,
    /// Jump to the next search match.
    NextLogMatch,
    /// Jump to the previous search match.
    PrevLogMatch,
    /// Toggle between highlight and filter mode for log search.
    ToggleLogFilterMode,
    /// Yank (copy) the currently selected log entry text.
    YankLogEntry,

    // -- plan tree filter (#219) --
    /// Enter plan tree filter mode on F2:Plans tab.
    StartPlanFilter,
    /// Accept plan tree filter and stay filtered.
    AcceptPlanFilter,
    /// Cancel plan tree filter and clear.
    CancelPlanFilter,

    // -- agent output search (#367) --
    /// Enter agent output search mode on F3:Agents tab.
    StartAgentOutputSearch,
    /// Accept current agent output search pattern.
    AcceptAgentOutputSearch,
    /// Cancel agent output search and clear pattern.
    CancelAgentOutputSearch,
    /// Jump to the next agent output search match.
    NextAgentOutputMatch,
    /// Jump to the previous agent output search match.
    PrevAgentOutputMatch,
    /// Toggle fold/unfold for the nearest tool result in agent output.
    ToggleAgentOutputFold,

    // -- recovery (#119) --
    /// Soft retry: re-dispatch only failed tasks.
    SoftRetry,
    /// Open diagnose modal showing error details.
    DiagnoseSelected,
    /// Repair with error context: re-dispatch with error info injected.
    RepairWithContext,
    /// Reverify gates only: skip agent, run gate pipeline.
    ReverifyGatesOnly,

    // -- task detail modal --
    ShowTaskDetail,
    CloseTaskDetail,

    // -- task picker modal --
    OpenTaskPicker,
    CloseTaskPicker,

    // -- drill navigation --
    DrillIn,
    DrillOut,

    // -- wave navigation --
    WaveNext,
    WavePrev,

    // -- mouse events --
    MouseClick {
        x: u16,
        y: u16,
    },
    MouseScrollUp {
        x: u16,
        y: u16,
    },
    MouseScrollDown {
        x: u16,
        y: u16,
    },

    // -- refresh --
    Refresh,

    // -- marketplace job form --
    SubmitJob,

    // -- cost table sort --
    CycleCostSort,

    // -- welcome modal --
    WelcomeInit,
    WelcomeDismiss,

    // -- no-op --
    None,
}

// ---------------------------------------------------------------------------
// Key dispatch
// ---------------------------------------------------------------------------

/// Top-level key dispatch with exhaustive 4-step priority (#365).
///
/// Priority order (highest first):
/// 1. Ctrl-C emergency quit (always, regardless of mode or modal).
/// 2. Non-normal `InputMode` handler (text input captures keystrokes).
/// 3. Active modal handler (when mode is `Normal` and a modal is open).
/// 4. Global keys, then per-tab keys.
pub fn handle_key(
    key: KeyEvent,
    mode: InputMode,
    active_tab: Tab,
    focus: FocusZone,
    modals: &ModalVisibility,
) -> TuiAction {
    // Step 1: Ctrl-C emergency quit — always takes precedence.
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return TuiAction::QuitConfirmed;
    }

    // Step 2: Non-normal InputMode handlers consume keystrokes first.
    // When a text-input mode is active the user is typing; all keys belong
    // to that mode until it is dismissed.
    match mode {
        InputMode::Normal => {} // fall through to step 3
        InputMode::Confirm => return handle_confirm_key(key),
        InputMode::Inject => return handle_inject_key(key),
        InputMode::Filter => return handle_filter_key(key),
        InputMode::ConfigEdit => return handle_config_edit_key(key),
        InputMode::LogSearch => return handle_log_search_key(key),
        InputMode::PlanFilter => return handle_plan_filter_key(key),
        InputMode::AgentOutputSearch => return handle_agent_output_search_key(key),
    }

    // Step 2b: F-keys always switch tabs, even when a modal is open.
    // Only Ctrl-C (step 1) and text-input modes (step 2) take precedence.
    if let Some(tab) = Tab::from_key(key.code) {
        return TuiAction::SwitchTab(tab);
    }

    // Step 3: Active modal handler (Normal mode only).
    // Every ModalState variant MUST have an explicit arm -- no catch-all -- so the
    // compiler rejects any future variant added without a handler.
    if let Some(modal) = modals.active_modal {
        return match modal {
            ModalState::Help => handle_help_key(key),
            ModalState::Approval { .. } => handle_approval_key(key),
            ModalState::WaveOverview { .. } => handle_wave_overview_key(key),
            ModalState::PlanDetail { .. } => handle_plan_detail_key(key),
            ModalState::TaskPicker { .. } => handle_task_picker_key(key),
            ModalState::TaskDetail { .. } => handle_task_detail_key(key),
            ModalState::QueueOverview { .. } => handle_queue_overview_key(key),
            ModalState::AgentPool { .. } => handle_agent_pool_key(key),
            ModalState::Quit | ModalState::Confirm { .. } => handle_confirm_key(key),
            ModalState::Inject { .. } => handle_inject_key(key),
            ModalState::BatchReview { .. } => handle_batch_review_key(key),
            ModalState::NotificationHistory { .. } => handle_notification_history_key(key),
            ModalState::Welcome { initialized } => handle_welcome_key(key, *initialized),
        };
    }

    // Step 4: Global keys that work in any tab, then per-tab dispatch.
    if let Some(action) = handle_global_key(key, active_tab) {
        return action;
    }

    match active_tab {
        Tab::Dashboard => handle_dashboard_key(key, focus),
        Tab::Plans => handle_plans_key(key, focus),
        Tab::Agents => handle_agents_key(key, focus),
        Tab::Git => handle_git_key(key, focus),
        Tab::Logs => handle_logs_key(key, focus),
        Tab::Config => handle_config_key(key),
        Tab::Inspect => handle_inspect_key(key, focus),
        Tab::Marketplace => handle_marketplace_key(key, focus),
        Tab::Atelier => handle_atelier_key(key, focus),
        Tab::Learning => handle_learning_key(key),
    }
}

/// Active modal reference used by key dispatch.
#[derive(Debug, Clone, Copy, Default)]
pub struct ModalVisibility<'a> {
    pub active_modal: Option<&'a ModalState>,
}

impl<'a> ModalVisibility<'a> {
    #[must_use]
    pub fn from_active_modal(active_modal: Option<&'a ModalState>) -> Self {
        Self { active_modal }
    }
}

// ---------------------------------------------------------------------------
// Modal key handlers
// ---------------------------------------------------------------------------

fn handle_help_key(key: KeyEvent) -> TuiAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('?' | 'q') => TuiAction::ShowHelp,
        KeyCode::Up | KeyCode::Char('k') => TuiAction::ScrollFocusedUp,
        KeyCode::Down | KeyCode::Char('j') => TuiAction::ScrollFocusedDown,
        KeyCode::PageUp => TuiAction::ScrollPageUp,
        KeyCode::PageDown => TuiAction::ScrollPageDown,
        KeyCode::Home => TuiAction::ScrollFocusedHome,
        KeyCode::End => TuiAction::ScrollFocusedEnd,
        _ => TuiAction::None,
    }
}

fn handle_approval_key(key: KeyEvent) -> TuiAction {
    match key.code {
        KeyCode::Char('y' | 'Y') | KeyCode::Enter => TuiAction::ApproveCommand,
        KeyCode::Char('n' | 'N') | KeyCode::Esc => TuiAction::RejectCommand,
        KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            TuiAction::ApproveAll
        }
        KeyCode::Char('A') => TuiAction::ApproveAll,
        _ => TuiAction::None,
    }
}

fn handle_wave_overview_key(key: KeyEvent) -> TuiAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('w') => TuiAction::ShowWaveOverview,
        KeyCode::Up | KeyCode::Char('k') => TuiAction::ModalScrollUp,
        KeyCode::Down | KeyCode::Char('j') => TuiAction::ModalScrollDown,
        _ => TuiAction::None,
    }
}

fn handle_plan_detail_key(key: KeyEvent) -> TuiAction {
    match key.code {
        KeyCode::Esc => TuiAction::ClosePlanDetail,
        KeyCode::Up | KeyCode::Char('k') => TuiAction::ScrollDetailUp,
        KeyCode::Down | KeyCode::Char('j') => TuiAction::ScrollDetailDown,
        KeyCode::Tab => TuiAction::SwitchDetailTab(0), // next detail sub-tab
        KeyCode::PageUp => TuiAction::ScrollPageUp,
        KeyCode::PageDown => TuiAction::ScrollPageDown,
        _ => TuiAction::None,
    }
}

fn handle_task_picker_key(key: KeyEvent) -> TuiAction {
    match key.code {
        KeyCode::Esc => TuiAction::CloseTaskPicker,
        KeyCode::Enter => TuiAction::ShowTaskDetail,
        KeyCode::Up | KeyCode::Char('k') => TuiAction::TaskPickerUp,
        KeyCode::Down | KeyCode::Char('j') => TuiAction::TaskPickerDown,
        _ => TuiAction::None,
    }
}

fn handle_task_detail_key(key: KeyEvent) -> TuiAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => TuiAction::CloseTaskDetail,
        KeyCode::Up | KeyCode::Char('k') => TuiAction::ScrollDetailUp,
        KeyCode::Down | KeyCode::Char('j') => TuiAction::ScrollDetailDown,
        KeyCode::Tab => TuiAction::SwitchDetailTab(0), // next detail sub-tab
        KeyCode::PageUp => TuiAction::ScrollPageUp,
        KeyCode::PageDown => TuiAction::ScrollPageDown,
        _ => TuiAction::None,
    }
}

fn handle_queue_overview_key(key: KeyEvent) -> TuiAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => TuiAction::ShowQueueOverview, // toggle off
        KeyCode::Up | KeyCode::Char('k') => TuiAction::QueueOverviewUp,
        KeyCode::Down | KeyCode::Char('j') => TuiAction::QueueOverviewDown,
        _ => TuiAction::None,
    }
}

fn handle_agent_pool_key(key: KeyEvent) -> TuiAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => TuiAction::CloseModal,
        KeyCode::Up | KeyCode::Char('k') => TuiAction::ModalScrollUp,
        KeyCode::Down | KeyCode::Char('j') => TuiAction::ModalScrollDown,
        _ => TuiAction::None,
    }
}

fn handle_confirm_key(key: KeyEvent) -> TuiAction {
    match key.code {
        KeyCode::Char('y' | 'Y') | KeyCode::Enter => TuiAction::ConfirmYes,
        KeyCode::Char('n' | 'N') | KeyCode::Esc => TuiAction::ConfirmNo,
        _ => TuiAction::None,
    }
}

fn handle_notification_history_key(key: KeyEvent) -> TuiAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => TuiAction::CloseModal,
        KeyCode::Up | KeyCode::Char('k') => TuiAction::ModalScrollUp,
        KeyCode::Down | KeyCode::Char('j') => TuiAction::ModalScrollDown,
        KeyCode::PageUp => TuiAction::NotifPageUp,
        KeyCode::PageDown => TuiAction::NotifPageDown,
        KeyCode::Home => TuiAction::NotifHome,
        KeyCode::End => TuiAction::NotifEnd,
        KeyCode::Char('1') => TuiAction::NotifFilterToggle(1),
        KeyCode::Char('2') => TuiAction::NotifFilterToggle(2),
        KeyCode::Char('3') => TuiAction::NotifFilterToggle(3),
        KeyCode::Char('4') => TuiAction::NotifFilterToggle(4),
        KeyCode::Enter => TuiAction::NotifJumpToRelated,
        _ => TuiAction::None,
    }
}

fn handle_welcome_key(key: KeyEvent, initialized: bool) -> TuiAction {
    if initialized {
        // After init, any key dismisses
        return TuiAction::WelcomeDismiss;
    }
    match key.code {
        KeyCode::Enter => TuiAction::WelcomeInit,
        KeyCode::Esc | KeyCode::Char('q') => TuiAction::WelcomeDismiss,
        _ => TuiAction::None,
    }
}

fn handle_batch_review_key(key: KeyEvent) -> TuiAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => TuiAction::CloseModal,
        KeyCode::Up | KeyCode::Char('k') => TuiAction::ModalScrollUp,
        KeyCode::Down | KeyCode::Char('j') => TuiAction::ModalScrollDown,
        KeyCode::Char('a') => TuiAction::ConfirmYes, // approve
        KeyCode::Char('r') => TuiAction::ConfirmNo,  // reject
        KeyCode::Char('s') => TuiAction::CloseModal, // skip (dismiss without action)
        _ => TuiAction::None,
    }
}

fn handle_inject_key(key: KeyEvent) -> TuiAction {
    match key.code {
        KeyCode::Enter => TuiAction::SubmitInject,
        KeyCode::Esc => TuiAction::CancelInject,
        KeyCode::Backspace => TuiAction::InputBackspace,
        KeyCode::Char(c) => TuiAction::InputChar(c),
        _ => TuiAction::None,
    }
}

fn handle_filter_key(key: KeyEvent) -> TuiAction {
    match key.code {
        KeyCode::Enter => TuiAction::AcceptFilter,
        KeyCode::Esc => TuiAction::CancelFilter,
        KeyCode::Backspace => TuiAction::InputBackspace,
        KeyCode::Char(c) => TuiAction::InputChar(c),
        _ => TuiAction::None,
    }
}

/// Key handler for log search mode (#217).
fn handle_log_search_key(key: KeyEvent) -> TuiAction {
    match key.code {
        KeyCode::Enter => TuiAction::AcceptLogSearch,
        KeyCode::Esc => TuiAction::CancelLogSearch,
        KeyCode::Backspace => TuiAction::InputBackspace,
        KeyCode::Char(c) => TuiAction::InputChar(c),
        _ => TuiAction::None,
    }
}

/// Key handler for agent output search mode (#367).
fn handle_agent_output_search_key(key: KeyEvent) -> TuiAction {
    match key.code {
        KeyCode::Enter => TuiAction::AcceptAgentOutputSearch,
        KeyCode::Esc => TuiAction::CancelAgentOutputSearch,
        KeyCode::Backspace => TuiAction::InputBackspace,
        KeyCode::Char(c) => TuiAction::InputChar(c),
        _ => TuiAction::None,
    }
}

/// Key handler for plan tree filter mode (#219).
fn handle_plan_filter_key(key: KeyEvent) -> TuiAction {
    match key.code {
        KeyCode::Enter => TuiAction::AcceptPlanFilter,
        KeyCode::Esc => TuiAction::CancelPlanFilter,
        KeyCode::Backspace => TuiAction::InputBackspace,
        KeyCode::Char(c) => TuiAction::InputChar(c),
        _ => TuiAction::None,
    }
}

// ---------------------------------------------------------------------------
// Global keys
// ---------------------------------------------------------------------------

fn handle_global_key(key: KeyEvent, active_tab: Tab) -> Option<TuiAction> {
    // F-keys switch tabs
    if let Some(tab) = Tab::from_key(key.code) {
        return Some(TuiAction::SwitchTab(tab));
    }

    // Number keys 1-9 switch top-level tabs (same as F1-F9), but only when
    // the active tab does NOT use number keys for its own purpose (e.g.
    // Agents uses 1-7 for agent sub-tabs, Logs uses 1-4 for filter levels,
    // Plans uses 1-9 for direct plan selection).
    // 0 switches to F10 (Learning). Plain digit press, no modifiers.
    let tab_uses_numbers = matches!(active_tab, Tab::Agents | Tab::Logs | Tab::Plans);
    if key.modifiers.is_empty() && !tab_uses_numbers {
        let tab = match key.code {
            KeyCode::Char('1') => Some(Tab::Dashboard),
            KeyCode::Char('2') => Some(Tab::Plans),
            KeyCode::Char('3') => Some(Tab::Agents),
            KeyCode::Char('4') => Some(Tab::Git),
            KeyCode::Char('5') => Some(Tab::Logs),
            KeyCode::Char('6') => Some(Tab::Config),
            KeyCode::Char('7') => Some(Tab::Inspect),
            KeyCode::Char('8') => Some(Tab::Marketplace),
            KeyCode::Char('9') => Some(Tab::Atelier),
            KeyCode::Char('0') => Some(Tab::Learning),
            _ => None,
        };
        if let Some(tab) = tab {
            return Some(TuiAction::SwitchTab(tab));
        }
    }

    // Alt+number switches sub-views within the current tab (UI-04).
    if key.modifiers.contains(KeyModifiers::ALT) {
        if let KeyCode::Char(c @ '1'..='9') = key.code {
            let index = (c as usize) - ('1' as usize); // 0-based
            return Some(TuiAction::SwitchSubView(index));
        }
    }

    // Ctrl-n: open notification history modal (before plain `n` consumes the key).
    if key.code == KeyCode::Char('n') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(TuiAction::ShowNotificationHistory);
    }

    match key.code {
        // `q` is global quit, except on Plans tab where it opens queue overview.
        KeyCode::Char('q') if active_tab != Tab::Plans => Some(TuiAction::Quit),
        KeyCode::Char('?') => Some(TuiAction::ShowHelp),
        // `n` dismisses notifications globally, except on Logs tab where
        // it navigates to the next search match.
        KeyCode::Char('n') if active_tab != Tab::Logs => Some(TuiAction::DismissNotification),
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(TuiAction::Refresh)
        }
        // Ctrl-a: approve all pending
        KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(TuiAction::ApproveAll)
        }
        // Ctrl-t: toggle the agent-topology panel from anywhere.
        KeyCode::Char('t') if is_ctrl_t(key) => Some(TuiAction::ToggleAgentTopology),
        // Ctrl-x: force advance (confirm)
        KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(TuiAction::ForceAdvance)
        }
        // Ctrl-d: reset selected plan (confirm)
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(TuiAction::ResetPlanState)
        }
        // Ctrl-e: toggle full-screen post-processing
        KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(TuiAction::ToggleScreenPostFx)
        }
        // v: verify/reverify (mori parity); effects cycling via Ctrl-E only
        KeyCode::Char('v') => Some(TuiAction::ReverifyPlan),
        // Ctrl-g: reconcile git state (confirm)
        KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(TuiAction::RequestConfirm(ConfirmAction::GitReconcile))
        }
        // u: queue overview (F8 switches to the Marketplace tab, not this).
        KeyCode::Char('u') => Some(TuiAction::ShowQueueOverview),
        KeyCode::Tab => Some(TuiAction::FocusNext),
        KeyCode::BackTab => Some(TuiAction::FocusPrev),
        _ => Option::None,
    }
}

/// Returns `true` when the key event matches the global `Ctrl+T` shortcut.
#[must_use]
pub(crate) fn is_ctrl_t(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('t'))
        && key.modifiers.contains(KeyModifiers::CONTROL)
        && !key.modifiers.contains(KeyModifiers::ALT)
        && !key.modifiers.contains(KeyModifiers::SHIFT)
        && !key.modifiers.contains(KeyModifiers::SUPER)
}

// ---------------------------------------------------------------------------
// Per-tab key handlers
// ---------------------------------------------------------------------------

fn handle_dashboard_key(key: KeyEvent, focus: FocusZone) -> TuiAction {
    match key.code {
        // Navigation — focus-aware
        KeyCode::Up | KeyCode::Char('k') => match focus {
            FocusZone::PlanTree => TuiAction::SelectPlanUp,
            FocusZone::AgentOutput => TuiAction::ScrollAgentUp,
            _ => TuiAction::ScrollFocusedUp,
        },
        KeyCode::Down | KeyCode::Char('j') => match focus {
            FocusZone::PlanTree => TuiAction::SelectPlanDown,
            FocusZone::AgentOutput => TuiAction::ScrollAgentDown,
            _ => TuiAction::ScrollFocusedDown,
        },
        KeyCode::PageUp => TuiAction::ScrollPageUp,
        KeyCode::PageDown => TuiAction::ScrollPageDown,
        KeyCode::Home => TuiAction::ScrollFocusedHome,
        KeyCode::End => TuiAction::ScrollFocusedEnd,

        // Plan tree operations
        KeyCode::Enter => TuiAction::ShowPlanDetail,
        KeyCode::Esc => TuiAction::ClosePlanDetail,
        KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => TuiAction::WavePrev,
        KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => TuiAction::WaveNext,
        KeyCode::Left | KeyCode::Char('h') => TuiAction::DrillOut,
        KeyCode::Right | KeyCode::Char('l') => TuiAction::DrillIn,

        // Sub-tab switching (a/o/d/e/g/m/L/P)
        KeyCode::Char('a') => TuiAction::SwitchDetailTab(0), // Agents
        KeyCode::Char('o') => TuiAction::SwitchDetailTab(1), // Output
        KeyCode::Char('d') => TuiAction::SwitchDetailTab(2), // Diff
        KeyCode::Char('e') => TuiAction::SwitchDetailTab(3), // Errors
        KeyCode::Char('g') => TuiAction::SwitchDetailTab(4), // Git
        KeyCode::Char('m') => TuiAction::SwitchDetailTab(5), // MCP
        KeyCode::Char('L') => TuiAction::SwitchDetailTab(6), // Learning
        KeyCode::Char('P') => TuiAction::SwitchDetailTab(7), // Processes
        KeyCode::Char('C') => TuiAction::SwitchDetailTab(8), // Conductor

        // Modal triggers
        KeyCode::Char('w') => TuiAction::ShowWaveOverview,
        KeyCode::Char('p') => TuiAction::TogglePause,
        KeyCode::Char('i') => TuiAction::StartInject,
        KeyCode::Char('y') => TuiAction::ApproveCommand,
        // Agent role tabs (backtick cycles, Alt+N selects)
        KeyCode::Char('`') => TuiAction::SwitchAgentTab(usize::MAX), // cycle

        _ => TuiAction::None,
    }
}

fn handle_plans_key(key: KeyEvent, focus: FocusZone) -> TuiAction {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => match focus {
            FocusZone::PlanTree => TuiAction::SelectPlanUp,
            _ => TuiAction::ScrollFocusedUp,
        },
        KeyCode::Down | KeyCode::Char('j') => match focus {
            FocusZone::PlanTree => TuiAction::SelectPlanDown,
            _ => TuiAction::ScrollFocusedDown,
        },
        KeyCode::Enter => TuiAction::ShowPlanDetail,
        KeyCode::Esc => TuiAction::ClosePlanDetail,
        KeyCode::Char('e') => TuiAction::ExpandCollapse,
        KeyCode::Char('w') => TuiAction::ShowWaveOverview,
        KeyCode::Char('o') => TuiAction::ShowQueueOverview,
        KeyCode::Char('q') => TuiAction::ShowQueueOverview, // queue overview (global quit suppressed on Plans)
        KeyCode::Char('t') => TuiAction::OpenTaskPicker,
        KeyCode::Char('[') => TuiAction::WavePrev,
        KeyCode::Char(']') => TuiAction::WaveNext,
        KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => TuiAction::WavePrev,
        KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => TuiAction::WaveNext,
        KeyCode::Left | KeyCode::Char('h') => TuiAction::DrillOut,
        KeyCode::Right | KeyCode::Char('l') => TuiAction::DrillIn,
        KeyCode::PageUp => TuiAction::ScrollPageUp,
        KeyCode::PageDown => TuiAction::ScrollPageDown,
        KeyCode::Home => TuiAction::ScrollFocusedHome,
        KeyCode::End => TuiAction::ScrollFocusedEnd,

        // Plan tree filter (#219)
        KeyCode::Char('/') => TuiAction::StartPlanFilter,

        // Plan operations (Mori parity)
        KeyCode::Char('d') => TuiAction::RequestConfirm(ConfirmAction::DiagnosePlan(String::new())),
        KeyCode::Char('m') => TuiAction::RequestConfirm(ConfirmAction::MergePlan {
            plan_id: String::new(),
            branch: String::new(),
        }),
        KeyCode::Char('M') => TuiAction::RequestConfirm(ConfirmAction::MergeAllDone {
            branches: Vec::new(),
        }),

        // Recovery keybindings (#119)
        KeyCode::Char('s') => TuiAction::SoftRetry, // soft retry failed tasks
        KeyCode::Char('z') => TuiAction::DiagnoseSelected, // diagnose modal
        KeyCode::Char('S') => TuiAction::RepairWithContext, // repair with error context
        KeyCode::Char('R') => {
            TuiAction::RequestConfirm(ConfirmAction::ResetSelectedPlan(String::new()))
        } // reset plan (confirm)
        KeyCode::Char('c') => TuiAction::ReverifyGatesOnly, // reverify gates only
        KeyCode::Char('F') => TuiAction::ForceAdvance,
        KeyCode::Char('V') => TuiAction::ReverifyPlan,

        // Direct plan selection by number (1-9 select plan at that 0-based index)
        KeyCode::Char(c @ '1'..='9') => TuiAction::SelectPlanByIndex((c as usize) - ('1' as usize)),
        _ => TuiAction::None,
    }
}

fn handle_agents_key(key: KeyEvent, focus: FocusZone) -> TuiAction {
    match key.code {
        // Focus-aware navigation
        KeyCode::Up | KeyCode::Char('k') => match focus {
            FocusZone::AgentOutput => TuiAction::ScrollAgentUp,
            FocusZone::RightPanel => TuiAction::ScrollDiffUp,
            _ => TuiAction::SelectPlanUp,
        },
        KeyCode::Down | KeyCode::Char('j') => match focus {
            FocusZone::AgentOutput => TuiAction::ScrollAgentDown,
            FocusZone::RightPanel => TuiAction::ScrollDiffDown,
            _ => TuiAction::SelectPlanDown,
        },
        KeyCode::PageUp => TuiAction::ScrollPageUp,
        KeyCode::PageDown => TuiAction::ScrollPageDown,
        KeyCode::Home => TuiAction::ScrollFocusedHome,
        KeyCode::End => TuiAction::ScrollFocusedEnd,
        KeyCode::Char('G') => TuiAction::ScrollAgentEnd,

        // Agent role tab switching (1-4 direct, backtick cycles)
        KeyCode::Char('`') => TuiAction::SwitchAgentTab(usize::MAX), // cycle
        KeyCode::Char('1') => TuiAction::SwitchAgentTab(0),
        KeyCode::Char('2') => TuiAction::SwitchAgentTab(1),
        KeyCode::Char('3') => TuiAction::SwitchAgentTab(2),
        KeyCode::Char('4') => TuiAction::SwitchAgentTab(3),
        KeyCode::Char('5') => TuiAction::SwitchAgentTab(4),
        KeyCode::Char('6') => TuiAction::SwitchAgentTab(5),
        KeyCode::Char('7') => TuiAction::SwitchAgentTab(6),

        // Agent approval and interaction
        KeyCode::Char('a') => TuiAction::ApproveCommand,
        KeyCode::Char('A') => TuiAction::ApproveAll,
        KeyCode::Char('x') => TuiAction::RejectCommand,
        KeyCode::Char('i') => TuiAction::StartInject,
        KeyCode::Char('g') => TuiAction::ToggleAgentPaneGroup,
        KeyCode::Char('t') => TuiAction::ToggleAgentTopology,

        // Agent output search (#367): / enters search, n/N navigate
        KeyCode::Char('/') => TuiAction::StartAgentOutputSearch,
        KeyCode::Char('n') => TuiAction::NextAgentOutputMatch,
        KeyCode::Char('N') => TuiAction::PrevAgentOutputMatch,
        // Fold/unfold tool output: f toggles nearest tool result
        KeyCode::Char('f') => TuiAction::ToggleAgentOutputFold,
        _ => TuiAction::None,
    }
}

fn handle_git_key(key: KeyEvent, _focus: FocusZone) -> TuiAction {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => TuiAction::ScrollFocusedUp,
        KeyCode::Down | KeyCode::Char('j') => TuiAction::ScrollFocusedDown,
        KeyCode::PageUp => TuiAction::ScrollPageUp,
        KeyCode::PageDown => TuiAction::ScrollPageDown,
        KeyCode::Home => TuiAction::ScrollFocusedHome,
        KeyCode::End => TuiAction::ScrollFocusedEnd,
        KeyCode::Left | KeyCode::Char('h') => TuiAction::DrillOut,
        KeyCode::Right | KeyCode::Char('l') => TuiAction::DrillIn,
        KeyCode::Enter => TuiAction::ExpandCollapse,
        _ => TuiAction::None,
    }
}

fn handle_logs_key(key: KeyEvent, _focus: FocusZone) -> TuiAction {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => TuiAction::ScrollLogUp,
        KeyCode::Down | KeyCode::Char('j') => TuiAction::ScrollLogDown,
        KeyCode::PageUp => TuiAction::ScrollPageUp,
        KeyCode::PageDown => TuiAction::ScrollPageDown,
        KeyCode::Home => TuiAction::ScrollFocusedHome,
        KeyCode::End => TuiAction::ScrollLogEnd,
        KeyCode::Char('G') => TuiAction::ScrollLogEnd,
        KeyCode::Char('1') => TuiAction::ToggleLogFilter(LogFilterLevel::Info),
        KeyCode::Char('2') => TuiAction::ToggleLogFilter(LogFilterLevel::Warn),
        KeyCode::Char('3') => TuiAction::ToggleLogFilter(LogFilterLevel::Error),
        KeyCode::Char('4') => TuiAction::ToggleLogFilter(LogFilterLevel::Debug),
        KeyCode::Char('a') => TuiAction::ShowAllLogFilters,
        // Log search (#217): / enters search, n/N navigate, f toggles filter
        KeyCode::Char('/') => TuiAction::StartLogSearch,
        KeyCode::Char('n') => TuiAction::NextLogMatch,
        KeyCode::Char('N') => TuiAction::PrevLogMatch,
        KeyCode::Char('f') => TuiAction::ToggleLogFilterMode,
        // Yank selected log entry text
        KeyCode::Char('y') => TuiAction::YankLogEntry,
        _ => TuiAction::None,
    }
}

fn handle_config_key(key: KeyEvent) -> TuiAction {
    // Ctrl-S saves pending config edits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
        return TuiAction::ConfigSave;
    }
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => TuiAction::ConfigUp,
        KeyCode::Down | KeyCode::Char('j') => TuiAction::ConfigDown,
        KeyCode::Left | KeyCode::Char('h') => TuiAction::ConfigCycleLeft,
        KeyCode::Right | KeyCode::Char('l') => TuiAction::ConfigCycleRight,
        KeyCode::Enter | KeyCode::Char(' ') => TuiAction::ConfigToggle,
        // Plain r reloads roko.toml (advertised as `r:reload` in the status bar).
        KeyCode::Char('r') => TuiAction::ConfigReload,
        _ => TuiAction::None,
    }
}

/// Key handler for config text-edit mode (typing a value).
fn handle_config_edit_key(key: KeyEvent) -> TuiAction {
    match key.code {
        KeyCode::Enter => TuiAction::ConfigCommitEdit,
        KeyCode::Esc => TuiAction::ConfigCancelEdit,
        KeyCode::Backspace => TuiAction::InputBackspace,
        KeyCode::Char(c) => TuiAction::InputChar(c),
        _ => TuiAction::None,
    }
}

fn handle_inspect_key(key: KeyEvent, _focus: FocusZone) -> TuiAction {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => TuiAction::ScrollFocusedUp,
        KeyCode::Down | KeyCode::Char('j') => TuiAction::ScrollFocusedDown,
        KeyCode::PageUp => TuiAction::ScrollPageUp,
        KeyCode::PageDown => TuiAction::ScrollPageDown,
        KeyCode::Home => TuiAction::ScrollFocusedHome,
        KeyCode::End => TuiAction::ScrollFocusedEnd,
        KeyCode::Left | KeyCode::Char('h') => TuiAction::DrillOut,
        KeyCode::Right | KeyCode::Char('l') => TuiAction::DrillIn,
        KeyCode::Enter => TuiAction::ExpandCollapse,
        KeyCode::Char('s') => TuiAction::CycleCostSort,
        _ => TuiAction::None,
    }
}

fn handle_marketplace_key(key: KeyEvent, _focus: FocusZone) -> TuiAction {
    // Ctrl-S submits the job creation form.
    if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return TuiAction::SubmitJob;
    }
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => TuiAction::ScrollFocusedDown,
        KeyCode::Char('k') | KeyCode::Up => TuiAction::ScrollFocusedUp,
        KeyCode::Enter => TuiAction::ExpandCollapse,
        KeyCode::Char('n') => TuiAction::SwitchSubView(2), // CreateJob sub-view
        KeyCode::Char('r') => TuiAction::Refresh,
        KeyCode::Home => TuiAction::ScrollFocusedHome,
        KeyCode::End => TuiAction::ScrollFocusedEnd,
        _ => TuiAction::None,
    }
}

fn handle_atelier_key(key: KeyEvent, _focus: FocusZone) -> TuiAction {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => TuiAction::ScrollFocusedDown,
        KeyCode::Char('k') | KeyCode::Up => TuiAction::ScrollFocusedUp,
        KeyCode::Enter => TuiAction::ExpandCollapse,
        KeyCode::Char('r') => TuiAction::Refresh,
        KeyCode::Home => TuiAction::ScrollFocusedHome,
        KeyCode::End => TuiAction::ScrollFocusedEnd,
        _ => TuiAction::None,
    }
}

fn handle_learning_key(key: KeyEvent) -> TuiAction {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => TuiAction::ScrollFocusedDown,
        KeyCode::Char('k') | KeyCode::Up => TuiAction::ScrollFocusedUp,
        KeyCode::Char('r') => TuiAction::Refresh,
        KeyCode::Home => TuiAction::ScrollFocusedHome,
        KeyCode::End => TuiAction::ScrollFocusedEnd,
        _ => TuiAction::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    fn key_with_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    fn modals<'a>(active_modal: Option<&'a ModalState>) -> ModalVisibility<'a> {
        ModalVisibility::from_active_modal(active_modal)
    }

    #[test]
    fn ctrl_c_always_quits() {
        let action = handle_key(
            key_with_mod(KeyCode::Char('c'), KeyModifiers::CONTROL),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(action, TuiAction::QuitConfirmed);
    }

    #[test]
    fn global_n_dismisses_notifications_except_logs_tab() {
        // On most tabs, `n` dismisses notifications.
        let action = handle_key(
            key(KeyCode::Char('n')),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(action, TuiAction::DismissNotification);

        // On Logs tab, `n` navigates to the next search match instead.
        let action = handle_key(
            key(KeyCode::Char('n')),
            InputMode::Normal,
            Tab::Logs,
            FocusZone::LogList,
            &modals(None),
        );
        assert_eq!(action, TuiAction::NextLogMatch);
    }

    #[test]
    fn f_keys_switch_tabs() {
        let action = handle_key(
            key(KeyCode::F(3)),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(action, TuiAction::SwitchTab(Tab::Agents));
    }

    #[test]
    fn confirm_mode_intercepts() {
        let action = handle_key(
            key(KeyCode::Char('y')),
            InputMode::Confirm,
            Tab::Plans,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(action, TuiAction::ConfirmYes);
    }

    #[test]
    fn inject_mode_captures_chars() {
        let action = handle_key(
            key(KeyCode::Char('x')),
            InputMode::Inject,
            Tab::Agents,
            FocusZone::AgentOutput,
            &modals(None),
        );
        assert_eq!(action, TuiAction::InputChar('x'));
    }

    #[test]
    fn filter_mode_enter_accepts() {
        let action = handle_key(
            key(KeyCode::Enter),
            InputMode::Filter,
            Tab::Logs,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(action, TuiAction::AcceptFilter);
    }

    #[test]
    fn number_keys_switch_tabs_from_non_number_tab() {
        // Number keys switch tabs when current tab doesn't use them.
        let action = handle_key(
            key(KeyCode::Char('3')),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(action, TuiAction::SwitchTab(Tab::Agents));
    }

    #[test]
    fn number_keys_do_not_shadow_agents_tab() {
        // On Agents tab, 1-7 should go to per-tab handler (SwitchAgentTab),
        // not global tab switching.
        let action = handle_key(
            key(KeyCode::Char('3')),
            InputMode::Normal,
            Tab::Agents,
            FocusZone::AgentOutput,
            &modals(None),
        );
        assert_eq!(action, TuiAction::SwitchAgentTab(2));
    }

    #[test]
    fn number_keys_do_not_shadow_logs_tab() {
        // On Logs tab, 1-4 should go to per-tab handler (ToggleLogFilter),
        // not global tab switching.
        let action = handle_key(
            key(KeyCode::Char('1')),
            InputMode::Normal,
            Tab::Logs,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(action, TuiAction::ToggleLogFilter(LogFilterLevel::Info));
    }

    #[test]
    fn number_keys_do_not_shadow_plans_tab() {
        // On Plans tab, 1-9 should go to per-tab handler (SelectPlanByIndex),
        // not global tab switching.
        let action = handle_key(
            key(KeyCode::Char('1')),
            InputMode::Normal,
            Tab::Plans,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(action, TuiAction::SelectPlanByIndex(0));
    }

    #[test]
    fn q_key_opens_queue_overview_on_plans_tab() {
        // On Plans tab, `q` should open queue overview, not quit.
        let action = handle_key(
            key(KeyCode::Char('q')),
            InputMode::Normal,
            Tab::Plans,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(action, TuiAction::ShowQueueOverview);
    }

    #[test]
    fn q_key_quits_on_other_tabs() {
        // On other tabs, `q` should still quit.
        let action = handle_key(
            key(KeyCode::Char('q')),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(action, TuiAction::Quit);
    }

    #[test]
    fn zero_key_switches_to_learning_tab() {
        let action = handle_key(
            key(KeyCode::Char('0')),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(action, TuiAction::SwitchTab(Tab::Learning));
    }

    #[test]
    fn alt_number_switches_subview() {
        let action = handle_key(
            key_with_mod(KeyCode::Char('2'), KeyModifiers::ALT),
            InputMode::Normal,
            Tab::Logs,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(action, TuiAction::SwitchSubView(1));
    }

    #[test]
    fn ctrl_e_toggles_screen_postfx() {
        let action = handle_key(
            key_with_mod(KeyCode::Char('e'), KeyModifiers::CONTROL),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(action, TuiAction::ToggleScreenPostFx);
    }

    #[test]
    fn ctrl_t_toggles_agent_topology_globally() {
        let action = handle_key(
            key_with_mod(KeyCode::Char('t'), KeyModifiers::CONTROL),
            InputMode::Normal,
            Tab::Logs,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(action, TuiAction::ToggleAgentTopology);
    }

    #[test]
    fn v_triggers_reverify_globally() {
        let action = handle_key(
            key(KeyCode::Char('v')),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(action, TuiAction::ReverifyPlan);
    }

    #[test]
    fn v_triggers_reverify_on_plans_tab() {
        let action = handle_key(
            key(KeyCode::Char('v')),
            InputMode::Normal,
            Tab::Plans,
            FocusZone::PlanTree,
            &modals(None),
        );
        // Plans tab has its own 'V' (uppercase) for ReverifyPlan; lowercase
        // 'v' is intercepted by the global handler first.
        assert_eq!(action, TuiAction::ReverifyPlan);
    }

    #[test]
    fn logs_tab_a_restores_all_levels() {
        let action = handle_key(
            key(KeyCode::Char('a')),
            InputMode::Normal,
            Tab::Logs,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(action, TuiAction::ShowAllLogFilters);
    }

    #[test]
    fn task_picker_modal_intercepts() {
        let modal = ModalState::TaskPicker {
            tasks: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
        };
        let m = modals(Some(&modal));
        let action = handle_key(
            key(KeyCode::Esc),
            InputMode::Normal,
            Tab::Plans,
            FocusZone::PlanTree,
            &m,
        );
        assert_eq!(action, TuiAction::CloseTaskPicker);
    }

    #[test]
    fn approval_modal_intercepts_yes_and_no() {
        let modal = ModalState::Approval {
            role: "implementer".to_string(),
            command: "cargo check".to_string(),
        };
        let m = modals(Some(&modal));

        let approve = handle_key(
            key(KeyCode::Char('y')),
            InputMode::Normal,
            Tab::Plans,
            FocusZone::PlanTree,
            &m,
        );
        assert_eq!(approve, TuiAction::ApproveCommand);

        let reject = handle_key(
            key(KeyCode::Char('n')),
            InputMode::Normal,
            Tab::Plans,
            FocusZone::PlanTree,
            &m,
        );
        assert_eq!(reject, TuiAction::RejectCommand);
    }

    #[test]
    fn modal_visibility_reads_active_modal() {
        let help = ModalState::Help;
        let vis = ModalVisibility::from_active_modal(Some(&help));
        assert!(matches!(vis.active_modal, Some(ModalState::Help)));

        let approval = ModalState::Approval {
            role: "implementer".to_string(),
            command: "cargo check".to_string(),
        };
        let vis = ModalVisibility::from_active_modal(Some(&approval));
        assert!(matches!(
            vis.active_modal,
            Some(ModalState::Approval { .. })
        ));

        let task_picker = ModalState::TaskPicker {
            tasks: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
        };
        let vis = ModalVisibility::from_active_modal(Some(&task_picker));
        assert!(matches!(
            vis.active_modal,
            Some(ModalState::TaskPicker { .. })
        ));

        let agent_pool = ModalState::AgentPool {
            agents: Vec::new(),
            scroll_offset: 0,
        };
        let vis = ModalVisibility::from_active_modal(Some(&agent_pool));
        assert!(matches!(
            vis.active_modal,
            Some(ModalState::AgentPool { .. })
        ));
    }

    #[test]
    fn help_modal_intercepts_background_navigation_and_scrolls_locally() {
        let modal = ModalState::Help;
        let vis = modals(Some(&modal));

        let action = handle_key(
            key(KeyCode::Tab),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::None);

        let action = handle_key(
            key(KeyCode::Down),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::ScrollFocusedDown);
    }

    #[test]
    fn modal_specific_navigation_stays_local() {
        let modal = ModalState::PlanDetail {
            plan_id: "plan-1".to_string(),
        };
        let vis = modals(Some(&modal));

        let action = handle_key(
            key(KeyCode::Char('k')),
            InputMode::Normal,
            Tab::Plans,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::ScrollDetailUp);

        let action = handle_key(
            key(KeyCode::Char('j')),
            InputMode::Normal,
            Tab::Plans,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::ScrollDetailDown);

        // Tab cycles detail sub-tabs within the plan detail modal.
        let action = handle_key(
            key(KeyCode::Tab),
            InputMode::Normal,
            Tab::Plans,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::SwitchDetailTab(0));
    }

    #[test]
    fn wave_overview_modal_still_handles_local_scroll() {
        let modal = ModalState::WaveOverview {
            waves: Vec::new(),
            scroll_offset: 0,
        };
        let vis = modals(Some(&modal));

        let action = handle_key(
            key(KeyCode::Up),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::ModalScrollUp);

        let action = handle_key(
            key(KeyCode::Char('j')),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::ModalScrollDown);

        let action = handle_key(
            key(KeyCode::Tab),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::None);
    }

    #[test]
    fn queue_overview_modal_uses_local_navigation() {
        let modal = ModalState::QueueOverview {
            milestones: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
        };
        let vis = modals(Some(&modal));

        let action = handle_key(
            key(KeyCode::Up),
            InputMode::Normal,
            Tab::Plans,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::QueueOverviewUp);

        let action = handle_key(
            key(KeyCode::Char('j')),
            InputMode::Normal,
            Tab::Plans,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::QueueOverviewDown);
    }

    #[test]
    fn agent_pool_modal_blocks_navigation_keys() {
        let modal = ModalState::AgentPool {
            agents: Vec::new(),
            scroll_offset: 0,
        };
        let vis = modals(Some(&modal));

        let action = handle_key(
            key(KeyCode::Up),
            InputMode::Normal,
            Tab::Agents,
            FocusZone::AgentOutput,
            &vis,
        );
        assert_eq!(action, TuiAction::ModalScrollUp);

        let action = handle_key(
            key(KeyCode::Char('j')),
            InputMode::Normal,
            Tab::Agents,
            FocusZone::AgentOutput,
            &vis,
        );
        assert_eq!(action, TuiAction::ModalScrollDown);

        let action = handle_key(
            key(KeyCode::Tab),
            InputMode::Normal,
            Tab::Agents,
            FocusZone::AgentOutput,
            &vis,
        );
        assert_eq!(action, TuiAction::None);

        let action = handle_key(
            key(KeyCode::Esc),
            InputMode::Normal,
            Tab::Agents,
            FocusZone::AgentOutput,
            &vis,
        );
        assert_eq!(action, TuiAction::CloseModal);
    }

    #[test]
    fn ctrl_c_takes_precedence_over_open_modal() {
        let modal = ModalState::WaveOverview {
            waves: Vec::new(),
            scroll_offset: 0,
        };
        let vis = modals(Some(&modal));

        let action = handle_key(
            key_with_mod(KeyCode::Char('c'), KeyModifiers::CONTROL),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::QuitConfirmed);
    }

    #[test]
    fn ctrl_c_takes_precedence_over_modal_and_mode_intercepts() {
        let modal = ModalState::Approval {
            role: "implementer".to_string(),
            command: "cargo check".to_string(),
        };
        let vis = modals(Some(&modal));

        let action = handle_key(
            key_with_mod(KeyCode::Char('c'), KeyModifiers::CONTROL),
            InputMode::Confirm,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::QuitConfirmed);
    }

    #[test]
    fn plans_tab_focus_routing() {
        let action = handle_key(
            key(KeyCode::Up),
            InputMode::Normal,
            Tab::Plans,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(action, TuiAction::SelectPlanUp);

        let action = handle_key(
            key(KeyCode::Up),
            InputMode::Normal,
            Tab::Plans,
            FocusZone::AgentOutput,
            &modals(None),
        );
        assert_eq!(action, TuiAction::ScrollFocusedUp);
    }

    #[test]
    fn agents_tab_diff_scroll() {
        let action = handle_key(
            key(KeyCode::Down),
            InputMode::Normal,
            Tab::Agents,
            FocusZone::RightPanel,
            &modals(None),
        );
        assert_eq!(action, TuiAction::ScrollDiffDown);
    }

    #[test]
    fn page_keys_use_page_scroll_actions() {
        let action = handle_key(
            key(KeyCode::PageUp),
            InputMode::Normal,
            Tab::Plans,
            FocusZone::TaskProgress,
            &modals(None),
        );
        assert_eq!(action, TuiAction::ScrollPageUp);

        let action = handle_key(
            key(KeyCode::PageDown),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::CommandOutput,
            &modals(None),
        );
        assert_eq!(action, TuiAction::ScrollPageDown);
    }

    #[test]
    fn page_keys_keep_page_scroll_actions_on_logs_tab() {
        let action = handle_key(
            key(KeyCode::PageUp),
            InputMode::Normal,
            Tab::Logs,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(action, TuiAction::ScrollPageUp);

        let action = handle_key(
            key(KeyCode::PageDown),
            InputMode::Normal,
            Tab::Logs,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(action, TuiAction::ScrollPageDown);
    }

    #[test]
    fn logs_tab_slash_starts_filter() {
        let action = handle_key(
            key(KeyCode::Char('/')),
            InputMode::Normal,
            Tab::Logs,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(action, TuiAction::StartLogSearch);
    }

    #[test]
    fn home_end_use_focused_jump_actions() {
        let action = handle_key(
            key(KeyCode::Home),
            InputMode::Normal,
            Tab::Agents,
            FocusZone::RightPanel,
            &modals(None),
        );
        assert_eq!(action, TuiAction::ScrollFocusedHome);

        let action = handle_key(
            key(KeyCode::End),
            InputMode::Normal,
            Tab::Logs,
            FocusZone::CommandOutput,
            &modals(None),
        );
        assert_eq!(action, TuiAction::ScrollLogEnd);

        let action = handle_key(
            key(KeyCode::Char('G')),
            InputMode::Normal,
            Tab::Logs,
            FocusZone::AgentOutput,
            &modals(None),
        );
        assert_eq!(action, TuiAction::ScrollLogEnd);
    }

    #[test]
    fn shift_arrow_keys_navigate_waves() {
        let action = handle_key(
            key_with_mod(KeyCode::Left, KeyModifiers::SHIFT),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(action, TuiAction::WavePrev);

        let action = handle_key(
            key_with_mod(KeyCode::Right, KeyModifiers::SHIFT),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(action, TuiAction::WaveNext);

        let action = handle_key(
            key_with_mod(KeyCode::Left, KeyModifiers::SHIFT),
            InputMode::Normal,
            Tab::Plans,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(action, TuiAction::WavePrev);

        let action = handle_key(
            key_with_mod(KeyCode::Right, KeyModifiers::SHIFT),
            InputMode::Normal,
            Tab::Plans,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(action, TuiAction::WaveNext);
    }

    #[test]
    fn plans_tab_confirm_shortcuts_route_to_request_confirm() {
        let action = handle_key(
            key(KeyCode::Char('d')),
            InputMode::Normal,
            Tab::Plans,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(
            action,
            TuiAction::RequestConfirm(ConfirmAction::DiagnosePlan(String::new()))
        );

        let action = handle_key(
            key(KeyCode::Char('m')),
            InputMode::Normal,
            Tab::Plans,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(
            action,
            TuiAction::RequestConfirm(ConfirmAction::MergePlan {
                plan_id: String::new(),
                branch: String::new(),
            })
        );

        let action = handle_key(
            key(KeyCode::Char('M')),
            InputMode::Normal,
            Tab::Plans,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(
            action,
            TuiAction::RequestConfirm(ConfirmAction::MergeAllDone {
                branches: Vec::new(),
            })
        );
    }

    #[test]
    fn ctrl_g_requests_git_reconcile_confirmation() {
        let action = handle_key(
            key_with_mod(KeyCode::Char('g'), KeyModifiers::CONTROL),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &modals(None),
        );
        assert_eq!(
            action,
            TuiAction::RequestConfirm(ConfirmAction::GitReconcile)
        );
    }

    // =================================================================
    // #365 — modal input precedence tests
    // =================================================================

    #[test]
    fn input_mode_takes_precedence_over_active_modal() {
        // When InputMode is non-Normal AND a modal is open, InputMode wins.
        let modal = ModalState::Help;
        let vis = modals(Some(&modal));

        // Inject mode captures 'x' even with Help modal open.
        let action = handle_key(
            key(KeyCode::Char('x')),
            InputMode::Inject,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::InputChar('x'));
    }

    #[test]
    fn confirm_mode_takes_precedence_over_active_modal() {
        let modal = ModalState::PlanDetail {
            plan_id: "p1".to_string(),
        };
        let vis = modals(Some(&modal));

        let action = handle_key(
            key(KeyCode::Char('y')),
            InputMode::Confirm,
            Tab::Plans,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::ConfirmYes);
    }

    #[test]
    fn log_search_mode_takes_precedence_over_modal() {
        let modal = ModalState::Help;
        let vis = modals(Some(&modal));

        let action = handle_key(
            key(KeyCode::Char('a')),
            InputMode::LogSearch,
            Tab::Logs,
            FocusZone::LogList,
            &vis,
        );
        assert_eq!(action, TuiAction::InputChar('a'));
    }

    #[test]
    fn modal_allows_fkey_tab_switching() {
        // F-keys always switch tabs, even when a modal is open.
        let modal = ModalState::BatchReview {
            batch_name: "b".to_string(),
            results: Vec::new(),
            scroll_offset: 0,
        };
        let vis = modals(Some(&modal));

        // F3 should switch tabs even with a modal open.
        let action = handle_key(
            key(KeyCode::F(3)),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::SwitchTab(Tab::Agents));
    }

    #[test]
    fn modal_blocks_tab_and_number_navigation() {
        // Tab and number keys are blocked by modals (only F-keys pass through).
        let modal = ModalState::BatchReview {
            batch_name: "b".to_string(),
            results: Vec::new(),
            scroll_offset: 0,
        };
        let vis = modals(Some(&modal));

        // Tab should NOT cycle focus; swallowed by modal.
        let action = handle_key(
            key(KeyCode::Tab),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::None);

        // Number keys should NOT switch tabs.
        let action = handle_key(
            key(KeyCode::Char('3')),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::None);
    }

    #[test]
    fn batch_review_skip_key() {
        let modal = ModalState::BatchReview {
            batch_name: "test".to_string(),
            results: Vec::new(),
            scroll_offset: 0,
        };
        let vis = modals(Some(&modal));

        let action = handle_key(
            key(KeyCode::Char('s')),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::CloseModal);
    }

    #[test]
    fn batch_review_approve_and_reject() {
        let modal = ModalState::BatchReview {
            batch_name: "test".to_string(),
            results: Vec::new(),
            scroll_offset: 0,
        };
        let vis = modals(Some(&modal));

        let approve = handle_key(
            key(KeyCode::Char('a')),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(approve, TuiAction::ConfirmYes);

        let reject = handle_key(
            key(KeyCode::Char('r')),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(reject, TuiAction::ConfirmNo);
    }

    // =================================================================
    // #237 — focus zone cycling tests
    // =================================================================

    #[test]
    fn plans_tab_focus_cycles_three_zones() {
        // F2 Plans: PlanTree -> TaskProgress -> RightPanel -> PlanTree
        let z1 = FocusZone::PlanTree;
        let z2 = z1.next(Tab::Plans);
        assert_eq!(z2, FocusZone::TaskProgress);

        let z3 = z2.next(Tab::Plans);
        assert_eq!(z3, FocusZone::RightPanel);

        let z4 = z3.next(Tab::Plans);
        assert_eq!(z4, FocusZone::PlanTree);
    }

    #[test]
    fn plans_tab_focus_cycles_three_zones_reverse() {
        let z1 = FocusZone::PlanTree;
        let z2 = z1.prev(Tab::Plans);
        assert_eq!(z2, FocusZone::RightPanel);

        let z3 = z2.prev(Tab::Plans);
        assert_eq!(z3, FocusZone::TaskProgress);

        let z4 = z3.prev(Tab::Plans);
        assert_eq!(z4, FocusZone::PlanTree);
    }

    #[test]
    fn inspect_tab_focus_cycles_three_zones() {
        // F7 Inspect: InspectTree -> InspectDetail -> RightPanel -> InspectTree
        let z1 = FocusZone::InspectTree;
        let z2 = z1.next(Tab::Inspect);
        assert_eq!(z2, FocusZone::InspectDetail);

        let z3 = z2.next(Tab::Inspect);
        assert_eq!(z3, FocusZone::RightPanel);

        let z4 = z3.next(Tab::Inspect);
        assert_eq!(z4, FocusZone::InspectTree);
    }

    #[test]
    fn inspect_tab_focus_cycles_three_zones_reverse() {
        let z1 = FocusZone::InspectTree;
        let z2 = z1.prev(Tab::Inspect);
        assert_eq!(z2, FocusZone::RightPanel);

        let z3 = z2.prev(Tab::Inspect);
        assert_eq!(z3, FocusZone::InspectDetail);

        let z4 = z3.prev(Tab::Inspect);
        assert_eq!(z4, FocusZone::InspectTree);
    }

    #[test]
    fn quit_modal_confirm_y_confirms() {
        let modal = ModalState::Quit;
        let vis = modals(Some(&modal));

        let action = handle_key(
            key(KeyCode::Char('y')),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::ConfirmYes);
    }

    #[test]
    fn quit_modal_n_cancels() {
        let modal = ModalState::Quit;
        let vis = modals(Some(&modal));

        let action = handle_key(
            key(KeyCode::Char('n')),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::ConfirmNo);
    }

    #[test]
    fn notification_history_modal_dismisses_on_esc() {
        let modal = ModalState::NotificationHistory {
            scroll_offset: 0,
            selected_index: 0,
            filter: super::super::modals::LevelFilter::default(),
        };
        let vis = modals(Some(&modal));

        let action = handle_key(
            key(KeyCode::Esc),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::CloseModal);
    }

    #[test]
    fn notification_history_filter_keys() {
        let modal = ModalState::NotificationHistory {
            scroll_offset: 0,
            selected_index: 0,
            filter: super::super::modals::LevelFilter::default(),
        };
        let vis = modals(Some(&modal));

        let action = handle_key(
            key(KeyCode::Char('1')),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::NotifFilterToggle(1));

        let action = handle_key(
            key(KeyCode::Char('2')),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::NotifFilterToggle(2));

        let action = handle_key(
            key(KeyCode::Char('3')),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::NotifFilterToggle(3));

        let action = handle_key(
            key(KeyCode::Char('4')),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::NotifFilterToggle(4));
    }

    #[test]
    fn notification_history_page_navigation() {
        let modal = ModalState::NotificationHistory {
            scroll_offset: 0,
            selected_index: 0,
            filter: super::super::modals::LevelFilter::default(),
        };
        let vis = modals(Some(&modal));

        let action = handle_key(
            key(KeyCode::PageUp),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::NotifPageUp);

        let action = handle_key(
            key(KeyCode::PageDown),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::NotifPageDown);

        let action = handle_key(
            key(KeyCode::Home),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::NotifHome);

        let action = handle_key(
            key(KeyCode::End),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::NotifEnd);
    }

    #[test]
    fn notification_history_enter_jumps() {
        let modal = ModalState::NotificationHistory {
            scroll_offset: 0,
            selected_index: 0,
            filter: super::super::modals::LevelFilter::default(),
        };
        let vis = modals(Some(&modal));

        let action = handle_key(
            key(KeyCode::Enter),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::NotifJumpToRelated);
    }

    #[test]
    fn welcome_modal_enter_initializes() {
        let modal = ModalState::Welcome { initialized: false };
        let vis = modals(Some(&modal));

        let action = handle_key(
            key(KeyCode::Enter),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::WelcomeInit);
    }

    #[test]
    fn welcome_modal_any_key_dismisses_after_init() {
        let modal = ModalState::Welcome { initialized: true };
        let vis = modals(Some(&modal));

        let action = handle_key(
            key(KeyCode::Char('x')),
            InputMode::Normal,
            Tab::Dashboard,
            FocusZone::PlanTree,
            &vis,
        );
        assert_eq!(action, TuiAction::WelcomeDismiss);
    }
}
