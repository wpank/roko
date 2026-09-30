+++
id = "gap-8bdf5e"
kind = "gap"
title = "model_swap needs served-model checks, in records and run_roko, that accept a declared swap"
status = "done"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "a89cc55f6"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-bench's report)"
anchors = ["benchmarks/viabilitybench/driver/disturb.py", "benchmarks/viabilitybench/driver/records.py", "benchmarks/viabilitybench/driver/run_roko.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-15bb83", "gap-dad97b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_a_declared_model_swap_passes_the_model_checks' benchmarks/viabilitybench/driver/test_run_roko.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_roko.py -k test_a_declared_model_swap_passes_the_model_checks -q"

[closed]
at = 2026-09-30
commit = "a89cc55f6"
by = "wk-bench-fix1"
evidence = "a89cc55f6: model_swap is built. For covered tasks the metering proxy sends params.to upstream in place of the pin and logs model_swap; TaskContext.model_swap declares it; records.final_status and run_roko's checks accept exactly that served model (Roko's model_mismatch mark included) and mark the attempts it served model_swapped; an undeclared swap, another model, or Roko's failover stays model_mismatch/infra_error; vb run refuses a swap without --proxy or without a price row for the model. Verify passes (test_a_declared_model_swap_passes_the_model_checks: settle with and without the declaration, the proxy's swapped rows, records.final_status, and an end-to-end vb run through the proxy with a Roko stand-in that records the served model: completed, model_reported glm-4.7, model_swapped). Also test_a_declared_model_swap_runs_on_the_direct_arm (pin at position 1, swap at 2). The Claude Code arm cannot go through the proxy, so it cannot run a swap. Full bench suite: 370 passed, 2 skipped."
+++

## Problem

The `model_swap` disturbance would have the proxy rewrite a request's model. But the driver's model checks (`records.py`'s `same_model`, `run_roko.py`'s `model_mismatch`) then mark every swapped task `infra_error` (`driver/disturb.py:31`, :62), so the disturbance is refused.

## Why it matters

Pilot benchmark (epic spec-567e52): H6 can't test how the arms handle a silent model change. p3.

## Where

The model checks in `records.py` and `run_roko.py` (see gap-dad97b for the new record fields they will read), and `disturb.py`.

## Plan

1. Let a run declare its swap (from the disturbance manifest). The checks then accept a served model that matches the declared swap, and record it as `model_swapped`, not `model_mismatch`.
2. Add `test_a_declared_model_swap_passes_the_model_checks`.

## Done when

- [ ] A declared swap runs and is recorded as a swap. An undeclared one is still `infra_error`.
- [ ] The `[[verify]]` command passes.
