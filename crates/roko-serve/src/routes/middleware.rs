//! Shared API auth and scrubbing middleware for `/api/*` routes.

use std::sync::{Arc, OnceLock};

use axum::body::Body;
use axum::extract::State;
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{HeaderMap, Method, Request};
use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use base64::Engine;
use chrono::Utc;
use roko_chain::x402::{PaymentAuthorization, PaymentRequest};
use roko_core::config::{ApiKeyEntry, ServeAuthConfig};
use roko_core::feed::{FeedAccess, FeedInfo};
use roko_core::obs::LogScrubber;
use sha2::{Digest, Sha256};
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::error::ApiError;
use crate::rbac::{Permission, Role, enforce_permission};
use crate::routes::auth::{AgentCapability, AgentCredentialClaims, parse_rfc3339};
use crate::state::AppState;

static UNSAFE_PUBLIC_CORS_WARNING: OnceLock<()> = OnceLock::new();

/// Header containing an ERC-3009 payment authorization as JSON.
pub const X_PAYMENT_AUTHORIZATION: &str = "x-payment-authorization";

/// Header carrying the same payment challenge returned in an HTTP 402 body.
pub const X_PAYMENT_REQUEST: &str = "x-payment-request";

/// Verify the structural x402 authorization required to read a paid feed.
///
/// Public and private feeds bypass the payment cell. Signature verification is
/// deliberately deferred to the chain settlement layer; this boundary checks
/// only JSON shape and that the authorized value covers the advertised price.
// Axum handlers consume `Response` directly on denial; boxing it here would only move the
// allocation and complicate every caller without reducing the HTTP response itself.
#[allow(clippy::result_large_err)]
pub fn require_payment(feed: &FeedInfo, headers: &HeaderMap) -> Result<(), Response> {
    if feed.access != FeedAccess::Paid {
        return Ok(());
    }

    let required_amount = feed
        .pricing
        .as_ref()
        .map(|pricing| {
            let adjusted = pricing.per_request_cost * pricing.tier.price_multiplier();
            if adjusted.is_finite() && adjusted > 0.0 {
                adjusted.ceil() as u128
            } else {
                0
            }
        })
        .unwrap_or(1);
    if required_amount == 0 {
        return Ok(());
    }

    let now = u64::try_from(Utc::now().timestamp()).unwrap_or(0);
    let challenge = PaymentRequest {
        recipient: feed.agent_id.clone(),
        amount: required_amount,
        token: "KORAI".to_string(),
        nonce: u128::from(now),
        deadline: now.saturating_add(600),
        reason: format!("paid feed access: {}", feed.id),
    };

    let authorization = headers
        .get(X_PAYMENT_AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| serde_json::from_str::<PaymentAuthorization>(value).ok());

    match authorization {
        Some(authorization) if authorization.value >= required_amount => Ok(()),
        _ => Err(payment_required(challenge)),
    }
}

fn payment_required(challenge: PaymentRequest) -> Response {
    let challenge_header = serde_json::to_string(&challenge)
        .ok()
        .and_then(|value| HeaderValue::from_str(&value).ok());
    let mut response = (StatusCode::PAYMENT_REQUIRED, axum::Json(challenge)).into_response();
    if let Some(value) = challenge_header {
        response.headers_mut().insert(X_PAYMENT_REQUEST, value);
    }
    response
}

// ── Extension route whitelist ─────────────────────────────────────────────────
//
// Routes registered by plugins/extensions must declare an explicit scope
// requirement. At server startup the plugin loader calls
// [`register_extension_route_scopes`] with the (path-prefix, scope) pairs
// declared in each plugin manifest's `[[triggers]]` webhook section.
//
// [`required_scope_for`] checks these runtime-registered entries *before*
// falling through to [`SCOPE_WRITE_UNCLASSIFIED`], so an extension route that
// declares its scope is never misclassified as unclassified.

/// Runtime-registered extension route scopes.
///
/// Populated once at startup by [`register_extension_route_scopes`].
/// Each entry is `(path_prefix, required_scope)`.
static EXTENSION_ROUTE_SCOPES: OnceLock<Vec<(String, String)>> = OnceLock::new();

/// Register extension (plugin) route scopes at server startup.
///
/// Must be called **once** before the first request arrives. Subsequent calls
/// are no-ops (the `OnceLock` is already set). Each entry is
/// `(path_prefix, required_scope)`; prefixes are matched via `starts_with` in
/// the same way as [`ROUTE_SCOPE_MANIFEST`].
///
/// ### Scope values
///
/// Use the same vocabulary as the static manifest:
/// `"read"`, `"write"`, `"admin"`, `"agent:write"`, `"plan:write"`,
/// `"terminal:write"`. Any value not in that set will be treated as an unknown
/// scope and will fail `is_scope_sufficient` for all callers except `"admin"`.
///
/// Callers that cannot call this function at startup (e.g. integration tests)
/// may skip it; extension paths will then resolve to [`SCOPE_WRITE_UNCLASSIFIED`]
/// as before.
pub fn register_extension_route_scopes(entries: impl IntoIterator<Item = (String, String)>) {
    // OnceLock::set returns Err if already set; silently ignore re-entrant
    // startup paths (test harnesses, hot-reload stubs) so they don't panic.
    let _ = EXTENSION_ROUTE_SCOPES.set(entries.into_iter().collect());
}

/// Look up the required scope for a path in the extension route registry.
///
/// Returns `Some(scope_str)` if the path matches a registered extension prefix,
/// `None` otherwise.
fn extension_scope_for(path: &str) -> Option<&str> {
    let entries = EXTENSION_ROUTE_SCOPES.get()?;
    for (prefix, scope) in entries {
        if path.starts_with(prefix.as_str()) {
            return Some(scope.as_str());
        }
    }
    None
}

/// Map a runtime scope string to a stable `&'static str` token.
///
/// Extension routes store their scope as an owned `String`. This helper maps
/// recognised scope values to their `&'static str` equivalents so
/// [`required_scope_for`] can keep its `&'static str` return type.
/// Unrecognised values fall through to [`SCOPE_WRITE_UNCLASSIFIED`] which is
/// the fail-closed sentinel, but this should not happen for well-formed plugin
/// manifests validated by `roko-plugin`.
fn known_static_scope(scope: &str) -> &'static str {
    match scope {
        "read" => "read",
        "write" => "write",
        "admin" => "admin",
        "agent:write" => "agent:write",
        "plan:write" => "plan:write",
        "terminal:write" => "terminal:write",
        _ => SCOPE_WRITE_UNCLASSIFIED,
    }
}

/// Extract a bearer token from an `Authorization` header value.
///
/// Performs case-insensitive prefix matching on "bearer", trims whitespace,
/// and returns `None` if the token portion is empty.
pub fn extract_bearer_token(header_value: &str) -> Option<&str> {
    let lower = header_value.as_bytes();
    if lower.len() < 7 {
        return None;
    }
    if !lower[..6].eq_ignore_ascii_case(b"bearer") {
        return None;
    }
    let rest = &header_value[6..];
    // Must be followed by whitespace (or be exactly "bearer" + space).
    let token = rest.trim();
    if token.is_empty() {
        return None;
    }
    Some(token)
}

/// Returns `true` when `token` looks structurally like a JWT (three
/// non-empty dot-separated segments of valid base64url characters).
///
/// No signature verification is performed.
pub fn is_structurally_valid_jwt(token: &str) -> bool {
    let segments: Vec<&str> = token.split('.').collect();
    if segments.len() != 3 {
        return false;
    }
    segments
        .iter()
        .all(|s| !s.is_empty() && s.bytes().all(is_base64url_byte))
}

fn is_base64url_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'='
}

/// Which authentication method was used for a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMethod {
    /// Authenticated via `X-Api-Key` header.
    ApiKey,
    /// Authenticated via a structurally valid JWT in `Authorization: Bearer`.
    Jwt,
    /// Authenticated via a non-JWT bearer token.
    Bearer,
    /// Authenticated via a deployment-scoped worker callback token.
    WorkerToken,
}

impl AuthMethod {
    /// Machine-readable label set in the `X-Auth-Method` response header.
    pub fn header_value(self) -> &'static str {
        match self {
            Self::ApiKey => "api_key",
            Self::Jwt => "jwt",
            Self::Bearer => "bearer",
            Self::WorkerToken => "worker_token",
        }
    }
}

fn worker_callback_id<'a>(method: &Method, path: &'a str) -> Option<&'a str> {
    if method != Method::POST {
        return None;
    }
    path.strip_prefix("/api/deployments/")
        .or_else(|| path.strip_prefix("/deployments/"))?
        .strip_suffix("/callback")
        .filter(|id| !id.is_empty() && !id.contains('/'))
}

/// Authenticated caller context injected into request extensions.
///
/// Routes can extract this via `req.extensions().get::<AuthContext>()` or
/// the axum `Extension<AuthContext>` extractor.
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// How the caller authenticated.
    pub method: AuthMethod,
    /// Permission scope (e.g. "admin", "agent:write", "read").
    pub scope: String,
    /// Optional user/key identifier.
    pub user_id: Option<String>,
}

/// Agent identity authenticated from either modern or legacy agent credentials.
#[derive(Debug, Clone)]
struct AuthenticatedAgentId(String);

/// Compute the hex-encoded SHA-256 hash of a plaintext API key.
pub fn hash_api_key(plaintext: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(plaintext.as_bytes());
    let digest = hasher.finalize();
    // Inline hex encoding to avoid adding a `hex` dependency.
    digest.iter().fold(String::with_capacity(64), |mut s, b| {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// Result of matching a token against the stored API key entries.
#[derive(Debug)]
enum ApiKeyMatchResult<'a> {
    /// Token matched an active, non-expired key.
    Valid(&'a ApiKeyEntry),
    /// Token matched a key that is past its `expires_at`.
    Expired(&'a ApiKeyEntry),
    /// Token matched a previous (rotated) hash within its 5-minute grace period.
    GracePeriod(&'a ApiKeyEntry),
    /// Token did not match any stored key (including grace period hashes).
    NotFound,
}

/// Check an API key against the list of named key entries.
///
/// Returns a discriminated match result so callers can distinguish an expired
/// key from a wrong key, enabling the `X-Key-Expired` response header.
fn match_api_key_entry<'a>(token: &str, entries: &'a [ApiKeyEntry]) -> ApiKeyMatchResult<'a> {
    let token_hash = hash_api_key(token);
    let now = Utc::now();

    for entry in entries {
        // Check current key hash.
        if entry.key_hash == token_hash {
            if let Some(ref expires) = entry.expires_at {
                // Malformed persisted expiry data fails closed.
                if parse_rfc3339(expires).is_none_or(|expires| expires <= now) {
                    return ApiKeyMatchResult::Expired(entry);
                }
            }
            return ApiKeyMatchResult::Valid(entry);
        }
        // Check previous (rotated) hashes still within their grace period.
        for (prev_hash, grace_expires) in &entry.previous_key_hashes {
            if prev_hash == &token_hash
                && parse_rfc3339(grace_expires).is_some_and(|expires| expires > now)
            {
                return ApiKeyMatchResult::GracePeriod(entry);
            }
        }
    }
    ApiKeyMatchResult::NotFound
}

enum ApiCredential<'a> {
    Missing,
    XApiKey(&'a str),
    InvalidXApiKey,
    Bearer(&'a str),
    InvalidAuthorization,
}

fn api_credential(headers: &HeaderMap) -> ApiCredential<'_> {
    if let Some(value) = headers.get("X-Api-Key") {
        return match value.to_str() {
            Ok(value) => ApiCredential::XApiKey(value),
            Err(_) => ApiCredential::InvalidXApiKey,
        };
    }

    if let Some(value) = headers.get(AUTHORIZATION) {
        return match value.to_str() {
            Ok(value) => match extract_bearer_token(value) {
                Some(token) => ApiCredential::Bearer(token),
                None => ApiCredential::InvalidAuthorization,
            },
            Err(_) => ApiCredential::InvalidAuthorization,
        };
    }

    ApiCredential::Missing
}

/// Outcome of an API-key authentication attempt.
#[derive(Debug)]
enum ApiKeyAuthResult {
    /// Authentication succeeded.
    Ok(AuthMethod, String, Option<String>),
    /// The token matched a key that has expired.
    KeyExpired(String),
    /// The token did not match any key.
    NoMatch,
}

/// Authenticate the supplied token against the legacy single key and the
/// named `api_keys` list. Returns an [`ApiKeyAuthResult`] so callers can
/// distinguish an expired key from a wrong key and emit `X-Key-Expired`.
///
/// This function handles API-key-based auth only — Privy JWT verification
/// is handled asynchronously by [`try_privy_jwt`].
fn authenticate_api_key(
    token: &str,
    auth: &ServeAuthConfig,
    entries: &[ApiKeyEntry],
    via_header: bool,
) -> ApiKeyAuthResult {
    // 1. Try named API keys first.
    match match_api_key_entry(token, entries) {
        ApiKeyMatchResult::Valid(entry) | ApiKeyMatchResult::GracePeriod(entry) => {
            let method = if via_header {
                AuthMethod::ApiKey
            } else if is_structurally_valid_jwt(token) {
                AuthMethod::Jwt
            } else {
                AuthMethod::Bearer
            };
            return ApiKeyAuthResult::Ok(method, entry.scope.clone(), Some(entry.name.clone()));
        }
        ApiKeyMatchResult::Expired(entry) => {
            return ApiKeyAuthResult::KeyExpired(entry.name.clone());
        }
        ApiKeyMatchResult::NotFound => {}
    }

    // 2. Fall back to legacy single api_key for backwards compatibility.
    if !auth.api_key.is_empty() && token == auth.api_key {
        let method = if via_header {
            AuthMethod::ApiKey
        } else if is_structurally_valid_jwt(token) {
            AuthMethod::Jwt
        } else {
            AuthMethod::Bearer
        };
        return ApiKeyAuthResult::Ok(method, "admin".to_string(), None);
    }

    ApiKeyAuthResult::NoMatch
}

/// Attempt to validate a Bearer token as a Privy JWT using the JWKS cache.
///
/// Performs three checks in order:
/// 1. Signature + app-id verification via JWKS.
/// 2. Workspace membership: if `privy_workspace_id` is configured the JWT
///    `org_id` claim **must** match. Tokens without an `org_id` claim are
///    rejected (fail closed).
/// 3. Role authorization: if `privy_allowed_roles` is non-empty the JWT
///    `role` claim must be present and contained in the allowed list.
///    Tokens with an unrecognised or missing role are downgraded to
///    `"read"` scope instead of receiving `"admin"`.
async fn try_privy_jwt(
    token: &str,
    auth: &ServeAuthConfig,
    state: &Arc<AppState>,
) -> Option<(AuthMethod, String, Option<String>)> {
    let privy_app_id = auth.privy_app_id.as_deref()?;
    if !is_structurally_valid_jwt(token) {
        return None;
    }
    let claims = state.jwks_cache.validate(token, privy_app_id).await?;

    // --- Workspace / org membership check (fail closed) ---
    if let Some(ref required_workspace) = auth.privy_workspace_id {
        match claims.org_id.as_deref() {
            Some(org) if org == required_workspace.as_str() => { /* membership confirmed */ }
            _ => {
                tracing::warn!(
                    sub = %claims.sub,
                    org_id = ?claims.org_id,
                    required = %required_workspace,
                    "Privy JWT rejected: workspace membership mismatch or missing org_id"
                );
                return None;
            }
        }
    }

    // --- Role authorization ---
    let scope = if auth.privy_allowed_roles.is_empty() {
        // No role filter configured — grant admin (legacy behaviour).
        "admin".to_string()
    } else {
        match claims.role.as_deref() {
            Some(role) if auth.privy_allowed_roles.iter().any(|r| r == role) => "admin".to_string(),
            _ => {
                tracing::info!(
                    sub = %claims.sub,
                    role = ?claims.role,
                    allowed = ?auth.privy_allowed_roles,
                    "Privy JWT role not in allowed list — downgrading to read scope"
                );
                "read".to_string()
            }
        }
    };

    Some((AuthMethod::Jwt, scope, Some(claims.sub)))
}

/// Attempt to validate a Bearer token as an agent token.
///
/// Agent tokens are issued via `POST /api/agents/{id}/token` and stored as
/// `base64(SHA-256(token))` in `DiscoveredAgent.token_hash`. Returns the
/// matching agent_id on success.
async fn try_agent_token(
    token: &str,
    state: &Arc<AppState>,
) -> Option<(AuthMethod, String, Option<String>)> {
    // Compute the same hash format used by rotate_agent_token().
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let digest = hasher.finalize();
    let token_hash = base64::engine::general_purpose::STANDARD_NO_PAD.encode(digest);

    let agents = state.discovered_agents.read().await;
    for agent in agents.values() {
        if agent.token_hash.as_deref() == Some(&token_hash) {
            // Check expiry.
            if let Some(expires_at) = agent.token_expires_at {
                if Utc::now() > expires_at {
                    return None; // Token expired.
                }
            }
            return Some((
                AuthMethod::Bearer,
                "agent:write".to_string(),
                Some(agent.agent_id.clone()),
            ));
        }
    }
    None
}

/// Validate a `roko_agent_`-prefixed bearer token against the stored agent
/// token registry (`.roko/agent-tokens.json`).
///
/// Returns `(AuthMethod, scope, user_id)` on success where scope is derived
/// from the token's `capabilities` field and `user_id` is the `agent_id`.
async fn try_named_agent_token(
    token: &str,
    state: &Arc<AppState>,
) -> Option<AgentCredentialClaims> {
    state.auth_registry.authenticate_agent_secret(token).await
}

/// Resolve the concrete capability needed for an agent-token request.
/// Unclassified routes return `None` and fail closed for agent tokens.
fn required_agent_capability(method: &Method, path: &str) -> Option<AgentCapability> {
    if agent_observation_target(method, path).is_some() {
        Some(AgentCapability::BusPublish)
    } else if path.starts_with("/api/inference") || path.starts_with("/inference") {
        Some(AgentCapability::Inference)
    } else if path.starts_with("/api/rpc") || path.starts_with("/rpc") {
        Some(AgentCapability::Tools)
    } else if path.starts_with("/api/events/ingest")
        || path.starts_with("/events/ingest")
        || path == "/relay"
        || path.starts_with("/relay/")
    {
        Some(AgentCapability::BusPublish)
    } else if path.starts_with("/api/neuro/query") || path.starts_with("/neuro/query") {
        Some(AgentCapability::StoreRead)
    } else if path.starts_with("/api/neuro") || path.starts_with("/neuro") {
        if matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS) {
            Some(AgentCapability::StoreRead)
        } else {
            Some(AgentCapability::StoreWrite)
        }
    } else if path.starts_with("/api/learning") || path.starts_with("/learning") {
        Some(AgentCapability::StoreRead)
    } else {
        None
    }
}

fn agent_observation_target<'a>(method: &Method, path: &'a str) -> Option<&'a str> {
    if *method != Method::POST {
        return None;
    }
    path.strip_prefix("/api/agents/")
        .or_else(|| path.strip_prefix("/agents/"))?
        .strip_suffix("/observation")
        .filter(|agent| !agent.is_empty() && !agent.contains('/'))
}

