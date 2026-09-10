//! TUI event loop: `run()` async entry point and `main_loop()` sync entry point.

use std::io::Stdout;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};
use ratatui::Frame;

use super::*;

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
            let input_pending = app.render_dirty.contains(RenderDirty::INPUT)
                && app.frame_stats.last_input_at.is_some();
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

    pub(super) fn main_loop(&mut self, terminal: &mut TuiTerminal) -> Result<()> {
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
                let input_pending = self.render_dirty.contains(RenderDirty::INPUT)
                    && self.frame_stats.last_input_at.is_some();
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

    // -----------------------------------------------------------------------
    // Drawing
    // -----------------------------------------------------------------------

    pub(super) fn draw(&mut self, frame: &mut Frame<'_>) {
        let theme = self.theme;
        let full_area = frame.area();

        // Establish a real canvas every frame.  Relying on terminal defaults
        // leaves stale cells and makes post-processing dependent on the host
        // theme; Mori's visual hierarchy starts from an explicit black field.
        frame.render_widget(Block::default().style(Theme::block_style()), full_area);

        // Guard: if the terminal is too small for useful rendering, show a
        // short message instead of attempting layout that would panic or clip.
        if super::super::layout::is_terminal_too_small(full_area) {
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
        let content_area = super::super::layout::responsive_outer_margin(full_area);

        // Main layout: header + warning + wave + optional sub-view bar +
        // content + footer. Dashboard and Agents already render their own
        // purpose-built internal navigation bars.
        let has_waves = !self.tui_state.execution_waves.is_empty();
        let wave_row_height = if has_waves { 1 } else { 0 };
        let warning_height = super::super::widgets::header_bar::warning_bar_height(&self.tui_state);
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
            super::super::widgets::header_bar::render_warning_bar(frame, main_layout[1], &self.tui_state);
        }

        // Wave indicator row (only when waves exist)
        if has_waves {
            super::super::widgets::wave_progress::render_wave_progress(
                frame,
                main_layout[2],
                &self.tui_state,
            );
        }

        // Breadcrumb trail: Tab > SubView > Focus
        super::super::widgets::header_bar::render_breadcrumb_bar(frame, main_layout[3], &self.tui_state);

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
            super::super::postfx_pipeline::apply_pipeline(
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
                super::super::postfx::fade_overlay(content_area, frame.buffer_mut(), t);
            }
        }

        // Dim overlay before modals
        if self.tui_state.active_modal.is_some() {
            let buf = frame.buffer_mut();
            super::super::postfx::dim_overlay(content_area, buf, 0.45);
        }

        // Modal rendering
        modals_mod::render_modals(
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
    // Rendering helpers
    // -----------------------------------------------------------------------

    pub(super) fn render_tab_header(&self, frame: &mut Frame<'_>, area: Rect, _theme: &Theme) {
        // Use the Mori-ported header_bar widget with full progress/ETA/tokens
        super::super::widgets::header_bar::render_header_bar(frame, area, &self.tui_state);
    }

    pub(super) fn render_subview_bar(&self, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
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

    pub(super) fn render_status_footer(&self, frame: &mut Frame<'_>, area: Rect, _theme: &Theme) {
        // Use the Mori-ported status_bar widget with context-sensitive hints
        super::super::widgets::status_bar::render_status_bar(frame, area, &self.tui_state);
    }

    pub(super) fn render_input_bar(&self, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let label = self.tui_state.input_mode_label();
        if label.is_empty() || area.width == 0 {
            return;
        }

        let buffer = match self.tui_state.input_mode {
            InputMode::Inject => self.tui_state.message_input.as_str(),
            InputMode::Filter => self.tui_state.filter_text.as_str(),
            InputMode::LogSearch => self.tui_state.log_search.pattern.as_str(),
            InputMode::PlanFilter => self.tui_state.plan_tree_filter.pattern.as_str(),
            InputMode::AgentOutputSearch => self.tui_state.agent_output_search.pattern.as_str(),
            _ => return,
        };

        // Build suffix for search mode (match count + filter mode indicator)
        let suffix = match self.tui_state.input_mode {
            InputMode::LogSearch if !self.tui_state.log_search.pattern.is_empty() => {
                let mode_label = match self.tui_state.log_search.mode {
                    super::super::state::SearchMode::Highlight => "highlight",
                    super::super::state::SearchMode::Filter => "filter",
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
            InputMode::AgentOutputSearch
                if !self.tui_state.agent_output_search.pattern.is_empty() =>
            {
                if self.tui_state.agent_output_search.pattern_error {
                    " [invalid regex]".to_string()
                } else {
                    format!(
                        " [{}/{}]",
                        self.tui_state.agent_output_search.current_match + 1,
                        self.tui_state.agent_output_search.match_count,
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

    // -----------------------------------------------------------------------
    // Tick policy helpers
    // -----------------------------------------------------------------------

    pub(super) fn has_modal(&self) -> bool {
        self.tui_state.active_modal.is_some()
    }

    /// Build the tick-policy inputs from current `App` state.
    ///
    /// This keeps the pure `next_tick_policy()` function free of `App`
    /// coupling while letting both event loops share identical policy.
    pub(super) fn tick_policy_inputs(&self) -> TickPolicyInputs {
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
    pub(super) fn current_tick_duration(&self) -> std::time::Duration {
        next_tick_policy(&self.tick_policy_inputs()).duration()
    }

    // -----------------------------------------------------------------------
    // Snapshot refresh helpers
    // -----------------------------------------------------------------------

    /// Full refresh — async version for the connected `run()` path.
    pub(super) async fn refresh_snapshot_async(&mut self) {
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
    pub(super) fn refresh_snapshot(&mut self) {
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
    pub(super) fn tick_snapshot(&mut self) {
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

    // -----------------------------------------------------------------------
    // Verdicts aggregator
    // -----------------------------------------------------------------------

    pub(super) async fn reseed_verdicts_aggregator(&mut self) {
        self.verdicts_aggregator = VerdictsAggregator::open(&self.workdir).await.ok();
    }

    pub(super) async fn refresh_verdicts_from_aggregator(&mut self) {
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
    pub(super) fn reseed_verdicts_aggregator_blocking(&mut self) {
        self.verdicts_aggregator = VerdictsAggregator::open_blocking(&self.workdir).ok();
    }

    /// Sync variant of [`Self::refresh_verdicts_from_aggregator`] for the
    /// standalone `main_loop` path (no Tokio runtime active).
    pub(super) fn refresh_verdicts_from_aggregator_blocking(&mut self) {
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

    // -----------------------------------------------------------------------
    // Notification helpers
    // -----------------------------------------------------------------------

    pub(super) fn expire_notifications(&mut self) {
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
    pub(super) fn push_notification_history(&mut self, notif: super::super::modals::Notification) {
        let id = self.tui_state.notification_next_id;
        self.tui_state.notification_next_id += 1;
        let record = super::super::modals::NotificationRecord {
            id,
            created_at: notif.created,
            level: notif.level,
            source: String::new(),
            message: super::super::modals::redact_message(&notif.message),
            related_run: None,
            related_task: None,
            dismissed_at: None,
        };
        self.tui_state.notification_history.push_back(record);
        while self.tui_state.notification_history.len() > super::super::modals::MAX_HISTORY {
            self.tui_state.notification_history.pop_front();
            self.tui_state.notification_evicted_count += 1;
        }
    }
}
