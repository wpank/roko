//! Subscription CRUD endpoints.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;

use roko_core::config::schema::SubscriptionConfig;

use crate::dispatch::Subscription;
use crate::error::ApiError;
use crate::extract::{RequestPayload, ValidJson};
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/subscriptions/catalog", get(subscriptions_catalog))
        .route(
            "/subscriptions/relay/status",
            get(relay_subscription_status),
        )
        .route(
            "/subscriptions",
            get(list_subscriptions).post(create_subscription),
        )
        .route(
            "/subscriptions/{id}",
            put(update_subscription).delete(delete_subscription),
        )
        .route("/subscriptions/{id}/enable", post(enable_subscription))
        .route("/subscriptions/{id}/disable", post(disable_subscription))
}

/// `GET /api/subscriptions/relay/status` — durable relay-consumer cursor and
/// reconciliation diagnostics. The parent API router applies normal read
/// authentication/scope enforcement when serve auth is enabled.
async fn relay_subscription_status(
    State(state): State<Arc<AppState>>,
) -> Json<crate::subscription_relay::SubscriptionRelayStatus> {
    Json(state.subscription_relay.status().await)
}

/// `GET /api/subscriptions/catalog` — describe available trigger types and filter fields.
async fn subscriptions_catalog() -> Json<serde_json::Value> {
    Json(json!({
        "trigger_types": [
            { "name": "webhook", "description": "Fires on incoming GitHub/GitLab webhooks" },
            { "name": "plan_completed", "description": "Fires when a plan execution finishes" },
            { "name": "gate_result", "description": "Fires on gate pass/fail verdicts" },
            { "name": "episode_recorded", "description": "Fires when an episode is persisted" },
            { "name": "job_transitioned", "description": "Fires on marketplace job state changes" },
            { "name": "cron", "description": "Fires on a cron schedule (via trigger_config)" },
            { "name": "file_watch", "description": "Fires when watched files change (via trigger_config)" },
        ],
        "filter_fields": [
            { "name": "repo", "type": "glob[]", "description": "Repository name glob patterns" },
            { "name": "branch", "type": "regex[]", "description": "Branch name regex patterns" },
            { "name": "path", "type": "glob[]", "description": "File path glob patterns" },
            { "name": "label", "type": "exact[]", "description": "Exact label matches" },
            { "name": "author", "type": "exact[]", "description": "Exact author matches" },
        ],
    }))
}

#[derive(Clone, Debug, Serialize)]
struct SubscriptionResponse {
    id: String,
    template: String,
    trigger: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    trigger_config: Option<roko_core::config::schema::SubscriptionTrigger>,
    filter: roko_core::config::schema::SubscriptionFilterConfig,
    concurrency_limit: usize,
    cooldown_secs: u64,
    debounce_ms: u64,
    enabled: bool,
    status: &'static str,
}

impl From<&Subscription> for SubscriptionResponse {
    fn from(subscription: &Subscription) -> Self {
        Self {
            id: subscription.id.clone(),
            template: subscription.template.clone(),
            trigger: subscription.trigger.clone(),
            trigger_config: subscription.trigger_config.clone(),
            filter: subscription.filter.clone(),
            concurrency_limit: subscription.concurrency_limit,
            cooldown_secs: subscription.cooldown_secs,
            debounce_ms: subscription.debounce_ms,
            enabled: subscription.enabled,
            status: subscription_status(subscription.enabled),
        }
    }
}

#[derive(Debug, Deserialize)]
struct SubscriptionUpdateRequest(SubscriptionConfig);

/// `GET /api/subscriptions` — list all subscriptions with their enabled status.
async fn list_subscriptions(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let subscriptions = state.subscriptions.all();
    let items: Vec<SubscriptionResponse> = subscriptions
        .iter()
        .map(SubscriptionResponse::from)
        .collect();
    Ok(Json(json!({ "subscriptions": items })))
}