fn is_relay_delegation_endpoint(method: &Method, path: &str) -> bool {
    *method == Method::POST && matches!(path, "/api/relay-tokens" | "/relay-tokens")
}

fn append_auth_audit(state: &AppState, event: crate::auth_audit::AuthAuditEvent) {
    if let Some(log) = state.auth_audit.as_ref() {
        log.append(&event);
    }
}

/// Build a 401 response with `X-Key-Expired: true` to signal the client that
/// the key was recognised but has passed its expiry timestamp.
fn expired_key_response() -> Response {
    let mut resp = ApiError::unauthorized("API key has expired").into_response();
    resp.headers_mut().insert(
        HeaderName::from_static("x-key-expired"),
        axum::http::HeaderValue::from_static("true"),
    );
    resp
}

/// Require a matching API credential for the request to continue.
///
/// Supports callback-scoped credentials on the worker callback route, then
/// five user/agent credential sources:
/// 1. `X-Roko-Worker-Token` on `POST /api/deployments/:id/callback`
/// 2. `X-Api-Key` header (API key only)
/// 3. `Authorization: Bearer roko_agent_*` — agent bearer tokens (T02)
/// 4. `Authorization: Bearer <token>` matched against API keys
/// 5. `Authorization: Bearer <jwt>` verified via Privy JWKS
/// 6. Named API keys from `api_keys` list (SHA-256 hash comparison)
///
/// On success, injects [`AuthContext`] into request extensions so downstream
/// routes can inspect the caller's scope and identity.
///
/// Expired keys return 401 with `X-Key-Expired: true` so clients can
/// distinguish "wrong key" from "key needs rotation".
pub async fn require_api_key(
    State(state): State<Arc<AppState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let auth = state.load_roko_config().serve.auth.clone();
    let named_api_keys = state.auth_registry.api_keys_snapshot().await;
    let route_label = format!("{} {}", req.method(), req.uri().path());

    // Worker callbacks authenticate with their own narrowly-scoped token, not
    // a user-facing API key. Validate it here so an auth-enabled server does
    // not reject the worker before the callback handler can perform the same
    // fail-closed verification.
    if let Some(callback_id) = worker_callback_id(req.method(), req.uri().path()) {
        let supplied = req
            .headers()
            .get("X-Roko-Worker-Token")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("");
        if let Some(deployment_id) =
            super::deployments::authenticate_worker_callback_token(&state, callback_id, supplied)
                .await
        {
            let method = AuthMethod::WorkerToken;
            req.extensions_mut().insert(AuthContext {
                method,
                scope: "write".to_string(),
                user_id: Some(format!("worker:{deployment_id}")),
            });
            let mut response = next.run(req).await;
            response.headers_mut().insert(
                "X-Auth-Method",
                axum::http::HeaderValue::from_static(method.header_value()),
            );
            return Ok(response);
        }
    }

    let (auth_method, ctx, key_name_for_tracking, agent_context) =
        match api_credential(req.headers()) {
            ApiCredential::XApiKey(supplied) => {
                match authenticate_api_key(supplied, &auth, &named_api_keys, true) {
                    ApiKeyAuthResult::Ok(method, scope, user_id) => {
                        let name = user_id.clone();
                        (
                            method,
                            AuthContext {
                                method,
                                scope,
                                user_id,
                            },
                            name,
                            None,
                        )
                    }
                    ApiKeyAuthResult::KeyExpired(name) => {
                        append_auth_audit(
                            &state,
                            crate::auth_audit::AuthAuditEvent::new(
                                format!("api-key:{name}"),
                                crate::auth_audit::AuthAuditAction::KeyExpired,
                                route_label.clone(),
                                crate::auth_audit::AuthOutcome::Denied,
                            ),
                        );
                        return Ok(expired_key_response());
                    }
                    ApiKeyAuthResult::NoMatch => {
                        return Err(ApiError::unauthorized(
                            "invalid or missing X-Api-Key header",
                        ));
                    }
                }
            }
            ApiCredential::Bearer(supplied) => {
                // Detect agent tokens by their `roko_agent_` prefix and route them
                // to the dedicated agent token validator (T02).
                if supplied.starts_with("roko_agent_") {
                    if let Some(agent) = try_named_agent_token(supplied, &state).await {
                        req.extensions_mut()
                            .insert(AuthenticatedAgentId(agent.agent_id.clone()));
                        (
                            AuthMethod::Bearer,
                            AuthContext {
                                method: AuthMethod::Bearer,
                                scope: "agent:capability".to_string(),
                                user_id: Some(agent.agent_id.clone()),
                            },
                            None,
                            Some(agent),
                        )
                    } else {
                        return Err(ApiError::unauthorized(
                            "invalid or expired agent bearer token",
                        ));
                    }
                } else if supplied.starts_with("roko_relay_") {
                    match state
                        .auth_registry
                        .authenticate_relay_secret(supplied)
                        .await
                    {
                        Ok(agent) => {
                            req.extensions_mut()
                                .insert(AuthenticatedAgentId(agent.agent_id.clone()));
                            (
                                AuthMethod::Bearer,
                                AuthContext {
                                    method: AuthMethod::Bearer,
                                    scope: "agent:capability".to_string(),
                                    user_id: Some(agent.agent_id.clone()),
                                },
                                None,
                                Some(agent),
                            )
                        }
                        Err(_) => {
                            return Err(ApiError::unauthorized(
                                "invalid or expired agent bearer token",
                            ));
                        }
                    }
                } else {
                    // Try API key (sync) → per-agent sidecar token (async) → Privy JWT (async).
                    match authenticate_api_key(supplied, &auth, &named_api_keys, false) {
                        ApiKeyAuthResult::Ok(method, scope, user_id) => {
                            let name = user_id.clone();
                            (
                                method,
                                AuthContext {
                                    method,
                                    scope,
                                    user_id,
                                },
                                name,
                                None,
                            )
                        }
                        ApiKeyAuthResult::KeyExpired(name) => {
                            append_auth_audit(
                                &state,
                                crate::auth_audit::AuthAuditEvent::new(
                                    format!("api-key:{name}"),
                                    crate::auth_audit::AuthAuditAction::KeyExpired,
                                    route_label.clone(),
                                    crate::auth_audit::AuthOutcome::Denied,
                                ),
                            );
                            return Ok(expired_key_response());
                        }
                        ApiKeyAuthResult::NoMatch => {
                            if let Some((method, scope, user_id)) =
                                try_agent_token(supplied, &state).await
                            {
                                if let Some(agent_id) = user_id.clone() {
                                    req.extensions_mut().insert(AuthenticatedAgentId(agent_id));
                                }
                                (
                                    method,
                                    AuthContext {
                                        method,
                                        scope,
                                        user_id,
                                    },
                                    None,
                                    None,
                                )
                            } else if let Some((method, scope, user_id)) =
                                try_privy_jwt(supplied, &auth, &state).await
                            {
                                (
                                    method,
                                    AuthContext {
                                        method,
                                        scope,
                                        user_id,
                                    },
                                    None,
                                    None,
                                )
                            } else {
                                return Err(ApiError::unauthorized(
                                    "invalid or missing Authorization bearer token",
                                ));
                            }
                        }
                    }
                }
            }
            ApiCredential::InvalidXApiKey => {
                return Err(ApiError::unauthorized(
                    "invalid or missing X-Api-Key header",
                ));
            }
            ApiCredential::InvalidAuthorization => {
                return Err(ApiError::unauthorized(
                    "invalid or missing Authorization bearer token",
                ));
            }
            ApiCredential::Missing => {
                return Err(ApiError::unauthorized(
                    "missing X-Api-Key header or Authorization bearer token",
                ));
            }
        };

    if let Some(target_agent) = agent_observation_target(req.method(), req.uri().path()) {
        if let Some(authenticated_agent) = req.extensions().get::<AuthenticatedAgentId>() {
            if authenticated_agent.0 != target_agent {
                return Err(ApiError::forbidden(format!(
                    "agent credential for `{}` cannot publish observations for `{target_agent}`",
                    authenticated_agent.0
                )));
            }
        }
    }

    // Persist usage through the same locked registry used by rotation/revocation.
    if let Some(ref key_name) = key_name_for_tracking {
        if let Err(error) = state.auth_registry.record_api_key_use(key_name).await {
            tracing::warn!(key = %key_name, error = %error, "failed to persist API-key usage");
        }
        append_auth_audit(
            &state,
            crate::auth_audit::AuthAuditEvent::new(
                format!("api-key:{key_name}"),
                crate::auth_audit::AuthAuditAction::TokenUsed,
                route_label.clone(),
                crate::auth_audit::AuthOutcome::Success,
            ),
        );
    }

    // Agent tokens are fail-closed outside their concrete capability routes.
    if let Some(agent) = agent_context {
        let required = required_agent_capability(req.method(), req.uri().path());
        let delegation = is_relay_delegation_endpoint(req.method(), req.uri().path());
        let allowed =
            delegation || required.is_some_and(|required| agent.capabilities.contains(&required));
        if !allowed {
            let required_label = required
                .map(|capability| format!("{capability:?}"))
                .unwrap_or_else(|| "unclassified".to_string());
            append_auth_audit(
                &state,
                crate::auth_audit::AuthAuditEvent::new(
                    format!("agent:{}", agent.agent_id),
                    crate::auth_audit::AuthAuditAction::PermissionDenied,
                    route_label.clone(),
                    crate::auth_audit::AuthOutcome::Denied,
                )
                .with_meta("required_capability", required_label.clone())
                .with_meta("token_id", agent.token_id.clone()),
            );
            return Err(ApiError {
                status: axum::http::StatusCode::FORBIDDEN,
                code: "insufficient_capability".to_string(),
                message: format!(
                    "agent token lacks capability '{required_label}' for {route_label}"
                ),
                details: Some(Box::new(serde_json::json!({
                    "required_capability": required_label,
                    "route": route_label,
                }))),
            });
        }

        let capability_label = required
            .map(|capability| format!("{capability:?}"))
            .unwrap_or_else(|| "Delegate".to_string());
        append_auth_audit(
            &state,
            crate::auth_audit::AuthAuditEvent::new(
                format!("agent:{}", agent.agent_id),
                crate::auth_audit::AuthAuditAction::TokenUsed,
                route_label.clone(),
                crate::auth_audit::AuthOutcome::Success,
            )
            .with_meta("capability", capability_label)
            .with_meta("token_id", agent.token_id.clone()),
        );
        req.extensions_mut().insert(agent);
    }

    // Inject identity headers so downstream handlers (team.rs, etc.)
    // can read the caller's identity without parsing extensions.
    if let Some(ref uid) = ctx.user_id {
        if let Ok(val) = axum::http::HeaderValue::from_str(uid) {
            req.headers_mut().insert("x-user-id", val);
        }
    }

    // Inject AuthContext for downstream handlers.
    req.extensions_mut().insert(ctx);

    let mut response = next.run(req).await;
    response.headers_mut().insert(
        "X-Auth-Method",
        axum::http::HeaderValue::from_static(auth_method.header_value()),
    );
    Ok(response)
}

/// A single entry in the route-to-scope manifest.
///
/// The manifest documents which scope every mutating route prefix requires.
/// Adding a new mutating route to the router without updating
/// [`ROUTE_SCOPE_MANIFEST`] causes `route_scope_manifest_matches_router` to
/// fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RouteScopeEntry {
    /// Path prefix matched via `starts_with`. Order matters: first match wins.
    pub prefix: &'static str,
    /// Required scope for mutating requests (POST/PUT/DELETE/PATCH).
    pub scope: &'static str,
}

