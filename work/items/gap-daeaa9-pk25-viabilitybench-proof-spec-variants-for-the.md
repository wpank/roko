+++
id = "gap-daeaa9"
kind = "gap"
title = "PK25 ViabilityBench proof: Spec variants for the 48 H3 instances: precise, vague by D-v1, and the recoverability… (+3 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
rank = 25
size = "L"
subsystem = ["benchmarks/viabilitybench/streams"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK25"
anchors = ["benchmarks/viabilitybench/driver/materialize.py", "benchmarks/viabilitybench/families/f1_pyconv/spec/", "benchmarks/viabilitybench/families/f4_kvtool/spec/"]
lane = "bench"
parent = "spec-fef7c5"
links = { depends_on = ["gap-de0b87", "gap-46fd19", "gap-eb1aa3", "gap-e120a1"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_h3_variants_pass_the_manipulation_check' benchmarks/viabilitybench/driver/test_materialize.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_materialize.py -k test_h3_variants_pass_the_manipulation_check -q"

[[verify]]
command = "grep -qw 'def test_log1_compiler_emits_s09_cell_counts' benchmarks/viabilitybench/streams/test_streams.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/streams/test_streams.py -k test_log1_compiler_emits_s09_cell_counts -q"

[[verify]]
command = "grep -qw 'def test_s_streams_match_s08_compositions' benchmarks/viabilitybench/streams/test_streams.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/streams/test_streams.py -k test_s_streams_match_s08_compositions -q"

[[verify]]
command = "grep -qw 'def test_select_is_reproducible_and_caps_tasks_per_repo' benchmarks/viabilitybench/external/swebench/test_swebench.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/external/swebench/test_swebench.py -k test_select_is_reproducible_and_caps_tasks_per_repo -q"
+++

## Problem

This package delivers 4 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK25, slice 33xx, phase 3), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3329 | M | p2 | Spec variants for the 48 H3 instances: precise, vague by D-v1, and the recoverability check | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3329-h3-spec-variants-and-recoverability-check.md` |
| 2 | 3330 | M | p2 | The LOG1 stream compiler: blocks A-F with S09's cell counts | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3330-log1-stream-compiler.md` |
| 3 | 3331 | S | p2 | Streams s1_learncurve, s3_disturbance and s5_holdout | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3331-streams-s1-s3-s5.md` |
| 4 | 3332 | M | p2 | SWE-bench slice: seeded selection and the contamination probe | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3332-swe-bench-slice-selection-and-probe.md` |

## Why it matters

Phase 3: golden-path proof. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3300-the-proof-viabilitybench-pilots-log1-and-head-to-head.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/driver/materialize.py`, `benchmarks/viabilitybench/driver/test_materialize.py`, `benchmarks/viabilitybench/external/swebench/probe.py`, `benchmarks/viabilitybench/external/swebench/select.py`, `benchmarks/viabilitybench/external/swebench/test_swebench.py`, `benchmarks/viabilitybench/families/f1_pyconv/spec/`, `benchmarks/viabilitybench/families/f4_kvtool/spec/`, `benchmarks/viabilitybench/streams/compile.py`, `benchmarks/viabilitybench/streams/log1.toml`, `benchmarks/viabilitybench/streams/s1_learncurve.toml`, `benchmarks/viabilitybench/streams/s3_disturbance.toml`, `benchmarks/viabilitybench/streams/s5_holdout.toml`, `benchmarks/viabilitybench/streams/test_streams.py`.

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

- Waits on: PK19 (gap-de0b87), PK22 (gap-46fd19), PK23 (gap-eb1aa3), PK24 (gap-e120a1).
- Suggested model: sonnet.
