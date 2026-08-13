//! Trigger binding CRUD and manual fire endpoints.
//!
//! - `GET    /api/triggers`             — list all trigger bindings
//! - `GET    /api/triggers/{name}`      — get a specific trigger binding
//! - `POST   /api/triggers`             — create a new trigger binding
//! - `POST   /api/triggers/{name}/fire` — manually fire a trigger
//! - `DELETE /api/triggers/{name}`      — remove a trigger binding

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use roko_core::trigger::{TriggerBinding, TriggerEvent, TriggerSource};

use crate::error::ApiError;
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/triggers", get(list_triggers).post(create_trigger))
        .route("/triggers/{name}", get(get_trigger).delete(delete_trigger))
        .route("/triggers/{name}/fire", axum::routing::post(fire_trigger))
}

// ── Response types ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct TriggerListResponse {
    triggers: Vec<TriggerBinding>,
    total: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct DeleteTriggerResponse {
    name: String,
    deleted: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct FireTriggerResponse {
    trigger_name: String,
    fired: bool,
    event: TriggerEvent,
}

// ── Request types ───────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct FireTriggerRequest {
    /// Optional payload to include in the fired event.
    #[serde(default = "default_fire_payload")]
    payload: Value,
    /// Optional user attribution for the manual fire.
    #[serde(default = "default_fire_user")]
    user: String,
}

fn default_fire_payload() -> Value {
    json!({})
}

fn default_fire_user() -> String {
    "api".to_string()
}

// ── Handlers ────────────────────────────────────────────────────

/// `GET /api/triggers` — list all trigger bindings.
async fn list_triggers(State(state): State<Arc<AppState>>) -> Json<TriggerListResponse> {
    let bindings = state.trigger_bindings.read().await;
    let triggers: Vec<TriggerBinding> = bindings.values().cloned().collect();
    let total = triggers.len();
    Json(TriggerListResponse { triggers, total })
}

