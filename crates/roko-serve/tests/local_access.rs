//! Integration tests for local-access authentication through the HTTP router.
//!
//! These tests prove that:
//! 1. With no credential, `GET /api/plans` and `GET /api/events` return 401.
//! 2. The launch token as `X-Api-Key`, and as `Authorization: Bearer`, returns 200.
//! 3. `POST /api/auth/session` with `{"token": <token>}` returns 204, with a
//!    `Set-Cookie` carrying `roko_session`, `HttpOnly`, `SameSite=Strict` and
//!    `Path=/`. A wrong token returns 401.
//! 4. With that cookie, `GET /api/plans` and `GET /api/events` return 200 (check
//!    the status only for the stream).
//! 5. With the cookie, a POST carrying `Origin: http://evil.example` is 403. The
//!    same POST with the server's own origin is not rejected by auth.
//! 6. After `DELETE /api/auth/session`, the cookie no longer authenticates.
//! 7. With a configured API key and no launch token, exchanging the key through
//!    `X-Api-Key` yields a working cookie.
//!
//! Auth is **always** enabled. A `NoOpRuntime` is used so no real agent is
//! dispatched.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use roko_core::config::ServeAuthConfig;
use roko_core::config::schema::RokoConfig;
use roko_serve::deploy::create_backend;
use roko_serve::plan_types::PlanSummaryDto;
use roko_serve::routes::build_router;
use roko_serve::runtime::{CliRuntime, DashboardInfo, RunResult, SessionStatusInfo};
use roko_serve::state::{AppState, LocalAccess};
use tempfile::tempdir;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// Minimal no-op runtime for integration tests
// ---------------------------------------------------------------------------

/// Minimal [`CliRuntime`] that never dispatches a real agent.
///
/// The three non-defaulted methods return the simplest valid values so that
/// auth-focused tests compile and run without touching the CLI or LLM layer.
struct MinimalRuntime;

#[async_trait::async_trait]
impl CliRuntime for MinimalRuntime {
    async fn run_once(&self, _workdir: &Path, _prompt: &str) -> anyhow::Result<RunResult> {
        Ok(RunResult {
            success: true,
            output_text: None,
            usage: None,
            gate_results: Vec::new(),
        })
    }

    fn session_status(&self, workdir: PathBuf) -> SessionStatusInfo {
        SessionStatusInfo {
            session_id: None,
            workdir,
            daemon_running: false,
            signal_count: None,
            episode_count: None,
            last_episode_passed: None,
        }
    }

    fn dashboard_scaffold(&self, _workdir: &Path) -> DashboardInfo {
        DashboardInfo {
            rendered: String::new(),
        }
    }

    async fn list_plans(&self, _workdir: &Path) -> anyhow::Result<Vec<PlanSummaryDto>> {
        Ok(Vec::new())
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Plaintext launch token used in all launch-token test cases.
const LAUNCH_TOKEN: &str = "test-launch-token-abc123";

/// Plaintext API key used in test case 7 (configured key, no launch token).
const CONFIGURED_API_KEY: &str = "my-configured-api-key-xyz";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build an `AppState` + axum `Router` with:
/// - Auth **enabled**
/// - A launch token set to [`LAUNCH_TOKEN`]
/// - No configured named API keys
///
/// Returns the `TempDir` guard (must be held alive for the test duration) and the router.
fn build_auth_router_with_launch_token() -> (tempfile::TempDir, Arc<AppState>, axum::Router) {
    let dir = tempdir().expect("tempdir");
    let workdir: PathBuf = dir.path().to_path_buf();

    let mut config = RokoConfig::default();
    config.serve.auth.enabled = true;

    let deploy = Arc::from(create_backend("manual", None, None, None).expect("manual backend"));

    let mut state = AppState::new(workdir, Arc::new(MinimalRuntime), config.clone(), deploy)
        .expect("AppState::new");

    // Set the launch token (AppState::new always creates LocalAccess::new(None)).
    state.local_access = LocalAccess::new(Some(LAUNCH_TOKEN.to_string()));

    let state = Arc::new(state);

    let router = build_router(
        Arc::clone(&state),
        &[],
        ServeAuthConfig {
            enabled: true,
            ..ServeAuthConfig::default()
        },
    );

    (dir, state, router)
}

/// Build an `AppState` + axum `Router` with:
/// - Auth **enabled**
/// - No launch token
/// - The legacy `api_key` set to [`CONFIGURED_API_KEY`]
fn build_auth_router_with_api_key() -> (tempfile::TempDir, Arc<AppState>, axum::Router) {
    let dir = tempdir().expect("tempdir");
    let workdir: PathBuf = dir.path().to_path_buf();

    let mut config = RokoConfig::default();
    config.serve.auth.enabled = true;
    config.serve.auth.api_key = CONFIGURED_API_KEY.to_string();

    let deploy = Arc::from(create_backend("manual", None, None, None).expect("manual backend"));

    let state = Arc::new(
        AppState::new(workdir, Arc::new(MinimalRuntime), config.clone(), deploy)
            .expect("AppState::new"),
    );

    let router = build_router(
        Arc::clone(&state),
        &[],
        ServeAuthConfig {
            enabled: true,
            api_key: CONFIGURED_API_KEY.to_string(),
            ..ServeAuthConfig::default()
        },
    );

    (dir, state, router)
}

/// Parse the `roko_session` value out of a `Set-Cookie` header.
///
/// Returns `None` when the header is absent or the session value is empty.
fn extract_session_from_set_cookie(headers: &axum::http::HeaderMap) -> Option<String> {
    let value = headers.get("set-cookie")?.to_str().ok()?;
    let raw = value
        .split(';')
        .next()?
        .trim()
        .strip_prefix("roko_session=")?
        .to_string();
    if raw.is_empty() { None } else { Some(raw) }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// 1a. Without any credential `GET /api/plans` returns 401.
#[tokio::test]
async fn no_credential_get_plans_returns_401() {
    let (_dir, _state, router) = build_auth_router_with_launch_token();

    let resp = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/plans")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");

    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "GET /api/plans with no credential must return 401"
    );
}

/// 1b. Without any credential `GET /api/events` returns 401.
#[tokio::test]
async fn no_credential_get_events_returns_401() {
    let (_dir, _state, router) = build_auth_router_with_launch_token();

    let resp = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/events")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");

    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "GET /api/events with no credential must return 401"
    );
}

