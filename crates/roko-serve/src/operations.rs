//! Background operations listed by `GET /api/operations/{id}` (bug-a0f01e).
//!
//! [`spawn_operation`] starts one. It registers the handle in
//! `AppState.operations` before the work can finish, records how the work
//! ended (`Completed` or `Failed`; a panic counts as failed), and only then
//! publishes `OperationCompleted`, so a client reacting to that event never
//! reads a stale `running` status.

use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use futures::FutureExt;
use serde_json::Value;

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
    if let Some(operation) = state.operations.write().await.get_mut(id) {
        operation.status = status;
    }
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

        async fn generate_plan_from_prd(
            &self,
            workdir: &Path,
            slug: &str,
            _prd_path: &Path,
        ) -> anyhow::Result<PlanGenerationResult> {
            let plans_root = workdir.join("plans");
            Ok(PlanGenerationResult {
                plan_targets: vec![plans_root.join(slug)],
                plans_root,
                artifacts: Vec::new(),
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
            ("/api/prds/alpha/draft", json!({})),
            ("/api/prds/alpha/plan", json!({})),
            ("/api/prd/consolidate", json!({})),
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
