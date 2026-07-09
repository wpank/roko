# Authentication and secrets

> Part of the [Roko Architecture Specification](00-INDEX.md). Extracted from v2 redesign doc.
> Merges the "Authentication" and "Secret and API key management" sections.

---

## Authentication

Four auth paths for four surfaces.

### 1. Dashboard users: Privy

```
Browser → Privy SDK → JWT → roko-serve validates signature
```

Privy handles login (email, social, wallet). The dashboard includes the JWT in every API call. roko-serve validates the JWT signature against Privy's JWKS endpoint.

```
GET https://auth.privy.io/.well-known/jwks.json
→ Cache JWKS, verify JWT signature + expiry
→ Extract: sub (privy user ID), email, wallet address
→ Lookup or create user in .roko/users/
```

**JWKS caching strategy**:

```
JWT arrives for validation
         │
         ▼
Check in-memory JWKS cache
         │
    cache hit ──────────────► Validate JWT with cached keys
    (< 1 hour old)                    │
         │                       valid ──► accept
    cache miss                        │
    (or expired)                 invalid ──► refetch JWKS once (key rotation)
         │                                        │
         ▼                                   valid ──► accept
Fetch GET https://auth.privy.io/                  │
      /.well-known/jwks.json              still invalid ──► return 401
         │
    success ──► update cache, validate JWT
         │
    failure ──► use stale cache if available (stale-while-revalidate)
         │           │
         │      no stale cache ──► return 401
         │
         ▼
    Log warning if cache is > 24 hours old:
    "[WARN] JWKS cache stale (26h). Privy endpoint may be down."
```

- **Cache TTL**: 1 hour. After 1 hour, the next validation triggers a background refetch.
- **Key rotation handling**: When a JWT fails validation against cached keys, refetch JWKS once before returning 401. Privy rotates keys periodically; the single retry catches rotation without adding latency to every request.
- **Endpoint unavailability**: If the JWKS endpoint is down, use stale cached keys. This is safe because Privy key rotation is infrequent (weeks or months between rotations). Log a warning if the cache exceeds 24 hours of staleness.
- **Startup**: On server start, fetch JWKS eagerly. If the fetch fails, the server starts but JWT validation returns 401 until the cache is populated. API key auth and agent token auth are unaffected.

Privy also provides an embedded wallet for chain interactions (signing transactions, delegating to agents). Optional -- users who don't need chain features never see wallet UI.

### 2. CLI: API keys + roko login

```bash
# Generate an API key (from dashboard or CLI)
roko config secrets set api.my-key
# → sk_roko_aBcDeFgHiJkLmNoP

# Use it
roko status --server https://my-roko.up.railway.app --api-key sk_roko_...

# Or: roko login (browser-based)
roko login https://my-roko.up.railway.app
# → Opens browser for Privy auth
# → Stores session token in OS keychain

# On headless machines: device flow
roko login https://my-roko.up.railway.app
# → Visit https://my-roko.up.railway.app/auth/device
#   Enter code: ABCD-EFGH
# → Polls until approved
# → Token stored in OS keychain
```

API keys have scopes:

```rust
pub enum ApiKeyScope {
    Read,        // GET endpoints only
    AgentWrite,  // Agent CRUD + messaging
    PlanWrite,   // Plan/PRD creation and execution
    Admin,       // Everything including secrets and config
}
```

**Scope-to-route mapping**:

```
Scope          Allowed methods    Allowed routes
─────          ───────────────    ──────────────
read           GET                Any route

agent:write    POST, PUT, DELETE  /api/agents/*
               POST               /api/agents/*/message
               POST               /api/agents/*/token

plan:write     POST, PUT, DELETE  /api/plans/*
               POST               /api/plans/*/run
               POST, PUT, DELETE  /api/prd/*

admin          *                  * (all routes, including:)
                                  /api/api-keys/*
                                  /api/config/*
                                  /api/secrets/*
                                  /api/gateway/*
```

A key with multiple scopes has the union of their permissions. For example, a key with `[read, agent:write]` can GET any route and POST/PUT/DELETE on agent routes, but cannot touch plans or config.

**Insufficient scope response**:

```json
HTTP 403 Forbidden

{
  "error": "insufficient_scope",
  "required": "agent:write",
  "has": "read",
  "route": "POST /api/agents/coder-1/message"
}
```

The response tells the caller exactly what scope they need, what they have, and which route triggered the rejection. This makes debugging straightforward for both humans and agents.

### 3. Agent auth: bearer tokens

Agents authenticate to the relay and to the inference proxy using bearer tokens issued by the control plane.

```bash
# Control plane issues token for an agent
POST /api/agents/:id/token
→ { "token": "roko_agent_...", "expires_at": "2026-04-25T00:00:00Z" }

# Agent uses token for relay connection
WS wss://relay.nunchi.dev/relay/ws
→ First message: { "type": "auth", "token": "roko_agent_..." }

# Agent uses token for inference proxy
POST /api/inference/proxy
Authorization: Bearer roko_agent_...
```

