//! Integration tests for `POST /api/plans/execute` (the plan-set endpoint).
//!
//! Proves that:
//! 1. `POST /api/plans/execute` with `{}` returns 202 with the stub's fixture
//!    order and the stub received the workspace plans root.
//! 2. `{"plans":["b","a"]}` passes `only_plans` through; `{"target":"plans/x"}`
//!    passes that directory; `{"target":"../"}`, an absolute path, and a body
//!    with both fields each return 400; an unknown id returns 404.
//! 3. While a set run is active, `POST /api/plans/execute` and
//!    `POST /api/plans/{id}/execute` both return 409.
//! 4. `GET /api/plans/{member}/status` finds the run through a member plan id;
//!    `POST /api/plans/{member}/cancel` returns 200 and the stub observed its
//!    token cancelled; a subsequent status check returns 404.
//! 5. Once a run has finished, executing again does not return 409.
//! 6. `POST /api/plans/{id}/execute` with `{"resume":true}` passes
//!    `force_resume`; with no body it passes `fresh`.
//! 7. `{"max_parallel_plans":3}` reaches the stub and is echoed in the
//!    response; `{"max_parallel_plans":0}` returns 422; with no value the
//!    response echoes the config's `[conductor] max_parallel_plans`.
//!
//! A stub `CliRuntime` is used so no real agent is dispatched.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use roko_core::config::ServeAuthConfig;
use roko_core::config::schema::RokoConfig;
use roko_serve::deploy::create_backend;
use roko_serve::plan_types::{PlanSummaryDto, PlanTasksDto};
use roko_serve::routes::build_router;
use roko_serve::runtime::{
    CliRuntime, DashboardInfo, PlanExecutionResult, PlanRunOptions, RunResult, SessionStatusInfo,
};
use roko_serve::state::AppState;
use tempfile::tempdir;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// Fixture constants
// ---------------------------------------------------------------------------

/// Plan ids the stub recognises.
const KNOWN_IDS: &[&str] = &["a", "b", "my-plan"];

/// Fixed execution order returned by `plan_run_order` when running all plans.
const FIXTURE_ORDER: &[&str] = &["a", "b"];

// ---------------------------------------------------------------------------
// Stub runtime
// ---------------------------------------------------------------------------

/// A single call recorded by `run_plan_with_options`.
struct RecordedPlanSetCall {
    workdir: PathBuf,
    plan_target: PathBuf,
    fresh: bool,
    force_resume: bool,
    only_plans: Option<Vec<String>>,
    max_parallel_plans: Option<usize>,
}

/// A `CliRuntime` stub that:
/// - records calls to `run_plan_with_options` (target, options)
/// - waits until its cancel token fires or `run_timeout_ms` elapses
/// - answers `plan_run_order` from the fixture, erroring on unknown ids
/// - answers `load_plan_summary` / `load_plan_tasks` for `KNOWN_IDS`
struct StubSetRuntime {
    known_ids: HashSet<String>,
    fixture_order: Vec<String>,
    recorded_calls: Arc<Mutex<Vec<RecordedPlanSetCall>>>,
    /// Set to `true` when the cancel token observed in `run_plan_with_options` fires.
    cancel_observed: Arc<AtomicBool>,
    /// How long (ms) `run_plan_with_options` sleeps before returning.
    /// `None` means block indefinitely — only the cancel token can unblock it.
    run_timeout_ms: Option<u64>,
}

impl StubSetRuntime {
    /// Creates a stub that blocks until its cancel token fires.
    fn new_blocking() -> Self {
        Self {
            known_ids: KNOWN_IDS.iter().map(|s| s.to_string()).collect(),
            fixture_order: FIXTURE_ORDER.iter().map(|s| s.to_string()).collect(),
            recorded_calls: Arc::new(Mutex::new(Vec::new())),
            cancel_observed: Arc::new(AtomicBool::new(false)),
            run_timeout_ms: None,
        }
    }

    /// Creates a stub that returns quickly (after 5 ms) so tests that need a
    /// completed run can do so without cancelling it first.
    fn new_fast() -> Self {
        Self {
            known_ids: KNOWN_IDS.iter().map(|s| s.to_string()).collect(),
            fixture_order: FIXTURE_ORDER.iter().map(|s| s.to_string()).collect(),
            recorded_calls: Arc::new(Mutex::new(Vec::new())),
            cancel_observed: Arc::new(AtomicBool::new(false)),
            run_timeout_ms: Some(5),
        }
    }

