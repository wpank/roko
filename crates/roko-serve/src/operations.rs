//! Background operations listed by `GET /api/operations/{id}` (bug-a0f01e).
//!
//! [`spawn_operation`] starts one. It registers the handle in
//! `AppState.operations` before the work can finish, records how the work
//! ended (`Completed` or `Failed`; a panic counts as failed), and only then
//! publishes `OperationCompleted`, so a client reacting to that event never
//! reads a stale `running` status.
//!
//! [`finish_operation`] records how any operation ended, in its handle and in
//! `.roko/state/operations.jsonl`, which keeps the newest
//! [`FINISHED_OPERATIONS_KEPT`] finished operations. The handle is collected
//! a minute after its work ends and lost on a restart; the kept record is not,
//! so the status routes still say how the operation ended
//! ([`finished_operation`]).

use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures::FutureExt;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::events::ServerEvent;
use crate::runtime::RunResult;
use crate::state::{AppState, OperationHandle, OperationStatus};

/// How a background operation ended: an optional JSON result, or an error.
pub(crate) type OperationOutcome = Result<Option<Value>, String>;

/// Run `work` in the background as operation `id`, listed under `kind`.
///
/// The handle is inserted while the operations lock is held, so `work` cannot
/// finish before it is registered. When `work` returns, the operation becomes
/// `Completed { result }` or `Failed { error }`, and `OperationCompleted` is
/// published with `event_kind`.
pub(crate) async fn spawn_operation<F>(
    state: &Arc<AppState>,
    id: String,
    kind: String,
    event_kind: &'static str,
    work: F,
) where
    F: Future<Output = OperationOutcome> + Send + 'static,
{
    let task_state = Arc::clone(state);
    let task_id = id.clone();
    let mut operations = state.operations.write().await;
    let handle = tokio::spawn(async move {
        let outcome = AssertUnwindSafe(work)
            .catch_unwind()
            .await
            .unwrap_or_else(|_| Err(format!("{event_kind} operation panicked")));
        let success = outcome.is_ok();
        record_outcome(&task_state, &task_id, outcome).await;
        task_state
            .event_bus
            .publish(ServerEvent::OperationCompleted {
                op_id: task_id,
                kind: event_kind.into(),
                success,
            });
    });
    operations.insert(
        id.clone(),
        OperationHandle {
            id,
            kind,
            status: OperationStatus::Running,
            handle,
            started: std::time::Instant::now(),
            progress: None,
        },
    );
}

/// Outcome of an operation whose work is one runtime run: it completes when
/// the run succeeds, and fails with "`what` did not succeed" otherwise.
pub(crate) fn run_outcome(result: &RunResult, what: &str) -> OperationOutcome {
    if result.success {
        Ok(None)
    } else {
        Err(format!("{what} did not succeed"))
    }
}

/// Set a registered operation's terminal status from its outcome.
async fn record_outcome(state: &AppState, id: &str, outcome: OperationOutcome) {
    let status = match outcome {
        Ok(result) => OperationStatus::Completed {
            result: result.map(|value| value.to_string()),
        },
        Err(error) => OperationStatus::Failed { error },
    };
    finish_operation(state, id, status).await;
}

/// How many finished operations `.roko/state/operations.jsonl` keeps.
pub(crate) const FINISHED_OPERATIONS_KEPT: usize = 500;

/// The size at which the kept log rotates, in MB; the count cap keeps it
/// well below this.
const FINISHED_OPERATIONS_MAX_MB: u64 = 16;

/// A finished operation as `.roko/state/operations.jsonl` keeps it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FinishedOperation {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) status: OperationStatus,
    /// The last progress its producer reported (gap-11d870).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) progress: Option<Value>,
    pub(crate) finished_at: chrono::DateTime<chrono::Utc>,
}

