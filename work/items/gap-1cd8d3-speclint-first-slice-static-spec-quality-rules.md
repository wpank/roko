+++
id = "gap-1cd8d3"
kind = "gap"
title = "speclint first slice: static spec-quality rules SQ01–SQ12 with hard fails (S07.1)"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "248431af0"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e8"
discovered_from = "tmp/cybernetic-harness/execution/checklist.json (S07.1); specs/S07-spec-quality.md §4.2"
anchors = ["benchmarks/viabilitybench/speclint/speclint.py", "benchmarks/viabilitybench/speclint/tests/test_speclint.py"]
lane = "bench"
parent = "spec-e57870"
links = { depends_on = ["dec-b78874"], blocks = [], related = ["find-70edcb", "gap-46ab3f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_golden_fixture_per_rule' benchmarks/viabilitybench/speclint/tests/test_speclint.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/speclint/tests/test_speclint.py -k test_golden_fixture_per_rule -q"

[[verify]]
command = "grep -qw 'def test_e2e_smoke_t02_is_band_d' benchmarks/viabilitybench/speclint/tests/test_speclint.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/speclint/tests/test_speclint.py -k test_e2e_smoke_t02_is_band_d -q"

[closed]
at = 2026-09-29
commit = "248431af0"
by = "commit trailer"
evidence = "248431af0: speclint.py scores all 551 tasks of plans/**/tasks.toml at a17d4dadd (0 parse errors; two runs identical apart from ts) with SQ01-SQ12 and HF1/HF2/HF4/HF5. 16 hand-computed golden fixtures (one per rule and static hard fail) pass; e2e-smoke-test T02 scores 27.0, band D, SQ04 = 0. On the prototype corpus (git archive 725f21e05 plans, 484 tasks) the S07 section 3.3 rates are 82.0/15.7/27.5/24.8% against 81.6/16.7/27.7/24.8%, and all 33 cargo test --no-run steps are compile. Both [[verify]] commands pass."
+++

## Problem

Nothing measures whether a task spec is good enough to execute. `plan validate` checks structure and runtime safety
only. `TasksFile::quality_warnings` checks presence (description length, read files, verify steps) and prints only
to stderr during dry runs. A prototype linter scored 484 tasks for spec S07: mean 69.7 of 100; 81.6% declare no
acceptance criteria; 16.7% have only grep-style verify steps; none has a hidden-test hook. The prototype lived in a
scratchpad and is gone.

## Why it matters

Spec quality is the cost lever S07 tests (hypothesis H3: vague specs hurt cheap models most). This linter is S07's
first slice. It feeds the H3 experiment and the pilot benchmark (epic E12), and `plan validate --spec-quality`
(gap-46ab3f) must match it. tldr/05 P1 #10. Part of epic spec-e57870.

## Where

- **New:** `benchmarks/viabilitybench/speclint/speclint.py`, `fixtures/` (one golden fixture per rule) and
  `tests/test_speclint.py`. **The path follows decision D4** (dec-b78874: the benchmark's name and location).
  Checklist S07.1 still cites the stale `benchmarks/rokocyber/speclint/`.
- Input: every `plans/**/tasks.toml` (136 files at `41c7ffbd6`; 130 when S07 was written).
- Output: one `spec.quality` JSONL record per task (S07 §5), written outside the repo (`$VB_RESULTS`, default
  `~/.roko-bench/viability`).

## Current state

Checked at `41c7ffbd6`: `benchmarks/` holds only `dev-audit/`. No speclint or viabilitybench file exists.

## Plan

1. Parse each `tasks.toml` with `tomllib`. Score SQ01–SQ12 with S07 §4.2's weights, bands and vague-term lexicon v1,
   plus the static hard fails HF1, HF2, HF4 and HF5. In static mode SQ06 and HF3 are `unknown`; gap-b3fa0a adds them.
2. Classify verify strength: test 1.0, a run with a state assertion 0.8, compile 0.5, structural 0.25. A
   `cargo test --no-run` step counts as compile.
3. `speclint.py plans/ --out <dir>/speclint.jsonl` prints a summary: score distribution, bands, per-rule pass rates,
   verify classes and the worst tasks. Report archived plans separately, since they dominate the corpus.
4. Golden fixtures: one per rule and one per static hard fail.

## Done when

- [ ] It runs over every `plans/**/tasks.toml` with 0 crashes and one record per task; two runs differ only in `ts`.
- [ ] Rule-level rates reproduce S07 §3.3 within ±3 points. The 69.7 mean is not a target.
- [ ] `plans/e2e-smoke-test` T02 is band D with SQ04 = 0.
- [ ] Both `[[verify]]` commands pass.

## Notes

- Python standard library only, no model calls, $0.
- Waits for D4. If D4 moves the tree outside this repo, re-anchor this item and gap-b3fa0a, and vendor the fixtures
  for gap-46ab3f's parity test.
- 2026-09-29 (wk-speclint): built. On the prototype's own corpus (`git archive 725f21e05 plans`: 484 tasks, 323
  archived) the §3.3 rates are no acceptance criteria 82.0% (81.6), structural-only strongest verify 15.7% (16.7),
  test-runner verify 27.5% (27.7), no `read_files` 24.8% (24.8), hidden hook 0; all 33 `cargo test --no-run` steps
  are compile. The acceptance-phrasing test is lexical, as the prototype's was: counting only the `acceptance` and
  `acceptance_contract` fields gives 86.2%. At BASE `a17d4dadd` (551 tasks) the mean is 48.2 and no task reaches
  band A: SQ03, SQ06 and SQ12 (25 points) are 0 until the TSS fields and the dynamic checker land.