    fn make_summary(id: &str) -> PlanSummaryDto {
        PlanSummaryDto {
            id: id.to_string(),
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
        }
    }
}

#[async_trait::async_trait]
impl CliRuntime for StubSetRuntime {
    async fn run_once(&self, _workdir: &Path, _prompt: &str) -> anyhow::Result<RunResult> {
        Ok(RunResult {
            success: true,
            output_text: None,
            usage: None,
            gate_results: Vec::new(),
        })
    }

    /// Records the call arguments, then waits until the cancel token fires
    /// or `run_timeout_ms` elapses, whichever comes first.
    async fn run_plan_with_options(
        &self,
        workdir: &Path,
        plan_target: &Path,
        options: PlanRunOptions,
    ) -> anyhow::Result<PlanExecutionResult> {
        // Record the call before blocking.
        {
            let mut calls = self.recorded_calls.lock().expect("lock recorded_calls");
            calls.push(RecordedPlanSetCall {
                workdir: workdir.to_path_buf(),
                plan_target: plan_target.to_path_buf(),
                fresh: options.fresh,
                force_resume: options.force_resume,
                only_plans: options.only_plans.clone(),
                max_parallel_plans: options.max_parallel_plans,
            });
        }

        let cancel_observed = Arc::clone(&self.cancel_observed);

        match (options.cancel.as_ref(), self.run_timeout_ms) {
            // Cancel-only mode: block until the token fires.
            (Some(token), None) => {
                token.cancelled().await;
                cancel_observed.store(true, Ordering::SeqCst);
            }
            // Both cancel and timeout: whichever arrives first.
            (Some(token), Some(ms)) => {
                tokio::select! {
                    _ = token.cancelled() => {
                        cancel_observed.store(true, Ordering::SeqCst);
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_millis(ms)) => {}
                }
            }
            // Timeout only (no cancel token).
            (None, Some(ms)) => {
                tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
            }
            // No token, no timeout: block forever (should not happen in tests).
            (None, None) => {
                std::future::pending::<()>().await;
            }
        }

        Ok(PlanExecutionResult {
            success: true,
            output_text: None,
            gate_results: Vec::new(),
        })
    }

    /// Returns the fixture order for any known plan ids; errors for unknown ones.
    async fn plan_run_order(
        &self,
        _workdir: &Path,
        _plan_target: &Path,
        only_plans: Option<Vec<String>>,
    ) -> anyhow::Result<Vec<String>> {
        if let Some(ref ids) = only_plans {
            for id in ids {
                if !self.known_ids.contains(id.as_str()) {
                    anyhow::bail!("unknown plan: {id}");
                }
            }
            return Ok(ids.clone());
        }
        Ok(self.fixture_order.clone())
    }

