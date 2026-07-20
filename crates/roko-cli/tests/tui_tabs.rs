//! Verify F8 Marketplace, F9 Atelier, and F10 Learning tabs are fully wired.
//! Also tests TUI responsiveness invariants (SH06-T03): parallel agents,
//! gate failure diagnosis, token counters, phase transitions, agent timing,
//! error ring buffers, and agent completion.

use roko_cli::tui::tabs::Tab;
use roko_cli::tui::views::{SubView, ViewState};
use roko_core::dashboard_snapshot::{
    DashboardEvent, DashboardSnapshot, DiagnosisSeverity, DiagnosisSummary,
};

#[test]
fn tab_all_has_ten_entries() {
    assert_eq!(Tab::ALL.len(), 10);
}

#[test]
fn marketplace_tab_basics() {
    assert_eq!(Tab::Marketplace.fkey(), crossterm::event::KeyCode::F(8));
    assert_eq!(
        Tab::from_key(crossterm::event::KeyCode::F(8)),
        Some(Tab::Marketplace)
    );
    assert_eq!(Tab::Marketplace.label(), "Marketplace");
    assert_eq!(Tab::Marketplace.index(), 7);
}

#[test]
fn atelier_tab_basics() {
    assert_eq!(Tab::Atelier.fkey(), crossterm::event::KeyCode::F(9));
    assert_eq!(
        Tab::from_key(crossterm::event::KeyCode::F(9)),
        Some(Tab::Atelier)
    );
    assert_eq!(Tab::Atelier.label(), "Atelier");
    assert_eq!(Tab::Atelier.index(), 8);
}

#[test]
fn next_prev_cycle_ten_tabs() {
    let mut t = Tab::Dashboard;
    for _ in 0..10 {
        t = t.next();
    }
    assert_eq!(t, Tab::Dashboard);

    for _ in 0..10 {
        t = t.prev();
    }
    assert_eq!(t, Tab::Dashboard);
}

#[test]
fn learning_tab_basics() {
    assert_eq!(Tab::Learning.fkey(), crossterm::event::KeyCode::F(10));
    assert_eq!(
        Tab::from_key(crossterm::event::KeyCode::F(10)),
        Some(Tab::Learning)
    );
    assert_eq!(Tab::Learning.label(), "Learning");
    assert_eq!(Tab::Learning.index(), 9);
}

#[test]
fn learning_has_subviews() {
    let subs = SubView::for_tab(Tab::Learning);
    assert!(!subs.is_empty());
    assert!(subs.iter().any(|s| s.label() == "Route"));
}

#[test]
fn marketplace_has_subviews() {
    let subs = SubView::for_tab(Tab::Marketplace);
    assert!(!subs.is_empty());
    assert!(subs.iter().any(|s| s.label() == "Jobs"));
}

#[test]
fn atelier_has_subviews() {
    let subs = SubView::for_tab(Tab::Atelier);
    assert!(!subs.is_empty());
    assert!(subs.iter().any(|s| s.label() == "PRDs"));
}

#[test]
fn fkey_roundtrip_all_ten() {
    for tab in Tab::ALL {
        assert_eq!(Tab::from_key(tab.fkey()), Some(tab));
    }
}

#[test]
fn index_sequential_all_ten() {
    for (i, tab) in Tab::ALL.iter().enumerate() {
        assert_eq!(tab.index(), i);
    }
}

#[test]
fn view_state_resolves_marketplace_subview() {
    let vs = ViewState {
        sub_tab: 0,
        ..Default::default()
    };
    let sub = vs.active_sub_view(Tab::Marketplace);
    assert_eq!(sub.label(), "Jobs");
}

#[test]
fn view_state_resolves_atelier_subview() {
    let vs = ViewState {
        sub_tab: 0,
        ..Default::default()
    };
    let sub = vs.active_sub_view(Tab::Atelier);
    assert_eq!(sub.label(), "PRDs");
}

// ---------------------------------------------------------------------------
// SH06-T03: Dashboard snapshot responsiveness and invariant tests
// ---------------------------------------------------------------------------

/// Helper: spawn an agent via `apply_with_ts`.
fn spawn(
    s: &mut DashboardSnapshot,
    id: &str,
    plan: &str,
    task: &str,
    att: u32,
    role: &str,
    model: &str,
    ts: u64,
) {
    s.apply_with_ts(
        &DashboardEvent::AgentSpawned {
            agent_id: id.into(),
            plan_id: plan.into(),
            task_id: task.into(),
            attempt: att,
            role: role.into(),
            model: model.into(),
        },
        ts,
    );
}

/// Helper: emit an efficiency metric.
fn eff(s: &mut DashboardSnapshot, plan: &str, task: &str, metric: &str, v: f64, ts: u64) {
    s.apply_with_ts(
        &DashboardEvent::EfficiencyEvent {
            plan_id: plan.into(),
            task_id: task.into(),
            metric: metric.into(),
            value: v,
        },
        ts,
    );
}

