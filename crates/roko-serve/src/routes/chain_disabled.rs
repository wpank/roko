//! Stable diagnostics for chain RPC routes in a lean build.

use std::sync::Arc;

use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/chain/agents", get(disabled))
        .route("/chain/bounties", get(disabled))
        .route("/chain/status", get(disabled))
        .route("/chain/blocks", get(disabled))
        .route("/chain/transactions", get(disabled))
        .route("/chain/events", get(disabled))
        .route("/chain/watcher", get(disabled))
}

async fn disabled() -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "error": "chain RPC support is not included in this build",
            "required_feature": "alloy-backend",
            "hint": "rebuild roko with `--features alloy-backend`",
        })),
    )
}
