# Security, Auth, And Scope Matrix

This ledger is the canonical trust-boundary map for roko. It connects HTTP route
auth, the relay/proxy pass-throughs, terminal/shell tool safety, the ACP
tool-permission gate, MCP server trust, workspace/worktree isolation, and secret
handling. Every row is verified against current code with `file:line` evidence
(git HEAD `5852c93c05`, `main`). Tags: **Wired** (enforced), **Partial** (built
but incompletely wired), **Stub** (present, informational only), **Missing**
(no enforcement), **Stale** (doc claim not matching code).

> Security is high-stakes. The P0 rows below are exploitable today on any
> non-loopback deployment. Read the P0/P1 roadmap first.

---

## TL;DR — Exploitability-Ranked Findings

| # | Severity | Boundary | One-line gap |
|---|---|---|---|
| P0-1 | **Critical** | Relay proxy | `/relay/{*path}` + `/relay/*/ws` are merged at the **top-level router, outside `/api`**, so `require_api_key`/`require_scope` never run — writes proxied fully unauthed. `routes/mod.rs:248`. |
| P0-2 | **Critical** | Serve scope fallback | Unlisted mutating `/api/*` routes fall through to `"read"`, which always passes → a read-scoped key can `POST` run/jobs/dream/research/deploy/gateway/etc. `middleware.rs:385`. |
| P0-3 | **High** | ACP permission gate | `request_permission` is fully built + tested but has **zero production callers**; `write_file`/`edit_file`/`bash` execute unconditionally. `builtin_tools.rs:269,291-300`; `bridge_events.rs:768,2926`. |
| P1-1 | **High** | `config show --effective` | `serialize_effective` is a bare TOML dump with no redaction, run **after** secret interpolation → prints API keys/tokens in plaintext. `loader.rs:567`; `config_cmd.rs:222-228`. |
| P1-2 | **High** | Safety post-checks | SecretLeak / PathEscape / ContractViolation are `Warn` severity only — they log, they do not block the turn or the write. `safety/mod.rs:767,780,803`. |
| P1-3 | **High** | Workspace create | `body.prefix` joined into a `temp_dir()` path with no sanitization (traversal), and resolved config (interpolated secrets) written to the new `roko.toml`. `workspaces.rs:101-102,128`. |
| P1-4 | **Medium** | Privy JWT → admin | Any structurally valid, signature-passing Privy JWT is granted `"admin"` scope with no membership/role check. `middleware.rs:202-213`. |
| P1-5 | **Medium** | Terminal PTY | When enabled on loopback, PTY spawns arbitrary shells with **no auth**; `ROKO_SERVER_AUTH_TOKEN` is injected into the shell env. `routes/mod.rs:210-222`; `terminal.rs:156`. |
| P2-1 | **Medium** | SSE/WS scrub gap | `scrub_secrets` deliberately skips `text/event-stream`; SSE/WS producers must self-scrub and largely do not. `middleware.rs:535`. |
| P2-2 | **Medium** | MCP stdio trust | Session/agent MCP servers spawn arbitrary configured commands and inherit env/stderr — trusted local code execution. `mcp/client.rs`, `mcp/bridge.rs`. |
| P2-3 | **Low** | Legacy api_key → admin | Single plaintext `api_key` maps straight to `"admin"` with no expiry. `middleware.rs:184-193`. |

---

## Trust-Boundary Matrix (surface → authz mechanism → gap)

