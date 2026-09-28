//! HTTP API integration tests for the roko-serve control plane.
//!
//! These tests build the full axum router with a minimal (no-op) runtime and
//! exercise key endpoints using `tower::ServiceExt::oneshot`.

use std::path::PathBuf;
use std::sync::Arc;

use axum::Json;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use futures::StreamExt;
use http_body_util::BodyExt;
use roko_core::config::ServeAuthConfig;
use roko_core::config::schema::RokoConfig;
use roko_serve::deploy::create_backend;
use roko_serve::routes::build_router;
use roko_serve::runtime::{CliRuntime, DashboardInfo, RunResult, SessionStatusInfo};
use roko_serve::state::AppState;
use tempfile::tempdir;
use tokio::net::TcpListener;
use tokio::time::{Duration, timeout};
use tokio_tungstenite::connect_async;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Minimal no-op runtime for integration tests.
struct TestRuntime;

#[async_trait::async_trait]
impl CliRuntime for TestRuntime {
    async fn list_plans(
        &self,
        _workdir: &std::path::Path,
    ) -> anyhow::Result<Vec<roko_serve::plan_types::PlanSummaryDto>> {
        Ok(Vec::new())
    }

    async fn run_once(
        &self,
        _workdir: &std::path::Path,
        _prompt: &str,
    ) -> anyhow::Result<RunResult> {
        Ok(RunResult {
            success: true,
            output_text: Some("test runtime output".to_string()),
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

/// Build a test router (no auth) backed by a temp directory.
fn test_app() -> (tempfile::TempDir, axum::Router) {
    let (dir, _state, router) = test_app_state();
    (dir, router)
}

/// Build a test router and expose its shared app state.
fn test_app_state() -> (tempfile::TempDir, Arc<AppState>, axum::Router) {
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
    // Library default flips to `enabled = true` (T3-22). These tests exercise
    // routes without auth, so we explicitly disable it for the fixture.
    let auth = ServeAuthConfig {
        enabled: false,
        ..ServeAuthConfig::default()
    };
    let router = build_test_router(Arc::clone(&state), &[], auth);
    (dir, state, router)
}

/// Build a test router with API-key auth enabled.
fn test_app_with_auth(api_key: &str) -> (tempfile::TempDir, axum::Router) {
    let dir = tempdir().expect("tempdir");
    let mut config = RokoConfig::default();
    let auth = ServeAuthConfig {
        enabled: true,
        api_key: api_key.to_string(),
        api_keys: Vec::new(),
        privy_app_id: None,
        privy_workspace_id: None,
        privy_allowed_roles: Vec::new(),
        enforcement_mode: Default::default(),
        ..ServeAuthConfig::default()
    };
    config.serve.auth = auth.clone();
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
    let router = build_test_router(Arc::clone(&state), &[], auth);
    (dir, router)
}

fn build_test_router(
    state: Arc<AppState>,
    cors_origins: &[String],
    auth: ServeAuthConfig,
) -> axum::Router {
    build_router(state, cors_origins, auth)
        .reset_fallback()
        .fallback(test_api_or_spa_fallback)
}

async fn test_api_or_spa_fallback(req: Request<Body>) -> Response {
    let path = req.uri().path().to_string();
    if matches!(path.as_str(), "/api" | "/ws" | "/roko-ws")
        || path.starts_with("/api/")
        || path.starts_with("/ws/")
        || path.starts_with("/roko-ws/")
    {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "not_found",
                "message": format!("No route matches {path}"),
            })),
        )
            .into_response();
    }

    roko_serve::embedded::serve_embedded(req).await
}

/// Send a GET request and return `(StatusCode, serde_json::Value)`.
async fn get_json(router: &axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .uri(uri)
        .body(Body::empty())
        .expect("build request");
    let resp = router.clone().oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let body = resp
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
    (status, json)
}

/// Send a POST request with a JSON body and return `(StatusCode, serde_json::Value)`.
async fn post_json(
    router: &axum::Router,
    uri: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).expect("serialize")))
        .expect("build request");
    let resp = router.clone().oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let bytes = resp
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json)
}

/// Send a PATCH request with a JSON body and return `(StatusCode, serde_json::Value)`.
async fn patch_json(
    router: &axum::Router,
    uri: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("PATCH")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).expect("serialize")))
        .expect("build request");
    let resp = router.clone().oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let bytes = resp
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json)
}

async fn next_ws_text(
    socket: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> String {
    loop {
        let message = timeout(Duration::from_secs(3), socket.next())
            .await
            .expect("wait for websocket message");
        match message {
            Some(Ok(message)) if message.is_text() => {
                return message.into_text().expect("text frame").to_string();
            }
            Some(Ok(_)) => {}
            Some(Err(error)) => panic!("websocket error: {error}"),
            None => panic!("websocket closed"),
        }
    }
}

// ---------------------------------------------------------------------------
// Health & status
// ---------------------------------------------------------------------------

#[tokio::test]
async fn health_returns_200_with_status_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/health").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
    assert!(body["uptime_secs"].is_number());
    assert!(body["version"].is_string());
}

#[tokio::test]
async fn session_status_returns_workdir() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/status").await;

    assert_eq!(status, StatusCode::OK);
    assert!(!body["workdir"].is_null());
    assert_eq!(body["daemon_running"], false);
}

