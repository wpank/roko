+++
id = "gap-799698"
kind = "gap"
title = "PK91 M1 controller: Admin demo disturbance route: allowlisted kinds, showcase-owned runs, budget reserved…"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 91
size = "S"
hold = "waits on Will's deferred decision(s) 9303 (tmp/backlog/2026-10-02-complete-and-wire/DECISIONS.md)"
subsystem = ["roko-serve/routes"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK91"
anchors = ["crates/roko-serve/src/routes/learning/mod.rs"]
lane = "rust-cold"
parent = "spec-0b3a32"
links = { depends_on = ["gap-f7bab8", "gap-099513", "gap-fbd580"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn showcase_disturbance_route_rejects_unlisted_kind' crates/roko-serve/ && cargo test -p roko-serve showcase_disturbance_route_rejects_unlisted_kind"
+++

## Problem

This package delivers 1 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK91, slice 81xx, phase 9), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 8132 | S | p2 | Admin demo disturbance route: allowlisted kinds, showcase-owned runs, budget reserved first | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8132-admin-demo-disturbance-route-with-caps.md` |

## Why it matters

Phase 9: domains, assistant, held and parked work, cleanup, showcase and deploy. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8100-m1-ultrastable-controller-and-guarded-commit.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-serve/src/routes/learning/disturbances.rs`, `crates/roko-serve/src/routes/learning/mod.rs`.

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

- Waits on: PK62 (gap-f7bab8), PK71 (gap-099513), PK90 (gap-fbd580).
- On hold until Will takes the deferred decision(s) 9303 (spend or a public release); see `DECISIONS.md`.
- Suggested model: opus.
- 2026-10-04 (wave-13 follow-up, PK71 8135): confirmed at main HEAD `b7ad508ce`.
  `roko_core::disturbance::CeilingOverlay` (`crates/roko-core/src/disturbance.rs:536-541`) is
  already wired into `GraphTaskDispatcher` as a real field (`graph_task_dispatch.rs:305`,
  defaulted at line 365), with a getter `ceiling_overlay()` and builder `with_ceiling_overlay()`
  (`graph_task_dispatch/budget.rs:1042-1048`) — but `with_ceiling_overlay` has zero call sites
  anywhere outside its own definition, and `ceiling_overlay()` has exactly one caller, a test
  (`budget.rs:2095`). No caller sets a non-default overlay anywhere. When this task (8132) is
  picked back up, its admin route should take the handle from these two methods rather than
  inventing a new ceiling mechanism.
