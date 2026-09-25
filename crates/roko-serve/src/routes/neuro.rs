//! Neuro knowledge store query endpoint + RAG retrieval stats (RAG-14).

use std::sync::Arc;
use std::time::Instant;

use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use roko_core::ObservableEvent;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::ApiError;
use crate::extract::{RequestPayload, ValidJson};
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/neuro/query", post(neuro_query))
        .route("/knowledge", get(knowledge_query))
        // RAG-14: retrieval quality stats and search
        .route("/retrieval/stats", get(retrieval_stats))
        .route("/retrieval/query", get(retrieval_query))
}

#[derive(Debug, Deserialize)]
struct NeuroQueryRequest {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
    /// Tier filter hint — accepted from the request body but not yet consumed
    /// by the underlying query implementation.
    #[serde(default)]
    #[allow(dead_code)]
    min_tier: Option<String>,
}

fn default_limit() -> usize {
    10
}

impl RequestPayload for NeuroQueryRequest {
    fn validate_payload(&self) -> Result<(), ApiError> {
        if self.query.trim().is_empty() {
            return Err(ApiError::bad_request("query must not be blank"));
        }
        Ok(())
    }
}

/// `POST /api/neuro/query` — query the knowledge store.
async fn neuro_query(
    State(state): State<Arc<AppState>>,
    ValidJson(body): ValidJson<NeuroQueryRequest>,
) -> Result<Json<Value>, ApiError> {
    let layout = &state.layout;
    let store = roko_neuro::knowledge_store::KnowledgeStore::for_layout(layout);

    let started = Instant::now();
    let results = store
        .query(&body.query, body.limit)
        .map_err(|e| ApiError::internal(format!("neuro query failed: {e}")))?;

    let total = results.len();
    crate::emit_lens_observation(
        &state,
        ObservableEvent::MemoryRetrieved {
            query: body.query.clone(),
            results: total,
            duration_ms: started.elapsed().as_millis() as u64,
        },
    );
    let entries: Vec<Value> = results
        .into_iter()
        .map(|entry| {
            json!({
                "id": entry.id,
                "content": entry.content,
                "kind": format!("{:?}", entry.kind),
                "tier": format!("{:?}", entry.tier),
                "relevance": entry.confidence,
                "created_at": entry.created_at.to_rfc3339(),
            })
        })
        .collect();

    Ok(Json(json!({
        "results": entries,
        "total": total,
    })))
}