/// Canonical route-to-scope manifest derived from the classifier below.
///
/// **Ordering contract**: more-specific prefixes before less-specific ones.
/// Every mutating route registered in [`super::build_router`] must be covered
/// by an entry here. The `route_scope_manifest_matches_router` test verifies
/// that each entry agrees with [`required_scope_for`].
pub(crate) const ROUTE_SCOPE_MANIFEST: &[RouteScopeEntry] = &[
    // --- admin ---------------------------------------------------------------
    RouteScopeEntry {
        prefix: "/api/api-keys",
        scope: "admin",
    },
    RouteScopeEntry {
        prefix: "/api/agent-tokens",
        scope: "admin",
    },
    RouteScopeEntry {
        prefix: "/api/secrets",
        scope: "admin",
    },
    RouteScopeEntry {
        prefix: "/api/config",
        scope: "admin",
    },
    // --- agent:write ---------------------------------------------------------
    RouteScopeEntry {
        prefix: "/api/events/ingest",
        scope: "agent:write",
    },
    RouteScopeEntry {
        prefix: "/api/agents",
        scope: "agent:write",
    },
    RouteScopeEntry {
        prefix: "/api/meta",
        scope: "agent:write",
    },
    RouteScopeEntry {
        prefix: "/relay",
        scope: "agent:write",
    },
    // --- plan:write ----------------------------------------------------------
    RouteScopeEntry {
        prefix: "/api/plans",
        scope: "plan:write",
    },
    RouteScopeEntry {
        prefix: "/api/prd",
        scope: "plan:write",
    },
    // --- terminal:write ------------------------------------------------------
    RouteScopeEntry {
        prefix: "/api/terminal",
        scope: "terminal:write",
    },
    RouteScopeEntry {
        prefix: "/ws/terminal",
        scope: "terminal:write",
    },
    // --- write (explicit, not fallback) --------------------------------------
    RouteScopeEntry {
        prefix: "/api/workspaces",
        scope: "write",
    },
    RouteScopeEntry {
        prefix: "/api/jobs",
        scope: "write",
    },
    RouteScopeEntry {
        prefix: "/api/marketplace",
        scope: "write",
    },
    RouteScopeEntry {
        prefix: "/api/registries",
        scope: "admin",
    },
    RouteScopeEntry {
        prefix: "/api/run",
        scope: "write",
    },
    RouteScopeEntry {
        prefix: "/api/dream",
        scope: "write",
    },
    RouteScopeEntry {
        prefix: "/api/deployments",
        scope: "write",
    },
    RouteScopeEntry {
        prefix: "/api/research",
        scope: "write",
    },
    RouteScopeEntry {
        prefix: "/api/subscriptions",
        scope: "write",
    },
    RouteScopeEntry {
        prefix: "/api/templates",
        scope: "write",
    },
    RouteScopeEntry {
        prefix: "/api/heartbeats",
        scope: "write",
    },
    RouteScopeEntry {
        prefix: "/api/neuro",
        scope: "write",
    },
    RouteScopeEntry {
        prefix: "/api/signals",
        scope: "write",
    },
    RouteScopeEntry {
        prefix: "/api/inference",
        scope: "write",
    },
    RouteScopeEntry {
        prefix: "/api/gateway",
        scope: "write",
    },
    RouteScopeEntry {
        prefix: "/api/bench",
        scope: "write",
    },
    RouteScopeEntry {
        prefix: "/api/connectors",
        scope: "write",
    },
    RouteScopeEntry {
        prefix: "/api/feeds",
        scope: "write",
    },
    RouteScopeEntry {
        prefix: "/api/groups",
        scope: "write",
    },
    RouteScopeEntry {
        prefix: "/api/arenas",
        scope: "write",
    },
    RouteScopeEntry {
        prefix: "/api/defi",
        scope: "write",
    },
    RouteScopeEntry {
        prefix: "/api/invitations",
        scope: "write",
    },
    RouteScopeEntry {
        prefix: "/api/recipes",
        scope: "write",
    },
    RouteScopeEntry {
        prefix: "/api/rpc",
        scope: "write",
    },
    RouteScopeEntry {
        prefix: "/api/vision-loop",
        scope: "write",
    },
    // Authenticated JWT invitees may accept an invitation without already
    // holding TeamManage. The handler itself rejects non-JWT identities.
    RouteScopeEntry {
        prefix: "/api/team/join",
        scope: "read",
    },
    RouteScopeEntry {
        prefix: "/api/team",
        scope: "write",
    },
    RouteScopeEntry {
        prefix: "/api/webhooks",
        scope: "write",
    },
    RouteScopeEntry {
        prefix: "/api/providers",
        scope: "write",
    },
];

/// Sentinel scope returned by [`required_scope_for`] when a mutating route is
/// not explicitly classified in [`ROUTE_SCOPE_MANIFEST`]. At runtime this is
/// treated identically to `"write"` by [`is_scope_sufficient`], but the CI
/// guard test [`mutating_routes_are_classified`] uses it to detect missing
/// classifications and fail the build.
///
/// When adding a new mutating route, add an entry in
/// [`ROUTE_SCOPE_MANIFEST`] **and** a representative path in the
/// `route_scope_manifest_matches_router` test so the guard keeps passing.
pub(crate) const SCOPE_WRITE_UNCLASSIFIED: &str = "write:unclassified";

/// Determine the required scope for a given HTTP method and path.
///
/// Read-only methods (`GET`, `HEAD`, `OPTIONS`) always return `"read"`.
/// Mutating methods are classified in priority order:
///
/// 1. [`ROUTE_SCOPE_MANIFEST`] — static first-party route prefixes.
/// 2. Extension route registry populated by [`register_extension_route_scopes`]
///    — plugin/extension webhook routes that declared an explicit scope.
/// 3. [`SCOPE_WRITE_UNCLASSIFIED`] fallback — fail-closed at runtime, caught
///    at test time by the CI guard.
///
/// The static manifest takes precedence over extension routes so that a
/// misconfigured plugin cannot downgrade scope requirements for core routes.
pub(crate) fn required_scope_for(method: &Method, path: &str) -> &'static str {
    // Read-only methods always pass.
    if method == Method::GET || method == Method::HEAD || method == Method::OPTIONS {
        return "read";
    }
    // Middleware on the nested API router can observe `/registries/...`
    // rather than the externally visible `/api/registries/...`. Match both
    // forms so an admin route never falls back to the weaker generic write
    // scope merely because Axum stripped the nest prefix.
    let nested_path = (!path.starts_with("/api/")).then(|| format!("/api{path}"));
    // 1. Static first-party manifest (O(n), n ≤ ~30 entries).
    for entry in ROUTE_SCOPE_MANIFEST {
        if path.starts_with(entry.prefix)
            || nested_path
                .as_deref()
                .is_some_and(|path| path.starts_with(entry.prefix))
        {
            return entry.scope;
        }
    }
    // 2. Extension routes registered at startup with an explicit scope.
    if let Some(ext_scope) =
        extension_scope_for(path).or_else(|| nested_path.as_deref().and_then(extension_scope_for))
    {
        return known_static_scope(ext_scope);
    }
    // 3. Fail-closed: any unclassified mutating route gets the sentinel scope
    //    which behaves as "write" at runtime but is detectable by the CI guard.
    SCOPE_WRITE_UNCLASSIFIED
}

/// Check whether the caller's scope is sufficient for the required scope.
///
/// `"write:unclassified"` is treated identically to `"write"` so that the
/// fallback sentinel does not change runtime behaviour — it is only detectable
/// by the regression test.
fn is_scope_sufficient(has: &str, required: &str) -> bool {
    if matches!(has, "owner" | "admin") {
        return true;
    }
    if required == "read" {
        return true;
    }
    // The unclassified fallback behaves as "write" at runtime so read-only
    // keys are still blocked. The sentinel only matters for the CI guard.
    let normalised = if required == SCOPE_WRITE_UNCLASSIFIED {
        "write"
    } else {
        required
    };
    // `write` scope covers sub-scoped write requirements (e.g. terminal:write).
    if has == "write" && normalised.ends_with(":write") {
        return true;
    }
    has == normalised
}

/// Emit an [`AuthAuditEvent`] for a permission decision (best-effort).
///
/// Opens the audit log on each call. Errors are swallowed — audit logging
/// must never block or fail a request.
fn emit_enforcement_audit(
    state: &AppState,
    actor: &str,
    action: crate::auth_audit::AuthAuditAction,
    route: &str,
    outcome: crate::auth_audit::AuthOutcome,
    scope_has: &str,
    scope_required: &str,
) {
    let event = crate::auth_audit::AuthAuditEvent::new(actor, action, route, outcome)
        .with_meta("scope_has", scope_has)
        .with_meta("scope_required", scope_required);
    append_auth_audit(state, event);
}

/// Enforce scope requirements on mutating routes.
///
/// Runs after [`require_api_key`] and reads the [`AuthContext`] from
/// request extensions. GET/HEAD/OPTIONS always pass through.
///
/// Behaviour is controlled by [`EnforcementMode`] from `serve.auth.enforcement_mode`:
/// - `enforce` (default): blocks insufficient scope with 403 and emits a deny audit event.
/// - `audit`: logs the violation and emits a deny audit event, but allows the request through.
/// - `disabled`: skips scope checks entirely (no logging, no audit events).
///
/// Permission grants on mutating routes are logged at `debug` level and recorded
/// as `PermissionGranted` audit events.
pub async fn require_scope(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    // Named agent tokens were authorized against a concrete capability by
    // `require_api_key`; do not reinterpret them through coarse user scopes.
    if req.extensions().get::<AgentCredentialClaims>().is_some() {
        return Ok(next.run(req).await);
    }

    // Read-only methods bypass scope checks.
    if method == Method::GET || method == Method::HEAD || method == Method::OPTIONS {
        return Ok(next.run(req).await);
    }

    // Check enforcement mode.
    let enforcement_mode = state.load_roko_config().serve.auth.enforcement_mode;
    if enforcement_mode == roko_core::config::EnforcementMode::Disabled {
        return Ok(next.run(req).await);
    }

    let required = required_scope_for(&method, &path);
    let (has_scope, actor) = req
        .extensions()
        .get::<AuthContext>()
        .map(|ctx| {
            (
                ctx.scope.clone(),
                ctx.user_id.clone().unwrap_or_else(|| "unknown".to_string()),
            )
        })
        .unwrap_or_else(|| ("read".to_string(), "unknown".to_string()));

    let route_label = format!("{method} {path}");

    if !is_scope_sufficient(&has_scope, required) {
        // --- Permission denied ---
        tracing::warn!(
            actor = %actor,
            scope_has = %has_scope,
            scope_required = %required,
            route = %route_label,
            enforcement = ?enforcement_mode,
            "auth_audit: permission denied",
        );
        emit_enforcement_audit(
            &state,
            &actor,
            crate::auth_audit::AuthAuditAction::PermissionDenied,
            &route_label,
            crate::auth_audit::AuthOutcome::Denied,
            &has_scope,
            required,
        );

        // In audit mode, log but do NOT block.
        if enforcement_mode == roko_core::config::EnforcementMode::Audit {
            return Ok(next.run(req).await);
        }

        return Err(ApiError {
            status: axum::http::StatusCode::FORBIDDEN,
            code: "insufficient_scope".into(),
            message: format!(
                "scope '{has_scope}' is not sufficient for '{required}' on {method} {path}"
            ),
            details: Some(Box::new(serde_json::json!({
                "required": required,
                "has": has_scope,
                "route": route_label,
            }))),
        });
    }

    // --- Permission granted ---
    tracing::debug!(
        actor = %actor,
        scope_has = %has_scope,
        scope_required = %required,
        route = %route_label,
        "auth_audit: permission granted",
    );
    emit_enforcement_audit(
        &state,
        &actor,
        crate::auth_audit::AuthAuditAction::PermissionGranted,
        &route_label,
        crate::auth_audit::AuthOutcome::Success,
        &has_scope,
        required,
    );

    Ok(next.run(req).await)
}

// ── RBAC middleware ──────────────────────────────────────────────────────────

/// Map an auth scope string (from [`AuthContext::scope`]) to an RBAC [`Role`].
///
/// The scope vocabulary comes from API key entries and Privy JWT auth:
///
/// | Scope           | Role    |
/// |-----------------|---------|
/// | `"admin"`       | Admin   |
/// | `"owner"`       | Owner   |
/// | `"write"`       | Member  |
/// | `"agent:write"` | Member  |
/// | `"plan:write"`  | Member  |
/// | `"read"`        | Viewer  |
/// | anything else   | Viewer  |
///
/// This mapping is intentionally conservative: unrecognised scopes default
/// to the least-privileged role.
pub(crate) fn scope_to_role(scope: &str) -> Role {
    match scope {
        "owner" => Role::Owner,
        "admin" => Role::Admin,
        "write" | "agent:write" | "plan:write" | "terminal:write" => Role::Member,
        "read" => Role::Viewer,
        _ => Role::Viewer,
    }
}

/// Build an axum middleware layer that enforces a specific RBAC [`Permission`].
///
/// The returned middleware:
/// 1. Extracts the caller's [`AuthContext`] from request extensions (injected
///    by [`require_api_key`]).
/// 2. Maps the `scope` string to an RBAC [`Role`] via [`scope_to_role`].
/// 3. Calls [`enforce_permission`] and returns 403 Forbidden on denial.
///
/// # Usage
///
/// ```ignore
/// use roko_serve::rbac::Permission;
/// use roko_serve::routes::middleware::require_permission;
///
/// let plans = plans::routes()
///     .layer(axum::middleware::from_fn(require_permission(Permission::PlanExecute)));
/// ```
pub fn require_permission(
    required: Permission,
) -> impl Fn(
    Request<Body>,
    Next,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, ApiError>> + Send>>
+ Clone
+ Send
+ 'static {
    move |req: Request<Body>, next: Next| {
        let required = required;
        Box::pin(async move {
            // Route groups contain both read and mutation handlers. RBAC
            // permissions gate mutations; read-only methods remain governed
            // by the scope middleware's `read` classification.
            if matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS) {
                return Ok(next.run(req).await);
            }

            let (role, actor) = match req.extensions().get::<AuthContext>() {
                Some(ctx) => (
                    scope_to_role(&ctx.scope),
                    ctx.user_id.clone().unwrap_or_else(|| "unknown".to_string()),
                ),
                None => {
                    // No AuthContext means auth middleware didn't run or auth
                    // is disabled. Default to Viewer (least-privileged) which
                    // will be rejected for any write-level permission.
                    (Role::Viewer, "unauthenticated".to_string())
                }
            };

            if let Err(api_err) = enforce_permission(role, required) {
                tracing::warn!(
                    actor = %actor,
                    role = %role.as_str(),
                    permission = %required.as_str(),
                    "rbac: permission denied",
                );
                return Err(api_err);
            }

            Ok(next.run(req).await)
        })
    }
}

/// Methods the server actually serves on browser-callable routes.
///
/// T3-28: previously the CORS layer answered preflight checks with
/// `Access-Control-Allow-Methods: *`, which is permissive enough to accept
/// arbitrary verbs (TRACE, CONNECT, …) the server has no handler for.
fn allowed_cors_methods() -> [Method; 6] {
    [
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::PATCH,
        Method::OPTIONS,
    ]
}

/// Headers the server actually consumes on browser-callable routes.
///
/// T3-28: replaces the previous `Any` allow-list. Webhook-only headers
/// (`X-Hub-Signature-256`, `X-Slack-Signature`, …) are intentionally
/// omitted because those endpoints are server-to-server, not browser.
fn allowed_cors_headers() -> [HeaderName; 5] {
    [
        CONTENT_TYPE,
        AUTHORIZATION,
        HeaderName::from_static("x-api-key"),
        HeaderName::from_static("x-user-id"),
        HeaderName::from_static("x-user-email"),
    ]
}

/// Build the CORS layer from configured origins.
pub fn cors_layer(cors_origins: &[String], unsafe_public: bool) -> CorsLayer {
    if !cors_origins.is_empty() {
        let allowed: Vec<axum::http::HeaderValue> =
            cors_origins.iter().filter_map(|o| o.parse().ok()).collect();
        return CorsLayer::new()
            .allow_origin(allowed)
            .allow_methods(allowed_cors_methods())
            .allow_headers(allowed_cors_headers());
    }

    if unsafe_public {
        if UNSAFE_PUBLIC_CORS_WARNING.set(()).is_ok() {
            tracing::warn!(
                "CORS is unrestricted (allow *) because server.unsafe_public_cors = true and no \
                 cors_origins are configured. Set cors_origins to limit access."
            );
        }
        return CorsLayer::permissive();
    }

    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(
            |origin: &axum::http::HeaderValue, _parts: &axum::http::request::Parts| match origin
                .to_str()
            {
                Ok(origin) => is_local_origin(origin),
                Err(_) => false,
            },
        ))
        .allow_methods(allowed_cors_methods())
        .allow_headers(allowed_cors_headers())
}

/// Returns `true` when `origin` is a localhost or loopback origin on any port.
fn is_local_origin(origin: &str) -> bool {
    let Ok(uri) = origin.parse::<axum::http::Uri>() else {
        return false;
    };
    let Some(scheme) = uri.scheme_str() else {
        return false;
    };
    if !matches!(scheme, "http" | "https") {
        return false;
    }
    let Some(authority) = uri.authority() else {
        return false;
    };
    let host = authority.host();
    host.eq_ignore_ascii_case("localhost")
        || host == "127.0.0.1"
        || host == "::1"
        || host == "[::1]"
}

