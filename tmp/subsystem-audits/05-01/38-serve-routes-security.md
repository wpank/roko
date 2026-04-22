# 38 -- Serve Routes Security Audit

Audit date: 2026-05-01
Scope: `crates/roko-serve/src/` -- HTTP control plane security posture

---

## Summary

The roko HTTP control plane (~85 routes on :6677) has several well-designed security layers (auth middleware, secret scrubbing, CORS policy, bind-safety validation). However, auth is **disabled by default**, multiple routes bypass auth entirely, the terminal subsystem allows arbitrary command execution, and there is no rate limiting or request body size enforcement. The SSRF surface through agent registration is real.

**Finding count**: 4 CRITICAL, 5 HIGH, 5 MEDIUM, 3 LOW

---

## 1. Auth -- Disabled by Default

### Finding: AUTH IS OFF BY DEFAULT [CRITICAL]

**File**: `crates/roko-core/src/config/serve.rs:83-92`

```rust
impl Default for ServeAuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,       // <-- auth disabled
            api_key: String::new(),
            api_keys: Vec::new(),
            privy_app_id: None,
        }
    }
}
```

When `serve.auth.enabled` is `false` (the default), the auth middleware layer is never applied. This is controlled at `crates/roko-serve/src/routes/mod.rs:124-132`:

```rust
let api = if api_auth.enabled {
    api.layer(axum::middleware::from_fn(middleware::require_scope))
        .layer(axum::middleware::from_fn_with_state(
            Arc::clone(&state),
            middleware::require_api_key,
        ))
} else {
    api   // <-- entire /api/* tree is unauthenticated
};
```

The `validate_bind_safety()` function at `crates/roko-serve/src/lib.rs:641-659` prevents binding to a non-loopback address without auth *unless* `serve.acknowledge_public_risk = true`. This is a reasonable guardrail, but anyone deploying on Railway/Fly (where PORT env is set) hits the `0.0.0.0` override at line 236:

```rust
if let Ok(p) = std::env::var("PORT") {
    info!("PORT env var detected ({p}), binding to 0.0.0.0:{p}");
    format!("0.0.0.0:{p}")
}
```

Auto-auth via Privy credential detection exists (line 773-785) but only triggers when a stored Privy credential is found on disk. In a fresh cloud deploy, this would not exist.

**Mitigation**: There IS a safety check at `validate_bind_safety()`. For loopback-only use (the default `127.0.0.1` bind), unauthenticated access is acceptable. The risk materializes on cloud deploys.

### Finding: Routes That Bypass Auth Entirely [HIGH]

**File**: `crates/roko-serve/src/routes/mod.rs:164-176`

These routes are mounted **outside** the `/api` auth layer:

| Route | Handler | Auth? | Concern |
|---|---|---|---|
| `GET /health` | `top_level_health` | None | Liveness probe -- acceptable |
| `POST /webhooks/github` | `github_webhook` | HMAC signature | Properly verified |
| `POST /webhooks/slack` | `slack_webhook` | HMAC signature | Properly verified |
| `POST /webhooks/generic` | `generic_webhook` | **None** | Unauthenticated ingress |
| `GET /runs/{id}` | `get_run_html` | **None** | Public share reader |
| `GET /api/runs/{id}` | `get_run_json` | **None** | Public share reader |
| `GET /api/shared/{token}` | `get_shared_run` | **None** | Public share reader |
| `/ws/terminal/{id}` | `ws_terminal` | Config-gated | Terminal disabled by default |
| `/ws` and `/roko-ws` | `ws_upgrade` | Conditional | Only auth'd if `api_auth.enabled` |

The **generic webhook** (`/webhooks/generic`) at `crates/roko-serve/src/routes/webhooks.rs:158-169` is the most concerning. Its own docstring says "intended for internal use behind auth" but it is mounted outside the auth layer. Any caller can inject signals into the system:

```rust
async fn generic_webhook(
    State(state): State<Arc<AppState>>,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    let payload: Value = serde_json::from_slice(&body)
        .map_err(|e| ApiError::bad_request(...))?;
    let signal = attach_hdc_fingerprint(generic_webhook_signal(payload));
    persist_webhook_signal(&state, signal).await?;
    Ok(StatusCode::OK)
}
```