/// `POST /api/subscriptions` — create a new subscription file and register it.
async fn create_subscription(
    State(state): State<Arc<AppState>>,
    ValidJson(body): ValidJson<SubscriptionConfig>,
) -> Result<impl IntoResponse, ApiError> {
    let config = body;
    let id = next_subscription_id(&state, &config);
    let path = subscription_path(&state, &id);

    write_subscription_file(&path, &config).await?;

    let mut subscription = Subscription::from_config(config);
    subscription.id = id.clone();
    state.subscriptions.insert(subscription.clone());

    Ok((
        axum::http::StatusCode::CREATED,
        Json(json!({ "subscription": SubscriptionResponse::from(&subscription) })),
    ))
}

/// `PUT /api/subscriptions/:id` — replace an existing subscription.
async fn update_subscription(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    ValidJson(body): ValidJson<SubscriptionUpdateRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let config = body.0;
    let path = subscription_path(&state, &id);

    if state.subscriptions.get_by_id(&id).is_none() {
        return Err(ApiError::not_found(format!(
            "subscription '{id}' not found"
        )));
    }

    write_subscription_file(&path, &config).await?;

    let mut subscription = Subscription::from_config(config);
    subscription.id = id.clone();
    let updated = state
        .subscriptions
        .update_by_id(&id, subscription)
        .ok_or_else(|| ApiError::not_found(format!("subscription '{id}' not found")))?;

    Ok(Json(
        json!({ "subscription": SubscriptionResponse::from(&updated) }),
    ))
}

/// `DELETE /api/subscriptions/:id` — remove the subscription file and registry entry.
async fn delete_subscription(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let path = subscription_path(&state, &id);
    let removed = state
        .subscriptions
        .remove_by_id(&id)
        .ok_or_else(|| ApiError::not_found(format!("subscription '{id}' not found")))?;

    if path.exists() {
        tokio::fs::remove_file(&path)
            .await
            .map_err(|e| ApiError::internal(format!("remove subscription file: {e}")))?;
    }

    Ok(Json(
        json!({ "deleted": true, "subscription": SubscriptionResponse::from(&removed) }),
    ))
}

/// `POST /api/subscriptions/:id/enable` — mark a subscription enabled.
async fn enable_subscription(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    set_subscription_enabled(&state, &id, true).await
}

/// `POST /api/subscriptions/:id/disable` — mark a subscription disabled.
async fn disable_subscription(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    set_subscription_enabled(&state, &id, false).await
}

async fn set_subscription_enabled(
    state: &Arc<AppState>,
    id: &str,
    enabled: bool,
) -> Result<Json<serde_json::Value>, ApiError> {
    let path = subscription_path(state, id);
    let current = state
        .subscriptions
        .get_by_id(id)
        .ok_or_else(|| ApiError::not_found(format!("subscription '{id}' not found")))?;

    let mut config = current.to_config();
    config.enabled = enabled;
    write_subscription_file(&path, &config).await?;

    let mut updated = Subscription::from_config(config);
    updated.id = id.to_string();
    let updated = state
        .subscriptions
        .update_by_id(id, updated)
        .ok_or_else(|| ApiError::not_found(format!("subscription '{id}' not found")))?;

    Ok(Json(
        json!({ "subscription": SubscriptionResponse::from(&updated) }),
    ))
}

fn validate_subscription(config: &SubscriptionConfig) -> Result<(), ApiError> {
    if config.template.trim().is_empty() {
        return Err(ApiError::bad_request(
            "subscription template must not be empty",
        ));
    }
    if config.trigger.trim().is_empty() {
        return Err(ApiError::bad_request(
            "subscription trigger must not be empty",
        ));
    }
    Ok(())
}

impl RequestPayload for SubscriptionConfig {
    fn validate_payload(&self) -> Result<(), ApiError> {
        validate_subscription(self)
    }
}

impl RequestPayload for SubscriptionUpdateRequest {
    fn validate_payload(&self) -> Result<(), ApiError> {
        validate_subscription(&self.0)
    }
}

