//! Feed management routes.
//!
//! Descriptor CRUD (static feed registry):
//! - `GET    /api/feeds`                — list feeds (with optional `?kind=` and `?agent_id=` filters)
//! - `POST   /api/feeds`               — register a feed
//! - `GET    /api/feeds/{id}`          — get feed detail
//! - `DELETE /api/feeds/{id}`          — unregister a feed
//! - `GET    /api/feeds/catalog`       — list built-in feed agents and descriptors
//!
//! Runtime feeds (live status from the serve layer):
//! - `GET    /api/feeds/runtime`       — list all runtime feeds with status
//! - `GET    /api/feeds/runtime/{id}`  — get detailed runtime status for a feed

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use roko_core::feed::{FeedAccess, FeedInfo, FeedKind, FeedPricingConfig};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::ApiError;
use crate::routes::middleware::require_payment;
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/feeds", get(list_feeds).post(create_feed))
        // Catalog must be registered before the wildcard `/feeds/{id}`.
        .route("/feeds/catalog", get(get_feed_catalog))
        // Runtime feed routes must be registered before the wildcard `/feeds/{id}`
        // so that "/feeds/runtime" is not captured as id="runtime".
        .route("/feeds/runtime", get(list_runtime_feeds))
        .route("/feeds/runtime/{id}", get(get_runtime_feed_status))
        .route("/feeds/discover", get(discover_feeds))
        .route("/feeds/search", get(search_feeds))
        .route("/feeds/health", get(feed_health))
        .route("/feeds/start/{id}", post(start_feed))
        .route("/feeds/stop/{id}", post(stop_feed))
        .route("/feeds/{id}", get(get_feed).delete(delete_feed))
}

// ── Request / Response types ──────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CreateFeedRequest {
    name: String,
    kind: FeedKind,
    #[serde(default = "default_access")]
    access: FeedAccess,
    agent_id: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    schema: Option<Value>,
    #[serde(default)]
    pricing: Option<FeedPricingConfig>,
}

fn default_access() -> FeedAccess {
    FeedAccess::Public
}

