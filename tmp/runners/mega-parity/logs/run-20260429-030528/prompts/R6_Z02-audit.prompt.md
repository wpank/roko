# AUDIT: Batch R6_Z02 — Document policy decisions

Run: run-20260429-030528 | Phase: AUDIT | Model: codex-5.5

## Your Role

You are an **auditor**. A fast implementation agent just completed batch `R6_Z02`.
Your job is to verify correctness and fix any issues — do NOT rewrite from scratch.

## Audit Checklist

1. **Compiles:** `cargo check -p <crate>` for each crate touched by this batch
2. **Clippy clean:** `cargo clippy -p <crate> --no-deps -- -D warnings`
3. **Prompt compliance:** Compare the implementation against the original prompt below
4. **No regressions:** Changed files don't break existing functionality
5. **Anti-patterns:** No stubs that silently pass, no inline prompts, no raw CLI spawns
6. **Correct types:** Field names, method signatures, and imports match the actual codebase
7. **Tests pass:** If the prompt required tests, verify they pass

## If You Find Issues

Fix them directly in the files. Then run the verification commands from the prompt.
If you cannot fix an issue, leave a comment in the file explaining why.

## Scope

Only touch files in the batch's write scope. Do NOT refactor unrelated code.

---

## Original Implementation Prompt

## Task
Document policy decisions

## Runner Context
You are working in runner `mega-parity`, batch R6_Z02.
This batch is part of Runner 6: security-posture — Make server, terminal, CORS, and share behavior safe enough for real use.

## Problem
Five security policy questions need resolution before implementation. Based on the R6_Z01 audit, the following facts are established:
1. Terminal routes (`/api/terminal/*`, `/ws/terminal/*`) have zero auth — any HTTP client can spawn a shell
2. `PORT` env var override in `lib.rs:231` forces `0.0.0.0` bind without auth check
3. CORS defaults to `CorsLayer::permissive()` (wildcard `*`) when `server.cors_origins` is empty — see `middleware.rs:427`
4. Background serve in `unified.rs:45–49` starts unconditionally unless `--no-serve` is passed
5. Shared run files written without secret scrubbing (`shared_runs.rs:106–108`); no expiration field on `RunTranscript`

## Architecture Contract
- Default is safe
- Terminal requires auth
- No wildcards for public
- Explicit over implicit

## Changes Required
This is a context-only decision batch. No code changes. Depends on R6_Z01.

### How to confirm the current state before deciding

```bash
# Confirm no auto_start field exists in ServeConfig
grep -n "auto_start\|terminal_enabled\|acknowledge_public_risk" \
  crates/roko-core/src/config/serve.rs

# Confirm auth.enabled default is false
grep -n "enabled.*false\|default.*ServeAuth\|impl Default for ServeAuth" \
  crates/roko-core/src/config/serve.rs

# Confirm terminal routes have no middleware
grep -n "fn routes" crates/roko-serve/src/terminal.rs

# Confirm CorsLayer::permissive() is the empty-origins path
grep -n "permissive\|cors_origins" crates/roko-serve/src/routes/middleware.rs

# Confirm PORT override
sed -n '224,234p' crates/roko-serve/src/lib.rs
```

---

### Decision 1 — No-args serve (background auto-start)

**Question:** Should `roko` (no subcommand) auto-start the HTTP server in the background?

**Current behavior:**
- File: `crates/roko-cli/src/unified.rs`, lines 45–49
- `spawn_background_serve` is called unconditionally when `no_serve = false`
- `ServeConfig` in `crates/roko-core/src/config/serve.rs` lines 11–35 has NO `auto_start` field

**Decision:** No-args `roko` must NOT auto-start serve by default. Add `auto_start: bool` with default `false`.

**Rationale:** Silently exposing ~85 HTTP routes (including terminal) when the user just wants to chat violates "default is safe."

**Implementation approach:**
1. Add `auto_start: bool` field to `ServeConfig` in `crates/roko-core/src/config/serve.rs`
   - Place after line 17 (`pub auto_orchestrate: bool`)
   - `#[serde(default)]` → default `false`
2. In `crates/roko-cli/src/unified.rs`, change lines 45–49 from:
   ```rust
   let serve_state = if no_serve {
       None
   } else {
       spawn_background_serve(&config, &workdir).await
   };
   ```
   to:
   ```rust
   let serve_state = if no_serve || !config.roko.serve.auto_start {
       if !no_serve && !config.roko.serve.auto_start {
           eprintln!("Tip: run `roko serve` to start the HTTP control plane");
       }
       None
   } else {
       spawn_background_serve(&config, &workdir).await
   };
   ```
   Note: check actual field path (`config.roko.serve.auto_start` or `config.serve.auto_start`) by running:
   ```bash
   grep -n "pub serve\|pub roko\|ServeConfig" crates/roko-cli/src/config.rs | head -20
   ```
