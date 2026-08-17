//! Public-contract tests for C-factor telemetry calculations.

use roko_core::telemetry_observe::{
    CFactorPayload, citation_reciprocity, delivery_rate, hdc_diversity, peer_prediction_accuracy,
    turn_taking_entropy,
};

fn assert_close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-12, "{actual} != {expected}");
}

#[test]
fn telemetry_cfactor_entropy_handles_empty_single_and_uniform_cohorts() {
    assert_close(turn_taking_entropy(&[]), 0.0);
    assert_close(turn_taking_entropy(&[4]), 1.0);
    assert_close(turn_taking_entropy(&[0]), 0.0);
    assert_close(turn_taking_entropy(&[5, 5, 5, 5]), 1.0);
    assert_close(turn_taking_entropy(&[20, 0, 0, 0]), 0.0);
}

#[test]
fn telemetry_cfactor_sub_lenses_cover_no_data_and_boundaries() {
    assert_close(peer_prediction_accuracy(&[], &[]), 0.5);
    assert_close(peer_prediction_accuracy(&[0.0, 1.0], &[0.0, 1.0]), 1.0);
    assert_close(peer_prediction_accuracy(&[0.0, 1.0], &[1.0, 0.0]), 0.0);

    assert_close(citation_reciprocity(&[], &[]), 0.5);
    assert_close(citation_reciprocity(&[0.8, 0.2], &[0.8, 0.2, 1.0]), 0.5);
    assert_close(delivery_rate(0, 0), 1.0);
    assert_close(delivery_rate(3, 1), 0.75);
    assert_close(delivery_rate(u64::MAX, u64::MAX), 0.5);

    assert_close(hdc_diversity(&[]), 0.0);
    assert_close(hdc_diversity(&[1.0, 0.5, 0.0]), 0.5);
}

#[test]
fn telemetry_cfactor_payload_uses_millisecond_wire_field() {
    let payload = CFactorPayload {
        space: "default".into(),
        interval_ms: 30_000,
        c_factor: 0.7,
        turn_taking_entropy: 0.8,
        peer_prediction_accuracy: 0.75,
        citation_reciprocity: 0.6,
        hdc_diversity: 0.55,
        agent_count: 4,
        active_agents: 3,
        dominant_agent_share: 0.4,
        knowledge_flow_edges: 12,
        avg_agent_vitality: 0.9,
    };

    let value = serde_json::to_value(&payload).expect("serialize C-factor payload");
    assert_eq!(value["interval_ms"], 30_000);
    assert!(value.get("interval").is_none());
    assert_eq!(
        serde_json::from_value::<CFactorPayload>(value).unwrap(),
        payload
    );
}
