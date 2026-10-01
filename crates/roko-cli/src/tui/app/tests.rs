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
    let mut app = App::new(dir.path()).with_execution_command_sender(sender, ack_receiver);

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
        app.notifications
            .iter()
            .any(|n| n.message.contains("Pause requested"))
    );

    // Simulate ack completing the pause state change (the real
    // event loop commits on Completed ack; in tests we do it manually).
    app.tui_state.is_paused = true;

    // Second toggle: should send Resume
    app.dispatch_action(TuiAction::TogglePause);
    let received = cmd_rx.try_recv().unwrap();
    assert_eq!(
        received.kind,
        crate::execution_control::ExecutionCommandKind::Resume
    );
}

/// gap-c002bb: the reset keys cancel the selected plan, and their
/// confirmation says so; a Graph run cannot reset a plan yet.
#[test]
fn reset_plan_key_confirms_and_sends_a_cancel() {
    let dir = tempdir().unwrap();
    let (sender, mut cmd_rx, _ack_tx, ack_rx) =
        crate::execution_control::ExecutionCommandSender::channel("test-run");
    let ack_receiver = crate::execution_control::CommandAckReceiver::new(ack_rx);
    let mut app = App::new(dir.path()).with_execution_command_sender(sender, ack_receiver);
    app.tui_state.plans = vec![super::super::state::PlanEntry {
        id: "plan-7".to_string(),
        ..Default::default()
    }];

    app.dispatch_action(TuiAction::RequestConfirm(ConfirmAction::ResetSelectedPlan(
        String::new(),
    )));
    let pending = app.tui_state.pending_confirm.clone().unwrap();
    assert_eq!(pending.to_string(), "Cancel plan plan-7?");
    app.dispatch_action(TuiAction::ConfirmYes);

    let sent = cmd_rx.try_recv().unwrap();
    assert_eq!(
        sent.kind,
        crate::execution_control::ExecutionCommandKind::Cancel
    );
    assert_eq!(sent.plan_id.as_deref(), Some("plan-7"));
}

