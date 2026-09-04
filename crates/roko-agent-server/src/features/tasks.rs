//! Task queue routes.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};

use crate::state::{AgentState, CreateTaskRequest, CreateTaskResult, TaskCompletionRequest, TaskEntry};

/// Task routes.
pub fn router() -> Router<Arc<AgentState>> {
    Router::new()
        .route("/tasks", get(list_tasks).post(create_task))
        .route("/tasks/{id}/accept", post(accept_task))
        .route("/tasks/{id}/complete", post(complete_task))
}

async fn list_tasks(State(state): State<Arc<AgentState>>) -> Json<Vec<TaskEntry>> {
    Json(state.list_tasks().await)
}

async fn create_task(
    State(state): State<Arc<AgentState>>,
    Json(request): Json<CreateTaskRequest>,
) -> impl IntoResponse {
    match state.create_task(request).await {
        CreateTaskResult::Created(task) => (StatusCode::CREATED, Json(serde_json::json!(task))).into_response(),
        CreateTaskResult::Duplicate(task) => (StatusCode::OK, Json(serde_json::json!(task))).into_response(),
        CreateTaskResult::Conflict => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "idempotency key reused with different request body"
            })),
        )
            .into_response(),
        CreateTaskResult::Invalid(msg) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({ "error": msg })),
        )
            .into_response(),
        CreateTaskResult::Unavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "task creation requires a durable state store"
            })),
        )
            .into_response(),
    }
}

async fn accept_task(
    State(state): State<Arc<AgentState>>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    state.accept_task(id).await.map_or_else(
        || StatusCode::NOT_FOUND.into_response(),
        |task| (StatusCode::OK, Json(task)).into_response(),
    )
}

async fn complete_task(
    State(state): State<Arc<AgentState>>,
    Path(id): Path<u64>,
    Json(request): Json<TaskCompletionRequest>,
) -> impl IntoResponse {
    state.complete_task(id, request).await.map_or_else(
        || StatusCode::NOT_FOUND.into_response(),
        |task| (StatusCode::OK, Json(task)).into_response(),
    )
}
