+++
id = "gap-cf427e"
kind = "gap"
title = "A chat host's run is accepted, then fails in milliseconds, when no model its routing can pick runs in roko's own tool loop; refuse it up front so the host's probe sees it"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-serve/mcp"]
created = 2026-10-13
updated = 2026-10-13
last_verified = 2026-10-13
source = "hermes-plugin-roko docs walkthrough 2026-10-13 (roko 5de43f9ab)"
anchors = ["crates/roko-serve/src/routes/mcp.rs::refuse_unscreened_run", "crates/roko-cli/src/graph_task_dispatch/failover.rs::failover_candidate"]
links = { depends_on = [], blocks = [], related = ["bug-103bce"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn chat_run_refused_without_a_tool_loop_model' crates/ && cargo test -p roko-serve chat_run_refused_without_a_tool_loop_model"
+++

## Problem

roko serve accepts a run a chat host starts (MCP `run_prompt`, `plan_run`) even when no model its routing can
pick is reachable through roko's own tool loop. Such a run carries the `stage` outbound policy, which a CLI agent
(Claude Code, Codex, Gemini CLI, Cursor) cannot honour, so the run fails about 4 ms after dispatch with "no usable
provider for this task". Seen 2026-10-13 in a fresh `roko init` workspace (roko `5de43f9ab`): the ladder's lower
rungs were not runnable, every task fell to the `top` rung, claude-sonnet-4-6 on `claude_cli`, and
`hermes roko doctor` still reported "✓ runs: roko accepts runs started from Hermes".

## Why it matters

The first run anyone starts from Hermes in a new workspace fails, after the person approved the spend, and the
host's readiness probe says all is well. bug-103bce now shows the reason after the fact.

## Where

- `crates/roko-serve/src/routes/mcp.rs::refuse_unscreened_run` and `RokoConfig::chat_run_refusal`: the refusal a
  host's probe sees, which checks only the data-model boundary.
- `crates/roko-cli/src/graph_task_dispatch/failover.rs::failover_candidate`: what makes a provider usable.

## Current state

Open at `87e341409`. The docs (hermes-plugin-roko docs/getting-started.md, step 1) tell people to give roko an
API model.

## Plan

1. Refuse a chat host's run up front, with the reason, when no model the workspace can route a task to runs in
   roko's own tool loop with its credentials set; the probe the plugin's doctor uses then reports it.
2. Test: a workspace whose only provider is a CLI agent refuses `run_prompt` with that reason.

## Done when

`chat_run_refused_without_a_tool_loop_model` passes (the `[[verify]]` command).
