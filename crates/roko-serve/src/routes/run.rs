//! Single-prompt run endpoints.

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use roko_core::TaskDomain;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::oneshot;
use validator::Validate;

use crate::error::ApiError;
use crate::events::ServerEvent;
use crate::extract::{RequestPayload, ValidJson, validate_with_validator};
use crate::runtime::{CliRuntime, PromptPlanOptions, RunOrigin, RunResult};
use crate::sanitize::sanitize_agent_content;
use crate::state::{
    AppState, OperationStatus, PLAN_RUN_QUEUE_CAPACITY, QueuedPromptRun, RunHandle, RunState,
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/run", post(start_run))
        .route("/run/{id}/status", get(run_status))
        .route("/surface-events", post(handle_surface_event))
}

#[derive(Deserialize, Validate)]
struct RunRequest {
    #[validate(
        length(min = 1),
        custom(function = "crate::extract::validate_non_blank")
    )]
    prompt: String,
    #[serde(default)]
    workdir: Option<String>,
    /// The run's work domain label (`code`, `research`, `docs`, `chain` or
    /// a custom one), which picks its tool policy and verifier pack (9121).
    /// Default: the project's `default_domain`.
    #[serde(default)]
    domain: Option<String>,
}

impl RequestPayload for RunRequest {
    fn validate_payload(&self) -> Result<(), ApiError> {
        validate_with_validator(self)
    }
}

/// `POST /api/run` — run the prompt in the background as a gated one-task
/// plan through the Graph engine, as `roko run` does, under the id the 202
/// returns (9113).
///
/// One plan executor runs at a time: while another run holds the workspace,
/// the run waits in the run queue, and the 202 says `"queued": true` and its
/// `"position"` (gap-9a8d66).
async fn start_run(
    State(state): State<Arc<AppState>>,
    ValidJson(body): ValidJson<RunRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let domain = body.domain.as_deref().and_then(TaskDomain::from_label);
    let options = PromptPlanOptions {
        domain,
        ..PromptPlanOptions::default()
    };
    let started = start_gated_run(
        &state,
        body.prompt.clone(),
        body.workdir.map(PathBuf::from),
        options,
    )
    .await?;

    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(json!({
            "id": started.run_id,
            "queued": started.queued.is_some(),
            "position": started.queued,
        })),
    ))
}

/// `GET /api/run/:id/status` — check the status of a background run.
///
/// A run that has ended reports its verdict as its status: `succeeded`,
/// `failed`, or `unverified` when no gate checked its output (G42). Only
/// `succeeded` sets `success`.
async fn run_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let runs = state.active_runs.read().await;
    let Some(handle) = runs.get(&id) else {
        drop(runs);
        // A prompt run waiting in the run queue (gap-9a8d66).
        let prompt = state
            .prompt_queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .find(|queued| queued.run_id == id)
            .map(|queued| queued.prompt.clone())
            .ok_or_else(|| ApiError::not_found("run not found"))?;
        return Ok(Json(json!({
            "id": id,
            "prompt": prompt,
            "status": RunState::Queued.as_str(),
            "position": state.queue_position(&id),
            "verdict": null,
            "success": null,
            "output_text": null,
            "error": null,
            "finished": false,
        })));
    };

    let (status, error) = run_handle_state(handle);
    let ended = !matches!(handle.status, OperationStatus::Running);
    let result = Json(json!({
        "id": handle.id,
        "prompt": handle.prompt,
        "status": status.as_str(),
        "verdict": ended.then_some(status),
        "success": handle.result.as_ref().map(|_| status == RunState::Succeeded),
        "output_text": handle.result.as_ref().and_then(|result| result.output_text.clone()),
        "error": error,
        "finished": handle.handle.is_finished(),
    }));
    drop(runs);

    Ok(result)
}

/// How a background run executes its prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunMode {
    /// One model call whose answer no gate checks, as an agent's chat reply
    /// is: its verdict comes from its result (G42).
    Answer,
    /// A gated one-task plan through the Graph engine, as `roko run` runs a
    /// prompt (9113): its verdict is the plan's.
    GatedPlan,
}

/// Spawn a background run that answers `prompt` with one model call, as an
/// agent's chat reply does, and return its id.
pub(crate) async fn spawn_background_run(
    state: &Arc<AppState>,
    prompt: String,
    workdir: Option<PathBuf>,
    agent_target: Option<String>,
) -> String {
    spawn_run(
        state,
        prompt,
        workdir,
        agent_target,
        RunMode::Answer,
        PromptPlanOptions::default(),
    )
    .await
}

/// A gated prompt run [`start_gated_run`] started or queued.
pub(crate) struct StartedPromptRun {
    /// The id it runs under.
    pub(crate) run_id: String,
    /// Its place in the run queue, 1 being next, while it waits for the live
    /// run to end (gap-9a8d66).
    pub(crate) queued: Option<usize>,
}