/// bug-6c3491: no transport reaches a live run yet, so the inject key says
/// so and writes nothing, instead of reporting "Injected" for a directive
/// nothing reads.
#[test]
fn inject_key_fails_closed_and_writes_nothing() {
    let dir = tempdir().unwrap();
    let mut app = App::new(dir.path());

    app.dispatch_action(TuiAction::StartInject);
    assert_eq!(app.tui_state.input_mode, InputMode::Normal);
    assert!(
        app.notifications
            .iter()
            .any(|n| n.message.contains("not available"))
    );

    // Text typed into inject mode some other way is not sent either.
    app.tui_state.input_mode = InputMode::Inject;
    app.tui_state.message_input = "ship it".to_string();
    app.dispatch_action(TuiAction::SubmitInject);
    assert_eq!(app.tui_state.input_mode, InputMode::Normal);
    assert!(app.tui_state.message_input.is_empty());
    assert!(!dir.path().join(".roko/signals.jsonl").exists());
    assert!(
        app.notifications
            .iter()
            .all(|n| !n.message.starts_with("Injected"))
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
fn app_defaults_to_mouse_capture_enabled() {
    let dir = tempdir().unwrap();
    let app = App::new(dir.path());
    assert!(app.capture_mouse);
}

#[test]
fn without_mouse_capture_disables_mouse() {
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

fn publish_plan_set(hub: &crate::state_hub::SharedStateHub, plan_ids: &[&str]) {
    hub.publish(roko_core::DashboardEvent::PlanSetLoaded {
        plans: plan_ids
            .iter()
            .map(|plan_id| roko_core::dashboard_snapshot::PlanSetEntry {
                plan_id: (*plan_id).to_string(),
                title: (*plan_id).to_string(),
                tasks_total: 2,
                ..Default::default()
            })
            .collect(),
    });
}

fn run_plan(hub: &crate::state_hub::SharedStateHub, plan_id: &str, success: bool) {
    hub.publish(roko_core::DashboardEvent::PlanStarted {
        plan_id: plan_id.to_string(),
        tasks_total: 2,
    });
    hub.publish(roko_core::DashboardEvent::PlanCompleted {
        plan_id: plan_id.to_string(),
        success,
    });
}

#[test]
fn connected_app_exits_only_after_whole_plan_set_completes() {
    let dir = tempdir().unwrap();
    let hub = crate::state_hub::shared_state_hub();
    let mut app = App::new_connected(dir.path(), &hub).with_exit_on_plan_completion();

    publish_plan_set(&hub, &["01-first", "02-second"]);
    app.drain_snapshot_channel();
    assert!(app.running);

    // Between plans nothing is active; the set is not done, so stay open.
    run_plan(&hub, "01-first", true);
    app.drain_snapshot_channel();
    assert!(app.running, "TUI exited in the gap between plans");

    run_plan(&hub, "02-second", false);
    app.drain_snapshot_channel();
    assert!(!app.running);
}

#[test]
fn connected_app_exits_when_blocked_plan_closes_the_set() {
    let dir = tempdir().unwrap();
    let hub = crate::state_hub::shared_state_hub();
    let mut app = App::new_connected(dir.path(), &hub).with_exit_on_plan_completion();

    publish_plan_set(&hub, &["01-first", "02-blocked"]);
    run_plan(&hub, "01-first", false);
    app.drain_snapshot_channel();
    assert!(app.running);

    // A plan blocked by a failed prerequisite never starts.
    hub.publish(roko_core::DashboardEvent::PlanCompleted {
        plan_id: "02-blocked".to_string(),
        success: false,
    });
    app.drain_snapshot_channel();
    assert!(!app.running);
}

#[test]
fn connected_app_exits_when_run_reaches_terminal_outcome() {
    let dir = tempdir().unwrap();
    let hub = crate::state_hub::shared_state_hub();
    let mut app = App::new_connected(dir.path(), &hub).with_exit_on_plan_completion();

    publish_plan_set(&hub, &["01-first", "02-second"]);
    hub.publish(roko_core::DashboardEvent::PlanStarted {
        plan_id: "01-first".to_string(),
        tasks_total: 2,
    });
    app.drain_snapshot_channel();
    assert!(app.running);

    hub.publish(roko_core::DashboardEvent::RunCompleted {
        outcome: "cancelled".to_string(),
        duration_ms: 10,
        cleanup_degraded: false,
        surviving_agent_ids: Vec::new(),
        surviving_agent_pids: Vec::new(),
    });
    app.drain_snapshot_channel();
    assert!(!app.running);
}

#[test]
fn connected_app_does_not_treat_idle_plan_as_run_completion() {
    let dir = tempdir().unwrap();
    let hub = crate::state_hub::shared_state_hub();
    let mut app = App::new_connected(dir.path(), &hub).with_exit_on_plan_completion();

    // Without an announced plan set, one finished plan says nothing about
    // whether more plans follow.
    run_plan(&hub, "live-plan", true);
    app.drain_snapshot_channel();
    assert!(app.running);
}

#[test]
fn connected_plan_set_keeps_run_clock_and_whole_set_totals_between_plans() {
    let dir = tempdir().unwrap();
    let hub = crate::state_hub::shared_state_hub();
    let mut app = App::new_connected(dir.path(), &hub).with_exit_on_plan_completion();

    publish_plan_set(&hub, &["02-second", "01-first", "03-third"]);
    hub.publish(roko_core::DashboardEvent::PlanStarted {
        plan_id: "02-second".to_string(),
        tasks_total: 2,
    });
    app.drain_snapshot_channel();
    let started = app.tui_state.run_started.expect("run clock started");

    hub.publish(roko_core::DashboardEvent::PlanCompleted {
        plan_id: "02-second".to_string(),
        success: true,
    });
    app.drain_snapshot_channel();

    assert!(app.tui_state.plan_set_running);
    assert_eq!(app.tui_state.run_started, Some(started), "run clock reset");
    let plans = app
        .tui_state
        .plans
        .iter()
        .map(|plan| (plan.id.as_str(), plan.status))
        .collect::<Vec<_>>();
    assert_eq!(
        plans,
        vec![
            ("02-second", crate::tui::state::PlanPhase::Done),
            ("01-first", crate::tui::state::PlanPhase::Pending),
            ("03-third", crate::tui::state::PlanPhase::Pending),
        ]
    );
    assert_eq!(app.tui_state.task_counts(), (0, 6));
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
    std::fs::write(roko_dir.join("events.jsonl"), format!("{stale_event}\n")).expect("events log");

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
                    ..Default::default()
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
                    ..Default::default()
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
        super::super::modals::NotificationLevel::Error
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
        super::super::modals::Notification::info("same message"),
    );
    push_deduped_notification(
        &mut notifications,
        super::super::modals::Notification::info("same message"),
    );
    assert_eq!(notifications.len(), 1);
}

#[test]
fn notification_cap_at_20() {
    let dir = tempdir().unwrap();
    let mut app = App::new(dir.path());
    for i in 0..25 {
        app.notifications
            .push_back(super::super::modals::Notification::info(format!("msg {i}")));
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
    // The dashboard's right panel follows the same keys (`e:Verify`).
    assert_eq!(app.tui_state.dashboard_sub_tab, 3);

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

    app.tui_state.active_modal = Some(ModalState::BatchReview {
        batch_name: "b".to_string(),
        results: Vec::new(),
        scroll_offset: 4,
    });

    app.dispatch_action(TuiAction::ModalScrollUp);

    assert!(matches!(
        app.tui_state.active_modal,
        Some(ModalState::BatchReview {
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
            action: modals_mod::ConfirmAction::Custom { .. }
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
    let mut app = App::new(dir.path());
    // Clear welcome modal and disable postfx so the tick policy only
    // depends on work-related state, not first-run UI chrome.
    app.tui_state.active_modal = None;
    app.fx_config.screen_postfx = false;
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
    // Clear welcome modal and disable postfx so the tick policy only
    // depends on work-related state.
    app.tui_state.active_modal = None;
    app.fx_config.screen_postfx = false;
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
    let mut app = App::new(dir.path());
    // Clear welcome modal and disable postfx so the tick policy only
    // depends on work-related state.
    app.tui_state.active_modal = None;
    app.fx_config.screen_postfx = false;
    // Idle app with no active work: should be 250ms (Dormant)
    let dur = app.current_tick_duration();
    assert_eq!(
        dur,
        std::time::Duration::from_millis(250),
        "dormant app should use 250ms tick"
    );
}

#[test]
fn full_refresh_keeps_the_plan_set() {
    let dir = tempdir().unwrap();
    // A workspace plan outside the run's plan set: the disk loader lists it.
    let unrelated = dir.path().join("plans").join("03-unrelated");
    std::fs::create_dir_all(&unrelated).unwrap();
    std::fs::write(
        unrelated.join("tasks.toml"),
        "[[task]]\nid = \"T1\"\ntitle = \"Task\"\n",
    )
    .unwrap();
    let mut app = App::new(dir.path());
    let hub = app._state_hub.clone().expect("hub");
    publish_plan_set(&hub, &["01-first", "02-second"]);
    app.drain_snapshot_channel();
    let plan_ids = |app: &App| -> Vec<String> {
        app.tui_state
            .plans
            .iter()
            .map(|plan| plan.id.clone())
            .collect()
    };
    assert_eq!(plan_ids(&app), ["01-first", "02-second"]);

    // An explicit full refresh reloads `DashboardData` from disk, which lists
    // every workspace plan; the hub's plan set must still be what is shown.
    app.refresh_snapshot();
    assert_eq!(plan_ids(&app), ["01-first", "02-second"]);
}

/// gap-633184: a consumer far behind a long, fast stream loses no event
/// without counting it, a replay longer than one tick is not cut short, and
/// every task's terminal status still reaches the TUI, through the snapshot.
#[test]
fn control_events_survive_long_stream_backpressure() {
    use super::channels::take_state_events;
    use crate::runner::tui_bridge::TuiBridge;
    use crate::state_hub::{SharedStateHub, StateHub, StateHubSubscription};
    use crate::tui::state::TaskStatus;

    const TICK: usize = 256;
    let tasks = ["t1", "t2", "t3"];
    // A Graph run's stream: per task, hundreds of tool calls, results and
    // text deltas between its start, gate result and completion.
    let publish_run = |hub: &SharedStateHub| {
        let bridge = TuiBridge::new(hub.sender());
        let start = hub.cursor_snapshot().next_seq;
        bridge.plan_started("p1", tasks.len());
        for task_id in tasks {
            let agent_id = format!("p1/{task_id}");
            bridge.task_started("p1", task_id, task_id, "implement");
            for step in 0..200 {
                let tool_id = format!("{task_id}-{step}");
                bridge.tool_call(&agent_id, "p1", task_id, 1, &tool_id, "Bash");
                bridge.tool_output(&agent_id, "p1", task_id, 1, &tool_id, "ok");
                bridge.agent_text_delta(&agent_id, "p1", task_id, 1, "working");
            }
            bridge.gate_result("p1", task_id, "verify[0]", true);
            bridge.task_completed("p1", task_id, "passed");
        }
        bridge.plan_completed("p1", true);
        hub.cursor_snapshot().next_seq - start
    };
    // Everything a subscription yields, a tick at a time, and the number of
    // events it reported dropped.
    let drain = |subscription: &mut StateHubSubscription| {
        let (mut events, mut dropped) = (Vec::new(), 0);
        loop {
            let (taken, missed) = take_state_events(subscription, TICK);
            dropped += missed;
            if taken.is_empty() && missed == 0 {
                return (events, dropped);
            }
            events.extend(taken);
        }
    };

    // A replay longer than one tick arrives whole, over several ticks.
    let hub = SharedStateHub::new(StateHub::new(4096));
    let published = publish_run(&hub);
    let (events, dropped) = drain(&mut hub.subscribe_events_from(0));
    assert!(published > TICK as u64);
    assert_eq!((events.len() as u64, dropped), (published, 0));

    // A live consumer that falls far behind an 8-event ring loses events,
    // but counts every one of them.
    let hub = SharedStateHub::new(StateHub::new(8));
    let dir = tempdir().expect("tempdir");
    let mut app = App::new_connected(dir.path(), &hub);
    let mut subscription = hub.subscribe_events_from(hub.cursor_snapshot().next_seq);
    let published = publish_run(&hub);
    let (events, dropped) = drain(&mut subscription);
    assert!(dropped > 0, "the stream outran the ring");
    assert_eq!(events.len() as u64 + dropped, published);

    // The TUI reports that count, and shows every task finished: plan, task
    // and gate state come from the snapshot, not from the dropped events.
    app.drain_snapshot_channel();
    app.drain_state_events();
    let marker = format!("[stream lagged: {dropped} StateHub events; snapshot resynced]");
    let system = &app.tui_state.agent_streams["system"];
    assert!(system.chunks.contains(&marker), "{:?}", system.chunks);
    let plan = app
        .tui_state
        .plans
        .iter()
        .find(|plan| plan.id == "p1")
        .expect("p1 is listed");
    for task_id in tasks {
        let task = plan
            .tasks
            .iter()
            .find(|task| task.id == task_id)
            .expect("the task is listed");
        assert_eq!(task.status, TaskStatus::Done, "{task_id}");
    }
}
