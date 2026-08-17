# 10 — Tier 0: Stop Active Bleeding (Reference; All Done)

All seven Tier-0 items shipped before this folder was created. This file
documents what shipped so a regression can be detected, not what to
implement.

If a verification command in this file starts failing, that means a
regression has been introduced and the fix below is the canonical reference.

Verification headline: `git log --oneline | rg '^[a-f0-9]+ T0-'` returns 7
commits.

---

## [x] T0-1: Mask secrets in `GET /api/config/toml`

**Commit**: `196e6087 T0-1: Mask secrets in GET /api/config/toml`

**File**: `crates/roko-serve/src/routes/config.rs:48-65`

**What landed**: `get_config_toml` now serializes the config to JSON, calls
`mask_secret_fields`, strips `*_note` hint fields and JSON nulls (TOML has
no null), then converts to `toml::Value` and serializes.

**Verify (regression check)**:

```bash
cargo test -p roko-serve config_toml --lib
rg 'mask_secret_fields\(' crates/roko-serve/src/routes/config.rs
# Both get_config and get_config_toml must call it
```

If TOML output ever exposes `chain.wallet_key`, `serve.auth.api_key`,
`webhooks.github.secret`, or any provider `api_key`, that is a
regression — restore the masking call.

---

## [x] T0-2: Expand `mask_secret_fields` coverage

**Commit**: `9264f4fe T0-2: Expand mask_secret_fields for chain, GitHub webhooks, providers`

**File**: `crates/roko-serve/src/routes/config.rs:290-320`

**What landed**: `mask_secret_fields` now masks:

- `serve.auth.api_key` (existed)
- `server.auth_token` (existed)
- `deploy.railway_api_token` (existed)
- `chain.wallet_key` ← added
- `webhooks.github.secret` ← added
- All `providers.<name>.api_key` ← added (loop)

`api_key_env` is intentionally **not** masked; it is the env-var name, not
the secret.

**Verify**:

```bash
cargo test -p roko-serve mask_secret --lib
```

If a new secret field is added to the schema, it must be added to
`mask_secret_fields` in the same PR. There is no auto-discovery.

---

## [x] T0-3: Add path validation to shared runs

**Commit**: `fa8f2780 T0-3: Validate shared run transcript path segments`

**File**: `crates/roko-serve/src/routes/shared_runs.rs:145, 149, 277`

**What landed**: `validate_path_segment(...)` is called on the run id and
share token in `create_share`, and on the id in `load_transcript_record`.
Bad values return a typed `ApiError`.

**Verify**:

```bash
rg 'validate_path_segment' crates/roko-serve/src/routes/shared_runs.rs
# Should return at least 3 lines (creates and loads).
```

`validate_path_segment` lives in `crates/roko-serve/src/error.rs`. Reject
inputs containing `..`, `/`, `\\`, or empty.

---

## [x] T0-4: Move generic webhook behind auth

**Commit**: `d5e2a353 T0-4: Mount generic webhook under authenticated API router`

**Files**:

- `crates/roko-serve/src/routes/webhooks.rs:29-37` — split into `public_routes` (GitHub, Slack) and `authenticated_routes` (`/api/webhooks/generic`).
- `crates/roko-serve/src/routes/mod.rs:121, 171` — `webhooks::authenticated_routes()` merged into `api`; `webhooks::public_routes()` merged at the top level.

**What landed**: `/api/webhooks/generic` accepts arbitrary JSON. It now sits
under the auth layer. GitHub and Slack webhooks remain public because they
verify HMAC signatures.

**Verify**:

```bash
rg 'webhooks::authenticated_routes|webhooks::public_routes' crates/roko-serve/src/routes/mod.rs
cargo test -p roko-serve webhook --lib
```

---

## [x] T0-5: Validate agent registration URLs against SSRF

**Commit**: `cf186346 T0-5: Validate agent registration URLs against SSRF`

**File**: `crates/roko-serve/src/routes/agents.rs:1635, 1718-1727, 1760+`

**What landed**: `validate_agent_url(url)` parses the URL, allows only
`http`/`https`, and rejects loopback/private/link-local hosts:
`localhost`, `127.0.0.1`, `[::1]`, `10.*`, `172.16-31.*`, `192.168.*`,
`169.254.*`, `fe80::`. Called in `RegisterAgentRequest::validate_payload()`
for every endpoint URL (rest, websocket, a2a, mcp).

**Verify**:

```bash
cargo test -p roko-serve validate_agent_url --lib
rg 'validate_agent_url' crates/roko-serve/src/routes/agents.rs
```

If a new agent endpoint URL field is added, it must call `validate_agent_url`
before any server-side fetch.

---

## [x] T0-6: Fix knowledge sink filename mismatch

**Commit**: `91b58eb3 T0-6: Align knowledge sink filename with neuro admission constant`

**Files**:

- `crates/roko-cli/src/commands/plan.rs:389` — uses path that resolves to `knowledge-candidates.jsonl`.
- `crates/roko-neuro/src/admission.rs:26` — `DEFAULT_KNOWLEDGE_CANDIDATES_FILE = "knowledge-candidates.jsonl"`.

**What landed**: The sink writes to the same filename the neuro reader
expects (hyphen, not underscore). The orphaned-output bug is gone.

**Verify**:

```bash
rg 'knowledge_candidates\.jsonl' crates/ -g '*.rs'
# Should be empty (variable names ok; filename literal not)
rg 'knowledge-candidates\.jsonl' crates/ -g '*.rs'
# Should appear in admission.rs and runtime_feedback/knowledge.rs
```

T4-29 (plan 14) wires `with_ingestor()` so writes are also ingested live.

---

## [x] T0-7: Fix duplicate model slugs with wrong context_windows

**Commits**: `94291d0d T0-7: Fix duplicate model alias context_window values` + `bab972e3 T0-7: ...minimal`

**File**: `roko.toml`

**What landed**: Six aliases corrected to match canonical model context
windows. Verify by inspecting the relevant blocks; the audit's table is the
canonical reference (sonnet 200000, opus 200000, gemini-pro 1048576,
kimi-k25 262144, kimi-k26 262144, sonar 127000).

**Verify**:

```bash
rg -A1 'context_window' roko.toml | rg -B1 '^(sonnet|opus|gemini-pro|kimi-k25|kimi-k26|sonar)'
```

A regression here would cause incorrect prompt truncation and silent quality
loss.

---

## Anti-Patterns to Watch (Tier 0 specific)

Even though this tier is done, future config / serve work must follow:

1. **Every new secret field updates `mask_secret_fields`** in the same PR.
   The masking is a chokepoint; missing it leaks via the public TOML/JSON
   endpoints.
2. **Every new route group uses `validate_path_segment`** when it accepts
   user-supplied IDs that hit the filesystem.
3. **Every new external URL field calls `validate_agent_url`** (or an
   equivalent SSRF check) before being fetched.
4. **Every new sink shares the constant** for its filename. Don't reintroduce
   the underscore/hyphen drift.
5. **Every new model alias** validates `context_window` against the backing
   model's spec; add a unit test if you can.

---

## Status

- [x] T0-1 — Mask secrets in `GET /api/config/toml`
- [x] T0-2 — Expand `mask_secret_fields` coverage
- [x] T0-3 — Add path validation to shared runs
- [x] T0-4 — Move generic webhook behind auth
- [x] T0-5 — Validate agent registration URLs against SSRF
- [x] T0-6 — Fix knowledge sink filename mismatch
- [x] T0-7 — Fix duplicate model slugs with wrong context_windows

**Tier 0 complete.** Move on to Tier 2 (`12-tier2-delete-dead-code.md`).