| Trust boundary (surface) | Authz mechanism today | Enforcement point | Status | Gap |
|---|---|---|---|---|
| `/api/*` REST | API-key / Bearer / agent-token / Privy JWT + coarse scope matrix | `require_api_key` + `require_scope` layered *only* on the `/api` nest when `auth.enabled` | **Wired** (auth) / **Partial** (scope) | Scope matrix is prefix-based with a `read` catch-all (P0-2). Layers absent entirely when `auth.enabled=false`. |
| `/relay/{*path}`, `/relay/*/ws`, `/relay` | **None** | merged at top-level router (`routes/mod.rs:248`), outside `/api` | **Missing** | Any client can proxy arbitrary GET/POST/DELETE + WS to the internal agent-relay unauthed (P0-1). |
| `/api/rpc/*` (rpc_proxy) | Same as `/api/*` | inside `/api` nest (`routes/mod.rs:190`) | **Wired** | Contrast with relay: this one *is* behind auth. Good. |
| `/health`, `/ready`, `/metrics` | Intentionally public | top-level, pre-auth (`routes/mod.rs:235-239`) | **Wired** | Prometheus `/metrics` unauthed — acceptable but confirm no label leaks. |
| Webhook public routes | Signature verify (per-provider) | `webhooks::public_routes()` top-level (`routes/mod.rs:240`) | **Wired** | Server-to-server; confirm every provider verifies HMAC before side effects. |
| Shared-run receipts | Deliberately public reader | `shared_runs::public_routes()` (`routes/mod.rs:243`) | **Wired** | By design (share links). Confirm receipts carry no secrets. |
| Terminal PTY | Config-gated + bind-policy | `terminal_requires_auth = enabled && !loopback && !acknowledge_public_risk` (`routes/mod.rs:144-146,210-222`) | **Partial** | Loopback+enabled = no auth; arbitrary shell; injects `ROKO_SERVER_AUTH_TOKEN` (P1-5). Not on the `/api` scope stack. |
| CORS | Local-origin predicate by default; explicit allow-list; `unsafe_public_cors` wildcard | `cors_layer` (`middleware.rs:469-500`) | **Wired** | Default is safe (loopback only); wildcard requires explicit opt-in + logs a warning. |
| Response secret scrubbing | Shared `LogScrubber` over text/JSON bodies | `scrub_secrets` (`middleware.rs:548-585`) | **Partial** | SSE (`text/event-stream`) and binary bypass by design → streaming producers must self-scrub (P2-1). |
| ACP builtin tools (`write_file`/`edit_file`/`bash`/`web_fetch`) | Path-in-workdir check for file ops; **no** permission prompt | `execute_acp_builtin_tool` (`builtin_tools.rs:269`) | **Missing** (permission) / **Partial** (path) | `request_permission` never called in the tool loop (P0-3). Path check exists (`resolve_path` 217-234) but `bash`/`web_fetch` unconstrained. |
| ACP permission protocol | `session/request_permission` w/ always-allow persistence to `.roko/trust/permissions.json` | `request_permission` (`bridge_events.rs:768`) | **Stub** | Fully built, tested (5154-5236), zero production callers (P0-3). |
| MCP stdio servers | Command/args/env from config; timeout | `mcp/client.rs`, `mcp/bridge.rs`, `mcp/config.rs` | **Partial** | Spawns arbitrary configured commands, inherits env/stderr → trusted local exec. No env allowlist proof surfaced in `doctor` (P2-2). |
| `roko-agent` safety pre-check | Role contracts, allow/deny, network/bash policy | `pre_dispatch_check` (`safety/mod.rs`) | **Wired** | `Block`-severity violations exist for pre-check (702,716,732). |
| `roko-agent` safety post-check | Secret-leak scrub, path-escape, forbidden-tool | `post_dispatch_check` (`safety/mod.rs:749`) | **Partial** | All three are `Warn` only — advisory, non-blocking (P1-2). |
| Config secret display | `serialize_effective` bare TOML dump | `cmd_show_effective` (`config_cmd.rs:222`) → `loader.rs:567` | **Missing** (redaction) | Runs after `resolve_file_secrets`; prints interpolated secrets plaintext (P1-1). `cmd_show` (resolved w/ source tags) also unredacted. |
| Workspace creation | none on `prefix`; writes resolved config | `workspaces.rs:101-102,128` | **Missing** | `prefix` path-traversal into temp; secret-interpolated `roko.toml` written to fresh dir (P1-3). |
| API-key storage | SHA-256 hash + optional expiry; scope string | `match_api_key_entry` (`middleware.rs:115-130`) | **Wired** | Hashes only; expiry honored. Legacy single key → admin, no expiry (P2-3). |
| Agent token | `base64(SHA-256(token))` + expiry → `agent:write` | `try_agent_token` (`middleware.rs:220-247`) | **Wired** | Scoped correctly; `agent:write` does NOT imply `plan:write`/`write`/admin. |
| Privy JWT | JWKS signature validate → `"admin"` | `try_privy_jwt` (`middleware.rs:202-213`) | **Partial** | No workspace/team membership authorization; every dashboard user is admin (P1-4). |
| Rate limiting | Global 100 req/s single bucket | `rate_limit_middleware` (`routes/mod.rs:106-122`) | **Wired** | Not per-key/per-IP; a single hostile client can consume the whole budget (DoS of others). |
| Body size limit | 4 MiB global, 1 MiB webhook | `DefaultBodyLimit` (`routes/mod.rs:255`) | **Wired** | Reasonable. |

---

## Scope Rules In Code (verified `middleware.rs:356-397`)

`required_scope_for` classifies by **method then path prefix**:

| Route class | Required scope | Line |
|---|---|---|
| `GET` / `HEAD` / `OPTIONS` (any path) | `read` (always passes) | 358 |
| `/api/api-keys`, `/api/secrets`, `/api/config` (mutating) | `admin` | 362-366 |
| `/api/events/ingest` (mutating) | `agent:write` | 370 |
| `/api/agents*` (mutating) | `agent:write` | 374 |
| `/api/plans*`, `/api/prd*` (mutating) | `plan:write` | 378 |
| `/api/workspaces*` (mutating) | `write` | 382 |
| **all other mutating `/api/*`** | **`read` fallback** | 385 |

