# 24 -- Authentication

> Six credential paths, scope-based route authorization, four RBAC roles, and a
> structured audit trail. The auth middleware fails closed: every `/api/*`
> request must present a valid credential and satisfy the route's scope
> requirement before any handler runs. Credentials are never stored in
> plaintext -- API keys are persisted as SHA-256 hashes, agent tokens use the
> same scheme, and worker callback tokens are one-way verifiers that survive
> server restarts.

> **Implementation status (2026-09):** E35 COMPLETE (8/8). API-key lifecycle
> (create / rotate / revoke / expire), scoped agent bearer tokens, Privy/JWT
> with hardened multi-provider JWKS, four-role workspace RBAC, route-wide
> permission enforcement, parent-linked relay delegation, token-based
> invitations, and the shared auth audit trail are implemented. Device flow and
> the Cell/Graph-shaped auth pipeline remain target-architecture work outside
> the E35 manifest.

### Implementation sources

| Surface | Authority | Shipped boundary |
|---------|-----------|-----------------|
| Serve-auth middleware | `crates/roko-serve/src/routes/middleware.rs` | `require_api_key` axum middleware, 6 credential paths, scope enforcement |
| Auth routes (API keys, agent tokens, relay tokens) | `crates/roko-serve/src/routes/auth.rs` | CRUD for named keys, agent tokens (T02), relay delegation (E35-T06), `AuthRegistry` |
| Auth audit trail | `crates/roko-serve/src/auth_audit.rs` | `AuthAuditLog`, 13 action types, 30-day default retention, JSONL |
| RBAC | `crates/roko-serve/src/rbac.rs` | 4 roles (Owner/Admin/Member/Viewer), 11 permissions, `enforce_permission` |
| JWKS cache | `crates/roko-serve/src/jwks.rs` | Multi-provider JWKS with proactive refresh, coalesced fetch, 48h fail-closed |
| Auth config | `crates/roko-core/src/config/serve.rs` | `ServeAuthConfig`, `ApiKeyEntry`, `JwksProvider`, `EnforcementMode` |
| CLI auth commands | `crates/roko-cli/src/commands/auth.rs` | `cmd_login`, `cmd_logout`, `cmd_whoami` |
| Credential storage | `crates/roko-cli/src/credentials.rs` | `~/.roko/credentials.json`, 0600 permissions, profile-based |
| CLI auth helpers | `crates/roko-cli/src/auth.rs` | 4-source precedence chain, header construction |
| Worker callback auth | `crates/roko-serve/src/routes/deployments.rs` | Per-deployment opaque IDs, hashed token verifiers, constant-time comparison |

---

## 1. Credential Paths

The `require_api_key` middleware in `crates/roko-serve/src/routes/middleware.rs` is an
axum layer applied to all `/api/*` routes when `serve.auth.enabled = true` (the
secure-by-default). It evaluates six credential sources in order. The first match wins;
if none matches, the request is rejected with 401.

| Priority | Credential source | Header / transport | Scope resolution |
|----------|-------------------|-------------------|-----------------|
| 1 | Worker callback token | `X-Roko-Worker-Token` on `POST /api/deployments/:id/callback` | Fixed `"write"`, user_id = `worker:{deployment_id}` |
| 2 | `X-Api-Key` header | `X-Api-Key` | From `ApiKeyEntry.scope` or legacy `"admin"` |
| 3 | Agent bearer token (`roko_agent_` prefix) | `Authorization: Bearer roko_agent_...` | Capability-based (see Section 3) |
| 4 | Relay bearer token (`roko_relay_` prefix) | `Authorization: Bearer roko_relay_...` | Narrowed from parent capabilities |
| 5 | Bearer matched against API key entries | `Authorization: Bearer <token>` | Same as API key scope |
| 6 | Privy JWT | `Authorization: Bearer <jwt>` | From `privy_allowed_roles` / `privy_workspace_id`; rejected when neither is set (Section 7.2) |

On success, the middleware injects an `AuthContext` into request extensions:

```rust
// crates/roko-serve/src/routes/middleware.rs
pub struct AuthContext {
    /// How the caller authenticated (ApiKey, Jwt, Bearer, WorkerToken).
    pub method: AuthMethod,
    /// Permission scope (e.g. "admin", "agent:write", "read").
    pub scope: String,
    /// Optional user/key identifier.
    pub user_id: Option<String>,
}
```

Every response includes an `X-Auth-Method` header reflecting which path authenticated
the request (`api_key`, `jwt`, `bearer`, or `worker_token`).

### 1.1 Fail-Closed Semantics

The middleware rejects requests with missing or invalid credentials immediately.
Specific denial signals:

- **401 Unauthorized** -- no credential found, or credential does not match any
  stored entry.
- **401 with `X-Key-Expired: true`** -- the API key matched an entry whose
  `expires_at` has passed. This lets CLI tooling distinguish "wrong key" from
  "key needs rotation."
- **403 Forbidden** -- credential is valid but lacks the scope required for the
  requested route (see Section 5).

---

## 2. API Key Management

### 2.1 Named API Keys

Named keys are the primary server-side credential. Each key has a human-readable
name, a permission scope, optional expiry, and usage tracking.

```toml
# roko.toml -- static key declaration (merged on server startup)
[serve.auth]
enabled = true

[[serve.auth.api_keys]]
name = "github-actions"
key_hash = "e3b0c44298fc1c14..."   # SHA-256 hex of the plaintext
scope = "plan:write"
created_at = "2026-09-01T00:00:00Z"
expires_at = "2027-09-01T00:00:00Z"
```

**Storage**: keys declared in `roko.toml` are merged with the runtime registry at
`.roko/api-keys.json` on startup. The persisted registry always takes precedence
for keys that exist in both locations. Plaintext key values are never stored -- only
their SHA-256 hex hashes (`hash_api_key` in `middleware.rs`).

**Valid scopes**: `admin`, `write`, `read`, `agent:write`, `plan:write`,
`terminal:write`.

### 2.2 HTTP API

| Method | Route | Description |
|--------|-------|-------------|
| `POST` | `/api/api-keys` | Create a new named API key. The plaintext is returned **once**. |
| `GET` | `/api/api-keys` | List all keys (metadata only -- never the key itself). |
| `DELETE` | `/api/api-keys/:name` | Revoke a key by name. |
| `POST` | `/api/api-keys/:name/rotate` | Rotate: issue a new key, retain the old hash for a 5-minute grace period. |

**Rotation grace period**: when a key is rotated, the previous hash is kept in
`previous_key_hashes` with a 5-minute expiry. Requests presenting the old key
during the grace window are accepted normally. At most 2 previous hashes are
retained per key.

### 2.3 Legacy Single-Key Mode

For backwards compatibility, `serve.auth.api_key` (a single string in config) is
still supported. If a token does not match any named key, it falls back to
constant-time comparison against this legacy value. Matched tokens receive
`"admin"` scope. New deployments should use named keys instead.

Keep the key out of `roko.toml`: agents can read that file, and roko refuses to
load a readable config file that holds a secret. Set it in the
`ROKO__SERVE__AUTH__API_KEY` variable instead:
`roko config set serve.auth.api_key <key>` stores it in `.roko/.env`, which roko
loads at startup and agents cannot read. The field may also hold a `${VAR}`
reference to another variable, such as `"${ROKO_SERVE_KEY}"`, which roko expands
when it loads the config; an unset variable stops the load.

---

## 3. Agent and Relay Tokens

### 3.1 Agent Bearer Tokens

Agent tokens are scoped credentials issued to individual agents. They use a
`roko_agent_` prefix for routing in the middleware and carry an explicit
capability set.

| Capability | What it grants |
|------------|---------------|
| `Inference` | Submit inference requests to LLM backends |
| `Tools` | Invoke registered tools on behalf of an agent |
| `BusPublish` | Publish messages to the internal event bus |
| `StoreWrite` | Write to the knowledge / neuro store |
| `StoreRead` | Read from the knowledge / neuro store |

**HTTP API**:

