+++
id = "bug-a70def"
kind = "bug"
title = "The Claude MCP isolation tests assume the host has no managed-mcp.json"
status = "open"
triage = "unverified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-agent/claude_cli"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-cc-isolate's report on gap-b7a2d5, branch work/gap-b7a2d5 at 42859fc78)"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = ["gap-b7a2d5"], blocks = [], related = ["gap-b7a2d5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn mcp_isolation_tests_ignore_the_hosts_managed_mcp_json' crates/roko-agent/src/ && cargo test -p roko-agent --lib mcp_isolation_tests_ignore_the_hosts_managed_mcp_json"
+++

## Problem

Claude Code reads a system-wide `managed-mcp.json` that can't be moved ("Claude Code 2.1.282 has no way to move it", `claude_cli_agent.rs:137` on gap-b7a2d5's branch). The isolation code checks for it (:150). Some MCP tests build their own managed directory (:2502, :2549), but wk-cc-isolate reports that the MCP tests as a whole assume the host has no `managed-mcp.json`. On a machine with one, they fail or test the wrong thing.

## Why it matters

Hygiene (epic spec-9a3131): the tests depend on the machine. p3.

## Where

The MCP isolation tests in `claude_cli_agent.rs`, and the function that locates the managed file.

## Plan

1. Make the managed-file directory injectable (a parameter, or an env override used only in tests), and point every test at a temporary directory.
2. Add `mcp_isolation_tests_ignore_the_hosts_managed_mcp_json`, which passes with and without a host file.

## Done when

- [ ] The MCP tests pass whether or not the host has a `managed-mcp.json`.
- [ ] The `[[verify]]` command passes.
