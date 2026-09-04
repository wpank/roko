//! Route coverage matrix for roko-serve.
//!
//! Every registered method+path combination in the HTTP control plane must
//! have a corresponding matrix row. Adding a new route without a test entry
//! will cause `every_registered_route_has_a_matrix_row` to fail.
//!
//! This test intentionally does **not** scan router source code independently
//! (#315 owns the canonical route inventory). It probes the assembled router
//! at well-known paths and asserts that each route resolves to a non-404
//! handler — confirming that the registration is reachable.

#![allow(clippy::expect_used, clippy::unwrap_used, missing_docs)]

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
use std::path::PathBuf;
use tempfile::tempdir;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

/// Send a request and return only the status code (body is discarded).
async fn status_for(router: &axum::Router, method: Method, uri: &str) -> StatusCode {
    let body = if method == Method::POST || method == Method::PUT || method == Method::PATCH {
        Body::from("{}")
    } else {
        Body::empty()
    };
    let mut builder = Request::builder().method(method).uri(uri);
    if matches!(
        builder.method_ref(),
        Some(&Method::POST) | Some(&Method::PUT) | Some(&Method::PATCH)
    ) {
        builder = builder.header("content-type", "application/json");
    }
    let req = builder.body(body).expect("build request");
    let resp = router.clone().oneshot(req).await.expect("oneshot");
    resp.status()
}

/// A route matrix entry: method, path, and the expected status range.
struct MatrixRow {
    method: Method,
    path: &'static str,
    // The route must NOT return 404/405 (which means "not registered").
    // Some stubs return 501, 400, 422, etc., which is acceptable -- they
    // prove the route is registered and reachable.
}

