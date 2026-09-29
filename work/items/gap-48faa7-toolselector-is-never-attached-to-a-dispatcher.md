+++
id = "gap-48faa7"
kind = "gap"
title = "ToolSelector is never attached to a dispatcher"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-agent/dispatcher"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e3-discovery"
anchors = ["crates/roko-agent/src/dispatcher/mod.rs::with_tool_selector", "crates/roko-agent/src/dispatcher/tool_selector.rs::ToolSelector"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -rn "with_tool_selector(" crates --include="*.rs" | grep -v "pub fn with_tool_selector" | grep -q .'
+++

`with_tool_selector` (dispatcher/mod.rs:435) has no caller, and `ToolSelector::for_role` is used only in its own tests, so the per-role tool selection it implements never runs; tool exposure comes from contracts and task allowlists. The parked umbrella bug-e1a7d8 separately notes that it uses legacy tool names.

Fix: wire it where dispatchers are built (`dispatch/factory.rs`) or delete it.
