+++
id = "gap-6af7e2"
kind = "gap"
title = "ACP tool policy leftovers: remote MCP tool names and the session safety layer's mode"
status = "open"
triage = "unverified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-acp"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-55eada"
anchors = ["crates/roko-acp/src/bridge_events/mod.rs", "crates/roko-acp/src/bridge_events/dispatch.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-55eada"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-acp --lib remote_mcp_tools_respect_forbidden_tools"
+++

## Problem

gap-55eada applied the AgentContract tool policy to ACP tool dispatch. Two parts are left: Plan step 5(b), checking remote MCP tool names against ForbiddenTools; and the session pre/post-dispatch SafetyLayer in `bridge_events/mod.rs`, which still takes the raw mode, so "code" gets the restricted contract there.

## Plan

Check remote MCP tool names against ForbiddenTools, and map the mode to its contract role in the session layer. Add a test named `remote_mcp_tools_respect_forbidden_tools`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-01 by wk-specq, working on gap-55eada, during the evening close-out round.
