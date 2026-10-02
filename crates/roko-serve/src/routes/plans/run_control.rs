//! Plan run control: run a plan or a plan set, and report, pause, resume and
//! cancel a run.

use super::*;

use crate::state::{PlanRunStatus, RunState};
use roko_core::dashboard_snapshot::classify_task_outcome;

// ── Active-run bookkeeping helpers ────────────────────────────────────

/// Returns the map key of any live run entry ([`PlanHandle::is_live`]), or
/// `None` when every entry has ended or the map is empty.
///
/// A finished entry does **not** constitute a conflict: `execute_plan`
/// replaces a stale finished entry rather than blocking on it.
pub(super) fn active_run_conflict(
    active: &std::collections::HashMap<String, PlanHandle>,
) -> Option<String> {
    active
        .iter()
        .find(|(_, h)| h.is_live())
        .map(|(key, _)| key.clone())
}

/// Whether the run of `handle`, stored under `key`, is the one `id` names:
/// `id` is its key, its run id or one of its member plan ids.
fn run_named(key: &str, handle: &PlanHandle, id: &str) -> bool {
    key == id || handle.id == id || handle.members.iter().any(|m| m == id)
}

/// Returns the map key of the live entry whose key or run id equals `id`, or
/// whose `members` list contains `id`.
///
/// Returns `None` when no live entry matches — either because `id` is
/// unknown or because every matching entry has already ended.
pub(super) fn active_run_for(
    active: &std::collections::HashMap<String, PlanHandle>,
    id: &str,
) -> Option<String> {
    active
        .iter()
        .find(|(key, h)| h.is_live() && run_named(key, h, id))
        .map(|(key, _)| key.clone())
}

/// Returns the map key of the newest entry `id` names whose run has ended,
/// kept so its status route can report how it ended (G43).
fn finished_run_for(
    active: &std::collections::HashMap<String, PlanHandle>,
    id: &str,
) -> Option<String> {
    active
        .iter()
        .filter(|(key, h)| !h.is_live() && run_named(key, h, id))
        .max_by_key(|(_, h)| h.status.finished_at)
        .map(|(key, _)| key.clone())
}

/// How a plan run ended (G43): [`RunState::of_ended_run`] over whether it was
/// cancelled, whether the runtime reported success, and the last outcome
/// each task of `plans` published into `hub` from `first_seq` on, with
/// `failure` as the error of a run that failed.
fn plan_run_end(
    hub: &roko_runtime::SharedStateHub,
    first_seq: u64,
    plans: &[String],
    cancelled: bool,
    success: bool,
    failure: Option<String>,
) -> PlanRunStatus {
    let mut outcomes = std::collections::BTreeMap::new();
    for envelope in hub.replay_from(first_seq) {
        if let roko_core::DashboardEvent::TaskCompleted {
            plan_id,
            task_id,
            outcome,
        } = envelope.payload
            && plans.contains(&plan_id)
        {
            outcomes.insert((plan_id, task_id), outcome);
        }
    }
    let tasks = outcomes
        .values()
        .map(String::as_str)
        .map(classify_task_outcome);
    let state = RunState::of_ended_run(cancelled, success, tasks);
    let error = (state == RunState::Failed)
        .then(|| failure.unwrap_or_else(|| "a task of the run failed".to_string()));
    PlanRunStatus::ended(state, error)
}

/// Record `status`, how the plan run `run_id` ended, on its handle under
/// `key`, unless a newer run has taken the key or the run's end is already
/// recorded.
async fn record_plan_run_end(state: &AppState, key: &str, run_id: &str, status: PlanRunStatus) {
    if let Some(handle) = state.active_plans.write().await.get_mut(key)
        && handle.id == run_id
        && !handle.status.state.is_terminal()
    {
        handle.status = status;
    }
}

/// Optional request body for `POST /api/plans/:id/execute`.
///
/// Both fields default to `false`, so the portal's "run" button (which sends
/// no body) always triggers a fresh run without needing to supply a payload.
#[derive(Deserialize, Default)]
pub(super) struct ExecutePlanRequest {
    /// `true` — resume from the last checkpoint (`--force-resume`).
    /// `false` (default) — start fresh, discarding any checkpoint (`--fresh`).
    #[serde(default)]
    pub(super) resume: bool,
}

