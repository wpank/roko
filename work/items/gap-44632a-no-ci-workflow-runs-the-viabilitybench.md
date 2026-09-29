+++
id = "gap-44632a"
kind = "gap"
title = "No CI workflow runs the ViabilityBench verifier CI in benchmarks/viabilitybench/ci/"
status = "open"
triage = "unverified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/ci", "ci/workflows"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-rp-appAB's report)"
anchors = ["benchmarks/viabilitybench/ci/verify_verifiers.py", ".github/workflows/"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-7ee7c2", "find-8cc7ac"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqE 'viabilitybench/ci|verify_verifiers' .github/workflows/"
+++

## Problem

gap-7ee7c2 built the verifier CI (`benchmarks/viabilitybench/ci/`: `verify_verifiers.py`, `determinism.py`, `leak_check.py`, `test_ci.py`), but no workflow in `.github/workflows/` runs it. At d2cc43346 nothing there references `viabilitybench/ci` or `verify_verifiers`. So a change that breaks a family's reference, stub or gaming checks, or makes a generator non-deterministic, reaches the pilot unnoticed.

The report's other point, that the README still calls gap-308373 open, no longer holds at d2cc43346: README.md:111-118 describe the tripwire as built.

## Why it matters

Pilot benchmark (epic spec-567e52): S08 requires the verifier CI before every run (T5). p3, because it can be run by hand today.

## Where

`benchmarks/viabilitybench/ci/` and `.github/workflows/`.

## Plan

1. Add a workflow, or a job in an existing one, that sets up the benchmark's venv and runs `verify_verifiers.py`, `determinism.py`, `leak_check.py` and the pytest suite on changes under `benchmarks/viabilitybench/`.
2. Keep it offline: no provider calls.

## Done when

- [ ] Every change under `benchmarks/viabilitybench/` runs the verifier CI.
- [ ] The `[[verify]]` command passes.