| Method | Route | Description |
|--------|-------|-------------|
| `POST` | `/api/agent-tokens` | Issue a scoped agent bearer token (plaintext shown once) |
| `GET` | `/api/agent-tokens` | List active tokens (metadata only) |
| `DELETE` | `/api/agent-tokens/:token_id` | Revoke an agent token |

Agent tokens are fail-closed outside their declared capabilities. If a request
reaches a route for which no matching `AgentCapability` exists, the middleware
returns 403 with `insufficient_capability`. The capability-to-route mapping is
defined by `required_agent_capability` in the middleware.

### 3.2 Relay Token Delegation

Relay tokens (`roko_relay_` prefix) are parent-linked delegations that enable
agent-to-agent credential narrowing. A relay token can only carry a subset of
its parent's capabilities and has a bounded delegation depth.

```rust
// crates/roko-serve/src/routes/auth.rs
pub struct RelayToken {
    pub token_id: String,
    pub parent_token_id: String,
    pub issuer_agent_id: String,
    pub delegated_capabilities: Vec<AgentCapability>,
    pub target_agent_id: String,
    pub max_depth: u8,          // absolute max chain depth (up to 12)
    pub depth: u8,              // this token's depth below root
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
    pub token_hash: String,     // SHA-256, never plaintext
}
```

**Revocation cascade**: revoking an agent token also cascades revocation to all
relay tokens descended from it. The cascade is applied atomically: the root
revocation commits first, then the materialized relay update.

**HTTP API**:

| Method | Route | Description |
|--------|-------|-------------|
| `POST` | `/api/relay-tokens` | Issue a narrowed, parent-linked delegation |
| `DELETE` | `/api/relay-tokens/:token_id` | Revoke a delegation and its descendants |

---

## 4. Worker Callback Authentication

Deployed workers (created via `POST /api/deployments`) authenticate their result
callbacks with deployment-scoped, opaque credentials. This is a separate auth
path from user-facing API keys.

### 4.1 Credential Lifecycle

1. **Generation**: when `create_deployment` runs, it generates two UUIDs --
   a `callback_id` (opaque routing identifier) and a `callback_token` (secret).
   Both are injected into the worker's environment as `ROKO_DEPLOYMENT_ID` and
   `ROKO_WORKER_CALLBACK_TOKEN`.

2. **Hashing**: the plaintext token is hashed via `callback_token_hash`
   (which delegates to `hash_api_key`, i.e. SHA-256 hex). The hash is persisted
   in `deployment.callback_token_hash`. The plaintext is retained only in memory
   for the lifetime of the server process.

3. **Verification**: when the worker calls `POST /api/deployments/:id/callback`,
   the middleware extracts `X-Roko-Worker-Token`, hashes it, and compares the
   result against the stored hash using constant-time byte comparison
   (`token_eq` with `core::hint::black_box`).

4. **Restart safety**: because the hash is persisted alongside the deployment
   record, callback authentication survives server restarts even though the
   plaintext token is lost from memory.

### 4.2 Fallback Token

Workers created by the standalone `roko server deploy` CLI (not via the HTTP
API) use a process-wide `ROKO_WORKER_CALLBACK_TOKEN`. This token is checked as
a fallback when no per-deployment verifier matches, maintaining backwards
compatibility. Missing auth configuration is never treated as permission to
accept an unauthenticated callback.

---

## 5. Scope-Based Route Authorization

After a credential is validated, the middleware checks whether the caller's scope
is sufficient for the requested route. A static `ROUTE_SCOPE_MANIFEST` in the
middleware maps every mutating route prefix to a required scope.

### 5.1 Scope Hierarchy

```
admin > write > read
admin > agent:write
admin > plan:write
admin > terminal:write
```

An `"admin"` scope satisfies any route requirement. `"write"` satisfies `"write"`
and `"read"`. Scoped tokens like `"agent:write"` only satisfy their exact scope
and `"read"`.

### 5.2 Route Scope Manifest (excerpt)

