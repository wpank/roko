//! Integration test (audit checklist #30): connected-mode TUI + recorded
//! efficiency event → the F10 Efficiency surface renders the model row.
//!
//! Covers the wave-1 connected ingest path (#5, strategy A) end to end as far
//! as a test can, using only public API:
//!
//! 1. `TuiBridge::publish_event` pushes `EfficiencyTrendUpdated` and
//!    `CascadeRouterUpdated` into a `SharedStateHub` — the same variants the
//!    learning subscriber publishes during live runs.
//! 2. `App::new_connected_with_page` builds the connected TUI shell.
//! 3. `TuiState::update_from_dashboard_snapshot` — the exact call the app's
//!    snapshot loop makes — parses the pushed payloads and tails the local
//!    `.roko/learn/efficiency.jsonl` for per-event rows.
//! 4. `App::render_tabs_to_text` renders the real `App::draw` path headlessly.
//!
//! The recorded row uses the live codex shape: `backend = "codex-cli"`,
//! `model = "gpt-5.6-sol"`, non-zero tokens/cost, and `is_final_turn = false`
//! (the runner-v2 subscriber marks every in-flight turn non-final, audit #4).

use roko_cli::runner::tui_bridge::TuiBridge;
use roko_cli::state_hub::SharedStateHub;
use roko_cli::tui::{App, Tab};
use roko_core::dashboard_snapshot::{DashboardEvent, EfficiencyBucket};
use roko_learn::efficiency::AgentEfficiencyEvent;

const MODEL: &str = "gpt-5.6-sol";
const BACKEND: &str = "codex-cli";

/// A live-shape codex turn row: non-final, gate result not yet joined.
fn codex_turn_event(task_id: &str, timestamp: &str) -> AgentEfficiencyEvent {
    AgentEfficiencyEvent {
        agent_id: "agent-codex-1".into(),
        role: "implementer".into(),
        backend: BACKEND.into(),
        model: MODEL.into(),
        plan_id: "plan-1".into(),
        task_id: task_id.into(),
        attempt_id: format!("{task_id}-attempt-1"),
        input_tokens: 1_250,
        output_tokens: 340,
        cost_usd: 0.0312,
        wall_time_ms: 4_200,
        gate_passed: None,
        is_final_turn: false,
        timestamp: timestamp.into(),
        ..AgentEfficiencyEvent::default()
    }
}

/// Write one stamped row (schema discriminator from the `Serialize` impl)
/// and one legacy row with the discriminator stripped — both must load.
fn write_efficiency_jsonl(learn_dir: &std::path::Path) {
    std::fs::create_dir_all(learn_dir).expect("create learn dir");
    let stamped = serde_json::to_string(&codex_turn_event("task-1", "2026-09-01T12:00:00Z"))
        .expect("serialize stamped row");
    let mut legacy_value = serde_json::to_value(codex_turn_event("task-2", "2026-09-01T12:01:00Z"))
        .expect("serialize legacy row");
    legacy_value
        .as_object_mut()
        .expect("row is an object")
        .remove("schema");
    let legacy = serde_json::to_string(&legacy_value).expect("re-serialize legacy row");
    std::fs::write(
        learn_dir.join("efficiency.jsonl"),
        format!("{stamped}\n{legacy}\n"),
    )
    .expect("write efficiency.jsonl");
}

