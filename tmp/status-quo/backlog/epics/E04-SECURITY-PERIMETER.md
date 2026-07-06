# E04 — Security Perimeter

> **Epic owner surface**: `roko-serve` (HTTP control plane), `roko-acp` (ACP tool loop),
> `roko-agent` (safety funnel), `roko-cli` (custody, config, worker, workspaces).
> **Stakes**: High. Three P0s are exploitable on any non-loopback deployment **today**.
> This epic is also a **self-execution prerequisite**: unattended agents write files, run
> `bash`, and mutate state, so the safety funnel + audit chain must actually enforce before
> roko is allowed to develop itself without a human in the loop.
>
> **Evidence base** (git HEAD `5852c93c05`, verified `file:line`): source docs
> `75-SECURITY-AUTH-SCOPE-MATRIX`, `99-TRACE-AGENT-TURN`, `33-AGENT-SAFETY`,
> `100-TRACE-ACP-SESSION`. Every finding below was re-verified against code while authoring
> this epic (see Verification Log at the end).

---

## 1. Findings — ranked by exploitability

| ID | Sev | Boundary | Gap (verified) | Evidence |
|----|-----|----------|----------------|----------|
| **F1 / P0-1** | **Critical** | Relay proxy | `relay_proxy::routes()` is `.merge()`d at the **top-level router, outside `/api`**, so `require_api_key` + `require_scope` never run. Any client proxies arbitrary GET/POST/DELETE + 2 WS bridges to the internal agent-relay **fully unauthed**. | `routes/mod.rs:248`; `routes/relay_proxy.rs:23-31,92-118` |
| **F2 / P0-2** | **Critical** | Serve scope fallback | `required_scope_for` returns `"read"` for **all** unlisted mutating `/api/*` routes (line 385), and `is_scope_sufficient` treats `read` as always-satisfied (393). A read-scoped key can `POST` run/jobs/dream/research/deploy/gateway/neuro/learning/etc. | `routes/middleware.rs:356-397` (fallback 385) |
| **F3 / P0-3** | **High** | ACP permission gate | `request_permission` (fail-closed, AlwaysAllow-persisting) is fully built + tested but has **zero production callers**; `write_file`/`edit_file`/`bash` run unconditionally. Structurally unwireable without a reply channel: the gate needs `transport` + `&mut session`, both stranded behind the `tokio::spawn` task boundary at `bridge_events.rs:1320`. | `builtin_tools.rs:291-297` (unconditional exec), `173`/`326` (dead `needs_permission`); `bridge_events.rs:768-950` (gate, tests-only callers 5154/5181/5218); design in doc `100` §A |
| **F4 / P1-c** | **High** | Safety funnel | Per-tool `pre_dispatch_check`/`post_dispatch_check` run only on the openai-compat ToolLoop + orchestrate/ACP paths. The **default Claude-CLI provider path (`dispatch_v2.rs`) contains no safety calls** — the delegated `claude` binary self-polices; roko has no visibility. | `grep post_dispatch_check` → `orchestrate.rs`, `safety/mod.rs`, `roko-acp/*` but **not** `dispatch_v2.rs`; doc `99` row "Permission gate: external" |
| **F5 / P1-2** | **High** | Safety post-checks | SecretLeak / PathEscape / ContractViolation are emitted at **`Warn` severity only** — they log, they do not block the turn or the write. | `safety/mod.rs:767,780,803` |
| **F6 / P1-1** | **High** | `config show --effective` | `serialize_effective` is a bare TOML dump run **after** `resolve_file_secrets`, with no redaction → prints interpolated API keys/tokens in plaintext. | `config_cmd.rs:60-63,222`; `loader.rs:567` |
| **F7 / P1-e** | **Medium** | Custody verify | `cmd_custody_verify` checks only monotonic timestamps + non-empty `action`/`principal`. **No hash-chaining** (no `prev_hash` link, no digest recomputation) → prints "OK" for a tampered/rewritten log. | `custody.rs:183-235` |
| **F8 / P1-3** | **Medium** | Workspace create | `body.prefix` joined into a `temp_dir()` path with no sanitization (traversal), and secret-interpolated config written to the new `roko.toml`. | `workspaces.rs:101-102,128` |
| **F9 / P1-g** | **Medium** | Worker callback | Deployed-worker result callback carries **no auth header**; the control plane accepts unauthenticated status posts. | `worker` callback path (see F9 verify) |
| **F10 / P1-4** | **Medium** | Privy JWT → admin | Any signature-valid Privy JWT is granted `"admin"` with no membership/role check. | `middleware.rs:202-213` |
| **F11 / P1-5** | **Medium** | Terminal PTY | On loopback+enabled, PTY spawns arbitrary shells with **no auth**, and injects `ROKO_SERVER_AUTH_TOKEN` into the shell env. | `routes/mod.rs:210-222`; `terminal.rs:156` |
| **F12 / P2-1** | **Medium** | SSE/WS scrub | `scrub_secrets` deliberately skips `text/event-stream`; streaming producers must self-scrub and largely do not. | `middleware.rs:535` |
| **F13 / P2** | **Medium** | ACP bash/fetch | `bash`/`web_fetch` run unconstrained (workdir jail only for file ops); no SSRF/private-network block. | `builtin_tools.rs:126,299,678` |
| **F14 / P2-2** | **Low** | MCP stdio trust | Session/agent MCP servers spawn arbitrary configured commands, inherit env/stderr; no allowlist surfaced in `doctor`. | `mcp/client.rs`, `mcp/bridge.rs` |
| **F15 / P3** | **Low** | Rate limit | Global single 100 rps bucket, not keyed per API-key/IP → one client drains everyone's budget. | `routes/mod.rs:90-122` |