/// Helper: emit a diagnosis.
fn diag(s: &mut DashboardSnapshot, id: &str, sev: DiagnosisSeverity, detail: &str, ts: u64) {
    s.apply_with_ts(
        &DashboardEvent::Diagnosis {
            summary: DiagnosisSummary {
                id: id.into(),
                severity: sev,
                subject: id.into(),
                detail: detail.into(),
                ..Default::default()
            },
        },
        ts,
    );
}

// ---- 1. Parallel agents ---------------------------------------------------

#[test]
fn parallel_agents_create_distinct_entries() {
    let mut s = DashboardSnapshot::default();
    spawn(&mut s, "a", "p1", "t1", 1, "implementer", "sonnet", 1000);
    spawn(&mut s, "b", "p1", "t2", 1, "reviewer", "opus", 1001);
    spawn(&mut s, "c", "p2", "t1", 2, "tester", "haiku", 1002);

    assert_eq!(s.agents.len(), 3);
    assert_eq!(s.stats.agents_active, 3);

    let a = &s.agents["a"];
    assert_eq!(
        (a.current_plan.as_str(), a.current_task.as_str(), a.attempt),
        ("p1", "t1", 1)
    );
    assert_eq!(
        (a.role.as_str(), a.model.as_str()),
        ("implementer", "sonnet")
    );
    assert!(a.active);

    let b = &s.agents["b"];
    assert_eq!(
        (b.current_task.as_str(), b.role.as_str()),
        ("t2", "reviewer")
    );

    let c = &s.agents["c"];
    assert_eq!(
        (c.current_plan.as_str(), c.attempt, c.role.as_str()),
        ("p2", 2, "tester")
    );
}

#[test]
fn parallel_agents_independent_output() {
    let mut s = DashboardSnapshot::default();
    spawn(&mut s, "a1", "p", "t1", 1, "c", "m", 100);
    spawn(&mut s, "a2", "p", "t2", 1, "c", "m", 100);
    s.apply_with_ts(
        &DashboardEvent::AgentOutput {
            agent_id: "a1".into(),
            plan_id: "p".into(),
            task_id: "t1".into(),
            attempt: 1,
            content: "hello".into(),
        },
        200,
    );
    s.apply_with_ts(
        &DashboardEvent::AgentOutput {
            agent_id: "a2".into(),
            plan_id: "p".into(),
            task_id: "t2".into(),
            attempt: 1,
            content: "hello world!".into(),
        },
        200,
    );
    assert_eq!(s.agents["a1"].output_bytes, 5);
    assert_eq!(s.agents["a2"].output_bytes, 12);
}

// ---- 2. Diagnosis ring buffer ---------------------------------------------

#[test]
fn diagnosis_populates_and_orders() {
    let mut s = DashboardSnapshot::default();
    for i in 0..5 {
        diag(
            &mut s,
            &format!("d-{i}"),
            DiagnosisSeverity::Warn,
            &format!("det-{i}"),
            1000 + i,
        );
    }
    assert_eq!(s.diagnoses.len(), 5);
    assert_eq!(s.diagnoses.front().unwrap().id, "d-0");
    assert_eq!(s.diagnoses.back().unwrap().id, "d-4");
}

#[test]
fn diagnosis_ring_evicts_at_max_50() {
    let mut s = DashboardSnapshot::default();
    for i in 0..60 {
        diag(
            &mut s,
            &format!("d-{i}"),
            DiagnosisSeverity::Info,
            "",
            i as u64,
        );
    }
    assert_eq!(s.diagnoses.len(), 50);
    assert_eq!(s.diagnoses.front().unwrap().id, "d-10");
    assert_eq!(s.diagnoses.back().unwrap().id, "d-59");
}

#[test]
fn diagnosis_dedup_by_id() {
    let mut s = DashboardSnapshot::default();
    diag(&mut s, "dup", DiagnosisSeverity::Info, "v1", 100);
    diag(&mut s, "other", DiagnosisSeverity::Warn, "", 200);
    diag(&mut s, "dup", DiagnosisSeverity::Alert, "v2", 300);
    assert_eq!(s.diagnoses.len(), 2);
    let latest = s.diagnoses.back().unwrap();
    assert_eq!(
        (latest.id.as_str(), latest.detail.as_str(), latest.severity),
        ("dup", "v2", DiagnosisSeverity::Alert)
    );
}

// ---- 3. Token counters ----------------------------------------------------

