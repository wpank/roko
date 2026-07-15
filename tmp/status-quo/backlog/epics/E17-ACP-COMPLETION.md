# E17 — ACP Completion

> **Epic owner surface**: `roko-acp` (the ACP/Zed editor bridge — `bridge_events.rs`,
> `builtin_tools.rs`, `handler.rs`, `types.rs`).
> **Goal**: close the four ways the ACP session surface diverges from every other roko
> dispatch path — the permission gate never fires, the learning loop is write-only, MCP
> session tools reach only some providers, and the capabilities roko advertises/enforces are
> untruthful. When this epic lands, an editor-driven ACP turn behaves like a `plan run` turn:
> consent-gated, learning-informed, MCP-equipped, and honest about what it can do.
>
> **Evidence base** (git HEAD `5852c93c05`, re-verified `file:line` while authoring): source
> docs `100-TRACE-ACP-SESSION`, `51-ACP`. See the Verification Log (§6).
>
> **Cross-epic dependencies**: **E04** (Security Perimeter — owns the F3 permission
> reply-channel), **E07** (Learning & Knowledge — owns the router/experiment durability),
> **E15** (MCP — not yet authored; owns cross-provider MCP wiring). Plus five in-flight ACP
> plans (P19/P21/P22/P25/P28) reconciled in §2.

---

## 1. Findings — verified against HEAD `5852c93c05`

| ID | Area | Gap (verified) | Evidence |
|----|------|----------------|----------|
| **Fa** | Permission gate (unwireable) | `request_permission` (fail-closed, `AlwaysAllow`-persisting, tested) has **zero production callers** — only tests `5154/5181/5218`. Tool execution runs `write_file`/`edit_file`/`bash` unconditionally. The gate needs `transport` + `&mut session`; the actual builtin handler `AcpBuiltinToolHandler::execute` takes **`_ctx` (ignored)** and runs inside the detached `tokio::spawn` cognitive task that holds **only `event_sender`**. Structurally unfixable without a reply channel. | `request_permission` `bridge_events.rs:768`; detached spawn `:1320`; handler ignores ctx `:2926`; `execute_acp_builtin_tool` `builtin_tools.rs:269`; `needs_permission` `builtin_tools.rs:173`; dead `tool_needs_permission` `:326`; design in doc `100` §A |
| **Fb** | Learning decisioning (write-only telemetry) | ACP **records but never decides**. `record_cascade_observation` calls only `router.observe(...)` (`:712`) — `select_model` is never invoked. `DaimonPolicy::default()` is hardcoded (`:634`). `ExperimentStore` is never consulted. Same episode gets a *decision* on `plan run` but only *telemetry* on ACP. | `bridge_events.rs:634`, `685-718` (observe), `655` (slug helper); doc `40-LEARNING-TELEMETRY` finding (c) surface #3 |
| **Fc** | MCP session-tool parity | `session/new` accepts `mcpServers` and `session.mcp_servers` is threaded (`:1270`), but only the **openai-compat** cognitive task receives `&session_mcp_servers` and calls `setup_session_mcp_tools` (`:2313`). The **Anthropic** path `run_anthropic_cognitive_task` (`:1444-1457`) is invoked **without the `mcp_servers` arg** → Anthropic ACP sessions get no session MCP tools. | dispatch branch `bridge_events.rs:1440-1476`; setup `:2277-2313`; Anthropic call omits arg `:1444-1457` |
| **Fd** | Untruthful capabilities | `InitializeResult` advertises `image: false, audio: false` (`handler.rs:290-291`) **while image parts are actually built** (openai `image_url` `:4351`, Anthropic `image` `:4378/4395`, used `:1223-1228`). Separately, `tool_context.capabilities` is **hard-forced all-true** (`read/write/exec/git/network`) at `:1829-1835`, `:2360`, `:2514` regardless of role or consent — a client that declined a scope is ignored. | `handler.rs:288-291`; `bridge_events.rs:1829-1835,2360,2514,4351,4378` |

---

## 2. Reconciliation with existing ACP plans (P19 / P21 / P22 / P25 / P28)

Five plans already touch ACP. E17 is the **completion layer over them** — it does not re-own
what they deliver. The mapping below states what each *covers* and what each *punts on*.

