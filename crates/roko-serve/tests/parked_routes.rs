//! Parked route groups answer a typed 501 that names the cargo feature which
//! builds them, instead of a 404 (9214, 9219).

#![cfg(any(not(feature = "chain"), not(feature = "groups")))]
#![allow(clippy::expect_used, clippy::unwrap_used, missing_docs)]

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use roko_core::config::ServeAuthConfig;
use roko_core::config::schema::RokoConfig;
use roko_serve::deploy::create_backend;
use roko_serve::routes::build_router;
use roko_serve::runtime::{CliRuntime, DashboardInfo, RunResult, SessionStatusInfo};
use roko_serve::state::AppState;
use serde_json::Value;
use tempfile::tempdir;
use tower::ServiceExt;

struct TestRuntime;

#[async_trait::async_trait]
impl CliRuntime for TestRuntime {
    async fn run_once(
        &self,
        _workdir: &std::path::Path,
        _prompt: &str,
    ) -> anyhow::Result<RunResult> {
        Ok(RunResult {
            success: true,
            output_text: None,
            usage: None,
            gate_results: Vec::new(),
        })
    }

    fn session_status(&self, workdir: PathBuf) -> SessionStatusInfo {
        SessionStatusInfo {
            session_id: None,
            workdir,
            daemon_running: false,
            signal_count: Some(0),
            episode_count: Some(0),
            last_episode_passed: None,
        }
    }

    fn dashboard_scaffold(&self, _workdir: &std::path::Path) -> DashboardInfo {
        DashboardInfo {
            rendered: String::new(),
        }
    }
}

fn test_router() -> (tempfile::TempDir, axum::Router) {
    let dir = tempdir().expect("tempdir");
    let config = RokoConfig::default();
    let deploy = Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
    let state = Arc::new(
        AppState::new(
            dir.path().to_path_buf(),
            Arc::new(TestRuntime),
            config,
            deploy,
        )
        .expect("AppState::new"),
    );
    let auth = ServeAuthConfig {
        enabled: false,
        ..ServeAuthConfig::default()
    };
    let router = build_router(Arc::clone(&state), &[], auth);
    (dir, router)
}

/// Send `method uri` and return the status and the JSON body.
async fn parked_answer(router: &axum::Router, method: Method, uri: &str) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .expect("build request");
    let response = router.clone().oneshot(request).await.expect("oneshot");
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    (status, json)
}

/// One path from each chain-family group: arenas, the marketplace, DeFi, the
/// registries and the Mirage JSON-RPC proxy.
#[cfg(not(feature = "chain"))]
#[tokio::test]
async fn chain_family_routes_return_501_without_chain_feature() {
    let (_dir, router) = test_router();
    let requests = [
        (Method::GET, "/api/arenas"),
        (Method::POST, "/api/arenas/demo/attempts"),
        (Method::GET, "/api/marketplace/browse"),
        (Method::POST, "/api/defi/bonds"),
        (Method::GET, "/api/registries/stats"),
        (Method::GET, "/api/rpc/health"),
    ];
    for (method, uri) in requests {
        let (status, body) = parked_answer(&router, method.clone(), uri).await;
        assert_eq!(status, StatusCode::NOT_IMPLEMENTED, "{method} {uri}");
        assert_eq!(body["required_feature"], "chain", "{method} {uri}: {body}");
        assert!(
            body["hint"]
                .as_str()
                .is_some_and(|hint| hint.contains("--features chain")),
            "{method} {uri}: {body}"
        );
    }
}

/// Groups, their sub-resources and invitations are parked in a default build.
#[cfg(not(feature = "groups"))]
#[tokio::test]
async fn group_routes_return_501_by_default() {
    let (_dir, router) = test_router();
    let requests = [
        (Method::GET, "/api/groups"),
        (Method::POST, "/api/groups"),
        (Method::GET, "/api/groups/grp-1/members"),
        (Method::POST, "/api/groups/grp-1/pheromones"),
        (Method::POST, "/api/invitations/inv-1/accept"),
    ];
    for (method, uri) in requests {
        let (status, body) = parked_answer(&router, method.clone(), uri).await;
        assert_eq!(status, StatusCode::NOT_IMPLEMENTED, "{method} {uri}");
        assert_eq!(body["required_feature"], "groups", "{method} {uri}: {body}");
    }
}
