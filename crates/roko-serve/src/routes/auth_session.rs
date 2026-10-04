//! Browser session minting and teardown.
//!
//! These routes are mounted **outside** the `require_api_key` middleware layer
//! — they are how a browser exchanges a launch token (or API key) for a
//! `roko_session` cookie. They still run inside the global rate limiter.
//!
//! In showcase mode (`[showcase] enabled`) the same endpoint takes only a
//! passphrase login (S11 §4.3): token exchange is refused, every request carries
//! `X-Roko-CSRF: 1` from the exact public origin, and the session it mints has
//! the narrow `showcase` scope and the configured cookie name.
//!
//! ## Routes
//! - `GET    /api/auth/session` — whether the caller holds a live session
//! - `POST   /api/auth/session` — create a session cookie
//! - `DELETE /api/auth/session` — revoke the current session cookie

use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::{FromRequest as _, Request, State};
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE, RETRY_AFTER, SET_COOKIE};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use chrono::Utc;
use roko_core::config::schema::RokoConfig;
use roko_core::config::{ApiKeyEntry, ServeAuthConfig};
use serde::Deserialize;
use serde_json::json;

use crate::auth_audit::{AuthAuditAction, AuthAuditEvent, AuthOutcome};
use crate::error::ApiError;
use crate::routes::auth::parse_rfc3339;
use crate::routes::middleware::{
    constant_time_eq, extract_bearer_token, extract_named_cookie, hash_api_key,
};
use crate::showcase::auth::{
    check_csrf_and_origin, clear_session_cookie, client_ip, ip_prefix, session_cookie_name,
    set_session_cookie,
};
use crate::state::{AppState, SessionGrant, SessionLookup};

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
const CLEAR_SESSION_COOKIE: &str = "roko_session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0";

/// Append an event to the auth audit log (best-effort; errors are swallowed).
fn audit(state: &AppState, event: AuthAuditEvent) {
    if let Some(log) = state.auth_audit.as_ref() {
        log.append(&event);
    }
}

/// The JSON body of a showcase login (S11 §4.3).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PassphraseLogin {
    passphrase: String,
}

/// The largest showcase login body, in bytes (S11 §4.3).
const LOGIN_BODY_LIMIT: usize = 1024;

/// The passphrase lengths, in bytes, a showcase login takes (S11 §4.3).
const PASSPHRASE_BYTES: std::ops::RangeInclusive<usize> = 12..=256;

