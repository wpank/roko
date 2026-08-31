//! StateHub-backed projection routes for remote read and watch flows.

use std::collections::VecDeque;
use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use futures::stream::{self, Stream, StreamExt};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::broadcast;
use tracing::warn;

use crate::error::ApiError;
use crate::projection_contract::{
    AutonomyProjection, CanvasProjection, InboxProjection, MinimapProjection, ProjectionEnvelope,
    ProjectionQuery, RuntimeProjectionSet, WorkbenchProjection, projection_accepts_event,
    projection_delta_frame,
};
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/projections/catalog", get(projections_catalog))
        .route("/projections/telemetry", get(get_telemetry))
        .route("/projections/telemetry/stream", get(stream_telemetry))
        .route("/projections/workbench", get(get_workbench_surface))
        .route("/projections/inbox", get(get_inbox_surface))
        .route("/projections/canvas", get(get_canvas_surface))
        .route("/projections/minimap", get(get_minimap_surface))
        .route("/projections/autonomy", get(get_autonomy_surface))
        .route("/projections/{name}", get(get_projection))
        .route("/projections/{name}/stream", get(stream_projection))
        .route("/statehub/lens-runtimes", get(get_lens_runtimes))
        .route(
            "/statehub/lens-runtimes/{runtime_id}",
            get(get_lens_runtime),
        )
        .route(
            "/statehub/lens-runtimes/{runtime_id}/{lens}/reset",
            post(reset_lens_runtime),
        )
        .route(
            "/statehub/lens-runtimes/{runtime_id}/{lens}/enable",
            post(enable_lens_runtime),
        )
        .route(
            "/statehub/lens-runtimes/{runtime_id}/{lens}/disable",
            post(disable_lens_runtime),
        )
        .route("/statehub/{projection_id}", get(get_statehub_projection))
        .route(
            "/statehub/{projection_id}/history",
            get(get_statehub_projection_history),
        )
}

/// `GET /api/projections/catalog` — return projection names, versions, and invalidation policies.
async fn projections_catalog() -> Json<Value> {
    let entries = crate::projection_contract::projection_policies();
    Json(json!({ "projections": entries }))
}

async fn get_projection(
    Path(name): Path<String>,
    Query(query): Query<ProjectionQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    let projections = RuntimeProjectionSet::load(&state).await?;
    let projection = projections.project(&name, &query)?;
    Ok(Json(projections.state_frame(&name, projection)))
}

