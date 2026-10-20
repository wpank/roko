+++
id = "gap-cf427e"
kind = "gap"
title = "A chat host's run is accepted, then fails in milliseconds, when no model its routing can pick runs in roko's own tool loop; refuse it up front so the host's probe sees it"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-serve/mcp"]
created = 2026-10-13
updated = 2026-10-15
last_verified = 2026-10-15
last_verified_rev = "6e0b6676f"
source = "hermes-plugin-roko docs walkthrough 2026-10-13 (roko 5de43f9ab)"
anchors = ["crates/roko-serve/src/routes/mcp.rs::refuse_unscreened_run", "crates/roko-cli/src/graph_task_dispatch/failover.rs::failover_candidate"]
links = { depends_on = [], blocks = [], related = ["bug-103bce"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn chat_run_refused_without_a_tool_loop_model' crates/ && cargo test -p roko-serve chat_run_refused_without_a_tool_loop_model"

[closed]
at = 2026-10-15
at_ts = "2026-10-15T16:12:02Z"
commit = "6e0b6676f"
by = "roko-chathost"
executor = "claude-agent"
size = "M"
claimed_at = "2026-10-15T13:47:03Z"
forced = false
evidence = "6e0b6676f: a chat host's run keeps only the ladder rungs roko's own tool loop runs (plan_runner keep_tool_loop_rungs, RoutingLadder::in_own_tool_loop) and failover checks each candidate against the task's contract; serve's /mcp refuses such a run up front, before the plan lookup, with structuredContent.error.data.refusal = no_data_boundary | no_tool_loop_model and a reason naming each rung's cause (RokoConfig::chat_run_refusal, tool_loop_models); /health lists features. Tests: routes::mcp::tests::chat_run_refused_without_a_tool_loop_model (the verify), graph_task_dispatch::failover::tests::chat_run_ladder_skips_rungs_whose_agent_runs_its_own_tools, config::serve::chat_run_tests (6), dispatch_v2::tests::config_tool_loop_classification_matches_dispatch; lib suites roko-core 2018, roko-learn 1351, roko-serve 1018, roko-cli 3630 passed (share_run_scrubs_output_text_and_prompt skipped); +nightly fmt and clippy -D warnings clean on the four crates."
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