/// Request body for `POST /api/plans/execute`.
///
/// All fields are optional.  `plans` and `target` are mutually exclusive —
/// provide at most one.  Omitting both runs every plan under the workspace
/// plans root ("Run all").
#[derive(Deserialize, Default)]
pub(super) struct ExecutePlansRequest {
    /// Specific plan ids to run from the workspace plans root.
    /// Mutually exclusive with `target`.
    #[serde(default)]
    pub(super) plans: Option<Vec<String>>,
    /// Workspace-relative directory to run.
    /// Mutually exclusive with `plans`.
    #[serde(default)]
    pub(super) target: Option<String>,
    /// When `true`, resume from the last checkpoint; default is a fresh run.
    #[serde(default)]
    pub(super) resume: bool,
    /// Maximum number of independent plans to run concurrently.
    /// Must be ≥ 1 when given; defaults to `[conductor] max_parallel_plans`.
    #[serde(default)]
    pub(super) max_parallel_plans: Option<usize>,
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
pub(super) async fn execute_plans(
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

    // Refuse plans `roko plan run` would refuse, before the run takes the
    // workspace (gap-655d19).
    if let Some(validation) = state
        .runtime
        .validate_plan_run(&state.workdir, &plan_target, only_plans.as_deref())
        .await
        .map_err(|e| ApiError::internal(format!("validate plans: {e}")))?
    {
        return Err(plan_run_rejected("the plan set", &validation));
    }

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
    let state_for_task = Arc::clone(&state);
    let plans_for_task = order.clone();

    // Atomically check-and-insert with the write lock, then spawn the task.
    let order_for_response = order.clone();
    let mut active = state.active_plans.write().await;
    if let Some(conflict_key) = active_run_conflict(&active) {
        return Err(ApiError::conflict(format!(
            "a plan run is already active (run key: {conflict_key})"
        )));
    }

    // Every hub event of this run is sequenced at or after this point.
    let hub = state.state_hub.clone();
    let first_run_seq = hub.total_published();

    let handle = tokio::spawn(async move {
        let options = PlanRunOptions {
            cancel: Some(task_cancel.clone()),
            fresh: !resume,
            force_resume: resume,
            only_plans,
            max_parallel_plans: Some(effective_max),
            live_agent_output: Some(live_agent_output),
            run_id: Some(run_id_for_task.clone()),
        };
        // Do NOT publish plan lifecycle events (plan_started, plan_completed)
        // for the run_id.  The runtime publishes its own per-plan events
        // (plan_set_loaded, run_completed) with the correct metadata.
        let (success, failure) = match runtime
            .run_plan_with_options(&workdir, &plan_target_for_task, options)
            .await
        {
            Ok(result) => {
                let failure = (!result.success)
                    .then(|| format!("plan set run {run_id_for_task} completed with failures"));
                (result.success, failure)
            }
            Err(err) => {
                let message = format!("plan set execution failed (run {run_id_for_task}): {err}");
                bus.publish(ServerEvent::Error {
                    message: message.clone(),
                });
                (false, Some(message))
            }
        };
        // Record how the run ended on its handle, so its status route still
        // answers once it is over (G43).
        let status = plan_run_end(
            &hub,
            first_run_seq,
            &plans_for_task,
            task_cancel.is_cancelled(),
            success,
            failure,
        );
        record_plan_run_end(&state_for_task, &run_id_for_task, &run_id_for_task, status).await;
    });

    let plan_handle = PlanHandle {
        id: run_id.clone(),
        plan_dir: plan_target,
        // members carries every plan id so cancel/status by member id works.
        members: order,
        status: PlanRunStatus::running(),
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
            "run_id": run_id,
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
/// Returns the run it started.
pub(super) async fn start_plan_run(
    state: &Arc<AppState>,
    id: String,
    resume: bool,
) -> Result<StartedPlanRun, ApiError> {
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

    // Refuse a plan `roko plan run` would refuse, before the run takes the
    // workspace (gap-655d19).
    if let Some(validation) = state
        .runtime
        .validate_plan_run(&state.workdir, &plan_dir, None)
        .await
        .map_err(|e| ApiError::internal(format!("validate plan '{id}': {e}")))?
    {
        return Err(plan_run_rejected(&format!("plan '{id}'"), &validation));
    }

    // What a resume replays, read before the run can touch the checkpoint
    // (gap-b07969). It only informs the caller: a runtime that cannot tell,
    // or a checkpoint it cannot read, leaves it unknown.
    let skippable_task_ids = if resume {
        state
            .runtime
            .resume_skippable_tasks(&state.workdir, &plan_dir)
            .await
            .unwrap_or_else(|error| {
                tracing::warn!(plan_id = %id, %error, "could not preview the resume");
                None
            })
    } else {
        Some(Vec::new())
    };

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

    // Every hub event of this run is sequenced at or after this point.
    let hub = state.state_hub.clone();
    let first_run_seq = hub.total_published();

    let handle = tokio::spawn({
        let plan_id = plan_id.clone();
        let plan_dir = plan_dir.clone();
        let run_id = run_id.clone();
        let state_for_task = Arc::clone(state);
        async move {
            // Do NOT publish PlanStarted here. The runtime publishes its own
            // PlanStarted event (with the correct tasks_total) into the server
            // hub. A duplicate from the handler would be forwarded by the
            // bus-to-hub bridge with tasks_total: 0, confusing the portal.
            //
            // Pass the cancel token to the runtime so it can stop itself when
            // the handler calls cancel.cancel(). The run observes the token
            // internally; there is no select! race here.
            let options = PlanRunOptions {
                cancel: Some(task_cancel.clone()),
                fresh: !resume,
                force_resume: resume,
                live_agent_output: Some(live_agent_output),
                // The run takes the id this handler returns (bug-4f833d).
                run_id: Some(run_id.clone()),
                ..PlanRunOptions::default()
            };
            let (success, failure) = match runtime
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
                    let failure = (!success)
                        .then(|| format!("plan {plan_id} completed with task-level failures"));
                    if let Some(message) = &failure {
                        bus.publish(ServerEvent::Error {
                            message: message.clone(),
                        });
                    }
                    (success, failure)
                }
                Err(err) => {
                    let message = format!("plan execution failed for {plan_id}: {err}");
                    bus.publish(ServerEvent::Error {
                        message: message.clone(),
                    });
                    (false, Some(message))
                }
            };
            let status = plan_run_end(
                &hub,
                first_run_seq,
                std::slice::from_ref(&plan_id),
                task_cancel.is_cancelled(),
                success,
                failure,
            );
            // The Graph run settles the plan itself. Publish PlanCompleted
            // only for a run that did not: one that failed before the plan
            // started, or a runtime that publishes no plan lifecycle. Clients
            // then see exactly one.
            if !hub_published_plan_completed(&hub, first_run_seq, &plan_id) {
                bus.publish(ServerEvent::PlanCompleted {
                    plan_id: plan_id.clone(),
                    success,
                });
            }
            // Record how the run ended on its handle, so its status route
            // still answers once it is over (G43).
            record_plan_run_end(&state_for_task, &plan_id, &run_id, status).await;
        }
    });

    let plan_handle = PlanHandle {
        id: run_id.clone(),
        // Store the specific plan's directory, not the parent plans directory.
        plan_dir: plan_dir.clone(),
        // Single-plan run: the only member is this plan.
        members: vec![plan_id.clone()],
        status: PlanRunStatus::running(),
        handle,
        cancel,
    };

    active.insert(id, plan_handle);
    drop(active);

    Ok(StartedPlanRun {
        run_id,
        skippable_task_ids,
    })
}

/// A run [`start_plan_run`] started.
pub(super) struct StartedPlanRun {
    pub(super) run_id: String,
    /// Tasks the run replays from its checkpoint instead of running: none for
    /// a fresh run, `None` when the runtime cannot tell (gap-b07969).
    pub(super) skippable_task_ids: Option<Vec<String>>,
}

/// 422 for a run `roko plan run` would refuse: `details` is the validation
/// report, shaped as `POST /api/plans/{id}/validate` returns it.
pub(super) fn plan_run_rejected(what: &str, validation: &PlanValidationDto) -> ApiError {
    let mut error = ApiError::unprocessable_entity(format!(
        "{what} failed validation with {} error(s); fix them before running it",
        validation.errors.len()
    ));
    error.details = serde_json::to_value(validation).ok().map(Box::new);
    error
}

/// Whether the hub carries a `PlanCompleted` for `plan_id` sequenced at or
/// after `from_seq`. A run's completion is among its last events, so the
/// retained ring still holds it when the run returns.
pub(super) fn hub_published_plan_completed(
    hub: &roko_runtime::SharedStateHub,
    from_seq: u64,
    plan_id: &str,
) -> bool {
    hub.replay_from(from_seq).iter().any(|envelope| {
        matches!(
            &envelope.payload,
            roko_core::DashboardEvent::PlanCompleted { plan_id: completed, .. } if completed == plan_id
        )
    })
}

/// `POST /api/plans/:id/execute` — spawn a background plan execution task.
///
/// Accepts an optional JSON body `{ "resume": bool }` (P-7 of the portal
/// contract). When `resume: true`, the run continues from the last checkpoint;
/// when absent or `false`, the run starts fresh.
pub(super) async fn execute_plan(
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

    let started = start_plan_run(&state, id, resume).await?;

    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(json!({
            "id": started.run_id,
            "run_id": started.run_id,
            "resume": resume,
            "skippable_task_ids": started.skippable_task_ids,
        })),
    ))
}

