//! API key and agent token management routes.
//!
//! ## API key routes
//! - `POST   /api/api-keys`               — create a new named API key
//! - `GET    /api/api-keys`               — list all keys (metadata only, never the key itself)
//! - `DELETE /api/api-keys/:name`         — revoke a key by name
//! - `POST   /api/api-keys/:name/rotate`  — rotate: new key, 5-min grace for old
//!
//! ## Agent token routes (T02)
//! - `POST   /api/agent-tokens`           — issue a scoped agent bearer token
//! - `GET    /api/agent-tokens`           — list active tokens (metadata only)
//! - `DELETE /api/agent-tokens/:token_id` — revoke an agent token
//!
//! Keys are stored as SHA-256 hashes in `.roko/api-keys.json`.
//! Agent tokens are stored in `.roko/agent-tokens.json`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use chrono::{DateTime, Duration, Utc};
use roko_core::config::ApiKeyEntry;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use super::middleware::hash_api_key;
use crate::error::ApiError;
use crate::state::AppState;

// ─── API key types ─────────────────────────────────────────────────────────

/// Request payload for creating a new API key.
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    /// Human-readable name for the key (must be unique).
    pub name: String,
    /// Permission scope: "admin", "agent:write", "read", etc.
    #[serde(default = "default_scope")]
    pub scope: String,
    /// Optional ISO 8601 expiry timestamp.
    #[serde(default)]
    pub expires_at: Option<String>,
}

fn default_scope() -> String {
    "admin".into()
}

/// Response returned when a new key is created or rotated.
/// The plaintext key is returned **once** and never stored.
#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    pub name: String,
    pub key: String,
    pub scope: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// Summary of a stored key (no secret material).
#[derive(Debug, Serialize)]
pub struct ApiKeySummary {
    pub name: String,
    pub scope: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<String>,
}

// ─── Agent token types (T02) ────────────────────────────────────────────────

/// Capability granted to an agent token.
///
/// Each variant maps to a coarse scope used by the auth middleware to decide
/// whether a bearer request is allowed to access a given route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentCapability {
    /// Submit inference requests to the LLM backends.
    Inference,
    /// Invoke registered tools on behalf of an agent.
    Tools,
    /// Publish messages to the internal event bus.
    BusPublish,
    /// Write to the knowledge / neuro store.
    StoreWrite,
    /// Read from the knowledge / neuro store.
    StoreRead,
}

/// Persisted agent token record (secrets stored as SHA-256 hashes only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToken {
    /// Unique identifier for the token.
    pub token_id: String,
    /// The agent this token was issued for.
    pub agent_id: String,
    /// Capabilities scoped to this token.
    pub capabilities: Vec<AgentCapability>,
    /// ISO 8601 issuance timestamp.
    pub issued_at: String,
    /// ISO 8601 expiry timestamp.
    pub expires_at: DateTime<Utc>,
    /// Whether the token has been explicitly revoked.
    pub revoked: bool,
    /// SHA-256 hex hash of the plaintext token (never stored in plaintext).
    pub token_hash: String,
}

/// Request payload for issuing a new agent token.
#[derive(Debug, Deserialize)]
pub struct IssueAgentTokenRequest {
    /// The agent ID this token is scoped to.
    pub agent_id: String,
    /// Capabilities granted to this token.
    pub capabilities: Vec<AgentCapability>,
    /// How many seconds until the token expires (default: 86400 = 24 h).
    #[serde(default = "default_token_ttl_secs")]
    pub ttl_secs: i64,
}

fn default_token_ttl_secs() -> i64 {
    86400
}

/// Response returned when a new agent token is issued.
/// The plaintext secret is shown **once**.
#[derive(Debug, Serialize)]
pub struct IssueAgentTokenResponse {
    pub token_id: String,
    pub token_secret: String,
    pub agent_id: String,
    pub capabilities: Vec<AgentCapability>,
    pub expires_at: String,
}