#[test]
fn token_accumulation() {
    let mut s = DashboardSnapshot::default();
    spawn(&mut s, "tok", "px", "tx", 1, "c", "sonnet", 500);
    for (m, v) in [
        ("input_tokens", 100.0),
        ("input_tokens", 50.0),
        ("output_tokens", 200.0),
        ("output_tokens", 75.0),
        ("cache_read_tokens", 30.0),
        ("cache_write_tokens", 10.0),
        ("cache_write_tokens", 5.0),
        ("cost_usd", 0.01),
        ("cost_usd", 0.02),
    ] {
        eff(&mut s, "px", "tx", m, v, 600);
    }
    let a = &s.agents["tok"];
    assert_eq!((a.input_tokens, a.output_tokens), (150, 275));
    assert_eq!((a.cache_read_tokens, a.cache_write_tokens), (30, 15));
    assert!((a.cost_usd - 0.03).abs() < 1e-9);
    assert!((s.stats.cost_usd_total - 0.03).abs() < 1e-9);
}

#[test]
fn tokens_routed_to_correct_agent() {
    let mut s = DashboardSnapshot::default();
    spawn(&mut s, "alpha", "p1", "t1", 1, "c", "m", 100);
    spawn(&mut s, "beta", "p1", "t2", 1, "r", "m", 100);
    eff(&mut s, "p1", "t1", "input_tokens", 42.0, 200);
    eff(&mut s, "p1", "t2", "output_tokens", 99.0, 200);
    assert_eq!(
        (
            s.agents["alpha"].input_tokens,
            s.agents["alpha"].output_tokens
        ),
        (42, 0)
    );
    assert_eq!(
        (
            s.agents["beta"].output_tokens,
            s.agents["beta"].input_tokens
        ),
        (99, 0)
    );
}

// ---- 4. Phase transitions --------------------------------------------------

#[test]
fn phase_transitions() {
    let mut s = DashboardSnapshot::default();
    s.apply_with_ts(
        &DashboardEvent::PlanStarted {
            plan_id: "ph".into(),
        },
        100,
    );
    assert_eq!(s.plans["ph"].phase, "started");

    s.apply_with_ts(
        &DashboardEvent::PhaseTransition {
            plan_id: "ph".into(),
            from: "started".into(),
            to: "compose".into(),
        },
        200,
    );
    assert_eq!(s.plans["ph"].phase, "compose");

    s.apply_with_ts(
        &DashboardEvent::PhaseTransition {
            plan_id: "ph".into(),
            from: "compose".into(),
            to: "dispatch".into(),
        },
        300,
    );
    assert_eq!(s.plans["ph"].phase, "dispatch");
}

#[test]
fn phase_transition_unknown_plan_noop() {
    let mut s = DashboardSnapshot::default();
    s.apply_with_ts(
        &DashboardEvent::PhaseTransition {
            plan_id: "ghost".into(),
            from: "a".into(),
            to: "b".into(),
        },
        100,
    );
    assert!(!s.plans.contains_key("ghost"));
}

// ---- 5. Agent timing -------------------------------------------------------

#[test]
fn agent_timing_lifecycle() {
    let mut s = DashboardSnapshot::default();
    spawn(&mut s, "ag", "p", "t", 1, "c", "m", 5000);
    assert_eq!(
        (
            s.agents["ag"].spawned_at_ms,
            s.agents["ag"].last_event_at_ms
        ),
        (5000, 5000)
    );

    // Output advances last_event_at but not spawned_at.
    s.apply_with_ts(
        &DashboardEvent::AgentOutput {
            agent_id: "ag".into(),
            plan_id: "p".into(),
            task_id: "t".into(),
            attempt: 1,
            content: "x".into(),
        },
        6000,
    );
    assert_eq!(
        (
            s.agents["ag"].spawned_at_ms,
            s.agents["ag"].last_event_at_ms
        ),
        (5000, 6000)
    );

    // Completion advances last_event_at.
    s.apply_with_ts(
        &DashboardEvent::AgentCompleted {
            agent_id: "ag".into(),
            plan_id: "p".into(),
            task_id: "t".into(),
            attempt: 1,
        },
        7000,
    );
    assert_eq!(s.agents["ag"].last_event_at_ms, 7000);
}

#[test]
fn respawn_resets_timing_preserves_tokens() {
    let mut s = DashboardSnapshot::default();
    spawn(&mut s, "re", "p", "t", 1, "c", "m", 1000);
    eff(&mut s, "p", "t", "input_tokens", 42.0, 1500);
    s.apply_with_ts(
        &DashboardEvent::AgentCompleted {
            agent_id: "re".into(),
            plan_id: "p".into(),
            task_id: "t".into(),
            attempt: 1,
        },
        2000,
    );
    assert!(!s.agents["re"].active);

    spawn(&mut s, "re", "p", "t", 2, "c", "m", 3000);
    let a = &s.agents["re"];
    assert!(a.active);
    assert_eq!((a.spawned_at_ms, a.attempt, a.input_tokens), (3000, 2, 42));
}