/// `{"error": code}` with `status`: the showcase login's uniform errors.
fn showcase_error(status: StatusCode, code: &str) -> Response {
    (status, Json(json!({ "error": code }))).into_response()
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
            if prev_hash == &token_hash && parse_rfc3339(grace_expires).is_some_and(|e| e > now) {
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
/// On failure returns **401 Unauthorized** without setting a cookie. In showcase
/// mode the request is a passphrase login instead ([`passphrase_login`]).
async fn create_session(State(state): State<Arc<AppState>>, req: Request) -> Response {
    let route_label = "POST /api/auth/session";
    let config = state.load_roko_config();
    if config.showcase.enabled {
        return passphrase_login(&state, &config, req).await;
    }
    let headers = req.headers().clone();
    let body = match Option::<Json<CreateSessionRequest>>::from_request(req, &state).await {
        Ok(body) => body,
        Err(rejection) => return rejection.into_response(),
    };

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
        AuthAuditEvent::new(
            actor,
            AuthAuditAction::Login,
            route_label,
            AuthOutcome::Success,
        ),
    );

    (StatusCode::NO_CONTENT, [(SET_COOKIE, cookie_value)]).into_response()
}

/// `POST /api/auth/session` in showcase mode: a passphrase login (S11 §4.3).
///
/// The request carries `X-Roko-CSRF: 1` from the exact public origin and a JSON
/// body of at most 1 KiB, `{"passphrase": "…"}`; token exchange is refused. The
/// passphrase is checked with Argon2id against `ROKO_SHOWCASE_PASSPHRASE_HASH`
/// behind a bounded queue (`429 login_busy` past it). Success mints a `showcase`
/// session under the passphrase generation; every failure is the same
/// `401 invalid_passphrase`, and nothing echoes the passphrase.
async fn passphrase_login(state: &AppState, config: &RokoConfig, req: Request) -> Response {
    let route_label = "POST /api/auth/session";
    let showcase = &config.showcase;
    let prefix = ip_prefix(client_ip(&req, showcase.login.trust_fly_client_ip));
    let headers = req.headers();
    if let Err(code) = check_csrf_and_origin(headers, showcase.public_origin.as_deref()) {
        return showcase_error(StatusCode::FORBIDDEN, code);
    }
    if headers.contains_key("X-Api-Key") || headers.contains_key(AUTHORIZATION) {
        return showcase_error(StatusCode::BAD_REQUEST, "token_exchange_disabled");
    }
    let json_body = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/json"));
    if !json_body {
        return showcase_error(StatusCode::UNSUPPORTED_MEDIA_TYPE, "json_required");
    }
    let Ok(body) = axum::body::to_bytes(req.into_body(), LOGIN_BODY_LIMIT).await else {
        return showcase_error(StatusCode::PAYLOAD_TOO_LARGE, "body_too_large");
    };
    let Ok(login) = serde_json::from_slice::<PassphraseLogin>(&body) else {
        return showcase_error(StatusCode::BAD_REQUEST, "invalid_request");
    };

    let hash = state
        .local_access
        .passphrase_hash()
        .filter(|_| PASSPHRASE_BYTES.contains(&login.passphrase.len()));
    let verified = match hash {
        Some(hash) => {
            let concurrency = showcase.login.verify_concurrency;
            let Some(admission) = state.local_access.login_verifier(concurrency).admit() else {
                return (
                    StatusCode::TOO_MANY_REQUESTS,
                    [(RETRY_AFTER, "1")],
                    Json(json!({ "error": "login_busy" })),
                )
                    .into_response();
            };
            admission.verify(&hash, &login.passphrase).await
        }
        None => false,
    };
    if !verified {
        audit(
            state,
            AuthAuditEvent::new(
                "showcase",
                AuthAuditAction::LoginFailed,
                route_label,
                AuthOutcome::Denied,
            )
            .with_ip(Some(prefix)),
        );
        return showcase_error(StatusCode::UNAUTHORIZED, "invalid_passphrase");
    }

    let generation = state.local_access.passphrase_generation();
    let grant = SessionGrant::showcase(&showcase.session, generation);
    let access = &state.local_access;
    let session_id = access.create_scoped_session(&grant, Utc::now());
    audit(
        state,
        AuthAuditEvent::new(
            "showcase",
            AuthAuditAction::LoginSucceeded,
            route_label,
            AuthOutcome::Success,
        )
        .with_ip(Some(prefix)),
    );
    let cookie = set_session_cookie(&showcase.session, &session_id);
    (StatusCode::NO_CONTENT, [(SET_COOKIE, cookie)]).into_response()
}

/// `GET /api/auth/session` (public): whether the caller holds a live session.
///
/// Returns `{"authenticated", "login", "showcase_mode", "scopes", "expires_at"}`;
/// `login` is `passphrase` in showcase mode and `token` otherwise. The SPA asks
/// after a stream error, to decide whether to show the login page.
async fn session_probe(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let config = state.load_roko_config();
    let cookie = session_cookie_name(&config);
    let access = &state.local_access;
    let lookup = extract_named_cookie(&headers, cookie)
        .map(|session_id| access.authenticate_session(session_id, Utc::now()));
    let (scopes, expires_at) = match lookup {
        Some(SessionLookup::Live { scope, expires_at }) => (vec![scope], expires_at),
        _ => (Vec::new(), None),
    };
    let showcase_mode = config.showcase.enabled;
    Json(json!({
        "authenticated": !scopes.is_empty(),
        "login": if showcase_mode { "passphrase" } else { "token" },
        "showcase_mode": showcase_mode,
        "scopes": scopes,
        "expires_at": expires_at.map(|at| at.to_rfc3339()),
    }))
    .into_response()
}

/// `DELETE /api/auth/session`
///
/// Revokes the session identified by the session cookie and clears the
/// cookie on the client (`Max-Age=0`).
///
/// Always returns **204 No Content** and always sets the clearing cookie,
/// even when no session cookie was present or the session was not found.
/// In showcase mode the request must carry `X-Roko-CSRF: 1` from the exact
/// public origin, or it is refused with **403**.
async fn delete_session(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let route_label = "DELETE /api/auth/session";
    let config = state.load_roko_config();
    let showcase = &config.showcase;
    if showcase.enabled
        && let Err(code) = check_csrf_and_origin(&headers, showcase.public_origin.as_deref())
    {
        return showcase_error(StatusCode::FORBIDDEN, code);
    }
    let action = if showcase.enabled {
        AuthAuditAction::SessionRevoked
    } else {
        AuthAuditAction::TokenRevoked
    };

    let session_id = extract_named_cookie(&headers, session_cookie_name(&config));

    if let Some(sid) = session_id {
        let removed = state.local_access.end_session(sid);
        let outcome = if removed {
            AuthOutcome::Success
        } else {
            AuthOutcome::Denied
        };
        audit(
            &state,
            AuthAuditEvent::new("session", action, route_label, outcome),
        );
    } else {
        // No cookie present — still clear and audit as denied (nothing to revoke).
        audit(
            &state,
            AuthAuditEvent::new("anonymous", action, route_label, AuthOutcome::Denied),
        );
    }

    // Always clear the cookie so a stale/invalid cookie is removed from the
    // browser regardless of whether the server found a live session.
    let clear = if showcase.enabled {
        clear_session_cookie(&showcase.session)
    } else {
        CLEAR_SESSION_COOKIE.to_string()
    };
    (StatusCode::NO_CONTENT, [(SET_COOKIE, clear)]).into_response()
}

/// Assemble the session auth routes.
///
/// These routes must be mounted **outside** the `require_api_key` middleware
/// layer — mounting them inside would prevent browsers from ever obtaining
/// a credential.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route(
        "/api/auth/session",
        get(session_probe)
            .post(create_session)
            .delete(delete_session),
    )
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
        let deploy = Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
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

    fn build_test_state_router(
        config: RokoConfig,
    ) -> (tempfile::TempDir, Arc<AppState>, axum::Router) {
        let dir = tempdir().expect("tempdir");
        let deploy = Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
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
        assert!(
            cookie.contains("SameSite=Strict"),
            "SameSite=Strict required"
        );
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
        assert!(
            resp.headers().get("set-cookie").is_none(),
            "no cookie on denial"
        );
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
        assert!(
            cookie.contains("; Secure"),
            "Secure flag required for HTTPS"
        );
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

    // ─── Showcase mode: passphrase login (9323) ──────────────────────────────

    const PASSPHRASE: &str = "correct horse battery staple";
    const ORIGIN: &str = "https://showcase.test";

    /// An Argon2id PHC string of `passphrase` at the lowest cost, so the tests stay fast.
    fn cheap_phc(passphrase: &str) -> String {
        use argon2::password_hash::{PasswordHasher as _, SaltString};
        let params = argon2::Params::new(8, 1, 1, None).expect("cheap Argon2 parameters");
        let hasher =
            argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
        let salt = SaltString::encode_b64(b"showcase-test-salt").expect("salt");
        hasher
            .hash_password(passphrase.as_bytes(), &salt)
            .expect("hash the passphrase")
            .to_string()
    }

    /// A showcase-mode server whose passphrase is [`PASSPHRASE`].
    fn showcase_state_router() -> (tempfile::TempDir, Arc<AppState>, axum::Router) {
        let mut config = RokoConfig::default();
        config.serve.auth.enabled = true;
        config.showcase.enabled = true;
        config.showcase.public_origin = Some(ORIGIN.to_string());
        let (dir, state, router) = build_test_state_router(config);
        state
            .local_access
            .set_passphrase_hash(Some(cheap_phc(PASSPHRASE)));
        (dir, state, router)
    }

    /// A login as the SPA sends it: JSON, the CSRF header and the public origin.
    fn login(passphrase: &str) -> Request<Body> {
        Request::builder()
            .method(Method::POST)
            .uri("/api/auth/session")
            .header("Content-Type", "application/json")
            .header("X-Roko-CSRF", "1")
            .header("Origin", ORIGIN)
            .body(Body::from(json!({ "passphrase": passphrase }).to_string()))
            .expect("login request")
    }

    async fn body_text(resp: Response) -> String {
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("response body");
        String::from_utf8(bytes.to_vec()).expect("UTF-8 body")
    }

    /// The session id in a `Set-Cookie: __Host-roko_session=<id>; ...` header.
    fn showcase_session_id(headers: &axum::http::HeaderMap) -> Option<String> {
        let value = headers.get("set-cookie")?.to_str().ok()?;
        let id = value
            .split(';')
            .next()?
            .trim()
            .strip_prefix("__Host-roko_session=")?;
        (!id.is_empty()).then(|| id.to_string())
    }

    #[tokio::test]
    async fn passphrase_login_sets_a_showcase_session() {
        let (_dir, state, app) = showcase_state_router();

        let resp = app.clone().oneshot(login(PASSPHRASE)).await.expect("login");
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        let cookie = resp.headers()["set-cookie"]
            .to_str()
            .expect("cookie")
            .to_string();
        for attribute in [
            "HttpOnly",
            "SameSite=Strict",
            "Path=/",
            "Max-Age=259200",
            "Secure",
        ] {
            assert!(cookie.contains(attribute), "{attribute} missing from {cookie}");
        }
        let session_id = showcase_session_id(resp.headers()).expect("session cookie");
        let access = &state.local_access;
        let lookup = access.authenticate_session(&session_id, Utc::now());
        let SessionLookup::Live { scope, expires_at } = lookup else {
            panic!("no live session: {lookup:?}");
        };
        assert_eq!(scope, "showcase");
        assert!(expires_at.is_some());

        let probe = Request::builder()
            .uri("/api/auth/session")
            .header("Cookie", format!("__Host-roko_session={session_id}"))
            .body(Body::empty())
            .expect("probe request");
        let resp = app.clone().oneshot(probe).await.expect("probe");
        assert_eq!(resp.status(), StatusCode::OK);
        let probe: serde_json::Value =
            serde_json::from_str(&body_text(resp).await).expect("probe JSON");
        assert_eq!(probe["authenticated"], true);
        assert_eq!(probe["login"], "passphrase");
        assert_eq!(probe["showcase_mode"], true);
        assert_eq!(probe["scopes"], json!(["showcase"]));
        assert!(probe["expires_at"].is_string());

        let anonymous = Request::builder()
            .uri("/api/auth/session")
            .body(Body::empty())
            .expect("probe request");
        let resp = app.oneshot(anonymous).await.expect("probe");
        let probe: serde_json::Value =
            serde_json::from_str(&body_text(resp).await).expect("probe JSON");
        assert_eq!(probe["authenticated"], false);
    }

    #[tokio::test]
    async fn passphrase_login_requires_csrf_and_origin() {
        let (_dir, state, app) = showcase_state_router();
        let mut no_csrf = login(PASSPHRASE);
        no_csrf.headers_mut().remove("X-Roko-CSRF");
        let mut foreign = login(PASSPHRASE);
        foreign
            .headers_mut()
            .insert("Origin", "https://evil.test".parse().expect("origin"));
        let mut no_origin = login(PASSPHRASE);
        no_origin.headers_mut().remove("Origin");
        let cases = [
            (no_csrf, "csrf_required"),
            (foreign, "origin_mismatch"),
            (no_origin, "origin_mismatch"),
        ];
        for (req, code) in cases {
            let resp = app.clone().oneshot(req).await.expect("login");
            assert_eq!(resp.status(), StatusCode::FORBIDDEN, "{code}");
            assert!(resp.headers().get("set-cookie").is_none(), "{code}");
            assert_eq!(body_text(resp).await, json!({ "error": code }).to_string());
        }

        // Ending a session needs both too.
        let resp = app.clone().oneshot(login(PASSPHRASE)).await.expect("login");
        let session_id = showcase_session_id(resp.headers()).expect("session cookie");
        let end = |csrf: bool| {
            let mut builder = Request::builder()
                .method(Method::DELETE)
                .uri("/api/auth/session")
                .header("Cookie", format!("__Host-roko_session={session_id}"))
                .header("Origin", ORIGIN);
            if csrf {
                builder = builder.header("X-Roko-CSRF", "1");
            }
            builder.body(Body::empty()).expect("delete request")
        };
        let resp = app.clone().oneshot(end(false)).await.expect("delete");
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        assert!(state.local_access.session_valid(&session_id));
        let resp = app.oneshot(end(true)).await.expect("delete");
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        let cleared = resp.headers()["set-cookie"].to_str().expect("cookie");
        assert!(cleared.starts_with("__Host-roko_session=;"), "{cleared}");
        assert!(!state.local_access.session_valid(&session_id));
    }

    #[tokio::test]
    async fn passphrase_login_beyond_the_queue_is_429() {
        let (_dir, state, app) = showcase_state_router();
        let concurrency = RokoConfig::default().showcase.login.verify_concurrency;
        let verifier = state.local_access.login_verifier(concurrency);
        let held: Vec<_> = std::iter::from_fn(|| verifier.admit()).collect();
        assert!(!held.is_empty());

        let resp = app.clone().oneshot(login(PASSPHRASE)).await.expect("login");
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(resp.headers().get("set-cookie").is_none());
        assert_eq!(
            body_text(resp).await,
            json!({ "error": "login_busy" }).to_string()
        );

        drop(held);
        let resp = app.oneshot(login(PASSPHRASE)).await.expect("login");
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn passphrase_login_never_echoes_the_passphrase() {
        let (_dir, _state, app) = showcase_state_router();
        let wrong = "a wrong but long passphrase";
        for attempt in [wrong, "too short"] {
            let resp = app.clone().oneshot(login(attempt)).await.expect("login");
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "{attempt}");
            assert!(resp.headers().get("set-cookie").is_none());
            let body = body_text(resp).await;
            assert_eq!(body, json!({ "error": "invalid_passphrase" }).to_string());
            assert!(!body.contains(attempt));
        }

        // Token exchange is refused in showcase mode.
        let exchange = Request::builder()
            .method(Method::POST)
            .uri("/api/auth/session")
            .header("X-Api-Key", PASSPHRASE)
            .header("X-Roko-CSRF", "1")
            .header("Origin", ORIGIN)
            .body(Body::empty())
            .expect("exchange request");
        let resp = app.oneshot(exchange).await.expect("exchange");
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert!(resp.headers().get("set-cookie").is_none());
        assert!(!body_text(resp).await.contains(PASSPHRASE));
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
        assert!(
            cookie.contains("roko_session=;"),
            "empty value clears the cookie"
        );
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
        let create_resp = app
            .clone()
            .oneshot(create_req)
            .await
            .expect("create response");
        assert_eq!(create_resp.status(), StatusCode::NO_CONTENT);
        let session_id =
            extract_set_cookie_session(create_resp.headers()).expect("session id from create");

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