Tokens are SHA-256 hashed before storage. The plaintext is returned exactly once at issuance.

**Token lifecycle**:

```
Agent created                Token issued                 30 days later
(POST /api/agents)           (response includes token)    (token expires)
       │                            │                            │
       ▼                            ▼                            ▼
  agent record              token = "roko_agent_..."      agent gets 401
  stored in DB              SHA-256 hash stored           on next request
                            plaintext returned ONCE
                                                                 │
                                                                 ▼
                                                          request new token
                                                          via relay or API
```

- **Issuance**: Tokens are issued when an agent is created (`POST /api/agents`). The response body includes the `token` field with the plaintext. This is the only time the plaintext is available.
- **Expiry**: Tokens expire after 30 days by default. Configurable per agent via `token_ttl_days` in `roko.toml`.
- **Revocation**: `DELETE /api/agents/{id}/token` immediately invalidates the token. The SHA-256 hash is removed from the valid token set. The agent receives `401 Unauthorized` on its next request.
- **Rotation**: To rotate without downtime, issue a new token (`POST /api/agents/{id}/token`) before revoking the old one. During rotation, both the old and new tokens are valid for a 5-minute grace period. After 5 minutes, the old token is automatically invalidated.
- **Re-issuance**: An agent that receives 401 (expired or revoked token) should request a new token through the relay control channel or by calling `POST /api/agents/{id}/token` with admin-scoped auth.

### 4. Relay auth: reads public, writes authenticated

```
Read operations (subscribe, list feeds):    No auth required
Write operations (publish, register feed):  Require agent token
Admin operations (force-disconnect):        Require API key with admin scope
```

This means the dashboard can subscribe to presence and feeds without authentication. It needs auth only to send messages to agents or modify configuration.

### Agent-to-agent auth for paid feeds

Paid feed subscriptions use the same agent token mechanism. The subscribing agent's token is validated by the relay, and payment is recorded against the agent's budget.

### 5. Shared workspace access (team development)

A deployed roko instance is private by default — only the owner can access it. The owner can invite teammates by email or wallet address, giving them authenticated access to the same workspace.

**Key constraint:** Nunchi owns the Privy app. Users deploying their own roko instances don't have access to Privy's dashboard or app secret. Therefore:

- **Privy's allowlist is NOT used.** Privy login is open — anyone can authenticate.
- **Roko-serve handles all authorization.** Each roko instance has its own user table in `.roko/users/`. It decides who gets in.
- **No `PRIVY_APP_SECRET` needed** on user deployments. JWT verification uses Privy's public JWKS endpoint, which requires no secret.

```
Nunchi (company)                    User's roko instance (Railway)
────────────────                    ──────────────────────────────
Owns Privy app                      Has PRIVY_APP_ID (public, baked in)
  open login (no allowlist)         Does NOT have PRIVY_APP_SECRET
  anyone can authenticate           Verifies JWTs via public JWKS (no secret)

                                    .roko/users/
                                    ├── owner: will@nunchi.dev (first login)
                                    ├── invited: sarah@example.com → member
                                    └── (anyone else) → 403 "Not a member"
```

```
User opens dashboard
  → Privy login modal (email/Google/Apple/wallet) — open, anyone can log in
  → Privy issues JWT (contains privy_user_id + email)
  → Dashboard sends JWT to THIS roko instance
  → Roko-serve validates JWT signature (public JWKS, no secret)
  → Roko-serve extracts email from JWT
  → Looks up email in .roko/users/
     → Found? → authorize with stored role
     → Email matches pending invitation? → auto-create user with invited role
     → Not found, no invitation? → 403 "Not a member of this workspace"
```

**Invitation flow:**

1. Owner goes to Settings > Team, types `sarah@example.com`, picks role "Member"
2. Dashboard calls `POST /api/team/invite` on the roko instance
3. Roko-serve stores invitation in `.roko/users/invitations.json` (local to this instance)
4. Dashboard shows a shareable link: `https://your-roko.up.railway.app`
5. Owner sends the link to Sarah (email, Slack, whatever)
6. Sarah opens link → Privy login → logs in with `sarah@example.com`
7. Privy issues JWT → dashboard sends to roko-serve
8. Roko-serve sees email matches invitation → creates user record with "member" role
9. Sarah sees the dashboard with member-level permissions

**No Privy dashboard access needed by anyone.** No allowlist management. No app secret on the deployment. Privy is purely an identity provider — a "login with email/Google/Apple" black box.

**Roles:**

| Role | Agents | Plans | Secrets | Team | System |
|------|--------|-------|---------|------|--------|
| `owner` | full | full | full | manage | full |
| `admin` | full | full | view | invite | view |
| `member` | full | full | — | — | view |
| `viewer` | view | view | — | — | view |

Roles are stored locally in `.roko/users/{email}.json`. No Privy custom_metadata needed.

**Revoking access:**