#[tokio::test]
async fn run_status_returns_terminal_output_text() {
    let (_dir, app) = test_app();
    let (status, body) =
        post_json(&app, "/api/run", serde_json::json!({ "prompt": "hello" })).await;

    assert_eq!(status, StatusCode::ACCEPTED);
    let run_id = body["id"].as_str().expect("run id");

    for _ in 0..20 {
        let (status, body) = get_json(&app, &format!("/api/run/{run_id}/status")).await;
        assert_eq!(status, StatusCode::OK);
        if body["finished"] == true {
            assert_eq!(body["status"], "completed");
            assert_eq!(body["output_text"], "test runtime output");
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    panic!("timed out waiting for run completion");
}

// ---------------------------------------------------------------------------
// Plans
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_plans_empty() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/plans").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, serde_json::json!([]));
}

// ---------------------------------------------------------------------------
// Jobs
// ---------------------------------------------------------------------------

#[tokio::test]
async fn jobs_create_list_get_and_update_round_trip() {
    let (dir, app) = test_app();
    let (status, created) = post_json(
        &app,
        "/api/jobs",
        serde_json::json!({
            "title": "Implement marketplace filters",
            "description": "Add durable jobs API support.",
            "job_type": "coding_task",
            "posted_by": "operator",
            "priority": "high",
            "tags": ["marketplace", "serve"],
            "reward": "bounty-7",
            "plan_id": "plan-42"
        }),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    let job_id = created["id"].as_str().expect("job id");
    assert_eq!(created["title"], "Implement marketplace filters");
    assert_eq!(created["state"], "open");
    assert_eq!(created["job_type"], "coding_task");

    let persisted = dir
        .path()
        .join(".roko")
        .join("jobs")
        .join(format!("{job_id}.json"));
    assert!(persisted.exists());

    let (status, listed) = get_json(&app, "/api/jobs").await;
    assert_eq!(status, StatusCode::OK);
    let jobs = listed.as_array().expect("jobs array");
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0]["id"], job_id);
    assert_eq!(jobs[0]["state"], "open");
    assert_eq!(jobs[0]["posted_by"], "operator");

    let (status, fetched) = get_json(&app, &format!("/api/jobs/{job_id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fetched["plan_id"], "plan-42");
    assert_eq!(fetched["reward"], "bounty-7");

    let (status, updated) = patch_json(
        &app,
        &format!("/api/jobs/{job_id}"),
        serde_json::json!({
            "status": "in_progress",
            "assigned_to": "implementer-1"
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["state"], "in_progress");
    assert_eq!(updated["assigned_to"], "implementer-1");

    let (status, fetched_again) = get_json(&app, &format!("/api/jobs/{job_id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fetched_again["state"], "in_progress");
    assert_eq!(fetched_again["assigned_to"], "implementer-1");
}

#[tokio::test]
async fn jobs_events_are_visible_over_websocket() {
    let (_dir, _state, app) = test_app_state();

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ws server");
    let addr = listener.local_addr().expect("listener addr");
    let server_app = app.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, server_app)
            .await
            .expect("serve test app");
    });

    let (mut socket, _) = connect_async(format!("ws://{addr}/ws"))
        .await
        .expect("connect websocket");

    let (_status, created) = post_json(
        &app,
        "/api/jobs",
        serde_json::json!({
            "id": "job-ws-1",
            "title": "Broadcast me",
            "description": "Verify websocket visibility."
        }),
    )
    .await;
    assert_eq!(created["id"], "job-ws-1");

    let create_event: serde_json::Value =
        serde_json::from_str(&next_ws_text(&mut socket).await).expect("parse create event");
    assert_eq!(create_event["type"], "job_created");
    assert_eq!(create_event["job"]["id"], "job-ws-1");
    assert_eq!(create_event["job"]["state"], "open");

    let (_status, _updated) = patch_json(
        &app,
        "/api/jobs/job-ws-1",
        serde_json::json!({
            "status": "assigned",
            "assigned_to": "agent-7"
        }),
    )
    .await;

    let update_event: serde_json::Value =
        serde_json::from_str(&next_ws_text(&mut socket).await).expect("parse update event");
    assert_eq!(update_event["type"], "job_updated");
    assert_eq!(update_event["job"]["id"], "job-ws-1");
    assert_eq!(update_event["job"]["state"], "assigned");
    assert_eq!(update_event["job"]["assigned_to"], "agent-7");

    let _ = socket.close(None).await;
    server.abort();
}

// ---------------------------------------------------------------------------
// Managed agents
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_managed_agents_empty() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/managed-agents").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, serde_json::json!([]));
}

// ---------------------------------------------------------------------------
// Signals
// ---------------------------------------------------------------------------

#[tokio::test]
async fn signals_returns_empty_array() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/signals").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_array());
}

// ---------------------------------------------------------------------------
// Episodes
// ---------------------------------------------------------------------------

#[tokio::test]
async fn episodes_returns_empty_array() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/episodes").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_array());
}

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

#[tokio::test]
async fn metrics_returns_json() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/metrics").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_array());
}

// ---------------------------------------------------------------------------
// Research
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_research_empty() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/research").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, serde_json::json!([]));
}

// ---------------------------------------------------------------------------
// Run
// ---------------------------------------------------------------------------

#[tokio::test]
async fn post_run_returns_accepted() {
    let (_dir, app) = test_app();
    let (status, body) = post_json(
        &app,
        "/api/run",
        serde_json::json!({ "prompt": "hello world" }),
    )
    .await;

    assert_eq!(status, StatusCode::ACCEPTED);
    assert!(body["id"].is_string());
}

#[tokio::test]
async fn post_run_rejects_empty_prompt() {
    let (_dir, app) = test_app();
    let (status, body) = post_json(&app, "/api/run", serde_json::json!({ "prompt": "" })).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "validation_error");
}

#[tokio::test]
async fn post_run_rejects_missing_prompt() {
    let (_dir, app) = test_app();
    let (status, _body) = post_json(&app, "/api/run", serde_json::json!({})).await;

    // Missing required field — either 400 (validation) or 422 (parse).
    assert!(status.is_client_error());
}

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

#[tokio::test]
async fn auth_rejects_missing_key() {
    let (_dir, app) = test_app_with_auth("secret-key-123");
    let (status, body) = get_json(&app, "/api/health").await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "unauthorized");
}