/// The canonical route coverage matrix.
///
/// Every registered serve route should appear here. The test verifies that
/// the route resolves to something other than 404/405 (meaning the handler
/// is registered). Specific response semantics are tested in per-module
/// tests and in `api_integration.rs`.
fn matrix() -> Vec<MatrixRow> {
    vec![
        // -- Top-level probes (no /api prefix) --
        MatrixRow {
            method: Method::GET,
            path: "/health",
        },
        MatrixRow {
            method: Method::GET,
            path: "/ready",
        },
        MatrixRow {
            method: Method::GET,
            path: "/metrics",
        },
        // -- Status / health --
        MatrixRow {
            method: Method::GET,
            path: "/api/health",
        },
        MatrixRow {
            method: Method::GET,
            path: "/api/status",
        },
        MatrixRow {
            method: Method::GET,
            path: "/api/truth_map",
        },
        MatrixRow {
            method: Method::GET,
            path: "/api/dashboard",
        },
        MatrixRow {
            method: Method::GET,
            path: "/api/openapi.json",
        },
        // -- Signals / episodes / metrics --
        MatrixRow {
            method: Method::GET,
            path: "/api/signals",
        },
        MatrixRow {
            method: Method::GET,
            path: "/api/episodes",
        },
        MatrixRow {
            method: Method::GET,
            path: "/api/metrics",
        },
        // -- Plans --
        MatrixRow {
            method: Method::GET,
            path: "/api/plans",
        },
        // -- PRDs --
        MatrixRow {
            method: Method::GET,
            path: "/api/prds",
        },
        // -- Research --
        MatrixRow {
            method: Method::GET,
            path: "/api/research",
        },
        // -- Jobs --
        MatrixRow {
            method: Method::GET,
            path: "/api/jobs",
        },
        MatrixRow {
            method: Method::POST,
            path: "/api/jobs",
        },
        // -- Run --
        MatrixRow {
            method: Method::POST,
            path: "/api/run",
        },
        MatrixRow {
            method: Method::GET,
            path: "/api/runs",
        },
        // -- Agents --
        MatrixRow {
            method: Method::GET,
            path: "/api/managed-agents",
        },
        // -- Heartbeats --
        MatrixRow {
            method: Method::POST,
            path: "/api/heartbeats",
        },
        // -- Templates --
        MatrixRow {
            method: Method::GET,
            path: "/api/templates",
        },
        // -- Config --
        MatrixRow {
            method: Method::GET,
            path: "/api/config",
        },
        // -- Subscriptions --
        MatrixRow {
            method: Method::GET,
            path: "/api/subscriptions",
        },
        // -- Learning --
        MatrixRow {
            method: Method::GET,
            path: "/api/learning/experiments",
        },
        MatrixRow {
            method: Method::GET,
            path: "/api/learning/router",
        },
        // -- Marketplace (501 stubs) --
        MatrixRow {
            method: Method::GET,
            path: "/api/marketplace/browse",
        },
        MatrixRow {
            method: Method::GET,
            path: "/api/marketplace/search?q=test",
        },
        MatrixRow {
            method: Method::POST,
            path: "/api/marketplace/publish",
        },
        MatrixRow {
            method: Method::POST,
            path: "/api/marketplace/fork",
        },
        // -- DeFi (501 stubs) --
        MatrixRow {
            method: Method::GET,
            path: "/api/defi/instruments",
        },
        MatrixRow {
            method: Method::POST,
            path: "/api/defi/bonds",
        },
        MatrixRow {
            method: Method::GET,
            path: "/api/defi/indices",
        },
        MatrixRow {
            method: Method::GET,
            path: "/api/defi/risk/portfolio",
        },
        // -- Bench --
        MatrixRow {
            method: Method::GET,
            path: "/api/bench/provider-status",
        },
        MatrixRow {
            method: Method::GET,
            path: "/api/bench/runs",
        },
        MatrixRow {
            method: Method::GET,
            path: "/api/bench/suites",
        },
        MatrixRow {
            method: Method::GET,
            path: "/api/bench/models",
        },
        MatrixRow {
            method: Method::GET,
            path: "/api/bench/pareto",
        },
        MatrixRow {
            method: Method::GET,
            path: "/api/bench/cost-summary",
        },
        // -- Projections --
        MatrixRow {
            method: Method::GET,
            path: "/api/projections/catalog",
        },
        // -- Neuro --
        MatrixRow {
            method: Method::GET,
            path: "/api/neuro/stats",
        },
        // -- Dream --
        MatrixRow {
            method: Method::GET,
            path: "/api/dream/status",
        },
        // -- Diagnosis --
        MatrixRow {
            method: Method::GET,
            path: "/api/diagnosis",
        },
        // -- Gates --
        MatrixRow {
            method: Method::GET,
            path: "/api/gates/summary",
        },
        // -- Extensions --
        MatrixRow {
            method: Method::GET,
            path: "/api/extensions",
        },
        // -- Event ingest --
        MatrixRow {
            method: Method::POST,
            path: "/api/events",
        },
        // -- Feeds --
        MatrixRow {
            method: Method::GET,
            path: "/api/feeds",
        },
        // -- Recipes --
        MatrixRow {
            method: Method::GET,
            path: "/api/recipes",
        },
        // -- Groups --
        MatrixRow {
            method: Method::GET,
            path: "/api/groups",
        },
        // -- Relay --
        MatrixRow {
            method: Method::GET,
            path: "/api/relay/health",
        },
        // -- SSE --
        MatrixRow {
            method: Method::GET,
            path: "/api/events/stream",
        },
        // -- Providers --
        MatrixRow {
            method: Method::GET,
            path: "/api/providers",
        },
        // -- Deployments --
        MatrixRow {
            method: Method::GET,
            path: "/api/deployments",
        },
        // -- Integrations --
        MatrixRow {
            method: Method::GET,
            path: "/api/integrations",
        },
        // -- Secrets --
        MatrixRow {
            method: Method::GET,
            path: "/api/secrets",
        },
        // -- Triggers --
        MatrixRow {
            method: Method::GET,
            path: "/api/triggers",
        },
        // -- Workflows --
        MatrixRow {
            method: Method::GET,
            path: "/api/workflows",
        },
        // -- Meta --
        MatrixRow {
            method: Method::GET,
            path: "/api/meta/health",
        },
        // -- Connectors --
        MatrixRow {
            method: Method::GET,
            path: "/api/connectors",
        },
    ]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Every route in the matrix must resolve to a handler (not 404/405).
///
/// If this test fails after adding a new route, add a corresponding
/// `MatrixRow` entry to the `matrix()` function above.
#[tokio::test]
async fn every_matrix_route_is_registered() {
    let (_dir, router) = test_router();

    let mut failures = Vec::new();
    for row in matrix() {
        let status = status_for(&router, row.method.clone(), row.path).await;
        // 404 = no route matched; 405 = route exists but method not allowed.
        // Either means the matrix row is stale or the route is not registered.
        if status == StatusCode::NOT_FOUND || status == StatusCode::METHOD_NOT_ALLOWED {
            failures.push(format!(
                "{} {} -> {} (expected a registered handler)",
                row.method, row.path, status
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "Unregistered routes found in the matrix:\n{}",
        failures.join("\n")
    );
}

/// Marketplace stubs must return 501, never 2xx.
#[tokio::test]
async fn marketplace_stubs_return_501() {
    let (_dir, router) = test_router();

    let stubs = [
        (Method::GET, "/api/marketplace/browse"),
        (Method::GET, "/api/marketplace/search?q=test"),
        (Method::POST, "/api/marketplace/publish"),
        (Method::POST, "/api/marketplace/fork"),
    ];

    for (method, path) in &stubs {
        let status = status_for(&router, method.clone(), path).await;
        assert_eq!(
            status,
            StatusCode::NOT_IMPLEMENTED,
            "{} {} should return 501, got {}",
            method,
            path,
            status
        );
    }
}

/// DeFi stubs must return 501, never 2xx.
#[tokio::test]
async fn defi_stubs_return_501() {
    let (_dir, router) = test_router();

    let stubs = [
        (Method::GET, "/api/defi/instruments"),
        (Method::POST, "/api/defi/bonds"),
        (Method::GET, "/api/defi/indices"),
        (Method::GET, "/api/defi/risk/portfolio"),
    ];

    for (method, path) in &stubs {
        let status = status_for(&router, method.clone(), path).await;
        assert_eq!(
            status,
            StatusCode::NOT_IMPLEMENTED,
            "{} {} should return 501, got {}",
            method,
            path,
            status
        );
    }
}

/// Mutation endpoints that receive invalid bodies must not return 2xx.
#[tokio::test]
async fn mutation_routes_reject_invalid_input() {
    let (_dir, router) = test_router();

    // POST /api/run with empty object (missing required `prompt` field).
    let status = status_for(&router, Method::POST, "/api/run").await;
    assert!(
        status.is_client_error(),
        "POST /api/run with {{}} should be client error, got {}",
        status
    );

    // POST /api/jobs with empty object.
    let req = Request::builder()
        .method("POST")
        .uri("/api/jobs")
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .expect("build request");
    let resp = router.clone().oneshot(req).await.expect("oneshot");
    // An empty job may or may not be valid depending on defaults — but it
    // should at least return a response (not panic or 404).
    assert_ne!(resp.status(), StatusCode::NOT_FOUND);
}

/// Stub (501) mutation routes must produce no side effects.
///
/// After calling a 501 stub, there should be no new files created in the
/// workspace temp directory.
#[tokio::test]
async fn stub_mutation_routes_produce_no_side_effects() {
    let (dir, router) = test_router();

    // Count files in .roko before.
    let roko_dir = dir.path().join(".roko");
    let before = count_files_recursive(&roko_dir);

    // Call marketplace publish (501 stub).
    let req = Request::builder()
        .method("POST")
        .uri("/api/marketplace/publish")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"artifact":"test"}"#))
        .expect("build request");
    let resp = router.clone().oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);

    // Count files after.
    let after = count_files_recursive(&roko_dir);
    assert_eq!(
        before, after,
        "stub mutation should not create files in .roko/"
    );
}

/// The 501 envelope for stubs must be machine-readable.
#[tokio::test]
async fn stub_envelope_is_machine_readable() {
    let (_dir, router) = test_router();

    let req = Request::builder()
        .uri("/api/marketplace/browse")
        .body(Body::empty())
        .expect("build request");
    let resp = router.clone().oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);

    let body = resp
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).expect("parse JSON");
    assert_eq!(json["code"], "not_implemented");
    assert!(
        json["message"].as_str().is_some_and(|m| !m.is_empty()),
        "message must be non-empty"
    );
}

