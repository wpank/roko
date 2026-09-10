//! Focus zone management, scroll delegation, viewport calculations, and
//! hit-zone-to-focus mapping.

use super::*;

impl App {
    pub(super) fn map_hit_zone(&self, zone: super::super::hit_test::FocusZone) -> FocusZone {
        match zone {
            super::super::hit_test::FocusZone::PlanTree => FocusZone::PlanTree,
            super::super::hit_test::FocusZone::TaskProgress => FocusZone::TaskProgress,
            super::super::hit_test::FocusZone::AgentOutput => FocusZone::AgentOutput,
            super::super::hit_test::FocusZone::CommandOutput => FocusZone::CommandOutput,
            super::super::hit_test::FocusZone::RightContent => FocusZone::RightPanel,
            super::super::hit_test::FocusZone::HeaderTab(_) | super::super::hit_test::FocusZone::DetailTab(_) => {
                FocusZone::RightPanel
            }
            super::super::hit_test::FocusZone::LeftPane => match self.tui_state.active_tab {
                Tab::Git => FocusZone::GitBranches,
                Tab::Logs => FocusZone::LogList,
                Tab::Config => FocusZone::ConfigKeys,
                Tab::Inspect => FocusZone::InspectTree,
                Tab::Marketplace => FocusZone::MarketList,
                Tab::Atelier => FocusZone::AtelierList,
                Tab::Learning => FocusZone::LearningMetrics,
                Tab::Providers => FocusZone::ProviderList,
                _ => FocusZone::PlanTree,
            },
            super::super::hit_test::FocusZone::RightPane => match self.tui_state.active_tab {
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
    ///
    /// Uses the `HitRegionRegistry`-derived `ScrollTarget` to route the scroll
    /// to the correct state field. Updates focus to the hovered panel so
    /// subsequent keyboard scrolling continues from the same panel (#368).
    /// Falls back to `scroll_focused` when the cursor is outside any known zone.

    pub(super) fn scroll_at(&mut self, x: u16, y: u16, delta: i32) {
        let zones = super::super::hit_test::HitZones::compute(
            super::super::layout::responsive_outer_margin(Rect::new(
                0,
                0,
                self.terminal_size.0,
                self.terminal_size.1,
            )),
            self.tui_state.active_tab as usize,
            Tab::ALL.len(),
        );
        let registry = zones.into_registry(self.tui_state.active_tab);

        if let Some(region) = registry.region_at(x, y) {
            // Update focus to the hovered panel so keyboard scrolling follows.
            let mapped = self.map_hit_zone(region.focus_zone);
            self.tui_state.focus = mapped;

            // Route scroll to the dedicated target, avoiding the generic
            // diff_scroll fallback for unrelated detail panes.
            self.scroll_by_target(region.scroll_target, delta);
        } else {
            // Cursor outside any panel -- fall back to keyboard-focused panel.
            self.scroll_focused(delta);
        }
    }

    /// Apply a scroll delta to a specific `ScrollTarget`.
    ///
    /// Each target maps to exactly one scroll state field, avoiding the old
    /// pattern of temporarily swapping focus and falling through to a generic
    /// diff_scroll fallback.

    pub(super) fn scroll_by_target(&mut self, target: super::super::hit_test::ScrollTarget, delta: i32) {
        use super::super::hit_test::ScrollTarget;
        match target {
            ScrollTarget::PlanTree => {
                let current = self.tui_state.plan_scroll_offset as i32;
                self.tui_state.plan_scroll_offset = (current + delta).max(0) as usize;
            }
            ScrollTarget::TaskProgress => {
                let current = self.tui_state.task_scroll as i32;
                self.tui_state.task_scroll = (current + delta).max(0) as usize;
            }
            ScrollTarget::AgentOutput => self.scroll_agent_output_by(delta),
            ScrollTarget::CommandOutput => {
                let current = self.tui_state.command_output_scroll as i32;
                self.tui_state.command_output_scroll = (current + delta).max(0) as usize;
            }
            ScrollTarget::RightPanel => {
                let current = self.tui_state.diff_scroll as i32;
                self.tui_state.diff_scroll = (current + delta).max(0) as usize;
            }
            ScrollTarget::Procs => {
                let current = self.tui_state.procs_scroll as i32;
                self.tui_state.procs_scroll = (current + delta).max(0) as usize;
            }
            ScrollTarget::GitDetail => {
                let current = self.tui_state.git_detail_scroll as i32;
                self.tui_state.git_detail_scroll = (current + delta).max(0) as usize;
            }
            ScrollTarget::ConfigValues => {
                let current = self.tui_state.config_values_scroll as i32;
                self.tui_state.config_values_scroll = (current + delta).max(0) as usize;
            }
            ScrollTarget::InspectDetail => {
                let current = self.tui_state.inspect_detail_scroll as i32;
                self.tui_state.inspect_detail_scroll = (current + delta).max(0) as usize;
            }
            ScrollTarget::LearningDetail => {
                let current = self.tui_state.learning_detail_scroll as i32;
                self.tui_state.learning_detail_scroll = (current + delta).max(0) as usize;
            }
            ScrollTarget::ConfigKeys => {
                let current = self.tui_state.config_scroll_offset as i32;
                self.tui_state.config_scroll_offset = (current + delta).max(0) as usize;
            }
            ScrollTarget::LogList => self.scroll_logs_by(delta),
            ScrollTarget::AgentRoster => {
                let max = self.tui_state.agents.len().saturating_sub(1);
                let next = (self.tui_state.selected_agent as i32 + delta).clamp(0, max as i32);
                self.tui_state.selected_agent = next as usize;
            }
            ScrollTarget::MarketplaceJobs => {
                if !self.tui_state.marketplace_jobs.is_empty() {
                    let max = self.tui_state.marketplace_jobs.len().saturating_sub(1);
                    let next = (self.tui_state.marketplace_selected_job as i32 + delta)
                        .clamp(0, max as i32);
                    self.tui_state.marketplace_selected_job = next as usize;
                }
            }
            ScrollTarget::AtelierPrds => {
                if !self.tui_state.atelier_prds.is_empty() {
                    let max = self.tui_state.atelier_prds.len().saturating_sub(1);
                    let next =
                        (self.tui_state.atelier_selected_prd as i32 + delta).clamp(0, max as i32);
                    self.tui_state.atelier_selected_prd = next as usize;
                }
            }
            ScrollTarget::Modal | ScrollTarget::None => {
                // Modal scroll is handled by handle_mouse before reaching here.
                // None is not scrollable.
            }
        }
    }


    pub(super) fn scroll_focused(&mut self, delta: i32) {
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
            // Dashboard RightPanel: route to inbox_scroll when on the Inbox sub-tab.
            (Tab::Dashboard, FocusZone::RightPanel) if self.tui_state.plan_detail_tab == 9 => {
                let current = self.tui_state.inbox_scroll as i32;
                self.tui_state.inbox_scroll = (current + delta).max(0) as usize;
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
            // Per-tab left/detail zones not captured above: route to their
            // dedicated scroll field so that no unrelated pane bleeds into
            // diff_scroll (#368).
            (Tab::Git, FocusZone::GitBranches) => {
                let current = self.tui_state.plan_scroll_offset as i32;
                self.tui_state.plan_scroll_offset = (current + delta).max(0) as usize;
            }
            (Tab::Inspect, FocusZone::InspectTree) => {
                let current = self.tui_state.plan_scroll_offset as i32;
                self.tui_state.plan_scroll_offset = (current + delta).max(0) as usize;
            }
            (Tab::Learning, FocusZone::LearningMetrics) => {
                let current = self.tui_state.plan_scroll_offset as i32;
                self.tui_state.plan_scroll_offset = (current + delta).max(0) as usize;
            }
            (Tab::Providers, FocusZone::ProviderList) => {
                let current = self.tui_state.providers_selected as i32;
                let max = self
                    .tui_state
                    .provider_statuses
                    .len()
                    .saturating_sub(1) as i32;
                self.tui_state.providers_selected =
                    (current + delta).clamp(0, max) as usize;
            }
            // Exhaustive: any remaining (tab, zone) combination is a no-op
            // rather than leaking into a shared scroll field.
            _ => {}
        }
    }


    pub(super) fn set_focused_scroll(&mut self, offset: usize) {
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
            // Dashboard RightPanel: route to inbox_scroll when on the Inbox sub-tab.
            (Tab::Dashboard, FocusZone::RightPanel) if self.tui_state.plan_detail_tab == 9 => {
                self.tui_state.inbox_scroll = offset;
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
            (Tab::Providers, FocusZone::ProviderDetail) => {
                self.tui_state.providers_detail_scroll = offset;
            }
            // Per-tab left/detail zones not captured above: route to their
            // dedicated field so that no unrelated pane bleeds into
            // diff_scroll (#368).
            (Tab::Git, FocusZone::GitBranches)
            | (Tab::Inspect, FocusZone::InspectTree)
            | (Tab::Learning, FocusZone::LearningMetrics) => {
                self.tui_state.plan_scroll_offset = offset;
            }
            (Tab::Providers, FocusZone::ProviderList) => {
                let max = self.tui_state.provider_statuses.len().saturating_sub(1);
                self.tui_state.providers_selected = offset.min(max);
            }
            // Exhaustive: any remaining (tab, zone) combination is a no-op.
            _ => {}
        }
    }


    pub(super) fn apply_signed_scroll(current: usize, delta: i16) -> usize {
        if delta < 0 {
            current.saturating_sub(delta.saturating_abs() as usize)
        } else {
            current.saturating_add(delta as usize)
        }
    }


    pub(super) fn page_scroll_lines(&self) -> i32 {
        i32::from(self.terminal_size.1.saturating_sub(4).max(1))
    }


    pub(super) fn current_agent_scroll_offset(&self) -> usize {
        self.tui_state
            .agent_scroll
            .unwrap_or_else(|| self.current_agent_max_scroll())
    }


    pub(super) fn current_agent_topology_max_scroll(&self) -> usize {
        views::agents_view::agent_topology_lines(&self.tui_state)
            .len()
            .saturating_sub(self.current_agent_topology_viewport_height())
            .min(u16::MAX as usize)
    }


    pub(super) fn current_agent_max_scroll(&self) -> usize {
        self.current_agent_output_line_count()
            .saturating_sub(self.current_agent_output_viewport_height())
            .min(u16::MAX as usize)
    }


    pub(super) fn current_log_max_scroll(&self) -> usize {
        let content_area = self.current_content_area();
        let sections =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(content_area);
        let viewport_height = sections[1].height.saturating_sub(2) as usize;
        super::super::views::logs_view::filtered_entry_count(&self.data, &self.tui_state)
            .saturating_sub(viewport_height)
            .min(u16::MAX as usize)
    }


    pub(super) fn current_git_max_scroll(&self) -> usize {
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


    pub(super) fn scroll_agent_output_by(&mut self, delta: i32) {
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

    pub(super) fn scroll_logs_by(&mut self, delta: i32) {
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

    pub(super) fn current_content_area(&self) -> Rect {
        let full_area = Rect::new(0, 0, self.terminal_size.0, self.terminal_size.1);
        let content_area = super::super::layout::responsive_outer_margin(full_area);
        let has_waves = !self.tui_state.execution_waves.is_empty();
        let wave_row_height = if has_waves { 1 } else { 0 };
        let warning_height = super::super::widgets::header_bar::warning_bar_height(&self.tui_state);
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

    pub(super) fn clamp_scroll_state_to_view(&mut self) {
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
                    self.tui_state.selected_plan_idx = self
                        .tui_state
                        .selected_plan_idx
                        .min(plan_count.saturating_sub(1));
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
            Tab::Providers => {
                if !self.tui_state.provider_statuses.is_empty() {
                    let max = self.tui_state.provider_statuses.len().saturating_sub(1);
                    self.tui_state.providers_selected =
                        self.tui_state.providers_selected.min(max);
                }
            }
        }
    }

    pub(super) fn current_agent_output_line_count(&self) -> usize {
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

    pub(super) fn current_agent_output_viewport_height(&self) -> usize {
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

    pub(super) fn current_agent_topology_viewport_height(&self) -> usize {
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

    pub(super) fn split_content_area(&self, area: Rect) -> (Rect, Option<Rect>) {
        if self.tui_state.is_text_input() && area.height > 0 {
            let sections =
                Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
            (sections[0], Some(sections[1]))
        } else {
            (area, None)
        }
    }

}
