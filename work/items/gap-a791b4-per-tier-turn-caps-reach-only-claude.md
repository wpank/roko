+++
id = "gap-a791b4"
kind = "gap"
title = "Per-tier turn caps reach only Claude CLI; other providers ignore AgentOptions.max_turns"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-agent/providers"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e4-perf"
anchors = ["crates/roko-agent/src/provider/mod.rs::AgentOptions", "crates/roko-cli/src/graph_task_dispatch/turn_policy.rs::task_turn_limit", "crates/roko-agent/src/codex_agent.rs"]
links = { depends_on = [], blocks = [], related = ["gap-3870d9"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q max_turns crates/roko-agent/src/codex_agent.rs && grep -q max_turns crates/roko-agent/src/cursor_cli_agent.rs && grep -rq max_turns crates/roko-agent/src/tool_loop/"
+++

gap-3870d9 was closed by giving every Graph task its tier's `[pipeline.<tier>] max_turns` through `task_turn_limit`. The limit travels in `AgentOptions.max_turns`, which only the Claude CLI adapter reads (`provider/claude_cli.rs`, `claude_cli_agent.rs`). Codex, Cursor, Gemini CLI and Hermes ignore it, and the HTTP tool loops use the per-model `max_tool_iterations` (default 50), so tasks on those providers still run without the tier's turn cap.

Fix: map the cap onto each adapter's native limit where one exists, or enforce it in the shared tool loop, and document providers that cannot honour it.