### Finding: WebSocket Auth Is Conditional [MEDIUM]

**File**: `crates/roko-serve/src/routes/mod.rs:155-162`

```rust
let ws = if api_auth.enabled {
    ws::routes().layer(axum::middleware::from_fn_with_state(
        Arc::clone(&state),
        middleware::require_api_key,
    ))
} else {
    ws::routes()  // <-- unauthenticated WebSocket
};
```

When auth is disabled (the default), anyone can connect to `/ws` or `/roko-ws` and receive the full `ServerEvent` stream, which includes agent output, run status, configuration changes, and more.

### Auth Quality -- GOOD

When auth IS enabled, the implementation is solid:
- SHA-256 hashed API keys with expiry (`crates/roko-serve/src/routes/middleware.rs:114-129`)
- Scope-based RBAC: admin, agent:write, plan:write, write, read (`middleware.rs:355-380`)
- Privy JWT validation via JWKS cache (`middleware.rs:201-212`)
- Agent bearer tokens with per-agent hashing and expiry (`middleware.rs:219-246`)
- Auth context injected into request extensions for downstream handlers (`middleware.rs:343-344`)

---

## 2. SSRF -- Agent Registration Allows External URLs

### Finding: Unrestricted URL Registration for Agent Endpoints [CRITICAL]

**File**: `crates/roko-serve/src/routes/agents.rs:1636-1684`

`RegisterAgentRequest` accepts arbitrary URLs for `rest_endpoint`, `websocket_endpoint`, `a2a_endpoint`, and `mcp_endpoint`. No validation is performed on these URLs:

```rust
struct RegisterAgentRequest {
    // ...
    rest_endpoint: Option<String>,
    websocket_endpoint: Option<String>,
    a2a_endpoint: Option<String>,
    mcp_endpoint: Option<String>,
    // ...
}

impl RequestPayload for RegisterAgentRequest {
    fn validate_payload(&self) -> Result<(), ApiError> {
        validate_with_validator(self)  // Only validates agent_id length
    }
}
```

These URLs are then used directly in server-side requests:

**Proxy logs** (`agents.rs:1025-1065`):
```rust
async fn proxy_agent_logs(...) -> Result<Response, ApiError> {
    let rest = agent.endpoints.rest.ok_or_else(...)?;
    let url = format!("{}/logs", rest.trim_end_matches('/'));
    let mut request = state.http_client.get(url);  // SSRF
    // ...
}
```

**Send message** (`agents.rs:1131-1136`):
```rust
let url = format!("{}/message", rest.trim_end_matches('/'));
let mut request = state.http_client.post(url).json(&json!({...}));  // SSRF
```

**WebSocket proxy** (`agents.rs:1303-1338`):
```rust
async fn proxy_sidecar_stream(..., ws_url: String, ...) {
    let mut request = ws_url.as_str().into_client_request()?;
    connect_async(request).await?;  // SSRF via WebSocket
}
```

Attack: register an agent with `rest_endpoint: "http://169.254.169.254/latest/meta-data/"` and then hit `GET /api/agents/{id}/logs` to read the response. On cloud deployments this exposes instance metadata (AWS/GCP credentials).

### Finding: Connector Registration Also Accepts Arbitrary Endpoints [MEDIUM]

**File**: `crates/roko-serve/src/routes/connectors.rs:34-41`

```rust
struct CreateConnectorRequest {
    name: String,
    kind: ConnectorKind,
    endpoint: String,  // No URL validation
    metadata: Value,
}
```

---

## 3. Path Traversal -- Shared Runs

### Finding: Shared Run IDs Not Validated for Path Components [HIGH]

**File**: `crates/roko-serve/src/routes/shared_runs.rs:270-276`

The `load_transcript_record` function constructs a file path from the user-supplied `id` without calling `validate_path_segment`:

```rust
fn load_transcript_record(state: &AppState, id: &str) -> Option<LoadedTranscript> {
    let path = state
        .workdir
        .join(".roko")
        .join("shared")
        .join(format!("{id}.json"));  // <-- id is user-controlled
    let data = std::fs::read_to_string(path).ok()?;
```

The public routes `GET /api/runs/{id}`, `GET /api/shared/{token}`, and `GET /runs/{id}` all pass the path parameter directly to this function. An attacker could use `../../secrets` as the ID to attempt reading `.roko/secrets.toml` (though the `.json` suffix limits exploitability since it would look for `../../secrets.json`).

The `create_share` handler at line 219 also writes to this path:
```rust
let path = shared_dir.join(format!("{token}.json"));
```

Where `token = format!("{}-{:04x}", id, std::process::id() as u16)` derives from the route parameter `id`.

Note: Other route groups (plans, jobs, PRDs, research, auth) consistently use `validate_path_segment()` which rejects `..`, `/`, and `\\`. Shared runs is the exception.

### Finding: Path Traversal Protection Exists Elsewhere [LOW -- positive]

**File**: `crates/roko-serve/src/error.rs:155-162`

The codebase has a `validate_path_segment` utility and it IS used in:
- `routes/jobs.rs` (12 call sites)
- `routes/plans.rs` (6 call sites)
- `routes/auth.rs` (1 call site)
- `routes/research.rs` (3 call sites)
- `routes/prds.rs` (3 call sites)

The fact that `shared_runs.rs` does NOT use it is an oversight, not a design gap.

---

## 4. Secret Leakage

### Finding: GET /api/config/toml Bypasses Secret Masking [CRITICAL]

**File**: `crates/roko-serve/src/routes/config.rs:48-58`

```rust
async fn get_config_toml(
    State(state): State<Arc<AppState>>,
) -> Result<([(axum::http::header::HeaderName, &'static str); 1], String), ApiError> {
    let cfg = state.load_roko_config();
    let toml_str = toml::to_string_pretty(cfg.as_ref())  // Full config, no masking
        .map_err(|e| ApiError::internal(format!("serialize toml: {e}")))?;
    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/toml")],
        toml_str,
    ))
}
```

Compare with `GET /api/config` (JSON) at line 36-43 which calls `mask_secret_fields(&mut value)`. The TOML endpoint does NOT mask secrets. If `serve.auth.api_key`, `server.auth_token`, or `deploy.railway_api_token` are set in the config, they are returned in cleartext.

The scrubber middleware (`middleware::scrub_secrets`) is applied to `/api/*` routes, but it only matches known secret patterns (API key prefixes like `sk-ant-`, `ghp_`, etc.). Config-embedded secrets like a plain API key string or a Railway token would not match these patterns.

### Finding: mask_secret_fields Only Covers 3 Fields [MEDIUM]

**File**: `crates/roko-serve/src/routes/config.rs:245-259`

```rust
fn mask_secret_fields(value: &mut Value) {
    mask_secret_field(value, &["serve", "auth"], "api_key", ...);      // 1
    mask_secret_field(value, &["server"], "auth_token", ...);           // 2
    mask_secret_field(value, &["deploy"], "railway_api_token", ...);    // 3
}
```

Missing fields that could contain secrets:
- `providers.*.api_key` -- LLM provider API keys
- `webhooks.github.secret` -- HMAC shared secret
- `chain.private_key` or similar chain wallet credentials
- Any provider-specific credentials in `[providers]`

Note: The `scrub_secrets` response middleware (`middleware.rs:508-544`) using `LogScrubber` adds a second layer that catches known API key patterns (Anthropic `sk-ant-*`, GitHub `ghp_*`, etc.), but this is regex-based and will miss custom tokens.

### Finding: Response Scrubber Has Good Coverage [LOW -- positive]

**File**: `crates/roko-serve/src/routes/middleware.rs:508-544`

The `scrub_secrets` middleware:
- Applied to all `/api` routes (line 136-139 in mod.rs)
- Collects response body up to 16 MiB
- Uses `LogScrubber` which recognizes common API key patterns
- Skips binary responses and SSE streams (to avoid blocking)
- Tests confirm Anthropic keys, GitHub PATs are redacted

