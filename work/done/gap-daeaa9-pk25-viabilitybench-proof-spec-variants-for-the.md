+++
id = "gap-daeaa9"
kind = "gap"
title = "PK25 ViabilityBench proof: Spec variants for the 48 H3 instances: precise, vague by D-v1, and the recoverability… (+3 more)"
status = "done"
triage = "verified"
severity = "p2"
goal = "proof"
rank = 25
size = "L"
subsystem = ["benchmarks/viabilitybench/streams"]
created = 2026-10-02
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "823f2cfca"
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

[closed]
at = 2026-10-03
at_ts = "2026-10-03T06:07:30Z"
commit = "823f2cfca"
executor = "claude-agent"
via = "work-batch"
size = "L"
claimed_at = "2026-10-03T04:13:07Z"
forced = false
evidence = "Gate 7a (work/backlog-batch-7a, merged into main as 823f2cfca): cargo check --workspace --tests, nightly fmt, cargo clippy --workspace -D warnings, nextest --lib over roko-cli, -compose, -core, -gate, -learn and -runtime (8,225 tests; two calibration-gate fixture failures fixed in 1952f3e82), roko-cli bin 429 passed and the golden-path canaries pass (plan_validate: only bug-2a31bc's two known alias tests fail), roko-learn integration tests pass, the parked features' lib tests pass (cognitive-clock 286, cross-cut-functors 556, spc 699, active-inference 1,297), the default roko-cli tree still has no roko-chain or Alloy provider graph and --features chain builds, ViabilityBench suite 652 passed after the gate's path fix; every [[verify]] passes. PK25 4/4; test_materialize.py's family paths anchored on the test file so the verify passes from the repo root (1952f3e82)."
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

## Progress

- 3329: implemented at `78bb3417d`. `materialize.py` builds `spec.vague.md` itself on the first
  `spec_variant="vague"` request (F1 and F4 only), caches it in the manifest, and checks recoverability before
  writing (a `recoverability` entry backed only by `spec.precise.md#ACn` references, for an AC the vague spec no
  longer states, fails materialization). Fixed a real gap this surfaced: `f4_kvtool/gen.py`'s recoverability had
  two entries with no file-based evidence at all. Verified against the real F1/F4 generators: 0 recoverability
  violations, and `specops.manipulation_check` (task 3234) reports gaps of 49-70 points on 4 real instances.
- 3330: implemented at `44a17db14`. `streams/compile.py` gains `compile_log1`, one row per cell, reusing the
  p1_core/p1_h3/pass^5 streams and `pilot.toml`. Verified: billed totals match S09 §4.3 exactly (A 960, B 660,
  C 336, D 120, E 80 = 2,156); block C's precise row covers only rep 3, noting block A's seeds 1-2 as reps 1-2.
- 3331: implemented at `1928525de`. `compile_s1`/`compile_s3`/`compile_s5` match S08 §4.7: S1 is F1-F4 x 24 in
  24 blocks of 4; S3 is 10 nominal + 30 disturbed (6 per S08 §4.6 hook), F1/F4 only (discovered F2/F3 build
  latent v1 only, so they can't take the convention_flip hook); S5 is 120 spread over P1-core's 6 families.
  `compile_s3` also writes a real `vb.disturbance/1` file; verified it loads through `driver/disturb.py` with
  each position resolving to exactly one covering hook.
- 3332: implemented at `f1f20552c`. `external/swebench/select.py`: seed 20260928, stratified-by-repository
  selection of 60, filtered by difficulty/FAIL_TO_PASS/patch size, capped at 12/repo, refusing fewer than 5
  repos. `probe.py`: gives each model only the issue text, excludes at 2-of-5 gold-file agreement; its live run
  is task 3353, out of scope here. Found and fixed a real bug along the way: naming the module `select.py`
  collides with the standard library's own `select` (silently resolved once anything else imports it) - both
  `probe.py` and its test load it by path instead. Verified offline against a synthetic fixture dataset and
  stub models (9/9 tests); the real dataset fetch is never called by a test.

All four tasks' own verify commands pass locally (pure Python, no cargo). No live model calls or spend, per
this wave's brief. Full regression checks along the way (families/f1_pyconv, families/f4_kvtool, specops/,
speclint/, the whole streams/ and external/swebench/ suites) all green.