/// Start a gated prompt run (9113) with `options` and return the id it runs
/// under: the route of `POST /api/run` and of the MCP `run_prompt` tool.
///
/// One plan executor runs at a time. While another run of this server holds
/// the workspace, the run waits in the run queue with plan runs, oldest first,
/// and starts when the runs before it end (decision 9105, gap-9a8d66): it never
/// fails because the workspace is busy after its id has been handed out. A full
/// queue, or a run outside this server holding the workspace (`roko plan run`
/// in a terminal), refuses it with 409 before it gets an id.
pub(crate) async fn start_gated_run(
    state: &Arc<AppState>,
    prompt: String,
    workdir: Option<PathBuf>,
    options: PromptPlanOptions,
) -> Result<StartedPromptRun, ApiError> {
    // Lock both run maps once, in the lock order, to check and start or queue
    // in one step. A queue left waiting with no live run starts first, and
    // this run goes behind it.
    let mut runs = state.active_runs.write().await;
    let mut plans = state.active_plans.write().await;
    super::plans::start_queued_run(state, &mut runs, &mut plans);
    let Some(live) = super::plans::workspace_busy(&runs, &plans) else {
        let run_workdir = workdir.clone().unwrap_or_else(|| state.workdir.clone());
        if let Some(holder) = state.runtime.runner_lock_holder(&run_workdir) {
            return Err(ApiError::conflict(format!(
                "a run outside this server holds the workspace ({holder}); start this run once \
                 it ends"
            )));
        }
        let (run_id, handle, start) = prepare_run(
            state,
            None,
            prompt,
            workdir,
            None,
            RunMode::GatedPlan,
            options,
        );
        runs.insert(run_id.clone(), handle);
        let _ = start.send(());
        return Ok(StartedPromptRun {
            run_id,
            queued: None,
        });
    };
    let queued = state.queued_runs();
    if queued >= PLAN_RUN_QUEUE_CAPACITY {
        return Err(live.conflict(queued));
    }
    let run_id = uuid::Uuid::new_v4().to_string();
    state
        .prompt_queue
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push_back(QueuedPromptRun {
            run_id: run_id.clone(),
            prompt,
            workdir,
            options,
            queued_at: chrono::Utc::now(),
        });
    drop((plans, runs));
    let position = state.queue_position(&run_id).or(Some(queued + 1));
    Ok(StartedPromptRun {
        run_id,
        queued: position,
    })
}

/// Start the queued prompt run `queued` under the id it was given (gap-9a8d66):
/// `runs` is the locked `active_runs` map, which holds no live run.
pub(crate) fn launch_queued_prompt_run(
    state: &Arc<AppState>,
    runs: &mut std::collections::HashMap<String, RunHandle>,
    queued: QueuedPromptRun,
) {
    let (run_id, handle, start) = prepare_run(
        state,
        Some(queued.run_id),
        queued.prompt,
        queued.workdir,
        None,
        RunMode::GatedPlan,
        queued.options,
    );
    runs.insert(run_id, handle);
    let _ = start.send(());
}

/// Take the queued prompt run `run_id` out of the run queue (`run_cancel`,
/// gap-9a8d66). Its handle records it as cancelled, so its status answers;
/// the run never started, so the handle's task only stands in, and its index
/// gets the row that says how it ended, so its summary answers once the handle
/// is gone. `false` when no queued prompt run has that id.
pub(crate) async fn cancel_queued_prompt_run(state: &Arc<AppState>, run_id: &str) -> bool {
    let mut runs = state.active_runs.write().await;
    let removed = {
        let mut queue = state
            .prompt_queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let index = queue.iter().position(|queued| queued.run_id == run_id);
        index.and_then(|index| queue.remove(index))
    };
    let Some(queued) = removed else {
        return false;
    };
    runs.insert(
        queued.run_id.clone(),
        RunHandle {
            id: queued.run_id.clone(),
            prompt: queued.prompt,
            status: OperationStatus::Completed { result: None },
            result: None,
            verdict: Some(RunState::Cancelled),
            cancel: None,
            handle: tokio::spawn(async {}),
        },
    );
    drop(runs);
    record_unstarted_run(state, run_id, RunState::Cancelled, None);
    publish_run_completed(
        &state.event_bus,
        run_id,
        None,
        RunState::Cancelled,
        Some(json!({ "error": "cancelled while it waited in the run queue" })),
    );
    true
}

