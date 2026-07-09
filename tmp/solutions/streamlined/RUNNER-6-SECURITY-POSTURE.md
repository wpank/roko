# Runner 6: `security-posture` — Granular Batch Specification

Date: 2026-04-28

Parent: [FULL-WORK-PLAN.md](./FULL-WORK-PLAN.md) Runner 6 section.

---

## Runner Goal (one sentence)

Make server, terminal, CORS, and share behavior safe enough for real use by binding
local-only by default, gating dangerous routes, and preventing accidental public exposure.

## Context Pack Files

```text
tmp/runners/security-posture/
  README.md
  batches.toml
  context/
    00-RULES.md                     — universal + runner-specific anti-patterns
    ARCHITECTURE-CONTRACT.md        — single-owner map for this runner
    ANTI-PATTERNS.md                — forbidden patterns with repo examples
    ACCEPTANCE.md                   — proof commands including negative proofs
    FILE-OWNERSHIP.md               — batch → write path map
    ISSUE-MAP.md                    — batch → issue id map
    SECURITY-AUDIT.md              — current exposure surface (Group 0 output)
    POLICY-DECISIONS.md            — resolved product decisions (Group 0 output)
```

---

## Anti-Pattern Rules (00-RULES.md)

Include the universal rules from FULL-WORK-PLAN.md plus:

```markdown
# Security-Posture Anti-Patterns

SP-1. **Default is safe.** Every new deployment, fresh install, or first run MUST be safe
      without explicit hardening. Unsafe modes require explicit flags.

      EXISTING ANTI-PATTERN (do not repeat):
      - `crates/roko-core/src/config/serve.rs:54-57` sets `auth_enabled: false` by default.
      - `crates/roko-serve/src/routes/mod.rs:140` merges terminal routes outside any auth path.
      - `crates/roko-cli/src/unified.rs:45-64` starts background serve by default for no-args `roko`.

SP-2. **Terminal = shell access = auth required.** Any route that provides PTY/terminal
      access is equivalent to granting shell access. It must NEVER be available without
      authentication or explicit local-only trust.

SP-3. **Wildcards are forbidden for public bind.** CORS `*`, bind `0.0.0.0` without auth,
      and share URLs without expiration are all forbidden for anything that leaves localhost.

SP-4. **Explicit over implicit.** Users must CHOOSE to expose. Never expose by default and
      hope they configure restrictions.
```

---

## Policy Decisions Required

This runner requires product decisions before implementation. These should be resolved in
Z02 and documented in `POLICY-DECISIONS.md`:

1. **Should no-args `roko` start `serve` at all?**
   - Recommended: No. Background serve disabled for interactive mode. User runs `roko serve`
     explicitly.

2. **Should terminal routes be disabled by default?**
   - Recommended: Yes. Enabled only with `--enable-terminal` flag or config
     `serve.terminal_enabled = true`.

3. **What auth mechanism for local browser UI?**
   - Recommended: Bearer token from `~/.roko/serve-token` (generated on first serve start).
     Browser gets token from a one-time localhost redirect flow.

4. **What bind addresses are allowed without explicit flag?**
   - Recommended: `127.0.0.1` and `::1` only. Any other address requires `--bind <addr>` plus
     `--unsafe-public` or auth enabled in config.

5. **Should share links be local-only, signed, expiring, or scrubbed?**
   - Recommended: Local-only by default (share token valid only from local server). Public
     share requires explicit `--public` flag plus content scrubbing.

---

## Group 0: Contract Guardrails

### Z01 — Audit current security surface

**Type:** Context-only (no code changes)

**Goal:** Map every route, binding, and access path that has security implications.

**Write scope:**
- `tmp/runners/security-posture/context/SECURITY-AUDIT.md`