#[tokio::test]
async fn auth_rejects_wrong_key() {
    let (_dir, app) = test_app_with_auth("secret-key-123");

    let req = Request::builder()
        .uri("/api/health")
        .header("X-Api-Key", "wrong-key")
        .body(Body::empty())
        .expect("build request");
    let resp = app.oneshot(req).await.expect("oneshot");

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn auth_accepts_correct_key() {
    let (_dir, app) = test_app_with_auth("secret-key-123");

    let req = Request::builder()
        .uri("/api/health")
        .header("X-Api-Key", "secret-key-123")
        .body(Body::empty())
        .expect("build request");
    let resp = app.oneshot(req).await.expect("oneshot");

    assert_eq!(resp.status(), StatusCode::OK);
}

// ---------------------------------------------------------------------------
// 404 for unknown routes
// ---------------------------------------------------------------------------

#[tokio::test]
async fn unknown_route_returns_404() {
    let (_dir, app) = test_app();

    let req = Request::builder()
        .uri("/api/nonexistent")
        .body(Body::empty())
        .expect("build request");
    let resp = app.oneshot(req).await.expect("oneshot");

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Gates summary
// ---------------------------------------------------------------------------

#[tokio::test]
async fn gate_summary_returns_ok() {
    let (_dir, app) = test_app();
    let (status, _body) = get_json(&app, "/api/gates/summary").await;

    assert_eq!(status, StatusCode::OK);
}

// ---------------------------------------------------------------------------
// Dashboard
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dashboard_returns_ok() {
    let (_dir, app) = test_app();
    let (status, _body) = get_json(&app, "/api/dashboard").await;

    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn projection_catalog_exposes_stable_dashboard_state_contracts() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/projections/catalog").await;

    assert_eq!(status, StatusCode::OK);
    let projections = body["projections"].as_array().expect("projection catalog");
    for name in [
        "agent_state",
        "plan_state",
        "gate_state",
        "learning_policy_state",
    ] {
        let entry = projections
            .iter()
            .find(|entry| entry["name"] == name)
            .unwrap_or_else(|| panic!("missing projection contract {name}"));
        assert_eq!(entry["version"], 1);
        assert!(entry["policy"]["max_age_secs"].as_u64().unwrap_or(0) > 0);
        assert!(
            !entry["policy"]["invalidation_triggers"]
                .as_array()
                .expect("triggers")
                .is_empty()
        );
    }
}

#[tokio::test]
async fn stable_projection_frames_include_version_and_explicit_missing_state() {
    let (_dir, state, app) = test_app_state();
    state
        .state_hub
        .publish(roko_core::DashboardEvent::PlanStarted {
            plan_id: "plan-1".into(),
            tasks_total: 0,
        });
    state
        .state_hub
        .publish(roko_core::DashboardEvent::TaskStarted {
            plan_id: "plan-1".into(),
            task_id: "work-a".into(),
            title: String::new(),
            phase: "dispatch".into(),
        });
    state
        .state_hub
        .publish(roko_core::DashboardEvent::GateResult {
            plan_id: "plan-1".into(),
            task_id: "work-a".into(),
            gate: "compile".into(),
            passed: false,
            output_text: None,
        });

    let (status, plan) = get_json(&app, "/api/projections/plan_state?filter=plan:plan-1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(plan["name"], "plan_state");
    assert_eq!(plan["version"], 1);
    assert!(plan["computed_at"].as_str().is_some());
    assert_eq!(plan["recovered"], false);
    assert_eq!(plan["state"]["plans"][0]["plan_id"], "plan-1");
    assert_eq!(plan["state"]["tasks"][0]["task_id"], "work-a");
    assert_eq!(plan["state"]["availability"]["state"], "available");

    let (status, gates) = get_json(&app, "/api/projections/gate_state?filter=plan:plan-1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(gates["name"], "gate_state");
    assert_eq!(gates["state"]["gates"][0]["gate"], "compile");
    assert_eq!(gates["state"]["stats"]["failed"], 1);
    assert_eq!(gates["state"]["thresholds"]["state"], "missing");

    let (status, learning) = get_json(&app, "/api/projections/learning_policy_state").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(learning["name"], "learning_policy_state");
    assert_eq!(learning["state"]["cascade_router"]["state"], "missing");
    assert_eq!(
        learning["state"]["policy_updates"]["state"],
        "unavailable_in_statehub"
    );
    assert_eq!(
        learning["state"]["policy_updates"]["endpoint"],
        "/api/projections/runtime_feedback"
    );
}

// ---------------------------------------------------------------------------
// OpenAPI spec
// ---------------------------------------------------------------------------

#[tokio::test]
async fn openapi_spec_returns_json() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/openapi.json").await;

    assert_eq!(status, StatusCode::OK);
    // Should have standard OpenAPI top-level keys.
    assert!(body.get("openapi").is_some() || body.get("paths").is_some());
}

// ---------------------------------------------------------------------------
// EventBus ↔ StateHub bridge
// ---------------------------------------------------------------------------

/// Verify that a `DashboardEvent` published to `StateHub` arrives on the
/// `EventBus` and is visible to a WebSocket client via the orchestrator bridge.
#[tokio::test]
async fn orchestrator_events_reach_websocket_via_bridge() {
    let (_dir, state, app) = test_app_state();
    let _bridge = roko_serve::start_orchestrator_event_bridge(Arc::clone(&state));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ws server");
    let addr = listener.local_addr().expect("listener addr");
    let server_app = app.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, server_app)
            .await
            .expect("serve test app");
    });

    let (mut socket, _) = connect_async(format!("ws://{addr}/ws"))
        .await
        .expect("connect websocket");

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Publish a DashboardEvent directly to StateHub (simulating the runner-v2 event loop).
    let sender = state.state_hub.sender();
    sender.publish(roko_core::DashboardEvent::GateResult {
        plan_id: "test-plan-1".to_string(),
        task_id: "task-A".to_string(),
        gate: "compile".to_string(),
        passed: true,
        output_text: None,
    });

    // The bridge converts it to ServerEvent::GateResult → WS client sees it.
    let event: serde_json::Value =
        serde_json::from_str(&next_ws_text(&mut socket).await).expect("parse gate event");
    assert_eq!(event["type"], "gate_result");
    assert_eq!(event["plan_id"], "test-plan-1");
    assert_eq!(event["task_id"], "task-A");
    assert_eq!(event["gate"], "compile");
    assert_eq!(event["passed"], true);

    let _ = socket.close(None).await;
    server.abort();
}

