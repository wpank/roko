# 13 — Tier 3: Security Hardening (7 items, ~1 done)

These are required before any non-loopback deployment. They are
deployment-blocking. ~2 sessions.

**Source**: doc 41 backlog T3-22..T3-28, doc 38 (serve routes security), doc
36 (deep audit ACP/terminal/safety).

---

## Cross-Cutting Notes

The roko-serve binary already has a **`validate_bind_safety`** helper
(`crates/roko-serve/src/lib.rs:641`). It refuses to start when the bind is
non-loopback **unless** auth is enabled or `serve.acknowledge_public_risk =
true`. This is the correct fail-closed posture for public binds.

What's still wrong:

- Default `auth.enabled = false` (T3-22). Defensible because it's overridden
  by `validate_bind_safety` for public binds, but explicit-default-true is
  still the audit's recommendation.
- `PORT` env var coercively binds `0.0.0.0:$PORT` (T3-25). Currently all
  cloud platforms hit this path, so it's deployment-relevant, not just
  local-dev.
- No rate limiting (T3-23), body limits are 32 MiB global / no per-endpoint
  (T3-24), no WebSocket message size caps (T3-26).
- Agent manifest TOML is `format!`-built with `toml_quote()` (T3-27); good
  enough for trusted input but still string-interpolated.
- CORS is `allow_methods(Any).allow_headers(Any)` even for the
  cors-origins-restricted path (T3-28).

### Anti-patterns to enforce

1. **Don't disable `validate_bind_safety`.** It is the chokepoint that
   prevents accidental public binds without auth. New entry points must
   call it.
2. **Don't add a route that bypasses auth without explicit reason.** If a
   route must be public (health probe, HMAC-verified webhook), document
   why in a doc-comment.
3. **Don't lower a stricter limit to make a test pass.** The test should
   fit within the limit, or use a fixture below it.
4. **Don't `allow_origin(Any)` outside `unsafe_public_cors = true`.**
   Permissive CORS is opt-in only.
5. **Don't use raw `format!` for any wire protocol.** TOML, JSON, SSE
   payloads use typed serializers.

### Reference: existing security middleware in `roko-serve`

- `routes/mod.rs:80, 183` — top-level CORS layer + 32 MiB body limit
- `routes/middleware.rs:require_api_key` — header-based API key check
- `routes/middleware.rs:require_scope` — scoped permission check
- `routes/middleware.rs:scrub_secrets` — response-body secret redaction
- `routes/middleware.rs:cors_layer` — origin-restricted by default;
  permissive only when `unsafe_public_cors = true`
- `lib.rs:validate_bind_safety` — pre-bind public-vs-loopback check

---

## [ ] T3-22: Flip `auth.enabled` default to enabled

**Why**: Defense in depth. Auto-enable already kicks in for non-loopback
binds (`crates/roko-serve/src/lib.rs:790-805`), but a user editing config
incorrectly could still ship `auth.enabled = false` on a public bind if
they also set `acknowledge_public_risk = true` for some other reason.

**File**: `crates/roko-core/src/config/serve.rs:83-91`

**Implementation**:

1. Change `enabled: false` to `enabled: true` in the `Default` impl.

```rust
impl Default for ServeAuthConfig {
    fn default() -> Self {
        Self {
            enabled: true,                  // was: false
            api_key: String::new(),
            api_keys: Vec::new(),
            privy_app_id: None,
        }
    }
}
```

2. Update the `roko init` template at
   `crates/roko-cli/src/init.rs` (search for `[serve.auth]` block):

```toml
[serve.auth]
# disable auth for local development only
enabled = false
api_key = ""
```

3. Update tests that assume the default is false. Search:

```bash
rg 'auth.*enabled.*false|enabled = false' crates/roko-serve/tests/ crates/roko-serve/src/
```

For each, decide: was the test asserting "auth is off by default"? If yes,
flip the assertion. Was the test setting up a fixture that needs auth off?
Set it explicitly to `false` in the fixture.