`is_scope_sufficient` (389-397): `admin` overrides everything; `read` is always
sufficient for a `read` requirement; otherwise exact string match. **There is no
`write ⊇ plan:write ⊇ agent:write` hierarchy** — each is an independent bucket,
and `admin` is the only super-scope.

**The `read` fallback (line 385) is the core defect.** Every mutating route not
in the explicit list above — jobs, run, research, templates, deployments,
gateway, connectors, feeds, bench, subscriptions, dream, neuro, learning,
vision_loop, chain, isfr, workflows, generic webhooks — is protected only by
`read`, and because a `read`-scope credential satisfies a `read` requirement,
**a read-only key can invoke all of them** (P0-2).

---

## Concrete High-Risk Edges (verified)

| Edge | Verified location | Current risk | Required fix / proof |
|---|---|---|---|
| Relay proxy unauthed | `routes/mod.rs:248`; `relay_proxy.rs:23-31,93-118` | Full read/write proxy + 2 WS bridges to internal relay with no auth layer | Nest relay under `/api` or add `require_api_key`+scope to the relay router; add a test asserting 401 without a key. |
| Scope `read` fallback | `middleware.rs:385` | Read key mutates run/jobs/dream/research/deploy/… | Replace fallback with `write`/deny-by-default; CI test that fails when a new mutating route isn't explicitly classified. |
| ACP no permission call | `builtin_tools.rs:291-300`; `bridge_events.rs:2926` | `write_file`/`edit_file`/`bash` run with no prompt | Call `request_permission` before executing `needs_permission` tools; E2E for write/bash/fetch. |
| `bash`/`web_fetch` unconstrained | `builtin_tools.rs:126,678,299` | `bash` runs any command in workdir; `web_fetch` has no private-network block visible | Route through `roko-agent` bash/network policy; add SSRF DNS-resolution block. |
| `config show --effective` leak | `config_cmd.rs:222-228`; `loader.rs:567` | Prints interpolated secrets plaintext | Redact secret-typed fields in `serialize_effective`; add a redaction test with a seeded API key. |
| Post-check Warn-only | `safety/mod.rs:767,780,803` | Secret leak / path escape / forbidden write logged, not blocked | Promote SecretLeak + PathEscape to `Block` and fail the turn; keep governance advisory if intended. |
| Workspace prefix traversal | `workspaces.rs:101-102` | `prefix="../.."` escapes temp dir | Sanitize/allow-list `prefix` chars; reject `.` / `/`. |
| Workspace secret write | `workspaces.rs:128` | Interpolated `roko.toml` written to new dir | Write source (un-interpolated) config or `${VAR}` placeholders. |
| Privy → admin | `middleware.rs:212` | Valid JWT = admin, no team check | Add membership/role authorization or document Privy-as-admin-only deployment. |
| Terminal auth token in env | `terminal.rs:156` | Shell sees `ROKO_SERVER_AUTH_TOKEN` | Avoid injecting unless required; ensure PTY stream scrubs it before publish. |
| Terminal loopback no-auth | `routes/mod.rs:144-146,217` | Any loopback caller gets a shell when enabled | Require explicit write/admin scope even on loopback; document risk. |
| Auth-disabled `x-user-id` spoof | `middleware.rs:338-342`; team routes | With `auth.enabled=false`, callers self-assert identity | Make the auth-disabled trust boundary explicit; guard team mutations. |
| SSE/WS scrub bypass | `middleware.rs:535` | Streamed secrets skip the scrubber | Scrub at producer for agent output, traces, terminal, event ingest. |
| MCP stdio arbitrary exec | `mcp/client.rs`, `mcp/bridge.rs` | Configured command = local code execution | Treat as trusted; surface command/env in `doctor`; add env allowlist + stderr scrub. |
| Rate limit not per-caller | `routes/mod.rs:90-99` | One client drains the shared 100 rps bucket | Key the limiter by API-key/IP. |

---

## Cross-Cutting Drift (for navigation layer)

- **Route-count drift**: CLAUDE.md and older docs say "~85 routes"; the router
  merges 40+ route groups (`routes/mod.rs:148-191`) plus nested provider/model/
  routing routers and top-level relay/terminal/webhook/ws — the effective count
  is materially higher and un-audited. A generated route→scope manifest is the
  fix and is still absent.
- **Auth-boundary drift**: docs imply "`/api/*` is authed"; in reality the
  **relay proxy and terminal live outside `/api`** and thus outside the auth
  stack. Any navigation doc describing the auth perimeter must call these out.