**The three P0s (fix first, in order):** F1 (relay unauth), F2 (scope read-fallback), F3 (ACP permission gate).

---

## 2. Reconciliation with existing plans P16 / P22

Both plans predate this security pass and are **partial mitigations**, not closures.

### P16-safety-contracts (5 tasks, `plans/P16-safety-contracts/tasks.toml`)
- **What it does**: adds `disallowed_tools` to `CliDispatchRequest` + `AgentSpawnConfig`, a
  `forbidden_tool_names()` helper on `AgentContract`, and loads the role contract at runner
  dispatch to pass `--disallowed-tools` to the Claude CLI (event_loop.rs). Bridge path only logs.
- **Overlap**: partially mitigates **F4** by handing the CLI a deny-list, so some dangerous
  tools are blocked *by the claude binary*. It does **not** invoke roko's own `pre/post_dispatch_check`,
  does not promote post-checks to `Block` (**F5**), and does not cover the openai-compat/ACP paths.
- **Verdict**: **keep P16 as a prerequisite for E04-T06/T07**. E04 layers roko-side enforcement
  on top of P16's plumbing. P16 T5 itself documents the bridge path as a known unenforceable gap.

### P22-acp-tool-permission (5 tasks, `plans/P22-acp-tool-permission/tasks.toml`)
- **What it does**: replaces `ToolContext::testing()` with real `ToolContext::new()` in three ACP
  loops (T1), adds a **static `denied_tools`/`allowed_tools` check** to `AcpBuiltinToolHandler::execute`
  (T2), per-slash-command allowlists (T3), wires them into `ToolContext` (T4), adds tests (T5).
- **Explicit non-goal**: P22 T2 anti-pattern says *"Do NOT try to call `request_permission()` here —
  the handler has no transport reference."* So P22 delivers a **static, non-interactive** allow/deny
  filter — a real improvement, but **not** the fail-closed consent gate F3 requires.
- **Verdict**: **P22 is the substrate for the F3 fix**, not a substitute. E04-T14 (interactive gate)
  **depends_on P22-T2's `denied_tools` infrastructure** and supersedes its "no consent" limitation by
  adding the reply-channel refactor from doc `100` §A. Sequence: land P22 first, then E04-T12→T14.

**Net**: E04 does not duplicate P16/P22. It (a) closes the serve-side P0s they never touched
(F1, F2), (b) upgrades P16's deny-list into roko-side blocking enforcement (F4/F5), and
(c) upgrades P22's static filter into an interactive consent gate (F3).

