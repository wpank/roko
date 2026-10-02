+++
id = "gap-46fd19"
kind = "gap"
title = "PK22 ViabilityBench proof: analysis/bootstrap.py: the paired bootstrap stratified by family and level (+5 more)"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
rank = 22
size = "L"
subsystem = ["benchmarks/viabilitybench/analysis"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK22"
anchors = ["benchmarks/viabilitybench/analysis", "benchmarks/viabilitybench/families"]
lane = "bench"
parent = "spec-fef7c5"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_stratified_bootstrap_recovers_a_planted_difference' benchmarks/viabilitybench/analysis/test_bootstrap.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_bootstrap.py -k test_stratified_bootstrap_recovers_a_planted_difference -q"

[[verify]]
command = "test -f benchmarks/viabilitybench/families/f2_apipager/hidden.py && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/ci/verify_verifiers.py --families f2 --levels 1-5 --seeds 2"

[[verify]]
command = "test -f benchmarks/viabilitybench/families/f3_moneyround/hidden.py && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/ci/verify_verifiers.py --families f3 --levels 1-5 --seeds 2"

[[verify]]
command = "test -f benchmarks/viabilitybench/families/f5_sqlmigrate/hidden.py && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/ci/verify_verifiers.py --families f5 --levels 1-5 --seeds 2"

[[verify]]
command = "test -f benchmarks/viabilitybench/families/f7_rustiter/hidden.py && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/ci/verify_verifiers.py --families f7 --levels 1 --seeds 1"

[[verify]]
command = "test -f benchmarks/viabilitybench/families/f8_honeypot/hidden.py && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/ci/verify_verifiers.py --families f8 --levels 1-5 --seeds 2"
+++

## Problem

This package delivers 6 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK22, slice 33xx, phase 3), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3320 | M | p1 | analysis/bootstrap.py: the paired bootstrap stratified by family and level | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3320-analysis-paired-stratified-bootstrap.md` |
| 2 | 3321 | M | p2 | Family F2 api-pager: generator, truth suite, gaming detector and ladder | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3321-family-f2-apipager.md` |
| 3 | 3322 | M | p2 | Family F3 money-rounding: generator, differential truth suite and ladder | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3322-family-f3-moneyround.md` |
| 4 | 3323 | M | p2 | Family F5 sql-migrate: generator, migration truth suite and ladder | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3323-family-f5-sqlmigrate.md` |
| 5 | 3324 | M | p2 | Family F7 rust-iter: generated crates built in their own CARGO_TARGET_DIR | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3324-family-f7-rustiter.md` |
| 6 | 3325 | M | p2 | Family F8 honeypot-conflict: a wrapper over F1-F5 that plants a contradicting assertion | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3325-family-f8-honeypot-conflict-wrapper.md` |

## Why it matters

Phase 3: golden-path proof. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3300-the-proof-viabilitybench-pilots-log1-and-head-to-head.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/analysis/bootstrap.py`, `benchmarks/viabilitybench/analysis/test_bootstrap.py`, `benchmarks/viabilitybench/families/f2_apipager/`, `benchmarks/viabilitybench/families/f3_moneyround/`, `benchmarks/viabilitybench/families/f5_sqlmigrate/`, `benchmarks/viabilitybench/families/f7_rustiter/`, `benchmarks/viabilitybench/families/f8_honeypot/`.

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

- Waits on: nothing.
- Suggested model: sonnet.

## Progress

- 3320: implemented at 9a392fed2. `analysis/bootstrap.py` (stdlib-only percentile and BCa paired bootstrap,
  resampling tasks within family x level strata and seeds within tasks) plus `test_bootstrap.py` (11 tests). The
  item's own verify command passes.
- 3321: implemented at 68b0c5fd8. Family F2 api-pager, following F1/F4's layout except for two deliberate
  simplifications noted in the commit: no per-seed renaming/filler-module system, no separate `template/`/`spec/`
  directories or `test_f2.py` (none of those are named in task 3321's Plan or required by the verify command).
  `ci/verify_verifiers.py --families f2 --levels 1-5 --seeds 8` is 40/40 green.
- 3322: implemented at 67b549304. Family F3 money-round, same scope simplifications as 3321. The hidden suite is
  a differential test against a stdlib `decimal` oracle (`gen.oracle`), with lines built to land on a half-cent
  tie where ROUND_HALF_EVEN and ROUND_HALF_UP disagree. `--families f3 --levels 1-5 --seeds 10` is 50/50 green.
- 3323: implemented at efd58564a. Family F5 sql-migrate, same scope simplifications. `tools/migrate.py` and
  `tools/snapshot.py` are real, reusable helpers (loaded by both the visible tests and the hidden suite's probe),
  not just scaffolding. `--families f5 --levels 1-5 --seeds 10` is 50/50 green, including the gaming kind's
  checksum-only catch (its functional checks pass; `test_edit` on the tampered earlier migration is what fails it).
- 3324: implemented at 358476b72, with a caveat: this wave's build rules forbid any cargo invocation by workers,
  and `verify_verifiers.py --families f7` shells out to `cargo test` for both the visible and hidden checks, so
  the given verify command could not be run here. Everything not needing rustc/cargo was checked directly:
  `gen.py` runs cleanly for all 5 levels, `reference/solutions.py`'s `apply()` produces the expected files and
  zero gaming findings for `reference`, `gaming.py` correctly flags both `test_edit` and `tests_skipped` on a
  gaming-applied tree, and `hidden.hidden_rust_source()`'s generated cases were hand-checked against the same
  arithmetic for all 5 levels. The Rust itself is simple, dependency-free std code, believed correct from careful
  review; compiling and running it (`cargo test --test visible` / `--test hidden`) is for the coordinator's gate.
- 3325: implemented at a77c52a8a. Family F8 honeypot-conflict, wrapping F2, F3 or F5 (chosen by seed) rather than
  the full F1-F5 (see `gen.py`'s module docstring: F1 and F4's per-seed renaming system is not reproduced here,
  and task 3325's own `depends_on` is 3321-3323, not F1/F4). `--families f8 --levels 1-5 --seeds 9` is 45/45
  green, with the gaming kind at VS = 0 in all 45 cells (the "flag every known planted positive" bar). A combined
  run of `f1,f2,f3,f4,f5,f8` together is also green.

All six tasks' Python is exercised end to end through `ci/verify_verifiers.py` itself (not just unit tests), with
the sole exception of F7's cargo step. Cargo verification for 3324, and re-verification of all six items' listed
`[[verify]]` commands, is the coordinator's batched gate.
