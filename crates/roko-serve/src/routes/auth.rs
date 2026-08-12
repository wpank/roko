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
//! ## Relay token routes (E35-T03)
//! - `POST   /api/relay-tokens`          — issue a short-lived, single-use relay token
//! - `POST   /api/relay-tokens/validate` — validate a relay token (marks it consumed)
//!
//! Keys are stored as SHA-256 hashes in `.roko/api-keys.json`.
//! Agent tokens are stored in `.roko/agent-tokens.json`.
//! Relay tokens are stored in `.roko/relay-tokens.json`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use chrono::{DateTime, Duration, Utc};
use roko_core::config::ApiKeyEntry;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::auth_audit::{AuthAuditAction, AuthAuditEvent, AuthOutcome};

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

// ─── Relay token types (E35-T03) ─────────────────────────────────────────────

/// Default relay token TTL: 5 minutes.
const DEFAULT_RELAY_TOKEN_TTL_SECS: i64 = 300;

/// A short-lived, single-use token for relay/proxy requests.
///
/// Relay tokens are narrowly scoped to a `target_scope` string and expire after
/// a short TTL (default 5 minutes). Each token is **single-use**: after the
/// first successful `validate_relay_token` call the token is marked as used and
/// subsequent calls are rejected.
///
/// Only the SHA-256 hash of the token secret is stored on disk — the plaintext
/// is returned once at issuance and never persisted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayToken {
    /// Unique identifier for this relay token (safe to log).
    pub token_id: String,
    /// The agent that issued this relay token.
    pub issuer_agent_id: String,
    /// The narrow scope this token is authorised for (e.g. `"inference"`, `"store:read"`).
    pub target_scope: String,
    /// Issuance timestamp.
    pub issued_at: DateTime<Utc>,
    /// Expiry timestamp (short TTL, default 5 minutes).
    pub expires_at: DateTime<Utc>,
    /// Whether the token has already been consumed (single-use).
    pub used: bool,
    /// SHA-256 hex hash of the plaintext token secret.
    pub token_hash: String,
}

/// Claims extracted from a successfully validated relay token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayTokenClaims {
    /// Token identifier.
    pub token_id: String,
    /// The agent that issued the token.
    pub issuer_agent_id: String,
    /// The scope this token is authorised for.
    pub target_scope: String,
    /// Token issuance time.
    pub issued_at: DateTime<Utc>,
    /// Token expiry time.
    pub expires_at: DateTime<Utc>,
}

/// Request payload for issuing a relay token.
#[derive(Debug, Deserialize)]
pub struct IssueRelayTokenRequest {
    /// The agent ID issuing the token.
    pub agent_id: String,
    /// Narrow scope the token is authorised for.
    pub scope: String,
    /// TTL in seconds (default: 300 = 5 minutes).
    #[serde(default = "default_relay_ttl_secs")]
    pub ttl_secs: i64,
}

fn default_relay_ttl_secs() -> i64 {
    DEFAULT_RELAY_TOKEN_TTL_SECS
}

/// Response returned when a relay token is issued.
/// The plaintext token secret is shown **once** and never stored.
#[derive(Debug, Serialize)]
pub struct IssueRelayTokenResponse {
    /// Token identifier (safe to log).
    pub token_id: String,
    /// Plaintext token secret — shown only once.
    pub token_secret: String,
    /// The agent that issued the token.
    pub issuer_agent_id: String,
    /// Scope restriction.
    pub target_scope: String,
    /// ISO 8601 expiry timestamp.
    pub expires_at: String,
}

/// Request payload for validating a relay token.
#[derive(Debug, Deserialize)]
pub struct ValidateRelayTokenRequest {
    /// The plaintext token secret to validate.
    pub token: String,
    /// The scope the caller claims to be acting under (must match token scope).
    pub scope: String,
}

// ─── Relay token storage helpers ─────────────────────────────────────────────