---

## 5. Terminal Security

### Finding: Arbitrary Command Execution via PTY [CRITICAL]

**File**: `crates/roko-serve/src/terminal.rs:58-66`

```rust
pub struct CreateSessionRequest {
    pub cols: u16,
    pub rows: u16,
    pub command: Option<String>,   // <-- arbitrary command
    pub workdir: Option<String>,   // <-- arbitrary working directory
}
```

The `create_session` handler (line 217-258) spawns whatever command is requested:

```rust
let (mut cmd, command_label) = if let Some(command) = command {
    let mut parts = command.split_whitespace();
    let Some(program) = parts.next() else { ... };
    let mut cmd = CommandBuilder::new(program);
    for arg in parts { cmd.arg(arg); }
    (cmd, command.to_string())
} else {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    (CommandBuilder::new(shell.clone()), shell)
};
```

And the `workdir` is used directly without validation:

```rust
let wd = workdir
    .map(std::path::PathBuf::from)
    .unwrap_or_else(|| self.workdir.clone());
cmd.cwd(&wd);
```

**Mitigations already present**:
1. Terminal is **disabled by default** (`serve.terminal_enabled` defaults to `false`, line 24-25 in serve.rs)
2. When on a public bind, terminal routes require auth (`mod.rs:141-153`)
3. When disabled, all terminal routes return 403 (`terminal.rs:624-649`)

**Remaining risk**: On loopback (the default), terminal routes work without auth if `terminal_enabled = true`. This is intentional for local dev but any local process can exec commands.

### Finding: No Session Isolation [MEDIUM]

**File**: `crates/roko-serve/src/terminal.rs:176-193`

All PTY sessions run as the same OS user that runs the server. There is no sandboxing, no per-session resource limits, no command allowlist. The `SessionManager` tracks sessions by UUID but provides no isolation between them.

The WebSocket terminal bridge at `ws_terminal` (line 514-535) auto-creates a session on connection, meaning any WS client can spawn a shell.

---

## 6. Rate Limiting

### Finding: No Rate Limiting Anywhere [HIGH]

No rate limiting middleware was found in the entire `crates/roko-serve/src/` tree. A search for `rate_limit`, `RateLimit`, `throttle`, and `Throttle` returned zero results.

This applies to all ~85 routes including:
- `POST /api/inference/complete` -- unbounded LLM API calls
- `POST /api/agents/register` -- unbounded agent registration
- `POST /webhooks/generic` -- unbounded signal injection
- `POST /api/secrets/{ns}/{key}` -- unbounded secret writes
- `POST /api/terminal/sessions` -- unbounded PTY spawning

### Finding: No Request Body Size Limits [HIGH]

No `DefaultBodyLimit` or `RequestBodyLimit` layer was found. The only body size cap is in the scrubber middleware (`middleware.rs:524`):

```rust
let Ok(bytes) = axum::body::to_bytes(body, 16 * 1024 * 1024).await else {
    return ApiError::internal("response body collection failed").into_response();
};
```

But this limits **response** body collection, not request bodies. Axum's default body limit is 2 MiB for `Json` extractors, but `Bytes` extractors (used in webhook routes) have no limit.

---

## 7. CORS

### Finding: CORS Policy Is Reasonable by Default [LOW -- positive]

**File**: `crates/roko-serve/src/routes/middleware.rs:432-463`

The default CORS behavior (no `cors_origins` configured, `unsafe_public_cors` = false) uses a predicate that only allows localhost/loopback origins:

```rust
CorsLayer::new()
    .allow_origin(AllowOrigin::predicate(
        |origin, _parts| match origin.to_str() {
            Ok(origin) => is_local_origin(origin),
            Err(_) => false,
        },
    ))
    .allow_methods(Any)
    .allow_headers(Any)
```

The `is_local_origin` function correctly validates:
- `http://localhost:*` -- accepted
- `http://127.0.0.1:*` -- accepted
- `http://[::1]:*` -- accepted
- Everything else -- rejected

