+++
id = "gap-e00238"
kind = "gap"
title = "PK02 Failure paths: Immune denials leave provider health alone, and each attempt records provider health once (+7 more)"
status = "done"
triage = "verified"
severity = "p1"
goal = "truth"
rank = 2
size = "L"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-10-02
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "2724386ea"
source = "tmp/backlog/2026-10-02-complete-and-wire PK02"
anchors = ["crates/roko-cli/src/dispatch/mod.rs", "crates/roko-cli/src/dispatch/model_routing.rs", "crates/roko-cli/src/dispatch_v2.rs", "crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs", "crates/roko-cli/src/graph_task_dispatch/failover.rs", "crates/roko-cli/src/graph_task_dispatch/ladder.rs", "crates/roko-cli/src/graph_task_dispatch/red_flags.rs", "crates/roko-cli/src/graph_task_dispatch/streaming.rs", "crates/roko-core/src/error/mod.rs", "crates/roko-graph/src/cells/task_executor.rs", "crates/roko-learn/src/model_call_feedback.rs", "crates/roko-learn/src/provider_health.rs"]
lane = "rust-hot"
parent = "spec-65c828"
links = { depends_on = ["gap-625195"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn immune_denial_leaves_provider_health_unchanged' crates/roko-cli/ && cargo test -p roko-cli immune_denial_leaves_provider_health_unchanged"

[[verify]]
command = "grep -rqw 'fn bridge_attempt_records_provider_health_once' crates/roko-cli/ && cargo test -p roko-cli bridge_attempt_records_provider_health_once"

[[verify]]
command = "grep -rqw 'fn auth_failure_refuses_provider_with_login_hint' crates/roko-cli/ && cargo test -p roko-cli auth_failure_refuses_provider_with_login_hint"

[[verify]]
command = "grep -rqw 'fn permanent_provider_denial_is_not_retried' crates/roko-cli/ && cargo test -p roko-cli permanent_provider_denial_is_not_retried"

[[verify]]
command = "grep -rqw 'fn retries_back_off_and_skip_permanent_denials' crates/roko-graph/ && cargo test -p roko-graph retries_back_off_and_skip_permanent_denials"

[[verify]]
command = "grep -rqw 'fn substitute_failure_does_not_count_against_routed_rung' crates/roko-cli/ && cargo test -p roko-cli substitute_failure_does_not_count_against_routed_rung"

[[verify]]
command = "grep -rqw 'fn open_circuit_on_rung_fails_over_to_next_rung' crates/roko-cli/ && cargo test -p roko-cli open_circuit_on_rung_fails_over_to_next_rung"

[[verify]]
command = "grep -rqw 'fn preflight_skips_rung_that_returns_blank_answer' crates/roko-cli/ && cargo test -p roko-cli preflight_skips_rung_that_returns_blank_answer"

[[verify]]
command = "grep -rqw 'fn retry_after_tamper_rejection_starts_from_plan_tip' crates/roko-cli/ && cargo test -p roko-cli retry_after_tamper_rejection_starts_from_plan_tip"

[closed]
at = 2026-10-03
at_ts = "2026-10-02T22:17:20Z"
commit = "2724386ea"
executor = "claude-agent"
via = "work-batch"
size = "L"
claimed_at = "2026-10-02T19:13:13Z"
forced = false
evidence = "Gate 4c (merged into main as 2724386ea, tree identical to work/backlog-batch-4c apart from work/): cargo check --workspace --tests, cargo clippy --workspace -D warnings, nextest --lib 8,717 passed over 7 crates, roko bin tests + golden-path canaries 436/436, ViabilityBench suite 528 passed; every [[verify]] passes (lib/bin/integration tests named in each verify passed; static parts rc=0)."
+++

## Problem

This package delivers 8 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK02, slice 11xx, phase 1), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 1114 | M | p1 | Immune denials leave provider health alone, and each attempt records provider health once | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1114-immune-denials-skip-provider-health.md` |
| 2 | 1115 | S | p2 | An auth failure takes its provider out of the run and says how to log in, instead of opening a circuit | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1115-auth-failure-is-definitive-in-failover.md` |
| 3 | 1116 | S | p2 | Graph dispatch fails an attempt with a non-retryable error for denials a retry cannot fix | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1116-permanent-denials-are-not-retried.md` |
| 4 | 1117 | M | p2 | `TaskExecutorCell` waits between retries, with jitter, and at least as long as a provider's retry-after | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1117-task-retries-back-off-and-honour-retry-after.md` |
| 5 | 1118 | S | p2 | The ladder does not count a failover substitute's failure against the rung that was routed | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1118-ladder-ignores-substitute-failures.md` |
| 6 | 1120 | M | p2 | Failover of a ladder-routed task tries the same model elsewhere, then the next rung up, never a cheaper model | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1120-failover-climbs-the-ladder.md` |
| 7 | 1121 | M | p3 | Probe each ladder rung with one tool-use call at plan start and skip rungs that cannot do agent work | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1121-probe-ladder-rungs-at-plan-start.md` |
| 8 | 1122 | M | p1 | A retry after a tamper rejection starts in a fresh checkout of the plan branch | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1122-retry-after-tamper-starts-fresh.md` |

## Why it matters

Phase 1: safe runs. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1100-make-provider-failures-safe.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/dispatch/mod.rs`, `crates/roko-cli/src/dispatch/model_routing.rs`, `crates/roko-cli/src/dispatch/rung_probe.rs`, `crates/roko-cli/src/dispatch_v2.rs`, `crates/roko-cli/src/graph_execution/plan_runner.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, `crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs`, `crates/roko-cli/src/graph_task_dispatch/failover.rs`, `crates/roko-cli/src/graph_task_dispatch/ladder.rs`, `crates/roko-cli/src/graph_task_dispatch/red_flags.rs`, `crates/roko-cli/src/graph_task_dispatch/streaming.rs`, `crates/roko-core/src/error/mod.rs`, `crates/roko-graph/src/cells/task_executor.rs`, `crates/roko-learn/src/model_call_feedback.rs`, `crates/roko-learn/src/provider_health.rs`.

It also edits the hot file(s) `crates/roko-cli/src/graph_execution/plan_runner.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, which are left out of this item's anchors so that two hot packages can run at once; the coordinator resolves any merge conflict.

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

- Waits on: PK01 (gap-625195).
- Suggested model: opus.

## Progress

Static worker, 2026-10-02, branch `work/gap-e00238` from `248d279c7`. Each task is implemented on the branch; cargo
verification is deferred to the batch gate.

- 1114: implemented at 8b7ceee1f
- 1115: implemented at d51fbb059
- 1116: implemented at 2a8bc2a4d (deviation: `agent_isolated` stays retryable, because decision 1107/1109 gives
  each attempt its own agent id; the test drives the auth and unreadable-ledger denials instead)
- 1117: implemented at c4ee39306
- 1118: implemented at 76e1ec475
- 1120: implemented at bfc8eb72b (an open circuit with no usable rung above still probes the routed model, as
  before; the no-usable-provider error is for definitive refusals)
- 1121: implemented at a0971af2d
- 1122: implemented at 27896a439