/// `GET /api/triggers/{name}` — get a specific trigger binding by name.
async fn get_trigger(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<TriggerBinding>, ApiError> {
    let bindings = state.trigger_bindings.read().await;
    let binding = bindings
        .get(&name)
        .ok_or_else(|| ApiError::not_found(format!("trigger '{name}' not found")))?;
    Ok(Json(binding.clone()))
}

/// `POST /api/triggers` — create a new trigger binding.
async fn create_trigger(
    State(state): State<Arc<AppState>>,
    Json(binding): Json<TriggerBinding>,
) -> Result<(StatusCode, Json<TriggerBinding>), ApiError> {
    if binding.name.trim().is_empty() {
        return Err(ApiError::bad_request("trigger name must not be empty"));
    }
    if binding.graph.trim().is_empty() {
        return Err(ApiError::bad_request("trigger graph must not be empty"));
    }

    let mut bindings = state.trigger_bindings.write().await;
    if bindings.contains_key(&binding.name) {
        return Err(ApiError::conflict(format!(
            "trigger '{}' already exists",
            binding.name
        )));
    }
    bindings.insert(binding.name.clone(), binding.clone());

    // Persist to `.roko/triggers/{name}.toml`.
    let path = trigger_path(&state, &binding.name);
    write_trigger_file(&path, &binding).await?;

    Ok((StatusCode::CREATED, Json(binding)))
}

/// `POST /api/triggers/{name}/fire` — manually fire a trigger.
async fn fire_trigger(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    body: Option<Json<FireTriggerRequest>>,
) -> Result<Json<FireTriggerResponse>, ApiError> {
    let bindings = state.trigger_bindings.read().await;
    let binding = bindings
        .get(&name)
        .ok_or_else(|| ApiError::not_found(format!("trigger '{name}' not found")))?;

    if !binding.enabled {
        return Err(ApiError::bad_request(format!(
            "trigger '{name}' is disabled"
        )));
    }

    let req = body.map(|b| b.0).unwrap_or(FireTriggerRequest {
        payload: default_fire_payload(),
        user: default_fire_user(),
    });

    let trace_id = uuid::Uuid::new_v4().to_string();
    let event = TriggerEvent::new(
        name.clone(),
        req.payload,
        TriggerSource::Manual { user: req.user },
        trace_id,
    );

    // Publish the event on the server event bus so downstream subscribers
    // (subscriptions, SSE clients) can react to it.
    let _ = state
        .event_bus
        .publish(crate::events::ServerEvent::TriggerFired {
            trigger_name: name.clone(),
            event: event.clone(),
        });

    Ok(Json(FireTriggerResponse {
        trigger_name: name,
        fired: true,
        event,
    }))
}

/// `DELETE /api/triggers/{name}` — remove a trigger binding.
async fn delete_trigger(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<DeleteTriggerResponse>, ApiError> {
    let mut bindings = state.trigger_bindings.write().await;
    let removed = bindings.remove(&name);
    if removed.is_none() {
        return Err(ApiError::not_found(format!("trigger '{name}' not found")));
    }

    // Remove the persisted TOML file if it exists.
    let path = trigger_path(&state, &name);
    if path.exists() {
        let _ = tokio::fs::remove_file(&path).await;
    }

    Ok(Json(DeleteTriggerResponse {
        name,
        deleted: true,
    }))
}

// ── Helpers ─────────────────────────────────────────────────────

fn trigger_path(state: &AppState, name: &str) -> std::path::PathBuf {
    state
        .workdir
        .join(".roko")
        .join("triggers")
        .join(format!("{name}.toml"))
}

async fn write_trigger_file(
    path: &std::path::Path,
    binding: &TriggerBinding,
) -> Result<(), ApiError> {
    let parent = path
        .parent()
        .ok_or_else(|| ApiError::internal("invalid trigger path"))?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|e| ApiError::internal(format!("create triggers dir: {e}")))?;

    let rendered = toml::to_string_pretty(binding)
        .map_err(|e| ApiError::internal(format!("serialize trigger: {e}")))?;
    tokio::fs::write(path, rendered)
        .await
        .map_err(|e| ApiError::internal(format!("write trigger: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use roko_core::config::schema::RokoConfig;
    use tower::ServiceExt;

    use crate::deploy::create_backend;
    use crate::runtime::NoOpRuntime;

    fn test_state(workdir: std::path::PathBuf) -> Arc<AppState> {
        let deploy_backend =
            Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
        Arc::new(
            AppState::new(
                workdir,
                Arc::new(NoOpRuntime),
                RokoConfig::default(),
                deploy_backend,
            )
            .expect("AppState::new"),
        )
    }

    #[tokio::test]
    async fn trigger_list_empty() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let state = test_state(dir.path().to_path_buf());

        let response = routes()
            .with_state(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/triggers")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: TriggerListResponse = serde_json::from_slice(&body).expect("parse");
        assert!(payload.triggers.is_empty());
        assert_eq!(payload.total, 0);
    }

    #[tokio::test]
    async fn trigger_create_then_get() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let state = test_state(dir.path().to_path_buf());
        let app = routes().with_state(Arc::clone(&state));

        // Create a manual trigger.
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/triggers")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&json!({
                            "name": "my-trigger",
                            "kind": { "type": "manual" },
                            "graph": "plans/test.toml",
                            "concurrency": { "kind": "skip" },
                            "enabled": true
                        }))
                        .unwrap(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let created: TriggerBinding = serde_json::from_slice(&body).expect("parse");
        assert_eq!(created.name, "my-trigger");
        assert_eq!(created.graph, "plans/test.toml");
        assert!(created.enabled);

        // GET by name.
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/triggers/my-trigger")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let fetched: TriggerBinding = serde_json::from_slice(&body).expect("parse");
        assert_eq!(fetched.name, "my-trigger");

        // List should show 1 trigger.
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/triggers")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: TriggerListResponse = serde_json::from_slice(&body).expect("parse");
        assert_eq!(payload.total, 1);
    }

    #[tokio::test]
    async fn trigger_create_duplicate_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let state = test_state(dir.path().to_path_buf());
        let app = routes().with_state(Arc::clone(&state));

        let body_str = serde_json::to_string(&json!({
            "name": "dupe",
            "kind": { "type": "manual" },
            "graph": "plans/x.toml",
            "concurrency": { "kind": "skip" },
            "enabled": true
        }))
        .unwrap();

        // First create succeeds.
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/triggers")
                    .header("content-type", "application/json")
                    .body(Body::from(body_str.clone()))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::CREATED);

        // Second create with same name returns 409.
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/triggers")
                    .header("content-type", "application/json")
                    .body(Body::from(body_str))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn trigger_delete_success() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let state = test_state(dir.path().to_path_buf());
        let app = routes().with_state(Arc::clone(&state));

        // Create first.
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/triggers")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&json!({
                            "name": "del-me",
                            "kind": { "type": "manual" },
                            "graph": "plans/y.toml",
                            "concurrency": { "kind": "skip" },
                            "enabled": true
                        }))
                        .unwrap(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        // Delete.
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/triggers/del-me")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: DeleteTriggerResponse = serde_json::from_slice(&body).expect("parse");
        assert!(payload.deleted);
        assert_eq!(payload.name, "del-me");

        // GET should now return 404.
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/triggers/del-me")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn trigger_delete_not_found() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let state = test_state(dir.path().to_path_buf());

        let response = routes()
            .with_state(state)
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/triggers/ghost")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn trigger_get_not_found() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let state = test_state(dir.path().to_path_buf());

        let response = routes()
            .with_state(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/triggers/nonexistent")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn trigger_fire_success() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let state = test_state(dir.path().to_path_buf());
        let app = routes().with_state(Arc::clone(&state));

        // Create a trigger first.
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/triggers")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&json!({
                            "name": "fireable",
                            "kind": { "type": "manual" },
                            "graph": "plans/fire.toml",
                            "concurrency": { "kind": "skip" },
                            "enabled": true
                        }))
                        .unwrap(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        // Fire it.
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/triggers/fireable/fire")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&json!({
                            "payload": { "reason": "test" },
                            "user": "test-runner"
                        }))
                        .unwrap(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: FireTriggerResponse = serde_json::from_slice(&body).expect("parse");
        assert!(payload.fired);
        assert_eq!(payload.trigger_name, "fireable");
        assert_eq!(payload.event.trigger_id, "fireable");
    }

    #[tokio::test]
    async fn trigger_fire_not_found() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let state = test_state(dir.path().to_path_buf());

        let response = routes()
            .with_state(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/triggers/missing/fire")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn trigger_fire_disabled_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let state = test_state(dir.path().to_path_buf());
        let app = routes().with_state(Arc::clone(&state));

        // Create a disabled trigger.
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/triggers")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&json!({
                            "name": "disabled-trig",
                            "kind": { "type": "manual" },
                            "graph": "plans/d.toml",
                            "concurrency": { "kind": "skip" },
                            "enabled": false
                        }))
                        .unwrap(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        // Try to fire disabled trigger.
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/triggers/disabled-trig/fire")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn trigger_create_empty_name_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let state = test_state(dir.path().to_path_buf());

        let response = routes()
            .with_state(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/triggers")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&json!({
                            "name": "  ",
                            "kind": { "type": "manual" },
                            "graph": "plans/x.toml",
                            "concurrency": { "kind": "skip" },
                            "enabled": true
                        }))
                        .unwrap(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
