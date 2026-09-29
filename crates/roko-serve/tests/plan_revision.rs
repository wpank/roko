//! Integration tests for plan revision through the HTTP router.
//!
//! These tests prove that:
//! 1. `POST /api/plans/:id/revise` with valid feedback returns HTTP 202
//!    `{ "id", "plan_id" }` and `GET /api/operations/{id}` transitions from
//!    `"running"` to `"completed"` carrying `result.task_count`.
//! 2. A rejected revision (the stub returns `revised: false`) ends with the
//!    operation in `"failed"` state and an error message.
//! 3. Blank `feedback` gives 422; an unknown plan gives 404.
//! 4. The route answers 409 while a registered active run includes the plan.
//!
//! A stub `CliRuntime` is used so no real agent is dispatched.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use roko_core::config::ServeAuthConfig;
use roko_core::config::schema::RokoConfig;
use roko_serve::deploy::create_backend;
use roko_serve::plan_types::{PlanSummaryDto, PlanTasksDto, PlanValidationDto, RevisionDto};
use roko_serve::routes::build_router;
use roko_serve::runtime::{
    CliRuntime, DashboardInfo, PlanExecutionResult, RunResult, SessionStatusInfo,
};
use roko_serve::state::AppState;
use tempfile::tempdir;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// Fixture — controls what `revise_plan` returns
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum RevisionFixture {
    /// Revision succeeds; the plan is written with `task_count` tasks.
    Success { task_count: usize },
    /// Revision is rejected by validation with `error_count` errors.
    Rejected { error_count: usize },
}

// ---------------------------------------------------------------------------
// Stub runtime
// ---------------------------------------------------------------------------

/// A minimal `CliRuntime` that:
/// - Returns a synthetic `PlanSummaryDto` for plan ids in `known_ids`.
/// - Returns `Ok(None)` for all other ids (produces HTTP 404 in the handler).
/// - Blocks in `run_plan` until the Tokio task is cancelled / aborted
///   (simulates a long-running plan so the active-plans map stays populated
///   across the 409 assertion).
/// - Returns a fixture result from `revise_plan` based on `fixture`.
struct StubRevisionRuntime {
    known_ids: HashSet<String>,
    fixture: RevisionFixture,
}

impl StubRevisionRuntime {
    fn new(
        known_ids: impl IntoIterator<Item = impl Into<String>>,
        fixture: RevisionFixture,
    ) -> Self {
        Self {
            known_ids: known_ids.into_iter().map(|s| s.into()).collect(),
            fixture,
        }
    }

    fn summary_for(plan_id: &str) -> PlanSummaryDto {
        PlanSummaryDto {
            id: plan_id.to_string(),
            title: "Stub plan".to_string(),
            task_count: 1,
            tasks_done: 0,
            tasks_failed: 0,
            completed: false,
            status: "ready".to_string(),
            superseded_by: None,
            old_format: false,
            last_error: None,
            group: None,
            estimated_minutes: None,
        }
    }
}

#[async_trait::async_trait]
impl CliRuntime for StubRevisionRuntime {
    async fn run_once(&self, _workdir: &Path, _prompt: &str) -> anyhow::Result<RunResult> {
        Ok(RunResult {
            success: true,
            output_text: None,
            usage: None,
            gate_results: Vec::new(),
        })
    }

    /// Block until the task is cancelled / aborted. This keeps the entry in
    /// `active_plans` so the 409 assertion can observe it.
    async fn run_plan(
        &self,
        _workdir: &Path,
        _plan_target: &Path,
    ) -> anyhow::Result<PlanExecutionResult> {
        std::future::pending::<()>().await;
        unreachable!()
    }

    async fn load_plan_summary(
        &self,
        _workdir: &Path,
        plan_id: &str,
    ) -> anyhow::Result<Option<PlanSummaryDto>> {
        if self.known_ids.contains(plan_id) {
            Ok(Some(Self::summary_for(plan_id)))
        } else {
            Ok(None)
        }
    }

    async fn load_plan_tasks(
        &self,
        _workdir: &Path,
        plan_id: &str,
    ) -> anyhow::Result<Option<PlanTasksDto>> {
        if self.known_ids.contains(plan_id) {
            Ok(Some(PlanTasksDto {
                plan_id: plan_id.to_string(),
                task_count: 0,
                tasks: Vec::new(),
                title: None,
                max_parallel: 1,
            }))
        } else {
            Ok(None)
        }
    }

    async fn list_plans(&self, _workdir: &Path) -> anyhow::Result<Vec<PlanSummaryDto>> {
        Ok(self
            .known_ids
            .iter()
            .map(|id| Self::summary_for(id))
            .collect())
    }

