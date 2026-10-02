//! Tests of the plan routes and their handlers.

use super::*;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

use axum::body::{Body, to_bytes};
use axum::http::Request;
use roko_core::config::ServeAuthConfig;
use tempfile::tempdir;
use tokio::sync::Notify;
use tower::ServiceExt;

use crate::deploy::create_backend;
use crate::routes::build_router;
use crate::runtime::{
    CliRuntime, DashboardInfo, NoOpRuntime, PlanExecutionResult, RunResult, SessionStatusInfo,
};

/// Calls recorded by `RecordingRuntime`.
///
/// `run_once` entries: `("once", workdir_string, prompt)`.
/// `run_plan` entries: `("plan", workdir_string, plan_target_string)`.
#[derive(Clone, Debug)]
struct RecordedCall {
    kind: &'static str,
    workdir: PathBuf,
    arg: String,
}

#[derive(Clone)]
struct RecordingRuntime {
    calls: Arc<Mutex<Vec<RecordedCall>>>,
    notify: Arc<Notify>,
    success: bool,
    call_count: Arc<AtomicUsize>,
    /// Optional group returned from `load_plan_summary` so tests can
    /// assert that the plan directory is derived from the group.
    group: Option<String>,
    /// Most-recently-received `PlanRunOptions` from `run_plan_with_options`.
    last_options: Arc<Mutex<Option<PlanRunOptions>>>,
    /// When `Some(id)`, `load_plan_summary` and `load_plan_tasks` return
    /// data only for that plan id; any other id returns `Ok(None)` (404).
    /// When `None`, all ids are accepted (backward-compat default).
    known_plan_id: Option<String>,
    /// Tasks returned by `load_plan_tasks`.  When empty, a single default
    /// task (id="T1", tier="focused", completed=false) is synthesised.
    plan_tasks: Vec<crate::plan_types::PlanTaskDto>,
    /// Optional `estimated_minutes` returned from `load_plan_summary`.
    summary_estimated_minutes: Option<u32>,
}

#[async_trait::async_trait]
impl CliRuntime for RecordingRuntime {
    async fn run_once(&self, workdir: &std::path::Path, prompt: &str) -> anyhow::Result<RunResult> {
        self.calls.lock().expect("lock calls").push(RecordedCall {
            kind: "once",
            workdir: workdir.to_path_buf(),
            arg: prompt.to_string(),
        });
        self.call_count.fetch_add(1, Ordering::SeqCst);
        self.notify.notify_waiters();
        Ok(RunResult {
            success: self.success,
            output_text: None,
            usage: None,
            gate_results: Vec::new(),
        })
    }

    async fn run_plan(
        &self,
        workdir: &std::path::Path,
        plan_target: &std::path::Path,
    ) -> anyhow::Result<PlanExecutionResult> {
        self.calls.lock().expect("lock calls").push(RecordedCall {
            kind: "plan",
            workdir: workdir.to_path_buf(),
            arg: plan_target.to_string_lossy().into_owned(),
        });
        self.call_count.fetch_add(1, Ordering::SeqCst);
        self.notify.notify_waiters();
        Ok(PlanExecutionResult {
            success: self.success,
            output_text: None,
            gate_results: Vec::new(),
        })
    }

    /// Return a synthetic summary for a plan ID.
    ///
    /// When `self.known_plan_id` is `Some(id)`, only that id returns a
    /// summary; all other ids return `Ok(None)` (404).  When
    /// `known_plan_id` is `None`, every id is accepted (backward-compat
    /// default for tests that do not exercise 404 routing).
    async fn load_plan_summary(
        &self,
        _workdir: &std::path::Path,
        plan_id: &str,
    ) -> anyhow::Result<Option<crate::plan_types::PlanSummaryDto>> {
        if let Some(ref known) = self.known_plan_id {
            if plan_id != known {
                return Ok(None);
            }
        }
        let task_count = if self.plan_tasks.is_empty() {
            1
        } else {
            self.plan_tasks.len()
        };
        Ok(Some(crate::plan_types::PlanSummaryDto {
            id: plan_id.to_string(),
            title: "Test Plan".to_string(),
            task_count,
            tasks_done: 0,
            tasks_failed: 0,
            completed: false,
            status: "ready".to_string(),
            superseded_by: None,
            old_format: false,
            last_error: None,
            group: self.group.clone(),
            estimated_minutes: self.summary_estimated_minutes,
        }))
    }

    /// Return synthetic tasks for a plan ID.
    ///
    /// When `self.known_plan_id` is `Some(id)`, only that id returns
    /// tasks; all other ids return `Ok(None)`.  When `known_plan_id` is
    /// `None`, every id is accepted.  The tasks returned are
    /// `self.plan_tasks` when non-empty; otherwise a single default
    /// task is synthesised.
    async fn load_plan_tasks(
        &self,
        _workdir: &std::path::Path,
        plan_id: &str,
    ) -> anyhow::Result<Option<crate::plan_types::PlanTasksDto>> {
        if let Some(ref known) = self.known_plan_id {
            if plan_id != known {
                return Ok(None);
            }
        }
        let tasks: Vec<crate::plan_types::PlanTaskDto> = if self.plan_tasks.is_empty() {
            vec![crate::plan_types::PlanTaskDto {
                id: "T1".to_string(),
                title: "Default task".to_string(),
                description: None,
                role: None,
                tier: "focused".to_string(),
                status: "pending".to_string(),
                depends_on: vec![],
                files: vec![],
                completed: false,
                verify_phases: vec![],
                model_hint: None,
                estimated_minutes: None,
                verify: vec![],
            }]
        } else {
            self.plan_tasks.clone()
        };
        let task_count = tasks.len();
        Ok(Some(crate::plan_types::PlanTasksDto {
            plan_id: plan_id.to_string(),
            task_count,
            tasks,
            title: None,
            max_parallel: 1,
        }))
    }

    /// Override the default so that tests can inspect the options received
    /// by the executor (e.g. `force_resume`, `fresh`).
    async fn run_plan_with_options(
        &self,
        workdir: &std::path::Path,
        plan_target: &std::path::Path,
        options: PlanRunOptions,
    ) -> anyhow::Result<PlanExecutionResult> {
        *self.last_options.lock().expect("lock last_options") = Some(options);
        self.calls.lock().expect("lock calls").push(RecordedCall {
            kind: "plan",
            workdir: workdir.to_path_buf(),
            arg: plan_target.to_string_lossy().into_owned(),
        });
        self.call_count.fetch_add(1, Ordering::SeqCst);
        self.notify.notify_waiters();
        Ok(PlanExecutionResult {
            success: self.success,
            output_text: None,
            gate_results: Vec::new(),
        })
    }

