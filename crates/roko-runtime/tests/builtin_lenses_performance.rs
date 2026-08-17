//! Focused contracts for the performance built-in event Lenses.

use std::collections::BTreeMap;

use roko_core::{
    LensScope, ObservableEvent, ObservableEventKind, TelemetryObserve, TestCount, Verdict,
};
use roko_runtime::{EfficiencyLens, LatencyLens, LensPayload, LensSignalEnvelope, QualityLens};
use serde_json::{Value, json};

fn params(values: &[(&str, Value)]) -> BTreeMap<String, Value> {
    values
        .iter()
        .map(|(key, value)| ((*key).to_string(), value.clone()))
        .collect()
}

fn empty_params() -> BTreeMap<String, Value> {
    BTreeMap::new()
}

fn latency_lens(params: BTreeMap<String, Value>) -> LatencyLens {
    LatencyLens::new(
        "latency-main",
        LensScope::Graph("ci".into()),
        vec![
            ObservableEventKind::CellLifecycle,
            ObservableEventKind::GraphLifecycle,
        ],
        params,
    )
    .unwrap()
}

fn quality_lens(params: BTreeMap<String, Value>) -> QualityLens {
    QualityLens::new(
        "quality-main",
        LensScope::Graph("ci".into()),
        vec![
            ObservableEventKind::VerifyLifecycle,
            ObservableEventKind::SignalLifecycle,
        ],
        params,
    )
    .unwrap()
}

fn efficiency_lens(params: BTreeMap<String, Value>) -> EfficiencyLens {
    EfficiencyLens::new(
        "efficiency-main",
        LensScope::Agent("builder".into()),
        vec![
            ObservableEventKind::CellLifecycle,
            ObservableEventKind::AgentLifecycle,
        ],
        params,
    )
    .unwrap()
}

async fn payload(lens: &dyn TelemetryObserve, event: ObservableEvent) -> LensSignalEnvelope {
    let signals = lens.observe(&event).await.unwrap();
    assert_eq!(signals.len(), 1);
    LensSignalEnvelope::from_signal(&signals[0]).unwrap()
}

fn cell_completed(block: &str, duration_ms: u64, cost_usd: f64) -> ObservableEvent {
    ObservableEvent::CellCompleted {
        block: block.into(),
        run: "run-1".into(),
        duration_ms,
        cost_usd,
    }
}

#[tokio::test]
async fn latency_computes_nearest_rank_percentiles_and_evicts_old_samples() {
    let lens = latency_lens(params(&[
        ("window_size", json!(4)),
        ("max_targets", json!(2)),
    ]));
    let mut last = None;
    for duration in [10, 20, 30, 40] {
        last = Some(payload(&lens, cell_completed("compile", duration, 0.0)).await);
    }
    let envelope = last.unwrap();
    assert_eq!(envelope.source_lens, "latency-main");
    let LensPayload::Latency(measurement) = envelope.payload else {
        panic!("expected latency payload");
    };
    assert_eq!(measurement.target, "cell:compile");
    assert_eq!(measurement.interval_ms, 0);
    assert_eq!(measurement.count, 4);
    assert_eq!(measurement.p50_ms, 20);
    assert_eq!(measurement.p95_ms, 40);
    assert_eq!(measurement.p99_ms, 40);
    assert_eq!(measurement.mean_ms, 25);

    let envelope = payload(&lens, cell_completed("compile", 100, 0.0)).await;
    let LensPayload::Latency(measurement) = envelope.payload else {
        panic!("expected latency payload");
    };
    assert_eq!(measurement.count, 4);
    assert_eq!(measurement.p50_ms, 30);
    assert_eq!(measurement.p95_ms, 100);
    assert_eq!(measurement.mean_ms, 47);
}

#[tokio::test]
async fn latency_keeps_event_targets_distinct_and_enforces_cardinality() {
    let lens = latency_lens(params(&[
        ("window_size", json!(2)),
        ("max_targets", json!(2)),
    ]));
    let cell = payload(&lens, cell_completed("compile", 0, 0.0)).await;
    assert!(matches!(cell.payload, LensPayload::Latency(_)));

    let graph = payload(
        &lens,
        ObservableEvent::GraphCompleted {
            graph: "ci".into(),
            run: "run-1".into(),
            duration_ms: 50,
            cost_usd: 0.0,
        },
    )
    .await;
    let LensPayload::Latency(graph) = graph.payload else {
        panic!("expected latency payload");
    };
    assert_eq!(graph.target, "graph:ci");

    let error = lens
        .observe(&cell_completed("test", 5, 0.0))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("exceeded max_targets (2)"));
}

