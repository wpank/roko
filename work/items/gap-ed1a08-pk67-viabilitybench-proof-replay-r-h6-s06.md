+++
id = "gap-ed1a08"
kind = "gap"
title = "PK67 ViabilityBench proof: Replay R-H6: S06's controllers A0-A5 plus A3-gated and A3-mis (+2 more)"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
rank = 67
size = "L"
subsystem = ["benchmarks/viabilitybench/analysis"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK67"
anchors = ["benchmarks/viabilitybench/driver/planemit.py"]
lane = "bench"
parent = "spec-635697"
links = { depends_on = ["gap-cb5133", "gap-de0b87", "gap-1149aa", "gap-c06ff3", "gap-2ca903", "gap-c1d920", "gap-85d176", "gap-7ec3ef", "gap-63fd4c", "gap-0c429f", "gap-147c4d", "gap-f7bab8", "gap-414e56"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_r_h6_replay_runs_every_controller_and_hook' benchmarks/viabilitybench/analysis/test_replay_h6.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_replay_h6.py -k test_r_h6_replay_runs_every_controller_and_hook -q"

[[verify]]
command = "grep -qw 'def test_closure_replays_emit_their_preregistered_estimands' benchmarks/viabilitybench/analysis/test_replay_closure.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_replay_closure.py -k test_closure_replays_emit_their_preregistered_estimands -q"

[[verify]]
command = "grep -qw 'def test_roko_full_overlay_turns_on_gate_routing_audits_and_holdout' benchmarks/viabilitybench/driver/test_planemit.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_planemit.py -k test_roko_full_overlay_turns_on_gate_routing_audits_and_holdout -q"
+++

## Problem

This package delivers 3 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK67, slice 33xx, phase 8), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3357 | S | p2 | Replay R-H6: S06's controllers A0-A5 plus A3-gated and A3-mis | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3357-replay-r-h6-controllers.md` |
| 2 | 3358 | M | p2 | Closure replays X1-X4, pre-registered as exploratory | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3358-closure-replays-x1-to-x4.md` |
| 3 | 3360 | M | p1 | The roko_full arm overlay: the ladder plus S07's gate, S04 routing, S05 audits and the holdout | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3360-roko-full-arm-overlay.md` |

## Why it matters

Phase 8: M1 controller and guarded commit (S06). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3300-the-proof-viabilitybench-pilots-log1-and-head-to-head.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/analysis/replay_closure.py`, `benchmarks/viabilitybench/analysis/replay_h6.py`, `benchmarks/viabilitybench/analysis/test_replay_closure.py`, `benchmarks/viabilitybench/analysis/test_replay_h6.py`, `benchmarks/viabilitybench/arms/roko_full.toml`, `benchmarks/viabilitybench/driver/planemit.py`, `benchmarks/viabilitybench/driver/test_planemit.py`.

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

- Waits on: PK17 (gap-cb5133), PK19 (gap-de0b87), PK21 (gap-1149aa), PK28 (gap-c06ff3), PK30 (gap-2ca903), PK43 (gap-c1d920), PK44 (gap-85d176), PK49 (gap-7ec3ef), PK50 (gap-63fd4c), PK52 (gap-0c429f), PK59 (gap-147c4d), PK62 (gap-f7bab8), PK66 (gap-414e56).
- Suggested model: sonnet.