---

## 3. Task breakdown (E04-Txx)

DAG note: T01/T02 are independent P0 quick wins. T03 guards T02. T12→T13→T14 is the ACP
reply-channel chain and **depends on P22-T2**. T06 depends on **P16** landing.

| Task | Title | Finding | Tier | Files | depends_on |
|------|-------|---------|------|-------|------------|
| **E04-T01** | Move relay proxy under the auth stack | F1 | focused | `routes/mod.rs`, `routes/relay_proxy.rs` | — |
| **E04-T02** | Deny-by-default scope fallback | F2 | focused | `routes/middleware.rs` | — |
| **E04-T03** | CI test: every mutating route explicitly classified | F2 | focused | `routes/middleware.rs` (+ test) | E04-T02 |
| **E04-T04** | Redact secrets in `serialize_effective` / `config show` | F6 | focused | `roko-core .../loader.rs`, `config_cmd.rs` | — |
| **E04-T05** | Promote SecretLeak + PathEscape post-checks to `Block` | F5 | focused | `roko-agent/src/safety/mod.rs` | — |
| **E04-T06** | Invoke safety funnel on default Claude-CLI path | F4 | integrative | `dispatch_v2.rs`, `orchestrate.rs`, `safety/mod.rs` | P16 (all), E04-T05 |
| **E04-T07** | Real hash-chaining in custody append + verify | F7 | integrative | `custody.rs` | — |
| **E04-T08** | Auth the worker result callback | F9 | focused | `roko-cli worker.rs`, `roko-serve` ingest route | E04-T02 |
| **E04-T09** | Sanitize workspace `prefix`; write un-interpolated config | F8 | focused | `routes/workspaces.rs` | — |
| **E04-T10** | Privy JWT membership/role authorization | F10 | focused | `routes/middleware.rs` | — |
| **E04-T11** | Require scope for terminal even on loopback; drop token-in-env | F11 | focused | `routes/mod.rs`, `terminal.rs` | E04-T02 |
| **E04-T12** | Add `CognitiveEvent::PermissionRequest { …, reply: oneshot }` | F3 | integrative | `roko-acp/src/bridge_events.rs` | P22-T2 |
| **E04-T13** | Answer PermissionRequest in parent loop via `request_permission` | F3 | integrative | `bridge_events.rs` (`stream_events_to_editor` 954-1013) | E04-T12 |
| **E04-T14** | Gate `execute_acp_builtin_tool` on the decision (fail-closed) | F3 | integrative | `roko-acp/src/builtin_tools.rs` | E04-T13 |
| **E04-T15** | Producer-side SSE/WS secret scrub | F12 | focused | agent output / trace / terminal / event-ingest producers | — |
| **E04-T16** | Route ACP `bash`/`web_fetch` through network policy + SSRF block | F13 | integrative | `builtin_tools.rs`, `roko-agent` policy | E04-T14 |
| **E04-T17** | Surface MCP command/env allowlist in `roko doctor` | F14 | focused | `mcp/config.rs`, `doctor` cmd | — |
| **E04-T18** | Per-API-key / per-IP rate limiter | F15 | focused | `routes/mod.rs` | — |
| **E04-T19** | Generate route→scope manifest from router assembly | F2 (nav) | focused | `routes/mod.rs` (+ build/test) | E04-T02, E04-T03 |

**Task count: 19** (E04-T01 … E04-T19), plus 2 reconciled upstream plans (P16, P22) treated
as prerequisites.

### Self-execution prerequisites (must land before unattended self-hosting)
Unattended agents write files and run `bash` with no human present. The interactive ACP consent
gate (T12–T14) does **not** help unattended runs (no one to answer the prompt); instead the
**enforcing** controls matter:
- **P16** (tool deny-list plumbing) + **E04-T05** (block SecretLeak/PathEscape) + **E04-T06**
  (safety funnel on the actual default Claude-CLI dispatch path) — together these make the runner
  refuse dangerous writes/secret leaks instead of merely logging them.