3. `--no-serve` flag remains for explicit suppression when `auto_start = true`

**Out of scope:** Changing `roko serve` behavior (always starts), daemon behavior.

---

### Decision 2 — Terminal disabled by default

**Question:** Should terminal routes be disabled unless explicitly enabled?

**Current behavior:**
- File: `crates/roko-serve/src/routes/mod.rs`, line 142
- `crate::terminal::routes()` is merged onto the top-level router unconditionally
- `terminal::routes()` (terminal.rs lines 374–390) registers 5 routes with zero auth

**Decision:** Terminal routes must be disabled by default. Add `terminal_enabled: bool` with default `false`.

**Rationale:** PTY terminal routes allow arbitrary command execution on the server machine. This is the highest-severity finding from the audit (F1). Disabling by default is the only safe posture.

**Implementation approach:**
1. Add `terminal_enabled: bool` field to `ServeConfig` in `crates/roko-core/src/config/serve.rs`
   - Place after `auto_start: bool`
   - `#[serde(default)]` → default `false`
2. In `crates/roko-serve/src/routes/mod.rs`, pass `terminal_enabled` into `build_router`:
   - `build_router` signature is at line 64; add `terminal_enabled: bool` parameter
   - Change line 142 from:
     ```rust
     .merge(crate::terminal::routes())
     ```
     to:
     ```rust
     // conditional merge — only when terminal_enabled = true
     ```
   - See R6_B01 for the exact implementation pattern
3. When disabled: terminal routes return `403 {"error": "Terminal disabled", "hint": "Set serve.terminal_enabled=true in roko.toml"}`
4. Add `--enable-terminal` CLI flag to `roko serve` (see R6_B01)

**Out of scope:** Deleting terminal code, per-session auth, terminal-specific auth tokens.

---

### Decision 3 — Bearer token auth for non-local access

**Question:** How should auth work for non-local (non-loopback) access?

**Current behavior:**
- `ServeAuthConfig::default()` in `crates/roko-core/src/config/serve.rs` line 54: `enabled: false`
- When false, all `/api/*` routes are unprotected (lines 115–117 in `routes/mod.rs`)
- `require_api_key` middleware (middleware.rs line 256) already handles `X-Api-Key` and `Authorization: Bearer`
- `ServeAuthConfig.api_key: String` (serve.rs line 45) supports legacy single-key mode

**Decision:** Simple bearer token. When `auth_enabled = true`:
- Generate a random token on first start, store at `~/.roko/serve-token`
- Print token on startup: `"Auth token: {token} (stored at ~/.roko/serve-token)"`
- Non-loopback requests require `Authorization: Bearer {token}` OR `X-Api-Key: {token}` header
- Loopback (127.0.0.1, ::1) always allowed without token
- `GET /health` always allowed without token

**Rationale:** The `require_api_key` middleware already exists and already handles both `X-Api-Key` and `Bearer` headers. We need token generation + loopback bypass, not a new auth system.

**Implementation approach:**
1. Token generation: `Uuid::new_v4().to_string()` + `Uuid::new_v4().to_string()` concatenated (no hyphens)
2. Storage: `~/.roko/serve-token` (NOT in `roko.toml`)
3. In `start_background` (lib.rs around line 224): when `auth_enabled = true`, read or generate token, store it, print to stderr, inject into `config.roko_config.serve.auth.api_key`
4. Loopback bypass: new middleware layer that checks `std::net::IpAddr::is_loopback()` on the peer address; if loopback, skip auth. Use `axum::extract::ConnectInfo<SocketAddr>` to get the peer addr
5. `GET /health` bypasses via existing top-level route placement (outside `/api`)

**Out of scope:** OAuth, sessions, cookies, multi-user, per-route auth granularity.

---

### Decision 4 — Bind addresses

**Question:** What should default bind behavior be?

**Current behavior:**
- `default_bind()` in `serve.rs:170–172` returns `"127.0.0.1"` — safe default
- `lib.rs:226–231`: `PORT` env var forces `"0.0.0.0:{p}"` without auth check
- No guard preventing public bind when `auth_enabled = false`

**Decision:**
- Default: `127.0.0.1:6677` (already correct — no change to default)
- Loopback bind: always works, no auth required
- Non-loopback + `auth_enabled = false` + no `acknowledge_public_risk`: **error** with message: `"Non-local bind requires auth_enabled = true in [serve.auth] or --unsafe-public flag"`
- Non-loopback + `auth_enabled = true`: works
- Non-loopback + `acknowledge_public_risk = true`: works with stderr warning