Wide-open CORS (`CorsLayer::permissive()`) only activates when `server.unsafe_public_cors = true` AND no `cors_origins` are configured. A log warning is emitted.

**Note**: `allow_methods(Any)` and `allow_headers(Any)` are more permissive than necessary. Standard practice would restrict methods to the ones actually used and limit allowed headers.

---

## 8. Bind Address

### Finding: Default Bind Is Loopback, with Safety Validation [LOW -- positive]

**File**: `crates/roko-core/src/config/serve.rs:222-228`

```rust
fn default_bind() -> String {
    "127.0.0.1".into()
}

const fn default_port() -> u16 {
    6677
}
```

**File**: `crates/roko-serve/src/lib.rs:637-659`

```rust
pub fn validate_bind_safety(addr: &str, serve: &ServeConfig) -> Result<()> {
    if is_loopback_addr(addr) || serve.auth.enabled {
        return Ok(());
    }
    if serve.acknowledge_public_risk {
        warn!("binding to a public address without authentication; ...");
        return Ok(());
    }
    anyhow::bail!(
        "Public bind requires `serve.auth.enabled = true` or ..."
    );
}
```

This is good defense-in-depth. The PORT env override (line 236) that binds to `0.0.0.0` still requires passing `validate_bind_safety`.

---

## 9. WebSocket Security

### Finding: No Message Size Limits on WebSocket Connections [MEDIUM]

**File**: `crates/roko-serve/src/routes/ws.rs:29-30`

```rust
async fn ws_upgrade(State(state): State<Arc<AppState>>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(state, socket))
}
```

The WebSocket upgrade uses axum defaults with no `max_message_size` or `max_frame_size` configuration. By default, axum/tungstenite allows messages up to 64 MiB. A malicious client could send oversized messages.

This applies to all WebSocket endpoints:
- `/ws` and `/roko-ws` (event streaming)
- `/ws/terminal/{id}` (PTY bridge)
- `/api/workflow/ws` (workflow events)
- `/api/ws` (aggregator)

### Finding: No Authentication on WebSocket Upgrade When Auth Disabled [MEDIUM]

Covered above in Section 1 -- when auth is disabled, the `/ws` route has no auth layer. Additionally, the terminal WebSocket at `/ws/terminal/{id}` is gated by config + bind policy but does NOT have its own authentication mechanism for the WebSocket upgrade specifically. The auth middleware applies to HTTP routes, and WebSocket upgrades pass through it, but only if the layer is applied.

---

## 10. Webhook Routes

### Finding: Generic Webhook Is Public and Unauthenticated [HIGH]

**File**: `crates/roko-serve/src/routes/webhooks.rs:155-169`

Covered above. The `/webhooks/generic` endpoint:
- Has no signature verification
- Has no authentication
- Accepts arbitrary JSON
- Persists it as a signal into `.roko/engrams.jsonl`
- Publishes it onto the server event bus

This allows external actors to inject arbitrary signals that could trigger downstream workflows if subscriptions are configured.

### Finding: GitHub and Slack Webhooks Are Properly Verified [LOW -- positive]

**File**: `crates/roko-serve/src/routes/webhooks.rs:37-153`

- GitHub: HMAC-SHA256 signature verified via `X-Hub-Signature-256` with constant-time comparison (`constant_time_eq`)
- Slack: HMAC-SHA256 signature verified via `X-Slack-Signature` with timestamp replay protection (300-second window)
- Both reject requests when the shared secret is not configured

---

## Consolidated Finding Table