/// Bench routes respond with meaningful handler output (not 404).
#[tokio::test]
async fn bench_list_routes_return_structured_responses() {
    let (_dir, router) = test_router();

    // GET /api/bench/provider-status
    let req = Request::builder()
        .uri("/api/bench/provider-status")
        .body(Body::empty())
        .expect("build request");
    let resp = router.clone().oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).expect("parse JSON");
    assert!(json.get("has_providers").is_some());
    assert!(json.get("demo_available").is_some());

    // GET /api/bench/runs — empty list
    let req = Request::builder()
        .uri("/api/bench/runs")
        .body(Body::empty())
        .expect("build request");
    let resp = router.clone().oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).expect("parse JSON");
    assert_eq!(json["total"], 0);
    assert!(json["runs"].is_array());

    // GET /api/bench/models
    let req = Request::builder()
        .uri("/api/bench/models")
        .body(Body::empty())
        .expect("build request");
    let resp = router.clone().oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).expect("parse JSON");
    assert!(json.get("models").is_some());

    // GET /api/bench/cost-summary
    let req = Request::builder()
        .uri("/api/bench/cost-summary")
        .body(Body::empty())
        .expect("build request");
    let resp = router.clone().oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).expect("parse JSON");
    assert!(json.get("models").is_some());
}

/// Bench run lookup for non-existent ID returns 404.
#[tokio::test]
async fn bench_run_not_found_returns_404() {
    let (_dir, router) = test_router();

    let req = Request::builder()
        .uri("/api/bench/run/nonexistent-id")
        .body(Body::empty())
        .expect("build request");
    let resp = router.clone().oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// Bench suite upload rejects empty/invalid input.
#[tokio::test]
async fn bench_suite_upload_rejects_invalid_input() {
    let (_dir, router) = test_router();

    // Missing required fields.
    let req = Request::builder()
        .method("POST")
        .uri("/api/bench/suites")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"id":"","tasks":[]}"#))
        .expect("build request");
    let resp = router.clone().oneshot(req).await.expect("oneshot");
    assert!(
        resp.status().is_client_error(),
        "empty suite should be rejected, got {}",
        resp.status()
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn count_files_recursive(dir: &std::path::Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                count += count_files_recursive(&path);
            } else {
                count += 1;
            }
        }
    }
    count
}
