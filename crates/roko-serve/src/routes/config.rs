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
use roko_core::config::loader::{
    LoadOptions, load_config_unified, load_config_validated,
    normalize_and_validate_dispatch_models, resolve_config_source,
};
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
    update_config_transaction_with_hooks(state, partial, after_lock, || {})
}

fn update_config_transaction_with_hooks<F, G>(
    state: &AppState,
    partial: Value,
    after_lock: F,
    before_ephemeral_propagation: G,
) -> Result<UpdateCommit, ApiError>
where
    F: FnOnce(),
    G: FnOnce(),
{
    update_config_transaction_with_persistence_hook(
        state,
        partial,
        after_lock,
        || {},
        before_ephemeral_propagation,
    )
}

fn update_config_transaction_with_persistence_hook<F, G, H>(
    state: &AppState,
    partial: Value,
    after_lock: F,
    before_main_persistence: G,
    before_ephemeral_propagation: H,
) -> Result<UpdateCommit, ApiError>
where
    F: FnOnce(),
    G: FnOnce(),
    H: FnOnce(),
{
    let _owner = CONFIG_MUTATION_GATE
        .lock()
        .map_err(|_| ApiError::internal("config mutation ownership lock is poisoned"))?;
    after_lock();

    // Source snapshot selection is inside the ownership boundary. Runtime
    // overlays in AppState are deliberately not a persistence base: every
    // accepted PUT merges over the preceding project-source generation.
    let config_path = state.workdir.join("roko.toml");
    let source = load_project_config_source(&state.workdir, &config_path)?;
    let mut source_value = serde_json::to_value(&source)
        .map_err(|e| ApiError::internal(format!("serialize source config: {e}")))?;
    merge_json(&mut source_value, &partial);

    let mut source_updated: RokoConfig = serde_json::from_value(source_value)
        .map_err(|e| ApiError::bad_request(format!("invalid config after merge: {e}")))?;
    normalize_and_validate_dispatch_models(&mut source_updated).map_err(map_load_config_error)?;

    // Derive the prospective live generation entirely before the authoritative
    // write. This reapplies global/env/interpolation/file-secret layers with
    // normal loader precedence while retaining source references separately.
    let mut effective_updated = resolve_config_source(
        source_updated.clone(),
        &config_path,
        &LoadOptions::default(),
    )
    .map_err(map_load_config_error)?;
    normalize_and_validate_dispatch_models(&mut effective_updated)
        .map_err(map_load_config_error)?;

    let source_toml = toml::to_string_pretty(&source_updated)
        .map_err(|e| ApiError::internal(format!("serialize toml: {e}")))?;
    let mut response = serde_json::to_value(&effective_updated)
        .map_err(|e| ApiError::internal(format!("serialize normalized config: {e}")))?;
    expose_dashboard_config_fields(&mut response, &effective_updated);
    before_main_persistence();
    roko_fs::atomic_write_bytes(&config_path, source_toml.as_bytes())
        .map_err(|e| ApiError::internal(format!("write roko.toml: {e}")))?;

    // The request's blocking transaction owns main-file persistence,
    // propagation, and live publication as one ordered unit. Once this task
    // starts, dropping the async request future does not cancel it. Keeping the
    // ownership gate through propagation prevents an older accepted update
    // from overwriting a newer generation in an ephemeral workspace.
    before_ephemeral_propagation();
    let workspaces = state
        .ephemeral_workspaces
        .blocking_read()
        .values()
        .map(|workspace| (workspace.id.clone(), workspace.path.clone()))
        .collect::<Vec<_>>();
    for (workspace_id, path) in workspaces {
        let workspace_config = path.join("roko.toml");
        if let Err(error) = roko_fs::atomic_write_bytes(&workspace_config, source_toml.as_bytes()) {
            // Ephemeral copies are best-effort replicas. A failure is isolated
            // to that workspace and does not roll back authoritative main-file
            // truth or leave the main file and live state divergent.
            tracing::warn!(
                %workspace_id,
                path = %workspace_config.display(),
                %error,
                "failed to propagate committed config to ephemeral workspace"
            );
        }
    }

    // Main-file atomic persistence is the authoritative fallible commit. The
    // live swap is infallible and occurs before releasing mutation ownership.
    state.store_roko_config(effective_updated);
    let generation = CONFIG_MUTATION_GENERATION.fetch_add(1, Ordering::AcqRel) + 1;
    tracing::debug!(generation, path = %config_path.display(), "committed config update");

    Ok(UpdateCommit { response })
}