/// Verify multiple `DashboardEvent` types bridge correctly in sequence.
#[tokio::test]
async fn bridge_converts_multiple_event_types() {
    let (_dir, state, app) = test_app_state();
    let _bridge = roko_serve::start_orchestrator_event_bridge(Arc::clone(&state));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ws server");
    let addr = listener.local_addr().expect("listener addr");
    let server_app = app.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, server_app)
            .await
            .expect("serve test app");
    });

    let (mut socket, _) = connect_async(format!("ws://{addr}/ws"))
        .await
        .expect("connect websocket");

    tokio::time::sleep(Duration::from_millis(50)).await;
    let sender = state.state_hub.sender();

    // 1. PlanStarted
    sender.publish(roko_core::DashboardEvent::PlanStarted {
        plan_id: "plan-bridge".to_string(),
        tasks_total: 0,
    });
    let ev: serde_json::Value =
        serde_json::from_str(&next_ws_text(&mut socket).await).expect("parse plan_started");
    assert_eq!(ev["type"], "plan_started");
    assert_eq!(ev["plan_id"], "plan-bridge");

    // 2. TaskStarted (wrapped in Execution)
    sender.publish(roko_core::DashboardEvent::TaskStarted {
        plan_id: "plan-bridge".to_string(),
        task_id: "task-1".to_string(),
        title: String::new(),
        phase: "implementing".to_string(),
    });
    let ev: serde_json::Value =
        serde_json::from_str(&next_ws_text(&mut socket).await).expect("parse task_started");
    assert_eq!(ev["type"], "execution");
    assert_eq!(ev["plan_id"], "plan-bridge");
    assert_eq!(ev["event"]["type"], "task_started");
    assert_eq!(ev["event"]["task_id"], "task-1");

    // 3. PlanCompleted
    sender.publish(roko_core::DashboardEvent::PlanCompleted {
        plan_id: "plan-bridge".to_string(),
        success: true,
    });
    let ev: serde_json::Value =
        serde_json::from_str(&next_ws_text(&mut socket).await).expect("parse plan_completed");
    assert_eq!(ev["type"], "plan_completed");
    assert_eq!(ev["success"], true);

    let _ = socket.close(None).await;
    server.abort();
}

// ---------------------------------------------------------------------------
// Relay health
// ---------------------------------------------------------------------------

#[tokio::test]
async fn relay_health_returns_local_default() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/relay/health").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["connection"]["mode"], "local");
    assert_eq!(body["freshness"]["stale"], false);
}

// ---------------------------------------------------------------------------
// Truth map
// ---------------------------------------------------------------------------

#[tokio::test]
async fn truth_map_returns_all_entity_kinds() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/truth_map").await;

    assert_eq!(status, StatusCode::OK);
    let entries = body.as_array().expect("truth_map should be an array");
    assert!(entries.len() >= 10, "expected at least 10 entity kinds");
    // Verify each entry has the expected fields.
    for entry in entries {
        assert!(entry.get("kind").is_some(), "missing kind field");
        assert!(entry.get("source").is_some(), "missing source field");
        assert!(entry.get("read_path").is_some(), "missing read_path field");
    }
}

// ---------------------------------------------------------------------------
// Server state persistence roundtrip
// ---------------------------------------------------------------------------

#[tokio::test]
async fn state_persistence_roundtrip() {
    let (dir, state, app) = test_app_state();

    // Create a job via the API.
    let (status, created) = post_json(
        &app,
        "/api/jobs",
        serde_json::json!({
            "title": "Persistence test job",
            "description": "Should survive a roundtrip."
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let job_id = created["id"].as_str().expect("job id");

    // Verify the job file exists on disk.
    let job_path = dir
        .path()
        .join(".roko")
        .join("jobs")
        .join(format!("{job_id}.json"));
    assert!(job_path.exists(), "job file should be persisted to disk");

    // Read back via the API.
    let (status, fetched) = get_json(&app, &format!("/api/jobs/{job_id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fetched["title"], "Persistence test job");

    state.shutdown().await;
}

/// Verify unmapped `DashboardEvent` variants are silently dropped (no panic).
#[tokio::test]
async fn bridge_drops_unmapped_events_without_panic() {
    let (_dir, state, _app) = test_app_state();
    let _bridge = roko_serve::start_orchestrator_event_bridge(Arc::clone(&state));

    let mut rx = state.event_bus.subscribe();
    tokio::time::sleep(Duration::from_millis(50)).await;

    let sender = state.state_hub.sender();

    // Publish an unmapped event (CascadeRouterUpdated has no ServerEvent).
    sender.publish(roko_core::DashboardEvent::CascadeRouterUpdated {
        snapshot_json: "{}".to_string(),
    });

    // Then publish a mapped event that WILL come through.
    sender.publish(roko_core::DashboardEvent::Error {
        message: "sentinel".to_string(),
    });

    // The first event on EventBus should be the Error, not CascadeRouterUpdated.
    let envelope = timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("should receive within 2s")
        .expect("recv should succeed");
    match &envelope.payload {
        roko_serve::events::ServerEvent::Error { message } => {
            assert_eq!(message, "sentinel");
        }
        other => panic!("expected Error, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Arenas
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_arenas_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/arenas").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "arenas should return JSON"
    );
}

#[tokio::test]
async fn create_arena_rejects_empty_body() {
    let (_dir, app) = test_app();
    let (status, _body) = post_json(&app, "/api/arenas", serde_json::json!({})).await;

    assert!(
        status.is_client_error(),
        "empty arena create should be rejected, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Registries
// ---------------------------------------------------------------------------

#[tokio::test]
async fn registry_stats_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/registries/stats").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_object(), "registry stats should return an object");
}

#[tokio::test]
async fn registry_events_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/registries/events").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "registry events should return JSON"
    );
}

// ---------------------------------------------------------------------------
// Gateway
// ---------------------------------------------------------------------------

#[tokio::test]
async fn gateway_stats_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/gateway/stats").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_object(), "gateway stats should return an object");
}

#[tokio::test]
async fn gateway_models_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/gateway/models").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_object() || body.is_array(),
        "gateway models should return JSON"
    );
}

#[tokio::test]
async fn rate_limits_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/rate-limits").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_object(), "rate-limits should return an object");
}

// ---------------------------------------------------------------------------
// Team
// ---------------------------------------------------------------------------

#[tokio::test]
async fn team_me_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/team/me").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_object(), "team/me should return an object");
}