fn subscription_status(enabled: bool) -> &'static str {
    if enabled { "enabled" } else { "disabled" }
}

fn subscription_path(state: &AppState, id: &str) -> std::path::PathBuf {
    state
        .workdir
        .join(".roko")
        .join("subscriptions")
        .join(format!("{id}.toml"))
}

fn next_subscription_id(state: &AppState, config: &SubscriptionConfig) -> String {
    let base = slugify_subscription_id(&config.template, &config.trigger);
    let mut candidate = base.clone();
    let mut suffix = 2usize;

    while state.subscriptions.get_by_id(&candidate).is_some()
        || subscription_path(state, &candidate).exists()
    {
        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }

    candidate
}

fn slugify_subscription_id(template: &str, trigger: &str) -> String {
    let mut slug = format!("{template}-{trigger}");
    slug = slug
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "subscription".to_string()
    } else {
        slug
    }
}

async fn write_subscription_file(
    path: &std::path::Path,
    config: &SubscriptionConfig,
) -> Result<(), ApiError> {
    let parent = path
        .parent()
        .ok_or_else(|| ApiError::internal("invalid subscription path"))?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|e| ApiError::internal(format!("create subscriptions dir: {e}")))?;

    let rendered = toml::to_string_pretty(config)
        .map_err(|e| ApiError::internal(format!("serialize subscription: {e}")))?;
    tokio::fs::write(path, rendered)
        .await
        .map_err(|e| ApiError::internal(format!("write subscription: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use roko_core::config::ApiKeyEntry;
    use roko_core::config::schema::{RokoConfig, SubscriptionConfig, SubscriptionTrigger};
    use tempfile::tempdir;
    use tower::ServiceExt;

    use super::*;
    use crate::deploy::create_backend;
    use crate::routes::{build_router, middleware};
    use crate::runtime::NoOpRuntime;

    #[test]
    fn runtime_subscription_preserves_trigger_config_and_debounce_roundtrip() {
        let config = SubscriptionConfig {
            template: "digest".to_string(),
            trigger: "workspace:updates".to_string(),
            trigger_config: Some(SubscriptionTrigger::Webhook {
                event: "workspace:updates".to_string(),
            }),
            debounce_ms: 750,
            cooldown_secs: 3,
            ..SubscriptionConfig::default()
        };
        let runtime = Subscription::from_config(config.clone());
        assert_eq!(runtime.to_config(), config);
        let response = SubscriptionResponse::from(&runtime);
        assert_eq!(response.trigger_config, config.trigger_config);
        assert_eq!(response.debounce_ms, 750);
    }

    #[tokio::test]
    async fn relay_status_route_uses_normal_api_authentication() {
        let dir = tempdir().expect("tempdir");
        let mut config = RokoConfig::default();
        config.serve.auth.enabled = true;
        config.serve.auth.api_keys = vec![ApiKeyEntry {
            name: "relay-reader".to_string(),
            key_hash: middleware::hash_api_key("relay-status-secret"),
            scope: "read".to_string(),
            created_at: "2026-08-16T00:00:00Z".to_string(),
            expires_at: None,
            last_used_at: None,
            previous_key_hashes: Vec::new(),
        }];
        let deploy_backend =
            Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
        let state = Arc::new(
            AppState::new(
                dir.path().to_path_buf(),
                Arc::new(NoOpRuntime),
                config.clone(),
                deploy_backend,
            )
            .expect("app state"),
        );
        let app = build_router(Arc::clone(&state), &[], config.serve.auth.clone());

        let unauthenticated = app
            .clone()
            .oneshot(
                Request::get("/api/subscriptions/relay/status")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

        let authenticated = app
            .oneshot(
                Request::get("/api/subscriptions/relay/status")
                    .header("X-Api-Key", "relay-status-secret")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(authenticated.status(), StatusCode::OK);
    }
}
