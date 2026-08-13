//! Extension listing route.
//!
//! - `GET /api/extensions` — list configured extensions with health state.

use std::sync::Arc;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/extensions", get(list_extensions))
}

/// Health state for an extension, following the circuit-breaker pattern used by
/// [`roko_core::extension::ExtensionHealthTracker`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionHealthState {
    /// Extension is healthy (consecutive failures below threshold).
    Healthy,
    /// Extension has been disabled by the circuit breaker.
    Disabled,
    /// Extension health is unknown (no runtime chain available).
    Unknown,
}

/// Summary information for a single extension returned by `GET /api/extensions`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtensionInfo {
    /// Extension name as declared in config.
    pub name: String,
    /// Pipeline layer this extension operates in (if known from a registered
    /// manifest), or `null` when only the name is available from config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layer: Option<roko_core::extension::ExtensionLayer>,
    /// Current health state.
    pub health: ExtensionHealthState,
    /// Number of consecutive failures recorded by the circuit breaker.
    /// Always 0 at the serve layer since no runtime chain is active here.
    pub consecutive_failures: u32,
}

/// Response body for `GET /api/extensions`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtensionListResponse {
    /// All configured extensions.
    pub extensions: Vec<ExtensionInfo>,
    /// Total number of extensions.
    pub total: usize,
}

/// `GET /api/extensions` — list extensions configured in `roko.toml`
/// (`[agent] extensions`) together with their health / circuit-breaker state.
///
/// Because the serve layer does not own a live [`ExtensionChain`], the
/// consecutive-failure count is always 0 and the health state is `healthy`
/// for every configured extension. Future work may expose per-agent runtime
/// chains through the aggregator, but for now this provides a configuration-
/// level inventory that dashboards and diagnostics can consume.
async fn list_extensions(State(state): State<Arc<AppState>>) -> Json<ExtensionListResponse> {
    let config = state.load_roko_config();
    let extension_names = &config.agent.extensions;

    let extensions: Vec<ExtensionInfo> = extension_names
        .iter()
        .map(|name| ExtensionInfo {
            name: name.clone(),
            layer: None,
            health: ExtensionHealthState::Healthy,
            consecutive_failures: 0,
        })
        .collect();

    let total = extensions.len();
    Json(ExtensionListResponse { extensions, total })
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use roko_core::config::schema::RokoConfig;
    use tower::ServiceExt;

    use crate::deploy::create_backend;
    use crate::runtime::NoOpRuntime;

    fn test_state(workdir: std::path::PathBuf) -> Arc<AppState> {
        let deploy_backend =
            Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
        Arc::new(
            AppState::new(
                workdir,
                Arc::new(NoOpRuntime),
                RokoConfig::default(),
                deploy_backend,
            )
            .expect("AppState::new"),
        )
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
    async fn extension_list_empty_by_default() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let state = test_state(dir.path().to_path_buf());

        let response = routes()
            .with_state(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/extensions")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: ExtensionListResponse = serde_json::from_slice(&body).expect("parse");
        assert!(payload.extensions.is_empty());
        assert_eq!(payload.total, 0);
    }

    #[tokio::test]
    async fn extension_list_returns_configured_extensions() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");

        let mut config = RokoConfig::default();
        config.agent.extensions = vec![
            "safety-guard".to_string(),
            "cost-tracker".to_string(),
            "memory-store".to_string(),
        ];

        let state = test_state_with_config(dir.path().to_path_buf(), config);

        let response = routes()
            .with_state(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/extensions")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: ExtensionListResponse = serde_json::from_slice(&body).expect("parse");
        assert_eq!(payload.total, 3);
        assert_eq!(payload.extensions.len(), 3);

        // Verify names match config order.
        assert_eq!(payload.extensions[0].name, "safety-guard");
        assert_eq!(payload.extensions[1].name, "cost-tracker");
        assert_eq!(payload.extensions[2].name, "memory-store");

        // All extensions should report healthy with zero failures.
        for ext in &payload.extensions {
            assert_eq!(ext.health, ExtensionHealthState::Healthy);
            assert_eq!(ext.consecutive_failures, 0);
            assert!(ext.layer.is_none());
        }
    }

    #[tokio::test]
    async fn extension_response_serializes_correctly() {
        let info = ExtensionInfo {
            name: "test-ext".to_string(),
            layer: Some(roko_core::extension::ExtensionLayer::Cognition),
            health: ExtensionHealthState::Disabled,
            consecutive_failures: 5,
        };

        let json = serde_json::to_value(&info).expect("serialize");
        assert_eq!(json["name"], "test-ext");
        assert_eq!(json["layer"], "cognition");
        assert_eq!(json["health"], "disabled");
        assert_eq!(json["consecutive_failures"], 5);
    }

    #[tokio::test]
    async fn extension_health_state_roundtrips() {
        for state in [
            ExtensionHealthState::Healthy,
            ExtensionHealthState::Disabled,
            ExtensionHealthState::Unknown,
        ] {
            let json = serde_json::to_string(&state).expect("serialize");
            let parsed: ExtensionHealthState = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(parsed, state);
        }
    }
}
