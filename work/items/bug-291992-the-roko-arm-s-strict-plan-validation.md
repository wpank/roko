+++
id = "bug-291992"
kind = "bug"
title = "The Roko arm's strict plan validation ends 10 of the 20 pilot instances infra_error on PLAN_TIER_SIZE: their specs run past the focused tier's 350 words"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
source = "gap-327242 worker (claude-agent), 2026-10-20: Pilot B's first roko-fixed-s1 unit"
discovered_from = "gap-327242"
anchors = ["benchmarks/viabilitybench/driver/run_roko.py::advisory_only", "benchmarks/viabilitybench/driver/test_run_roko.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-327242", "gap-dd670b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'def test_a_validation_that_failed_only_on_the_tier_size_advice_does_not_fail_the_task' benchmarks/viabilitybench/driver/test_run_roko.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_roko.py -q -p no:cacheprovider -k 'tier_size'"
+++

## Problem

The Roko arm's runner (`benchmarks/viabilitybench/driver/run_roko.py`, step 2) runs `roko plan validate --strict --dag`
on every task's emitted plan and ends the task `infra_error` when it fails. Since afd17b79e (2026-09-30, gap-1d1fa6),
`plan validate` warns about a task over its tier's size limits (`PLAN_TIER_SIZE`), and `--strict` turns that warning
into a failure. A focused task may have at most 350 words of description, and planemit makes the task's description
the spec text every arm gets.

10 of the 20 pilot instances are longer than that, so `roko_fixed` (tier `focused`) could not run them:

| instances | words | smallest tier that fits |
|---|---|---|
| F1-l1-0001/0002, F1-l2-0001/0002 (k_doc `spelled_out` since 2775a1b52) | 380 | integrative |
| F1-l5-0001/0002 | 371 | integrative |
| F4-l1-0001/0002, F4-l2-0001/0002 (the `## kvtool` section since 2775a1b52) | 458 | integrative |
| F1-l3, F1-l4 | 312-314 | focused |
| F4-l3, F4-l4, F4-l5 | 262-266 | mechanical |

Seen live in Pilot B (gap-327242) on 2026-10-20: the first `roko-fixed-s1` unit at c962955af ended F1-l2-0001,
F1-l5-0001, F1-l5-0002 and F4-l1-0002 `infra_error` ("PLAN_TIER_SIZE focused task is over its tier's size limits:
380-word description (limit 350)") before any model call. The unit was stopped after 7 tasks and moved to
`~/.roko-bench/viability/PILOT-B.abandoned/roko-fixed-s1-1/` ($0.1463 billed on BL0 for its 3 completed tasks).

Nothing caught it earlier: the offline rehearsals (`test_pilot_b.py`, `test_shakedown.py`) use a fake roko, the
`real_roko` tests use the toy family's short spec, and `run_roko.preflight` validates a short stand-in task.

## Why it matters

Half of the Roko arm's pilot runs would be `infra_error`, which the report excludes, so the arm would have no data on
F1 ℓ1, ℓ2 and ℓ5 or F4 ℓ1 and ℓ2, the cells G0's ladder checks read. The same holds for every pinned Roko arm (LOG1
block A) on any family whose specs pass 350 words.

## Where

- `benchmarks/viabilitybench/driver/run_roko.py`: `run_task` step 2, `ADVISORY_RULES`, `advisory_only`.
- `crates/roko-cli/src/plan_policy.rs::TierSizeLimits::for_tier` (the limits) and `plan_validate.rs` (the warning).

## Current state

Fixed on `work/gap-327242`. The runner now runs `plan validate --strict --dag --json`. A failure whose report holds
no error and only `PLAN_TIER_SIZE` warnings no longer fails the task; the transcript keeps those warnings as a
`validate_advisory` event. Any other warning or error still ends the task `infra_error`.

Why tolerate the warning rather than raise the arm's tier to `integrative`: `plan run` never checks tier sizes, so
Roko runs exactly the plan it ran before; a benchmark task's description is the spec every arm gets, which the arm
may neither shorten nor split; and the tier changes Roko's behaviour (the complexity band, `integrative` → Complex,
and the per-tier pipeline table), which would change the arm.

## Plan

1. Done: tolerate `PLAN_TIER_SIZE` alone in the runner's per-task validation, with tests.
2. If the author prefers the other reading (the task is "too big for focused", so the arm should run it as
   integrative), change `[roko] tier` per arm instead and drop the tolerance; either way, say which in S08 §4.9.

## Done when

- [x] A pilot instance whose spec is over 350 words runs through `plan run` on the Roko arm; the `[[verify]]` tests
      (a unit test of `advisory_only`, and the real binary failing a 400-word spec on `PLAN_TIER_SIZE` alone) pass.

## Notes

- `run_roko.preflight` still validates the short stand-in task in text mode; it never meets this warning.
- An offline `plan run` of F4-l1-0001 (458 words) against the tests' ToolStub at c962955af ran its three attempts on
  gpt-oss-120b only, with no tier-size refusal.