    async fn load_plan_summary(
        &self,
        _workdir: &Path,
        plan_id: &str,
    ) -> anyhow::Result<Option<PlanSummaryDto>> {
        if self.known_ids.contains(plan_id) {
            Ok(Some(Self::make_summary(plan_id)))
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
            }))
        } else {
            Ok(None)
        }
    }

    async fn list_plans(&self, _workdir: &Path) -> anyhow::Result<Vec<PlanSummaryDto>> {
        Ok(self.known_ids.iter().map(|id| Self::make_summary(id)).collect())
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

/// Build a test `AppState` backed by the given stub runtime.
///
/// Creates `<workdir>/plans/` (the top-level plans root used by `plans_dir()`)
/// and `<workdir>/plans/x/` for target-directory tests.
async fn make_state(runtime: Arc<StubSetRuntime>) -> (tempfile::TempDir, Arc<AppState>) {
    let dir = tempdir().expect("tempdir");
    let workdir = dir.path().to_path_buf();

    let plans_root = workdir.join("plans");
    tokio::fs::create_dir_all(&plans_root)
        .await
        .expect("create plans root");
    tokio::fs::create_dir_all(plans_root.join("x"))
        .await
        .expect("create plans/x");

    let deploy_backend =
        Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
    let state = Arc::new(
        AppState::new(
            workdir,
            runtime as Arc<dyn CliRuntime>,
            RokoConfig::default(),
            deploy_backend,
        )
        .expect("AppState::new"),
    );
    (dir, state)
}

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

/// 1. `POST /api/plans/execute` with `{}` returns 202 with the fixture order,
///    and the stub received the workspace plans root as `plan_target`.
#[tokio::test(flavor = "multi_thread")]
async fn execute_plans_all_returns_202_with_fixture_order() {
    let runtime = Arc::new(StubSetRuntime::new_fast());
    let recorded = Arc::clone(&runtime.recorded_calls);
    let (_dir, state) = make_state(runtime).await;
    let plans_root = state.workdir.join("plans");

    let app = build_app(Arc::clone(&state));
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/execute")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(resp.status(), StatusCode::ACCEPTED, "must return 202");

    let payload = body_json(resp).await;
    assert!(
        payload.get("id").and_then(|v| v.as_str()).is_some(),
        "response must carry a run id: {payload}"
    );

    let order: Vec<&str> = payload["order"]
        .as_array()
        .expect("order must be array")
        .iter()
        .map(|v| v.as_str().expect("order entry must be string"))
        .collect();
    assert_eq!(order, FIXTURE_ORDER, "order must match fixture: {payload}");

    // Give the background task a moment to record its call before inspecting it.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let calls = recorded.lock().expect("lock calls");
    assert_eq!(calls.len(), 1, "run_plan_with_options must be called exactly once");
    assert_eq!(calls[0].workdir, state.workdir, "stub must receive the workspace workdir");
    assert_eq!(
        calls[0].plan_target, plans_root,
        "stub must receive the plans root (no target or plans filter)"
    );
}

/// 2a. `{"plans":["b","a"]}` forwards `only_plans` to the stub as-is.
#[tokio::test(flavor = "multi_thread")]
async fn execute_plans_only_plans_passes_through() {
    let runtime = Arc::new(StubSetRuntime::new_fast());
    let recorded = Arc::clone(&runtime.recorded_calls);
    let (_dir, state) = make_state(runtime).await;

    let app = build_app(Arc::clone(&state));
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/execute")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"plans":["b","a"]}"#))
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(resp.status(), StatusCode::ACCEPTED, "must return 202");

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let calls = recorded.lock().expect("lock calls");
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0].only_plans,
        Some(vec!["b".to_string(), "a".to_string()]),
        "only_plans must be forwarded verbatim: {:?}",
        calls[0].only_plans
    );
}

/// 2b. `{"target":"plans/x"}` passes that canonicalised directory to the stub.
#[tokio::test(flavor = "multi_thread")]
async fn execute_plans_target_passes_directory() {
    let runtime = Arc::new(StubSetRuntime::new_fast());
    let recorded = Arc::clone(&runtime.recorded_calls);
    let (_dir, state) = make_state(runtime).await;
    let expected_target = state.workdir.join("plans").join("x").canonicalize().unwrap();

    let app = build_app(Arc::clone(&state));
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/execute")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"target":"plans/x"}"#))
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(resp.status(), StatusCode::ACCEPTED, "must return 202");

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let calls = recorded.lock().expect("lock calls");
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0].plan_target, expected_target,
        "plan_target must be the canonicalized plans/x directory"
    );
}

