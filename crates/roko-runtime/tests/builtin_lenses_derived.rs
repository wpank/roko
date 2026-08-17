//! Behavioral coverage for derived, usage, and collective built-in Lenses.

use std::collections::BTreeMap;

use roko_core::{
    Body, ContentHash, CostReportPayload, HdcFingerprint, HdcVector, Kind, LensScope,
    ObservableEvent, ObservableEventKind, Provenance, Signal, TelemetryObserve, TrendDirection,
    TrendPayload, Verdict,
};
use roko_runtime::builtin_lenses_derived::{
    ANOMALY_LENS_ALIASES, COLLECTIVE_INTELLIGENCE_LENS_ALIASES, DELIVERY_CONFIRMED_TAG,
    DELIVERY_DROPPED_TAG, PEER_OUTCOME_TAG, PEER_PREDICTION_TAG, TREND_LENS_ALIASES,
    USAGE_LENS_ALIASES,
};
use roko_runtime::{
    AnomalyLens, CollectiveIntelligenceLens, LensPayload, LensSignalEnvelope, TrendLens, UsageLens,
};
use serde_json::{Value, json};

fn params(entries: &[(&str, Value)]) -> BTreeMap<String, Value> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_owned(), value.clone()))
        .collect()
}

fn signal_event(source: &str, payload: LensPayload) -> ObservableEvent {
    ObservableEvent::SignalCreated(
        LensSignalEnvelope::new(source, payload)
            .to_signal()
            .expect("canonical Lens output"),
    )
}

fn cost_event(value: f64) -> ObservableEvent {
    signal_event(
        "cost-monitor",
        LensPayload::CostReport(CostReportPayload {
            target: "space:test".to_owned(),
            interval_ms: 60_000,
            total_usd: value,
            total_tokens: 0,
            input_tokens: 0,
            output_tokens: 0,
            model_breakdown: BTreeMap::new(),
            cumulative_usd: value,
            budget_remaining: None,
            vitality: None,
        }),
    )
}

fn at_timestamp(mut event: ObservableEvent, timestamp_ms: i64) -> ObservableEvent {
    let ObservableEvent::SignalCreated(signal) = &mut event else {
        panic!("test helper requires SignalCreated");
    };
    signal.created_at_ms = timestamp_ms;
    event
}

fn trend_event(value: f64) -> ObservableEvent {
    signal_event(
        "cost-trend",
        LensPayload::Trend(TrendPayload {
            source_lens: "cost-monitor".to_owned(),
            metric: "total_usd".to_owned(),
            window_ms: 600_000,
            slope: 0.0,
            ema: value,
            ema_previous: value,
            direction: TrendDirection::Stable,
            r_squared: 0.0,
            data_points: 5,
        }),
    )
}

fn decode_only(signals: Vec<Signal>) -> LensSignalEnvelope {
    assert_eq!(signals.len(), 1);
    LensSignalEnvelope::from_signal(&signals[0]).expect("canonical output")
}

#[test]
fn aliases_cover_canonical_and_short_block_names() {
    assert!(TREND_LENS_ALIASES.contains(&"roko:trend-lens"));
    assert!(ANOMALY_LENS_ALIASES.contains(&"anomaly-lens"));
    assert!(USAGE_LENS_ALIASES.contains(&"roko:usage-lens"));
    assert!(COLLECTIVE_INTELLIGENCE_LENS_ALIASES.contains(&"roko:c-factor-lens"));
    assert!(COLLECTIVE_INTELLIGENCE_LENS_ALIASES.contains(&"collective-intelligence-lens"));
}