/// `GET /api/plans/:id/status` — check execution status for a plan.
///
/// `{id}` may be the run key, the run id **or** any member plan id of a run.
/// A live run reports `running`. Once it ends, the newest run `{id}` names
/// reports how it ended, `succeeded`, `failed` (with `error`), `unverified`
/// or `cancelled`, with `finished: true` and `finished_at`, for as long as
/// its handle is kept (G43).
pub(super) async fn plan_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let active = state.active_plans.read().await;
    let key = active_run_for(&active, &id)
        .or_else(|| finished_run_for(&active, &id))
        .ok_or_else(|| ApiError::not_found("no execution of this plan is known"))?;
    let h = active
        .get(&key)
        .expect("key from active_run_for must exist in map");
    let run_state = h.state();
    Ok(Json(json!({
        "id": h.id,
        "run_id": h.id,
        "plan_dir": h.plan_dir,
        "status": run_state.as_str(),
        "error": h.status.error,
        "finished": run_state.is_terminal(),
        "finished_at": h.status.finished_at,
    })))
}

// ── Pause / Resume ───────────────────────────────────────────────────

/// How long `pause_plan` and `resume_plan` wait for the run to take the
/// command they send it. The plan-set driver reads its control file every
/// 100 ms.
const RUN_CONTROL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// How often they look whether it has.
const RUN_CONTROL_POLL: std::time::Duration = std::time::Duration::from_millis(50);