| Route prefix | Required scope |
|--------------|---------------|
| `/api/api-keys` | `admin` |
| `/api/agent-tokens` | `admin` |
| `/api/secrets` | `admin` |
| `/api/config` | `admin` |
| `/api/registries` | `admin` |
| `/api/events/ingest` | `agent:write` |
| `/api/agents` | `agent:write` |
| `/api/relay` | `agent:write` |
| `/api/plans` | `plan:write` |
| `/api/prd` | `plan:write` |
| `/api/terminal` | `terminal:write` |
| `/api/deployments` | `write` |
| `/api/jobs` | `write` |
| `/api/run` | `write` |
| All GET/HEAD/OPTIONS | `read` |

Extension routes registered by plugins at startup are checked against the same
scope vocabulary via `register_extension_route_scopes`. Unclassified mutating
routes fall through to `SCOPE_WRITE_UNCLASSIFIED`, which is the fail-closed
sentinel.

### 5.3 Enforcement Modes

Configurable via `serve.auth.enforcement_mode` in `roko.toml`:

| Mode | Behavior |
|------|----------|
| `enforce` (default) | Block requests that fail scope checks (403) |
| `audit` | Log the violation but allow the request through |
| `disabled` | Skip scope checks entirely -- no logging, no blocking |

---

## 6. RBAC: Workspace Roles and Permissions

Four workspace roles with strictly ordered permissions:

```
Owner > Admin > Member > Viewer
```

### 6.1 Permission Matrix

| Permission | Owner | Admin | Member | Viewer |
|------------|-------|-------|--------|--------|
| PlanExecute | Y | Y | Y | |
| PlanCreate | Y | Y | Y | |
| AgentSpawn | Y | Y | Y | |
| AgentStop | Y | Y | Y | |
| ConfigEdit | Y | Y | | |
| SecretsRead | Y | Y | | |
| SecretsWrite | Y | Y | | |
| TeamManage | Y | | | |
| ApiKeyCreate | Y | Y | | |
| TokenIssue | Y | Y | | |
| ViewDashboard | Y | Y | Y | Y |

Permissions are enforced by `enforce_permission` in `crates/roko-serve/src/rbac.rs`,
which is called from route handlers for operations that require discrete
authorization beyond scope checks.

---

## 7. JWT / JWKS Verification

Privy JWTs are verified via a cached JWKS endpoint. The JWKS cache
(`crates/roko-serve/src/jwks.rs`) supports multiple issuer-bound providers.

Privy JWT auth is off by default. `roko serve` does not fill in a Privy app
ID; the operator sets `serve.auth.privy_app_id` in `roko.toml`, together with
an allow-list (Section 7.2). Anyone can sign in to a Privy app, so a valid
token for the app ID alone grants nothing.

### 7.1 Cache Lifecycle

| Parameter | Value |
|-----------|-------|
| Refresh interval | 1 hour (`CACHE_TTL`) |
| Staleness warning | 24 hours (`MAX_STALE`) |
| Fail-closed threshold | 48 hours (`FAIL_CLOSED_STALE`) |
| Default fetch timeout | 10 seconds |

Keys are refreshed proactively via a background task. On unknown `kid`, a
coalesced on-demand refresh is triggered so that key rotation does not cause
a request-driven refresh storm.

### 7.2 JWT Validation Chain

1. **Allow-list** -- `privy_allowed_roles` or `privy_workspace_id` must be
   configured. Without either, every Privy JWT is rejected (fail closed) and
   the server logs a warning naming both settings.
2. **Signature + app-id** -- verified against the JWKS keyset for the configured
   `privy_app_id`.
3. **Workspace membership** -- if `privy_workspace_id` is configured, the JWT
   `org_id` claim must match. Tokens without an `org_id` claim are rejected
   (fail closed). With no `privy_allowed_roles`, members receive `"admin"`.
4. **Role authorization** -- if `privy_allowed_roles` is non-empty, the JWT
   `role` claim must be present and contained in the allowed list. Tokens with
   an unrecognized or missing role are downgraded to `"read"` scope instead of
   receiving `"admin"`.

### 7.3 Additional JWKS Providers

