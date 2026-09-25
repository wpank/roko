# 24-auth / tokens

> API key lifecycle, agent bearer tokens, relay token delegation, worker
> callback tokens, and the `AuthRegistry` that backs them all.

---

## 1. AuthRegistry

The `AuthRegistry` struct in `crates/roko-serve/src/routes/auth.rs` is the
in-memory credential store. It holds three token collections behind
`tokio::sync::RwLock`s, backed by atomically replaced JSON files on disk.

```rust
pub(crate) struct AuthRegistry {
    workdir: PathBuf,
    api_keys: RwLock<Vec<ApiKeyEntry>>,
    agent_tokens: RwLock<Vec<AgentToken>>,
    relay_tokens: RwLock<Vec<RelayToken>>,
}
```

### 1.1 Startup Merge

`AuthRegistry::load` reads persisted JSON files from the workspace directory and
merges any `api_keys` declared in `roko.toml` that are not already present on
disk. Invalid registry JSON fails server startup rather than silently disabling
credentials.

### 1.2 Atomic Persistence

All mutations (create, rotate, revoke) follow the same pattern:

1. Acquire the write lock.
2. Clone the current state.
3. Apply the mutation.
4. Write a temporary file (`.{name}.{uuid}.tmp`).
5. `fsync` the temporary file.
6. Atomic rename to the target path.
7. Update the in-memory state.
8. If the write fails, remove the temp file and return an error.

This guarantees that a crash at any point leaves either the old or new file
intact, never a partial write.

---

## 2. API Key Lifecycle

### 2.1 Data Model

```rust
// crates/roko-core/src/config/serve.rs
pub struct ApiKeyEntry {
    pub name: String,                           // human-readable, unique
    pub key_hash: String,                       // SHA-256 hex of plaintext
    pub scope: String,                          // "admin", "write", "read", etc.
    pub created_at: String,                     // ISO 8601
    pub expires_at: Option<String>,             // optional ISO 8601
    pub last_used_at: Option<String>,           // updated on each use
    pub previous_key_hashes: Vec<(String, String)>, // (hash, grace_expires_at)
}
```

Stored at `.roko/api-keys.json`.

### 2.2 Creation

`POST /api/api-keys` generates 32 random bytes, base64url-encodes them with a
`roko_` prefix, hashes the result, and persists only the hash. The plaintext key
is returned in the response body **once** and is never stored.

### 2.3 Rotation

`POST /api/api-keys/:name/rotate`:

1. Generates a new random key.
2. Moves the current `key_hash` to `previous_key_hashes` with a 5-minute
   grace-period expiry.
3. Sets the new hash as `key_hash`.
4. Clears `last_used_at`.
5. Prunes any expired entries from `previous_key_hashes` (max 2 retained).

During the grace period, both old and new keys are accepted. After expiry, only
the new key works.

### 2.4 Expiry

Keys with `expires_at` in the past are rejected with 401 and the response header
`X-Key-Expired: true`. This header lets client tooling distinguish "wrong key"
from "key needs rotation" without parsing the error body.

### 2.5 Usage Tracking

On each successful authentication, `record_api_key_use` updates the
`last_used_at` timestamp in the persisted registry. This write is best-effort:
failures are logged but do not block the request.

---

## 3. Agent Bearer Tokens

### 3.1 Data Model

```rust
// crates/roko-serve/src/routes/auth.rs
pub struct AgentToken {
    pub token_id: String,
    pub agent_id: String,
    pub capabilities: Vec<AgentCapability>,
    pub issued_at: String,
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
    pub token_hash: String,  // SHA-256 hex
}
```

Stored at `.roko/agent-tokens.json`.

### 3.2 Token Format

Agent tokens use the prefix `roko_agent_` followed by 32 random bytes
base64url-encoded. The prefix enables the middleware to route them directly to
the agent token validator without attempting API key or JWT checks.

### 3.3 Capabilities