/// Load the project-local source selected by the config PUT contract.
///
/// PUT has always persisted `workdir/roko.toml`; keep that path policy
/// explicit rather than accidentally editing an ancestor or `ROKO_CONFIG`
/// override. When the local file does not exist, the raw (never effective)
/// representation selected by unified discovery seeds the new local source;
/// if discovery finds nothing, that representation is schema defaults.
fn load_project_config_source(
    workdir: &std::path::Path,
    config_path: &std::path::Path,
) -> Result<RokoConfig, ApiError> {
    let source = match std::fs::read_to_string(config_path) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return load_config_validated(workdir)
                .map(|validated| validated.raw)
                .map_err(map_load_config_error);
        }
        Err(error) => {
            return Err(ApiError::internal(format!(
                "read project config {}: {error}",
                config_path.display()
            )));
        }
    };
    toml::from_str(&source).map_err(|error| {
        ApiError::bad_request(format!(
            "parse project config {}: {error}",
            config_path.display()
        ))
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

    // The unified loader returns effective runtime state: global values,
    // environment overrides, interpolation, and file-secret resolution have
    // already been applied. It must never be serialized back into the project
    // source. Reload therefore preserves `roko.toml` byte-for-byte and only
    // publishes the validated effective object.
    let config_path = state.workdir.join("roko.toml");
    // Compute diff and apply only hot-reloadable sections.
    let old_config = state.load_roko_config();
    let changes = hot_reload::config_diff(&old_config, &new_config);

    if changes.is_empty() {
        state.store_roko_config(new_config);
        let generation = CONFIG_MUTATION_GENERATION.fetch_add(1, Ordering::AcqRel) + 1;
        tracing::debug!(
            generation,
            path = %config_path.display(),
            "published validated config reload"
        );
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

    fn effective_from_source(path: &std::path::Path, source: &RokoConfig) -> RokoConfig {
        let mut effective = resolve_config_source(source.clone(), path, &LoadOptions::default())
            .expect("resolve effective test config");
        normalize_and_validate_dispatch_models(&mut effective)
            .expect("normalize effective test config");
        effective
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
        std::fs::write(
            workdir.join("roko.toml"),
            toml::to_string_pretty(&config).expect("serialize source config"),
        )
        .expect("write source config");
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
        let expected = effective_from_source(&dir.path().join("roko.toml"), &persisted);
        assert_eq!(*live, expected, "live effective generation diverged");
    }

    #[test]
    fn put_seeds_missing_local_source_from_discovered_raw_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workdir = dir.path().join("nested/project");
        std::fs::create_dir_all(&workdir).expect("create nested workdir");
        let ancestor_path = dir.path().join("roko.toml");
        let ancestor_source = "config_version = 2\n\n[server]\nport = 4111\n";
        std::fs::write(&ancestor_path, ancestor_source).expect("write ancestor source");
        let initial = load_config_unified(&workdir).expect("load discovered effective config");
        let state = test_state(workdir.clone(), initial);

        update_config_transaction(&state, serde_json::json!({"server": {"bind": "127.0.0.2"}}))
            .expect("commit local source");

        assert_eq!(
            std::fs::read_to_string(&ancestor_path).expect("read ancestor source"),
            ancestor_source,
            "PUT must not edit the discovered ancestor path"
        );
        let local_text =
            std::fs::read_to_string(workdir.join("roko.toml")).expect("read local source");
        let local: RokoConfig = toml::from_str(&local_text).expect("parse local source");
        assert_eq!(local.server.port, 4111);
        assert_eq!(local.server.bind, "127.0.0.2");
        assert!(
            local.providers.is_empty() && local.models.is_empty(),
            "machine-global registries leaked into the local source"
        );
    }

    #[test]
    fn failed_atomic_persistence_keeps_live_generation_unchanged() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = test_state(dir.path().to_path_buf(), RokoConfig::default());
        let before = state.load_roko_config();
        std::fs::write(dir.path().join("roko.toml"), "config_version = 2\n")
            .expect("write readable source");

        let config_path = dir.path().join("roko.toml");
        let error = update_config_transaction_with_persistence_hook(
            &state,
            serde_json::json!({"server": {"port": 4321}}),
            || {},
            || {
                std::fs::remove_file(&config_path).expect("remove source before persistence");
                std::fs::create_dir(&config_path).expect("replace target with directory");
            },
            || {},
        )
        .expect_err("atomic replacement of a directory must fail");

        assert!(error.to_string().contains("write roko.toml"));
        assert_eq!(*state.load_roko_config(), *before);
        assert!(dir.path().join("roko.toml").is_dir());
    }

    #[test]
    fn waiting_reload_cannot_publish_stale_work_over_put_generation() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = test_state(dir.path().to_path_buf(), RokoConfig::default());
        let workspace = dir.path().join("ephemeral-reload-order");
        std::fs::create_dir_all(&workspace).expect("create ephemeral workspace");
        state.ephemeral_workspaces.blocking_write().insert(
            "reload-order".to_string(),
            crate::state::WorkspaceInfo {
                id: "reload-order".to_string(),
                path: workspace.clone(),
                created_at: 0,
                last_accessed_at: 0,
                status: crate::state::WorkspaceStatus::Active,
            },
        );

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
            live.server.port, 1111,
            "PUT did not preserve its locked source snapshot"
        );
        let persisted: RokoConfig = toml::from_str(
            &std::fs::read_to_string(dir.path().join("roko.toml")).expect("read persisted config"),
        )
        .expect("parse persisted config");
        assert_eq!(persisted.server.bind, live.server.bind);
        assert_eq!(persisted.server.port, live.server.port);
        let ephemeral: RokoConfig = toml::from_str(
            &std::fs::read_to_string(workspace.join("roko.toml")).expect("read propagated config"),
        )
        .expect("parse propagated config");
        assert_eq!(
            ephemeral, persisted,
            "reload raced or rewrote PUT propagation"
        );
    }

    #[test]
    fn older_put_cannot_propagate_after_newer_put() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = test_state(dir.path().to_path_buf(), RokoConfig::default());
        let workspace = dir.path().join("ephemeral-ordered");
        std::fs::create_dir_all(&workspace).expect("create ephemeral workspace");
        state.ephemeral_workspaces.blocking_write().insert(
            "ordered".to_string(),
            crate::state::WorkspaceInfo {
                id: "ordered".to_string(),
                path: workspace.clone(),
                created_at: 0,
                last_accessed_at: 0,
                status: crate::state::WorkspaceStatus::Active,
            },
        );

        let (older_ready_tx, older_ready_rx) = std::sync::mpsc::channel();
        let (release_older_tx, release_older_rx) = std::sync::mpsc::channel();
        let older_state = Arc::clone(&state);
        let older = std::thread::spawn(move || {
            update_config_transaction_with_hooks(
                &older_state,
                serde_json::json!({"server": {"port": 1111}}),
                || {},
                || {
                    older_ready_tx
                        .send(())
                        .expect("signal older before propagation");
                    release_older_rx.recv().expect("release older propagation");
                },
            )
            .expect("commit older PUT");
        });
        older_ready_rx
            .recv()
            .expect("older reached propagation barrier");

        let newer_state = Arc::clone(&state);
        let newer = std::thread::spawn(move || {
            update_config_transaction(&newer_state, serde_json::json!({"server": {"port": 2222}}))
                .expect("commit newer PUT");
        });
        std::thread::sleep(std::time::Duration::from_millis(75));
        assert!(
            !newer.is_finished(),
            "newer PUT entered while older propagation retained ownership"
        );
        assert!(
            !workspace.join("roko.toml").exists(),
            "older propagated before its deterministic release"
        );

        release_older_tx.send(()).expect("release older PUT");
        older.join().expect("older PUT thread");
        newer.join().expect("newer PUT thread");

        let live = state.load_roko_config();
        assert_eq!(live.server.port, 2222);
        let main: RokoConfig = toml::from_str(
            &std::fs::read_to_string(dir.path().join("roko.toml")).expect("read main config"),
        )
        .expect("parse main config");
        let replica: RokoConfig = toml::from_str(
            &std::fs::read_to_string(workspace.join("roko.toml")).expect("read ephemeral config"),
        )
        .expect("parse ephemeral config");
        assert_eq!(main.server.port, 2222);
        assert_eq!(replica, main, "older propagation overwrote newer truth");
        let expected = effective_from_source(&dir.path().join("roko.toml"), &main);
        assert_eq!(*live, expected, "live effective generation diverged");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cancelled_put_finishes_propagation_and_publication() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = test_state(dir.path().to_path_buf(), RokoConfig::default());
        let workspace = dir.path().join("ephemeral-cancelled");
        tokio::fs::create_dir_all(&workspace)
            .await
            .expect("create ephemeral workspace");
        state.ephemeral_workspaces.write().await.insert(
            "cancelled".to_string(),
            crate::state::WorkspaceInfo {
                id: "cancelled".to_string(),
                path: workspace.clone(),
                created_at: 0,
                last_accessed_at: 0,
                status: crate::state::WorkspaceStatus::Active,
            },
        );

        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let transaction_state = Arc::clone(&state);
        let caller = tokio::spawn(async move {
            tokio::task::spawn_blocking(move || {
                update_config_transaction_with_hooks(
                    &transaction_state,
                    serde_json::json!({"server": {"port": 3333}}),
                    || {},
                    || {
                        ready_tx.send(()).expect("signal transaction ownership");
                        release_rx.recv().expect("release propagation");
                    },
                )
            })
            .await
        });
        ready_rx.await.expect("transaction reached propagation");
        caller.abort();
        assert!(caller.await.expect_err("caller cancelled").is_cancelled());
        release_tx.send(()).expect("release detached transaction");

        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while state.load_roko_config().server.port != 3333 {
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("detached blocking transaction published live state");
        let main: RokoConfig = toml::from_str(
            &tokio::fs::read_to_string(dir.path().join("roko.toml"))
                .await
                .expect("read main config"),
        )
        .expect("parse main config");
        let replica: RokoConfig = toml::from_str(
            &tokio::fs::read_to_string(workspace.join("roko.toml"))
                .await
                .expect("read ephemeral config"),
        )
        .expect("parse ephemeral config");
        assert_eq!(main.server.port, 3333);
        assert_eq!(replica, main);
        let expected = effective_from_source(&dir.path().join("roko.toml"), &main);
        assert_eq!(*state.load_roko_config(), expected);
    }

    #[test]
    fn ephemeral_write_failure_does_not_diverge_main_and_live() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = test_state(dir.path().to_path_buf(), RokoConfig::default());
        let workspace = dir.path().join("ephemeral-failure");
        std::fs::create_dir_all(workspace.join("roko.toml"))
            .expect("make replica target unwritable as a file");
        state.ephemeral_workspaces.blocking_write().insert(
            "failure".to_string(),
            crate::state::WorkspaceInfo {
                id: "failure".to_string(),
                path: workspace.clone(),
                created_at: 0,
                last_accessed_at: 0,
                status: crate::state::WorkspaceStatus::Active,
            },
        );

        update_config_transaction(&state, serde_json::json!({"server": {"port": 4444}}))
            .expect("ephemeral failure is non-authoritative");

        let main: RokoConfig = toml::from_str(
            &std::fs::read_to_string(dir.path().join("roko.toml")).expect("read main config"),
        )
        .expect("parse main config");
        assert_eq!(main.server.port, 4444);
        let expected = effective_from_source(&dir.path().join("roko.toml"), &main);
        assert_eq!(*state.load_roko_config(), expected);
        assert!(
            workspace.join("roko.toml").is_dir(),
            "failed replica must remain isolated"
        );
    }

    #[test]
    fn put_preserves_runtime_overlay_sources_after_startup_and_reload() {
        let root = tempfile::tempdir().expect("tempdir");
        let home = root.path().join("home");
        std::fs::create_dir_all(home.join(".roko")).expect("create global config directory");
        let secret_path = root.path().join("authorization.secret");
        std::fs::write(&secret_path, "literal-file-secret\n").expect("write secret file");
        let source = format!(
            r#"config_version = 2

[providers.project]
kind = "openai_compat"
base_url = "https://source.invalid/${{CTRL02_INTERPOLATED}}"

[providers.project.extra_headers]
authorization_file = "{}"
trace = "${{CTRL02_INTERPOLATED}}"

[models.project-model]
provider = "project"
slug = "project-slug"

[agent]
default_model = "project-model"

[server]
port = 3000
"#,
            secret_path.display()
        );
        std::fs::write(
            home.join(".roko/config.toml"),
            r#"[providers.global]
kind = "openai_compat"
base_url = "https://global.invalid/v1"

[models.global-model]
provider = "global"
slug = "global-slug"
"#,
        )
        .expect("write global source");

        for mode in ["startup", "reload", "reject"] {
            let workdir = root.path().join(mode).join("project");
            std::fs::create_dir_all(&workdir).expect("create project directory");
            std::fs::write(workdir.join("roko.toml"), &source).expect("write project source");
            let output = std::process::Command::new(std::env::current_exe().expect("test binary"))
                .arg("--exact")
                .arg("routes::config::tests::put_preserves_runtime_overlay_sources_child")
                .arg("--nocapture")
                .env_clear()
                .env("HOME", &home)
                .env("CTRL02_CONFIG_CHILD", mode)
                .env("CTRL02_WORKDIR", &workdir)
                .env("CTRL02_INTERPOLATED", "runtime-fragment")
                .env("ROKO_MODEL", "global-model")
                .env("ROKO__SERVER__PORT", "4555")
                .output()
                .expect("run isolated config source test");
            assert!(
                output.status.success(),
                "{mode} child failed:\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );

            let after = std::fs::read_to_string(workdir.join("roko.toml"))
                .expect("read preserved project source");
            if mode == "reject" {
                assert_eq!(after, source, "rejected candidate mutated project source");
                continue;
            }
            let parsed: RokoConfig = toml::from_str(&after).expect("parse committed source");
            assert_eq!(
                parsed.server.port, 4333,
                "PUT source value was not committed"
            );
            assert_eq!(parsed.server.bind, "127.0.0.2");
            assert!(!parsed.providers.contains_key("global"));
            assert!(!parsed.models.contains_key("global-model"));
            assert!(!after.contains("literal-file-secret"));
            assert!(!after.contains("runtime-fragment"));
            assert!(!after.contains("global-model"));
            assert!(after.contains("authorization_file"));
            assert!(after.contains("${CTRL02_INTERPOLATED}"));
            let replica = std::fs::read_to_string(workdir.join("ephemeral/roko.toml"))
                .expect("read source replica");
            assert_eq!(
                replica, after,
                "replica received effective rather than source bytes"
            );
        }
    }

    #[test]
    fn put_preserves_runtime_overlay_sources_child() {
        let Some(mode) = std::env::var_os("CTRL02_CONFIG_CHILD") else {
            return;
        };
        let mode = mode.to_string_lossy();
        let workdir =
            std::path::PathBuf::from(std::env::var_os("CTRL02_WORKDIR").expect("child workdir"));
        let initial = if mode == "startup" {
            load_config_unified(&workdir).expect("load startup effective config")
        } else {
            RokoConfig::default()
        };
        let state = test_state(workdir.clone(), initial);

        if mode != "startup" {
            reload_config_from_disk(&state).expect("reload effective config");
        }

        let workspace = workdir.join("ephemeral");
        std::fs::create_dir_all(&workspace).expect("create ephemeral workspace");
        state.ephemeral_workspaces.blocking_write().insert(
            "source-replica".to_string(),
            crate::state::WorkspaceInfo {
                id: "source-replica".to_string(),
                path: workspace.clone(),
                created_at: 0,
                last_accessed_at: 0,
                status: crate::state::WorkspaceStatus::Active,
            },
        );

        let before_source =
            std::fs::read_to_string(workdir.join("roko.toml")).expect("read source before PUT");
        let before_live = state.load_roko_config();
        if mode == "reject" {
            std::fs::write(workspace.join("roko.toml"), "replica sentinel\n")
                .expect("seed replica");
            let error = update_config_transaction(
                &state,
                serde_json::json!({
                    "models": {
                        "shadow": {
                            "provider": "project",
                            "slug": "global-slug"
                        }
                    }
                }),
            )
            .expect_err("global merge must make the candidate slug ambiguous");
            assert!(error.to_string().contains("ambiguous model slug"));
            assert_eq!(
                std::fs::read_to_string(workdir.join("roko.toml")).expect("read retained source"),
                before_source
            );
            assert_eq!(*state.load_roko_config(), *before_live);
            assert_eq!(
                std::fs::read_to_string(workspace.join("roko.toml"))
                    .expect("read retained replica"),
                "replica sentinel\n"
            );
            return;
        }

        update_config_transaction(
            &state,
            serde_json::json!({"server": {"bind": "127.0.0.2", "port": 4333}}),
        )
        .expect("commit source-only PUT");

        let effective = state.load_roko_config();
        assert_eq!(effective.agent.default_model, "global-model");
        assert_eq!(
            effective.server.port, 4555,
            "hierarchical env precedence was lost after PUT"
        );
        assert_eq!(effective.server.bind, "127.0.0.2");
        assert!(effective.providers.contains_key("global"));
        let headers = effective.providers["project"]
            .extra_headers
            .as_ref()
            .expect("resolved headers");
        assert_eq!(
            headers.get("authorization").map(String::as_str),
            Some("literal-file-secret")
        );
        assert!(!headers.contains_key("authorization_file"));
        assert_eq!(
            headers.get("trace").map(String::as_str),
            Some("runtime-fragment")
        );
    }
}
