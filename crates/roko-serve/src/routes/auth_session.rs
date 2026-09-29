//! Browser session minting and teardown.
//!
//! These routes are mounted **outside** the `require_api_key` middleware layer
//! — they are how a browser exchanges a launch token (or API key) for a
//! `roko_session` cookie. They still run inside the global rate limiter.
//!
//! ## Routes
//! - `POST   /api/auth/session` — create a session cookie
//! - `DELETE /api/auth/session` — revoke the current session cookie

use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::header::{AUTHORIZATION, SET_COOKIE};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use chrono::Utc;
use roko_core::config::{ApiKeyEntry, ServeAuthConfig};
use serde::Deserialize;

use crate::auth_audit::{AuthAuditAction, AuthAuditEvent, AuthOutcome};
use crate::error::ApiError;
use crate::routes::auth::parse_rfc3339;
use crate::routes::middleware::{constant_time_eq, extract_bearer_token, hash_api_key};
use crate::state::AppState;

/// Optional JSON body for `POST /api/auth/session`.
///
/// The credential may alternatively be supplied via `X-Api-Key` or
/// `Authorization: Bearer` headers; the JSON body is the most ergonomic
/// option for a browser `fetch()` call.
#[derive(Debug, Deserialize, Default)]
pub struct CreateSessionRequest {
    /// Launch token or API key value.
    #[serde(default)]
    pub token: Option<String>,
}

/// Returns `true` when the request appears to be over HTTPS.
///
/// Checks `X-Forwarded-Proto` first (populated by reverse proxies), then
/// falls back to `false` (HTTP) for direct connections, which is the common
/// local-dev case.
fn is_https(headers: &HeaderMap) -> bool {
    headers
        .get("X-Forwarded-Proto")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("https"))
        .unwrap_or(false)
}

/// Build the `Set-Cookie` header value for a newly minted session.
///
/// Attributes:
/// - `HttpOnly` — prevents JavaScript access.
/// - `SameSite=Strict` — no cross-site delivery.
/// - `Path=/` — available for all API paths.
/// - `Secure` — added only when the connection is HTTPS.
fn session_cookie(session_id: &str, https: bool) -> String {
    let mut cookie = format!("roko_session={session_id}; HttpOnly; SameSite=Strict; Path=/");
    if https {
        cookie.push_str("; Secure");
    }
    cookie
}

/// `Set-Cookie` value that instructs the browser to delete the session cookie.
const CLEAR_SESSION_COOKIE: &str =
    "roko_session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0";

/// Append an event to the auth audit log (best-effort; errors are swallowed).
fn audit(state: &AppState, event: AuthAuditEvent) {
    if let Some(log) = state.auth_audit.as_ref() {
        log.append(&event);
    }
}

