+++
id = "gap-93c748"
kind = "gap"
title = "PK89 M4 deep audits: Serve: GET /api/audit/estimates and /api/audit/incidents, and audit events on SSE (S05…"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
rank = 89
size = "M"
hold = "waits on Will's deferred decision(s) 9303 (tmp/backlog/2026-10-02-complete-and-wire/DECISIONS.md)"
subsystem = ["roko-serve/routes"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK89"
anchors = ["crates/roko-serve/src/routes/mod.rs", "crates/roko-serve/src/routes/sse.rs"]
lane = "rust-cold"
parent = "spec-0b3a32"
links = { depends_on = ["gap-940e44", "gap-4119fb"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn audit_routes_serve_estimates_and_incidents' crates/roko-serve/src/routes/audit.rs && cargo test -p roko-serve --lib audit_routes_serve_estimates_and_incidents"
+++

## Problem

This package delivers 1 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK89, slice 71xx, phase 9), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 7137 | M | p3 | Serve: GET /api/audit/estimates and /api/audit/incidents, and audit events on SSE (S05 task 13) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7137-serve-audit-routes-and-sse.md` |

## Why it matters

Phase 9: domains, assistant, held and parked work, cleanup, showcase and deploy. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7100-m4-random-deep-audits.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-serve/src/routes/audit.rs`, `crates/roko-serve/src/routes/mod.rs`, `crates/roko-serve/src/routes/sse.rs`.

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

- Waits on: PK60 (gap-940e44), PK83 (gap-4119fb).
- On hold until Will takes the deferred decision(s) 9303 (spend or a public release); see `DECISIONS.md`.
- Suggested model: opus.
- 2026-10-02 (filer-grpD, backlog wave reports, PK28 gap-c06ff3): when this task's live view/SSE is implemented,
  have it call `benchmarks/viabilitybench/audit/estimate.py`'s `betting_cs`/`sequence_holds` with
  `z_max=lottery.EPS_FLOOR/pi_min` (the window's actual minimum inclusion probability) instead of leaving the
  default `z_max=1.0` — the narrower bound is already implemented and tested
  (`audit/replay.py:267-275`'s `"uniform_scaled"` demonstration; `test_estimate.py::test_betting_cs_contains_the_exact_sequence_and_narrows`),
  it just has nothing calling it from a live/served path yet. Confirmed at HEAD: no caller of `betting_cs` or
  `sequence_holds` exists outside `estimate.py`, its tests, and `replay.py`'s offline CLI.
