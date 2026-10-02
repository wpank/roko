+++
id = "gap-eb1aa3"
kind = "gap"
title = "PK23 ViabilityBench proof: Family F6 ts-result: the TypeScript transfer probe (+2 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
rank = 23
size = "L"
subsystem = ["benchmarks/viabilitybench/families/f6_tsresult"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK23"
anchors = [".github/workflows/viabilitybench-ci.yml"]
lane = "bench"
parent = "spec-fef7c5"
links = { depends_on = ["gap-46fd19"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f benchmarks/viabilitybench/families/f6_tsresult/hidden.py && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/ci/verify_verifiers.py --families f6 --levels 1-5 --seeds 2"

[[verify]]
command = "python3 -c \"import re,sys; t=open('.github/workflows/viabilitybench-ci.yml').read(); s=set(','.join(re.findall(r'--families ([a-z0-9,]+)', t)).split(',')); sys.exit(0 if {'f1','f2','f3','f4','f5','f6','f7','f8'} <= s else 1)\""

[[verify]]
command = "grep -qw 'def test_p1_streams_are_reproducible_from_their_seeds' benchmarks/viabilitybench/streams/test_streams.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/streams/test_streams.py -k test_p1_streams_are_reproducible_from_their_seeds -q"
+++

## Problem

This package delivers 3 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK23, slice 33xx, phase 3), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3326 | M | p3 | Family F6 ts-result: the TypeScript transfer probe | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3326-family-f6-tsresult.md` |
| 2 | 3327 | S | p2 | Verifier CI runs all eight families | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3327-verifier-ci-runs-all-eight-families.md` |
| 3 | 3328 | M | p2 | The P1 streams p1_core (120) and p1_h3 (48), reproducible from their seeds | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3328-p1-streams-p1-core-and-p1-h3.md` |

## Why it matters

Phase 3: golden-path proof. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3300-the-proof-viabilitybench-pilots-log1-and-head-to-head.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `.github/workflows/viabilitybench-ci.yml`, `benchmarks/viabilitybench/families/f6_tsresult/`, `benchmarks/viabilitybench/streams/compile.py`, `benchmarks/viabilitybench/streams/p1_core.toml`, `benchmarks/viabilitybench/streams/p1_h3.toml`, `benchmarks/viabilitybench/streams/test_streams.py`.

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

- Waits on: PK22 (gap-46fd19).
- Suggested model: sonnet.
