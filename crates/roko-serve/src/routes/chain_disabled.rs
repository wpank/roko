//! Typed 501 answers for the route groups a build leaves out.
//!
//! A parked group answers every method on its paths with 501 and names the
//! cargo feature that builds it, so a client can tell "not in this build"
//! from "no such route" (9214). Without `alloy-backend` the `/chain/*` RPC
//! routes answer this way, and without `chain` the chain-family routes do.

use std::sync::Arc;

use axum::http::StatusCode;
#[cfg(not(feature = "chain"))]
use axum::routing::any;
#[cfg(not(feature = "alloy-backend"))]
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::state::AppState;

/// The `/chain/*` RPC routes, which need `alloy-backend`.
#[cfg(not(feature = "alloy-backend"))]
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

#[cfg(not(feature = "alloy-backend"))]
async fn disabled() -> (StatusCode, Json<Value>) {
    parked("chain RPC support", "alloy-backend")
}

/// The chain-family routes, which need `chain`: arenas, the marketplace,
/// DeFi, the registries and the Mirage JSON-RPC proxy, every sub-path.
#[cfg(not(feature = "chain"))]
pub(crate) fn chain_family_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/arenas", any(chain_family_parked))
        .route("/arenas/{*path}", any(chain_family_parked))
        .route("/defi/{*path}", any(chain_family_parked))
        .route("/marketplace/{*path}", any(chain_family_parked))
        .route("/registries/{*path}", any(chain_family_parked))
        .route("/rpc", any(chain_family_parked))
        .route("/rpc/{*path}", any(chain_family_parked))
}

#[cfg(not(feature = "chain"))]
async fn chain_family_parked() -> (StatusCode, Json<Value>) {
    parked("chain support", "chain")
}

/// The answer of a parked route: `what` is not included in this build, and
/// `feature` is the cargo feature that adds it.
pub(crate) fn parked(what: &str, feature: &str) -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "error": format!("{what} is not included in this build"),
            "required_feature": feature,
            "hint": format!("rebuild roko with `--features {feature}`"),
        })),
    )
}
