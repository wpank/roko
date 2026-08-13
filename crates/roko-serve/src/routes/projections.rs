//! StateHub-backed projection routes for remote read and watch flows.

use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::get;
use axum::{Json, Router};
use futures::stream::{self, Stream, StreamExt};
use serde_json::{Value, json};
use tokio::sync::broadcast;
use tracing::warn;

use crate::error::ApiError;
use crate::projection_contract::{
    ProjectionQuery, RuntimeProjectionSet, projection_accepts_event, projection_delta_frame,
};
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/projections/catalog", get(projections_catalog))
        .route("/projections/telemetry", get(get_telemetry))
        .route("/projections/telemetry/stream", get(stream_telemetry))
        .route("/projections/{name}", get(get_projection))
        .route("/projections/{name}/stream", get(stream_projection))
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
    let delta_stream = stream::unfold(state.state_hub.subscribe_events(), move |mut rx| {
        let query = query_for_stream.clone();
        async move {
            loop {
                match rx.recv().await {
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
                        return Some((Ok(event), rx));
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!(projection = "telemetry", skipped, "telemetry stream lagged");
                    }
                    Err(broadcast::error::RecvError::Closed) => return None,
                }
            }
        }
    });

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
    let delta_stream = stream::unfold(state.state_hub.subscribe_events(), move |mut rx| {
        let name = name_for_stream.clone();
        let query = query_for_stream.clone();
        async move {
            loop {
                match rx.recv().await {
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
                        return Some((Ok(event), rx));
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!(projection = %name, skipped, "projection stream lagged");
                    }
                    Err(broadcast::error::RecvError::Closed) => return None,
                }
            }
        }
    });

    Ok(
        Sse::new(stream::once(async move { Ok(initial) }).chain(delta_stream))
            .keep_alive(KeepAlive::default()),
    )
}

#[cfg(test)]
mod tests {
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
    async fn projection_telemetry_alias_resolves() {
        use crate::projection_contract::canonical_projection_name;

        assert_eq!(canonical_projection_name("telemetry"), "telemetry");
        assert_eq!(canonical_projection_name("watchers"), "telemetry");
        assert_eq!(canonical_projection_name("circuit_breakers"), "telemetry");
        assert_eq!(canonical_projection_name("observations"), "telemetry");
    }
}
