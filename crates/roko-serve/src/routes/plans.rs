//! Plan CRUD, execution, generation, control, and estimation endpoints.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};
use validator::Validate;

use crate::error::{ApiError, validate_path_segment};
use crate::events::ServerEvent;
use crate::extract::{RequestPayload, ValidJson, validate_with_validator};
use crate::plan_types::{CreatePlanOutcome, Plan, PlanSourceDto, PlanTask, PlanValidationDto};
use crate::runtime::{PlanExecutionResult, PlanRunOptions, RunResult};
use crate::state::{AppState, OperationHandle, OperationStatus, PlanHandle};
use roko_core::agent::resolve_model;
use roko_learn::cost_projection::{CompletedTask, CostProjector, RemainingTask};
use roko_runtime::cancel::CancelToken;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/plans", get(list_plans).post(create_plan))
        .route("/plans/{id}", get(get_plan))
        .route("/plans/{id}/tasks", get(plan_tasks))
        .route("/plans/{id}/execute", post(execute_plan))
        .route("/plans/{id}/status", get(plan_status))
        .route("/plans/{id}/pause", post(pause_plan))
        .route("/plans/{id}/resume", post(resume_plan))
        .route("/plans/{id}/cancel", post(cancel_plan))
        .route("/plans/{id}/gates", get(plan_gates))
        .route("/plans/{id}/costs", get(plan_costs))
        .route("/plans/{id}/reviews", get(list_reviews))
        .route("/plans/{id}/tasks/{task_id}/review", post(submit_review))
        .route("/plans/{id}/tasks/{task_id}/diff", get(task_diff))
        .route("/plans/{id}/chat", post(plan_chat))
        .route("/plans/{id}/estimate", post(plan_estimate))
        .route(
            "/plans/{id}/source",
            get(get_plan_source).put(put_plan_source),
        )
        .route("/plans/{id}/validate", post(validate_plan))
        .route("/plans/{id}/revise", post(revise_plan))
        .route("/plans/generate", post(generate_plan))
        .route("/plans/execute", post(execute_plans))
}

/// `GET /api/plans` — list plans by delegating to the runtime's plan discovery.
///
/// The previous implementation walked `.roko/plans/` and filtered by file
/// extension, which silently skipped every plan stored as a directory (the
/// normal layout). Delegating to `state.runtime.list_plans()` fixes that and
/// also surfaces richer status metadata the portal needs to colour plans.
async fn list_plans(State(state): State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    let plans = state
        .runtime
        .list_plans(&state.workdir)
        .await
        .map_err(|e| {
            ApiError::internal(format!("list plans in {}: {e}", state.workdir.display()))
        })?;

    let summaries: Vec<Value> = plans
        .into_iter()
        .map(|dto| {
            let mut v = serde_json::to_value(&dto).unwrap_or(Value::Null);
            // Keep `completed_task_count` as an alias of `tasks_done` for older clients.
            if let Some(tasks_done) = v.get("tasks_done").and_then(Value::as_u64) {
                v["completed_task_count"] = tasks_done.into();
            }
            v
        })
        .collect();

    Ok(Json(Value::Array(summaries)))
}

/// `GET /api/plans/:id` — load a specific plan summary.
///
/// Delegates to `state.runtime.load_plan_summary()` so that directory-layout
/// plans (the normal layout) are discovered correctly. The previous
/// implementation called `find_plan`, which only probed flat `.json`/`.toml`
/// files and returned 404 for every directory plan.
async fn get_plan(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    validate_path_segment(&id, "plan id")?;

    let dto = state
        .runtime
        .load_plan_summary(&state.workdir, &id)
        .await
        .map_err(|e| ApiError::internal(format!("load plan '{id}': {e}")))?
        .ok_or_else(|| ApiError::not_found(format!("plan '{id}' not found")))?;

    let mut v = serde_json::to_value(&dto)
        .map_err(|e| ApiError::internal(format!("serialize plan summary: {e}")))?;
    // Keep `completed_task_count` as an alias of `tasks_done` for older clients.
    if let Some(tasks_done) = v.get("tasks_done").and_then(Value::as_u64) {
        v["completed_task_count"] = tasks_done.into();
    }
    Ok(Json(v))
}

/// `GET /api/plans/:id/tasks` — return the task list for a specific plan.
///
/// Delegates to `state.runtime.load_plan_tasks()` so that directory-layout
/// plans (the normal layout) are discovered correctly. The previous
/// implementation called `find_plan`, which only probed flat `.json`/`.toml`
/// files and returned 404 for every directory plan.
///
/// Response envelope: `{ plan_id, task_count, tasks: [...] }`.
/// Each task carries `id`, `title`, `description`, `role`, `tier`,
/// `depends_on`, `files`, `completed`, `status`, and `verify_phases`.
async fn plan_tasks(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    validate_path_segment(&id, "plan id")?;

    let dto = state
        .runtime
        .load_plan_tasks(&state.workdir, &id)
        .await
        .map_err(|e| ApiError::internal(format!("load tasks for plan '{id}': {e}")))?
        .ok_or_else(|| ApiError::not_found(format!("plan '{id}' not found")))?;

    let tasks: Vec<Value> = dto
        .tasks
        .iter()
        .map(|t| serde_json::to_value(t).unwrap_or(Value::Null))
        .collect();

    Ok(Json(json!({
        "plan_id": dto.plan_id,
        "task_count": dto.task_count,
        "title": dto.title,
        "max_parallel": dto.max_parallel,
        "tasks": tasks,
    })))
}

#[derive(Deserialize, Validate)]
struct CreatePlanRequest {
    /// Human-readable plan title (required).
    #[validate(
        length(min = 1),
        custom(function = "crate::extract::validate_non_blank")
    )]
    title: String,
    /// Optional slug.  When absent the slug is derived from the title:
    /// lowercase ASCII, runs of non-alphanumeric characters collapsed to
    /// `-`, truncated to 48 characters.
    #[serde(default)]
    slug: Option<String>,
}

impl RequestPayload for CreatePlanRequest {
    fn validate_payload(&self) -> Result<(), ApiError> {
        validate_with_validator(self)
    }
}

/// `POST /api/plans` — create a new directory-layout plan.
///
/// Accepts `{ "title": "...", "slug"?: "..." }`.  Extra fields are ignored.
/// When `slug` is absent it is derived from the title (see [`slug_from_title`]).
///
/// Returns:
/// - 201 `{ "id": slug, "path": "plans/<slug>" }` on success.
/// - 409 when a plan with that slug already exists.
/// - 400 for a blank title or a path-traversal slug.
async fn create_plan(
    State(state): State<Arc<AppState>>,
    ValidJson(body): ValidJson<CreatePlanRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let slug = match body.slug {
        Some(s) => s,
        None => slug_from_title(&body.title),
    };
    validate_path_segment(&slug, "plan slug")?;

    match state
        .runtime
        .create_plan(&state.workdir, &slug, &body.title)
        .await
        .map_err(|e| ApiError::internal(format!("create plan '{slug}': {e}")))?
    {
        CreatePlanOutcome::Created { slug, path } => Ok((
            axum::http::StatusCode::CREATED,
            Json(json!({ "id": slug, "path": path })),
        )),
        CreatePlanOutcome::AlreadyExists { slug } => {
            Err(ApiError::conflict(format!("plan '{slug}' already exists")))
        }
        CreatePlanOutcome::Rejected { validation } => Err(ApiError::bad_request(format!(
            "starter source validation failed: {} error(s)",
            validation.errors
        ))),
    }
}

// ── Active-run bookkeeping helpers ────────────────────────────────────

/// Returns the map key of any unfinished run entry, or `None` when every
/// entry is already finished or the map is empty.
///
/// A finished entry does **not** constitute a conflict: `execute_plan`
/// replaces a stale finished entry rather than blocking on it.
fn active_run_conflict(active: &std::collections::HashMap<String, PlanHandle>) -> Option<String> {
    active
        .iter()
        .find(|(_, h)| !h.handle.is_finished())
        .map(|(key, _)| key.clone())
}

/// Returns the map key of the unfinished entry whose key equals `id` or
/// whose `members` list contains `id`.
///
/// Returns `None` when no live entry matches — either because `id` is
/// unknown or because every matching entry has already finished.
fn active_run_for(
    active: &std::collections::HashMap<String, PlanHandle>,
    id: &str,
) -> Option<String> {
    active
        .iter()
        .find(|(key, h)| {
            !h.handle.is_finished() && (*key == id || h.members.iter().any(|m| m == id))
        })
        .map(|(key, _)| key.clone())
}

/// Optional request body for `POST /api/plans/:id/execute`.
///
/// Both fields default to `false`, so the portal's "run" button (which sends
/// no body) always triggers a fresh run without needing to supply a payload.
#[derive(Deserialize, Default)]
struct ExecutePlanRequest {
    /// `true` — resume from the last checkpoint (`--force-resume`).
    /// `false` (default) — start fresh, discarding any checkpoint (`--fresh`).
    #[serde(default)]
    resume: bool,
}

/// Request body for `POST /api/plans/execute`.
///
/// All fields are optional.  `plans` and `target` are mutually exclusive —
/// provide at most one.  Omitting both runs every plan under the workspace
/// plans root ("Run all").
#[derive(Deserialize, Default)]
struct ExecutePlansRequest {
    /// Specific plan ids to run from the workspace plans root.
    /// Mutually exclusive with `target`.
    #[serde(default)]
    plans: Option<Vec<String>>,
    /// Workspace-relative directory to run.
    /// Mutually exclusive with `plans`.
    #[serde(default)]
    target: Option<String>,
    /// When `true`, resume from the last checkpoint; default is a fresh run.
    #[serde(default)]
    resume: bool,
    /// Maximum number of independent plans to run concurrently.
    /// Must be ≥ 1 when given; defaults to `[conductor] max_parallel_plans`.
    #[serde(default)]
    max_parallel_plans: Option<usize>,
}

