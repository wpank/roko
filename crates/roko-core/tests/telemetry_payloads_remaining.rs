//! Public wire-contract tests for the remaining built-in Lens payloads.

use std::collections::BTreeMap;

use roko_core::{
    AlertLevel, AnomalyDirection, AnomalyLevel, AnomalyPayload, BudgetAlertPayload, DriftPayload,
    ErrorCategory, ErrorPayload, TrendDirection, TrendPayload, UsagePayload,
};
use serde::{Serialize, de::DeserializeOwned};

fn roundtrip<T>(value: &T) -> serde_json::Value
where
    T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let json = serde_json::to_value(value).expect("serialize Lens payload");
    let decoded: T = serde_json::from_value(json.clone()).expect("deserialize Lens payload");
    assert_eq!(&decoded, value);
    json
}

#[test]
fn error_payload_preserves_maps_and_millisecond_interval() {
    let payload = ErrorPayload {
        target: "graph:build".into(),
        interval_ms: 30_000,
        total_errors: 3,
        by_category: BTreeMap::from([("Timeout".into(), 2), ("External".into(), 1)]),
        by_block: BTreeMap::from([("compile".into(), 3)]),
        retry_count: 2,
        retry_success_rate: 0.5,
        error_rate: 0.03,
    };
    let json = roundtrip(&payload);
    assert_eq!(json["interval_ms"], 30_000);
    assert_eq!(json["by_category"]["Timeout"], 2);
    assert!(json.get("interval").is_none());

    for (category, wire_name) in [
        (ErrorCategory::Timeout, "Timeout"),
        (ErrorCategory::CapabilityDenied, "CapabilityDenied"),
        (ErrorCategory::External, "External"),
        (ErrorCategory::LogicError, "LogicError"),
        (ErrorCategory::InputInvalid, "InputInvalid"),
        (ErrorCategory::Cancelled, "Cancelled"),
    ] {
        assert_eq!(roundtrip(&category), wire_name);
    }
}

#[test]
fn drift_payload_preserves_all_quality_metrics() {
    let payload = DriftPayload {
        memory: "agent:researcher".into(),
        interval_ms: 60_000,
        total_entries: 42,
        tier_distribution: BTreeMap::from([("working".into(), 30), ("episodic".into(), 12)]),
        avg_balance: 0.7,
        balance_delta: -0.05,
        promotion_rate: 0.2,
        demotion_rate: 0.1,
        cold_entries: 4,
        anti_knowledge_count: 2,
        heuristic_calibration_avg: 0.8,
    };
    let json = roundtrip(&payload);
    assert_eq!(json["memory"], "agent:researcher");
    assert_eq!(json["tier_distribution"]["working"], 30);
    assert_eq!(json["heuristic_calibration_avg"], 0.8);
}

#[test]
fn budget_alert_uses_portable_phase_and_epoch_milliseconds() {
    let payload = BudgetAlertPayload {
        target: "space:alpha".into(),
        budget_total: 10.0,
        budget_spent: 7.5,
        budget_remaining: 2.5,
        vitality: 0.25,
        vitality_phase: "conserve".into(),
        projected_exhaustion_ms: Some(1_800_000_000_000),
        burn_rate: 0.5,
        level: AlertLevel::Warning,
    };
    let json = roundtrip(&payload);
    assert_eq!(json["vitality_phase"], "conserve");
    assert_eq!(json["projected_exhaustion_ms"], 1_800_000_000_000_i64);
    assert_eq!(json["level"], "Warning");
    assert!(json.get("projected_exhaustion").is_none());

    for level in [AlertLevel::Info, AlertLevel::Warning, AlertLevel::Critical] {
        roundtrip(&level);
    }
}

#[test]
fn chained_lens_payloads_preserve_directions_and_severity() {
    let trend = TrendPayload {
        source_lens: "cost".into(),
        metric: "total_usd".into(),
        window_ms: 300_000,
        slope: 0.2,
        ema: 4.2,
        ema_previous: 4.0,
        direction: TrendDirection::Rising,
        r_squared: 0.95,
        data_points: 20,
    };
    let trend_json = roundtrip(&trend);
    assert_eq!(trend_json["window_ms"], 300_000);
    assert_eq!(trend_json["direction"], "Rising");
    assert!(trend_json.get("window").is_none());

    let anomaly = AnomalyPayload {
        source_lens: "latency".into(),
        metric: "p95_ms".into(),
        observed_value: 900.0,
        expected_value: 300.0,
        deviation: 3.5,
        direction: AnomalyDirection::Above,
        severity: AnomalyLevel::Severe,
    };
    let anomaly_json = roundtrip(&anomaly);
    assert_eq!(anomaly_json["direction"], "Above");
    assert_eq!(anomaly_json["severity"], "Severe");

    for direction in [
        TrendDirection::Rising,
        TrendDirection::Falling,
        TrendDirection::Stable,
    ] {
        roundtrip(&direction);
    }
    for direction in [AnomalyDirection::Above, AnomalyDirection::Below] {
        roundtrip(&direction);
    }
    for level in [
        AnomalyLevel::Moderate,
        AnomalyLevel::Severe,
        AnomalyLevel::Critical,
    ] {
        roundtrip(&level);
    }
}

#[test]
fn usage_payload_reports_only_lifecycle_evidence() {
    let payload = UsagePayload {
        target: "space:default".into(),
        interval_ms: 450,
        cell_runs: 3,
        graph_runs: 1,
        trigger_fires: 2,
        total_cost_usd: 0.75,
        total_duration_ms: 450,
    };
    let json = roundtrip(&payload);
    assert_eq!(json["cell_runs"], 3);
    assert_eq!(json["graph_runs"], 1);
    assert_eq!(json["trigger_fires"], 2);
    assert!(json.get("total_tokens").is_none());
}
