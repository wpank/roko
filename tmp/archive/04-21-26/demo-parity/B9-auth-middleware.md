# B9: JWT Bearer token auth middleware

## Context

**Repo:** `/Users/will/dev/nunchi/roko/roko`
**Branch:** `demo-backend`
**Language:** Rust (workspace with ~29 crates)
**Key crate paths:**
- CLI + orchestrator: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/`
- Core types: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/`
- HTTP server: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/`
- Agent dispatch: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/`

**Key files:**
- Orchestrator (20K lines): `crates/roko-cli/src/orchestrate.rs`
- CLI entry: `crates/roko-cli/src/main.rs`
- Server routes: `crates/roko-serve/src/routes/mod.rs`
- Server state: `crates/roko-serve/src/state.rs`
- Server events: `crates/roko-serve/src/events.rs`
- Server WS: `crates/roko-serve/src/routes/ws.rs`

**Architecture:**
- `roko-serve` is an axum HTTP server on port 6677 with ~85 REST routes + WebSocket
- `AppState` uses `tokio::sync::RwLock` -- all lock ops are `.read().await` / `.write().await` (NOT `.unwrap()`)
- Event bus: `state.event_bus.publish(event)` -- always present, no Option wrapping
- The TUI gets data two ways: (1) StateHub push via `watch<DashboardSnapshot>` channel, (2) file polling via `DashboardData::tick()` reading `.roko/` files

### Pre-commit (MANDATORY)

```bash
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

## What this task does

Upgrade the existing auth middleware to accept `Authorization: Bearer <token>` in addition to the existing `X-Api-Key` header. For the demo, JWT validation is structural only (three dot-separated base64url segments). No cryptographic signature verification. The middleware also sets `X-Auth-Method` response header for debugging.

## Existing middleware

The auth middleware is at `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/middleware.rs`. The key function is `require_api_key`:

```rust
pub async fn require_api_key(
    State(auth): State<ServeAuthConfig>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let supplied = req
        .headers()
        .get("X-Api-Key")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    if supplied != auth.api_key {
        return Err(ApiError::unauthorized(
            "invalid or missing X-Api-Key header",
        ));
    }

    Ok(next.run(req).await)
}
```

The middleware is applied in `build_router()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/mod.rs` lines 69-76, only when `api_auth.enabled` is true.

`ServeAuthConfig` is defined in roko-core. Verify its fields:
```
grep -rn "pub struct ServeAuthConfig" crates/roko-core/ --include='*.rs'
```
Confirm it has `enabled: bool` and `api_key: String`.

## Steps

- [ ] **Read the full middleware file.** Read `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/middleware.rs` completely before making changes.

- [ ] **Check that `base64` is in roko-serve deps.** Look at `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/Cargo.toml`. `base64` is already there (line ~40). No need to add it.

- [ ] **Replace `require_api_key` with the new implementation:**

```rust
/// Require either a matching `X-Api-Key` header or a structurally valid
/// `Authorization: Bearer <JWT>` token.
///
/// Auth method is reported via the `X-Auth-Method` response header:
/// - `"api_key"` when the `X-Api-Key` path is taken.
/// - `"jwt"` when the Bearer path is taken.
/// This header is present only on successful authentication and is useful
/// for debugging which code path was exercised.
///
/// # Security note
///
/// JWT validation is **structural only**: three dot-separated base64url
/// segments, each non-empty. No cryptographic signature verification is
/// performed. For production use, replace this with proper JWT verification
/// using the `jsonwebtoken` crate.
pub async fn require_api_key(
    State(auth): State<ServeAuthConfig>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    // Path 1: X-Api-Key header (exact match against configured key).
    if let Some(api_key) = req
        .headers()
        .get("X-Api-Key")
        .and_then(|v| v.to_str().ok())
    {
        if api_key == auth.api_key {
            let mut resp = next.run(req).await;
            resp.headers_mut().insert(
                axum::http::HeaderName::from_static("x-auth-method"),
                axum::http::HeaderValue::from_static("api_key"),
            );
            return Ok(resp);
        }
        // Wrong key -- reject immediately; don't fall through to Bearer.
        return Err(ApiError::unauthorized("invalid X-Api-Key"));
    }

    // Path 2: Authorization: Bearer <token> (case-insensitive "Bearer " prefix).
    if let Some(auth_header) = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        let token = extract_bearer_token(auth_header).ok_or_else(|| {
            ApiError::unauthorized(
                "Authorization header must use Bearer scheme: 'Authorization: Bearer <token>'",
            )
        })?;

        if !is_structurally_valid_jwt(token) {
            return Err(ApiError::unauthorized(
                "invalid Bearer token: must be three dot-separated base64url segments",
            ));
        }

        let mut resp = next.run(req).await;
        resp.headers_mut().insert(
            axum::http::HeaderName::from_static("x-auth-method"),
            axum::http::HeaderValue::from_static("jwt"),
        );
        return Ok(resp);
    }

    Err(ApiError::unauthorized(
        "authentication required: supply 'X-Api-Key' header or 'Authorization: Bearer <jwt>'",
    ))
}