/// `POST /api/plans/execute` — run a named set of plans, a target directory, or
/// every plan under the workspace plans root ("Run all").
///
/// This is a static route (like `/plans/generate`) under the `/api/plans` →
/// `plan:write` scope.
///
/// Body (all optional):
/// ```json
/// { "plans": ["id1", "id2"], "target": "rel/path", "resume": false, "max_parallel_plans": 2 }
/// ```
///
/// * `plans` and `target` are mutually exclusive (400 if both are given).
/// * `plans` – run those ids from `plans_dir` via `PlanRunOptions::only_plans`.
/// * `target` – a workspace-relative directory; absolute paths, `..` escapes,
///   and symlinks pointing outside the workspace are rejected with 400.
/// * Neither – run every plan under `plans_dir` ("Run all").
///
/// Returns `202 { "id": run_id, "order": [...], "max_parallel_plans": N }`.
async fn execute_plans(
    State(state): State<Arc<AppState>>,
    body: axum::body::Bytes,
) -> Result<impl IntoResponse, ApiError> {
    let req: ExecutePlansRequest = if body.is_empty() {
        ExecutePlansRequest::default()
    } else {
        serde_json::from_slice(&body).map_err(ApiError::parse)?
    };

    // `plans` and `target` are mutually exclusive.
    if req.plans.is_some() && req.target.is_some() {
        return Err(ApiError::bad_request(
            "plans and target are mutually exclusive; provide at most one",
        ));
    }

    // Validate max_parallel_plans ≥ 1.
    if let Some(n) = req.max_parallel_plans {
        if n < 1 {
            return Err(ApiError::unprocessable_entity(
                "max_parallel_plans must be at least 1",
            ));
        }
    }

    let pdir = plans_dir(&state.workdir);

    // Determine the execution target and optional plan-id filter.
    let (plan_target, only_plans) = if let Some(ids) = req.plans.clone() {
        // Run specific ids under the plans root.
        (pdir, Some(ids))
    } else if let Some(ref target_str) = req.target {
        // Reject absolute paths immediately.
        if std::path::Path::new(target_str.as_str()).is_absolute() {
            return Err(ApiError::bad_request(
                "target must be a workspace-relative path; absolute paths are not allowed",
            ));
        }
        let raw = state.workdir.join(target_str);
        // Canonicalize resolves symlinks and `..` so we can check containment.
        let canonical = raw.canonicalize().map_err(|e| {
            ApiError::bad_request(format!("target path is invalid or does not exist: {e}"))
        })?;
        let workdir_canonical = state
            .workdir
            .canonicalize()
            .map_err(|e| ApiError::internal(format!("canonicalize workdir: {e}")))?;
        if !canonical.starts_with(&workdir_canonical) {
            return Err(ApiError::bad_request(
                "target must be a directory inside the workspace",
            ));
        }
        if !canonical.is_dir() {
            return Err(ApiError::bad_request("target must be a directory"));
        }
        (canonical, None)
    } else {
        // Run all plans under the plans root.
        (pdir, None)
    };

    // Compute dependency order before acquiring the run lock so that a
    // 404/422 error does not consume the runner slot.
    let order = match state
        .runtime
        .plan_run_order(&state.workdir, &plan_target, only_plans.clone())
        .await
    {
        Ok(order) => order,
        Err(err) => {
            let msg = err.to_string();
            // An error whose message names an unknown plan id → 404.
            // Any other ordering failure (cycle, unsupported runtime, …) → 422.
            if msg.contains("unknown plan")
                || msg.contains("not found")
                || msg.contains("unknown id")
            {
                return Err(ApiError::not_found(msg));
            }
            return Err(ApiError::unprocessable_entity(msg));
        }
    };

    // Effective parallelism: body value, else workspace [conductor] setting.
    let config = state.load_roko_config();
    let effective_max = req
        .max_parallel_plans
        .unwrap_or(config.conductor.max_parallel_plans);

    let run_id = uuid::Uuid::new_v4().to_string();
    let bus = state.event_bus.clone();
    let runtime = state.runtime.clone();
    let workdir = state.workdir.clone();
    let resume = req.resume;
    let live_agent_output = state.effective_live_agent_output();
    let cancel = CancelToken::new();
    let task_cancel = cancel.clone();
    let plan_target_for_task = plan_target.clone();
    let run_id_for_task = run_id.clone();

    // Atomically check-and-insert with the write lock, then spawn the task.
    let order_for_response = order.clone();
    let mut active = state.active_plans.write().await;
    if let Some(conflict_key) = active_run_conflict(&active) {
        return Err(ApiError::conflict(format!(
            "a plan run is already active (run key: {conflict_key})"
        )));
    }

    let handle = tokio::spawn(async move {
        let options = PlanRunOptions {
            cancel: Some(task_cancel),
            fresh: !resume,
            force_resume: resume,
            only_plans,
            max_parallel_plans: Some(effective_max),
            live_agent_output: Some(live_agent_output),
        };
        // Do NOT publish plan lifecycle events (plan_started, plan_completed)
        // for the run_id.  The runtime publishes its own per-plan events
        // (plan_set_loaded, run_completed) with the correct metadata.
        if let Err(err) = runtime
            .run_plan_with_options(&workdir, &plan_target_for_task, options)
            .await
        {
            bus.publish(ServerEvent::Error {
                message: format!("plan set execution failed (run {run_id_for_task}): {err}"),
            });
        }
    });

    let plan_handle = PlanHandle {
        id: run_id.clone(),
        plan_dir: plan_target,
        // members carries every plan id so cancel/status by member id works.
        members: order,
        status: OperationStatus::Running,
        handle,
        cancel,
    };
    // Key by run_id (not a single plan id) since this run may span many plans.
    active.insert(run_id.clone(), plan_handle);
    drop(active);

    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(json!({
            "id": run_id,
            "order": order_for_response,
            "max_parallel_plans": effective_max,
        })),
    ))
}

/// Shared execution helper for [`execute_plan`] and [`resume_plan`].
///
/// - Validates `id` as a safe path segment.
/// - Resolves the plan with `runtime.load_plan_summary` (returns 404 when absent).
/// - Derives `plan_dir` from the summary's `group`, exactly as `execute_plan`
///   did before this refactor: `plans_dir(..)`, joined with `group` when set,
///   then with the plan `id`.
/// - Returns 409 when an unfinished run is already active.
/// - Spawns a background task that calls `run_plan_with_options` with the
///   cancel token; `force_resume` and `fresh` are set from `resume`.
///
/// Returns `run_id` on success.
async fn start_plan_run(
    state: &Arc<AppState>,
    id: String,
    resume: bool,
) -> Result<String, ApiError> {
    validate_path_segment(&id, "plan id")?;

    // Resolve the plan through the runtime so directory-layout plans are found.
    // The old `find_plan` helper only probed flat `.json`/`.toml` files and
    // returned 404 for every real plan directory.
    let dto = state
        .runtime
        .load_plan_summary(&state.workdir, &id)
        .await
        .map_err(|e| ApiError::internal(format!("load plan '{id}': {e}")))?
        .ok_or_else(|| ApiError::not_found(format!("plan '{id}' not found")))?;

    let run_id = uuid::Uuid::new_v4().to_string();
    let bus = state.event_bus.clone();
    let runtime = state.runtime.clone();
    let workdir = state.workdir.clone();
    let live_agent_output = state.effective_live_agent_output();

    // Plans inside a plan set live under their group directory.
    let plan_dir = dto
        .group
        .as_deref()
        .map_or_else(
            || plans_dir(&state.workdir),
            |group| plans_dir(&state.workdir).join(group),
        )
        .join(&id);
    let plan_id = id.clone();

    // Acquire write lock once to check-and-insert atomically (no TOCTOU race).
    let mut active = state.active_plans.write().await;
    if let Some(conflict_key) = active_run_conflict(&active) {
        return Err(ApiError::conflict(format!(
            "a plan run is already active (run key: {conflict_key})"
        )));
    }

    // Create the cancel token before spawning so the task can observe it.
    // The `pause_plan` handler calls `cancel.cancel()` on the stored copy;
    // the clone moved into the task is what the task actually checks.
    let cancel = CancelToken::new();
    let task_cancel = cancel.clone();

    let handle = tokio::spawn({
        let plan_id = plan_id.clone();
        let plan_dir = plan_dir.clone();
        async move {
            // Do NOT publish PlanStarted here. The runtime publishes its own
            // PlanStarted event (with the correct tasks_total) into the server
            // hub. A duplicate from the handler would be forwarded by the
            // bus-to-hub bridge with tasks_total: 0, confusing the portal.
            //
            // Pass the cancel token to the runtime so it can stop itself when
            // the handler calls cancel.cancel(). The run observes the token
            // internally; there is no select! race here, so PlanCompleted is
            // always published exactly once — by this task, after run returns.
            let options = PlanRunOptions {
                cancel: Some(task_cancel),
                fresh: !resume,
                force_resume: resume,
                live_agent_output: Some(live_agent_output),
                ..PlanRunOptions::default()
            };
            let success = match runtime
                .run_plan_with_options(&workdir, &plan_dir, options)
                .await
            {
                Ok(PlanExecutionResult { success, .. }) => {
                    // Emit an explicit failure event so the portal's alert
                    // band can show why a run died, not only that it ended.
                    // NOTE: the bridge maps ServerEvent::Error to
                    // DashboardEvent::Error { message } — plan_id is
                    // embedded in the message string because
                    // DashboardEvent::Error carries no structured plan_id
                    // field; the portal cannot recover it separately.
                    if !success {
                        bus.publish(ServerEvent::Error {
                            message: format!("plan {plan_id} completed with task-level failures"),
                        });
                    }
                    success
                }
                Err(err) => {
                    bus.publish(ServerEvent::Error {
                        message: format!("plan execution failed for {plan_id}: {err}"),
                    });
                    false
                }
            };
            bus.publish(ServerEvent::PlanCompleted { plan_id, success });
        }
    });

    let plan_handle = PlanHandle {
        id: run_id.clone(),
        // Store the specific plan's directory, not the parent plans directory.
        plan_dir: plan_dir.clone(),
        // Single-plan run: the only member is this plan.
        members: vec![plan_id.clone()],
        status: OperationStatus::Running,
        handle,
        cancel,
    };

    active.insert(id, plan_handle);
    drop(active);

    Ok(run_id)
}

/// `POST /api/plans/:id/execute` — spawn a background plan execution task.
///
/// Accepts an optional JSON body `{ "resume": bool }` (P-7 of the portal
/// contract). When `resume: true`, the run continues from the last checkpoint;
/// when absent or `false`, the run starts fresh.
async fn execute_plan(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    body: axum::body::Bytes,
) -> Result<impl IntoResponse, ApiError> {
    // Parse the optional request body. An empty body (the common case from
    // the portal's run button) means a fresh run; `{ "resume": true }`
    // resumes from the last checkpoint.
    let req: ExecutePlanRequest = if body.is_empty() {
        ExecutePlanRequest::default()
    } else {
        serde_json::from_slice(&body).map_err(ApiError::parse)?
    };
    let resume = req.resume;

    let run_id = start_plan_run(&state, id, resume).await?;

    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(json!({ "id": run_id, "resume": resume })),
    ))
}

/// `GET /api/plans/:id/status` — check execution status for a plan.
///
/// `{id}` may be the run key **or** any member plan id of an active run.
async fn plan_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let active = state.active_plans.read().await;
    let key = active_run_for(&active, &id)
        .ok_or_else(|| ApiError::not_found("no active execution for this plan"))?;
    let h = active
        .get(&key)
        .expect("key from active_run_for must exist in map");
    Ok(Json(json!({
        "id": h.id,
        "plan_dir": h.plan_dir,
        "status": format!("{:?}", h.status),
        "finished": h.handle.is_finished(),
    })))
}