/// Extract the `roko_session` cookie value from the `Cookie` header.
///
/// Parses the header naively (semicolon-split) to avoid an extra dependency.
/// Never logs the raw value.
fn extract_session_cookie<'h>(headers: &'h HeaderMap) -> Option<&'h str> {
    let cookie_str = headers.get("Cookie")?.to_str().ok()?;
    for part in cookie_str.split(';') {
        let part = part.trim();
        if let Some(value) = part.strip_prefix("roko_session=") {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

/// Validate a token against the launch token and all configured API keys.
///
/// Returns `Some(actor_label)` on success (suitable for the auth audit), or
/// `None` when the token does not match any accepted credential.
///
/// Matching order mirrors `require_api_key`:
/// 1. Server launch token.
/// 2. Named API keys (current hash + grace-period previous hashes).
/// 3. Legacy single `api_key` from config.
fn validate_token(
    token: &str,
    auth_config: &ServeAuthConfig,
    named_keys: &[ApiKeyEntry],
    state: &AppState,
) -> Option<String> {
    // 1. Launch token — highest priority.
    if state.local_access.launch_token_matches(token) {
        return Some("launch_token".to_string());
    }

    // 2. Named API keys.
    let token_hash = hash_api_key(token);
    let now = Utc::now();

    for entry in named_keys {
        // Current key hash.
        if entry.key_hash == token_hash {
            if let Some(ref expires) = entry.expires_at {
                // Expired keys cannot mint sessions.
                if parse_rfc3339(expires).is_none_or(|e| e <= now) {
                    return None;
                }
            }
            return Some(format!("api-key:{}", entry.name));
        }
        // Previous (rotated) hashes within their grace period.
        for (prev_hash, grace_expires) in &entry.previous_key_hashes {
            if prev_hash == &token_hash
                && parse_rfc3339(grace_expires).is_some_and(|e| e > now)
            {
                return Some(format!("api-key:{}", entry.name));
            }
        }
    }

    // 3. Legacy single api_key.
    if !auth_config.api_key.is_empty()
        && constant_time_eq(token.as_bytes(), auth_config.api_key.as_bytes())
    {
        return Some("api-key:legacy".to_string());
    }

    None
}

/// `POST /api/auth/session`
///
/// Exchanges a launch token or API key for a browser session cookie.
///
/// Credential resolution order (first match wins):
/// 1. `X-Api-Key` header
/// 2. `Authorization: Bearer <token>` header
/// 3. JSON body `{ "token": "..." }`
///
/// On success returns **204 No Content** with
/// `Set-Cookie: roko_session=<id>; HttpOnly; SameSite=Strict; Path=/`
/// (plus `Secure` for HTTPS connections).
///
/// On failure returns **401 Unauthorized** without setting a cookie.
async fn create_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Option<Json<CreateSessionRequest>>,
) -> Response {
    let route_label = "POST /api/auth/session";

    // Resolve the credential, preferring explicit headers over JSON body.
    let maybe_token: Option<String> = {
        if let Some(v) = headers.get("X-Api-Key").and_then(|v| v.to_str().ok()) {
            Some(v.to_string())
        } else if let Some(auth) = headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok()) {
            extract_bearer_token(auth).map(|t| t.to_string())
        } else {
            body.and_then(|b| b.0.token)
        }
    };

    let Some(token) = maybe_token else {
        audit(
            &state,
            AuthAuditEvent::new(
                "anonymous",
                AuthAuditAction::Login,
                route_label,
                AuthOutcome::Denied,
            ),
        );
        return ApiError::unauthorized("token is required to create a session").into_response();
    };

    let auth_config = state.load_roko_config().serve.auth.clone();
    let named_keys = state.auth_registry.api_keys_snapshot().await;

    let Some(actor) = validate_token(&token, &auth_config, &named_keys, &state) else {
        audit(
            &state,
            AuthAuditEvent::new(
                "anonymous",
                AuthAuditAction::Login,
                route_label,
                AuthOutcome::Denied,
            ),
        );
        return ApiError::unauthorized("invalid or missing credential").into_response();
    };

    // Mint the session and build the cookie.
    let session_id = state.local_access.create_session();
    let https = is_https(&headers);
    let cookie_value = session_cookie(&session_id, https);

    audit(
        &state,
        AuthAuditEvent::new(actor, AuthAuditAction::Login, route_label, AuthOutcome::Success),
    );

    (StatusCode::NO_CONTENT, [(SET_COOKIE, cookie_value)]).into_response()
}

/// `DELETE /api/auth/session`
///
/// Revokes the session identified by the `roko_session` cookie and clears the
/// cookie on the client (`Max-Age=0`).
///
/// Always returns **204 No Content** and always sets the clearing cookie,
/// even when no session cookie was present or the session was not found.
async fn delete_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    let route_label = "DELETE /api/auth/session";

    let session_id = extract_session_cookie(&headers);

    if let Some(sid) = session_id {
        let removed = state.local_access.end_session(sid);
        let outcome = if removed {
            AuthOutcome::Success
        } else {
            AuthOutcome::Denied
        };
        audit(
            &state,
            AuthAuditEvent::new("session", AuthAuditAction::TokenRevoked, route_label, outcome),
        );
    } else {
        // No cookie present — still clear and audit as denied (nothing to revoke).
        audit(
            &state,
            AuthAuditEvent::new(
                "anonymous",
                AuthAuditAction::TokenRevoked,
                route_label,
                AuthOutcome::Denied,
            ),
        );
    }

    // Always clear the cookie so a stale/invalid cookie is removed from the
    // browser regardless of whether the server found a live session.
    (
        StatusCode::NO_CONTENT,
        [(SET_COOKIE, CLEAR_SESSION_COOKIE)],
    )
        .into_response()
}