**Read:**
- `crates/roko-serve/src/routes/mod.rs` (all route registrations)
- `crates/roko-serve/src/terminal.rs` (PTY session management)
- `crates/roko-core/src/config/serve.rs` (serve config defaults)
- `crates/roko-serve/src/routes/middleware.rs` (CORS config)
- `crates/roko-cli/src/unified.rs` (background serve start)
- `crates/roko-cli/src/share.rs` (share URL generation)
- `crates/roko-serve/src/routes/shared_runs.rs` (shared run access)

**Required output:**
- All routes with security classification: public-safe, auth-required, admin-only
- Current CORS configuration and what origins are allowed
- Current bind address logic
- Current auth state (enabled/disabled/configurable)
- Terminal session lifecycle and what shell access is granted
- Share URL format and what data is exposed
- Background serve: when it starts, what it exposes

**DO NOT:** Change any source code.

---

### Z02 — Document policy decisions

**Type:** Context-only (no code changes)

**Goal:** Record the resolved product decisions for this runner.

**Write scope:**
- `tmp/runners/security-posture/context/POLICY-DECISIONS.md`

**Required output:**
- Resolve the 5 questions in the "Policy Decisions Required" section above
- For each: decision, rationale, implementation approach
- Document what's OUT of scope (enterprise auth, OAuth, SSO, etc.)

**DO NOT:** Change any source code.

---

## Group A: Bind and Serve Defaults

### A01 — Bind local-only by default

**Goal:** `roko serve` binds to localhost unless explicitly overridden.

**Write scope:**
- `crates/roko-core/src/config/serve.rs` (default address)
- `crates/roko-serve/src/lib.rs` OR `crates/roko-serve/src/runtime.rs` (bind logic)

**Required behavior:**
- Default bind: `127.0.0.1:6677`
- Config `serve.bind = "0.0.0.0:6677"` works but only if EITHER:
  - `serve.auth_enabled = true` in config, OR
  - `--unsafe-public` flag is passed to CLI
- Without one of those: print error and refuse to start:
  ```
  Error: cannot bind to 0.0.0.0 without auth enabled.
  Add `auth_enabled = true` to [serve] in roko.toml, or pass --unsafe-public.
  ```
- `--bind 127.0.0.1:8080` always works (loopback is safe)

**DO NOT:**
- Break existing `roko serve` for local development (localhost still works)
- Add complex auth implementation (just the bind gate)
- Change port defaults

**Verify:** `cargo check -p roko-serve -p roko-cli`

**Evidence:** COMPREHENSIVE-ISSUES 15.3

---

### A02 — Require explicit flag for public bind

**Goal:** Accidental public exposure is impossible.

**Write scope:**
- `crates/roko-cli/src/commands/` (serve command handler)
- OR `crates/roko-cli/src/main.rs` (serve flag handling)

**Required behavior:**
- Add `--unsafe-public` CLI flag to `roko serve`
- This flag is required when bind address is non-loopback AND auth is disabled
- The flag name intentionally includes "unsafe" to make the risk obvious
- Print a warning when used: `"WARNING: serving without auth on public interface"`
- Config alternative: `serve.acknowledge_public_risk = true`

**DO NOT:**
- Make `--unsafe-public` the only way (config should also work for deployment)
- Add the flag to other commands
- Log credentials or tokens in the warning

**Verify:** `cargo check -p roko-cli`

---

### A03 — Disable background serve for no-args `roko`

**Goal:** Interactive chat does not auto-start an HTTP server.

**Write scope:**
- `crates/roko-cli/src/unified.rs` (background serve start logic)

**Required behavior:**
- Remove or gate the background `serve` start in the interactive chat path
- Gating option: only start background serve if `config.serve.auto_start = true`
- Default for new config: `auto_start = false`
- Print info when serve is skipped: `"tip: run 'roko serve' separately for the web UI"`
- `roko serve` command still works explicitly

**DO NOT:**
- Remove the serve command
- Break the explicit `roko serve` path
- Remove the config option entirely (some users want auto-start)