// ---- 6. Error ring buffer --------------------------------------------------

#[test]
fn error_ring_evicts_at_max_64() {
    let mut s = DashboardSnapshot::default();
    for i in 0..80 {
        s.apply_with_ts(
            &DashboardEvent::Error {
                message: format!("e-{i}"),
            },
            i as u64,
        );
    }
    assert_eq!(s.errors.len(), 64);
    assert_eq!(s.stats.errors_total, 80);
    assert_eq!(s.errors[0].message, "e-16");
    assert_eq!(s.errors[63].message, "e-79");
}

#[test]
fn error_records_timestamp() {
    let mut s = DashboardSnapshot::default();
    s.apply_with_ts(
        &DashboardEvent::Error {
            message: "oops".into(),
        },
        9999,
    );
    assert_eq!(
        (s.errors[0].ts_millis, s.errors[0].message.as_str()),
        (9999, "oops")
    );
}

// ---- 7. AgentCompleted deactivation ----------------------------------------

#[test]
fn agent_completed_deactivates() {
    let mut s = DashboardSnapshot::default();
    spawn(&mut s, "d", "p", "t", 1, "c", "m", 100);
    assert!(s.agents["d"].active);
    s.apply_with_ts(
        &DashboardEvent::AgentCompleted {
            agent_id: "d".into(),
            plan_id: "p".into(),
            task_id: "t".into(),
            attempt: 1,
        },
        500,
    );
    assert!(!s.agents["d"].active);
    assert_eq!(
        (s.stats.agents_active, s.agents["d"].last_event_at_ms),
        (0, 500)
    );
}

#[test]
fn completing_one_leaves_others_active() {
    let mut s = DashboardSnapshot::default();
    spawn(&mut s, "stay", "p", "t1", 1, "c", "m", 100);
    spawn(&mut s, "leave", "p", "t2", 1, "r", "m", 100);
    assert_eq!(s.stats.agents_active, 2);
    s.apply_with_ts(
        &DashboardEvent::AgentCompleted {
            agent_id: "leave".into(),
            plan_id: "p".into(),
            task_id: "t2".into(),
            attempt: 1,
        },
        200,
    );
    assert!((s.agents["stay"].active, !s.agents["leave"].active) == (true, true));
    assert_eq!(s.stats.agents_active, 1);
}

// ---- Cross-cutting lifecycle replay ----------------------------------------

#[test]
fn full_lifecycle_replay() {
    let mut s = DashboardSnapshot::default();
    s.apply_with_ts(
        &DashboardEvent::PlanStarted {
            plan_id: "lc".into(),
        },
        100,
    );
    s.apply_with_ts(
        &DashboardEvent::TaskStarted {
            plan_id: "lc".into(),
            task_id: "ta".into(),
            title: "feat".into(),
            phase: "compose".into(),
        },
        200,
    );
    spawn(&mut s, "w", "lc", "ta", 1, "impl", "sonnet", 300);
    s.apply_with_ts(
        &DashboardEvent::PhaseTransition {
            plan_id: "lc".into(),
            from: "compose".into(),
            to: "dispatch".into(),
        },
        400,
    );
    s.apply_with_ts(
        &DashboardEvent::AgentOutput {
            agent_id: "w".into(),
            plan_id: "lc".into(),
            task_id: "ta".into(),
            attempt: 1,
            content: "code...".into(),
        },
        500,
    );
    eff(&mut s, "lc", "ta", "input_tokens", 1000.0, 600);
    s.apply_with_ts(
        &DashboardEvent::GateResult {
            plan_id: "lc".into(),
            task_id: "ta".into(),
            gate: "compile".into(),
            passed: true,
        },
        700,
    );
    s.apply_with_ts(
        &DashboardEvent::AgentCompleted {
            agent_id: "w".into(),
            plan_id: "lc".into(),
            task_id: "ta".into(),
            attempt: 1,
        },
        800,
    );
    s.apply_with_ts(
        &DashboardEvent::TaskCompleted {
            plan_id: "lc".into(),
            task_id: "ta".into(),
            outcome: "success".into(),
        },
        900,
    );
    s.apply_with_ts(
        &DashboardEvent::PlanCompleted {
            plan_id: "lc".into(),
            success: true,
        },
        1000,
    );

    assert_eq!(
        (
            s.stats.plans_active,
            s.stats.plans_completed,
            s.stats.tasks_completed
        ),
        (0, 1, 1)
    );
    assert_eq!((s.stats.agents_active, s.stats.gates_passed), (0, 1));
    assert!(!s.plans["lc"].active);
    assert_eq!(s.plans["lc"].phase, "completed");
    let w = &s.agents["w"];
    assert!(!w.active);
    assert_eq!(
        (w.input_tokens, w.last_event_at_ms, w.spawned_at_ms),
        (1000, 800, 300)
    );
}