```toml
# roko.toml
[[serve.auth.jwks_providers]]
url = "https://example.com/.well-known/jwks.json"
expected_issuer = "example.com"
```

An empty `jwks_providers` list uses Privy's per-app endpoint for
`privy_app_id`, `https://auth.privy.io/api/v1/apps/<privy_app_id>/jwks.json`
(`jwks::privy_jwks_url`). Privy's generic `/.well-known/jwks.json` returns 404.
A non-empty list replaces that default, so list the Privy endpoint as well when
you add another provider.

Missing or stale keys stop JWT sign-in, but they do not make the server
unhealthy: `GET /api/health` reports them in its `jwks` section and returns
`degraded` with HTTP 200, so a liveness probe does not restart the server during
an identity-provider outage.

---

## 8. CLI Authentication

Three CLI commands manage client-side credentials stored at
`~/.roko/credentials.json`:

### 8.1 `roko login [url]`

Two modes:

- **Browser flow** (default): starts a localhost callback server on a random
  port, opens `{dashboard_url}/cli/auth?port={port}`, and waits up to 5 minutes
  for Privy to post an `access_token` back. The credential is stored with
  `method: "privy"`.

- **API key flow** (`--api-key`): prompts for a key (with echo suppression via
  raw terminal mode), validates it against `GET {url}/api/health`, and stores it
  with `method: "api_key"`.

- **Check mode** (`--api-key --check`): non-interactive validation of an existing
  stored credential. Exits 0 if valid, 1 if invalid or missing.

### 8.2 `roko logout`

Removes `~/.roko/credentials.json`. No server-side state is modified.

### 8.3 `roko whoami`

Displays the stored credential metadata:

```
server:     http://localhost:6677
method:     api_key
stored at:  2026-09-01T12:00:00Z
token:      rk_abc12...
status:     valid
```

For Privy credentials, also shows `user`, `email`, and `via` (login method).
For API key credentials, validates liveness against the server.

### 8.4 Credential File Format

```json
{
  "default": {
    "url": "http://localhost:6677",
    "token": "rk_...",
    "method": "api_key",
    "stored_at": "2026-09-01T12:00:00Z"
  }
}
```

The file is written with 0600 permissions on Unix. Writes use atomic
temp-file-then-rename to prevent partial writes.

**Deprecated field**: `wallet_address` -- this field remains in the `Credential`
struct for deserialization compatibility but is no longer populated. It was used
for on-chain identity binding which has been removed from the authentication
surface.

### 8.5 API Key Resolution Precedence

The CLI resolves credentials from four sources in strict order:

1. `--api-key` CLI flag
2. `ROKO_API_KEY` environment variable
3. `serve.auth.api_key` in the loaded config, which `ROKO__SERVE__AUTH__API_KEY`
   sets (for example in `.roko/.env`)
4. Stored credential from `~/.roko/credentials.json` (`roko login`)

The first non-empty source wins. This chain is implemented in `resolve_api_key`
(`crates/roko-cli/src/auth.rs`) and shared by all CLI paths that contact the
server (chat, doctor, TUI, etc.).

---

## 9. Auth Audit Trail

Every authentication and authorization event is appended to
`.roko/auth-audit.jsonl` as a structured JSONL record.

### 9.1 Record Format

```json
{
  "timestamp": "2026-09-01T10:00:00Z",
  "actor":     "api-key:github-actions",
  "action":    "TokenUsed",
  "target":    "POST /api/plans/run",
  "outcome":   "success",
  "ip":        "127.0.0.1",
  "user_agent": "roko-cli/0.1",
  "metadata":  {}
}
```

### 9.2 Audit Actions

| Action | When |
|--------|------|
| `Login` | User/key successfully authenticated |
| `TokenIssued` | New API key or agent token created |
| `TokenRevoked` | Token/key explicitly revoked |
| `TokenRotated` | Key rotated (old hash retired) |
| `TokenUsed` | Token/key accepted for a request |
| `PermissionGranted` | Permission check passed |
| `PermissionDenied` | Permission check failed |
| `InviteCreated` | Workspace invitation created |
| `InviteAccepted` | Invitation accepted |
| `InviteExpired` | Invitation expired |
| `KeyExpired` | Key rejected due to expiry |
| `RoleChanged` | Team member role changed |
| `TokenDelegated` | Relay token delegated |