/// Returns `true` when the response content-type indicates a text-like body
/// that should be scrubbed for secrets.
fn is_scrubbable_content_type(response: &Response) -> bool {
    let Some(ct) = response.headers().get(axum::http::header::CONTENT_TYPE) else {
        // No content-type — assume JSON (axum default for Json responses).
        return true;
    };
    let Ok(ct_str) = ct.to_str() else {
        return false;
    };
    let ct_lower = ct_str.to_ascii_lowercase();
    // SSE responses are infinite streams — buffering them would block forever.
    if ct_lower.contains("text/event-stream") {
        return false;
    }
    ct_lower.contains("json")
        || ct_lower.contains("text/")
        || ct_lower.contains("javascript")
        || ct_lower.contains("xml")
}

/// Axum middleware that scrubs secret patterns from text/JSON response bodies.
///
/// Binary or image responses are passed through unchanged.
/// Uses the shared [`LogScrubber`] stored in `AppState.scrubber`.
pub async fn scrub_secrets(
    State(scrubber): State<Arc<LogScrubber>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let response = next.run(req).await;

    if !is_scrubbable_content_type(&response) {
        return response;
    }

    let (parts, body) = response.into_parts();

    // Collect the full body. On error (e.g. connection drop) return an
    // empty 500 rather than leaking unscrubbed partial data.
    // Cap at 16 MiB to avoid unbounded memory growth.
    let Ok(bytes) = axum::body::to_bytes(body, 16 * 1024 * 1024).await else {
        return ApiError::internal("response body collection failed").into_response();
    };

    // Fast path: if the body is empty or not valid UTF-8, pass through.
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return Response::from_parts(parts, Body::from(bytes));
    };

    if text.is_empty() {
        return Response::from_parts(parts, Body::from(bytes));
    }

    let redacted = scrubber.scrub(text);

    // Avoid an allocation when nothing was redacted.
    if redacted.len() == text.len() && redacted == text {
        return Response::from_parts(parts, Body::from(bytes));
    }

    Response::from_parts(parts, Body::from(redacted))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Extension;
    use axum::Router;
    use axum::http::StatusCode;
    use axum::http::header::AUTHORIZATION;
    use axum::http::header::CONTENT_TYPE;
    use axum::routing::{get, post};
    use roko_core::config::{RokoConfig, ServeAuthConfig};
    use serde_json::Value;
    use tempfile::tempdir;
    use tower::ServiceExt;

    use crate::deploy::manual::ManualBackend;
    use crate::runtime::NoOpRuntime;
    use crate::state::AgentRegistrationRecord;

    /// Build a test router that echoes the provided body, with the scrub
    /// middleware wired in.
    fn test_app(scrubber: Arc<LogScrubber>, body: &'static str) -> Router {
        let handler = move || async move { body.to_string() };
        Router::new()
            .route("/test", get(handler))
            .layer(axum::middleware::from_fn_with_state(
                scrubber,
                scrub_secrets,
            ))
    }

    fn test_app_json(scrubber: Arc<LogScrubber>, body: &'static str) -> Router {
        let handler =
            move || async move { axum::Json(serde_json::Value::String(body.to_string())) };
        Router::new()
            .route("/test", get(handler))
            .layer(axum::middleware::from_fn_with_state(
                scrubber,
                scrub_secrets,
            ))
    }

    fn legacy_auth(api_key: &str) -> ServeAuthConfig {
        ServeAuthConfig {
            enabled: true,
            api_key: api_key.into(),
            api_keys: Vec::new(),
            privy_app_id: None,
            ..Default::default()
        }
    }

    #[test]
    fn worker_callback_path_matches_before_and_after_axum_nesting() {
        assert_eq!(
            worker_callback_id(&Method::POST, "/api/deployments/callback-123/callback"),
            Some("callback-123")
        );
        assert_eq!(
            worker_callback_id(&Method::POST, "/deployments/callback-123/callback"),
            Some("callback-123")
        );
        assert_eq!(
            worker_callback_id(&Method::GET, "/api/deployments/callback-123/callback"),
            None
        );
        assert_eq!(
            worker_callback_id(&Method::POST, "/api/deployments/callback-123/task"),
            None
        );
    }

    fn make_test_state(auth: ServeAuthConfig) -> Arc<AppState> {
        let tempdir = tempdir().expect("invariant: tempdir creates");
        make_test_state_at(tempdir.path(), auth)
    }

    fn make_test_state_at(path: &std::path::Path, auth: ServeAuthConfig) -> Arc<AppState> {
        let mut config = RokoConfig::default();
        config.serve.auth = auth;
        Arc::new(
            AppState::new(
                path.to_path_buf(),
                Arc::new(NoOpRuntime),
                config,
                Arc::new(ManualBackend::default()),
            )
            .expect("AppState::new"),
        )
    }

    fn auth_test_app(auth: ServeAuthConfig) -> Router {
        let state = make_test_state(auth);
        Router::new()
            .route("/test", get(|| async { StatusCode::NO_CONTENT }))
            .layer(axum::middleware::from_fn_with_state(state, require_api_key))
    }

    async fn auth_response(
        app: Router,
        build: impl FnOnce(axum::http::request::Builder) -> axum::http::request::Builder,
    ) -> Response {
        let req = build(Request::builder().uri("/test"))
            .body(Body::empty())
            .expect("invariant: auth test request builds");
        app.oneshot(req)
            .await
            .expect("invariant: auth test router responds")
    }

    async fn auth_error_body(response: Response) -> Value {
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .expect("invariant: auth test response body buffers");
        serde_json::from_slice(&body).expect("invariant: auth error payload is valid json")
    }

    #[tokio::test]
    async fn require_api_key_accepts_matching_x_api_key_header() {
        let app = auth_test_app(legacy_auth("secret-key-123"));

        let response = auth_response(app, |req| req.header("X-Api-Key", "secret-key-123")).await;

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn require_api_key_accepts_matching_bearer_token() {
        let app = auth_test_app(legacy_auth("secret-key-123"));

        let response = auth_response(app, |req| {
            req.header(AUTHORIZATION, "Bearer secret-key-123")
        })
        .await;

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn require_api_key_rejects_missing_credentials() {
        let app = auth_test_app(legacy_auth("secret-key-123"));

        let response = auth_response(app, |req| req).await;
        let status = response.status();
        let body = auth_error_body(response).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "unauthorized");
        assert_eq!(
            body["message"],
            "missing X-Api-Key header or Authorization bearer token"
        );
    }

    #[tokio::test]
    async fn require_api_key_rejects_invalid_bearer_token() {
        let app = auth_test_app(legacy_auth("secret-key-123"));

        let response =
            auth_response(app, |req| req.header(AUTHORIZATION, "Bearer wrong-key")).await;
        let status = response.status();
        let body = auth_error_body(response).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            body["message"],
            "invalid or missing Authorization bearer token"
        );
    }

    #[tokio::test]
    async fn require_api_key_prefers_x_api_key_when_both_headers_are_present() {
        let app = auth_test_app(legacy_auth("secret-key-123"));

        let response = auth_response(app, |req| {
            req.header("X-Api-Key", "wrong-key")
                .header(AUTHORIZATION, "Bearer secret-key-123")
        })
        .await;
        let status = response.status();
        let body = auth_error_body(response).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["message"], "invalid or missing X-Api-Key header");
    }

    #[tokio::test]
    async fn named_api_key_use_is_persisted_and_audited() {
        let dir = tempdir().expect("tempdir");
        let plaintext = "tracked-api-key";
        let auth = keyed_auth("tracked", plaintext, "admin");
        let state = make_test_state_at(dir.path(), auth);
        let app = Router::new()
            .route("/api/status", get(|| async { StatusCode::NO_CONTENT }))
            .layer(axum::middleware::from_fn_with_state(
                Arc::clone(&state),
                require_api_key,
            ));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/status")
                    .header("X-Api-Key", plaintext)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let keys = state.auth_registry.api_keys_snapshot().await;
        assert!(keys[0].last_used_at.is_some());
        let events = state
            .auth_audit
            .as_ref()
            .expect("audit log")
            .query(None, None, None, None)
            .expect("query audit");
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].action,
            crate::auth_audit::AuthAuditAction::TokenUsed
        );
        assert_eq!(events[0].actor, "api-key:tracked");
    }

    #[tokio::test]
    async fn expired_named_key_returns_header_and_audits_rejection() {
        let dir = tempdir().expect("tempdir");
        let plaintext = "expired-api-key";
        let mut auth = keyed_auth("expired", plaintext, "admin");
        auth.api_keys[0].expires_at =
            Some((Utc::now() - chrono::Duration::minutes(1)).to_rfc3339());
        let state = make_test_state_at(dir.path(), auth);
        let app = Router::new()
            .route("/api/status", get(|| async { StatusCode::NO_CONTENT }))
            .layer(axum::middleware::from_fn_with_state(
                Arc::clone(&state),
                require_api_key,
            ));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/status")
                    .header("X-Api-Key", plaintext)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(response.headers()["x-key-expired"], "true");
        let events = state
            .auth_audit
            .as_ref()
            .expect("audit log")
            .query(None, None, None, None)
            .expect("query audit");
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].action,
            crate::auth_audit::AuthAuditAction::KeyExpired
        );
    }

    #[tokio::test]
    async fn agent_capabilities_are_order_independent_and_fail_closed() {
        let dir = tempdir().expect("tempdir");
        let state = make_test_state_at(dir.path(), ServeAuthConfig::default());
        let plaintext = "roko_agent_capability-test";
        state
            .auth_registry
            .insert_agent_token(crate::routes::auth::AgentToken {
                token_id: "token-1".to_string(),
                agent_id: "agent-1".to_string(),
                // Inference is deliberately second: authorization must not
                // depend on capability ordering.
                capabilities: vec![AgentCapability::StoreRead, AgentCapability::Inference],
                issued_at: Utc::now().to_rfc3339(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
                revoked: false,
                token_hash: hash_api_key(plaintext),
            })
            .await
            .expect("insert agent token");
        let app = Router::new()
            .route(
                "/api/inference/complete",
                post(|| async { StatusCode::NO_CONTENT }),
            )
            .route(
                "/api/events/ingest",
                post(|| async { StatusCode::NO_CONTENT }),
            )
            .route(
                "/api/agent-tokens",
                post(|| async { StatusCode::NO_CONTENT }),
            )
            .layer(axum::middleware::from_fn_with_state(
                Arc::clone(&state),
                require_scope,
            ))
            .layer(axum::middleware::from_fn_with_state(
                Arc::clone(&state),
                require_api_key,
            ));
        let request = |path: &'static str| {
            Request::builder()
                .method(Method::POST)
                .uri(path)
                .header(AUTHORIZATION, format!("Bearer {plaintext}"))
                .body(Body::empty())
                .expect("request")
        };

        let allowed = app
            .clone()
            .oneshot(request("/api/inference/complete"))
            .await
            .unwrap();
        assert_eq!(allowed.status(), StatusCode::NO_CONTENT);
        let missing = app
            .clone()
            .oneshot(request("/api/events/ingest"))
            .await
            .unwrap();
        assert_eq!(missing.status(), StatusCode::FORBIDDEN);
        let self_issue = app.oneshot(request("/api/agent-tokens")).await.unwrap();
        assert_eq!(self_issue.status(), StatusCode::FORBIDDEN);

        let events = state
            .auth_audit
            .as_ref()
            .expect("audit log")
            .query(None, None, None, None)
            .expect("query audit");
        assert_eq!(events.len(), 3);
        assert_eq!(
            events[0].action,
            crate::auth_audit::AuthAuditAction::TokenUsed
        );
        assert_eq!(
            events[1].action,
            crate::auth_audit::AuthAuditAction::PermissionDenied
        );
        assert_eq!(
            events[2].action,
            crate::auth_audit::AuthAuditAction::PermissionDenied
        );
    }

    #[tokio::test]
    async fn agent_observation_auth_requires_bus_publish_and_matching_agent_identity() {
        let dir = tempdir().expect("tempdir");
        let state = make_test_state_at(dir.path(), ServeAuthConfig::default());
        for agent_id in ["agent-1", "agent-2"] {
            state
                .upsert_discovered_agent(AgentRegistrationRecord {
                    agent_id: agent_id.to_string(),
                    ..Default::default()
                })
                .await;
        }

        let named_secret = "roko_agent_observation-test";
        state
            .auth_registry
            .insert_agent_token(crate::routes::auth::AgentToken {
                token_id: "observation-token".to_string(),
                agent_id: "agent-1".to_string(),
                capabilities: vec![AgentCapability::BusPublish],
                issued_at: Utc::now().to_rfc3339(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
                revoked: false,
                token_hash: hash_api_key(named_secret),
            })
            .await
            .expect("insert observation agent token");
        let legacy_secret = state
            .rotate_agent_token("agent-1")
            .await
            .expect("issue legacy agent token")
            .token;

        let app = Router::new()
            .route(
                "/api/agents/{id}/observation",
                post(|| async { StatusCode::NO_CONTENT }),
            )
            .layer(axum::middleware::from_fn_with_state(
                Arc::clone(&state),
                require_scope,
            ))
            .layer(axum::middleware::from_fn_with_state(
                Arc::clone(&state),
                require_api_key,
            ));
        let request = |agent: &'static str, secret: &str| {
            Request::post(format!("/api/agents/{agent}/observation"))
                .header(AUTHORIZATION, format!("Bearer {secret}"))
                .body(Body::empty())
                .expect("agent observation request")
        };

        assert_eq!(
            app.clone()
                .oneshot(request("agent-1", named_secret))
                .await
                .expect("modern token own-agent response")
                .status(),
            StatusCode::NO_CONTENT
        );
        assert_eq!(
            app.clone()
                .oneshot(request("agent-2", named_secret))
                .await
                .expect("modern token cross-agent response")
                .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            app.clone()
                .oneshot(request("agent-1", &legacy_secret))
                .await
                .expect("legacy token own-agent response")
                .status(),
            StatusCode::NO_CONTENT
        );
        assert_eq!(
            app.oneshot(request("agent-2", &legacy_secret))
                .await
                .expect("legacy token cross-agent response")
                .status(),
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn relay_bearer_uses_target_identity_and_narrowed_route_capabilities() {
        let dir = tempdir().expect("tempdir");
        let state = make_test_state_at(dir.path(), ServeAuthConfig::default());
        let root_secret = "roko_agent_relay-root";
        state
            .auth_registry
            .insert_agent_token(crate::routes::auth::AgentToken {
                token_id: "relay-root-token".to_string(),
                agent_id: "relay-root-agent".to_string(),
                capabilities: vec![AgentCapability::Inference, AgentCapability::BusPublish],
                issued_at: Utc::now().to_rfc3339(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
                revoked: false,
                token_hash: hash_api_key(root_secret),
            })
            .await
            .expect("insert root token");
        let root = state
            .auth_registry
            .authenticate_agent_secret(root_secret)
            .await
            .expect("authenticate root");
        let (_relay, relay_secret) = state
            .auth_registry
            .issue_relay_token(
                &root,
                &crate::routes::auth::IssueRelayTokenRequest {
                    target_agent_id: "delegated-agent".to_string(),
                    delegated_capabilities: vec![AgentCapability::Inference],
                    max_depth: Some(2),
                    ttl_secs: 300,
                },
            )
            .await
            .expect("issue relay");

        let app = Router::new()
            .route(
                "/api/inference/complete",
                post(|Extension(context): Extension<AuthContext>| async move {
                    assert_eq!(context.user_id.as_deref(), Some("delegated-agent"));
                    StatusCode::NO_CONTENT
                }),
            )
            .route(
                "/api/events/ingest",
                post(|| async { StatusCode::NO_CONTENT }),
            )
            .layer(axum::middleware::from_fn_with_state(
                Arc::clone(&state),
                require_scope,
            ))
            .layer(axum::middleware::from_fn_with_state(
                Arc::clone(&state),
                require_api_key,
            ));
        let request = |path: &'static str| {
            Request::post(path)
                .header(AUTHORIZATION, format!("Bearer {relay_secret}"))
                .body(Body::empty())
                .expect("relay request")
        };

        let allowed = app
            .clone()
            .oneshot(request("/api/inference/complete"))
            .await
            .expect("allowed response");
        assert_eq!(allowed.status(), StatusCode::NO_CONTENT);
        let denied = app
            .oneshot(request("/api/events/ingest"))
            .await
            .expect("denied response");
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn concrete_agent_capability_mapping_covers_each_capability() {
        assert_eq!(
            required_agent_capability(&Method::POST, "/api/inference/complete"),
            Some(AgentCapability::Inference)
        );
        assert_eq!(
            required_agent_capability(&Method::POST, "/api/rpc"),
            Some(AgentCapability::Tools)
        );
        assert_eq!(
            required_agent_capability(&Method::POST, "/api/events/ingest"),
            Some(AgentCapability::BusPublish)
        );
        assert_eq!(
            required_agent_capability(&Method::POST, "/api/agents/agent-1/observation"),
            Some(AgentCapability::BusPublish)
        );
        assert_eq!(
            required_agent_capability(&Method::POST, "/api/neuro/write"),
            Some(AgentCapability::StoreWrite)
        );
        assert_eq!(
            required_agent_capability(&Method::POST, "/api/neuro/query"),
            Some(AgentCapability::StoreRead)
        );
        assert_eq!(
            required_agent_capability(&Method::POST, "/api/agent-tokens"),
            None
        );
    }

    #[test]
    fn api_key_expiry_uses_parsed_instants_not_string_ordering() {
        let plaintext = "offset-expiry-key";
        let mut auth = keyed_auth("offset-expiry", plaintext, "admin");
        // This timestamp has a lexically later calendar day than UTC now in
        // some offsets, but represents an instant one minute in the past.
        let expired = Utc::now() - chrono::Duration::minutes(1);
        auth.api_keys[0].expires_at = Some(
            expired
                .with_timezone(&chrono::FixedOffset::east_opt(14 * 3600).unwrap())
                .to_rfc3339(),
        );
        assert!(matches!(
            match_api_key_entry(plaintext, &auth.api_keys),
            ApiKeyMatchResult::Expired(_)
        ));
    }

    #[tokio::test]
    async fn privy_jwt_without_cache_returns_401() {
        // Configure privy_app_id but no JWKS cache is primed — should reject.
        let auth = ServeAuthConfig {
            enabled: true,
            api_key: String::new(),
            api_keys: Vec::new(),
            privy_app_id: Some("app-id-123".to_string()),
            ..Default::default()
        };
        let app = auth_test_app(auth);
        // Send a structurally valid JWT that won't pass signature verification.
        let fake_jwt = "eyJhbGciOiJFUzI1NiIsInR5cCI6IkpXVCIsImtpZCI6InRlc3Qta2V5In0.\
                         eyJzdWIiOiJkaWQ6cHJpdnk6dGVzdCIsImlzcyI6InByaXZ5LmlvIn0.\
                         AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let response = auth_response(app, |req| {
            req.header(AUTHORIZATION, format!("Bearer {fake_jwt}"))
        })
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn privy_jwt_requires_membership() {
        // When privy_workspace_id is configured, try_privy_jwt must reject
        // tokens whose org_id claim is missing or does not match — even if
        // the signature would otherwise be valid.
        //
        // We cannot forge a real ES256 signature in a unit test, so we test
        // the membership gate indirectly: a structurally valid JWT that fails
        // JWKS validation returns None regardless of membership config (the
        // JWKS check short-circuits before we reach the membership gate).
        //
        // The key assertion is that configuring privy_workspace_id does not
        // accidentally let tokens *through* that would otherwise be blocked.
        let auth = ServeAuthConfig {
            enabled: true,
            api_key: String::new(),
            api_keys: Vec::new(),
            privy_app_id: Some("app-id-123".to_string()),
            privy_workspace_id: Some("ws_required_org".to_string()),
            privy_allowed_roles: vec!["admin".to_string()],
            ..Default::default()
        };
        let state = make_test_state(auth.clone());

        // A structurally valid JWT (3 base64url segments) that will fail JWKS
        // signature verification — should return None.
        let fake_jwt = "eyJhbGciOiJFUzI1NiIsInR5cCI6IkpXVCIsImtpZCI6InRlc3Qta2V5In0.\
                         eyJzdWIiOiJkaWQ6cHJpdnk6dGVzdCIsImlzcyI6InByaXZ5LmlvIn0.\
                         AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

        // try_privy_jwt must return None (JWKS cache unprimed, signature fails).
        let result = try_privy_jwt(fake_jwt, &auth, &state).await;
        assert!(
            result.is_none(),
            "Privy JWT without valid JWKS cache must be rejected"
        );

        // Verify the full middleware also rejects: the auth-test-app round-trip
        // should produce 401 even with membership config present.
        let app = auth_test_app(auth);
        let response = auth_response(app, |req| {
            req.header(AUTHORIZATION, format!("Bearer {fake_jwt}"))
        })
        .await;
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "Middleware must reject Privy JWT that fails signature verification"
        );
    }

    // --- scope enforcement tests ---

    fn scope_test_app(scope: &str) -> Router {
        let state = make_test_state(ServeAuthConfig::default());
        let handler = || async { StatusCode::NO_CONTENT };
        Router::new()
            .route("/api/secrets", post(handler))
            .route("/api/agents/test", post(handler))
            .route("/api/plans/run", post(handler))
            .route("/api/status", post(handler))
            .route("/api/status", get(handler))
            .layer(axum::middleware::from_fn_with_state(state, require_scope))
            .layer(axum::Extension(AuthContext {
                method: AuthMethod::ApiKey,
                scope: scope.to_string(),
                user_id: None,
            }))
    }

    #[tokio::test]
    async fn scope_enforcement_blocks_write_with_read_scope() {
        let app = scope_test_app("read");
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/secrets")
            .body(Body::empty())
            .expect("invariant: scope test request builds");
        let resp = app.oneshot(req).await.expect("invariant: router responds");
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn scope_enforcement_allows_get_with_read_scope() {
        let app = scope_test_app("read");
        let req = Request::builder()
            .method(Method::GET)
            .uri("/api/status")
            .body(Body::empty())
            .expect("invariant: scope test request builds");
        let resp = app.oneshot(req).await.expect("invariant: router responds");
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn admin_scope_allows_everything() {
        let app = scope_test_app("admin");
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/secrets")
            .body(Body::empty())
            .expect("invariant: scope test request builds");
        let resp = app.oneshot(req).await.expect("invariant: router responds");
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn agent_write_scope_allows_agent_routes() {
        let app = scope_test_app("agent:write");
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/agents/test")
            .body(Body::empty())
            .expect("invariant: scope test request builds");
        let resp = app.oneshot(req).await.expect("invariant: router responds");
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn agent_write_scope_blocks_secrets() {
        let app = scope_test_app("agent:write");
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/secrets")
            .body(Body::empty())
            .expect("invariant: scope test request builds");
        let resp = app.oneshot(req).await.expect("invariant: router responds");
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    // --- scrubbing tests ---

    #[tokio::test]
    async fn scrubs_api_key_from_json_response() {
        let scrubber = Arc::new(LogScrubber::new());
        let leaked = "your key is sk-ant-api03-abcdefghijklmnopqrstuvwxyz1234567890 ok";
        let app = test_app(scrubber, leaked);
        let req = Request::builder()
            .uri("/test")
            .body(Body::empty())
            .expect("invariant: building request body for test");
        let resp = app
            .oneshot(req)
            .await
            .expect("invariant: middleware test router responds");
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 4096)
            .await
            .expect("invariant: test response body buffers");
        let text =
            String::from_utf8(body.to_vec()).expect("invariant: middleware test body is utf-8");
        assert!(!text.contains("sk-ant-api03"));
        assert!(text.contains("[REDACTED"));
    }

    #[tokio::test]
    async fn clean_response_passes_through_unchanged() {
        let scrubber = Arc::new(LogScrubber::new());
        let clean = "all good, no secrets here";
        let app = test_app(scrubber, clean);
        let req = Request::builder()
            .uri("/test")
            .body(Body::empty())
            .expect("invariant: building request body for test");
        let resp = app
            .oneshot(req)
            .await
            .expect("invariant: middleware test router responds");
        let body = axum::body::to_bytes(resp.into_body(), 4096)
            .await
            .expect("invariant: test response body buffers");
        assert_eq!(
            std::str::from_utf8(&body).expect("invariant: clean response remains utf-8"),
            clean
        );
    }

    #[tokio::test]
    async fn binary_content_type_passes_through() {
        let scrubber = Arc::new(LogScrubber::new());
        let leaked = "sk-ant-api03-abcdefghijklmnopqrstuvwxyz1234567890";
        let handler = move || async move {
            Response::builder()
                .header(CONTENT_TYPE, "image/png")
                .body(Body::from(leaked))
                .expect("invariant: image response body builds")
        };
        let app =
            Router::new()
                .route("/test", get(handler))
                .layer(axum::middleware::from_fn_with_state(
                    scrubber,
                    scrub_secrets,
                ));
        let req = Request::builder()
            .uri("/test")
            .body(Body::empty())
            .expect("invariant: building request body for test");
        let resp = app
            .oneshot(req)
            .await
            .expect("invariant: middleware test router responds");
        let body = axum::body::to_bytes(resp.into_body(), 4096)
            .await
            .expect("invariant: test response body buffers");
        // Binary/image content should NOT be scrubbed.
        assert_eq!(
            std::str::from_utf8(&body).expect("invariant: binary test payload is utf-8"),
            leaked
        );
    }

    #[tokio::test]
    async fn scrubs_github_pat_from_json_response() {
        let scrubber = Arc::new(LogScrubber::new());
        let leaked = "token: ghp_ABCDEFGHIJKLMNOPqrstuvwxyz1234567890";
        let app = test_app_json(scrubber, leaked);
        let req = Request::builder()
            .uri("/test")
            .body(Body::empty())
            .expect("invariant: building request body for test");
        let resp = app
            .oneshot(req)
            .await
            .expect("invariant: middleware test router responds");
        let body = axum::body::to_bytes(resp.into_body(), 4096)
            .await
            .expect("invariant: test response body buffers");
        let text =
            String::from_utf8(body.to_vec()).expect("invariant: middleware test body is utf-8");
        assert!(!text.contains("ghp_"));
        assert!(text.contains("[REDACTED"));
    }

    #[test]
    fn is_scrubbable_detects_json() {
        let resp = Response::builder()
            .header(CONTENT_TYPE, "application/json")
            .body(Body::empty())
            .expect("invariant: response builder constructs json response");
        assert!(is_scrubbable_content_type(&resp));
    }

    #[test]
    fn is_scrubbable_detects_text() {
        let resp = Response::builder()
            .header(CONTENT_TYPE, "text/plain")
            .body(Body::empty())
            .expect("invariant: response builder constructs text response");
        assert!(is_scrubbable_content_type(&resp));
    }

    #[test]
    fn is_scrubbable_rejects_image() {
        let resp = Response::builder()
            .header(CONTENT_TYPE, "image/png")
            .body(Body::empty())
            .expect("invariant: response builder constructs image response");
        assert!(!is_scrubbable_content_type(&resp));
    }

    #[test]
    fn is_scrubbable_rejects_octet_stream() {
        let resp = Response::builder()
            .header(CONTENT_TYPE, "application/octet-stream")
            .body(Body::empty())
            .expect("invariant: response builder constructs octet-stream response");
        assert!(!is_scrubbable_content_type(&resp));
    }

    #[test]
    fn is_scrubbable_rejects_event_stream() {
        let resp = Response::builder()
            .header(CONTENT_TYPE, "text/event-stream")
            .body(Body::empty())
            .expect("invariant: response builder constructs sse response");
        assert!(!is_scrubbable_content_type(&resp));
    }

    #[test]
    fn is_scrubbable_assumes_json_when_no_content_type() {
        let resp = Response::builder()
            .body(Body::empty())
            .expect("invariant: response builder constructs empty response");
        assert!(is_scrubbable_content_type(&resp));
    }

    // --- extract_bearer_token tests ---

    #[test]
    fn extract_bearer_token_standard_case() {
        assert_eq!(extract_bearer_token("Bearer mytoken"), Some("mytoken"));
    }

    #[test]
    fn extract_bearer_token_lowercase() {
        assert_eq!(extract_bearer_token("bearer mytoken"), Some("mytoken"));
    }

    #[test]
    fn extract_bearer_token_uppercase() {
        assert_eq!(extract_bearer_token("BEARER mytoken"), Some("mytoken"));
    }

    #[test]
    fn extract_bearer_token_no_prefix() {
        assert_eq!(extract_bearer_token("mytoken"), None);
    }

    #[test]
    fn extract_bearer_token_empty_string() {
        assert_eq!(extract_bearer_token(""), None);
    }

    #[test]
    fn extract_bearer_token_empty_after_strip() {
        assert_eq!(extract_bearer_token("Bearer "), None);
    }

    // --- is_structurally_valid_jwt tests ---

    #[test]
    fn jwt_valid_three_segments() {
        assert!(is_structurally_valid_jwt("abc.def.ghi"));
    }

    #[test]
    fn jwt_rejects_two_segments() {
        assert!(!is_structurally_valid_jwt("abc.def"));
    }

    #[test]
    fn jwt_rejects_four_segments() {
        assert!(!is_structurally_valid_jwt("a.b.c.d"));
    }

    #[test]
    fn jwt_rejects_empty_segment() {
        assert!(!is_structurally_valid_jwt("a..c"));
    }

    #[test]
    fn jwt_accepts_base64url_chars() {
        assert!(is_structurally_valid_jwt(
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abc_DEF-123="
        ));
    }

    // --- X-Auth-Method response header tests ---

    #[tokio::test]
    async fn auth_method_header_set_to_api_key() {
        let app = auth_test_app(legacy_auth("secret-key-123"));
        let response = auth_response(app, |req| req.header("X-Api-Key", "secret-key-123")).await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert_eq!(
            response
                .headers()
                .get("X-Auth-Method")
                .unwrap()
                .to_str()
                .unwrap(),
            "api_key"
        );
    }

    #[tokio::test]
    async fn auth_method_header_set_to_bearer() {
        let app = auth_test_app(legacy_auth("secret-key-123"));
        let response = auth_response(app, |req| {
            req.header(AUTHORIZATION, "Bearer secret-key-123")
        })
        .await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert_eq!(
            response
                .headers()
                .get("X-Auth-Method")
                .unwrap()
                .to_str()
                .unwrap(),
            "bearer"
        );
    }

    #[tokio::test]
    async fn auth_method_header_set_to_jwt() {
        // Use a JWT-shaped token (3 dot-separated base64url segments) as the api_key
        let jwt_key = "eyJhbGci.eyJzdWIi.abc123";
        let app = auth_test_app(legacy_auth(jwt_key));
        let response = auth_response(app, |req| {
            req.header(AUTHORIZATION, format!("Bearer {jwt_key}"))
        })
        .await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert_eq!(
            response
                .headers()
                .get("X-Auth-Method")
                .unwrap()
                .to_str()
                .unwrap(),
            "jwt"
        );
    }

    // --- scope helper unit tests ---

    #[test]
    fn required_scope_for_get_is_read() {
        assert_eq!(required_scope_for(&Method::GET, "/api/secrets"), "read");
    }

    #[test]
    fn registry_reads_require_read_and_mutations_require_admin() {
        assert_eq!(
            required_scope_for(&Method::GET, "/api/registries/passports"),
            "read"
        );
        for (method, path) in [
            (Method::POST, "/api/registries/passports"),
            (Method::POST, "/api/registries/passports/1/transfer"),
            (Method::PUT, "/api/registries/passports/1/metadata"),
            (Method::POST, "/api/registries/passports/1/delegations"),
            (Method::DELETE, "/api/registries/passports/1/delegations/2"),
            (Method::POST, "/api/registries/knowledge"),
            (Method::POST, "/api/registries/knowledge/abc/validate"),
            (Method::POST, "/api/registries/knowledge/abc/challenge"),
            (
                Method::POST,
                "/api/registries/knowledge/challenges/abc/resolve",
            ),
            (Method::POST, "/api/registries/indexer/sync"),
            (Method::POST, "/api/registries/indexer/rebuild"),
        ] {
            assert_eq!(
                required_scope_for(&method, path),
                "admin",
                "{method} {path}"
            );
            assert_eq!(
                required_scope_for(&method, path.trim_start_matches("/api")),
                "admin",
                "nested {method} {path}"
            );
        }
    }

    #[test]
    fn required_scope_for_post_secrets_is_admin() {
        assert_eq!(required_scope_for(&Method::POST, "/api/secrets"), "admin");
    }

    #[test]
    fn required_scope_for_post_agents_is_agent_write() {
        assert_eq!(
            required_scope_for(&Method::POST, "/api/agents/test"),
            "agent:write"
        );
    }

    #[test]
    fn required_scope_for_post_plans_is_plan_write() {
        assert_eq!(
            required_scope_for(&Method::POST, "/api/plans/run"),
            "plan:write"
        );
    }

    #[test]
    fn required_scope_for_post_workspaces_is_write() {
        assert_eq!(
            required_scope_for(&Method::POST, "/api/workspaces"),
            "write"
        );
    }

    #[test]
    fn required_scope_for_delete_workspaces_is_write() {
        assert_eq!(
            required_scope_for(&Method::DELETE, "/api/workspaces/abc123"),
            "write"
        );
    }

    #[test]
    fn required_scope_for_classified_mutating_routes_return_write() {
        // Routes explicitly classified as "write" in ROUTE_SCOPE_MANIFEST.
        assert_eq!(required_scope_for(&Method::POST, "/api/jobs"), "write");
        assert_eq!(required_scope_for(&Method::POST, "/api/run"), "write");
        assert_eq!(
            required_scope_for(&Method::POST, "/api/research/query"),
            "write"
        );
        // Unclassified routes get the sentinel (not plain "write").
        assert_eq!(
            required_scope_for(&Method::DELETE, "/api/deploy/abc"),
            SCOPE_WRITE_UNCLASSIFIED
        );
    }

    #[tokio::test]
    async fn read_scope_denied_on_mutation() {
        // A read-scoped key must get 403 on POST to an unclassified mutating
        // route (one that hits the "write" fallback in required_scope_for).
        let state = make_test_state(ServeAuthConfig::default());
        let handler = || async { StatusCode::NO_CONTENT };
        let app = Router::new()
            .route("/api/jobs", post(handler))
            .layer(axum::middleware::from_fn_with_state(state, require_scope))
            .layer(axum::Extension(AuthContext {
                method: AuthMethod::ApiKey,
                scope: "read".to_string(),
                user_id: None,
            }));
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/jobs")
            .body(Body::empty())
            .expect("invariant: scope test request builds");
        let resp = app.oneshot(req).await.expect("invariant: router responds");
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "read-scoped key must be denied on unclassified mutating route"
        );
    }

    #[test]
    fn admin_scope_is_sufficient_for_everything() {
        assert!(is_scope_sufficient("admin", "admin"));
        assert!(is_scope_sufficient("admin", "agent:write"));
        assert!(is_scope_sufficient("admin", "plan:write"));
        assert!(is_scope_sufficient("admin", "write"));
        assert!(is_scope_sufficient("admin", "read"));
    }

    #[test]
    fn read_scope_only_sufficient_for_read() {
        assert!(is_scope_sufficient("read", "read"));
        assert!(!is_scope_sufficient("read", "admin"));
        assert!(!is_scope_sufficient("read", "agent:write"));
    }

    // --- cors / local origin tests ---

    #[test]
    fn local_origin_accepts_localhost() {
        assert!(is_local_origin("http://localhost:5173"));
        assert!(is_local_origin("https://localhost:443"));
        assert!(is_local_origin("http://localhost"));
    }

    #[test]
    fn local_origin_accepts_127_0_0_1() {
        assert!(is_local_origin("http://127.0.0.1:3000"));
        assert!(is_local_origin("https://127.0.0.1"));
    }

    #[test]
    fn local_origin_accepts_ipv6_loopback() {
        assert!(is_local_origin("http://[::1]:3000"));
    }

    #[test]
    fn local_origin_rejects_external_or_malformed() {
        assert!(!is_local_origin("http://evil.com"));
        assert!(!is_local_origin("https://api.example.com"));
        assert!(!is_local_origin("localhost:3000"));
        assert!(!is_local_origin("http://192.168.1.1:6677"));
    }

    // --- T3-28: CORS allow-list tests ----------------------------------

    /// Build a tiny router protected by the production `cors_layer` so
    /// preflight OPTIONS requests exercise the real allow-lists.
    fn cors_test_app(allowed_origin: &str) -> axum::Router {
        let cors = cors_layer(&[allowed_origin.to_string()], false);
        axum::Router::new()
            .route("/api/ping", axum::routing::get(|| async { "pong" }))
            .layer(cors)
    }

    async fn preflight(
        app: &axum::Router,
        origin: &str,
        method: &str,
        request_headers: Option<&str>,
    ) -> axum::http::Response<Body> {
        let mut req = Request::builder()
            .method(Method::OPTIONS)
            .uri("/api/ping")
            .header(axum::http::header::ORIGIN, origin)
            .header("access-control-request-method", method);
        if let Some(headers) = request_headers {
            req = req.header("access-control-request-headers", headers);
        }
        let req = req.body(Body::empty()).expect("request");
        tower::ServiceExt::oneshot(app.clone(), req)
            .await
            .expect("oneshot")
    }

    #[tokio::test]
    async fn cors_preflight_allows_listed_method_and_header() {
        let app = cors_test_app("https://app.example.com");
        let resp = preflight(
            &app,
            "https://app.example.com",
            "POST",
            Some("content-type, x-api-key"),
        )
        .await;

        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        let allow_methods = resp
            .headers()
            .get("access-control-allow-methods")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_ascii_uppercase();
        for verb in ["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"] {
            assert!(
                allow_methods.contains(verb),
                "{verb} missing from {allow_methods:?}"
            );
        }

        let allow_headers = resp
            .headers()
            .get("access-control-allow-headers")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();
        for header in ["content-type", "authorization", "x-api-key"] {
            assert!(
                allow_headers.contains(header),
                "{header} missing from {allow_headers:?}"
            );
        }
    }

    #[tokio::test]
    async fn cors_preflight_rejects_disallowed_method() {
        let app = cors_test_app("https://app.example.com");
        let resp = preflight(&app, "https://app.example.com", "TRACE", None).await;

        // tower-http answers with 200 for any preflight but only echoes the
        // matching headers. The absence of `access-control-allow-methods`
        // is what makes the browser refuse the actual request.
        let allow_methods = resp
            .headers()
            .get("access-control-allow-methods")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_ascii_uppercase();
        assert!(
            !allow_methods.contains("TRACE"),
            "TRACE leaked into allow-methods: {allow_methods:?}"
        );
    }

    #[tokio::test]
    async fn cors_preflight_rejects_disallowed_header() {
        let app = cors_test_app("https://app.example.com");
        let resp = preflight(
            &app,
            "https://app.example.com",
            "POST",
            Some("x-totally-fake"),
        )
        .await;

        // Same shape as the method case: the request-header is not echoed
        // back, so the browser refuses the cross-origin call.
        let allow_headers = resp
            .headers()
            .get("access-control-allow-headers")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();
        assert!(
            !allow_headers.contains("x-totally-fake"),
            "x-totally-fake leaked into allow-headers: {allow_headers:?}"
        );
    }

    // --- T55: default local-only and unsafe_public_cors tests ---

    /// Build a router using the default cors_layer (empty origins, not unsafe).
    /// This should only allow local origins.
    fn cors_default_local_app() -> axum::Router {
        let cors = cors_layer(&[], false);
        axum::Router::new()
            .route("/api/ping", axum::routing::get(|| async { "pong" }))
            .layer(cors)
    }

    /// Build a router using unsafe_public_cors = true (wildcard CORS).
    fn cors_unsafe_public_app() -> axum::Router {
        let cors = cors_layer(&[], true);
        axum::Router::new()
            .route("/api/ping", axum::routing::get(|| async { "pong" }))
            .layer(cors)
    }

    #[tokio::test]
    async fn cors_default_allows_local_origin() {
        let app = cors_default_local_app();
        let resp = preflight(&app, "http://localhost:5173", "GET", None).await;

        // Local origin should be reflected back in access-control-allow-origin.
        let allow_origin = resp
            .headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(
            allow_origin, "http://localhost:5173",
            "local origin should be allowed by default"
        );
    }

    #[tokio::test]
    async fn cors_default_allows_127_0_0_1_origin() {
        let app = cors_default_local_app();
        let resp = preflight(&app, "http://127.0.0.1:3000", "POST", None).await;

        let allow_origin = resp
            .headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(
            allow_origin, "http://127.0.0.1:3000",
            "127.0.0.1 origin should be allowed by default"
        );
    }

    #[tokio::test]
    async fn cors_default_rejects_non_local_origin() {
        let app = cors_default_local_app();
        let resp = preflight(&app, "https://evil.com", "GET", None).await;

        // Non-local origin should NOT get an access-control-allow-origin header.
        let allow_origin = resp
            .headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            allow_origin.is_empty() || allow_origin == "null",
            "non-local origin should be rejected, got: {allow_origin:?}"
        );
    }

    #[tokio::test]
    async fn cors_unsafe_public_allows_any_origin() {
        let app = cors_unsafe_public_app();
        let resp = preflight(&app, "https://anything.evil.com", "POST", None).await;

        // Wildcard CORS should respond with `*` or the request origin.
        let allow_origin = resp
            .headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            allow_origin == "*" || allow_origin == "https://anything.evil.com",
            "unsafe_public_cors should allow any origin, got: {allow_origin:?}"
        );
    }

    #[tokio::test]
    async fn cors_exact_origin_rejects_unlisted_origin() {
        let app = cors_test_app("https://app.example.com");
        let resp = preflight(&app, "https://not-allowed.com", "GET", None).await;

        // An origin not in the allow-list should not get reflected.
        let allow_origin = resp
            .headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            allow_origin.is_empty()
                || allow_origin == "null"
                || !allow_origin.contains("not-allowed"),
            "unlisted origin should be rejected, got: {allow_origin:?}"
        );
    }

    // --- E04-T03 / E04-T19: route-to-scope manifest CI guard -------------------

    /// Representative mutating paths derived from the router assembly in
    /// `routes/mod.rs`. Each entry is a `(method, path)` tuple.
    ///
    /// **When you add a new mutating route**, add a representative path here
    /// so the guard test below catches it. If this list drifts from the
    /// router, the test will still catch unclassified routes -- but keeping
    /// it in sync means the failure message points to the exact route.
    const EXPECTED_MUTATING_ROUTES: &[(Method, &str)] = &[
        // --- /api/agents (agent:write) ---
        (Method::POST, "/api/agents/register"),
        (Method::POST, "/api/agents/create"),
        (Method::POST, "/api/agents/123/stop"),
        (Method::POST, "/api/agents/123/message"),
        (Method::POST, "/api/agents/123/start"),
        (Method::POST, "/api/agents/123/restart"),
        (Method::POST, "/api/agents/123/token"),
        // --- /api/meta (agent:write) ---
        (Method::POST, "/api/meta/agents"),
        (Method::POST, "/api/meta/agents/123/validate"),
        (Method::POST, "/api/meta/agents/123/morph"),
        (Method::POST, "/api/meta/agents/123/morph/rollback"),
        (Method::POST, "/api/meta/agents/123/deactivate"),
        // --- /api/api-keys (admin) ---
        (Method::POST, "/api/api-keys"),
        (Method::DELETE, "/api/api-keys/test-key"),
        (Method::POST, "/api/api-keys/test-key/rotate"),
        // --- /api/agent-tokens (admin) ---
        (Method::POST, "/api/agent-tokens"),
        (Method::DELETE, "/api/agent-tokens/tok-123"),
        // --- /api/secrets (admin) ---
        (Method::POST, "/api/secrets/ns/key"),
        (Method::DELETE, "/api/secrets/ns/key"),
        (Method::POST, "/api/secrets/ns/key/test"),
        // --- /api/config (admin) ---
        (Method::PUT, "/api/config"),
        (Method::POST, "/api/config/reload"),
        // --- /api/events/ingest (agent:write) ---
        (Method::POST, "/api/events/ingest"),
        (Method::POST, "/api/events/ingest/batch"),
        // --- /api/plans (plan:write) ---
        (Method::POST, "/api/plans"),
        (Method::POST, "/api/plans/123/execute"),
        (Method::POST, "/api/plans/123/pause"),
        (Method::POST, "/api/plans/123/resume"),
        (Method::POST, "/api/plans/123/tasks/t1/review"),
        (Method::POST, "/api/plans/123/chat"),
        (Method::POST, "/api/plans/123/estimate"),
        (Method::POST, "/api/plans/generate"),
        // --- /api/prd (plan:write) ---
        (Method::POST, "/api/prds/ideas"),
        (Method::POST, "/api/prd/consolidate"),
        (Method::POST, "/api/prds/consolidate"),
        (Method::POST, "/api/prds/my-slug/draft"),
        (Method::POST, "/api/prds/my-slug/promote"),
        (Method::POST, "/api/prds/my-slug/plan"),
        // --- /api/workspaces (write) ---
        (Method::POST, "/api/workspaces"),
        // --- /api/jobs (write) ---
        (Method::POST, "/api/jobs"),
        (Method::POST, "/api/jobs/match"),
        (Method::POST, "/api/jobs/123/assign"),
        (Method::POST, "/api/jobs/123/start"),
        (Method::POST, "/api/jobs/123/submit"),
        (Method::POST, "/api/jobs/123/evaluate"),
        (Method::POST, "/api/jobs/123/execute"),
        (Method::POST, "/api/jobs/123/cancel"),
        // --- /api/run (write) ---
        (Method::POST, "/api/run"),
        // --- /api/runs (write — shares /api/run prefix in manifest) ---
        (Method::POST, "/api/runs/123/share"),
        // --- /api/dream (write) ---
        (Method::POST, "/api/dream/run"),
        // --- /api/deployments (write) ---
        (Method::POST, "/api/deployments"),
        (Method::DELETE, "/api/deployments/123"),
        (Method::POST, "/api/deployments/123/task"),
        (Method::POST, "/api/deployments/123/callback"),
        // --- /api/research (write) ---
        (Method::POST, "/api/research/topic"),
        (Method::POST, "/api/research/enhance-prd/my-slug"),
        (Method::POST, "/api/research/enhance-plan/my-plan"),
        (Method::POST, "/api/research/enhance-tasks/my-plan"),
        (Method::POST, "/api/research/analyze"),
        // --- /api/subscriptions (write) ---
        (Method::POST, "/api/subscriptions"),
        (Method::PUT, "/api/subscriptions/123"),
        (Method::DELETE, "/api/subscriptions/123"),
        (Method::POST, "/api/subscriptions/123/enable"),
        (Method::POST, "/api/subscriptions/123/disable"),
        // --- /api/templates (write) ---
        (Method::POST, "/api/templates"),
        (Method::POST, "/api/templates/my-tmpl/deploy"),
        // --- /api/heartbeats (write) ---
        (Method::POST, "/api/heartbeats"),
        // --- /api/neuro (write) ---
        (Method::POST, "/api/neuro/query"),
        // --- /api/inference (write) ---
        (Method::POST, "/api/inference/complete"),
        (Method::POST, "/api/inference/batch/submit"),
        // --- /api/gateway (write) ---
        (Method::POST, "/api/gateway/inference"),
        (Method::POST, "/api/gateway/batch/submit"),
        (Method::POST, "/api/gateway/batch/flush"),
        // --- /api/bench (write) ---
        (Method::POST, "/api/bench/run"),
        (Method::POST, "/api/bench/runs"),
        (Method::DELETE, "/api/bench/run/123"),
        (Method::DELETE, "/api/bench/runs/123"),
        (Method::POST, "/api/bench/runs/123/cancel"),
        (Method::POST, "/api/bench/suites"),
        (Method::POST, "/api/bench/swe/run"),
        // --- /api/connectors (write) ---
        (Method::POST, "/api/connectors"),
        (Method::DELETE, "/api/connectors/my-conn"),
        // --- /api/feeds (write) ---
        (Method::POST, "/api/feeds"),
        (Method::DELETE, "/api/feeds/123"),
        (Method::POST, "/api/feeds/start/file-watch-roko-dir"),
        (Method::POST, "/api/feeds/stop/file-watch-roko-dir"),
        // --- /api/recipes (write) ---
        (Method::POST, "/api/recipes"),
        (Method::DELETE, "/api/recipes/blend"),
        (Method::POST, "/api/recipes/blend/evaluate"),
        // --- /api/groups and invitations (write) ---
        (Method::POST, "/api/groups"),
        (Method::PATCH, "/api/groups/grp-123"),
        (Method::DELETE, "/api/groups/grp-123"),
        (Method::POST, "/api/groups/grp-123/invite"),
        (Method::PATCH, "/api/groups/grp-123/members/agent-1"),
        (Method::DELETE, "/api/groups/grp-123/members/agent-1"),
        (Method::POST, "/api/groups/grp-123/knowledge"),
        (Method::POST, "/api/groups/grp-123/pheromones"),
        (Method::POST, "/api/groups/grp-123/message"),
        (Method::POST, "/api/invitations/inv-123/accept"),
        (Method::POST, "/api/invitations/inv-123/reject"),
        // --- /api/marketplace (write; handlers remain explicit E38 stubs) ---
        (Method::POST, "/api/marketplace/publish"),
        (Method::POST, "/api/marketplace/fork"),
        // --- /api/registries (admin; durable local lifecycle/indexer control) ---
        (Method::POST, "/api/registries/passports"),
        (Method::POST, "/api/registries/passports/1/transfer"),
        (Method::PUT, "/api/registries/passports/1/metadata"),
        (Method::POST, "/api/registries/passports/1/delegations"),
        (Method::DELETE, "/api/registries/passports/1/delegations/2"),
        (Method::POST, "/api/registries/knowledge"),
        (Method::POST, "/api/registries/knowledge/abc/validate"),
        (Method::POST, "/api/registries/knowledge/abc/challenge"),
        (
            Method::POST,
            "/api/registries/knowledge/challenges/abc/resolve",
        ),
        (Method::POST, "/api/registries/indexer/sync"),
        (Method::POST, "/api/registries/indexer/rebuild"),
        // --- /api/arenas (write; durable lifecycle and settlement) ---
        (Method::POST, "/api/arenas"),
        (Method::PATCH, "/api/arenas/arena-123"),
        (Method::POST, "/api/arenas/arena-123/attempts"),
        (
            Method::POST,
            "/api/arenas/arena-123/attempts/attempt-1/submit",
        ),
        (
            Method::POST,
            "/api/arenas/arena-123/attempts/attempt-1/settle",
        ),
        // --- /api/defi (write; handlers remain explicit E41 stubs) ---
        (Method::POST, "/api/defi/bonds"),
        (Method::POST, "/api/defi/options/price"),
        (Method::POST, "/api/defi/insurance"),
        (Method::POST, "/api/defi/insurance/policy-123/claims"),
        // --- /api/rpc (write) ---
        (Method::POST, "/api/rpc"),
        // --- /api/vision-loop (write) ---
        (Method::POST, "/api/vision-loop"),
        (Method::POST, "/api/vision-loop/run123/cancel"),
        // --- /api/team (write) ---
        (Method::POST, "/api/team/join"),
        (Method::POST, "/api/team/invite"),
        (Method::PUT, "/api/team/members/did:test"),
        (Method::DELETE, "/api/team/members/did:test"),
        // --- /api/webhooks (write) ---
        (Method::POST, "/api/webhooks/generic"),
        // --- /api/providers (write) ---
        (Method::POST, "/api/providers/openai/test"),
        // --- /relay (agent:write) ---
        (Method::POST, "/relay/agents"),
        (Method::DELETE, "/relay/agents/123"),
    ];

    /// CI guard: every mutating route registered in the router must have an
    /// explicit scope classification in [`ROUTE_SCOPE_MANIFEST`]. If a new
    /// route is added without a manifest entry, it will resolve to
    /// [`SCOPE_WRITE_UNCLASSIFIED`] and this test will fail, preventing a
    /// scope regression from reaching production.
    #[test]
    fn mutating_routes_are_classified() {
        let mut failures = Vec::new();
        for (method, path) in EXPECTED_MUTATING_ROUTES {
            let scope = required_scope_for(method, path);
            if scope == SCOPE_WRITE_UNCLASSIFIED {
                failures.push(format!(
                    "  {method} {path} -> {scope} (unclassified fallback)"
                ));
            }
        }
        assert!(
            failures.is_empty(),
            "The following mutating routes are not explicitly classified in \
             ROUTE_SCOPE_MANIFEST. Add a manifest entry for each:\n{}",
            failures.join("\n"),
        );
    }

    #[test]
    fn unclassified_fallback_returns_sentinel() {
        // A path that does not match any manifest entry must return the
        // unclassified sentinel, NOT plain "write".
        assert_eq!(
            required_scope_for(&Method::POST, "/api/totally-unknown-endpoint"),
            SCOPE_WRITE_UNCLASSIFIED,
        );
    }

    #[test]
    fn unclassified_fallback_still_denies_read_scope() {
        // Even though the sentinel is distinct from "write", is_scope_sufficient
        // must treat it as "write" so read-only keys are still blocked.
        assert!(!is_scope_sufficient("read", SCOPE_WRITE_UNCLASSIFIED));
        assert!(is_scope_sufficient("write", SCOPE_WRITE_UNCLASSIFIED));
        assert!(is_scope_sufficient("admin", SCOPE_WRITE_UNCLASSIFIED));
    }

    /// No duplicate prefixes in the manifest (copy-paste guard).
    #[test]
    fn route_scope_manifest_has_no_duplicates() {
        let prefixes: Vec<&str> = ROUTE_SCOPE_MANIFEST.iter().map(|e| e.prefix).collect();
        for (i, p) in prefixes.iter().enumerate() {
            assert!(
                !prefixes[i + 1..].contains(p),
                "duplicate manifest prefix: {p}"
            );
        }
    }

    // --- E04-T03: auth fixture tests (full require_api_key + require_scope stack) ---
    //
    // These tests exercise the complete authentication + scope enforcement path
    // using real `ApiKeyEntry` fixtures with hashed keys. Unlike `scope_test_app`
    // which injects `AuthContext` directly, these go through `require_api_key` so
    // the full credential-lookup → scope-injection → scope-enforcement chain is
    // verified end-to-end.

    fn keyed_auth(name: &str, plaintext: &str, scope: &str) -> ServeAuthConfig {
        ServeAuthConfig {
            enabled: true,
            api_key: String::new(),
            api_keys: vec![roko_core::config::ApiKeyEntry {
                name: name.to_string(),
                key_hash: hash_api_key(plaintext),
                scope: scope.to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                expires_at: None,
                last_used_at: None,
                previous_key_hashes: Vec::new(),
            }],
            privy_app_id: None,
            ..Default::default()
        }
    }

    /// Build a router with BOTH `require_api_key` AND `require_scope` layered,
    /// mounting a trivial POST handler at the given path.
    fn full_auth_app(auth: ServeAuthConfig, route: &str) -> Router {
        let state = make_test_state(auth);
        let handler = || async { StatusCode::NO_CONTENT };
        // Layer order: require_scope outer (runs after auth), require_api_key
        // inner (runs first, injects AuthContext).
        Router::new()
            .route(route, post(handler))
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                require_scope,
            ))
            .layer(axum::middleware::from_fn_with_state(state, require_api_key))
    }

    async fn fixture_response(app: Router, key: &str, method: Method, uri: &str) -> Response {
        let req = Request::builder()
            .method(method)
            .uri(uri)
            .header("X-Api-Key", key)
            .body(Body::empty())
            .expect("invariant: fixture auth request builds");
        app.oneshot(req)
            .await
            .expect("invariant: fixture auth router responds")
    }

    /// A `read`-scoped key must be denied (403) on a POST to an admin route.
    #[tokio::test]
    async fn auth_fixture_read_key_denied_on_admin_route() {
        let plaintext = "fixture-read-key-001";
        let app = full_auth_app(
            keyed_auth("read-fixture", plaintext, "read"),
            "/api/secrets",
        );
        let resp = fixture_response(app, plaintext, Method::POST, "/api/secrets").await;
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "read-scoped key must be denied 403 on POST /api/secrets (admin route)"
        );
    }

    /// An `agent:write`-scoped key must be denied (403) on a POST to an admin route.
    #[tokio::test]
    async fn auth_fixture_agent_write_key_denied_on_admin_route() {
        let plaintext = "fixture-agent-write-key-001";
        let app = full_auth_app(
            keyed_auth("agent-write-fixture", plaintext, "agent:write"),
            "/api/secrets",
        );
        let resp = fixture_response(app, plaintext, Method::POST, "/api/secrets").await;
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "agent:write-scoped key must be denied 403 on POST /api/secrets (admin route)"
        );
    }

    /// A `write`-scoped key must be allowed (204) on a POST to a write route.
    #[tokio::test]
    async fn auth_fixture_write_key_allowed_on_write_route() {
        let plaintext = "fixture-write-key-001";
        let app = full_auth_app(keyed_auth("write-fixture", plaintext, "write"), "/api/jobs");
        let resp = fixture_response(app, plaintext, Method::POST, "/api/jobs").await;
        assert_eq!(
            resp.status(),
            StatusCode::NO_CONTENT,
            "write-scoped key must succeed (204) on POST /api/jobs"
        );
    }

    /// An `admin`-scoped key must be allowed (204) on every route, including admin.
    #[tokio::test]
    async fn auth_fixture_admin_key_allowed_on_admin_route() {
        let plaintext = "fixture-admin-key-001";
        let app = full_auth_app(
            keyed_auth("admin-fixture", plaintext, "admin"),
            "/api/secrets",
        );
        let resp = fixture_response(app, plaintext, Method::POST, "/api/secrets").await;
        assert_eq!(
            resp.status(),
            StatusCode::NO_CONTENT,
            "admin-scoped key must succeed (204) on POST /api/secrets"
        );
    }

    // ── E04-T06: Extension route whitelist ────────────────────────────────────

    /// [`known_static_scope`] maps every recognised scope token to the
    /// corresponding `&'static str` and falls back to the unclassified sentinel
    /// for unrecognised values.
    #[test]
    fn known_static_scope_maps_recognised_scopes() {
        assert_eq!(known_static_scope("read"), "read");
        assert_eq!(known_static_scope("write"), "write");
        assert_eq!(known_static_scope("admin"), "admin");
        assert_eq!(known_static_scope("agent:write"), "agent:write");
        assert_eq!(known_static_scope("plan:write"), "plan:write");
        assert_eq!(known_static_scope("terminal:write"), "terminal:write");
        // Unrecognised values fall to the sentinel (fail-closed).
        assert_eq!(
            known_static_scope("unknown:scope"),
            SCOPE_WRITE_UNCLASSIFIED
        );
        assert_eq!(known_static_scope(""), SCOPE_WRITE_UNCLASSIFIED);
    }

    /// When an extension route is registered with an explicit scope, that scope
    /// must not resolve to the unclassified sentinel.
    /// We test via [`known_static_scope`] directly to avoid the `OnceLock`
    /// singleton constraint between tests.
    #[test]
    fn extension_scope_for_returns_declared_scope_when_registered() {
        // Every scope a plugin can declare in its manifest must map to a
        // recognised static token, not to SCOPE_WRITE_UNCLASSIFIED.
        for declared in [
            "read",
            "write",
            "admin",
            "agent:write",
            "plan:write",
            "terminal:write",
        ] {
            let resolved = known_static_scope(declared);
            assert_ne!(
                resolved, SCOPE_WRITE_UNCLASSIFIED,
                "scope '{declared}' declared by an extension should not resolve to the \
                 unclassified sentinel — it must be an explicit whitelist entry"
            );
        }
    }

    /// A plugin that does NOT declare a scope for its webhook trigger uses the
    /// default `"write"`. Verify this maps to the explicit `"write"` token, not
    /// the unclassified sentinel.
    #[test]
    fn extension_default_write_scope_is_not_sentinel() {
        assert_eq!(
            known_static_scope("write"),
            "write",
            "the default extension scope 'write' must map to the explicit 'write' \
             token, not to SCOPE_WRITE_UNCLASSIFIED"
        );
        assert_ne!(
            known_static_scope("write"),
            SCOPE_WRITE_UNCLASSIFIED,
            "the default extension scope must not trigger the unclassified fallback"
        );
    }

    /// Core routes in [`ROUTE_SCOPE_MANIFEST`] take precedence over any
    /// extension route with the same prefix. A plugin cannot downgrade the
    /// scope requirement for a first-party route.
    #[test]
    fn static_manifest_takes_precedence_over_extension_routes() {
        // The static manifest entry for "/api/secrets" → "admin" must always
        // win, regardless of what the extension registry might contain.
        let scope = required_scope_for(&Method::POST, "/api/secrets");
        assert_eq!(
            scope, "admin",
            "static ROUTE_SCOPE_MANIFEST must take precedence for /api/secrets"
        );
        let scope = required_scope_for(&Method::POST, "/api/agents/register");
        assert_eq!(scope, "agent:write");
    }

    /// `register_extension_route_scopes` is idempotent: calling it twice must
    /// not panic (the second call is a no-op via `OnceLock::set`).
    #[test]
    fn register_extension_route_scopes_is_idempotent() {
        register_extension_route_scopes([("/hooks/test-plugin".to_string(), "write".to_string())]);
        // Second call — must be a silent no-op.
        register_extension_route_scopes([(
            "/hooks/other-plugin".to_string(),
            "agent:write".to_string(),
        )]);
        // No panic means the test passes.
    }

    // ── RBAC middleware tests ───────────────────────────────────────────

    use crate::rbac::{Permission, Role};

    #[test]
    fn rbac_scope_to_role_maps_admin() {
        assert_eq!(scope_to_role("admin"), Role::Admin);
    }

    #[test]
    fn rbac_scope_to_role_maps_owner() {
        assert_eq!(scope_to_role("owner"), Role::Owner);
    }

    #[test]
    fn rbac_scope_to_role_maps_write_variants_to_member() {
        assert_eq!(scope_to_role("write"), Role::Member);
        assert_eq!(scope_to_role("agent:write"), Role::Member);
        assert_eq!(scope_to_role("plan:write"), Role::Member);
        assert_eq!(scope_to_role("terminal:write"), Role::Member);
    }

    #[test]
    fn rbac_scope_to_role_maps_read_to_viewer() {
        assert_eq!(scope_to_role("read"), Role::Viewer);
    }

    #[test]
    fn rbac_scope_to_role_defaults_unknown_to_viewer() {
        assert_eq!(scope_to_role("unknown-scope"), Role::Viewer);
        assert_eq!(scope_to_role(""), Role::Viewer);
    }

    /// Helper: build a minimal router with the `require_permission` middleware
    /// and a pre-injected `AuthContext`.
    fn rbac_test_app(scope: &str, required: Permission) -> Router {
        let handler = || async { StatusCode::NO_CONTENT };
        Router::new()
            .route("/test", post(handler))
            .route("/test", get(handler))
            .layer(axum::middleware::from_fn(require_permission(required)))
            .layer(axum::Extension(AuthContext {
                method: AuthMethod::ApiKey,
                scope: scope.to_string(),
                user_id: Some("test-user".to_string()),
            }))
    }

    #[tokio::test]
    async fn rbac_admin_allowed_config_edit() {
        let app = rbac_test_app("admin", Permission::ConfigEdit);
        let req = Request::builder()
            .method(Method::POST)
            .uri("/test")
            .body(Body::empty())
            .expect("build request");
        let resp = app.oneshot(req).await.expect("oneshot");
        assert_eq!(
            resp.status(),
            StatusCode::NO_CONTENT,
            "admin scope must be allowed ConfigEdit"
        );
    }

    #[tokio::test]
    async fn rbac_viewer_denied_config_edit() {
        let app = rbac_test_app("read", Permission::ConfigEdit);
        let req = Request::builder()
            .method(Method::POST)
            .uri("/test")
            .body(Body::empty())
            .expect("build request");
        let resp = app.oneshot(req).await.expect("oneshot");
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "read/viewer scope must be denied ConfigEdit"
        );
    }

    #[tokio::test]
    async fn rbac_member_allowed_plan_execute() {
        let app = rbac_test_app("write", Permission::PlanExecute);
        let req = Request::builder()
            .method(Method::POST)
            .uri("/test")
            .body(Body::empty())
            .expect("build request");
        let resp = app.oneshot(req).await.expect("oneshot");
        assert_eq!(
            resp.status(),
            StatusCode::NO_CONTENT,
            "write/member scope must be allowed PlanExecute"
        );
    }

    #[tokio::test]
    async fn rbac_viewer_denied_plan_execute() {
        let app = rbac_test_app("read", Permission::PlanExecute);
        let req = Request::builder()
            .method(Method::POST)
            .uri("/test")
            .body(Body::empty())
            .expect("build request");
        let resp = app.oneshot(req).await.expect("oneshot");
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "viewer must be denied PlanExecute"
        );
    }

    #[tokio::test]
    async fn rbac_member_denied_secrets_write() {
        let app = rbac_test_app("write", Permission::SecretsWrite);
        let req = Request::builder()
            .method(Method::POST)
            .uri("/test")
            .body(Body::empty())
            .expect("build request");
        let resp = app.oneshot(req).await.expect("oneshot");
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "member scope must be denied SecretsWrite (requires Admin+)"
        );
    }

    #[tokio::test]
    async fn rbac_admin_denied_team_manage() {
        let app = rbac_test_app("admin", Permission::TeamManage);
        let req = Request::builder()
            .method(Method::POST)
            .uri("/test")
            .body(Body::empty())
            .expect("build request");
        let resp = app.oneshot(req).await.expect("oneshot");
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "admin scope must be denied TeamManage (requires Owner)"
        );
    }

    #[tokio::test]
    async fn rbac_owner_allowed_team_manage() {
        let app = rbac_test_app("owner", Permission::TeamManage);
        let req = Request::builder()
            .method(Method::POST)
            .uri("/test")
            .body(Body::empty())
            .expect("build request");
        let resp = app.oneshot(req).await.expect("oneshot");
        assert_eq!(
            resp.status(),
            StatusCode::NO_CONTENT,
            "owner scope must be allowed TeamManage"
        );
    }

    #[tokio::test]
    async fn rbac_get_requests_pass_through() {
        // Even a viewer-level scope must be able to GET through RBAC middleware
        // that gates a write-level permission like ConfigEdit.
        let app = rbac_test_app("read", Permission::ConfigEdit);
        let req = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(Body::empty())
            .expect("build request");
        let resp = app.oneshot(req).await.expect("oneshot");
        assert_eq!(
            resp.status(),
            StatusCode::NO_CONTENT,
            "GET requests must pass through RBAC regardless of permission"
        );
    }

    #[tokio::test]
    async fn rbac_no_auth_context_defaults_to_viewer() {
        // When no AuthContext is present (auth disabled), the middleware
        // should default to Viewer and deny write-level permissions.
        let handler = || async { StatusCode::NO_CONTENT };
        let app = Router::new()
            .route("/test", post(handler))
            .layer(axum::middleware::from_fn(require_permission(
                Permission::PlanExecute,
            )));
        let req = Request::builder()
            .method(Method::POST)
            .uri("/test")
            .body(Body::empty())
            .expect("build request");
        let resp = app.oneshot(req).await.expect("oneshot");
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "missing AuthContext must default to Viewer and be denied PlanExecute"
        );
    }

    #[tokio::test]
    async fn rbac_denied_response_has_forbidden_code() {
        let app = rbac_test_app("read", Permission::AgentSpawn);
        let req = Request::builder()
            .method(Method::POST)
            .uri("/test")
            .body(Body::empty())
            .expect("build request");
        let resp = app.oneshot(req).await.expect("oneshot");
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        let body = axum::body::to_bytes(resp.into_body(), 4096)
            .await
            .expect("collect body");
        let json: Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(json["code"], "forbidden");
        assert!(
            json["message"]
                .as_str()
                .unwrap_or("")
                .contains("agent:spawn"),
            "error message should mention the denied permission"
        );
    }

    // ── E35-T08: enforcement mode + audit event tests ─────────────────────────

    /// Build a test `ServeAuthConfig` with a specific `EnforcementMode`.
    fn auth_with_enforcement(mode: roko_core::config::EnforcementMode) -> ServeAuthConfig {
        ServeAuthConfig {
            enforcement_mode: mode,
            ..Default::default()
        }
    }

    /// Build a scope test app whose backing `AppState` uses the given
    /// enforcement mode. The caller's `AuthContext` is injected via Extension.
    fn enforcement_test_app(
        mode: roko_core::config::EnforcementMode,
        scope: &str,
        user_id: Option<String>,
    ) -> Router {
        let state = make_test_state(auth_with_enforcement(mode));
        let handler = || async { StatusCode::NO_CONTENT };
        Router::new()
            .route("/api/secrets", post(handler))
            .route("/api/agents/test", post(handler))
            .route("/api/status", get(handler))
            .layer(axum::middleware::from_fn_with_state(state, require_scope))
            .layer(axum::Extension(AuthContext {
                method: AuthMethod::ApiKey,
                scope: scope.to_string(),
                user_id,
            }))
    }

    // -- enforce mode (default) -----------------------------------------------

    #[tokio::test]
    async fn enforcement_enforce_blocks_insufficient_scope() {
        let app = enforcement_test_app(roko_core::config::EnforcementMode::Enforce, "read", None);
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/secrets")
            .body(Body::empty())
            .expect("invariant: request builds");
        let resp = app.oneshot(req).await.expect("invariant: router responds");
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "enforce mode must block insufficient scope with 403"
        );
    }

    #[tokio::test]
    async fn enforcement_enforce_allows_sufficient_scope() {
        let app = enforcement_test_app(roko_core::config::EnforcementMode::Enforce, "admin", None);
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/secrets")
            .body(Body::empty())
            .expect("invariant: request builds");
        let resp = app.oneshot(req).await.expect("invariant: router responds");
        assert_eq!(
            resp.status(),
            StatusCode::NO_CONTENT,
            "enforce mode must allow admin scope on admin route"
        );
    }

    // -- audit mode -----------------------------------------------------------

    #[tokio::test]
    async fn enforcement_audit_allows_insufficient_scope() {
        let app = enforcement_test_app(roko_core::config::EnforcementMode::Audit, "read", None);
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/secrets")
            .body(Body::empty())
            .expect("invariant: request builds");
        let resp = app.oneshot(req).await.expect("invariant: router responds");
        assert_eq!(
            resp.status(),
            StatusCode::NO_CONTENT,
            "audit mode must allow the request through even with insufficient scope"
        );
    }

    #[tokio::test]
    async fn enforcement_audit_allows_sufficient_scope() {
        let app = enforcement_test_app(roko_core::config::EnforcementMode::Audit, "admin", None);
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/secrets")
            .body(Body::empty())
            .expect("invariant: request builds");
        let resp = app.oneshot(req).await.expect("invariant: router responds");
        assert_eq!(
            resp.status(),
            StatusCode::NO_CONTENT,
            "audit mode must allow admin scope"
        );
    }

    // -- disabled mode --------------------------------------------------------

    #[tokio::test]
    async fn enforcement_disabled_skips_scope_check() {
        let app = enforcement_test_app(roko_core::config::EnforcementMode::Disabled, "read", None);
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/secrets")
            .body(Body::empty())
            .expect("invariant: request builds");
        let resp = app.oneshot(req).await.expect("invariant: router responds");
        assert_eq!(
            resp.status(),
            StatusCode::NO_CONTENT,
            "disabled mode must skip scope checks entirely"
        );
    }

    // -- GET always bypasses regardless of mode --------------------------------

    #[tokio::test]
    async fn enforcement_get_always_bypasses() {
        for mode in [
            roko_core::config::EnforcementMode::Enforce,
            roko_core::config::EnforcementMode::Audit,
            roko_core::config::EnforcementMode::Disabled,
        ] {
            let app = enforcement_test_app(mode, "read", None);
            let req = Request::builder()
                .method(Method::GET)
                .uri("/api/status")
                .body(Body::empty())
                .expect("invariant: request builds");
            let resp = app.oneshot(req).await.expect("invariant: router responds");
            assert_eq!(
                resp.status(),
                StatusCode::NO_CONTENT,
                "GET must bypass scope checks in {mode:?} mode"
            );
        }
    }

    // -- audit event emission -------------------------------------------------

    #[tokio::test]
    async fn enforcement_emits_audit_event_on_deny() {
        let dir = tempdir().unwrap();
        let mut config = RokoConfig::default();
        config.serve.auth.enforcement_mode = roko_core::config::EnforcementMode::Enforce;
        let state = Arc::new(
            AppState::new(
                dir.path().to_path_buf(),
                Arc::new(NoOpRuntime),
                config,
                Arc::new(ManualBackend::default()),
            )
            .expect("AppState::new"),
        );
        let handler = || async { StatusCode::NO_CONTENT };
        let app = Router::new()
            .route("/api/secrets", post(handler))
            .layer(axum::middleware::from_fn_with_state(state, require_scope))
            .layer(axum::Extension(AuthContext {
                method: AuthMethod::ApiKey,
                scope: "read".to_string(),
                user_id: Some("test-user".to_string()),
            }));

        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/secrets")
            .body(Body::empty())
            .expect("invariant: request builds");
        let resp = app.oneshot(req).await.expect("invariant: router responds");
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        // Verify audit event was written.
        let log = crate::auth_audit::open_audit_log(dir.path()).unwrap();
        let events = log.query(None, None, None, None).unwrap();
        assert_eq!(events.len(), 1, "exactly one audit event should be emitted");
        assert_eq!(
            events[0].action,
            crate::auth_audit::AuthAuditAction::PermissionDenied
        );
        assert_eq!(events[0].outcome, crate::auth_audit::AuthOutcome::Denied);
        assert_eq!(events[0].actor, "test-user");
        assert!(events[0].target.contains("POST /api/secrets"));
        assert_eq!(
            events[0].metadata.get("scope_has").map(|s| s.as_str()),
            Some("read")
        );
        assert_eq!(
            events[0].metadata.get("scope_required").map(|s| s.as_str()),
            Some("admin")
        );
    }

    #[tokio::test]
    async fn enforcement_emits_audit_event_on_grant() {
        let dir = tempdir().unwrap();
        let mut config = RokoConfig::default();
        config.serve.auth.enforcement_mode = roko_core::config::EnforcementMode::Enforce;
        let state = Arc::new(
            AppState::new(
                dir.path().to_path_buf(),
                Arc::new(NoOpRuntime),
                config,
                Arc::new(ManualBackend::default()),
            )
            .expect("AppState::new"),
        );
        let handler = || async { StatusCode::NO_CONTENT };
        let app = Router::new()
            .route("/api/secrets", post(handler))
            .layer(axum::middleware::from_fn_with_state(state, require_scope))
            .layer(axum::Extension(AuthContext {
                method: AuthMethod::ApiKey,
                scope: "admin".to_string(),
                user_id: Some("admin-user".to_string()),
            }));

        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/secrets")
            .body(Body::empty())
            .expect("invariant: request builds");
        let resp = app.oneshot(req).await.expect("invariant: router responds");
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // Verify audit event was written.
        let log = crate::auth_audit::open_audit_log(dir.path()).unwrap();
        let events = log.query(None, None, None, None).unwrap();
        assert_eq!(events.len(), 1, "exactly one audit event should be emitted");
        assert_eq!(
            events[0].action,
            crate::auth_audit::AuthAuditAction::PermissionGranted
        );
        assert_eq!(events[0].outcome, crate::auth_audit::AuthOutcome::Success);
        assert_eq!(events[0].actor, "admin-user");
        assert!(events[0].target.contains("POST /api/secrets"));
    }

    #[tokio::test]
    async fn enforcement_audit_mode_emits_deny_event_but_allows_request() {
        let dir = tempdir().unwrap();
        let mut config = RokoConfig::default();
        config.serve.auth.enforcement_mode = roko_core::config::EnforcementMode::Audit;
        let state = Arc::new(
            AppState::new(
                dir.path().to_path_buf(),
                Arc::new(NoOpRuntime),
                config,
                Arc::new(ManualBackend::default()),
            )
            .expect("AppState::new"),
        );
        let handler = || async { StatusCode::NO_CONTENT };
        let app = Router::new()
            .route("/api/secrets", post(handler))
            .layer(axum::middleware::from_fn_with_state(state, require_scope))
            .layer(axum::Extension(AuthContext {
                method: AuthMethod::ApiKey,
                scope: "read".to_string(),
                user_id: Some("audit-user".to_string()),
            }));

        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/secrets")
            .body(Body::empty())
            .expect("invariant: request builds");
        let resp = app.oneshot(req).await.expect("invariant: router responds");
        // Must allow through in audit mode.
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // Verify a deny audit event was still emitted.
        let log = crate::auth_audit::open_audit_log(dir.path()).unwrap();
        let events = log.query(None, None, None, None).unwrap();
        assert_eq!(
            events.len(),
            1,
            "audit event should be emitted in audit mode"
        );
        assert_eq!(
            events[0].action,
            crate::auth_audit::AuthAuditAction::PermissionDenied
        );
        assert_eq!(events[0].outcome, crate::auth_audit::AuthOutcome::Denied);
        assert_eq!(events[0].actor, "audit-user");
    }

    #[tokio::test]
    async fn enforcement_disabled_emits_no_audit_events() {
        let dir = tempdir().unwrap();
        let mut config = RokoConfig::default();
        config.serve.auth.enforcement_mode = roko_core::config::EnforcementMode::Disabled;
        let state = Arc::new(
            AppState::new(
                dir.path().to_path_buf(),
                Arc::new(NoOpRuntime),
                config,
                Arc::new(ManualBackend::default()),
            )
            .expect("AppState::new"),
        );
        let handler = || async { StatusCode::NO_CONTENT };
        let app = Router::new()
            .route("/api/secrets", post(handler))
            .layer(axum::middleware::from_fn_with_state(state, require_scope))
            .layer(axum::Extension(AuthContext {
                method: AuthMethod::ApiKey,
                scope: "read".to_string(),
                user_id: None,
            }));

        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/secrets")
            .body(Body::empty())
            .expect("invariant: request builds");
        let resp = app.oneshot(req).await.expect("invariant: router responds");
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // Disabled mode should not emit any audit events.
        let log = crate::auth_audit::open_audit_log(dir.path()).unwrap();
        let events = log.query(None, None, None, None).unwrap();
        assert!(
            events.is_empty(),
            "disabled mode must not emit audit events, found {}",
            events.len()
        );
    }

    // -- EnforcementMode serde -----------------------------------------------

    #[test]
    fn enforcement_mode_serde_roundtrip() {
        use roko_core::config::EnforcementMode;

        let cases = [
            (EnforcementMode::Enforce, "\"enforce\""),
            (EnforcementMode::Audit, "\"audit\""),
            (EnforcementMode::Disabled, "\"disabled\""),
        ];
        for (mode, expected_json) in cases {
            let serialized = serde_json::to_string(&mode).unwrap();
            assert_eq!(serialized, expected_json, "serde round-trip for {mode:?}");
            let deserialized: EnforcementMode = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, mode);
        }
    }

    #[test]
    fn enforcement_mode_default_is_enforce() {
        assert_eq!(
            roko_core::config::EnforcementMode::default(),
            roko_core::config::EnforcementMode::Enforce,
        );
    }
}
