//! Episodes and signals endpoints.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::ApiError;
use crate::projection_contract::{ProjectionQuery, RuntimeProjectionSet};
use crate::state::AppState;
use roko_core::{ContentHash, Signal, SignalStatus};

use super::helpers::{MAX_JSONL_RESULTS, read_jsonl_entries};

/// `GET /api/episodes` — normalized episode proof rows from canonical projections.
pub async fn episodes(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ProjectionQuery>,
) -> Result<Json<Value>, ApiError> {
    let projections = RuntimeProjectionSet::load(&state).await?;
    Ok(Json(Value::Array(projections.episode_items(&query))))
}

#[derive(Deserialize)]
pub struct SignalQuery {
    pub limit: Option<usize>,
}

/// `GET /api/signals` — read signals JSONL as a JSON array, with optional `?limit=N`.
pub async fn signals(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SignalQuery>,
) -> Result<Json<Value>, ApiError> {
    let path = state.workdir.join(".roko").join("engrams.jsonl");
    let entries = read_jsonl_entries(&path).await?;
    let cap = q.limit.unwrap_or(MAX_JSONL_RESULTS).min(MAX_JSONL_RESULTS);
    let limited: Vec<Value> = entries
        .into_iter()
        .rev()
        .take(cap)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    Ok(Json(Value::Array(limited)))
}

/// Request for an exact monotonic Signal tier promotion.
#[derive(Debug, Deserialize)]
pub struct PromoteSignalRequest {
    /// Requested next tier.
    pub target: SignalStatus,
    /// Minimum effective score for transient-to-working promotion.
    #[serde(default)]
    pub min_score: f32,
    /// Minimum age for consolidated-to-persistent promotion.
    #[serde(default)]
    pub min_age_secs: u64,
    /// Minimum accesses for consolidated-to-persistent promotion.
    #[serde(default)]
    pub min_accesses: u32,
}

/// `PATCH /api/signals/{id}/promote` — apply one durable tier transition.
pub async fn promote_signal(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(request): Json<PromoteSignalRequest>,
) -> Result<Json<Signal>, ApiError> {
    let signal_id = ContentHash::from_hex(&id)
        .ok_or_else(|| ApiError::bad_request("signal id must be a 64-character content hash"))?;
    let canonical_id = signal_id.to_hex();
    state
        .signal_store
        .promote(
            &signal_id,
            request.target,
            request.min_score,
            request.min_age_secs,
            request.min_accesses,
        )
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?
        .map(Json)
        .ok_or_else(|| ApiError::not_found(format!("signal '{canonical_id}' not found")))
}

/// `DELETE /api/signals/{id}` — durably prune one non-persistent Signal.
pub async fn prune_signal(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let signal_id = ContentHash::from_hex(&id)
        .ok_or_else(|| ApiError::bad_request("signal id must be a 64-character content hash"))?;
    let canonical_id = signal_id.to_hex();
    let pruned = state
        .signal_store
        .prune(&signal_id)
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    Ok(Json(json!({ "id": canonical_id, "pruned": pruned })))
}