// ── Pause / Resume ───────────────────────────────────────────────────

/// `POST /api/plans/:id/pause` — pause a running plan execution.
///
/// Cancels the background task and saves a snapshot so the plan can be
/// resumed later.  Returns 200 with `{ "paused": true }` on success, 404
/// if the plan is not actively executing, or 409 if it already finished.
///
/// `{id}` may be the run key **or** any member plan id of an active run.
async fn pause_plan(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let active = state.active_plans.write().await;
    // Resolve by key or by member plan id.
    let key = active_run_for(&active, &id)
        .ok_or_else(|| ApiError::not_found("no active execution for this plan"))?;
    let handle = active
        .get(&key)
        .expect("key from active_run_for must exist in map");

    if handle.handle.is_finished() {
        return Err(ApiError::conflict("plan execution already finished"));
    }

    // Extract data needed for the snapshot before releasing the lock.
    let task_abort = handle.handle.abort_handle();
    let captured_plan_dir = handle.plan_dir.clone();
    let captured_run_id = handle.id.clone();

    // Signal ordered cancellation so the task can unwind cleanly.
    handle.cancel.cancel();

    // Release the write lock so the spawned task can make progress during the
    // grace window.  Holding the lock while sleeping would deadlock if the task
    // tries to acquire it on its way out.
    drop(active);

    // Give the task a short grace period to observe the cancel signal and shut
    // down in an orderly way.  Abort is kept as a last resort — it skips
    // ordered shutdown and may leave shared state partially updated.
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let had_to_abort = {
        // Re-acquire briefly to check task liveness; abort only if still running.
        let active_check = state.active_plans.read().await;
        if let Some(h) = active_check.get(&key) {
            if !h.handle.is_finished() {
                task_abort.abort();
                true
            } else {
                false
            }
        } else {
            false
        }
    };

    // Write a lightweight snapshot so the dashboard knows the plan is
    // paused and `POST /resume` can restart it.
    let snapshot_dir = state.workdir.join(".roko").join("state");
    if let Err(err) = tokio::fs::create_dir_all(&snapshot_dir).await {
        tracing::warn!(path = %snapshot_dir.display(), error = %err, "failed to create state dir for pause snapshot");
    }
    let snapshot_path = snapshot_dir.join(format!("{id}.paused.json"));
    let snapshot = json!({
        "plan_id": id,
        "paused": true,
        "paused_at": chrono::Utc::now().to_rfc3339(),
        "plan_dir": captured_plan_dir,
        "run_id": captured_run_id,
    });
    if let Err(err) = tokio::fs::write(
        &snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_default(),
    )
    .await
    {
        tracing::warn!(path = %snapshot_path.display(), error = %err, "failed to write pause snapshot");
    }

    // Remove from active set using the resolved key.
    let mut active_final = state.active_plans.write().await;
    drop(active_final.remove(&key));

    // Publish PlanCompleted only when the task did not finish cleanly on its
    // own.  If the run observed the cancel token and returned, it already
    // published PlanCompleted; publishing again here would deliver a duplicate
    // to every connected WebSocket client.
    if had_to_abort {
        state.event_bus.publish(ServerEvent::PlanCompleted {
            plan_id: id.clone(),
            success: false,
        });
    }

    Ok(Json(json!({ "paused": true, "snapshot": snapshot_path })))
}

/// `POST /api/plans/:id/resume` — resume a paused plan execution.
///
/// Removes the `.roko/state/<id>.paused.json` marker written by `pause_plan`,
/// then delegates to [`start_plan_run`] with `resume: true` — the same path
/// that `POST /api/plans/{id}/execute` with `{ "resume": true }` follows.
///
/// This fixes the previous implementation, which used the legacy flat-file
/// `find_plan` (404 for every directory plan) and sent the plan as a text
/// prompt to `run_once` instead of running it through `run_plan_with_options`.
async fn resume_plan(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    // Remove the paused marker written by pause_plan (ignore errors: the file
    // may not exist if the plan was never paused, or was already cleaned up).
    let paused_path = state
        .workdir
        .join(".roko")
        .join("state")
        .join(format!("{id}.paused.json"));
    let _ = tokio::fs::remove_file(&paused_path).await;

    // Delegate to the shared helper (validates id, resolves plan dir from
    // group, checks for conflicts, spawns the run with force_resume: true).
    let run_id = start_plan_run(&state, id, true).await?;

    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(json!({ "id": run_id, "resumed": true, "resume": true })),
    ))
}

/// `POST /api/plans/:id/cancel` — permanently cancel a running plan execution.
///
/// Unlike `/pause`, this handler does **not** write a snapshot file, so the
/// plan cannot be resumed afterwards.  It signals the cancel token for ordered
/// shutdown, waits a short grace window, aborts the task if still running, and
/// then removes the plan from the active-plans map.
///
/// Returns 200 `{ "cancelled": true }` on success, or 404 when the plan is not
/// actively executing.
///
/// `{id}` may be the run key **or** any member plan id of an active run.
async fn cancel_plan(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let active = state.active_plans.write().await;
    // Resolve by key or by member plan id.
    let key = active_run_for(&active, &id)
        .ok_or_else(|| ApiError::not_found("no active execution for this plan"))?;
    let handle = active
        .get(&key)
        .expect("key from active_run_for must exist in map");

    if handle.handle.is_finished() {
        return Err(ApiError::not_found("no active execution for this plan"));
    }

    // Capture the abort handle before releasing the lock.
    let task_abort = handle.handle.abort_handle();

    // Signal ordered cancellation so the task can unwind cleanly.
    handle.cancel.cancel();

    // Release the write lock so the spawned task can make progress during the
    // grace window.
    drop(active);

    // Give the task a short grace period to observe the cancel signal.
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let had_to_abort = {
        let active_check = state.active_plans.read().await;
        if let Some(h) = active_check.get(&key) {
            if !h.handle.is_finished() {
                task_abort.abort();
                true
            } else {
                false
            }
        } else {
            false
        }
    };

    // Remove from the active set using the resolved key.  No snapshot is
    // written — a cancelled plan is not resumable.
    let mut active_final = state.active_plans.write().await;
    drop(active_final.remove(&key));

    // Publish PlanCompleted only when the task did not finish cleanly on its
    // own.  If the run observed the cancel token and returned, it already
    // published PlanCompleted; publishing again here would deliver a duplicate
    // to every connected WebSocket client.
    if had_to_abort {
        state.event_bus.publish(ServerEvent::PlanCompleted {
            plan_id: id.clone(),
            success: false,
        });
    }

    Ok(Json(json!({ "cancelled": true })))
}

// ── Verify results query ──────────────────────────────────────────────

/// `GET /api/plans/:id/gates` — query gate results for a specific plan.
///
/// Returns gate verdicts from the materialized dashboard snapshot, filtered
/// to the requested plan.
async fn plan_gates(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    validate_path_segment(&id, "plan id")?;

    // Verify the plan exists via the runtime (finds directory-layout plans).
    let _plan = resolve_plan(&state, &id).await?;

    // Pull gate results from the materialized snapshot.
    let snapshot = state.state_hub.current_snapshot();
    let gates: Vec<Value> = snapshot
        .gates
        .iter()
        .filter(|g| g.plan_id == id)
        .map(|g| {
            json!({
                "plan_id": g.plan_id,
                "task_id": g.task_id,
                "gate": g.gate,
                "passed": g.passed,
                "ts_millis": g.ts_millis,
            })
        })
        .collect();

    Ok(Json(json!({
        "plan_id": id,
        "gate_count": gates.len(),
        "passed": gates.iter().filter(|g| g["passed"] == true).count(),
        "failed": gates.iter().filter(|g| g["passed"] == false).count(),
        "gates": gates,
    })))
}

