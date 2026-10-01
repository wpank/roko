+++
id = "gap-55eada"
kind = "gap"
title = "AgentContract tool policy not applied to ACP tool dispatch"
status = "open"
triage = "verified"
severity = "p1"
size = "M"
goal = "hermes"
subsystem = ["roko-acp/bridge_events"]
created = 2026-09-01
updated = 2026-09-29
last_verified = 2026-10-01
last_verified_rev = "a17d9d766"
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F007"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F007"
anchors = ["crates/roko-acp/src/bridge_events/dispatch.rs::run_openai_compat_mcp_tool_loop", "crates/roko-acp/src/bridge_events/dispatch.rs::run_anthropic_tool_loop", "crates/roko-acp/src/bridge_events/tools.rs::AcpMcpToolHandler", "crates/roko-acp/src/bridge_events/tools.rs::AcpBuiltinToolHandler", "crates/roko-acp/src/bridge_events/cost.rs::acp_role_for_mode", "crates/roko-agent/src/safety/mod.rs::with_role"]
links = { depends_on = ["bug-bfb8ce"], blocks = [], related = ["gap-da70fd"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn mcp_tool_loop_denies_tool_outside_role_contract' crates/roko-acp/src/ && grep -rqw 'fn mcp_tool_loop_allows_tool_permitted_by_role_contract' crates/roko-acp/src/ && cargo test -p roko-acp mcp_tool_loop_"
+++

## Problem

In `roko acp`, tools supplied by session MCP servers never pass through the role's `AgentContract`.
ACP builtin tools (`read_file`, `write_file`, `bash`, ...) are checked against the contract inside
`AcpBuiltinToolHandler::execute`. MCP tools are not: their handler (`AcpMcpToolHandler`) only checks the
plugin tier, and the `ToolDispatcher` that runs them carries the generic default `SafetyLayer`, not a
role-scoped one. So the role's `allowed_tools`, `ForbiddenTools`, invariants, governance limits and
`max_taint_level` do not apply to MCP tool calls.

Expected: every tool an ACP session can call, builtin or MCP, is checked against the same role contract.
A call the contract does not permit returns `ToolError::PermissionDenied` before the MCP server is contacted.

## Why it matters

- Goal `hermes` (Hermes and ACP integration). An editor or Hermes client that attaches MCP servers to a
  session gets a tool surface that ignores role policy. That is a gap in the safety story roko tells:
  unknown or unsupported tool use should fail closed.
- Follow-up to `gap-da70fd` (done: every ACP path builds a `SafetyLayer`); its closure evidence names this
  item as the residual.
- Tied to `bug-bfb8ce`: the ACP session "role" is the mode string (`code`, `plan`, `research`), and no
  bundled contract exists for those names. Applying contracts to MCP tools without a mode-to-role mapping
  would deny every MCP tool in the default `code` mode.

## Where

Entry point: `roko acp` -> session prompt handler in `crates/roko-acp/src/bridge_events/mod.rs`, which takes
`session_agent_role = session.config_state.agent_mode` (`:607`, default `"code"`, `session.rs:179`) and calls
`run_anthropic_cognitive_task` (`:799`) or `run_openai_compat_cognitive_task` (`:821`).

- `crates/roko-acp/src/bridge_events/dispatch.rs::run_openai_compat_mcp_tool_loop` (~`:777-958`): MCP loop for
  OpenAI-compatible, Perplexity and Cerebras providers. Builds `ToolDispatcher::new(registry, resolver)` (`:864`)
  and sets only `tool_context.allowed_tools` (`:886`). It has no `role` parameter.
- `crates/roko-acp/src/bridge_events/dispatch.rs::run_openai_compat_cognitive_task` (~`:633`): has `role`, but
  calls the MCP loop at `:670` without it and passes `allowed_tools = None`.
- `crates/roko-acp/src/bridge_events/dispatch.rs::run_anthropic_tool_loop` (~`:153`): merges session MCP tools
  and handlers into the builtin handler map (`:235-249`) and builds `ToolDispatcher::new` at `:257`. The MCP
  handlers in this loop skip the contract too.
- `crates/roko-acp/src/bridge_events/tools.rs::AcpMcpToolHandler::execute` (~`:327`): only
  `check_plugin_tier(self.plugin_tier, &capability_for_tool(&self.remote_name))`, then calls the MCP server.
- `crates/roko-acp/src/bridge_events/tools.rs::AcpBuiltinToolHandler::execute` (`:421-478`): the reference
  behaviour. It loads `AgentContract::load_for_role_with_mode(role, ContractLoadMode::RestrictedFallback)` and
  denies when `!contract.permits_tool(&self.tool_name)`.
- `crates/roko-acp/src/bridge_events/tools.rs::setup_session_mcp_tools` (`:94`): exposes each MCP tool as
  `<server>_<tool>` (sanitized, `:226-231`) and builds handlers with `PluginTier::Sandboxed`.
- `crates/roko-acp/src/bridge_events/cost.rs::acp_role_for_mode` (`:415`): `plan` -> `Strategist`,
  `research` -> `Researcher`, anything else -> `Implementer`. `AgentRole`'s `Display` gives the contract name
  (`"implementer"` etc., `crates/roko-core/src/agent.rs:1143`, `label()`).
- `crates/roko-agent/src/safety/mod.rs`: `SafetyLayer::with_defaults` (`:413`), `from_config` (`:496`),
  `with_role` (`:511`), `with_contract` (`:527`); `check_pre_execution` (`:616`) runs the contract at `:713`.
  `crates/roko-agent/src/safety/contract.rs`: `permits_tool` (`:338`, exact names), `check_pre_execution` (`:441`).
- `crates/roko-agent/src/dispatcher/mod.rs`: `ToolDispatcher::new` (`:347`, installs `SafetyLayer::with_defaults()`),
  `with_safety` (`:402`, also rebuilds `ProductionSafetyChain`), per-call safety check at `:775`.

## Current state

- Builtin tools: the contract gate in `AcpBuiltinToolHandler::execute` landed in `244f564e1` (2026-09-21).
  That covers builtin tools in both the Anthropic loop and `run_openai_compat_builtin_tool_loop`.
- MCP tools (both loops) are checked only by:
  1. the dispatcher's capability ceiling (`dispatcher/mod.rs:716-744`), using `ctx.capabilities` from
     `derive_acp_tool_capabilities(mode, ...)`, which already derives permission bits from
     `acp_role_for_mode(mode)`;
  2. the plugin-tier name heuristic: session MCP servers run at `PluginTier::Sandboxed`, which refuses any tool
     whose remote name looks like a write, exec or network call (`capability_for_tool`,
     `crates/roko-agent/src/mcp/handler.rs:127`; `check_plugin_tier`, `crates/roko-agent/src/safety/capabilities.rs:16`);
  3. the default layer from `ToolDispatcher::new`, whose contract is `AgentContract::hardened_default("default")`,
     not the session role's contract.
- Role contracts (`crates/roko-agent/src/safety/contracts/*.yaml`) set no `allowed_tools`; they use
  `ForbiddenTools` with builtin names (for example architect forbids `edit_file`, `write_file`, `bash`;
  implementer forbids `web_fetch`, `web_search`). MCP exposed names are `<server>_<tool>`, so name matching
  alone will rarely deny an MCP tool for a known role. Invariants, governance and `max_taint_level` still
  matter: `check_pre_execution_with_def` checks the taint level for MCP tools with write/exec/git/network
  bits, and this loop runs at `CamelTaintLevel::External` (`dispatch.rs:885`).
- Unconfirmed (static reading only, not run): `SafetyLayer::with_defaults()` sets
  `tool_permission_policy = AllowExplicit` with an empty `tool_permission_list` (`safety/mod.rs:436-440`), and
  step 0 of `check_pre_execution` (`:631-640`) denies any tool not in that list. The dispatcher calls it for
  every call (`dispatcher/mod.rs:775`). If that holds, every ACP tool loop built with `ToolDispatcher::new`
  currently denies all tools, MCP and builtin, with "not permitted under AllowExplicit policy". The existing
  ACP tests only assert denials (`bridge_events/tests.rs:275-313`), so they would not notice. Installing a
  role-scoped layer (step 2 below) lifts that gate, because `with_role`/`with_contract` set the list to `["*"]`.

## Plan

1. Confirm the baseline. Add a positive test: dispatch one allowed builtin call (for example `ls`) through
   `ToolDispatcher::new` built the way `run_anthropic_tool_loop` builds it. If it is denied with
   "AllowExplicit", steps 2-4 fix that too; say so in the commit message.
2. Map mode to contract role once. Add `acp_contract_role_for_mode(mode: &str) -> String` next to
   `acp_role_for_mode` in `cost.rs` (return `acp_role_for_mode(mode).to_string()`). Use it for the builtin
   handler's `role` and for the MCP loops. This is the fix `bug-bfb8ce` asks for; do both in one change, or
   land `bug-bfb8ce` first.
3. Build one role-scoped `SafetyLayer` per dispatch:
   `SafetyLayer::from_config(&roko_config).with_role(acp_contract_role_for_mode(mode))`. `with_role` loads the
   bundled contract with `RestrictedFallback`, so an unknown role still fails closed.
4. Thread it into the loops. Add a `role: &str` parameter (or a prebuilt `SafetyLayer`) to
   `run_openai_compat_mcp_tool_loop` and pass `role` from `run_openai_compat_cognitive_task` (`:670`). In all
   three ACP loops (`run_anthropic_tool_loop` `:257`, the MCP loop `:864`, `run_openai_compat_builtin_tool_loop`
   `:1034`), replace `ToolDispatcher::new(registry, resolver)` with `.with_safety(layer)` added. Put this in one
   helper (for example `acp_tool_dispatcher(registry, resolver, layer)`) so tests use the production code.
5. Decide how contracts name MCP tools. Options:
   - (a) exposed name only (`<server>_<tool>`), which is what `permits_tool` sees today. Simple, but contract
     authors must predict the sanitized name.
   - (b) also deny when the remote tool name (`AcpMcpToolHandler::remote_name`) is forbidden, so an MCP server
     that exposes `bash` or `write_file` is caught by the builtin names in `ForbiddenTools`.
   Recommended: (a) now via the dispatcher layer, plus (b) as a small check in `AcpMcpToolHandler::execute`
   (give it the loaded `AgentContract`, or a `role` field like `AcpBuiltinToolHandler`).
6. Tests in `crates/roko-acp/src/bridge_events/tests.rs`, using the `sh -c` stdio MCP fixture pattern already
   there (`:440-470`, `:3259`):
   - `mcp_tool_loop_denies_tool_outside_role_contract`: role with no contract (restricted fallback) or a
     contract that forbids the fixture tool -> `ToolError::PermissionDenied`, and the fixture server never
     receives `tools/call`.
   - `mcp_tool_loop_allows_tool_permitted_by_role_contract`: mode `code` (maps to implementer) -> the fixture
     tool runs and returns its text.

## Done when

- An MCP tool call in an ACP session is denied with `PermissionDenied` when the session's role contract does not
  permit it, in both the Anthropic loop and the OpenAI-compatible MCP loop.
- An MCP tool that the role permits still runs in the default `code` mode.
- Builtin tools behave as before for known roles.
- Verify:
  `grep -rqw 'fn mcp_tool_loop_denies_tool_outside_role_contract' crates/roko-acp/src/ && grep -rqw 'fn mcp_tool_loop_allows_tool_permitted_by_role_contract' crates/roko-acp/src/ && cargo test -p roko-acp mcp_tool_loop_`

## Notes

- Safety-sensitive: changes what tools ACP sessions may run. Keep the fail-closed default for unknown roles
  (`ContractLoadMode::RestrictedFallback`); do not switch to a permissive contract to make tests pass.
- Do not change the bundled contract YAMLs or `AgentContract::permits_tool` semantics here; other dispatch paths
  use them.
- Overlaps `bug-bfb8ce` (same files: `bridge_events/tools.rs`, `cost.rs`, `mod.rs`). Do them together or in
  sequence, not in parallel.
- `ToolDispatcher::with_safety` rebuilds the `ProductionSafetyChain` from the new layer; keep that call rather
  than assigning the field.
- The old verify command grepped a fixed line window (`777-968`) in `dispatch.rs` and ran an unguarded test
  filter (cargo passes when no test matches); the command above replaces it.
- 2026-10-01 (wk-specq): implemented on work/bug-8dbffd; cargo verification deferred to the batch check.
  The baseline claim holds statically: `ToolDispatcher::new` keeps `SafetyLayer::with_defaults()`, whose
  `AllowExplicit` list is empty, so every ACP tool loop call was denied. All three loops now build their dispatcher
  with `dispatch.rs::acp_tool_dispatcher` and a role-scoped layer from `acp_tool_safety` (`from_config` +
  `with_role(acp_contract_role_for_mode(mode))`, from bug-bfb8ce). Tests: the two `mcp_tool_loop_` tests (sh MCP
  fixture; the denied call never reaches the server) and `acp_tool_dispatcher_runs_builtin_tool_in_code_mode`.
  Not done: plan step 5(b), checking the remote MCP tool name against `ForbiddenTools`; the Sandboxed plugin tier
  already refuses most such names. The session-level pre/post-dispatch `SafetyLayer` in `mod.rs` still takes the
  raw mode.

## Original notes

The `AgentContract` role/task allowlists are enforced in the runner-v2 tool dispatcher but are not threaded through the ACP tool dispatch path. ACP-dispatched agents can call any tool regardless of their role's permitted tool set.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F007`
- `tmp/archive/provider-audit/04-acp-integration.md`

Warning: every file this item cites is gone (`crates/roko-acp/src/bridge_events.rs`) — likely obsolete or moved.

How to verify: CLAUDE.md claims AgentContract tool policy wired generally; check ACP dispatch path specifically. Confirm in crates/roko-acp/src/bridge_events.rs whether still true: `AgentContract` tool policy not applied to ACP tool dispatch

Verified 2026-09-28: narrowed but still true. `AgentContract` is now enforced for ACP builtin tools: `AcpBuiltinToolHandler::execute` (crates/roko-acp/src/bridge_events/tools.rs:455-466) loads the role contract with `RestrictedFallback` and denies anything it does not permit. That covers the Anthropic and builtin OpenAI-compat tool loops. The OpenAI-compat MCP tool loop (bridge_events/dispatch.rs:777-968) builds `ToolDispatcher::new` (:864) with only an `allowed_tools` filter (:886) and the default `SafetyLayer::with_defaults()`, with no role contract, so MCP tools bypass the role's `AgentContract`.
