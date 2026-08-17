//! Wire-contract coverage for the portable telemetry projection schemas.

use roko_core::telemetry_projections::{
    ActiveTasksProjection, AgentVitalityProjection, CFactorProjection, CohortHealthProjection,
    CostMeterProjection, GatePipelineProjection, KnowledgeHealthProjection,
};

fn roundtrip<T>(value: T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let encoded = serde_json::to_value(&value).expect("serialize projection");
    let decoded: T = serde_json::from_value(encoded).expect("deserialize projection");
    assert_eq!(decoded, value);
}

#[test]
fn all_core_projection_defaults_round_trip() {
    roundtrip(CohortHealthProjection::default());
    roundtrip(ActiveTasksProjection::default());
    roundtrip(GatePipelineProjection::default());
    roundtrip(CostMeterProjection::default());
    roundtrip(KnowledgeHealthProjection::default());
    roundtrip(CFactorProjection::default());
    roundtrip(AgentVitalityProjection::default());
}

#[test]
fn duration_and_portable_enum_fields_have_stable_wire_names() {
    let active = serde_json::to_value(ActiveTasksProjection::default()).unwrap();
    assert!(active.get("avg_task_duration_ms").is_some());

    let cost = serde_json::to_value(CostMeterProjection::default()).unwrap();
    assert!(cost["cost_trend"].is_string());

    let c_factor = serde_json::to_value(CFactorProjection::default()).unwrap();
    assert!(c_factor["trend"].is_string());
}
