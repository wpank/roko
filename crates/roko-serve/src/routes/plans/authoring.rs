//! Plan authoring: create, generate and revise plans, chat about a plan, and
//! read, write and validate a plan's source.

use super::*;
use crate::runtime::{ProgressReporter, RunOrigin};

#[derive(Deserialize, Validate)]
pub(super) struct CreatePlanRequest {
    /// Human-readable plan title (required).
    #[validate(
        length(min = 1),
        custom(function = "crate::extract::validate_non_blank")
    )]
    pub(super) title: String,
    /// Optional slug.  When absent the slug is derived from the title:
    /// lowercase ASCII, runs of non-alphanumeric characters collapsed to
    /// `-`, truncated to 48 characters.
    #[serde(default)]
    pub(super) slug: Option<String>,
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
pub(super) async fn create_plan(
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
            validation.errors.len()
        ))),
    }
}

// ── Chat-based plan editing ─────────────────────────────────────────

#[derive(Deserialize, Validate)]
pub(super) struct PlanChatRequest {
    #[validate(
        length(min = 1),
        custom(function = "crate::extract::validate_non_blank")
    )]
    pub(super) message: String,
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
pub(super) async fn plan_chat(
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

    let work = async move {
        match runtime.run_once(&workdir, &prompt).await {
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
                if success {
                    Ok(None)
                } else {
                    Err(format!("plan chat for {plan_id} did not succeed"))
                }
            }
            Err(err) => {
                let message = format!("plan chat failed for {plan_id}: {err}");
                bus.publish(ServerEvent::Error {
                    message: message.clone(),
                });
                Err(message)
            }
        }
    };
    let op_kind = format!("plan_chat:{id}");
    crate::operations::spawn_operation(&state, op_id.clone(), op_kind, "plan_chat", work).await;

    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(json!({ "id": op_id })),
    ))
}

/// Request body for `POST /api/plans/generate`: the request to plan from.
///
/// `prompt` is required and must not be blank; the portal's "Generate…"
/// field sends it. A `slug` naming a PRD to plan from is refused with 422:
/// the PRD pipeline was removed on 2026-10-02, and plans come from a prompt.
#[derive(Deserialize, Validate)]
pub(super) struct GenerateRequest {
    /// The removed PRD slug, read only to refuse it with a clear message.
    #[serde(default)]
    pub(super) slug: Option<String>,
    /// The request to plan from. Non-blank.
    #[serde(default)]
    pub(super) prompt: Option<String>,
}

impl RequestPayload for GenerateRequest {
    fn validate_payload(&self) -> Result<(), ApiError> {
        if self.slug.is_some() {
            return Err(ApiError::unprocessable_entity(
                "'slug' is no longer accepted: PRDs were removed; send the request to plan \
                 from as 'prompt'",
            ));
        }
        match &self.prompt {
            None => Err(ApiError::unprocessable_entity("'prompt' is required")),
            Some(prompt) if prompt.trim().is_empty() => {
                Err(ApiError::unprocessable_entity("'prompt' must not be blank"))
            }
            Some(_) => Ok(()),
        }
    }
}

/// `POST /api/plans/generate` — spawn background plan generation from a prompt.
///
/// Derives a unique plan slug from the prompt's first line, then runs the
/// plan generator through `runtime.generate_plan_from_prompt`.
///
/// Responds 202 with `{ "id": op_id, "plan_id": slug }`.  The slug is known
/// before the background work starts so the portal's generate hook can use it
/// immediately.
///
/// The operation ends `Completed { result: {"slug", "task_count", "cost_usd"} }`
/// only when plan `slug` loads afterwards. A generation that fails, or that
/// finishes without writing that plan, ends it `Failed { error }`.
///
/// The operation handle is registered in `state.operations` before the spawned
/// task can finish (a oneshot start signal gates the task exactly as
/// `spawn_background_run` in `routes/run.rs` does), so polling
/// `GET /api/operations/{id}` is race-free from the moment this handler returns.
pub(super) async fn generate_plan(
    State(state): State<Arc<AppState>>,
    ValidJson(body): ValidJson<GenerateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // `validate_payload` guarantees a non-blank prompt.
    let prompt_text = body.prompt.clone().unwrap_or_default();
    let (op_id, slug) = start_plan_generation(&state, prompt_text, &RunOrigin::Http, None).await;

    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(json!({ "id": op_id, "plan_id": slug })),
    ))
}

