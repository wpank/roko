+++
id = "gap-fa4d4b"
kind = "gap"
title = "PK70 M1 controller: B3 and B7: publish verify-depth floors and audit boosts to M4, with automatic audit…"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 70
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK70"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs"]
lane = "rust-cold"
parent = "spec-635697"
links = { depends_on = ["gap-f0a7ee", "gap-940e44", "gap-8b67de", "gap-eb39c1", "gap-2e4a81"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn audit_coupling_doubles_rate_after_cost_reducing_move' crates/roko-learn/ && grep -rqw 'fn extra_rungs_reach_m4_ladder_as_floor' crates/roko-cli/ && cargo test -p roko-learn audit_coupling_doubles_rate_after_cost_reducing_move && cargo test -p roko-cli extra_rungs_reach_m4_ladder_as_floor"
+++

## Problem

This package delivers 1 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK70, slice 81xx, phase 8), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 8127 | M | p2 | B3 and B7: publish verify-depth floors and audit boosts to M4, with automatic audit coupling | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8127-b3-b7-floor-requests-and-audit-coupling-to-m4.md` |

## Why it matters

Phase 8: M1 controller and guarded commit (S06). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8100-m1-ultrastable-controller-and-guarded-commit.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/graph_task_dispatch/verification.rs`, `crates/roko-learn/src/homeostasis/coupling.rs`.

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

- Waits on: PK58 (gap-f0a7ee), PK60 (gap-940e44), PK61 (gap-8b67de), PK63 (gap-eb39c1), PK64 (gap-2e4a81).
- Suggested model: opus.
