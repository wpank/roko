//! Integration tests for plan execution through the HTTP router.
//!
//! These tests prove that:
//! 1. `POST /api/plans/:id/execute` on a directory-layout plan returns HTTP 202
//!    with a run id, rather than the 404 the deprecated stub used to produce.
//! 2. A second execute while the first is still active is queued behind it:
//!    HTTP 202 with `queued: true` and its position (decision 9105).
//! 3. `POST /api/plans/:id/execute` on a nonexistent plan id returns HTTP 404.
//! 4. `POST /api/plans/:id/cancel` on an active plan returns HTTP 200 with
//!    `{ "cancelled": true }` and ends the run, so a subsequent
//!    `GET /api/plans/:id/status` reports it `cancelled` and finished (G43).
//! 5. The run id a 202 returns is the one the runtime runs the plan under,
//!    for a single plan and for a plan set.
//! 6. A plan that fails validation is refused with 422 and its report, and no
//!    run starts.
//!
//! A stub `CliRuntime` is used so no real agent is dispatched.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use roko_core::config::ServeAuthConfig;
use roko_core::config::schema::RokoConfig;
use roko_serve::deploy::create_backend;
use roko_serve::plan_types::{PlanDiagnosticDto, PlanSummaryDto, PlanTasksDto, PlanValidationDto};
use roko_serve::routes::build_router;
use roko_serve::runtime::{
    CliRuntime, DashboardInfo, PlanExecutionResult, PlanRunOptions, RunResult, SessionStatusInfo,
};
use roko_serve::state::AppState;
use tempfile::tempdir;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// Stub runtime
// ---------------------------------------------------------------------------

/// A plan the stub's validation refuses.
const INVALID_PLAN_ID: &str = "invalid-plan";

/// A minimal `CliRuntime` that:
/// - returns a synthetic `PlanSummaryDto` for plan ids in `known_ids`
/// - returns `Ok(None)` for all other ids (produces HTTP 404 in the handler)
/// - blocks in `run_plan` until the Tokio task is cancelled / aborted
///   (simulates a long-running plan so the active-plans map stays populated
///   across the 409 and cancel assertions)
struct StubRuntime {
    known_ids: HashSet<String>,
    /// The run id each `run_plan_with_options` call was given.
    run_ids: Mutex<Vec<Option<String>>>,
}

