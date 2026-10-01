+++
id = "gap-44632a"
kind = "gap"
title = "No CI workflow runs the ViabilityBench verifier CI in benchmarks/viabilitybench/ci/"
status = "done"
triage = "verified"
last_verified = 2026-09-30
last_verified_rev = "8640a17f9"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/ci", "ci/workflows"]
created = 2026-09-29
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-rp-appAB's report)"
anchors = ["benchmarks/viabilitybench/ci/verify_verifiers.py", ".github/workflows/"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-7ee7c2", "find-8cc7ac"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqE 'viabilitybench/ci|verify_verifiers' .github/workflows/"

[closed]
at = 2026-09-30
commit = "8640a17f9"
by = "wk-bench-fix2"
evidence = "New workflow .github/workflows/viabilitybench-ci.yml (no existing workflow edited): on pushes to main and pull requests touching benchmarks/viabilitybench/** it makes the hash-locked venv on pinned Python 3.12.8 and runs verify_verifiers.py on F1/F4 levels 1-5 seeds 1-2 (20 cells), the plan slice (pl, seeds 3) and ci/test_ci.py; permissions contents: read, no secrets, offline. The [[verify]] command passes (grep finds verify_verifiers in .github/workflows/); the YAML parses (PyYAML, structural checks; actionlint is not installed); every step passed locally: 20/20, 18/18, 6 passed."
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

- [x] Every change under `benchmarks/viabilitybench/` runs the verifier CI.
- [x] The `[[verify]]` command passes.

## Notes

- **Done 2026-09-30 (wk-bench-fix2).** It is a new workflow, `.github/workflows/viabilitybench-ci.yml`, and no
  existing one was edited.
  - **Triggers.** It runs on pushes to main and on pull requests that touch `benchmarks/viabilitybench/**` or the
    workflow itself.
  - **Setup.** `permissions: contents: read`, and no repository secret: the CI makes its own throwaway secret.
    Python is pinned to 3.12.8, and the venv comes from `requirements.lock` with hashes.
  - **Steps.** The README's quick verifier CI:
    - `verify_verifiers.py --families f1,f4 --levels 1-5 --seeds 2` (20 cells, which exercise `determinism.compare`
      and the leak scan);
    - `--families pl --seeds 3`;
    - `pytest ci/test_ci.py`.
  - **Local check.** Every step passed locally (20/20, 18/18, 6 passed; about 100 s together). actionlint isn't
    installed, so the YAML was checked with PyYAML plus a structural check of triggers, permissions, actions and
    steps.
- **Left out:**
  - The 100-cell run (S08 §7.1) stays manual, before a pilot, as `ci/README.md` now says.
  - So does the full benchmark suite: it has never run on Linux, and several of its tests exercise
    platform-specific isolation (`ps -E`, `sandbox-exec`, the tripwire's ctime).