/// `GET /api/plans/{id}/costs` -- cost breakdown from efficiency events.
///
/// Reads `.roko/learn/efficiency.jsonl`, filters by plan_id, and returns
/// per-task cost breakdown, remaining-cost projection, and budget status.
async fn plan_costs(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    validate_path_segment(&id, "plan id")?;

    // Verify the plan exists via the runtime (finds directory-layout plans).
    let plan = resolve_plan(&state, &id).await?;

    // Read efficiency events from the learning log.
    let efficiency_path = state
        .workdir
        .join(".roko")
        .join("learn")
        .join("efficiency.jsonl");
    let content = tokio::fs::read_to_string(&efficiency_path)
        .await
        .unwrap_or_default();

    #[derive(serde::Deserialize)]
    struct EffRow {
        #[serde(default)]
        plan_id: String,
        #[serde(default)]
        task_id: String,
        #[serde(default)]
        model: String,
        #[serde(default)]
        cost_usd: f64,
        #[serde(default)]
        input_tokens: u64,
        #[serde(default)]
        output_tokens: u64,
    }

    let mut task_costs: std::collections::HashMap<String, (f64, u64, u64, String)> =
        std::collections::HashMap::new();
    let mut total_cost = 0.0_f64;
    let mut total_input = 0_u64;
    let mut total_output = 0_u64;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row: EffRow = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if row.plan_id != id {
            continue;
        }
        total_cost += row.cost_usd;
        total_input += row.input_tokens;
        total_output += row.output_tokens;
        let entry = task_costs
            .entry(row.task_id.clone())
            .or_insert((0.0, 0, 0, String::new()));
        entry.0 += row.cost_usd;
        entry.1 += row.input_tokens;
        entry.2 += row.output_tokens;
        if entry.3.is_empty() {
            entry.3 = row.model;
        }
    }

    let mut projector = CostProjector::new();
    for (task_id, (cost, _, _, model)) in &task_costs {
        let tier = plan
            .tasks
            .iter()
            .find(|task| task.id == *task_id)
            .map_or("focused", |task| task.tier.as_str());
        projector.record_completed(&CompletedTask {
            tier: tier.to_string(),
            model: model.clone(),
            cost_usd: *cost,
        });
    }
    let remaining = plan
        .tasks
        .iter()
        .filter(|task| !task.completed && !task_costs.contains_key(&task.id))
        .map(|task| RemainingTask {
            tier: task.tier.clone(),
            model_hint: task.model_hint.clone().unwrap_or_default(),
        })
        .collect::<Vec<_>>();
    let config = state.load_roko_config();
    let default_model = resolve_model(&config, &config.agent.default_model).slug;
    let projection = projector.project_remaining_cost(&remaining, &default_model);
    let projected_total_usd = projection.projected_total_usd();
    let budget_limit_usd = f64::from(config.budget.max_plan_usd);
    let budget_enabled = budget_limit_usd > 0.0;
    let budget_remaining_usd = budget_enabled.then(|| (budget_limit_usd - total_cost).max(0.0));
    let budget_utilization = budget_enabled.then(|| total_cost / budget_limit_usd);
    let budget_status = if !budget_enabled {
        "unlimited"
    } else if total_cost >= budget_limit_usd {
        "exceeded"
    } else if projected_total_usd > budget_limit_usd {
        "projected_exceeded"
    } else if total_cost / budget_limit_usd >= 0.8 {
        "warning"
    } else {
        "ok"
    };

    let mut tasks: Vec<Value> = plan
        .tasks
        .iter()
        .map(|task| {
            let (cost, input, output, model) =
                task_costs.get(&task.id).cloned().unwrap_or_default();
            let task_budget = config
                .budget
                .task_limit_usd(&task.tier, task.model_hint.as_deref());
            json!({
                "id": task.id,
                "task_id": task.id,
                "tier": task.tier,
                "spent": cost,
                "budget": (task_budget > 0.0).then_some(task_budget),
                "cost_usd": cost,
                "input_tokens": input,
                "output_tokens": output,
                "model": model,
                "budget_exhausted": task_budget > 0.0 && cost >= task_budget,
            })
        })
        .collect();
    tasks.sort_by(|a, b| a["task_id"].as_str().cmp(&b["task_id"].as_str()));

    let mut provider_health = state
        .provider_health_registry
        .snapshot()
        .into_values()
        .map(|health| {
            json!({
                "id": health.provider_id,
                "state": health.state,
                "cooldown_until_ms": health.cooldown_until,
            })
        })
        .collect::<Vec<_>>();
    provider_health.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));

    Ok(Json(json!({
        "plan_id": id,
        "plan_spent": total_cost,
        "plan_budget": budget_enabled.then_some(budget_limit_usd),
        "task_costs": tasks,
        "provider_health": provider_health,
        "total_cost_usd": total_cost,
        "total_input_tokens": total_input,
        "total_output_tokens": total_output,
        "task_count": tasks.len(),
        "tasks": tasks,
        "projection": {
            "optimistic_remaining_usd": projection.optimistic_usd,
            "expected_remaining_usd": projection.expected_usd,
            "pessimistic_remaining_usd": projection.pessimistic_usd,
            "projected_total_usd": projected_total_usd,
            "tasks_completed": projection.tasks_completed,
            "tasks_remaining": projection.tasks_remaining,
            "confidence": projection.confidence,
        },
        "budget": {
            "enabled": budget_enabled,
            "limit_usd": budget_enabled.then_some(budget_limit_usd),
            "remaining_usd": budget_remaining_usd,
            "utilization": budget_utilization,
            "status": budget_status,
            "projected_exceeded": budget_enabled && projection.exceeds_budget(budget_limit_usd),
        },
    })))
}

// ── Chat-based plan editing ─────────────────────────────────────────

#[derive(Deserialize, Validate)]
struct PlanChatRequest {
    #[validate(
        length(min = 1),
        custom(function = "crate::extract::validate_non_blank")
    )]
    message: String,
}

impl RequestPayload for PlanChatRequest {
    fn validate_payload(&self) -> Result<(), ApiError> {
        validate_with_validator(self)
    }
}

/// `POST /api/plans/:id/chat` — LLM-powered plan mutation via natural language.
///
/// Sends the plan context + user message to the LLM, which returns structured
/// plan mutations (add/remove/update tasks, reorder, add dependencies).
async fn plan_chat(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    ValidJson(body): ValidJson<PlanChatRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let plan = resolve_plan(&state, &id).await?;
    let plan_json = plan_to_json(&plan);

    let prompt = format!(
        "You are a plan editor. Given the plan below and the user's request, return a JSON \
         object with a `mutations` array. Each mutation is one of:\n\
         - {{\"op\":\"add_task\",\"task\":{{\"id\":\"T-new\",\"description\":\"...\",\"depends_on\":[],\"files\":[]}}}}\n\
         - {{\"op\":\"remove_task\",\"task_id\":\"T-old\"}}\n\
         - {{\"op\":\"update_task\",\"task_id\":\"T1\",\"patch\":{{\"description\":\"new desc\"}}}}\n\
         - {{\"op\":\"add_dependency\",\"task_id\":\"T2\",\"depends_on\":\"T1\"}}\n\
         - {{\"op\":\"reorder\",\"order\":[\"T1\",\"T3\",\"T2\"]}}\n\n\
         Return ONLY the JSON object, no markdown fences.\n\n\
         ## Current plan\n```json\n{plan}\n```\n\n## User request\n{msg}",
        plan = serde_json::to_string_pretty(&plan_json).unwrap_or_default(),
        msg = body.message,
    );

    let op_id = uuid::Uuid::new_v4().to_string();
    let bus = state.event_bus.clone();
    let runtime = state.runtime.clone();
    let workdir = state.workdir.clone();
    let plan_id = id.clone();

    let op_id_inner = op_id.clone();
    let handle = tokio::spawn(async move {
        let success = match runtime.run_once(&workdir, &prompt).await {
            Ok(RunResult {
                success,
                output_text,
                ..
            }) => {
                // Try to parse mutations from the LLM output and apply them.
                if let Some(ref text) = output_text {
                    if let Ok(mutations) = serde_json::from_str::<Value>(text) {
                        // Write the mutations to a response file for the caller.
                        let mutations_path = workdir
                            .join(".roko")
                            .join("state")
                            .join(format!("{plan_id}.chat-response.json"));
                        if let Err(err) = tokio::fs::write(
                            &mutations_path,
                            serde_json::to_string_pretty(&mutations).unwrap_or_default(),
                        )
                        .await
                        {
                            tracing::warn!(
                                path = %mutations_path.display(),
                                error = %err,
                                "failed to write plan chat mutations response"
                            );
                        }
                    }
                }
                success
            }
            Err(err) => {
                bus.publish(ServerEvent::Error {
                    message: format!("plan chat failed for {plan_id}: {err}"),
                });
                false
            }
        };
        bus.publish(ServerEvent::OperationCompleted {
            op_id: op_id_inner,
            kind: "plan_chat".into(),
            success,
        });
    });

    let op = OperationHandle {
        id: op_id.clone(),
        kind: format!("plan_chat:{id}"),
        status: OperationStatus::Running,
        handle,
    };
    state.operations.write().await.insert(op_id.clone(), op);

    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(json!({ "id": op_id })),
    ))
}

// ── Cost estimation ─────────────────────────────────────────────────

/// `POST /api/plans/:id/estimate` — estimate cost and time for plan execution.
///
/// Reads historical efficiency events to build per-task estimates based on
/// past performance for similar roles and models.
async fn plan_estimate(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let plan = resolve_plan(&state, &id).await?;

    // Load historical efficiency data.
    let efficiency_path = state
        .workdir
        .join(".roko")
        .join("learn")
        .join("efficiency.jsonl");
    let historical = load_efficiency_history(&efficiency_path).await;

    // Compute per-task estimates.
    let mut task_estimates = Vec::new();
    let mut total_input_tokens: u64 = 0;
    let mut total_output_tokens: u64 = 0;
    let mut total_cost_usd: f64 = 0.0;
    let mut total_duration_secs: f64 = 0.0;

    for task in &plan.tasks {
        if task.completed {
            continue;
        }
        // Find similar historical tasks (by matching plan role or averaging all).
        let (est_input, est_output, est_cost, est_duration) =
            estimate_task_from_history(&historical);

        total_input_tokens += est_input;
        total_output_tokens += est_output;
        total_cost_usd += est_cost;
        total_duration_secs += est_duration;

        task_estimates.push(json!({
            "task_id": task.id,
            "description": task.description,
            "estimated_input_tokens": est_input,
            "estimated_output_tokens": est_output,
            "estimated_cost_usd": format!("{:.4}", est_cost),
            "estimated_duration_secs": format!("{:.0}", est_duration),
        }));
    }

    let completed = plan.tasks.iter().filter(|t| t.completed).count();

    Ok(Json(json!({
        "plan_id": id,
        "total_tasks": plan.tasks.len(),
        "completed_tasks": completed,
        "remaining_tasks": plan.tasks.len() - completed,
        "estimate": {
            "total_input_tokens": total_input_tokens,
            "total_output_tokens": total_output_tokens,
            "total_cost_usd": format!("{:.4}", total_cost_usd),
            "total_duration_secs": format!("{:.0}", total_duration_secs),
        },
        "per_task": task_estimates,
        "confidence": if historical.is_empty() { "low" } else { "medium" },
        "note": if historical.is_empty() {
            "No historical data available; using default estimates"
        } else {
            "Based on historical efficiency events"
        },
    })))
}

/// A simplified efficiency record parsed from the JSONL log.
struct HistoricalEfficiency {
    input_tokens: u64,
    output_tokens: u64,
    cost_usd: f64,
    duration_secs: f64,
}