/// 2a. The launch token presented as `X-Api-Key` gives 200 on `GET /api/plans`.
#[tokio::test]
async fn launch_token_via_x_api_key_returns_200() {
    let (_dir, _state, router) = build_auth_router_with_launch_token();

    let resp = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/plans")
                .header("X-Api-Key", LAUNCH_TOKEN)
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "GET /api/plans with launch token via X-Api-Key must return 200"
    );
}

/// 2b. The launch token presented as `Authorization: Bearer` gives 200 on `GET /api/plans`.
#[tokio::test]
async fn launch_token_via_authorization_bearer_returns_200() {
    let (_dir, _state, router) = build_auth_router_with_launch_token();

    let resp = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/plans")
                .header("Authorization", format!("Bearer {LAUNCH_TOKEN}"))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "GET /api/plans with launch token via Authorization: Bearer must return 200"
    );
}

/// 3a. `POST /api/auth/session` with the correct launch token returns 204 and a
///     properly-formed session cookie.
#[tokio::test]
async fn post_session_with_correct_token_returns_204_and_cookie() {
    let (_dir, _state, router) = build_auth_router_with_launch_token();

    let body = serde_json::json!({ "token": LAUNCH_TOKEN });
    let resp = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/session")
                .header("Content-Type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("build request"),
        )
        .await
        .expect("oneshot");

    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "POST /api/auth/session with correct token must return 204"
    );

    // Verify the Set-Cookie header is present with the expected attributes.
    let cookie_header = resp
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .expect("Set-Cookie header must be present");

    assert!(
        cookie_header.contains("roko_session="),
        "Set-Cookie must carry roko_session: {cookie_header}"
    );
    assert!(
        cookie_header.contains("HttpOnly"),
        "Set-Cookie must be HttpOnly: {cookie_header}"
    );
    assert!(
        cookie_header.contains("SameSite=Strict"),
        "Set-Cookie must be SameSite=Strict: {cookie_header}"
    );
    assert!(
        cookie_header.contains("Path=/"),
        "Set-Cookie must have Path=/: {cookie_header}"
    );

    // The session value must not be empty.
    assert!(
        extract_session_from_set_cookie(resp.headers()).is_some(),
        "Set-Cookie must carry a non-empty session id"
    );
}

/// 3b. `POST /api/auth/session` with a wrong token returns 401 and no cookie.
#[tokio::test]
async fn post_session_with_wrong_token_returns_401() {
    let (_dir, _state, router) = build_auth_router_with_launch_token();

    let body = serde_json::json!({ "token": "definitely-wrong-token" });
    let resp = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/session")
                .header("Content-Type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("build request"),
        )
        .await
        .expect("oneshot");

    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "POST /api/auth/session with wrong token must return 401"
    );
    assert!(
        resp.headers().get("set-cookie").is_none(),
        "No Set-Cookie must be set on denial"
    );
}