    /// Return the ids from `only_plans` when given, or a single-element
    /// list so that `execute_plans` tests get a non-empty order without
    /// creating plan files on disk.
    async fn plan_run_order(
        &self,
        _workdir: &std::path::Path,
        _plan_target: &std::path::Path,
        only_plans: Option<Vec<String>>,
    ) -> anyhow::Result<Vec<String>> {
        Ok(only_plans.unwrap_or_else(|| vec!["mock-plan".to_string()]))
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

    fn dashboard_scaffold(&self, _workdir: &std::path::Path) -> DashboardInfo {
        DashboardInfo {
            rendered: String::new(),
        }
    }

    /// Return a synthetic plan source DTO.
    ///
    /// Returns `Ok(None)` when `known_plan_id` is set and does not match.
    async fn plan_source(
        &self,
        _workdir: &std::path::Path,
        plan_id: &str,
    ) -> anyhow::Result<Option<crate::plan_types::PlanSourceDto>> {
        if let Some(ref known) = self.known_plan_id {
            if plan_id != known {
                return Ok(None);
            }
        }
        Ok(Some(crate::plan_types::PlanSourceDto {
            id: plan_id.to_string(),
            path: format!("plans/{plan_id}/tasks.toml"),
            toml: "[meta]\ntitle = \"Test Plan\"\n".to_string(),
        }))
    }

    /// Validate and save a plan source text.
    ///
    /// Returns `Ok(None)` when `known_plan_id` is set and does not match.
    /// Always returns `valid: true` otherwise (test stub).
    async fn save_plan_source(
        &self,
        _workdir: &std::path::Path,
        plan_id: &str,
        _toml: String,
    ) -> anyhow::Result<Option<crate::plan_types::PlanValidationDto>> {
        if let Some(ref known) = self.known_plan_id {
            if plan_id != known {
                return Ok(None);
            }
        }
        Ok(Some(
            crate::plan_types::PlanValidationDto::from_diagnostics(vec![]),
        ))
    }

    /// Validate a plan source text without saving.
    ///
    /// Returns `Ok(None)` when `known_plan_id` is set and does not match.
    /// Always returns `valid: true` otherwise (test stub).
    async fn validate_plan_source(
        &self,
        _workdir: &std::path::Path,
        plan_id: &str,
        _toml: Option<String>,
    ) -> anyhow::Result<Option<crate::plan_types::PlanValidationDto>> {
        if let Some(ref known) = self.known_plan_id {
            if plan_id != known {
                return Ok(None);
            }
        }
        Ok(Some(
            crate::plan_types::PlanValidationDto::from_diagnostics(vec![]),
        ))
    }

    /// Create a new plan.
    ///
    /// Returns `AlreadyExists` when `known_plan_id` matches the slug
    /// (simulating a plan that is already on disk).  Otherwise returns
    /// `Created` so callers can test the happy path.
    async fn create_plan(
        &self,
        _workdir: &std::path::Path,
        slug: &str,
        _title: &str,
    ) -> anyhow::Result<crate::plan_types::CreatePlanOutcome> {
        if self.known_plan_id.as_deref() == Some(slug) {
            return Ok(crate::plan_types::CreatePlanOutcome::AlreadyExists {
                slug: slug.to_string(),
            });
        }
        Ok(crate::plan_types::CreatePlanOutcome::Created {
            slug: slug.to_string(),
            path: format!("plans/{slug}"),
        })
    }

    /// Generate a plan from a PRD; records the call and returns a
    /// synthetic result with one plan target.
    async fn generate_plan_from_prd(
        &self,
        workdir: &std::path::Path,
        slug: &str,
        prd_path: &std::path::Path,
    ) -> anyhow::Result<crate::runtime::PlanGenerationResult> {
        self.calls.lock().expect("lock calls").push(RecordedCall {
            kind: "prd_plan",
            workdir: workdir.to_path_buf(),
            arg: prd_path.to_string_lossy().into_owned(),
        });
        self.call_count.fetch_add(1, Ordering::SeqCst);
        self.notify.notify_waiters();
        let plan_dir = workdir.join("plans").join(slug);
        Ok(crate::runtime::PlanGenerationResult {
            plans_root: workdir.join("plans"),
            plan_targets: vec![plan_dir],
            artifacts: vec![],
        })
    }
}

fn test_state() -> (tempfile::TempDir, Arc<AppState>) {
    let dir = tempdir().expect("tempdir");
    let workdir = dir.path().to_path_buf();
    let deploy_backend =
        Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
    let state = Arc::new(
        AppState::new(
            workdir,
            Arc::new(NoOpRuntime),
            roko_core::config::schema::RokoConfig::default(),
            deploy_backend,
        )
        .expect("AppState::new"),
    );
    (dir, state)
}

fn test_state_with_runtime(runtime: Arc<dyn CliRuntime>) -> (tempfile::TempDir, Arc<AppState>) {
    let dir = tempdir().expect("tempdir");
    let workdir = dir.path().to_path_buf();
    let deploy_backend =
        Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
    let state = Arc::new(
        AppState::new(
            workdir,
            runtime,
            roko_core::config::schema::RokoConfig::default(),
            deploy_backend,
        )
        .expect("AppState::new"),
    );
    (dir, state)
}

#[tokio::test]
async fn get_plan_returns_404_for_missing_plan() {
    let (_dir, state) = test_state();

    let err = get_plan(State(state), Path("missing-plan".into()))
        .await
        .expect_err("missing plan should error");

    assert_eq!(err.status, axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn execute_plan_returns_404_for_missing_plan() {
    let (_dir, state) = test_state();

    let err = match execute_plan(
        State(state),
        Path("missing-plan".into()),
        axum::body::Bytes::new(),
    )
    .await
    {
        Ok(_) => panic!("missing plan should error"),
        Err(err) => err,
    };

    assert_eq!(err.status, axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn plan_status_returns_404_when_plan_is_not_active() {
    let (_dir, state) = test_state();

    let err = plan_status(State(state), Path("missing-plan".into()))
        .await
        .expect_err("missing active plan should error");

    assert_eq!(err.status, axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_plan_rejects_blank_title() {
    let request = CreatePlanRequest {
        title: "   ".into(),
        slug: None,
    };
    assert!(
        request.validate().is_err(),
        "blank title must fail validation"
    );
}

#[tokio::test]
async fn create_plan_route_returns_top_level_validation_error() {
    let (_dir, state) = test_state();
    let app = build_router(
        Arc::clone(&state),
        &[],
        ServeAuthConfig {
            enabled: false,
            ..ServeAuthConfig::default()
        },
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans")
                .body(Body::from(r#"{"title":"   ","description":"desc"}"#))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let payload: Value = serde_json::from_slice(&body).expect("parse response body");
    assert_eq!(payload["code"], "validation_error");
    assert_eq!(payload["message"], "request body validation failed");
    assert!(payload.get("error").is_none());
}

// ── slug_from_title unit tests ────────────────────────────────────────

/// bug-7feee7: a first line longer than 80 bytes whose byte 80 falls
/// inside a multi-byte character is cut at a character boundary, so the
/// slug is derived instead of the request panicking.
#[tokio::test]
async fn derive_unique_slug_cuts_a_multi_byte_first_line_at_a_char_boundary() {
    let dir = tempdir().expect("tempdir");
    let prompt = format!("a{}\nsecond line", "é".repeat(100));
    assert!(!prompt.is_char_boundary(80));

    assert_eq!(derive_unique_slug(dir.path(), &prompt).await, "a");
}

#[test]
fn slug_from_title_basic() {
    assert_eq!(slug_from_title("Hello World"), "hello-world");
}

#[test]
fn slug_from_title_collapses_runs() {
    assert_eq!(slug_from_title("foo  --  bar"), "foo-bar");
}

#[test]
fn slug_from_title_truncates_at_48() {
    let long = "a".repeat(60);
    let s = slug_from_title(&long);
    assert!(s.len() <= 48);
}

#[test]
fn slug_from_title_strips_leading_trailing_separators() {
    assert_eq!(slug_from_title("  hello  "), "hello");
}

// ── create_plan handler tests ─────────────────────────────────────────

#[tokio::test]
async fn create_plan_201_via_router() {
    let runtime = recording_runtime_for_plan("other-plan");
    let (_dir, state) = test_state_with_runtime(runtime);
    let app = build_router(
        Arc::clone(&state),
        &[],
        ServeAuthConfig {
            enabled: false,
            ..ServeAuthConfig::default()
        },
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"title":"My New Plan"}"#))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), axum::http::StatusCode::CREATED);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let payload: Value = serde_json::from_slice(&body).expect("parse response body");
    // id must be the slug derived from the title
    assert_eq!(payload["id"], "my-new-plan");
    assert_eq!(payload["path"], "plans/my-new-plan");
}

#[tokio::test]
async fn create_plan_409_when_slug_already_exists() {
    // known_plan_id == slug → runtime returns AlreadyExists → 409.
    let runtime = recording_runtime_for_plan("existing-plan");
    let (_dir, state) = test_state_with_runtime(runtime);
    let app = build_router(
        Arc::clone(&state),
        &[],
        ServeAuthConfig {
            enabled: false,
            ..ServeAuthConfig::default()
        },
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Existing Plan","slug":"existing-plan"}"#,
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), axum::http::StatusCode::CONFLICT);
}

#[tokio::test]
async fn create_plan_uses_explicit_slug_over_derived() {
    let runtime = recording_runtime_for_plan("other");
    let (_dir, state) = test_state_with_runtime(runtime);
    let app = build_router(
        Arc::clone(&state),
        &[],
        ServeAuthConfig {
            enabled: false,
            ..ServeAuthConfig::default()
        },
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"title":"Something Else","slug":"my-slug"}"#))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), axum::http::StatusCode::CREATED);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let payload: Value = serde_json::from_slice(&body).expect("parse response body");
    assert_eq!(payload["id"], "my-slug");
    assert_eq!(payload["path"], "plans/my-slug");
}

#[tokio::test]
async fn create_plan_rejects_path_traversal_slug() {
    let runtime = recording_runtime_for_plan("x");
    let (_dir, state) = test_state_with_runtime(runtime);
    let app = build_router(
        Arc::clone(&state),
        &[],
        ServeAuthConfig {
            enabled: false,
            ..ServeAuthConfig::default()
        },
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"title":"x","slug":"../etc/passwd"}"#))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn generate_plan_rejects_empty_slug() {
    // Blank slug → 422 from validate_payload.
    let req = GenerateRequest {
        slug: Some("  ".into()),
        prompt: None,
    };
    assert!(req.validate_payload().is_err());
}

#[tokio::test]
async fn generate_plan_rejects_neither_field() {
    let req = GenerateRequest {
        slug: None,
        prompt: None,
    };
    let err = req.validate_payload().unwrap_err();
    // Must be a 422.
    assert_eq!(err.status.as_u16(), 422);
}

#[tokio::test]
async fn generate_plan_rejects_both_fields() {
    let req = GenerateRequest {
        slug: Some("demo".into()),
        prompt: Some("build something".into()),
    };
    let err = req.validate_payload().unwrap_err();
    assert_eq!(err.status.as_u16(), 422);
}

#[tokio::test]
async fn generate_plan_rejects_blank_prompt() {
    let req = GenerateRequest {
        slug: None,
        prompt: Some("   ".into()),
    };
    let err = req.validate_payload().unwrap_err();
    assert_eq!(err.status.as_u16(), 422);
}

#[tokio::test]
async fn generate_plan_accepts_slug_only() {
    let req = GenerateRequest {
        slug: Some("my-plan".into()),
        prompt: None,
    };
    assert!(req.validate_payload().is_ok());
}

#[tokio::test]
async fn generate_plan_accepts_prompt_only() {
    let req = GenerateRequest {
        slug: None,
        prompt: Some("build a widget".into()),
    };
    assert!(req.validate_payload().is_ok());
}

#[tokio::test]
async fn execute_plan_runs_runtime_with_plan_context() {
    let runtime = Arc::new(RecordingRuntime {
        calls: Arc::new(Mutex::new(Vec::new())),
        notify: Arc::new(Notify::new()),
        success: true,
        call_count: Arc::new(AtomicUsize::new(0)),
        group: None,
        last_options: Arc::new(Mutex::new(None)),
        known_plan_id: None,
        plan_tasks: vec![],
        summary_estimated_minutes: None,
    });
    let notify = Arc::clone(&runtime.as_ref().notify);
    let calls = Arc::clone(&runtime.as_ref().calls);
    let (_dir, state) = test_state_with_runtime(runtime);

    // RecordingRuntime.load_plan_summary returns a synthetic DTO for any
    // plan ID, so the directory does not need to exist on disk for the
    // existence check. We create it anyway to keep the test realistic and
    // to verify the target path passed to run_plan is the directory.
    let plan_dir = state.workdir.join(".roko").join("plans").join("demo");
    tokio::fs::create_dir_all(&plan_dir)
        .await
        .expect("create plan dir");
    tokio::fs::write(
        plan_dir.join("tasks.toml"),
        "[meta]\ntitle = \"Demo Plan\"\n\n[[tasks]]\nid = \"T1\"\ndescription = \"Update the widget\"\n",
    )
    .await
    .expect("write tasks.toml");

    let response = execute_plan(
        State(Arc::clone(&state)),
        Path("demo".into()),
        axum::body::Bytes::new(),
    )
    .await
    .expect("execute plan");

    assert_eq!(
        response.into_response().status(),
        axum::http::StatusCode::ACCEPTED
    );

    tokio::time::timeout(std::time::Duration::from_secs(1), notify.notified())
        .await
        .expect("runtime should be called");

    let calls = calls.lock().expect("lock calls");
    assert_eq!(calls.len(), 1);
    // execute_plan must delegate to run_plan (not run_once) and pass the
    // plan's own directory — not a reconstructed flat .json / .toml path.
    assert_eq!(calls[0].kind, "plan", "execute_plan must call run_plan");
    assert_eq!(calls[0].workdir, state.workdir);
    assert_eq!(
        calls[0].arg,
        plan_dir.to_string_lossy(),
        "plan_target must be the plan directory, not a flat file path"
    );
}

/// A runtime whose plan `demo` resumes by replaying `skippable`.
struct ResumableRuntime {
    skippable: Vec<String>,
}

#[async_trait::async_trait]
impl CliRuntime for ResumableRuntime {
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

    async fn load_plan_summary(
        &self,
        _workdir: &std::path::Path,
        plan_id: &str,
    ) -> anyhow::Result<Option<crate::plan_types::PlanSummaryDto>> {
        Ok(Some(crate::plan_types::PlanSummaryDto {
            id: plan_id.to_string(),
            title: "Resumable plan".to_string(),
            task_count: 2,
            tasks_done: 1,
            tasks_failed: 0,
            completed: false,
            status: "ready".to_string(),
            superseded_by: None,
            old_format: false,
            last_error: None,
            group: None,
            estimated_minutes: None,
        }))
    }

    async fn resume_skippable_tasks(
        &self,
        _workdir: &std::path::Path,
        _plan_dir: &std::path::Path,
    ) -> anyhow::Result<Option<Vec<String>>> {
        Ok(Some(self.skippable.clone()))
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

    fn dashboard_scaffold(&self, _workdir: &std::path::Path) -> DashboardInfo {
        DashboardInfo {
            rendered: String::new(),
        }
    }
}

/// gap-b07969: `POST /api/plans/{id}/execute` with `{ "resume": true }`
/// says in its 202 which tasks the resume replays instead of running, and
/// a fresh run replays none.
#[tokio::test]
async fn execute_resume_reports_skippable_tasks() {
    let accepted = |body: &'static [u8]| async move {
        let runtime = Arc::new(ResumableRuntime {
            skippable: vec!["T1".to_string()],
        });
        let (_dir, state) = test_state_with_runtime(runtime);
        let response = execute_plan(
            State(state),
            Path("demo".into()),
            axum::body::Bytes::from_static(body),
        )
        .await
        .expect("execute plan")
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::ACCEPTED);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        serde_json::from_slice::<Value>(&body).expect("parse response body")
    };

    let resumed = accepted(br#"{"resume": true}"#).await;
    assert_eq!(resumed["resume"], true);
    assert_eq!(resumed["skippable_task_ids"], json!(["T1"]));
    let fresh = accepted(b"").await;
    assert_eq!(fresh["resume"], false);
    assert_eq!(fresh["skippable_task_ids"], json!([]));
}

#[tokio::test]
async fn generate_plan_runs_runtime_with_prd_context() {
    let runtime = Arc::new(RecordingRuntime {
        calls: Arc::new(Mutex::new(Vec::new())),
        notify: Arc::new(Notify::new()),
        success: true,
        call_count: Arc::new(AtomicUsize::new(0)),
        group: None,
        last_options: Arc::new(Mutex::new(None)),
        known_plan_id: None,
        plan_tasks: vec![],
        summary_estimated_minutes: None,
    });
    let notify = Arc::clone(&runtime.as_ref().notify);
    let calls = Arc::clone(&runtime.as_ref().calls);
    let (_dir, state) = test_state_with_runtime(runtime);

    let published_dir = state.workdir.join(".roko").join("prd").join("published");
    tokio::fs::create_dir_all(&published_dir)
        .await
        .expect("create published dir");
    tokio::fs::write(
        published_dir.join("demo.md"),
        "---\nstatus: published\n---\n# Demo PRD\nBuild the widget.\n",
    )
    .await
    .expect("write prd");

    let response = generate_plan(
        State(Arc::clone(&state)),
        ValidJson(GenerateRequest {
            slug: Some("demo".into()),
            prompt: None,
        }),
    )
    .await
    .expect("generate plan");

    let http_response = response.into_response();
    assert_eq!(http_response.status(), axum::http::StatusCode::ACCEPTED);

    // Verify the body contains plan_id so the portal generate hook can use it.
    let body_bytes = to_bytes(http_response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let body: Value = serde_json::from_slice(&body_bytes).expect("parse body");
    assert_eq!(
        body.get("plan_id").and_then(Value::as_str),
        Some("demo"),
        "response body must include plan_id"
    );

    tokio::time::timeout(std::time::Duration::from_secs(1), notify.notified())
        .await
        .expect("runtime should be called");

    let calls = calls.lock().expect("lock calls");
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0].kind, "prd_plan",
        "must call generate_plan_from_prd, not run_once"
    );
    assert_eq!(calls[0].workdir, state.workdir);
    assert!(
        calls[0].arg.contains(".roko/prd/published/demo.md"),
        "generate_plan_from_prd prd_path must be the published PRD: {}",
        calls[0].arg
    );
}

#[tokio::test]
async fn generate_plan_from_prompt_writes_prd_draft_and_calls_runtime() {
    let runtime = Arc::new(RecordingRuntime {
        calls: Arc::new(Mutex::new(Vec::new())),
        notify: Arc::new(Notify::new()),
        success: true,
        call_count: Arc::new(AtomicUsize::new(0)),
        group: None,
        last_options: Arc::new(Mutex::new(None)),
        known_plan_id: None,
        plan_tasks: vec![],
        summary_estimated_minutes: None,
    });
    let notify = Arc::clone(&runtime.as_ref().notify);
    let calls = Arc::clone(&runtime.as_ref().calls);
    let (_dir, state) = test_state_with_runtime(runtime);

    let response = generate_plan(
        State(Arc::clone(&state)),
        ValidJson(GenerateRequest {
            slug: None,
            prompt: Some("Build a widget library".into()),
        }),
    )
    .await
    .expect("generate plan from prompt");

    let http_response = response.into_response();
    assert_eq!(http_response.status(), axum::http::StatusCode::ACCEPTED);

    let body_bytes = to_bytes(http_response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let body: Value = serde_json::from_slice(&body_bytes).expect("parse body");
    let plan_id = body
        .get("plan_id")
        .and_then(Value::as_str)
        .expect("plan_id in response");
    assert!(!plan_id.is_empty(), "plan_id must not be empty");

    tokio::time::timeout(std::time::Duration::from_secs(1), notify.notified())
        .await
        .expect("runtime should be called");

    let calls = calls.lock().expect("lock calls");
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0].kind, "prd_plan",
        "must call generate_plan_from_prd for prompt path"
    );
    // The prd_path must be the draft we wrote.
    assert!(
        calls[0].arg.contains(".roko/prd/drafts/"),
        "prd_path must be inside drafts/: {}",
        calls[0].arg
    );
    // The draft file must exist on disk.
    let draft_path = std::path::PathBuf::from(&calls[0].arg);
    assert!(
        draft_path.is_file(),
        "PRD draft must exist on disk: {}",
        calls[0].arg
    );
}

/// A runtime whose plan generation returns without writing a plan.
struct NoPlanRuntime;

#[async_trait::async_trait]
impl CliRuntime for NoPlanRuntime {
    async fn run_once(
        &self,
        _workdir: &std::path::Path,
        _prompt: &str,
    ) -> anyhow::Result<RunResult> {
        anyhow::bail!("NoPlanRuntime only generates")
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

    fn dashboard_scaffold(&self, _workdir: &std::path::Path) -> DashboardInfo {
        DashboardInfo {
            rendered: String::new(),
        }
    }

    async fn generate_plan_from_prd(
        &self,
        workdir: &std::path::Path,
        _slug: &str,
        _prd_path: &std::path::Path,
    ) -> anyhow::Result<crate::runtime::PlanGenerationResult> {
        Ok(crate::runtime::PlanGenerationResult {
            plans_root: workdir.join("plans"),
            plan_targets: Vec::new(),
            artifacts: Vec::new(),
        })
    }
}

/// A generation that writes no plan fails its operation, with the error,
/// just as the stream reports `plan_generate.failed`: a `completed`
/// operation sends the portal to a plan that does not exist.
#[tokio::test]
async fn generate_plan_that_writes_no_plan_fails_its_operation() {
    let (_dir, state) = test_state_with_runtime(Arc::new(NoPlanRuntime));

    let response = generate_plan(
        State(Arc::clone(&state)),
        ValidJson(GenerateRequest {
            slug: None,
            prompt: Some("a rust app that prints hello world".into()),
        }),
    )
    .await
    .expect("generate plan");
    let body_bytes = to_bytes(response.into_response().into_body(), usize::MAX)
        .await
        .expect("read body");
    let body: Value = serde_json::from_slice(&body_bytes).expect("parse body");
    let op_id = body["id"].as_str().expect("operation id").to_string();

    let status = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if let Some(op) = state.operations.read().await.get(&op_id)
                && !matches!(op.status, OperationStatus::Running)
            {
                return op.status.clone();
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the operation finishes");
    match status {
        OperationStatus::Failed { error } => assert!(
            error.contains("finished without writing plan"),
            "unexpected error: {error}"
        ),
        other => panic!("a generation that wrote no plan must fail, got {other:?}"),
    }

    let lifecycle: Vec<String> = state
        .state_hub
        .replay_from(0)
        .into_iter()
        .filter_map(|envelope| match envelope.payload {
            roko_core::DashboardEvent::EventLogEntry { event_type, .. }
                if event_type.starts_with("plan_generate.") =>
            {
                Some(event_type)
            }
            _ => None,
        })
        .collect();
    assert_eq!(lifecycle, ["plan_generate.started", "plan_generate.failed"]);
}

#[tokio::test]
async fn plan_costs_reports_projection_and_budget_status() {
    // Build a runtime stub that knows the "cost-demo" plan with two tasks.
    // This replaces the previous flat-file fixture so that the test works
    // with directory-layout plans (no `.roko/plans/cost-demo.json` file).
    let runtime = Arc::new(RecordingRuntime {
        calls: Arc::new(Mutex::new(Vec::new())),
        notify: Arc::new(Notify::new()),
        success: true,
        call_count: Arc::new(AtomicUsize::new(0)),
        group: None,
        last_options: Arc::new(Mutex::new(None)),
        known_plan_id: Some("cost-demo".to_string()),
        plan_tasks: vec![
            crate::plan_types::PlanTaskDto {
                id: "T1".to_string(),
                title: "completed task".to_string(),
                description: Some("completed task".to_string()),
                role: None,
                tier: "mechanical".to_string(),
                status: "completed".to_string(),
                depends_on: vec![],
                files: vec![],
                completed: true,
                verify_phases: vec![],
                model_hint: None,
                estimated_minutes: None,
                verify: vec![],
            },
            crate::plan_types::PlanTaskDto {
                id: "T2".to_string(),
                title: "remaining task".to_string(),
                description: Some("remaining task".to_string()),
                role: None,
                tier: "focused".to_string(),
                status: "pending".to_string(),
                depends_on: vec!["T1".to_string()],
                files: vec![],
                completed: false,
                verify_phases: vec![],
                model_hint: None,
                estimated_minutes: None,
                verify: vec![],
            },
        ],
        summary_estimated_minutes: None,
    });
    let (_dir, state) = test_state_with_runtime(runtime);
    let mut config = (*state.load_roko_config()).clone();
    config.budget.max_plan_usd = 0.27;
    config.budget.max_task_usd = 1.0;
    state.roko_config.store(Arc::new(config));
    state.provider_health_registry.record_success("anthropic");

    let learn_dir = state.workdir.join(".roko").join("learn");
    tokio::fs::create_dir_all(&learn_dir)
        .await
        .expect("create learn dir");
    tokio::fs::write(
        learn_dir.join("efficiency.jsonl"),
        serde_json::to_string(&json!({
            "plan_id": "cost-demo",
            "task_id": "T1",
            "model": "claude-sonnet-4-6",
            "cost_usd": 0.25,
            "input_tokens": 1000,
            "output_tokens": 500
        }))
        .expect("serialize efficiency event"),
    )
    .await
    .expect("write efficiency log");

    let Json(payload) = plan_costs(State(state), Path("cost-demo".into()))
        .await
        .expect("cost report");

    assert_eq!(payload["total_cost_usd"], 0.25);
    assert_eq!(payload["plan_spent"], 0.25);
    assert_eq!(payload["task_costs"][0]["spent"], 0.25);
    assert!((payload["task_costs"][0]["budget"].as_f64().unwrap() - 0.2).abs() < 1e-6);
    assert_eq!(payload["task_costs"][0]["budget_exhausted"], true);
    assert_eq!(payload["provider_health"][0]["id"], "anthropic");
    assert_eq!(payload["projection"]["tasks_completed"], 1);
    assert_eq!(payload["projection"]["tasks_remaining"], 1);
    assert!(
        payload["projection"]["projected_total_usd"]
            .as_f64()
            .expect("projected total")
            > 0.25
    );
    let limit = payload["budget"]["limit_usd"]
        .as_f64()
        .expect("budget limit");
    assert!((limit - 0.27).abs() < 1e-6);
    assert_eq!(payload["budget"]["status"], "projected_exceeded");
    assert_eq!(payload["budget"]["projected_exceeded"], true);
}

/// gap-a6e2c3 records plan generation and revision spend under the task
/// ids `generate` and `revise`. That spend belongs in the plan's totals,
/// but it is not a completed task of the plan.
#[tokio::test]
async fn plan_costs_counts_generation_spend_in_the_total_but_not_as_a_task() {
    let task = |id: &str, tier: &str, completed: bool| crate::plan_types::PlanTaskDto {
        id: id.to_string(),
        title: id.to_string(),
        description: None,
        role: None,
        tier: tier.to_string(),
        status: if completed { "completed" } else { "pending" }.to_string(),
        depends_on: vec![],
        files: vec![],
        completed,
        verify_phases: vec![],
        model_hint: None,
        estimated_minutes: None,
        verify: vec![],
    };
    let runtime = Arc::new(RecordingRuntime {
        plan_tasks: vec![task("T1", "mechanical", true), task("T2", "focused", false)],
        ..(*recording_runtime_for_plan("cost-demo")).clone()
    });
    let (_dir, state) = test_state_with_runtime(runtime);

    let learn_dir = state.workdir.join(".roko").join("learn");
    tokio::fs::create_dir_all(&learn_dir)
        .await
        .expect("create learn dir");
    let rows = [("generate", 0.10), ("T1", 0.25), ("revise", 0.05)]
        .iter()
        .map(|(task_id, cost_usd)| {
            json!({
                "plan_id": "cost-demo",
                "task_id": task_id,
                "model": "claude-sonnet-4-6",
                "cost_usd": cost_usd,
            })
            .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");
    tokio::fs::write(learn_dir.join("efficiency.jsonl"), rows)
        .await
        .expect("write efficiency log");

    let Json(payload) = plan_costs(State(state), Path("cost-demo".into()))
        .await
        .expect("cost report");

    let total = payload["total_cost_usd"].as_f64().expect("total cost");
    assert!(
        (total - 0.40).abs() < 1e-9,
        "generation and revision spend stay in the total: {payload}"
    );
    assert_eq!(payload["plan_spent"], payload["total_cost_usd"]);
    assert_eq!(
        payload["projection"]["tasks_completed"], 1,
        "only T1 is a completed task: {payload}"
    );
    assert_eq!(payload["projection"]["tasks_remaining"], 1);
    let task_ids = payload["task_costs"]
        .as_array()
        .expect("task costs")
        .iter()
        .map(|task| task["task_id"].as_str().expect("task id"))
        .collect::<Vec<_>>();
    assert_eq!(task_ids, ["T1", "T2"]);
    let expected_remaining = payload["projection"]["expected_remaining_usd"]
        .as_f64()
        .expect("expected remaining");
    let projected_total = payload["projection"]["projected_total_usd"]
        .as_f64()
        .expect("projected total");
    assert!(
        (projected_total - (total + expected_remaining)).abs() < 1e-9,
        "the projected total starts from everything spent: {payload}"
    );
}

// ── plans_dir helper ────────────────────────────────────────────────

/// When `<workdir>/plans/` already exists as a directory, `plans_dir`
/// should return it rather than the legacy `.roko/plans` location.
#[test]
fn plans_dir_prefers_top_level_when_it_exists() {
    let dir = tempdir().expect("tempdir");
    let top = dir.path().join("plans");
    std::fs::create_dir_all(&top).expect("create top-level plans dir");

    let result = plans_dir(dir.path());
    assert_eq!(result, top, "should return top-level plans/ directory");
}

/// In a new workspace — no `plans/`, and at most the empty `.roko/plans`
/// that `roko init` creates — the first plan goes to `plans/`, which
/// `plans_dir` returns without creating.
#[test]
fn plans_dir_is_top_level_in_a_new_workspace() {
    let dir = tempdir().expect("tempdir");
    let expected = dir.path().join("plans");

    assert_eq!(plans_dir(dir.path()), expected);

    std::fs::create_dir_all(dir.path().join(".roko").join("plans"))
        .expect("create empty .roko/plans");
    assert_eq!(
        plans_dir(dir.path()),
        expected,
        "an empty .roko/plans must not make the workspace legacy"
    );
    // Confirm the helper did not create the directory as a side effect.
    assert!(
        !expected.exists(),
        "plans_dir must not create the top-level directory"
    );
}

/// A workspace that already keeps its plans in `.roko/plans` (and has no
/// `plans/`) goes on using it, for reads and for new plans.
#[test]
fn plans_dir_keeps_a_legacy_workspace_in_dotted_roko() {
    let dir = tempdir().expect("tempdir");
    let legacy = dir.path().join(".roko").join("plans");
    std::fs::create_dir_all(legacy.join("old-plan")).expect("create legacy plan");
    std::fs::write(legacy.join("old-plan").join("tasks.toml"), "").expect("write tasks");

    assert_eq!(plans_dir(dir.path()), legacy);
}

/// After `execute_plan` returns, the event bus must carry **no**
/// `PlanStarted` event for that plan id.  The run publishes its own
/// `PlanStarted` (with the correct `tasks_total`) via the runtime; the
/// handler must not publish a duplicate with `tasks_total: 0`.
#[tokio::test]
async fn execute_leaves_plan_started_to_the_run() {
    let runtime = Arc::new(RecordingRuntime {
        calls: Arc::new(Mutex::new(Vec::new())),
        notify: Arc::new(Notify::new()),
        success: true,
        call_count: Arc::new(AtomicUsize::new(0)),
        group: None,
        last_options: Arc::new(Mutex::new(None)),
        known_plan_id: None,
        plan_tasks: vec![],
        summary_estimated_minutes: None,
    });
    let notify = Arc::clone(&runtime.as_ref().notify);
    let (_dir, state) = test_state_with_runtime(runtime);

    // Subscribe before executing to catch every event the handler emits.
    let mut rx = state.event_bus.subscribe();

    let plan_id = "no-started-event";
    execute_plan(
        State(Arc::clone(&state)),
        Path(plan_id.into()),
        axum::body::Bytes::new(),
    )
    .await
    .expect("execute plan");

    // Wait until the spawned task has called into the runtime.
    tokio::time::timeout(std::time::Duration::from_secs(1), notify.notified())
        .await
        .expect("runtime should be called");

    // Give the task a moment to publish PlanCompleted.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Drain all buffered events and assert no PlanStarted was emitted by
    // the handler.  A duplicate from the handler would appear here with
    // tasks_total: 0 because it has no task-count information.
    let mut plan_started_seen = false;
    while let Ok(envelope) = rx.try_recv() {
        if let ServerEvent::PlanStarted { plan_id: ref pid } = envelope.payload {
            if pid == plan_id {
                plan_started_seen = true;
            }
        }
    }
    assert!(
        !plan_started_seen,
        "execute_plan must not publish PlanStarted; the run publishes its own"
    );
}

/// Runs plans the way the Graph engine reports them: the plan's lifecycle
/// goes straight into the server's hub. With `fail_before_start` the run
/// fails before announcing anything, as one with an invalid config does.
struct HubLifecycleRuntime {
    hub: roko_runtime::SharedStateHub,
    fail_before_start: bool,
}

#[async_trait::async_trait]
impl CliRuntime for HubLifecycleRuntime {
    async fn run_once(
        &self,
        _workdir: &std::path::Path,
        _prompt: &str,
    ) -> anyhow::Result<RunResult> {
        anyhow::bail!("HubLifecycleRuntime only runs plans")
    }

    async fn load_plan_summary(
        &self,
        _workdir: &std::path::Path,
        plan_id: &str,
    ) -> anyhow::Result<Option<crate::plan_types::PlanSummaryDto>> {
        Ok(Some(crate::plan_types::PlanSummaryDto {
            id: plan_id.to_string(),
            title: "Hub plan".to_string(),
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
        }))
    }

    async fn run_plan_with_options(
        &self,
        _workdir: &std::path::Path,
        plan_target: &std::path::Path,
        _options: PlanRunOptions,
    ) -> anyhow::Result<PlanExecutionResult> {
        if self.fail_before_start {
            anyhow::bail!("load Graph runtime config: invalid model");
        }
        let plan_id = plan_target
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        let hub = self.hub.sender();
        hub.publish(roko_core::DashboardEvent::PlanStarted {
            plan_id: plan_id.clone(),
            tasks_total: 1,
        });
        hub.publish(roko_core::DashboardEvent::PlanCompleted {
            plan_id,
            success: true,
        });
        hub.publish(roko_core::DashboardEvent::RunCompleted {
            outcome: "succeeded".to_string(),
            duration_ms: 5,
            cleanup_degraded: false,
            surviving_agent_ids: Vec::new(),
            surviving_agent_pids: Vec::new(),
        });
        Ok(PlanExecutionResult {
            success: true,
            output_text: None,
            gate_results: Vec::new(),
        })
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

    fn dashboard_scaffold(&self, _workdir: &std::path::Path) -> DashboardInfo {
        DashboardInfo {
            rendered: String::new(),
        }
    }
}

/// Execute plan `hello` on a [`HubLifecycleRuntime`] and return the
/// `success` of every PlanCompleted published for it: by the run into
/// the hub, or by the route onto the event bus.
async fn plan_completions_of_single_plan_run(fail_before_start: bool) -> Vec<bool> {
    let hub = roko_runtime::SharedStateHub::new_in_process();
    let runtime = Arc::new(HubLifecycleRuntime {
        hub: hub.clone(),
        fail_before_start,
    });
    let dir = tempdir().expect("tempdir");
    let deploy_backend =
        Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
    let state = Arc::new(
        AppState::new_with_state_hub(
            dir.path().to_path_buf(),
            runtime,
            roko_core::config::schema::RokoConfig::default(),
            deploy_backend,
            hub.clone(),
        )
        .expect("AppState::new_with_state_hub"),
    );
    let mut bus = state.event_bus.subscribe();

    execute_plan(
        State(Arc::clone(&state)),
        Path("hello".into()),
        axum::body::Bytes::new(),
    )
    .await
    .expect("execute plan");
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let finished = state
                .active_plans
                .read()
                .await
                .get("hello")
                .is_none_or(|run| run.handle.is_finished());
            if finished {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the run task should finish");

    let mut completions = hub
        .replay_from(0)
        .into_iter()
        .filter_map(|envelope| match envelope.payload {
            roko_core::DashboardEvent::PlanCompleted { plan_id, success } if plan_id == "hello" => {
                Some(success)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    while let Ok(envelope) = bus.try_recv() {
        if let ServerEvent::PlanCompleted { plan_id, success } = envelope.payload
            && plan_id == "hello"
        {
            completions.push(success);
        }
    }
    completions
}

/// bug-08d912: the Graph run publishes the plan's PlanCompleted, so the
/// route must not publish a second one after the run returns.
#[tokio::test]
async fn single_plan_run_publishes_plan_completed_once() {
    assert_eq!(plan_completions_of_single_plan_run(false).await, vec![true]);
}

/// A run that fails before its plan starts publishes no PlanCompleted, so
/// the route settles the plan, once and as failed.
#[tokio::test]
async fn run_failing_before_the_plan_starts_completes_it_once() {
    assert_eq!(plan_completions_of_single_plan_run(true).await, vec![false]);
}

/// A plan runtime whose runs end as a test scripts them: a run records the
/// id it runs under, waits for a permit from `gate` when there is one,
/// publishes `task_outcome`, when set, as the outcome of task `T1` of its
/// plan into the server's hub, then returns `success`.
struct ScriptedPlanRuntime {
    hub: roko_runtime::SharedStateHub,
    success: bool,
    task_outcome: Option<&'static str>,
    gate: Option<Arc<tokio::sync::Semaphore>>,
    run_ids: Arc<Mutex<Vec<String>>>,
}

#[async_trait::async_trait]
impl CliRuntime for ScriptedPlanRuntime {
    async fn run_once(
        &self,
        _workdir: &std::path::Path,
        _prompt: &str,
    ) -> anyhow::Result<RunResult> {
        anyhow::bail!("ScriptedPlanRuntime only runs plans")
    }

    async fn load_plan_summary(
        &self,
        _workdir: &std::path::Path,
        plan_id: &str,
    ) -> anyhow::Result<Option<crate::plan_types::PlanSummaryDto>> {
        Ok(Some(crate::plan_types::PlanSummaryDto {
            id: plan_id.to_string(),
            title: "Scripted plan".to_string(),
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
        }))
    }

    async fn run_plan_with_options(
        &self,
        _workdir: &std::path::Path,
        plan_target: &std::path::Path,
        options: PlanRunOptions,
    ) -> anyhow::Result<PlanExecutionResult> {
        self.run_ids
            .lock()
            .expect("lock run ids")
            .push(options.run_id.unwrap_or_default());
        if let Some(gate) = &self.gate {
            gate.acquire().await.expect("the gate stays open").forget();
        }
        if let Some(outcome) = self.task_outcome {
            let plan_id = plan_target
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_string();
            let hub = self.hub.sender();
            hub.publish(roko_core::DashboardEvent::TaskCompleted {
                plan_id,
                task_id: "T1".to_string(),
                outcome: outcome.to_string(),
            });
        }
        Ok(PlanExecutionResult {
            success: self.success,
            output_text: None,
            gate_results: Vec::new(),
        })
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

    fn dashboard_scaffold(&self, _workdir: &std::path::Path) -> DashboardInfo {
        DashboardInfo {
            rendered: String::new(),
        }
    }
}

/// A [`ScriptedPlanRuntime`] whose runs return `success`, after publishing
/// `task_outcome`, and wait for a permit from `gate` when there is one.
fn scripted_runtime(
    success: bool,
    task_outcome: Option<&'static str>,
    gate: Option<Arc<tokio::sync::Semaphore>>,
) -> Arc<ScriptedPlanRuntime> {
    Arc::new(ScriptedPlanRuntime {
        hub: roko_runtime::SharedStateHub::new_in_process(),
        success,
        task_outcome,
        gate,
        run_ids: Arc::default(),
    })
}

/// Server state over `runtime`, sharing its hub.
fn scripted_state(runtime: &Arc<ScriptedPlanRuntime>) -> (tempfile::TempDir, Arc<AppState>) {
    let dir = tempdir().expect("tempdir");
    let deploy_backend =
        Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
    let state = Arc::new(
        AppState::new_with_state_hub(
            dir.path().to_path_buf(),
            runtime.clone(),
            roko_core::config::schema::RokoConfig::default(),
            deploy_backend,
            runtime.hub.clone(),
        )
        .expect("AppState::new_with_state_hub"),
    );
    (dir, state)
}

/// What `GET /api/plans/{id}/status` reports once the run `id` names has
/// ended, waiting at most five seconds for it to end.
async fn ended_plan_status(state: &Arc<AppState>, id: &str) -> Value {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let Json(status) = plan_status(State(Arc::clone(state)), Path(id.to_string()))
                .await
                .expect("the run's status");
            if status["finished"] == true {
                break status;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the run ends")
}

/// Wait, at most five seconds, until `done` holds.
async fn wait_until(mut done: impl FnMut() -> bool) {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !done() {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the condition holds in time");
}

/// Run plan `hello` on a [`ScriptedPlanRuntime`], wait for the run to end,
/// and return what `GET /api/plans/hello/status` then reports, after checking
/// that it names the run the start returned.
async fn status_after_plan_run(success: bool, task_outcome: Option<&'static str>) -> Value {
    let runtime = scripted_runtime(success, task_outcome, None);
    let (_dir, state) = scripted_state(&runtime);

    let started = start_plan_run(&state, "hello".into(), false)
        .await
        .expect("start the run");
    let status = ended_plan_status(&state, "hello").await;
    assert_eq!(status["run_id"], started.run_id.as_str(), "{status}");
    status
}

/// G43: a plan run's handle keeps how the run ended. After a run that fails,
/// one that succeeds and one whose task ended unverified, `GET
/// /api/plans/{id}/status` still answers instead of 404ing, with `finished:
/// true`, when the run ended, and `failed` (with its error), `succeeded` or
/// `unverified`.
#[tokio::test]
async fn plan_status_reports_terminal_state() {
    use roko_core::dashboard_snapshot::TASK_OUTCOME_UNVERIFIED;

    for (success, task_outcome, expected) in [
        (false, None, "failed"),
        (true, None, "succeeded"),
        (false, Some(TASK_OUTCOME_UNVERIFIED), "unverified"),
    ] {
        let status = status_after_plan_run(success, task_outcome).await;
        assert_eq!(status["status"], expected, "{status}");
        assert_eq!(status["finished"], true, "{status}");
        assert!(status["finished_at"].is_string(), "{status}");
        assert_eq!(
            status["error"].is_string(),
            expected == "failed",
            "{status}"
        );
    }
}

/// Decision 9105: while a plan run is live, another is queued instead of
/// refused with 409. It answers with `queued` and its place, its status says
/// `queued`, and it starts under the id it was given once the live run ends.
/// Cancelling a queued run takes it out of the queue: it never runs and ends
/// `cancelled`.
#[tokio::test]
async fn second_plan_run_is_queued_not_refused() {
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let runtime = scripted_runtime(true, None, Some(Arc::clone(&gate)));
    let run_ids = Arc::clone(&runtime.run_ids);
    let (_dir, state) = scripted_state(&runtime);
    let ran = || run_ids.lock().expect("lock run ids").clone();

    let first = start_plan_run(&state, "first".into(), false)
        .await
        .expect("start the first run");
    assert_eq!(first.queued, None);
    wait_until(|| ran().len() == 1).await;

    let second = start_plan_run(&state, "second".into(), false)
        .await
        .expect("queue the second run");
    assert_eq!(second.queued, Some(1));
    let third = start_plan_run(&state, "third".into(), false)
        .await
        .expect("queue the third run");
    assert_eq!(third.queued, Some(2));
    let Json(status) = plan_status(State(Arc::clone(&state)), Path(second.run_id.clone()))
        .await
        .expect("the queued run's status");
    assert_eq!(status["status"], "queued", "{status}");
    assert_eq!(status["position"], 1, "{status}");
    assert_eq!(status["finished"], false, "{status}");

    let Json(cancelled) = cancel_plan(State(Arc::clone(&state)), Path(third.run_id.clone()))
        .await
        .expect("cancel the queued run");
    assert_eq!(cancelled["cancelled"], true, "{cancelled}");
    let status = ended_plan_status(&state, &third.run_id).await;
    assert_eq!(status["status"], "cancelled", "{status}");

    // The first run ends, and the second starts under the id it was given.
    gate.add_permits(1);
    wait_until(|| ran().len() == 2).await;
    assert_eq!(ran(), [first.run_id.clone(), second.run_id.clone()]);
    let Json(status) = plan_status(State(Arc::clone(&state)), Path(second.run_id.clone()))
        .await
        .expect("the second run's status");
    assert_eq!(status["status"], "running", "{status}");

    // The second ends too, and the cancelled third never runs.
    gate.add_permits(1);
    let status = ended_plan_status(&state, &second.run_id).await;
    assert_eq!(status["status"], "succeeded", "{status}");
    assert_eq!(ran(), [first.run_id, second.run_id]);
}

/// Decision 9105: the queue holds [`PLAN_RUN_QUEUE_CAPACITY`] runs behind
/// the live one, and refuses one more with 409.
#[tokio::test]
async fn plan_run_queue_refuses_runs_past_its_capacity() {
    use crate::state::PLAN_RUN_QUEUE_CAPACITY;

    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let runtime = scripted_runtime(true, None, Some(gate));
    let (_dir, state) = scripted_state(&runtime);

    let live = start_plan_run(&state, "live".into(), false)
        .await
        .expect("start the live run");
    assert_eq!(live.queued, None);
    for place in 1..=PLAN_RUN_QUEUE_CAPACITY {
        let queued = start_plan_run(&state, format!("queued-{place}"), false)
            .await
            .expect("queue a run");
        assert_eq!(queued.queued, Some(place));
    }
    let err = match start_plan_run(&state, "one-too-many".into(), false).await {
        Ok(_) => panic!("a full queue must refuse the run"),
        Err(err) => err,
    };
    assert_eq!(err.status, axum::http::StatusCode::CONFLICT);
}

#[tokio::test]
async fn list_plans_returns_internal_error_for_corrupt_plan_file() {
    let (dir, state) = test_state();
    let plans_dir = state.workdir.join(".roko").join("plans");
    tokio::fs::create_dir_all(&plans_dir)
        .await
        .expect("create plans dir");
    tokio::fs::write(plans_dir.join("broken.json"), "{not-json}")
        .await
        .expect("write corrupt plan");

    let err = list_plans(State(state))
        .await
        .expect_err("corrupt plan should fail");

    assert_eq!(err.status, axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    drop(dir);
}

/// `resume_plan` must:
///   - call `run_plan_with_options` (not `run_once`),
///   - pass `plans/<group>/<id>` as the plan directory when the summary
///     carries a `group`,
///   - set `force_resume: true` in the options.
#[tokio::test]
async fn resume_plan_runs_the_plan_directory() {
    let runtime = Arc::new(RecordingRuntime {
        calls: Arc::new(Mutex::new(Vec::new())),
        notify: Arc::new(Notify::new()),
        success: true,
        call_count: Arc::new(AtomicUsize::new(0)),
        // The summary returned by load_plan_summary will carry this group.
        group: Some("portal-programme".to_string()),
        last_options: Arc::new(Mutex::new(None)),
        known_plan_id: None,
        plan_tasks: vec![],
        summary_estimated_minutes: None,
    });
    let notify = Arc::clone(&runtime.as_ref().notify);
    let calls = Arc::clone(&runtime.as_ref().calls);
    let last_options = Arc::clone(&runtime.as_ref().last_options);
    let (_dir, state) = test_state_with_runtime(runtime);

    let response = resume_plan(State(Arc::clone(&state)), Path("my-plan".into()))
        .await
        .expect("resume plan");

    assert_eq!(
        response.into_response().status(),
        axum::http::StatusCode::ACCEPTED,
        "resume_plan must return 202"
    );

    // Wait until the spawned task calls into the runtime.
    tokio::time::timeout(std::time::Duration::from_secs(1), notify.notified())
        .await
        .expect("runtime should be called within 1 second");

    // Verify exactly one plan run was recorded (not a run_once call).
    let calls = calls.lock().expect("lock calls");
    assert_eq!(
        calls.len(),
        1,
        "exactly one call — resume must not fall through to run_once"
    );
    assert_eq!(
        calls[0].kind, "plan",
        "resume_plan must call run_plan_with_options, not run_once"
    );

    // The plan directory must be plans/<group>/<id>, not plans/<id>.
    // The workdir holds no plans on disk, so plans_dir returns plans/.
    let expected_dir = state
        .workdir
        .join("plans")
        .join("portal-programme")
        .join("my-plan");
    assert_eq!(
        calls[0].arg,
        expected_dir.to_string_lossy(),
        "plan_target must be plans/<group>/<id>"
    );

    // force_resume must be set; fresh must be unset.
    let opts = last_options
        .lock()
        .expect("lock last_options")
        .clone()
        .expect("options must have been recorded");
    assert!(opts.force_resume, "resume must set force_resume: true");
    assert!(!opts.fresh, "resume must not set fresh: true");
}

/// 1208 (decision 1206: pause holds): `POST /api/plans/{id}/pause` sends
/// the run's plan-set driver a pause through its control file and leaves the
/// run going, uncancelled; `POST /api/plans/{id}/resume` sends the held run a
/// resume instead of starting the plan again.
#[tokio::test]
async fn rest_pause_holds_the_run_without_cancelling() {
    use crate::state::PlanHandle;
    use roko_runtime::cancel::CancelToken;

    let runtime = recording_runtime_for_plan("held-plan");
    let calls = Arc::clone(&runtime.calls);
    let (_dir, state) = test_state_with_runtime(runtime);
    let cancel = CancelToken::new();
    let handle = tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    });
    let plan_handle = PlanHandle {
        id: "run-1".to_string(),
        plan_dir: state.workdir.join("plans").join("held-plan"),
        members: vec!["held-plan".to_string()],
        status: crate::state::PlanRunStatus::running(),
        handle,
        cancel: cancel.clone(),
    };
    state
        .active_plans
        .write()
        .await
        .insert("held-plan".to_string(), plan_handle);

    // The run's plan-set driver takes each command written to its control
    // file, as `forward_control_file` does.
    let control = roko_fs::RokoLayout::for_project(&state.workdir)
        .state_dir()
        .join("control.json");
    let (taken_tx, mut taken) = tokio::sync::mpsc::unbounded_channel::<String>();
    let driver = tokio::spawn(async move {
        loop {
            if let Ok(text) = tokio::fs::read_to_string(&control).await {
                let _ = tokio::fs::remove_file(&control).await;
                let command: Value = serde_json::from_str(&text).expect("a control command");
                let action = command["command"].as_str().unwrap_or_default();
                if taken_tx.send(action.to_string()).is_err() {
                    break;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    });

    let paused = pause_plan(State(Arc::clone(&state)), Path("held-plan".into()))
        .await
        .expect("pause the run");
    assert_eq!(paused.0["paused"], true);
    assert_eq!(paused.0["run_id"], "run-1");
    assert_eq!(taken.recv().await.as_deref(), Some("pause"));
    assert!(!cancel.is_cancelled(), "pause cancelled the run");
    {
        let active = state.active_plans.read().await;
        let run = active.get("held-plan").expect("the run is still active");
        assert!(!run.handle.is_finished(), "pause stopped the run");
    }

    let resumed = resume_plan(State(Arc::clone(&state)), Path("held-plan".into()))
        .await
        .expect("resume the run");
    assert_eq!(resumed.status(), axum::http::StatusCode::OK);
    assert_eq!(taken.recv().await.as_deref(), Some("resume"));
    assert!(!cancel.is_cancelled(), "resume cancelled the run");
    assert!(
        calls.lock().expect("lock calls").is_empty(),
        "resuming a held run must not start the plan again"
    );
    driver.abort();
}

/// `GET /api/plans/{id}/costs` and `GET /api/plans/{id}/gates` return 200
/// for a plan the runtime knows about, even when no plan file exists on
/// disk (directory-layout plan).  An unknown id must answer 404.
#[tokio::test]
async fn per_plan_routes_find_directory_plans() {
    use roko_core::config::ServeAuthConfig;

    let runtime = Arc::new(RecordingRuntime {
        calls: Arc::new(Mutex::new(Vec::new())),
        notify: Arc::new(Notify::new()),
        success: true,
        call_count: Arc::new(AtomicUsize::new(0)),
        group: None,
        last_options: Arc::new(Mutex::new(None)),
        // Only "dir-plan" is known; "unknown-id" must 404.
        known_plan_id: Some("dir-plan".to_string()),
        plan_tasks: vec![],
        summary_estimated_minutes: None,
    });
    let (_dir, state) = test_state_with_runtime(runtime);

    // No plan files exist on disk — only the runtime knows this plan.

    let app = build_router(
        Arc::clone(&state),
        &[],
        ServeAuthConfig {
            enabled: false,
            ..ServeAuthConfig::default()
        },
    );

    // costs — known plan → 200
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/plans/dir-plan/costs")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(
        resp.status(),
        axum::http::StatusCode::OK,
        "costs for known directory plan must return 200"
    );

    // gates — known plan → 200
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/plans/dir-plan/gates")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(
        resp.status(),
        axum::http::StatusCode::OK,
        "gates for known directory plan must return 200"
    );

    // costs — unknown id → 404
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/plans/unknown-id/costs")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(
        resp.status(),
        axum::http::StatusCode::NOT_FOUND,
        "costs for unknown plan must return 404"
    );
}

// ── execute_plans ────────────────────────────────────────────────────

/// `POST /api/plans/execute` with a `plans` list must return 202 with
/// `order` containing the requested ids and `max_parallel_plans` equal to
/// the conductor default (since no body override was given).
#[tokio::test]
async fn execute_plans_returns_202_with_order_and_parallelism() {
    let runtime = Arc::new(RecordingRuntime {
        calls: Arc::new(Mutex::new(Vec::new())),
        notify: Arc::new(Notify::new()),
        success: true,
        call_count: Arc::new(AtomicUsize::new(0)),
        group: None,
        last_options: Arc::new(Mutex::new(None)),
        known_plan_id: None,
        plan_tasks: vec![],
        summary_estimated_minutes: None,
    });
    let notify = Arc::clone(&runtime.as_ref().notify);
    let (_dir, state) = test_state_with_runtime(runtime);

    let response = match execute_plans(
        State(Arc::clone(&state)),
        axum::body::Bytes::from(r#"{"plans":["plan-a","plan-b"],"max_parallel_plans":2}"#),
    )
    .await
    {
        Ok(r) => r.into_response(),
        Err(e) => panic!("execute_plans should succeed, got error: {e:?}"),
    };
    assert_eq!(
        response.status(),
        axum::http::StatusCode::ACCEPTED,
        "execute_plans must return 202"
    );
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let payload: Value = serde_json::from_slice(&body).expect("parse body");
    assert!(payload["id"].as_str().is_some(), "response must have an id");
    assert_eq!(
        payload["order"],
        json!(["plan-a", "plan-b"]),
        "order must match the requested plan ids"
    );
    assert_eq!(
        payload["max_parallel_plans"], 2,
        "max_parallel_plans must reflect the body value"
    );

    // Wait for the spawned task to notify the runtime.
    tokio::time::timeout(std::time::Duration::from_secs(1), notify.notified())
        .await
        .expect("runtime should be called");
}

/// Providing both `plans` and `target` must be rejected with 400.
#[tokio::test]
async fn execute_plans_rejects_plans_and_target_together() {
    let (_dir, state) = test_state();

    let err = match execute_plans(
        State(state),
        axum::body::Bytes::from(r#"{"plans":["plan-a"],"target":"subdir"}"#),
    )
    .await
    {
        Ok(_) => panic!("mutually exclusive fields must error"),
        Err(e) => e,
    };

    assert_eq!(
        err.status,
        axum::http::StatusCode::BAD_REQUEST,
        "both plans+target must return 400"
    );
}

/// `max_parallel_plans: 0` must be rejected with 422.
#[tokio::test]
async fn execute_plans_rejects_zero_max_parallel_plans() {
    let (_dir, state) = test_state();

    let err = match execute_plans(
        State(state),
        axum::body::Bytes::from(r#"{"max_parallel_plans":0}"#),
    )
    .await
    {
        Ok(_) => panic!("max_parallel_plans 0 must error"),
        Err(e) => e,
    };

    assert_eq!(
        err.status,
        axum::http::StatusCode::UNPROCESSABLE_ENTITY,
        "max_parallel_plans:0 must return 422"
    );
}

/// An absolute `target` path must be rejected with 400.
#[tokio::test]
async fn execute_plans_rejects_absolute_target() {
    let (_dir, state) = test_state();

    let err = match execute_plans(
        State(state),
        axum::body::Bytes::from(r#"{"target":"/etc/passwd"}"#),
    )
    .await
    {
        Ok(_) => panic!("absolute target must error"),
        Err(e) => e,
    };

    assert_eq!(
        err.status,
        axum::http::StatusCode::BAD_REQUEST,
        "absolute target must return 400"
    );
}

/// An empty body ("Run all") must return 202 and call the runtime with the
/// plans root as the target.
#[tokio::test]
async fn execute_plans_empty_body_runs_all() {
    let runtime = Arc::new(RecordingRuntime {
        calls: Arc::new(Mutex::new(Vec::new())),
        notify: Arc::new(Notify::new()),
        success: true,
        call_count: Arc::new(AtomicUsize::new(0)),
        group: None,
        last_options: Arc::new(Mutex::new(None)),
        known_plan_id: None,
        plan_tasks: vec![],
        summary_estimated_minutes: None,
    });
    let notify = Arc::clone(&runtime.as_ref().notify);
    let (_dir, state) = test_state_with_runtime(runtime);

    let response = match execute_plans(State(Arc::clone(&state)), axum::body::Bytes::new()).await {
        Ok(r) => r.into_response(),
        Err(e) => panic!("execute_plans empty body should succeed, got error: {e:?}"),
    };

    assert_eq!(response.status(), axum::http::StatusCode::ACCEPTED);

    tokio::time::timeout(std::time::Duration::from_secs(1), notify.notified())
        .await
        .expect("runtime should be called for run-all");
}

/// A second call while a run is active must return 409.
#[tokio::test]
async fn execute_plans_queues_behind_active_run() {
    let runtime = Arc::new(RecordingRuntime {
        calls: Arc::new(Mutex::new(Vec::new())),
        notify: Arc::new(Notify::new()),
        success: true,
        call_count: Arc::new(AtomicUsize::new(0)),
        group: None,
        last_options: Arc::new(Mutex::new(None)),
        known_plan_id: None,
        plan_tasks: vec![],
        summary_estimated_minutes: None,
    });
    let (_dir, state) = test_state_with_runtime(runtime);

    // First call — should succeed.
    execute_plans(State(Arc::clone(&state)), axum::body::Bytes::new())
        .await
        .expect("first execute_plans should succeed");

    // Second call while the first run is registered — queued behind it
    // (decision 9105), not refused with 409.
    let response = execute_plans(State(Arc::clone(&state)), axum::body::Bytes::new())
        .await
        .expect("second execute_plans should be queued")
        .into_response();
    assert_eq!(response.status(), axum::http::StatusCode::ACCEPTED);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body");
    let payload: Value = serde_json::from_slice(&body).expect("json body");
    assert_eq!(payload["queued"], true, "{payload}");
    assert_eq!(payload["position"], 1, "{payload}");
}

#[tokio::test]
async fn get_plan_source_rejects_path_traversal() {
    let runtime = recording_runtime_for_plan("x");
    let (_dir, state) = test_state_with_runtime(runtime);

    let err = get_plan_source(State(state), Path("../etc/passwd".into()))
        .await
        .expect_err("path traversal should be rejected");

    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_plan_source_returns_200_with_toml() {
    let runtime = recording_runtime_for_plan("my-plan");
    let (_dir, state) = test_state_with_runtime(runtime);

    let Json(payload) = get_plan_source(State(state), Path("my-plan".into()))
        .await
        .expect("get plan source should succeed");

    assert_eq!(payload["id"], "my-plan");
    assert!(
        payload["toml"].as_str().is_some(),
        "toml field must be present"
    );
    assert!(
        payload["path"].as_str().is_some(),
        "path field must be present"
    );
}

#[tokio::test]
async fn get_plan_source_returns_404_for_missing_plan() {
    let runtime = recording_runtime_for_plan("known-plan");
    let (_dir, state) = test_state_with_runtime(runtime);

    let err = get_plan_source(State(state), Path("unknown-plan".into()))
        .await
        .expect_err("missing plan should return 404");

    assert_eq!(err.status, axum::http::StatusCode::NOT_FOUND);
}

/// `GET /api/plans` and `GET /api/plans/{id}` must carry every field in
/// `PlanSummaryDto` without dropping `group`, `tasks_done`, `old_format`,
/// `superseded_by`, `last_error`, or `estimated_minutes`.
#[tokio::test]
async fn plan_list_and_detail_carry_the_summary_fields() {
    use roko_core::config::ServeAuthConfig;

    // A minimal runtime that returns a single richly-populated summary.
    #[derive(Clone)]
    struct RichSummaryRuntime;

    fn rich_summary() -> crate::plan_types::PlanSummaryDto {
        crate::plan_types::PlanSummaryDto {
            id: "p1".to_string(),
            title: "Rich Plan".to_string(),
            task_count: 3,
            tasks_done: 2,
            tasks_failed: 1,
            completed: false,
            status: "in-progress".to_string(),
            superseded_by: Some("p2".to_string()),
            old_format: true,
            last_error: Some("some error".to_string()),
            group: Some("test-group".to_string()),
            estimated_minutes: Some(45),
        }
    }

    #[async_trait::async_trait]
    impl CliRuntime for RichSummaryRuntime {
        async fn run_once(
            &self,
            _workdir: &std::path::Path,
            _prompt: &str,
        ) -> anyhow::Result<crate::runtime::RunResult> {
            Ok(crate::runtime::RunResult {
                success: true,
                output_text: None,
                usage: None,
                gate_results: Vec::new(),
            })
        }

        async fn list_plans(
            &self,
            _workdir: &std::path::Path,
        ) -> anyhow::Result<Vec<crate::plan_types::PlanSummaryDto>> {
            Ok(vec![rich_summary()])
        }

        async fn load_plan_summary(
            &self,
            _workdir: &std::path::Path,
            plan_id: &str,
        ) -> anyhow::Result<Option<crate::plan_types::PlanSummaryDto>> {
            if plan_id == "p1" {
                Ok(Some(rich_summary()))
            } else {
                Ok(None)
            }
        }

        async fn load_plan_tasks(
            &self,
            _workdir: &std::path::Path,
            plan_id: &str,
        ) -> anyhow::Result<Option<crate::plan_types::PlanTasksDto>> {
            if plan_id == "p1" {
                Ok(Some(crate::plan_types::PlanTasksDto {
                    plan_id: "p1".to_string(),
                    task_count: 0,
                    tasks: vec![],
                    title: None,
                    max_parallel: 1,
                }))
            } else {
                Ok(None)
            }
        }

        fn session_status(&self, workdir: PathBuf) -> crate::runtime::SessionStatusInfo {
            crate::runtime::SessionStatusInfo {
                session_id: None,
                workdir,
                daemon_running: false,
                signal_count: None,
                episode_count: None,
                last_episode_passed: None,
            }
        }

        fn dashboard_scaffold(&self, _workdir: &std::path::Path) -> crate::runtime::DashboardInfo {
            crate::runtime::DashboardInfo {
                rendered: String::new(),
            }
        }
    }

    let (_dir, state) = test_state_with_runtime(Arc::new(RichSummaryRuntime));
    let app = build_router(
        Arc::clone(&state),
        &[],
        ServeAuthConfig {
            enabled: false,
            ..ServeAuthConfig::default()
        },
    );

    // ── GET /api/plans ──────────────────────────────────────────────
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/plans")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX).await.expect("body");
    let list: Value = serde_json::from_slice(&body).expect("parse list response");
    let first = &list[0];
    assert_eq!(first["id"], "p1", "list must carry id");
    assert_eq!(first["group"], "test-group", "list must carry group");
    assert_eq!(first["tasks_done"], 2, "list must carry tasks_done");
    assert_eq!(
        first["completed_task_count"], 2,
        "list must carry completed_task_count alias"
    );
    assert_eq!(first["old_format"], true, "list must carry old_format");
    assert_eq!(
        first["superseded_by"], "p2",
        "list must carry superseded_by"
    );
    assert_eq!(
        first["last_error"], "some error",
        "list must carry last_error"
    );
    assert_eq!(
        first["estimated_minutes"], 45,
        "list must carry estimated_minutes"
    );

    // ── GET /api/plans/p1 ──────────────────────────────────────────
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/plans/p1")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX).await.expect("body");
    let detail: Value = serde_json::from_slice(&body).expect("parse detail response");
    assert_eq!(detail["id"], "p1", "detail must carry id");
    assert_eq!(detail["group"], "test-group", "detail must carry group");
    assert_eq!(detail["tasks_done"], 2, "detail must carry tasks_done");
    assert_eq!(
        detail["completed_task_count"], 2,
        "detail must carry completed_task_count alias"
    );
    assert_eq!(detail["old_format"], true, "detail must carry old_format");
    assert_eq!(
        detail["superseded_by"], "p2",
        "detail must carry superseded_by"
    );
    assert_eq!(
        detail["last_error"], "some error",
        "detail must carry last_error"
    );
    assert_eq!(
        detail["estimated_minutes"], 45,
        "detail must carry estimated_minutes"
    );
}

#[tokio::test]
async fn put_plan_source_rejects_path_traversal() {
    let runtime = recording_runtime_for_plan("x");
    let (_dir, state) = test_state_with_runtime(runtime);

    let body = axum::body::Bytes::from(r#"{"toml":"x"}"#);
    let result = put_plan_source(State(state), Path("../evil".into()), body).await;
    let err = match result {
        Err(e) => e,
        Ok(_) => panic!("path traversal should be rejected but returned Ok"),
    };

    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn put_plan_source_returns_200_on_valid_toml() {
    let runtime = recording_runtime_for_plan("my-plan");
    let (_dir, state) = test_state_with_runtime(runtime);

    let body = axum::body::Bytes::from(r#"{"toml":"[meta]\ntitle=\"My Plan\"\n"}"#);
    let resp = put_plan_source(State(Arc::clone(&state)), Path("my-plan".into()), body)
        .await
        .expect("valid toml should succeed");

    assert_eq!(resp.into_response().status(), axum::http::StatusCode::OK);
}

#[tokio::test]
async fn put_plan_source_returns_404_for_missing_plan() {
    let runtime = recording_runtime_for_plan("known-plan");
    let (_dir, state) = test_state_with_runtime(runtime);

    let body = axum::body::Bytes::from(r#"{"toml":"[meta]\ntitle=\"x\"\n"}"#);
    let result = put_plan_source(State(state), Path("unknown-plan".into()), body).await;
    let err = match result {
        Err(e) => e,
        Ok(_) => panic!("missing plan should return 404 but returned Ok"),
    };

    assert_eq!(err.status, axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn put_plan_source_returns_409_when_run_is_active() {
    use crate::state::PlanHandle;
    use roko_runtime::cancel::CancelToken;

    let runtime = recording_runtime_for_plan("active-plan");
    let (_dir, state) = test_state_with_runtime(runtime);

    // Inject a fake active run for "active-plan".
    let cancel = CancelToken::new();
    let handle = tokio::spawn(async {
        // Stay alive long enough for the test assertion.
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    });
    let plan_handle = PlanHandle {
        id: "run-1".to_string(),
        plan_dir: state.workdir.join("plans").join("active-plan"),
        members: vec!["active-plan".to_string()],
        status: crate::state::PlanRunStatus::running(),
        handle,
        cancel,
    };
    state
        .active_plans
        .write()
        .await
        .insert("active-plan".to_string(), plan_handle);

    let body = axum::body::Bytes::from(r#"{"toml":"[meta]\ntitle=\"x\"\n"}"#);
    let result = put_plan_source(State(state), Path("active-plan".into()), body).await;
    let err = match result {
        Err(e) => e,
        Ok(_) => panic!("active run should cause 409 but returned Ok"),
    };

    assert_eq!(err.status, axum::http::StatusCode::CONFLICT);
    assert_eq!(err.code, "conflict");
}

/// Run git in `dir` and return its trimmed stdout.
fn run_git(dir: &std::path::Path, args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .current_dir(dir)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// gap-0d64d5: a Graph task's diff is the result its run recorded (the
/// accepted attempt commit against its parent); an attempt held for
/// review shows the change it waits with, and a decision on it is
/// recorded for that attempt instead of merging anything.
#[tokio::test]
async fn task_diff_reads_the_graph_task_result() {
    let (dir, state) = test_state_with_runtime(recording_runtime_for_plan("plan-a"));
    let repo = dir.path();
    run_git(repo, &["init", "--quiet", "-b", "main"]);
    for (key, value) in [
        ("user.name", "Operator"),
        ("user.email", "operator@example.test"),
        ("commit.gpgsign", "false"),
    ] {
        run_git(repo, &["config", key, value]);
    }
    std::fs::write(repo.join("README.md"), "base\n").expect("write");
    run_git(repo, &["add", "-A"]);
    run_git(repo, &["commit", "--quiet", "-m", "base"]);
    std::fs::write(repo.join("README.md"), "base\nmore\n").expect("write");
    std::fs::write(repo.join("feature.txt"), "feature\n").expect("write");
    run_git(repo, &["add", "-A"]);
    run_git(repo, &["commit", "--quiet", "-m", "roko: plan-a/T1"]);
    let commit = run_git(repo, &["rev-parse", "HEAD"]);
    let graph = repo.join(".roko/state/graph/plan-a");
    std::fs::create_dir_all(&graph).expect("graph state");
    let record = json!({
        "graph_id": "plan-a",
        "run_id": "run-1",
        "node_id": "T1",
        "tick": 1,
        "signals": [{ "tags": {
            "plan_id": "plan-a",
            "task_id": "T1",
            "workspace.attempt_commit": commit,
            "workspace.plan_branch": "roko/plan/plan-a",
        }}],
    });
    std::fs::write(graph.join("activities.jsonl"), format!("{record}\n")).expect("log");

    let Json(diff) = task_diff(
        State(Arc::clone(&state)),
        Path(("plan-a".to_string(), "T1".to_string())),
    )
    .await
    .expect("the task's diff");
    assert_eq!(diff["source"], "graph", "{diff}");
    assert_eq!(diff["commit"], commit.as_str());
    assert_eq!(diff["branch"], "roko/plan/plan-a");
    let files = diff["files"].as_array().expect("files");
    let paths: Vec<&str> = files.iter().filter_map(|f| f["path"].as_str()).collect();
    assert_eq!(paths, ["README.md", "feature.txt"], "{diff}");
    assert_eq!(diff["total_additions"], 2);
    assert!(
        files[1]["patch"]
            .as_str()
            .is_some_and(|p| p.contains("+feature"))
    );

    // An attempt held for review: the diff is the one it waits with.
    let hold_path = roko_fs::RokoLayout::for_project(repo).review_hold("plan-a", "T1");
    std::fs::create_dir_all(hold_path.parent().expect("hold dir")).expect("hold dir");
    let hold = json!({
        "plan_id": "plan-a",
        "task_id": "T1",
        "attempt_key": "run-1:plan-a:T1:2",
        "branch": "roko/attempt/attempt-1",
        "base": commit,
        "numstat": "1\t0\tnext.txt\n",
        "patch": "diff --git a/next.txt b/next.txt\nnew file mode 100644\n--- /dev/null\n+++ b/next.txt\n@@ -0,0 +1 @@\n+next\n",
    });
    std::fs::write(&hold_path, hold.to_string()).expect("hold");
    let Json(held) = task_diff(
        State(Arc::clone(&state)),
        Path(("plan-a".to_string(), "T1".to_string())),
    )
    .await
    .expect("the held diff");
    assert_eq!(held["source"], "review_hold", "{held}");
    assert_eq!(held["status"], "awaiting_approval");
    assert_eq!(held["files"][0]["path"], "next.txt");
    assert_eq!(held["files"][0]["status"], "added");

    // A decision on the held attempt is recorded for it; nothing merges.
    let Json(reply) = submit_review(
        State(Arc::clone(&state)),
        Path(("plan-a".to_string(), "T1".to_string())),
        ValidJson(ReviewDecision {
            decision: "reject".to_string(),
            comment: "keep it smaller".to_string(),
        }),
    )
    .await
    .expect("review");
    assert_eq!(reply["status"], "rejected");
    assert_eq!(reply["held"], true);
    let log = std::fs::read_to_string(roko_fs::RokoLayout::for_project(repo).reviews_log())
        .expect("review log");
    let entry: Value = serde_json::from_str(log.lines().last().expect("an entry")).expect("json");
    assert_eq!(entry["attempt_key"], "run-1:plan-a:T1:2");
    assert_eq!(entry["decision"], "rejected");
    assert_eq!(entry["comment"], "keep it smaller");
    assert_eq!(run_git(repo, &["rev-parse", "HEAD"]), commit);
}

fn recording_runtime_for_plan(plan_id: &str) -> Arc<RecordingRuntime> {
    Arc::new(RecordingRuntime {
        calls: Arc::new(Mutex::new(Vec::new())),
        notify: Arc::new(Notify::new()),
        success: true,
        call_count: Arc::new(AtomicUsize::new(0)),
        group: None,
        last_options: Arc::new(Mutex::new(None)),
        known_plan_id: Some(plan_id.to_string()),
        plan_tasks: vec![],
        summary_estimated_minutes: None,
    })
}

#[tokio::test]
async fn validate_plan_rejects_path_traversal() {
    let runtime = recording_runtime_for_plan("x");
    let (_dir, state) = test_state_with_runtime(runtime);

    let err = validate_plan(
        State(state),
        Path("../etc/passwd".into()),
        axum::body::Bytes::new(),
    )
    .await
    .expect_err("path traversal should be rejected");

    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn validate_plan_returns_200_for_disk_source() {
    let runtime = recording_runtime_for_plan("my-plan");
    let (_dir, state) = test_state_with_runtime(runtime);

    let Json(payload) = validate_plan(
        State(state),
        Path("my-plan".into()),
        axum::body::Bytes::new(),
    )
    .await
    .expect("disk validation should return 200");

    assert!(payload.get("valid").is_some(), "response must have 'valid'");
    assert!(
        payload.get("errors").is_some(),
        "response must have 'errors'"
    );
    assert!(
        payload.get("warnings").is_some(),
        "response must have 'warnings'"
    );
    assert!(
        payload.get("diagnostics").is_some(),
        "response must have 'diagnostics'"
    );
}

#[tokio::test]
async fn validate_plan_returns_200_for_toml_body() {
    let runtime = recording_runtime_for_plan("my-plan");
    let (_dir, state) = test_state_with_runtime(runtime);

    let body = axum::body::Bytes::from(r#"{"toml":"[meta]\ntitle=\"Test\"\n"}"#);
    let Json(payload) = validate_plan(State(state), Path("my-plan".into()), body)
        .await
        .expect("toml body validation should return 200");

    assert!(
        payload["valid"].as_bool().is_some(),
        "valid field must be a boolean"
    );
    assert!(
        payload.get("diagnostics").is_some(),
        "diagnostics field must be present"
    );
}

#[tokio::test]
async fn validate_plan_returns_400_for_malformed_body() {
    let runtime = recording_runtime_for_plan("my-plan");
    let (_dir, state) = test_state_with_runtime(runtime);

    // Body is non-empty but not valid JSON.
    let body = axum::body::Bytes::from(&b"not-valid-json"[..]);
    let err = validate_plan(State(state), Path("my-plan".into()), body)
        .await
        .expect_err("malformed body should return 400");

    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn validate_plan_returns_404_for_missing_plan() {
    let runtime = recording_runtime_for_plan("known-plan");
    let (_dir, state) = test_state_with_runtime(runtime);

    let err = validate_plan(
        State(state),
        Path("unknown-plan".into()),
        axum::body::Bytes::new(),
    )
    .await
    .expect_err("missing plan should return 404");

    assert_eq!(err.status, axum::http::StatusCode::NOT_FOUND);
}
