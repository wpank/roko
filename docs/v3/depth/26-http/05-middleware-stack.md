# 26.05 -- Middleware Stack

> Depth file for [26-HTTP.md](../../26-HTTP.md).

---

## Layer Order

Axum middleware layers execute in reverse declaration order. The `build_router()`
function in `mod.rs` stacks them so the outermost layer runs first on inbound
requests:

```
Inbound request
  |
  v
[1] CORS                    -- tower_http CorsLayer
  |
  v
[2] Tracing                 -- tower_http TraceLayer
  |
  v
[3] Per-Caller Rate Limit   -- keyed_rate_limit_middleware (30 req/s per key)
  |
  v
[4] Global Rate Limit       -- rate_limit_middleware (100 req/s total)
  |
  v
[5] Body Size Limit         -- DefaultBodyLimit (4 MiB)
  |
  v
[6] Secret Scrubbing        -- scrub_secrets (response filter)
  |
  v
[7] API Key Authentication  -- require_api_key (when auth enabled)
  |
  v
[8] Scope Enforcement       -- require_scope (when auth enabled)
  |
  v
[9] RBAC Permission Check   -- require_route_permission (when auth enabled)
  |
  v
[10] Per-Route Rate Limits   -- keyed_rate_limit_middleware (specific routes)
  |
  v
  Handler
```

## Rate Limiting

### Global Backstop

A single shared token bucket (`GlobalRateLimiter`) caps total throughput:

```rust
pub(crate) const DEFAULT_GLOBAL_RATE_PER_SEC: u32 = 100;
```

Exceeding the bucket returns 429 with a `Retry-After` header (in seconds,
minimum 1).

### Per-Caller Keyed Limit

A keyed rate limiter (`KeyedRateLimiter`) bounds each caller independently.
The key is resolved in priority order:

1. `X-Api-Key` header -- hashed with SHA-256 via `hash_api_key()`
2. `Authorization: Bearer` token -- hashed with SHA-256
3. `X-Forwarded-For` / `X-Real-Ip` header -- leftmost IP
4. `ConnectInfo<SocketAddr>` -- connected peer address
5. `"anon"` -- fallback constant

```rust
pub(crate) const DEFAULT_PER_KEY_RATE_PER_SEC: u32 = 30;
```

Raw API keys are never stored or logged; only their SHA-256 hashes are used
for rate-limit keying.

### Per-Route Rate Limits

Expensive route groups have additional keyed limits:

| Route Group | Rate | Burst | Limiter |
|---|---|---|---|
| Terminal creation | 2/min | 3 | `terminal_create_limiter` |
| Inference dispatch (`/run`, `/inference`) | 30/min | 10 | `infer_limiter` |
| Agent registration (`/agents`) | 5/min | 5 | `agent_reg_limiter` |

These are applied via `axum::middleware::from_fn_with_state` on specific
route merges:

```rust
.merge(run::routes().layer(axum::middleware::from_fn_with_state(
    Arc::clone(&infer_limiter),
    keyed_rate_limit_middleware,
)))
```

### Rate Limit Responses

All rate limit responses share the same format:

```json
{
  "code": "rate_limited",
  "message": "per-caller rate limit exceeded"
}
```

Status: 429 Too Many Requests, with `Retry-After: <seconds>` header.

## Authentication

### Credential Sources

The `require_api_key` middleware accepts six credential sources in order:

1. `X-Roko-Worker-Token` -- deployment worker callbacks only
2. `X-Api-Key` header -- direct API key
3. `Authorization: Bearer roko_agent_*` -- named agent tokens
4. `Authorization: Bearer roko_relay_*` -- relay delegation tokens
5. `Authorization: Bearer <key>` -- matched against API key registry
6. `Authorization: Bearer <jwt>` -- Privy JWT verification via JWKS

### API Key Matching

Named API keys are stored as SHA-256 hashes with support for key rotation:

```rust
enum ApiKeyMatchResult<'a> {
    Valid(&'a ApiKeyEntry),      // Active key matched
    Expired(&'a ApiKeyEntry),    // Key past expires_at
    GracePeriod(&'a ApiKeyEntry),// Previous hash within 5-min grace
    NotFound,                    // No match
}
```

Expired keys return 401 with `X-Key-Expired: true` so clients can distinguish
rotation-needed from wrong-key errors.

