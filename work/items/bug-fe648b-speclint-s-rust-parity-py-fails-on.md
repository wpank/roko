+++
id = "bug-fe648b"
kind = "bug"
title = "speclint's rust_parity.py fails on main: the dynamic fixture's flaky step loses SPECLINT_FLAKY_COUNTER to the check environment, and roko scores no task of two plans that speclint scores"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-gate/spec-quality"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "round-v7-roko-planner"
discovered_from = "agent:roko-planner (round v7, 2026-10-19, parity check of gap-d21da9)"
anchors = ["benchmarks/viabilitybench/speclint/rust_parity.py::dynamic_records", "crates/roko-cli/src/spec_red_on_base.rs::run_step"]
links = { depends_on = [], blocks = [], related = ["gap-d21da9"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 benchmarks/viabilitybench/speclint/rust_parity.py --roko \"$PWD/target/debug/roko\""
+++

## Problem

`benchmarks/viabilitybench/speclint/rust_parity.py`, the check that roko's spec-quality port and red-on-base check
agree with speclint, fails on main. Run 2026-10-19 by roko-planner (round v7) with the roko of main at 3b34aed92
(`--roko <absolute path>`; a relative path fails, since the dynamic part runs roko from a scratch workspace):

```
rust parity (sq-3): 2 plan validate runs, 133 files, 549 tasks compared
  score: max |delta| 0.00 (tolerance 0.5)
  failures (score, hard fails, task ids, parse errors): 2
    plans/_fixtures/tss-v1-example/tasks.toml: task ids differ: roko [], speclint ['T1']
    plans/demos/_cli/demo-resume-recovery/tasks.toml: task ids differ: roko [], speclint ['RESUME-1', 'RESUME-2']
  dynamic fixture (red on base): 8 tasks, 3 differences
    tasks.toml T3: red_on_base 'fail' vs 'unknown'
    tasks.toml T3: outcome 'fail' vs 'flaky'
    tasks.toml T3: SQ06 1.0 vs 0
FAIL
```

The output is the same with the build of bug-27f00b/gap-d21da9, so neither change caused it.

## Why it matters

The parity check is how a change to the spec-quality rules or the red-on-base check is held to speclint, the
reference the papers' figures come from. While it fails for these old reasons, a new difference hides among them.

## Where

- Dynamic T3: the fixture's flaky step reads `SPECLINT_FLAKY_COUNTER`, which `rust_parity.py` sets in roko's
  environment. Since bug-3c4600 every red-on-base step runs in the check environment
  (`spec_red_on_base.rs::run_step`, `roko_gate::inherit_gate_env_std`), which drops it, so the step fails the same way
  in both runs (`: "${SPECLINT_FLAKY_COUNTER:?…}"`) and is `fail`, not `flaky`.
- Task ids: roko scores no task of `plans/_fixtures/tss-v1-example` or `plans/demos/_cli/demo-resume-recovery`, while
  speclint scores them; `rust_parity.py::validated_files` and roko's `collect_tasks_files` disagree about which
  `tasks.toml` files a run covers, or roko does not parse them.

## Plan

1. Make the fixture's flaky state reach the step another way (a file at a path inside the scratch dir the fixture names,
   or `[gates] env_passthrough` in the fixture's `roko.toml`), in both speclint's `tests/test_dynamic.py` and
   `rust_parity.py`.
2. Find why roko leaves out the two plans' tasks and make the tools agree on the file set.

## Done when

`python3 benchmarks/viabilitybench/speclint/rust_parity.py --roko "$PWD/target/debug/roko"` prints no failures and
exits 0 on a fresh build of main.

## Notes

Found while checking that round v7's red-on-base change (gap-d21da9) kept parity: it did, as the old binary shows the
same three differences.