- **ACP "wired" drift**: `00-INDEX`/CLAUDE-adjacent notes treat ACP permissions
  as complete; they are **Stub** (built+tested, zero callers). Downstream docs
  claiming safe tool execution inherit this error.
- **Safety "enforced" drift**: post-dispatch checks are described as enforcement;
  they are advisory `Warn`. Any doc asserting roko "blocks secret leaks" is
  Stale.
- **Undocumented subsystem**: `.roko/trust/permissions.json` (always-allow
  persistence, `bridge_events.rs:767`) is not covered elsewhere in the
  status-quo pack; it is dead storage until the gate has callers, but it is a
  real trust-state file that belongs in the data-layer doc.
- **Contracts fallback**: `AgentContract` falls back to a permissive default when
  the YAML is missing (per CLAUDE.md "Safety contracts — Partial"), so the
  post-check governance rules frequently evaluate against an empty contract.

---

## Status Matrix (roll-up)

| Boundary | Status |
|---|---|
| `/api/*` auth (when enabled) | Wired |
| `/api/*` scope granularity | Partial (read fallback) |
| Relay proxy auth | **Missing** |
| Terminal auth (loopback) | Partial |
| CORS | Wired |
| Response scrub (text/JSON) | Wired |
| Response scrub (SSE/WS) | Partial |
| ACP permission gate | **Stub** (no callers) |
| ACP path confinement (file ops) | Wired |
| ACP bash/fetch confinement | Missing |
| MCP stdio trust controls | Partial |
| Agent safety pre-check | Wired |
| Agent safety post-check (blocking) | Partial (Warn only) |
| API-key hashing/expiry | Wired |
| Privy JWT authorization | Partial (admin-for-all) |
| Config secret redaction on display | Missing |
| Workspace prefix/secret hygiene | Missing |
| Rate limit / body cap | Wired |

---

## Checklist

- [ ] **P0** Move `relay_proxy::routes()` under the auth stack (nest in `/api` or wrap in `require_api_key`+`require_scope`); add a 401-without-key test.
- [ ] **P0** Replace the `read` scope fallback (`middleware.rs:385`) with deny/`write` default; add a CI test that fails when a mutating route lacks explicit classification.
- [ ] **P0** Call `request_permission` before `write_file`/`edit_file`/`bash` in the ACP tool loop; add E2E for write, bash, fetch, and MCP calls.
- [ ] **P1** Redact secret-typed fields in `serialize_effective`; test `config show --effective` with a seeded key.
- [ ] **P1** Promote SecretLeak + PathEscape post-checks to `Block` and fail the turn.
- [ ] **P1** Sanitize workspace `prefix` (reject `.`/`/`) and write un-interpolated config to new `roko.toml`.
- [ ] **P1** Add Privy/team membership authorization test, or document Privy-as-admin explicitly.
- [ ] **P2** Scrub SSE/WS at the producer (agent output, traces, terminal, event ingest).
- [ ] **P2** Route ACP `bash`/`web_fetch` through `roko-agent` bash/network policy; add DNS-to-private-network SSRF block.
- [ ] **P2** Surface MCP command/env/allowlist in `roko doctor`; add stderr-scrub + env-allowlist proof.
- [ ] **P2** Require explicit write/admin scope for terminal even on loopback; avoid injecting `ROKO_SERVER_AUTH_TOKEN`.
- [ ] **P3** Key the rate limiter by API-key/IP.
- [ ] **P3** Deprecate the legacy single `api_key`→admin path in favor of named keys.
- [ ] **P3** Generate a route→scope manifest from the router assembly for navigation docs.

---

## Ordered Roadmap (ranked by exploitability)

1. **Relay proxy auth (P0-1)** — smallest diff, biggest win; it's an unauthed
   write proxy on the public port. One-line router move + one test.
2. **Scope `read` fallback (P0-2)** — flip the default to deny/`write` and add
   the CI classifier test so it can't regress.
3. **ACP permission gate (P0-3)** — wire the already-built `request_permission`
   into the two tool loops; this is connection work, not new code.
4. **Config secret redaction (P1-1)** — redact in `serialize_effective`; blocks
   the easiest local secret exfil.
5. **Blocking post-checks (P1-2)** — promote SecretLeak/PathEscape to `Block`.
6. **Workspace hygiene (P1-3)** — sanitize `prefix`, stop writing secrets.
7. **Privy authorization (P1-4)** — membership check or documented assumption.
8. **Terminal hardening (P1-5)** — scope requirement + drop token injection.
9. **SSE/WS producer scrub, MCP allowlist, SSRF block, per-key rate limit (P2)**.
10. **Legacy-key deprecation + route manifest generation (P3)**.
</content>
</invoke>