4. Update integration / smoke tests that POST to `/api/...` without an
   `X-API-Key` header — they need to set `enabled = false` in fixture or
   add an `Authorization` header.

**Verify**:

```bash
cargo test --workspace
cargo run -p roko-cli -- init --dry-run
# Generated TOML should include [serve.auth] enabled = false (with comment)
```

**Do not**:

- Remove the auto-enable-for-public-bind logic. That stays as a defense.
- Touch `validate_bind_safety` semantics.
- Land this without `roko init` template update — first-run users will
  hit a confusing 401.

**Estimated effort**: 30-60 minutes (default flip + fixture sweep).

---

## [ ] T3-23: Add rate limiting

**Why**: A single client can saturate inference cost or fill the workspace
disk by spamming `POST /api/inference/complete` or `POST /api/agents/register`.
No rate limit exists today.

**File(s)**:

- `crates/roko-serve/Cargo.toml` — add dep
- `crates/roko-serve/src/routes/mod.rs` — add layer
- `crates/roko-serve/src/routes/middleware.rs` — add helper

**Library choice**: `tower-governor` is the standard. It supports per-IP
buckets, per-route overrides, and `tower::Layer` composition.

```toml
# crates/roko-serve/Cargo.toml
[dependencies]
tower-governor = "0.4"   # confirm version with `cargo add` first
governor = "0.6"
```

**Implementation**:

1. Add a helper in `middleware.rs`:

```rust
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

/// Build a global rate limiter: 100 req/sec/client, with burst 200.
pub fn global_rate_limit_layer() -> GovernorLayer<...> {
    GovernorLayer {
        config: Arc::new(
            GovernorConfigBuilder::default()
                .per_second(100)
                .burst_size(200)
                .finish()
                .expect("valid governor config"),
        ),
    }
}

/// Build a per-route rate limiter for expensive endpoints.
/// Used for: terminal session create, inference complete, agent register.
pub fn expensive_rate_limit_layer(per_minute: u32) -> GovernorLayer<...> {
    GovernorLayer {
        config: Arc::new(
            GovernorConfigBuilder::default()
                .per_second((per_minute / 60).max(1) as u64)
                .burst_size((per_minute / 10).max(2) as u32)
                .finish()
                .expect("valid governor config"),
        ),
    }
}
```

2. Apply layers in `routes/mod.rs::build_router` after the existing
   `RequestBodyLimitLayer`:

```rust
router
    .layer(global_rate_limit_layer())
    .layer(RequestBodyLimitLayer::new(32 * 1024 * 1024))
    // ...
```

3. Apply per-route caps via `Router::route_layer` or per-handler
   middleware. Suggested limits:

| Route | Cap |
|---|---|
| `POST /api/terminal/sessions` | 5/min |
| `POST /api/inference/complete` | 30/min |
| `POST /api/agents/register` | 10/min |
| `POST /api/agents/create` | 10/min |
| `POST /webhooks/github` | 60/min |
| `POST /webhooks/slack` | 60/min |

The cleanest pattern is to push the per-route layer into each route's
`pub fn routes()`:

```rust
// e.g. routes/agents.rs
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/agents/register",
            post(register_agent).layer(middleware::expensive_rate_limit_layer(10))
        )
        // ...
}
```

4. Tests:

```rust
#[tokio::test]
async fn rate_limit_returns_429_after_burst() {
    let app = build_test_app().await;
    for _ in 0..205 { let r = post(&app, "/api/health").await; }
    let r = post(&app, "/api/health").await;
    assert_eq!(r.status(), 429);
}
```

**Verify**:

```bash
cargo test -p roko-serve rate_limit --lib
rg 'GovernorLayer|RateLimitLayer' crates/roko-serve/src/routes/mod.rs
```

**Do not**:

- Apply per-IP only. Behind a load balancer all clients share one IP. Use
  the X-Forwarded-For-aware extractor or route by API key (preferred):
  identify clients by `X-API-Key` when present, fall back to peer IP.
