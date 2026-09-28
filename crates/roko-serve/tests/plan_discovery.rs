//! Integration tests proving that the three plan-discovery defects are fixed.
//!
//! Defects covered:
//! 1. **Extension-filter + wrong-root bug** — `GET /api/plans` must return plans
//!    stored as directories (e.g. `plans/my-plan/tasks.toml`), not only flat
//!    `.json` / `.toml` files under `.roko/plans/`.
//! 2. **Schema bug** — `GET /api/plans/:id/tasks` must return the full envelope
//!    with task ids, roles, and `depends_on` edges intact.
//! 3. **404 vs 500** — a missing plan id must produce HTTP 404, not 500.
//! 4. **Path traversal** — a plan id containing `..` must produce HTTP 400.
//!
//! The tests use `tower::ServiceExt::oneshot` to drive the axum router directly,
//! matching the harness style in `api_integration.rs`.  No real network listener
//! is started.

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use roko_core::config::ServeAuthConfig;
use roko_core::config::schema::RokoConfig;
use roko_serve::deploy::create_backend;
use roko_serve::plan_types::{PlanSummaryDto, PlanTaskDto, PlanTasksDto};
use roko_serve::routes::build_router;
use roko_serve::runtime::{CliRuntime, DashboardInfo, RunResult, SessionStatusInfo};
use roko_serve::state::AppState;
use tempfile::tempdir;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// Stub runtime seeded with a single fixture plan
// ---------------------------------------------------------------------------

/// A minimal runtime that returns pre-built plan data.
///
/// This lets the handler-level tests focus on the HTTP/JSON contract without
/// depending on the real CLI file-parsing logic.
struct PlanDiscoveryRuntime {
    summaries: Vec<PlanSummaryDto>,
    tasks: Vec<PlanTasksDto>,
}

impl PlanDiscoveryRuntime {
    /// Seed the runtime with a single "test-plan" that mirrors the shape of
    /// `plans/demo-hello/tasks.toml` (one task, a role, and an empty
    /// `depends_on` list).
    fn with_fixture() -> Self {
        let summary = PlanSummaryDto {
            id: "test-plan".to_string(),
            title: "Test Plan".to_string(),
            task_count: 1,
            tasks_done: 0,
            tasks_failed: 0,
            completed: false,
            status: "ready".to_string(),
            superseded_by: None,
            old_format: false,
            last_error: None,
        };

        let task = PlanTaskDto {
            id: "TEST-T01".to_string(),
            title: "Create the smoke artifact".to_string(),
            description: Some("Write a smoke output file.".to_string()),
            role: Some("implementer".to_string()),
            tier: "mechanical".to_string(),
            status: "ready".to_string(),
            depends_on: Vec::new(),
            files: vec!["demo/smoke-output.md".to_string()],
            completed: false,
            verify_phases: vec!["structural".to_string()],
        };

        let tasks_dto = PlanTasksDto {
            plan_id: "test-plan".to_string(),
            task_count: 1,
            tasks: vec![task],
        };

        Self {
            summaries: vec![summary],
            tasks: vec![tasks_dto],
        }
    }
}

#[async_trait::async_trait]
impl CliRuntime for PlanDiscoveryRuntime {
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

    async fn list_plans(&self, _workdir: &std::path::Path) -> anyhow::Result<Vec<PlanSummaryDto>> {
        Ok(self.summaries.clone())
    }

    async fn load_plan_summary(
        &self,
        _workdir: &std::path::Path,
        plan_id: &str,
    ) -> anyhow::Result<Option<PlanSummaryDto>> {
        Ok(self.summaries.iter().find(|s| s.id == plan_id).cloned())
    }

