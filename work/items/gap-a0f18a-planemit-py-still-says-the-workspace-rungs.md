+++
id = "gap-a0f18a"
kind = "gap"
title = "planemit.py still says the workspace rungs are inert, and plan validate doesn't list which rungs will run"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver", "roko-cli/plan_validate"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "b128de876"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-gates's report, checked on work/bug-5b43a9 at 7b25c478b)"
anchors = ["benchmarks/viabilitybench/driver/planemit.py", "crates/roko-cli/src/plan_validate.rs"]
lane = "rust-hot"
parent = "spec-e9d7ec"
links = { depends_on = ["bug-5b43a9"], blocks = [], related = ["gap-3506f1", "bug-5b43a9"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'as inert on the Graph engine' benchmarks/viabilitybench/driver/planemit.py && grep -rqw 'fn plan_validate_lists_the_rungs_that_will_run' crates/roko-cli/src/ && cargo test -p roko-cli --lib plan_validate_lists_the_rungs_that_will_run"
+++

## Problem

- `benchmarks/viabilitybench/driver/planemit.py`'s docstring (:15-18) says `plan run` runs only the authored `[[task.verify]]` steps, and that Roko reports `gates.rungs` as inert on the Graph engine. Once the gates branch lands, that is no longer true, and the Roko arm's emitted rungs will gate its tasks.
- `roko plan validate` doesn't show which workspace rungs will run for a plan, so a user can't see the gates before running.

## Why it matters

Honest verdicts (epic spec-e9d7ec): the benchmark and users should know which checks judge a task. p3.

## Where

`planemit.py`'s docstring, and `plan_validate.rs`'s report.

## Plan

1. Update the docstring (and check that the Roko arm's emitted rungs are what it wants gating its tasks).
2. Make `plan validate` list the rungs that will run (name, command, required), honouring the per-plan opt-out if one lands (gap-3506f1).
3. Add `plan_validate_lists_the_rungs_that_will_run`.

## Done when

- [ ] Both are true.
- [ ] The `[[verify]]` command passes.

## Notes

- 2026-09-30 (wk-gates): Implemented on `work/gap-3506f1b` at `1cb14e268`; cargo verification deferred to the batch check. The Roko arm's one rung is its visible check, the same command as its verify step, so plan run skips it as a duplicate and the check runs once per attempt, which is what the arm wants. `roko plan validate` prints the rungs after its diagnostics (name, command, and whether each runs), then the plans that opt out; `--json` adds `workspace_rungs` when the workspace declares any. The driver's `test_run_roko.py` passes (10 passed, 3 skipped that need a built roko).