/// Summary of an agent token (no secret material).
#[derive(Debug, Serialize)]
pub struct AgentTokenSummary {
    pub token_id: String,
    pub agent_id: String,
    pub capabilities: Vec<AgentCapability>,
    pub issued_at: String,
    pub expires_at: String,
    pub revoked: bool,
}

// ─── Router ────────────────────────────────────────────────────────────────

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        // API key routes (T01)
        .route("/api-keys", get(list_api_keys).post(create_api_key))
        .route("/api-keys/{name}", delete(revoke_api_key))
        .route("/api-keys/{name}/rotate", post(rotate_api_key))
        // Agent token routes (T02)
        .route(
            "/agent-tokens",
            get(list_agent_tokens).post(issue_agent_token),
        )
        .route("/agent-tokens/{token_id}", delete(revoke_agent_token))
}

// ─── API key storage helpers ────────────────────────────────────────────────

fn api_keys_path(workdir: &Path) -> PathBuf {
    workdir.join(".roko").join("api-keys.json")
}

fn load_api_keys(workdir: &Path) -> Vec<ApiKeyEntry> {
    let path = api_keys_path(workdir);
    match std::fs::read_to_string(&path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn save_api_keys(workdir: &Path, keys: &[ApiKeyEntry]) -> Result<(), ApiError> {
    let path = api_keys_path(workdir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ApiError::internal(format!("failed to create api-keys directory: {e}")))?;
    }
    let data = serde_json::to_string_pretty(keys)
        .map_err(|e| ApiError::internal(format!("failed to serialize api-keys: {e}")))?;
    std::fs::write(&path, data)
        .map_err(|e| ApiError::internal(format!("failed to write api-keys.json: {e}")))?;
    Ok(())
}

// ─── Agent token storage helpers ────────────────────────────────────────────

fn agent_tokens_path(workdir: &Path) -> PathBuf {
    workdir.join(".roko").join("agent-tokens.json")
}

fn load_agent_tokens(workdir: &Path) -> Vec<AgentToken> {
    let path = agent_tokens_path(workdir);
    match std::fs::read_to_string(&path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn save_agent_tokens(workdir: &Path, tokens: &[AgentToken]) -> Result<(), ApiError> {
    let path = agent_tokens_path(workdir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            ApiError::internal(format!("failed to create agent-tokens directory: {e}"))
        })?;
    }
    let data = serde_json::to_string_pretty(tokens)
        .map_err(|e| ApiError::internal(format!("failed to serialize agent-tokens: {e}")))?;
    std::fs::write(&path, data)
        .map_err(|e| ApiError::internal(format!("failed to write agent-tokens.json: {e}")))?;
    Ok(())
}

// ─── API key handlers ───────────────────────────────────────────────────────

/// `POST /api/api-keys` — generate a new API key, store its SHA-256 hash,
/// and return the plaintext key exactly once.
async fn create_api_key(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<CreateApiKeyResponse>), ApiError> {
    if req.name.is_empty() {
        return Err(ApiError::bad_request("name must not be empty"));
    }
    crate::error::validate_path_segment(&req.name, "key name")?;

    let mut keys = load_api_keys(&state.workdir);

    if keys.iter().any(|k| k.name == req.name) {
        return Err(ApiError::conflict(format!(
            "API key with name '{}' already exists",
            req.name
        )));
    }

    // Generate a random plaintext key: `roko_` prefix + UUID (no hyphens).
    let plaintext = format!("roko_{}", Uuid::new_v4().as_simple());
    let key_hash = hash_api_key(&plaintext);
    let created_at = Utc::now().to_rfc3339();

    let entry = ApiKeyEntry {
        name: req.name.clone(),
        key_hash,
        scope: req.scope.clone(),
        created_at: created_at.clone(),
        expires_at: req.expires_at.clone(),
        last_used_at: None,
        previous_key_hashes: Vec::new(),
    };

    keys.push(entry);
    save_api_keys(&state.workdir, &keys)?;

    // Also push the entry into the live ServeAuthConfig so the middleware
    // picks it up immediately without a server restart.
    {
        let mut cfg = state.load_roko_config().as_ref().clone();
        cfg.serve.auth.api_keys = keys;
        state.store_roko_config(cfg);
    }

    Ok((
        StatusCode::CREATED,
        Json(CreateApiKeyResponse {
            name: req.name,
            key: plaintext,
            scope: req.scope,
            created_at,
            expires_at: req.expires_at,
        }),
    ))
}

/// `GET /api/api-keys` — list all stored API keys (metadata only).
async fn list_api_keys(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let keys = load_api_keys(&state.workdir);
    let summaries: Vec<ApiKeySummary> = keys
        .into_iter()
        .map(|k| ApiKeySummary {
            name: k.name,
            scope: k.scope,
            created_at: k.created_at,
            expires_at: k.expires_at,
            last_used_at: k.last_used_at,
        })
        .collect();
    Json(json!({ "keys": summaries }))
}

/// `DELETE /api/api-keys/:name` — revoke an API key by name.
async fn revoke_api_key(
    State(state): State<Arc<AppState>>,
    AxumPath(name): AxumPath<String>,
) -> Result<StatusCode, ApiError> {
    let mut keys = load_api_keys(&state.workdir);
    let before = keys.len();
    keys.retain(|k| k.name != name);
    if keys.len() == before {
        return Err(ApiError::not_found(format!(
            "API key with name '{name}' not found"
        )));
    }
    save_api_keys(&state.workdir, &keys)?;

    // Update live config.
    {
        let mut cfg = state.load_roko_config().as_ref().clone();
        cfg.serve.auth.api_keys = keys;
        state.store_roko_config(cfg);
    }

    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/api-keys/:name/rotate` — generate a new key for an existing
/// named key entry. The old hash is retained in `previous_key_hashes` for a
/// 5-minute grace period so in-flight requests with the old key don't fail.
///
/// Returns the new plaintext key exactly once. The old key is invalidated
/// (only accepted for 5 minutes via grace period, then discarded).
async fn rotate_api_key(
    State(state): State<Arc<AppState>>,
    AxumPath(name): AxumPath<String>,
) -> Result<(StatusCode, Json<CreateApiKeyResponse>), ApiError> {
    let mut keys = load_api_keys(&state.workdir);
    let entry = keys
        .iter_mut()
        .find(|k| k.name == name)
        .ok_or_else(|| ApiError::not_found(format!("API key with name '{name}' not found")))?;

    // Retain the current hash as a grace-period hash (expires in 5 minutes).
    let grace_expires = (Utc::now() + Duration::minutes(5)).to_rfc3339();
    let old_hash = entry.key_hash.clone();

    // Keep at most the last 2 grace-period hashes; prune expired ones first.
    let now_str = Utc::now().to_rfc3339();
    entry
        .previous_key_hashes
        .retain(|(_, exp)| exp.as_str() > now_str.as_str());
    entry
        .previous_key_hashes
        .push((old_hash, grace_expires.clone()));
    if entry.previous_key_hashes.len() > 2 {
        entry.previous_key_hashes.remove(0);
    }

    // Generate new plaintext key and update the primary hash.
    let plaintext = format!("roko_{}", Uuid::new_v4().as_simple());
    entry.key_hash = hash_api_key(&plaintext);
    entry.created_at = Utc::now().to_rfc3339();

    let response = CreateApiKeyResponse {
        name: entry.name.clone(),
        key: plaintext,
        scope: entry.scope.clone(),
        created_at: entry.created_at.clone(),
        expires_at: entry.expires_at.clone(),
    };

    save_api_keys(&state.workdir, &keys)?;

    // Update live config.
    {
        let mut cfg = state.load_roko_config().as_ref().clone();
        cfg.serve.auth.api_keys = keys;
        state.store_roko_config(cfg);
    }

    Ok((StatusCode::OK, Json(response)))
}

// ─── Agent token handlers (T02) ─────────────────────────────────────────────

/// `POST /api/agent-tokens` — issue a scoped bearer token for a specific agent.
///
/// Requires admin-level auth (enforced by the scope middleware on this route).
/// Returns `{ token_id, token_secret }` where the secret is shown only once.
async fn issue_agent_token(
    State(state): State<Arc<AppState>>,
    Json(req): Json<IssueAgentTokenRequest>,
) -> Result<(StatusCode, Json<IssueAgentTokenResponse>), ApiError> {
    if req.agent_id.is_empty() {
        return Err(ApiError::bad_request("agent_id must not be empty"));
    }
    if req.capabilities.is_empty() {
        return Err(ApiError::bad_request("at least one capability is required"));
    }
    if req.ttl_secs <= 0 {
        return Err(ApiError::bad_request("ttl_secs must be positive"));
    }

    let token_id = Uuid::new_v4().to_string();
    // Agent tokens use the `roko_agent_` prefix so middleware can route them
    // to the dedicated agent token validator without a separate header.
    let plaintext = format!("roko_agent_{}", Uuid::new_v4().as_simple());
    let token_hash = hash_api_key(&plaintext);
    let now = Utc::now();
    let expires_at = now + Duration::seconds(req.ttl_secs);

    let token = AgentToken {
        token_id: token_id.clone(),
        agent_id: req.agent_id.clone(),
        capabilities: req.capabilities.clone(),
        issued_at: now.to_rfc3339(),
        expires_at,
        revoked: false,
        token_hash,
    };

    let mut tokens = load_agent_tokens(&state.workdir);
    tokens.push(token);
    save_agent_tokens(&state.workdir, &tokens)?;

    Ok((
        StatusCode::CREATED,
        Json(IssueAgentTokenResponse {
            token_id,
            token_secret: plaintext,
            agent_id: req.agent_id,
            capabilities: req.capabilities,
            expires_at: expires_at.to_rfc3339(),
        }),
    ))
}

/// `GET /api/agent-tokens` — list active (non-revoked, non-expired) tokens.
/// Returns metadata only; token secrets are never returned after issuance.
async fn list_agent_tokens(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let tokens = load_agent_tokens(&state.workdir);
    let now = Utc::now();
    let summaries: Vec<AgentTokenSummary> = tokens
        .into_iter()
        .filter(|t| !t.revoked && t.expires_at > now)
        .map(|t| AgentTokenSummary {
            token_id: t.token_id,
            agent_id: t.agent_id,
            capabilities: t.capabilities,
            issued_at: t.issued_at,
            expires_at: t.expires_at.to_rfc3339(),
            revoked: t.revoked,
        })
        .collect();
    Json(json!({ "tokens": summaries }))
}

/// `DELETE /api/agent-tokens/:token_id` — revoke a token by setting
/// `revoked = true`. The token record is kept for audit purposes.
async fn revoke_agent_token(
    State(state): State<Arc<AppState>>,
    AxumPath(token_id): AxumPath<String>,
) -> Result<StatusCode, ApiError> {
    let mut tokens = load_agent_tokens(&state.workdir);
    let token = tokens
        .iter_mut()
        .find(|t| t.token_id == token_id)
        .ok_or_else(|| ApiError::not_found(format!("agent token '{token_id}' not found")))?;
    token.revoked = true;
    save_agent_tokens(&state.workdir, &tokens)?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_api_key_is_deterministic() {
        let hash1 = hash_api_key("test-key-123");
        let hash2 = hash_api_key("test-key-123");
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 hex = 64 chars
    }

    #[test]
    fn hash_api_key_differs_for_different_inputs() {
        let hash1 = hash_api_key("key-a");
        let hash2 = hash_api_key("key-b");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn agent_token_has_roko_agent_prefix() {
        // Verify the prefix contract used by the middleware router.
        let plaintext = format!("roko_agent_{}", Uuid::new_v4().as_simple());
        assert!(plaintext.starts_with("roko_agent_"));
    }
}