/// Query params for the GET knowledge alias.
#[derive(Debug, Deserialize)]
struct KnowledgeQueryParams {
    #[serde(default)]
    q: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

/// `GET /api/knowledge?q=<topic>&limit=N` — alias for neuro query.
async fn knowledge_query(
    State(state): State<Arc<AppState>>,
    Query(params): Query<KnowledgeQueryParams>,
) -> Result<Json<Value>, ApiError> {
    if params.q.trim().is_empty() {
        return Ok(Json(json!({ "results": [], "total": 0 })));
    }

    let layout = &state.layout;
    let store = roko_neuro::knowledge_store::KnowledgeStore::for_layout(layout);

    let started = Instant::now();
    let results = store
        .query(&params.q, params.limit)
        .map_err(|e| ApiError::internal(format!("knowledge query failed: {e}")))?;

    let total = results.len();
    crate::emit_lens_observation(
        &state,
        ObservableEvent::MemoryRetrieved {
            query: params.q.clone(),
            results: total,
            duration_ms: started.elapsed().as_millis() as u64,
        },
    );
    let entries: Vec<Value> = results
        .into_iter()
        .map(|entry| {
            json!({
                "id": entry.id,
                "content": entry.content,
                "kind": format!("{:?}", entry.kind),
                "tier": format!("{:?}", entry.tier),
                "relevance": entry.confidence,
                "created_at": entry.created_at.to_rfc3339(),
            })
        })
        .collect();

    Ok(Json(json!({
        "results": entries,
        "total": total,
    })))
}

// ─── RAG-14: Retrieval stats and query routes ────────────────────────────────

/// `GET /api/retrieval/stats` — aggregate stats from `.roko/learn/retrieval-outcomes.jsonl`.
///
/// Returns precision (gate-pass rate), avg latency, miss rate, and a per-strategy breakdown.
async fn retrieval_stats(State(state): State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    let path = state
        .workdir
        .join(".roko")
        .join("learn")
        .join("retrieval-outcomes.jsonl");

    let outcomes = roko_learn::retrieval_outcome::RetrievalOutcomeStore::at(&path)
        .read_all()
        .await
        .unwrap_or_default();

    let mut total = 0usize;
    let mut passed = 0usize;
    let mut total_latency_ms: u64 = 0;
    let mut latency_count = 0usize;
    let mut per_strategy: std::collections::HashMap<String, (usize, usize, u64)> =
        std::collections::HashMap::new();

    for rec in &outcomes {
        let Some(gate_ok) = rec.gate_passed else {
            continue;
        };
        total += 1;
        if gate_ok {
            passed += 1;
        }
        if let Some(lat) = rec.latency_ms {
            total_latency_ms = total_latency_ms.saturating_add(lat);
            latency_count += 1;
        }
        let entry = per_strategy.entry(rec.strategy.clone()).or_default();
        entry.0 += 1;
        if gate_ok {
            entry.1 += 1;
        }
        if let Some(lat) = rec.latency_ms {
            entry.2 = entry.2.saturating_add(lat);
        }
    }

    let precision_pct = if total > 0 {
        passed as f64 / total as f64 * 100.0
    } else {
        0.0
    };
    let miss_rate_pct = 100.0 - precision_pct;
    let avg_latency_ms = if latency_count > 0 {
        total_latency_ms as f64 / latency_count as f64
    } else {
        0.0
    };

    let strategies: Vec<Value> = per_strategy
        .iter()
        .map(|(name, (attempts, passes, lat_total))| {
            let prec = if *attempts > 0 {
                *passes as f64 / *attempts as f64 * 100.0
            } else {
                0.0
            };
            let avg_lat = if *attempts > 0 {
                *lat_total as f64 / *attempts as f64
            } else {
                0.0
            };
            json!({
                "strategy": name,
                "attempts": attempts,
                "passed": passes,
                "precision_pct": prec,
                "avg_latency_ms": avg_lat,
            })
        })
        .collect();

    Ok(Json(json!({
        "total_settled": total,
        "passed": passed,
        "precision_pct": precision_pct,
        "miss_rate_pct": miss_rate_pct,
        "avg_latency_ms": avg_latency_ms,
        "strategies": strategies,
    })))
}

/// Query params for `GET /api/retrieval/query`.
#[derive(Debug, Deserialize)]
struct RetrievalQueryParams {
    /// Search query string.
    #[serde(default)]
    q: String,
    /// Maximum number of results.
    #[serde(default = "default_limit")]
    limit: usize,
}

/// `GET /api/retrieval/query?q=<text>&limit=N` — search the knowledge store and return results.
///
/// This is RAG-14's unified retrieval endpoint: callers can use it to search
/// the knowledge store directly, integrating with external tools that need
/// real-time knowledge retrieval.
async fn retrieval_query(
    State(state): State<Arc<AppState>>,
    Query(params): Query<RetrievalQueryParams>,
) -> Result<Json<Value>, ApiError> {
    if params.q.trim().is_empty() {
        return Ok(Json(json!({ "results": [], "total": 0, "query": "" })));
    }

    let layout = &state.layout;
    let store = roko_neuro::knowledge_store::KnowledgeStore::for_layout(layout);

    let started = Instant::now();
    let results = store
        .query(&params.q, params.limit)
        .map_err(|e| ApiError::internal(format!("retrieval query failed: {e}")))?;

    let duration_ms = started.elapsed().as_millis() as u64;
    let total = results.len();

    crate::emit_lens_observation(
        &state,
        ObservableEvent::MemoryRetrieved {
            query: params.q.clone(),
            results: total,
            duration_ms,
        },
    );

    let entries: Vec<Value> = results
        .into_iter()
        .map(|entry| {
            json!({
                "id": entry.id,
                "content": entry.content,
                "kind": format!("{:?}", entry.kind),
                "tier": format!("{:?}", entry.tier),
                "relevance": entry.confidence,
                "created_at": entry.created_at.to_rfc3339(),
            })
        })
        .collect();

    Ok(Json(json!({
        "query": params.q,
        "results": entries,
        "total": total,
        "latency_ms": duration_ms,
    })))
}