| Plan | Covers | Punts on | E17 resolution |
|------|--------|----------|----------------|
| **P19-cascade-router-acp** | Wires cascade **selection** (`select_model`, not just `observe`) + loads real `DaimonState` into the ACP dispatch path; records the decision in episode metadata. | The **`ExperimentStore`** A/B loop — P19 is router+daimon only. | **Adopt P19 for Fb's `select_model`/`DaimonPolicy` half.** E17-T02 covers only the remaining `ExperimentStore` consult. `depends_on_plan = P19`. |
| **P21-acp-streaming** | Streams `run_slash_command` stdout/stderr immediately; wires `AcpProgressSink` under `ROKO_ACP_PROGRESS`. | Nothing permission/learning related — pure output-timing. | **Orthogonal, but co-located.** Both P21 and E17-T01 edit the `stream_events_to_editor` / slash-command region; sequence E17-T01 **after** P21 so the reply-channel answer lands in the post-P21 loop shape. |
| **P22-acp-tool-permission** | Replaces `ToolContext::testing()` with real `ToolContext::new()` (T1); adds a **static `denied_tools`/`allowed_tools`** filter in the handler (T2); per-slash-command allowlists (T3-T5). | The **interactive** consent gate — P22 T2 anti-pattern says *"Do NOT call `request_permission()` here — the handler has no transport reference."* P22 T1 **deliberately keeps capabilities all-true** (anti-pattern line 40). | **P22 is the substrate for Fa, not the fix.** E17-T01 `depends_on_plan = P22` and supersedes its "no consent" limit via the reply-channel. E17-T04 supersedes P22-T1's forced-all-true capabilities (Fd). |
| **P25-mcp-acp-passthrough** | Threads MCP **config** roko.toml → `AgentConfig` → ACP workflow runner → session tool-loop (T1-T4); MCP auto-discovery at session init. | **Per-provider parity** — P25's tool-loop wiring targets the openai-compat/workflow path; it does not add the `mcp_servers` arg to `run_anthropic_cognitive_task`. | **P25 delivers the config plumbing; E17-T03 closes the Anthropic-path gap (Fc).** `depends_on_plan = P25`; coordinate with **E15** (canonical MCP home). |
| **P28-image-support** | Sets ACP `image` capability **from model vision support** (T1 → fixes the `image:false` half of Fd); image injection helper + Anthropic passthrough (T2-T5). | The **`tool_context.capabilities` all-true** forcing (Fd's second half); `audio` advertisement. | **P28 owns the image-capability truthfulness.** E17-T04 covers only the forced tool-capabilities; E17-T05 is the end-to-end "advertised == accepted" guard tying P28 + E17-T04 together. |

**Net**: E17 adds four things none of P19/P21/P22/P25/P28 deliver: (a) the **interactive**
consent gate via a reply channel (Fa, co-owned with E04-T12→T14), (b) the **`ExperimentStore`**
consult (Fb remainder), (c) **Anthropic-path** MCP session tools (Fc), (d) **consent-derived**
tool capabilities + an advertised-vs-accepted guard (Fd remainder).

> **Co-ownership note (Fa / E04)**: E04-T12→T14 already specs the identical reply-channel
> chain from the *security* lens (P0-3). E17-T01 is the same work from the *ACP-completion*
> lens. **Execute once** — whichever epic runs first; the DAG dep (`depends_on_plan` E04)
> prevents a double build. If E04 lands first, E17-T01 collapses to the ACP conformance
> assertion in E17-T06.

---

## 3. Task breakdown (E17-Txx)

DAG: T01 is the reply-channel chain (needs E04 + P22 substrate, after P21). T02/T03/T04 are
independent of T01 and of each other. T05 depends on T04 + P28. T06 is the end-to-end
conformance capstone over T01–T05.

| Task | Title | Finding | Tier | Files | depends_on / _plan |
|------|-------|---------|------|-------|--------------------|
| **E17-T01** | Reply-channel permission gate: emit `PermissionRequest`, answer in parent loop, gate exec fail-closed | Fa | integrative | `roko-acp/src/bridge_events.rs`, `roko-acp/src/builtin_tools.rs` | `_plan`: E04, P22; after P21 |
| **E17-T02** | Consult `ExperimentStore` for ACP prompt/model A/B (P19 gives select; this gives experiment) | Fb | integrative | `roko-acp/src/bridge_events.rs` | `_plan`: P19, E07 |
| **E17-T03** | MCP session-tool parity: thread `session_mcp_servers` into the Anthropic dispatch path | Fc | integrative | `roko-acp/src/bridge_events.rs` | `_plan`: P25, E15 |
| **E17-T04** | Derive `tool_context.capabilities` from role/session consent instead of hard-forced all-true | Fd | focused | `roko-acp/src/bridge_events.rs` | `_plan`: P22 (T1) |
| **E17-T05** | Advertised-vs-accepted capability guard (`image`/`audio`) tying together P28 + T04 | Fd | focused | `roko-acp/src/handler.rs`, `roko-acp/src/types.rs` | E17-T04; `_plan`: P28 |
| **E17-T06** | End-to-end ACP conformance test (consent precedes write; select+experiment recorded; Anthropic MCP tool callable) | Fa–Fd | integrative | `roko-acp/src/bridge_events.rs` (tests) | E17-T01, E17-T02, E17-T03, E17-T04 |

**Task count: 6** (E17-T01 … E17-T06), over **3 cross-epic deps** (E04, E07, E15) and
**5 reconciled ACP plans** (P19, P21, P22, P25, P28) treated as substrate/prerequisites.

---

## 4. First three tasks (executable native TOML)

```toml
[meta]
plan = "E17-acp-completion"
total = 6
done = 0
status = "ready"
max_parallel = 2

# ─────────────────────────────────────────────────────────────────────────────
# E17-T01: Reply-channel permission gate (Fa)
#
# request_permission (bridge_events.rs:768, fail-closed, tested) has zero prod
# callers. The builtin handler AcpBuiltinToolHandler::execute (:2926) takes _ctx
# (ignored) and runs inside the detached tokio::spawn cognitive task (:1320) that
# holds only event_sender — it cannot reach transport/&mut session, which
# request_permission needs. Fix: add a CognitiveEvent::PermissionRequest carrying
# a oneshot reply sender; the cognitive task emits it (instead of executing a
# needs_permission tool) and awaits the decision; the parent stream_events_to_editor
# loop (holds transport+session) calls request_permission and replies through the
# oneshot; execute_acp_builtin_tool then runs the tool ONLY on Allow/AlwaysAllow.
# This is the SAME chain as E04-T12→T14 (security lens) — execute once.
# ─────────────────────────────────────────────────────────────────────────────
[[task]]
id = "E17-T01"
title = "Add PermissionRequest reply channel and gate ACP builtin tool exec fail-closed"
status = "ready"
tier = "integrative"
model_hint = "claude-sonnet-4-6"
max_loc = 120
files = [
    "crates/roko-acp/src/bridge_events.rs",
    "crates/roko-acp/src/builtin_tools.rs",
]
role = "implementer"
depends_on = []
depends_on_plan = ["E04-security-perimeter", "P22-acp-tool-permission"]

[task.context]
read_files = [
    { path = "crates/roko-acp/src/bridge_events.rs", lines = "760-820", why = "request_permission signature (<R,W>, needs transport + &mut session); the fail-closed decision logic to reuse verbatim." },
    { path = "crates/roko-acp/src/bridge_events.rs", lines = "1312-1360", why = "The detached tokio::spawn cognitive task (:1320) — holds only event_sender; this is what must emit PermissionRequest instead of executing." },
    { path = "crates/roko-acp/src/bridge_events.rs", lines = "1479-1520", why = "stream_events_to_editor invocation — the parent loop that owns transport + &mut session and must answer PermissionRequest." },
    { path = "crates/roko-acp/src/builtin_tools.rs", lines = "160-320", why = "needs_permission (:173) classifies write_file/edit_file/bash; execute_acp_builtin_tool (:269) runs them unconditionally; dead tool_needs_permission (:326)." },
]
symbols = [
    "request_permission<R,W>(transport, session, action, ...) -> PermissionDecision — bridge_events.rs:768",
    "enum CognitiveEvent — add PermissionRequest { action, title, detail, reply: tokio::sync::oneshot::Sender<PermissionDecision> }",
    "execute_acp_builtin_tool(call, ctx) — builtin_tools.rs:269, gate on decision",
    "needs_permission(name: &str) -> bool — builtin_tools.rs:173",
    "stream_events_to_editor(...) — parent loop, bridge_events.rs (~954-1013 + invoked :1479)",
]
anti_patterns = [
    "Do NOT execute a needs_permission tool then ask — emit PermissionRequest and AWAIT the decision BEFORE execute_acp_builtin_tool runs (fail-closed).",
    "Do NOT try to call request_permission from the handler/cognitive task — it has no transport (that is exactly Fa). Route through the oneshot to the parent loop.",
    "Do NOT drop the reply sender on the cancel/error path — a dropped oneshot must resolve to Reject, not hang the tool.",
    "Do NOT duplicate E04-T12→T14 — this is the same chain; if E04 landed, reuse its CognitiveEvent variant and only wire the ACP handler side.",
]

# Steps:
# 1. Add CognitiveEvent::PermissionRequest { action, title, detail,
#    reply: oneshot::Sender<PermissionDecision> } (skip if E04-T12 already added it).
# 2. In the cognitive task, when a tool with needs_permission(name)==true is about
#    to run, build a oneshot, send PermissionRequest, await recv() (Err => Reject).
# 3. In stream_events_to_editor, match PermissionRequest, call request_permission
#    (existing fail-closed logic, persists AlwaysAllow to .roko/trust/permissions.json),
#    then reply.send(decision).
# 4. In execute_acp_builtin_tool, run the tool only on Allow | AlwaysAllow; on Reject
#    return a ToolResult error "permission denied" without side effects.

[[task.verify]]
phase = "structural"
command = "grep -q 'PermissionRequest' crates/roko-acp/src/bridge_events.rs && grep -c 'request_permission(' crates/roko-acp/src/bridge_events.rs"
fail_msg = "a PermissionRequest event must exist and request_permission must have a non-test caller"
[[task.verify]]
phase = "compile"
command = "cargo check -p roko-acp 2>&1"
fail_msg = "roko-acp must compile after adding the reply channel"
[[task.verify]]
phase = "test"
command = "cargo test -p roko-acp -- permission_prompt_precedes_write 2>&1"
fail_msg = "a test must assert request_permission fires before write_file executes and Reject blocks the write"

acceptance = "An ACP turn that calls write_file/edit_file/bash emits an outbound session/request_permission BEFORE the tool runs; a Reject decision blocks the side effect and returns a permission-denied ToolResult; an AlwaysAllow persists to .roko/trust/permissions.json and skips future prompts. With no editor answer (dropped reply) the tool is denied, not hung."


# ─────────────────────────────────────────────────────────────────────────────
# E17-T02: Consult ExperimentStore in the ACP path (Fb remainder)
#
# ACP is write-only telemetry: record_cascade_observation only calls
# router.observe (:712); DaimonPolicy::default() is hardcoded (:634). P19 wires
# select_model + real DaimonState. This task closes the LAST inert loop:
# ExperimentStore is never consulted on ACP, so ACP turns never participate in
# prompt/model A/B experiments the way plan run does.
# ─────────────────────────────────────────────────────────────────────────────
[[task]]
id = "E17-T02"
title = "Consult ExperimentStore for ACP prompt/model variant selection"
status = "ready"
tier = "integrative"
model_hint = "claude-sonnet-4-6"
max_loc = 80
files = ["crates/roko-acp/src/bridge_events.rs"]
role = "implementer"
depends_on = []
depends_on_plan = ["P19-cascade-router-acp", "E07-learning-knowledge"]

[task.context]
read_files = [
    { path = "crates/roko-acp/src/bridge_events.rs", lines = "620-720", why = "DaimonPolicy::default() hardcode (:634) [P19 replaces]; record_cascade_observation (:685-718) is observe-only — the telemetry site to pair with a selection call." },
    { path = "crates/roko-acp/src/bridge_events.rs", lines = "1590-1625", why = "resolved slug / cascade_router_model_slugs call site (:1606-1618) where a variant assignment belongs." },
    { path = "crates/roko-learn/src/experiments.rs", why = "ExperimentStore assign/record API — mirror how orchestrate.rs consults it per turn." },
]
symbols = [
    "ExperimentStore::assign(experiment_id, unit) -> Variant — the consult P19 does not add",
    "record_cascade_observation(...) — bridge_events.rs:685, the paired telemetry sink",
    "select_model — added by P19; ExperimentStore selection layers alongside it",
]
anti_patterns = [
    "Do NOT re-implement select_model / DaimonState loading — P19 owns those; depend on it.",
    "Do NOT assign a variant without recording the outcome — assignment and reward must both land or the experiment is biased.",
    "Do NOT hardcode an experiment id — read the active experiment set from ExperimentStore, no-op cleanly when none is active.",
]

[[task.verify]]
phase = "structural"
command = "grep -q 'ExperimentStore\\|experiment' crates/roko-acp/src/bridge_events.rs"
fail_msg = "the ACP path must consult ExperimentStore, not just observe"
[[task.verify]]
phase = "compile"
command = "cargo check -p roko-acp 2>&1"
fail_msg = "roko-acp must compile after wiring ExperimentStore"
[[task.verify]]
phase = "test"
command = "cargo test -p roko-acp -- experiment_assignment 2>&1"
fail_msg = "a test must assert an ACP turn assigns a variant when an experiment is active and records its outcome"

acceptance = "When an experiment is active in .roko/learn/experiments.json, an ACP turn is assigned a variant and its outcome recorded, so ACP turns contribute to A/B statistics identically to a plan run turn. With no active experiment the path is a clean no-op."


# ─────────────────────────────────────────────────────────────────────────────
# E17-T03: MCP session-tool parity for the Anthropic dispatch path (Fc)
#
# session/new accepts mcpServers and session.mcp_servers is populated (:1270),
# but only the openai-compat cognitive task receives &session_mcp_servers and
# calls setup_session_mcp_tools (:2313). The Anthropic branch invokes
# run_anthropic_cognitive_task (:1444-1457) with NO mcp_servers arg, so Anthropic
# ACP sessions silently get no session MCP tools. Thread the servers through.
# ─────────────────────────────────────────────────────────────────────────────
[[task]]
id = "E17-T03"
title = "Thread session_mcp_servers into run_anthropic_cognitive_task and register MCP tools"
status = "ready"
tier = "integrative"
model_hint = "claude-sonnet-4-6"
max_loc = 90
files = ["crates/roko-acp/src/bridge_events.rs"]
role = "implementer"
depends_on = []
depends_on_plan = ["P25-mcp-acp-passthrough"]

[task.context]
read_files = [
    { path = "crates/roko-acp/src/bridge_events.rs", lines = "1440-1476", why = "Provider dispatch branch: AnthropicApi -> run_anthropic_cognitive_task (no mcp arg); _ -> run_openai_compat_cognitive_task(&session_mcp_servers) which does get it." },
    { path = "crates/roko-acp/src/bridge_events.rs", lines = "2277-2320", why = "setup_session_mcp_tools(:2313) — the registration helper the Anthropic path must also call." },
    { path = "crates/roko-acp/src/bridge_events.rs", lines = "1260-1275", why = "session.mcp_servers clone (:1270) — the source of truth already available at the call site." },
]
symbols = [
    "run_anthropic_cognitive_task(session_id, messages, model_key, slug, config, workdir, effort, tools_enabled, cancel, sender) — add mcp_servers: &[McpServerConfig]",
    "setup_session_mcp_tools(session_id, mcp_servers, sender) — bridge_events.rs:2277",
    "run_openai_compat_cognitive_task — the reference impl that already threads mcp_servers",
]
anti_patterns = [
    "Do NOT duplicate MCP config discovery — P25 already plumbs config to session.mcp_servers; only pass the existing slice through.",
    "Do NOT register MCP tools when session_mcp_servers is empty — match the openai-compat guard so no-MCP sessions are unchanged.",
    "Do NOT block on unreachable MCP servers — reuse setup_session_mcp_tools' existing timeout/error handling.",
]

[[task.verify]]
phase = "structural"
command = "awk '/fn run_anthropic_cognitive_task/,/^}/' crates/roko-acp/src/bridge_events.rs | grep -q 'setup_session_mcp_tools\\|mcp_servers'"
fail_msg = "the Anthropic cognitive task must receive and register session MCP servers"
[[task.verify]]
phase = "compile"
command = "cargo check -p roko-acp 2>&1"
fail_msg = "roko-acp must compile after threading mcp_servers into the Anthropic path"
[[task.verify]]
phase = "test"
command = "cargo test -p roko-acp -- anthropic_session_mcp_tools 2>&1"
fail_msg = "a test must assert an Anthropic-provider ACP session registers tools from a configured mcpServers entry"

acceptance = "An ACP session created with an mcpServers entry and an Anthropic-provider model exposes those MCP tools to the model, matching the openai-compat path. Sessions with no mcpServers are unchanged on both provider paths."
```

---

## 5. Remaining task stubs (E17-T04 … E17-T06)

Same schema; key acceptance/verify notes:

- **E17-T04 (Fd — capabilities)** — replace the three hard-forced `tool_context.capabilities =
  ToolPermission { read/write/exec/git/network: true }` blocks (`bridge_events.rs:1829-1835,
  2360, 2514`) with values derived from the session's declared client capabilities + role.
  This **supersedes P22-T1's deliberate all-true** (its anti-pattern line 40). Verify: a session
  that declines `exec` cannot run `bash` even before the consent gate; `cargo test -p roko-acp
  capabilities_reflect_session`.
- **E17-T05 (Fd — advertisement guard)** — P28-T1 sets `image` from model vision support; this
  task adds the paired guard so advertised `PromptCapabilities` (`handler.rs:288-291`) can never
  claim `image:false` while the dispatch path builds image parts (`:4351/4378`). Verify: a test
  asserts `InitializeResult.prompt_capabilities.image == dispatch_accepts_image()`; extend to
  `audio` (currently always false + no audio path — assert consistency).
- **E17-T06 (Fa–Fd — conformance capstone)** — one end-to-end ACP session test asserting:
  (1) `session/request_permission` precedes any `write_file` and a `Reject` blocks it (T01);
  (2) an active experiment assigns+records a variant and a cascade `select_model` decision is in
  episode metadata (T02 + P19); (3) an Anthropic session with `mcpServers` lists the MCP tool
  (T03); (4) advertised capabilities match enforced ones (T04/T05). Depends on T01–T04.

---

## 6. Verification log (authored against HEAD `5852c93c05`)

| Finding | Re-verified | Result |
|---|---|---|
| Fa | `bridge_events.rs:768` (request_permission), `:1320` (detached spawn holds only event_sender), `:2926` (`execute` takes `_ctx`), `:5154/5181/5218` (only test callers); `builtin_tools.rs:173/269/326` | Confirmed: gate built + tested, zero prod callers; tool exec unconditional; handler cannot reach transport → reply channel required. |
| Fb | `bridge_events.rs:634` (`DaimonPolicy::default()`), `:685-718` (`record_cascade_observation` → `router.observe` at `:712`, no `select_model`) | Confirmed: write-only. `ExperimentStore` grep in `roko-acp` → absent. |
| Fc | `bridge_events.rs:1440-1476` (Anthropic branch omits mcp arg; openai branch passes `&session_mcp_servers`), `:2277-2313` (`setup_session_mcp_tools`), `:1270` | Confirmed: Anthropic path receives no session MCP servers. |
| Fd | `handler.rs:288-291` (`image:false, audio:false`); `bridge_events.rs:1829-1835,2360,2514` (all-true forced); image parts `:1223-1228,4351,4378` | Confirmed both halves: advertised `image:false` while image parts built; tool caps forced all-true. |
| Reconcile | `plans/P19|P21|P22|P25|P28/tasks.toml` read | Confirmed: P19=select+daimon, P21=streaming, P22=static filter+testing→real ctx (keeps all-true), P25=MCP config plumbing, P28=image capability from vision. |

> **Fc nuance (verify at pickup)**: the source brief lists both Claude-CLI and Anthropic as
> missing MCP tools. Verified here: the **Anthropic** path definitively omits the arg. Claude-CLI
> nominally routes through the openai-compat cognitive task (`:1459-1460`) which *does* thread
> `mcp_servers`; confirm at pickup whether the Claude-CLI *dispatch* (delegated `claude` binary)
> actually consumes them, and fold into E17-T03 / **E15** if not.

## CTRL-08 ownership reconciliation

E17-T07/T08 remain ACP-specific adapters only. They consume SH05/E48 budget state
and E48 provider-health state after those plans; they must not define another
budget guardrail, limiter, health registry, or router. The plan runs at
`max_parallel = 1` because its adapters share ACP bridge/handler/type hot surfaces.
See
[`17-OPERATIONAL-OWNERSHIP.md`](../17-OPERATIONAL-OWNERSHIP.md).
