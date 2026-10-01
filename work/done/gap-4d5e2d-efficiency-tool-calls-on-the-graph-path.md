+++
id = "gap-4d5e2d"
kind = "gap"
title = "Efficiency tool calls on the Graph path record succeeded = null"
status = "done"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-f9ae3e"
anchors = ["crates/roko-cli/src/graph_task_dispatch/feedback.rs", "crates/roko-cli/src/dispatch_v2.rs::ToolCallRecord"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-f9ae3e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib efficiency_tool_calls_record_outcome"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:30Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T17:36:57Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

bug-f9ae3e made `ToolCallMeta.succeeded` an `Option<bool>`, so the Graph path records `null` instead of a fabricated `true`. That is honest, but efficiency events now say nothing about tool outcomes. Filling the field in needs the tool-audit records or the provider's tool results.

## Plan

Join the attempt's tool-audit or provider tool results to its tool calls, and set `succeeded` where the outcome is known. Add a test named `efficiency_tool_calls_record_outcome_*`.

## Done when

- `cargo test -p roko-cli --lib efficiency_tool_calls_record_outcome` passes.

## Notes

- Reported on 2026-10-01 by wk-settle, working on bug-f9ae3e.
- 2026-10-01 (wk-settle): implemented on work/bug-f9ae3e; cargo verification deferred to the batch check. A
  dispatch marks the tool audit's length before its agent runs and reads back the lines its attempt key wrote
  after (`dispatch_v2::ToolAuditMark`, `AgentResultDispatch::tool_calls`). `efficiency_tool_calls` sets each
  audited call's outcome and adds the calls the events missed. Tests:
  `efficiency_tool_calls_record_outcome_from_the_tool_audit`, `efficiency_tool_calls_record_outcome_of_an_audited_graph_run`.
- Left, reported to the coordinator: the Graph result bridge projects no tool events into `dispatch.events`, so
  Claude CLI attempts still list no tool calls. Recording them needs the live-output tap's `ToolCallEnd` and
  `ToolResult`, and their outcomes need the `is_error` that `claude_cli_agent.rs` drops from `tool_result` blocks.
