+++
id = "gap-eb39c1"
kind = "gap"
title = "PK63 M1 controller: HomeostasisSink: one task resolution per chain, registered in the Graph feedback facade (+4 more)"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 63
size = "L"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-02
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "b68e41c37"
source = "tmp/backlog/2026-10-02-complete-and-wire PK63"
anchors = ["crates/roko-cli/src/dispatch/model_routing.rs", "crates/roko-cli/src/graph_task_dispatch/attempt.rs", "crates/roko-cli/src/graph_task_dispatch/failover.rs", "crates/roko-cli/src/graph_task_dispatch/ladder.rs", "crates/roko-cli/src/graph_task_dispatch/retry_budget.rs", "crates/roko-cli/src/graph_task_dispatch/streaming.rs", "crates/roko-cli/src/graph_task_dispatch/turn_policy.rs", "crates/roko-cli/src/graph_task_dispatch/verification.rs", "crates/roko-cli/src/runtime_feedback/mod.rs", "crates/roko-graph/src/cells/task_executor.rs", "crates/roko-learn/src/telemetry/writer.rs"]
lane = "rust-hot"
parent = "spec-635697"
links = { depends_on = ["gap-f548c1", "gap-cc5051", "gap-9e3134", "gap-b5caf3", "gap-7ec3ef", "gap-8b67de", "gap-f7bab8"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn homeostasis_sink_receives_one_resolution_per_chain' crates/roko-cli/ && cargo test -p roko-cli homeostasis_sink_receives_one_resolution_per_chain"

[[verify]]
command = "grep -rqw 'fn harness_policy_decision_row_per_dispatch' crates/roko-cli/ && cargo test -p roko-cli harness_policy_decision_row_per_dispatch"

[[verify]]
command = "grep -rqw 'fn tier_floor_raises_start_rung_but_pins_win' crates/roko-cli/ && cargo test -p roko-cli tier_floor_raises_start_rung_but_pins_win"

[[verify]]
command = "grep -rqw 'fn b4_b6_b8_knobs_read_per_dispatch' crates/roko-cli/ && cargo test -p roko-cli b4_b6_b8_knobs_read_per_dispatch"

[[verify]]
command = "grep -rqw 'fn task_executor_reads_live_retry_budget' crates/roko-graph/ && grep -rqw 'fn retry_delta_never_changes_authored_budget' crates/roko-cli/ && cargo test -p roko-graph task_executor_reads_live_retry_budget && cargo test -p roko-cli retry_delta_never_changes_authored_budget"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T20:53:45Z"
commit = "b68e41c37"
executor = "claude-agent"
via = "work-batch"
size = "L"
claimed_at = "2026-10-03T17:08:32Z"
forced = false
evidence = "Gate 9c (work/backlog-batch-9c, merged into main as b68e41c37): nightly fmt check, cargo check --workspace --tests, cargo clippy --workspace -D warnings, nextest --workspace --lib 15,087 passed, roko-cli bin 438 passed and the golden-path canaries pass (plan_validate: only bug-2a31bc's two known alias tests fail), roko-learn and roko-agent integration tests pass; every [[verify]] passes. PK63 5/5: HomeostasisSink and the SelfModelPredictor adapter, harness_policy rows and the verdict's theta stamp, the floor/cap ladder binding (merged with PK52's early climb so a skip also respects the cap, 97305f85c), per-attempt B4/B6/B8 knobs, the live retry budget. Gate fixes 9f955369a and 00e878ba6."
+++

## Problem

This package delivers 5 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK63, slice 81xx, phase 8), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 8122 | M | p2 | HomeostasisSink: one task resolution per chain, registered in the Graph feedback facade | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8122-homeostasis-sink-one-resolution-per-chain.md` |
| 2 | 8123 | M | p2 | Read the HarnessParams handle per dispatch, draw the arm, write one harness_policy decision row | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8123-per-dispatch-harness-params-and-decision-row.md` |
| 3 | 8124 | M | p2 | B1 actuator: tier floor and cap on the model ladder, with pins keeping precedence | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8124-b1-tier-floor-and-cap-on-the-model-ladder.md` |
| 4 | 8125 | M | p2 | B4, B6 and B8 actuators: error-pattern count, promise thresholds and task-budget scale per dispatch | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8125-b4-b6-b8-actuators-read-per-dispatch.md` |
| 5 | 8126 | M | p2 | B2 actuators: Graph retries read a live retry budget, and the turn cap takes a multiplier | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8126-b2-live-retry-budget-and-turn-cap-multiplier.md` |

## Why it matters

Phase 8: M1 controller and guarded commit (S06). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8100-m1-ultrastable-controller-and-guarded-commit.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/dispatch/model_routing.rs`, `crates/roko-cli/src/graph_execution/plan_runner.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, `crates/roko-cli/src/graph_task_dispatch/attempt.rs`, `crates/roko-cli/src/graph_task_dispatch/failover.rs`, `crates/roko-cli/src/graph_task_dispatch/ladder.rs`, `crates/roko-cli/src/graph_task_dispatch/retry_budget.rs`, `crates/roko-cli/src/graph_task_dispatch/streaming.rs`, `crates/roko-cli/src/graph_task_dispatch/turn_policy.rs`, `crates/roko-cli/src/graph_task_dispatch/verification.rs`, `crates/roko-cli/src/runtime_feedback/homeostasis.rs`, `crates/roko-cli/src/runtime_feedback/mod.rs`, `crates/roko-graph/src/cells/task_executor.rs`, `crates/roko-learn/src/telemetry/writer.rs`.

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

- Waits on: PK07 (gap-f548c1), PK09 (gap-cc5051), PK13 (gap-9e3134), PK32 (gap-b5caf3), PK49 (gap-7ec3ef), PK61 (gap-8b67de), PK62 (gap-f7bab8).
- Suggested model: opus.

## Progress

- 8122: implemented at bd8beb198 (cargo verification deferred to the batch gate). A-CTL rows go through the run's
  TelemetryWriter to the cross-run `learn/controller.jsonl` (S01 v1.3 §5.10); M3's prior reads the self-model through
  a SelfModelPredictor adapter.
- 8123: implemented at e033aae04 (cargo verification deferred to the batch gate).
- 8124: implemented at e2e9238e5 (cargo verification deferred to the batch gate). The floor raises the start rung
  through a rung hint on the routed task; the cap stops the climb; only knobs M1 moved from θ₀ bind; the
  harness_policy row says `pinned`.
- 8125: implemented at 3f340ad60 (cargo verification deferred to the batch gate). Budget admission runs before the
  attempt opens, so it reads the chain's θ itself.
- 8126: implemented at 941b58654 (cargo verification deferred to the batch gate). The live source sits on
  TaskExecutionSpec (`retry_budget`) and is attached by plan_runner; it also keeps M1's sink on the live limits.
