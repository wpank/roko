//! Integration tests for plan authoring routes.
//!
//! Proves the HTTP contract for:
//!
//! 1. `GET /api/plans/{id}/source` — 200 with `toml`; 404 for an unknown plan.
//! 2. `PUT /api/plans/{id}/source` — valid body gives 200 `saved: true`; invalid
//!    gives 422 with `code` and `diagnostics`; 409 when a registered active run
//!    includes the plan.
//! 3. `POST /api/plans/{id}/validate` — 200 both with no body and with `{toml}`.
//!    The text reaches the stub unchanged; an invalid report is still 200.
//! 4. `POST /api/plans` — 201 with the slug derived from the title; 409 when the
//!    stub reports the slug already exists.
//! 5. `POST /api/plans/generate` — `{}` and `{"prompt":"x","slug":"y"}` give 422.
//!    `{"prompt":"a rust app that prints hello world"}` gives 202 with `plan_id`,
//!    and the PRD draft exists at `.roko/prd/drafts/<plan_id>.md`.
//! 6. `GET /api/operations/{id}` — reports running, then completed with
//!    `result.slug`; 404 for an unknown id.
//!
//! A stub `CliRuntime` is used so no real agent is dispatched.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use roko_core::config::ServeAuthConfig;
use roko_core::config::schema::RokoConfig;
use roko_serve::deploy::create_backend;
use roko_serve::plan_types::{
    CreatePlanOutcome, PlanDiagnosticDto, PlanSourceDto, PlanSummaryDto, PlanTasksDto,
    PlanValidationDto,
};
use roko_serve::routes::build_router;
use roko_serve::runtime::{
    CliRuntime, DashboardInfo, PlanExecutionResult, PlanGenerationResult, RunResult,
    SessionStatusInfo,
};
use roko_serve::state::AppState;
use tempfile::tempdir;
use tokio::sync::Mutex;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// Recorded call log
// ---------------------------------------------------------------------------

/// A single call recorded by the stub runtime.
#[derive(Debug)]
struct RecordedCall {
    /// Method name.
    pub method: &'static str,
    /// Optional extra argument (toml text, slug, etc.).
    pub arg: Option<String>,
}

// ---------------------------------------------------------------------------
// Stub runtime
// ---------------------------------------------------------------------------

/// A `CliRuntime` stub that:
/// - Records every call in `calls`.
/// - Returns pre-configured fixtures for each authoring method.
/// - Blocks `generate_plan_from_prd` until a `Notify` fires so the
///   running/completed operation lifecycle can be tested.
struct StubAuthoringRuntime {
    calls: Arc<Mutex<Vec<RecordedCall>>>,

    // plan_id → PlanSourceDto (for get source)
    sources: HashMap<String, PlanSourceDto>,

    // plan_id → PlanValidationDto (for save_plan_source)
    save_results: HashMap<String, PlanValidationDto>,

    // plan_id → PlanValidationDto (for validate_plan_source)
    validate_results: HashMap<String, PlanValidationDto>,

    // slug → CreatePlanOutcome (for create_plan)
    create_outcomes: HashMap<String, CreatePlanOutcome>,

    // Notify gating generate_plan_from_prd; None = return immediately
    generate_gate: Option<Arc<tokio::sync::Notify>>,
}

impl StubAuthoringRuntime {
    fn builder() -> StubBuilder {
        StubBuilder::default()
    }
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

#[derive(Default)]
struct StubBuilder {
    calls: Arc<Mutex<Vec<RecordedCall>>>,
    sources: HashMap<String, PlanSourceDto>,
    save_results: HashMap<String, PlanValidationDto>,
    validate_results: HashMap<String, PlanValidationDto>,
    create_outcomes: HashMap<String, CreatePlanOutcome>,
    generate_gate: Option<Arc<tokio::sync::Notify>>,
}

impl StubBuilder {
    /// Add a plan whose source is known (GET source returns a dto).
    fn with_source(mut self, plan_id: impl Into<String>, toml: impl Into<String>) -> Self {
        let id = plan_id.into();
        let toml_str = toml.into();
        self.sources.insert(
            id.clone(),
            PlanSourceDto {
                id: id.clone(),
                path: format!("plans/{id}/tasks.toml"),
                toml: toml_str,
            },
        );
        self
    }