**Verify:** `cargo check -p roko-cli`

**Evidence:** MY-TAKE-SHORTEST-PATH.md §4

---

## Group B: Terminal Route Security

### B01 — Gate terminal routes behind auth/flag

**Goal:** PTY terminal access requires explicit opt-in.

**Write scope:**
- `crates/roko-serve/src/routes/mod.rs` (route registration)
- `crates/roko-serve/src/terminal.rs` (session creation guard)

**Required behavior:**
- Terminal routes (`/api/terminal/*`, `/ws/terminal/*`) are NOT registered unless:
  - Config `serve.terminal_enabled = true`, OR
  - CLI flag `--enable-terminal` is passed
- When disabled: these routes return 403 with:
  ```json
  {"error": "terminal routes disabled", "hint": "add terminal_enabled = true to [serve]"}
  ```
- When enabled on non-loopback: require auth token (from B02)
- When enabled on loopback: allow without auth (local trust)

**DO NOT:**
- Delete terminal route code (just gate access)
- Break the demo-app terminal functionality when explicitly enabled
- Add complex permission models

**Verify:** `cargo check -p roko-serve`

**Evidence:** COMPREHENSIVE-ISSUES 15.2

---

### B02 — Add bearer token auth for non-local access

**Goal:** Non-local requests must present a token.

**Write scope:**
- `crates/roko-serve/src/routes/middleware.rs` (auth middleware)
- `crates/roko-core/src/config/serve.rs` (token config)

**Required behavior:**
- Generate a random token on first `roko serve` start: write to `~/.roko/serve-token`
- Print token on startup: `"Auth token: <token> (saved to ~/.roko/serve-token)"`
- Middleware: if `auth_enabled = true` AND request is not from loopback:
  - Check `Authorization: Bearer <token>` header
  - If missing/invalid: 401 `{"error": "unauthorized"}`
- Loopback requests (127.0.0.1, ::1): always allowed (local trust)
- `/health` and `/api/health`: always allowed (for probes)

**DO NOT:**
- Implement OAuth, sessions, or cookies (too complex for this runner)
- Make auth required for localhost
- Store token in config file (separate file, user-only permissions)

**Verify:** `cargo check -p roko-serve`

---

## Group C: CORS and Share Security

### C01 — Restrict CORS to local origins by default

**Goal:** Wildcard CORS is never the default.

**Write scope:**
- `crates/roko-serve/src/routes/middleware.rs` (CORS config)

**Required behavior:**
- Default allowed origins: `http://localhost:*`, `http://127.0.0.1:*`
- Config `serve.cors_origins = ["https://my-domain.com"]` for custom origins
- With `--unsafe-public` AND no cors_origins configured: warn but allow `*`
- Without unsafe-public: non-local origins get no CORS headers (blocked by browser)

**DO NOT:**
- Break the local demo-app development flow (localhost always works)
- Add complex origin validation (just exact match + localhost pattern)
- Block preflight requests for health endpoints

**Verify:** `cargo check -p roko-serve`

**Evidence:** MY-TAKE-SHORTEST-PATH.md §4

---

### C02 — Share output scrubbing

**Goal:** Shared runs don't accidentally expose secrets.

**Write scope:**
- `crates/roko-cli/src/share.rs` (share creation logic)

**Required behavior:**
- Before creating a share: scrub known secret patterns from the transcript:
  - `ANTHROPIC_API_KEY=sk-ant-*` → `ANTHROPIC_API_KEY=[REDACTED]`
  - `OPENAI_API_KEY=sk-*` → `OPENAI_API_KEY=[REDACTED]`
  - Bearer tokens in headers
  - Any value matching `[A-Za-z0-9_-]{32,}` preceded by `key`, `token`, `secret`, `password`
- Add a metadata field: `scrubbed: true` when scrubbing was applied
- Default share: local-only (valid on the local server only)
- `--public` flag required for shares accessible from non-local origins