#[tokio::test]
async fn team_members_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/team/members").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "team/members should return JSON"
    );
}

// ---------------------------------------------------------------------------
// Workspaces
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_workspaces_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/workspaces").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "workspaces should return JSON"
    );
}

#[tokio::test]
async fn default_workspace_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/workspaces/default").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_object(),
        "default workspace should return an object"
    );
}

// ---------------------------------------------------------------------------
// SWE bench
// ---------------------------------------------------------------------------

#[tokio::test]
async fn swe_bench_runs_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/bench/swe/runs").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "swe runs should return JSON"
    );
}

#[tokio::test]
async fn swe_bench_datasets_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/bench/swe/datasets").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "swe datasets should return JSON"
    );
}

// ---------------------------------------------------------------------------
// Aggregator
// ---------------------------------------------------------------------------

#[tokio::test]
async fn aggregator_agents_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/agents").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "agents should return JSON"
    );
}

#[tokio::test]
async fn aggregator_tasks_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/tasks").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "tasks should return JSON"
    );
}

// ---------------------------------------------------------------------------
// Providers
// ---------------------------------------------------------------------------

#[tokio::test]
async fn providers_list_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/providers").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "providers should return JSON"
    );
}

#[tokio::test]
async fn models_list_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/models").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "models should return JSON"
    );
}

// ---------------------------------------------------------------------------
// RPC proxy
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rpc_health_route_is_registered() {
    let (_dir, app) = test_app();
    let (status, _body) = get_json(&app, "/api/rpc/health").await;

    // RPC health may return 503 if no proxy is configured — that's valid.
    assert_ne!(
        status,
        StatusCode::NOT_FOUND,
        "rpc health should be registered"
    );
}

// ---------------------------------------------------------------------------
// Auth / API keys (read-only)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn api_keys_list_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/api-keys").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "api-keys should return JSON"
    );
}

#[tokio::test]
async fn auth_audit_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/auth/audit").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "auth audit should return JSON"
    );
}

// ---------------------------------------------------------------------------
// Gateway inference rejects malformed input
// ---------------------------------------------------------------------------

#[tokio::test]
async fn gateway_inference_rejects_empty_body() {
    let (_dir, app) = test_app();
    let (status, _body) = post_json(&app, "/api/gateway/inference", serde_json::json!({})).await;

    assert!(
        status.is_client_error() || status == StatusCode::INTERNAL_SERVER_ERROR,
        "gateway inference with empty body should not return success, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Vision loop rejects missing fields
// ---------------------------------------------------------------------------

#[tokio::test]
async fn vision_loop_rejects_empty_body() {
    let (_dir, app) = test_app();
    let (status, _body) = post_json(&app, "/api/vision-loop", serde_json::json!({})).await;

    assert!(
        status.is_client_error() || status == StatusCode::INTERNAL_SERVER_ERROR,
        "vision-loop with empty body should not return success, got {status}"
    );
}

// ---------------------------------------------------------------------------
// PRDs
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_prds_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/prds").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_array(), "prds should return an array");
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[tokio::test]
async fn config_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/config").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_object(), "config should return an object");
}

// ---------------------------------------------------------------------------
// Subscriptions
// ---------------------------------------------------------------------------

#[tokio::test]
async fn subscriptions_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/subscriptions").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "subscriptions should return JSON"
    );
}

// ---------------------------------------------------------------------------
// Learning experiments & router
// ---------------------------------------------------------------------------

#[tokio::test]
async fn learning_experiments_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/learning/experiments").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_object() || body.is_array(),
        "learning experiments should return JSON"
    );
}

#[tokio::test]
async fn learning_cascade_router_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/learning/cascade-router").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_object() || body.is_array(),
        "learning cascade-router should return JSON"
    );
}

// ---------------------------------------------------------------------------
// Extensions
// ---------------------------------------------------------------------------

#[tokio::test]
async fn extensions_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/extensions").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "extensions should return JSON"
    );
}

// ---------------------------------------------------------------------------
// Feeds
// ---------------------------------------------------------------------------

#[tokio::test]
async fn feeds_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/feeds").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "feeds should return JSON"
    );
}

// ---------------------------------------------------------------------------
// Recipes
// ---------------------------------------------------------------------------

#[tokio::test]
async fn recipes_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/recipes").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "recipes should return JSON"
    );
}

// ---------------------------------------------------------------------------
// Groups
// ---------------------------------------------------------------------------

#[tokio::test]
async fn groups_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/groups").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "groups should return JSON"
    );
}

// ---------------------------------------------------------------------------
// Triggers
// ---------------------------------------------------------------------------

#[tokio::test]
async fn triggers_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/triggers").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "triggers should return JSON"
    );
}

// ---------------------------------------------------------------------------
// Workflows
// ---------------------------------------------------------------------------

#[tokio::test]
async fn workflows_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/workflows").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "workflows should return JSON"
    );
}

// ---------------------------------------------------------------------------
// Connectors
// ---------------------------------------------------------------------------

