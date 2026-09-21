//! Background channel draining: snapshot, state events, execution acks,
//! sys metrics, git, filesystem, agent topology, and agent stream clients.

use super::*;

impl App {
    pub(super) fn drain_background_channels(&mut self) {
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
                while sys.cpu_history.len() > super::super::state::MAX_METRIC_HISTORY {
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
                while sys.mem_history.len() > super::super::state::MAX_METRIC_HISTORY {
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

    pub(super) fn apply_git_bg_data(&mut self, bg: GitBgData) {
        // Derive commit/worktree views before moving view_data.
        self.tui_state.git_commit_graph = convert_git_commit_graph(&bg.view_data.commits);
        self.tui_state.git_worktree_list = convert_git_worktree_list(&bg.view_data.worktrees);
        // Propagate diff_text into the dashboard's git_diff field so the
        // Dashboard diff sub-view stays current (P7.1).  Extract before
        // the view_data move below.
        self.tui_state.git_diff = bg.view_data.diff_text.clone();
        // git_branch_tree accessed via git_view_data when present; skip clone.
        self.tui_state.git_view_data = Some(bg.view_data);
        self.tui_state.git_summary_lines = bg.summary_lines;
        self.tui_state.git_branch = bg.branch;
        self.tui_state.git_commit_short = bg.commit_short;
        self.tui_state.git_age = bg.age;
    }

    pub(super) fn drain_snapshot_channel(&mut self) {
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
    /// TUI state accordingly. `Completed` acks commit the state change (e.g.
    /// flip `is_paused`); `Rejected`/`Failed` acks show a toast/error entry
    /// and remove the pending command.
    pub(super) fn drain_execution_acks(&mut self) {
        let Some(ack_rx) = self.exec_ack_receiver.as_mut() else {
            return;
        };
        for ack in ack_rx.drain() {
            use crate::execution_control::{CommandAckStatus, ExecutionCommandKind};
            let pending_kind = self.pending_exec_commands.remove(&ack.command_id);
            match ack.status {
                CommandAckStatus::Accepted => {
                    let msg = ack.message.unwrap_or_else(|| "command accepted".into());
                    self.notifications
                        .push_back(super::super::modals::Notification::info(msg));
                }
                CommandAckStatus::Completed => {
                    // Commit the state change based on the original command kind.
                    if let Some(kind) = &pending_kind {
                        match kind {
                            ExecutionCommandKind::Pause => {
                                self.tui_state.is_paused = true;
                            }
                            ExecutionCommandKind::Resume => {
                                self.tui_state.is_paused = false;
                            }
                            _ => {}
                        }
                    }
                    let msg = ack.message.unwrap_or_else(|| "command completed".into());
                    self.notifications
                        .push_back(super::super::modals::Notification::info(msg));
                    self.render_dirty.insert(RenderDirty::SNAPSHOT);
                }
                CommandAckStatus::Rejected => {
                    let msg = ack.message.unwrap_or_else(|| "command rejected".into());
                    self.notifications
                        .push_back(super::super::modals::Notification::warn(msg));
                }
                CommandAckStatus::Failed => {
                    let msg = ack.message.unwrap_or_else(|| "command failed".into());
                    self.notifications
                        .push_back(super::super::modals::Notification::error(msg));
                }
            }
        }
    }

    pub(super) fn drain_state_events(&mut self) {
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
            // RC-1: Unified data model — the DashboardSnapshot (updated inside
            // StateHub::publish *before* the event is broadcast) is the single
            // source of truth for all plan/task/gate state.  drain_snapshot_channel()
            // always runs before drain_state_events(), so by the time we process
            // a lifecycle event here the TuiState has already been reconciled via
            // update_from_dashboard_snapshot().
            //
            // The inline handlers below update ONLY TUI-local state that the
            // snapshot cannot carry (Instant timers), or mark render_dirty so
            // the frame redraws immediately rather than waiting until the next
            // animation tick.  They MUST NOT mutate derived counters (tasks_done,
            // tasks_failed) because those are owned by the snapshot path and
            // double-counting would corrupt the progress display.
            match &event {
                roko_core::DashboardEvent::PlanStarted { plan_id, .. } => {
                    // The snapshot has already set plan.active = true.  Set
                    // started_at (a TUI-local Instant not carried by the
                    // snapshot) so tick_elapsed() can advance the live timer.
                    if let Some(plan) =
                        self.tui_state.plans.iter_mut().find(|p| p.id == *plan_id)
                    {
                        if plan.started_at.is_none() {
                            plan.started_at = Some(std::time::Instant::now());
                        }
                    }
                    self.render_dirty.insert(RenderDirty::SNAPSHOT);
                }
                roko_core::DashboardEvent::PlanCompleted { plan_id, .. } => {
                    // Freeze the elapsed timer before the snapshot clears active.
                    // elapsed_secs is TUI-local (derived from started_at Instant);
                    // the snapshot does not carry it.
                    if let Some(plan) =
                        self.tui_state.plans.iter_mut().find(|p| p.id == *plan_id)
                    {
                        if let Some(started) = plan.started_at.take() {
                            plan.elapsed_secs = started.elapsed().as_secs_f64();
                        }
                    }
                    self.render_dirty.insert(RenderDirty::SNAPSHOT);
                }
                roko_core::DashboardEvent::TaskStarted { .. }
                | roko_core::DashboardEvent::TaskCompleted { .. }
                | roko_core::DashboardEvent::GateResult { .. } => {
                    // State already applied to TuiState via update_from_dashboard_snapshot().
                    // Only mark dirty so the render fires immediately.
                    self.render_dirty.insert(RenderDirty::SNAPSHOT);
                }
                roko_core::DashboardEvent::AgentOutput {
                    agent_id, content, ..
                } => {
                    // Streaming text/tool records — handled below.
                    // Each record is pushed into the canonical AgentOutputHistory
                    // (P1-TUI-G4) so the structured semantic renderer sees typed
                    // records rather than raw text.  The legacy agent_streams
                    // chunk path is retained to keep the Live Stream panel alive.
                    let Some(record) =
                        content.strip_prefix(crate::runner::tui_bridge::STREAM_RECORD_PREFIX)
                    else {
                        // Non-prefixed line: push as plain text record.
                        self.tui_state.push_agent_output_record(
                            agent_id,
                            super::super::state::OutputRecordKind::Text,
                            content.clone(),
                            None,
                            None,
                        );
                        self.tui_state.push_agent_chunk(agent_id, content.clone());
                        continue;
                    };
                    let Ok(record) =
                        serde_json::from_str::<serde_json::Value>(record)
                    else {
                        continue;
                    };
                    let kind = record
                        .get("kind")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("text");
                    let payload = record.get("payload").cloned().unwrap_or_default();
                    match kind {
                        "text" => {
                            if let Some(text) =
                                payload.get("text").and_then(serde_json::Value::as_str)
                            {
                                self.tui_state.push_agent_output_record(
                                    agent_id,
                                    super::super::state::OutputRecordKind::Text,
                                    text.to_string(),
                                    None,
                                    None,
                                );
                                self.tui_state
                                    .push_agent_chunk(agent_id, text.to_string());
                            }
                        }
                        "reasoning" => {
                            if let Some(text) =
                                payload.get("text").and_then(serde_json::Value::as_str)
                            {
                                self.tui_state.push_agent_output_record(
                                    agent_id,
                                    super::super::state::OutputRecordKind::Reasoning,
                                    text.to_string(),
                                    None,
                                    None,
                                );
                                self.tui_state
                                    .push_agent_chunk(agent_id, format!("[thinking] {text}"));
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
                            self.tui_state.push_agent_output_record(
                                agent_id,
                                super::super::state::OutputRecordKind::ToolCall,
                                String::new(),
                                if id.is_empty() { None } else { Some(id.to_string()) },
                                Some(tool.to_string()),
                            );
                            self.tui_state
                                .push_agent_chunk(agent_id, format!("[tool ⏵ {tool} {id}]"));
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
                            self.tui_state.push_agent_output_record(
                                agent_id,
                                super::super::state::OutputRecordKind::ToolResult,
                                output.to_string(),
                                if id.is_empty() { None } else { Some(id.to_string()) },
                                None,
                            );
                            self.tui_state
                                .push_agent_chunk(agent_id, format!("[tool ✓ {id}]\n{output}"));
                        }
                        _ => {}
                    }
                }
                roko_core::DashboardEvent::AgentTopologyUpdated { .. } => {
                    // Topology changes (node/edge additions and state transitions)
                    // are already applied to the DashboardSnapshot inside
                    // StateHub::publish.  Mark dirty here so the Agents tab
                    // redraws immediately on push rather than waiting for the
                    // next animation tick (P3-TUI-1).
                    self.render_dirty.insert(RenderDirty::SNAPSHOT);
                }
                roko_core::DashboardEvent::AgentSpawned { .. }
                | roko_core::DashboardEvent::AgentCompleted { .. } => {
                    // Agent lifecycle events are reflected in the snapshot;
                    // mark dirty for immediate redraw.
                    self.render_dirty.insert(RenderDirty::SNAPSHOT);
                }
                _ => {
                    // All other events (EfficiencyEvent, etc.) are reflected
                    // in the DashboardSnapshot and handled by
                    // drain_snapshot_channel().  No inline action needed.
                }
            }
        }
    }

    pub(super) fn drain_shutdown_signal(&mut self) {
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

    pub(super) fn update_plan_completion_exit(&mut self, snapshot: &roko_core::DashboardSnapshot) {
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

    pub(super) fn request_agent_topology_refresh(&mut self) {
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

    pub(super) fn drain_agent_topology_fetch(&mut self) {
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

    pub(super) fn apply_state_hub_agent_topology(&self, topology: roko_core::AgentTopology) {
        let Some(state_hub) = &self._state_hub else {
            return;
        };

        state_hub.update_snapshot(|snapshot| snapshot.agent_topology = topology);
    }

    pub(super) fn sync_agent_stream_clients(&mut self) {
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

    pub(super) fn clear_agent_stream_clients(&mut self) {
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

    pub(super) fn drain_agent_stream_clients(&mut self) {
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
                        // Push typed record into canonical history (P1-TUI-G4).
                        self.tui_state.push_agent_output_record(
                            &agent_id,
                            super::super::state::OutputRecordKind::Text,
                            text.clone(),
                            None,
                            None,
                        );
                        self.tui_state.push_agent_chunk(&agent_id, text);
                    }
                    Ok(StreamChunk::Reasoning(text)) => {
                        self.tui_state.push_agent_output_record(
                            &agent_id,
                            super::super::state::OutputRecordKind::Reasoning,
                            text.clone(),
                            None,
                            None,
                        );
                        self.tui_state
                            .push_agent_chunk(&agent_id, format!("[reasoning] {text}"));
                    }
                    Ok(StreamChunk::ToolCall(tool_call)) => {
                        // Extract name and id from the tool_call JSON for semantic record.
                        let tool_name = tool_call
                            .get("name")
                            .or_else(|| tool_call.get("tool"))
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_string);
                        let tool_id = tool_call
                            .get("tool_id")
                            .or_else(|| tool_call.get("id"))
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_string);
                        self.tui_state.push_agent_output_record(
                            &agent_id,
                            super::super::state::OutputRecordKind::ToolCall,
                            String::new(),
                            tool_id,
                            tool_name,
                        );
                        if let Ok(text) = serde_json::to_string(&tool_call) {
                            self.tui_state
                                .push_agent_chunk(&agent_id, format!("[tool_call] {text}"));
                        }
                    }
                    Ok(StreamChunk::Usage(usage)) => {
                        // Usage events are informational; push as system records.
                        if let Ok(text) = serde_json::to_string(&usage) {
                            self.tui_state.push_agent_output_record(
                                &agent_id,
                                super::super::state::OutputRecordKind::System,
                                format!("[usage] {text}"),
                                None,
                                None,
                            );
                            self.tui_state
                                .push_agent_chunk(&agent_id, format!("[usage] {text}"));
                        }
                    }
                    Ok(StreamChunk::Error(error)) => {
                        self.tui_state.push_agent_output_record(
                            &agent_id,
                            super::super::state::OutputRecordKind::Error,
                            error.clone(),
                            None,
                            None,
                        );
                        self.tui_state
                            .push_agent_chunk(&agent_id, format!("[error] {error}"));
                    }
                    Ok(StreamChunk::Done { session }) => {
                        if let Some(session_id) = session {
                            let msg = format!("[done] session {session_id}");
                            self.tui_state.push_agent_output_record(
                                &agent_id,
                                super::super::state::OutputRecordKind::System,
                                msg.clone(),
                                None,
                                None,
                            );
                            self.tui_state.push_agent_chunk(&agent_id, msg);
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

    pub(super) fn merge_process_metrics(&mut self, samples: Vec<ProcessMetricSample>) {
        const PROCESS_HISTORY_LIMIT: usize = 60;

        let mut existing = std::mem::take(&mut self.tui_state.process_metrics);
        let mut merged = Vec::with_capacity(samples.len());

        for sample in samples {
            let mut metric =
                if let Some(index) = existing.iter().position(|entry| entry.pid == sample.pid) {
                    existing.swap_remove(index)
                } else {
                    super::super::state::ProcessMetrics {
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
}