/// The id of the active run of plan (or run key) `id`: 404 when there is
/// none, 409 when it already finished.
async fn running_run(state: &AppState, id: &str) -> Result<String, ApiError> {
    let active = state.active_plans.read().await;
    let key = active_run_for(&active, id)
        .ok_or_else(|| ApiError::not_found("no active execution for this plan"))?;
    let handle = active
        .get(&key)
        .expect("key from active_run_for must exist in map");
    if handle.handle.is_finished() {
        return Err(ApiError::conflict("plan execution already finished"));
    }
    Ok(handle.id.clone())
}

/// Send `action` (`pause` or `resume`) to the plan-set driver of the run in
/// `workdir` through its control file, `.roko/state/control.json`, as `roko
/// plan pause` and `roko plan resume` do, and wait for the run to take it.
///
/// # Errors
///
/// 500 when the command cannot be written; 409 when the run did not take it
/// in time, in which case it is withdrawn.
async fn send_run_control(workdir: &std::path::Path, action: &str) -> Result<(), ApiError> {
    let state_dir = roko_fs::RokoLayout::for_project(workdir).state_dir();
    let path = state_dir.join("control.json");
    let failed = |error: std::io::Error| {
        ApiError::internal(format!("cannot send the {action} to the run: {error}"))
    };
    tokio::fs::create_dir_all(&state_dir)
        .await
        .map_err(failed)?;
    // Written whole, so the run never reads half a command.
    let staged_name = format!("control.json.{}.tmp", uuid::Uuid::new_v4().simple());
    let staged = state_dir.join(staged_name);
    let command = json!({ "command": action }).to_string();
    tokio::fs::write(&staged, command).await.map_err(failed)?;
    tokio::fs::rename(&staged, &path).await.map_err(failed)?;

    let started = tokio::time::Instant::now();
    while tokio::fs::try_exists(&path).await.unwrap_or(false) {
        if started.elapsed() >= RUN_CONTROL_TIMEOUT {
            // Withdrawn, unless the run took it meanwhile.
            return match tokio::fs::remove_file(&path).await {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                _ => Err(ApiError::conflict(format!(
                    "the plan run did not take the {action} within {}s",
                    RUN_CONTROL_TIMEOUT.as_secs()
                ))),
            };
        }
        tokio::time::sleep(RUN_CONTROL_POLL).await;
    }
    Ok(())
}

