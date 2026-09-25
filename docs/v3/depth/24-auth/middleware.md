# 24-auth / middleware

> Detailed walkthrough of the serve-auth middleware request flow, credential
> extraction order, scope enforcement, and response headers.

---

## 1. Middleware Entry Point

The `require_api_key` function in `crates/roko-serve/src/routes/middleware.rs` is
registered as an axum `middleware::from_fn_with_state` layer on all `/api/*`
routes. It runs before every handler when `serve.auth.enabled = true`.

```rust
pub async fn require_api_key(
    State(state): State<Arc<AppState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, ApiError>
```

The middleware loads the current `ServeAuthConfig` and the named API key snapshot
from the `AuthRegistry` at the start of each request. This means configuration
changes (key rotation, scope updates) take effect on the next request without
server restart.

---

## 2. Credential Extraction Order

### 2.1 Worker Callback (Priority 1)

Before checking user-facing credentials, the middleware tests whether the
request is a worker callback: `POST /api/deployments/:id/callback`. If so, it
extracts the `X-Roko-Worker-Token` header and delegates to
`authenticate_worker_callback_token` in `deployments.rs`.

Worker callbacks receive a fixed `"write"` scope and a user_id of
`worker:{deployment_id}`. This short-circuits all other auth paths -- a worker
callback never falls through to API key or JWT checking.

### 2.2 API Credential Detection

For non-worker requests, `api_credential(headers)` classifies the request into
one of five states:

| State | Condition |
|-------|-----------|
| `XApiKey(token)` | `X-Api-Key` header present and valid UTF-8 |
| `Bearer(token)` | `Authorization: Bearer <token>` present |
| `InvalidXApiKey` | `X-Api-Key` present but not valid UTF-8 |
| `InvalidAuthorization` | `Authorization` present but malformed |
| `Missing` | Neither header present |

`InvalidXApiKey`, `InvalidAuthorization`, and `Missing` all produce immediate
401 responses.

### 2.3 X-Api-Key Path

When `X-Api-Key` is present, `authenticate_api_key` checks:

1. Named key entries -- `match_api_key_entry` hashes the token with SHA-256 and
   compares against every `ApiKeyEntry.key_hash`, including grace-period entries
   from recent rotations.
2. Legacy single key -- constant-time comparison against `auth.api_key`.

Results:
- `Valid` or `GracePeriod` -- authentication succeeds with the entry's scope.
- `Expired` -- 401 with `X-Key-Expired: true`.
- `NotFound` -- 401.

### 2.4 Bearer Token Path

Bearer tokens are routed by prefix:

1. **`roko_agent_`** -- routed to `try_named_agent_token`, which checks the
   `AuthRegistry.agent_tokens` list. On match, the `AgentCredentialClaims`
   (containing capabilities, depth, expiry) are injected into request extensions.

2. **`roko_relay_`** -- routed to `authenticate_relay_secret`, which validates
   the relay chain: token hash, expiry, revocation, and depth bounds.

3. **Other tokens** -- tried in sequence:
   - `authenticate_api_key` (same as X-Api-Key path, but via Bearer).
   - `try_agent_token` (legacy per-agent sidecar tokens stored in
     `DiscoveredAgent.token_hash`).
   - `try_privy_jwt` (JWKS-based JWT verification).

If all four paths fail, 401 is returned.

---

## 3. Agent Capability Enforcement

After authentication, agent tokens face a second authorization check. The
middleware calls `required_agent_capability(method, path)` to determine which
`AgentCapability` the route requires:

| Route pattern | Required capability |
|--------------|---------------------|
| `/api/agents/:id/observation` (POST) | `BusPublish` |
| `/api/inference/*` | `Inference` |
| `/api/rpc/*` | `Tools` |
| `/api/events/ingest/*`, `/relay/*` | `BusPublish` |
| `/api/neuro/query/*` | `StoreRead` |
| `/api/neuro/*` (GET) | `StoreRead` |
| `/api/neuro/*` (POST/PUT/DELETE) | `StoreWrite` |
| `/api/learning/*` | `StoreRead` |
| Other routes | `None` (fail closed) |

If the agent's token does not include the required capability, the middleware
returns 403 with `insufficient_capability`. Routes that return `None` for
required capability also fail closed for agent tokens -- an agent token cannot
access unclassified routes.

**Identity enforcement**: for observation endpoints, the middleware additionally
verifies that the authenticated agent ID matches the target agent ID in the URL.
Agent `alice` cannot publish observations for agent `bob`.

---

## 4. Scope Enforcement

For user-facing credentials (API keys, JWTs), the middleware enforces
route-level scope requirements via the `ROUTE_SCOPE_MANIFEST`. This is a static
array of `(prefix, required_scope)` entries checked by `required_scope_for`.

The `is_scope_sufficient` function implements the scope hierarchy:
- `"admin"` satisfies any scope.
- `"write"` satisfies `"write"` and `"read"`.
- `"read"` satisfies only `"read"`.
- Scoped tokens (`"agent:write"`, `"plan:write"`, `"terminal:write"`) satisfy
  their exact scope and `"read"`.

Extension routes registered by plugins at startup are checked before the static
manifest. Unrecognized scopes map to `SCOPE_WRITE_UNCLASSIFIED`, the fail-closed
sentinel.

### 4.1 Enforcement Modes

The `enforcement_mode` config controls what happens on scope failure:

| Mode | On scope failure |
|------|-----------------|
| `Enforce` | Return 403 Forbidden |
| `Audit` | Log warning, allow request |
| `Disabled` | No check, no log |

---

## 5. Response Enrichment

After the handler completes, the middleware adds:

- `X-Auth-Method` response header: one of `api_key`, `jwt`, `bearer`,
  `worker_token`.
- `x-user-id` request header (internal): injected so downstream handlers can
  read the caller's identity without parsing extensions.

---

## 6. Audit Integration

Every significant auth event is recorded via `append_auth_audit`:

- Key expired -- `AuthAuditAction::KeyExpired`
- Token used successfully -- `AuthAuditAction::TokenUsed`
- Agent capability denied -- `AuthAuditAction::PermissionDenied` with metadata
  `required_capability` and `token_id`.

The audit log is best-effort: write failures are logged but never propagate to
the caller.

---

## References

- `crates/roko-serve/src/routes/middleware.rs` -- `require_api_key`,
  `AuthContext`, `AuthMethod`, `ROUTE_SCOPE_MANIFEST`
- `crates/roko-serve/src/routes/auth.rs` -- `AgentCredentialClaims`,
  `AuthRegistry`
- `crates/roko-serve/src/auth_audit.rs` -- `AuthAuditEvent`, `AuthAuditLog`