/// 2c. `"../"`-escape, absolute path, and `plans`+`target` together each return 400.
#[tokio::test]
async fn execute_plans_bad_inputs_return_400() {
    let runtime = Arc::new(StubSetRuntime::new_fast());
    let (_dir, state) = make_state(runtime).await;

    for (label, body) in [
        ("dotdot target", r#"{"target":"../"}"#),
        ("absolute target", r#"{"target":"/tmp/foo"}"#),
        ("plans and target together", r#"{"plans":["a"],"target":"plans/x"}"#),
    ] {
        let app = build_app(Arc::clone(&state));
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/plans/execute")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .expect("build request"),
            )
            .await
            .expect("send request");

        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "{label} must return 400"
        );
    }
}

/// 2d. An unknown id inside `plans` returns 404.
#[tokio::test]
async fn execute_plans_unknown_id_returns_404() {
    let runtime = Arc::new(StubSetRuntime::new_fast());
    let (_dir, state) = make_state(runtime).await;

    let app = build_app(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/execute")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"plans":["does-not-exist"]}"#))
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(resp.status(), StatusCode::NOT_FOUND, "unknown plan id must return 404");
}

/// 3. While a set run is active, both `POST /api/plans/execute` and
///    `POST /api/plans/{id}/execute` return 409.
#[tokio::test(flavor = "multi_thread")]
async fn execute_plans_409_while_set_run_active() {
    let runtime = Arc::new(StubSetRuntime::new_blocking());
    let (_dir, state) = make_state(runtime).await;

    // First call — starts a blocking run.
    let app1 = build_app(Arc::clone(&state));
    let r1 = app1
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/execute")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("build request"),
        )
        .await
        .expect("send first execute");
    assert_eq!(r1.status(), StatusCode::ACCEPTED, "first execute must return 202");

    // Second `POST /api/plans/execute` while first is still active → 409.
    let app2 = build_app(Arc::clone(&state));
    let r2 = app2
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/execute")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("build request"),
        )
        .await
        .expect("send second execute");
    assert_eq!(
        r2.status(),
        StatusCode::CONFLICT,
        "second POST /api/plans/execute while active must return 409"
    );

    // `POST /api/plans/a/execute` while set run is active → also 409.
    let app3 = build_app(Arc::clone(&state));
    let r3 = app3
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/a/execute")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("send single-plan execute");
    assert_eq!(
        r3.status(),
        StatusCode::CONFLICT,
        "POST /api/plans/a/execute while set run active must return 409"
    );
}

/// 4. `GET /api/plans/{member}/status` finds the run via a member plan id.
///    `POST /api/plans/{member}/cancel` returns 200 and the stub observed its
///    token cancelled. A subsequent status check returns 404.
#[tokio::test(flavor = "multi_thread")]
async fn execute_plans_status_and_cancel_via_member_id() {
    let runtime = Arc::new(StubSetRuntime::new_blocking());
    let cancel_observed = Arc::clone(&runtime.cancel_observed);
    let (_dir, state) = make_state(runtime).await;

    // Start a set run. fixture_order is ["a", "b"], so both are members.
    let app_exec = build_app(Arc::clone(&state));
    let exec_resp = app_exec
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/execute")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("build request"),
        )
        .await
        .expect("send execute");
    assert_eq!(exec_resp.status(), StatusCode::ACCEPTED, "execute must return 202");

    // Status via member id "a" must return 200.
    let app_status = build_app(Arc::clone(&state));
    let status_resp = app_status
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/plans/a/status")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("send status");
    assert_eq!(status_resp.status(), StatusCode::OK, "member status must return 200");

    // Cancel via member id "b" must return 200 with `{cancelled: true}`.
    let app_cancel = build_app(Arc::clone(&state));
    let cancel_resp = app_cancel
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/b/cancel")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("send cancel");
    assert_eq!(cancel_resp.status(), StatusCode::OK, "cancel must return 200");
    let cancel_payload = body_json(cancel_resp).await;
    assert_eq!(
        cancel_payload["cancelled"], true,
        "cancel response must have cancelled:true: {cancel_payload}"
    );

    // The cancel handler fires the token and waits a grace period for the
    // stub to observe it.  Give it a little extra time here.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    assert!(
        cancel_observed.load(Ordering::SeqCst),
        "stub must have observed the cancel token"
    );

    // After cancel the entry is removed; status via member id must return 404.
    let app_status_after = build_app(Arc::clone(&state));
    let status_after = app_status_after
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/plans/a/status")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("send status after cancel");
    assert_eq!(
        status_after.status(),
        StatusCode::NOT_FOUND,
        "status must return 404 after cancel"
    );
}

/// 5. Once a run has finished, executing the same set again does not return 409.
#[tokio::test(flavor = "multi_thread")]
async fn execute_plans_no_conflict_after_run_finishes() {
    let runtime = Arc::new(StubSetRuntime::new_fast());
    let (_dir, state) = make_state(runtime).await;

    // First execute — starts a fast-completing run.
    let app1 = build_app(Arc::clone(&state));
    let r1 = app1
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/execute")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("build request"),
        )
        .await
        .expect("first execute");
    assert_eq!(r1.status(), StatusCode::ACCEPTED, "first execute must return 202");

    // Wait for the background task to complete (stub returns after 5 ms).
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Second execute — previous run has finished (handle.is_finished()), so no conflict.
    let app2 = build_app(Arc::clone(&state));
    let r2 = app2
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/execute")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("build request"),
        )
        .await
        .expect("second execute");
    assert_eq!(
        r2.status(),
        StatusCode::ACCEPTED,
        "second execute after run finishes must return 202, not 409"
    );
}