/// End operation `id` with `status`, a terminal one: in the kept record of
/// finished operations, which outlives the handle's collection and a
/// restart (bug-a0f01e), and then in its handle, so a status read from the
/// handle is already kept. An operation that was never registered keeps
/// nothing.
pub(crate) async fn finish_operation(state: &AppState, id: &str, status: OperationStatus) {
    let finished = {
        let operations = state.operations.read().await;
        let Some(operation) = operations.get(id) else {
            return;
        };
        FinishedOperation {
            id: id.to_string(),
            kind: operation.kind.clone(),
            status: status.clone(),
            progress: latest_progress(operation),
            finished_at: chrono::Utc::now(),
        }
    };
    let path = finished_operations_path(state);
    let kept = tokio::task::spawn_blocking(move || {
        keep_finished(&path, &finished, FINISHED_OPERATIONS_KEPT)
    })
    .await;
    if let Err(error) = kept
        .map_err(std::io::Error::other)
        .and_then(|result| result)
    {
        tracing::warn!(operation = id, %error, "the finished operation is not kept");
    }
    if let Some(operation) = state.operations.write().await.get_mut(id) {
        operation.status = status;
    }
}

/// The kept record of finished operation `id`: the latest, should it have
/// ended twice. `None` when none is kept.
pub(crate) fn finished_operation(state: &AppState, id: &str) -> Option<FinishedOperation> {
    let text = std::fs::read_to_string(finished_operations_path(state)).ok()?;
    text.lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<FinishedOperation>(line).ok())
        .find(|finished| finished.id == id)
}

/// The latest progress `operation`'s producer reported, when it reports any.
pub(crate) fn latest_progress(operation: &OperationHandle) -> Option<Value> {
    let progress = operation.progress.as_ref()?.borrow().clone();
    (!progress.is_null()).then_some(progress)
}

/// An operation's status as `GET /api/operations/{id}` reports it:
/// `{id, kind, status, result?, error?, elapsed_secs?, progress?}`, with the
/// stored result parsed back into JSON.
pub(crate) fn status_json(
    id: &str,
    kind: &str,
    status: &OperationStatus,
    elapsed_secs: Option<u64>,
    progress: Option<Value>,
) -> Value {
    let mut response = json!({ "id": id, "kind": kind });
    match status {
        OperationStatus::Running => response["status"] = json!("running"),
        OperationStatus::Completed { result } => {
            response["status"] = json!("completed");
            if let Some(result) = result
                .as_deref()
                .and_then(|result| serde_json::from_str::<Value>(result).ok())
            {
                response["result"] = result;
            }
        }
        OperationStatus::Failed { error } => {
            response["status"] = json!("failed");
            response["error"] = json!(error);
        }
    }
    if let Some(elapsed_secs) = elapsed_secs {
        response["elapsed_secs"] = json!(elapsed_secs);
    }
    if let Some(progress) = progress {
        response["progress"] = progress;
    }
    response
}

/// Where finished operations are kept.
fn finished_operations_path(state: &AppState) -> PathBuf {
    state.layout.state_dir().join("operations.jsonl")
}

