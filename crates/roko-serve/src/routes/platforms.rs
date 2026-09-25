//! Chat-platform adapter status routes (#414/#224).
//!
//! - `GET /api/platforms`       — list all registered platform adapters
//! - `GET /api/platforms/{id}`  — get status snapshot for a single adapter

use std::sync::Arc;

use axum::Router;
use axum::extract::{Path, State};
use axum::routing::get;
use roko_runtime::PlatformSnapshot;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::state::AppState;

/// Build the platforms router.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/platforms", get(list_platforms))
        .route("/platforms/{id}", get(get_platform))
}

#[derive(Serialize, Deserialize)]
struct PlatformListResponse {
    platforms: Vec<PlatformSnapshot>,
    total: usize,
}

/// `GET /api/platforms` — list all registered platform adapters.
async fn list_platforms(State(state): State<Arc<AppState>>) -> axum::Json<PlatformListResponse> {
    let platforms = state.platform_registry.list_snapshots().await;
    let total = platforms.len();
    axum::Json(PlatformListResponse { platforms, total })
}

/// `GET /api/platforms/{id}` — get status snapshot for a single adapter.
async fn get_platform(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<axum::Json<PlatformSnapshot>, ApiError> {
    state
        .platform_registry
        .snapshot(&id)
        .await
        .map(axum::Json)
        .ok_or_else(|| ApiError::not_found(format!("platform `{id}` not found")))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use crate::deploy::create_backend;
    use crate::runtime::NoOpRuntime;
    use roko_core::config::schema::RokoConfig;

    fn test_state() -> Arc<AppState> {
        let dir = tempfile::tempdir().expect("tempdir");
        let deploy_backend =
            Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
        Arc::new(
            AppState::new(
                dir.path().to_path_buf(),
                Arc::new(NoOpRuntime),
                RokoConfig::default(),
                deploy_backend,
            )
            .expect("AppState::new"),
        )
    }

    #[tokio::test]
    async fn list_platforms_empty() {
        let state = test_state();
        let app = Router::new()
            .merge(routes())
            .with_state(Arc::clone(&state));

        let req = Request::builder()
            .uri("/platforms")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn get_platform_not_found() {
        let state = test_state();
        let app = Router::new()
            .merge(routes())
            .with_state(Arc::clone(&state));

        let req = Request::builder()
            .uri("/platforms/nonexistent")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
