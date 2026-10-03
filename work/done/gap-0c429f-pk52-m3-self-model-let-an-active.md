+++
id = "gap-0c429f"
kind = "gap"
title = "PK52 M3 self-model: Let an active self-model climb early after a failure, within the ladder's limits"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 52
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-02
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "73a96794e"
source = "tmp/backlog/2026-10-02-complete-and-wire PK52"
anchors = ["crates/roko-cli/src/graph_task_dispatch/ladder.rs"]
lane = "rust-cold"
parent = "spec-abbc62"
links = { depends_on = ["gap-7ec3ef"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn self_model_climbs_early_within_k_max' crates/roko-cli/src/ && cargo test -p roko-cli self_model_climbs_early_within_k_max"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T19:23:33Z"
commit = "73a96794e"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-03T17:08:29Z"
forced = false
evidence = "Gate 9b (work/backlog-batch-9b, merged into main as 73a96794e): cargo check --workspace --tests, roko-cli and roko-serve with fault-injection, nightly fmt, cargo clippy --workspace -D warnings, nextest --lib 5,854 tests over roko-cli, -learn and -serve, roko-cli bin 436 passed, the golden-path canaries and the loop-audit census run pass (plan_validate: only bug-2a31bc's two known alias tests fail), roko-learn integration tests pass, 159 fault-injection lib tests pass; every [[verify]] passes. PK52 (6131): a self-model-started chain climbs after one failure when its re-forecast says so, within K_max; the test's training was fixed in 9637c908b (27 interleaved outcomes, under N_MIN, so L0 forecasts drive it); the code was right."
+++

## Problem

This package delivers 1 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK52, slice 61xx, phase 6), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 6131 | M | p2 | Let an active self-model climb early after a failure, within the ladder's limits | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6131-let-an-active-self-model-climb-early-after-a-failure-within.md` |

## Why it matters

Phase 6: M3 calibrated self-model (S04). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6100-epic-m3-calibrated-self-model.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/graph_task_dispatch/ladder.rs`, `crates/roko-cli/src/graph_task_dispatch/self_model.rs`.

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

- Waits on: PK49 (gap-7ec3ef).
- Suggested model: opus.

## Progress

- 6131: implemented at fb7f1b462 (cargo verification deferred to the batch gate)