- Use `tower::limit::RateLimitLayer` — it's a global token bucket, not
  per-client.
- Skip the per-route caps. The global limit doesn't protect cost-amplifying
  endpoints (one inference call ≠ one health check in cost terms).

**Estimated effort**: 2-3 hours.

---

## [ ] T3-24: Add per-endpoint request body size limits

**Why**: Global limit is 32 MiB. Webhooks accept JSON bodies; a 30 MiB
payload is implausible. Larger limits enable a memory-pressure DoS.

**File**: `crates/roko-serve/src/routes/mod.rs:183` and per-route handlers.

**Current state** (verified 2026-05-01):

```rust
router
    .layer(RequestBodyLimitLayer::new(32 * 1024 * 1024))
```

**Implementation**:

1. Lower the global limit to 4 MiB:

```rust
router.layer(RequestBodyLimitLayer::new(4 * 1024 * 1024))
```

2. Add per-route overrides for routes that legitimately accept larger
   bodies:

| Route | Limit | Rationale |
|---|---|---|
| `POST /api/inference/complete` | 4 MiB | Default global. Most prompts < 200 KiB. |
| `POST /api/agents/register` | 256 KiB | Manifest only, no payload. |
| `POST /webhooks/github` | 1 MiB | Largest GH webhooks (push events with diff stat) |
| `POST /webhooks/slack` | 256 KiB | Slack events are small. |
| `POST /webhooks/generic` | 64 KiB | Trusted internal use. |
| `POST /api/jobs/{id}/log` (if exists) | 16 MiB | Log uploads can be large. |
| File-upload routes (if any) | per-route, max 100 MiB | Document each. |

3. Apply with `RequestBodyLimitLayer::new(N)` per-route:

```rust
.route(
    "/webhooks/github",
    post(github_webhook).layer(RequestBodyLimitLayer::new(1 * 1024 * 1024)),
)
```

4. Test 413 response:

```rust
#[tokio::test]
async fn rejects_oversize_body() {
    let app = build_test_app().await;
    let body = vec![b'x'; 5 * 1024 * 1024];
    let r = post_bytes(&app, "/api/inference/complete", body).await;
    assert_eq!(r.status(), 413);
}
```

**Verify**:

```bash
cargo test -p roko-serve body_limit --lib
rg 'RequestBodyLimitLayer::new' crates/roko-serve/src/routes/
```

**Do not**:

- Apply uniformly per-route. Some routes need bigger limits than 4 MiB; lower
  the global, raise specific ones.
- Use Bytes extractor without an explicit `RequestBodyLimitLayer` — the
  router-level limit doesn't propagate consistently into all extractors.

**Estimated effort**: 1-2 hours.

---

## [ ] T3-25: Require explicit opt-in for non-loopback bind

**Why**: Today `PORT=8080 roko serve` binds `0.0.0.0:8080` regardless of
`serve.bind`. Cloud platforms (Railway, Fly, etc.) set `PORT`; binding
`0.0.0.0` is correct for those, but silent mode-switching is dangerous if
`PORT` leaks into a local-dev session (or a config edit).

**File**: `crates/roko-serve/src/lib.rs:230-240`

**Current code**:

```rust
let addr = if let Ok(env_port) = std::env::var("PORT") {
    let p: u16 = env_port.parse().context("PORT env var must be a valid u16")?;
    info!("PORT env var detected ({p}), binding to 0.0.0.0:{p}");
    format!("0.0.0.0:{p}")
} else {
    self.addr.clone()
};
```

**New code**:

```rust
let addr = if let Ok(env_port) = std::env::var("PORT") {
    let p: u16 = env_port.parse().context("PORT env var must be a valid u16")?;
    // PORT overrides the *port* only. The bind address still comes from
    // self.addr / serve.bind. Cloud platforms that need 0.0.0.0 must set
    // serve.bind = "0.0.0.0" (or equivalent) explicitly.
    let bind = self
        .addr
        .rsplit_once(':')
        .map(|(host, _)| host.to_string())
        .unwrap_or_else(|| "127.0.0.1".to_string());
    info!("PORT env var detected ({p}); binding to {bind}:{p}");
    format!("{bind}:{p}")
} else {
    self.addr.clone()
};
```

**Implementation**:

1. Apply the change above.
2. Update the `roko init` template to make `serve.bind` explicit:

```toml
[server]
# Bind address. "127.0.0.1" for local-only. "0.0.0.0" for public — requires
# auth.enabled = true or acknowledge_public_risk = true (see [serve.auth]).
bind = "127.0.0.1"
```

3. Update Railway / Fly deploy scripts (if any in `roko-deploy/`) to set
   `bind = "0.0.0.0"` explicitly in the generated config when needed.
4. Tests:

```rust
#[test]
fn port_env_does_not_force_public_bind() {
    std::env::set_var("PORT", "4444");
    let server = ServeServer::default(); // assumes 127.0.0.1 default
    let addr = server.compute_addr();
    assert!(addr.starts_with("127.0.0.1:"));
}
```

5. Consider deprecating: a separate `BIND` env var (not `PORT`) for the
   address override path. **Out of scope** for this task; document the idea
   in a follow-up.

**Verify**:

```bash
cargo test -p roko-serve port_env --lib
PORT=8080 cargo run -p roko-cli -- serve --dry-run
# Should bind 127.0.0.1:8080, not 0.0.0.0:8080
```

**Do not**:

- Reject `PORT` at parse time. It's a valid env var; only the binding
  semantic changes.
- Hardcode "0.0.0.0" in any scenario other than Railway/Fly/cloud
  invocations driven by config.
- Remove `validate_bind_safety` — it remains the gate.

**Estimated effort**: 1-2 hours including tests + deploy script update.

---

## [ ] T3-26: Add WebSocket message size limits

**Why**: WebSocket frames are not subject to the HTTP body limit.
Unbounded WS messages can exhaust memory.

**File**: `crates/roko-serve/src/routes/ws.rs` (and `terminal/ws.rs` if
that exists).

**Discover the upgrade points**:

```bash
rg 'WebSocketUpgrade|on_upgrade|axum::extract::ws' crates/roko-serve/src/ -n
```

For each `WebSocketUpgrade::on_upgrade` call, set caps via
`WebSocketUpgrade::max_message_size` and `WebSocketUpgrade::max_frame_size`:

```rust
ws_upgrade
    .max_message_size(1 * 1024 * 1024)   // 1 MiB
    .max_frame_size(256 * 1024)          // 256 KiB
    .on_upgrade(|socket| handle(socket, state))
```

**Recommended caps**:

| Endpoint | Max message | Max frame | Reason |
|---|---|---|---|
| `/ws` (general) | 1 MiB | 256 KiB | Most messages tiny; large = abuse |
| `/api/terminal/sessions/.../io` | 256 KiB | 64 KiB | Terminal output chunked anyway |
| `/api/workflow/events` | 64 KiB | 16 KiB | Server-pushed only |

**Implementation**:

For each upgrade:

```rust
async fn handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    ws.max_message_size(1 * 1024 * 1024)
        .max_frame_size(256 * 1024)
        .on_upgrade(|socket| handle_socket(socket, state))
}
```

Add a regression test:

```rust
#[tokio::test]
async fn oversize_ws_message_closes_connection() {
    let app = build_test_app().await;
    let mut ws = connect_ws(&app, "/ws").await;
    let big = vec![b'x'; 2 * 1024 * 1024];
    ws.send(Message::Binary(big)).await.unwrap_err(); // close
}
```

**Verify**:

```bash
rg 'max_message_size|max_frame_size' crates/roko-serve/src/
cargo test -p roko-serve ws_size --lib
```

**Do not**:

- Set caps so low that legitimate streaming output gets truncated. The
  terminal `Output` event chunks at 4 KiB by default; 256 KiB / 64 KiB is
  comfortable.
- Leave any upgrade point uncapped. If a new WS endpoint appears, it must
  set caps.
- Mix `max_message_size` and the HTTP body limit — they're independent.

**Estimated effort**: 1-2 hours.

---

## [ ] T3-27: Fix path traversal + TOML injection in agent creation

**Why**:

- `routes/agents.rs::create_agent` constructs `agents_dir =
  state.workdir.join(".roko").join("agents").join(&req.name)`. `req.name`
  is unvalidated. `name = "../../../etc"` writes arbitrary paths.
- The manifest TOML is `format!`-built with `toml_quote(prompt)`. While
  `toml_quote` escapes `"` and `\\`, it does **not** prevent injection of
  bracket-delimited tables (`[malicious]` on its own line in the prompt).

**File**: `crates/roko-serve/src/routes/agents.rs:601-697`

### Subtask A: Path canonicalization

1. Add a helper in `error.rs` (or reuse `validate_path_segment`):

```rust
/// Validate a name component for use as a directory name.
/// Rejects: empty, `.`, `..`, `/`, `\\`, control characters, leading dot.
pub fn validate_workspace_dir_segment(name: &str, what: &str) -> Result<(), ApiError> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.starts_with('.')
        || name.contains('/')
        || name.contains('\\')
        || name.chars().any(|c| c.is_control())
    {
        return Err(ApiError::bad_request(format!("invalid {what}: {name:?}")));
    }
    Ok(())
}
```

2. In `create_agent`, validate first:

```rust
async fn create_agent(...) -> Result<...> {
    crate::error::validate_workspace_dir_segment(&req.name, "agent name")?;
    let agents_dir = state.workdir.join(".roko").join("agents").join(&req.name);
    // ...
}
```

3. After `create_dir_all`, canonicalize and assert containment (defense in
   depth):

```rust
tokio::fs::create_dir_all(&agents_dir).await
    .map_err(|e| ApiError::internal(format!("create agent dir: {e}")))?;

let workspace_root = state.workdir.canonicalize()
    .map_err(|e| ApiError::internal(format!("canonicalize workspace: {e}")))?;
let canonical_dir = agents_dir.canonicalize()
    .map_err(|e| ApiError::internal(format!("canonicalize agent dir: {e}")))?;
if !canonical_dir.starts_with(&workspace_root) {
    return Err(ApiError::bad_request("agent path escaped workspace"));
}
```

### Subtask B: Structured TOML serialization

1. Define an `AgentManifest` struct (in `routes/agents.rs` or a new
   `manifest.rs`):

```rust
#[derive(Debug, Serialize)]
struct AgentManifest {
    schema_version: u32,
    core: AgentCore,
}

#[derive(Debug, Serialize)]
struct AgentCore {
    prompt: String,
    mode: String,
    domain: AgentDomain,
}

#[derive(Debug, Serialize)]
struct AgentDomain {
    #[serde(flatten)]
    inner: BTreeMap<String, serde_json::Value>,
}
```

2. Construct it and use `toml::to_string_pretty`:

```rust
let manifest = AgentManifest {
    schema_version: 1,
    core: AgentCore {
        prompt: prompt.to_string(),
        mode: "self_hosted".into(),
        domain: AgentDomain {
            inner: {
                let mut m = BTreeMap::new();
                m.insert(req.domain.clone(), serde_json::json!({}));
                m
            },
        },
    },
};
let manifest_toml = toml::to_string_pretty(&manifest)
    .map_err(|e| ApiError::internal(format!("serialize manifest: {e}")))?;
```

3. **Validate `req.domain`**: it becomes a TOML key path. Restrict to
   `[a-z][a-z0-9_-]{0,31}`:

```rust
fn validate_domain_key(s: &str) -> Result<(), ApiError> {
    if !s.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        || !s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        || s.len() > 32
    {
        return Err(ApiError::bad_request(format!("invalid domain: {s:?}")));
    }
    Ok(())
}
```

4. Remove `toml_quote()` (or mark it `#[deprecated]` if other call sites
   use it; check first).

### Subtask C: Tests

```rust
#[tokio::test]
async fn rejects_path_traversal_in_agent_name() {
    let app = build_test_app().await;
    let r = post_json(&app, "/api/agents/create", json!({"name": "../etc/passwd"})).await;
    assert_eq!(r.status(), 400);
}

#[tokio::test]
async fn manifest_serialization_escapes_table_injection() {
    let app = build_test_app().await;
    let evil = "harmless\n[malicious]\ninjected = true";
    let r = post_json(&app, "/api/agents/create",
        json!({"name": "ok", "prompt": evil, "domain": "research"})).await;
    assert_eq!(r.status(), 200);

    let manifest = std::fs::read_to_string(
        format!("{}/.roko/agents/ok/manifest.toml", workspace.path().display())
    ).unwrap();
    let parsed: toml::Value = toml::from_str(&manifest).unwrap();
    // Confirm the prompt was preserved as-is, no `[malicious]` table at top level.
    assert!(parsed.get("malicious").is_none());
    assert_eq!(parsed["core"]["prompt"].as_str().unwrap(), evil);
}

#[tokio::test]
async fn rejects_invalid_domain_key() {
    let app = build_test_app().await;
    let r = post_json(&app, "/api/agents/create",
        json!({"name": "ok", "domain": "bad domain!"})).await;
    assert_eq!(r.status(), 400);
}
```

### Verification

```bash
cargo test -p roko-serve agent_manifest --lib
cargo test -p roko-serve create_agent --lib
rg 'toml_quote' crates/roko-serve/  # only behind #[deprecated] or removed
rg 'format!\([^)]*manifest' crates/roko-serve/  # 0 matches
```

### Do not

- Lower validation rules to admit "common but unusual" agent names. The
  existing names should already pass `[a-z0-9_-]+`.
- Add the canonicalize step **before** `create_dir_all` (the path doesn't
  exist yet at that point; canonicalize fails).
- Use `Path::strip_prefix` instead of `canonicalize().starts_with` —
  `strip_prefix` does not resolve symlinks and can be fooled.

**Estimated effort**: 3-4 hours.

---

## [ ] T3-28: Restrict CORS methods/headers

**Why**: `allow_methods(Any).allow_headers(Any)` is broader than necessary.
Restricting to the methods/headers we actually use prevents accidental
admission of unusual verbs.

**File**: `crates/roko-serve/src/routes/middleware.rs:432-462`

**Current code**:

```rust
pub fn cors_layer(cors_origins: &[String], unsafe_public: bool) -> CorsLayer {
    if !cors_origins.is_empty() {
        // ...
        return CorsLayer::new()
            .allow_origin(allowed)
            .allow_methods(Any)
            .allow_headers(Any);
    }
    if unsafe_public { return CorsLayer::permissive(); }
    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(...))
        .allow_methods(Any)
        .allow_headers(Any)
}
```

**Implementation**:

1. Replace `Any` with explicit lists:

```rust
use axum::http::{Method, header};

const ALLOWED_METHODS: [Method; 6] = [
    Method::GET,
    Method::POST,
    Method::PUT,
    Method::DELETE,
    Method::PATCH,
    Method::OPTIONS,
];

const ALLOWED_HEADERS: [HeaderName; 6] = [
    header::CONTENT_TYPE,
    header::AUTHORIZATION,
    header::ACCEPT,
    HeaderName::from_static("x-api-key"),
    HeaderName::from_static("x-request-id"),
    HeaderName::from_static("x-roko-session"),
];

pub fn cors_layer(cors_origins: &[String], unsafe_public: bool) -> CorsLayer {
    let mut layer = CorsLayer::new()
        .allow_methods(ALLOWED_METHODS)
        .allow_headers(ALLOWED_HEADERS);

    if !cors_origins.is_empty() {
        let allowed: Vec<axum::http::HeaderValue> =
            cors_origins.iter().filter_map(|o| o.parse().ok()).collect();
        return layer.allow_origin(allowed);
    }

    if unsafe_public {
        if UNSAFE_PUBLIC_CORS_WARNING.set(()).is_ok() {
            tracing::warn!(...);
        }
        // Even for permissive, restrict methods/headers.
        return layer.allow_origin(AllowOrigin::any());
    }

    layer.allow_origin(AllowOrigin::predicate(...))
}
```