/// 4a. A session cookie obtained from the launch token authenticates `GET /api/plans`.
#[tokio::test]
async fn session_cookie_authenticates_get_plans() {
    let (_dir, state, router) = build_auth_router_with_launch_token();

    // Mint a session via the state directly (avoids a two-request roundtrip in a
    // oneshot-only test harness; the auth_session module's own tests cover the
    // HTTP minting path thoroughly).
    let session_id = state.local_access.create_session();

    let resp = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/plans")
                .header("Cookie", format!("roko_session={session_id}"))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "GET /api/plans with valid session cookie must return 200"
    );
}

/// 4b. A session cookie authenticates `GET /api/events` (check status only — it
///     is an SSE stream so the body is never fully consumed in the test).
#[tokio::test]
async fn session_cookie_authenticates_get_events() {
    let (_dir, state, router) = build_auth_router_with_launch_token();

    let session_id = state.local_access.create_session();

    let resp = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/events")
                .header("Cookie", format!("roko_session={session_id}"))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "GET /api/events with valid session cookie must return 200"
    );
}

/// 5a. A POST from a cross-origin (`Origin: http://evil.example`) with a valid
///     session cookie returns 403 — the same-origin guard fires before any
///     handler logic runs.
#[tokio::test]
async fn session_cookie_cross_origin_post_returns_403() {
    let (_dir, state, router) = build_auth_router_with_launch_token();

    let session_id = state.local_access.create_session();

    // POST /api/plans is an authenticated, state-changing endpoint. The
    // same-origin guard checks the Origin vs Host headers for cookie auth.
    let resp = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans")
                .header("Cookie", format!("roko_session={session_id}"))
                .header("Origin", "http://evil.example")
                .header("Host", "localhost:6677")
                .header("Content-Type", "application/json")
                .body(Body::from("{}"))
                .expect("build request"),
        )
        .await
        .expect("oneshot");

    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "POST from a foreign origin with session cookie must return 403"
    );
}

/// 5b. The same POST with a matching origin (`Origin: http://localhost:6677`) is
///     **not** rejected by the auth layer (the response may be 400/422 from
///     request validation, but must not be 403 from the origin guard).
#[tokio::test]
async fn session_cookie_same_origin_post_not_forbidden() {
    let (_dir, state, router) = build_auth_router_with_launch_token();

    let session_id = state.local_access.create_session();

    let resp = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans")
                .header("Cookie", format!("roko_session={session_id}"))
                .header("Origin", "http://localhost:6677")
                .header("Host", "localhost:6677")
                .header("Content-Type", "application/json")
                .body(Body::from("{}"))
                .expect("build request"),
        )
        .await
        .expect("oneshot");

    assert_ne!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "POST from the server's own origin must not be rejected by the auth layer (got {})",
        resp.status()
    );
}

/// 6. After `DELETE /api/auth/session` the cookie no longer authenticates.
#[tokio::test]
async fn delete_session_revokes_cookie() {
    let (_dir, state, router) = build_auth_router_with_launch_token();

    let session_id = state.local_access.create_session();

    // Verify the session is currently live.
    assert!(
        state.local_access.session_valid(&session_id),
        "session must be valid before DELETE"
    );

    // DELETE the session via the HTTP handler.
    let delete_resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/auth/session")
                .header("Cookie", format!("roko_session={session_id}"))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");

    assert_eq!(
        delete_resp.status(),
        StatusCode::NO_CONTENT,
        "DELETE /api/auth/session must return 204"
    );

    // The session must be gone from the state.
    assert!(
        !state.local_access.session_valid(&session_id),
        "session must be invalid after DELETE"
    );

    // A subsequent authenticated request with the old cookie must return 401.
    let get_resp = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/plans")
                .header("Cookie", format!("roko_session={session_id}"))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");

    assert_eq!(
        get_resp.status(),
        StatusCode::UNAUTHORIZED,
        "GET /api/plans with revoked session cookie must return 401"
    );
}

/// 7. With a configured API key and no launch token, presenting the key via
///    `X-Api-Key` on `POST /api/auth/session` mints a working cookie.
#[tokio::test]
async fn configured_api_key_yields_working_session_cookie() {
    let (_dir, state, router) = build_auth_router_with_api_key();

    // Exchange the API key for a session cookie.
    let create_resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/session")
                .header("X-Api-Key", CONFIGURED_API_KEY)
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");

    assert_eq!(
        create_resp.status(),
        StatusCode::NO_CONTENT,
        "POST /api/auth/session with configured API key must return 204"
    );

    let session_id = extract_session_from_set_cookie(create_resp.headers())
        .expect("Set-Cookie with session id must be present");

    // The session must be live in state.
    assert!(
        state.local_access.session_valid(&session_id),
        "session must be valid in state after creation"
    );

    // The session cookie must authenticate a subsequent request.
    let get_resp = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/plans")
                .header("Cookie", format!("roko_session={session_id}"))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");

    assert_eq!(
        get_resp.status(),
        StatusCode::OK,
        "GET /api/plans with cookie minted from configured API key must return 200"
    );
}
