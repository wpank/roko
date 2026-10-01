+++
id = "bug-a05c53"
kind = "bug"
title = "ViabilityBench's Roko arm fails plan validate --strict on PLAN_041 since gap-dbf2a6, so every Roko-arm task ends infra_error"
status = "open"
triage = "unverified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-tamper's report, 2026-10-01, real_roko tests run against the batch binary at 1ce6526f8)"
anchors = ["benchmarks/viabilitybench/driver/planemit.py", "benchmarks/viabilitybench/driver/run_roko.py", "crates/roko-cli/src/plan_validate.rs"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-c33709", "gap-327242"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -x target/debug/roko && cd benchmarks/viabilitybench && ROKO_BIN=$PWD/../../target/debug/roko .venv/bin/python -m pytest -q -p no:cacheprovider driver/test_run_roko.py -k real_roko -rs"
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