#[tokio::test]
async fn trend_accepts_documented_toml_shape_and_emits_bounded_canonical_output() {
    let lens = TrendLens::new(
        "cost-trend",
        LensScope::Lens("cost-monitor".to_owned()),
        vec![ObservableEventKind::SignalLifecycle],
        &params(&[
            ("metric", json!("total_usd")),
            ("window", json!("600s")),
            ("max_points", json!(3)),
            ("ema_alpha", json!(0.5)),
        ]),
    )
    .expect("documented TrendLens params construct");

    assert!(
        lens.observe(&cost_event(1.0))
            .await
            .expect("first sample succeeds")
            .is_empty()
    );
    assert!(
        lens.observe(&cost_event(2.0))
            .await
            .expect("second sample succeeds")
            .is_empty()
    );
    let envelope = decode_only(
        lens.observe(&cost_event(3.0))
            .await
            .expect("third sample succeeds"),
    );
    assert_eq!(envelope.source_lens, "cost-trend");
    let LensPayload::Trend(payload) = envelope.payload else {
        panic!("expected Trend payload");
    };
    assert_eq!(payload.source_lens, "cost-monitor");
    assert_eq!(payload.metric, "total_usd");
    assert_eq!(payload.window_ms, 600_000);
    assert_eq!(payload.data_points, 3);
    assert!((payload.slope - 1.0).abs() < 1e-12);
    assert!((payload.r_squared - 1.0).abs() < 1e-12);
    assert_eq!(payload.direction, TrendDirection::Rising);
    assert!((payload.ema - 2.25).abs() < 1e-12);
    assert!((payload.ema_previous - 1.5).abs() < 1e-12);

    let envelope = decode_only(
        lens.observe(&cost_event(4.0))
            .await
            .expect("fourth sample succeeds"),
    );
    let LensPayload::Trend(payload) = envelope.payload else {
        panic!("expected Trend payload");
    };
    assert_eq!(payload.data_points, 3, "rolling state stays bounded");
    assert!((payload.slope - 1.0).abs() < 1e-12);
}

#[tokio::test]
async fn trend_enforces_its_time_window_and_ordered_chain_input() {
    let lens = TrendLens::new(
        "trend",
        LensScope::Lens("cost-monitor".to_owned()),
        vec![ObservableEventKind::SignalLifecycle],
        &params(&[
            ("metric", json!("total_usd")),
            ("window_ms", json!(100)),
            ("min_data_points", json!(2)),
        ]),
    )
    .expect("valid time-windowed TrendLens");
    assert!(
        lens.observe(&at_timestamp(cost_event(1.0), 0))
            .await
            .expect("first timestamp succeeds")
            .is_empty()
    );
    assert!(
        lens.observe(&at_timestamp(cost_event(2.0), 200))
            .await
            .expect("stale point is evicted")
            .is_empty()
    );
    let envelope = decode_only(
        lens.observe(&at_timestamp(cost_event(3.0), 250))
            .await
            .expect("second in-window point succeeds"),
    );
    let LensPayload::Trend(payload) = envelope.payload else {
        panic!("expected Trend payload");
    };
    assert_eq!(payload.data_points, 2);
    assert!(
        lens.observe(&at_timestamp(cost_event(4.0), 249))
            .await
            .is_err(),
        "out-of-order chained evidence fails closed"
    );
}

#[tokio::test]
async fn derived_lenses_ignore_unrelated_signals_and_fail_closed_on_malformed_canonical_data() {
    let lens = TrendLens::new(
        "trend",
        LensScope::Lens("cost-monitor".to_owned()),
        vec![ObservableEventKind::SignalLifecycle],
        &params(&[("metric", json!("total_usd"))]),
    )
    .expect("valid TrendLens");
    let unrelated = ObservableEvent::SignalCreated(
        Signal::builder(Kind::Metric)
            .body(Body::text("not a Lens envelope"))
            .build(),
    );
    assert!(
        lens.observe(&unrelated)
            .await
            .expect("unrelated Signal is ignored")
            .is_empty()
    );

    let malformed = ObservableEvent::SignalCreated(
        Signal::builder(Kind::Custom(
            roko_runtime::telemetry_projection_aggregator::LENS_SIGNAL_KIND.to_owned(),
        ))
        .body(Body::text("not JSON"))
        .build(),
    );
    assert!(lens.observe(&malformed).await.is_err());
}

#[test]
fn derived_constructor_validation_rejects_impossible_or_unknown_config() {
    let documented_anomaly = AnomalyLens::new(
        "cost-anomaly",
        LensScope::Lens("cost-trend".to_owned()),
        vec![ObservableEventKind::SignalLifecycle],
        &params(&[("sigma_moderate", json!(3.0))]),
    );
    assert!(documented_anomaly.is_ok());

    assert!(
        TrendLens::new(
            "trend",
            LensScope::Global,
            vec![ObservableEventKind::SignalLifecycle],
            &params(&[("metric", json!("total_usd"))]),
        )
        .is_err()
    );
    assert!(
        TrendLens::new(
            "trend",
            LensScope::Lens(String::new()),
            vec![ObservableEventKind::SignalLifecycle],
            &params(&[("metric", json!("total_usd"))]),
        )
        .is_err()
    );
    assert!(
        TrendLens::new(
            "trend",
            LensScope::Lens("source".to_owned()),
            vec![ObservableEventKind::SignalLifecycle],
            &params(&[("metric", json!("x")), ("mystery", json!(1))]),
        )
        .is_err()
    );
}