    /// Set the validation/save result for a plan (used by both save and validate).
    fn with_save_result(mut self, plan_id: impl Into<String>, dto: PlanValidationDto) -> Self {
        self.save_results.insert(plan_id.into(), dto);
        self
    }

    fn with_validate_result(mut self, plan_id: impl Into<String>, dto: PlanValidationDto) -> Self {
        self.validate_results.insert(plan_id.into(), dto);
        self
    }

    /// Pre-register a CreatePlanOutcome for a specific slug.
    fn with_create_outcome(mut self, slug: impl Into<String>, outcome: CreatePlanOutcome) -> Self {
        self.create_outcomes.insert(slug.into(), outcome);
        self
    }

    /// Make generate_plan_from_prd block until the returned Notify is notified.
    fn with_generate_gate(mut self) -> (Self, Arc<tokio::sync::Notify>) {
        let notify = Arc::new(tokio::sync::Notify::new());
        self.generate_gate = Some(Arc::clone(&notify));
        (self, notify)
    }

    fn build(self) -> StubAuthoringRuntime {
        StubAuthoringRuntime {
            calls: self.calls,
            sources: self.sources,
            save_results: self.save_results,
            validate_results: self.validate_results,
            create_outcomes: self.create_outcomes,
            generate_gate: self.generate_gate,
        }
    }
}

// ---------------------------------------------------------------------------
// CliRuntime implementation
// ---------------------------------------------------------------------------

fn valid_dto() -> PlanValidationDto {
    PlanValidationDto {
        valid: true,
        errors: 0,
        warnings: 0,
        diagnostics: vec![],
    }
}

fn invalid_dto() -> PlanValidationDto {
    PlanValidationDto {
        valid: false,
        errors: 1,
        warnings: 0,
        diagnostics: vec![PlanDiagnosticDto {
            severity: "error".to_string(),
            rule_id: "PLAN_TEST".to_string(),
            task_id: None,
            message: "stub validation error".to_string(),
        }],
    }
}

fn stub_summary(plan_id: &str) -> PlanSummaryDto {
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

#[async_trait::async_trait]
impl CliRuntime for StubAuthoringRuntime {
    async fn run_once(&self, _workdir: &Path, _prompt: &str) -> anyhow::Result<RunResult> {
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

    // ── plan source ────────────────────────────────────────────────

    async fn plan_source(
        &self,
        _workdir: &Path,
        plan_id: &str,
    ) -> anyhow::Result<Option<PlanSourceDto>> {
        self.calls.lock().await.push(RecordedCall {
            method: "plan_source",
            arg: None,
        });
        Ok(self.sources.get(plan_id).cloned())
    }

    async fn validate_plan_source(
        &self,
        _workdir: &Path,
        plan_id: &str,
        toml: Option<String>,
    ) -> anyhow::Result<Option<PlanValidationDto>> {
        self.calls.lock().await.push(RecordedCall {
            method: "validate_plan_source",
            arg: toml,
        });
        // Return the configured result if the plan is known, else None (→ 404).
        Ok(self.validate_results.get(plan_id).cloned())
    }

    async fn save_plan_source(
        &self,
        _workdir: &Path,
        plan_id: &str,
        toml: String,
    ) -> anyhow::Result<Option<PlanValidationDto>> {
        self.calls.lock().await.push(RecordedCall {
            method: "save_plan_source",
            arg: Some(toml),
        });
        Ok(self.save_results.get(plan_id).cloned())
    }

    // ── plan creation ──────────────────────────────────────────────

    async fn create_plan(
        &self,
        _workdir: &Path,
        slug: &str,
        title: &str,
    ) -> anyhow::Result<CreatePlanOutcome> {
        self.calls.lock().await.push(RecordedCall {
            method: "create_plan",
            arg: Some(title.to_string()),
        });
        if let Some(outcome) = self.create_outcomes.get(slug) {
            return Ok(outcome.clone());
        }
        // Default: created successfully.
        Ok(CreatePlanOutcome::Created {
            slug: slug.to_string(),
            path: format!("plans/{slug}"),
        })
    }

    // ── plan discovery ─────────────────────────────────────────────

    async fn list_plans(&self, _workdir: &Path) -> anyhow::Result<Vec<PlanSummaryDto>> {
        Ok(self.sources.keys().map(|id| stub_summary(id)).collect())
    }

    async fn load_plan_summary(
        &self,
        _workdir: &Path,
        plan_id: &str,
    ) -> anyhow::Result<Option<PlanSummaryDto>> {
        // A plan is "known" if it has a source or a save_result registered.
        let known = self.sources.contains_key(plan_id)
            || self.save_results.contains_key(plan_id)
            || self.validate_results.contains_key(plan_id);
        Ok(known.then(|| stub_summary(plan_id)))
    }

    async fn load_plan_tasks(
        &self,
        _workdir: &Path,
        plan_id: &str,
    ) -> anyhow::Result<Option<PlanTasksDto>> {
        let known = self.sources.contains_key(plan_id)
            || self.save_results.contains_key(plan_id)
            || self.validate_results.contains_key(plan_id);
        Ok(known.then(|| PlanTasksDto {
            plan_id: plan_id.to_string(),
            task_count: 0,
            tasks: Vec::new(),
            title: None,
            max_parallel: 1,
        }))
    }

    // ── run (never resolves — used to hold a plan active for 409 tests) ──

    async fn run_plan(
        &self,
        _workdir: &Path,
        _plan_target: &Path,
    ) -> anyhow::Result<PlanExecutionResult> {
        std::future::pending::<()>().await;
        unreachable!()
    }

    // ── plan generation ────────────────────────────────────────────

    async fn generate_plan_from_prd(
        &self,
        workdir: &Path,
        slug: &str,
        _prd_path: &Path,
    ) -> anyhow::Result<PlanGenerationResult> {
        self.calls.lock().await.push(RecordedCall {
            method: "generate_plan_from_prd",
            arg: Some(workdir.display().to_string()),
        });

        // Optionally block until the test signals us.
        if let Some(gate) = &self.generate_gate {
            gate.notified().await;
        }

        let plans_root = workdir.join("plans");
        let plan_target = plans_root.join(slug);
        Ok(PlanGenerationResult {
            plans_root,
            plan_targets: vec![plan_target],
            artifacts: Vec::new(),
        })
    }
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Build a minimal `AppState` backed by the given stub runtime.
///
/// A `plans/<plan_id>/tasks.toml` directory structure is created for any plan
/// id present in the stub's `sources` map, so the execute-plan path resolves
/// the directory correctly.  The temporary directory is returned so it lives
/// for the duration of the test.
async fn make_state(runtime: StubAuthoringRuntime) -> (tempfile::TempDir, Arc<AppState>) {
    let dir = tempdir().expect("tempdir");
    let workdir = dir.path().to_path_buf();

    // Create plan directories for all known plans.
    for plan_id in runtime
        .sources
        .keys()
        .chain(runtime.save_results.keys())
        .chain(runtime.validate_results.keys())
    {
        let plan_dir = workdir.join("plans").join(plan_id);
        tokio::fs::create_dir_all(&plan_dir)
            .await
            .expect("create plan dir");
        tokio::fs::write(
            plan_dir.join("tasks.toml"),
            "[meta]\ntitle = \"Stub plan\"\n\n[[tasks]]\nid = \"T1\"\ndescription = \"stub\"\n",
        )
        .await
        .expect("write tasks.toml");
    }

    let runtime: Arc<dyn CliRuntime> = Arc::new(runtime);
    let deploy_backend =
        Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
    let state = Arc::new(
        AppState::new(workdir, runtime, RokoConfig::default(), deploy_backend)
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
// 1. GET source
// ---------------------------------------------------------------------------

/// 1a. GET source for a known plan returns 200 with `toml`.
#[tokio::test]
async fn get_source_known_plan_returns_200_with_toml() {
    let toml_fixture = "[meta]\ntitle = \"My Plan\"\n";
    let stub = StubAuthoringRuntime::builder()
        .with_source("my-plan", toml_fixture)
        .build();
    let (_dir, state) = make_state(stub).await;
    let app = build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/plans/my-plan/source")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK, "expected 200");
    let body = body_json(response).await;
    assert_eq!(
        body["toml"].as_str().unwrap_or(""),
        toml_fixture,
        "toml must match fixture: {body}"
    );
    assert_eq!(body["id"].as_str().unwrap_or(""), "my-plan");
}

/// 1b. GET source for an unknown plan returns 404.
#[tokio::test]
async fn get_source_unknown_plan_returns_404() {
    let stub = StubAuthoringRuntime::builder().build();
    let (_dir, state) = make_state(stub).await;
    let app = build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/plans/does-not-exist/source")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::NOT_FOUND, "expected 404");
}

// ---------------------------------------------------------------------------
// 2. PUT source
// ---------------------------------------------------------------------------

/// 2a. PUT source with a valid body returns 200 `{ "saved": true }`.
#[tokio::test]
async fn put_source_valid_body_returns_200_saved() {
    let stub = StubAuthoringRuntime::builder()
        .with_save_result("my-plan", valid_dto())
        .build();
    let (_dir, state) = make_state(stub).await;
    let app = build_app(state);

    let body = r#"{"toml":"[meta]\ntitle=\"ok\"\n"}"#;
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/plans/my-plan/source")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK, "expected 200");
    let payload = body_json(response).await;
    assert_eq!(
        payload["saved"], true,
        "response must contain saved:true: {payload}"
    );
}

/// 2b. PUT source with an invalid body returns 422 with `code` and `diagnostics`.
#[tokio::test]
async fn put_source_invalid_body_returns_422_with_code_and_diagnostics() {
    let stub = StubAuthoringRuntime::builder()
        .with_save_result("my-plan", invalid_dto())
        .build();
    let (_dir, state) = make_state(stub).await;
    let app = build_app(state);

    let body = r#"{"toml":"bad toml !!!"}"#;
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/plans/my-plan/source")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(
        response.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "expected 422"
    );
    let payload = body_json(response).await;
    assert!(
        payload.get("code").is_some(),
        "response must contain 'code': {payload}"
    );
    let diagnostics = payload.get("diagnostics").and_then(|v| v.as_array());
    assert!(
        diagnostics.map_or(false, |d| !d.is_empty()),
        "response must contain non-empty diagnostics: {payload}"
    );
}

/// 2c. PUT source while an active run includes the plan returns 409.
#[tokio::test(flavor = "multi_thread")]
async fn put_source_while_plan_active_returns_409() {
    let plan_id = "active-plan";
    let stub = StubAuthoringRuntime::builder()
        .with_save_result(plan_id, valid_dto())
        .build();
    let (_dir, state) = make_state(stub).await;

    // Start plan execution (the stub's run_plan never resolves, so the plan
    // stays active while we send the PUT).
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
        .expect("send execute");
    assert_eq!(
        exec_resp.status(),
        StatusCode::ACCEPTED,
        "execute must return 202 before PUT"
    );

    // Now try to PUT source while the plan is executing.
    let app_put = build_app(Arc::clone(&state));
    let body = r#"{"toml":"[meta]\ntitle=\"ok\"\n"}"#;
    let put_resp = app_put
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/plans/{plan_id}/source"))
                .header("content-type", "application/json")
                .body(Body::from(body))
                .expect("build request"),
        )
        .await
        .expect("send put");

    assert_eq!(
        put_resp.status(),
        StatusCode::CONFLICT,
        "PUT source while plan is active must return 409"
    );
}

// ---------------------------------------------------------------------------
// 3. POST validate
// ---------------------------------------------------------------------------

/// 3a. POST validate with no body returns 200 (validates the on-disk file).
#[tokio::test]
async fn validate_no_body_returns_200() {
    let stub = StubAuthoringRuntime::builder()
        .with_validate_result("my-plan", valid_dto())
        .build();
    let (_dir, state) = make_state(stub).await;
    let app = build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/my-plan/validate")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK, "expected 200");
    let payload = body_json(response).await;
    assert!(
        payload.get("valid").is_some(),
        "response must contain 'valid': {payload}"
    );
}

/// 3b. POST validate with `{toml}` returns 200 and the text reaches the stub unchanged.
#[tokio::test]
async fn validate_with_toml_body_reaches_stub_unchanged() {
    let toml_text = "[meta]\ntitle = \"Test Plan\"\n[[tasks]]\nid = \"T1\"\ndescription = \"x\"\n";
    let calls: Arc<Mutex<Vec<RecordedCall>>> = Arc::new(Mutex::new(Vec::new()));
    let calls_clone = Arc::clone(&calls);

    let mut stub = StubAuthoringRuntime::builder()
        .with_validate_result("my-plan", valid_dto())
        .build();
    // Inject the shared calls log into the stub so we can inspect it.
    stub.calls = calls_clone;

    let (_dir, state) = make_state(stub).await;
    let app = build_app(state);

    let body = serde_json::json!({ "toml": toml_text }).to_string();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/my-plan/validate")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK, "expected 200");

    // Verify the stub received the exact toml text.
    let recorded = calls.lock().await;
    let call = recorded
        .iter()
        .find(|c| c.method == "validate_plan_source")
        .expect("validate_plan_source must be called");
    assert_eq!(
        call.arg.as_deref(),
        Some(toml_text),
        "toml must reach stub unchanged; got: {:?}",
        call.arg
    );
}

/// 3c. POST validate returns 200 even when the plan is invalid.
#[tokio::test]
async fn validate_invalid_plan_returns_200() {
    let stub = StubAuthoringRuntime::builder()
        .with_validate_result("my-plan", invalid_dto())
        .build();
    let (_dir, state) = make_state(stub).await;
    let app = build_app(state);

    let body = serde_json::json!({ "toml": "this is not valid toml @@" }).to_string();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/my-plan/validate")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .expect("build request"),
        )
        .await
        .expect("send request");

    // validate always returns 200 — invalid source is a normal editing state.
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "validate must be 200 even for invalid plans"
    );
    let payload = body_json(response).await;
    assert_eq!(
        payload["valid"], false,
        "valid must be false for invalid plan: {payload}"
    );
}

// ---------------------------------------------------------------------------
// 4. POST /api/plans (create)
// ---------------------------------------------------------------------------

/// 4a. POST /api/plans with a new title returns 201 with the derived slug.
#[tokio::test]
async fn create_plan_new_title_returns_201_with_slug() {
    let stub = StubAuthoringRuntime::builder().build();
    let (_dir, state) = make_state(stub).await;
    let app = build_app(state);

    let body = serde_json::json!({ "title": "My New Plan" }).to_string();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::CREATED, "expected 201");
    let payload = body_json(response).await;
    let id = payload["id"].as_str().expect("response must have 'id'");
    // The slug is derived from the title: lowercase, non-alnum → '-'.
    assert!(
        id.contains("my") || id.contains("new") || id.contains("plan"),
        "slug must be derived from title; got: {id}"
    );
}

/// 4b. POST /api/plans returns 409 when the stub reports the slug already exists.
#[tokio::test]
async fn create_plan_existing_slug_returns_409() {
    let slug = "my-existing-plan";
    let stub = StubAuthoringRuntime::builder()
        .with_create_outcome(
            slug,
            CreatePlanOutcome::AlreadyExists {
                slug: slug.to_string(),
            },
        )
        .build();
    let (_dir, state) = make_state(stub).await;
    let app = build_app(state);

    // Pass the exact slug so the derived slug matches the registered outcome.
    let body = serde_json::json!({
        "title": "My Existing Plan",
        "slug": slug,
    })
    .to_string();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(
        response.status(),
        StatusCode::CONFLICT,
        "duplicate slug must return 409"
    );
}

// ---------------------------------------------------------------------------
// 5. POST /api/plans/generate
// ---------------------------------------------------------------------------

/// 5a. Empty body `{}` returns 422.
#[tokio::test]
async fn generate_plan_empty_body_returns_422() {
    let stub = StubAuthoringRuntime::builder().build();
    let (_dir, state) = make_state(stub).await;
    let app = build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/generate")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(
        response.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "empty generate body must return 422"
    );
}

