//! End-to-end wire and reduction tests for Lens projection materialization.

use std::collections::BTreeMap;

use roko_core::telemetry_observe::{
    CFactorPayload, CostReportPayload, EfficiencyPayload, LatencyPayload, PassFailCounts,
    QualityPayload,
};
use roko_core::{
    AlertLevel, Body, BudgetAlertPayload, DriftPayload, ErrorPayload, Kind, Signal, TrendDirection,
    TrendPayload, UsagePayload,
};
use roko_runtime::StateHub;
use roko_runtime::telemetry_projection_aggregator::{
    ACTIVE_TASKS, AGENT_VITALITY, C_FACTOR, COHORT_HEALTH, COST_METER, GATE_PIPELINE,
    KNOWLEDGE_HEALTH, LENS_SIGNAL_KIND, LENS_SIGNAL_SCHEMA_VERSION, LENS_SIGNAL_SOURCE_TAG,
    LENS_SIGNAL_TOPIC_TAG, LensPayload, LensSignalEnvelope, TelemetryProjectionAggregator,
    TelemetryProjectionError,
};

fn lens_signal(source: &str, payload: LensPayload) -> Signal {
    LensSignalEnvelope::new(source, payload)
        .to_signal()
        .expect("valid Lens Signal")
}

fn update<'a>(
    updates: &'a [roko_runtime::ProjectionUpdate],
    id: &str,
) -> &'a roko_runtime::ProjectionUpdate {
    updates
        .iter()
        .find(|update| update.projection_id == id)
        .expect("projection update")
}

#[test]
fn envelope_round_trips_with_stable_kind_topic_and_routing_tags() {
    let envelope = LensSignalEnvelope::new(
        "cost-lens:build",
        LensPayload::CostReport(CostReportPayload {
            target: "graph:build".into(),
            interval_ms: 30_000,
            total_usd: 0.5,
            total_tokens: 1_000,
            input_tokens: 800,
            output_tokens: 200,
            model_breakdown: BTreeMap::from([("sonnet".into(), 0.5)]),
            cumulative_usd: 2.5,
            budget_remaining: Some(7.5),
            vitality: None,
        }),
    );
    let signal = envelope.to_signal().expect("encode envelope");

    assert_eq!(signal.kind.as_str(), LENS_SIGNAL_KIND);
    assert_eq!(
        signal.tag(LENS_SIGNAL_TOPIC_TAG),
        Some("telemetry.lens.cost_report.v1")
    );
    assert_eq!(signal.tag(LENS_SIGNAL_SOURCE_TAG), Some("cost-lens:build"));
    assert_eq!(
        LensSignalEnvelope::from_signal(&signal).expect("decode envelope"),
        envelope
    );

    let body: serde_json::Value = signal.body.as_json().expect("JSON body");
    assert_eq!(body["schema_version"], LENS_SIGNAL_SCHEMA_VERSION);
    assert_eq!(body["payload_type"], "cost_report");
    assert_eq!(body["payload"]["interval_ms"], 30_000);
}

#[test]
fn malformed_unknown_and_mismatched_envelopes_are_rejected_without_mutation() {
    let mut aggregator = TelemetryProjectionAggregator::new();
    let unrelated = Signal::builder(Kind::Metric).body(Body::empty()).build();
    assert!(matches!(
        aggregator.consume(&unrelated),
        Err(TelemetryProjectionError::UnexpectedSignalKind(_))
    ));

    let malformed = Signal::builder(Kind::Custom(LENS_SIGNAL_KIND.into()))
        .body(Body::text("not json"))
        .tag(LENS_SIGNAL_TOPIC_TAG, "telemetry.lens.cost_report.v1")
        .tag(LENS_SIGNAL_SOURCE_TAG, "cost")
        .build();
    assert!(matches!(
        aggregator.consume(&malformed),
        Err(TelemetryProjectionError::MalformedEnvelope(_))
    ));

    let unknown = Signal::builder(Kind::Custom(LENS_SIGNAL_KIND.into()))
        .body(Body::Json(serde_json::json!({
            "schema_version": 1,
            "topic": "telemetry.lens.future.v1",
            "source_lens": "future",
            "payload_type": "future",
            "payload": {}
        })))
        .tag(LENS_SIGNAL_TOPIC_TAG, "telemetry.lens.future.v1")
        .tag(LENS_SIGNAL_SOURCE_TAG, "future")
        .build();
    assert!(matches!(
        aggregator.consume(&unknown),
        Err(TelemetryProjectionError::MalformedEnvelope(_))
    ));

    let mut mismatch = LensSignalEnvelope::new(
        "latency",
        LensPayload::Latency(LatencyPayload {
            target: "graph:build".into(),
            interval_ms: 1_000,
            count: 1,
            p50_ms: 10,
            p95_ms: 10,
            p99_ms: 10,
            mean_ms: 10,
        }),
    );
    mismatch.topic = "telemetry.lens.quality.v1".into();
    assert!(matches!(
        aggregator.apply_envelope(mismatch),
        Err(TelemetryProjectionError::TopicMismatch { .. })
    ));

    assert_eq!(aggregator.state(), &Default::default());
}