/// Extract the token from an `Authorization` header value, accepting both
/// `Bearer ` and `bearer ` prefixes (case-insensitive).
fn extract_bearer_token(header_value: &str) -> Option<&str> {
    // Compare the first 7 characters case-insensitively.
    if header_value.len() > 7 && header_value[..7].eq_ignore_ascii_case("bearer ") {
        let token = header_value[7..].trim();
        if token.is_empty() {
            None
        } else {
            Some(token)
        }
    } else {
        None
    }
}

/// Check that a token looks like a JWT: three dot-separated, non-empty,
/// base64url-encoded segments. Does NOT verify the signature.
fn is_structurally_valid_jwt(token: &str) -> bool {
    let parts: Vec<&str> = token.splitn(4, '.').collect();
    if parts.len() != 3 {
        return false;
    }
    parts.iter().all(|part| !part.is_empty() && is_base64url(part))
}

/// Check if a string contains only base64url characters (RFC 4648 §5).
///
/// Valid characters: `A-Z`, `a-z`, `0-9`, `-`, `_`.
/// The `=` padding character is also accepted to handle tokens that
/// include base64 padding.
fn is_base64url(s: &str) -> bool {
    s.bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'=')
}
```

- [ ] **Update tests.** In the existing `#[cfg(test)] mod tests` block in middleware.rs, add these tests. Keep all existing tests -- they must continue to pass.

```rust
    #[tokio::test]
    async fn api_key_still_works() {
        let auth = ServeAuthConfig {
            enabled: true,
            api_key: "test-key-123".into(),
        };

        let handler = || async { "ok" };
        let app = axum::Router::new()
            .route("/test", axum::routing::get(handler))
            .layer(axum::middleware::from_fn_with_state(auth, require_api_key));

        let req = axum::http::Request::builder()
            .uri("/test")
            .header("X-Api-Key", "test-key-123")
            .body(axum::body::Body::empty())
            .expect("request");

        let resp = tower::ServiceExt::oneshot(app, req).await.expect("response");
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        assert_eq!(
            resp.headers().get("x-auth-method").and_then(|v| v.to_str().ok()),
            Some("api_key"),
        );
    }

    #[tokio::test]
    async fn wrong_api_key_rejected() {
        let auth = ServeAuthConfig {
            enabled: true,
            api_key: "correct-key".into(),
        };

        let handler = || async { "ok" };
        let app = axum::Router::new()
            .route("/test", axum::routing::get(handler))
            .layer(axum::middleware::from_fn_with_state(auth, require_api_key));

        let req = axum::http::Request::builder()
            .uri("/test")
            .header("X-Api-Key", "wrong-key")
            .body(axum::body::Body::empty())
            .expect("request");

        let resp = tower::ServiceExt::oneshot(app, req).await.expect("response");
        assert_eq!(resp.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn bearer_token_accepted() {
        let auth = ServeAuthConfig {
            enabled: true,
            api_key: "unused".into(),
        };

        // Structurally valid JWT: three base64url segments.
        let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ0ZXN0In0.dGVzdHNpZw";

        let handler = || async { "ok" };
        let app = axum::Router::new()
            .route("/test", axum::routing::get(handler))
            .layer(axum::middleware::from_fn_with_state(auth, require_api_key));

        let req = axum::http::Request::builder()
            .uri("/test")
            .header("Authorization", format!("Bearer {jwt}"))
            .body(axum::body::Body::empty())
            .expect("request");

        let resp = tower::ServiceExt::oneshot(app, req).await.expect("response");
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        assert_eq!(
            resp.headers().get("x-auth-method").and_then(|v| v.to_str().ok()),
            Some("jwt"),
        );
    }

    #[tokio::test]
    async fn bearer_case_insensitive() {
        let auth = ServeAuthConfig {
            enabled: true,
            api_key: "unused".into(),
        };

        let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ0ZXN0In0.dGVzdHNpZw";
        let handler = || async { "ok" };
        let app = axum::Router::new()
            .route("/test", axum::routing::get(handler))
            .layer(axum::middleware::from_fn_with_state(auth, require_api_key));

        // Lowercase "bearer"
        let req = axum::http::Request::builder()
            .uri("/test")
            .header("Authorization", format!("bearer {jwt}"))
            .body(axum::body::Body::empty())
            .expect("request");

        let resp = tower::ServiceExt::oneshot(app, req).await.expect("response");
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn invalid_bearer_rejected() {
        let auth = ServeAuthConfig {
            enabled: true,
            api_key: "unused".into(),
        };

        let handler = || async { "ok" };
        let app = axum::Router::new()
            .route("/test", axum::routing::get(handler))
            .layer(axum::middleware::from_fn_with_state(auth, require_api_key));

        // Only two segments -- structurally invalid.
        let req = axum::http::Request::builder()
            .uri("/test")
            .header("Authorization", "Bearer header.payload")
            .body(axum::body::Body::empty())
            .expect("request");

        let resp = tower::ServiceExt::oneshot(app, req).await.expect("response");
        assert_eq!(resp.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn no_auth_header_rejected() {
        let auth = ServeAuthConfig {
            enabled: true,
            api_key: "test-key".into(),
        };

        let handler = || async { "ok" };
        let app = axum::Router::new()
            .route("/test", axum::routing::get(handler))
            .layer(axum::middleware::from_fn_with_state(auth, require_api_key));

        let req = axum::http::Request::builder()
            .uri("/test")
            .body(axum::body::Body::empty())
            .expect("request");

        let resp = tower::ServiceExt::oneshot(app, req).await.expect("response");
        assert_eq!(resp.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    // Unit tests for the helper functions.

    #[test]
    fn extract_bearer_token_standard() {
        assert_eq!(
            extract_bearer_token("Bearer abc.def.ghi"),
            Some("abc.def.ghi"),
        );
    }

    #[test]
    fn extract_bearer_token_lowercase() {
        assert_eq!(
            extract_bearer_token("bearer abc.def.ghi"),
            Some("abc.def.ghi"),
        );
    }

    #[test]
    fn extract_bearer_token_empty_returns_none() {
        assert_eq!(extract_bearer_token("Bearer "), None);
        assert_eq!(extract_bearer_token("Bearer"), None);
        assert_eq!(extract_bearer_token(""), None);
    }

    #[test]
    fn structurally_valid_jwt_passes() {
        assert!(is_structurally_valid_jwt(
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ0ZXN0In0.dGVzdHNpZw"
        ));
    }

    #[test]
    fn two_segment_token_fails() {
        assert!(!is_structurally_valid_jwt("header.payload"));
    }

    #[test]
    fn four_segment_token_fails() {
        // splitn(4) produces at most 4 parts, so "a.b.c.d" gives ["a","b","c","d"].
        assert!(!is_structurally_valid_jwt("a.b.c.d"));
    }

    #[test]
    fn empty_segment_fails() {
        assert!(!is_structurally_valid_jwt("a..c"));
    }

    #[test]
    fn non_base64url_chars_fail() {
        assert!(!is_structurally_valid_jwt("a+b.c/d.e f"));
    }

    #[test]
    fn base64url_charset() {
        // Valid: alphanumeric + - _ =
        assert!(is_base64url("abcABC012-_="));
        // Invalid: + / (standard base64 but not base64url)
        assert!(!is_base64url("abc+def"));
        assert!(!is_base64url("abc/def"));
        // Invalid: space
        assert!(!is_base64url("abc def"));
        // Empty string is considered valid (each segment check handles empty separately)
        assert!(is_base64url(""));
    }
```