**Rationale:** The `PORT` env var Railway bypass is the dangerous gap. Guard added in `start_background` (lib.rs) at approximately line 234 (after the `addr` is resolved).

**Implementation approach:**
1. Add `acknowledge_public_risk: bool` to `ServeConfig` in `serve.rs` (default `false`)
2. In `start_background` after resolving `addr` (lib.rs line 234): parse the host portion of `addr`, call `std::net::IpAddr::is_loopback()` — if non-loopback AND `!auth_enabled` AND `!acknowledge_public_risk`: `return Err(anyhow!(...))`
3. Error message: `"Non-local bind requires auth_enabled = true in [serve.auth] or --unsafe-public flag"`
4. Add `--unsafe-public` CLI flag to the `Serve` command in `main.rs` (see R6_A02)

**Out of scope:** Changing the default port, complex IPv6 handling.

---

### Decision 5 — Share defaults

**Question:** What should share behavior default to?

**Current behavior:**
- `create_share` (`shared_runs.rs:71`): writes JSON to `.roko/shared/{token}.json` without scrubbing
- `RunTranscript` (lines 19–49): no `expires_at` field
- `scrub_secrets` middleware (middleware.rs line 460) only scrubs `/api/*` responses; `shared_runs::routes()` is on the outer router (mod.rs line 140) and bypasses the scrubber

**Decisions:**
- **Scrubbing:** Scrub known secret patterns before calling `serde_json::to_string_pretty` at `shared_runs.rs:106`. Patterns handled by existing `LogScrubber` in `AppState.scrubber`. Call `state.scrubber.scrub(&json)` before writing. Add `scrubbed: true` to share metadata.
- **Expiration:** Add `expires_at: Option<String>` to `RunTranscript` (ISO 8601). Default TTL = 7 days. Add `serve.share_ttl_days: u32` to `ServeConfig` (default: 7). Expired share → 410 Gone. `--no-expire` on a future share CLI flag → `expires_at: None`.
- **Backward compat:** Existing shares without `expires_at` are treated as non-expiring (use `Option` with no default value).

**Rationale:** Scrubbing before storage ensures even if the file is leaked the secrets are not exposed. Expiration reduces the surface of old shares.

**Out of scope:** Deleting shares, background cleanup jobs, public share URLs.

---

## No-contradiction check

| Decision | Contradicts | Resolved |
|---|---|---|
| D1: auto_start=false | D2 terminal disabled | No conflict — both gate new capabilities |
| D2: terminal disabled | D3 bearer auth | Terminal also requires auth when non-loopback (D3 is prerequisite) |
| D3: bearer token | D4 bind guard | Auth enables public bind (D4 allows when auth_enabled) |
| D4: bind guard | D5 share | Independent — share is a data concern, bind is a network concern |
| D5: share TTL | D1 | Independent |

All decisions are consistent with "default is safe."

## Write Scope (files you may modify)
- None. This is a decision-only batch.

## Read-Only Context (do not modify these)
- R6_Z01 audit output
- `crates/roko-serve/src/` — current implementation
- `crates/roko-core/src/` — config types

## Acceptance Criteria
- [ ] All 5 questions resolved with clear decisions
- [ ] Each decision has rationale
- [ ] Each decision has implementation approach with exact file locations
- [ ] Out-of-scope items marked
- [ ] No contradictions between decisions
- [ ] Decisions are consistent with "default is safe" principle

## Verification
```bash
# No code verification — this is a policy document
# Confirm source facts:
grep -n "auto_start\|terminal_enabled\|acknowledge_public_risk" crates/roko-core/src/config/serve.rs
grep -n "enabled.*false" crates/roko-core/src/config/serve.rs
grep -n "permissive" crates/roko-serve/src/routes/middleware.rs
```

## Do NOT
- Write any code
- Leave any question unresolved
- Make decisions that contradict "default is safe"
- Over-engineer (no OAuth, no sessions, no complex auth)

## Evidence
- R6_Z01 audit results
- `crates/roko-serve/src/` — current implementation
- `crates/roko-core/src/config/serve.rs` — config types

---

## Verification Commands

Run these and fix any failures:
```bash
cargo check --workspace
cargo clippy --workspace --no-deps -- -D warnings
```

## Do NOT

- Rewrite the entire implementation from scratch
- Add features not in the original prompt
- Modify files outside the write scope
- Skip running verification commands
