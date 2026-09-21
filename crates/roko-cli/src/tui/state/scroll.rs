//! Scroll-position management for every TUI tab.
//!
//! All `clamp_*`, `reset_scrolls`, agent-topology toggles, and
//! log-filter-level methods live here so the main `TuiState` impl block
//! stays focused on data loading.

use super::super::input::{InputMode, LogFilterLevel};
use super::{AgentTopologyStatus, TuiState};

impl TuiState {
    /// Whether the state is in a text-input mode (inject or filter).
    #[must_use]
    pub const fn is_text_input(&self) -> bool {
        matches!(
            self.input_mode,
            InputMode::Inject
                | InputMode::Filter
                | InputMode::LogSearch
                | InputMode::PlanFilter
                | InputMode::AgentOutputSearch
        )
    }

    /// Reset all scroll positions to zero.
    pub fn reset_scrolls(&mut self) {
        self.agent_scroll = None;
        self.diff_scroll = 0;
        self.procs_scroll = 0;
        self.inbox_scroll = 0;
        self.git_detail_scroll = 0;
        self.log_detail_scroll = 0;
        self.config_values_scroll = 0;
        self.inspect_detail_scroll = 0;
        self.marketplace_detail_scroll = 0;
        self.atelier_detail_scroll = 0;
        self.learning_detail_scroll = 0;
        self.task_scroll = 0;
        self.command_output_scroll = 0;
        self.plan_detail_scroll = 0;
        self.plan_scroll_offset = 0;
        self.log_scroll = 0;
        self.agent_topology_scroll_offset = 0;
        self.log_auto_tail = true;
    }

    /// Clamp the plan tree scroll offset to the current rendered maximum.
    pub fn clamp_plan_scroll(&mut self, max: usize) {
        self.plan_scroll_offset = self.plan_scroll_offset.min(max);
    }

    /// Clamp the pinned agent-output scroll offset to the current rendered maximum.
    pub fn clamp_agent_scroll(&mut self, max: usize) {
        if let Some(scroll) = self.agent_scroll.as_mut() {
            *scroll = (*scroll).min(max);
        }
    }

    /// Clamp the log scroll offset to the current rendered maximum.
    pub fn clamp_log_scroll(&mut self, max: usize) {
        if self.log_auto_tail {
            self.log_scroll = 0;
        } else {
            self.log_scroll = self.log_scroll.min(max);
        }
    }

    /// Toggle visibility for the agent-topology overlay.
    pub fn toggle_agent_topology(&mut self) {
        self.agent_topology_visible = !self.agent_topology_visible;
    }

    /// Close the agent-topology overlay.
    pub fn close_agent_topology(&mut self) {
        self.agent_topology_visible = false;
    }

    /// Clamp the agent-topology scroll offset to the current rendered maximum.
    pub fn clamp_agent_topology_scroll(&mut self, max: usize) {
        self.agent_topology_scroll_offset = self.agent_topology_scroll_offset.min(max);
    }

    /// Mark the agent-topology panel as loading.
    pub fn set_agent_topology_loading(&mut self) {
        self.agent_topology_status = AgentTopologyStatus::Loading;
    }

    /// Store the latest fetched agent-topology payload.
    pub fn set_agent_topology(&mut self, topology: roko_core::AgentTopology) {
        self.agent_topology = topology;
        self.agent_topology_status = AgentTopologyStatus::Ready;
        self.agent_topology_scroll_offset = 0;
    }

    /// Mark the agent-topology endpoint as unavailable.
    pub fn set_agent_topology_unavailable(&mut self) {
        self.agent_topology = roko_core::AgentTopology::default();
        self.agent_topology_status = AgentTopologyStatus::Unavailable;
        self.agent_topology_scroll_offset = 0;
    }

    /// Record a topology fetch error message.
    pub fn set_agent_topology_error(&mut self, message: impl Into<String>) {
        self.agent_topology = roko_core::AgentTopology::default();
        self.agent_topology_status = AgentTopologyStatus::Error(message.into());
        self.agent_topology_scroll_offset = 0;
    }

    /// Clamp the right-panel scroll offset to the current rendered maximum.
    pub fn clamp_diff_scroll(&mut self, max: usize) {
        self.diff_scroll = self.diff_scroll.min(max);
    }

    /// Clamp the task list scroll offset to the current rendered maximum.
    pub fn clamp_task_scroll(&mut self, max: usize) {
        self.task_scroll = self.task_scroll.min(max);
    }

    /// Clamp the command-output scroll offset to the current rendered maximum.
    pub fn clamp_command_output_scroll(&mut self, max: usize) {
        self.command_output_scroll = self.command_output_scroll.min(max);
    }

    /// Clamp the git detail scroll offset to the current rendered maximum.
    pub fn clamp_git_detail_scroll(&mut self, max: usize) {
        self.git_detail_scroll = self.git_detail_scroll.min(max);
    }

    /// Clamp the config values scroll offset to the current rendered maximum.
    pub fn clamp_config_values_scroll(&mut self, max: usize) {
        self.config_values_scroll = self.config_values_scroll.min(max);
    }

    /// Clamp the config keys scroll offset to the current rendered maximum.
    pub fn clamp_config_scroll_offset(&mut self, max: usize) {
        self.config_scroll_offset = self.config_scroll_offset.min(max);
    }

    /// Clamp the inspect detail scroll offset to the current rendered maximum.
    pub fn clamp_inspect_detail_scroll(&mut self, max: usize) {
        self.inspect_detail_scroll = self.inspect_detail_scroll.min(max);
    }

    /// Clamp the learning detail scroll offset to the current rendered maximum.
    pub fn clamp_learning_detail_scroll(&mut self, max: usize) {
        self.learning_detail_scroll = self.learning_detail_scroll.min(max);
    }

    /// Clamp the procs scroll offset to the current rendered maximum.
    pub fn clamp_procs_scroll(&mut self, max: usize) {
        self.procs_scroll = self.procs_scroll.min(max);
    }

    /// Clamp the log detail scroll offset to the current rendered maximum.
    pub fn clamp_log_detail_scroll(&mut self, max: usize) {
        self.log_detail_scroll = self.log_detail_scroll.min(max);
    }

    /// Clamp the marketplace detail scroll offset to the current rendered maximum.
    pub fn clamp_marketplace_detail_scroll(&mut self, max: usize) {
        self.marketplace_detail_scroll = self.marketplace_detail_scroll.min(max);
    }

    /// Clamp the atelier detail scroll offset to the current rendered maximum.
    pub fn clamp_atelier_detail_scroll(&mut self, max: usize) {
        self.atelier_detail_scroll = self.atelier_detail_scroll.min(max);
    }

    /// Toggle visibility for a single log level in the Logs tab.
    pub fn toggle_log_filter_level(&mut self, level: LogFilterLevel) {
        if !self.log_filter_levels.insert(level) {
            self.log_filter_levels.remove(&level);
        }
    }

    /// Restore the Logs tab to show all available levels.
    pub fn show_all_log_filter_levels(&mut self) {
        self.log_filter_levels = LogFilterLevel::all().into_iter().collect();
    }

    /// Whether a log level is currently visible in the Logs tab.
    #[must_use]
    pub fn log_level_visible(&self, level: LogFilterLevel) -> bool {
        self.log_filter_levels.contains(&level)
    }
}
