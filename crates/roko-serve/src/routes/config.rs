//! Configuration read/write endpoints.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use roko_core::config::LoadConfigError;
use roko_core::config::hot_reload;
use roko_core::config::loader::{load_config_unified, normalize_and_validate_dispatch_models};
use roko_core::config::schema::RokoConfig;

use crate::error::ApiError;
use crate::events::ServerEvent;
use crate::extract::ApiJson;
use crate::state::AppState;

/// A single ownership boundary for every mutation of the persisted and live
/// configuration. The watcher-facing reload API is synchronous, so the
/// transaction itself deliberately performs only synchronous work. Async HTTP
/// callers run it on Tokio's blocking pool; no blocking mutex is held across an
/// `.await`.
static CONFIG_MUTATION_GATE: Mutex<()> = Mutex::new(());
static CONFIG_MUTATION_GENERATION: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
struct UpdateCommit {
    response: Value,
    toml: String,
    generation: u64,
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/config", get(get_config).put(update_config))
        .route("/config/toml", get(get_config_toml))
        .route("/config/reload", post(reload_config))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReloadResponse {
    pub success: bool,
    pub warnings: Vec<String>,
    pub timestamp: String,
}

/// `GET /api/config` — return the current `RokoConfig` as JSON.
async fn get_config(State(state): State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    let cfg = state.load_roko_config();
    let mut value = serde_json::to_value(cfg.as_ref())
        .map_err(|e| ApiError::internal(format!("serialize config: {e}")))?;
    expose_dashboard_config_fields(&mut value, cfg.as_ref());
    mask_secret_fields(&mut value);
    Ok(Json(value))
}

/// `GET /api/config/toml` — return the raw `roko.toml` content as `text/toml`.
///
/// Used by Builder workspaces to copy the live config into ephemeral directories.
async fn get_config_toml(
    State(state): State<Arc<AppState>>,
) -> Result<([(axum::http::header::HeaderName, &'static str); 1], String), ApiError> {
    let cfg = state.load_roko_config();
    let mut value = serde_json::to_value(cfg.as_ref())
        .map_err(|e| ApiError::internal(format!("serialize config: {e}")))?;
    mask_secret_fields(&mut value);
    strip_mask_hint_fields(&mut value);
    strip_json_nulls(&mut value);
    let toml_value: toml::Value = serde_json::from_value(value)
        .map_err(|e| ApiError::internal(format!("convert to toml: {e}")))?;
    let toml_str = toml::to_string_pretty(&toml_value)
        .map_err(|e| ApiError::internal(format!("serialize toml: {e}")))?;
    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/toml")],
        toml_str,
    ))
}

/// `PUT /api/config` — merge partial config JSON into the current config,
/// then write the result to `roko.toml`.
async fn update_config(
    State(state): State<Arc<AppState>>,
    ApiJson(partial): ApiJson<Value>,
) -> Result<Json<Value>, ApiError> {
    let transaction_state = Arc::clone(&state);
    let commit =
        tokio::task::spawn_blocking(move || update_config_transaction(&transaction_state, partial))
            .await
            .map_err(|error| {
                ApiError::internal(format!("config transaction task failed: {error}"))
            })??;

    // Ephemeral copies follow the committed main-file/live-state truth. Their
    // failures are reported independently and never roll that truth back.
    let workspaces = {
        let workspaces = state.ephemeral_workspaces.read().await;
        workspaces
            .values()
            .map(|workspace| (workspace.id.clone(), workspace.path.clone()))
            .collect::<Vec<_>>()
    };
    for (workspace_id, path) in workspaces {
        if let Err(error) = tokio::fs::write(path.join("roko.toml"), &commit.toml).await {
            tracing::warn!(
                %workspace_id,
                path = %path.display(),
                %error,
                generation = commit.generation,
                "failed to propagate committed config to ephemeral workspace"
            );
        }
    }

    let mut response = commit.response;
    mask_secret_fields(&mut response);
    Ok(Json(response))
}

fn update_config_transaction(state: &AppState, partial: Value) -> Result<UpdateCommit, ApiError> {
    update_config_transaction_with_hook(state, partial, || {})
}

fn update_config_transaction_with_hook<F>(
    state: &AppState,
    partial: Value,
    after_lock: F,
) -> Result<UpdateCommit, ApiError>
where
    F: FnOnce(),
{
    let _owner = CONFIG_MUTATION_GATE
        .lock()
        .map_err(|_| ApiError::internal("config mutation ownership lock is poisoned"))?;
    after_lock();

    // Snapshot selection is inside the ownership boundary. A concurrent PUT
    // therefore always merges over the preceding committed generation.
    let cfg = state.load_roko_config();
    let mut response = serde_json::to_value(cfg.as_ref())
        .map_err(|e| ApiError::internal(format!("serialize current config: {e}")))?;
    merge_json(&mut response, &partial);

    let mut updated: RokoConfig = serde_json::from_value(response)
        .map_err(|e| ApiError::bad_request(format!("invalid config after merge: {e}")))?;
    normalize_and_validate_dispatch_models(&mut updated).map_err(map_load_config_error)?;

    let toml = toml::to_string_pretty(&updated)
        .map_err(|e| ApiError::internal(format!("serialize toml: {e}")))?;
    let config_path = state.workdir.join("roko.toml");
    roko_fs::atomic_write_bytes(&config_path, toml.as_bytes())
        .map_err(|e| ApiError::internal(format!("write roko.toml: {e}")))?;

    // Atomic persistence is the only fallible commit operation. The in-memory
    // swap follows immediately, so a persistence failure leaves both the old
    // file and old live generation intact.
    let mut response = serde_json::to_value(&updated)
        .map_err(|e| ApiError::internal(format!("serialize normalized config: {e}")))?;
    expose_dashboard_config_fields(&mut response, &updated);
    state.store_roko_config(updated);
    let generation = CONFIG_MUTATION_GENERATION.fetch_add(1, Ordering::AcqRel) + 1;
    tracing::debug!(generation, path = %config_path.display(), "committed config update");

    Ok(UpdateCommit {
        response,
        toml,
        generation,
    })
}

fn expose_dashboard_config_fields(value: &mut Value, config: &RokoConfig) {
    if let Some(map) = value.as_object_mut() {
        let default_model = config.agent.default_model.trim();
        map.insert(
            "default_model".to_string(),
            if default_model.is_empty() {
                Value::Null
            } else {
                Value::String(default_model.to_string())
            },
        );
    }
}

/// `POST /api/config/reload` — reload `roko.toml` from disk and swap it into
/// the live server state without a restart.
pub async fn reload_config(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ReloadResponse>, ApiError> {
    let transaction_state = Arc::clone(&state);
    let warnings = tokio::task::spawn_blocking(move || reload_config_from_disk(&transaction_state))
        .await
        .map_err(|error| ApiError::internal(format!("config reload task failed: {error}")))?
        .map_err(map_load_config_error)?;

    Ok(Json(ReloadResponse {
        success: true,
        warnings,
        timestamp: Utc::now().to_rfc3339(),
    }))
}

fn map_load_config_error(err: LoadConfigError) -> ApiError {
    match err {
        LoadConfigError::Read { .. } => ApiError::internal(err.to_string()),
        LoadConfigError::Parse { .. }
        | LoadConfigError::Validation { .. }
        | LoadConfigError::ProviderReference { .. }
        | LoadConfigError::AmbiguousModelSlug { .. }
        | LoadConfigError::UnresolvedModel { .. } => ApiError::bad_request(err.to_string()),
    }
}

/// Reload `roko.toml` from disk into the live state and return validation warnings.
///
/// Uses the hot-reload diff engine: sections classified as hot-reloadable
/// (`[budget]`, `[tools]`, `[learning]`, `[gates]`, etc.) are applied
/// immediately. Non-hot-reloadable sections (`[agent]`, `[providers]`,
/// `[models]`) log a warning suggesting a restart.
///
/// A `ConfigReloaded` event is published on the server event bus so
/// dashboards and SSE clients can react to config changes.
///
/// # Errors
///
/// Returns [`LoadConfigError::Read`] when `roko.toml` cannot be read and
/// [`LoadConfigError::Parse`] when the file contents are not valid config.
pub fn reload_config_from_disk(state: &AppState) -> Result<Vec<String>, LoadConfigError> {
    reload_config_from_disk_with_hook(state, || {})
}

fn reload_config_from_disk_with_hook<F>(
    state: &AppState,
    after_lock: F,
) -> Result<Vec<String>, LoadConfigError>
where
    F: FnOnce(),
{
    let _owner = CONFIG_MUTATION_GATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    after_lock();

    let mut new_config = load_config_unified(&state.workdir)?;
    normalize_and_validate_dispatch_models(&mut new_config)?;
    let mut warnings = validate_references(&new_config);

    // A reload may canonicalize aliases or merge an effective configuration.
    // Publish that validated generation atomically before exposing it live.
    let config_path = state.workdir.join("roko.toml");
    let serialized =
        toml::to_string_pretty(&new_config).map_err(|error| LoadConfigError::Read {
            path: config_path.clone(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, error),
        })?;
    let persisted_changed = match std::fs::read(&config_path) {
        Ok(current) => current != serialized.as_bytes(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
        Err(source) => {
            return Err(LoadConfigError::Read {
                path: config_path,
                source,
            });
        }
    };
    if persisted_changed {
        roko_fs::atomic_write_bytes(&config_path, serialized.as_bytes()).map_err(|source| {
            LoadConfigError::Read {
                path: config_path.clone(),
                source,
            }
        })?;
    }

    // Compute diff and apply only hot-reloadable sections.
    let old_config = state.load_roko_config();
    let changes = hot_reload::config_diff(&old_config, &new_config);

    if changes.is_empty() {
        if persisted_changed {
            let generation = CONFIG_MUTATION_GENERATION.fetch_add(1, Ordering::AcqRel) + 1;
            tracing::debug!(
                generation,
                path = %config_path.display(),
                "committed canonical config reload"
            );
        }
        return Ok(warnings);
    }

    let mut current = old_config.as_ref().clone();
    let result = hot_reload::apply_hot_reload(&mut current, &new_config, &changes);

    // For non-hot-reloadable changes, we still store the full new config
    // so a subsequent restart picks up the new values. But we surface
    // the restart-required warning.
    for change in &result.needs_restart {
        warnings.push(format!(
            "{}: {} (requires restart)",
            change.summary,
            change.section.is_hot_reloadable(),
        ));
    }

    // Store the fully merged config (hot-reloaded + pending restart sections).
    state.store_roko_config(new_config);
    let generation = CONFIG_MUTATION_GENERATION.fetch_add(1, Ordering::AcqRel) + 1;
    tracing::debug!(
        generation,
        path = %config_path.display(),
        "committed config reload"
    );

    // Emit config-changed event for dashboard visibility.
    let applied_count = result.applied.len();
    let restart_count = result.needs_restart.len();
    if applied_count > 0 || restart_count > 0 {
        state.event_bus.publish(ServerEvent::ConfigReloaded {
            applied_sections: result.applied.iter().map(|c| c.summary.clone()).collect(),
            restart_required: result
                .needs_restart
                .iter()
                .map(|c| c.summary.clone())
                .collect(),
        });
    }

    Ok(warnings)
}

/// Reload STRATEGY.md from disk and return the parsed strategy document.
///
/// Returns `None` if the file does not exist or cannot be read.
pub fn reload_strategy_from_disk(state: &AppState) -> Option<hot_reload::StrategyDocument> {
    let strategy_path = state.workdir.join("STRATEGY.md");
    let content = std::fs::read_to_string(&strategy_path).ok()?;
    let doc = hot_reload::parse_strategy_md(&content);

    tracing::info!(
        goals = doc.goals.len(),
        tactics = doc.tactics.len(),
        risk_bounds = doc.risk_bounds.len(),
        "STRATEGY.md reloaded"
    );

    state.event_bus.publish(ServerEvent::StrategyReloaded {
        goals_count: doc.goals.len(),
        tactics_count: doc.tactics.len(),
    });

    Some(doc)
}

fn validate_references(config: &RokoConfig) -> Vec<String> {
    let providers = config.effective_providers();
    let mut warnings: Vec<String> = config
        .models
        .iter()
        .filter_map(|(model_key, profile)| {
            (!providers.contains_key(&profile.provider)).then(|| {
                format!(
                    "model `{model_key}` references provider `{}` which is not configured",
                    profile.provider
                )
            })
        })
        .collect();
    warnings.sort();
    warnings
}

/// Recursively merge `patch` into `base`. Object keys from `patch` override
/// those in `base`; everything else is replaced wholesale.
fn merge_json(base: &mut Value, patch: &Value) {
    match (base, patch) {
        (Value::Object(base_map), Value::Object(patch_map)) => {
            for (k, v) in patch_map {
                let entry = base_map.entry(k.clone()).or_insert(Value::Null);
                merge_json(entry, v);
            }
        }
        (base, patch) => {
            *base = patch.clone();
        }
    }
}

/// Remove `*_note` keys added by [`mask_secret_fields`] so the JSON can become TOML.
fn strip_mask_hint_fields(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.retain(|k, _| !k.ends_with("_note"));
            for v in map.values_mut() {
                strip_mask_hint_fields(v);
            }
        }
        Value::Array(items) => {
            for v in items {
                strip_mask_hint_fields(v);
            }
        }
        _ => {}
    }
}

/// TOML has no `null`. Drop null values before converting [`serde_json::Value`] to [`toml::Value`].
fn strip_json_nulls(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.retain(|_, v| !v.is_null());
            for v in map.values_mut() {
                strip_json_nulls(v);
            }
        }
        Value::Array(items) => {
            for v in items {
                strip_json_nulls(v);
            }
        }
        _ => {}
    }
}