#[tokio::test]
async fn anomaly_uses_prior_baseline_and_classifies_zero_variance_outlier() {
    let lens = AnomalyLens::new(
        "cost-anomaly",
        LensScope::Lens("cost-trend".to_owned()),
        vec![ObservableEventKind::SignalLifecycle],
        &params(&[("sigma_moderate", json!(3.0))]),
    )
    .expect("valid AnomalyLens");
    for _ in 0..5 {
        assert!(
            lens.observe(&trend_event(10.0))
                .await
                .expect("baseline sample succeeds")
                .is_empty()
        );
    }
    let envelope = decode_only(
        lens.observe(&trend_event(30.0))
            .await
            .expect("outlier sample succeeds"),
    );
    let LensPayload::Anomaly(payload) = envelope.payload else {
        panic!("expected Anomaly payload");
    };
    assert_eq!(payload.source_lens, "cost-trend");
    assert_eq!(payload.metric, "ema");
    assert!((payload.observed_value - 30.0).abs() < f64::EPSILON);
    assert!((payload.expected_value - 10.0).abs() < f64::EPSILON);
    assert!((payload.deviation - 20.0).abs() < f64::EPSILON);
    assert_eq!(payload.direction, roko_core::AnomalyDirection::Above);
    assert_eq!(payload.severity, roko_core::AnomalyLevel::Critical);
}

#[tokio::test]
async fn usage_counts_observed_runs_and_deduplicates_graph_rollups() {
    let lens = UsageLens::new(
        "usage",
        LensScope::Space("dev".to_owned()),
        vec![
            ObservableEventKind::CellLifecycle,
            ObservableEventKind::GraphLifecycle,
            ObservableEventKind::TriggerLifecycle,
        ],
        &params(&[("interval", json!("5m"))]),
    )
    .expect("valid UsageLens");

    for (block, duration_ms, cost_usd) in [("a", 10, 1.0), ("b", 20, 2.0)] {
        lens.observe(&ObservableEvent::CellCompleted {
            block: block.to_owned(),
            run: "run-1".to_owned(),
            duration_ms,
            cost_usd,
        })
        .await
        .expect("cell usage observation succeeds");
    }
    lens.observe(&ObservableEvent::GraphCompleted {
        graph: "g".to_owned(),
        run: "run-1".to_owned(),
        duration_ms: 999,
        cost_usd: 99.0,
    })
    .await
    .expect("rolled-up graph observation succeeds");
    lens.observe(&ObservableEvent::TriggerFired {
        trigger: "nightly".to_owned(),
        graph: "g".to_owned(),
    })
    .await
    .expect("trigger observation succeeds");
    let envelope = decode_only(
        lens.observe(&ObservableEvent::GraphCompleted {
            graph: "g".to_owned(),
            run: "run-2".to_owned(),
            duration_ms: 50,
            cost_usd: 5.0,
        })
        .await
        .expect("standalone graph observation succeeds"),
    );
    let LensPayload::Usage(payload) = envelope.payload else {
        panic!("expected Usage payload");
    };
    assert_eq!(payload.target, "space:dev");
    assert_eq!(payload.interval_ms, 300_000);
    assert_eq!(payload.cell_runs, 2);
    assert_eq!(payload.graph_runs, 2);
    assert_eq!(payload.trigger_fires, 1);
    assert_eq!(payload.total_duration_ms, 80);
    assert!((payload.total_cost_usd - 8.0).abs() < f64::EPSILON);
}

#[test]
fn usage_and_collective_reject_scopes_or_filters_that_cannot_supply_evidence() {
    assert!(
        UsageLens::new(
            "usage",
            LensScope::Cell("x".to_owned()),
            vec![ObservableEventKind::All],
            &params(&[]),
        )
        .is_err()
    );
    assert!(
        CollectiveIntelligenceLens::new(
            "c",
            LensScope::Space("dev".to_owned()),
            vec![ObservableEventKind::AgentLifecycle],
            &params(&[]),
        )
        .is_err()
    );
}

fn collective_lens() -> CollectiveIntelligenceLens {
    CollectiveIntelligenceLens::new(
        "collective",
        LensScope::Space("dev".to_owned()),
        vec![
            ObservableEventKind::AgentLifecycle,
            ObservableEventKind::SignalLifecycle,
            ObservableEventKind::MemoryLifecycle,
        ],
        &params(&[]),
    )
    .expect("valid collective Lens")
}