/// Append `finished` to the kept log at `path`, then keep its newest `kept`
/// records.
fn keep_finished(path: &Path, finished: &FinishedOperation, kept: usize) -> std::io::Result<()> {
    let line = serde_json::to_vec(finished)?;
    roko_fs::log_rotation::append_jsonl_line_sync(path, &line, FINISHED_OPERATIONS_MAX_MB)?;
    roko_fs::log_rotation::compact_jsonl_to_latest_sync(path, kept)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use axum::Router;
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use roko_core::config::ServeAuthConfig;
    use roko_core::config::schema::RokoConfig;
    use serde_json::json;
    use tempfile::tempdir;
    use tower::ServiceExt;

    use crate::deploy::manual::ManualBackend;
    use crate::plan_types::{PlanSummaryDto, PlanTasksDto};
    use crate::routes::build_router;
    use crate::runtime::{
        CliRuntime, DashboardInfo, NoOpRuntime, PlanGenerationResult, SessionStatusInfo,
    };

    /// Succeeds at every runtime call an operation producer makes, and serves
    /// an empty plan `demo` for plan chat.
    struct SucceedingRuntime;

    #[async_trait::async_trait]
    impl CliRuntime for SucceedingRuntime {
        async fn run_once(&self, _workdir: &Path, _prompt: &str) -> anyhow::Result<RunResult> {
            Ok(RunResult {
                success: true,
                output_text: None,
                usage: None,
                gate_results: Vec::new(),
            })
        }

        async fn generate_plan_from_prompt(
            &self,
            workdir: &Path,
            slug: &str,
            _prompt: &str,
        ) -> anyhow::Result<PlanGenerationResult> {
            let plans_root = workdir.join("plans");
            Ok(PlanGenerationResult {
                plan_targets: vec![plans_root.join(slug)],
                plans_root,
                artifacts: Vec::new(),
                cost_usd: 0.0,
            })
        }

        async fn load_plan_summary(
            &self,
            _workdir: &Path,
            plan_id: &str,
        ) -> anyhow::Result<Option<PlanSummaryDto>> {
            Ok(Some(serde_json::from_value(json!({
                "id": plan_id,
                "title": "Demo",
                "task_count": 0,
                "tasks_done": 0,
                "tasks_failed": 0,
                "completed": false,
                "status": "ready",
                "old_format": false,
            }))?))
        }

        async fn load_plan_tasks(
            &self,
            _workdir: &Path,
            plan_id: &str,
        ) -> anyhow::Result<Option<PlanTasksDto>> {
            Ok(Some(serde_json::from_value(json!({
                "plan_id": plan_id,
                "task_count": 0,
                "tasks": [],
            }))?))
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

    fn state_with(dir: &Path, runtime: Arc<dyn CliRuntime>) -> Arc<AppState> {
        Arc::new(
            AppState::new(
                dir.to_path_buf(),
                runtime,
                RokoConfig::default(),
                Arc::new(ManualBackend::default()),
            )
            .expect("AppState::new"),
        )
    }

    async fn panicking_work() -> OperationOutcome {
        panic!("work panicked")
    }

    async fn status_of(state: &AppState, id: &str) -> OperationStatus {
        state.operations.read().await[id].status.clone()
    }

    /// Wait until operation `id` leaves `Running`, and return its status.
    async fn wait_terminal(state: &AppState, id: &str) -> OperationStatus {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let status = status_of(state, id).await;
                if !matches!(status, OperationStatus::Running) {
                    return status;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("operation {id} still running"))
    }

    #[tokio::test]
    async fn spawn_operation_records_completed_failed_and_panicked_work() {
        let dir = tempdir().expect("tempdir");
        let state = state_with(dir.path(), Arc::new(NoOpRuntime));

        spawn_operation(&state, "ok".into(), "test:ok".into(), "test", async {
            Ok::<_, String>(Some(json!({ "answer": 42 })))
        })
        .await;
        spawn_operation(&state, "err".into(), "test:err".into(), "test", async {
            Err::<Option<Value>, _>("boom".to_string())
        })
        .await;
        spawn_operation(
            &state,
            "panic".into(),
            "test:panic".into(),
            "test",
            panicking_work(),
        )
        .await;

        match wait_terminal(&state, "ok").await {
            OperationStatus::Completed { result } => {
                assert_eq!(result.as_deref(), Some(r#"{"answer":42}"#));
            }
            other => panic!("ok work should complete, got {other:?}"),
        }
        match wait_terminal(&state, "err").await {
            OperationStatus::Failed { error } => assert_eq!(error, "boom"),
            other => panic!("failing work should fail, got {other:?}"),
        }
        match wait_terminal(&state, "panic").await {
            OperationStatus::Failed { error } => assert_eq!(error, "test operation panicked"),
            other => panic!("panicking work should fail, got {other:?}"),
        }
        assert_eq!(state.operations.read().await["ok"].kind, "test:ok");

        let completed = state
            .event_bus
            .replay_from(0)
            .into_iter()
            .filter(|event| matches!(event.payload, ServerEvent::OperationCompleted { .. }))
            .count();
        assert_eq!(completed, 3, "one OperationCompleted per operation");
    }

    #[tokio::test]
    async fn every_producer_records_terminal_status() {
        let dir = tempdir().expect("tempdir");
        let state = state_with(dir.path(), Arc::new(SucceedingRuntime));
        state.templates.write().await.seed_builtins();
        let app = build_router(
            Arc::clone(&state),
            &[],
            ServeAuthConfig {
                enabled: false,
                ..ServeAuthConfig::default()
            },
        );

        // Every route that starts a background operation, except
        // `POST /api/dream/run`: it runs the real dream consolidation, which
        // this hermetic test does not exercise. It uses `spawn_operation` too.
        // `POST /api/inference/batch` records its own result.
        let producers = [
            ("/api/research/topic", json!({ "topic": "retry policies" })),
            ("/api/templates/pr-review/deploy", json!({})),
            ("/api/plans/demo/chat", json!({ "message": "split T1" })),
        ];
        let mut started = Vec::new();
        for (uri, body) in producers {
            let id = post_for_operation(&app, uri, body).await;
            started.push((uri, id));
        }

        for (uri, id) in started {
            let status = wait_terminal(&state, &id).await;
            assert!(
                matches!(status, OperationStatus::Completed { .. }),
                "{uri} left operation {id} as {status:?}"
            );
            let reported = get_json(&app, &format!("/api/operations/{id}")).await;
            assert_eq!(reported["status"], "completed", "{uri}: {reported}");
        }
    }

    /// bug-a0f01e: a finished operation's status outlives its handle. Once
    /// handle GC has dropped it, and after a restart, `GET
    /// /api/operations/{id}` still says how it ended: its result, or its error.
    #[tokio::test]
    async fn a_finished_operation_survives_collection_and_a_restart() {
        let dir = tempdir().expect("tempdir");
        let state = state_with(dir.path(), Arc::new(NoOpRuntime));
        spawn_operation(&state, "ok".into(), "test:ok".into(), "test", async {
            Ok::<_, String>(Some(json!({ "answer": 42 })))
        })
        .await;
        spawn_operation(&state, "err".into(), "test:err".into(), "test", async {
            Err::<Option<Value>, _>("boom".to_string())
        })
        .await;
        for id in ["ok", "err"] {
            wait_terminal(&state, id).await;
        }
        tokio::time::timeout(Duration::from_secs(5), async {
            while !state.operations.read().await.is_empty() {
                state.gc_completed_handles().await;
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("handle GC drops both handles");

        let reported = |state: Arc<AppState>| async move {
            let app = build_router(
                state,
                &[],
                ServeAuthConfig {
                    enabled: false,
                    ..ServeAuthConfig::default()
                },
            );
            (
                get_json(&app, "/api/operations/ok").await,
                get_json(&app, "/api/operations/err").await,
            )
        };
        let collected = reported(Arc::clone(&state)).await;
        drop(state);
        let restarted = reported(state_with(dir.path(), Arc::new(NoOpRuntime))).await;
        for (ok, err) in [collected, restarted] {
            assert_eq!(ok["status"], "completed", "{ok}");
            assert_eq!(ok["kind"], "test:ok", "{ok}");
            assert_eq!(ok["result"]["answer"], 42, "{ok}");
            assert_eq!(err["status"], "failed", "{err}");
            assert_eq!(err["error"], "boom", "{err}");
        }
    }

    /// bug-a0f01e: the kept record of finished operations holds only its
    /// newest entries.
    #[test]
    fn the_kept_record_holds_only_the_newest_operations() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("operations.jsonl");
        for id in ["a", "b", "c"] {
            let finished = FinishedOperation {
                id: id.to_string(),
                kind: "test".to_string(),
                status: OperationStatus::Failed {
                    error: id.to_string(),
                },
                progress: None,
                finished_at: chrono::Utc::now(),
            };
            keep_finished(&path, &finished, 2).expect("kept");
        }
        let ids: Vec<String> = std::fs::read_to_string(&path)
            .expect("kept record")
            .lines()
            .map(|line| {
                serde_json::from_str::<FinishedOperation>(line)
                    .expect("a finished operation")
                    .id
            })
            .collect();
        assert_eq!(ids, ["b", "c"]);
    }

    /// POST `body` to `uri`, expect 202 Accepted, and return the operation id.
    async fn post_for_operation(app: &Router, uri: &str, body: Value) -> String {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .expect("request"),
            )
            .await
            .expect("response");
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let payload: Value = serde_json::from_slice(&bytes).expect("json body");
        assert_eq!(status, StatusCode::ACCEPTED, "{uri}: {payload}");
        payload["id"]
            .as_str()
            .unwrap_or_else(|| panic!("{uri} returned no operation id: {payload}"))
            .to_string()
    }

    async fn get_json(app: &Router, uri: &str) -> Value {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK, "{uri}");
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        serde_json::from_slice(&bytes).expect("json body")
    }
}