/// 6. `POST /api/plans/{id}/execute` with `{"resume":true}` sets `force_resume`;
///    with no body it sets `fresh`.
#[tokio::test(flavor = "multi_thread")]
async fn execute_plan_resume_and_fresh_options() {
    let runtime = Arc::new(StubSetRuntime::new_fast());
    let recorded = Arc::clone(&runtime.recorded_calls);
    let (_dir, state) = make_state(runtime).await;

    // Create the plan directory so the handler can derive plan_dir (group=None → plans/<id>).
    let plan_dir = state.workdir.join("plans").join("my-plan");
    tokio::fs::create_dir_all(&plan_dir).await.expect("create plan dir");

    // No body → fresh=true, force_resume=false.
    let app1 = build_app(Arc::clone(&state));
    let r1 = app1
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/my-plan/execute")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("send no-body execute");
    assert_eq!(r1.status(), StatusCode::ACCEPTED, "no-body execute must return 202");

    // Wait for the run to complete.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    {
        let calls = recorded.lock().expect("lock calls");
        assert_eq!(calls.len(), 1, "must have one recorded call");
        assert!(
            calls[0].fresh,
            "no-body execute must set fresh=true (got fresh={})",
            calls[0].fresh
        );
        assert!(
            !calls[0].force_resume,
            "no-body execute must not set force_resume (got force_resume={})",
            calls[0].force_resume
        );
    }

    // {"resume":true} → force_resume=true, fresh=false.
    let app2 = build_app(Arc::clone(&state));
    let r2 = app2
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/my-plan/execute")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"resume":true}"#))
                .expect("build request"),
        )
        .await
        .expect("send resume execute");
    assert_eq!(r2.status(), StatusCode::ACCEPTED, "resume execute must return 202");

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    {
        let calls = recorded.lock().expect("lock calls");
        assert_eq!(calls.len(), 2, "must have two recorded calls");
        assert!(
            calls[1].force_resume,
            "resume=true must set force_resume (got force_resume={})",
            calls[1].force_resume
        );
        assert!(
            !calls[1].fresh,
            "resume=true must not set fresh (got fresh={})",
            calls[1].fresh
        );
    }
}

/// 7a. `{"max_parallel_plans":3}` is forwarded to the stub and echoed in the response.
#[tokio::test(flavor = "multi_thread")]
async fn execute_plans_max_parallel_echoed_in_response_and_stub() {
    let runtime = Arc::new(StubSetRuntime::new_fast());
    let recorded = Arc::clone(&runtime.recorded_calls);
    let (_dir, state) = make_state(runtime).await;

    let app = build_app(Arc::clone(&state));
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/execute")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"max_parallel_plans":3}"#))
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(resp.status(), StatusCode::ACCEPTED, "must return 202");
    let payload = body_json(resp).await;
    assert_eq!(
        payload["max_parallel_plans"], 3,
        "max_parallel_plans must be echoed in the 202 response: {payload}"
    );

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let calls = recorded.lock().expect("lock calls");
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0].max_parallel_plans,
        Some(3),
        "stub must receive max_parallel_plans=3 via PlanRunOptions"
    );
}

/// 7b. `{"max_parallel_plans":0}` returns 422.
#[tokio::test]
async fn execute_plans_zero_max_parallel_returns_422() {
    let runtime = Arc::new(StubSetRuntime::new_fast());
    let (_dir, state) = make_state(runtime).await;

    let app = build_app(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/execute")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"max_parallel_plans":0}"#))
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(
        resp.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "max_parallel_plans=0 must return 422"
    );
}

/// 7c. With no `max_parallel_plans`, the response echoes the config default (1).
#[tokio::test]
async fn execute_plans_defaults_to_config_max_parallel() {
    let runtime = Arc::new(StubSetRuntime::new_fast());
    let (_dir, state) = make_state(runtime).await;

    // Default `RokoConfig` has `conductor.max_parallel_plans = 1`.
    let app = build_app(Arc::clone(&state));
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/execute")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(resp.status(), StatusCode::ACCEPTED, "must return 202");
    let payload = body_json(resp).await;
    assert_eq!(
        payload["max_parallel_plans"], 1,
        "response must echo config's default max_parallel_plans=1: {payload}"
    );
}
