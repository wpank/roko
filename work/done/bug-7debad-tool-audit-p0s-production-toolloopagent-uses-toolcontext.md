+++
id = "bug-7debad"
kind = "bug"
title = "Tool audit P0s: production ToolLoopAgent uses ToolContext::testing; cancellation/timeout/audit gaps"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-agent/tool-loop"]
created = 2026-09-04
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#3. Tool Audit (`tmp/tool-audit/`, 13 files, 36 findings)"
discovered_from = "audit:tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#3. Tool Audit (`tmp/tool-audit/`, 13 files, 36 findings)"
anchors = ["crates/roko-agent/src/tool_loop/mod.rs:1283", "crates/roko-agent/src/dispatcher/mod.rs::dispatch_batch", "crates/roko-agent/src/tool_loop/context_factory.rs::ToolExecutionContextFactory", "crates/roko-cli/src/dispatch/factory.rs:278"]
links = { depends_on = [], blocks = [], related = ["find-f489db"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'fn turn_trace_pairs_results_with_their_calls' crates/roko-agent/src/tool_loop/mod.rs && cargo test -p roko-agent --lib tool_loop::tests::turn_trace_pairs_results_with_their_calls"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:20Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:49Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++
TD-001 production ToolLoopAgent uses ToolContext::testing (broad test capabilities); TD-002 cancellation not propagated to tools; TD-003 audit log unwired; TD-004 per-tool timeout ignored; TD-006 parallel result miscorrelation.

Imported without verification from:
- `tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#3. Tool Audit (`tmp/tool-audit/`, 13 files, 36 findings)`
- `tmp/tool-audit/`

How to verify: grep for ToolContext::testing outside #[cfg(test)].

Verified 2026-09-28: partly fixed. The remaining scope is lower severity, so p1 -> p2. Fixed: TD-001, since `ToolLoopAgent` builds contexts through `ToolExecutionContextFactory` -> `ToolContext::production` (crates/roko-agent/src/tool_loop/agent_wrapper.rs:213, context_factory.rs:151), and `ToolContext::testing` now appears only under `#[cfg(test)]` and in testutil.rs. TD-002, since providers pass their cancel token into the agent (e.g. provider/anthropic_api/tool_loop.rs:86) and agent_wrapper.rs:202 threads it into every context. TD-004, since dispatcher/mod.rs:652-660 (T029) enforces `ToolDef::timeout_ms`. Still open: TD-003, the durable audit log has no production writer (tracked in find-f489db). TD-006, crates/roko-agent/src/tool_loop/mod.rs:1266-1280 stores `current_calls` in provider order beside previews built from `dispatch_batch` results, so traces still join them by vector index (current `dispatch_batch` ordering not re-checked).

Re-checked 2026-09-29: TD-001, TD-002 and TD-004 are still fixed. Two defects remain. TD-006: ToolDispatcher::dispatch_batch returns parallel-bucket results in completion order (buffer_unordered) followed by serial-bucket results, and tool_loop/mod.rs:1283-1302 pairs the provider-order current_calls with previews built from those results in ToolLoopTurnTrace and TurnProgress, so traces and on_turn progress can attach a result to the wrong call. The next-turn tool messages are not affected because render_results uses the (call, result) pairs. TD-003: DispatcherFactory::with_tool_audit (dispatch/factory.rs:278) still has no production caller; this is tracked in find-f489db, so the unique scope left here is TD-006.

## Notes

- 2026-10-01 (wk-guard2): implemented on work/bug-a70def; cargo verification deferred to the batch check.
- TD-006: `tool_result_previews` now takes the provider-order calls and gives each the preview of the result that carries its id, so `ToolLoopTurnTrace.tool_results` and `TurnProgress.tool_results` are index-aligned with `tool_calls` whatever order `dispatch_batch` returns them in. Test `turn_trace_pairs_results_with_their_calls` makes the first of two parallel calls finish last. TD-003 is fixed at BASE: `graph_execution/plan_runner.rs:838` attaches the scrubbed tool audit log (`df746d76c`, find-f489db, done), so no scope is left here.