Five discrete capabilities control which API surfaces the token can access:

| Capability | Routes granted |
|------------|---------------|
| `Inference` | `/api/inference/*` |
| `Tools` | `/api/rpc/*` |
| `BusPublish` | `/api/events/ingest/*`, `/relay/*`, `/api/agents/:id/observation` |
| `StoreWrite` | `/api/neuro/*` (mutating methods) |
| `StoreRead` | `/api/neuro/*` (GET), `/api/learning/*` |

A token must hold the capability matching the target route. Unclassified routes
fail closed for agent tokens.

### 3.4 Revocation

`DELETE /api/agent-tokens/:token_id` sets `revoked = true` and triggers a
cascade: all relay tokens descended from this agent token are also revoked.
The cascade commits the root revocation first so that even if the relay write
encounters an I/O error, the root token is already invalid.

---

## 4. Relay Token Delegation

### 4.1 Data Model

```rust
pub struct RelayToken {
    pub token_id: String,
    pub parent_token_id: String,
    pub issuer_agent_id: String,
    pub delegated_capabilities: Vec<AgentCapability>,
    pub target_agent_id: String,
    pub max_depth: u8,
    pub depth: u8,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
    pub token_hash: String,
}
```

Stored at `.roko/relay-tokens.json`.

### 4.2 Delegation Constraints

- **Capability narrowing**: delegated capabilities must be a subset of the
  parent's capabilities.
- **Depth budget**: `max_depth - depth` must be positive. The maximum chain
  depth is 12 (`MAX_RELAY_MAX_DEPTH`). Default relay TTL is 5 minutes
  (`DEFAULT_RELAY_TOKEN_TTL_SECS = 300`).
- **Expiry**: a relay token cannot outlive its parent.
- **Identity**: each relay is bound to a specific `target_agent_id`.

### 4.3 Authentication

Relay tokens use the `roko_relay_` prefix. The middleware delegates to
`authenticate_relay_secret`, which validates: hash match, not revoked, not
expired, parent chain not revoked, and depth within bounds. On success, the
claims carry the relay's narrowed capabilities and depth.

### 4.4 Revocation Cascade

When an agent token or relay token is revoked, `cascade_relay_revocation`
recursively marks all descendant relay tokens as revoked. The cascade is
materialized and persisted atomically.

---

## 5. Worker Callback Tokens

### 5.1 Generation

Each `POST /api/deployments` call generates:

- `callback_id`: UUID used as the routing key in callback URLs.
- `callback_token`: UUID used as the authentication secret.

Both are injected into the worker environment as `ROKO_DEPLOYMENT_ID` and
`ROKO_WORKER_CALLBACK_TOKEN`.

### 5.2 Storage

The hash of the callback token is stored in `deployment.callback_token_hash`
(persisted to disk). The plaintext is stored in `deployment.callback_token`
(memory only, never serialized to disk). After a server restart, only the hash
survives.

### 5.3 Verification

`authenticate_worker_callback_token` in `deployments.rs`:

1. Looks up the deployment by `callback_id` (tries both direct lookup and
   scanning `callback_id` fields).
2. Resolves the expected hash: from the deployment record, or from
   `deployment.callback_token` if no hash is stored yet, or from the
   process-wide fallback token.
3. Hashes the supplied token and compares using constant-time byte comparison
   (`token_eq` with `core::hint::black_box`) to prevent timing side channels.

Returns the canonical provider deployment ID on success.

---

## References

- `crates/roko-serve/src/routes/auth.rs` -- `AuthRegistry`, `AgentToken`,
  `RelayToken`, `AgentCapability`
- `crates/roko-core/src/config/serve.rs` -- `ApiKeyEntry`
- `crates/roko-serve/src/routes/deployments.rs` --
  `authenticate_worker_callback_token`, `callback_token_hash`
- `crates/roko-serve/src/routes/middleware.rs` -- `hash_api_key`,
  `match_api_key_entry`
