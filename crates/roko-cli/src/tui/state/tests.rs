#![cfg(test)]

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;

use chrono::Utc;
use roko_learn::episode_logger::Episode;

use super::snapshot::{
    build_phase_pipeline, derive_plan_waves, plan_task_counts, rebuild_execution_waves,
};
use super::*;
use crate::tui::dashboard::{
    AgentSummary, DashboardData, GateResultSummary, PlanTaskListSnapshot, TaskSummary, Theme,
};
use crate::tui::input::LogFilterLevel;
use roko_learn::efficiency::AgentEfficiencyEvent;
use tempfile::tempdir;

fn efficiency_event(
    role: &str,
    input_tokens: u64,
    output_tokens: u64,
    timestamp: &str,
) -> AgentEfficiencyEvent {
    AgentEfficiencyEvent {
        role: role.to_string(),
        input_tokens,
        output_tokens,
        timestamp: timestamp.to_string(),
        ..AgentEfficiencyEvent::default()
    }
}

#[test]
fn default_state_is_idle_dashboard() {
    let state = TuiState::default();
    assert_eq!(state.active_tab, Tab::Dashboard);
    assert_eq!(state.input_mode, InputMode::Normal);
    assert_eq!(state.focus, FocusZone::PlanTree);
    assert_eq!(state.orchestrator_state, "idle");
    assert!(!state.is_text_input());
}

#[test]
fn reset_scrolls_zeroes_all() {
    let mut state = TuiState::default();
    state.agent_scroll = Some(50);
    state.diff_scroll = 10;
    state.log_scroll = 100;
    state.agent_topology_scroll_offset = 8;

    state.reset_scrolls();

    assert_eq!(state.agent_scroll, None);
    assert_eq!(state.diff_scroll, 0);
    assert_eq!(state.log_scroll, 0);
    assert_eq!(state.agent_topology_scroll_offset, 0);
}

#[test]
fn agent_topology_toggle_and_clamp_work() {
    let mut state = TuiState::default();

    assert!(!state.agent_topology_visible);

    state.toggle_agent_topology();
    assert!(state.agent_topology_visible);

    state.agent_topology_scroll_offset = 42;
    state.clamp_agent_topology_scroll(12);
    assert_eq!(state.agent_topology_scroll_offset, 12);

    state.close_agent_topology();
    assert!(!state.agent_topology_visible);
}

#[test]
fn task_counts_sums_across_plans() {
    let mut state = TuiState::default();
    state.plans = vec![
        PlanEntry {
            tasks_total: 5,
            tasks_done: 3,
            ..PlanEntry::default()
        },
        PlanEntry {
            tasks_total: 10,
            tasks_done: 7,
            ..PlanEntry::default()
        },
    ];
    assert_eq!(state.task_counts(), (10, 15));
}

#[test]
fn log_filter_defaults_to_all_levels() {
    let state = TuiState::default();
    for level in LogFilterLevel::all() {
        assert!(state.log_level_visible(level));
    }
}

#[test]
fn log_filter_toggle_and_reset_work() {
    let mut state = TuiState::default();
    state.toggle_log_filter_level(LogFilterLevel::Warn);
    assert!(!state.log_level_visible(LogFilterLevel::Warn));

    state.show_all_log_filter_levels();
    assert!(state.log_level_visible(LogFilterLevel::Warn));
}

#[test]
fn unified_log_cache_refreshes_from_sources() {
    let mut state = TuiState::default();
    state.recent_signals.push(SignalSummary {
        id: "sig-1".into(),
        kind: "gate:compile".into(),
        created_at_ms: 1_700_000_000_000,
        confidence: None,
        plan_id: None,
        task_id: None,
        parent_hash: None,
        lineage: Vec::new(),
        payload_preview: "passed".into(),
    });

    assert!(state.unified_log_entries().is_empty());
    // Use force_refresh since we manually pushed signals without bumping revision.
    state.force_refresh_cached_unified_log();

    assert_eq!(state.unified_log_entries().len(), 1);
    assert_eq!(state.unified_log_entries()[0].source, "signal:gate:compile");
    assert_eq!(state.unified_log_entries()[0].level, LogEntryLevel::Info);
}

#[test]
fn elapsed_secs_zero_when_not_started() {
    let state = TuiState::default();
    assert_eq!(state.elapsed_secs(), 0.0);
}

#[test]
fn wave_count_and_current_wave() {
    let mut state = TuiState::default();
    assert_eq!(state.wave_count(), 0);
    assert_eq!(state.current_wave(), 0);

    state.execution_waves = vec![
        Wave {
            index: 0,
            total: 2,
            ..Wave::default()
        },
        Wave {
            index: 1,
            total: 1,
            ..Wave::default()
        },
    ];
    state.selected_wave_idx = 1;
    assert_eq!(state.wave_count(), 2);
    assert_eq!(state.current_wave(), 1);
}

#[test]
fn derive_plan_waves_uses_cross_plan_dependencies() {
    let tmpdir = tempdir().expect("tempdir");
    let plans_root = tmpdir.path().join("plans");
    fs::create_dir_all(plans_root.join("plan-a")).expect("create plan-a");
    fs::create_dir_all(plans_root.join("plan-b")).expect("create plan-b");
    fs::create_dir_all(plans_root.join("plan-c")).expect("create plan-c");

    fs::write(
        plans_root.join("plan-a").join("tasks.toml"),
        r#"
[meta]
plan = "Plan A"
total = 1

[[task]]
id = "T1"
title = "start"
depends_on = []
"#,
    )
    .expect("write plan-a");

    fs::write(
        plans_root.join("plan-b").join("tasks.toml"),
        r#"
[meta]
plan = "Plan B"
total = 1

[[task]]
id = "T1"
title = "after a"
depends_on = []
depends_on_plan = ["plan-a"]
"#,
    )
    .expect("write plan-b");

    fs::write(
        plans_root.join("plan-c").join("tasks.toml"),
        r#"
[meta]
plan = "Plan C"
total = 1

[[task]]
id = "T1"
title = "after b"
depends_on = ["plan-b:T1"]
"#,
    )
    .expect("write plan-c");

    let plans = vec![
        PlanSummary {
            id: "plan-a".into(),
            title: "Plan A".into(),
            task_count: 1,
            tasks_done: 0,
            tasks_failed: 0,
            completed: false,
            status: "ready".into(),
            superseded_by: None,
            old_format: false,
            last_error: None,
            group: None,
        },
        PlanSummary {
            id: "plan-b".into(),
            title: "Plan B".into(),
            task_count: 1,
            tasks_done: 0,
            tasks_failed: 0,
            completed: false,
            status: "ready".into(),
            superseded_by: None,
            old_format: false,
            last_error: None,
            group: None,
        },
        PlanSummary {
            id: "plan-c".into(),
            title: "Plan C".into(),
            task_count: 1,
            tasks_done: 0,
            tasks_failed: 0,
            completed: false,
            status: "ready".into(),
            superseded_by: None,
            old_format: false,
            last_error: None,
            group: None,
        },
    ];

    let plan_waves = derive_plan_waves(tmpdir.path(), &plans);
    assert_eq!(plan_waves.get("plan-a"), Some(&0));
    assert_eq!(plan_waves.get("plan-b"), Some(&1));
    assert_eq!(plan_waves.get("plan-c"), Some(&2));
}

#[test]
fn rebuild_execution_waves_preserves_expanded_state_by_index() {
    let plans = vec![
        PlanEntry {
            id: "plan-a".into(),
            wave: Some(1),
            ..PlanEntry::default()
        },
        PlanEntry {
            id: "plan-b".into(),
            wave: Some(2),
            ..PlanEntry::default()
        },
    ];
    let previous = vec![
        Wave {
            index: 1,
            expanded: false,
            ..Wave::default()
        },
        Wave {
            index: 2,
            expanded: true,
            ..Wave::default()
        },
    ];

    let waves = rebuild_execution_waves(&plans, &previous);

    assert_eq!(waves.len(), 2);
    assert_eq!(waves[0].index, 1);
    assert!(!waves[0].expanded);
    assert_eq!(waves[1].index, 2);
    assert!(waves[1].expanded);
}

#[test]
fn active_agent_count_filters_correctly() {
    let mut state = TuiState::default();
    state.agents = vec![
        AgentRow {
            active: true,
            ..AgentRow::default()
        },
        AgentRow {
            active: false,
            ..AgentRow::default()
        },
        AgentRow {
            active: true,
            ..AgentRow::default()
        },
    ];
    assert_eq!(state.active_agent_count(), 2);
}

#[test]
fn tab_badge_returns_agent_count_for_agents_tab() {
    let mut state = TuiState::default();
    // No agents => badge is 0.
    assert_eq!(state.tab_badge(Tab::Agents), 0);
    // Two active agents => badge is 2.
    state.agents = vec![
        AgentRow {
            active: true,
            ..AgentRow::default()
        },
        AgentRow {
            active: false,
            ..AgentRow::default()
        },
        AgentRow {
            active: true,
            ..AgentRow::default()
        },
    ];
    assert_eq!(state.tab_badge(Tab::Agents), 2);
}

#[test]
fn tab_badge_returns_failed_task_count_for_plans_tab() {
    let mut state = TuiState::default();
    assert_eq!(state.tab_badge(Tab::Plans), 0);
    state.plans = vec![
        PlanEntry {
            tasks_failed: 1,
            ..PlanEntry::default()
        },
        PlanEntry {
            tasks_failed: 2,
            ..PlanEntry::default()
        },
    ];
    assert_eq!(state.tab_badge(Tab::Plans), 3);
}

#[test]
fn tab_badge_returns_zero_for_tabs_without_badges() {
    let state = TuiState::default();
    assert_eq!(state.tab_badge(Tab::Dashboard), 0);
    assert_eq!(state.tab_badge(Tab::Config), 0);
    assert_eq!(state.tab_badge(Tab::Inspect), 0);
    assert_eq!(state.tab_badge(Tab::Marketplace), 0);
    assert_eq!(state.tab_badge(Tab::Atelier), 0);
}

#[test]
fn tab_label_with_badge_shows_count_when_nonzero() {
    let mut state = TuiState::default();
    // No active agents: plain label.
    assert_eq!(state.tab_label_with_badge(Tab::Agents), "Agents");
    // One active agent: label with count.
    state.agents = vec![AgentRow {
        active: true,
        ..AgentRow::default()
    }];
    assert_eq!(state.tab_label_with_badge(Tab::Agents), "Agents (1)");
    // Tabs with no badge never append a count.
    assert_eq!(state.tab_label_with_badge(Tab::Dashboard), "Dashboard");
}

#[test]
fn from_dashboard_data_creates_valid_state() {
    let data = DashboardData::default();
    let state = TuiState::from_dashboard_data(&data);
    assert_eq!(state.orchestrator_state, "idle");
    assert!(state.plans.is_empty());
    assert!(state.agents.is_empty());
    assert_eq!(state.token_total, 0);
    assert_eq!(state.cost_dollars, 0.0);
}