/// Write the rows that say how run `run_id` ended into its own index, for a
/// run that never started (gap-9a8d66): an `error` row when `error` says why,
/// and its `run_completed`. Nothing is published on the state hub, whose
/// events another run's index takes (bug-09e21e).
fn record_unstarted_run(state: &AppState, run_id: &str, outcome: RunState, error: Option<&str>) {
    use roko_core::DashboardEvent;
    use std::io::Write as _;

    let now = run_now_millis();
    let mut events = Vec::new();
    if let Some(message) = error {
        events.push(DashboardEvent::Error {
            message: message.to_string(),
        });
    }
    events.push(DashboardEvent::RunCompleted {
        run_id: Some(run_id.to_string()),
        outcome: outcome.as_str().to_string(),
        duration_ms: 0,
        cleanup_degraded: false,
        surviving_agent_ids: Vec::new(),
        surviving_agent_pids: Vec::new(),
    });
    // The rows a run's own event log writes: the event, its run and its time.
    let rows = events.iter().map(|event| {
        let mut row = serde_json::to_value(event).unwrap_or_default();
        if let Some(fields) = row.as_object_mut() {
            fields.insert("run_id".into(), json!(run_id));
            fields.insert("ts_millis".into(), json!(now));
        }
        row
    });
    let written =
        roko_fs::run_index::open_run_index_append(&state.layout.events_jsonl_path(), run_id)
            .and_then(|(_, mut file)| {
                for row in rows {
                    writeln!(file, "{row}")?;
                }
                file.sync_data()
            });
    if let Err(error) = written {
        tracing::warn!(%run_id, %error, "could not record how the run ended");
    }
}

/// Stop background run `run_id` when it is a gated prompt run still going
/// (`run_cancel`, 9115). `None` when no background run has that id;
/// otherwise whether it was stopped, or why not.
pub(crate) async fn cancel_background_run(
    state: &AppState,
    run_id: &str,
) -> Option<Result<(), &'static str>> {
    let runs = state.active_runs.read().await;
    let run = runs.get(run_id)?;
    if !matches!(run.status, OperationStatus::Running) {
        return Some(Err("the run has already ended"));
    }
    let Some(cancel) = &run.cancel else {
        return Some(Err("an agent's reply cannot be cancelled"));
    };
    cancel.cancel();
    Some(Ok(()))
}

/// Run `prompt` as `mode` says: the run's verdict and result. A gated run
/// takes `options`; an answer needs none.
async fn execute_run(
    runtime: &dyn CliRuntime,
    workdir: &std::path::Path,
    prompt: &str,
    mode: RunMode,
    options: PromptPlanOptions,
) -> anyhow::Result<(RunState, RunResult)> {
    match mode {
        RunMode::Answer => {
            let result = runtime.run_once(workdir, prompt).await?;
            Ok((result.verdict(), result))
        }
        RunMode::GatedPlan => {
            let plan = runtime.run_prompt_plan(workdir, prompt, options).await?;
            let result = RunResult {
                success: plan.success,
                output_text: plan.output_text,
                usage: None,
                gate_results: Vec::new(),
            };
            Ok((plan.verdict, result))
        }
    }
}

/// Spawn a background run of `prompt` in `mode` and return its id: the id it
/// runs under. A gated run takes `options` and stops when the server shuts
/// down or `run_cancel` stops it.
async fn spawn_run(
    state: &Arc<AppState>,
    prompt: String,
    workdir: Option<PathBuf>,
    agent_target: Option<String>,
    mode: RunMode,
    options: PromptPlanOptions,
) -> String {
    let (run_id, handle, start) =
        prepare_run(state, None, prompt, workdir, agent_target, mode, options);
    state
        .active_runs
        .write()
        .await
        .insert(run_id.clone(), handle);
    let _ = start.send(());
    run_id
}

