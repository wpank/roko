+++
id = "gap-48faa7"
kind = "gap"
title = "ToolSelector is never attached to a dispatcher"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-agent/dispatcher"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e3-discovery"
anchors = ["crates/roko-agent/src/dispatcher/mod.rs::with_tool_selector", "crates/roko-agent/src/dispatcher/tool_selector.rs::ToolSelector"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rn --include='*.rs' -E 'with_tool_selector\\(|ToolSelector::for_role' crates/roko-cli/src crates/roko-execution/src crates/roko-serve/src | grep -q . || ! grep -rq 'pub struct ToolSelector' crates/roko-agent/src"
+++

`with_tool_selector` (dispatcher/mod.rs:435) has no caller, and `ToolSelector::for_role` is used only in its own tests, so the per-role tool selection it implements never runs; tool exposure comes from contracts and task allowlists. The parked umbrella bug-e1a7d8 separately notes that it uses legacy tool names.

Fix: wire it where dispatchers are built (`dispatch/factory.rs`) or delete it.

## Notes

- 2026-10-01 (wk-guard2): implemented on work/bug-a70def; cargo verification deferred to the batch check.
- Deleted rather than wired: attaching it in `dispatch/factory.rs` would have denied every tool outside each role's small menu (MCP and plugin tools, `run_tests`, `apply_patch`, `todo_write`), while the contracts already govern tool use. Removed `dispatcher/tool_selector.rs` (with `canonicalize_tool_name`, which had no other caller, and `menus_stay_within_contracts`), the dispatcher's `tool_selector` field, `with_tool_selector`, `tool_selector()`, the dispatch-time selector check and `EffectiveCatalogSnapshot::selector_active` (no reader outside the dispatcher). Docs still describe it: `docs/v3/05-AGENT.md`, `docs/v3/depth/05-agent/{17-dispatcher-architecture,dispatcher-architecture,tool-loop-protocol}.md` and `docs/v2/ARCHITECTURE-GUIDE.md`.