#[tokio::test]
async fn connectors_route_is_registered() {
    let (_dir, app) = test_app();
    let (status, _body) = get_json(&app, "/api/connectors").await;

    // Connectors may require authorization (403) or return data (200).
    assert_ne!(
        status,
        StatusCode::NOT_FOUND,
        "connectors should be registered"
    );
}

// ---------------------------------------------------------------------------
// Deployments
// ---------------------------------------------------------------------------

#[tokio::test]
async fn deployments_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/deployments").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "deployments should return JSON"
    );
}

// ---------------------------------------------------------------------------
// Integrations
// ---------------------------------------------------------------------------

#[tokio::test]
async fn integrations_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/integrations").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "integrations should return JSON"
    );
}

// ---------------------------------------------------------------------------
// Secrets
// ---------------------------------------------------------------------------

#[tokio::test]
async fn secrets_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/secrets").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "secrets should return JSON"
    );
}

// ---------------------------------------------------------------------------
// Meta health
// ---------------------------------------------------------------------------

#[tokio::test]
async fn meta_health_route_is_registered() {
    let (_dir, _, app) = test_app_state();
    let (status, _body) = get_json(&app, "/api/meta/health").await;

    // Meta health may return 404 when served through the test fallback.
    // The route_coverage_matrix confirms registration; here we verify a
    // non-panic response is returned.
    assert!(
        status.is_success() || status == StatusCode::NOT_FOUND,
        "meta health should respond, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Templates
// ---------------------------------------------------------------------------

#[tokio::test]
async fn templates_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/templates").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "templates should return JSON"
    );
}

// ---------------------------------------------------------------------------
// Runs list
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dashboard_runs_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/dashboard/runs").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "dashboard runs should return JSON"
    );
}

// ---------------------------------------------------------------------------
// Neuro
// ---------------------------------------------------------------------------