fn agent_signal(
    author: &str,
    seed: &[u8],
    tags: &[(&str, &str)],
    lineage: &[ContentHash],
) -> Signal {
    let mut builder = Signal::builder(Kind::Metric)
        .body(Body::text(format!("evidence from {author}")))
        .provenance(Provenance::agent(author))
        .fingerprint(HdcFingerprint::new(HdcVector::from_seed(seed), 1))
        .lineage(lineage.iter().copied());
    for (key, value) in tags {
        builder = builder.tag(*key, *value);
    }
    builder.build()
}

#[tokio::test]
async fn collective_lens_waits_for_complete_observable_evidence_then_emits_c_factor() {
    let lens = collective_lens();
    for (agent, vitality) in [("a", 0.8), ("b", 0.6)] {
        assert!(
            lens.observe(&ObservableEvent::AgentTick {
                agent: agent.to_owned(),
                regime: "steady".to_owned(),
                prediction_error: 0.1,
                vitality,
            })
            .await
            .expect("AgentTick succeeds")
            .is_empty()
        );
    }

    let first = agent_signal(
        "a",
        b"agent-a",
        &[
            (PEER_PREDICTION_TAG, "0.8"),
            (PEER_OUTCOME_TAG, "1.0"),
            (DELIVERY_CONFIRMED_TAG, "3"),
        ],
        &[],
    );
    assert!(
        lens.observe(&ObservableEvent::SignalCreated(first))
            .await
            .expect("first Signal evidence succeeds")
            .is_empty()
    );
    let second = agent_signal("b", b"agent-b", &[(DELIVERY_DROPPED_TAG, "1")], &[]);
    assert!(
        lens.observe(&ObservableEvent::SignalCreated(second))
            .await
            .expect("second Signal evidence succeeds")
            .is_empty()
    );
    let citing = agent_signal("a", b"agent-a-citation", &[], &[ContentHash::of(b"parent")]);
    let citing_id = citing.id.to_hex();
    assert!(
        lens.observe(&ObservableEvent::SignalCreated(citing))
            .await
            .expect("citation evidence succeeds")
            .is_empty(),
        "unverified citations must not be reported as reciprocity"
    );

    let envelope = decode_only(
        lens.observe(&ObservableEvent::SignalVerified(
            citing_id,
            Verdict::pass("quality"),
        ))
        .await
        .expect("citation verification succeeds"),
    );
    let LensPayload::CFactor(payload) = envelope.payload else {
        panic!("expected CFactor payload");
    };
    assert_eq!(payload.space, "dev");
    assert_eq!(payload.agent_count, 2);
    assert_eq!(payload.active_agents, 2);
    assert_eq!(payload.knowledge_flow_edges, 1);
    assert!((payload.citation_reciprocity - 1.0).abs() < f64::EPSILON);
    assert!((payload.peer_prediction_accuracy - 0.96).abs() < 1e-12);
    assert!((payload.avg_agent_vitality - 0.7).abs() < 1e-12);
    assert!((0.0..=1.0).contains(&payload.turn_taking_entropy));
    assert!((0.0..=1.0).contains(&payload.hdc_diversity));
    assert!((0.0..=1.0).contains(&payload.c_factor));
}

#[tokio::test]
async fn collective_lens_fails_closed_on_partial_or_malformed_explicit_evidence() {
    let lens = collective_lens();
    assert!(
        lens.observe(&ObservableEvent::AgentTick {
            agent: "a".to_owned(),
            regime: "steady".to_owned(),
            prediction_error: 0.1,
            vitality: 1.1,
        })
        .await
        .is_err()
    );

    let lens = collective_lens();
    let partial = agent_signal("a", b"partial", &[(PEER_PREDICTION_TAG, "0.5")], &[]);
    assert!(
        lens.observe(&ObservableEvent::SignalCreated(partial))
            .await
            .is_err()
    );

    let lens = collective_lens();
    let invalid = agent_signal(
        "a",
        b"invalid",
        &[
            (PEER_PREDICTION_TAG, "not-a-number"),
            (PEER_OUTCOME_TAG, "0.5"),
        ],
        &[],
    );
    assert!(
        lens.observe(&ObservableEvent::SignalCreated(invalid))
            .await
            .is_err()
    );
}

#[test]
fn derived_lenses_are_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<TrendLens>();
    assert_send_sync::<AnomalyLens>();
    assert_send_sync::<UsageLens>();
    assert_send_sync::<CollectiveIntelligenceLens>();
}