    async fn revise_plan(
        &self,
        _workdir: &Path,
        plan_id: &str,
        _feedback: &str,
    ) -> anyhow::Result<Option<RevisionDto>> {
        if !self.known_ids.contains(plan_id) {
            return Ok(None);
        }
        match self.fixture {
            RevisionFixture::Success { task_count } => Ok(Some(RevisionDto {
                revised: true,
                task_count,
                validation: PlanValidationDto {
                    valid: true,
                    errors: 0,
                    warnings: 0,
                    diagnostics: Vec::new(),
                },
            })),
            RevisionFixture::Rejected { error_count } => Ok(Some(RevisionDto {
                revised: false,
                task_count: 0,
                validation: PlanValidationDto {
                    valid: false,
                    errors: error_count,
                    warnings: 0,
                    diagnostics: Vec::new(),
                },
            })),
        }
    }

    fn session_status(&self, workdir: PathBuf) -> SessionStatusInfo {
        SessionStatusInfo {
            session_id: None,
            workdir,
            daemon_running: false,
            signal_count: None,
            episode_count: None,
            last_episode_passed: None,
        }
    }

    fn dashboard_scaffold(&self, _workdir: &Path) -> DashboardInfo {
        DashboardInfo {
            rendered: String::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a test `AppState` backed by `StubRevisionRuntime`.
///
/// Also creates a plan directory under `<workdir>/plans/<plan_id>/` so the
/// path that `execute_plan` passes to `run_plan` is a real directory.
async fn make_state(plan_id: &str, fixture: RevisionFixture) -> (tempfile::TempDir, Arc<AppState>) {
    let dir = tempdir().expect("tempdir");
    let workdir = dir.path().to_path_buf();

    let plan_dir = workdir.join("plans").join(plan_id);
    tokio::fs::create_dir_all(&plan_dir)
        .await
        .expect("create plan dir");
    tokio::fs::write(
        plan_dir.join("tasks.toml"),
        "[meta]\ntitle = \"Stub plan\"\n\n[[tasks]]\nid = \"T1\"\ndescription = \"stub task\"\n",
    )
    .await
    .expect("write tasks.toml");

    let runtime: Arc<dyn CliRuntime> = Arc::new(StubRevisionRuntime::new([plan_id], fixture));
    let deploy_backend =
        Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
    let state = Arc::new(
        AppState::new(workdir, runtime, RokoConfig::default(), deploy_backend)
            .expect("AppState::new"),
    );
    (dir, state)
}

/// Build the axum router with auth disabled.
fn build_app(state: Arc<AppState>) -> axum::Router {
    build_router(
        state,
        &[],
        ServeAuthConfig {
            enabled: false,
            ..ServeAuthConfig::default()
        },
    )
}

/// Read the response body as a `serde_json::Value`.
async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    serde_json::from_slice(&bytes).expect("parse JSON body")
}

/// Poll `GET /api/operations/{op_id}` until the status is no longer `"running"`
/// or until a 500 ms hard deadline.  Returns the final JSON payload.
async fn poll_until_done(state: Arc<AppState>, op_id: &str) -> serde_json::Value {
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(10)).await;
        let resp = build_app(Arc::clone(&state))
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/operations/{op_id}"))
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("poll operation");
        let payload = body_json(resp).await;
        if payload["status"].as_str() != Some("running") {
            return payload;
        }
    }
    panic!("operation {op_id} never left 'running' within the timeout");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// 1a. Valid feedback returns HTTP 202 with `{ "id", "plan_id" }`.
#[tokio::test(flavor = "multi_thread")]
async fn revise_valid_feedback_returns_202() {
    let plan_id = "my-dir-plan";
    let (_dir, state) = make_state(plan_id, RevisionFixture::Success { task_count: 3 }).await;
    let app = build_app(Arc::clone(&state));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/plans/{plan_id}/revise"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"feedback":"make it better"}"#))
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(
        response.status(),
        StatusCode::ACCEPTED,
        "valid revision must return 202"
    );

    let payload = body_json(response).await;
    let op_id = payload["id"].as_str().expect("response must carry 'id'");
    assert!(!op_id.is_empty(), "op id must not be empty");

    let plan_id_in_resp = payload["plan_id"]
        .as_str()
        .expect("response must carry 'plan_id'");
    assert_eq!(
        plan_id_in_resp, plan_id,
        "plan_id in response must match the route parameter"
    );
}

/// 1b. After a successful revision, `GET /api/operations/{id}` eventually
///     shows `"completed"` with `result.task_count` matching the fixture.
#[tokio::test(flavor = "multi_thread")]
async fn revise_success_operation_transitions_to_completed() {
    let plan_id = "my-dir-plan";
    let task_count: usize = 5;
    let (_dir, state) = make_state(plan_id, RevisionFixture::Success { task_count }).await;

    // Kick off the revision.
    let revise_resp = build_app(Arc::clone(&state))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/plans/{plan_id}/revise"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"feedback":"add more tasks"}"#))
                .expect("build request"),
        )
        .await
        .expect("revise request");
    assert_eq!(revise_resp.status(), StatusCode::ACCEPTED);

