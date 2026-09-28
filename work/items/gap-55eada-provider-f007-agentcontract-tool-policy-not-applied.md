+++
id = "gap-55eada"
kind = "gap"
title = "AgentContract tool policy not applied to ACP tool dispatch"
status = "open"
triage = "verified"
severity = "p1"
goal = "hermes"
subsystem = ["roko-acp/bridge_events"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F007"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F007"
anchors = ["crates/roko-acp/src/bridge_events/dispatch.rs::run_openai_compat_mcp_tool_loop", "crates/roko-acp/src/bridge_events/tools.rs:455"]
links = { depends_on = [], blocks = [], related = ["gap-da70fd"], supersedes = [], duplicate_of = "" }
+++
The `AgentContract` role/task allowlists are enforced in the runner-v2 tool dispatcher but are not threaded through the ACP tool dispatch path. ACP-dispatched agents can call any tool regardless of their role's permitted tool set.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F007`
- `tmp/archive/provider-audit/04-acp-integration.md`

Warning: every file this item cites is gone (`crates/roko-acp/src/bridge_events.rs`) — likely obsolete or moved.

How to verify: CLAUDE.md claims AgentContract tool policy wired generally; check ACP dispatch path specifically. Confirm in crates/roko-acp/src/bridge_events.rs whether still true: `AgentContract` tool policy not applied to ACP tool dispatch

Verified 2026-09-28: narrowed but still true. `AgentContract` is now enforced for ACP builtin tools: `AcpBuiltinToolHandler::execute` (crates/roko-acp/src/bridge_events/tools.rs:455-466) loads the role contract with `RestrictedFallback` and denies anything it does not permit. That covers the Anthropic and builtin OpenAI-compat tool loops. The OpenAI-compat MCP tool loop (bridge_events/dispatch.rs:777-968) builds `ToolDispatcher::new` (:864) with only an `allowed_tools` filter (:886) and the default `SafetyLayer::with_defaults()`, with no role contract, so MCP tools bypass the role's `AgentContract`.