/// Load efficiency history from the JSONL log file.
async fn load_efficiency_history(path: &std::path::Path) -> Vec<HistoricalEfficiency> {
    let content = match tokio::fs::read_to_string(path).await {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    content
        .lines()
        .filter_map(|line| {
            let v: Value = serde_json::from_str(line).ok()?;
            Some(HistoricalEfficiency {
                input_tokens: v.get("input_tokens")?.as_u64()?,
                output_tokens: v.get("output_tokens")?.as_u64()?,
                cost_usd: v.get("cost_usd").and_then(|c| c.as_f64()).unwrap_or(0.0),
                duration_secs: v
                    .get("wall_clock_ms")
                    .and_then(|d| d.as_f64())
                    .unwrap_or(30_000.0)
                    / 1000.0,
            })
        })
        .collect()
}

/// Estimate a single task from historical averages, with fallback defaults.
fn estimate_task_from_history(history: &[HistoricalEfficiency]) -> (u64, u64, f64, f64) {
    if history.is_empty() {
        // Default estimates for a single agent task.
        return (8_000, 4_000, 0.05, 60.0);
    }

    let n = history.len() as f64;
    let avg_input = (history.iter().map(|h| h.input_tokens).sum::<u64>() as f64 / n) as u64;
    let avg_output = (history.iter().map(|h| h.output_tokens).sum::<u64>() as f64 / n) as u64;
    let avg_cost = history.iter().map(|h| h.cost_usd).sum::<f64>() / n;
    let avg_duration = history.iter().map(|h| h.duration_secs).sum::<f64>() / n;

    (avg_input, avg_output, avg_cost, avg_duration)
}

// ── Review workflow ──────────────────────────────────────────────────

/// `GET /api/plans/:id/reviews` — list tasks pending review.
///
/// Scans plan tasks that are completed (by an agent) and checks for
/// corresponding agent branches.  Returns gate results from the snapshot
/// and diff summaries from git.
async fn list_reviews(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let plan = resolve_plan(&state, &id).await?;

    // Pull gate results for this plan from the snapshot.
    let snapshot = state.state_hub.current_snapshot();
    let plan_gates: Vec<_> = snapshot.gates.iter().filter(|g| g.plan_id == id).collect();

    let mut reviews = Vec::new();

    for task in &plan.tasks {
        // Detect agent branches: convention is `agent/<name>/<task_id>`.
        let branch = find_agent_branch(&state.workdir, &task.id).await;

        // Gather gate results for this task.
        let task_gates: Vec<Value> = plan_gates
            .iter()
            .filter(|g| g.task_id == task.id)
            .map(|g| {
                json!({
                    "gate": g.gate,
                    "passed": g.passed,
                })
            })
            .collect();

        // Compute diff summary if branch exists.
        let (diff_summary, files_changed) = if let Some(ref branch_name) = branch {
            diff_summary(&state.workdir, branch_name).await
        } else {
            (String::new(), Vec::new())
        };

        // Determine review status.
        let status = if branch.is_some() && !files_changed.is_empty() {
            "pending_review"
        } else if task.completed {
            "completed"
        } else {
            "pending"
        };

        reviews.push(json!({
            "task_id": task.id,
            "description": task.description,
            "status": status,
            "branch": branch,
            "diff_summary": diff_summary,
            "gate_results": task_gates,
            "files_changed": files_changed,
        }));
    }

    Ok(Json(json!({
        "plan_id": id,
        "reviews": reviews,
    })))
}

#[derive(Deserialize, Validate)]
struct ReviewDecision {
    #[validate(custom(function = "validate_decision"))]
    decision: String,
    #[serde(default)]
    comment: String,
}

fn validate_decision(decision: &str) -> Result<(), validator::ValidationError> {
    match decision {
        "approve" | "reject" | "skip" => Ok(()),
        _ => {
            let mut err = validator::ValidationError::new("invalid_decision");
            err.message = Some("decision must be approve, reject, or skip".into());
            Err(err)
        }
    }
}

impl RequestPayload for ReviewDecision {
    fn validate_payload(&self) -> Result<(), ApiError> {
        validate_with_validator(self)
    }
}

/// `POST /api/plans/:id/tasks/:task_id/review` — approve, reject, or skip.
///
/// - **approve**: merge the agent branch into the base branch, mark approved.
/// - **reject**: record rejection with comment, keep branch for rework.
/// - **skip**: mark task as skipped, no merge.
async fn submit_review(
    State(state): State<Arc<AppState>>,
    Path((id, task_id)): Path<(String, String)>,
    ValidJson(body): ValidJson<ReviewDecision>,
) -> Result<Json<Value>, ApiError> {
    validate_path_segment(&id, "plan id")?;
    validate_path_segment(&task_id, "task id")?;

    // Verify plan and task exist.
    let plan = resolve_plan(&state, &id).await?;
    if !plan.tasks.iter().any(|t| t.id == task_id) {
        return Err(ApiError::not_found(format!(
            "task '{task_id}' not found in plan '{id}'"
        )));
    }

    let branch = find_agent_branch(&state.workdir, &task_id).await;

    let result = match body.decision.as_str() {
        "approve" => {
            let merged = if let Some(ref branch_name) = branch {
                merge_branch(&state.workdir, branch_name).await
            } else {
                false
            };

            // Record the review in the state directory.
            record_review(&state.workdir, &id, &task_id, "approved", &body.comment).await;

            json!({
                "task_id": task_id,
                "status": "approved",
                "merged": merged,
                "branch": branch,
            })
        }
        "reject" => {
            // Send feedback to the agent if possible.
            if let Some(ref branch_name) = branch {
                // Extract agent name from branch: `agent/<name>/<task_id>`
                let agent_name = branch_name
                    .strip_prefix("agent/")
                    .and_then(|s| s.split('/').next())
                    .unwrap_or("");

                if !agent_name.is_empty() {
                    let feedback = format!(
                        "Review rejected for task {task_id}: {}",
                        if body.comment.is_empty() {
                            "No comment provided"
                        } else {
                            &body.comment
                        }
                    );
                    // Publish as an event so agents can pick it up.
                    state.event_bus.publish(ServerEvent::Error {
                        message: format!(
                            "review:reject agent={agent_name} task={task_id}: {feedback}"
                        ),
                    });
                }
            }

            record_review(&state.workdir, &id, &task_id, "rejected", &body.comment).await;

            json!({
                "task_id": task_id,
                "status": "needs_rework",
                "branch": branch,
                "comment": body.comment,
            })
        }
        "skip" => {
            record_review(&state.workdir, &id, &task_id, "skipped", &body.comment).await;
            json!({ "task_id": task_id, "status": "skipped" })
        }
        _ => unreachable!("validated above"),
    };

    Ok(Json(result))
}

/// `GET /api/plans/:id/tasks/:task_id/diff` — structured diff for a task.
///
/// Finds the agent branch and runs `git diff` against main. Returns per-file
/// diff entries with path, status, additions, deletions, and unified patch.
async fn task_diff(
    State(state): State<Arc<AppState>>,
    Path((id, task_id)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    validate_path_segment(&id, "plan id")?;
    validate_path_segment(&task_id, "task id")?;

    // Verify plan and task exist.
    let plan = resolve_plan(&state, &id).await?;
    if !plan.tasks.iter().any(|t| t.id == task_id) {
        return Err(ApiError::not_found(format!(
            "task '{task_id}' not found in plan '{id}'"
        )));
    }

    let branch = find_agent_branch(&state.workdir, &task_id)
        .await
        .ok_or_else(|| {
            ApiError::not_found(format!("no agent branch found for task '{task_id}'"))
        })?;

    let files = parse_git_diff(&state.workdir, &branch).await?;

    Ok(Json(json!({
        "task_id": task_id,
        "branch": branch,
        "base": "main",
        "file_count": files.len(),
        "total_additions": files.iter().map(|f| f.additions).sum::<u32>(),
        "total_deletions": files.iter().map(|f| f.deletions).sum::<u32>(),
        "files": files.iter().map(|f| json!({
            "path": f.path,
            "status": f.status,
            "additions": f.additions,
            "deletions": f.deletions,
            "patch": f.patch,
        })).collect::<Vec<_>>(),
    })))
}

// ── Review helpers ──────────────────────────────────────────────────

/// Find an agent branch matching the task ID.
///
/// Convention: `agent/<agent-name>/<task_id>`.
async fn find_agent_branch(workdir: &std::path::Path, task_id: &str) -> Option<String> {
    let output = tokio::process::Command::new("git")
        .args(["branch", "--list", &format!("agent/*/{task_id}")])
        .current_dir(workdir)
        .output()
        .await
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    // git branch --list returns "  branch-name\n" or "* branch-name\n"
    stdout
        .lines()
        .map(|l| l.trim_start_matches(['*', ' '].as_ref()).trim().to_string())
        .find(|l| !l.is_empty())
}

/// Compute a short diff summary ("+N -M across K files") for a branch.
async fn diff_summary(workdir: &std::path::Path, branch: &str) -> (String, Vec<String>) {
    let output = tokio::process::Command::new("git")
        .args(["diff", "--stat", &format!("main...{branch}")])
        .current_dir(workdir)
        .output()
        .await;

    let output = match output {
        Ok(o) => o,
        Err(_) => return (String::new(), Vec::new()),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    // Last line is summary: " N files changed, M insertions(+), K deletions(-)"
    let summary = lines
        .last()
        .map(|l| l.trim().to_string())
        .unwrap_or_default();

    // Each preceding line is " path/to/file | N ++--"
    let files: Vec<String> = lines
        .iter()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.contains('|') {
                Some(trimmed.split('|').next().unwrap_or("").trim().to_string())
            } else {
                None
            }
        })
        .collect();

    (summary, files)
}

struct DiffFile {
    path: String,
    status: String,
    additions: u32,
    deletions: u32,
    patch: String,
}

/// Parse `git diff --numstat` + `git diff` into structured per-file entries.
async fn parse_git_diff(
    workdir: &std::path::Path,
    branch: &str,
) -> Result<Vec<DiffFile>, ApiError> {
    // Get numstat for additions/deletions counts.
    let numstat = tokio::process::Command::new("git")
        .args(["diff", "--numstat", &format!("main...{branch}")])
        .current_dir(workdir)
        .output()
        .await
        .map_err(|e| ApiError::internal(format!("git diff --numstat: {e}")))?;

    let numstat_str = String::from_utf8_lossy(&numstat.stdout);

    // Get the full diff for patches.
    let full_diff = tokio::process::Command::new("git")
        .args(["diff", &format!("main...{branch}")])
        .current_dir(workdir)
        .output()
        .await
        .map_err(|e| ApiError::internal(format!("git diff: {e}")))?;

    let full_diff_str = String::from_utf8_lossy(&full_diff.stdout);

    // Parse per-file patches from the full diff.
    let mut file_patches: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut current_file = String::new();
    let mut current_patch = String::new();

    for line in full_diff_str.lines() {
        if line.starts_with("diff --git") {
            if !current_file.is_empty() {
                file_patches.insert(current_file.clone(), current_patch.clone());
            }
            // Extract filename from "diff --git a/path b/path"
            current_file = line.split(" b/").nth(1).unwrap_or("").to_string();
            current_patch = String::new();
        }
        current_patch.push_str(line);
        current_patch.push('\n');
    }
    if !current_file.is_empty() {
        file_patches.insert(current_file, current_patch);
    }

    // Parse numstat lines: "additions\tdeletions\tpath"
    let mut files = Vec::new();
    for line in numstat_str.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            continue;
        }
        let additions = parts[0].parse::<u32>().unwrap_or(0);
        let deletions = parts[1].parse::<u32>().unwrap_or(0);
        let path = parts[2].to_string();

        let status = if additions > 0 && deletions == 0 {
            "added"
        } else if additions == 0 && deletions > 0 {
            "deleted"
        } else {
            "modified"
        };

        let patch = file_patches.get(&path).cloned().unwrap_or_default();

        files.push(DiffFile {
            path,
            status: status.to_string(),
            additions,
            deletions,
            patch,
        });
    }

    Ok(files)
}

/// Merge an agent branch into the current branch.
async fn merge_branch(workdir: &std::path::Path, branch: &str) -> bool {
    let output = tokio::process::Command::new("git")
        .args([
            "merge",
            "--no-ff",
            "-m",
            &format!("Merge {branch} (approved via review)"),
            branch,
        ])
        .current_dir(workdir)
        .output()
        .await;

    matches!(output, Ok(o) if o.status.success())
}