    let revise_payload = body_json(revise_resp).await;
    let op_id = revise_payload["id"]
        .as_str()
        .expect("op id must be present");

    // Poll until the operation leaves "running".
    let op_payload = poll_until_done(Arc::clone(&state), op_id).await;

    assert_eq!(
        op_payload["status"].as_str(),
        Some("completed"),
        "operation must reach 'completed': {op_payload}"
    );
    assert_eq!(
        op_payload["result"]["task_count"].as_u64(),
        Some(task_count as u64),
        "result.task_count must match the fixture value: {op_payload}"
    );
}

/// 2. A rejected revision (validation errors) ends the operation in `"failed"`.
#[tokio::test(flavor = "multi_thread")]
async fn revise_rejected_operation_ends_in_failed() {
    let plan_id = "my-dir-plan";
    let (_dir, state) = make_state(plan_id, RevisionFixture::Rejected { error_count: 2 }).await;

    let revise_resp = build_app(Arc::clone(&state))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/plans/{plan_id}/revise"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"feedback":"break everything"}"#))
                .expect("build request"),
        )
        .await
        .expect("revise request");
    assert_eq!(revise_resp.status(), StatusCode::ACCEPTED);

    let revise_payload = body_json(revise_resp).await;
    let op_id = revise_payload["id"]
        .as_str()
        .expect("op id must be present");

    let op_payload = poll_until_done(Arc::clone(&state), op_id).await;

    assert_eq!(
        op_payload["status"].as_str(),
        Some("failed"),
        "rejected revision must end in 'failed': {op_payload}"
    );
    let error = op_payload["error"]
        .as_str()
        .expect("failed op must carry 'error'");
    assert!(
        !error.is_empty(),
        "error message must not be empty: {op_payload}"
    );
    // The handler embeds the error count ("2 error(s)") in the message.
    assert!(
        error.contains("2"),
        "error message must mention the error count: {error}"
    );
}

/// 3a. Blank `feedback` returns HTTP 422.
#[tokio::test]
async fn revise_blank_feedback_returns_422() {
    let plan_id = "my-dir-plan";
    let (_dir, state) = make_state(plan_id, RevisionFixture::Success { task_count: 1 }).await;
    let app = build_app(state);

    // Blank string after trim.
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/plans/{plan_id}/revise"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"feedback":"   "}"#))
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(
        response.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "blank feedback must return 422"
    );
}

/// 3b. An unknown plan id returns HTTP 404.
#[tokio::test]
async fn revise_unknown_plan_returns_404() {
    let plan_id = "my-dir-plan";
    let (_dir, state) = make_state(plan_id, RevisionFixture::Success { task_count: 1 }).await;
    let app = build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/does-not-exist/revise")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"feedback":"some feedback"}"#))
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "unknown plan must return 404"
    );
}

/// 4. The route answers 409 while a registered active run includes the plan.
///
/// Starts an execution run (which blocks forever in the stub) then tries to
/// revise the same plan — the handler must detect the conflict via
/// `active_run_for` and return 409.
#[tokio::test(flavor = "multi_thread")]
async fn revise_returns_409_while_active_run_exists() {
    let plan_id = "my-dir-plan";
    let (_dir, state) = make_state(plan_id, RevisionFixture::Success { task_count: 1 }).await;

    // Start an execution run — the stub's `run_plan` blocks forever, keeping
    // the entry in `active_plans` until the test ends.
    let exec_resp = build_app(Arc::clone(&state))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/plans/{plan_id}/execute"))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("execute request");
    assert_eq!(
        exec_resp.status(),
        StatusCode::ACCEPTED,
        "execute must return 202 before revise"
    );

    // Now attempt to revise — must be rejected as 409.
    let revise_resp = build_app(Arc::clone(&state))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/plans/{plan_id}/revise"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"feedback":"some feedback"}"#))
                .expect("build request"),
        )
        .await
        .expect("revise request");

    assert_eq!(
        revise_resp.status(),
        StatusCode::CONFLICT,
        "revise must return 409 while an active run exists"
    );
}