fn mask_secret_fields(value: &mut Value) {
    mask_secret_field(
        value,
        &["serve", "auth"],
        "api_key",
        "ROKO_SERVE_AUTH_API_KEY",
    );
    mask_secret_field(value, &["server"], "auth_token", "ROKO_SERVER_AUTH_TOKEN");
    mask_secret_field(
        value,
        &["deploy"],
        "railway_api_token",
        "ROKO_DEPLOY_RAILWAY_API_TOKEN",
    );
    mask_secret_field(value, &["chain"], "wallet_key", "ROKO_CHAIN_WALLET_KEY");
    mask_secret_field(
        value,
        &["webhooks", "github"],
        "secret",
        "ROKO_WEBHOOKS_GITHUB_SECRET",
    );
    if let Some(providers) = value.get_mut("providers").and_then(|v| v.as_object_mut()) {
        for (_name, provider) in providers.iter_mut() {
            if let Some(obj) = provider.as_object_mut() {
                if obj.contains_key("api_key") {
                    obj.insert("api_key".to_string(), Value::String("****".to_string()));
                }
            }
        }
    }
}

fn mask_secret_field(value: &mut Value, path: &[&str], field: &str, env_var: &str) {
    let mut cursor = value;
    for key in path {
        let Some(next) = cursor.get_mut(*key) else {
            return;
        };
        cursor = next;
    }

    let Some(map) = cursor.as_object_mut() else {
        return;
    };

    if map.contains_key(field) {
        map.insert(field.to_string(), Value::String("***".to_string()));
        map.insert(
            format!("{field}_note"),
            Value::String(format!("Set `{env_var}` in the environment.")),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state(workdir: std::path::PathBuf, config: RokoConfig) -> Arc<AppState> {
        use crate::deploy::create_backend;
        use crate::runtime::NoOpRuntime;

        let deploy_backend =
            Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
        Arc::new(
            AppState::new(workdir, Arc::new(NoOpRuntime), config, deploy_backend)
                .expect("AppState::new"),
        )
    }

    #[test]
    fn expose_dashboard_config_fields_adds_default_model() {
        let mut config = RokoConfig::default();
        config.agent.default_model = "claude-sonnet-4-20250514".into();
        let mut value = serde_json::to_value(&config).expect("serialize config");

        expose_dashboard_config_fields(&mut value, &config);

        assert_eq!(value["default_model"], "claude-sonnet-4-20250514");
        assert_eq!(value["agent"]["default_model"], "claude-sonnet-4-20250514");
    }

    #[test]
    fn mask_secret_fields_redacts_config_secrets() {
        let mut value = serde_json::json!({
            "serve": {
                "auth": {
                    "enabled": true,
                    "api_key": "secret"
                }
            },
            "server": {
                "auth_token": "server-secret"
            },
            "deploy": {
                "railway_api_token": "railway-secret"
            }
        });

        mask_secret_fields(&mut value);

        assert_eq!(value["serve"]["auth"]["api_key"], "***");
        assert_eq!(
            value["serve"]["auth"]["api_key_note"],
            "Set `ROKO_SERVE_AUTH_API_KEY` in the environment."
        );
        assert_eq!(value["server"]["auth_token"], "***");
        assert_eq!(
            value["server"]["auth_token_note"],
            "Set `ROKO_SERVER_AUTH_TOKEN` in the environment."
        );
        assert_eq!(value["deploy"]["railway_api_token"], "***");
        assert_eq!(
            value["deploy"]["railway_api_token_note"],
            "Set `ROKO_DEPLOY_RAILWAY_API_TOKEN` in the environment."
        );
    }

    #[test]
    fn mask_secret_fields_redacts_extended_secrets() {
        let mut value = serde_json::json!({
            "chain": { "wallet_key": "0xdeadbeef" },
            "webhooks": { "github": { "secret": "ghsecret" } },
            "providers": {
                "anthropic": { "api_key": "sk-ant-xxx" }
            }
        });

        mask_secret_fields(&mut value);

        assert_eq!(value["chain"]["wallet_key"], "***");
        assert_eq!(
            value["chain"]["wallet_key_note"],
            "Set `ROKO_CHAIN_WALLET_KEY` in the environment."
        );
        assert_eq!(value["webhooks"]["github"]["secret"], "***");
        assert_eq!(
            value["webhooks"]["github"]["secret_note"],
            "Set `ROKO_WEBHOOKS_GITHUB_SECRET` in the environment."
        );
        assert_eq!(value["providers"]["anthropic"]["api_key"], "****");
    }

    #[tokio::test]
    async fn get_config_toml_masks_secrets() {
        use std::sync::Arc;

        use axum::body::{Body, to_bytes};
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        use crate::deploy::create_backend;
        use crate::runtime::NoOpRuntime;

        let dir = tempfile::tempdir().expect("tempdir");
        let workdir = dir.path().to_path_buf();
        let deploy_backend =
            Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
        let mut cfg = RokoConfig::default();
        cfg.serve.auth.api_key = "test-secret-key".into();
        let state = Arc::new(
            AppState::new(workdir, Arc::new(NoOpRuntime), cfg, deploy_backend)
                .expect("AppState::new"),
        );

        let response = routes()
            .with_state(Arc::clone(&state))
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/config/toml")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let text = String::from_utf8(body.to_vec()).expect("utf8");
        assert!(
            !text.contains("test-secret-key"),
            "TOML response leaked api key: {text}"
        );
    }

    #[tokio::test]
    async fn reload_config_reloads_state_from_disk() {
        use std::sync::Arc;

        use axum::body::{Body, to_bytes};
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        use crate::deploy::create_backend;
        use crate::runtime::NoOpRuntime;

        let dir = tempfile::tempdir().expect("tempdir");
        let workdir = dir.path().to_path_buf();
        let deploy_backend =
            Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
        let state = Arc::new(
            AppState::new(
                workdir.clone(),
                Arc::new(NoOpRuntime),
                RokoConfig::default(),
                deploy_backend,
            )
            .expect("AppState::new"),
        );

        tokio::fs::write(
            workdir.join("roko.toml"),
            r#"
[server]
port = 4567
"#,
        )
        .await
        .expect("write roko.toml");

        let response = routes()
            .with_state(Arc::clone(&state))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/config/reload")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("reload response");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(state.load_roko_config().server.port, 4567);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let payload: ReloadResponse = serde_json::from_slice(&body).expect("parse reload response");
        assert!(payload.success);
        // [server] is a non-hot-reloadable section, so a restart warning is expected.
        assert!(
            payload.warnings.iter().any(|w| w.contains("server")),
            "expected a restart warning for [server] change, got: {:?}",
            payload.warnings
        );
        assert!(!payload.timestamp.is_empty());
    }

    #[tokio::test]
    async fn reload_config_rejects_ambiguous_models_as_bad_request() {
        use axum::body::{Body, to_bytes};
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let workdir = dir.path().to_path_buf();
        let state = test_state(workdir.clone(), RokoConfig::default());
        tokio::fs::write(
            workdir.join("roko.toml"),
            r#"
[providers.provider]
kind = "openai_compat"
base_url = "https://example.com/v1"

[models.first]
provider = "provider"
slug = "duplicate-slug"

[models.second]
provider = "provider"
slug = "duplicate-slug"

[agent]
default_model = "first"
"#,
        )
        .await
        .expect("write config");

        let response = routes()
            .with_state(Arc::clone(&state))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/config/reload")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(state.load_roko_config().models.is_empty());
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let body = String::from_utf8(body.to_vec()).expect("utf8");
        assert!(body.contains("ambiguous model slug"), "body: {body}");
    }

    #[tokio::test]
    async fn update_config_rejects_unresolved_model_without_writing() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode, header::CONTENT_TYPE};
        use tower::ServiceExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let workdir = dir.path().to_path_buf();
        let state = test_state(workdir.clone(), RokoConfig::default());
        let patch = serde_json::json!({
            "models": {
                "focused": {
                    "provider": "provider",
                    "slug": "provider-model-v1"
                }
            },
            "agent": { "default_model": "missing" }
        });

        let response = routes()
            .with_state(Arc::clone(&state))
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/config")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(patch.to_string()))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(state.load_roko_config().models.is_empty());
        assert!(
            !workdir.join("roko.toml").exists(),
            "invalid config must not be persisted"
        );
    }

    #[tokio::test]
    async fn update_config_normalizes_alias_before_store_and_response() {
        use axum::body::{Body, to_bytes};
        use axum::http::{Request, StatusCode, header::CONTENT_TYPE};
        use tower::ServiceExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let workdir = dir.path().to_path_buf();
        let mut config = RokoConfig::default();
        config.models.insert(
            "focused".to_string(),
            roko_core::config::schema::ModelProfile {
                provider: "provider".to_string(),
                slug: "provider-model-v1".to_string(),
                ..Default::default()
            },
        );
        config.agent.default_model = "provider-model-v1".to_string();
        let state = test_state(workdir.clone(), config);

        let response = routes()
            .with_state(Arc::clone(&state))
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/config")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from("{}"))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(state.load_roko_config().agent.default_model, "focused");
        let persisted = tokio::fs::read_to_string(workdir.join("roko.toml"))
            .await
            .expect("read config");
        let persisted: RokoConfig = toml::from_str(&persisted).expect("parse config");
        assert_eq!(persisted.agent.default_model, "focused");
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let payload: Value = serde_json::from_slice(&body).expect("parse body");
        assert_eq!(payload["agent"]["default_model"], "focused");
        assert_eq!(payload["default_model"], "focused");
    }

    #[test]
    fn concurrent_disjoint_updates_preserve_both_commits() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = test_state(dir.path().to_path_buf(), RokoConfig::default());
        let barrier = Arc::new(std::sync::Barrier::new(3));

        let port_state = Arc::clone(&state);
        let port_barrier = Arc::clone(&barrier);
        let port_update = std::thread::spawn(move || {
            port_barrier.wait();
            update_config_transaction(&port_state, serde_json::json!({"server": {"port": 4321}}))
                .expect("commit port update");
        });
        let bind_state = Arc::clone(&state);
        let bind_barrier = Arc::clone(&barrier);
        let bind_update = std::thread::spawn(move || {
            bind_barrier.wait();
            update_config_transaction(
                &bind_state,
                serde_json::json!({"server": {"bind": "127.0.0.2"}}),
            )
            .expect("commit bind update");
        });

        barrier.wait();
        port_update.join().expect("port thread");
        bind_update.join().expect("bind thread");

        let live = state.load_roko_config();
        assert_eq!(live.server.port, 4321);
        assert_eq!(live.server.bind, "127.0.0.2");
        let persisted: RokoConfig = toml::from_str(
            &std::fs::read_to_string(dir.path().join("roko.toml")).expect("read persisted config"),
        )
        .expect("parse persisted config");
        assert_eq!(persisted, *live, "file and live generations diverged");
    }

    #[test]
    fn failed_atomic_persistence_keeps_live_generation_unchanged() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = test_state(dir.path().to_path_buf(), RokoConfig::default());
        let before = state.load_roko_config();
        std::fs::create_dir(dir.path().join("roko.toml"))
            .expect("block config file with directory");

        let error =
            update_config_transaction(&state, serde_json::json!({"server": {"port": 4321}}))
                .expect_err("atomic replacement of a directory must fail");

        assert!(error.to_string().contains("write roko.toml"));
        assert_eq!(*state.load_roko_config(), *before);
        assert!(dir.path().join("roko.toml").is_dir());
    }

    #[test]
    fn waiting_reload_cannot_publish_stale_work_over_put_generation() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = test_state(dir.path().to_path_buf(), RokoConfig::default());

        let mut stale = RokoConfig::default();
        stale.server.port = 1111;
        std::fs::write(
            dir.path().join("roko.toml"),
            toml::to_string_pretty(&stale).expect("serialize stale config"),
        )
        .expect("stage stale disk config");

        let (put_acquired_tx, put_acquired_rx) = std::sync::mpsc::channel();
        let (release_put_tx, release_put_rx) = std::sync::mpsc::channel();
        let put_state = Arc::clone(&state);
        let put = std::thread::spawn(move || {
            update_config_transaction_with_hook(
                &put_state,
                serde_json::json!({"server": {"bind": "127.0.0.2"}}),
                || {
                    put_acquired_tx.send(()).expect("signal PUT ownership");
                    release_put_rx.recv().expect("release PUT ownership");
                },
            )
            .expect("commit winning PUT");
        });
        put_acquired_rx.recv().expect("PUT acquired ownership");

        let (reload_started_tx, reload_started_rx) = std::sync::mpsc::channel();
        let reload_state = Arc::clone(&state);
        let reload = std::thread::spawn(move || {
            reload_started_tx.send(()).expect("signal reload attempt");
            reload_config_from_disk(&reload_state).expect("reload committed PUT file")
        });
        reload_started_rx
            .recv()
            .expect("reload attempted ownership");
        release_put_tx.send(()).expect("release PUT");

        put.join().expect("PUT thread");
        reload.join().expect("reload thread");

        let live = state.load_roko_config();
        assert_eq!(live.server.bind, "127.0.0.2");
        assert_eq!(
            live.server.port,
            RokoConfig::default().server.port,
            "reload published the stale pre-transaction file"
        );
        let persisted: RokoConfig = toml::from_str(
            &std::fs::read_to_string(dir.path().join("roko.toml")).expect("read persisted config"),
        )
        .expect("parse persisted config");
        assert_eq!(persisted, *live, "file and live generations diverged");
    }
}