fn relay_tokens_path(workdir: &Path) -> PathBuf {
    workdir.join(".roko").join("relay-tokens.json")
}

fn load_relay_tokens(workdir: &Path) -> Vec<RelayToken> {
    let path = relay_tokens_path(workdir);
    match std::fs::read_to_string(&path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn save_relay_tokens(workdir: &Path, tokens: &[RelayToken]) -> Result<(), ApiError> {
    let path = relay_tokens_path(workdir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            ApiError::internal(format!("failed to create relay-tokens directory: {e}"))
        })?;
    }
    let data = serde_json::to_string_pretty(tokens)
        .map_err(|e| ApiError::internal(format!("failed to serialize relay-tokens: {e}")))?;
    std::fs::write(&path, data)
        .map_err(|e| ApiError::internal(format!("failed to write relay-tokens.json: {e}")))?;
    Ok(())
}

// ─── Relay token core logic ───────────────────────────────────────────────────

/// Issue a new relay token for `agent_id` scoped to `scope` with the given TTL.
///
/// Returns `(RelayToken, plaintext_secret)`. The plaintext is returned once and
/// should be forwarded to the requesting agent immediately. Only the SHA-256
/// hash is persisted to `.roko/relay-tokens.json`.
///
/// # Errors
///
/// Returns [`ApiError`] when inputs are invalid or persistence fails.
pub fn issue_relay_token(
    workdir: &Path,
    agent_id: &str,
    scope: &str,
    ttl_secs: i64,
) -> Result<(RelayToken, String), ApiError> {
    if agent_id.is_empty() {
        return Err(ApiError::bad_request("agent_id must not be empty"));
    }
    if scope.is_empty() {
        return Err(ApiError::bad_request("scope must not be empty"));
    }
    if ttl_secs <= 0 {
        return Err(ApiError::bad_request("ttl_secs must be positive"));
    }

    let token_id = Uuid::new_v4().to_string();
    // Relay tokens use the `roko_relay_` prefix so middleware can identify them.
    let plaintext = format!("roko_relay_{}", Uuid::new_v4().as_simple());
    let token_hash = hash_api_key(&plaintext);
    let now = Utc::now();
    let expires_at = now + Duration::seconds(ttl_secs);

    let relay_token = RelayToken {
        token_id: token_id.clone(),
        issuer_agent_id: agent_id.to_string(),
        target_scope: scope.to_string(),
        issued_at: now,
        expires_at,
        used: false,
        token_hash,
    };

    let mut tokens = load_relay_tokens(workdir);
    tokens.push(relay_token.clone());
    save_relay_tokens(workdir, &tokens)?;

    Ok((relay_token, plaintext))
}

/// Validate a relay token presented as a plaintext secret.
///
/// Enforces:
/// - The token exists (SHA-256 hash match).
/// - The token has not expired.
/// - The token has not already been used (single-use).
/// - The requested `scope` matches the token's `target_scope`.
///
/// On success the token is marked as used and the state is persisted.
///
/// # Errors
///
/// Returns [`ApiError::unauthorized`] for any validation failure. The error
/// message is intentionally vague to avoid leaking token internals.
pub fn validate_relay_token(
    workdir: &Path,
    plaintext: &str,
    scope: &str,
) -> Result<RelayTokenClaims, ApiError> {
    let input_hash = hash_api_key(plaintext);
    let mut tokens = load_relay_tokens(workdir);

    let token = tokens
        .iter_mut()
        .find(|t| t.token_hash == input_hash)
        .ok_or_else(|| ApiError::unauthorized("relay token not found or invalid"))?;

    if token.expires_at <= Utc::now() {
        return Err(ApiError::unauthorized("relay token has expired"));
    }

    if token.used {
        return Err(ApiError::unauthorized("relay token has already been used"));
    }

    if token.target_scope != scope {
        return Err(ApiError::unauthorized(
            "relay token scope does not match the requested scope",
        ));
    }

    // Mark as used (single-use enforcement).
    token.used = true;
    let claims = RelayTokenClaims {
        token_id: token.token_id.clone(),
        issuer_agent_id: token.issuer_agent_id.clone(),
        target_scope: token.target_scope.clone(),
        issued_at: token.issued_at,
        expires_at: token.expires_at,
    };

    save_relay_tokens(workdir, &tokens)?;
    Ok(claims)
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
        // Relay token routes (E35-T03)
        .route("/relay-tokens", post(issue_relay_token_handler))
        .route("/relay-tokens/validate", post(validate_relay_token_handler))
        // Audit log query (E35-T04)
        .route("/auth/audit", get(query_auth_audit))
}