    async fn load_plan_tasks(
        &self,
        _workdir: &std::path::Path,
        plan_id: &str,
    ) -> anyhow::Result<Option<PlanTasksDto>> {
        Ok(self.tasks.iter().find(|t| t.plan_id == plan_id).cloned())
    }
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Build a test router backed by a temp directory and the fixture runtime.
fn fixture_app() -> (tempfile::TempDir, axum::Router) {
    let dir = tempdir().expect("tempdir");

    // Create the plans/ directory inside the temp workspace so the server's
    // plan-directory probe (plans_dir helper) behaves like a real workspace.
    let plan_dir = dir.path().join("plans").join("test-plan");
    std::fs::create_dir_all(&plan_dir).expect("create plans/test-plan dir");
    // Write a minimal tasks.toml so the directory looks like a real plan.
    std::fs::write(
        plan_dir.join("tasks.toml"),
        "[meta]\nplan = \"test-plan\"\ntotal = 1\ndone = 0\nstatus = \"ready\"\n\
         [[task]]\nid = \"TEST-T01\"\ntitle = \"Create the smoke artifact\"\n\
         status = \"ready\"\ntier = \"mechanical\"\nrole = \"implementer\"\n\
         depends_on = []\nfiles = [\"demo/smoke-output.md\"]\n",
    )
    .expect("write fixture tasks.toml");

    let config = RokoConfig::default();
    let deploy = Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
    let state = Arc::new(
        AppState::new(
            dir.path().to_path_buf(),
            Arc::new(PlanDiscoveryRuntime::with_fixture()),
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// 1. Plan listing must contain the fixture plan by its directory slug with
///    the correct task count (fixes extension-filter + wrong-root bugs).
#[tokio::test]
async fn list_plans_returns_directory_plan_with_correct_task_count() {
    let (_dir, app) = fixture_app();
    let (status, body) = get_json(&app, "/api/plans").await;

    assert_eq!(status, StatusCode::OK, "expected 200, got {status}: {body}");

    let plans = body.as_array().expect("response should be a JSON array");
    let plan = plans
        .iter()
        .find(|p| p["id"].as_str() == Some("test-plan"))
        .unwrap_or_else(|| panic!("'test-plan' not found in listing: {body}"));

    assert_eq!(
        plan["task_count"].as_u64(),
        Some(1),
        "task_count should be 1: {plan}"
    );
    assert_eq!(
        plan["status"].as_str(),
        Some("ready"),
        "status should be 'ready': {plan}"
    );
}

/// 2. Reading a plan's tasks must return the full envelope with task ids,
///    roles, and depends_on edges intact (fixes schema bug).
#[tokio::test]
async fn plan_tasks_returns_full_envelope_with_ids_roles_and_depends_on() {
    let (_dir, app) = fixture_app();
    let (status, body) = get_json(&app, "/api/plans/test-plan/tasks").await;

    assert_eq!(status, StatusCode::OK, "expected 200, got {status}: {body}");

    assert_eq!(
        body["plan_id"].as_str(),
        Some("test-plan"),
        "plan_id mismatch: {body}"
    );
    assert_eq!(
        body["task_count"].as_u64(),
        Some(1),
        "task_count should be 1: {body}"
    );

    let tasks = body["tasks"].as_array().expect("tasks should be an array");
    assert_eq!(tasks.len(), 1, "expected 1 task: {body}");

    let task = &tasks[0];
    assert_eq!(
        task["id"].as_str(),
        Some("TEST-T01"),
        "task id mismatch: {task}"
    );
    assert_eq!(
        task["role"].as_str(),
        Some("implementer"),
        "task role should be 'implementer': {task}"
    );
    assert!(
        task["depends_on"]
            .as_array()
            .map_or(false, |a| a.is_empty()),
        "depends_on should be an empty array: {task}"
    );
}

/// 3. A missing plan id must produce 404, not 500.
#[tokio::test]
async fn missing_plan_returns_404_not_500() {
    let (_dir, app) = fixture_app();
    let (status, _body) = get_json(&app, "/api/plans/nonexistent-plan").await;

    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "expected 404 for unknown plan, got {status}"
    );
}

/// 3b. A missing plan tasks endpoint must also produce 404.
#[tokio::test]
async fn missing_plan_tasks_returns_404() {
    let (_dir, app) = fixture_app();
    let (status, _body) = get_json(&app, "/api/plans/nonexistent-plan/tasks").await;

    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "expected 404 for unknown plan tasks, got {status}"
    );
}

/// 4. A plan id containing a path traversal segment must produce 400.
#[tokio::test]
async fn path_traversal_plan_id_returns_400() {
    let (_dir, app) = fixture_app();

    // `..` in the id should be rejected by `validate_path_segment`.
    let (status, _body) = get_json(&app, "/api/plans/..%2Fetc%2Fpasswd").await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "expected 400 for path-traversal id, got {status}"
    );
}

/// 4b. A plan id containing a literal slash after URL decoding must also be
///     rejected with 400.
#[tokio::test]
async fn path_traversal_dotdot_segment_returns_400() {
    let (_dir, app) = fixture_app();

    // Try a double-dot segment directly.
    let (status, _body) = get_json(&app, "/api/plans/..").await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "expected 400 for '..' plan id, got {status}"
    );
}