**DO NOT:**
- Prevent all sharing (just scrub and gate)
- Add complex regex that causes false positives on code content
- Change the share URL format

**Verify:** `cargo check -p roko-cli`

---

### C03 — Share expiration

**Goal:** Shares don't persist forever by default.

**Write scope:**
- `crates/roko-serve/src/routes/shared_runs.rs` (share access logic)
- `crates/roko-cli/src/share.rs` (share creation)

**Required behavior:**
- Default share TTL: 7 days (configurable via `serve.share_ttl_days`)
- Share metadata includes `expires_at` timestamp
- Accessing an expired share: 410 Gone `{"error": "share expired"}`
- `--no-expire` flag for permanent shares (logged as intentional)
- Existing shares without `expires_at`: treated as non-expiring (backward compat)

**DO NOT:**
- Delete expired share files immediately (lazy deletion on access is fine)
- Make TTL too short for legitimate use (7 days is reasonable)
- Break existing share URLs (just add expiration to new ones)

**Verify:** `cargo check -p roko-serve -p roko-cli`

---

## Group D: Proof

### D01 — Security smoke test script

**Write scope:**
- `tmp/runners/security-posture/context/PROOF-SECURITY.sh` (executable script)

**Required proof steps:**
```bash
# 1. Default serve binds to localhost
roko serve &
PID=$!
curl -f http://127.0.0.1:6677/health  # should succeed
# (no external interface test needed if we verified bind address)

# 2. Terminal routes disabled by default
curl -s http://127.0.0.1:6677/api/terminal/sessions | jq .error
# Expected: "terminal routes disabled"

# 3. Non-local without auth fails (if we can simulate)
# This is harder to test locally — document the expectation

# 4. Share with secrets is scrubbed
# Create a share with fake secrets in transcript, verify they're redacted

kill $PID
```

---

### D02 — Negative proof: public bind without auth fails

**Write scope:**
- `crates/roko-serve/tests/` OR test module

**Required behavior (test):**
- Attempt to start server with bind `0.0.0.0:0` and `auth_enabled = false`
- Assert: returns error (not Ok)
- Assert: error message mentions auth or unsafe-public
- Attempt with `acknowledge_public_risk = true`: succeeds (user accepted risk)

**Verify:** `cargo test -p roko-serve -- security`

---

## Batch Summary

| Group | Batches | Main scope |
|---|---:|---|
| 0: Contracts | 2 | security audit, policy decisions |
| A: Bind/Serve | 3 | local-only, explicit flag, disable auto-serve |
| B: Terminal | 2 | gate routes, add bearer token |
| C: CORS/Share | 3 | restrict CORS, scrub secrets, add expiration |
| D: Proof | 2 | smoke test, negative proof |
| **Total** | **12** | |

## Suggested Execution Waves

Wave 1: Z01, Z02 (context-only, parallel)
Wave 2: A01, A03 (bind local + disable auto-serve — independent files)
Wave 3: A02 (unsafe-public flag — depends on A01)
Wave 4: B01, B02 (terminal gate + auth — related but can parallel if careful)
Wave 5: C01 (CORS — independent)
Wave 6: C02, C03 (share scrubbing + expiration — parallel)
Wave 7: D01, D02 (proofs)

## Acceptance Criteria

This runner is done when:

**Positive proofs:**
- `roko serve` binds to localhost by default, accessible from browser
- `roko serve --enable-terminal` enables terminal routes
- Explicit `--unsafe-public` allows non-local bind
- Token auth works for non-local requests
- Share content is scrubbed of secrets
- Shares expire after 7 days

**Negative proofs:**
- `roko serve --bind 0.0.0.0` without auth/flag → refuses to start
- Terminal routes without `--enable-terminal` → 403
- Non-local request without token → 401
- CORS preflight from random origin → no CORS headers (blocked)
- No-args `roko` does NOT start a background server by default
- Expired share → 410 Gone