- [ ] **Verify existing tests still pass.**
  ```bash
  cargo test -p roko-serve -- middleware --nocapture
  ```

## Verification

```bash
cd /Users/will/dev/nunchi/roko/roko

# Run middleware tests
cargo test -p roko-serve -- middleware --nocapture

# Run all serve tests
cargo test -p roko-serve 2>&1 | tail -30

# Clippy
cargo clippy -p roko-serve --no-deps -- -D warnings 2>&1 | head -20

# Format check
cargo +nightly fmt --all -- --check

# Manual test (start server with auth enabled in roko.toml):
#   [server.auth]
#   enabled = true
#   api_key = "test-key"
#
# API key (should succeed, X-Auth-Method: api_key):
#   curl -sv -H 'X-Api-Key: test-key' http://localhost:6677/api/status 2>&1 | grep -E 'HTTP/|x-auth'
#
# Bearer JWT (should succeed, X-Auth-Method: jwt):
#   curl -sv -H 'Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ0ZXN0In0.dGVzdA' \
#     http://localhost:6677/api/status 2>&1 | grep -E 'HTTP/|x-auth'
#
# lowercase bearer (should succeed):
#   curl -sv -H 'authorization: bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ0ZXN0In0.dGVzdA' \
#     http://localhost:6677/api/status 2>&1 | grep -E 'HTTP/|x-auth'
#
# No auth (should fail 401):
#   curl -sv http://localhost:6677/api/status 2>&1 | grep 'HTTP/'
```

Expected: existing API key path works (with `X-Auth-Method: api_key` header), Bearer JWT path works (with `X-Auth-Method: jwt` header), lowercase `bearer` prefix accepted, invalid tokens rejected, all tests pass.