### 9.3 Retention

`AuthAuditLog::sweep_old_entries` discards records older than the configured
retention period (default 30 days). Call it periodically or from a maintenance
endpoint to prevent unbounded growth.

---

## 10. Configuration Reference

```toml
[serve.auth]
# Whether /api/* routes require credentials (default: true).
enabled = true

# Legacy single API key (prefer named api_keys below). Never write it in
# roko.toml, which agents can read: set ROKO__SERVE__AUTH__API_KEY in .roko/.env,
# as `roko config set serve.auth.api_key <key>` does.

# Privy application ID for JWT validation.
privy_app_id = "cmhw01vut003tjx0d5lmqc8zs"

# Workspace/org membership requirement.
privy_workspace_id = "org_abc123"

# Allowed JWT roles. With this and privy_workspace_id both unset, Privy JWTs
# are rejected.
privy_allowed_roles = ["admin", "operator"]

# Enforcement mode: "enforce" | "audit" | "disabled"
enforcement_mode = "enforce"

# Invitation expiry (days).
invite_expiry_days = 7

# Named API keys.
[[serve.auth.api_keys]]
name = "ci-pipeline"
key_hash = "..."
scope = "plan:write"
created_at = "2026-09-01T00:00:00Z"

# JWKS providers. When set, they replace the default Privy endpoint for
# privy_app_id, so keep it in the list.
[[serve.auth.jwks_providers]]
url = "https://auth.privy.io/api/v1/apps/cmhw01vut003tjx0d5lmqc8zs/jwks.json"
expected_issuer = "privy.io"
```

---

## 11. Verification

```bash
# Authenticate with API key
roko login --api-key http://localhost:6677

# Authenticate via browser (Privy)
roko login http://localhost:6677

# Check stored credential
roko whoami

# Verify credential is still valid (non-interactive)
roko login --api-key --check

# Remove stored credentials
roko logout
```

Test that auth is enforced:

```bash
# Should return 401 (no credential)
curl -s -o /dev/null -w "%{http_code}" http://localhost:6677/api/health

# Should return 200 (valid key)
curl -s -o /dev/null -w "%{http_code}" \
  -H "X-Api-Key: $ROKO_API_KEY" \
  http://localhost:6677/api/health

# Create a scoped key via API
curl -X POST http://localhost:6677/api/api-keys \
  -H "X-Api-Key: $ROKO_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"name": "readonly", "scope": "read"}'
```

---

## 12. Depth Files

| File | Contents |
|------|----------|
| [depth/24-auth/middleware.md](depth/24-auth/middleware.md) | Full middleware request flow, credential extraction, scope enforcement |
| [depth/24-auth/tokens.md](depth/24-auth/tokens.md) | API key, agent token, and relay token lifecycle and storage |
| [depth/24-auth/cli-credentials.md](depth/24-auth/cli-credentials.md) | CLI login/logout/whoami, credential file format, resolution precedence |

---

## References

- `crates/roko-serve/src/routes/middleware.rs` -- serve-auth middleware (`require_api_key`)
- `crates/roko-serve/src/routes/auth.rs` -- API key, agent token, and relay token routes
- `crates/roko-serve/src/auth_audit.rs` -- structured auth audit log
- `crates/roko-serve/src/rbac.rs` -- role-based access control
- `crates/roko-serve/src/jwks.rs` -- JWKS cache and JWT verification
- `crates/roko-serve/src/routes/deployments.rs` -- worker callback authentication
- `crates/roko-core/src/config/serve.rs` -- `ServeAuthConfig`, `ApiKeyEntry`, `EnforcementMode`
- `crates/roko-cli/src/commands/auth.rs` -- login/logout/whoami CLI handlers
- `crates/roko-cli/src/credentials.rs` -- file-based credential storage
- `crates/roko-cli/src/auth.rs` -- API key resolution and header construction
