+++
id = "gap-c013c5"
kind = "gap"
title = "PK16 Defaults that apply: Three live golden-path runs pass (pass^3), with an independent review of each merged diff (+1 more)"
status = "open"
triage = "verified"
severity = "p1"
goal = "golden-path"
rank = 16
size = "M"
hold = "waits on Will's deferred decision(s) 3114 (tmp/backlog/2026-10-02-complete-and-wire/DECISIONS.md)"
subsystem = ["self-hosting"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK16"
anchors = ["work/history/"]
lane = "rust-cold"
parent = "spec-fef7c5"
links = { depends_on = ["gap-625195", "gap-e00238", "gap-f548c1", "gap-997366", "gap-c1f4ac"], blocks = [], related = ["gap-f30b8e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "set -- work/history/golden-path-live-*.md; test -f \"$1\" && test \"$(grep -Eo 'graph-(run-)?[0-9a-f]+' \"$1\" | sort -u | wc -l)\" -ge 3 && grep -q '3 of 3 live runs passed' \"$1\""

[[verify]]
command = "set -- work/history/plan-set-live-*.md; test -f \"$1\" && grep -q -- '--max-parallel-plans 3' \"$1\" && grep -Eq 'graph-(run-)?[0-9a-f]' \"$1\" && grep -q 'all three plans started before the first finished' \"$1\""
+++

## Problem

This package delivers 2 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK16, slice 31xx, phase 3), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3118 | M | p1 | Three live golden-path runs pass (pass^3), with an independent review of each merged diff | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3118-golden-path-three-live-runs-pass.md` |
| 2 | 3119 | S | p2 | One live plan set: three independent plans run side by side under per-task worktrees | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3119-live-plan-set-three-plans-side-by-side.md` |

## Why it matters

Phase 3: golden-path proof. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3100-defaults-that-apply-parallelism-ladder-integration.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `work/history/`.

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

- Waits on: PK01 (gap-625195), PK02 (gap-e00238), PK07 (gap-f548c1), PK14 (gap-997366), PK15 (gap-c1f4ac).
- On hold until Will takes the deferred decision(s) 3114 (spend or a public release); see `DECISIONS.md`.
- Existing work items this package covers or touches: gap-f30b8e. When its tasks are done, close those whose verify then passes.
- Suggested model: sonnet.