/// `POST /api/plans/:id/pause` — hold a running plan run (decision 1206).
///
/// Sends the run's plan-set driver a pause, as `roko plan pause` does: no
/// new plan, task or retry starts until resume, the attempts already running
/// finish, and nothing is cancelled. Returns 200 with `{ "paused": true }`
/// once the run took the pause, 404 if the plan is not actively executing,
/// or 409 if it already finished or did not take the pause.
///
/// `{id}` may be the run key **or** any member plan id of an active run.
pub(super) async fn pause_plan(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let run_id = running_run(&state, &id).await?;
    send_run_control(&state.workdir, "pause").await?;
    Ok(Json(json!({ "paused": true, "run_id": run_id })))
}

/// `POST /api/plans/:id/resume` — resume a held plan run, or run a plan
/// again from its checkpoint.
///
/// While a run of the plan is active, sends its plan-set driver a resume, as
/// `roko plan resume` does, and returns 200 with `{ "resumed": true }` once
/// the run took it (409 if it did not). Otherwise delegates to
/// [`start_plan_run`] with `resume: true` — the same path that
/// `POST /api/plans/{id}/execute` with `{ "resume": true }` follows — and
/// returns 202.
pub(super) async fn resume_plan(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, ApiError> {
    if let Ok(run_id) = running_run(&state, &id).await {
        send_run_control(&state.workdir, "resume").await?;
        let body = json!({ "resumed": true, "resume": true, "run_id": run_id });
        return Ok(Json(body).into_response());
    }

    // Delegate to the shared helper (validates id, resolves plan dir from
    // group, checks for conflicts, spawns the run with force_resume: true).
    let started = start_plan_run(&state, id, true).await?;
    let body = json!({
        "id": started.run_id,
        "run_id": started.run_id,
        "resumed": true,
        "resume": true,
        "skippable_task_ids": started.skippable_task_ids,
    });
    Ok((axum::http::StatusCode::ACCEPTED, Json(body)).into_response())
}

/// `POST /api/plans/:id/cancel` — permanently cancel a running plan execution.
///
/// Unlike `/pause`, which holds the run, this handler stops it. It signals
/// the cancel token for ordered shutdown, waits a short grace window, aborts
/// the task if still running, and then records the run as `cancelled`: the
/// handle stays, ended, so `GET /api/plans/{id}/status` reports it (G43).
///
/// Returns 200 `{ "cancelled": true }` on success, or 404 when the plan is not
/// actively executing.
///
/// `{id}` may be the run key **or** any member plan id of an active run.
pub(super) async fn cancel_plan(
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

    // Capture the abort handle and the run's id before releasing the lock.
    let task_abort = handle.handle.abort_handle();
    let run_id = handle.id.clone();

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

    // The run is over: record it as cancelled, unless its task recorded how
    // it ended first. No snapshot is written — a cancelled plan is not
    // resumable.
    let cancelled = PlanRunStatus::ended(RunState::Cancelled, None);
    record_plan_run_end(&state, &key, &run_id, cancelled).await;

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