impl StubRuntime {
    fn new(known_ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            known_ids: known_ids.into_iter().map(|s| s.into()).collect(),
            run_ids: Mutex::default(),
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
impl CliRuntime for StubRuntime {
    async fn run_once(&self, _workdir: &Path, _prompt: &str) -> anyhow::Result<RunResult> {
        Ok(RunResult {
            success: true,
            output_text: None,
            usage: None,
            gate_results: Vec::new(),
        })
    }

    /// Block until the task is cancelled / aborted. This keeps the entry in
    /// `active_plans` so the 409 and cancel assertions can observe it.
    async fn run_plan(
        &self,
        _workdir: &Path,
        _plan_target: &Path,
    ) -> anyhow::Result<PlanExecutionResult> {
        // Never resolves on its own; the `execute_plan` handler races this
        // against the cancel token so the task exits cleanly on cancel.
        std::future::pending::<()>().await;
        unreachable!()
    }

    /// Record the run id the handler gave the run, then block like `run_plan`.
    async fn run_plan_with_options(
        &self,
        workdir: &Path,
        plan_target: &Path,
        options: PlanRunOptions,
    ) -> anyhow::Result<PlanExecutionResult> {
        let run_id = options.run_id;
        self.run_ids.lock().expect("lock run ids").push(run_id);
        self.run_plan(workdir, plan_target).await
    }

    /// A run of [`INVALID_PLAN_ID`] fails validation with one blocking error.
    async fn validate_plan_run(
        &self,
        _workdir: &Path,
        plan_target: &Path,
        only_plans: Option<&[String]>,
    ) -> anyhow::Result<Option<PlanValidationDto>> {
        let names_it = only_plans.is_some_and(|ids| ids.iter().any(|id| id == INVALID_PLAN_ID));
        if !plan_target.ends_with(INVALID_PLAN_ID) && !names_it {
            return Ok(None);
        }
        Ok(Some(PlanValidationDto::from_diagnostics(vec![
            PlanDiagnosticDto {
                severity: "error".to_string(),
                rule_id: "PLAN_005".to_string(),
                task_id: Some("T1".to_string()),
                message: "task 'T1' depends on unknown task 'T9'".to_string(),
            },
        ])))
    }

    /// A set runs the plans it names, in the order given.
    async fn plan_run_order(
        &self,
        _workdir: &Path,
        _plan_target: &Path,
        only_plans: Option<Vec<String>>,
    ) -> anyhow::Result<Vec<String>> {
        Ok(only_plans.unwrap_or_default())
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

/// Build a test `AppState` backed by `StubRuntime` seeded with `plan_id`.
///
/// Also creates a plan directory under `<workdir>/plans/<plan_id>/` so the
/// path that `execute_plan` passes to `run_plan` is a real directory.
async fn make_state(plan_id: &str) -> (tempfile::TempDir, Arc<AppState>) {
    make_state_with(plan_id, Arc::new(StubRuntime::new([plan_id]))).await
}

/// [`make_state`] backed by `runtime`, so a test can read what it recorded.
async fn make_state_with(
    plan_id: &str,
    runtime: Arc<StubRuntime>,
) -> (tempfile::TempDir, Arc<AppState>) {
    let dir = tempdir().expect("tempdir");
    let workdir = dir.path().to_path_buf();

    // Create the plan directory at the top-level `plans/` location so that
    // `plans_dir()` returns it rather than the `.roko/plans` fallback.
    let plan_dir = workdir.join("plans").join(plan_id);
    tokio::fs::create_dir_all(&plan_dir)
        .await
        .expect("create plan dir");
    // Write a minimal tasks.toml so the directory looks like a real plan.
    tokio::fs::write(
        plan_dir.join("tasks.toml"),
        "[meta]\ntitle = \"Stub plan\"\n\n[[tasks]]\nid = \"T1\"\ndescription = \"stub task\"\n",
    )
    .await
    .expect("write tasks.toml");

    let runtime: Arc<dyn CliRuntime> = runtime;
    let deploy_backend =
        Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
    let state = Arc::new(
        AppState::new(workdir, runtime, RokoConfig::default(), deploy_backend)
            .expect("AppState::new"),
    );
    (dir, state)
}

/// Build the axum router with auth disabled (matches the integration test style
/// used throughout `api_integration.rs`).
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// 1. Execute on a directory plan returns HTTP 202 and a run id.
///
/// Proves that the handler resolves directory-layout plans via the runtime's
/// `load_plan_summary` (not the deprecated flat-file `find_plan` helper that
/// always returned 404 for directory plans).
#[tokio::test(flavor = "multi_thread")]
async fn execute_directory_plan_returns_202_with_run_id() {
    let plan_id = "my-dir-plan";
    let (_dir, state) = make_state(plan_id).await;
    let app = build_app(Arc::clone(&state));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/plans/{plan_id}/execute"))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(
        response.status(),
        StatusCode::ACCEPTED,
        "directory plan execute must return 202"
    );

    let payload = body_json(response).await;
    assert!(
        payload.get("id").and_then(|v| v.as_str()).is_some(),
        "response must carry a run id: {payload}"
    );
    let run_id = payload["id"].as_str().unwrap();
    assert!(!run_id.is_empty(), "run id must not be empty");
}

/// 2. A second execute while the first is active is queued behind it: HTTP
///    202 with `queued: true` and position 1 (decision 9105).
///
/// The handler performs a check-and-insert under a single write-lock so there
/// is no TOCTOU window.
#[tokio::test(flavor = "multi_thread")]
async fn second_execute_while_active_is_queued() {
    let plan_id = "my-dir-plan";
    let (_dir, state) = make_state(plan_id).await;

    // First execute — should succeed.
    let app1 = build_app(Arc::clone(&state));
    let r1 = app1
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/plans/{plan_id}/execute"))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("send first execute");
    assert_eq!(
        r1.status(),
        StatusCode::ACCEPTED,
        "first execute must return 202"
    );

    // Second execute while the first is still active — queued, not refused.
    let app2 = build_app(Arc::clone(&state));
    let r2 = app2
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/plans/{plan_id}/execute"))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("send second execute");
    assert_eq!(
        r2.status(),
        StatusCode::ACCEPTED,
        "second execute while active must be queued with 202"
    );
    let payload = body_json(r2).await;
    assert_eq!(payload["queued"], true, "{payload}");
    assert_eq!(payload["position"], 1, "{payload}");
}

/// 3. Execute on a nonexistent plan id returns HTTP 404.
///
/// The stub runtime returns `Ok(None)` for unknown ids, which the handler maps
/// to a 404 response.
#[tokio::test]
async fn execute_nonexistent_plan_returns_404() {
    let plan_id = "my-dir-plan";
    let (_dir, state) = make_state(plan_id).await;
    let app = build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/does-not-exist/execute")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "nonexistent plan must return 404"
    );
}

/// 4. Cancel on an active plan returns success and ends the run, so a
///    subsequent status request reports it cancelled (G43).
///
/// Proves that:
/// - `POST /api/plans/:id/cancel` returns HTTP 200 with `{ "cancelled": true }`
/// - The cancel handler records the run as `cancelled` and keeps its handle
/// - `GET /api/plans/:id/status` then returns 200 with `status: "cancelled"`
///   and `finished: true`
#[tokio::test(flavor = "multi_thread")]
async fn cancel_active_plan_ends_it_cancelled() {
    let plan_id = "my-dir-plan";
    let (_dir, state) = make_state(plan_id).await;

    // Start execution.
    let app_exec = build_app(Arc::clone(&state));
    let exec_resp = app_exec
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
        "execute must return 202 before cancel"
    );

    // Verify the plan is now active (status returns 200).
    let app_status_before = build_app(Arc::clone(&state));
    let status_before = app_status_before
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/plans/{plan_id}/status"))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("status request");
    assert_eq!(
        status_before.status(),
        StatusCode::OK,
        "status must return 200 while plan is active"
    );

    // Cancel the active plan.
    let app_cancel = build_app(Arc::clone(&state));
    let cancel_resp = app_cancel
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/plans/{plan_id}/cancel"))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("cancel request");
    assert_eq!(
        cancel_resp.status(),
        StatusCode::OK,
        "cancel must return 200"
    );
    let cancel_payload = body_json(cancel_resp).await;
    assert_eq!(
        cancel_payload["cancelled"], true,
        "cancel response must contain {{ \"cancelled\": true }}: {cancel_payload}"
    );

    // After cancel, the run is over and reports how it ended.
    let app_status_after = build_app(Arc::clone(&state));
    let status_after = app_status_after
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/plans/{plan_id}/status"))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("status-after-cancel request");
    assert_eq!(
        status_after.status(),
        StatusCode::OK,
        "status must still answer after the plan has been cancelled"
    );
    let status_payload = body_json(status_after).await;
    assert_eq!(status_payload["status"], "cancelled", "{status_payload}");
    assert_eq!(status_payload["finished"], true, "{status_payload}");
}

/// 5. bug-4f833d: the run id a 202 returns is the one the runtime runs the
///    plan under, for a single plan and for a plan set, so a client can find
///    the run's events, status and checkpoint by it.
#[tokio::test(flavor = "multi_thread")]
async fn execute_passes_run_id_to_runtime() {
    let plan_id = "my-dir-plan";
    for (uri, body) in [
        (format!("/api/plans/{plan_id}/execute"), String::new()),
        (
            "/api/plans/execute".to_string(),
            format!(r#"{{"plans":["{plan_id}"]}}"#),
        ),
    ] {
        let runtime = Arc::new(StubRuntime::new([plan_id]));
        let (_dir, state) = make_state_with(plan_id, Arc::clone(&runtime)).await;
        let response = build_app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&uri)
                    .body(Body::from(body))
                    .expect("build request"),
            )
            .await
            .expect("send request");
        assert_eq!(response.status(), StatusCode::ACCEPTED, "{uri}");
        let payload = body_json(response).await;
        let run_id = payload["id"].as_str().expect("the 202 names its run");
        assert_eq!(payload["run_id"], run_id, "{uri}: {payload}");

        // The run starts on a spawned task: wait until the runtime sees it.
        let seen = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if let Some(seen) = runtime.run_ids.lock().expect("lock run ids").first() {
                    return seen.clone();
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("the run starts");
        assert_eq!(seen.as_deref(), Some(run_id), "{uri}");
    }
}

/// 6. gap-655d19: a plan `roko plan run` would refuse is refused with 422 and
///    its validation report, through either route, and no run starts.
#[tokio::test(flavor = "multi_thread")]
async fn execute_refuses_a_plan_that_fails_validation() {
    let runtime = Arc::new(StubRuntime::new([INVALID_PLAN_ID]));
    let (_dir, state) = make_state_with(INVALID_PLAN_ID, Arc::clone(&runtime)).await;
    for (uri, body) in [
        (
            format!("/api/plans/{INVALID_PLAN_ID}/execute"),
            String::new(),
        ),
        (
            "/api/plans/execute".to_string(),
            format!(r#"{{"plans":["{INVALID_PLAN_ID}"]}}"#),
        ),
    ] {
        let response = build_app(Arc::clone(&state))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&uri)
                    .body(Body::from(body))
                    .expect("build request"),
            )
            .await
            .expect("send request");
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY, "{uri}");
        let payload = body_json(response).await;
        let details = &payload["details"];
        assert_eq!(details["valid"], false, "{uri}: {payload}");
        assert_eq!(
            details["diagnostics"][0]["rule_id"], "PLAN_005",
            "{uri}: {payload}"
        );
    }
    assert!(
        runtime.run_ids.lock().expect("lock run ids").is_empty(),
        "a run started"
    );
}