#[derive(Debug, Deserialize)]
struct FeedQuery {
    #[serde(default)]
    kind: Option<FeedKind>,
    #[serde(default)]
    agent_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FeedListResponse {
    feeds: Vec<FeedView>,
    total: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct FeedView {
    #[serde(flatten)]
    feed: FeedInfo,
    running: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    runtime_status: Option<roko_core::FeedRuntimeStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateFeedResponse {
    id: String,
    feed: FeedInfo,
}

#[derive(Debug, Serialize, Deserialize)]
struct DeleteFeedResponse {
    id: String,
    deleted: bool,
}

// ── Feed catalog types ───────────────────────────────────────────

use crate::state::{FeedCatalogAgent, FeedCatalogEntry};

#[derive(Debug, Serialize)]
struct FeedCatalogResponse {
    agents: Vec<FeedCatalogAgent>,
    feeds: Vec<FeedCatalogEntry>,
    stats: FeedCatalogStats,
}

#[derive(Debug, Serialize)]
struct FeedCatalogStats {
    total_agents: usize,
    total_feeds: usize,
    messages_per_sec: f64,
}

// ── Handlers ──────────────────────────────────────────────────────

/// `GET /api/feeds` — list feeds with optional kind and agent_id filters.
async fn list_feeds(
    State(state): State<Arc<AppState>>,
    Query(query): Query<FeedQuery>,
) -> Json<FeedListResponse> {
    let reg = state.feeds.read().await;

    let feeds: Vec<FeedInfo> = match (&query.kind, &query.agent_id) {
        (Some(kind), Some(agent_id)) => reg
            .list()
            .iter()
            .filter(|f| f.kind == *kind && f.agent_id == *agent_id)
            .cloned()
            .collect(),
        (Some(kind), None) => reg
            .list_by_kind(kind.clone())
            .into_iter()
            .cloned()
            .collect(),
        (None, Some(agent_id)) => reg.list_by_agent(agent_id).into_iter().cloned().collect(),
        (None, None) => reg.list().to_vec(),
    };

    let feeds = feeds
        .into_iter()
        .map(|feed| {
            let runtime_status = state
                .runtime_feeds
                .get(&feed.cell_id)
                .map(|handle| handle.status());
            FeedView {
                running: runtime_status.is_some(),
                runtime_status,
                feed,
            }
        })
        .collect::<Vec<_>>();
    let total = feeds.len();
    Json(FeedListResponse { feeds, total })
}

/// `POST /api/feeds` — register a new feed.
async fn create_feed(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateFeedRequest>,
) -> Result<(StatusCode, Json<CreateFeedResponse>), ApiError> {
    if req.name.trim().is_empty() {
        return Err(ApiError::bad_request("feed name must not be empty"));
    }
    if req.agent_id.trim().is_empty() {
        return Err(ApiError::bad_request("agent_id must not be empty"));
    }

    let info = FeedInfo {
        id: String::new(), // assigned by registry
        cell_id: String::new(),
        name: req.name,
        kind: req.kind,
        access: req.access,
        agent_id: req.agent_id,
        description: req.description,
        schema: req.schema,
        pricing: req.pricing,
        created_at: Utc::now(),
    };

    let mut reg = state.feeds.write().await;
    let id = reg.register(info);
    let feed = reg.get(&id).expect("just registered").clone();

    Ok((StatusCode::CREATED, Json(CreateFeedResponse { id, feed })))
}

/// `GET /api/feeds/{id}` — get a single feed by ID.
async fn get_feed(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let reg = state.feeds.read().await;
    let info = reg
        .get(&id)
        .ok_or_else(|| ApiError::not_found(format!("feed '{id}' not found")))?
        .clone();
    drop(reg);

    if let Err(response) = require_payment(&info, &headers) {
        return Ok(response);
    }
    let runtime_status = state
        .runtime_feeds
        .get(&info.cell_id)
        .map(|handle| handle.status());
    Ok(Json(FeedView {
        running: runtime_status.is_some(),
        runtime_status,
        feed: info,
    })
    .into_response())
}

/// `DELETE /api/feeds/{id}` — unregister a feed.
async fn delete_feed(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<DeleteFeedResponse>, ApiError> {
    let mut reg = state.feeds.write().await;
    let deleted = reg.unregister(&id);
    if !deleted {
        return Err(ApiError::not_found(format!("feed '{id}' not found")));
    }
    Ok(Json(DeleteFeedResponse { id, deleted }))
}

// ── Feed catalog handler ─────────────────────────────────────────

/// `GET /api/feeds/catalog` — aggregated feed catalog from feed agents.
async fn get_feed_catalog(State(state): State<Arc<AppState>>) -> Json<FeedCatalogResponse> {
    let snapshot = state.feed_agent_catalog.read().await;
    Json(FeedCatalogResponse {
        agents: snapshot.agents.clone(),
        feeds: snapshot.feeds.clone(),
        stats: FeedCatalogStats {
            total_agents: snapshot.agents.len(),
            total_feeds: snapshot.feeds.len(),
            messages_per_sec: snapshot.messages_per_sec,
        },
    })
}

// ── Runtime feed handlers ────────────────────────────────────────

/// `GET /api/feeds/runtime` -- list all runtime feeds with their current status.
async fn list_runtime_feeds(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<roko_core::FeedRuntimeStatus>> {
    Json(state.runtime_feeds.health())
}

/// `GET /api/feeds/runtime/{id}` -- get detailed status for a single runtime feed.
async fn get_runtime_feed_status(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<roko_core::FeedRuntimeStatus>, StatusCode> {
    state
        .runtime_feeds
        .status(&id)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: String,
}

/// `GET /api/feeds/discover` -- list every runnable built-in feed descriptor.
async fn discover_feeds(State(state): State<Arc<AppState>>) -> Json<Vec<FeedInfo>> {
    Json(state.runtime_feeds.discover())
}

/// `GET /api/feeds/search?q=` -- search runtime feed metadata.
async fn search_feeds(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> Json<Vec<FeedInfo>> {
    Json(state.runtime_feeds.search(&query.q))
}

/// `GET /api/feeds/health` -- aggregate runtime health including stopped feeds.
async fn feed_health(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<roko_core::FeedRuntimeStatus>> {
    Json(state.runtime_feeds.health())
}

/// `POST /api/feeds/start/{id}` -- start a registered runtime feed and bridge it to Bus.
async fn start_feed(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<roko_core::FeedRuntimeStatus>), ApiError> {
    let handle = state
        .runtime_feeds
        .start_registered(&id)
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let _bridge = state.feed_bus_bridge.spawn(handle.cell().subscribe());
    Ok((StatusCode::ACCEPTED, Json(handle.status())))
}

/// `POST /api/feeds/stop/{id}` -- cooperatively stop a running feed.
async fn stop_feed(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<roko_core::FeedRuntimeStatus>, ApiError> {
    let handle = state
        .runtime_feeds
        .get(&id)
        .ok_or_else(|| ApiError::not_found(format!("feed '{id}' is not running")))?;
    state
        .runtime_feeds
        .stop(&id)
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    Ok(Json(handle.status()))
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use roko_chain::x402::{PaymentAuthorization, PaymentRequest};
    use roko_core::config::schema::RokoConfig;
    use roko_core::feed::{PaymentProtocol, PricingTier};
    use tower::ServiceExt;

    use crate::deploy::create_backend;
    use crate::runtime::NoOpRuntime;

    fn test_state(workdir: std::path::PathBuf) -> Arc<AppState> {
        test_state_with_config(workdir, RokoConfig::default())
    }

    fn test_state_with_config(workdir: std::path::PathBuf, config: RokoConfig) -> Arc<AppState> {
        let deploy_backend =
            Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
        Arc::new(
            AppState::new(workdir, Arc::new(NoOpRuntime), config, deploy_backend)
                .expect("AppState::new"),
        )
    }

    #[tokio::test]
    async fn list_feeds_empty() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let state = test_state(dir.path().to_path_buf());

        let response = routes()
            .with_state(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/feeds")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: FeedListResponse = serde_json::from_slice(&body).expect("parse");
        assert!(payload.feeds.is_empty());
        assert_eq!(payload.total, 0);
    }

    #[tokio::test]
    async fn feed_catalog_contains_only_reduced_generic_agents() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let mut config = RokoConfig::default();
        config.feed_agents.enabled = true;
        let state = test_state_with_config(dir.path().to_path_buf(), config);
        let handles = crate::feed_agents::spawn_all(Arc::clone(&state));

        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            loop {
                if state.feed_agent_catalog.read().await.agents.len() == 10 {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("feed catalog populated");

        let response = routes()
            .with_state(Arc::clone(&state))
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/feeds/catalog")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("parse");
        let agent_ids: Vec<&str> = payload["agents"]
            .as_array()
            .expect("agents array")
            .iter()
            .map(|agent| agent["agent_id"].as_str().expect("agent id"))
            .collect();
        assert_eq!(
            agent_ids,
            [
                "chain-watcher",
                "gas-oracle",
                "agent-monitor",
                "relay-stats",
                "system-heartbeat",
                "block-space",
                "tx-throughput",
                "fee-burn",
                "network-health",
                "contract-activity",
            ]
        );
        assert_eq!(payload["feeds"].as_array().expect("feeds array").len(), 10);
        assert_eq!(payload["stats"]["total_agents"], 10);
        assert_eq!(payload["stats"]["total_feeds"], 10);

        state.cancel.cancel();
        for handle in handles {
            handle.abort();
        }
    }

    #[tokio::test]
    async fn create_then_get_feed() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let state = test_state(dir.path().to_path_buf());
        let app = routes().with_state(Arc::clone(&state));

        // Create a feed.
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/feeds")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "name": "eth-prices",
                            "kind": "raw",
                            "agent_id": "agent-1",
                            "description": "ETH/USD price feed"
                        }))
                        .unwrap(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let created: CreateFeedResponse = serde_json::from_slice(&body).expect("parse");
        assert_eq!(created.feed.name, "eth-prices");
        let feed_id = created.id;

        // Get by ID.
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&format!("/feeds/{feed_id}"))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let fetched: FeedInfo = serde_json::from_slice(&body).expect("parse");
        assert_eq!(fetched.name, "eth-prices");
        assert_eq!(fetched.agent_id, "agent-1");
    }

    #[tokio::test]
    async fn paid_feed_payment_cell_challenges_invalid_requests_and_accepts_sufficient_value() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let state = test_state(dir.path().to_path_buf());
        let feed_id = state.feeds.write().await.register(FeedInfo {
            id: String::new(),
            cell_id: String::new(),
            name: "paid-signals".into(),
            kind: FeedKind::Derived,
            access: FeedAccess::Paid,
            agent_id: "provider-7".into(),
            description: "paid".into(),
            schema: None,
            pricing: Some(FeedPricingConfig {
                tier: PricingTier::Standard,
                per_request_cost: 2.25,
                session_pricing: None,
                protocol: PaymentProtocol::X402,
            }),
            created_at: Utc::now(),
        });
        let app = routes().with_state(state);
        let uri = format!("/feeds/{feed_id}");

        let missing = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&uri)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(missing.status(), StatusCode::PAYMENT_REQUIRED);
        assert!(missing.headers().contains_key("x-payment-request"));
        let body = to_bytes(missing.into_body(), usize::MAX)
            .await
            .expect("body");
        let challenge: PaymentRequest = serde_json::from_slice(&body).expect("payment challenge");
        assert_eq!(challenge.recipient, "provider-7");
        assert_eq!(challenge.amount, 3);

        let malformed = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&uri)
                    .header("x-payment-authorization", "not-json")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(malformed.status(), StatusCode::PAYMENT_REQUIRED);

        let authorization = |value| {
            serde_json::to_string(&PaymentAuthorization {
                from: "subscriber-4".into(),
                to: "provider-7".into(),
                value,
                valid_after: 0,
                valid_before: u64::MAX,
                nonce: 99,
                v: 27,
                r: [0; 32],
                s: [1; 32],
            })
            .expect("serialize authorization")
        };
        let underpaid = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&uri)
                    .header("x-payment-authorization", authorization(2))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(underpaid.status(), StatusCode::PAYMENT_REQUIRED);

        let paid = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&uri)
                    .header("x-payment-authorization", authorization(3))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(paid.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn public_and_private_feed_reads_bypass_payment_cell() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let state = test_state(dir.path().to_path_buf());
        let mut ids = Vec::new();
        for access in [FeedAccess::Public, FeedAccess::Private] {
            ids.push(state.feeds.write().await.register(FeedInfo {
                id: String::new(),
                cell_id: String::new(),
                name: format!("{access:?}-feed"),
                kind: FeedKind::Raw,
                access,
                agent_id: "provider".into(),
                description: String::new(),
                schema: None,
                pricing: None,
                created_at: Utc::now(),
            }));
        }
        let app = routes().with_state(state);

        for id in ids {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri(format!("/feeds/{id}"))
                        .body(Body::empty())
                        .expect("request"),
                )
                .await
                .expect("response");
            assert_eq!(response.status(), StatusCode::OK);
        }
    }

    #[tokio::test]
    async fn list_feeds_with_kind_filter() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let state = test_state(dir.path().to_path_buf());
        let app = routes().with_state(Arc::clone(&state));

        // Create two feeds of different kinds.
        for (name, kind) in [("raw-feed", "raw"), ("derived-feed", "derived")] {
            let _ = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/feeds")
                        .header("content-type", "application/json")
                        .body(Body::from(
                            serde_json::to_string(&serde_json::json!({
                                "name": name,
                                "kind": kind,
                                "agent_id": "agent-x"
                            }))
                            .unwrap(),
                        ))
                        .expect("request"),
                )
                .await
                .expect("response");
        }