2. Verify the loopback predicate still works.
3. Add a regression test:

```rust
#[tokio::test]
async fn cors_rejects_disallowed_method() {
    let app = build_test_app().await;
    let r = preflight(&app, "/api/health", "TRACE", "Content-Type").await;
    assert!(!r.headers().contains_key("access-control-allow-methods"));
    // Or: assert that TRACE is not in the allowed methods header
}

#[tokio::test]
async fn cors_rejects_disallowed_header() {
    let app = build_test_app().await;
    let r = preflight(&app, "/api/health", "POST", "X-Custom-Bad").await;
    assert!(!cors_allows_header(r.headers(), "X-Custom-Bad"));
}
```

**Verify**:

```bash
rg 'allow_methods\(Any\)|allow_headers\(Any\)' crates/roko-serve/src/
# Should only match in src/lib.rs::build_cors_layer (legacy path) — fix that too,
# or document why it's distinct.
cargo test -p roko-serve cors --lib
```

**Note**: there is a second `build_cors_layer` in `crates/roko-serve/src/lib.rs:724`
that's still permissive. Audit usage:

```bash
rg 'build_cors_layer' crates/roko-serve/src/
```

If unused, delete. If used, apply the same restriction.

**Do not**:

- Allow `*` for `allow_credentials = true` paths. Browsers reject. Pick
  origin restriction instead.
- Add headers to the allowlist that aren't read by any handler. Audit
  `routes/middleware.rs::require_api_key` for the actual auth header
  names (`X-API-Key`, `Authorization` Bearer).
- Remove `OPTIONS` from the methods list. Preflight uses it.

**Estimated effort**: 1-2 hours.

---

## Combined Verification (after all of T3-22..T3-28)

```bash
cargo test --workspace
cargo clippy --workspace --no-deps -- -D warnings

# Defaults check
cargo run -p roko-cli -- serve --dry-run    # should require auth or warn

# Per-feature checks
rg 'GovernorLayer' crates/roko-serve/src/      # rate limiting present
rg 'RequestBodyLimitLayer::new\(4 \*' crates/roko-serve/src/routes/mod.rs
                                              # global limit lowered
rg 'max_message_size|max_frame_size' crates/roko-serve/src/
                                              # WS caps present
rg 'allow_methods\(Any\)|allow_headers\(Any\)' crates/roko-serve/src/
                                              # 0 matches
rg 'validate_workspace_dir_segment' crates/roko-serve/src/routes/agents.rs
                                              # path validation in create_agent
```

After all pass, the `scripts/roko-fitness-checks.sh` baseline should
update with these as no-new-violations gates. See plan 27.

---

## Status

- [ ] T3-22 — Flip auth default to enabled
- [ ] T3-23 — Add rate limiting
- [ ] T3-24 — Add per-endpoint request body size limits
- [ ] T3-25 — Require explicit opt-in for non-loopback bind
- [ ] T3-26 — Add WebSocket message size limits
- [ ] T3-27 — Fix path traversal + TOML injection in agent creation
- [ ] T3-28 — Restrict CORS methods/headers

**After completion**: roko-serve safe to deploy on a public bind without
ad-hoc reverse-proxy hardening. Move on to Tier 4
(`14-tier4-feedback-loops.md`).