#[test]
fn from_dashboard_data_populates_header_network_stats() {
    let mut data = DashboardData::default();
    data.agents = vec![
        AgentSummary {
            id: "active".into(),
            label: "Active".into(),
            plan_id: None,
            status: "active".into(),
        },
        AgentSummary {
            id: "done".into(),
            label: "Done".into(),
            plan_id: None,
            status: "completed".into(),
        },
        AgentSummary {
            id: "idle".into(),
            label: "Idle".into(),
            plan_id: None,
            status: "idle".into(),
        },
    ];
    data.gate_results = vec![
        GateResultSummary {
            plan_id: "p".into(),
            gate_name: "compile".into(),
            passed: true,
            rung: 1,
            duration_ms: 10,
            summary: String::new(),
        },
        GateResultSummary {
            plan_id: "p".into(),
            gate_name: "test".into(),
            passed: false,
            rung: 1,
            duration_ms: 20,
            summary: String::new(),
        },
    ];

    let state = TuiState::from_dashboard_data(&data);

    assert_eq!(state.agents_online, 2);
    assert_eq!(state.gate_pass_rate, Some(0.5));
}

#[test]
fn from_dashboard_data_counts_online_jobs_when_agents_are_missing() {
    let tmpdir = tempdir().expect("tempdir");
    let jobs_dir = tmpdir.path().join(".roko").join("jobs");
    fs::create_dir_all(&jobs_dir).expect("create jobs dir");
    fs::write(jobs_dir.join("open.json"), r#"{"status":"open"}"#).expect("write open job");
    fs::write(jobs_dir.join("assigned.json"), r#"{"state":"assigned"}"#)
        .expect("write assigned job");
    fs::write(jobs_dir.join("done.json"), r#"{"status":"completed"}"#).expect("write done job");

    let data = DashboardData::load_best_effort(tmpdir.path());
    let state = TuiState::from_dashboard_data(&data);

    assert_eq!(state.agents_online, 2);
    assert_eq!(state.gate_pass_rate, None);
}

#[test]
fn phase_pipeline_defaults_to_canonical_phases() {
    let data = DashboardData::default();
    let state = TuiState::from_dashboard_data(&data);
    assert_eq!(state.phase_pipeline.len(), 9);
    assert_eq!(state.phase_pipeline[0].name, "preflight");
    assert_eq!(state.phase_pipeline[8].name, "committing");
}

#[test]
fn phase_pipeline_uses_task_progression_instead_of_position_heuristics() {
    let tasks = vec![
        TaskSummary {
            plan_id: "plan-a".to_string(),
            task_id: "task-impl".to_string(),
            status: "implementing".to_string(),
            iteration: 1,
            assigned_agents: vec!["implementer-1".to_string()],
            latest_gate: None,
        },
        TaskSummary {
            plan_id: "plan-b".to_string(),
            task_id: "task-verify".to_string(),
            status: "verifying".to_string(),
            iteration: 1,
            assigned_agents: vec!["implementer-2".to_string()],
            latest_gate: Some("compile".to_string()),
        },
    ];

    let pipeline = build_phase_pipeline(&tasks);

    assert_eq!(pipeline[0].status, PhaseStatus::Done);
    assert_eq!(pipeline[0].pct, 100.0);
    assert_eq!(pipeline[1].status, PhaseStatus::Done);
    assert_eq!(pipeline[1].pct, 100.0);
    assert_eq!(pipeline[2].status, PhaseStatus::Active);
    assert_eq!(pipeline[2].pct, 50.0);
    assert_eq!(pipeline[3].status, PhaseStatus::Pending);
    assert_eq!(pipeline[3].pct, 50.0);
    assert_eq!(pipeline[4].status, PhaseStatus::Active);
    assert_eq!(pipeline[4].pct, 0.0);
    assert_eq!(pipeline[5].status, PhaseStatus::Pending);
    assert_eq!(pipeline[5].pct, 0.0);
}

#[test]
fn phase_pipeline_marks_failed_gate_from_task_data() {
    let tasks = vec![TaskSummary {
        plan_id: "plan-a".to_string(),
        task_id: "task-test".to_string(),
        status: "failed".to_string(),
        iteration: 2,
        assigned_agents: vec!["implementer-1".to_string()],
        latest_gate: Some("test".to_string()),
    }];

    let pipeline = build_phase_pipeline(&tasks);

    assert_eq!(pipeline[0].status, PhaseStatus::Done);
    assert_eq!(pipeline[1].status, PhaseStatus::Done);
    assert_eq!(pipeline[2].status, PhaseStatus::Done);
    assert_eq!(pipeline[3].status, PhaseStatus::Done);
    assert_eq!(pipeline[4].status, PhaseStatus::Failed);
    assert_eq!(pipeline[4].pct, 0.0);
}

#[test]
fn from_dashboard_data_populates_orchestrator_fields_from_executor_state() {
    let tmpdir = tempdir().expect("tempdir");
    let state_dir = tmpdir.path().join(".roko/state");
    fs::create_dir_all(&state_dir).expect("state dir");

    let executor_state = serde_json::json!({
        "plan_states": {
            "plan-a": {
                "current_phase": { "kind": "gating" },
                "iteration": 3,
                "started_at_ms": 10,
                "paused": false
            },
            "plan-b": {
                "current_phase": { "kind": "implementing" },
                "iteration": 2,
                "started_at_ms": 20,
                "paused": false
            }
        }
    });
    fs::write(
        state_dir.join("executor.json"),
        serde_json::to_vec(&executor_state).expect("executor json"),
    )
    .expect("write executor state");

    let data = DashboardData::load_best_effort(tmpdir.path());
    let state = TuiState::from_dashboard_data(&data);

    assert_eq!(state.orchestrator_state, "running");
    assert_eq!(state.current_iteration, 2);
    assert_eq!(state.current_phase, "implementing");
}

#[test]
fn from_dashboard_data_populates_plan_tasks_from_tracker_and_episodes() {
    let tmpdir = tempdir().expect("tempdir");
    let root = tmpdir.path();
    let state_dir = root.join(".roko/state");
    let plan_dir = root.join(".roko/plans/plan-a");
    let memory_dir = root.join(".roko/memory");

    fs::create_dir_all(&state_dir).expect("state dir");
    fs::create_dir_all(&plan_dir).expect("plan dir");
    fs::create_dir_all(&memory_dir).expect("memory dir");

    let executor_state = serde_json::json!({
        "plan_states": {
            "plan-a": {
                "current_phase": { "kind": "implementing" },
                "task_id": "task-2",
                "assigned_agents": ["agent-a"]
            }
        }
    });
    fs::write(
        state_dir.join("executor.json"),
        serde_json::to_vec(&executor_state).expect("executor json"),
    )
    .expect("write executor state");

    let tracker_state = serde_json::json!([
        {
            "plan_id": "plan-a",
            "completed": ["task-1"],
            "failed": ["task-3"],
            "current_group_index": 1
        }
    ]);
    fs::write(
        state_dir.join("task-trackers.json"),
        serde_json::to_vec(&tracker_state).expect("tracker json"),
    )
    .expect("write tracker state");

    fs::write(
        plan_dir.join("tasks.toml"),
        r#"
[meta]
plan = "Plan A"
iteration = 1
total = 3
done = 1
status = "running"

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

    let mut task_one = Episode::new("agent-a", "task-1");
    task_one.input_signal_hash = "plan-a".to_string();
    task_one
        .extra
        .insert("plan_id".to_string(), serde_json::json!("plan-a"));
    task_one
        .extra
        .insert("task_id".to_string(), serde_json::json!("task-1"));
    task_one.usage.wall_ms = 1_500;

    let mut task_two = Episode::new("agent-a", "task-2");
    task_two.input_signal_hash = "plan-a".to_string();
    task_two
        .extra
        .insert("plan_id".to_string(), serde_json::json!("plan-a"));
    task_two
        .extra
        .insert("task_id".to_string(), serde_json::json!("task-2"));
    task_two.usage.wall_ms = 2_500;

    let episodes = format!(
        "{}\n{}\n",
        serde_json::to_string(&task_one).expect("task one episode"),
        serde_json::to_string(&task_two).expect("task two episode")
    );
    fs::write(memory_dir.join("episodes.jsonl"), episodes).expect("write episodes");

    let data = DashboardData::load_best_effort(root);
    let state = TuiState::from_dashboard_data(&data);
    let plan = state
        .plans
        .iter()
        .find(|plan| plan.id == "plan-a")
        .expect("plan-a");

    assert_eq!(plan.status, PlanPhase::Active);
    assert_eq!(plan.phase, "implementing");
    assert!(plan.active);
    assert_eq!(plan.tasks_total, 3);
    assert_eq!(plan.tasks_done, 1);
    assert_eq!(plan.tasks_failed, 1);
    assert!((plan.elapsed_secs - 7.0).abs() < f64::EPSILON);
    assert_eq!(plan.wave, Some(2));
    assert_eq!(plan.tasks.len(), 3);
    assert_eq!(plan.tasks[0].id, "task-1");
    assert_eq!(plan.tasks[0].status, TaskStatus::Done);
    assert_eq!(plan.tasks[1].id, "task-2");
    assert_eq!(plan.tasks[1].status, TaskStatus::Active);
    assert_eq!(plan.tasks[1].agent_id.as_deref(), Some("agent-a"));
    assert_eq!(plan.tasks[2].id, "task-3");
    assert_eq!(plan.tasks[2].status, TaskStatus::Failed);
}

#[test]
fn plan_task_counts_uses_summary_progress_without_snapshot() {
    let summary = crate::plan::PlanSummary {
        id: "plan-a".into(),
        title: "Plan A".into(),
        task_count: 5,
        tasks_done: 2,
        tasks_failed: 1,
        completed: false,
        status: "ready".into(),
        superseded_by: None,
        old_format: false,
        last_error: None,
        group: None,
    };

    assert_eq!(plan_task_counts(&summary, None, 5), (2, 1));
}

#[test]
fn plan_task_counts_prefers_snapshot_task_statuses() {
    let summary = crate::plan::PlanSummary {
        id: "plan-a".into(),
        title: "Plan A".into(),
        task_count: 3,
        tasks_done: 0,
        tasks_failed: 0,
        completed: false,
        status: "ready".into(),
        superseded_by: None,
        old_format: false,
        last_error: None,
        group: None,
    };
    let snapshot = PlanTaskListSnapshot {
        tasks_done: 0,
        tasks_failed: 0,
        tasks: vec![
            crate::tui::dashboard::PlanTaskSnapshot {
                id: "task-1".into(),
                title: "Done".into(),
                status: "done".into(),
                agent_id: None,
                ..crate::tui::dashboard::PlanTaskSnapshot::default()
            },
            crate::tui::dashboard::PlanTaskSnapshot {
                id: "task-2".into(),
                title: "Active".into(),
                status: "implementing".into(),
                agent_id: None,
                ..crate::tui::dashboard::PlanTaskSnapshot::default()
            },
            crate::tui::dashboard::PlanTaskSnapshot {
                id: "task-3".into(),
                title: "Failed".into(),
                status: "failed".into(),
                agent_id: None,
                ..crate::tui::dashboard::PlanTaskSnapshot::default()
            },
        ],
        ..PlanTaskListSnapshot::default()
    };

    assert_eq!(plan_task_counts(&summary, Some(&snapshot), 3), (1, 1));
}

#[test]
fn new_fields_have_defaults() {
    let state = TuiState::default();
    assert!(state.phase_pipeline.is_empty());
    assert!(state.execution_waves.is_empty());
    assert!(state.current_task_checklist.is_empty());
    assert_eq!(state.sys.cpu_pct, 0.0);
    assert_eq!(state.token_total, 0);
    assert!(state.token_history.is_empty());
    assert_eq!(state.token_rate, 0.0);
    assert_eq!(state.cost_rate, 0.0);
    assert_eq!(state.cost_dollars, 0.0);
    assert!(state.git_commit_short.is_empty());
    assert!(state.git_age.is_empty());
    assert!(state.run_started.is_none());
    assert!(state.filter.is_empty());
    assert!(state.log_auto_tail);
    assert_eq!(state.selected_agent, 0);
    assert_eq!(state.agent_scroll, None);
    assert_eq!(state.plan_scroll_offset, 0);
}

#[test]
fn plan_budget_summary_projects_from_completed_task_average() {
    let mut state = TuiState::default();
    state.max_plan_budget_usd = 5.0;
    state.cost_per_plan.insert("plan".into(), 1.5);
    state.cost_per_task.insert("plan:T1".into(), 0.5);
    state.cost_per_task.insert("plan:T2".into(), 1.0);
    let plan = PlanEntry {
        id: "plan".into(),
        tasks_total: 4,
        tasks_done: 2,
        ..PlanEntry::default()
    };

    let summary = state.plan_budget_summary(&plan);
    assert_eq!(summary.spent_usd, 1.5);
    assert_eq!(summary.budget_usd, 5.0);
    assert_eq!(summary.projected_remaining_usd, 1.5);
    assert_eq!(summary.projected_total_usd, 3.0);
}

#[test]
fn update_from_dashboard_snapshot_maps_connected_state_and_preserves_navigation() {
    use roko_core::dashboard_snapshot::{
        AgentState as SnapshotAgentState, DashboardSnapshot, ErrorEntry, GateVerdictView, PlanState,
    };

    let mut state = TuiState::default();
    state.active_tab = Tab::Git;
    state.selected_plan_idx = 1;
    state.current_plan_idx = 0;
    state.selected_agent = 1;
    state.selected_agent_tab = 4;
    state.focus = FocusZone::AgentOutput;
    state.agent_scroll = Some(12);
    state.diff_scroll = 7;
    state.task_scroll = 9;
    state.command_output_scroll = 11;
    state.plan_detail_scroll = 13;
    state.plan_scroll_offset = 15;
    state.log_scroll = 17;
    state.log_auto_tail = false;

    state.plans = vec![
        PlanEntry {
            id: "plan-b".into(),
            expanded: false,
            ..PlanEntry::default()
        },
        PlanEntry {
            id: "plan-a".into(),
            expanded: true,
            ..PlanEntry::default()
        },
    ];
    state.agents = vec![
        AgentRow {
            id: "agent-b".into(),
            active: false,
            ..AgentRow::default()
        },
        AgentRow {
            id: "agent-a".into(),
            active: true,
            ..AgentRow::default()
        },
    ];

    let snap = DashboardSnapshot {
        run_duration_ms: None,
        run_outcome: None,
        run_cleanup_degraded: false,
        critical_path_eta_minutes: None,
        surviving_agent_pids: Vec::new(),
        plan_set: None,
        plans: [
            (
                "plan-a".to_string(),
                PlanState {
                    plan_id: "plan-a".into(),
                    phase: "started".into(),
                    tasks_total: 4,
                    tasks_done: 1,
                    tasks_failed: 0,
                    active: true,
                    ..Default::default()
                },
            ),
            (
                "plan-b".to_string(),
                PlanState {
                    plan_id: "plan-b".into(),
                    phase: "failed".into(),
                    tasks_total: 2,
                    tasks_done: 1,
                    tasks_failed: 1,
                    active: false,
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .collect(),
        tasks: Default::default(),
        agents: [
            (
                "agent-a".to_string(),
                SnapshotAgentState {
                    agent_id: "agent-a".into(),
                    role: "implementer".into(),
                    active: true,
                    output_bytes: 128,
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
                "agent-b".to_string(),
                SnapshotAgentState {
                    agent_id: "agent-b".into(),
                    role: "reviewer".into(),
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
        gates: vec![
            GateVerdictView {
                plan_id: "plan-a".into(),
                task_id: "task-1".into(),
                gate: "compile".into(),
                passed: true,
                ts_millis: 1_000,
            },
            GateVerdictView {
                plan_id: "plan-b".into(),
                task_id: "task-2".into(),
                gate: "test".into(),
                passed: false,
                ts_millis: 2_000,
            },
        ],
        diagnoses: Default::default(),
        experiment_winners: Vec::new(),
        agent_topology: roko_core::AgentTopology::default(),
        efficiency_trend: Vec::new(),
        cfactor_trend: Vec::new(),
        gate_trends: HashMap::new(),
        gate_recent_failures: Vec::new(),
        episodes: Default::default(),
        errors: vec![
            ErrorEntry {
                message: "compile failed".into(),
                ts_millis: 3_000,
            },
            ErrorEntry {
                message: "timeout".into(),
                ts_millis: 4_000,
            },
        ],
        event_log: Default::default(),
        task_outputs: Default::default(),
        cascade_router_json: String::new(),
        gate_thresholds_json: String::new(),
        marketplace_jobs: Vec::new(),
        atelier_prds: Vec::new(),
        atelier_tasks: HashMap::new(),
        knowledge_entries: Vec::new(),
        payment_count: 0,
        total_payment_korai: 0.0,
        payments_by_protocol: HashMap::new(),
        settlement_count: 0,
        inbox_items: HashMap::new(),
        inbox_resolved_ids: HashSet::new(),
        inbox_pending_count: 0,
        affect: None,
        gate_output_lines: Default::default(),
        task_gate_outputs: Default::default(),
        active_gate_rung: None,
        token_event_ring: Default::default(),
        stats: Default::default(),
    };

    state.update_from_dashboard_snapshot(&snap);

    assert_eq!(state.plans.len(), 2);
    let plan_a = state.plans.iter().find(|plan| plan.id == "plan-a").unwrap();
    let plan_b = state.plans.iter().find(|plan| plan.id == "plan-b").unwrap();
    assert_eq!(plan_a.status, PlanPhase::Active);
    assert!(plan_a.active);
    assert!(plan_a.expanded);
    assert_eq!(plan_b.status, PlanPhase::Failed);
    assert_eq!(plan_b.tasks_failed, 1);

    assert_eq!(state.agents.len(), 2);
    let agent_a = state
        .agents
        .iter()
        .find(|agent| agent.id == "agent-a")
        .unwrap();
    let agent_b = state
        .agents
        .iter()
        .find(|agent| agent.id == "agent-b")
        .unwrap();
    assert!(agent_a.active);
    assert_eq!(agent_a.role, "implementer");
    assert!(!agent_b.active);

    assert_eq!(state.gate_results.len(), 2);
    assert_eq!(state.gate_results[0].gate, "compile");
    assert_eq!(state.gate_results[1].plan_id, "plan-b");
    assert!(!state.gate_results[1].passed);

    assert_eq!(state.active_tab, Tab::Git);
    assert_eq!(state.plans[state.selected_plan_idx].id, "plan-a");
    assert_eq!(state.plans[state.current_plan_idx].id, "plan-b");
    assert_eq!(state.agents[state.selected_agent].id, "agent-a");
    assert_eq!(state.selected_agent_tab, 4);
    assert_eq!(state.focus, FocusZone::AgentOutput);
    assert_eq!(state.agent_scroll, Some(12));
    assert_eq!(state.diff_scroll, 7);
    assert_eq!(state.task_scroll, 9);
    assert_eq!(state.command_output_scroll, 11);
    assert_eq!(state.plan_detail_scroll, 13);
    assert_eq!(state.plan_scroll_offset, 15);
    assert_eq!(state.log_scroll, 17);
    assert!(!state.log_auto_tail);
}

#[test]
fn update_from_dashboard_snapshot_keeps_expanded_state_when_matching_plan_remains() {
    use roko_core::dashboard_snapshot::{DashboardSnapshot, PlanState};

    let mut state = TuiState::default();
    state.plans = vec![
        PlanEntry {
            id: "plan-a".into(),
            expanded: false,
            ..PlanEntry::default()
        },
        PlanEntry {
            id: "plan-b".into(),
            expanded: true,
            ..PlanEntry::default()
        },
    ];

    let snap = DashboardSnapshot {
        plans: [(
            "plan-b".to_string(),
            PlanState {
                plan_id: "plan-b".into(),
                phase: "completed".into(),
                tasks_total: 1,
                tasks_done: 1,
                tasks_failed: 0,
                active: false,
                ..Default::default()
            },
        )]
        .into_iter()
        .collect(),
        ..Default::default()
    };

    state.update_from_dashboard_snapshot(&snap);

    assert_eq!(state.plans.len(), 1);
    assert_eq!(state.plans[0].id, "plan-b");
    assert!(state.plans[0].expanded);
}

#[test]
fn update_from_dashboard_snapshot_populates_connected_learning_state() {
    use roko_core::dashboard_snapshot::{
        DashboardSnapshot, EfficiencyBucket as CoreEfficiencyBucket,
    };

    let tmpdir = tempdir().expect("tempdir");
    let learn_dir = tmpdir.path().join(".roko").join("learn");
    std::fs::create_dir_all(&learn_dir).expect("create learn dir");

    // Per-event payloads the snapshot cannot carry live in the local
    // append-only learning files.
    let now = Utc::now();
    let events = [
        AgentEfficiencyEvent {
            agent_id: "agent-a".into(),
            model: "gpt-5.6-sol".into(),
            backend: "codex-cli".into(),
            input_tokens: 120,
            output_tokens: 45,
            cost_usd: 0.25,
            wall_time_ms: 800,
            gate_passed: Some(true),
            is_final_turn: true,
            timestamp: now.to_rfc3339(),
            ..AgentEfficiencyEvent::default()
        },
        AgentEfficiencyEvent {
            agent_id: "agent-b".into(),
            model: "glm-5.1".into(),
            backend: "zai".into(),
            input_tokens: 60,
            output_tokens: 15,
            cost_usd: 0.05,
            wall_time_ms: 400,
            gate_passed: Some(false),
            is_final_turn: true,
            timestamp: now.to_rfc3339(),
            ..AgentEfficiencyEvent::default()
        },
    ];
    let mut log = String::new();
    for event in &events {
        log.push_str(&serde_json::to_string(event).expect("serialize event"));
        log.push('\n');
    }
    std::fs::write(learn_dir.join("efficiency.jsonl"), log).expect("write efficiency log");

    let mut store = roko_learn::prompt_experiment::ExperimentStore::new();
    store.register(roko_learn::prompt_experiment::PromptExperiment::new(
        "exp-1",
        "constraints",
        vec![roko_learn::prompt_experiment::PromptVariant {
            id: "v1".into(),
            name: "baseline".into(),
            section_name: "constraints".into(),
            content: "be terse".into(),
            slug: None,
            active: true,
        }],
    ));
    store
        .save(&learn_dir.join("experiments.json"))
        .expect("save experiments");

    let snap = DashboardSnapshot {
        cascade_router_json: serde_json::json!({
            "model_slugs": ["gpt-5.6-sol"],
            "confidence_stats": {"gpt-5.6-sol": {"trials": 3, "successes": 2}},
        })
        .to_string(),
        gate_thresholds_json: serde_json::json!({
            "rungs": {"1": {"pass_count": 4, "total_count": 5, "ema_pass_rate": 0.8}},
        })
        .to_string(),
        efficiency_trend: vec![CoreEfficiencyBucket {
            start: now,
            turns: 2,
            tokens_in: 180,
            tokens_out: 60,
            cost_usd_cents: 30,
            latency_ms_avg: 600.0,
        }],
        ..Default::default()
    };

    let mut state = TuiState::default();
    state.workdir = tmpdir.path().to_path_buf();
    state.update_from_dashboard_snapshot(&snap);

    // Pushed JSON parsed into the typed structs.
    assert_eq!(
        state.cascade_router.model_slugs,
        vec!["gpt-5.6-sol".to_string()]
    );
    assert_eq!(
        state.cascade_router.confidence_stats["gpt-5.6-sol"].successes,
        2
    );
    assert_eq!(state.gate_results_page.threshold_rows.len(), 1);
    assert_eq!(state.gate_results_page.threshold_rows[0].rung, 1);
    assert!((state.gate_results_page.threshold_rows[0].ema_pass_rate - 0.8).abs() < 1e-9);

    // Pushed trend converted into the chart-facing bucket type.
    assert_eq!(state.efficiency_trend.len(), 1);
    assert_eq!(state.efficiency_trend[0].turns, 2);

    // Per-event payloads tailed from the local learning files.
    assert_eq!(state.efficiency_events.len(), 2);
    assert_eq!(state.efficiency_events[0].model, "gpt-5.6-sol");
    // The event-derived summary wins over the pushed-bucket approximation
    // (pushed buckets hardcode passed_count = 0; events know the truth).
    assert_eq!(state.efficiency_summary.event_count, 2);
    assert!((state.efficiency_summary.total_cost_usd - 0.30).abs() < 1e-9);
    assert_eq!(state.efficiency_summary.passed_count, 1);

    // Experiment store loaded from disk; no concluded experiments yet, so
    // no winners — and the empty push did not invent any either.
    assert_eq!(state.experiments.len(), 1);
    assert_eq!(state.experiments[0].experiment_id, "exp-1");
    assert!(state.experiment_winners.is_empty());
}

#[test]
fn update_from_dashboard_snapshot_preserves_experiment_winners_when_push_is_empty() {
    use roko_core::dashboard_snapshot::DashboardSnapshot;

    let tmpdir = tempdir().expect("tempdir");
    let mut state = TuiState::default();
    state.workdir = tmpdir.path().to_path_buf();
    state.experiment_winners = vec![roko_core::ExperimentWinnerSummary {
        experiment_id: "exp-w".into(),
        ..Default::default()
    }];

    // Nothing publishes ExperimentWinnersUpdated today; an empty push must
    // not clobber winners already in state.
    state.update_from_dashboard_snapshot(&DashboardSnapshot::default());

    assert_eq!(state.experiment_winners.len(), 1);
    assert_eq!(state.experiment_winners[0].experiment_id, "exp-w");
}

#[test]
fn update_from_snapshot_populates_token_rate() {
    let mut data = DashboardData::default();
    data.efficiency_events = vec![
        efficiency_event("impl", 100, 50, "2026-04-14T12:00:00Z"),
        efficiency_event("review", 20, 10, "2026-04-14T12:05:00Z"),
        efficiency_event("impl", 40, 10, "2026-04-14T12:10:00Z"),
    ];

    let mut state = TuiState::default();
    state.update_from_snapshot(&data);

    assert!((state.token_rate - 8.0).abs() < f64::EPSILON);
}

#[test]
fn update_from_snapshot_populates_token_history_for_selected_agent() {
    let mut data = DashboardData::default();
    data.agents = vec![
        crate::tui::dashboard::AgentSummary {
            id: "agent-a".into(),
            label: "agent-a".into(),
            plan_id: None,
            status: "active".into(),
        },
        crate::tui::dashboard::AgentSummary {
            id: "agent-b".into(),
            label: "agent-b".into(),
            plan_id: None,
            status: "active".into(),
        },
    ];
    data.efficiency_events = vec![
        AgentEfficiencyEvent {
            agent_id: "agent-a".into(),
            role: "implementer".into(),
            input_tokens: 100,
            output_tokens: 20,
            timestamp: "2026-04-14T12:00:00Z".into(),
            ..AgentEfficiencyEvent::default()
        },
        AgentEfficiencyEvent {
            agent_id: "agent-b".into(),
            role: "reviewer".into(),
            input_tokens: 30,
            output_tokens: 10,
            timestamp: "2026-04-14T12:01:00Z".into(),
            ..AgentEfficiencyEvent::default()
        },
        AgentEfficiencyEvent {
            agent_id: "agent-a".into(),
            role: "implementer".into(),
            input_tokens: 50,
            output_tokens: 10,
            timestamp: "2026-04-14T12:02:00Z".into(),
            ..AgentEfficiencyEvent::default()
        },
        AgentEfficiencyEvent {
            agent_id: "agent-b".into(),
            role: "reviewer".into(),
            input_tokens: 50,
            output_tokens: 10,
            timestamp: "2026-04-14T12:04:00Z".into(),
            ..AgentEfficiencyEvent::default()
        },
    ];

    let mut state = TuiState::default();
    state.selected_agent = 1;
    state.update_from_snapshot(&data);

    assert_eq!(
        state.token_history.get("agent-a").cloned(),
        Some(VecDeque::from(vec![120, 180]))
    );
    assert_eq!(
        state.token_history.get("agent-b").cloned(),
        Some(VecDeque::from(vec![40, 100]))
    );
    assert!((state.token_rate - 20.0).abs() < f64::EPSILON);
}

#[test]
fn update_from_snapshot_populates_route_metrics() {
    let mut data = DashboardData::default();
    data.efficiency_events = vec![AgentEfficiencyEvent {
        agent_id: "agent-a".to_string(),
        role: "implementer".to_string(),
        model: "claude-haiku-4-5".to_string(),
        input_tokens: 12_000,
        output_tokens: 3_000,
        prompt_sections: vec![roko_learn::efficiency::PromptSectionMeta {
            name: "workspace_map".to_string(),
            tokens: 800,
            priority: 0,
            was_truncated: false,
            was_dropped: false,
        }],
        frequency: OperatingFrequency::Gamma,
        timestamp: "2026-04-14T12:00:00Z".to_string(),
        ..AgentEfficiencyEvent::default()
    }];
    data.cascade_router.confidence_stats.insert(
        "claude-haiku-4-5".to_string(),
        crate::tui::dashboard::CascadeRouterModelStats {
            trials: 10,
            successes: 8,
        },
    );

    let mut state = TuiState::default();
    state.update_from_snapshot(&data);

    let metrics = state.route_metrics.get("agent-a").expect("route metrics");
    assert_eq!(metrics.model, "claude-haiku-4-5");
    assert_eq!(metrics.tier, "fast");
    assert_eq!(metrics.context_used, 15_000);
    assert_eq!(metrics.context_limit, 200_000);
    assert!((metrics.focus_score - 0.8).abs() < f64::EPSILON);
}

#[test]
fn update_from_dashboard_snapshot_maps_streaming_fields() {
    let mut snap = roko_core::DashboardSnapshot::default();
    snap.plans.insert(
        "plan-a".into(),
        roko_core::dashboard_snapshot::PlanState {
            plan_id: "plan-a".into(),
            phase: "implementer".into(),
            tasks_total: 2,
            tasks_done: 1,
            tasks_failed: 0,
            active: true,
            ..Default::default()
        },
    );
    snap.tasks.insert(
        "plan-a/task-1".into(),
        roko_core::dashboard_snapshot::TaskState {
            task_id: "task-1".into(),
            title: String::new(),
            plan_id: "plan-a".into(),
            phase: "implementer".into(),
            outcome: None,
            blocked_by: None,
            blocked_reason: None,
        },
    );
    snap.tasks.insert(
        "plan-a/task-2".into(),
        roko_core::dashboard_snapshot::TaskState {
            task_id: "task-2".into(),
            title: String::new(),
            plan_id: "plan-a".into(),
            phase: "completed".into(),
            outcome: Some("success".into()),
            blocked_by: None,
            blocked_reason: None,
        },
    );
    snap.agents.insert(
        "agent-1".into(),
        roko_core::dashboard_snapshot::AgentState {
            agent_id: "agent-1".into(),
            role: "implementer".into(),
            active: true,
            output_bytes: 42,
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
    );
    snap.gates
        .push(roko_core::dashboard_snapshot::GateVerdictView {
            plan_id: "plan-a".into(),
            task_id: "task-2".into(),
            gate: "compile".into(),
            passed: true,
            ts_millis: 1,
        });
    snap.diagnoses
        .push_back(roko_core::dashboard_snapshot::DiagnosisSummary {
            id: "plan:plan-a:watcher:circuit-breaker:pattern:loop-detected".into(),
            ts: chrono::Utc::now(),
            severity: roko_core::dashboard_snapshot::DiagnosisSeverity::Warn,
            subject: "Circuit Breaker: Loop Detected".into(),
            detail: "repeated identical output".into(),
            suggested_action: Some("Restart Agent".into()),
            intervention_taken: Some("Paused plan".into()),
        });
    snap.experiment_winners
        .push(roko_core::ExperimentWinnerSummary {
            experiment_id: "exp-01".into(),
            parameter: "constraints".into(),
            winner: "claude-opus-4-6".into(),
            winner_variant_id: "opus".into(),
            win_rate: 0.71,
            sample_size: 142,
            ci_lower: 0.63,
            ci_upper: 0.78,
            confidence: 0.97,
        });
    snap.gate_trends.insert(
        "compile".into(),
        roko_core::TrendBuckets::new(3_600, 24, chrono::Utc::now()),
    );
    snap.gate_recent_failures.push(roko_core::FailureEntry {
        ts: chrono::Utc::now(),
        plan_id: "plan-a".into(),
        task_id: "task-2".into(),
        gate: "compile".into(),
        summary: "compile failed".into(),
        artifacts: None,
    });
    snap.errors.push(roko_core::dashboard_snapshot::ErrorEntry {
        message: "compile failed once".into(),
        ts_millis: 2,
    });
    snap.stats.plans_active = 1;
    snap.stats.tasks_active = 1;
    snap.stats.gates_passed = 1;
    snap.stats.errors_total = 1;

    let mut state = TuiState::default();
    state.update_from_dashboard_snapshot(&snap);

    assert_eq!(state.orchestrator_state, "running");
    assert_eq!(state.current_phase, "implementer");
    assert_eq!(state.plans.len(), 1);
    assert_eq!(state.plans[0].id, "plan-a");
    assert_eq!(state.plans[0].tasks_total, 2);
    assert_eq!(state.plans[0].tasks_done, 1);
    assert_eq!(state.plans[0].phase, "implementer");
    assert_eq!(state.plans[0].tasks.len(), 2);
    assert_eq!(state.current_task_checklist.len(), 2);
    assert_eq!(state.current_task_checklist[0].id, "task-1");
    assert_eq!(state.current_task_checklist[0].status, TaskStatus::Active);
    assert_eq!(state.current_task_checklist[1].status, TaskStatus::Done);
    assert_eq!(state.agents.len(), 1);
    assert_eq!(state.agents[0].id, "agent-1");
    assert_eq!(state.agents[0].role, "implementer");
    assert_eq!(state.agents[0].output_tokens, 42);
    assert_eq!(state.gate_results.len(), 1);
    assert_eq!(state.gate_results[0].gate, "compile");
    assert_eq!(state.gate_results[0].output, "task task-2");
    assert_eq!(state.diagnoses.len(), 1);
    assert_eq!(state.diagnoses[0].subject, "Circuit Breaker: Loop Detected");
    assert_eq!(state.experiment_winners.len(), 1);
    assert_eq!(state.experiment_winners[0].experiment_id, "exp-01");
    assert!(state.gate_trends.contains_key("compile"));
    assert_eq!(state.gate_recent_failures.len(), 1);
    assert_eq!(state.gate_recent_failures[0].task_id, "task-2");
    assert_eq!(state.execution_waves.len(), 1);
    assert_eq!(state.execution_waves[0].plans, vec![String::from("plan-a")]);
    assert_eq!(state.phase_pipeline.len(), 9);
    assert_eq!(state.phase_pipeline[2].status, PhaseStatus::Active);
}

#[test]
fn update_from_dashboard_snapshot_preserves_navigation_state_by_id() {
    let mut state = TuiState::default();
    state.active_tab = Tab::Agents;
    state.focus = FocusZone::RightPanel;
    state.selected_plan_idx = 1;
    state.current_plan_idx = 1;
    state.selected_agent = 1;
    state.selected_agent_tab = 4;
    state.agent_scroll = Some(9);
    state.plan_scroll_offset = 12;
    state.log_scroll = 7;
    state.plans = vec![
        PlanEntry {
            id: "plan-a".into(),
            expanded: false,
            ..PlanEntry::default()
        },
        PlanEntry {
            id: "plan-b".into(),
            expanded: true,
            ..PlanEntry::default()
        },
    ];
    state.agents = vec![
        AgentRow {
            id: "agent-1".into(),
            ..AgentRow::default()
        },
        AgentRow {
            id: "agent-2".into(),
            ..AgentRow::default()
        },
    ];
    state.current_task_checklist = vec![TaskRow {
        id: "task-2".into(),
        title: "task-2".into(),
        status: TaskStatus::Active,
        elapsed_secs: 15.0,
        depends_on: Vec::new(),
        acceptance_text: None,
        verify_command: None,
        files: Vec::new(),
    }];

    let mut snap = roko_core::DashboardSnapshot::default();
    snap.plans.insert(
        "plan-b".into(),
        roko_core::dashboard_snapshot::PlanState {
            plan_id: "plan-b".into(),
            phase: "implementer".into(),
            tasks_total: 1,
            tasks_done: 0,
            tasks_failed: 0,
            active: true,
            ..Default::default()
        },
    );
    snap.plans.insert(
        "plan-a".into(),
        roko_core::dashboard_snapshot::PlanState {
            plan_id: "plan-a".into(),
            phase: "pending".into(),
            tasks_total: 0,
            tasks_done: 0,
            tasks_failed: 0,
            active: false,
            ..Default::default()
        },
    );
    snap.tasks.insert(
        "plan-b/task-2".into(),
        roko_core::dashboard_snapshot::TaskState {
            task_id: "task-2".into(),
            title: String::new(),
            plan_id: "plan-b".into(),
            phase: "implementer".into(),
            outcome: None,
            blocked_by: None,
            blocked_reason: None,
        },
    );
    snap.agents.insert(
        "agent-2".into(),
        roko_core::dashboard_snapshot::AgentState {
            agent_id: "agent-2".into(),
            role: "reviewer".into(),
            active: true,
            output_bytes: 3,
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
    );
    snap.agents.insert(
        "agent-1".into(),
        roko_core::dashboard_snapshot::AgentState {
            agent_id: "agent-1".into(),
            role: "implementer".into(),
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
    );
    snap.stats.plans_active = 1;

    state.update_from_dashboard_snapshot(&snap);

    assert_eq!(state.active_tab, Tab::Agents);
    assert_eq!(state.focus, FocusZone::RightPanel);
    assert_eq!(state.agent_scroll, Some(9));
    assert_eq!(state.plan_scroll_offset, 12);
    assert_eq!(state.log_scroll, 7);
    assert_eq!(state.selected_plan_idx, 1);
    assert_eq!(state.current_plan_idx, 1);
    assert_eq!(state.plans[state.selected_plan_idx].id, "plan-b");
    assert_eq!(state.selected_agent, 1);
    assert_eq!(state.agents[state.selected_agent].id, "agent-2");
    assert_eq!(state.selected_agent_tab, 4);
    assert!(state.plans[1].expanded);
    assert_eq!(state.current_task_checklist[0].elapsed_secs, 15.0);
}

#[test]
fn connected_snapshots_replace_output_rings_without_duplication() {
    let mut state = TuiState::default();
    state.task_output_tails.insert(
        "stale-task".into(),
        vec!["this task is absent from the new snapshot".into()],
    );

    let mut snap = roko_core::DashboardSnapshot::default();
    snap.agents.insert(
        "agent-1".into(),
        roko_core::dashboard_snapshot::AgentState {
            agent_id: "agent-1".into(),
            role: "implementer".into(),
            active: true,
            output_bytes: 12,
            model: "test-model".into(),
            provider: String::new(),
            input_tokens: 0,
            output_tokens: 0,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            cost_usd: 0.0,
            current_task: "task-1".into(),
            current_plan: "plan-1".into(),
            attempt: 0,
            spawned_at_ms: 0,
            last_event_at_ms: 0,
            elapsed_ms: 0,
        },
    );
    snap.task_outputs.insert(
        "task-1".into(),
        VecDeque::from(vec!["first".into(), "second".into()]),
    );

    state.update_from_dashboard_snapshot(&snap);
    state.update_from_dashboard_snapshot(&snap);

    assert_eq!(state.agents[0].output_lines, ["first", "second"]);
    assert_eq!(state.task_output_tails["task-1"], ["first", "second"]);
    assert!(!state.task_output_tails.contains_key("stale-task"));
}

#[test]
fn connected_snapshot_output_is_bounded_to_source_ring_limit() {
    let mut snap = roko_core::DashboardSnapshot::default();
    snap.agents.insert(
        "agent-1".into(),
        roko_core::dashboard_snapshot::AgentState {
            agent_id: "agent-1".into(),
            role: "implementer".into(),
            active: true,
            output_bytes: 100,
            model: "test-model".into(),
            provider: String::new(),
            input_tokens: 0,
            output_tokens: 0,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            cost_usd: 0.0,
            current_task: "task-1".into(),
            current_plan: "plan-1".into(),
            attempt: 0,
            spawned_at_ms: 0,
            last_event_at_ms: 0,
            elapsed_ms: 0,
        },
    );
    snap.task_outputs.insert(
        "task-1".into(),
        (0..100).map(|line| format!("line-{line}")).collect(),
    );

    let mut state = TuiState::default();
    state.update_from_dashboard_snapshot(&snap);

    assert_eq!(state.agents[0].output_lines.len(), MAX_AGENT_OUTPUT_LINES);
    assert_eq!(state.agents[0].output_lines[0], "line-50");
    assert_eq!(state.agents[0].output_lines[49], "line-99");
}

#[test]
fn connected_snapshot_builds_cumulative_token_history() {
    let mut snap = roko_core::DashboardSnapshot::default();
    snap.agents.insert(
        "agent-1".into(),
        roko_core::dashboard_snapshot::AgentState {
            agent_id: "agent-1".into(),
            role: "implementer".into(),
            active: true,
            output_bytes: 0,
            model: "test-model".into(),
            provider: String::new(),
            input_tokens: 100,
            output_tokens: 50,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            cost_usd: 0.0,
            current_task: "task-1".into(),
            current_plan: "plan-1".into(),
            attempt: 1,
            spawned_at_ms: 0,
            last_event_at_ms: 0,
            elapsed_ms: 0,
        },
    );
    snap.token_event_ring = VecDeque::from([25, 50]);

    let mut state = TuiState::default();
    state.update_from_dashboard_snapshot(&snap);

    assert_eq!(state.token_total, 150);
    assert_eq!(
        state.token_history.get("connected"),
        Some(&VecDeque::from([100, 150]))
    );
}

#[test]
fn connected_snapshot_empty_output_ring_clears_previous_task_output() {
    let mut state = TuiState::default();
    state.agents.push(AgentRow {
        id: "agent-1".into(),
        active: true,
        current_task: "task-1".into(),
        output_lines: vec!["output from task-1".into()],
        last_output_line: "output from task-1".into(),
        ..AgentRow::default()
    });

    let mut snap = roko_core::DashboardSnapshot::default();
    snap.agents.insert(
        "agent-1".into(),
        roko_core::dashboard_snapshot::AgentState {
            agent_id: "agent-1".into(),
            role: "implementer".into(),
            active: true,
            output_bytes: 0,
            model: "test-model".into(),
            provider: String::new(),
            input_tokens: 0,
            output_tokens: 0,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            cost_usd: 0.0,
            current_task: "task-2".into(),
            current_plan: "plan-1".into(),
            attempt: 0,
            spawned_at_ms: 0,
            last_event_at_ms: 0,
            elapsed_ms: 0,
        },
    );
    snap.task_outputs.insert("task-2".into(), VecDeque::new());

    state.update_from_dashboard_snapshot(&snap);

    assert!(state.agents[0].output_lines.is_empty());
    assert!(state.agents[0].last_output_line.is_empty());
    assert_eq!(state.task_output_tails.get("task-2"), Some(&Vec::new()));
}

#[test]
fn smoothed_value_applies_ema() {
    let mut value = SmoothedValue::new(0.25);
    assert_eq!(value.update(100.0), 25.0);
    assert_eq!(value.update(100.0), 43.75);
}

// -----------------------------------------------------------------------
// #366 — TUI data-pipeline cache tests (tui_pipeline_cache)
// -----------------------------------------------------------------------

#[test]
fn tui_pipeline_cache_revision_skips_rebuild_when_unchanged() {
    let mut state = TuiState::default();
    state.recent_signals.push(SignalSummary {
        id: "s1".into(),
        kind: "test".into(),
        created_at_ms: 1_700_000_000_000,
        confidence: None,
        plan_id: None,
        task_id: None,
        parent_hash: None,
        lineage: Vec::new(),
        payload_preview: "hello".into(),
    });
    // Bump revision for signals and force initial build.
    state.rev_signals.bump();
    state.refresh_cached_unified_log();
    assert_eq!(state.unified_log_entries().len(), 1);

    // Calling refresh again without bumping any revision skips rebuild.
    let prev_ptr = state.cached_unified_log.as_ptr();
    state.refresh_cached_unified_log();
    // The cache should not have been reallocated.
    assert_eq!(state.cached_unified_log.as_ptr(), prev_ptr);
    assert_eq!(state.unified_log_entries().len(), 1);
}

#[test]
fn tui_pipeline_cache_revision_rebuilds_on_signal_change() {
    let mut state = TuiState::default();
    state.rev_signals.bump();
    state.refresh_cached_unified_log();
    assert!(state.unified_log_entries().is_empty());

    // Add a signal and bump the revision.
    state.recent_signals.push(SignalSummary {
        id: "s2".into(),
        kind: "activity".into(),
        created_at_ms: 1_700_000_001_000,
        confidence: None,
        plan_id: None,
        task_id: None,
        parent_hash: None,
        lineage: Vec::new(),
        payload_preview: "world".into(),
    });
    state.rev_signals.bump();
    state.refresh_cached_unified_log();
    assert_eq!(state.unified_log_entries().len(), 1);
}

#[test]
fn tui_pipeline_cache_revision_counter_is_monotonic() {
    let mut rev = Revision::new();
    assert_eq!(rev.get(), 0);
    assert_eq!(rev.bump(), 1);
    assert_eq!(rev.bump(), 2);
    assert_eq!(rev.get(), 2);
}

#[test]
fn tui_pipeline_cache_bounded_unified_log_enforces_limit() {
    let mut state = TuiState::default();
    // Push more than MAX_UNIFIED_LOG entries.
    for i in 0..(MAX_UNIFIED_LOG + 100) {
        state.recent_signals.push(SignalSummary {
            id: format!("s-{i}"),
            kind: "fill".into(),
            created_at_ms: 1_700_000_000_000 + i as i64,
            confidence: None,
            plan_id: None,
            task_id: None,
            parent_hash: None,
            lineage: Vec::new(),
            payload_preview: "fill".into(),
        });
    }
    state.force_refresh_cached_unified_log();
    assert!(state.unified_log_entries().len() <= MAX_UNIFIED_LOG);
}

#[test]
fn tui_pipeline_cache_eviction_counters_default_zero() {
    let counters = EvictionCounters::default();
    assert_eq!(counters.unified_log, 0);
    assert_eq!(counters.diagnoses, 0);
    assert_eq!(counters.episodes, 0);
    assert_eq!(counters.gate_output, 0);
    assert_eq!(counters.token_history, 0);
    assert_eq!(counters.notifications, 0);
}

#[test]
fn tui_pipeline_cache_bounded_gate_output_enforces_limit() {
    let mut state = TuiState::default();
    for i in 0..(MAX_GATE_LINES + 50) {
        state.gate_output_lines.push_back(format!("line {i}"));
    }
    // Manually enforce the bound (as done in update_from_dashboard_snapshot).
    while state.gate_output_lines.len() > MAX_GATE_LINES {
        state.gate_output_lines.pop_front();
        state.eviction_counters.gate_output += 1;
    }
    assert_eq!(state.gate_output_lines.len(), MAX_GATE_LINES);
    assert_eq!(state.eviction_counters.gate_output, 50);
}

#[test]
fn tui_pipeline_cache_theme_default_is_dark_not_env() {
    // #366: Theme::default() should return dark() to avoid per-frame env reads.
    let theme = Theme::default();
    let dark = Theme::dark();
    assert_eq!(theme, dark);
}

// =======================================================================
// Agent output history tests (#367)
// =======================================================================

fn make_record(text: &str, kind: OutputRecordKind) -> AgentOutputRecord {
    AgentOutputRecord {
        seq: 0,
        timestamp_ms: 1_000_000,
        role: "assistant".to_string(),
        kind,
        text: text.to_string(),
        redacted: false,
        tool_id: None,
        tool_name: None,
    }
}

fn make_tool_record(text: &str, tool_name: &str) -> AgentOutputRecord {
    AgentOutputRecord {
        seq: 0,
        timestamp_ms: 1_000_000,
        role: "assistant".to_string(),
        kind: OutputRecordKind::ToolCall,
        text: text.to_string(),
        redacted: false,
        tool_id: Some("t1".to_string()),
        tool_name: Some(tool_name.to_string()),
    }
}

#[test]
fn agent_output_history_push_and_retrieve() {
    let mut history = AgentOutputHistory::default();
    history.push("agent-1", make_record("hello", OutputRecordKind::Text));
    history.push("agent-1", make_record("world", OutputRecordKind::Text));

    assert_eq!(history.len("agent-1"), 2);
    let records = history.before("agent-1", None, 10);
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].text, "hello");
    assert_eq!(records[1].text, "world");
}

#[test]
fn agent_output_history_before_pagination() {
    let mut history = AgentOutputHistory::default();
    for i in 0..10 {
        history.push(
            "a",
            make_record(&format!("line-{i}"), OutputRecordKind::Text),
        );
    }

    // Get records before seq 6 (should be seqs 1-5)
    let page = history.before("a", Some(6), 3);
    assert_eq!(page.len(), 3);
    assert_eq!(page[0].seq, 3);
    assert_eq!(page[1].seq, 4);
    assert_eq!(page[2].seq, 5);
}

#[test]
fn agent_output_history_eviction_at_capacity() {
    let mut history = AgentOutputHistory::default();
    for i in 0..MAX_AGENT_OUTPUT_RECORDS + 100 {
        history.push(
            "a",
            make_record(&format!("line-{i}"), OutputRecordKind::Text),
        );
    }

    assert_eq!(history.len("a"), MAX_AGENT_OUTPUT_RECORDS);
    assert_eq!(history.evicted, 100);
    // Oldest sequence should have advanced past 1.
    assert!(history.oldest_sequence("a") > 1);
    // Newest records should be the last ones pushed.
    let tail = history.before("a", None, 1);
    assert_eq!(
        tail[0].text,
        format!("line-{}", MAX_AGENT_OUTPUT_RECORDS + 99)
    );
}

/// P1-TUI-G2 regression: eviction must not split a ToolCall/ToolResult pair.
/// When a ToolCall is the oldest record and must be evicted, its paired
/// ToolResult (same tool_id, next in deque) must be evicted together so the
/// viewer never sees an orphaned result.
#[test]
fn agent_output_history_eviction_preserves_tool_pairs() {
    // Build a deque that is exactly at capacity with a ToolCall/ToolResult pair
    // at the front, followed by enough Text records to fill the rest.
    let mut history = AgentOutputHistory::default();

    // Front: one ToolCall + one ToolResult pair sharing tool_id "t42".
    history.push("a", {
        AgentOutputRecord {
            seq: 0,
            timestamp_ms: 0,
            role: "assistant".to_string(),
            kind: OutputRecordKind::ToolCall,
            text: String::new(),
            redacted: false,
            tool_id: Some("t42".to_string()),
            tool_name: Some("read_file".to_string()),
        }
    });
    history.push("a", {
        AgentOutputRecord {
            seq: 0,
            timestamp_ms: 0,
            role: "assistant".to_string(),
            kind: OutputRecordKind::ToolResult,
            text: "file contents".to_string(),
            redacted: false,
            tool_id: Some("t42".to_string()),
            tool_name: None,
        }
    });

    // Fill remaining slots with Text records up to capacity.
    for i in 0..MAX_AGENT_OUTPUT_RECORDS - 2 {
        history.push(
            "a",
            make_record(&format!("filler-{i}"), OutputRecordKind::Text),
        );
    }
    assert_eq!(history.len("a"), MAX_AGENT_OUTPUT_RECORDS);

    // Now push one more record, which triggers eviction of the front ToolCall.
    // The paired ToolResult should also be evicted.
    history.push("a", make_record("trigger", OutputRecordKind::Text));

    // The ToolResult with tool_id "t42" must no longer be present.
    let all = history.records_for("a");
    let orphaned_results = all
        .iter()
        .filter(|r| r.kind == OutputRecordKind::ToolResult && r.tool_id.as_deref() == Some("t42"))
        .count();
    assert_eq!(
        orphaned_results, 0,
        "orphaned ToolResult for t42 must not remain after its ToolCall was evicted"
    );
    // The ToolCall with tool_id "t42" must also be gone.
    let orphaned_calls = all
        .iter()
        .filter(|r| r.kind == OutputRecordKind::ToolCall && r.tool_id.as_deref() == Some("t42"))
        .count();
    assert_eq!(
        orphaned_calls, 0,
        "evicted ToolCall for t42 must not remain"
    );
    // The final "trigger" record should be present.
    assert!(all.back().is_some_and(|r| r.text == "trigger"));
}

#[test]
fn agent_output_history_search_matches() {
    let mut history = AgentOutputHistory::default();
    history.push("a", make_record("cargo build", OutputRecordKind::Text));
    history.push("a", make_record("running tests", OutputRecordKind::Text));
    history.push(
        "a",
        make_record("cargo test passed", OutputRecordKind::Text),
    );
    history.push("a", make_record("all done", OutputRecordKind::Text));

    let re = regex::Regex::new("cargo").unwrap();
    let matches = history.search("a", &re, None, 100);
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].text, "cargo build");
    assert_eq!(matches[1].text, "cargo test passed");
}

#[test]
fn agent_output_history_search_tool_name() {
    let mut history = AgentOutputHistory::default();
    history.push("a", make_tool_record("reading file", "read_file"));
    history.push("a", make_record("some text", OutputRecordKind::Text));
    history.push("a", make_tool_record("writing file", "write_file"));

    let re = regex::Regex::new("read_file").unwrap();
    let matches = history.search("a", &re, None, 100);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].text, "reading file");
}

#[test]
fn agent_output_history_search_with_before_seq() {
    let mut history = AgentOutputHistory::default();
    for i in 0..5 {
        history.push(
            "a",
            make_record(&format!("match-{i}"), OutputRecordKind::Text),
        );
    }

    let re = regex::Regex::new("match").unwrap();
    let matches = history.search("a", &re, Some(3), 100);
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].text, "match-0");
    assert_eq!(matches[1].text, "match-1");
}

#[test]
fn agent_output_history_agent_switching_isolation() {
    let mut history = AgentOutputHistory::default();
    history.push("agent-1", make_record("a1-line", OutputRecordKind::Text));
    history.push("agent-2", make_record("a2-line", OutputRecordKind::Text));

    assert_eq!(history.len("agent-1"), 1);
    assert_eq!(history.len("agent-2"), 1);
    assert_eq!(history.before("agent-1", None, 10)[0].text, "a1-line");
    assert_eq!(history.before("agent-2", None, 10)[0].text, "a2-line");
    // Unknown agent returns empty
    assert_eq!(history.len("agent-3"), 0);
    assert!(history.before("agent-3", None, 10).is_empty());
}

#[test]
fn agent_output_history_clear_agent() {
    let mut history = AgentOutputHistory::default();
    history.push("a", make_record("line1", OutputRecordKind::Text));
    history.push("a", make_record("line2", OutputRecordKind::Text));
    assert_eq!(history.len("a"), 2);

    history.clear_agent("a");
    assert_eq!(history.len("a"), 0);
    assert_eq!(history.oldest_sequence("a"), 1);
    assert_eq!(history.next_sequence("a"), 1);
}

#[test]
fn agent_output_history_dedup_live_settled() {
    let mut history = AgentOutputHistory::default();
    // Push record with seq=1
    history.push("a", make_record("first", OutputRecordKind::Text));
    let first_seq = history.before("a", None, 1)[0].seq;

    // Attempt to push duplicate with same seq (simulating live+settled copy)
    let mut dup = make_record("first (settled)", OutputRecordKind::Text);
    dup.seq = first_seq;
    history.push_dedup("a", dup);

    // Only one record should exist
    assert_eq!(history.len("a"), 1);
    assert_eq!(history.before("a", None, 1)[0].text, "first");
}

#[test]
fn agent_output_history_ingest_lines() {
    let mut history = AgentOutputHistory::default();
    let lines = vec![
        "Running cargo test".to_string(),
        "All tests passed".to_string(),
    ];
    history.ingest_lines("a", &lines, "tool");

    assert_eq!(history.len("a"), 2);
    let records = history.before("a", None, 10);
    assert_eq!(records[0].text, "Running cargo test");
    assert_eq!(records[0].role, "tool");
    assert_eq!(records[1].text, "All tests passed");
}

#[test]
fn agent_output_history_tail_pin_behavior() {
    let mut history = AgentOutputHistory::default();
    for i in 0..100 {
        history.push(
            "a",
            make_record(&format!("line-{i}"), OutputRecordKind::Text),
        );
    }

    // Tail mode: no before_seq, limited results
    let tail = history.before("a", None, 5);
    assert_eq!(tail.len(), 5);
    assert_eq!(tail[4].text, "line-99");

    // Pinned mode: specific before_seq
    let pinned = history.before("a", Some(10), 5);
    assert_eq!(pinned.len(), 5);
    assert_eq!(pinned[0].text, "line-4");
    assert_eq!(pinned[4].text, "line-8");
}

#[test]
fn agent_output_history_record_kinds() {
    let mut history = AgentOutputHistory::default();
    history.push("a", make_record("hello", OutputRecordKind::Text));
    history.push("a", make_record("thinking...", OutputRecordKind::Reasoning));
    history.push("a", make_tool_record("grep", "search_files"));
    history.push(
        "a",
        AgentOutputRecord {
            seq: 0,
            timestamp_ms: 1_000,
            role: "tool".to_string(),
            kind: OutputRecordKind::ToolResult,
            text: "found 3 matches".to_string(),
            redacted: false,
            tool_id: Some("t1".to_string()),
            tool_name: None,
        },
    );
    history.push("a", make_record("compile error", OutputRecordKind::Error));
    history.push("a", make_record("restarting", OutputRecordKind::System));

    let all = history.before("a", None, 100);
    assert_eq!(all.len(), 6);
    assert_eq!(all[0].kind, OutputRecordKind::Text);
    assert_eq!(all[1].kind, OutputRecordKind::Reasoning);
    assert_eq!(all[2].kind, OutputRecordKind::ToolCall);
    assert_eq!(all[3].kind, OutputRecordKind::ToolResult);
    assert_eq!(all[4].kind, OutputRecordKind::Error);
    assert_eq!(all[5].kind, OutputRecordKind::System);
}

#[test]
fn agent_output_history_search_invalid_regex() {
    let mut search = AgentOutputSearchState::default();
    search.pattern = "[invalid(".to_string();
    search.recompile();

    assert!(search.pattern_error);
    assert!(search.compiled.is_none());
    assert_eq!(search.match_count, 0);
}

#[test]
fn agent_output_history_search_navigation() {
    let mut history = AgentOutputHistory::default();
    for i in 0..10 {
        history.push(
            "a",
            make_record(
                &format!("line-{i}"),
                if i % 3 == 0 {
                    OutputRecordKind::Error
                } else {
                    OutputRecordKind::Text
                },
            ),
        );
    }

    let mut search = AgentOutputSearchState::default();
    search.pattern = "line-[036]".to_string();
    search.recompile();
    search.update_matches(&history, "a");

    assert!(!search.pattern_error);
    assert_eq!(search.match_count, 3);
    assert_eq!(search.current_match, 0);

    search.next_match();
    assert_eq!(search.current_match, 1);

    search.next_match();
    assert_eq!(search.current_match, 2);

    // Wrap around
    search.next_match();
    assert_eq!(search.current_match, 0);

    // Prev wraps backward
    search.prev_match();
    assert_eq!(search.current_match, 2);
}

#[test]
fn agent_output_history_search_clear() {
    let mut search = AgentOutputSearchState::default();
    search.active = true;
    search.pattern = "test".to_string();
    search.recompile();

    let mut history = AgentOutputHistory::default();
    history.push("a", make_record("test line", OutputRecordKind::Text));
    search.update_matches(&history, "a");

    assert_eq!(search.match_count, 1);

    search.clear();
    assert!(!search.active);
    assert!(search.pattern.is_empty());
    assert!(search.compiled.is_none());
    assert_eq!(search.match_count, 0);
    assert!(search.match_seqs.is_empty());
}

#[test]
fn agent_output_history_redaction_flag() {
    let mut history = AgentOutputHistory::default();
    let mut record = make_record("secret: abc123", OutputRecordKind::Text);
    record.redacted = true;
    history.push("a", record);

    let records = history.before("a", None, 1);
    assert!(records[0].redacted);
}

#[test]
fn agent_output_history_classify_stream_records() {
    let lines = vec![
        "\x1eroko.stream.v1 {\"kind\":\"text\",\"content\":\"hello\"}".to_string(),
        "\x1eroko.stream.v1 {\"kind\":\"reasoning\",\"content\":\"thinking\"}".to_string(),
        "\x1eroko.stream.v1 {\"kind\":\"tool_start\",\"tool_name\":\"bash\",\"tool_id\":\"t1\"}"
            .to_string(),
        "\x1eroko.stream.v1 {\"kind\":\"tool_result\",\"tool_id\":\"t1\",\"output\":\"ok\"}"
            .to_string(),
        "ERROR: something failed".to_string(),
        "plain text line".to_string(),
        "────turn boundary".to_string(),
    ];

    let mut history = AgentOutputHistory::default();
    history.ingest_lines("a", &lines, "assistant");

    let records = history.before("a", None, 100);
    assert_eq!(records.len(), 7);
    assert_eq!(records[0].kind, OutputRecordKind::Text);
    assert_eq!(records[1].kind, OutputRecordKind::Reasoning);
    assert_eq!(records[2].kind, OutputRecordKind::ToolCall);
    assert_eq!(records[2].tool_name, Some("bash".to_string()));
    assert_eq!(records[3].kind, OutputRecordKind::ToolResult);
    assert_eq!(records[3].tool_id, Some("t1".to_string()));
    assert_eq!(records[4].kind, OutputRecordKind::Error);
    assert_eq!(records[5].kind, OutputRecordKind::Text);
    assert_eq!(records[6].kind, OutputRecordKind::System);
}

#[test]
fn agent_output_history_empty_agent_returns_defaults() {
    let history = AgentOutputHistory::default();
    assert_eq!(history.len("nonexistent"), 0);
    assert_eq!(history.oldest_sequence("nonexistent"), 1);
    assert_eq!(history.next_sequence("nonexistent"), 1);
    assert!(history.records_for("nonexistent").is_empty());
    assert!(history.before("nonexistent", None, 10).is_empty());

    let re = regex::Regex::new("test").unwrap();
    assert!(history.search("nonexistent", &re, None, 10).is_empty());
}

// -- #368 per-tab detail scroll clamp tests --

#[test]
fn clamp_git_detail_scroll_clamps_to_max() {
    let mut state = TuiState::default();
    state.git_detail_scroll = 100;
    state.clamp_git_detail_scroll(42);
    assert_eq!(state.git_detail_scroll, 42);

    // Already within range: no change.
    state.clamp_git_detail_scroll(50);
    assert_eq!(state.git_detail_scroll, 42);
}

#[test]
fn clamp_config_values_scroll_clamps_to_max() {
    let mut state = TuiState::default();
    state.config_values_scroll = 80;
    state.clamp_config_values_scroll(30);
    assert_eq!(state.config_values_scroll, 30);
}

#[test]
fn clamp_inspect_detail_scroll_clamps_to_max() {
    let mut state = TuiState::default();
    state.inspect_detail_scroll = 60;
    state.clamp_inspect_detail_scroll(25);
    assert_eq!(state.inspect_detail_scroll, 25);
}

#[test]
fn clamp_learning_detail_scroll_clamps_to_max() {
    let mut state = TuiState::default();
    state.learning_detail_scroll = 50;
    state.clamp_learning_detail_scroll(10);
    assert_eq!(state.learning_detail_scroll, 10);
}

#[test]
fn clamp_procs_scroll_clamps_to_max() {
    let mut state = TuiState::default();
    state.procs_scroll = 40;
    state.clamp_procs_scroll(15);
    assert_eq!(state.procs_scroll, 15);
}

#[test]
fn clamp_config_scroll_offset_clamps_to_max() {
    let mut state = TuiState::default();
    state.config_scroll_offset = 90;
    state.clamp_config_scroll_offset(20);
    assert_eq!(state.config_scroll_offset, 20);
}

#[test]
fn clamp_log_detail_scroll_clamps_to_max() {
    let mut state = TuiState::default();
    state.log_detail_scroll = 70;
    state.clamp_log_detail_scroll(35);
    assert_eq!(state.log_detail_scroll, 35);
}

#[test]
fn clamp_marketplace_detail_scroll_clamps_to_max() {
    let mut state = TuiState::default();
    state.marketplace_detail_scroll = 55;
    state.clamp_marketplace_detail_scroll(22);
    assert_eq!(state.marketplace_detail_scroll, 22);
}

#[test]
fn clamp_atelier_detail_scroll_clamps_to_max() {
    let mut state = TuiState::default();
    state.atelier_detail_scroll = 45;
    state.clamp_atelier_detail_scroll(18);
    assert_eq!(state.atelier_detail_scroll, 18);
}

#[test]
fn reset_scrolls_includes_all_detail_fields() {
    let mut state = TuiState::default();
    state.git_detail_scroll = 10;
    state.config_values_scroll = 20;
    state.inspect_detail_scroll = 30;
    state.learning_detail_scroll = 40;
    state.procs_scroll = 50;
    state.log_detail_scroll = 60;
    state.marketplace_detail_scroll = 70;
    state.atelier_detail_scroll = 80;

    state.reset_scrolls();

    assert_eq!(state.git_detail_scroll, 0);
    assert_eq!(state.config_values_scroll, 0);
    assert_eq!(state.inspect_detail_scroll, 0);
    assert_eq!(state.learning_detail_scroll, 0);
    assert_eq!(state.procs_scroll, 0);
    assert_eq!(state.log_detail_scroll, 0);
    assert_eq!(state.marketplace_detail_scroll, 0);
    assert_eq!(state.atelier_detail_scroll, 0);
}

#[test]
fn accepted_with_failures_is_its_own_task_state() {
    use roko_core::DashboardEvent;
    use roko_core::dashboard_snapshot::{DashboardSnapshot, TASK_OUTCOME_ACCEPTED_WITH_FAILURES};

    let mut snap = DashboardSnapshot::default();
    snap.apply(&DashboardEvent::PlanStarted {
        plan_id: "p1".into(),
        tasks_total: 3,
    });
    for task_id in ["t1", "t2", "t3"] {
        snap.apply(&DashboardEvent::TaskStarted {
            plan_id: "p1".into(),
            task_id: task_id.into(),
            title: task_id.into(),
            phase: "verify".into(),
        });
    }
    for (task_id, outcome) in [
        ("t1", "passed"),
        ("t2", TASK_OUTCOME_ACCEPTED_WITH_FAILURES),
        ("t3", "failed"),
    ] {
        snap.apply(&DashboardEvent::TaskCompleted {
            plan_id: "p1".into(),
            task_id: task_id.into(),
            outcome: outcome.into(),
        });
    }

    let mut state = TuiState::default();
    state.update_from_dashboard_snapshot(&snap);
    let plan = state.plans.iter().find(|plan| plan.id == "p1").expect("p1");
    let status = |id: &str| {
        plan.tasks
            .iter()
            .find(|task| task.id == id)
            .expect("task")
            .status
    };
    assert_eq!(status("t1"), TaskStatus::Done);
    assert_eq!(status("t2"), TaskStatus::AcceptedWithFailures);
    assert_eq!(status("t3"), TaskStatus::Failed);
    assert!(!status("t2").is_failed());
    assert_eq!(plan.tasks_accepted_with_failures(), 1);
    assert_eq!(plan.tasks_failed, 1);
    assert_eq!(
        TaskStatus::from(TASK_OUTCOME_ACCEPTED_WITH_FAILURES),
        TaskStatus::AcceptedWithFailures
    );
}

// =======================================================================
// Live-unscreened tracking and settle tests (T19)
// =======================================================================

/// Ingesting a live+unscreened text record tracks its seq in
/// `live_unscreened_seqs` so it can be dropped on settle.
#[test]
fn ingest_live_unscreened_text_is_tracked() {
    let mut history = AgentOutputHistory::default();
    let unscreened_line =
        "\x1eroko.stream.v1 {\"kind\":\"text\",\"content\":\"draft\",\"live\":true,\"screened\":false}"
            .to_string();
    history.ingest_lines("agent-a", &[unscreened_line], "assistant");

    // The record must be present.
    assert_eq!(history.len("agent-a"), 1);
    // The seq must be in the unscreened tracking set.
    let seq = history.before("agent-a", None, 1)[0].seq;
    assert!(
        history
            .live_unscreened_seqs
            .get("agent-a")
            .is_some_and(|s| s.contains(&seq)),
        "live unscreened seq {seq} must be tracked"
    );
}

/// `settle_screened_transcript` drops unscreened text records but keeps
/// tool steps (ToolCall/ToolResult) intact.
#[test]
fn settle_screened_transcript_keeps_tool_steps() {
    let mut history = AgentOutputHistory::default();

    // Push a live unscreened text record.
    let unscreened_text =
        "\x1eroko.stream.v1 {\"kind\":\"text\",\"content\":\"draft\",\"live\":true,\"screened\":false}"
            .to_string();
    // Push a live tool start (should be KEPT after settle).
    let tool_step = "\x1eroko.stream.v1 {\"kind\":\"tool_start\",\"tool_name\":\"Write\",\"tool_id\":\"t1\",\"live\":true}".to_string();
    // Push another unscreened reasoning record.
    let unscreened_reasoning = "\x1eroko.stream.v1 {\"kind\":\"reasoning\",\"content\":\"thinking\",\"live\":true,\"screened\":false}".to_string();

    history.ingest_lines(
        "a",
        &[unscreened_text, tool_step, unscreened_reasoning],
        "assistant",
    );
    assert_eq!(history.len("a"), 3);

    // Settle: provide the screened transcript as two new lines.
    let settled =
        vec!["\x1eroko.stream.v1 {\"kind\":\"text\",\"content\":\"settled output\"}".to_string()];
    history.settle_screened_transcript("a", &settled, "assistant");

    // After settle: 1 tool step (kept) + 1 settled text line.
    let records = history.before("a", None, 10);
    let kinds: Vec<OutputRecordKind> = records.iter().map(|r| r.kind).collect();
    assert!(
        kinds.contains(&OutputRecordKind::ToolCall),
        "tool call must be kept after settle: {kinds:?}"
    );
    let tool_count = kinds
        .iter()
        .filter(|&&k| k == OutputRecordKind::ToolCall)
        .count();
    assert_eq!(tool_count, 1, "exactly one tool call must remain");
    let text_count = kinds
        .iter()
        .filter(|&&k| k == OutputRecordKind::Text)
        .count();
    assert_eq!(
        text_count, 1,
        "exactly one settled text record must be present"
    );

    // Unscreened seqs must be cleared after settle.
    assert!(
        history
            .live_unscreened_seqs
            .get("a")
            .map_or(true, |s| s.is_empty()),
        "live_unscreened_seqs must be cleared after settle"
    );
}

/// Settling an agent with no unscreened records is a safe no-op.
#[test]
fn settle_screened_transcript_noop_when_no_unscreened() {
    let mut history = AgentOutputHistory::default();
    let lines = vec!["\x1eroko.stream.v1 {\"kind\":\"text\",\"content\":\"hello\"}".to_string()];
    history.ingest_lines("a", &lines, "assistant");
    assert_eq!(history.len("a"), 1);

    // Settle with new content — should just append (no records dropped).
    let settled = vec!["\x1eroko.stream.v1 {\"kind\":\"text\",\"content\":\"world\"}".to_string()];
    history.settle_screened_transcript("a", &settled, "assistant");
    assert_eq!(history.len("a"), 2);
}

/// gap-f59fe9: a task blocked by a failed one, which never started, is
/// listed in its plan's rows as blocked and names the task that blocked it.
/// It is not counted as done.
#[test]
fn update_from_dashboard_snapshot_lists_blocked_tasks() {
    use roko_core::DashboardEvent;

    let mut snap = roko_core::DashboardSnapshot::default();
    for event in [
        DashboardEvent::PlanStarted {
            plan_id: "plan-a".into(),
            tasks_total: 2,
        },
        DashboardEvent::TaskStarted {
            plan_id: "plan-a".into(),
            task_id: "T1".into(),
            title: "First".into(),
            phase: "implementer".into(),
        },
        DashboardEvent::TaskCompleted {
            plan_id: "plan-a".into(),
            task_id: "T1".into(),
            outcome: "failed".into(),
        },
        // The status poll reports the blocked task as skipped first.
        DashboardEvent::TaskCompleted {
            plan_id: "plan-a".into(),
            task_id: "T4".into(),
            outcome: "skipped".into(),
        },
        DashboardEvent::TaskBlocked {
            plan_id: "plan-a".into(),
            task_id: "T4".into(),
            title: "Fourth".into(),
            blocked_by: Some("T1".into()),
            reason: "blocked by failed task 'T1'".into(),
        },
    ] {
        snap.apply(&event);
    }

    let mut state = TuiState::default();
    state.update_from_dashboard_snapshot(&snap);

    assert_eq!(state.plans.len(), 1);
    assert_eq!(state.plans[0].tasks_done, 0, "a blocked task is not done");
    let entry = state.plans[0]
        .tasks
        .iter()
        .find(|task| task.id == "T4")
        .expect("T4 is listed");
    assert_eq!(entry.status, TaskStatus::Blocked);
    assert_eq!(entry.name, "Fourth");
    assert_eq!(entry.depends_on, ["T1"]);
    let row = state
        .current_task_checklist
        .iter()
        .find(|row| row.id == "T4")
        .expect("T4 has a row");
    assert_eq!(row.status, TaskStatus::Blocked);
    assert_eq!(row.depends_on, ["T1"]);
}

#[test]
fn agent_output_history_takes_later_ring_lines() {
    use roko_core::DashboardEvent;

    let lines = |range: std::ops::Range<usize>| -> Vec<String> {
        range.map(|n| format!("line-{n}")).collect()
    };
    let append = |snap: &mut roko_core::DashboardSnapshot, range| {
        snap.apply(&DashboardEvent::TaskOutputAppended {
            task_id: "task-1".into(),
            lines: lines(range),
        });
    };
    let texts = |state: &TuiState| -> Vec<String> {
        state
            .agent_output_history
            .records_for("agent-1")
            .iter()
            .map(|record| record.text.clone())
            .collect()
    };
    let mut snap = roko_core::DashboardSnapshot::default();
    snap.apply(&DashboardEvent::AgentSpawned {
        agent_id: "agent-1".into(),
        plan_id: "plan-1".into(),
        task_id: "task-1".into(),
        attempt: 1,
        role: "implementer".into(),
        model: "test-model".into(),
        provider: String::new(),
    });
    let mut state = TuiState::default();

    // The first ring is taken in whole.
    append(&mut snap, 0..3);
    state.update_from_dashboard_snapshot(&snap);
    assert_eq!(texts(&state), lines(0..3));

    // An unchanged ring adds nothing; the lines a later ring adds follow once.
    state.update_from_dashboard_snapshot(&snap);
    append(&mut snap, 3..5);
    state.update_from_dashboard_snapshot(&snap);
    assert_eq!(texts(&state), lines(0..5));

    // Once AgentOutput events feed the agent, rings only repeat them.
    state.push_agent_output_record(
        "agent-1",
        OutputRecordKind::Text,
        "from an event".into(),
        None,
        None,
    );
    append(&mut snap, 5..6);
    state.update_from_dashboard_snapshot(&snap);
    let after = texts(&state);
    assert_eq!(after.len(), 6, "{after:?}");
    assert_eq!(after.last().map(String::as_str), Some("from an event"));
}
