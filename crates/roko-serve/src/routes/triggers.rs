//! Trigger binding CRUD and manual fire endpoints.
//!
//! - `GET    /api/triggers`             — list all trigger bindings
//! - `GET    /api/triggers/{name}`      — get a specific trigger binding
//! - `POST   /api/triggers`             — create a new trigger binding
//! - `POST   /api/triggers/{name}/fire` — manually fire a trigger
//! - `DELETE /api/triggers/{name}`      — remove a trigger binding

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::body::to_bytes;
use axum::extract::{Path, Query, Request, State};
use axum::http::{Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use roko_core::trigger::{
    TriggerBinding, TriggerEvent, TriggerHistory, TriggerSource, load_trigger_history,
};

use crate::error::ApiError;
use crate::state::AppState;
use crate::trigger_runtime::{TriggerSubmitStatus, ensure_trigger_runtime, verify_webhook_auth};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/triggers", get(list_triggers).post(create_trigger))
        .route("/triggers/{name}", get(get_trigger).delete(delete_trigger))
        .route("/triggers/{name}/history", get(trigger_history))
        .route("/triggers/{name}/fire", axum::routing::post(fire_trigger))
}

/// Public dynamic webhook ingress. Static routes take precedence over this
/// catch-all; unmatched GET requests retain the embedded SPA fallback.
pub fn public_routes() -> Router<Arc<AppState>> {
    Router::new().route("/{*path}", any(webhook_or_spa))
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
    status: String,
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

#[derive(Debug, Deserialize)]
struct TriggerHistoryQuery {
    #[serde(default = "default_history_limit")]
    limit: usize,
}

const fn default_history_limit() -> usize {
    20
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
    let mut triggers: Vec<TriggerBinding> = bindings.values().cloned().collect();
    triggers.sort_by(|left, right| left.name.cmp(&right.name));
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

/// `GET /api/triggers/{name}/history` — read durable firings with Flow refs.
async fn trigger_history(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Query(query): Query<TriggerHistoryQuery>,
) -> Result<Json<TriggerHistory>, ApiError> {
    if !state.trigger_bindings.read().await.contains_key(&name) {
        return Err(ApiError::not_found(format!("trigger '{name}' not found")));
    }
    if query.limit == 0 || query.limit > 1_000 {
        return Err(ApiError::bad_request(
            "history limit must be between 1 and 1000",
        ));
    }
    load_trigger_history(&state.layout.triggers_dir(), &name, query.limit)
        .map(Json)
        .map_err(|error| ApiError::internal(format!("read trigger history: {error}")))
}

/// `POST /api/triggers` — create a new trigger binding.
async fn create_trigger(
    State(state): State<Arc<AppState>>,
    Json(binding): Json<TriggerBinding>,
) -> Result<(StatusCode, Json<TriggerBinding>), ApiError> {
    binding
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;

    let mut bindings = state.trigger_bindings.write().await;
    if bindings.contains_key(&binding.name) {
        return Err(ApiError::conflict(format!(
            "trigger '{}' already exists",
            binding.name
        )));
    }
    let path = trigger_path(&state, &binding.name);
    binding
        .save_to_file(&path)
        .map_err(|error| ApiError::internal(format!("persist trigger binding: {error}")))?;
    bindings.insert(binding.name.clone(), binding.clone());
    let snapshot = bindings.values().cloned().collect();
    drop(bindings);

    ensure_trigger_runtime(&state)
        .await
        .reconcile(snapshot)
        .await
        .map_err(ApiError::from)?;

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
        .ok_or_else(|| ApiError::not_found(format!("trigger '{name}' not found")))?
        .clone();
    drop(bindings);

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
    let mut event = TriggerEvent::new(
        name.clone(),
        req.payload,
        TriggerSource::Manual { user: req.user },
        trace_id,
    );
    if let Some(space_id) = &binding.space {
        event = event.with_space(space_id.clone());
    }

    let status = ensure_trigger_runtime(&state)
        .await
        .submit(event.clone())
        .await
        .map_err(ApiError::from)?;

    Ok(Json(FireTriggerResponse {
        trigger_name: name,
        fired: true,
        event,
        status: submit_status_label(status).to_string(),
    }))
}

/// `DELETE /api/triggers/{name}` — remove a trigger binding.
async fn delete_trigger(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<DeleteTriggerResponse>, ApiError> {
    let mut bindings = state.trigger_bindings.write().await;
    if !bindings.contains_key(&name) {
        return Err(ApiError::not_found(format!("trigger '{name}' not found")));
    }

    let path = trigger_path(&state, &name);
    if path.exists() {
        std::fs::remove_file(&path)
            .map_err(|error| ApiError::internal(format!("remove trigger binding: {error}")))?;
    }
    bindings.remove(&name);
    let snapshot = bindings.values().cloned().collect();
    drop(bindings);
    ensure_trigger_runtime(&state)
        .await
        .reconcile(snapshot)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(DeleteTriggerResponse {
        name,
        deleted: true,
    }))
}

// ── Helpers ─────────────────────────────────────────────────────

fn trigger_path(state: &AppState, name: &str) -> std::path::PathBuf {
    state.layout.triggers_dir().join(format!("{name}.toml"))
}

fn submit_status_label(status: TriggerSubmitStatus) -> &'static str {
    match status {
        TriggerSubmitStatus::Started => "started",
        TriggerSubmitStatus::Queued => "queued",
        TriggerSubmitStatus::Suppressed => "suppressed",
    }
}

async fn webhook_or_spa(State(state): State<Arc<AppState>>, request: Request) -> Response {
    let path = request.uri().path().to_string();
    let method = request.method().clone();
    let bindings = state.trigger_bindings.read().await;
    let path_binding = bindings.values().find(|binding| {
        matches!(&binding.kind, roko_core::trigger::TriggerKind::Webhook(config) if config.path == path)
    });
    let binding = path_binding.and_then(|binding| {
        let roko_core::trigger::TriggerKind::Webhook(config) = &binding.kind else {
            return None;
        };
        config
            .method
            .as_ref()
            .is_none_or(|expected| expected.eq_ignore_ascii_case(method.as_str()))
            .then(|| binding.clone())
    });
    let path_exists = path_binding.is_some();
    drop(bindings);

    let Some(binding) = binding else {
        if path_exists {
            return StatusCode::METHOD_NOT_ALLOWED.into_response();
        }
        if method == Method::GET || method == Method::HEAD {
            return crate::serve_api_or_spa_fallback(request).await;
        }
        return StatusCode::NOT_FOUND.into_response();
    };
    if !binding.enabled {
        return ApiError::not_found("webhook trigger is disabled").into_response();
    }

    let client_identity = request
        .extensions()
        .get::<crate::trigger_tls::VerifiedClientIdentity>()
        .cloned();
    let headers = request.headers().clone();
    let body = match to_bytes(request.into_body(), 10 * 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => {
            return ApiError::bad_request(format!("read webhook body: {error}")).into_response();
        }
    };
    let runtime = ensure_trigger_runtime(&state).await;
    if let Err(error) =
        verify_webhook_auth(&state, &binding, &headers, &body, client_identity.as_ref())
    {
        runtime
            .record_source_error(
                binding.name.clone(),
                json!({"phase": "authentication", "error": error.to_string()}),
            )
            .await;
        return ApiError::unauthorized("webhook authentication failed").into_response();
    }

    let payload = serde_json::from_slice(&body).unwrap_or_else(|_| {
        String::from_utf8(body.to_vec()).map_or_else(
            |_| {
                json!({
                    "base64": base64::Engine::encode(
                        &base64::engine::general_purpose::STANDARD,
                        &body,
                    ),
                })
            },
            Value::String,
        )
    });
    let mut selected_headers: BTreeMap<String, String> = headers
        .iter()
        .filter(|(name, _)| {
            matches!(
                name.as_str(),
                "content-type" | "user-agent" | "x-request-id" | "x-github-event"
            )
        })
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.to_string(), value.to_string()))
        })
        .collect();
    if let Some(identity) = client_identity {
        selected_headers.insert(
            "tls-client-certificate-sha256".to_string(),
            identity.certificate_sha256,
        );
    }
    let mut event = TriggerEvent::new(
        binding.name.clone(),
        payload,
        TriggerSource::Webhook {
            method: method.to_string(),
            path,
            headers: selected_headers,
        },
        uuid::Uuid::new_v4().to_string(),
    );
    if let Some(space_id) = &binding.space {
        event = event.with_space(space_id.clone());
    }
    match runtime.submit(event.clone()).await {
        Ok(status) => (
            StatusCode::ACCEPTED,
            Json(json!({
                "trigger_name": binding.name,
                "status": submit_status_label(status),
                "event": event,
            })),
        )
            .into_response(),
        Err(error) => ApiError::internal(error.to_string()).into_response(),
    }
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
    async fn trigger_history_returns_durable_event_with_flow_reference() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".roko")).expect("create .roko");
        let state = test_state(dir.path().to_path_buf());
        let app = routes().with_state(Arc::clone(&state));

        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/triggers")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "name": "historical",
                            "kind": { "type": "manual" },
                            "graph": "plans/history.toml",
                            "concurrency": { "kind": "skip" },
                            "enabled": true
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(created.status(), StatusCode::CREATED);

        let fired = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/triggers/historical/fire")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"payload":{"revision":7}}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(fired.status(), StatusCode::OK);

        let history = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                let response = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .method("GET")
                            .uri("/triggers/historical/history?limit=1")
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(response.status(), StatusCode::OK);
                let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
                let history: TriggerHistory = serde_json::from_slice(&bytes).unwrap();
                let complete = history.records.first().is_some_and(|record| {
                    record.lifecycle.iter().any(|event| {
                        event.kind == roko_core::trigger::TriggerEventKind::FlowCompleted
                            && event.detail.get("run_id").and_then(Value::as_str).is_some()
                    })
                });
                if complete {
                    break history;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("history completed");

        assert_eq!(history.total, 1);
        assert_eq!(history.records[0].event.payload["revision"], 7);
        let trace = &history.records[0].event.trace_id;
        assert!(
            history.records[0]
                .lifecycle
                .iter()
                .all(|event| event.trace_id.as_ref() == Some(trace))
        );
        state.cancel.cancel();
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

    #[tokio::test]
    async fn trigger_create_rejects_path_traversal() {
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
                        serde_json::to_vec(&json!({
                            "name": "../escape",
                            "kind": { "type": "manual" },
                            "graph": "../../outside.toml",
                            "concurrency": { "kind": "skip" },
                            "enabled": true
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(!dir.path().join("escape.toml").exists());
    }

    #[tokio::test]
    async fn dynamic_webhook_authenticates_and_submits_to_runtime() {
        use roko_core::trigger::{SecretRef, TriggerAuth, TriggerKind, WebhookTrigger};

        let dir = tempfile::tempdir().expect("tempdir");
        let layout = roko_fs::RokoLayout::for_project(dir.path());
        std::fs::create_dir_all(layout.root()).unwrap();
        std::fs::write(layout.root().join("webhook-secret"), "expected-token\n").unwrap();
        let mut binding = TriggerBinding::new(
            "incoming",
            TriggerKind::Webhook(WebhookTrigger {
                method: Some("POST".to_string()),
                path: "/hook/incoming".to_string(),
            }),
            "graphs/incoming.toml",
        );
        binding.auth = Some(TriggerAuth::BearerToken {
            secret: SecretRef::File {
                path: std::path::PathBuf::from(".roko/webhook-secret"),
            },
        });
        binding
            .save_to_file(&layout.triggers_dir().join("incoming.toml"))
            .unwrap();
        let state = test_state(dir.path().to_path_buf());
        let app = public_routes().with_state(Arc::clone(&state));

        let rejected = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/hook/incoming")
                    .header("authorization", "Bearer wrong-token")
                    .body(Body::from(r#"{"event":"push"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(rejected.status(), StatusCode::UNAUTHORIZED);

        let accepted = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/hook/incoming")
                    .header("authorization", "Bearer expected-token")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"event":"push"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(accepted.status(), StatusCode::ACCEPTED);
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            let events = layout.triggers_dir().join("events");
            while !events.is_dir()
                || std::fs::read_dir(&events)
                    .map(|entries| entries.count() == 0)
                    .unwrap_or(true)
            {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("webhook event persisted");
        state.cancel.cancel();
    }

    #[tokio::test]
    async fn mutual_tls_webhook_requires_transport_verified_identity() {
        use roko_core::trigger::{SecretRef, TriggerAuth, TriggerKind, WebhookTrigger};

        let dir = tempfile::tempdir().expect("tempdir");
        let layout = roko_fs::RokoLayout::for_project(dir.path());
        let mut binding = TriggerBinding::new(
            "mtls-incoming",
            TriggerKind::Webhook(WebhookTrigger {
                method: Some("POST".to_string()),
                path: "/hook/mtls".to_string(),
            }),
            "graphs/incoming.toml",
        );
        binding.auth = Some(TriggerAuth::MutualTls {
            cert: "server.pem".into(),
            key: SecretRef::File {
                path: "server.key".into(),
            },
            client_ca: "client-ca.pem".into(),
        });
        binding
            .save_to_file(&layout.triggers_dir().join("mtls-incoming.toml"))
            .unwrap();
        let state = test_state(dir.path().to_path_buf());
        let app = public_routes().with_state(Arc::clone(&state));

        let unauthenticated = Request::builder()
            .method("POST")
            .uri("/hook/mtls")
            .body(Body::from("{}"))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(unauthenticated).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );

        let mut authenticated = Request::builder()
            .method("POST")
            .uri("/hook/mtls")
            .body(Body::from("{}"))
            .unwrap();
        authenticated
            .extensions_mut()
            .insert(crate::trigger_tls::VerifiedClientIdentity {
                certificate_sha256: "transport-verified".to_string(),
            });
        assert_eq!(
            app.oneshot(authenticated).await.unwrap().status(),
            StatusCode::ACCEPTED
        );
        state.cancel.cancel();
    }
}
