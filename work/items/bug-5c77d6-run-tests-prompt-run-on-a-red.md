+++
id = "bug-5c77d6"
kind = "bug"
title = "run::tests::prompt_run_on_a_red_base_judges_only_new_failures fails under load in the full roko-cli lib suite (a red base's own failure is judged Unverified, as if the attempt touched no test) and passes alone"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/baseline-verify"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
source = "round-v7-roko-planner#flake"
discovered_from = "agent:roko-planner (round v7, bug-daa54e full suite)"
anchors = ["crates/roko-cli/src/run.rs::prompt_run_on_a_red_base_judges_only_new_failures", "crates/roko-cli/src/graph_task_dispatch/baseline_verify.rs::judge"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

`run::tests::prompt_run_on_a_red_base_judges_only_new_failures` failed once in a full `cargo test -p roko-cli --lib`
run and passed when run alone straight after.

- The full run: branch work/bug-daa54e (5de243ec8 plus a planner change that touches neither `run.rs` nor
  `graph_task_dispatch/baseline_verify.rs`), through heavy.sh (`RUST_TEST_THREADS=4`), machine load about 100. Result:
  3895 passed, 1 failed, in 270 s.
- Alone, on the same build: 1 passed in 9.1 s.

Case 2 failed. The fake agent writes a syntax error into `synth/env.py`, and the test expects the settlement "failed
before this run too, but this task's to fix: test_env". Instead the step's output ends with:

```
[roko] 4 tests already failed before the change, and no other test ran and passed, so nothing verifies the change: add or edit a test that exercises it
```

So `judge` (baseline_verify.rs) returned `Judgement::Unverified`: `task_owns("test_env")` was false. Ownership comes
from `concerns_task(test, &text, &task.files)` or from `touched`, the test files among the attempt's changed files
(`self.changed_files(attempt_key)`). Under load, one of those was probably empty, but that hasn't been proven.

## Why it matters

A red flake in the roko-cli lib suite costs every agent a rerun and a doubt. The judgement it covers matters too: if
the attempt's changed files can be missed under load, a real run on a red base could read "Unverified" instead of
"this task's to fix".

## Where

- `crates/roko-cli/src/run.rs`: the test (case 2).
- `crates/roko-cli/src/graph_task_dispatch/baseline_verify.rs`: `judge`, `concerns_task`, `changed_files`, `touched_tests`.

## Plan

1. Reproduce: run `run::tests` with `--test-threads 8` (or the whole suite) a few times, and log `changed` and `touched`
   for case 2.
2. If the changed files come from git, check whether another test changes the process's cwd or env (GIT_DIR,
   GIT_WORK_TREE) at the same time.
3. Fix the cause. If it's the test's own isolation, fix that.

## Done when

The test passes in the full suite, run three times in a row under heavy.sh.

## Notes

- Evidence: `tmp/hermes-roko-v7/roko-planner/flake-prompt-run-red-base.log` (the panic, the step output and the passing
  rerun).
- Found by roko-planner (round v7) while testing bug-daa54e; filed at the lead's request. No round-v7 agent owns
  baseline_verify.
- 2026-10-26 (roko-learnguard, the full lib suite of bug-cdd885 on main c39ddd6fb, heavy.sh, `RUST_TEST_THREADS=4`):
  - **It failed again, at another assertion.** `run.rs:1618`, "the prompt names the old failures": the fake agent's
    recorded prompt lacked the base-check note.
  - **The run:** 4044 passed, 1 failed.
  - **The same test binary, alone, passed** in 4.6 s.
  - **The log** is `tmp/hermes-roko-v7/roko-learnguard/logs/cdd885-chain.log`.
  - **So two different assertions fail under load.** That points at the test's shared state or its timing more than
    at `judge`.
- 2026-10-26 (roko-speed, chain49: gap-dc24dd's full lib suites on main f5045c0e6, heavy.sh, `RUST_TEST_THREADS=4`):
  - **It failed again, as first reported.** Case 2 ended `Unverified` ("4 tests already failed before the change,
    and no other test ran and passed") instead of "this task's to fix: test_env".
  - **The run:** 4044 passed, 1 failed. The change under test adds only a `cold_verify: false` field to a
    `TaskMeta` literal in `run.rs` and leaves `baseline_verify.rs` alone.
  - **The same build, alone, passed twice** (4.5 s and 4.2 s; `tmp/hermes-roko-v7/roko-speed/logs/chain49b.log`).
  - **The log** is `tmp/hermes-roko-v7/roko-speed/logs/chain49.log.tests`.