#[test]
fn cost_and_budget_lenses_merge_without_double_counting_and_publish_to_statehub() {
    let mut aggregator = TelemetryProjectionAggregator::new();
    let cost_updates = aggregator
        .consume(&lens_signal(
            "cost-lens",
            LensPayload::CostReport(CostReportPayload {
                target: "agent:alice".into(),
                interval_ms: 30_000,
                total_usd: 1.0,
                total_tokens: 100,
                input_tokens: 80,
                output_tokens: 20,
                model_breakdown: BTreeMap::from([("sonnet".into(), 1.0)]),
                cumulative_usd: 4.0,
                budget_remaining: Some(6.0),
                vitality: None,
            }),
        ))
        .expect("apply cost payload");
    assert_eq!(cost_updates.len(), 2);
    assert_eq!(update(&cost_updates, COST_METER).data["total_usd"], 4.0);

    let budget_updates = aggregator
        .consume(&lens_signal(
            "budget-lens",
            LensPayload::BudgetAlert(BudgetAlertPayload {
                target: "agent:alice".into(),
                budget_total: 10.0,
                budget_spent: 5.0,
                budget_remaining: 5.0,
                vitality: 0.6,
                vitality_phase: "steady".into(),
                projected_exhaustion_ms: Some(2_000_000_000_000),
                burn_rate: 0.25,
                level: AlertLevel::Info,
            }),
        ))
        .expect("apply budget payload");
    assert_eq!(budget_updates.len(), 3);
    assert_eq!(update(&budget_updates, COST_METER).data["total_usd"], 5.0);
    assert_eq!(
        update(&budget_updates, COST_METER).data["budget_remaining"],
        5.0
    );
    assert_eq!(
        update(&budget_updates, AGENT_VITALITY).data["agents"][0]["name"],
        "alice"
    );
    assert_eq!(
        update(&budget_updates, COHORT_HEALTH).data["avg_vitality"],
        0.6
    );

    let hub = StateHub::new(8);
    let sender = hub.sender();
    for update in cost_updates.iter().chain(&budget_updates) {
        update.apply_to(&sender);
    }
    let cost = sender
        .get_projection(COST_METER)
        .expect("published cost projection");
    assert_eq!(cost.version, 2);
    assert_eq!(cost.source_lenses, vec!["cost-lens", "budget-lens"]);
    assert_eq!(cost.data["burn_rate_usd_per_hour"], 0.25);
}

#[test]
fn quality_and_latency_materialize_gate_and_active_task_projections() {
    let mut aggregator = TelemetryProjectionAggregator::new();
    let quality_updates = aggregator
        .consume(&lens_signal(
            "quality-lens",
            LensPayload::Quality(QualityPayload {
                target: "graph:build".into(),
                interval_ms: 60_000,
                total_verifications: 10,
                pre_verify_vetoes: 1,
                post_verify_passed: 8,
                post_verify_failed: 2,
                pass_rate: 0.8,
                avg_reward: 0.75,
                hard_criteria_failures: 2,
                rung_breakdown: BTreeMap::from([
                    (
                        "compile".into(),
                        PassFailCounts {
                            passed: 9,
                            failed: 1,
                        },
                    ),
                    (
                        "tests".into(),
                        PassFailCounts {
                            passed: 8,
                            failed: 2,
                        },
                    ),
                ]),
            }),
        ))
        .expect("apply quality payload");
    let gate = &update(&quality_updates, GATE_PIPELINE).data;
    assert_eq!(gate["rungs"][0]["name"], "compile");
    assert_eq!(gate["rungs"][1]["name"], "tests");
    assert_eq!(gate["hard_criteria_fail_rate"], 0.2);

    aggregator
        .consume(&lens_signal(
            "latency-a",
            LensPayload::Latency(LatencyPayload {
                target: "graph:a".into(),
                interval_ms: 60_000,
                count: 2,
                p50_ms: 100,
                p95_ms: 100,
                p99_ms: 100,
                mean_ms: 100,
            }),
        ))
        .expect("apply first latency payload");
    let latency_updates = aggregator
        .consume(&lens_signal(
            "latency-b",
            LensPayload::Latency(LatencyPayload {
                target: "graph:b".into(),
                interval_ms: 60_000,
                count: 1,
                p50_ms: 400,
                p95_ms: 400,
                p99_ms: 400,
                mean_ms: 400,
            }),
        ))
        .expect("apply second latency payload");
    assert_eq!(
        update(&latency_updates, ACTIVE_TASKS).data["avg_task_duration_ms"],
        200
    );
}

