//! TuiAction dispatch, key/mouse handling, and config/marketplace form helpers.

use super::*;

impl App {
    pub(super) fn handle_key(&mut self, key: KeyEvent) {
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

    pub(super) fn dispatch_action(&mut self, action: TuiAction) {
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
                    Tab::Providers => FocusZone::ProviderList,
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
                    self.tab_transition = Some((Instant::now(), Duration::from_millis(200)));
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
                        ModalState::QueueOverview { selected_index, .. } => {
                            *selected_index = selected_index.saturating_sub(1);
                        }
                        ModalState::TaskPicker { selected_index, .. } => {
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
                        ModalState::QueueOverview { selected_index, .. } => {
                            *selected_index = selected_index.saturating_add(1);
                        }
                        ModalState::TaskPicker { selected_index, .. } => {
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
                    "state",
                    "learn",
                    "jobs",
                    "prd",
                    "prd/published",
                    "prd/drafts",
                    "task-outputs",
                    "research",
                    "subscriptions",
                    "templates",
                ] {
                    let _ = std::fs::create_dir_all(roko_dir.join(sub));
                }
                // Create roko.toml if absent: the `roko init` template, which
                // is checked like `roko config validate` before it is written.
                if !roko_toml.exists() {
                    let provider = crate::init::InitProvider::detect();
                    let written = crate::init::write_init_config(&self.workdir, false, provider);
                    if let Err(err) = written {
                        tracing::warn!("failed to write roko.toml: {err:#}");
                    }
                }
                // Update workspace state
                self.tui_state.workdir = self.workdir.clone();
                self.tui_state.refresh_mcp_config_view();
                // Transition to the confirmation screen
                self.tui_state.active_modal = Some(ModalState::Welcome { initialized: true });
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
                    .push_back(super::super::modals::Notification::info(&format!(
                        "Screen postfx {state}"
                    )));
            }
            TuiAction::CycleEffectsPreset => {
                let preset = self.fx_config.cycle_preset();
                match self.fx_config.save_preset(&self.workdir) {
                    Ok(()) => {
                        self.notifications
                            .push_back(super::super::modals::Notification::info(&format!(
                                "Effects: {}",
                                preset.label()
                            )));
                    }
                    Err(error) => {
                        self.notifications
                            .push_back(super::super::modals::Notification::error(&format!(
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
                    let cmd = sender.build_command(kind.clone(), None, None, None);
                    let cmd_id = cmd.command_id.clone();
                    match sender.try_send(cmd) {
                        Ok(()) => {
                            // Track the pending command so we can commit state on ack.
                            self.pending_exec_commands.insert(cmd_id, kind);
                            // State changes on Completed ack; show pending.
                            self.notifications
                                .push_back(super::super::modals::Notification::info(
                                    if requested_pause {
                                        "Pause requested"
                                    } else {
                                        "Resume requested"
                                    },
                                ));
                        }
                        Err(crate::execution_control::CommandSendError::Full(_)) => {
                            self.notifications
                                .push_back(super::super::modals::Notification::warn(
                                    "command queue full",
                                ));
                        }
                        Err(crate::execution_control::CommandSendError::Disconnected(_)) => {
                            self.notifications
                                .push_back(super::super::modals::Notification::warn(
                                    "executor disconnected",
                                ));
                        }
                    }
                } else {
                    self.notifications
                        .push_back(super::super::modals::Notification::warn(
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
                // The dashboard's right panel renders its own sub-tab, so its
                // a/o/d/e/... keys (e.g. `e:Verify`) must switch that one too.
                if matches!(self.tui_state.active_tab, Tab::Dashboard) {
                    self.tui_state.set_sub_tab_for(Tab::Dashboard, idx);
                }
                // Move focus to the right panel so subsequent Up/Down/j/k
                // keys scroll the detail pane (e.g. procs_scroll for the
                // Processes sub-tab) instead of the plan tree (P6.5/P7.3).
                if matches!(self.tui_state.active_tab, Tab::Dashboard | Tab::Plans) {
                    self.tui_state.focus = FocusZone::RightPanel;
                }
            }
            TuiAction::ApproveCommand => {
                // P1-40: Emit SurfaceEvent for the approval action.
                if let Some(approval) = &self.tui_state.pending_approval {
                    self.emit_surface_event(roko_core::runtime_event::SurfaceEvent::HumanRespond {
                        run_id: approval.run_id.clone().unwrap_or_default(),
                        cell_id: approval.approval_id.clone().unwrap_or_default(),
                        response: serde_json::json!({"approved": true}),
                    });
                }
                if !self.resolve_active_approval(true) {
                    self.tui_state.pending_approval = None;
                }
            }
            TuiAction::ApproveAll => {
                if let Some(approval) = &self.tui_state.pending_approval {
                    self.emit_surface_event(roko_core::runtime_event::SurfaceEvent::HumanRespond {
                        run_id: approval.run_id.clone().unwrap_or_default(),
                        cell_id: approval.approval_id.clone().unwrap_or_default(),
                        response: serde_json::json!({"approved": true, "all": true}),
                    });
                }
                if !self.resolve_active_approval(true) {
                    self.tui_state.pending_approval = None;
                }
            }
            TuiAction::RejectCommand => {
                if let Some(approval) = &self.tui_state.pending_approval {
                    self.emit_surface_event(roko_core::runtime_event::SurfaceEvent::HumanRespond {
                        run_id: approval.run_id.clone().unwrap_or_default(),
                        cell_id: approval.approval_id.clone().unwrap_or_default(),
                        response: serde_json::json!({"approved": false}),
                    });
                }
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
                    // Write inject signal to .roko/signals.jsonl for the plan runner
                    let signal_path = self.workdir.join(".roko").join("signals.jsonl");
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
                            roko_core::io::write_jsonl_line(&mut f, &entry.to_string())
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
                        .push_back(super::super::modals::Notification::info(format!(
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
                } else if self.tui_state.input_mode == InputMode::AgentOutputSearch {
                    self.tui_state.agent_output_search.pattern.push(c);
                    self.tui_state.agent_output_search.recompile();
                    self.refresh_agent_output_search_matches();
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
                } else if self.tui_state.input_mode == InputMode::AgentOutputSearch {
                    self.tui_state.agent_output_search.pattern.pop();
                    self.tui_state.agent_output_search.recompile();
                    self.refresh_agent_output_search_matches();
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
                    let signal_path = self.workdir.join(".roko").join("signals.jsonl");
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
                            roko_core::io::write_jsonl_line(&mut f, &entry.to_string())
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
                        .push_back(super::super::modals::Notification::info(format!(
                            "Confirmed: {}",
                            truncate_str(&action_str, 40)
                        )));
                    // Also send the corresponding ExecutionCommand to the executor
                    // (P1.1). If no executor channel is connected, show a warning
                    // so the user knows the action was not forwarded.
                    let dispatched = self.send_tui_command_for_confirm(action);
                    if !dispatched {
                        self.notifications
                            .push_back(super::super::modals::Notification::warn(
                                "not connected to a running executor — command not forwarded",
                            ));
                    }
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
                    self.tui_state.active_modal = Some(ModalState::NotificationHistory {
                        scroll_offset: 0,
                        selected_index: 0,
                        filter: super::super::modals::LevelFilter::default(),
                    });
                }
            }
            TuiAction::NotifFilterToggle(key) => {
                if let Some(ModalState::NotificationHistory {
                    filter,
                    selected_index,
                    ..
                }) = self.tui_state.active_modal.as_mut()
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
                    let filtered: Vec<&super::super::modals::NotificationRecord> = self
                        .tui_state
                        .notification_history
                        .iter()
                        .rev()
                        .filter(|e| filter.accepts(e.level))
                        .collect();
                    filtered
                        .get(*selected_index)
                        .map(|entry| (entry.related_task.clone(), entry.related_run.clone()))
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
                            self.notifications
                                .push_back(super::super::modals::Notification::warn(format!(
                                    "Task {task_id} not found (may be stale)"
                                )));
                        }
                    } else if let Some(run_id) = related_run {
                        if let Some(idx) = self.tui_state.plans.iter().position(|p| p.id == run_id)
                        {
                            self.tui_state.active_modal = Some(ModalState::PlanDetail {
                                plan_id: self.tui_state.plans[idx].id.clone(),
                            });
                        } else {
                            self.notifications
                                .push_back(super::super::modals::Notification::warn(format!(
                                    "Run {run_id} not found (may be stale)"
                                )));
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
                Tab::Inspect | Tab::Marketplace | Tab::Atelier | Tab::Learning | Tab::Providers => {
                }
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
                Tab::Inspect | Tab::Marketplace | Tab::Atelier | Tab::Learning | Tab::Providers => {
                }
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
                let modal_action = modals_mod::ConfirmAction::Custom {
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
                    let modal_action = modals_mod::ConfirmAction::Custom {
                        message: format!("Cancel plan '{plan_id}'?"),
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
                    let modal_action = modals_mod::ConfirmAction::Custom {
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
                    let modal_action = modals_mod::ConfirmAction::Custom {
                        message: format!("Cancel plan '{plan_id}'?"),
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
                    let modal_action = modals_mod::ConfirmAction::Custom {
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
                    if let Some(super::super::config_meta::ConfigItem::Header(_)) =
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
                    if let Some(super::super::config_meta::ConfigItem::Header(_)) =
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
                        super::super::config_meta::ConfigItem::Field {
                            meta,
                            value,
                            source,
                        } => {
                            match &meta.kind {
                                super::super::config_meta::ConfigFieldKind::Bool => {
                                    let new_val = if value == "true" { "false" } else { "true" };
                                    self.tui_state
                                        .config_pending
                                        .insert(meta.key.to_string(), new_val.to_string());
                                }
                                super::super::config_meta::ConfigFieldKind::ReadOnly => {}
                                super::super::config_meta::ConfigFieldKind::Enum(_)
                                | super::super::config_meta::ConfigFieldKind::Int { .. } => {
                                    // For enums/presets, Enter cycles right
                                    if *source != super::super::config_meta::ConfigSource::Env {
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
                                    if *source != super::super::config_meta::ConfigSource::Env {
                                        self.tui_state.config_editing = true;
                                        self.tui_state.config_edit_buffer = value.clone();
                                        self.tui_state.config_edit_key = Some(meta.key.to_string());
                                        self.tui_state.input_mode = InputMode::ConfigEdit;
                                    }
                                }
                            }
                        }
                        super::super::config_meta::ConfigItem::SaveButton => {
                            self.save_config_changes();
                        }
                        super::super::config_meta::ConfigItem::Header(_) => {}
                    }
                }
            }
            TuiAction::ConfigCycleLeft | TuiAction::ConfigCycleRight => {
                let items = self.tui_state.config_items().to_vec();
                if let Some(super::super::config_meta::ConfigItem::Field {
                    meta,
                    value,
                    source,
                }) = items.get(self.tui_state.config_cursor)
                {
                    if *source == super::super::config_meta::ConfigSource::Env {
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
                // Use hit_test registry to determine click target (#368).
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
                    match region.click_target {
                        super::super::hit_test::ClickTarget::SwitchTab(idx) => {
                            if let Some(&tab) = Tab::ALL.get(idx) {
                                self.dispatch_action(TuiAction::SwitchTab(tab));
                            }
                        }
                        super::super::hit_test::ClickTarget::SwitchSubView(idx) => {
                            self.dispatch_action(TuiAction::SwitchSubView(idx));
                        }
                        super::super::hit_test::ClickTarget::SetFocus(zone) => {
                            let mapped = self.map_hit_zone(zone);
                            self.tui_state.focus = mapped;
                        }
                        super::super::hit_test::ClickTarget::None => {}
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
                // Dashboard owns ten purpose-built detail panels.  It does
                // not use the older four-item generic SubView list.
                let max = if tab == Tab::Dashboard {
                    9
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
                use super::super::state::SearchMode;
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
                    // Clipboard integration deferred (arboard/copypasta).
                    let _ = text;
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

            // -- Agent output search (#367) --
            TuiAction::StartAgentOutputSearch => {
                self.tui_state.input_mode = InputMode::AgentOutputSearch;
                self.tui_state.agent_output_search.active = true;
                self.tui_state.agent_output_search.pattern.clear();
                self.tui_state.agent_output_search.recompile();
            }
            TuiAction::AcceptAgentOutputSearch => {
                self.tui_state.input_mode = InputMode::Normal;
                // Keep search active with the current pattern for n/N navigation
            }
            TuiAction::CancelAgentOutputSearch => {
                self.tui_state.input_mode = InputMode::Normal;
                self.tui_state.agent_output_search.clear();
            }
            TuiAction::NextAgentOutputMatch => {
                self.tui_state.agent_output_search.next_match();
                // Scroll to the current match if we have one
                if self
                    .tui_state
                    .agent_output_search
                    .current_match_seq()
                    .is_some()
                {
                    // Pin the scroll (switch from tail to pinned mode)
                    if self.tui_state.agent_scroll.is_none() {
                        self.tui_state.agent_scroll = Some(0);
                    }
                }
            }
            TuiAction::PrevAgentOutputMatch => {
                self.tui_state.agent_output_search.prev_match();
                if self
                    .tui_state
                    .agent_output_search
                    .current_match_seq()
                    .is_some()
                {
                    if self.tui_state.agent_scroll.is_none() {
                        self.tui_state.agent_scroll = Some(0);
                    }
                }
            }
            TuiAction::ToggleAgentOutputFold => {
                // Collect tool IDs from the selected agent's output and toggle
                // the most recent one. Falls back to toggling the last tool ID
                // when no scroll position context is available.
                use crate::tui::widgets::stream_output::{StreamRecord, parse_stream_line};
                let selected_id = self
                    .tui_state
                    .agents
                    .get(self.tui_state.selected_agent)
                    .map(|a| a.id.as_str())
                    .unwrap_or("");
                let history_records = self.tui_state.agent_output_history.records_for(selected_id);
                let lines: Vec<String> = if history_records.is_empty() {
                    self.tui_state
                        .agents
                        .get(self.tui_state.selected_agent)
                        .map(|a| a.output_lines.clone())
                        .unwrap_or_default()
                } else {
                    history_records.iter().map(|r| r.text.clone()).collect()
                };
                // Find the last tool ID in the output.
                let mut last_tool_id: Option<String> = None;
                for line in &lines {
                    match parse_stream_line(line) {
                        StreamRecord::ToolStart { tool_id, .. }
                        | StreamRecord::ToolResult { tool_id, .. } => {
                            last_tool_id = Some(tool_id);
                        }
                        _ => {}
                    }
                }
                if let Some(tid) = last_tool_id {
                    if !self.tui_state.agent_output_unfolded.remove(&tid) {
                        self.tui_state.agent_output_unfolded.insert(tid);
                    }
                }
            }

            // -- Recovery keybindings (#119) --
            TuiAction::SoftRetry => {
                if let Some(plan) = self.tui_state.plans.get(self.tui_state.selected_plan_idx) {
                    if plan.status.is_failed() || plan.tasks_failed > 0 {
                        let plan_id = plan.id.clone();
                        self.open_confirm_modal(ConfirmAction::SoftRetryPlan(plan_id));
                    } else {
                        self.notifications
                            .push_back(super::super::modals::Notification::info(
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
                        .push_back(super::super::modals::Notification::info(truncate_str(
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

    pub(super) fn selected_plan_id(&self) -> Option<String> {
        self.tui_state
            .plans
            .get(self.tui_state.selected_plan_idx)
            .map(|plan| plan.id.clone())
    }

    pub(super) fn visible_plan_indices(&self) -> Vec<usize> {
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

    pub(super) fn normalize_selected_plan_for_filter(&mut self) {
        let visible = self.visible_plan_indices();
        if visible.is_empty() {
            return;
        }
        if !visible.contains(&self.tui_state.selected_plan_idx) {
            self.tui_state.selected_plan_idx = visible[0];
            self.tui_state.plan_scroll_offset = 0;
        }
    }

    pub(super) fn move_selected_plan(&mut self, direction: i8) {
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

    pub(super) fn refresh_log_search_matches(&mut self) {
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

    pub(super) fn current_log_match_display_index(&self) -> Option<usize> {
        let search = &self.tui_state.log_search;
        if search.mode == super::super::state::SearchMode::Filter {
            (search.match_count > 0).then_some(search.current_match)
        } else {
            search.match_indices.get(search.current_match).copied()
        }
    }

    /// Refresh agent output search matches for the currently selected agent (#367).
    pub(super) fn refresh_agent_output_search_matches(&mut self) {
        let selected_id = self
            .tui_state
            .agent_summaries
            .get(self.tui_state.selected_agent)
            .map(|a| a.id.clone())
            .unwrap_or_default();
        self.tui_state
            .agent_output_search
            .update_matches(&self.tui_state.agent_output_history, &selected_id);
    }

    pub(super) fn current_git_branch(&self) -> String {
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

    pub(super) fn completed_plan_branches(&self) -> Vec<String> {
        self.tui_state
            .plans
            .iter()
            .filter(|plan| !plan.active && !plan.status.is_failed())
            .map(|plan| plan.id.clone())
            .collect()
    }

    /// Map a hit_test::FocusZone to the input::FocusZone used by keyboard/scroll routing.
    pub(super) fn handle_mouse(&mut self, mouse: MouseEvent) {
        // When a modal is open, scroll/click cannot affect underlying content
        // (#368). Route scroll to the modal and consume all other events.
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

    pub(super) fn save_config_changes(&mut self) {
        if self.tui_state.config_pending.is_empty() {
            self.notifications
                .push_back(super::super::modals::Notification::info(
                    "No pending changes to save",
                ));
            return;
        }

        match super::super::config_meta::save_pending_edits(
            &self.workdir,
            &self.tui_state.config_pending,
        ) {
            Ok(()) => {
                self.tui_state.config_pending.clear();
                self.tui_state.invalidate_config_cache();
                self.fx_config = EffectsConfig::load_from_root(&self.workdir);
                self.pending_refresh = true;
                self.notifications
                    .push_back(super::super::modals::Notification::info(
                        "Config saved and reloaded",
                    ));
            }
            Err(error) => {
                self.notifications
                    .push_back(super::super::modals::Notification::error(&format!(
                        "Save failed: {error}"
                    )));
            }
        }
    }

    pub(super) fn submit_marketplace_job(&mut self) {
        let title = self.tui_state.job_form_title.trim().to_string();
        if title.is_empty() {
            self.notifications
                .push_back(super::super::modals::Notification::warn(
                    "Job title is required",
                ));
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
                .push_back(super::super::modals::Notification::error(&format!(
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
                        .push(super::super::state::CommandResult {
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
                        .push_back(super::super::modals::Notification::info(format!(
                            "Job '{title}' created"
                        )));
                }
                Err(e) => {
                    self.notifications
                        .push_back(super::super::modals::Notification::error(&format!(
                            "Failed to write job file: {e}"
                        )));
                }
            },
            Err(e) => {
                self.notifications
                    .push_back(super::super::modals::Notification::error(&format!(
                        "Failed to serialize job: {e}"
                    )));
            }
        }
    }

    pub(super) fn current_view_state(&self) -> ViewState {
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
            Tab::Providers => ViewState {
                scroll: self
                    .tui_state
                    .providers_detail_scroll
                    .min(u16::MAX as usize) as u16,
                selected: self.tui_state.providers_selected,
                sub_tab: self.tui_state.sub_tab_for(Tab::Providers),
                secondary_selected: 0,
                auto_tail: false,
                search_query: String::new(),
            },
        }
    }

    pub(super) fn git_branch_count(&self) -> usize {
        self.tui_state
            .git_view_data
            .as_ref()
            .map_or(self.tui_state.git_branch_tree.len(), |data| {
                data.branches.len()
            })
    }

    pub(super) fn dismiss_all_modals(&mut self) {
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
}