/// Record a review decision to `.roko/state/reviews.jsonl`.
async fn record_review(
    workdir: &std::path::Path,
    plan_id: &str,
    task_id: &str,
    decision: &str,
    comment: &str,
) {
    let reviews_path = workdir.join(".roko").join("state").join("reviews.jsonl");
    if let Err(err) = tokio::fs::create_dir_all(reviews_path.parent().unwrap_or(workdir)).await {
        tracing::warn!(path = %reviews_path.display(), error = %err, "failed to create reviews state directory");
        return;
    }

    let entry = serde_json::json!({
        "plan_id": plan_id,
        "task_id": task_id,
        "decision": decision,
        "comment": comment,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    let mut line = serde_json::to_string(&entry).unwrap_or_default();
    line.push('\n');

    // Append atomically.
    match tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&reviews_path)
        .await
    {
        Ok(mut f) => {
            use tokio::io::AsyncWriteExt;
            if let Err(err) = f.write_all(line.as_bytes()).await {
                tracing::warn!(path = %reviews_path.display(), error = %err, "failed to write review entry");
            } else if let Err(err) = f.flush().await {
                tracing::warn!(path = %reviews_path.display(), error = %err, "failed to flush review entry");
            }
        }
        Err(err) => {
            tracing::warn!(path = %reviews_path.display(), error = %err, "failed to open reviews file for append");
        }
    }
}

/// Request body for `POST /api/plans/generate`.
///
/// Exactly one of `slug` or `prompt` must be supplied:
/// - `slug` — generate from an existing PRD identified by its slug.
/// - `prompt` — generate directly from a free-text prompt (used by the portal
///   "Generate…" field, which never has a pre-existing PRD slug to hand).
///
/// Supplying both or neither is a 422 (Unprocessable Entity).  This two-field
/// design exists because the portal sends `prompt` while the CLI sends `slug`;
/// T09 will route between the two strategies in the handler body.
#[derive(Deserialize, Validate)]
struct GenerateRequest {
    /// An existing PRD slug.  Non-blank when present.
    #[serde(default)]
    slug: Option<String>,
    /// A free-text prompt used to generate the plan directly.  Non-blank when present.
    #[serde(default)]
    prompt: Option<String>,
}

impl RequestPayload for GenerateRequest {
    fn validate_payload(&self) -> Result<(), ApiError> {
        match (&self.slug, &self.prompt) {
            (None, None) => Err(ApiError::unprocessable_entity(
                "exactly one of 'slug' or 'prompt' must be supplied",
            )),
            (Some(_), Some(_)) => Err(ApiError::unprocessable_entity(
                "supply either 'slug' or 'prompt', not both",
            )),
            (Some(s), None) => {
                if s.trim().is_empty() {
                    Err(ApiError::unprocessable_entity("'slug' must not be blank"))
                } else {
                    Ok(())
                }
            }
            (None, Some(p)) => {
                if p.trim().is_empty() {
                    Err(ApiError::unprocessable_entity("'prompt' must not be blank"))
                } else {
                    Ok(())
                }
            }
        }
    }
}

/// `POST /api/plans/generate` — spawn background plan generation from a PRD slug or prompt.
///
/// Two paths:
/// - `slug`: find an existing PRD, then call `runtime.generate_plan_from_prd`.
/// - `prompt`: derive a slug, write a PRD draft, then call `runtime.generate_plan_from_prd`.
///
/// Responds 202 with `{ "id": op_id, "plan_id": slug }`.  The slug is known
/// before the background work starts so the portal's generate hook can use it
/// immediately.
///
/// The operation ends `Completed { result: {"slug", "task_count"} }` when the
/// runtime wrote a plan, and `Failed { error }` when generation failed or
/// finished without writing one.
///
/// The operation handle is registered in `state.operations` before the spawned
/// task can finish (a oneshot start signal gates the task exactly as
/// `spawn_background_run` in `routes/run.rs` does), so polling
/// `GET /api/operations/{id}` is race-free from the moment this handler returns.
async fn generate_plan(
    State(state): State<Arc<AppState>>,
    ValidJson(body): ValidJson<GenerateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // `validate_payload` guarantees exactly one of `slug`/`prompt` is Some.
    let (slug, prd_path) = if let Some(ref s) = body.slug {
        // Slug path: resolve the PRD that already exists on disk.
        let (path, _content) = find_prd(&state.workdir, s).await?;
        (s.clone(), path)
    } else {
        // Prompt path: derive a unique slug, write a PRD draft, use its path.
        let prompt_text = body.prompt.clone().unwrap_or_default();
        let slug = derive_unique_slug(&state.workdir, &prompt_text).await;
        let path = write_prompt_prd(&state.workdir, &slug, &prompt_text).await?;
        (slug, path)
    };

    let op_id = uuid::Uuid::new_v4().to_string();
    let bus = state.event_bus.clone();
    let runtime = state.runtime.clone();
    let workdir = state.workdir.clone();
    let kind = format!("plan_generate:{slug}");
    let slug_for_task = slug.clone();
    let state_for_task = Arc::clone(&state);

    // Gate the task on a start signal so the handle is always registered before
    // the task can write back its result (mirrors `spawn_background_run`).
    let (start_tx, start_rx) = tokio::sync::oneshot::channel::<()>();

    let handle = tokio::spawn({
        let op_id = op_id.clone();
        async move {
            // Wait until the caller has inserted the OperationHandle.
            let _ = start_rx.await;

            // Announce the start on the dashboard stream.
            {
                use roko_core::DashboardEvent;
                state_for_task
                    .state_hub
                    .publish_batch(vec![DashboardEvent::EventLogEntry {
                        timestamp_ms: generate_now_millis(),
                        event_type: "plan_generate.started".into(),
                        plan_id: slug_for_task.clone(),
                        task_id: String::new(),
                        message: format!("▶ generate op={op_id}"),
                    }]);
            }
            bus.publish(ServerEvent::OperationStarted {
                op_id: op_id.clone(),
                kind: "plan_generate".into(),
            });

            match runtime
                .generate_plan_from_prd(&workdir, &slug_for_task, &prd_path)
                .await
            {
                // Finishing without a plan is a failure: a `completed`
                // operation tells the portal to open the plan it names.
                Ok(gen_result) if gen_result.plan_targets.is_empty() => {
                    fail_generate_operation(
                        &state_for_task,
                        op_id,
                        &slug_for_task,
                        format!(
                            "plan generation for {slug_for_task} finished without writing a plan"
                        ),
                        "no_plan_written",
                    )
                    .await;
                }
                Ok(_) => {
                    // Count tasks via the runtime so the result carries live data.
                    let task_count = state_for_task
                        .runtime
                        .load_plan_summary(&workdir, &slug_for_task)
                        .await
                        .ok()
                        .flatten()
                        .map(|dto| dto.task_count)
                        .unwrap_or(0);

                    let result_json = serde_json::to_string(
                        &json!({ "slug": slug_for_task, "task_count": task_count }),
                    )
                    .unwrap_or_default();

                    // Update the handle to Completed.
                    if let Some(h) = state_for_task.operations.write().await.get_mut(&op_id) {
                        h.status = crate::state::OperationStatus::Completed {
                            result: Some(result_json),
                        };
                    }

                    {
                        use roko_core::DashboardEvent;
                        state_for_task.state_hub.publish_batch(vec![
                            DashboardEvent::EventLogEntry {
                                timestamp_ms: generate_now_millis(),
                                event_type: "plan_generate.completed".into(),
                                plan_id: slug_for_task.clone(),
                                task_id: String::new(),
                                message: format!("op={op_id} tasks={task_count}"),
                            },
                        ]);
                    }
                    bus.publish(ServerEvent::OperationCompleted {
                        op_id,
                        kind: "plan_generate".into(),
                        success: true,
                    });
                }
                Err(err) => {
                    fail_generate_operation(
                        &state_for_task,
                        op_id,
                        &slug_for_task,
                        format!("plan generation failed for {slug_for_task}: {err}"),
                        err,
                    )
                    .await;
                }
            }
        }
    });

    let op = OperationHandle {
        id: op_id.clone(),
        kind,
        status: OperationStatus::Running,
        handle,
    };

    state.operations.write().await.insert(op_id.clone(), op);
    // Unblock the task now that the handle is registered.
    let _ = start_tx.send(());

    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(json!({ "id": op_id, "plan_id": slug })),
    ))
}

/// Finish a generate operation as failed: its handle carries `error`, and the
/// event bus and the dashboard stream (`plan_generate.failed`, with `detail`)
/// announce it.
async fn fail_generate_operation(
    state: &AppState,
    op_id: String,
    plan_id: &str,
    error: String,
    detail: impl std::fmt::Display,
) {
    state.event_bus.publish(ServerEvent::Error {
        message: error.clone(),
    });
    if let Some(h) = state.operations.write().await.get_mut(&op_id) {
        h.status = OperationStatus::Failed { error };
    }
    state
        .state_hub
        .publish_batch(vec![roko_core::DashboardEvent::EventLogEntry {
            timestamp_ms: generate_now_millis(),
            event_type: "plan_generate.failed".into(),
            plan_id: plan_id.to_string(),
            task_id: String::new(),
            message: format!("op={op_id} error={detail}"),
        }]);
    state.event_bus.publish(ServerEvent::OperationCompleted {
        op_id,
        kind: "plan_generate".into(),
        success: false,
    });
}

/// Request body for `POST /api/plans/{id}/revise`.
#[derive(Deserialize)]
struct ReviseRequest {
    feedback: String,
}