/// 5b. `{"prompt":"x","slug":"y"}` (both present) returns 422.
#[tokio::test]
async fn generate_plan_both_prompt_and_slug_returns_422() {
    let stub = StubAuthoringRuntime::builder().build();
    let (_dir, state) = make_state(stub).await;
    let app = build_app(state);

    let body = serde_json::json!({ "prompt": "x", "slug": "y" }).to_string();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/generate")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(
        response.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "both prompt and slug must return 422"
    );
}

/// 5c. A valid prompt gives 202 with `plan_id`, and the PRD draft is written
///     to `.roko/prd/drafts/<plan_id>.md`.
#[tokio::test]
async fn generate_plan_valid_prompt_returns_202_and_writes_prd_draft() {
    let stub = StubAuthoringRuntime::builder().build();
    let (dir, state) = make_state(stub).await;
    let workdir = dir.path().to_path_buf();
    let app = build_app(state);

    let body = serde_json::json!({ "prompt": "a rust app that prints hello world" }).to_string();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/generate")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(
        response.status(),
        StatusCode::ACCEPTED,
        "valid generate must return 202"
    );
    let payload = body_json(response).await;
    let plan_id = payload["plan_id"]
        .as_str()
        .expect("202 response must contain plan_id");
    assert!(!plan_id.is_empty(), "plan_id must not be empty");

    // PRD draft must exist at the expected path.
    let draft_path = workdir
        .join(".roko")
        .join("prd")
        .join("drafts")
        .join(format!("{plan_id}.md"));
    assert!(
        draft_path.exists(),
        "PRD draft must exist at {}: path not found",
        draft_path.display()
    );
}

