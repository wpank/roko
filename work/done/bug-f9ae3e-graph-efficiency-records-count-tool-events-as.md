+++
id = "bug-f9ae3e"
kind = "bug"
title = "Graph efficiency records count tool events as tool calls and mark every call succeeded"
status = "done"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-cli/graph-dispatch", "roko-learn/efficiency"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/feedback.rs::GraphTaskDispatcher::emit_feedback", "crates/roko-cli/src/dispatch_v2.rs::agent_event_from_chunk", "crates/roko-learn/src/efficiency.rs::ToolCallMeta"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! sed -n '/let eff_tool_calls/,/let eff_tools_used/p' crates/roko-cli/src/graph_task_dispatch/feedback.rs | grep -q 'succeeded: true' && grep -rqw 'fn efficiency_counts_distinct_tool_call_ids' crates/roko-cli/src && cargo test -p roko-cli --lib efficiency_counts_distinct_tool_call_ids"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:30Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:11Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

The Graph writer builds a `ToolCallMeta` for every `AgentRuntimeEvent::ToolCall` event (`graph_task_dispatch.rs:1025-1034`) with `succeeded: true` and no size or status, and sets `tools_used` to the event count (`:1043`).
Start, end and argument-delta events each produce a record, so `.roko/learn/efficiency.jsonl` over-counts tool calls and reports every call as successful with 0 bytes.
Fix: count distinct non-empty tool-call ids, take success and size from the tool audit when enabled, and otherwise mark the outcome as unobserved.

2026-09-29: re-verified at d9e79e9d8. Still open; the record is built at graph_task_dispatch.rs:1572-1611 (was ~1025). The over-count comes from dispatch_v2.rs agent_event_from_chunk, which emits a ToolCall for every ToolCallDelta. The Claude CLI adapter emits one ToolCall per tool_use block. tools_available is also set to the call count.

## Notes

- 2026-10-01 (wk-settle): implemented on work/bug-f9ae3e; cargo verification deferred to the batch check.
  The Graph efficiency row now records one tool call per distinct call id (`efficiency_tool_calls`,
  `graph_task_dispatch/feedback.rs`): a streamed call's start, argument deltas and end collapse into one, an id-less
  call counts once per named event, and its tool output gives `result_tokens`. `ToolCallMeta::succeeded` is now
  `Option<bool>`, and Graph rows write `None`, since nothing on this path observes a call's outcome.
  The verify command's `sed` now reads `graph_task_dispatch/feedback.rs`, where the block moved.
- Left for later: success from the tool audit. Reading `.roko/tool_audit.jsonl` back per attempt grows with the
  file, and the stream events carry no error flag (`ToolOutput` has only the text). `tools_available` is gap-7a8474's.