### Auth Methods

On success, an `AuthContext` is injected into request extensions:

```rust
pub struct AuthContext {
    pub method: AuthMethod,  // ApiKey | Jwt | Bearer | WorkerToken
    pub scope: String,       // "admin", "read", "write", "agent:write", etc.
    pub user_id: Option<String>,
}
```

The `X-Auth-Method` response header is set for client introspection.

## Scope Enforcement

The `require_scope` middleware maps each API path to a required scope:

| Scope | Route Pattern |
|---|---|
| `read` | `GET /api/health`, `GET /api/status`, `GET /api/learning/*` |
| `write` | Most mutation endpoints |
| `admin` | `POST /api/config`, `DELETE /api/agents/*` |
| `agent:write` | Agent observation and inference endpoints |
| `plan:write` | Plan creation and execution |
| `terminal:write` | PTY terminal sessions |

Extension routes register their scope requirements at startup via
`register_extension_route_scopes()`. Unrecognised routes fall through to a
fail-closed `SCOPE_WRITE_UNCLASSIFIED` sentinel.

## RBAC Permission Enforcement

The typed RBAC layer (`require_route_permission` in `rbac_middleware.rs`) maps
routes to specific permissions. This runs after credential and scope checks:

```rust
const ROUTE_PERMISSION_MANIFEST: &[RoutePermission] = &[
    RoutePermission { prefix: "/api/auth/audit",   permission: Permission::SecretsRead },
    RoutePermission { prefix: "/api/api-keys",      permission: Permission::ApiKeyCreate },
    RoutePermission { prefix: "/api/agent-tokens",  permission: Permission::TokenIssue },
    RoutePermission { prefix: "/api/team",          permission: Permission::TeamManage },
    RoutePermission { prefix: "/api/secrets",       permission: Permission::SecretsWrite },
    RoutePermission { prefix: "/api/config",        permission: Permission::ConfigEdit },
    RoutePermission { prefix: "/api/plans",         permission: Permission::PlanCreate },
    RoutePermission { prefix: "/api/agents",        permission: Permission::AgentSpawn },
    RoutePermission { prefix: "/api/arenas",        permission: Permission::PlanCreate },
    RoutePermission { prefix: "/api/defi",          permission: Permission::PlanExecute },
    RoutePermission { prefix: "/api/terminal",      permission: Permission::AgentSpawn },
    // ... more entries
];
```

JWT callers resolve their role from the persisted team membership registry, not
from claims. Agent and worker credentials bypass the RBAC hierarchy.

Denied requests return 403:

```json
{
  "error": "forbidden",
  "permission": "plan_create",
  "role": "viewer"
}
```

All denials are logged and appended to the auth audit log.

## CORS

The `cors_layer()` function builds a `tower_http::cors::CorsLayer`:

```rust
pub(crate) struct CorsPolicy {
    pub origins: Vec<String>,     // Explicit allowed origins
    pub unsafe_public: bool,      // Allow any origin (dev only)
    pub auth_enabled: bool,       // Include auth headers in CORS
}
```

When `unsafe_public_cors = true`, a one-time warning is logged:

> "unsafe_public_cors is enabled -- ANY origin may call the API"

Allowed methods: GET, POST, PUT, PATCH, DELETE, OPTIONS.
Allowed headers: Content-Type, Authorization, X-Api-Key, Accept.

## Secret Scrubbing

The `scrub_secrets` middleware intercepts JSON responses and redacts API keys
and tokens using the shared `LogScrubber`. This prevents accidental credential
leakage in debug or diagnostic endpoints.

## Body Size Limits

```rust
pub(crate) const DEFAULT_REQUEST_BODY_LIMIT_BYTES: usize = 4 * 1024 * 1024;
```

The global 4 MiB cap applies to all routes. Webhook routes that accept opaque
`Bytes` payloads enforce a tighter 1 MiB limit locally.

## Source

- `crates/roko-serve/src/routes/mod.rs` -- Layer stacking and rate limiter construction
- `crates/roko-serve/src/routes/middleware.rs` -- Auth, CORS, scope, scrubbing
- `crates/roko-serve/src/routes/rbac_middleware.rs` -- Typed RBAC enforcement
- `crates/roko-serve/src/routes/route_permissions.rs` -- Route-to-permission manifest
