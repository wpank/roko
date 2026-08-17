//! Authenticated DeFi product API contract stubs.
//!
//! The chain crate provides local lifecycle and pricing primitives, but the
//! server has no durable DeFi adapter or execution/risk pipeline. These routes
//! therefore return an explicit structured 501 response.

use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;

use crate::state::AppState;

/// DeFi endpoint contract required by E41-T08.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/defi/instruments", get(not_implemented))
        .route("/defi/bonds", post(not_implemented))
        .route("/defi/bonds/{id}", get(not_implemented))
        .route("/defi/options/price", post(not_implemented))
        .route("/defi/insurance", post(not_implemented))
        .route("/defi/insurance/{id}/claims", post(not_implemented))
        .route("/defi/indices", get(not_implemented))
        .route("/defi/risk/portfolio", get(not_implemented))
}

#[derive(Debug, Serialize)]
struct NotImplementedResponse {
    status: &'static str,
    message: &'static str,
}

async fn not_implemented() -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(NotImplementedResponse {
            status: "not_implemented",
            message: "DeFi product endpoints are Phase 2",
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stub_response_is_explicit_and_machine_readable() {
        let response = not_implemented().await.into_response();
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }
}