/// `POST /api/plans/{id}/revise` — revise a plan's source based on textual feedback.
///
/// - 422 when `feedback` is blank.
/// - 404 when the plan does not exist.
/// - 409 when an active plan run includes this plan, or when another revision
///   operation for the same plan is already in progress.
/// - 202 `{ "id": op_id, "plan_id": id }` when the revision task was spawned.
///
/// The background task calls `runtime.revise_plan`, then finalises the
/// operation exactly as `generate_plan` does:
/// - `Completed { result: {"slug", "task_count"} }` when the source was written.
/// - `Failed { error }` when the plan was rejected (validation errors) or the
///   agent failed.
///
/// Publishes `event_log_entry` events for `plan_revise.started`,
/// `plan_revise.completed`, and `plan_revise.failed`.
async fn revise_plan(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    body: axum::body::Bytes,
) -> Result<impl IntoResponse, ApiError> {
    validate_path_segment(&id, "plan id")?;

    // Parse and validate the request body — blank feedback is 422.
    let req: ReviseRequest = serde_json::from_slice(&body).map_err(ApiError::parse)?;
    if req.feedback.trim().is_empty() {
        return Err(ApiError::unprocessable_entity(
            "'feedback' must not be blank",
        ));
    }

    // 404 — plan must exist before we queue work.
    let _ = state
        .runtime
        .load_plan_summary(&state.workdir, &id)
        .await
        .map_err(|e| ApiError::internal(format!("load plan '{id}': {e}")))?
        .ok_or_else(|| ApiError::not_found(format!("plan '{id}' not found")))?;

    // 409 — active plan run includes this plan.
    {
        let active = state.active_plans.read().await;
        if let Some(conflict_key) = active_run_for(&active, &id) {
            return Err(ApiError::conflict(format!(
                "plan '{id}' is part of an active run (run key: {conflict_key}); \
                 finish or cancel the run before revising the plan"
            )));
        }
    }

    // 409 — another revision for the same plan is already in progress.
    {
        let ops = state.operations.read().await;
        let revision_kind = format!("plan_revise:{id}");
        let already_running = ops
            .values()
            .any(|op| op.kind == revision_kind && matches!(op.status, OperationStatus::Running));
        if already_running {
            return Err(ApiError::conflict(format!(
                "a revision of plan '{id}' is already in progress"
            )));
        }
    }

    let op_id = uuid::Uuid::new_v4().to_string();
    let bus = state.event_bus.clone();
    let runtime = state.runtime.clone();
    let workdir = state.workdir.clone();
    let kind = format!("plan_revise:{id}");
    let plan_id_for_task = id.clone();
    let feedback = req.feedback.clone();
    let state_for_task = Arc::clone(&state);

    // Gate the task on a start signal so the handle is always registered before
    // the task can write back its result (mirrors `generate_plan`).
    let (start_tx, start_rx) = tokio::sync::oneshot::channel::<()>();

    let handle = tokio::spawn({
        let op_id = op_id.clone();
        async move {
            // Wait until the caller has inserted the OperationHandle.
            let _ = start_rx.await;

            // Announce start.
            {
                use roko_core::DashboardEvent;
                state_for_task
                    .state_hub
                    .publish_batch(vec![DashboardEvent::EventLogEntry {
                        timestamp_ms: generate_now_millis(),
                        event_type: "plan_revise.started".into(),
                        plan_id: plan_id_for_task.clone(),
                        task_id: String::new(),
                        message: format!("▶ revise op={op_id}"),
                    }]);
            }
            bus.publish(ServerEvent::OperationStarted {
                op_id: op_id.clone(),
                kind: "plan_revise".into(),
            });

            match runtime
                .revise_plan(&workdir, &plan_id_for_task, &feedback)
                .await
            {
                Ok(Some(dto)) => {
                    let (event_type, success) = if dto.revised {
                        // Written successfully.
                        let result_json = serde_json::to_string(
                            &json!({ "slug": plan_id_for_task, "task_count": dto.task_count }),
                        )
                        .unwrap_or_default();
                        if let Some(h) = state_for_task.operations.write().await.get_mut(&op_id) {
                            h.status = crate::state::OperationStatus::Completed {
                                result: Some(result_json),
                            };
                        }
                        ("plan_revise.completed", true)
                    } else {
                        // Rejected by validation.
                        let error_msg =
                            format!("plan revision rejected: {} error(s)", dto.validation.errors);
                        if let Some(h) = state_for_task.operations.write().await.get_mut(&op_id) {
                            h.status = crate::state::OperationStatus::Failed {
                                error: error_msg.clone(),
                            };
                        }
                        bus.publish(ServerEvent::Error { message: error_msg });
                        ("plan_revise.failed", false)
                    };

                    {
                        use roko_core::DashboardEvent;
                        state_for_task.state_hub.publish_batch(vec![
                            DashboardEvent::EventLogEntry {
                                timestamp_ms: generate_now_millis(),
                                event_type: event_type.into(),
                                plan_id: plan_id_for_task.clone(),
                                task_id: String::new(),
                                message: format!("op={op_id} tasks={}", dto.task_count),
                            },
                        ]);
                    }
                    bus.publish(ServerEvent::OperationCompleted {
                        op_id,
                        kind: "plan_revise".into(),
                        success,
                    });
                }
                Ok(None) => {
                    // Plan disappeared between the pre-check and now.
                    let error_msg = format!("plan '{plan_id_for_task}' not found during revision");
                    if let Some(h) = state_for_task.operations.write().await.get_mut(&op_id) {
                        h.status = crate::state::OperationStatus::Failed {
                            error: error_msg.clone(),
                        };
                    }
                    {
                        use roko_core::DashboardEvent;
                        state_for_task.state_hub.publish_batch(vec![
                            DashboardEvent::EventLogEntry {
                                timestamp_ms: generate_now_millis(),
                                event_type: "plan_revise.failed".into(),
                                plan_id: plan_id_for_task.clone(),
                                task_id: String::new(),
                                message: format!("op={op_id} error=not_found"),
                            },
                        ]);
                    }
                    bus.publish(ServerEvent::Error { message: error_msg });
                    bus.publish(ServerEvent::OperationCompleted {
                        op_id,
                        kind: "plan_revise".into(),
                        success: false,
                    });
                }
                Err(err) => {
                    let error_msg = format!("plan revision failed for {plan_id_for_task}: {err}");
                    bus.publish(ServerEvent::Error {
                        message: error_msg.clone(),
                    });
                    if let Some(h) = state_for_task.operations.write().await.get_mut(&op_id) {
                        h.status = crate::state::OperationStatus::Failed {
                            error: error_msg.clone(),
                        };
                    }
                    {
                        use roko_core::DashboardEvent;
                        state_for_task.state_hub.publish_batch(vec![
                            DashboardEvent::EventLogEntry {
                                timestamp_ms: generate_now_millis(),
                                event_type: "plan_revise.failed".into(),
                                plan_id: plan_id_for_task.clone(),
                                task_id: String::new(),
                                message: format!("op={op_id} error={err}"),
                            },
                        ]);
                    }
                    bus.publish(ServerEvent::OperationCompleted {
                        op_id,
                        kind: "plan_revise".into(),
                        success: false,
                    });
                }
            }
        }
    });

    let op = OperationHandle {
        id: op_id.clone(),
        kind,
        status: OperationStatus::Running,
        handle,
    };

    state.operations.write().await.insert(op_id.clone(), op);
    // Unblock the task now that the handle is registered.
    let _ = start_tx.send(());

    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(json!({ "id": op_id, "plan_id": id })),
    ))
}

/// Return the current time as milliseconds since the Unix epoch.
#[allow(clippy::cast_possible_truncation)]
fn generate_now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

// ── Plan source (TOML authoring) ─────────────────────────────────────

/// `GET /api/plans/{id}/source` — read the raw `tasks.toml` for a plan.
///
/// Returns `{ "id", "path", "toml" }` (200), or 404 for an unknown plan.
async fn get_plan_source(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    validate_path_segment(&id, "plan id")?;

    let dto: PlanSourceDto = state
        .runtime
        .plan_source(&state.workdir, &id)
        .await
        .map_err(|e| ApiError::internal(format!("read source for plan '{id}': {e}")))?
        .ok_or_else(|| ApiError::not_found(format!("plan '{id}' not found")))?;

    Ok(Json(json!({
        "id": dto.id,
        "path": dto.path,
        "toml": dto.toml,
    })))
}

/// Request body for `PUT /api/plans/{id}/source`.
#[derive(Deserialize)]
struct PutPlanSourceRequest {
    toml: String,
}

/// `PUT /api/plans/{id}/source` — overwrite the raw `tasks.toml` for a plan.
///
/// - 200 `{ "saved": true, "errors", "warnings", "diagnostics" }` on success.
/// - 422 `{ "code": "invalid_plan", "message", "errors", "warnings", "diagnostics" }`
///   when validation rejects the content (file on disk untouched).
/// - 404 for an unknown plan.
/// - 409 when an active run already includes the plan (it reads the source
///   mid-flight; changing it would corrupt the run).
async fn put_plan_source(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    body: axum::body::Bytes,
) -> Result<impl IntoResponse, ApiError> {
    validate_path_segment(&id, "plan id")?;

    // 409 when a live run includes this plan (source is read mid-flight).
    let active = state.active_plans.read().await;
    if let Some(conflict_key) = active_run_for(&active, &id) {
        return Err(ApiError::conflict(format!(
            "plan '{id}' is part of an active run (run key: {conflict_key}); \
             finish or cancel the run before editing its source"
        )));
    }
    drop(active);

    // Parse the request body.
    let req: PutPlanSourceRequest = serde_json::from_slice(&body).map_err(ApiError::parse)?;

    // Delegate to the runtime: validates, then writes only when valid.
    let dto: PlanValidationDto = state
        .runtime
        .save_plan_source(&state.workdir, &id, req.toml)
        .await
        .map_err(|e| ApiError::internal(format!("save source for plan '{id}': {e}")))?
        .ok_or_else(|| ApiError::not_found(format!("plan '{id}' not found")))?;

    if dto.valid {
        // 200 — saved successfully.
        Ok((
            axum::http::StatusCode::OK,
            Json(json!({
                "saved": true,
                "errors": dto.errors,
                "warnings": dto.warnings,
                "diagnostics": dto.diagnostics,
            })),
        )
            .into_response())
    } else {
        // 422 — rejected; file untouched.  Keep `code`/`message` so the
        // portal's typed API error parser can decode it.
        let body = json!({
            "code": "invalid_plan",
            "message": dto.diagnostics.iter()
                .find(|d| d.severity == "error")
                .map(|d| d.message.clone())
                .unwrap_or_else(|| "plan source is invalid".to_string()),
            "errors": dto.errors,
            "warnings": dto.warnings,
            "diagnostics": dto.diagnostics,
        });
        Ok((axum::http::StatusCode::UNPROCESSABLE_ENTITY, Json(body)).into_response())
    }
}

/// Request body for `POST /api/plans/{id}/validate`.
///
/// The entire body is optional: omit it to validate the file on disk.
#[derive(Deserialize)]
struct ValidatePlanRequest {
    toml: String,
}