/// The task of a background run of `prompt` in `mode`, under `run_id` or a new
/// id: its id, its handle for `active_runs`, and the signal that starts it once
/// the handle is in place. A gated run starts the next queued run when it ends
/// (gap-9a8d66).
fn prepare_run(
    state: &Arc<AppState>,
    run_id: Option<String>,
    prompt: String,
    workdir: Option<PathBuf>,
    agent_target: Option<String>,
    mode: RunMode,
    options: PromptPlanOptions,
) -> (String, RunHandle, oneshot::Sender<()>) {
    let run_id = run_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let workdir = workdir.unwrap_or_else(|| state.workdir.clone());
    let bus = state.event_bus.clone();
    let runtime = state.runtime.clone();
    let state_for_task = Arc::clone(state);
    let (start_tx, start_rx) = oneshot::channel::<()>();
    let cancel = (mode == RunMode::GatedPlan).then(|| state.cancel.child());
    let cancel_for_task = cancel.clone();
    let live_agent_output = state.effective_live_agent_output();
    // A gated run's start event says where its request came from (9116).
    let origin = (mode == RunMode::GatedPlan).then(|| options.origin.clone());

    let handle = tokio::spawn({
        let run_id = run_id.clone();
        let prompt_for_handle = prompt.clone();
        async move {
            let _ = start_rx.await;
            publish_run_started(
                &bus,
                &run_id,
                &prompt_for_handle,
                agent_target.as_deref(),
                origin,
            );

            // Emit rich DashboardEvents so the TUI shows run activity. The
            // plan is the one `RunStarted` and `RunCompleted` start and end.
            let plan_id = crate::run_plan_id(&run_id);
            // The hub keeps every event under `.roko/`, so the prompt that
            // names the task is scrubbed before it is cut short.
            let task_id: String = state_for_task
                .scrubber
                .scrub(&prompt_for_handle)
                .chars()
                .take(60)
                .collect();
            let agent_label = agent_target.as_deref().unwrap_or("claude");
            {
                use roko_core::DashboardEvent;
                state_for_task.state_hub.publish_batch(vec![
                    DashboardEvent::TaskStarted {
                        plan_id: plan_id.clone(),
                        task_id: task_id.clone(),
                        title: String::new(),
                        phase: "implementing".into(),
                    },
                    DashboardEvent::AgentSpawned {
                        agent_id: agent_label.to_string(),
                        plan_id: plan_id.clone(),
                        task_id: task_id.clone(),
                        attempt: 0,
                        role: "run".into(),
                        model: agent_label.to_string(),
                        provider: String::new(),
                    },
                    DashboardEvent::EventLogEntry {
                        timestamp_ms: run_now_millis(),
                        event_type: "run_started".into(),
                        plan_id: plan_id.clone(),
                        task_id: task_id.clone(),
                        message: format!("▶ {agent_label}: {task_id}"),
                    },
                ]);
            }

            // A gated run takes the id this route returns, and publishes the
            // live agent output a plan run of this server does (gap-5fda00).
            let options = PromptPlanOptions {
                run_id: Some(run_id.clone()),
                cancel: cancel_for_task,
                live_agent_output: Some(live_agent_output),
                ..options
            };
            let run = execute_run(
                runtime.as_ref(),
                workdir.as_path(),
                &prompt_for_handle,
                mode,
                options,
            );
            let hub = &state_for_task.state_hub;
            match run_with_heartbeats(hub, agent_label, &plan_id, &task_id, run).await {
                Ok((verdict, result)) => {
                    record_run_result(&state_for_task, &run_id, verdict, result.clone()).await;
                    publish_run_completed(
                        &bus,
                        &run_id,
                        agent_target.as_deref(),
                        verdict,
                        result.output_text.as_ref().map(|output| {
                            json!({
                                "output_text": output,
                            })
                        }),
                    );
                    // Rich TUI events on completion
                    {
                        use roko_core::DashboardEvent;
                        use roko_core::dashboard_snapshot::{
                            TASK_OUTCOME_PASSED, TASK_OUTCOME_UNVERIFIED,
                        };
                        let (outcome, mark) = match verdict {
                            RunState::Succeeded => (TASK_OUTCOME_PASSED, "✓"),
                            RunState::Unverified => (TASK_OUTCOME_UNVERIFIED, "?"),
                            _ => ("failed", "✗"),
                        };
                        let mut events = vec![
                            DashboardEvent::TaskCompleted {
                                plan_id: plan_id.clone(),
                                task_id: task_id.clone(),
                                outcome: outcome.into(),
                            },
                            DashboardEvent::EpisodeRecorded {
                                agent_id: agent_label.to_string(),
                                role: "run".into(),
                                episode_id: run_id.clone(),
                                passed: verdict == RunState::Succeeded,
                            },
                            DashboardEvent::EventLogEntry {
                                timestamp_ms: run_now_millis(),
                                event_type: "run_completed".into(),
                                plan_id: plan_id.clone(),
                                task_id: task_id.clone(),
                                message: format!("{mark} {agent_label}: {task_id}"),
                            },
                        ];
                        if let Some(ref text) = result.output_text {
                            let preview: String = text.chars().take(200).collect();
                            events.push(DashboardEvent::AgentOutput {
                                agent_id: agent_label.to_string(),
                                plan_id: plan_id.clone(),
                                task_id: task_id.clone(),
                                attempt: 0,
                                content: preview,
                            });
                            events.push(DashboardEvent::TaskOutputAppended {
                                task_id: task_id.clone(),
                                lines: text.lines().take(10).map(String::from).collect(),
                            });
                        }
                        state_for_task.state_hub.publish_batch(events);
                    }
                }
                Err(e) => {
                    let error_message = format!("run failed: {e}");
                    record_run_failure(&state_for_task, &run_id, &error_message).await;
                    bus.publish(ServerEvent::Error {
                        message: error_message.clone(),
                    });
                    publish_run_completed(
                        &bus,
                        &run_id,
                        agent_target.as_deref(),
                        RunState::Failed,
                        Some(serde_json::json!({ "error": error_message })),
                    );
                    // Rich TUI events on failure
                    {
                        use roko_core::DashboardEvent;
                        state_for_task.state_hub.publish_batch(vec![
                            DashboardEvent::TaskCompleted {
                                plan_id: plan_id.clone(),
                                task_id: task_id.clone(),
                                outcome: "failed".into(),
                            },
                            DashboardEvent::Error {
                                message: error_message.clone(),
                            },
                            DashboardEvent::EventLogEntry {
                                timestamp_ms: run_now_millis(),
                                event_type: "run_failed".into(),
                                plan_id,
                                task_id,
                                message: format!("✗ {error_message}"),
                            },
                        ]);
                    }
                }
            }
            // The workspace is free again: the oldest queued run starts
            // (gap-9a8d66).
            if mode == RunMode::GatedPlan {
                super::plans::start_next_queued_run(&state_for_task).await;
            }
        }
    });

    let run_handle = RunHandle {
        id: run_id.clone(),
        prompt,
        status: OperationStatus::Running,
        result: None,
        verdict: None,
        cancel,
        handle,
    };
    (run_id, run_handle, start_tx)
}

