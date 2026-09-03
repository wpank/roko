//! Marketplace artifact API contracts.
//!
//! Durable artifact storage and search are not configured yet, so all endpoints
//! return 501 Not Implemented with an explicit contract shape.

use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/marketplace/browse", get(market_browse))
        .route("/marketplace/search", get(market_search))
        .route("/marketplace/artifacts/{ref}", get(show_artifact))
        .route("/marketplace/publish", post(publish_artifact))
        .route("/marketplace/fork", post(fork_artifact))
}

#[derive(Debug, Default, Deserialize)]
struct BrowseQuery {
    tab: Option<String>,
    kind: Option<String>,
    tags: Option<String>,
    capabilities: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: String,
}

async fn market_browse(Query(query): Query<BrowseQuery>) -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "status": "not_implemented",
            "artifacts": [],
            "total": 0,
            "tab": query.tab.unwrap_or_else(|| "all".to_owned()),
            "filters": {
                "kind": query.kind,
                "tags": query.tags,
                "capabilities": query.capabilities,
            },
            "message": "marketplace browse is not connected to durable storage",
        })),
    )
}

async fn market_search(Query(query): Query<SearchQuery>) -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "status": "not_implemented",
            "artifacts": [],
            "total": 0,
            "query": query.q,
            "message": "marketplace search is not connected to durable storage",
        })),
    )
}

async fn show_artifact(Path(artifact_ref): Path<String>) -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "status": "not_implemented",
            "artifact_ref": artifact_ref,
            "artifact": null,
            "found": false,
            "message": "marketplace artifact lookup is not connected to durable storage",
        })),
    )
}

async fn publish_artifact() -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "status": "not_implemented",
            "artifact_ref": null,
            "message": "artifact publishing is not connected to durable storage",
        })),
    )
}

async fn fork_artifact() -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "status": "not_implemented",
            "artifact_ref": null,
            "message": "artifact forking is not connected to durable storage",
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    #[tokio::test]
    async fn marketplace_mutation_stubs_have_contract_status_codes() {
        assert_eq!(
            publish_artifact().await.into_response().status(),
            StatusCode::NOT_IMPLEMENTED
        );
        assert_eq!(
            fork_artifact().await.into_response().status(),
            StatusCode::NOT_IMPLEMENTED
        );
    }

    #[tokio::test]
    async fn marketplace_browse_stub_preserves_requested_facets() {
        let (status, Json(body)) = market_browse(Query(BrowseQuery {
            tab: Some("featured".to_owned()),
            kind: Some("graph".to_owned()),
            tags: Some("review,strict".to_owned()),
            capabilities: Some("filesystem.read".to_owned()),
        }))
        .await;
        assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
        assert_eq!(body["status"], "not_implemented");
        assert_eq!(body["artifacts"], json!([]));
        assert_eq!(body["total"], 0);
        assert_eq!(body["tab"], "featured");
        assert_eq!(body["filters"]["kind"], "graph");
    }
}