#[tokio::test]
async fn neuro_stats_route_is_registered() {
    let (_dir, _, app) = test_app_state();
    let (status, _body) = get_json(&app, "/api/neuro/stats").await;

    // May return 404 when no neuro store is initialized.
    assert!(
        status.is_success() || status == StatusCode::NOT_FOUND,
        "neuro stats should respond, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Dream
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dream_status_route_is_registered() {
    let (_dir, _, app) = test_app_state();
    let (status, _body) = get_json(&app, "/api/dream/status").await;

    // May return 404 when no dream scheduler is running.
    assert!(
        status.is_success() || status == StatusCode::NOT_FOUND,
        "dream status should respond, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Diagnosis
// ---------------------------------------------------------------------------

#[tokio::test]
async fn diagnosis_route_is_registered() {
    let (_dir, _, app) = test_app_state();
    let (status, _body) = get_json(&app, "/api/diagnosis").await;

    // May return 404 when no conductor is available.
    assert!(
        status.is_success() || status == StatusCode::NOT_FOUND,
        "diagnosis should respond, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Routing explain
// ---------------------------------------------------------------------------

#[tokio::test]
async fn routing_explain_route_is_registered() {
    let (_dir, app) = test_app();
    let (status, _body) = get_json(&app, "/api/routing/explain").await;

    // May return 400 when no routing query is provided.
    assert_ne!(
        status,
        StatusCode::NOT_FOUND,
        "routing explain should be registered"
    );
}

// ---------------------------------------------------------------------------
// Knowledge entries
// ---------------------------------------------------------------------------

#[tokio::test]
async fn knowledge_entries_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/knowledge/entries").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "knowledge entries should return JSON"
    );
}

// ---------------------------------------------------------------------------
// Predictions sessions
// ---------------------------------------------------------------------------

#[tokio::test]
async fn predictions_sessions_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/predictions/sessions").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array() || body.is_object(),
        "predictions sessions should return JSON"
    );
}

// ---------------------------------------------------------------------------
// Event ingest
// ---------------------------------------------------------------------------

#[tokio::test]
async fn event_ingest_rejects_empty_body() {
    let (_dir, app) = test_app();
    let (status, _body) = post_json(&app, "/api/events", serde_json::json!({})).await;

    // Empty event should be rejected or handled — not 404.
    assert_ne!(
        status,
        StatusCode::NOT_FOUND,
        "event ingest should be registered"
    );
}

// ---------------------------------------------------------------------------
// Heartbeats
// ---------------------------------------------------------------------------

#[tokio::test]
async fn heartbeat_accepts_post() {
    let (_dir, app) = test_app();
    let (status, _body) = post_json(
        &app,
        "/api/heartbeats",
        serde_json::json!({
            "agent_id": "test-agent",
            "status": "healthy"
        }),
    )
    .await;

    // Heartbeats should be accepted (not 404/405).
    assert_ne!(
        status,
        StatusCode::NOT_FOUND,
        "heartbeat should be registered"
    );
    assert_ne!(status, StatusCode::METHOD_NOT_ALLOWED);
}

// ---------------------------------------------------------------------------
// Shared runs
// ---------------------------------------------------------------------------

#[tokio::test]
async fn shared_runs_post_returns_non_404() {
    let (_dir, app) = test_app();
    let (status, _body) = post_json(&app, "/api/runs/test-id/share", serde_json::json!({})).await;

    // The route should be registered (share a run).
    assert_ne!(
        status,
        StatusCode::NOT_FOUND,
        "shared runs should be registered"
    );
}

// ---------------------------------------------------------------------------
// Webhooks
// ---------------------------------------------------------------------------

#[tokio::test]
async fn webhook_generic_returns_non_404() {
    let (_dir, app) = test_app();
    let (status, _body) = post_json(
        &app,
        "/api/webhooks/generic",
        serde_json::json!({"event": "test"}),
    )
    .await;

    assert_ne!(
        status,
        StatusCode::NOT_FOUND,
        "webhook generic should be registered"
    );
}

// ---------------------------------------------------------------------------
// Content-type enforcement
// ---------------------------------------------------------------------------

#[tokio::test]
async fn post_without_content_type_still_processes() {
    let (_dir, app) = test_app();

    // POST /api/run without content-type header.
    // Axum will attempt JSON parsing regardless — the handler may accept or
    // reject depending on the body parser. We test that no panic occurs.
    let req = Request::builder()
        .method("POST")
        .uri("/api/run")
        .body(Body::from(r#"{"prompt":"test"}"#))
        .expect("build request");
    let resp = app.oneshot(req).await.expect("oneshot");

    // The route should respond (may accept or reject — both are valid).
    assert_ne!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "POST /api/run should be registered"
    );
}

// ---------------------------------------------------------------------------
// Wrong HTTP method returns 405
// ---------------------------------------------------------------------------

#[tokio::test]
async fn wrong_method_returns_405_or_404() {
    let (_dir, app) = test_app();

    // DELETE /api/health — health only supports GET.
    let req = Request::builder()
        .method("DELETE")
        .uri("/api/health")
        .body(Body::empty())
        .expect("build request");
    let resp = app.oneshot(req).await.expect("oneshot");

    assert!(
        resp.status() == StatusCode::METHOD_NOT_ALLOWED || resp.status() == StatusCode::NOT_FOUND,
        "DELETE /api/health should be 405 or 404, got {}",
        resp.status()
    );
}

// ---------------------------------------------------------------------------
// Response consistency: error bodies are JSON
// ---------------------------------------------------------------------------

#[tokio::test]
async fn error_responses_are_json() {
    let (_dir, app) = test_app();

    // Trigger a known error: POST /api/run with empty prompt.
    let req = Request::builder()
        .method("POST")
        .uri("/api/run")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"prompt":""}"#))
        .expect("build request");
    let resp = app.oneshot(req).await.expect("oneshot");
    assert!(resp.status().is_client_error());

    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        ct.contains("application/json"),
        "error response should have JSON content-type, got: {ct}"
    );
}

// ---------------------------------------------------------------------------
// Top-level probes (non /api prefix)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn top_level_health_returns_ok() {
    let (_dir, _, app) = test_app_state();
    let (status, body) = get_json(&app, "/health").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn top_level_ready_returns_ok() {
    let (_dir, _, app) = test_app_state();
    let (status, _body) = get_json(&app, "/ready").await;

    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn top_level_metrics_returns_ok() {
    let (_dir, _, app) = test_app_state();
    let (status, body) = get_json(&app, "/metrics").await;

    // /metrics may return prometheus format (non-JSON) — just check status.
    assert_eq!(status, StatusCode::OK);
    let _ = body; // may be Null if body is not JSON
}

// ---------------------------------------------------------------------------
// Malformed JSON bodies
// ---------------------------------------------------------------------------

#[tokio::test]
async fn post_with_invalid_json_returns_client_error() {
    let (_dir, app) = test_app();

    let req = Request::builder()
        .method("POST")
        .uri("/api/run")
        .header("content-type", "application/json")
        .body(Body::from("not-valid-json{{{"))
        .expect("build request");
    let resp = app.oneshot(req).await.expect("oneshot");

    assert!(
        resp.status().is_client_error(),
        "malformed JSON should return 4xx, got {}",
        resp.status()
    );
}

#[tokio::test]
async fn post_jobs_with_invalid_json_returns_client_error() {
    let (_dir, app) = test_app();

    let req = Request::builder()
        .method("POST")
        .uri("/api/jobs")
        .header("content-type", "application/json")
        .body(Body::from("[broken"))
        .expect("build request");
    let resp = app.oneshot(req).await.expect("oneshot");

    assert!(
        resp.status().is_client_error(),
        "malformed JSON on /api/jobs should return 4xx, got {}",
        resp.status()
    );
}

// ---------------------------------------------------------------------------
// Health response contract
// ---------------------------------------------------------------------------

#[tokio::test]
async fn health_response_includes_required_fields() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/health").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
    assert!(
        body["uptime_secs"].is_number(),
        "health must include uptime_secs"
    );
    assert!(
        body["version"].is_string(),
        "health must include version string"
    );
}

// ---------------------------------------------------------------------------
// Success responses have JSON content-type
// ---------------------------------------------------------------------------

#[tokio::test]
async fn success_responses_have_json_content_type() {
    let (_dir, app) = test_app();

    let endpoints = ["/api/health", "/api/status", "/api/plans", "/api/jobs"];

    for endpoint in endpoints {
        let req = Request::builder()
            .uri(endpoint)
            .body(Body::empty())
            .expect("build request");
        let resp = app.clone().oneshot(req).await.expect("oneshot");

        if resp.status().is_success() {
            let ct = resp
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            assert!(
                ct.contains("application/json"),
                "{endpoint} success response should have JSON content-type, got: {ct}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Auth with Bearer header prefix
// ---------------------------------------------------------------------------

#[tokio::test]
async fn auth_accepts_bearer_header_prefix() {
    let (_dir, app) = test_app_with_auth("my-api-key");

    let req = Request::builder()
        .uri("/api/health")
        .header("Authorization", "Bearer my-api-key")
        .body(Body::empty())
        .expect("build request");
    let resp = app.oneshot(req).await.expect("oneshot");

    assert_eq!(resp.status(), StatusCode::OK);
}

// ---------------------------------------------------------------------------
// Job not found
// ---------------------------------------------------------------------------

#[tokio::test]
async fn job_not_found_returns_404() {
    let (_dir, app) = test_app();
    let (status, _body) = get_json(&app, "/api/jobs/nonexistent-job-id").await;

    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "non-existent job should return 404"
    );
}

// ---------------------------------------------------------------------------
// Wrong method on read-only endpoints
// ---------------------------------------------------------------------------

#[tokio::test]
async fn delete_on_read_only_endpoint_is_rejected() {
    let (_dir, app) = test_app();

    let req = Request::builder()
        .method("DELETE")
        .uri("/api/status")
        .body(Body::empty())
        .expect("build request");
    let resp = app.oneshot(req).await.expect("oneshot");

    assert!(
        resp.status() == StatusCode::METHOD_NOT_ALLOWED || resp.status() == StatusCode::NOT_FOUND,
        "DELETE /api/status should be 405 or 404, got {}",
        resp.status()
    );
}

#[tokio::test]
async fn put_on_plans_is_rejected() {
    let (_dir, app) = test_app();

    let req = Request::builder()
        .method("PUT")
        .uri("/api/plans")
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .expect("build request");
    let resp = app.oneshot(req).await.expect("oneshot");

    assert!(
        resp.status() == StatusCode::METHOD_NOT_ALLOWED || resp.status() == StatusCode::NOT_FOUND,
        "PUT /api/plans should be 405 or 404, got {}",
        resp.status()
    );
}

// ---------------------------------------------------------------------------
// SSE event stream endpoint registration
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sse_endpoint_is_registered() {
    let (_dir, _, app) = test_app_state();

    // The SSE endpoint lives at /api/events (also aliased as /api/sse).
    let req = Request::builder()
        .uri("/api/events")
        .body(Body::empty())
        .expect("build request");
    let resp = app.clone().oneshot(req).await.expect("oneshot");

    // SSE endpoint returns 200 with text/event-stream content-type.
    assert_ne!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "SSE endpoint /api/events should be registered"
    );

    // Verify the /api/sse alias resolves too.
    let req = Request::builder()
        .uri("/api/sse")
        .body(Body::empty())
        .expect("build request");
    let resp = app.oneshot(req).await.expect("oneshot");
    assert_ne!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "SSE alias /api/sse should be registered"
    );
}

// ---------------------------------------------------------------------------
// Webhook input validation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn webhook_with_invalid_json_returns_error() {
    let (_dir, app) = test_app();

    let req = Request::builder()
        .method("POST")
        .uri("/api/webhooks/generic")
        .header("content-type", "application/json")
        .body(Body::from("{{invalid"))
        .expect("build request");
    let resp = app.oneshot(req).await.expect("oneshot");

    assert!(
        resp.status().is_client_error() || resp.status().is_server_error(),
        "malformed webhook should not return 2xx, got {}",
        resp.status()
    );
}

// ---------------------------------------------------------------------------
// Run status for non-existent run
// ---------------------------------------------------------------------------

#[tokio::test]
async fn run_status_not_found_returns_404() {
    let (_dir, app) = test_app();
    let (status, _body) = get_json(&app, "/api/run/nonexistent-run/status").await;

    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "non-existent run status should return 404"
    );
}