#[test]
fn latency_constructor_rejects_bad_filters_and_params() {
    let invalid = |observes, params| {
        LatencyLens::new("latency", LensScope::Global, observes, params)
            .err()
            .expect("configuration should fail")
            .to_string()
    };
    assert!(
        invalid(vec![ObservableEventKind::VerifyLifecycle], empty_params())
            .contains("unsupported event-family")
    );
    assert!(
        invalid(
            vec![ObservableEventKind::CellLifecycle],
            params(&[("window_size", json!(0))])
        )
        .contains("1..=100000")
    );
    assert!(
        invalid(
            vec![ObservableEventKind::CellLifecycle],
            params(&[("mystery", json!(1))])
        )
        .contains("unknown param: mystery")
    );
    assert!(
        invalid(
            vec![ObservableEventKind::CellLifecycle],
            params(&[("window_size", json!(100_000)), ("max_targets", json!(6))])
        )
        .contains("must not exceed 500000")
    );
}

#[tokio::test]
async fn quality_tracks_terminal_pass_rate_reward_rungs_and_pre_vetoes() {
    let lens = quality_lens(params(&[("pass_rate_warn", json!(0.7))]));
    assert_eq!(lens.pass_rate_warn(), Some(0.7));

    let pre = Verdict::fail("policy", "veto").with_duration(4);
    payload(
        &lens,
        ObservableEvent::VerifyPreResult {
            block: "compile".into(),
            verdict: pre,
            evidence: vec!["policy evidence".into()],
        },
    )
    .await;
    let pass = Verdict::pass("compile").with_duration(10);
    payload(
        &lens,
        ObservableEvent::VerifyPostResult {
            block: "compile".into(),
            verdict: pass,
            reward: 0.8,
            evidence: Vec::new(),
        },
    )
    .await;
    let fail = Verdict::fail("test", "failed")
        .with_duration(20)
        .with_test_count(TestCount::new(9, 1, 0));
    payload(
        &lens,
        ObservableEvent::VerifyPostResult {
            block: "test".into(),
            verdict: fail,
            reward: 0.2,
            evidence: vec!["not interpreted as hard criteria".into()],
        },
    )
    .await;
    let scored = Verdict::pass("judge").with_score(0.6);
    let envelope = payload(
        &lens,
        ObservableEvent::SignalVerified("signal-1".into(), scored),
    )
    .await;

    assert_eq!(envelope.source_lens, "quality-main");
    let LensPayload::Quality(measurement) = envelope.payload else {
        panic!("expected quality payload");
    };
    assert_eq!(measurement.target, "graph:ci");
    assert_eq!(measurement.interval_ms, 0);
    assert_eq!(measurement.total_verifications, 3);
    assert_eq!(measurement.pre_verify_vetoes, 1);
    assert_eq!(measurement.post_verify_passed, 2);
    assert_eq!(measurement.post_verify_failed, 1);
    assert!((measurement.pass_rate - 2.0 / 3.0).abs() < f64::EPSILON);
    // SignalVerified adapts the portable f32 Verdict.score into f64.
    assert!((measurement.avg_reward - (0.8 + 0.2 + 0.6) / 3.0).abs() < 1e-7);
    assert_eq!(measurement.hard_criteria_failures, 0);
    assert_eq!(measurement.rung_breakdown["compile"].passed, 1);
    assert_eq!(measurement.rung_breakdown["test"].failed, 1);
    assert_eq!(measurement.rung_breakdown["judge"].passed, 1);
}

