# 24-auth / cli-credentials

> CLI login/logout/whoami flows, credential file format, API key resolution
> precedence, and the deprecated `wallet_address` field.

---

## 1. Credential Storage

### 1.1 File Location

All CLI credentials are stored at `~/.roko/credentials.json`. The `~/.roko/`
directory is created on first write if it does not exist.

### 1.2 File Format

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

The outer object maps profile names to credential entries. Currently only the
`"default"` profile is used.

### 1.3 Credential Struct

```rust
// crates/roko-cli/src/credentials.rs
pub struct Credential {
    pub url: String,              // roko-serve URL
    pub token: String,            // API key or Privy access token
    pub method: String,           // "api_key" or "privy"
    pub stored_at: String,        // ISO 8601
    pub privy_user_id: Option<String>,  // e.g. "did:privy:cm..."
    pub email: Option<String>,
    pub wallet_address: Option<String>, // DEPRECATED
    pub login_method: Option<String>,   // e.g. "email", "google"
}
```

### 1.4 Security

- **Unix permissions**: the file is written with mode 0600 (owner read/write
  only) via `OpenOptionsExt::mode`.
- **Atomic writes**: credentials are written to a `.json.tmp` file, fsynced,
  then atomically renamed to prevent partial writes on crash.
- **No plaintext in logs**: the `whoami` command masks tokens to the first 8
  characters followed by `...`.

### 1.5 Deprecated: `wallet_address`

The `wallet_address` field remains in the `Credential` struct for
deserialization compatibility with existing credential files. It is annotated
with `#[serde(default, skip_serializing_if = "Option::is_none")]` and is no
longer populated by any login flow.

This field was used for on-chain identity binding when the authentication
surface included wallet-based credentials. Chain-based identity has been
separated from HTTP authentication. Existing credential files that contain
this field will continue to load without error, but the value is not used.

---

## 2. Login Flows

### 2.1 Browser-Based Login (Default)

`roko login [url]` (without `--api-key`):

1. Starts a localhost callback server on a random port:
   ```
   127.0.0.1:{random_port}
   ```
2. Opens `{dashboard_url}/cli/auth?port={port}` in the default browser.
3. The dashboard page completes Privy authentication and POSTs a
   `CallbackPayload` back to `/callback`:
   ```json
   {
     "access_token": "eyJ...",
     "privy_user_id": "did:privy:cm...",
     "email": "user@example.com",
     "wallet_address": null,
     "login_method": "email"
   }
   ```
4. The CLI stores the credential with `method: "privy"` and shuts down the
   callback server.
5. Timeout: 5 minutes. If no callback arrives, the command exits with an error.

The callback server handles CORS preflight (`OPTIONS /callback`) to support
cross-origin POST from the dashboard frontend.

### 2.2 API Key Login

`roko login --api-key [url]`:

1. Prompts for the API key on stdin with echo suppression (raw terminal mode
   via crossterm's `RawModeGuard`).
2. Validates the key against `GET {url}/api/health` with the `X-Api-Key`
   header.
3. On 200: stores the credential with `method: "api_key"`.
4. On 401/403: prints an error and exits with failure.
5. On connection error: prints the error and exits.

**Non-interactive mode** (`--api-key --check`): validates the existing stored
credential without prompting. Exits 0 if valid, 1 if invalid or missing.

### 2.3 Credential Validation

`validate_credential` in `crates/roko-cli/src/commands/auth.rs`:

```rust
pub(crate) async fn validate_credential(url: &str, token: &str) -> Result<bool>
```

Sends `GET {url}/api/health` with `X-Api-Key: {token}`, 10-second timeout.
Returns:
- `Ok(true)` on 2xx
- `Ok(false)` on 401/403
- `Err` on connection failure or unexpected status

---

## 3. Logout

`roko logout`:

- Removes `~/.roko/credentials.json` if it exists.
- No server-side state is modified (tokens are not revoked).
- Prints the path of the removed file, or "no stored credentials found" if
  the file did not exist.

---

## 4. Whoami

`roko whoami`:

Loads and displays the stored credential:

```
server:     http://localhost:6677
method:     api_key
stored at:  2026-09-01T12:00:00Z
token:      rk_abc12...
status:     valid
```

For Privy credentials, additional fields:
```
user:       did:privy:cm...
email:      user@example.com
via:        email
```

For API key credentials, `whoami` calls `validate_credential` to check liveness
and displays one of: `valid`, `invalid (server returned 401)`, or
`unreachable ({error})`.

---

## 5. API Key Resolution Precedence

The `resolve_api_key` function in `crates/roko-cli/src/auth.rs` provides a
single source of truth for API key resolution across all CLI paths:

| Priority | Source | Label |
|----------|--------|-------|
| 1 | `--api-key` CLI flag | `CLI flag (--api-key)` |
| 2 | `ROKO_API_KEY` env var | `ROKO_API_KEY env var` |
| 3 | `serve.auth.api_key` in `roko.toml` | `roko.toml [serve.auth]` |
| 4 | `~/.roko/credentials.json` | `~/.roko/credentials.json (roko login)` |

The first non-empty source wins. Empty strings and whitespace-only values are
treated as absent.

### 5.1 Auth Method Selection

The resolved credential's HTTP transport method depends on its source:

- Sources 1-3 (CLI flag, env var, config): sent via `X-Api-Key` header
  (`AuthMethod::ApiKey`).
- Source 4 (stored credential): determined by the `method` field in the
  credential file. `"privy"` uses `Authorization: Bearer {token}`
  (`AuthMethod::Bearer`); all other methods use `X-Api-Key`.

### 5.2 Header Construction

`auth_headers_with_method` in `crates/roko-cli/src/auth.rs`:

```rust
pub fn auth_headers_with_method(credential: &str, method: AuthMethod) -> HeaderMap {
    let mut headers = HeaderMap::new();
    match method {
        AuthMethod::ApiKey => headers.insert("X-Api-Key", value),
        AuthMethod::Bearer => headers.insert(AUTHORIZATION, "Bearer {credential}"),
    }
    headers
}
```

This is called by `ResolvedApiKey::headers()` and shared by chat, doctor, TUI,
and all other CLI paths that contact `roko-serve`.

---

## References

- `crates/roko-cli/src/credentials.rs` -- `Credential`, `store_credential`,
  `load_credential`, `clear_credential`
- `crates/roko-cli/src/commands/auth.rs` -- `cmd_login`, `cmd_logout`,
  `cmd_whoami`, `validate_credential`
- `crates/roko-cli/src/auth.rs` -- `resolve_api_key`, `auth_headers`,
  `ResolvedApiKey`, `ApiKeySource`