/// Start generating a plan from `prompt_text` in the background, as `POST
/// /api/plans/generate` does, and return the operation's id and the new
/// plan's slug: the route and the MCP `plan_generate` tool (9115) share it.
/// The slug comes from the request; the planner gets a chat host's request
/// fenced as untrusted data ([`RunOrigin::request_text`], 9117).
///
/// With a `title` (gap-c2efe7), the slug comes from the title, and the written
/// plan's `[meta] title` is it.
pub(crate) async fn start_plan_generation(
    state: &Arc<AppState>,
    prompt_text: String,
    origin: &RunOrigin,
    title: Option<String>,
) -> (String, String) {
    let slug = derive_unique_slug(&state.workdir, title.as_deref().unwrap_or(&prompt_text)).await;
    let prompt_text = origin.request_text(&prompt_text).into_owned();
    // bug-7a08a8: a chat host's plan runs with nobody to answer its questions.
    let for_chat_host = origin.is_chat();

    let op_id = uuid::Uuid::new_v4().to_string();
    let bus = state.event_bus.clone();
    let runtime = state.runtime.clone();
    let workdir = state.workdir.clone();
    let kind = format!("plan_generate:{slug}");
    let slug_for_task = slug.clone();
    let state_for_task = Arc::clone(state);

    // Gate the task on a start signal so the handle is always registered before
    // the task can write back its result (mirrors `spawn_background_run`).
    let (start_tx, start_rx) = tokio::sync::oneshot::channel::<()>();
    // gap-11d870: the generation reports what it is doing, and the
    // operation's status shows its latest report.
    let (progress_tx, progress_rx) = tokio::sync::watch::channel(Value::Null);

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

            // The portal opens plan `slug` once the operation completes, so the
            // operation completes only when that plan loads, and fails otherwise.
            let no_plan = || {
                format!(
                    "plan generation for {slug_for_task} finished without writing plan \
                     '{slug_for_task}'"
                )
            };
            let report: ProgressReporter = Arc::new(move |progress| {
                progress_tx.send_replace(progress);
            });
            let generation = runtime
                .generate_plan_reporting(
                    &workdir,
                    &slug_for_task,
                    &prompt_text,
                    for_chat_host,
                    report,
                )
                .await;
            let generated = match generation {
                Ok(gen_result) if gen_result.plan_targets.is_empty() => Err(no_plan()),
                Ok(gen_result) => {
                    name_generated_plan(&workdir, &slug_for_task, title.as_deref());
                    match runtime.load_plan_summary(&workdir, &slug_for_task).await {
                        Ok(Some(summary)) => Ok((summary.task_count, gen_result.cost_usd)),
                        Ok(None) => Err(no_plan()),
                        Err(err) => Err(format!(
                            "plan '{slug_for_task}' does not load after generation: {err}"
                        )),
                    }
                }
                Err(err) => Err(format!("plan generation failed for {slug_for_task}: {err}")),
            };

            match generated {
                Ok((task_count, cost_usd)) => {
                    // bug-a0432b: the result says what writing the plan cost.
                    let result_json = serde_json::to_string(&json!({
                        "slug": slug_for_task,
                        "task_count": task_count,
                        "cost_usd": cost_usd,
                    }))
                    .unwrap_or_default();

                    // Completed, in the handle and the kept record (bug-a0f01e).
                    crate::operations::finish_operation(
                        &state_for_task,
                        &op_id,
                        crate::state::OperationStatus::Completed {
                            result: Some(result_json),
                        },
                    )
                    .await;

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
                Err(error_msg) => {
                    bus.publish(ServerEvent::Error {
                        message: error_msg.clone(),
                    });

                    // Failed, in the handle and the kept record (bug-a0f01e).
                    crate::operations::finish_operation(
                        &state_for_task,
                        &op_id,
                        crate::state::OperationStatus::Failed {
                            error: error_msg.clone(),
                        },
                    )
                    .await;

                    {
                        use roko_core::DashboardEvent;
                        state_for_task.state_hub.publish_batch(vec![
                            DashboardEvent::EventLogEntry {
                                timestamp_ms: generate_now_millis(),
                                event_type: "plan_generate.failed".into(),
                                plan_id: slug_for_task.clone(),
                                task_id: String::new(),
                                message: format!("op={op_id} error={error_msg}"),
                            },
                        ]);
                    }
                    bus.publish(ServerEvent::OperationCompleted {
                        op_id,
                        kind: "plan_generate".into(),
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
        started: std::time::Instant::now(),
        progress: Some(progress_rx),
    };

    state.operations.write().await.insert(op_id.clone(), op);
    // Unblock the task now that the handle is registered.
    let _ = start_tx.send(());

    (op_id, slug)
}

/// Request body for `POST /api/plans/{id}/revise`.
#[derive(Deserialize)]
pub(super) struct ReviseRequest {
    pub(super) feedback: String,
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
/// - `Completed { result: {"slug", "task_count", "diff"} }` when the source was
///   written; `diff` is the task-level plan diff (3216), which the portal shows.
/// - `Failed { error }` when the plan was rejected (validation errors) or the
///   agent failed.
///
/// Publishes `event_log_entry` events for `plan_revise.started`,
/// `plan_revise.completed`, and `plan_revise.failed`.
pub(super) async fn revise_plan(
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
                        let result_json = serde_json::to_string(&json!({
                            "slug": plan_id_for_task,
                            "task_count": dto.task_count,
                            "diff": dto.diff,
                        }))
                        .unwrap_or_default();
                        crate::operations::finish_operation(
                            &state_for_task,
                            &op_id,
                            crate::state::OperationStatus::Completed {
                                result: Some(result_json),
                            },
                        )
                        .await;
                        ("plan_revise.completed", true)
                    } else {
                        // Rejected by validation.
                        let error_msg = format!(
                            "plan revision rejected: {} error(s)",
                            dto.validation.errors.len()
                        );
                        crate::operations::finish_operation(
                            &state_for_task,
                            &op_id,
                            crate::state::OperationStatus::Failed {
                                error: error_msg.clone(),
                            },
                        )
                        .await;
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
                    crate::operations::finish_operation(
                        &state_for_task,
                        &op_id,
                        crate::state::OperationStatus::Failed {
                            error: error_msg.clone(),
                        },
                    )
                    .await;
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
                    crate::operations::finish_operation(
                        &state_for_task,
                        &op_id,
                        crate::state::OperationStatus::Failed {
                            error: error_msg.clone(),
                        },
                    )
                    .await;
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
        started: std::time::Instant::now(),
        progress: None,
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
pub(super) fn generate_now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

// ── Plan source (TOML authoring) ─────────────────────────────────────

/// `GET /api/plans/{id}/source` — read the raw `tasks.toml` for a plan.
///
/// Returns `{ "id", "path", "toml" }` (200), or 404 for an unknown plan.
pub(super) async fn get_plan_source(
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
pub(super) struct PutPlanSourceRequest {
    pub(super) toml: String,
}

/// `PUT /api/plans/{id}/source` — overwrite the raw `tasks.toml` for a plan.
///
/// - 200 `{ "saved": true, "errors", "warnings", "diagnostics" }` on success.
/// - 422 `{ "code": "invalid_plan", "message", "errors", "warnings", "diagnostics" }`
///   when validation rejects the content (file on disk untouched).
/// - 404 for an unknown plan.
/// - 409 when an active run already includes the plan (it reads the source
///   mid-flight; changing it would corrupt the run).
pub(super) async fn put_plan_source(
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
/// The entire body is optional, and so is `toml`: no body, `{}` and
/// `{"toml": null}` all validate the file on disk.
#[derive(Deserialize)]
pub(super) struct ValidatePlanRequest {
    #[serde(default)]
    pub(super) toml: Option<String>,
}

/// `POST /api/plans/{id}/validate` — validate a plan source without saving.
///
/// - With no body, or a body without `toml`: validates the plan file
///   currently on disk.
/// - With `{ "toml": "..." }`: validates that text exactly as a save would,
///   without writing anything.
///
/// Always returns 200 with `{ "valid", "errors", "warnings", "diagnostics" }`
/// regardless of whether the plan is valid — an invalid plan is a normal
/// editing state. `errors` and `warnings` are arrays of lines; the portal
/// renders its badge from `valid` and the diagnostics, and anchors each
/// diagnostic to the task it names via `task_id`.
///
/// - 404 when the plan does not exist.
/// - 400 for a malformed request body.
pub(super) async fn validate_plan(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    body: axum::body::Bytes,
) -> Result<Json<Value>, ApiError> {
    validate_path_segment(&id, "plan id")?;

    // Parse the optional body: no body or no `toml` → validate the on-disk
    // file; `{ "toml": "..." }` → validate that text without writing.
    let toml: Option<String> = if body.is_empty() {
        None
    } else {
        let req: ValidatePlanRequest = serde_json::from_slice(&body).map_err(ApiError::parse)?;
        req.toml
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

/// Set plan `slug`'s `[meta] title` to `title` (gap-c2efe7), in the `tasks.toml`
/// its generation wrote: the line after its `plan = …` key, replacing a
/// `title` the planner wrote. The overview `plan.md` roko wrote for the plan
/// takes the title too (bug-1602d4): plan listings read a plan's title from
/// that file's first heading. Nothing to do without a title; a plan that
/// cannot be named keeps none, which is logged.
fn name_generated_plan(workdir: &std::path::Path, slug: &str, title: Option<&str>) {
    let Some(title) = title else {
        return;
    };
    if let Err(error) = write_plan_title(workdir, slug, title) {
        tracing::warn!(plan = slug, %error, "the generated plan keeps no title");
        return;
    }
    let plan_md = plans_dir(workdir).join(slug).join("plan.md");
    if let Some(retitled) = std::fs::read_to_string(&plan_md)
        .ok()
        .and_then(|text| retitle_overview(&text, title))
        && let Err(error) = std::fs::write(&plan_md, retitled)
    {
        tracing::warn!(plan = slug, %error, "the plan's overview keeps its old title");
    }
}

/// `plan_md` titled `title`, when it is the overview roko wrote
/// ([`crate::runtime::PLAN_OVERVIEW_INTRO`]): its frontmatter `title:` and its
/// first `#` heading, which stays the body's first `#` line. `None` for a
/// `plan.md` the planner wrote, which keeps its own title.
fn retitle_overview(plan_md: &str, title: &str) -> Option<String> {
    if !plan_md.contains(crate::runtime::PLAN_OVERVIEW_INTRO) {
        return None;
    }
    let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut in_frontmatter = false;
    let mut headed = false;
    let mut lines = Vec::new();
    for (index, line) in plan_md.lines().enumerate() {
        let trimmed = line.trim();
        if index == 0 && trimmed == "---" {
            in_frontmatter = true;
        } else if in_frontmatter && trimmed == "---" {
            in_frontmatter = false;
        } else if in_frontmatter && trimmed.starts_with("title:") {
            // A JSON string is a YAML scalar whatever the title holds.
            let scalar = serde_json::to_string(&title).unwrap_or_default();
            lines.push(format!("title: {scalar}"));
            continue;
        } else if !in_frontmatter && !headed && trimmed.starts_with('#') {
            headed = true;
            lines.push(format!("# {title}"));
            continue;
        }
        lines.push(line.to_string());
    }
    Some(lines.join("\n") + "\n")
}

/// [`name_generated_plan`]'s edit of plan `slug`'s `tasks.toml`.
fn write_plan_title(workdir: &std::path::Path, slug: &str, title: &str) -> anyhow::Result<()> {
    let path = plans_dir(workdir).join(slug).join("tasks.toml");
    let text = std::fs::read_to_string(&path)?;
    let line = format!("title = {}", toml::Value::String(title.to_string()));
    let mut lines: Vec<String> = Vec::new();
    let mut in_meta = false;
    let mut named = false;
    for current in text.lines() {
        let trimmed = current.trim_start();
        if trimmed.starts_with('[') {
            in_meta = trimmed.starts_with("[meta]");
        }
        let is_key = |key: &str| {
            trimmed
                .strip_prefix(key)
                .is_some_and(|rest| rest.trim_start().starts_with('='))
        };
        if in_meta && is_key("title") {
            continue;
        }
        lines.push(current.to_string());
        if in_meta && !named && is_key("plan") {
            lines.push(line.clone());
            named = true;
        }
    }
    anyhow::ensure!(named, "{} has no [meta] plan key", path.display());
    std::fs::write(&path, lines.join("\n") + "\n")?;
    Ok(())
}

/// Derive a unique plan slug for a free-text prompt.
///
/// Takes the first line of the prompt (up to 80 chars) as the title, converts
/// it to a kebab-case slug via [`slug_from_title`], then checks whether a
/// plan directory already uses that name. If there is a collision it appends
/// `-2`, `-3`, and so on until a free name is found.
pub(super) async fn derive_unique_slug(workdir: &std::path::Path, prompt: &str) -> String {
    let first_line = prompt.lines().next().unwrap_or("").trim();
    // At most 80 characters: cutting at byte 80 can split a multi-byte
    // character, which panics (bug-7feee7).
    let title = first_line
        .char_indices()
        .nth(80)
        .map_or(first_line, |(end, _)| &first_line[..end]);
    let base = slug_from_title(title);
    let base = if base.is_empty() {
        "plan".to_string()
    } else {
        base
    };

    let plans_root = plans_dir(workdir);
    let is_used = |slug: &str| -> bool { plans_root.join(slug).exists() };

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

#[cfg(test)]
mod title_tests {
    use super::*;

    /// gap-c2efe7: a title given to plan generation becomes the written
    /// plan's `[meta] title`, the line after its `plan` key, in place of one
    /// the planner wrote; no title leaves the file as it was.
    #[test]
    fn a_generated_plan_takes_the_given_title() {
        let dir = tempfile::tempdir().expect("tempdir");
        let plan_dir = plans_dir(dir.path()).join("named");
        std::fs::create_dir_all(&plan_dir).expect("plan dir");
        let tasks = plan_dir.join("tasks.toml");
        let written = "[meta]\nplan = \"named\"\ntitle = \"The planner's\"\ntotal = 1\n\n\
                       [[task]]\nid = \"T1\"\ntitle = \"A task\"\n";
        std::fs::write(&tasks, written).expect("tasks.toml");

        name_generated_plan(dir.path(), "named", None);
        assert_eq!(
            std::fs::read_to_string(&tasks).expect("tasks.toml"),
            written
        );

        name_generated_plan(dir.path(), "named", Some("Count \"vowels\""));
        let named = std::fs::read_to_string(&tasks).expect("tasks.toml");
        let parsed: toml::Value = toml::from_str(&named).expect("still TOML");
        assert_eq!(parsed["meta"]["title"].as_str(), Some("Count \"vowels\""));
        assert_eq!(parsed["task"][0]["title"].as_str(), Some("A task"));
        assert_eq!(named.matches("title = ").count(), 2, "{named}");
    }

    /// bug-1602d4: naming a generated plan titles the overview `plan.md`
    /// roko wrote for it too, its frontmatter and its first heading, which
    /// stays the body's first `#` line, so plan listings show the name. A
    /// `plan.md` the planner wrote keeps its own title.
    #[test]
    fn a_named_plan_retitles_its_overview() {
        let dir = tempfile::tempdir().expect("tempdir");
        let write_plan = |slug: &str, plan_md: &str| {
            let plan_dir = plans_dir(dir.path()).join(slug);
            std::fs::create_dir_all(&plan_dir).expect("plan dir");
            std::fs::write(
                plan_dir.join("tasks.toml"),
                format!("[meta]\nplan = \"{slug}\"\ntotal = 1\n\n[[task]]\nid = \"T1\"\n"),
            )
            .expect("tasks.toml");
            std::fs::write(plan_dir.join("plan.md"), plan_md).expect("plan.md");
            plan_dir.join("plan.md")
        };
        let overview = write_plan(
            "word-count-demo",
            &format!(
                "---\nplan: word-count-demo\ntitle: word-count-demo\n---\n\n\
                 # word-count-demo\n\n{}\n\n## Request\n\nA chat host asked for this plan.\n\n\
                 ## Tasks\n\n### Wave 1\n\n- **T1: Count words** (implementer, focused)\n",
                crate::runtime::PLAN_OVERVIEW_INTRO
            ),
        );
        let own = "---\nplan: planner-titled\ntitle: Planner title\n---\n\n# Planner title\n\n\
                   The planner's own words.\n";
        let planner = write_plan("planner-titled", own);

        name_generated_plan(dir.path(), "word-count-demo", Some("Word count demo"));
        name_generated_plan(dir.path(), "planner-titled", Some("Word count demo"));

        let retitled = std::fs::read_to_string(&overview).expect("plan.md");
        assert!(
            retitled.starts_with("---\nplan: word-count-demo\ntitle: \"Word count demo\"\n---\n"),
            "{retitled}"
        );
        let body = retitled
            .splitn(3, "---\n")
            .nth(2)
            .expect("a body after the frontmatter");
        let first_heading = body.lines().find(|line| line.trim_start().starts_with('#'));
        assert_eq!(first_heading, Some("# Word count demo"), "{retitled}");
        assert!(retitled.contains("### Wave 1"), "{retitled}");
        assert_eq!(std::fs::read_to_string(&planner).expect("plan.md"), own);
    }
}
