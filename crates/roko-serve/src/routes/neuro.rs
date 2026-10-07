//! Neuro knowledge store query endpoint + RAG retrieval stats (RAG-14).

use std::sync::Arc;
use std::time::Instant;

use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use roko_core::ObservableEvent;
use roko_neuro::KnowledgeTier;
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
    /// Least durable tier to return (one of [`TIER_NAMES`]); entries in lower
    /// tiers are left out.
    #[serde(default)]
    min_tier: Option<String>,
}

fn default_limit() -> usize {
    10
}

/// Tier names `min_tier` accepts, least durable first.
const TIER_NAMES: [&str; 4] = ["transient", "working", "consolidated", "persistent"];

/// Durability rank of a tier: its position in [`TIER_NAMES`].
fn tier_rank(tier: KnowledgeTier) -> usize {
    match tier {
        KnowledgeTier::Transient => 0,
        KnowledgeTier::Working => 1,
        KnowledgeTier::Consolidated => 2,
        KnowledgeTier::Persistent => 3,
    }
}

/// Rank of a tier name, ignoring ASCII case; `None` for an unknown name.
fn tier_rank_by_name(name: &str) -> Option<usize> {
    TIER_NAMES
        .iter()
        .position(|tier| tier.eq_ignore_ascii_case(name.trim()))
}

impl RequestPayload for NeuroQueryRequest {
    fn validate_payload(&self) -> Result<(), ApiError> {
        if self.query.trim().is_empty() {
            return Err(ApiError::bad_request("query must not be blank"));
        }
        if let Some(min_tier) = self.min_tier.as_deref()
            && tier_rank_by_name(min_tier).is_none()
        {
            return Err(ApiError::bad_request(format!(
                "unknown min_tier '{min_tier}'; valid tiers: {}",
                TIER_NAMES.join(", ")
            )));
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
    let min_rank = body.min_tier.as_deref().and_then(tier_rank_by_name);
    let results = match min_rank {
        // Rank every match before filtering, so lower tiers cannot crowd out `limit`.
        Some(min_rank) => store.query(&body.query, usize::MAX).map(|entries| {
            entries
                .into_iter()
                .filter(|entry| tier_rank(entry.tier) >= min_rank)
                .take(body.limit)
                .collect::<Vec<_>>()
        }),
        None => store.query(&body.query, body.limit),
    }
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
    query_knowledge(&state, &params.q, params.limit).map(Json)
}

/// What the knowledge store holds on `query`: at most `limit` entries, most
/// relevant first, as `{ "results", "total" }`. A blank query finds nothing.
/// The MCP `recall` tool answers with it too.
pub(super) fn query_knowledge(
    state: &AppState,
    query: &str,
    limit: usize,
) -> Result<Value, ApiError> {
    if query.trim().is_empty() {
        return Ok(json!({ "results": [], "total": 0 }));
    }

    let layout = &state.layout;
    let store = roko_neuro::knowledge_store::KnowledgeStore::for_layout(layout);

    let started = Instant::now();
    let results = store
        .query(query, limit)
        .map_err(|e| ApiError::internal(format!("knowledge query failed: {e}")))?;

    let total = results.len();
    crate::emit_lens_observation(
        state,
        ObservableEvent::MemoryRetrieved {
            query: query.to_string(),
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

    Ok(json!({
        "results": entries,
        "total": total,
    }))
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

#[cfg(test)]
mod tests {
    use super::*;

    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use roko_core::config::schema::RokoConfig;
    use tempfile::tempdir;
    use tower::ServiceExt;

    use crate::deploy::manual::ManualBackend;
    use crate::runtime::NoOpRuntime;

    #[tokio::test]
    async fn query_filters_by_min_tier() {
        let dir = tempdir().expect("tempdir");
        let state = Arc::new(
            AppState::new(
                dir.path().to_path_buf(),
                Arc::new(NoOpRuntime),
                RokoConfig::default(),
                Arc::new(ManualBackend::default()),
            )
            .expect("AppState::new"),
        );
        // Every `KnowledgeEntry` field has a serde default, so raw lines suffice.
        let store = roko_neuro::knowledge_store::KnowledgeStore::for_layout(&state.layout);
        std::fs::create_dir_all(store.path().parent().expect("store dir")).expect("store dir");
        let lines: String = ["transient", "working", "consolidated"]
            .iter()
            .map(|tier| {
                let entry = json!({
                    "id": format!("k-{tier}"),
                    "tier": tier,
                    "content": "retry with exponential backoff",
                });
                format!("{entry}\n")
            })
            .collect();
        std::fs::write(store.path(), lines).expect("write knowledge store");
        let router = Router::new()
            .nest("/api", routes())
            .with_state(Arc::clone(&state));

        let (status, all) = post_query(&router, json!({ "query": "backoff" })).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            result_ids(&all),
            ["k-consolidated", "k-transient", "k-working"]
        );

        let working_up = json!({ "query": "backoff", "min_tier": "Working" });
        let (status, durable) = post_query(&router, working_up).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(result_ids(&durable), ["k-consolidated", "k-working"]);

        let first_only = json!({ "query": "backoff", "min_tier": "transient", "limit": 1 });
        let (status, top) = post_query(&router, first_only).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(top["total"], 1);

        let unknown_tier = json!({ "query": "backoff", "min_tier": "durable" });
        let (status, rejected) = post_query(&router, unknown_tier).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let message = rejected["message"].as_str().unwrap_or_default();
        assert!(message.contains("unknown min_tier 'durable'"), "{rejected}");
    }

    async fn post_query(router: &Router, body: Value) -> (StatusCode, Value) {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/neuro/query")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .expect("request"),
            )
            .await
            .expect("response");
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        (status, serde_json::from_slice(&bytes).expect("json body"))
    }

    /// Result ids, sorted: entries with equal scores have no fixed order.
    fn result_ids(body: &Value) -> Vec<String> {
        let mut ids: Vec<String> = body["results"]
            .as_array()
            .expect("results array")
            .iter()
            .filter_map(|entry| entry["id"].as_str().map(str::to_owned))
            .collect();
        ids.sort();
        ids
    }
}
