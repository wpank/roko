+++
id = "bug-a05c53"
kind = "bug"
title = "ViabilityBench's Roko arm fails plan validate --strict on PLAN_041 since gap-dbf2a6, so every Roko-arm task ends infra_error"
status = "done"
triage = "verified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "3c90c3151"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-tamper's report, 2026-10-01, real_roko tests run against the batch binary at 1ce6526f8)"
anchors = ["benchmarks/viabilitybench/driver/planemit.py", "benchmarks/viabilitybench/driver/run_roko.py", "crates/roko-cli/src/plan_validate.rs"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-c33709", "gap-327242"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -x target/debug/roko && cd benchmarks/viabilitybench && VB_TEST_ROKO_BIN=$PWD/../../target/debug/roko .venv/bin/python -m pytest -q -p no:cacheprovider driver/test_run_roko.py -k 'real_roko or refuses_a_binary' -rs"

[closed]
at = 2026-10-01
commit = "b167a45d8"
evidence = "The emitted roko.toml turns the routing ladder off ([routing.ladder] enabled = false, planemit-2), so PLAN_041 does not apply and plan validate --strict passes with the pin unchanged. vb run calls the runner's preflight before the first task: the Roko arm validates a stand-in plan with its binary and refuses the run if it is rejected. The real_roko skip reason is explicit and the documented test commands pass -rs. Verify: against a cp -c of roko-batch-target/debug/roko (git fcdaf32ae), the 3 real_roko tests and the refusal test pass (4 passed); the full bench suite passed 371, skipped 2."
+++

## Problem

`planemit.py` writes `model_hint = <pinned model>` into every task of the Roko arm's plan. Since 5b8efa148 (gap-dbf2a6), `plan validate` reports a task that pins a model while its routing ladder resolves as warning PLAN_041 ("pins a model and bypasses the routing ladder; use `rung`"). `run_roko.py` runs `roko plan validate --strict --dag`, where a warning fails validation (exit 1), and treats that as `infra_error`. So no Roko-arm task reaches `plan run` (wk-tamper: all three `real_roko` tests in `driver/test_run_roko.py` fail at validate with PLAN_041 against a binary at 1ce6526f8).

The suite stayed green because those tests skip when no roko binary is present.

## Why it matters

The pilots (gap-c33709, gap-327242) run the Roko arm. As it stands, every Roko-arm attempt would be recorded as an infrastructure error, and the pilot would compare nothing.

## Where

- `benchmarks/viabilitybench/driver/planemit.py`: the emitted plan's `model_hint` and the emitted `roko.toml`.
- `benchmarks/viabilitybench/driver/run_roko.py`: the `plan validate --strict --dag` step (about line 241).
- PLAN_041 in `crates/roko-cli/src/plan_validate.rs` fires only when `ladder.resolve(role, tier, …)` finds a ladder for the task.

## Plan

1. The fixed-model arm should not use the ladder at all. Prefer emitting a `roko.toml` whose routing ladder is empty or disabled for the arm, so that the pin is the only routing input and PLAN_041 does not apply. Check that the pinned model still reaches every dispatch, which the driver's model check already verifies.
2. If the config cannot express "no ladder", choose between emitting a `rung` hint whose ladder maps to the pinned model, and exempting PLAN_041 for the arm, and record which in the notes.
3. Make a missing binary visible: the `real_roko` tests should report a skip reason, and the pilot's preflight should refuse to start the Roko arm when the binary fails `plan validate` on an emitted plan.

## Done when

- [ ] The three `real_roko` tests pass against a current binary.
- [ ] The `[[verify]]` command passes.

## Notes

- **wk-tamper (2026-10-01):** The config can say "no ladder": the emitted `roko.toml` sets `[routing.ladder] enabled
  = false` (planemit-2), so the pin (`model_hint` and `--model`) is the only routing input and PLAN_041, which
  fires only when `ladder.resolve` finds a ladder, does not apply. `--strict` is unchanged, and no rule is exempted.
  The driver's model checks still pass on every record: the real-roko run test asserts the pinned model was
  dispatched, served and named, with no failed check.
- `vb run` calls a runner's optional `preflight` before the first task (harness.py). The Roko arm's validates a
  stand-in plan (`run_roko.PREFLIGHT_SPEC`) with the arm's binary and refuses the run when `plan validate --strict
  --dag` rejects it, instead of ending every task `infra_error`
  (`test_vb_run_refuses_a_binary_that_rejects_the_emitted_plan`).
- The `real_roko` skip reason now says the arm was not run against real Roko, and the documented test commands pass
  `-rs`. The `[[verify]]` now sets `VB_TEST_ROKO_BIN`, the variable the tests read (it said `ROKO_BIN`), and also
  runs the refusal test.
