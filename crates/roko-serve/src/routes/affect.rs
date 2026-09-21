//! Affect / daimon observability endpoint.
//!
//! `GET /api/affect/state` -- current DaimonState summary (PAD values,
//! behavioral state, energy, somatic marker count).

use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::routing::get;
use serde::Serialize;

use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/affect/state", get(affect_state_handler))
}

#[derive(Serialize)]
struct AffectStateResponse {
    pleasure: f64,
    arousal: f64,
    dominance: f64,
    confidence: f64,
    behavioral_state: String,
    tick_count: u64,
    half_life_hours: f64,
    somatic_marker_count: usize,
    cognitive_energy_current: f64,
    cognitive_energy_max: f64,
    goal_count: usize,
    updated_at: String,
}

async fn affect_state_handler(State(state): State<Arc<AppState>>) -> Json<AffectStateResponse> {
    let engine = state.affect_engine.lock().await;
    let affect = &engine.state;
    let response = AffectStateResponse {
        pleasure: affect.pad.pleasure,
        arousal: affect.pad.arousal,
        dominance: affect.pad.dominance,
        confidence: affect.confidence,
        behavioral_state: format!("{:?}", affect.behavioral_state),
        tick_count: affect.tick_count,
        half_life_hours: engine.half_life_hours,
        somatic_marker_count: engine.somatic_landscape.markers.len(),
        cognitive_energy_current: engine.cognitive_energy.current,
        cognitive_energy_max: engine.cognitive_energy.max,
        goal_count: engine.goal_tree.node_count(),
        updated_at: affect.updated_at.to_rfc3339(),
    };
    Json(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affect_response_serializes() {
        let response = AffectStateResponse {
            pleasure: 0.0,
            arousal: 0.0,
            dominance: 0.0,
            confidence: 0.5,
            behavioral_state: "Engaged".to_string(),
            tick_count: 0,
            half_life_hours: 12.0,
            somatic_marker_count: 0,
            cognitive_energy_current: 100.0,
            cognitive_energy_max: 100.0,
            goal_count: 0,
            updated_at: "2025-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"pleasure\":0.0"));
    }
}