1. Owner removes Sarah from team (`DELETE /api/team/members/:id`)
2. Roko-serve deletes Sarah's user record from `.roko/users/`
3. Sarah's existing Privy JWT still works (Privy doesn't know about roko-serve's user table)
4. On Sarah's next API call: roko-serve looks up her email → not found → 403 immediately
5. Dashboard shows "You are no longer a member of this workspace"

Revocation is instant from roko-serve's perspective. Sarah can still authenticate with Privy (it's open login), but roko-serve won't let her in.

**Wallet-based invitations:**

In addition to email, invitations can be by wallet address:
```json
{ "identifier": "0x7f3b...2c4a", "type": "wallet", "role": "member" }
```
Roko-serve matches the wallet address from the Privy JWT's linked accounts.

**API routes:**

```
POST /api/team/invite        — invite by email or wallet (owner/admin only)
  Body: { "identifier": "alice@example.com", "type": "email", "role": "member" }
  → Stores invitation locally in .roko/users/invitations.json
  → Returns: { "invited": true }

GET /api/team/members         — list team members (any authenticated user)
  → [{ "email": "will@...", "role": "owner", "joined_at": "..." },
     { "email": "alice@...", "role": "member", "joined_at": "..." }]

PUT /api/team/members/:id     — change role (owner/admin only)
  Body: { "role": "admin" }

DELETE /api/team/members/:id  — remove from team (owner/admin only)
  → Deletes user record, next API call gets 403

GET /api/team/me              — current user's role and permissions
```

**First-run bootstrap:**

1. User clicks "Deploy on Railway" → roko deploys with `PRIVY_APP_ID` already set
2. User visits dashboard URL → Privy login modal
3. User logs in (email/Google/Apple)
4. Roko-serve sees no users in `.roko/users/` → first user becomes Owner automatically
5. Owner configures provider keys (Settings > Provider Keys)
6. Owner invites team members (Settings > Team)

**Dashboard UX:**

```
Settings > Team

+-------------------------------------------------------------+
| Team Members                                                 |
|                                                              |
| will@nunchi.dev              owner    Joined 2d ago          |
| alice@example.com            member   Joined 1d ago          |
| bob@example.com              admin    Joined 3h ago          |
|                                                              |
| Pending Invitations                                          |
| carol@example.com            member   Invited 1h ago         |
|                                                              |
| +----------------------------------------------------------+|
| | Invite teammate                                           ||
| | Email or wallet: [                        ] Role: [member]||
| | [Send Invite]                                             ||
| +----------------------------------------------------------+|
+-------------------------------------------------------------+
```

**Workspace auto-discovery for team members:**

When roko registers with the relay, it includes the owner's wallet. Team members auto-discover by:
1. Owner's wallet → direct match for owner
2. Team members → workspace URL saved in localStorage from first visit
3. Optionally: roko registers all team member wallets with the relay for auto-discovery

---

## Secret and API key management

### Storage hierarchy

```
Priority    Source              Where
────────    ──────              ─────
1 (highest) Environment vars   ANTHROPIC_API_KEY, PERPLEXITY_API_KEY, etc.
2           Secrets store       .roko/secrets.toml (encrypted at rest)
3           Config file         roko.toml [providers] section (not recommended)
```

### Secrets store format

```toml
# .roko/secrets.toml
# Encrypted with age (https://age-encryption.org)
# Key derived from machine identity or user passphrase

[llm]
anthropic = "sk-ant-..."
perplexity = "pplx-..."
gemini = "AIza..."
openrouter = "sk-or-..."
moonshot = "sk-..."
zai = "..."

[integration]
github = "ghp_..."
slack = "xoxb-..."

[infra]
fly_api_token = "fo1_..."
railway_token = "..."
```

### From the CLI

```bash
# Set a secret (reads from stdin, never in shell history)
echo "sk-ant-xyz" | roko config secrets set llm.anthropic

# Interactive prompt
roko config secrets set llm.anthropic
# Enter secret: ****

# List configured secrets (keys, never values)
roko config secrets list
# NAMESPACE    KEY          SOURCE        STATUS
# llm          anthropic    secrets.toml  * valid
# llm          perplexity   env var       * valid
# integration  github       secrets.toml  * valid
# llm          gemini       --            o not set

# Validate all secrets
roko config check-secrets
# Anthropic: valid (claude-sonnet-4-6 accessible)
# Perplexity: valid
# GitHub: valid (repo scope, expires 2026-06-01)
# Gemini: not configured
```

### From the dashboard

Settings > Provider Keys page. Each provider shows a status indicator (connected / not set / invalid). Users paste keys into a form. The dashboard sends them to `POST /api/secrets/:ns/:key`.

Test button calls `POST /api/secrets/:ns/:key/test` -- the server makes a minimal API call to the provider and returns connection status.

**Client-only mode**: keys stored in `localStorage`, sent via `X-Provider-Keys` header per request. The server uses them but never persists them.