/// `POST /api/plans/{id}/validate` — validate a plan source without saving.
///
/// - With no body: validates the plan file currently on disk.
/// - With `{ "toml": "..." }`: validates that text exactly as a save would,
///   without writing anything.
///
/// Always returns 200 with `{ "valid", "errors", "warnings", "diagnostics" }`
/// regardless of whether the plan is valid — an invalid plan is a normal
/// editing state; the portal renders its badge from the counts and anchors
/// each diagnostic to the task it names via `task_id`.
///
/// - 404 when the plan does not exist.
/// - 400 for a malformed request body.
async fn validate_plan(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    body: axum::body::Bytes,
) -> Result<Json<Value>, ApiError> {
    validate_path_segment(&id, "plan id")?;

    // Parse optional body: empty body → validate the on-disk file;
    // body with `{ "toml": "..." }` → validate that text without writing.
    let toml: Option<String> = if body.is_empty() {
        None
    } else {
        let req: ValidatePlanRequest = serde_json::from_slice(&body).map_err(ApiError::parse)?;
        Some(req.toml)
    };

    let dto: PlanValidationDto = state
        .runtime
        .validate_plan_source(&state.workdir, &id, toml)
        .await
        .map_err(|e| ApiError::internal(format!("validate source for plan '{id}': {e}")))?
        .ok_or_else(|| ApiError::not_found(format!("plan '{id}' not found")))?;

    // Always 200 — an invalid plan is a normal editing state; the portal
    // decides how to display it based on `valid` and the diagnostic list.
    Ok(Json(json!({
        "valid": dto.valid,
        "errors": dto.errors,
        "warnings": dto.warnings,
        "diagnostics": dto.diagnostics,
    })))
}

// ── helpers ──────────────────────────────────────────────────────────

/// Derive a status string from a task's completion state.
fn task_status(task: &PlanTask) -> &'static str {
    if task.completed {
        "completed"
    } else {
        "pending"
    }
}

/// Serialize a `Plan` into a `serde_json::Value`.
fn plan_to_json(plan: &Plan) -> Value {
    json!({
        "id": plan.id,
        "title": plan.title,
        "description": plan.description,
        "tasks": plan.tasks.iter().map(|t| json!({
            "id": t.id,
            "description": t.description,
            "tier": t.tier,
            "model_hint": t.model_hint,
            "depends_on": t.depends_on,
            "files": t.files,
            "completed": t.completed,
            "status": task_status(t),
        })).collect::<Vec<_>>(),
    })
}

/// Derive a URL-safe slug from a human title.
///
/// Conversion rules (stable; T09 reuses this function):
/// 1. Lowercase the entire string (ASCII only — non-ASCII is treated as a
///    separator so multi-lingual titles get a deterministic safe slug).
/// 2. Collapse every run of characters that is not an ASCII alphanumeric or
///    `-` into a single `-`.
/// 3. Strip leading and trailing `-` characters.
/// 4. Truncate to at most 48 characters, then strip any newly trailing `-`.
///
/// Empty titles produce the empty string; callers should validate the result
/// with [`validate_path_segment`] before using it as a file-system component.
pub(crate) fn slug_from_title(title: &str) -> String {
    let lower = title.to_ascii_lowercase();
    let mut slug = String::with_capacity(lower.len());
    let mut in_sep = true; // true → skip leading dashes
    for ch in lower.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' {
            if ch == '-' {
                if !in_sep {
                    slug.push('-');
                    in_sep = true;
                }
            } else {
                slug.push(ch);
                in_sep = false;
            }
        } else if !in_sep {
            slug.push('-');
            in_sep = true;
        }
    }
    // Trim trailing separator and enforce the 48-character limit.
    let slug = slug.trim_end_matches('-');
    // Truncate at 48 chars (all chars are ASCII, so char == byte boundary).
    let slug = if slug.len() > 48 { &slug[..48] } else { slug };
    // A clean truncation boundary may leave a trailing dash (e.g. "foo-bar-"
    // after snipping at a separator position).
    slug.trim_end_matches('-').to_string()
}

async fn find_prd(
    workdir: &std::path::Path,
    slug: &str,
) -> Result<(std::path::PathBuf, String), ApiError> {
    validate_path_segment(slug, "PRD slug")?;

    let prds_dir = workdir.join(".roko").join("prd");
    for section in ["published", "drafts"] {
        let path = prds_dir.join(section).join(format!("{slug}.md"));
        if path.is_file() {
            let content = tokio::fs::read_to_string(&path)
                .await
                .map_err(|e| ApiError::internal(format!("read prd file: {e}")))?;
            return Ok((path, content));
        }
    }

    Err(ApiError::not_found(format!("PRD '{slug}' not found")))
}

/// Derive a unique plan slug for a free-text prompt.
///
/// Takes the first line of the prompt (up to 80 chars) as the title, converts
/// it to a kebab-case slug via [`slug_from_title`], then checks whether any
/// plan directory or PRD file (published or draft) already uses that name.
/// If there is a collision it appends `-2`, `-3`, and so on until a free
/// name is found.
async fn derive_unique_slug(workdir: &std::path::Path, prompt: &str) -> String {
    let first_line = prompt.lines().next().unwrap_or("").trim();
    let title = if first_line.len() > 80 {
        &first_line[..80]
    } else {
        first_line
    };
    let base = slug_from_title(title);
    let base = if base.is_empty() {
        "plan".to_string()
    } else {
        base
    };

    let plans_root = plans_dir(workdir);
    let prd_root = workdir.join(".roko").join("prd");

    let is_used = |slug: &str| -> bool {
        plans_root.join(slug).exists()
            || prd_root
                .join("published")
                .join(format!("{slug}.md"))
                .exists()
            || prd_root.join("drafts").join(format!("{slug}.md")).exists()
    };

    if !is_used(&base) {
        return base;
    }
    for n in 2u32.. {
        let candidate = format!("{base}-{n}");
        if !is_used(&candidate) {
            return candidate;
        }
    }
    base // unreachable in practice
}

/// Write a free-text prompt as a PRD draft at `.roko/prd/drafts/<slug>.md`.
///
/// The file has YAML front-matter with `title` (first line, ≤80 chars, no
/// quotes) and `source: api`, followed by `# <title>` and then the full
/// prompt verbatim.  `runtime.generate_plan_from_prd` derives the workspace
/// from the file path, so the file must live exactly at that location.
async fn write_prompt_prd(
    workdir: &std::path::Path,
    slug: &str,
    prompt: &str,
) -> Result<std::path::PathBuf, ApiError> {
    let first_line = prompt.lines().next().unwrap_or("").trim();
    let title = if first_line.len() > 80 {
        &first_line[..80]
    } else {
        first_line
    };

    let content = format!("---\ntitle: {title}\nsource: api\n---\n# {title}\n\n{prompt}\n");

    let drafts_dir = workdir.join(".roko").join("prd").join("drafts");
    tokio::fs::create_dir_all(&drafts_dir)
        .await
        .map_err(|e| ApiError::internal(format!("create prd drafts dir: {e}")))?;

    let prd_path = drafts_dir.join(format!("{slug}.md"));
    tokio::fs::write(&prd_path, content)
        .await
        .map_err(|e| ApiError::internal(format!("write prd draft for '{slug}': {e}")))?;

    Ok(prd_path)
}

/// Resolve the plans directory for the given workspace root.
///
/// Prefers the top-level `plans/` directory when it already exists as a
/// directory, and falls back to the legacy `.roko/plans` location otherwise.
/// This mirrors the identical helper in `roko-cli` so that `create_plan`
/// writes to the same location that `list_plans` / `get_plan` read from.
///
/// Note: this function only *probes* whether the top-level directory exists —
/// it never creates it as a side effect.
fn plans_dir(workdir: &std::path::Path) -> std::path::PathBuf {
    let top = workdir.join("plans");
    if top.is_dir() {
        return top;
    }
    workdir.join(".roko").join("plans")
}

/// Load a plan by id using the runtime's plan-discovery methods.
///
/// Returns 404 when the plan is absent, just like the legacy `find_plan`
/// helper did — but unlike `find_plan`, this resolves directory-layout
/// plans correctly by delegating to the runtime rather than probing for
/// flat `.json`/`.toml` files.
async fn resolve_plan(state: &Arc<AppState>, id: &str) -> Result<Plan, ApiError> {
    validate_path_segment(id, "plan id")?;

    let summary = state
        .runtime
        .load_plan_summary(&state.workdir, id)
        .await
        .map_err(|e| ApiError::internal(format!("load plan '{id}': {e}")))?
        .ok_or_else(|| ApiError::not_found(format!("plan '{id}' not found")))?;

    let tasks_dto = state
        .runtime
        .load_plan_tasks(&state.workdir, id)
        .await
        .map_err(|e| ApiError::internal(format!("load tasks for plan '{id}': {e}")))?
        .ok_or_else(|| ApiError::not_found(format!("plan '{id}' not found")))?;

    let mut plan = Plan::new(summary.id, summary.title, String::new());
    for t in tasks_dto.tasks {
        plan.add_task(PlanTask {
            id: t.id,
            description: t.description.unwrap_or_else(|| t.title.clone()),
            tier: t.tier,
            model_hint: t.model_hint,
            depends_on: t.depends_on,
            files: t.files,
            completed: t.completed,
        });
    }
    Ok(plan)
}

#[cfg(test)]
mod tests {
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
        async fn run_once(
            &self,
            workdir: &std::path::Path,
            prompt: &str,
        ) -> anyhow::Result<RunResult> {
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
            Ok(Some(crate::plan_types::PlanValidationDto {
                valid: true,
                errors: 0,
                warnings: 0,
                diagnostics: vec![],
            }))
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
            Ok(Some(crate::plan_types::PlanValidationDto {
                valid: true,
                errors: 0,
                warnings: 0,
                diagnostics: vec![],
            }))
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
                error.contains("finished without writing a plan"),
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

    /// When `<workdir>/plans/` does not exist, `plans_dir` should fall back
    /// to the legacy `.roko/plans` path (without creating any directory).
    #[test]
    fn plans_dir_falls_back_to_dotted_roko_when_top_level_absent() {
        let dir = tempdir().expect("tempdir");
        // Do NOT create `plans/` — only the dotted path should be returned.
        let expected = dir.path().join(".roko").join("plans");

        let result = plans_dir(dir.path());
        assert_eq!(result, expected, "should fall back to .roko/plans");
        // Confirm the helper did not create the directory as a side effect.
        assert!(
            !dir.path().join("plans").exists(),
            "plans_dir must not create the top-level directory"
        );
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
        // workdir has no top-level `plans/` dir, so plans_dir returns .roko/plans.
        let expected_dir = state
            .workdir
            .join(".roko")
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

        let response =
            match execute_plans(State(Arc::clone(&state)), axum::body::Bytes::new()).await {
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
    async fn execute_plans_conflicts_with_active_run() {
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

        // Second call while the first run is registered — should 409.
        let err = match execute_plans(State(Arc::clone(&state)), axum::body::Bytes::new()).await {
            Ok(_) => panic!("second execute_plans must error"),
            Err(e) => e,
        };

        assert_eq!(
            err.status,
            axum::http::StatusCode::CONFLICT,
            "concurrent execute_plans must return 409"
        );
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

            fn dashboard_scaffold(
                &self,
                _workdir: &std::path::Path,
            ) -> crate::runtime::DashboardInfo {
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
            status: crate::state::OperationStatus::Running,
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
}
