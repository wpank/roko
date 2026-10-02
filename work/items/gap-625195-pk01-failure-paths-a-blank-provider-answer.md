+++
id = "gap-625195"
kind = "gap"
title = "PK01 Failure paths: A blank provider answer fails as `empty_response` at the immune boundary, not as a… (+11 more)"
status = "open"
triage = "verified"
severity = "p1"
goal = "truth"
rank = 1
size = "L"
subsystem = ["roko-agent/immune"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK01"
anchors = ["crates/roko-agent/src/cursor_agent.rs", "crates/roko-agent/src/immune_boundary.rs", "crates/roko-agent/src/immune_evidence.rs", "crates/roko-agent/src/lib.rs", "crates/roko-agent/src/openai_compat_backend.rs", "crates/roko-agent/src/provider/error_classify.rs", "crates/roko-agent/src/streaming.rs", "crates/roko-agent/src/testutil.rs", "crates/roko-agent/src/tool_loop/mod.rs", "crates/roko-cli/src/commands/mod.rs", "crates/roko-cli/src/dispatch_v2.rs", "crates/roko-cli/src/main.rs", "crates/roko-serve/src/routes/route_permissions.rs", "crates/roko-serve/src/routes/safety.rs"]
lane = "rust-hot"
parent = "spec-65c828"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn blank_answer_then_retry_runs_second_attempt' crates/roko-agent/ && cargo test -p roko-agent blank_answer_then_retry_runs_second_attempt"

[[verify]]
command = "grep -rqw 'fn reasoning_only_stream_fails_as_empty_response' crates/roko-agent/ && cargo test -p roko-agent reasoning_only_stream_fails_as_empty_response"

[[verify]]
command = "grep -rqw 'fn multi_turn_stream_over_4096_events_is_accepted' crates/roko-agent/ && cargo test -p roko-agent multi_turn_stream_over_4096_events_is_accepted"

[[verify]]
command = "grep -rqw 'fn every_immune_denial_logs_its_reason' crates/roko-agent/ && cargo test -p roko-agent every_immune_denial_logs_its_reason"

[[verify]]
command = "grep -rqw 'fn released_control_lets_the_agent_run_again' crates/roko-agent/ && cargo test -p roko-agent released_control_lets_the_agent_run_again"

[[verify]]
command = "grep -rqw 'fn safety_release_clears_isolation_control' crates/roko-cli/tests/ && cargo test -p roko-cli --test safety_cli safety_release_clears_isolation_control"

[[verify]]
command = "grep -rqw 'fn release_route_clears_isolation_control' crates/roko-serve/ && cargo test -p roko-serve release_route_clears_isolation_control"

[[verify]]
command = "grep -rqw 'fn expired_isolation_control_does_not_deny' crates/roko-agent/ && cargo test -p roko-agent expired_isolation_control_does_not_deny"

[[verify]]
command = "grep -rqw 'fn full_control_ledger_admits_unknown_agent_after_pruning' crates/roko-agent/ && cargo test -p roko-agent full_control_ledger_admits_unknown_agent_after_pruning"

[[verify]]
command = "grep -rqw 'fn isolation_of_one_attempt_does_not_deny_the_next' crates/roko-cli/ && cargo test -p roko-cli isolation_of_one_attempt_does_not_deny_the_next"

[[verify]]
command = "grep -rqw 'fn sse_chunk_keeps_usage_finish_and_every_tool_call' crates/roko-agent/ && cargo test -p roko-agent sse_chunk_keeps_usage_finish_and_every_tool_call"

[[verify]]
command = "grep -rqw 'fn sse_error_object_fails_the_turn' crates/roko-agent/ && cargo test -p roko-agent sse_error_object_fails_the_turn"

[[verify]]
command = "grep -rqw 'fn stream_without_finish_reason_is_not_stop' crates/roko-agent/ && cargo test -p roko-agent stream_without_finish_reason_is_not_stop"

[[verify]]
command = "test -s crates/roko-agent/tests/fixtures/sse/glm-4.7-agent-turn.sse && grep -rqw 'fn glm_47_recorded_stream_replays_without_loss' crates/roko-agent/tests/ && cargo test -p roko-agent --test sse_replay glm_47_recorded_stream_replays_without_loss"

[[verify]]
command = "grep -rqw 'fn not_logged_in_classifies_as_auth_failure' crates/roko-cli/ && cargo test -p roko-cli not_logged_in_classifies_as_auth_failure"
+++

## Problem

This package delivers 12 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK01, slice 11xx, phase 1), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 1101 | S | p1 | A blank provider answer fails as `empty_response` at the immune boundary, not as a High-severity containment | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1101-blank-answer-fails-as-empty-response.md` |
| 2 | 1102 | S | p1 | The tool loop fails a run with no tool calls and no text as `empty_response`, whatever the finish reason | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1102-tool-loop-empty-run-is-empty-response.md` |
| 3 | 1103 | S | p1 | The provider stream cap counts one model call, leaves tool results out, and every denial logs its reason | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1103-stream-cap-counts-one-model-call.md` |
| 4 | 1104 | S | p1 | roko-agent can list isolation controls and release one, with an audit record | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1104-agent-control-list-and-release-api.md` |
| 5 | 1105 | M | p1 | `roko safety controls` lists immune isolations and `roko safety release` lifts one | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1105-roko-safety-release-command.md` |
| 6 | 1106 | S | p2 | roko serve lists immune isolations and releases one over REST | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1106-serve-route-releases-isolation.md` |
| 7 | 1108 | M | p1 | Isolation controls expire, and a full control ledger stops denying every new agent | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1108-isolation-controls-expire.md` |
| 8 | 1109 | S | p1 | Graph dispatch names each attempt's provider agent by its attempt, so one attempt's isolation cannot deny the next | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1109-graph-isolation-keyed-by-attempt.md` |
| 9 | 1110 | M | p1 | The OpenAI-compatible stream parser keeps every field of a chunk and every tool call | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1110-sse-chunk-yields-every-event.md` |
| 10 | 1111 | S | p1 | An SSE `error` object fails the turn, and a stream without a finish reason is not reported as `stop` | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1111-sse-error-objects-and-missing-finish.md` |
| 11 | 1112 | S | p2 | Capture one raw GLM-4.7 stream and keep it as a replay fixture | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1112-glm-raw-stream-replay-fixture.md` |
| 12 | 1113 | S | p1 | `classify_provider_error` reuses the shared classifier and names auth failures | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1113-classify-auth-failures.md` |

