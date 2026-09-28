+++
id = "gap-da70fd"
kind = "gap"
title = "[provider F006] SafetyLayer not always instantiated for all ACP code paths"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["roko-acp/bridge_events"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F006"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F006"
anchors = ["crates/roko-acp/src/bridge_events/mod.rs:624", "crates/roko-agent/src/dispatcher/mod.rs:347", "crates/roko-acp/src/runner.rs:697"]
links = { depends_on = [], blocks = [], related = ["gap-55eada"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'pre_dispatch_check_with_context' crates/roko-acp/src/bridge_events/mod.rs"

[[verify]]
command = "cargo test -p roko-acp"

[closed]
at = 2026-09-28
commit = "91b4745f8"
evidence = "Fixed at HEAD 91b4745f8: every ACP dispatch path builds a SafetyLayer. Session prompts run pre_dispatch_check_with_context before dispatch (crates/roko-acp/src/bridge_events/mod.rs:624, fail-closed SP-1) and post_dispatch_check afterwards (:912). The only dispatch entry points, run_anthropic_cognitive_task and run_openai_compat_cognitive_task, are called only from that handler (mod.rs:799, :821). Every ACP tool loop uses ToolDispatcher::new, which always installs SafetyLayer::with_defaults() plus ProductionSafetyChain (crates/roko-agent/src/dispatcher/mod.rs:347-357). The pipeline runner builds per-mode and per-role layers (runner.rs:696-718). Residual: role-contract gating of MCP tools is tracked in gap-55eada."
+++
The ACP pipeline has multiple entry points; some bypass `SafetyLayer` construction. This creates fail-open windows where ACP-dispatched tool calls may proceed without immune screening, taint enforcement, or corrigibility checks.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F006`
- `tmp/archive/provider-audit/04-acp-integration.md`

Warning: every file this item cites is gone (`crates/roko-acp/src/bridge_events.rs`) — likely obsolete or moved.

How to verify: Confirm in crates/roko-acp/src/bridge_events.rs whether still true: `SafetyLayer` not always instantiated for all ACP code paths

Verified 2026-09-28: fixed; see `[closed].evidence`.
