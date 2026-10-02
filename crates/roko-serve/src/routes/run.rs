//! Single-prompt run endpoints.

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::oneshot;
use validator::Validate;

use crate::error::ApiError;
use crate::events::ServerEvent;
use crate::extract::{RequestPayload, ValidJson, validate_with_validator};
use crate::runtime::RunResult;
use crate::sanitize::sanitize_agent_content;
use crate::state::{AppState, OperationStatus, RunHandle};

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
}

impl RequestPayload for RunRequest {
    fn validate_payload(&self) -> Result<(), ApiError> {
        validate_with_validator(self)
    }
}

/// `POST /api/run` — spawn a background `run_once()` invocation.
async fn start_run(
    State(state): State<Arc<AppState>>,
    ValidJson(body): ValidJson<RunRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let run_id = spawn_background_run(
        &state,
        body.prompt.clone(),
        body.workdir.map(PathBuf::from),
        None,
    )
    .await;

    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(json!({ "id": run_id })),
    ))
}

/// `GET /api/run/:id/status` — check the status of a background run.
async fn run_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let runs = state.active_runs.read().await;
    let handle = runs
        .get(&id)
        .ok_or_else(|| ApiError::not_found("run not found"))?;

    let (status, error) = operation_status_parts(&handle.status);
    let result = Json(json!({
        "id": handle.id,
        "prompt": handle.prompt,
        "status": status,
        "success": handle.result.as_ref().map(|result| result.success),
        "output_text": handle.result.as_ref().and_then(|result| result.output_text.clone()),
        "error": error,
        "finished": handle.handle.is_finished(),
    }));
    drop(runs);

    Ok(result)
}

pub(crate) async fn spawn_background_run(
    state: &Arc<AppState>,
    prompt: String,
    workdir: Option<PathBuf>,
    agent_target: Option<String>,
) -> String {
    let run_id = uuid::Uuid::new_v4().to_string();
    let workdir = workdir.unwrap_or_else(|| state.workdir.clone());
    let bus = state.event_bus.clone();
    let runtime = state.runtime.clone();
    let state_for_task = Arc::clone(state);
    let (start_tx, start_rx) = oneshot::channel::<()>();

    let handle = tokio::spawn({
        let run_id = run_id.clone();
        let prompt_for_handle = prompt.clone();
        async move {
            let _ = start_rx.await;
            publish_run_started(&bus, &run_id, &prompt_for_handle, agent_target.as_deref());

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

            let run = runtime.run_once(workdir.as_path(), &prompt_for_handle);
            let hub = &state_for_task.state_hub;
            match run_with_heartbeats(hub, agent_label, &plan_id, &task_id, run).await {
                Ok(result) => {
                    record_run_result(&state_for_task, &run_id, result.clone()).await;
                    publish_run_completed(
                        &bus,
                        &run_id,
                        agent_target.as_deref(),
                        result.success,
                        result.output_text.as_ref().map(|output| {
                            json!({
                                "output_text": output,
                            })
                        }),
                    );
                    // Rich TUI events on success
                    {
                        use roko_core::DashboardEvent;
                        let mut events = vec![
                            DashboardEvent::TaskCompleted {
                                plan_id: plan_id.clone(),
                                task_id: task_id.clone(),
                                outcome: if result.success {
                                    "success".into()
                                } else {
                                    "failed".into()
                                },
                            },
                            DashboardEvent::EpisodeRecorded {
                                agent_id: agent_label.to_string(),
                                role: "run".into(),
                                episode_id: run_id.clone(),
                                passed: result.success,
                            },
                            DashboardEvent::EventLogEntry {
                                timestamp_ms: run_now_millis(),
                                event_type: "run_completed".into(),
                                plan_id: plan_id.clone(),
                                task_id: task_id.clone(),
                                message: format!(
                                    "{} {agent_label}: {task_id}",
                                    if result.success { "✓" } else { "✗" }
                                ),
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
                        false,
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

async fn record_run_result(state: &AppState, run_id: &str, result: RunResult) {
    if let Some(handle) = state.active_runs.write().await.get_mut(run_id) {
        handle.status = OperationStatus::Completed {
            result: result.output_text.clone(),
        };
        handle.result = Some(result);
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

fn operation_status_parts(status: &OperationStatus) -> (&'static str, Option<&str>) {
    match status {
        OperationStatus::Running => ("running", None),
        OperationStatus::Completed { .. } => ("completed", None),
        OperationStatus::Failed { error } => ("failed", Some(error.as_str())),
    }
}

fn publish_run_started(
    bus: &crate::event_bus::EventBus<ServerEvent>,
    run_id: &str,
    prompt: &str,
    agent_target: Option<&str>,
) {
    bus.publish(ServerEvent::RunStarted {
        run_id: run_id.to_owned(),
        prompt: prompt.to_owned(),
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
    success: bool,
    metadata: Option<Value>,
) {
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
                "status": if success { "completed" } else { "failed" },
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