async fn get_workbench_surface(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ProjectionEnvelope<WorkbenchProjection>>, ApiError> {
    let projections = RuntimeProjectionSet::load(&state).await?;
    let data = projections.workbench_surface();
    Ok(Json(projections.envelope("workbench", data)))
}

async fn get_inbox_surface(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ProjectionEnvelope<InboxProjection>>, ApiError> {
    let projections = RuntimeProjectionSet::load(&state).await?;
    let data = projections.inbox_surface();
    Ok(Json(projections.envelope("inbox", data)))
}

async fn get_canvas_surface(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ProjectionEnvelope<CanvasProjection>>, ApiError> {
    let projections = RuntimeProjectionSet::load(&state).await?;
    let data = projections.canvas_surface();
    Ok(Json(projections.envelope("canvas", data)))
}

async fn get_minimap_surface(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ProjectionEnvelope<MinimapProjection>>, ApiError> {
    let projections = RuntimeProjectionSet::load(&state).await?;
    let data = projections.minimap_surface();
    Ok(Json(projections.envelope("minimap", data)))
}

async fn get_autonomy_surface(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ProjectionEnvelope<AutonomyProjection>>, ApiError> {
    let projections = RuntimeProjectionSet::load(&state).await?;
    let data = projections.autonomy_surface();
    Ok(Json(projections.envelope("autonomy", data)))
}

/// `GET /api/statehub/{projection_id}` — return the current materialized Lens projection.
async fn get_statehub_projection(
    Path(projection_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    let projection = state
        .state_hub
        .get_projection(&projection_id)
        .ok_or_else(|| ApiError::not_found(format!("projection '{projection_id}' not found")))?;
    Ok(Json(json!(projection)))
}

#[derive(Debug, Default, Deserialize)]
struct ProjectionHistoryQuery {
    from: Option<String>,
    to: Option<String>,
    from_version: Option<u64>,
    to_version: Option<u64>,
    resolution: Option<String>,
    limit: Option<usize>,
}

/// `GET /api/statehub/{projection_id}/history` — return bounded retained versions.
async fn get_statehub_projection_history(
    Path(projection_id): Path<String>,
    Query(query): Query<ProjectionHistoryQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    let from = parse_history_timestamp("from", query.from.as_deref())?;
    let to = parse_history_timestamp("to", query.to.as_deref())?;
    let resolution_ms = parse_history_resolution(query.resolution.as_deref())?;
    if matches!((&from, &to), (Some(from), Some(to)) if from > to) {
        return Err(ApiError::bad_request(
            "history `from` timestamp must not be after `to`",
        ));
    }
    if query
        .from_version
        .zip(query.to_version)
        .is_some_and(|(from, to)| from > to)
    {
        return Err(ApiError::bad_request(
            "history `from_version` must not exceed `to_version`",
        ));
    }

    let retained = state.state_hub.projection_history(&projection_id);
    if retained.is_empty() && state.state_hub.get_projection(&projection_id).is_none() {
        return Err(ApiError::not_found(format!(
            "projection '{projection_id}' not found"
        )));
    }
    let retained_count = retained.len();
    let mut history = retained
        .into_iter()
        .filter(|projection| {
            query
                .from_version
                .is_none_or(|version| projection.version >= version)
                && query
                    .to_version
                    .is_none_or(|version| projection.version <= version)
                && projection_timestamp(projection).is_some_and(|updated_at| {
                    from.as_ref().is_none_or(|from| &updated_at >= from)
                        && to.as_ref().is_none_or(|to| &updated_at <= to)
                })
        })
        .collect::<Vec<_>>();
    let matched = history.len();
    if let Some(resolution_ms) = resolution_ms {
        let mut buckets = std::collections::BTreeMap::new();
        for projection in history {
            if let Some(updated_at) = projection_timestamp(&projection) {
                let bucket = updated_at.timestamp_millis().div_euclid(resolution_ms);
                // Projection history is version ordered, so replacement keeps
                // the newest value observed in each time bucket.
                buckets.insert(bucket, projection);
            }
        }
        history = buckets.into_values().collect();
    }
    let coalesced = history.len();
    let limit = query.limit.unwrap_or(250).min(10_000);
    if history.len() > limit {
        history.drain(..history.len() - limit);
    }

    Ok(Json(json!({
        "projection_id": projection_id,
        "retained": retained_count,
        "matched": matched,
        "coalesced": coalesced,
        "resolution": query.resolution,
        "capacity": state.state_hub.projection_history_capacity(),
        "retention_seconds": state.state_hub.projection_history_retention().as_secs(),
        "retention_ms": state.state_hub.projection_history_retention().as_millis(),
        "history": history,
    })))
}

fn parse_history_resolution(value: Option<&str>) -> Result<Option<i64>, ApiError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    let split = value
        .find(|character: char| !character.is_ascii_digit())
        .ok_or_else(|| {
            ApiError::bad_request(
                "history `resolution` requires a unit: supported units are ms, s, m, h, and d",
            )
        })?;
    let (amount, unit) = value.split_at(split);
    let amount = amount.parse::<u64>().map_err(|error| {
        ApiError::bad_request(format!(
            "history `resolution` must start with a positive integer: {error}"
        ))
    })?;
    if amount == 0 {
        return Err(ApiError::bad_request(
            "history `resolution` must be greater than zero",
        ));
    }
    let multiplier = match unit {
        "ms" => 1_u64,
        "s" => 1_000,
        "m" => 60_000,
        "h" => 3_600_000,
        "d" => 86_400_000,
        _ => {
            return Err(ApiError::bad_request(
                "history `resolution` has an unsupported unit: use ms, s, m, h, or d",
            ));
        }
    };
    let milliseconds = amount
        .checked_mul(multiplier)
        .and_then(|value| i64::try_from(value).ok())
        .ok_or_else(|| ApiError::bad_request("history `resolution` is too large"))?;
    Ok(Some(milliseconds))
}

fn parse_history_timestamp(
    field: &str,
    value: Option<&str>,
) -> Result<Option<DateTime<Utc>>, ApiError> {
    value
        .map(|value| {
            DateTime::parse_from_rfc3339(value)
                .map(|timestamp| timestamp.with_timezone(&Utc))
                .map_err(|error| {
                    ApiError::bad_request(format!(
                        "history `{field}` must be an RFC 3339 timestamp: {error}"
                    ))
                })
        })
        .transpose()
}

fn projection_timestamp(projection: &roko_runtime::ProjectionState) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&projection.updated_at)
        .ok()
        .map(|timestamp| timestamp.with_timezone(&Utc))
}

/// `GET /api/statehub/lens-runtimes` — inspect all live queued Lens runtimes.
async fn get_lens_runtimes(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({ "runtimes": state.state_hub.lens_runtime_snapshots() }))
}

/// `GET /api/statehub/lens-runtimes/{runtime_id}` — inspect one Lens runtime.
async fn get_lens_runtime(
    Path(runtime_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    state
        .state_hub
        .lens_runtime_snapshot(&runtime_id)
        .map(|runtime| Json(json!(runtime)))
        .ok_or_else(|| ApiError::not_found(format!("Lens runtime '{runtime_id}' not found")))
}

async fn reset_lens_runtime(
    Path((runtime_id, lens)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    control_lens_runtime(&state, &runtime_id, &lens, None)
}

async fn enable_lens_runtime(
    Path((runtime_id, lens)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    control_lens_runtime(&state, &runtime_id, &lens, Some(true))
}

async fn disable_lens_runtime(
    Path((runtime_id, lens)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    control_lens_runtime(&state, &runtime_id, &lens, Some(false))
}

fn control_lens_runtime(
    state: &AppState,
    runtime_id: &str,
    lens: &str,
    enabled: Option<bool>,
) -> Result<Json<Value>, ApiError> {
    if state.state_hub.lens_runtime_snapshot(runtime_id).is_none() {
        return Err(ApiError::not_found(format!(
            "Lens runtime '{runtime_id}' not found"
        )));
    }
    let result = match enabled {
        Some(enabled) => state
            .state_hub
            .set_lens_runtime_enabled(runtime_id, lens, enabled),
        None => state.state_hub.reset_lens_runtime(runtime_id, lens),
    };
    result.map_err(ApiError::bad_request)?;
    get_lens_runtime_snapshot(state, runtime_id)
}

fn get_lens_runtime_snapshot(state: &AppState, runtime_id: &str) -> Result<Json<Value>, ApiError> {
    state
        .state_hub
        .lens_runtime_snapshot(runtime_id)
        .map(|runtime| Json(json!(runtime)))
        .ok_or_else(|| ApiError::not_found(format!("Lens runtime '{runtime_id}' not found")))
}

/// `GET /api/projections/telemetry` — current telemetry state: active watchers,
/// circuit breaker states, and observation counts.
async fn get_telemetry(
    Query(query): Query<ProjectionQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    let projections = RuntimeProjectionSet::load(&state).await?;
    let projection = projections.project("telemetry", &query)?;
    Ok(Json(projections.state_frame("telemetry", projection)))
}

/// `GET /api/projections/telemetry/stream` — SSE stream of telemetry updates.
///
/// Emits an initial `state` event with the full telemetry snapshot, then
/// streams `delta` events for each matching `DashboardEvent`.
async fn stream_telemetry(
    Query(query): Query<ProjectionQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let name = "telemetry";
    let projections = RuntimeProjectionSet::load(&state).await?;
    let initial_state = projections.project(name, &query)?;
    let initial = Event::default()
        .event("state")
        .id(projections.cursor.to_string())
        .data(projections.state_frame(name, initial_state).to_string());

    let query_for_stream = query.clone();
    let state_for_stream = Arc::clone(&state);
    let subscription = state.state_hub.subscribe_events_from(projections.cursor);
    let delta_stream = stream::unfold(
        (VecDeque::from(subscription.replay), subscription.live),
        move |(mut replay, mut rx)| {
            let query = query_for_stream.clone();
            let state = Arc::clone(&state_for_stream);
            async move {
                loop {
                    let received = match replay.pop_front() {
                        Some(envelope) => Ok(envelope),
                        None => rx.recv().await,
                    };
                    match received {
                        Ok(envelope) => {
                            if !projection_accepts_event("telemetry", &query, &envelope.payload) {
                                continue;
                            }
                            let event = Event::default()
                                .event("delta")
                                .id(envelope.seq.to_string())
                                .data(
                                    projection_delta_frame(
                                        "telemetry",
                                        envelope.seq,
                                        &envelope.payload,
                                    )
                                    .to_string(),
                                );
                            return Some((Ok(event), (replay, rx)));
                        }
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            warn!(projection = "telemetry", skipped, "telemetry stream lagged");
                            let Ok(refreshed) = RuntimeProjectionSet::load(&state).await else {
                                return None;
                            };
                            let Ok(replacement_state) = refreshed.project("telemetry", &query)
                            else {
                                return None;
                            };
                            let replacement = Event::default()
                                .event("state")
                                .id(refreshed.cursor.to_string())
                                .data(
                                    refreshed
                                        .state_frame("telemetry", replacement_state)
                                        .to_string(),
                                );
                            let subscription =
                                state.state_hub.subscribe_events_from(refreshed.cursor);
                            return Some((
                                Ok(replacement),
                                (VecDeque::from(subscription.replay), subscription.live),
                            ));
                        }
                        Err(broadcast::error::RecvError::Closed) => return None,
                    }
                }
            }
        },
    );

    Ok(
        Sse::new(stream::once(async move { Ok(initial) }).chain(delta_stream))
            .keep_alive(KeepAlive::default()),
    )
}

async fn stream_projection(
    Path(name): Path<String>,
    Query(query): Query<ProjectionQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let projections = RuntimeProjectionSet::load(&state).await?;
    let initial_state = projections.project(&name, &query)?;
    let initial = Event::default()
        .event("state")
        .id(projections.cursor.to_string())
        .data(projections.state_frame(&name, initial_state).to_string());

    let name_for_stream = name.clone();
    let query_for_stream = query.clone();
    let state_for_stream = Arc::clone(&state);
    let subscription = state.state_hub.subscribe_events_from(projections.cursor);
    let delta_stream = stream::unfold(
        (VecDeque::from(subscription.replay), subscription.live),
        move |(mut replay, mut rx)| {
            let name = name_for_stream.clone();
            let query = query_for_stream.clone();
            let state = Arc::clone(&state_for_stream);
            async move {
                loop {
                    let received = match replay.pop_front() {
                        Some(envelope) => Ok(envelope),
                        None => rx.recv().await,
                    };
                    match received {
                        Ok(envelope) => {
                            if !projection_accepts_event(&name, &query, &envelope.payload) {
                                continue;
                            }
                            let event = Event::default()
                                .event("delta")
                                .id(envelope.seq.to_string())
                                .data(
                                    projection_delta_frame(&name, envelope.seq, &envelope.payload)
                                        .to_string(),
                                );
                            return Some((Ok(event), (replay, rx)));
                        }
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            warn!(projection = %name, skipped, "projection stream lagged");
                            let Ok(refreshed) = RuntimeProjectionSet::load(&state).await else {
                                return None;
                            };
                            let Ok(replacement_state) = refreshed.project(&name, &query) else {
                                return None;
                            };
                            let replacement = Event::default()
                                .event("state")
                                .id(refreshed.cursor.to_string())
                                .data(refreshed.state_frame(&name, replacement_state).to_string());
                            let subscription =
                                state.state_hub.subscribe_events_from(refreshed.cursor);
                            return Some((
                                Ok(replacement),
                                (VecDeque::from(subscription.replay), subscription.live),
                            ));
                        }
                        Err(broadcast::error::RecvError::Closed) => return None,
                    }
                }
            }
        },
    );

    Ok(
        Sse::new(stream::once(async move { Ok(initial) }).chain(delta_stream))
            .keep_alive(KeepAlive::default()),
    )
}

#[cfg(test)]
mod tests {
    use super::parse_history_resolution;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use axum::body::Body as AxumBody;
    use axum::http::Request;
    use serde_json::Value;
    use tempfile::tempdir;
    use tower::ServiceExt;

    use crate::deploy::create_backend;
    use crate::routes::build_router;
    use crate::runtime::NoOpRuntime;
    use crate::state::AppState;
    use roko_core::config::ServeAuthConfig;
    use roko_core::{LensConfig, LensRegistry};
    use roko_runtime::{LensExecutor, LensQueueConfig};

    struct CostedCell;

    #[async_trait::async_trait]
    impl roko_graph::Cell for CostedCell {
        fn cell_id(&self) -> &str {
            "costed"
        }

        fn cell_name(&self) -> &str {
            "CostedCell"
        }

        fn estimated_cost(&self) -> Option<f64> {
            Some(0.25)
        }

        async fn execute(
            &self,
            input: Vec<roko_core::Signal>,
            _ctx: &roko_graph::CellContext,
        ) -> roko_core::Result<Vec<roko_core::Signal>> {
            Ok(input)
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

    fn write_runner_snapshot(root: &std::path::Path, plan_id: &str) {
        let state_dir = root.join(".roko/state");
        std::fs::create_dir_all(&state_dir).expect("state dir");
        let executor = serde_json::json!({
            "schema_version": 1,
            "plan_states": {(plan_id): {"plan_id": plan_id, "current_phase": {"kind": "implementing"}}},
            "queue_order": [plan_id],
            "speculative_executions": {},
            "timestamp_ms": 42
        });
        let snapshot = roko_runtime::StateSnapshot::new(
            42,
            executor.to_string(),
            serde_json::json!({"schema_version": 1, "executor": executor, "timestamp_ms": 42})
                .to_string(),
            serde_json::json!({
                "schema_version": 1,
                "run_id": format!("run-{plan_id}"),
                "timestamp_ms": 42,
                "tasks_total": 0,
                "tasks_completed": 0,
                "tasks_failed": 0,
                "total_tokens_in": 0,
                "total_tokens_out": 0,
                "total_cost_usd": 0.0,
                "total_agent_calls": 0,
                "replan_ledger": {}
            })
            .to_string(),
            serde_json::json!({"rungs": {}}).to_string(),
        );
        std::fs::write(
            state_dir.join("state-snapshot.json"),
            serde_json::to_vec(&snapshot).expect("serialize snapshot"),
        )
        .expect("write snapshot");
    }

    #[tokio::test]
    async fn recovered_projection_prefers_verified_snapshot_and_labels_source() {
        let (dir, state) = test_state();
        let state_dir = dir.path().join(".roko/state");
        std::fs::create_dir_all(&state_dir).expect("state dir");
        std::fs::write(
            state_dir.join("executor.json"),
            serde_json::json!({
                "plan_states": {"stale": {"current_phase": {"kind": "implementing"}}}
            })
            .to_string(),
        )
        .expect("legacy executor");
        write_runner_snapshot(dir.path(), "canonical");

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
                    .method("GET")
                    .uri("/api/projections/plan_state")
                    .body(AxumBody::empty())
                    .expect("request"),
            )
            .await
            .expect("projection response");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read response");
        let payload: Value = serde_json::from_slice(&body).expect("response json");
        assert_eq!(payload["recovered"], true);
        assert_eq!(payload["data"]["plans"][0]["plan_id"], "canonical");
        assert!(
            payload["data"]["plans"]
                .as_array()
                .is_some_and(|plans| plans.iter().all(|plan| plan["plan_id"] != "stale"))
        );
        assert_eq!(
            payload["evidence"]["runtime_feedback"]["executor_state"]["format"],
            "state_snapshot"
        );
    }

    #[tokio::test]
    async fn corrupt_snapshot_returns_uniform_invalid_source_without_legacy_projection() {
        let (dir, state) = test_state();
        let state_dir = dir.path().join(".roko/state");
        std::fs::create_dir_all(&state_dir).expect("state dir");
        std::fs::write(
            state_dir.join("executor.json"),
            serde_json::json!({
                "plan_states": {"legacy": {"current_phase": {"kind": "implementing"}}}
            })
            .to_string(),
        )
        .expect("legacy executor");
        write_runner_snapshot(dir.path(), "canonical");
        let snapshot_path = state_dir.join("state-snapshot.json");
        let mut snapshot: roko_runtime::StateSnapshot =
            serde_json::from_slice(&std::fs::read(&snapshot_path).expect("read snapshot"))
                .expect("decode snapshot");
        snapshot.checksum = "0".repeat(64);
        std::fs::write(
            snapshot_path,
            serde_json::to_vec(&snapshot).expect("serialize corrupt snapshot"),
        )
        .expect("write corrupt snapshot");

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
                    .method("GET")
                    .uri("/api/projections/plan_state")
                    .body(AxumBody::empty())
                    .expect("request"),
            )
            .await
            .expect("projection response");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read response");
        let payload: Value = serde_json::from_slice(&body).expect("response json");
        assert_eq!(payload["recovered"], false);
        assert!(
            payload["data"]["plans"]
                .as_array()
                .is_some_and(Vec::is_empty)
        );
        assert_eq!(
            payload["evidence"]["runtime_feedback"]["executor_state"]["format"],
            "invalid"
        );
        assert_eq!(
            payload["evidence"]["runtime_feedback"]["executor_state"]["state"],
            "invalid"
        );
        assert_eq!(payload["freshness"]["state"], "invalid");
        assert!(
            payload["evidence"]["runtime_feedback"]["executor_state"]["error"]
                .as_str()
                .is_some_and(|error| error.contains("checksum mismatch"))
        );
    }

    #[tokio::test]
    async fn dedicated_surface_routes_return_typed_live_state() {
        let (_dir, state) = test_state();
        state
            .state_hub
            .publish(roko_core::DashboardEvent::PlanStarted {
                plan_id: "release-flow".into(),
                tasks_total: 0,
            });
        state
            .state_hub
            .publish(roko_core::DashboardEvent::TaskStarted {
                plan_id: "release-flow".into(),
                task_id: "compile".into(),
                title: "Compile release".into(),
                phase: "execute".into(),
            });
        state
            .state_hub
            .publish(roko_core::DashboardEvent::AgentSpawned {
                agent_id: "agent-1".into(),
                plan_id: "release-flow".into(),
                task_id: "compile".into(),
                attempt: 1,
                role: "implementer".into(),
                model: "test-model".into(),
            });
        state
            .state_hub
            .publish(roko_core::DashboardEvent::InboxItemReceived {
                item_id: "question-1".into(),
                category: roko_core::dashboard_snapshot::InboxCategory::AgentQuestion,
                urgency: roko_core::dashboard_snapshot::UrgencyLevel::Question,
                summary: "Choose target".into(),
            });

        let app = build_router(
            Arc::clone(&state),
            &[],
            ServeAuthConfig {
                enabled: false,
                ..ServeAuthConfig::default()
            },
        );

        for surface in ["workbench", "inbox", "canvas", "minimap", "autonomy"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri(format!("/api/projections/{surface}"))
                        .body(AxumBody::empty())
                        .expect("request"),
                )
                .await
                .expect("surface response");
            assert_eq!(response.status(), axum::http::StatusCode::OK);
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("read body");
            let payload: Value = serde_json::from_slice(&body).expect("parse surface response");
            assert_eq!(payload["name"], surface);
            assert_eq!(payload["version"], 1);
            assert_eq!(payload["recovered"], false);
            assert!(payload["data"].is_object());
            match surface {
                "workbench" => assert_eq!(payload["data"]["flows"][0]["run_id"], "release-flow"),
                "inbox" => {
                    assert_eq!(payload["data"]["pending_count"], 1);
                    assert_eq!(payload["data"]["items"][0]["id"], "question-1");
                }
                "canvas" => {
                    assert_eq!(payload["data"]["graph_names"][0], "release-flow");
                    assert_eq!(payload["data"]["active_tasks"]["tasks"][0]["id"], "compile");
                }
                "minimap" => assert_eq!(payload["data"]["agents"][0]["id"], "agent-1"),
                "autonomy" => {
                    assert_eq!(payload["data"]["config_source"], "unavailable");
                    assert!(
                        payload["data"]["configs"]
                            .as_array()
                            .is_some_and(Vec::is_empty)
                    );
                    assert_eq!(
                        payload["data"]["agent_vitality"]["agents"][0]["name"],
                        "agent-1"
                    );
                }
                _ => unreachable!(),
            }
        }
    }

    #[tokio::test]
    async fn projection_telemetry_returns_telemetry_state() {
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
                    .method("GET")
                    .uri("/api/projections/telemetry")
                    .body(AxumBody::empty())
                    .expect("request"),
            )
            .await
            .expect("telemetry projection response");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let payload: Value = serde_json::from_slice(&body).expect("parse telemetry response");

        // Verify envelope fields.
        assert_eq!(payload["name"], "telemetry");
        assert_eq!(payload["canonical_name"], "telemetry");
        assert_eq!(payload["version"], 1);

        // Verify telemetry state structure.
        let data = &payload["data"];
        assert!(data["active_watchers"].is_array());
        assert!(data["circuit_breaker_states"].is_array());
        assert!(data["observation_counts"].is_object());
        assert!(data["provider_health"].is_array());
        assert!(data["lens_runtimes"].is_array());
        assert!(data["stats"].is_object());
    }

    #[tokio::test]
    async fn projection_telemetry_observation_counts_present() {
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
                    .method("GET")
                    .uri("/api/projections/telemetry")
                    .body(AxumBody::empty())
                    .expect("request"),
            )
            .await
            .expect("telemetry projection response");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let payload: Value = serde_json::from_slice(&body).expect("parse telemetry response");

        let counts = &payload["data"]["observation_counts"];
        // All counter fields should be present and numeric.
        assert!(counts["plans_active"].is_number());
        assert!(counts["plans_completed"].is_number());
        assert!(counts["tasks_active"].is_number());
        assert!(counts["agents_active"].is_number());
        assert!(counts["gates_passed"].is_number());
        assert!(counts["gates_failed"].is_number());
        assert!(counts["episodes_total"].is_number());
        assert!(counts["errors_total"].is_number());
        assert!(counts["efficiency_events"].is_number());
        assert!(counts["diagnoses"].is_number());
        assert!(counts["lens_queue_enqueued"].is_number());
        assert!(counts["lens_queue_processed"].is_number());
        assert!(counts["lens_queue_dropped_oldest"].is_number());
        assert!(counts["lens_queue_failed_dispatches"].is_number());
    }

    #[tokio::test]
    async fn projection_telemetry_via_generic_endpoint() {
        let (_dir, state) = test_state();
        let app = build_router(
            Arc::clone(&state),
            &[],
            ServeAuthConfig {
                enabled: false,
                ..ServeAuthConfig::default()
            },
        );

        // The generic projection endpoint should also return the telemetry projection.
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/projections/telemetry")
                    .body(AxumBody::empty())
                    .expect("request"),
            )
            .await
            .expect("generic telemetry projection response");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let payload: Value = serde_json::from_slice(&body).expect("parse response");
        assert_eq!(payload["canonical_name"], "telemetry");
    }

    #[tokio::test]
    async fn projection_catalog_includes_telemetry() {
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
                    .method("GET")
                    .uri("/api/projections/catalog")
                    .body(AxumBody::empty())
                    .expect("request"),
            )
            .await
            .expect("catalog response");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let payload: Value = serde_json::from_slice(&body).expect("parse catalog response");

        let projections = payload["projections"]
            .as_array()
            .expect("projections should be an array");
        let telemetry_entry = projections
            .iter()
            .find(|entry| entry["name"] == "telemetry");
        assert!(
            telemetry_entry.is_some(),
            "catalog should include telemetry projection"
        );
        let entry = telemetry_entry.unwrap();
        assert_eq!(entry["version"], 1);
        assert_eq!(entry["policy"]["max_age_secs"], 5);
        assert_eq!(entry["policy"]["incremental"], true);
    }

    #[tokio::test]
    async fn projection_catalog_includes_all_typed_lens_projections() {
        let (_dir, state) = test_state();
        let app = build_router(
            state,
            &[],
            ServeAuthConfig {
                enabled: false,
                ..ServeAuthConfig::default()
            },
        );
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/projections/catalog")
                    .body(AxumBody::empty())
                    .expect("request"),
            )
            .await
            .expect("catalog response");
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let payload: Value = serde_json::from_slice(&body).expect("parse catalog response");
        let projections = payload["projections"]
            .as_array()
            .expect("projections should be an array");

        for id in [
            "cohort_health",
            "active_tasks",
            "gate_pipeline",
            "cost_meter",
            "knowledge_health",
            "c_factor",
            "agent_vitality",
        ] {
            assert!(
                projections.iter().any(|entry| entry["name"] == id),
                "catalog should include {id}"
            );
        }
    }

    #[tokio::test]
    async fn statehub_routes_expose_current_projection_and_filtered_history() {
        let (_dir, state) = test_state();
        state.state_hub.update_projection(
            "cohort_health",
            serde_json::json!({"agent_count": 3}),
            "cohort",
        );

        let current_app = build_router(
            Arc::clone(&state),
            &[],
            ServeAuthConfig {
                enabled: false,
                ..ServeAuthConfig::default()
            },
        );
        let response = current_app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/statehub/cohort_health")
                    .body(AxumBody::empty())
                    .expect("request"),
            )
            .await
            .expect("current projection response");
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read current body");
        let current: Value = serde_json::from_slice(&body).expect("parse current projection");
        assert_eq!(current["id"], "cohort_health");
        assert_eq!(current["version"], 1);
        assert_eq!(current["data"]["agent_count"], 3);
        assert_eq!(current["source_lenses"], serde_json::json!(["cohort"]));

        state.state_hub.update_projection(
            "cohort_health",
            serde_json::json!({"agent_count": 4}),
            "cohort",
        );
        state.state_hub.update_projection(
            "cohort_health",
            serde_json::json!({"agent_count": 5}),
            "vitality",
        );

        let history_app = build_router(
            Arc::clone(&state),
            &[],
            ServeAuthConfig {
                enabled: false,
                ..ServeAuthConfig::default()
            },
        );
        let response = history_app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/statehub/cohort_health/history?from_version=2&limit=1")
                    .body(AxumBody::empty())
                    .expect("request"),
            )
            .await
            .expect("projection history response");
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read history body");
        let history: Value = serde_json::from_slice(&body).expect("parse projection history");
        assert_eq!(history["projection_id"], "cohort_health");
        assert_eq!(history["retained"], 3);
        assert_eq!(history["matched"], 2);
        assert_eq!(history["retention_seconds"], 7 * 24 * 60 * 60);
        assert_eq!(history["retention_ms"], 7 * 24 * 60 * 60 * 1_000);
        assert_eq!(history["history"].as_array().unwrap().len(), 1);
        assert_eq!(history["history"][0]["version"], 3);
        assert_eq!(history["history"][0]["data"]["agent_count"], 5);

        let response = build_router(
            Arc::clone(&state),
            &[],
            ServeAuthConfig {
                enabled: false,
                ..ServeAuthConfig::default()
            },
        )
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/statehub/cohort_health/history?resolution=1000000d")
                .body(AxumBody::empty())
                .expect("resolution request"),
        )
        .await
        .expect("coalesced projection history response");
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read coalesced history body");
        let history: Value = serde_json::from_slice(&body).expect("parse coalesced history");
        assert_eq!(history["matched"], 3);
        assert_eq!(history["coalesced"], 1);
        assert_eq!(history["resolution"], "1000000d");
        assert_eq!(history["history"].as_array().unwrap().len(), 1);
        assert_eq!(history["history"][0]["version"], 3);
    }

    #[test]
    fn history_resolution_parser_accepts_units_and_rejects_bad_values() {
        assert_eq!(parse_history_resolution(None).unwrap(), None);
        assert_eq!(parse_history_resolution(Some("250ms")).unwrap(), Some(250));
        assert_eq!(parse_history_resolution(Some("60s")).unwrap(), Some(60_000));
        assert_eq!(parse_history_resolution(Some("2m")).unwrap(), Some(120_000));
        assert_eq!(
            parse_history_resolution(Some("1d")).unwrap(),
            Some(86_400_000)
        );
        assert!(parse_history_resolution(Some("0s")).is_err());
        assert!(parse_history_resolution(Some("1")).is_err());
        assert!(parse_history_resolution(Some("1fortnight")).is_err());
        assert!(parse_history_resolution(Some("18446744073709551615d")).is_err());
    }

    #[tokio::test]
    async fn lens_runtime_routes_inspect_disable_enable_and_reset() {
        let (_dir, state) = test_state();
        let mut registry = LensRegistry::new();
        registry
            .register(LensConfig {
                name: "cost-main".into(),
                block: "roko:cost-lens@1".into(),
                scope: "global".into(),
                params: BTreeMap::new(),
            })
            .unwrap();
        let _queue = LensExecutor::from_registry(&registry, state.state_hub.sender())
            .unwrap()
            .into_queued("serve-test", LensQueueConfig::default())
            .unwrap();
        let auth = || ServeAuthConfig {
            enabled: false,
            ..ServeAuthConfig::default()
        };

        let response = build_router(Arc::clone(&state), &[], auth())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/statehub/lens-runtimes/serve-test/cost-main/disable")
                    .body(AxumBody::empty())
                    .expect("disable request"),
            )
            .await
            .expect("disable response");
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read disable body");
        let disabled: Value = serde_json::from_slice(&body).expect("parse disable response");
        assert_eq!(disabled["runtime_id"], "serve-test");
        assert_eq!(disabled["lenses"][0]["enabled"], false);

        let response = build_router(Arc::clone(&state), &[], auth())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/statehub/lens-runtimes/serve-test/cost-main/enable")
                    .body(AxumBody::empty())
                    .expect("enable request"),
            )
            .await
            .expect("enable response");
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read enable body");
        let enabled: Value = serde_json::from_slice(&body).expect("parse enable response");
        assert_eq!(enabled["lenses"][0]["enabled"], true);
        assert_eq!(enabled["lenses"][0]["breaker_stage"], "sampled");

        let response = build_router(Arc::clone(&state), &[], auth())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/statehub/lens-runtimes/serve-test/cost-main/reset")
                    .body(AxumBody::empty())
                    .expect("reset request"),
            )
            .await
            .expect("reset response");
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let response = build_router(state, &[], auth())
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/statehub/lens-runtimes")
                    .body(AxumBody::empty())
                    .expect("runtime list request"),
            )
            .await
            .expect("runtime list response");
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read runtime list body");
        let runtimes: Value = serde_json::from_slice(&body).expect("parse runtime list response");
        assert_eq!(runtimes["runtimes"].as_array().unwrap().len(), 1);
        assert_eq!(runtimes["runtimes"][0]["runtime_id"], "serve-test");
    }

    #[tokio::test]
    async fn graph_lifecycle_reaches_multiple_builtin_projection_http_surfaces() {
        let (_dir, state) = test_state();
        let graph = roko_graph::loader::load_from_str(
            r#"
[graph]
name = "http-observed"

[[lenses]]
name = "cost-monitor"
block = "roko:cost-lens@^1.0"
scope = "graph"

[[lenses]]
name = "latency-monitor"
block = "roko:latency-lens@^1.0"
scope = "graph"

[[nodes]]
id = "work"
cell_type = "costed"
"#,
        )
        .expect("load graph and Lens configuration");
        let executor =
            roko_runtime::LensExecutor::from_registry(&graph.lenses, state.state_hub.sender())
                .expect("build configured Lens executor");
        let mut cells = roko_graph::CellRegistry::new();
        cells.register("costed", |_| Box::new(CostedCell));
        let output = roko_graph::GraphEngine::new(graph, cells)
            .with_telemetry(Arc::new(executor))
            .execute(&roko_graph::CellContext::new().with_run_id("http-run".into()))
            .await
            .expect("execute observed graph");
        assert!(output.success);

        let auth = || ServeAuthConfig {
            enabled: false,
            ..ServeAuthConfig::default()
        };
        let response = build_router(Arc::clone(&state), &[], auth())
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/statehub/cost_meter")
                    .body(AxumBody::empty())
                    .expect("statehub request"),
            )
            .await
            .expect("statehub projection response");
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read statehub projection");
        let current: Value = serde_json::from_slice(&body).expect("parse statehub projection");
        assert_eq!(current["id"], "cost_meter");
        assert_eq!(current["version"], 1);
        assert_eq!(current["data"]["total_usd"], 0.25);
        assert_eq!(
            current["source_lenses"],
            serde_json::json!(["cost-monitor"])
        );

        let response = build_router(Arc::clone(&state), &[], auth())
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/statehub/active_tasks")
                    .body(AxumBody::empty())
                    .expect("latency statehub request"),
            )
            .await
            .expect("latency statehub projection response");
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read latency statehub projection");
        let latency: Value = serde_json::from_slice(&body).expect("parse latency projection");
        assert_eq!(latency["id"], "active_tasks");
        // Cell, graph-node, and graph completion are three distinct latency
        // targets and each commits one replacement projection.
        assert_eq!(latency["version"], 3);
        assert_eq!(
            latency["source_lenses"],
            serde_json::json!(["latency-monitor"])
        );

        let response = build_router(state, &[], auth())
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/projections/cost_meter")
                    .body(AxumBody::empty())
                    .expect("projection request"),
            )
            .await
            .expect("generic projection response");
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read generic projection");
        let projected: Value = serde_json::from_slice(&body).expect("parse generic projection");
        assert_eq!(projected["canonical_name"], "cost_meter");
        assert_eq!(projected["data"]["total_usd"], 0.25);
    }

    #[tokio::test]
    async fn generic_projection_prefers_live_statehub_value() {
        let (_dir, state) = test_state();
        state.state_hub.update_projection(
            "c_factor",
            serde_json::json!({
                "c_factor": 0.73,
                "components": {"coordination": 0.8},
                "trend": "rising",
                "agent_diversity": 0.61
            }),
            "c_factor",
        );
        let app = build_router(
            state,
            &[],
            ServeAuthConfig {
                enabled: false,
                ..ServeAuthConfig::default()
            },
        );
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/projections/c_factor")
                    .body(AxumBody::empty())
                    .expect("request"),
            )
            .await
            .expect("generic projection response");
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let payload: Value = serde_json::from_slice(&body).expect("parse projection");
        assert_eq!(payload["canonical_name"], "c_factor");
        assert_eq!(payload["data"]["c_factor"], 0.73);
        assert_eq!(payload["data"]["trend"], "rising");
    }

    #[tokio::test]
    async fn projection_telemetry_alias_resolves() {
        use crate::projection_contract::canonical_projection_name;

        assert_eq!(canonical_projection_name("telemetry"), "telemetry");
        assert_eq!(canonical_projection_name("watchers"), "telemetry");
        assert_eq!(canonical_projection_name("circuit_breakers"), "telemetry");
        assert_eq!(canonical_projection_name("observations"), "telemetry");
    }
}