- **E04-T07** (custody hash-chain) — so the audit trail of unattended actions is tamper-evident.
- **E04-T02** (deny-by-default scope) + **E04-T01** (relay auth) — required only if the unattended
  loop is driven through `roko-serve`; not on the pure-CLI `plan run` path.

---

## 4. First three tasks (executable native TOML)

```toml
[meta]
plan = "E04-security-perimeter"
total = 19
done = 0
status = "ready"
max_parallel = 2

# ── E04-T01: Move relay proxy under the auth stack (P0-1 / F1) ──
#
# relay_proxy::routes() is merged at the top-level router (routes/mod.rs:248),
# OUTSIDE the /api nest, so require_api_key + require_scope never run. Any
# client proxies arbitrary GET/POST/DELETE + 2 WS bridges to the internal
# agent-relay fully unauthed. Wrap the relay router in the same auth layers
# used for the /api nest and ws routes (only when api_auth.enabled).

[[task]]
id = "E04-T01"
title = "Gate relay proxy routes behind require_api_key + require_scope"
status = "ready"
tier = "focused"
model_hint = "claude-sonnet-4-6"
max_loc = 40
files = ["crates/roko-serve/src/routes/mod.rs", "crates/roko-serve/src/routes/relay_proxy.rs"]
role = "implementer"
depends_on = []

[task.context]
read_files = [
    { path = "crates/roko-serve/src/routes/mod.rs", lines = "224-263", why = "Router assembly. `ws` shows the conditional require_api_key layering pattern (224-231); relay is merged unauthed at 248." },
    { path = "crates/roko-serve/src/routes/relay_proxy.rs", lines = "23-31", why = "routes() returns the relay Router to be wrapped." },
    { path = "crates/roko-serve/src/routes/middleware.rs", lines = "356-397", why = "required_scope_for / is_scope_sufficient — relay writes should require `write` or `agent:write`, not `read`." },
]
symbols = [
    "relay_proxy::routes() -> Router<Arc<AppState>> — relay_proxy.rs:23",
    "middleware::require_api_key — layered on ws when api_auth.enabled",
    "middleware::require_scope — scope enforcement layer",
]
anti_patterns = [
    "Do NOT leave relay merged outside the auth layers — that is the bug.",
    "Do NOT auth relay when api_auth.enabled is false — match the existing ws/api conditional so local/dev is unchanged.",
    "Do NOT break the WS upgrade — apply the same layer pattern used for ws::routes() so /relay/*/ws still upgrades.",
]

# Steps:
# 1. Mirror the `ws` binding (mod.rs:224-231): build `let relay = if api_auth.enabled {
#    relay_proxy::routes().layer(from_fn_with_state(state, require_api_key))
#        .layer(from_fn_with_state(state, require_scope)) } else { relay_proxy::routes() };`
# 2. Replace `.merge(relay_proxy::routes())` at line 248 with `.merge(relay)`.
# 3. Confirm relay mutations resolve to a non-`read` scope (they hit `/relay/...`,
#    not `/api/...`, so add a relay arm to required_scope_for or force `write`).

[[task.verify]]
phase = "structural"
command = "! grep -n 'merge(relay_proxy::routes())' crates/roko-serve/src/routes/mod.rs | grep -q ."
fail_msg = "relay_proxy must no longer be merged unwrapped; it must go through the auth layers"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-serve 2>&1"
fail_msg = "roko-serve must compile"

[[task.verify]]
phase = "test"
command = "cargo test -p roko-serve -- relay_requires_auth 2>&1"
fail_msg = "an integration test must assert GET/POST/WS on /relay/* returns 401 without an API key when auth is enabled"

acceptance = "With auth.enabled=true, an unauthenticated GET/POST/DELETE to /relay/* and a WS upgrade to /relay/agents/ws all return 401; with a valid key + sufficient scope they proxy as before; with auth.enabled=false behavior is unchanged."


# ── E04-T02: Deny-by-default scope fallback (P0-2 / F2) ──
#
# required_scope_for (middleware.rs:385) returns "read" for every unlisted
# mutating /api route, and is_scope_sufficient treats "read" as always-satisfied,
# so a read key can POST run/jobs/dream/research/deploy/etc. Flip the fallback
# to a mutating default ("write") so read keys are rejected on unclassified
# mutations.

[[task]]
id = "E04-T02"
title = "Change unlisted-mutating-route scope fallback from read to write"
status = "ready"
tier = "focused"
model_hint = "claude-sonnet-4-6"
max_loc = 20
files = ["crates/roko-serve/src/routes/middleware.rs"]
role = "implementer"
depends_on = []

[task.context]
read_files = [
    { path = "crates/roko-serve/src/routes/middleware.rs", lines = "356-397", why = "required_scope_for classifies by method+prefix; the `read` fallback at 385 is the defect. is_scope_sufficient at 389-397 (read always passes)." },
]
symbols = [
    "required_scope_for(method: &Method, path: &str) -> &'static str — middleware.rs:356",
    "is_scope_sufficient(has: &str, required: &str) -> bool — middleware.rs:389",
]
anti_patterns = [
    "Do NOT change the GET/HEAD/OPTIONS `read` return (358) — read methods stay read.",
    "Do NOT weaken the admin/agent:write/plan:write/write classifications above the fallback.",
    "Do NOT invent a scope hierarchy here — only change the terminal fallback (385) from `read` to `write`.",
]

# Steps:
# 1. Replace the trailing `"read"` at line 385 with `"write"` so any mutating
#    /api route not explicitly classified requires at least a `write` scope.
# 2. Leave is_scope_sufficient as-is (admin overrides; exact match otherwise).

[[task.verify]]
phase = "structural"
command = "awk 'NR==385' crates/roko-serve/src/routes/middleware.rs | grep -q '\"write\"'"
fail_msg = "the mutating-route fallback must default to write, not read"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-serve 2>&1"
fail_msg = "roko-serve must compile"

[[task.verify]]
phase = "test"
command = "cargo test -p roko-serve -- read_scope_denied_on_mutation 2>&1"
fail_msg = "a test must assert a read-scoped key gets 403 on POST /api/jobs (an unlisted mutating route)"

acceptance = "A caller holding only a `read` scope receives 403 on POST to any previously-unclassified mutating /api route (jobs, run, dream, research, deploy, gateway, neuro, learning). Admin and route-specific scopes still succeed. GET requests are unaffected."


# ── E04-T03: CI test — every mutating route is explicitly classified (F2 regression guard) ──
#
# The deny-by-default fallback (T02) is safe but silent: a new mutating route
# will inherit `write` without a deliberate scope decision. Add a test that
# enumerates the assembled router's mutating routes and fails if any resolves
# to a scope only via the fallback, forcing explicit classification.

[[task]]
id = "E04-T03"
title = "CI guard: fail when a mutating route lacks explicit scope classification"
status = "ready"
tier = "focused"
model_hint = "claude-sonnet-4-6"
max_loc = 60
files = ["crates/roko-serve/src/routes/middleware.rs"]
role = "implementer"
depends_on = ["E04-T02"]

[task.context]
read_files = [
    { path = "crates/roko-serve/src/routes/middleware.rs", lines = "356-397", why = "required_scope_for is the classifier under test." },
    { path = "crates/roko-serve/src/routes/mod.rs", lines = "148-263", why = "Router assembly enumerates the route groups whose mutating paths must be classified." },
]
symbols = [
    "required_scope_for(method, path) -> &'static str — middleware.rs:356",
    "build_router / api nest assembly — routes/mod.rs",
]
anti_patterns = [
    "Do NOT hardcode a route list that silently drifts from the router — derive from a maintained EXPECTED_MUTATING_ROUTES const and assert each maps to a non-fallback scope, and add a note to update it when adding routes.",
    "Do NOT mark the fallback scope itself as acceptable — the test must distinguish an explicit `write` classification from the fallback `write`.",
]

# Steps:
# 1. Introduce a sentinel: have required_scope_for return a distinct
#    `\"write:unclassified\"` (or expose a bool) for the fallback branch so the
#    test can tell explicit-write from fallback-write. Map both to `write` for
#    is_scope_sufficient, but let the test detect the fallback.
# 2. Add a #[test] that iterates a maintained list of representative mutating
#    paths (POST /api/jobs, /api/run, /api/dream, /api/deployments, ...) and
#    asserts none resolve via the fallback sentinel — every one is explicitly
#    classified.

[[task.verify]]
phase = "structural"
command = "grep -q 'unclassified\\|fallback' crates/roko-serve/src/routes/middleware.rs"
fail_msg = "a fallback sentinel must exist so the guard test can detect unclassified mutating routes"

[[task.verify]]
phase = "test"
command = "cargo test -p roko-serve -- mutating_routes_are_classified 2>&1"
fail_msg = "the guard test must pass and must fail if a mutating route falls through to the scope fallback"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-serve 2>&1"
fail_msg = "roko-serve must compile"

acceptance = "A test enumerates mutating /api routes and fails CI if any resolves to a scope only through the fallback branch, so adding a new mutating route without an explicit scope classification breaks the build."
```

---

## 5. Remaining task stubs (E04-T04 … E04-T19)

These follow the same schema; key fields captured in the table in §3. The load-bearing
acceptance/verify notes for the security-critical ones:

- **E04-T04 (F6)** — verify: seed `MY_KEY=sk-secret`, run `config show --effective`, assert the
  literal secret is absent and a redaction marker present. Redact in `serialize_effective`
  (`loader.rs:567`) *after* interpolation, keyed on secret-typed fields.
- **E04-T05 (F5)** — verify: a turn whose output contains a seeded secret / a write escaping the
  workdir is **blocked** (turn fails), not merely `Warn`-logged. Promote at `safety/mod.rs:767,780`.
- **E04-T06 (F4)** — verify: a `plan run` on the **default Claude-CLI provider** exercises
  `pre/post_dispatch_check`; assert via a spy sink that the funnel ran (today it does not, per the
  `grep` showing `dispatch_v2.rs` has no safety calls). Depends on **P16** + T05.
- **E04-T07 (F7)** — verify: append two records, tamper with the first's payload, `custody verify`
  reports a **chain break**; today it prints OK. Add `prev_hash` link + digest recompute in
  `custody.rs` append + verify.
- **E04-T12→T14 (F3)** — the ACP consent chain from doc `100` §A: add
  `CognitiveEvent::PermissionRequest { action, title, detail, reply: oneshot::Sender<PermissionDecision> }`
  (T12), answer it in `stream_events_to_editor` (954-1013) by calling the existing
  `request_permission` (768) then `reply.send(decision)` (T13), and gate
  `execute_acp_builtin_tool` (`builtin_tools.rs:291`) so `needs_permission` tools only run on
  `Allow`/`AlwaysAllow` (T14). **Depends on P22-T2.** Conformance test: an outbound
  `session/request_permission` precedes any `write_file`; a `Reject` blocks the write;
  `AlwaysAllow` persists to `.roko/trust/permissions.json`.

---

## 6. Verification log (authored against HEAD 5852c93c05)

| Finding | Re-verified | Result |
|---|---|---|
| F1 | `routes/mod.rs:248`, `relay_proxy.rs:23-31,92-118` | Confirmed: `.merge(relay_proxy::routes())` at top level, no auth layer; `relay_proxy` forwards any method + WS. |
| F2 | `routes/middleware.rs:356-397` | Confirmed: fallback returns `"read"` (385); `is_scope_sufficient` passes any `read` requirement (393). |
| F3 | doc `100` lines 41,60-62,170-175; matrix `75` P0-3 | Confirmed design: gate stranded by `tokio::spawn` (1320); reply-channel refactor required. |
| F4 | `grep post_dispatch_check` → orchestrate/safety/roko-acp only; **absent** in `dispatch_v2.rs` | Confirmed: default Claude-CLI path has no safety funnel calls. |
| F6 | `config_cmd.rs:60-63,222` | Confirmed: `cmd_show_effective` path; no redaction. |
| F7 | `custody.rs:183-235` | Confirmed: only monotonic-timestamp + non-empty-field checks; no hash chain. |

> Not independently re-opened this pass (inherited from doc `75` with file:line): F5, F8–F15.
> Their fixes are scoped in §3/§5; re-verify at task pickup.