## Why it matters

Phase 1: safe runs. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1100-make-provider-failures-safe.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-agent/src/cursor_agent.rs`, `crates/roko-agent/src/immune_boundary.rs`, `crates/roko-agent/src/immune_evidence.rs`, `crates/roko-agent/src/lib.rs`, `crates/roko-agent/src/openai_compat_backend.rs`, `crates/roko-agent/src/provider/error_classify.rs`, `crates/roko-agent/src/streaming.rs`, `crates/roko-agent/src/testutil.rs`, `crates/roko-agent/src/tool_loop/mod.rs`, `crates/roko-agent/tests/fixtures/sse/glm-4.7-agent-turn.sse`, `crates/roko-agent/tests/sse_replay.rs`, `crates/roko-cli/src/commands/mod.rs`, `crates/roko-cli/src/commands/safety.rs`, `crates/roko-cli/src/dispatch_v2.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, `crates/roko-cli/src/main.rs`, `crates/roko-cli/tests/safety_cli.rs`, `crates/roko-serve/src/routes/route_permissions.rs`, `crates/roko-serve/src/routes/safety.rs`.

It also edits the hot file(s) `crates/roko-cli/src/graph_task_dispatch.rs`, which are left out of this item's anchors so that two hot packages can run at once; the coordinator resolves any merge conflict.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: nothing.
- Suggested model: opus.

## Progress

Implemented on `work/gap-625195` by a static worker (no cargo); cargo verification is deferred to the batch gate.

- 1101: implemented at 38dd54864
- 1102: implemented at 28714282b
- 1103: implemented at 6e4dd3383
- 1104: implemented at 220045bcf
- 1105: implemented at 80ab4098c
- 1106: implemented at efe7dc553
- 1108: implemented at 2b00d02be
- 1109: implemented at 9eed28f79
- 1110: implemented at 9dd006133
- 1111: implemented at 0830b0814
- 1112: blocked: no zai key in the worker's environment (`ZAI_API_KEY` unset; key files may not be read), so no raw GLM-4.7 stream was captured and no fixture was invented
- 1113: implemented at d562b56b2
