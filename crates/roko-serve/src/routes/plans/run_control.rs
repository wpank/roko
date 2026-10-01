//! Plan run control: run a plan or a plan set, and report, pause, resume and
//! cancel a run.

use super::*;

// ── Active-run bookkeeping helpers ────────────────────────────────────

/// Returns the map key of any unfinished run entry, or `None` when every
/// entry is already finished or the map is empty.
///
/// A finished entry does **not** constitute a conflict: `execute_plan`
/// replaces a stale finished entry rather than blocking on it.
pub(super) fn active_run_conflict(
    active: &std::collections::HashMap<String, PlanHandle>,
) -> Option<String> {
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
pub(super) fn active_run_for(
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
            run_id: Some(run_id_for_task.clone()),
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
                cancel: Some(task_cancel),
                fresh: !resume,
                force_resume: resume,
                live_agent_output: Some(live_agent_output),
                // The run takes the id this handler returns (bug-4f833d).
                run_id: Some(run_id),
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
            // The Graph run settles the plan itself. Publish PlanCompleted
            // only for a run that did not: one that failed before the plan
            // started, or a runtime that publishes no plan lifecycle. Clients
            // then see exactly one.
            if !hub_published_plan_completed(&hub, first_run_seq, &plan_id) {
                bus.publish(ServerEvent::PlanCompleted { plan_id, success });
            }
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
/// `{id}` may be the run key **or** any member plan id of an active run.
pub(super) async fn plan_status(
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
pub(super) async fn pause_plan(
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
pub(super) async fn resume_plan(
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
    let started = start_plan_run(&state, id, true).await?;

    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(json!({
            "id": started.run_id,
            "run_id": started.run_id,
            "resumed": true,
            "resume": true,
            "skippable_task_ids": started.skippable_task_ids,
        })),
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