/// How often a one-shot run's agent reports that it is still working.
const RUN_HEARTBEAT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

/// Drive `run` to its end, publishing a heartbeat for the run's agent every
/// [`RUN_HEARTBEAT_INTERVAL`], as a plan run's agents do, so the dashboard
/// shows how long it has worked; then publish the agent's completion
/// (gap-8a1fb3).
async fn run_with_heartbeats<T>(
    hub: &roko_runtime::SharedStateHub,
    agent_id: &str,
    plan_id: &str,
    task_id: &str,
    run: impl std::future::Future<Output = T>,
) -> T {
    use roko_core::DashboardEvent;

    let started = tokio::time::Instant::now();
    let mut heartbeat = tokio::time::interval(RUN_HEARTBEAT_INTERVAL);
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    // The first tick is immediate; the first heartbeat comes one interval in.
    heartbeat.tick().await;
    tokio::pin!(run);
    let result = loop {
        tokio::select! {
            result = &mut run => break result,
            _ = heartbeat.tick() => {
                let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
                hub.publish(DashboardEvent::AgentHeartbeat {
                    agent_id: agent_id.to_string(),
                    plan_id: plan_id.to_string(),
                    task_id: task_id.to_string(),
                    elapsed_ms,
                });
            }
        }
    };
    hub.publish(DashboardEvent::AgentCompleted {
        agent_id: agent_id.to_string(),
        plan_id: plan_id.to_string(),
        task_id: task_id.to_string(),
        attempt: 0,
    });
    result
}

async fn record_run_result(state: &AppState, run_id: &str, verdict: RunState, result: RunResult) {
    if let Some(handle) = state.active_runs.write().await.get_mut(run_id) {
        handle.status = OperationStatus::Completed {
            result: result.output_text.clone(),
        };
        handle.result = Some(result);
        handle.verdict = Some(verdict);
    }
}

async fn record_run_failure(state: &AppState, run_id: &str, error_message: &str) {
    if let Some(handle) = state.active_runs.write().await.get_mut(run_id) {
        handle.status = OperationStatus::Failed {
            error: error_message.to_string(),
        };
        handle.result = Some(RunResult {
            success: false,
            output_text: None,
            usage: None,
            gate_results: Vec::new(),
        });
    }
}

/// The state a run handle reports, with the error of a run that failed:
/// `running`, then the run's verdict once it ends: the one it recorded, else
/// the one its result gives ([`RunResult::verdict`]).
pub(crate) fn run_handle_state(handle: &RunHandle) -> (RunState, Option<&str>) {
    match &handle.status {
        OperationStatus::Running => (RunState::Running, None),
        OperationStatus::Completed { .. } => {
            let verdict = match (handle.verdict, &handle.result) {
                (Some(verdict), _) => verdict,
                (None, Some(result)) => result.verdict(),
                (None, None) => RunState::Unverified,
            };
            (verdict, None)
        }
        OperationStatus::Failed { error } => (RunState::Failed, Some(error.as_str())),
    }
}

fn publish_run_started(
    bus: &crate::event_bus::EventBus<ServerEvent>,
    run_id: &str,
    prompt: &str,
    agent_target: Option<&str>,
    origin: Option<RunOrigin>,
) {
    bus.publish(ServerEvent::RunStarted {
        run_id: run_id.to_owned(),
        prompt: prompt.to_owned(),
        origin,
    });
    if let Some(agent_id) = agent_target {
        bus.publish(ServerEvent::AgentOutput {
            agent_id: agent_id.to_owned(),
            run_id: Some(run_id.to_owned()),
            content: String::new(),
            done: false,
            metadata: Some(serde_json::json!({ "status": "started" })),
        });
    }
}

