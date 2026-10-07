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
use crate::state::{AppState, OperationStatus, RunHandle, RunState};

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
/// One plan executor runs at a time, and prompt runs are not queued behind
/// plan runs yet: while a plan run is live the request is refused with 409
/// instead of waiting for the workspace.
async fn start_run(
    State(state): State<Arc<AppState>>,
    ValidJson(body): ValidJson<RunRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let domain = body.domain.as_deref().and_then(TaskDomain::from_label);
    let options = PromptPlanOptions {
        domain,
        ..PromptPlanOptions::default()
    };
    let run_id = start_gated_run(
        &state,
        body.prompt.clone(),
        body.workdir.map(PathBuf::from),
        options,
    )
    .await?;

    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(json!({ "id": run_id })),
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
    let handle = runs
        .get(&id)
        .ok_or_else(|| ApiError::not_found("run not found"))?;

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

/// Start a gated prompt run (9113) with `options` and return the id it runs
/// under: the route of `POST /api/run` and of the MCP `run_prompt` tool.
///
/// One plan executor runs at a time, and prompt runs are not queued behind
/// plan runs yet: while a plan run is live this refuses with 409 instead of
/// waiting for the workspace.
pub(crate) async fn start_gated_run(
    state: &Arc<AppState>,
    prompt: String,
    workdir: Option<PathBuf>,
    options: PromptPlanOptions,
) -> Result<String, ApiError> {
    if state.live_plan_runs().await > 0 {
        return Err(ApiError::conflict(
            "a plan run is active in this workspace; start the prompt run once it ends",
        ));
    }
    let run_id = spawn_run(state, prompt, workdir, None, RunMode::GatedPlan, options).await;
    Ok(run_id)
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
    let run_id = uuid::Uuid::new_v4().to_string();
    let workdir = workdir.unwrap_or_else(|| state.workdir.clone());
    let bus = state.event_bus.clone();
    let runtime = state.runtime.clone();
    let state_for_task = Arc::clone(state);
    let (start_tx, start_rx) = oneshot::channel::<()>();
    let cancel = (mode == RunMode::GatedPlan).then(|| state.cancel.child());
    let cancel_for_task = cancel.clone();
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

            // A gated run takes the id this route returns.
            let options = PromptPlanOptions {
                run_id: Some(run_id.clone()),
                cancel: cancel_for_task,
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

    state
        .active_runs
        .write()
        .await
        .insert(run_id.clone(), run_handle);
    let _ = start_tx.send(());
    run_id
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

    /// 9113: prompt runs are not queued behind plan runs yet, so `POST
    /// /api/run` refuses with 409 while a plan run is live instead of waiting.
    #[tokio::test]
    async fn api_run_is_refused_while_a_plan_run_is_live() {
        let (_dir, state) = state_over(Arc::new(GatedPrompts::default()));
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
        let err = match start_run(State(Arc::clone(&state)), ValidJson(request)).await {
            Ok(_) => panic!("a live plan run must refuse the prompt run"),
            Err(err) => err,
        };
        assert_eq!(err.status, axum::http::StatusCode::CONFLICT);
        assert!(state.active_runs.read().await.is_empty());
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