| # | Severity | Category | Finding | File:Line |
|---|----------|----------|---------|-----------|
| 1 | CRITICAL | Auth | Auth disabled by default; entire /api tree unauthenticated | `config/serve.rs:86`, `routes/mod.rs:130` |
| 2 | CRITICAL | SSRF | Agent registration accepts arbitrary URLs, used in proxy requests | `routes/agents.rs:1039-1040`, `agents.rs:1132-1133` |
| 3 | CRITICAL | Secrets | GET /api/config/toml returns raw config with unmasked secrets | `routes/config.rs:48-58` |
| 4 | CRITICAL | Terminal | Arbitrary command execution via CreateSessionRequest.command | `terminal.rs:58-66`, `terminal.rs:239-254` |
| 5 | HIGH | Auth | Generic webhook is public, no auth, can inject signals | `routes/webhooks.rs:158-169` |
| 6 | HIGH | Path | Shared run IDs not validated, potential path traversal | `routes/shared_runs.rs:270-276` |
| 7 | HIGH | Rate | No rate limiting on any route | (absent) |
| 8 | HIGH | Rate | No request body size limits (Bytes extractors) | (absent) |
| 9 | HIGH | Auth | Routes bypassing auth: public shared runs, webhooks, health | `routes/mod.rs:164-176` |
| 10 | MEDIUM | SSRF | Connector registration accepts arbitrary endpoint URLs | `routes/connectors.rs:35-41` |
| 11 | MEDIUM | Secrets | mask_secret_fields only covers 3 of N secret fields | `routes/config.rs:245-259` |
| 12 | MEDIUM | WS | No message size limits on WebSocket connections | `routes/ws.rs:29-30` |
| 13 | MEDIUM | Terminal | No session isolation, sandboxing, or command allowlist | `terminal.rs:176-193` |
| 14 | MEDIUM | WS | WebSocket auth conditional on global auth toggle | `routes/mod.rs:155-162` |
| 15 | LOW | CORS | allow_methods(Any) and allow_headers(Any) overly broad | `routes/middleware.rs:437-439` |
| 16 | LOW | Bind | Default bind is 127.0.0.1 with safety validation | `config/serve.rs:222-228` |
| 17 | LOW | Auth | Auth implementation quality is solid when enabled | `routes/middleware.rs` |

---

## Recommended Fixes (Priority Order)

### P0 -- Before any public deployment

1. **Mask secrets in TOML endpoint**: Apply the same masking logic to `GET /api/config/toml` as `GET /api/config`. Serialize to TOML, parse back, mask, re-serialize. Or simply reject the TOML endpoint when auth is disabled.

2. **Validate agent/connector endpoint URLs**: Add URL validation to `RegisterAgentRequest` and `CreateConnectorRequest`. Reject internal IP ranges (169.254.0.0/16, 10.0.0.0/8, 172.16.0.0/12, 127.0.0.0/8, [::1]) and non-HTTP(S) schemes.

3. **Add `validate_path_segment` to shared_runs.rs**: Apply the existing `validate_path_segment` function to the `id` and `token` parameters in `get_run_json`, `get_shared_run`, `get_run_html`, and `create_share`.

4. **Move generic webhook behind auth**: Either mount `/webhooks/generic` inside the `/api` auth layer, or add its own authentication mechanism.

### P1 -- Before production use

5. **Add rate limiting**: Tower's `RateLimit` or `governor` middleware. Priority targets: inference endpoints, terminal session creation, agent registration, webhook ingress.

6. **Add request body size limits**: Apply `DefaultBodyLimit` to the router. Webhook `Bytes` extractors should have explicit size caps.

7. **Expand `mask_secret_fields`**: Cover `providers.*.api_key`, `webhooks.github.secret`, and any other credential fields.

8. **Add WebSocket message size configuration**: Set `max_message_size` on `WebSocketUpgrade`.

### P2 -- Hardening

9. **Terminal command allowlist**: When enabled, restrict commands to a configured allowlist or at minimum reject absolute paths outside the workdir.

10. **Restrict CORS methods/headers**: Replace `allow_methods(Any)` and `allow_headers(Any)` with the specific methods and headers the API actually uses.

---

## Architecture Notes

The security architecture has clearly been designed with intent:
- The auth middleware is well-implemented with multiple credential types
- The `validate_bind_safety` function prevents accidental public exposure
- The response scrubber is a defense-in-depth layer
- Terminal is disabled by default
- GitHub/Slack webhooks use proper signature verification
- The `validate_path_segment` utility exists and is used in most route groups

The gaps are mostly in consistency (shared_runs not using path validation, TOML endpoint not masking, generic webhook placement) and missing defense-in-depth layers (rate limiting, body limits, SSRF prevention).