/// Assemble the session auth routes.
///
/// These routes must be mounted **outside** the `require_api_key` middleware
/// layer — mounting them inside would prevent browsers from ever obtaining
/// a credential.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/api/auth/session", post(create_session).delete(delete_session))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use roko_core::config::RokoConfig;
    use tempfile::tempdir;
    use tower::ServiceExt as _;

    use crate::deploy::create_backend;
    use crate::runtime::NoOpRuntime;
    use crate::state::AppState;

    fn build_test_router(config: RokoConfig) -> (tempfile::TempDir, axum::Router) {
        let dir = tempdir().expect("tempdir");
        let deploy =
            Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
        let state = Arc::new(
            AppState::new(
                dir.path().to_path_buf(),
                Arc::new(NoOpRuntime),
                config.clone(),
                deploy,
            )
            .expect("AppState::new"),
        );
        let router = routes().with_state(Arc::clone(&state));
        (dir, router)
    }

    fn build_test_state_router(config: RokoConfig) -> (tempfile::TempDir, Arc<AppState>, axum::Router) {
        let dir = tempdir().expect("tempdir");
        let deploy =
            Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
        let state = Arc::new(
            AppState::new(
                dir.path().to_path_buf(),
                Arc::new(NoOpRuntime),
                config,
                deploy,
            )
            .expect("AppState::new"),
        );
        let router = routes().with_state(Arc::clone(&state));
        (dir, state, router)
    }

    /// Parse the session id from a `Set-Cookie: roko_session=<id>; ...` header.
    fn extract_set_cookie_session(headers: &axum::http::HeaderMap) -> Option<String> {
        let value = headers.get("set-cookie")?.to_str().ok()?;
        let part = value
            .split(';')
            .next()?
            .trim()
            .strip_prefix("roko_session=")?
            .to_string();
        if part.is_empty() { None } else { Some(part) }
    }

    // ─── POST /api/auth/session ───────────────────────────────────────────────

    #[tokio::test]
    async fn post_session_with_valid_api_key_returns_204_and_cookie() {
        let plaintext = "test-api-key";
        let mut config = RokoConfig::default();
        config.serve.auth.enabled = true;
        config.serve.auth.api_key = plaintext.to_string();
        let (_dir, app) = build_test_router(config);

        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/auth/session")
            .header("X-Api-Key", plaintext)
            .body(Body::empty())
            .expect("build request");
        let resp = app.oneshot(req).await.expect("response");

        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        let session = extract_set_cookie_session(resp.headers());
        assert!(session.is_some(), "Set-Cookie with session id expected");
        let cookie = resp.headers()["set-cookie"].to_str().unwrap();
        assert!(cookie.contains("HttpOnly"), "HttpOnly required");
        assert!(cookie.contains("SameSite=Strict"), "SameSite=Strict required");
        assert!(cookie.contains("Path=/"), "Path=/ required");
    }

    #[tokio::test]
    async fn post_session_with_api_key_in_json_body_returns_204() {
        let plaintext = "json-body-key";
        let mut config = RokoConfig::default();
        config.serve.auth.enabled = true;
        config.serve.auth.api_key = plaintext.to_string();
        let (_dir, app) = build_test_router(config);

        let body = serde_json::json!({ "token": plaintext });
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/auth/session")
            .header("Content-Type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("build request");
        let resp = app.oneshot(req).await.expect("response");

        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        assert!(extract_set_cookie_session(resp.headers()).is_some());
    }

    #[tokio::test]
    async fn post_session_with_bearer_token_returns_204() {
        let plaintext = "bearer-key";
        let mut config = RokoConfig::default();
        config.serve.auth.enabled = true;
        config.serve.auth.api_key = plaintext.to_string();
        let (_dir, app) = build_test_router(config);

        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/auth/session")
            .header(AUTHORIZATION, format!("Bearer {plaintext}"))
            .body(Body::empty())
            .expect("build request");
        let resp = app.oneshot(req).await.expect("response");

        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        assert!(extract_set_cookie_session(resp.headers()).is_some());
    }

    #[tokio::test]
    async fn post_session_with_invalid_token_returns_401() {
        let mut config = RokoConfig::default();
        config.serve.auth.enabled = true;
        config.serve.auth.api_key = "real-key".to_string();
        let (_dir, app) = build_test_router(config);

        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/auth/session")
            .header("X-Api-Key", "wrong-key")
            .body(Body::empty())
            .expect("build request");
        let resp = app.oneshot(req).await.expect("response");

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        assert!(resp.headers().get("set-cookie").is_none(), "no cookie on denial");
    }

    #[tokio::test]
    async fn post_session_with_no_credential_returns_401() {
        let mut config = RokoConfig::default();
        config.serve.auth.enabled = true;
        config.serve.auth.api_key = "some-key".to_string();
        let (_dir, app) = build_test_router(config);

        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/auth/session")
            .body(Body::empty())
            .expect("build request");
        let resp = app.oneshot(req).await.expect("response");

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn post_session_https_forwarded_proto_sets_secure_flag() {
        let plaintext = "https-key";
        let mut config = RokoConfig::default();
        config.serve.auth.enabled = true;
        config.serve.auth.api_key = plaintext.to_string();
        let (_dir, app) = build_test_router(config);

        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/auth/session")
            .header("X-Api-Key", plaintext)
            .header("X-Forwarded-Proto", "https")
            .body(Body::empty())
            .expect("build request");
        let resp = app.oneshot(req).await.expect("response");

        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        let cookie = resp.headers()["set-cookie"].to_str().unwrap();
        assert!(cookie.contains("; Secure"), "Secure flag required for HTTPS");
    }

    #[tokio::test]
    async fn post_session_http_no_secure_flag() {
        let plaintext = "http-key";
        let mut config = RokoConfig::default();
        config.serve.auth.enabled = true;
        config.serve.auth.api_key = plaintext.to_string();
        let (_dir, app) = build_test_router(config);

        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/auth/session")
            .header("X-Api-Key", plaintext)
            .body(Body::empty())
            .expect("build request");
        let resp = app.oneshot(req).await.expect("response");

        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        let cookie = resp.headers()["set-cookie"].to_str().unwrap();
        assert!(!cookie.contains("Secure"), "no Secure flag for HTTP");
    }

    // ─── DELETE /api/auth/session ─────────────────────────────────────────────

    #[tokio::test]
    async fn delete_session_clears_cookie_regardless_of_presence() {
        let mut config = RokoConfig::default();
        config.serve.auth.enabled = true;
        config.serve.auth.api_key = "key".to_string();
        let (_dir, app) = build_test_router(config);

        let req = Request::builder()
            .method(Method::DELETE)
            .uri("/api/auth/session")
            .body(Body::empty())
            .expect("build request");
        let resp = app.oneshot(req).await.expect("response");

        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        let cookie = resp.headers()["set-cookie"].to_str().unwrap();
        assert!(cookie.contains("Max-Age=0"), "Max-Age=0 clears the cookie");
        assert!(cookie.contains("roko_session=;"), "empty value clears the cookie");
    }

    #[tokio::test]
    async fn delete_session_revokes_live_session() {
        let plaintext = "revoke-key";
        let mut config = RokoConfig::default();
        config.serve.auth.enabled = true;
        config.serve.auth.api_key = plaintext.to_string();
        let (_dir, state, app) = build_test_state_router(config);

        // Mint a session first.
        let create_req = Request::builder()
            .method(Method::POST)
            .uri("/api/auth/session")
            .header("X-Api-Key", plaintext)
            .body(Body::empty())
            .expect("create request");
        let create_resp = app.clone().oneshot(create_req).await.expect("create response");
        assert_eq!(create_resp.status(), StatusCode::NO_CONTENT);
        let session_id = extract_set_cookie_session(create_resp.headers())
            .expect("session id from create");

        // Session should be live.
        assert!(state.local_access.session_valid(&session_id));

        // Delete it.
        let delete_req = Request::builder()
            .method(Method::DELETE)
            .uri("/api/auth/session")
            .header("Cookie", format!("roko_session={session_id}"))
            .body(Body::empty())
            .expect("delete request");
        let delete_resp = app.oneshot(delete_req).await.expect("delete response");
        assert_eq!(delete_resp.status(), StatusCode::NO_CONTENT);

        // Session must be gone.
        assert!(
            !state.local_access.session_valid(&session_id),
            "session must be revoked after DELETE"
        );
    }
}