// ---------------------------------------------------------------------------
// 6. GET /api/operations/{id}
// ---------------------------------------------------------------------------

/// 6a. GET /api/operations/{id} on an unknown id returns 404.
#[tokio::test]
async fn get_operation_unknown_id_returns_404() {
    let stub = StubAuthoringRuntime::builder().build();
    let (_dir, state) = make_state(stub).await;
    let app = build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/operations/no-such-op")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "unknown op must be 404"
    );
}

/// 6b. GET /api/operations/{id} reports `running` immediately after generate is
///     started, and `completed` with `result.slug` once the stub returns.
#[tokio::test(flavor = "multi_thread")]
async fn get_operation_reports_running_then_completed_with_slug() {
    let (builder, gate) = StubAuthoringRuntime::builder().with_generate_gate();
    let stub = builder.build();

    let (_dir, state) = make_state(stub).await;

    // POST generate — starts the background task, which blocks on the gate.
    let app_gen = build_app(Arc::clone(&state));
    let gen_resp = app_gen
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/generate")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "prompt": "a rust app that prints hello world" })
                        .to_string(),
                ))
                .expect("build request"),
        )
        .await
        .expect("send generate");

    assert_eq!(
        gen_resp.status(),
        StatusCode::ACCEPTED,
        "generate must be 202"
    );
    let gen_payload = body_json(gen_resp).await;
    let op_id = gen_payload["id"].as_str().expect("202 must have op id");
    let plan_id = gen_payload["plan_id"]
        .as_str()
        .expect("202 must have plan_id");
    assert!(!op_id.is_empty());
    assert!(!plan_id.is_empty());

    // Poll the operation — it must report "running" (the gate hasn't fired yet).
    // Give the background task a moment to register but not finish.
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    let app_status1 = build_app(Arc::clone(&state));
    let status1 = app_status1
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/operations/{op_id}"))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("send status1");

    assert_eq!(status1.status(), StatusCode::OK, "operations must be 200");
    let s1_payload = body_json(status1).await;
    assert_eq!(
        s1_payload["status"].as_str().unwrap_or(""),
        "running",
        "operation must be running before gate: {s1_payload}"
    );

    // Release the gate so generate_plan_from_prd returns.
    gate.notify_one();

    // Give the background task time to update the operation handle.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Now the operation should be completed.
    let app_status2 = build_app(Arc::clone(&state));
    let status2 = app_status2
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/operations/{op_id}"))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("send status2");

    assert_eq!(status2.status(), StatusCode::OK);
    let s2_payload = body_json(status2).await;
    assert_eq!(
        s2_payload["status"].as_str().unwrap_or(""),
        "completed",
        "operation must be completed after gate: {s2_payload}"
    );

    // `result.slug` must be present.
    let slug = s2_payload
        .get("result")
        .and_then(|r| r.get("slug"))
        .and_then(|s| s.as_str());
    assert!(
        slug.is_some(),
        "completed result must contain slug; payload: {s2_payload}"
    );
}