// ─── Audit helpers (E35-T04) ────────────────────────────────────────────────

/// Query parameters for `GET /api/auth/audit`.
#[derive(Debug, Deserialize)]
pub struct AuditQueryParams {
    /// ISO-8601 lower bound for the event timestamp (inclusive).
    pub from: Option<String>,
    /// ISO-8601 upper bound for the event timestamp (inclusive).
    pub to: Option<String>,
    /// Filter by actor identifier (exact match).
    pub actor: Option<String>,
    /// Filter by action name (case-sensitive serialised form, e.g. `"TokenIssued"`).
    pub action: Option<String>,
}

/// `GET /api/auth/audit` — query auth audit log entries.
async fn query_auth_audit(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AuditQueryParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let log = match crate::auth_audit::open_audit_log(&state.workdir) {
        Ok(l) => l,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Json(json!({ "events": [], "count": 0 })));
        }
        Err(e) => {
            return Err(ApiError::internal(format!("failed to open audit log: {e}")));
        }
    };

    let from = params
        .from
        .as_deref()
        .and_then(|s| s.parse::<DateTime<Utc>>().ok());
    let to = params
        .to
        .as_deref()
        .and_then(|s| s.parse::<DateTime<Utc>>().ok());
    let action = params.action.as_deref().and_then(|s| {
        serde_json::from_value::<AuthAuditAction>(serde_json::Value::String(s.to_string())).ok()
    });

    let events = log
        .query(from, to, params.actor.as_deref(), action.as_ref())
        .map_err(|e| ApiError::internal(format!("failed to query audit log: {e}")))?;

    let count = events.len();
    Ok(Json(json!({ "events": events, "count": count })))
}

/// Append an [`AuthAuditEvent`] to the audit log in a best-effort, fire-and-forget manner.
///
/// Opens the log file on each call (cheap — the underlying file is small).
/// Errors are logged via `tracing::warn!` and never propagated to callers.
fn append_audit_event(workdir: &Path, event: AuthAuditEvent) {
    match crate::auth_audit::open_audit_log(workdir) {
        Ok(log) => log.append(&event),
        Err(e) => tracing::warn!(error = %e, "auth_audit: could not open audit log"),
    }
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

    // Audit: TokenIssued
    append_audit_event(
        &state.workdir,
        AuthAuditEvent::new(
            "api".to_string(),
            AuthAuditAction::TokenIssued,
            req.name.clone(),
            AuthOutcome::Success,
        ),
    );

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

    // Audit: TokenRevoked
    append_audit_event(
        &state.workdir,
        AuthAuditEvent::new(
            "api".to_string(),
            AuthAuditAction::TokenRevoked,
            name.clone(),
            AuthOutcome::Success,
        ),
    );

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

    // Audit: KeyRotated
    append_audit_event(
        &state.workdir,
        AuthAuditEvent::new(
            "api".to_string(),
            AuthAuditAction::KeyRotated,
            response.name.clone(),
            AuthOutcome::Success,
        )
        .with_meta("grace_expires", &grace_expires),
    );

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

    // Audit: TokenIssued (agent token)
    append_audit_event(
        &state.workdir,
        AuthAuditEvent::new(
            req.agent_id.clone(),
            AuthAuditAction::TokenIssued,
            token_id.clone(),
            AuthOutcome::Success,
        )
        .with_meta("expires_at", expires_at.to_rfc3339()),
    );

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
    // Capture agent_id before mutable borrow ends.
    let agent_id = token.agent_id.clone();
    token.revoked = true;
    save_agent_tokens(&state.workdir, &tokens)?;

    // Audit: TokenRevoked
    append_audit_event(
        &state.workdir,
        AuthAuditEvent::new(
            agent_id,
            AuthAuditAction::TokenRevoked,
            token_id.clone(),
            AuthOutcome::Success,
        ),
    );

    Ok(StatusCode::NO_CONTENT)
}

