//! Live extension status routes.
//!
//! - `GET /api/extensions` — list loaded extensions.
//! - `GET /api/extensions/{name}` — inspect one loaded extension.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use roko_core::extension::{ExtensionLayer, ExtensionStatus, PackageTier};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/extensions", get(list_extensions))
        .route("/extensions/{name}", get(get_extension))
}

/// Circuit-breaker state for one loaded extension.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionHealth {
    /// Whether failures have disabled this extension.
    pub disabled: bool,
    /// Number of consecutive hook failures.
    pub consecutive_failures: u32,
}

/// Public metadata and live health for one extension.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtensionInfo {
    pub name: String,
    pub layer: ExtensionLayer,
    pub tier: PackageTier,
    pub version: String,
    pub optional: bool,
    pub health: ExtensionHealth,
}

impl From<ExtensionStatus> for ExtensionInfo {
    fn from(status: ExtensionStatus) -> Self {
        Self {
            name: status.meta.name,
            layer: status.meta.layer,
            tier: status.meta.tier,
            version: status.meta.version,
            optional: status.meta.optional,
            health: ExtensionHealth {
                disabled: status.disabled,
                consecutive_failures: status.consecutive_failures,
            },
        }
    }
}

/// Response body for `GET /api/extensions`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtensionListResponse {
    pub extensions: Vec<ExtensionInfo>,
    pub total: usize,
}

async fn extension_snapshot(state: &AppState) -> Vec<ExtensionInfo> {
    if let Some(chain) = state.runtime.extension_chain(&state.workdir) {
        return chain
            .lock()
            .await
            .statuses()
            .into_iter()
            .map(ExtensionInfo::from)
            .collect();
    }

    // A runtime without a chain has no loaded extensions. Do not synthesize
    // layer, tier, or health claims from configuration names alone.
    Vec::new()
}

async fn list_extensions(State(state): State<Arc<AppState>>) -> Json<ExtensionListResponse> {
    let extensions = extension_snapshot(&state).await;
    let total = extensions.len();
    Json(ExtensionListResponse { extensions, total })
}

async fn get_extension(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<ExtensionInfo>, (StatusCode, Json<serde_json::Value>)> {
    extension_snapshot(&state)
        .await
        .into_iter()
        .find(|extension| extension.name == name)
        .map(Json)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "extension_not_found",
                    "name": name,
                })),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    use async_trait::async_trait;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use roko_core::config::schema::RokoConfig;
    use roko_core::extension::{Extension, ExtensionChain, InferenceRequest};
    use tower::ServiceExt;

    use crate::deploy::create_backend;
    use crate::runtime::{CliRuntime, DashboardInfo, NoOpRuntime, RunResult, SessionStatusInfo};

    struct ChainRuntime {
        chain: Arc<tokio::sync::Mutex<ExtensionChain>>,
    }

    #[async_trait]
    impl CliRuntime for ChainRuntime {
        async fn run_once(
            &self,
            _workdir: &std::path::Path,
            _prompt: &str,
        ) -> anyhow::Result<RunResult> {
            unreachable!("not used by extension route tests")
        }

        fn session_status(&self, workdir: std::path::PathBuf) -> SessionStatusInfo {
            SessionStatusInfo {
                session_id: None,
                workdir,
                daemon_running: false,
                signal_count: None,
                episode_count: None,
                last_episode_passed: None,
            }
        }

        fn dashboard_scaffold(&self, _workdir: &std::path::Path) -> DashboardInfo {
            DashboardInfo {
                rendered: String::new(),
            }
        }

        fn extension_chain(
            &self,
            _workdir: &std::path::Path,
        ) -> Option<Arc<tokio::sync::Mutex<ExtensionChain>>> {
            Some(Arc::clone(&self.chain))
        }
    }

    struct FailingExtension;

    #[async_trait]
    impl Extension for FailingExtension {
        fn name(&self) -> &str {
            "failing"
        }

        fn layer(&self) -> ExtensionLayer {
            ExtensionLayer::Cognition
        }

        async fn pre_inference(
            &self,
            _request: &mut InferenceRequest,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Err(std::io::Error::other("extension failed").into())
        }
    }

    fn test_state(workdir: std::path::PathBuf, config: RokoConfig) -> Arc<AppState> {
        let deploy_backend =
            Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
        Arc::new(
            AppState::new(workdir, Arc::new(NoOpRuntime), config, deploy_backend)
                .expect("AppState::new"),
        )
    }

    #[tokio::test]
    async fn extension_list_is_empty_when_runtime_has_no_loaded_chain() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let mut config = RokoConfig::default();
        config.agent.extensions = vec!["safety-guard".to_string()];
        let app = routes().with_state(test_state(dir.path().to_path_buf(), config));

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/extensions")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let payload: ExtensionListResponse = serde_json::from_slice(
            &to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body"),
        )
        .expect("parse");
        assert_eq!(payload.total, 0);
        assert!(payload.extensions.is_empty());
    }

    #[tokio::test]
    async fn missing_extension_returns_typed_not_found() {
        let dir = tempfile::tempdir().expect("tempdir");
        let response = routes()
            .with_state(test_state(dir.path().to_path_buf(), RokoConfig::default()))
            .oneshot(
                Request::builder()
                    .uri("/extensions/missing")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn detail_reports_live_circuit_breaker_state() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut chain = ExtensionChain::new();
        chain.add(Box::new(FailingExtension));
        let mut request = InferenceRequest {
            plan_id: "plan".into(),
            task: "task".into(),
            role: "test".into(),
            model: "test".into(),
            prompt_tokens: 0,
            extra: serde_json::Value::Null,
        };
        for _ in 0..5 {
            chain
                .run_pre_inference(&mut request)
                .await
                .expect("isolated hook failure");
        }
        let chain = Arc::new(tokio::sync::Mutex::new(chain));
        let deploy_backend =
            Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
        let state = Arc::new(
            AppState::new(
                dir.path().to_path_buf(),
                Arc::new(ChainRuntime { chain }),
                RokoConfig::default(),
                deploy_backend,
            )
            .expect("AppState::new"),
        );

        let response = routes()
            .with_state(state)
            .oneshot(
                Request::builder()
                    .uri("/extensions/failing")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let detail: ExtensionInfo = serde_json::from_slice(
            &to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body"),
        )
        .expect("parse");
        assert!(detail.health.disabled);
        assert_eq!(detail.health.consecutive_failures, 5);
    }
}
