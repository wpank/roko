+++
id = "gap-2e4a81"
kind = "gap"
title = "PK64 M4 deep audits: DP3: dispatch applies the verify depth the ladder sets, V0 to V4 (S05 task 12, part 2)"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 64
size = "M"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK64"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs"]
lane = "rust-hot"
parent = "spec-635697"
links = { depends_on = ["gap-f0a7ee", "gap-940e44", "gap-eb39c1"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn dispatch_applies_the_ladder_depth_and_never_lowers_it_mid_window' crates/roko-cli/src/ && cargo test -p roko-cli --lib dispatch_applies_the_ladder_depth_and_never_lowers_it_mid_window"
+++

## Problem

This package delivers 1 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK64, slice 71xx, phase 8), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 7132 | M | p2 | DP3: dispatch applies the verify depth the ladder sets, V0 to V4 (S05 task 12, part 2) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7132-dp3-dispatch-applies-verify-depth.md` |

## Why it matters

Phase 8: M1 controller and guarded commit (S06). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7100-m4-random-deep-audits.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/graph_task_dispatch.rs`, `crates/roko-cli/src/graph_task_dispatch/verification.rs`, `crates/roko-cli/src/graph_task_dispatch/verify_depth.rs`.

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

- Waits on: PK58 (gap-f0a7ee), PK60 (gap-940e44), PK63 (gap-eb39c1).
- Suggested model: opus.
