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

## Notes

- Implemented on `work/gap-a791b4` at `e7a0a1752`; cargo verification deferred to the batch check.
- 2026-09-29: `ProviderAdapter::turn_cap_enforcement` now says how each adapter bounds a run. Native: Claude CLI
  and Hermes one-shot CLI (`--max-turns`). Tool loop: OpenAI-compatible, Anthropic API, Gemini API, Cerebras and
  Perplexity, where the cap replaces the default 50 iterations and a configured `max_tool_iterations` stays a
  ceiling. Tool calls: Cursor CLI, which stops once the run starts more ACP tool calls than the cap. Single turn:
  OpenClaw `infer`. Advisory, with a warning from `create_agent_for_model`: Codex CLI, Cursor ACP, Gemini CLI,
  Hermes HTTP and ACP, and OpenClaw ACP.