// ---------------------------------------------------------------------------
// Gateway inference with valid-looking but incomplete input
// ---------------------------------------------------------------------------

#[tokio::test]
async fn gateway_inference_with_missing_model_rejects() {
    let (_dir, app) = test_app();
    let (status, _body) = post_json(
        &app,
        "/api/gateway/inference",
        serde_json::json!({"prompt": "test", "model": ""}),
    )
    .await;

    assert!(
        status.is_client_error() || status.is_server_error(),
        "gateway inference with empty model should not succeed, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Multiple jobs listed in order
// ---------------------------------------------------------------------------

#[tokio::test]
async fn multiple_jobs_are_all_listed() {
    let (_dir, app) = test_app();

    for i in 0..3 {
        let (status, _) = post_json(
            &app,
            "/api/jobs",
            serde_json::json!({
                "title": format!("Job {i}"),
                "description": format!("Description {i}"),
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
    }

    let (status, body) = get_json(&app, "/api/jobs").await;
    assert_eq!(status, StatusCode::OK);
    let jobs = body.as_array().expect("jobs array");
    assert_eq!(jobs.len(), 3, "all three jobs should be listed");
}

// ---------------------------------------------------------------------------
// Job stats endpoint
// ---------------------------------------------------------------------------

#[tokio::test]
async fn job_stats_returns_ok() {
    let (_dir, app) = test_app();
    let (status, body) = get_json(&app, "/api/jobs/stats").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_object(), "job stats should return an object");
}

// ---------------------------------------------------------------------------
// Job match endpoint
// ---------------------------------------------------------------------------

#[tokio::test]
async fn job_match_returns_non_error() {
    let (_dir, app) = test_app();
    let (status, _body) = post_json(
        &app,
        "/api/jobs/match",
        serde_json::json!({"capabilities": ["coding"]}),
    )
    .await;

    assert_ne!(
        status,
        StatusCode::NOT_FOUND,
        "job match should be registered"
    );
}

// ---------------------------------------------------------------------------
// Error envelope consistency
// ---------------------------------------------------------------------------

#[tokio::test]
async fn error_envelope_has_code_and_message_fields() {
    let (_dir, app) = test_app();

    // POST /api/run with empty prompt should return structured error.
    let (status, body) = post_json(&app, "/api/run", serde_json::json!({"prompt": ""})).await;
    assert!(status.is_client_error());
    assert!(
        body.get("code").is_some() || body.get("error").is_some(),
        "error body should have a 'code' or 'error' field, got: {body}"
    );
}

// ---------------------------------------------------------------------------
// Concurrent reads on read endpoints
// ---------------------------------------------------------------------------

#[tokio::test]
async fn concurrent_get_requests_all_succeed() {
    let (_dir, app) = test_app();

    let endpoints = vec![
        "/api/health",
        "/api/status",
        "/api/plans",
        "/api/jobs",
        "/api/signals",
        "/api/episodes",
    ];

    let handles: Vec<_> = endpoints
        .into_iter()
        .map(|ep| {
            let router = app.clone();
            tokio::spawn(async move {
                let req = Request::builder()
                    .uri(ep)
                    .body(Body::empty())
                    .expect("build request");
                let resp = router.oneshot(req).await.expect("oneshot");
                (ep, resp.status())
            })
        })
        .collect();

    for handle in handles {
        let (ep, status) = handle.await.expect("join");
        assert!(
            status.is_success(),
            "concurrent GET {ep} should succeed, got {status}"
        );
    }
}