#[test]
fn drift_efficiency_cfactor_and_chained_trends_update_typed_state() {
    let mut aggregator = TelemetryProjectionAggregator::new();
    let drift = aggregator
        .consume(&lens_signal(
            "drift-lens",
            LensPayload::Drift(DriftPayload {
                memory: "agent:alice".into(),
                interval_ms: 60_000,
                total_entries: 20,
                tier_distribution: BTreeMap::from([("heuristic".into(), 3)]),
                avg_balance: 0.7,
                balance_delta: -0.1,
                promotion_rate: 0.2,
                demotion_rate: 0.1,
                cold_entries: 2,
                anti_knowledge_count: 1,
                heuristic_calibration_avg: 0.9,
            }),
        ))
        .expect("apply drift payload");
    assert_eq!(update(&drift, KNOWLEDGE_HEALTH).data["heuristic_count"], 3);

    aggregator
        .consume(&lens_signal(
            "efficiency-lens",
            LensPayload::Efficiency(EfficiencyPayload {
                agent: "alice".into(),
                interval_ms: 60_000,
                tasks_completed: 8,
                tokens_per_task: 100.0,
                usd_per_task: 0.1,
                quality_per_usd: 8.0,
                t0_hit_rate: 0.75,
                t1_hit_rate: 0.2,
                t2_hit_rate: 0.05,
                avg_prediction_error: 0.1,
                vitality: 0.8,
                vitality_phase: "thriving".into(),
            }),
        ))
        .expect("apply efficiency payload");
    let c_factor = aggregator
        .consume(&lens_signal(
            "collective-intelligence-lens",
            LensPayload::CFactor(CFactorPayload {
                space: "alpha".into(),
                interval_ms: 60_000,
                c_factor: 0.82,
                turn_taking_entropy: 0.8,
                peer_prediction_accuracy: 0.7,
                citation_reciprocity: 0.6,
                hdc_diversity: 0.9,
                agent_count: 4,
                active_agents: 3,
                dominant_agent_share: 0.4,
                knowledge_flow_edges: 12,
                avg_agent_vitality: 0.75,
            }),
        ))
        .expect("apply c-factor payload");
    assert_eq!(update(&c_factor, C_FACTOR).data["c_factor"], 0.82);
    assert_eq!(
        update(&c_factor, C_FACTOR).data["components"]["hdc_diversity"],
        0.9
    );
    assert_eq!(c_factor.len(), 1);
    assert_eq!(aggregator.state().cohort_health.agent_count, 1);

    let trend = aggregator
        .consume(&lens_signal(
            "trend-lens",
            LensPayload::Trend(TrendPayload {
                source_lens: "collective-intelligence-lens".into(),
                metric: "c_factor".into(),
                window_ms: 300_000,
                slope: 0.1,
                ema: 0.82,
                ema_previous: 0.72,
                direction: TrendDirection::Rising,
                r_squared: 0.95,
                data_points: 20,
            }),
        ))
        .expect("apply chained trend payload");
    assert_eq!(trend.len(), 1);
    assert_eq!(update(&trend, C_FACTOR).data["trend"], "rising");
}

#[test]
fn known_but_unprojected_anomaly_and_unknown_trend_are_noops() {
    use roko_core::{AnomalyDirection, AnomalyLevel, AnomalyPayload};

    let mut aggregator = TelemetryProjectionAggregator::new();
    let anomaly = aggregator
        .consume(&lens_signal(
            "anomaly-lens",
            LensPayload::Anomaly(AnomalyPayload {
                source_lens: "latency".into(),
                metric: "p99_ms".into(),
                observed_value: 500.0,
                expected_value: 100.0,
                deviation: 4.0,
                direction: AnomalyDirection::Above,
                severity: AnomalyLevel::Critical,
            }),
        ))
        .expect("apply anomaly payload");
    assert!(anomaly.is_empty());

    let usage = aggregator
        .consume(&lens_signal(
            "usage-lens",
            LensPayload::Usage(UsagePayload {
                target: "space:default".into(),
                interval_ms: 250,
                cell_runs: 2,
                graph_runs: 1,
                trigger_fires: 3,
                total_cost_usd: 0.25,
                total_duration_ms: 250,
            }),
        ))
        .expect("apply typed, unprojected usage payload");
    assert!(usage.is_empty());

    let unknown_trend = aggregator
        .consume(&lens_signal(
            "trend-lens",
            LensPayload::Trend(TrendPayload {
                source_lens: "custom-lens".into(),
                metric: "custom_metric".into(),
                window_ms: 1_000,
                slope: 0.0,
                ema: 0.0,
                ema_previous: 0.0,
                direction: TrendDirection::Stable,
                r_squared: 0.0,
                data_points: 1,
            }),
        ))
        .expect("apply unprojected trend payload");
    assert!(unknown_trend.is_empty());

    let error = aggregator
        .consume(&lens_signal(
            "error-lens",
            LensPayload::Error(ErrorPayload {
                target: "graph:build".into(),
                interval_ms: 1_000,
                total_errors: 1,
                by_category: BTreeMap::from([("Timeout".into(), 1)]),
                by_block: BTreeMap::new(),
                retry_count: 0,
                retry_success_rate: 0.0,
                error_rate: 0.1,
            }),
        ))
        .expect("apply error payload");
    assert_eq!(update(&error, COHORT_HEALTH).data["error_rate"], 0.1);
}