#[test]
fn connected_tui_renders_recorded_codex_efficiency_row() {
    let tmpdir = tempfile::tempdir().expect("tempdir");
    // Create .roko/ dir and a minimal roko.toml so the Welcome modal is
    // suppressed (it blocks tab rendering in headless mode).
    std::fs::create_dir_all(tmpdir.path().join(".roko")).expect("create .roko");
    std::fs::write(tmpdir.path().join("roko.toml"), "").expect("write roko.toml");
    write_efficiency_jsonl(&tmpdir.path().join(".roko").join("learn"));

    // -- 1. connected-mode pushes through the bridge ---------------------
    let hub = SharedStateHub::new_in_process();
    let bridge = TuiBridge::new(hub.sender());
    bridge.publish_event(DashboardEvent::EfficiencyTrendUpdated {
        buckets: vec![EfficiencyBucket {
            turns: 2,
            tokens_in: 42_000,
            tokens_out: 680,
            cost_usd_cents: 6,
            latency_ms_avg: 4_200.0,
            ..EfficiencyBucket::default()
        }],
    });
    bridge.publish_event(DashboardEvent::CascadeRouterUpdated {
        snapshot_json: serde_json::json!({
            "model_slugs": [MODEL],
            "confidence_stats": {MODEL: {"trials": 3, "successes": 2}},
        })
        .to_string(),
    });

    // -- 2. connected TUI shell ------------------------------------------
    let mut app = App::new_connected_with_page(tmpdir.path(), None, &hub);

    // -- 3. the snapshot tick's ingest call ------------------------------
    let snapshot = hub.snapshot().borrow().clone();
    app.tui_state.update_from_dashboard_snapshot(&snapshot);

    // Per-event rows tailed from `.roko/learn/efficiency.jsonl` — both the
    // stamped and the legacy (schema-less) row load, with identity intact.
    assert_eq!(app.tui_state.efficiency_events.len(), 2);
    for event in &app.tui_state.efficiency_events {
        assert_eq!(event.model, MODEL);
        assert_eq!(event.backend, BACKEND);
        assert!(!event.is_final_turn, "fixture must keep the live shape");
        assert!(event.input_tokens > 0 && event.cost_usd > 0.0);
    }
    // Event-derived summary preferred over the pushed-bucket approximation.
    assert_eq!(app.tui_state.efficiency_summary.event_count, 2);
    assert!((app.tui_state.efficiency_summary.total_cost_usd - 2.0 * 0.0312).abs() < 1e-9);
    // Pushed snapshot payloads parsed into the typed view structs; the push
    // wins over disk-derived trend data (42_000 cannot come from the file).
    assert_eq!(app.tui_state.efficiency_trend.len(), 1);
    assert_eq!(app.tui_state.efficiency_trend[0].tokens_in, 42_000);
    assert_eq!(
        app.tui_state.cascade_router.model_slugs,
        vec![MODEL.to_string()]
    );

    // -- 4. F10 → Efficiency renders the model row ------------------------
    app.tui_state.learning_sub_tab = 2; // SubView::LearningEfficiency
    let rendered = app.render_tabs_to_text(160, 50, &[Tab::Learning]);
    let learning_text = &rendered[0].1;
    // The table renders the slug through the canonical display shortener
    // (`display_model("gpt-5.6-sol")` → "5.6-sol"); derive the expected
    // label from the same helper so the assertion tracks display policy.
    let display_label = roko_cli::tui::display_utils::display_model(Some(MODEL));
    assert!(
        learning_text.contains(&display_label),
        "F10 Efficiency must render the model row ({display_label}), got:\n{learning_text}"
    );
    assert!(
        !learning_text.contains("No efficiency events recorded yet"),
        "F10 Efficiency must not be the empty state, got:\n{learning_text}"
    );
    assert!(
        !learning_text.contains("sonnet"),
        "codex rows must not be bucketed under the literal tier label, got:\n{learning_text}"
    );

    // -- 5. F7 → Cost/Model agrees (audit #4 consumer side) ---------------
    app.tui_state.inspect_sub_tab = 4; // SubView::CostByModel
    let rendered = app.render_tabs_to_text(160, 50, &[Tab::Inspect]);
    let inspect_text = &rendered[0].1;
    // This widget shows the full slug and the provider column.
    assert!(
        inspect_text.contains(MODEL),
        "Cost/Model must render the non-final-turn codex row, got:\n{inspect_text}"
    );
    assert!(
        inspect_text.contains(BACKEND),
        "Cost/Model must render the codex provider, got:\n{inspect_text}"
    );
    assert!(
        !inspect_text.contains("no efficiency data"),
        "Cost/Model must not render the empty state for live rows, got:\n{inspect_text}"
    );
    assert!(
        !inspect_text.contains("sonnet"),
        "codex rows must not be bucketed under the literal tier label, got:\n{inspect_text}"
    );
}
