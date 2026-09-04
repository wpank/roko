//! Research route.

use std::sync::Arc;

use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::post};

use crate::state::{AgentState, ResearchRequest};

/// Research routes.
pub fn router() -> Router<Arc<AgentState>> {
    Router::new().route("/research", post(research))
}

async fn research(
    State(state): State<Arc<AgentState>>,
    Json(request): Json<ResearchRequest>,
) -> impl IntoResponse {
    match state.research(request).await {
        Some(response) => (StatusCode::OK, Json(serde_json::json!(response))).into_response(),
        None => (
            StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!({
                "error": "active research is not supported; use mode=local_knowledge",
                "supported_modes": ["local_knowledge"],
            })),
        )
            .into_response(),
    }
}
