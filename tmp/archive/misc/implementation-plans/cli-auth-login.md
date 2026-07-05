# `roko auth login` — Browser-based CLI Authentication

## Overview

Standard CLI auth flow: `roko auth login` opens browser, user authenticates via Privy, credentials stored locally. Same pattern as `gh auth login`, `gcloud auth login`, `anthropic auth`.

## Flow

```
$ roko auth login
Opening browser to authenticate...
Waiting for login... ✓
Authenticated as will@nunchi.dev (did:privy:cm...)
Credentials saved to ~/.roko/credentials.json
```

## Implementation

### 1. Credential storage: `~/.roko/credentials.json`

```json
{
  "privy_user_id": "did:privy:cmXXX",
  "access_token": "eyJ...",
  "email": "will@nunchi.dev",
  "wallet_address": "0x90F7...b906",
  "login_method": "email",
  "authenticated_at": "2026-04-24T10:00:00Z"
}
```

### 2. CLI commands

| Command | What it does |
|---------|-------------|
| `roko auth login` | Open browser, authenticate, store credentials |
| `roko auth status` | Show current auth state (who's logged in, token expiry) |
| `roko auth logout` | Clear stored credentials |
| `roko auth token` | Print the current access token (for scripting) |

### 3. `roko auth login` implementation

**File:** `crates/roko-cli/src/auth.rs` (new)

```
1. Bind a tiny axum server on 127.0.0.1:0 (OS-assigned random port)
2. Read the assigned port
3. Open browser to: https://app.nunchi.dev/cli/auth?port={port}
   - Dev fallback: http://localhost:5173/cli/auth?port={port}
   - Configurable via NUNCHI_DASHBOARD_URL env var
4. Wait for POST /callback with JSON body
5. Validate the payload has required fields
6. Write to ~/.roko/credentials.json
7. Print success message
8. Shut down server
9. Timeout after 5 minutes with helpful error
```

The callback server has two routes:
- `OPTIONS /callback` — CORS preflight (returns 204 with permissive headers)
- `POST /callback` — receives credentials JSON, signals completion

### 4. Hardcoded Privy app ID

**File:** `crates/roko-serve/src/jwks.rs` or `crates/roko-core/src/config/schema.rs`

```rust
/// The Nunchi Privy application ID.
/// This is a project-level constant, not a secret.
pub const NUNCHI_PRIVY_APP_ID: &str = "cmhw01vut003tjx0d5lmqc8zs";
```

When `ServeAuthConfig.privy_app_id` is `None`, fall back to this constant. Users never need to configure it.

### 5. `roko serve` auto-loads credentials

On startup, `roko serve` should:
1. Check `~/.roko/credentials.json` exists
2. If so, auto-enable Privy JWT validation (use the hardcoded app ID)
3. Log: `"Privy auth enabled (user: will@nunchi.dev)"`

No TOML editing needed. The user's credential file is proof that auth should be active.

### 6. Dashboard callback page

See `nunchi-dashboard/docs/cli-auth-callback.md`. The dashboard hosts `/cli/auth` — a minimal Privy login page that POSTs credentials back to the CLI's localhost server.

## Token lifecycle

- Privy access tokens expire after ~1 hour
- For CLI sessions: user re-runs `roko auth login` when expired
- For `roko serve`: the server validates incoming JWTs from the dashboard (which handles its own refresh via PrivyAuthSync)
- The stored token in credentials.json is the CLI user's own token, not used by the server for validation

## Files to create/modify

| File | Action |
|------|--------|
| `crates/roko-cli/src/auth.rs` | **Create** — login/logout/status/token commands |
| `crates/roko-cli/src/main.rs` | **Modify** — add `auth` subcommand |
| `crates/roko-core/src/config/schema.rs` | **Modify** — add `NUNCHI_PRIVY_APP_ID` constant |
| `crates/roko-serve/src/jwks.rs` | **Modify** — fall back to hardcoded app ID |
| `crates/roko-serve/src/lib.rs` | **Modify** — auto-enable auth from credentials file |

## Dependencies

- Dashboard: `/cli/auth` callback page (see `nunchi-dashboard/docs/cli-auth-callback.md`)
- `open` crate for cross-platform browser opening (or `webbrowser` crate)
- No new external services needed