#[tokio::test]
async fn quality_window_and_skipped_verdicts_are_neutral() {
    let lens = quality_lens(params(&[("window_size", json!(2))]));
    let skipped = Verdict::skip("compile", "not wired");
    assert!(
        lens.observe(&ObservableEvent::VerifyPostResult {
            block: "compile".into(),
            verdict: skipped,
            reward: 1.0,
            evidence: Vec::new(),
        })
        .await
        .unwrap()
        .is_empty()
    );

    for (gate, passed) in [("one", true), ("two", false), ("three", true)] {
        let verdict = if passed {
            Verdict::pass(gate)
        } else {
            Verdict::fail(gate, "failed")
        };
        let _ = payload(
            &lens,
            ObservableEvent::VerifyPostResult {
                block: gate.into(),
                verdict,
                reward: f64::from(passed),
                evidence: Vec::new(),
            },
        )
        .await;
    }
    let envelope = payload(
        &lens,
        ObservableEvent::VerifyPreResult {
            block: "policy".into(),
            verdict: Verdict::fail("policy", "veto"),
            evidence: Vec::new(),
        },
    )
    .await;
    let LensPayload::Quality(measurement) = envelope.payload else {
        panic!("expected quality payload");
    };
    // Event-count window now contains terminal "three" and the pre veto.
    assert_eq!(measurement.total_verifications, 1);
    assert_eq!(measurement.post_verify_passed, 1);
    assert_eq!(measurement.post_verify_failed, 0);
    assert_eq!(measurement.pre_verify_vetoes, 1);
}

#[test]
fn quality_constructor_and_event_validation_fail_closed() {
    let out_of_range = QualityLens::new(
        "quality",
        LensScope::Global,
        vec![ObservableEventKind::VerifyLifecycle],
        params(&[("pass_rate_warn", json!(1.1))]),
    )
    .err()
    .unwrap();
    assert!(out_of_range.to_string().contains("0..=1"));
    let unknown = QualityLens::new(
        "quality",
        LensScope::Global,
        vec![ObservableEventKind::VerifyLifecycle],
        params(&[("alert", json!(true))]),
    )
    .err()
    .unwrap();
    assert!(unknown.to_string().contains("unknown param: alert"));
}

#[tokio::test]
async fn quality_rejects_non_finite_reward_and_blank_gate() {
    let lens = quality_lens(empty_params());
    let error = lens
        .observe(&ObservableEvent::VerifyPostResult {
            block: "compile".into(),
            verdict: Verdict::pass("compile"),
            reward: f64::NAN,
            evidence: Vec::new(),
        })
        .await
        .unwrap_err();
    assert!(error.to_string().contains("reward is not finite"));
    let error = lens
        .observe(&ObservableEvent::SignalVerified(
            "signal-1".into(),
            Verdict::pass(" "),
        ))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("gate is empty"));
}

#[tokio::test]
async fn efficiency_tracks_completion_cost_prediction_vitality_and_phase() {
    let lens = efficiency_lens(params(&[("window_size", json!(4))]));
    payload(&lens, cell_completed("compile", 10, 1.0)).await;
    payload(&lens, cell_completed("test", 20, 3.0)).await;
    payload(
        &lens,
        ObservableEvent::AgentTick {
            agent: "builder".into(),
            regime: "steady".into(),
            prediction_error: 0.2,
            vitality: 0.8,
        },
    )
    .await;
    payload(
        &lens,
        ObservableEvent::AgentPhaseChange {
            agent: "builder".into(),
            old: "steady".into(),
            new_phase: "thriving".into(),
        },
    )
    .await;
    let envelope = payload(
        &lens,
        ObservableEvent::CellCalibrationReceived {
            block: "compile".into(),
            error: 0.4,
        },
    )
    .await;

    assert_eq!(envelope.source_lens, "efficiency-main");
    let LensPayload::Efficiency(measurement) = envelope.payload else {
        panic!("expected efficiency payload");
    };
    assert_eq!(measurement.agent, "builder");
    assert_eq!(measurement.interval_ms, 0);
    assert_eq!(measurement.tasks_completed, 2);
    assert_eq!(measurement.usd_per_task, 2.0);
    assert!((measurement.avg_prediction_error - 0.3).abs() < 1e-12);
    assert_eq!(measurement.vitality, 0.8);
    assert_eq!(measurement.vitality_phase, "thriving");
    assert_eq!(measurement.tokens_per_task, 0.0);
    assert_eq!(measurement.quality_per_usd, 0.0);
    assert_eq!(measurement.t0_hit_rate, 0.0);
    assert_eq!(measurement.t1_hit_rate, 0.0);
    assert_eq!(measurement.t2_hit_rate, 0.0);
}