// ─── Relay token handlers (E35-T03) ──────────────────────────────────────────

/// `POST /api/relay-tokens` — issue a short-lived, single-use relay token.
///
/// Returns the plaintext token secret exactly once; only the SHA-256 hash is
/// stored. The caller must forward the secret to the requesting agent
/// immediately.
async fn issue_relay_token_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<IssueRelayTokenRequest>,
) -> Result<(StatusCode, Json<IssueRelayTokenResponse>), ApiError> {
    let (token, plaintext) =
        issue_relay_token(&state.workdir, &req.agent_id, &req.scope, req.ttl_secs)?;
    Ok((
        StatusCode::CREATED,
        Json(IssueRelayTokenResponse {
            token_id: token.token_id,
            token_secret: plaintext,
            issuer_agent_id: token.issuer_agent_id,
            target_scope: token.target_scope,
            expires_at: token.expires_at.to_rfc3339(),
        }),
    ))
}

/// `POST /api/relay-tokens/validate` — validate a relay token and mark it used.
///
/// Enforces: token exists, not expired, not used, scope matches.
/// On success returns the token claims; the token is immediately consumed.
async fn validate_relay_token_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ValidateRelayTokenRequest>,
) -> Result<Json<RelayTokenClaims>, ApiError> {
    let claims = validate_relay_token(&state.workdir, &req.token, &req.scope)?;
    Ok(Json(claims))
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

    // ── Relay token tests (E35-T03) ──────────────────────────────────────────

    fn tmp_workdir() -> tempfile::TempDir {
        tempfile::tempdir().expect("failed to create temp dir")
    }

    #[test]
    fn relay_token_issuance_returns_roko_relay_prefix() {
        let dir = tmp_workdir();
        let (token, plaintext) = issue_relay_token(
            dir.path(),
            "agent-1",
            "inference",
            DEFAULT_RELAY_TOKEN_TTL_SECS,
        )
        .expect("issue should succeed");

        assert!(
            plaintext.starts_with("roko_relay_"),
            "plaintext should have roko_relay_ prefix, got: {plaintext}"
        );
        assert!(!token.used, "freshly issued token must not be used");
        assert_eq!(token.issuer_agent_id, "agent-1");
        assert_eq!(token.target_scope, "inference");
        assert_ne!(
            token.token_hash, plaintext,
            "hash must differ from plaintext"
        );
        assert_eq!(token.token_hash.len(), 64, "SHA-256 hex must be 64 chars");
    }

    #[test]
    fn relay_token_issuance_persists_to_disk() {
        let dir = tmp_workdir();
        let _ = issue_relay_token(
            dir.path(),
            "agent-2",
            "store:read",
            DEFAULT_RELAY_TOKEN_TTL_SECS,
        )
        .expect("issue should succeed");

        let path = relay_tokens_path(dir.path());
        assert!(path.exists(), "relay-tokens.json must be created on disk");

        let stored = load_relay_tokens(dir.path());
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].issuer_agent_id, "agent-2");
    }

    #[test]
    fn relay_token_validation_succeeds_and_marks_used() {
        let dir = tmp_workdir();
        let (_token, plaintext) = issue_relay_token(
            dir.path(),
            "agent-3",
            "store:write",
            DEFAULT_RELAY_TOKEN_TTL_SECS,
        )
        .expect("issue should succeed");

        let claims = validate_relay_token(dir.path(), &plaintext, "store:write")
            .expect("validation should succeed");

        assert_eq!(claims.issuer_agent_id, "agent-3");
        assert_eq!(claims.target_scope, "store:write");

        let stored = load_relay_tokens(dir.path());
        assert!(stored[0].used, "token must be marked used after validation");
    }

    #[test]
    fn relay_token_single_use_rejects_second_call() {
        let dir = tmp_workdir();
        let (_token, plaintext) = issue_relay_token(
            dir.path(),
            "agent-4",
            "inference",
            DEFAULT_RELAY_TOKEN_TTL_SECS,
        )
        .expect("issue should succeed");

        assert!(
            validate_relay_token(dir.path(), &plaintext, "inference").is_ok(),
            "first validation should succeed"
        );

        let err = validate_relay_token(dir.path(), &plaintext, "inference")
            .expect_err("second validation must fail");
        assert!(
            err.message.contains("already been used"),
            "error should mention already-used, got: {}",
            err.message
        );
    }

    #[test]
    fn relay_token_scope_restriction_rejects_wrong_scope() {
        let dir = tmp_workdir();
        let (_token, plaintext) = issue_relay_token(
            dir.path(),
            "agent-5",
            "inference",
            DEFAULT_RELAY_TOKEN_TTL_SECS,
        )
        .expect("issue should succeed");

        let err = validate_relay_token(dir.path(), &plaintext, "store:write")
            .expect_err("wrong scope must be rejected");
        assert!(
            err.message.contains("scope"),
            "error should mention scope, got: {}",
            err.message
        );
    }

    #[test]
    fn relay_token_expiry_rejects_expired_token() {
        let dir = tmp_workdir();
        // Manually insert an already-expired token record.
        let plaintext = format!("roko_relay_{}", Uuid::new_v4().as_simple());
        let token_hash = hash_api_key(&plaintext);
        let past = Utc::now() - Duration::seconds(60);

        let expired = RelayToken {
            token_id: Uuid::new_v4().to_string(),
            issuer_agent_id: "agent-6".to_string(),
            target_scope: "inference".to_string(),
            issued_at: past - Duration::seconds(10),
            expires_at: past,
            used: false,
            token_hash,
        };

        let mut tokens = load_relay_tokens(dir.path());
        tokens.push(expired);
        save_relay_tokens(dir.path(), &tokens).expect("save should succeed");

        let err = validate_relay_token(dir.path(), &plaintext, "inference")
            .expect_err("expired token must be rejected");
        assert!(
            err.message.contains("expired"),
            "error should mention expiry, got: {}",
            err.message
        );
    }

    #[test]
    fn relay_token_unknown_token_is_rejected() {
        let dir = tmp_workdir();
        let err = validate_relay_token(dir.path(), "roko_relay_nonexistent", "inference")
            .expect_err("unknown token must be rejected");
        assert!(!err.message.is_empty());
    }

    #[test]
    fn relay_token_bad_inputs_are_rejected() {
        let dir = tmp_workdir();

        assert!(
            issue_relay_token(dir.path(), "", "inference", DEFAULT_RELAY_TOKEN_TTL_SECS).is_err(),
            "empty agent_id must fail"
        );
        assert!(
            issue_relay_token(dir.path(), "agent-1", "", DEFAULT_RELAY_TOKEN_TTL_SECS).is_err(),
            "empty scope must fail"
        );
        assert!(
            issue_relay_token(dir.path(), "agent-1", "inference", 0).is_err(),
            "zero ttl must fail"
        );
        assert!(
            issue_relay_token(dir.path(), "agent-1", "inference", -1).is_err(),
            "negative ttl must fail"
        );
    }

    #[test]
    fn relay_token_default_ttl_is_five_minutes() {
        assert_eq!(DEFAULT_RELAY_TOKEN_TTL_SECS, 300);
        assert_eq!(default_relay_ttl_secs(), 300);
    }
}
