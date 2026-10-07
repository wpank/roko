+++
id = "gap-59ebfd"
kind = "gap"
title = "PK84 M3 self-model: Serve the economics report and the calibration stream for the showcase"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
rank = 84
size = "S"
subsystem = ["roko-serve/showcase"]
created = 2026-10-02
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "cb8cbfeed"
source = "tmp/backlog/2026-10-02-complete-and-wire PK84"
anchors = ["crates/roko-serve/src/routes"]
lane = "rust-cold"
parent = "spec-0b3a32"
links = { depends_on = ["gap-d90ef6", "gap-7ec3ef", "gap-9ecd37", "gap-4119fb"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn economics_route_serves_stored_report_bytes' crates/roko-serve/ && cargo test -p roko-serve economics_route_serves_stored_report_bytes"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T04:48:55Z"
commit = "cb8cbfeed"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-04T01:55:05Z"
forced = false
evidence = "Gate 13c (merged cb8cbfeed): lib tests and serve integration pass; verify economics_route_serves_stored_report_bytes passes. 6134: GET /api/showcase/economics?experiment_id= serves econ-report.json byte for byte from the newest verified bundle behind showcase auth (400/404/409), and a self_model.calibration StateHub event mirrors calibration.json."
+++

## Problem

This package delivers 1 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK84, slice 61xx, phase 9), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 6134 | S | p3 | Serve the economics report and the calibration stream for the showcase | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6134-serve-the-economics-report-and-the-calibration-stream-for.md` |

## Why it matters

Phase 9: domains, assistant, held and parked work, cleanup, showcase and deploy. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6100-epic-m3-calibrated-self-model.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-serve/src/routes/showcase/economics.rs`, `crates/roko-serve/src/routes/showcase/mod.rs`.

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

- Waits on: PK48 (gap-d90ef6), PK49 (gap-7ec3ef), PK82 (gap-9ecd37), PK83 (gap-4119fb).
- Suggested model: opus.

## Progress

Implemented by w4-pk02 on `work/gap-59ebfd` (from batch 13b, d5f4f38d2); cargo verification deferred to the batch gate.

- 6134: implemented at 50f7f1602. `GET /api/showcase/economics?experiment_id=<id>` in `routes/showcase/economics.rs` serves `econ/<id>/econ-report.json` byte for byte from the newest verified bundle that lists it (400 without an id, 404 for an unknown or non-segment id, 409 when it was rejected), behind the showcase session auth. A calibration mirror, started beside the config watcher in both serve paths, publishes each new `.roko/learn/self-model/calibration.json` as a `self_model.calibration` DashboardEvent. The bundle builder does not copy reports into `econ/<id>/` yet.
