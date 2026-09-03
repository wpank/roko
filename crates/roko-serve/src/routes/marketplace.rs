//! Marketplace artifact API contracts.
//!
//! Durable artifact storage and search are not configured yet, so all endpoints
//! return 501 Not Implemented with the standard [`ApiError`] envelope.

use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;

use crate::error::ApiError;
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
    #[allow(dead_code)]
    tab: Option<String>,
    #[allow(dead_code)]
    kind: Option<String>,
    #[allow(dead_code)]
    tags: Option<String>,
    #[allow(dead_code)]
    capabilities: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    #[allow(dead_code)]
    q: String,
}

async fn market_browse(Query(_query): Query<BrowseQuery>) -> Result<(), ApiError> {
    Err(ApiError::not_implemented(
        "marketplace browse is not connected to durable storage",
        "Phase 2",
        "durable artifact storage and search must be configured first",
    ))
}

async fn market_search(Query(_query): Query<SearchQuery>) -> Result<(), ApiError> {
    Err(ApiError::not_implemented(
        "marketplace search is not connected to durable storage",
        "Phase 2",
        "durable artifact storage and search must be configured first",
    ))
}

async fn show_artifact(Path(_artifact_ref): Path<String>) -> Result<(), ApiError> {
    Err(ApiError::not_implemented(
        "marketplace artifact lookup is not connected to durable storage",
        "Phase 2",
        "durable artifact storage and search must be configured first",
    ))
}

async fn publish_artifact() -> Result<(), ApiError> {
    Err(ApiError::not_implemented(
        "artifact publishing is not connected to durable storage",
        "Phase 2",
        "durable artifact storage, signing, and publish pipeline must be configured first",
    ))
}

async fn fork_artifact() -> Result<(), ApiError> {
    Err(ApiError::not_implemented(
        "artifact forking is not connected to durable storage",
        "Phase 2",
        "durable artifact storage, signing, and fork pipeline must be configured first",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use serde_json::Value;

    /// Helper: call a handler and return (status, parsed JSON body).
    async fn call_handler(
        response: axum::response::Response,
    ) -> (StatusCode, Value) {
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let json: Value = serde_json::from_slice(&body).expect("parse JSON body");
        (status, json)
    }

    /// Assert the standard not-implemented error envelope shape.
    fn assert_not_implemented_envelope(status: StatusCode, body: &Value) {
        assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
        assert_eq!(body["code"], "not_implemented");
        assert!(
            body["message"].as_str().map_or(false, |m| !m.is_empty()),
            "message must be a non-empty string"
        );
        let details = &body["details"];
        assert!(
            details["phase"].as_str().map_or(false, |p| !p.is_empty()),
            "details.phase must be a non-empty string"
        );
        assert!(
            details["hint"].as_str().map_or(false, |h| !h.is_empty()),
            "details.hint must be a non-empty string"
        );
        // No success-shaped fields may appear.
        assert!(body.get("artifacts").is_none(), "must not contain artifacts");
        assert!(body.get("total").is_none(), "must not contain total");
        assert!(body.get("found").is_none(), "must not contain found");
        assert!(
            body.get("artifact_ref").is_none(),
            "must not contain artifact_ref"
        );
        assert!(body.get("stub").is_none(), "must not contain stub");
    }

    #[tokio::test]
    async fn browse_returns_501_with_standard_envelope() {
        let response = market_browse(Query(BrowseQuery::default()))
            .await
            .unwrap_err()
            .into_response();
        let (status, body) = call_handler(response).await;
        assert_not_implemented_envelope(status, &body);
    }

    #[tokio::test]
    async fn search_returns_501_with_standard_envelope() {
        let response = market_search(Query(SearchQuery {
            q: "test".to_owned(),
        }))
        .await
        .unwrap_err()
        .into_response();
        let (status, body) = call_handler(response).await;
        assert_not_implemented_envelope(status, &body);
    }

    #[tokio::test]
    async fn show_artifact_returns_501_with_standard_envelope() {
        let response = show_artifact(Path("test-ref".to_owned()))
            .await
            .unwrap_err()
            .into_response();
        let (status, body) = call_handler(response).await;
        assert_not_implemented_envelope(status, &body);
    }

    #[tokio::test]
    async fn publish_returns_501_with_standard_envelope() {
        let response = publish_artifact().await.unwrap_err().into_response();
        let (status, body) = call_handler(response).await;
        assert_not_implemented_envelope(status, &body);
    }

    #[tokio::test]
    async fn fork_returns_501_with_standard_envelope() {
        let response = fork_artifact().await.unwrap_err().into_response();
        let (status, body) = call_handler(response).await;
        assert_not_implemented_envelope(status, &body);
    }

    #[tokio::test]
    async fn mutation_stubs_never_return_2xx() {
        // Publish must never return 201/200.
        let publish_status = publish_artifact()
            .await
            .unwrap_err()
            .into_response()
            .status();
        assert!(
            !publish_status.is_success(),
            "publish must not return 2xx: got {publish_status}"
        );

        // Fork must never return 200.
        let fork_status = fork_artifact()
            .await
            .unwrap_err()
            .into_response()
            .status();
        assert!(
            !fork_status.is_success(),
            "fork must not return 2xx: got {fork_status}"
        );
    }
}
