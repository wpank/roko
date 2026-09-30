+++
id = "gap-8bdf5e"
kind = "gap"
title = "model_swap needs served-model checks, in records and run_roko, that accept a declared swap"
status = "open"
triage = "unverified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-bench's report)"
anchors = ["benchmarks/viabilitybench/driver/disturb.py", "benchmarks/viabilitybench/driver/records.py", "benchmarks/viabilitybench/driver/run_roko.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-15bb83", "gap-dad97b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_a_declared_model_swap_passes_the_model_checks' benchmarks/viabilitybench/driver/test_run_roko.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_roko.py -k test_a_declared_model_swap_passes_the_model_checks -q"
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