#[tokio::test]
async fn efficiency_window_wildcard_attribution_and_agent_limit_are_explicit() {
    let lens = efficiency_lens(params(&[
        ("window_size", json!(2)),
        ("max_agents", json!(1)),
    ]));
    for cost in [1.0, 3.0, 5.0] {
        let _ = payload(&lens, cell_completed("work", 1, cost)).await;
    }
    let envelope = payload(
        &lens,
        ObservableEvent::AgentBudgetUpdate {
            agent: "builder".into(),
            spent_usd: 9.0,
            remaining_usd: 1.0,
            vitality: 0.5,
        },
    )
    .await;
    let LensPayload::Efficiency(measurement) = envelope.payload else {
        panic!("expected efficiency payload");
    };
    assert_eq!(measurement.tasks_completed, 2);
    assert_eq!(measurement.usd_per_task, 4.0);

    let error = lens
        .observe(&ObservableEvent::AgentTick {
            agent: "reviewer".into(),
            regime: "steady".into(),
            prediction_error: 0.1,
            vitality: 0.9,
        })
        .await
        .unwrap_err();
    assert!(error.to_string().contains("exceeded max_agents (1)"));

    let wildcard = EfficiencyLens::new(
        "wildcard",
        LensScope::Agent(String::new()),
        vec![
            ObservableEventKind::CellLifecycle,
            ObservableEventKind::AgentLifecycle,
        ],
        empty_params(),
    )
    .unwrap();
    assert!(
        wildcard
            .observe(&cell_completed("work", 1, 2.0))
            .await
            .unwrap()
            .is_empty()
    );
    let envelope = payload(
        &wildcard,
        ObservableEvent::AgentTick {
            agent: "actual-agent".into(),
            regime: "steady".into(),
            prediction_error: 0.1,
            vitality: 0.9,
        },
    )
    .await;
    let LensPayload::Efficiency(measurement) = envelope.payload else {
        panic!("expected efficiency payload");
    };
    assert_eq!(measurement.agent, "actual-agent");
}

#[tokio::test]
async fn efficiency_rejects_invalid_event_measurements() {
    let lens = efficiency_lens(empty_params());
    let error = lens
        .observe(&cell_completed("work", 1, f64::NAN))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("finite and non-negative"));
    let error = lens
        .observe(&ObservableEvent::AgentTick {
            agent: "builder".into(),
            regime: "steady".into(),
            prediction_error: -0.1,
            vitality: 0.5,
        })
        .await
        .unwrap_err();
    assert!(error.to_string().contains("prediction_error"));
    let error = lens
        .observe(&ObservableEvent::AgentBudgetUpdate {
            agent: "builder".into(),
            spent_usd: 1.0,
            remaining_usd: 0.0,
            vitality: 1.1,
        })
        .await
        .unwrap_err();
    assert!(error.to_string().contains("vitality"));
}

#[test]
fn constructors_reject_duplicate_all_and_non_table_params() {
    let duplicate = EfficiencyLens::new(
        "efficiency",
        LensScope::Agent("a".into()),
        vec![
            ObservableEventKind::AgentLifecycle,
            ObservableEventKind::AgentLifecycle,
        ],
        empty_params(),
    )
    .err()
    .unwrap();
    assert!(duplicate.to_string().contains("duplicates"));
    let all_plus_specific = EfficiencyLens::new(
        "efficiency",
        LensScope::Agent("a".into()),
        vec![
            ObservableEventKind::All,
            ObservableEventKind::AgentLifecycle,
        ],
        empty_params(),
    )
    .err()
    .unwrap();
    assert!(all_plus_specific.to_string().contains("declared alone"));
    let non_table = EfficiencyLens::new(
        "efficiency",
        LensScope::Agent("a".into()),
        vec![ObservableEventKind::AgentLifecycle],
        json!([1, 2]),
    )
    .err()
    .unwrap();
    assert!(non_table.to_string().contains("must be a TOML table"));

    let zero_agents = EfficiencyLens::new(
        "efficiency",
        LensScope::Agent("a".into()),
        vec![ObservableEventKind::AgentLifecycle],
        params(&[("max_agents", json!(0))]),
    )
    .err()
    .unwrap();
    assert!(zero_agents.to_string().contains("1..=4096"));
    let unknown = EfficiencyLens::new(
        "efficiency",
        LensScope::Agent("a".into()),
        vec![ObservableEventKind::AgentLifecycle],
        params(&[("cost_model", json!("estimated"))]),
    )
    .err()
    .unwrap();
    assert!(unknown.to_string().contains("unknown param: cost_model"));
}
