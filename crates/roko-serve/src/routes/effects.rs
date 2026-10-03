//! Staged outbound effects (9133): a tool call a run held for a person's
//! approval instead of running it (9131).
//!
//! * `GET /api/effects?run_id=` -- the effects that wait for a decision,
//!   without their arguments, which may carry secrets, and the decisions
//!   made on them, newest first.
//! * `POST /api/effects/{id}/decision` with `{ "approve": bool, "note": ... }`
//!   -- approve an effect, which runs its call once and checks its receipt,
//!   or reject it, as the authenticated principal. The answer is the
//!   decision's record, with the call's result and the receipt verdicts. A
//!   decided effect answers 409 and an unknown one 404.
//!
//! The runtime does the work (`CliRuntime::decide_effect`, roko-cli's
//! `effects_apply`). A chat host's approval flow calls these through `/mcp`
//! (9138).

use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::{Extension, Path as AxumPath, Query, State};
use axum::routing::{get, post};
use serde::Deserialize;
use serde_json::Value;

use super::middleware::AuthContext;
use crate::error::ApiError;
use crate::runtime::{EffectDecisionError, EffectDecisionInput};
use crate::state::AppState;

/// One decision at a time: two approvals of one effect racing would look
/// to the runtime like an approval that crashed.
static DECISIONS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/effects", get(list_effects_handler))
        .route("/effects/{id}/decision", post(decide_effect_handler))
}

/// Query of `GET /api/effects`.
#[derive(Debug, Default, Deserialize)]
struct EffectsQuery {
    /// Only this run's effects.
    run_id: Option<String>,
}

async fn list_effects_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<EffectsQuery>,
) -> Result<Json<Value>, ApiError> {
    let effects = state
        .runtime
        .list_effects(&state.workdir, query.run_id.as_deref())
        .await
        .map_err(|error| ApiError::internal(format!("list staged effects: {error}")))?;
    Ok(Json(effects))
}

/// Body of `POST /api/effects/{id}/decision`.
#[derive(Debug, Deserialize)]
struct DecisionRequest {
    /// Approve the effect, or reject it.
    approve: bool,
    /// Why, recorded with the decision.
    #[serde(default)]
    note: Option<String>,
}

async fn decide_effect_handler(
    State(state): State<Arc<AppState>>,
    AxumPath(effect_id): AxumPath<String>,
    auth: Option<Extension<AuthContext>>,
    Json(request): Json<DecisionRequest>,
) -> Result<Json<Value>, ApiError> {
    let decided_by = auth
        .and_then(|Extension(context)| context.user_id)
        .unwrap_or_else(|| "local-api".to_string());
    let decision = EffectDecisionInput {
        approve: request.approve,
        note: request.note,
        decided_by,
    };
    let _one_at_a_time = DECISIONS.lock().await;
    let record = state
        .runtime
        .decide_effect(&state.workdir, &effect_id, decision)
        .await
        .map_err(|error| match error {
            EffectDecisionError::NotFound(_) => ApiError::not_found(error.to_string()),
            EffectDecisionError::AlreadyDecided(..) => ApiError::conflict(error.to_string()),
            EffectDecisionError::Unsupported => ApiError::not_implemented(
                error.to_string(),
                "runtime",
                "run roko serve from the roko CLI, whose runtime stages effects",
            ),
            EffectDecisionError::Failed(reason) => {
                ApiError::internal(format!("decide effect {effect_id}: {reason}"))
            }
        })?;
    Ok(Json(record))
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt as _;
    use roko_core::config::RokoConfig;
    use tower::ServiceExt as _;

    use super::*;
    use crate::deploy::manual::ManualBackend;
    use crate::runtime::{CliRuntime, DashboardInfo, RunResult, SessionStatusInfo};

    /// A runtime that holds one staged effect, `effect-1`, and records each
    /// decision it is asked to make.
    #[derive(Default)]
    struct EffectsRuntime {
        decisions: Mutex<Vec<(String, EffectDecisionInput)>>,
        applied: Mutex<usize>,
    }

    #[async_trait]
    impl CliRuntime for EffectsRuntime {
        async fn run_once(&self, _workdir: &Path, _prompt: &str) -> anyhow::Result<RunResult> {
            anyhow::bail!("not used")
        }

        fn session_status(&self, workdir: std::path::PathBuf) -> SessionStatusInfo {
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

        async fn decide_effect(
            &self,
            _workdir: &Path,
            effect_id: &str,
            decision: EffectDecisionInput,
        ) -> Result<Value, EffectDecisionError> {
            let approve = decision.approve;
            let by = decision.decided_by.clone();
            self.decisions
                .lock()
                .expect("decisions")
                .push((effect_id.to_string(), decision));
            if effect_id != "effect-1" {
                return Err(EffectDecisionError::NotFound(effect_id.to_string()));
            }
            let mut applied = self.applied.lock().expect("applied");
            if *applied > 0 {
                let outcome = "applied".to_string();
                return Err(EffectDecisionError::AlreadyDecided(
                    effect_id.to_string(),
                    outcome,
                ));
            }
            *applied += usize::from(approve);
            Ok(serde_json::json!({
                "effect_id": effect_id,
                "outcome": if approve { "applied" } else { "rejected" },
                "decided_by": by,
                "result": "delivered",
                "receipts": [{ "rung": "delivered", "passed": true, "output": "" }],
            }))
        }
    }

    async fn post(state: &Arc<AppState>, uri: &str, body: Value) -> (StatusCode, Value) {
        let request = Request::post(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("request");
        let response = routes()
            .with_state(Arc::clone(state))
            .oneshot(request)
            .await
            .expect("response");
        let status = response.status();
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, body)
    }

    /// 9133: approving `effect-1` decides it once, as the caller, and answers
    /// its record with the result and the receipt verdict; a second approval
    /// answers 409 and applies nothing more; an unknown id answers 404.
    #[tokio::test]
    async fn effect_decision_route_applies_an_approved_effect() {
        let workdir = tempfile::tempdir().expect("tempdir");
        let runtime = Arc::new(EffectsRuntime::default());
        let state = Arc::new(
            AppState::new(
                workdir.path().to_path_buf(),
                Arc::clone(&runtime) as Arc<dyn CliRuntime>,
                RokoConfig::default(),
                Arc::new(ManualBackend::default()),
            )
            .expect("create state"),
        );
        let approve = serde_json::json!({ "approve": true, "note": "checked" });

        let (status, record) = post(&state, "/effects/effect-1/decision", approve.clone()).await;
        assert_eq!(status, StatusCode::OK, "{record}");
        assert_eq!(record["outcome"], "applied", "{record}");
        assert_eq!(record["receipts"][0]["passed"], true, "{record}");
        assert_eq!(record["decided_by"], "local-api", "{record}");

        let (status, body) = post(&state, "/effects/effect-1/decision", approve.clone()).await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(*runtime.applied.lock().expect("applied"), 1);

        let (status, body) = post(&state, "/effects/effect-9/decision", approve).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
        let decisions = runtime.decisions.lock().expect("decisions");
        assert_eq!(decisions.len(), 3);
        assert_eq!(decisions[0].1.note.as_deref(), Some("checked"));
    }
}
