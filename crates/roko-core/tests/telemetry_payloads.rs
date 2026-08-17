//! Public-contract tests for built-in telemetry Lens payloads.

use std::collections::BTreeMap;

use roko_core::telemetry_observe::{
    CostReportPayload, EfficiencyPayload, LatencyPayload, PassFailCounts, QualityPayload,
};

fn roundtrip<T>(value: &T) -> serde_json::Value
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let json = serde_json::to_value(value).expect("serialize telemetry payload");
    let decoded: T = serde_json::from_value(json.clone()).expect("deserialize telemetry payload");
    assert_eq!(&decoded, value);
    json
}

#[test]
fn telemetry_payloads_cost_and_latency_preserve_wire_fields() {
    let cost = CostReportPayload {
        target: "graph:build".into(),
        interval_ms: 30_000,
        total_usd: 0.42,
        total_tokens: 1_200,
        input_tokens: 1_000,
        output_tokens: 200,
        model_breakdown: BTreeMap::from([("gpt".into(), 0.42)]),
        cumulative_usd: 4.2,
        budget_remaining: Some(5.8),
        vitality: None,
    };
    let cost_json = roundtrip(&cost);
    assert_eq!(cost_json["interval_ms"], 30_000);
    assert!(cost_json.get("interval").is_none());

    let latency = LatencyPayload {
        target: "graph:build".into(),
        interval_ms: 30_000,
        count: 10,
        p50_ms: 50,
        p95_ms: 95,
        p99_ms: 99,
        mean_ms: 60,
    };
    let latency_json = roundtrip(&latency);
    assert_eq!(latency_json["p95_ms"], 95);
    assert!(latency_json.get("p95").is_none());
}

#[test]
fn telemetry_payloads_quality_uses_typed_rung_counts() {
    let quality = QualityPayload {
        target: "graph:build".into(),
        interval_ms: 30_000,
        total_verifications: 5,
        pre_verify_vetoes: 1,
        post_verify_passed: 3,
        post_verify_failed: 1,
        pass_rate: 0.75,
        avg_reward: 0.8,
        hard_criteria_failures: 1,
        rung_breakdown: BTreeMap::from([(
            "tests".into(),
            PassFailCounts {
                passed: 3,
                failed: 1,
            },
        )]),
    };
    let json = roundtrip(&quality);
    assert_eq!(json["rung_breakdown"]["tests"]["passed"], 3);
    assert_eq!(json["rung_breakdown"]["tests"]["failed"], 1);
}

#[test]
fn telemetry_payloads_efficiency_uses_portable_vitality_phase() {
    let efficiency = EfficiencyPayload {
        agent: "implementer".into(),
        interval_ms: 60_000,
        tasks_completed: 4,
        tokens_per_task: 250.0,
        usd_per_task: 0.2,
        quality_per_usd: 4.0,
        t0_hit_rate: 0.5,
        t1_hit_rate: 0.25,
        t2_hit_rate: 0.25,
        avg_prediction_error: 0.1,
        vitality: 0.9,
        vitality_phase: "thriving".into(),
    };
    let json = roundtrip(&efficiency);
    assert_eq!(json["vitality_phase"], "thriving");
    assert_eq!(json["interval_ms"], 60_000);
}