        // Filter by kind=raw.
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/feeds?kind=raw")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: FeedListResponse = serde_json::from_slice(&body).expect("parse");
        assert_eq!(payload.total, 1);
        assert_eq!(payload.feeds[0].feed.name, "raw-feed");
    }

    #[tokio::test]
    async fn list_feeds_with_agent_filter() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let state = test_state(dir.path().to_path_buf());
        let app = routes().with_state(Arc::clone(&state));

        // Create feeds from different agents.
        for (name, agent) in [("f1", "agent-a"), ("f2", "agent-b"), ("f3", "agent-a")] {
            let _ = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/feeds")
                        .header("content-type", "application/json")
                        .body(Body::from(
                            serde_json::to_string(&serde_json::json!({
                                "name": name,
                                "kind": "raw",
                                "agent_id": agent
                            }))
                            .unwrap(),
                        ))
                        .expect("request"),
                )
                .await
                .expect("response");
        }

        // Filter by agent_id=agent-a.
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/feeds?agent_id=agent-a")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: FeedListResponse = serde_json::from_slice(&body).expect("parse");
        assert_eq!(payload.total, 2);
    }

    #[tokio::test]
    async fn delete_feed_success() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let state = test_state(dir.path().to_path_buf());
        let app = routes().with_state(Arc::clone(&state));

        // Create first.
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/feeds")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"temp","kind":"meta","agent_id":"a1"}"#,
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let created: CreateFeedResponse = serde_json::from_slice(&body).expect("parse");

        // Delete.
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(&format!("/feeds/{}", created.id))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: DeleteFeedResponse = serde_json::from_slice(&body).expect("parse");
        assert!(payload.deleted);
    }

    #[tokio::test]
    async fn delete_feed_not_found() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let state = test_state(dir.path().to_path_buf());

        let response = routes()
            .with_state(state)
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/feeds/feed-999")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_feed_not_found() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let state = test_state(dir.path().to_path_buf());

        let response = routes()
            .with_state(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/feeds/feed-999")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    // ── Runtime feed tests ───────────────────────────────────────

    #[tokio::test]
    async fn list_runtime_feeds_returns_three_entries() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let state = test_state(dir.path().to_path_buf());

        let response = routes()
            .with_state(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/feeds/runtime")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let feeds: Vec<roko_core::FeedRuntimeStatus> =
            serde_json::from_slice(&body).expect("parse");
        assert_eq!(feeds.len(), 3);
        assert!(
            feeds
                .iter()
                .any(|feed| feed.id == "file-watch-roko-dir" && feed.topic == "fs.changed")
        );
        assert!(
            feeds
                .iter()
                .any(|feed| feed.id == "provider-health-feed" && feed.topic == "provider.health")
        );
        assert!(
            feeds
                .iter()
                .any(|feed| feed.id == "episode-outcome-feed" && feed.topic == "episode.outcome")
        );
    }

    #[tokio::test]
    async fn get_runtime_feed_status_found() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let state = test_state(dir.path().to_path_buf());

        let response = routes()
            .with_state(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/feeds/runtime/file-watch-roko-dir")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let status: roko_core::FeedRuntimeStatus = serde_json::from_slice(&body).expect("parse");
        assert_eq!(status.id, "file-watch-roko-dir");
        assert_eq!(status.kind, "Raw");
        // AppState construction registers feeds; ServerBuilder starts them.
        assert!(!status.connected);
    }

    #[tokio::test]
    async fn get_runtime_feed_status_not_found() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let state = test_state(dir.path().to_path_buf());

        let response = routes()
            .with_state(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/feeds/runtime/nonexistent")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn lifecycle_health_and_search_are_live() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let state = test_state(dir.path().to_path_buf());
        let app = routes().with_state(state);

        let started = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/feeds/start/file-watch-roko-dir")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(started.status(), StatusCode::ACCEPTED);
        tokio::task::yield_now().await;

        let health = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/feeds/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(health.status(), StatusCode::OK);
        let body = to_bytes(health.into_body(), usize::MAX).await.unwrap();
        let statuses: Vec<roko_core::FeedRuntimeStatus> = serde_json::from_slice(&body).unwrap();
        assert!(statuses.iter().any(|feed| feed.id == "file-watch-roko-dir"));

        let search = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/feeds/search?q=health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = to_bytes(search.into_body(), usize::MAX).await.unwrap();
        let results: Vec<FeedInfo> = serde_json::from_slice(&body).unwrap();
        assert!(results.iter().any(|feed| feed.id == "provider-health-feed"));

        let stopped = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/feeds/stop/file-watch-roko-dir")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(stopped.status(), StatusCode::OK);
    }
}