fn publish_run_completed(
    bus: &crate::event_bus::EventBus<ServerEvent>,
    run_id: &str,
    agent_target: Option<&str>,
    verdict: RunState,
    metadata: Option<Value>,
) {
    let success = verdict == RunState::Succeeded;
    if let Some(agent_id) = agent_target {
        let raw_content = metadata
            .as_ref()
            .and_then(|value| value.get("output_text"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let clean_content = sanitize_agent_content(&raw_content);
        bus.publish(ServerEvent::AgentOutput {
            agent_id: agent_id.to_owned(),
            run_id: Some(run_id.to_owned()),
            content: clean_content,
            done: true,
            metadata: Some(serde_json::json!({
                "status": verdict.as_str(),
                "success": success,
                "details": metadata.clone().unwrap_or(Value::Null),
            })),
        });
        // Emit raw trace for debug subscribers.
        bus.publish(ServerEvent::AgentTrace {
            agent_id: agent_id.to_owned(),
            run_id: Some(run_id.to_owned()),
            content: raw_content,
            tool_calls: None,
            reasoning: None,
            usage: None,
            done: true,
        });
    }

    bus.publish(ServerEvent::RunCompleted {
        run_id: run_id.to_owned(),
        success,
        verdict: Some(verdict),
    });
}

/// P1-44: `POST /api/surface-events` -- accept a `SurfaceEvent` command and
/// translate it into runtime effects.
///
/// Currently handles `FlowCancel` and `FlowPause` (plan cancellation/pause).
/// Other variants are accepted and logged but do not yet trigger effects.
async fn handle_surface_event(
    State(state): State<Arc<AppState>>,
    Json(event): Json<roko_core::runtime_event::SurfaceEvent>,
) -> Result<impl IntoResponse, ApiError> {
    use roko_core::runtime_event::SurfaceEvent;

    tracing::info!(?event, "P1-44: surface event received");

    match &event {
        SurfaceEvent::FlowCancel { run_id } | SurfaceEvent::FlowPause { run_id } => {
            let action = if matches!(event, SurfaceEvent::FlowCancel { .. }) {
                "flow_cancel"
            } else {
                "flow_pause"
            };
            let plans = state.active_plans.read().await;
            if let Some(handle) = plans.get(run_id.as_str()) {
                handle.cancel.cancel();
                tracing::info!(run_id, action, "P1-44: flow control via surface event");
                Ok(Json(
                    json!({ "ok": true, "action": action, "run_id": run_id }),
                ))
            } else {
                drop(plans);
                Err(ApiError::not_found(format!("run {run_id} not found")))
            }
        }
        SurfaceEvent::HumanRespond {
            run_id, cell_id, ..
        } => {
            tracing::info!(
                run_id,
                cell_id,
                "P1-44: human response received (effect dispatch pending full wiring)"
            );
            Ok(Json(json!({
                "ok": true,
                "action": "human_respond",
                "run_id": run_id,
                "cell_id": cell_id,
                "wired": false,
            })))
        }
        _ => {
            // Accept but log other surface event types as not-yet-wired.
            tracing::debug!(
                ?event,
                "P1-44: surface event type accepted but not yet wired"
            );
            Ok(Json(
                json!({ "ok": true, "action": "accepted", "wired": false }),
            ))
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
fn run_now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::DashboardEvent;

    /// A key planted among the process's secrets, as a `.env` file plants one.
    const PLANTED_KEY: &str = "canary-serve-6f2c9a41d8";

    /// Puts a process secret scrubber that knows the planted key in place,
    /// and the previous one back when dropped.
    struct PlantedKeyScrubber(Option<Arc<roko_core::obs::LogScrubber>>);

    impl PlantedKeyScrubber {
        fn install() -> Self {
            let scrubber = roko_core::obs::LogScrubber::empty();
            scrubber
                .add_literal_value(PLANTED_KEY, "PLANTED_KEY")
                .expect("register the planted key");
            let previous = roko_core::obs::install_secret_scrubber(Some(Arc::new(scrubber)));
            Self(previous)
        }
    }

    impl Drop for PlantedKeyScrubber {
        fn drop(&mut self) {
            roko_core::obs::install_secret_scrubber(self.0.take());
        }
    }

    /// A runtime whose agent prints the planted key.
    struct PrintsPlantedKey;

    #[async_trait::async_trait]
    impl crate::runtime::CliRuntime for PrintsPlantedKey {
        async fn run_once(
            &self,
            _workdir: &std::path::Path,
            _prompt: &str,
        ) -> anyhow::Result<RunResult> {
            Ok(RunResult {
                success: true,
                output_text: Some(format!("Deployed.\nThe deploy key is {PLANTED_KEY}.\n")),
                usage: None,
                gate_results: Vec::new(),
            })
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

    /// Every regular file under `root`, at any depth.
    fn files_under(root: &std::path::Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        let mut dirs = vec![root.to_path_buf()];
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(&dir).expect("read a directory").flatten() {
                let path = entry.path();
                if path.is_dir() {
                    dirs.push(path);
                } else if path.is_file() {
                    files.push(path);
                }
            }
        }
        files
    }

    /// bug-a9788a: what serve keeps under `.roko/` is scrubbed of the
    /// process's secrets, as the CLI's event log is (the C2 canary). A
    /// serve-hosted run whose agent prints a planted key, the stream record a
    /// hosted plan run's agent publishes and an agent's output ingested as a
    /// runtime event leave the key nowhere under `.roko/`: serve's event logs
    /// hold it redacted.
    #[tokio::test]
    async fn serve_event_log_is_scrubbed() {
        let _scrubber = PlantedKeyScrubber::install();
        let dir = tempfile::tempdir().expect("tempdir");
        let deploy_backend = Arc::from(
            crate::deploy::create_backend("manual", None, None, None).expect("manual backend"),
        );
        let state = Arc::new(
            AppState::new(
                dir.path().to_path_buf(),
                Arc::new(PrintsPlantedKey),
                roko_core::config::schema::RokoConfig::default(),
                deploy_backend,
            )
            .expect("AppState::new"),
        );

        let run_id = spawn_background_run(&state, "deploy the site".into(), None, None).await;
        let run = state
            .active_runs
            .write()
            .await
            .remove(&run_id)
            .expect("the run is tracked");
        tokio::time::timeout(std::time::Duration::from_secs(10), run.handle)
            .await
            .expect("the run ends in time")
            .expect("the run's task");
        state.state_hub.publish(DashboardEvent::AgentOutput {
            agent_id: "plan-a/T1".into(),
            plan_id: "plan-a".into(),
            task_id: "T1".into(),
            attempt: 1,
            content: format!(
                "\u{1e}roko.stream.v1 {}",
                json!({ "kind": "tool_result", "payload": { "output": PLANTED_KEY } })
            ),
        });
        state
            .runtime_event_logger
            .consume_with_run_cursor(&roko_core::RuntimeEvent::AgentOutput {
                run_id: "run-1".into(),
                agent_id: "worker".into(),
                chunk: format!("the key is {PLANTED_KEY}"),
            });

        let roko_dir = dir.path().join(".roko");
        let events = std::fs::read_to_string(roko_dir.join("events.jsonl")).expect("the event log");
        assert!(events.contains("[REDACTED:PLANTED_KEY]"), "{events}");
        let runtime_events = std::fs::read_to_string(state.runtime_event_logger.path())
            .expect("the runtime event log");
        assert!(
            runtime_events.contains("[REDACTED:PLANTED_KEY]"),
            "{runtime_events}"
        );
        for path in files_under(&roko_dir) {
            let bytes = std::fs::read(&path).expect("read a file");
            assert!(
                !String::from_utf8_lossy(&bytes).contains(PLANTED_KEY),
                "{} holds the planted key",
                path.display()
            );
        }
    }

    /// Server state over `runtime` in a fresh workspace.
    fn state_over(runtime: Arc<dyn CliRuntime>) -> (tempfile::TempDir, Arc<AppState>) {
        let dir = tempfile::tempdir().expect("tempdir");
        let deploy_backend = Arc::from(
            crate::deploy::create_backend("manual", None, None, None).expect("manual backend"),
        );
        let state = Arc::new(
            AppState::new(
                dir.path().to_path_buf(),
                runtime,
                roko_core::config::schema::RokoConfig::default(),
                deploy_backend,
            )
            .expect("AppState::new"),
        );
        (dir, state)
    }

    /// What `GET /api/run/{id}/status` reports once run `run_id` has ended,
    /// waiting at most ten seconds for it to end.
    async fn ended_run_status(state: &Arc<AppState>, run_id: &str) -> Value {
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let Json(status) = run_status(State(Arc::clone(state)), Path(run_id.to_string()))
                    .await
                    .expect("the run is tracked");
                if status["finished"] == true {
                    break status;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("the run ends in time")
    }

    /// G42: a run nothing checked is not a success. With a runtime that says
    /// it succeeded but returns no gate results, as an agent's one-call reply
    /// does, `GET /api/run/{id}/status` reports `unverified` with `success:
    /// false`, and the run's completion event carries the same verdict.
    #[tokio::test]
    async fn api_run_without_gates_reports_unverified() {
        let (_dir, state) = state_over(Arc::new(crate::runtime::NoOpRuntime));

        let run_id = spawn_background_run(&state, "say hi".into(), None, None).await;
        let status = ended_run_status(&state, &run_id).await;

        assert_eq!(status["status"], "unverified", "{status}");
        assert_eq!(status["verdict"], "unverified", "{status}");
        assert_eq!(status["success"], false, "{status}");
        let completed = state
            .event_bus
            .replay_from(0)
            .into_iter()
            .find_map(|envelope| match envelope.payload {
                ServerEvent::RunCompleted {
                    success, verdict, ..
                } => Some((success, verdict)),
                _ => None,
            })
            .expect("the run's completion event");
        assert_eq!(completed, (false, Some(RunState::Unverified)));
    }

    /// A runtime whose prompt runs end as gated plans that succeeded, which
    /// records the run id each ran under.
    #[derive(Default)]
    struct GatedPrompts {
        run_ids: std::sync::Mutex<Vec<Option<String>>>,
    }

    #[async_trait::async_trait]
    impl crate::runtime::CliRuntime for GatedPrompts {
        async fn run_once(
            &self,
            _workdir: &std::path::Path,
            _prompt: &str,
        ) -> anyhow::Result<RunResult> {
            anyhow::bail!("POST /api/run must not answer with one model call")
        }

        async fn run_prompt_plan(
            &self,
            _workdir: &std::path::Path,
            _prompt: &str,
            options: PromptPlanOptions,
        ) -> anyhow::Result<crate::runtime::PromptPlanResult> {
            let run_id = options.run_id.clone().unwrap_or_default();
            self.run_ids
                .lock()
                .expect("lock run ids")
                .push(options.run_id);
            Ok(crate::runtime::PromptPlanResult {
                run_id,
                verdict: RunState::Succeeded,
                success: true,
                output_text: Some("Done.".to_string()),
                cost_usd: Some(0.02),
            })
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

    /// 9113: `POST /api/run` runs the prompt as a gated one-task plan under the
    /// id it returns. The runtime runs it under that id, and the run's status
    /// carries the plan's verdict.
    #[tokio::test]
    async fn api_run_reports_the_gated_runs_id_and_verdict() {
        let runtime = Arc::new(GatedPrompts::default());
        let (_dir, state) = state_over(Arc::clone(&runtime) as Arc<dyn CliRuntime>);

        let request = RunRequest {
            prompt: "add a test for the parser".into(),
            workdir: None,
            domain: None,
        };
        let response = start_run(State(Arc::clone(&state)), ValidJson(request))
            .await
            .expect("start the run")
            .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::ACCEPTED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("the response body");
        let body: Value = serde_json::from_slice(&body).expect("a JSON body");
        let run_id = body["id"].as_str().expect("the run's id").to_string();

        let status = ended_run_status(&state, &run_id).await;
        assert_eq!(status["status"], "succeeded", "{status}");
        assert_eq!(status["verdict"], "succeeded", "{status}");
        assert_eq!(status["success"], true, "{status}");
        assert_eq!(status["output_text"], "Done.", "{status}");
        let ran = runtime.run_ids.lock().expect("lock run ids").clone();
        assert_eq!(ran, [Some(run_id)]);
    }

    /// gap-9a8d66, gap-5e645b: while a plan run holds the workspace, `POST /api/run`
    /// queues the prompt run behind it instead of refusing it: the 202 says
    /// so, with the run's place, its status answers `queued`, and it starts
    /// under the id it was handed once the plan run ends.
    #[tokio::test]
    async fn prompt_run_is_queued_not_refused_while_a_plan_runs() {
        let runtime = Arc::new(GatedPrompts::default());
        let (_dir, state) = state_over(Arc::clone(&runtime) as Arc<dyn CliRuntime>);
        let plan_run = crate::state::PlanHandle {
            id: "run-1".into(),
            plan_dir: state.workdir.join("plans").join("live"),
            members: vec!["live".into()],
            status: crate::state::PlanRunStatus::running(),
            handle: tokio::spawn(tokio::time::sleep(std::time::Duration::from_secs(30))),
            cancel: roko_runtime::cancel::CancelToken::new(),
        };
        state
            .active_plans
            .write()
            .await
            .insert("live".into(), plan_run);

        let request = RunRequest {
            prompt: "add a test".into(),
            workdir: None,
            domain: None,
        };
        let response = start_run(State(Arc::clone(&state)), ValidJson(request))
            .await
            .expect("the run is queued")
            .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::ACCEPTED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("the response body");
        let body: Value = serde_json::from_slice(&body).expect("a JSON body");
        assert_eq!(body["queued"], true, "{body}");
        assert_eq!(body["position"], 1, "{body}");
        let run_id = body["id"].as_str().expect("the run's id").to_string();
        assert!(state.active_runs.read().await.is_empty());
        let Json(status) = run_status(State(Arc::clone(&state)), Path(run_id.clone()))
            .await
            .expect("a queued run has a status");
        assert_eq!(status["status"], "queued", "{status}");
        assert_eq!(status["position"], 1, "{status}");

        // The plan run ends: the queued prompt run starts under its id.
        if let Some(plan) = state.active_plans.write().await.get_mut("live") {
            plan.status = crate::state::PlanRunStatus::ended(RunState::Succeeded, None);
        }
        super::super::plans::start_next_queued_run(&state).await;
        let status = ended_run_status(&state, &run_id).await;
        assert_eq!(status["status"], "succeeded", "{status}");
        let ran = runtime.run_ids.lock().expect("lock run ids").clone();
        assert_eq!(ran, [Some(run_id)]);
    }

    /// gap-8a1fb3: a one-shot run's agent reports how long it has worked while
    /// the run goes on, as a plan run's agents do, and completes with it.
    #[tokio::test(start_paused = true)]
    async fn one_shot_run_agent_beats_while_it_works_then_completes() {
        let hub = roko_runtime::SharedStateHub::new_in_process();
        hub.publish(DashboardEvent::AgentSpawned {
            agent_id: "claude".into(),
            plan_id: "run-0123abcd".into(),
            task_id: "say hi".into(),
            attempt: 0,
            role: "run".into(),
            model: "claude".into(),
            provider: String::new(),
        });
        let work = tokio::time::sleep(std::time::Duration::from_secs(12));
        run_with_heartbeats(&hub, "claude", "run-0123abcd", "say hi", work).await;

        let beats: Vec<u64> = hub
            .subscribe_events_from(0)
            .replay
            .iter()
            .filter_map(|envelope| match &envelope.payload {
                DashboardEvent::AgentHeartbeat { elapsed_ms, .. } => Some(*elapsed_ms),
                _ => None,
            })
            .collect();
        assert_eq!(beats, [5_000, 10_000]);
        let snapshot = hub.current_snapshot();
        let agent = snapshot.agents.get("claude").expect("the run's agent");
        assert!(!agent.active, "the agent completes with the run");
        assert_eq!(agent.elapsed_ms, 10_000);
    }
}
