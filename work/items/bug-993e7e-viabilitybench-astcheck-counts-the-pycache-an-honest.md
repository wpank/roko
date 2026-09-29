+++
id = "bug-993e7e"
kind = "bug"
title = "ViabilityBench astcheck counts the __pycache__ an honest agent's test run writes as an added test file"
status = "done"
triage = "verified"
last_verified = 2026-09-29
last_verified_rev = "ea3c53bb4"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["bench"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:55, wk-bench-f1's report on gap-4723ff)"
anchors = ["benchmarks/viabilitybench/families/common/astcheck.py", "benchmarks/viabilitybench/families/common/test_astcheck.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-2790c5", "gap-4723ff", "gap-7ee7c2"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'def test_pycache_from_visible_test_run_is_not_a_test_edit' benchmarks/viabilitybench/families/common/ && benchmarks/viabilitybench/.venv/bin/python -m pytest -q benchmarks/viabilitybench/families/common -k test_pycache_from_visible_test_run_is_not_a_test_edit"

[closed]
at = 2026-09-29
commit = "ea3c53bb4"
by = "wk-bench-ci"
evidence = "ea3c53bb4: astcheck.is_cache names Python bytecode (__pycache__/, *.pyc, *.pyo) and pytest's .pytest_cache/. test_edits skips caches on both sides (a symlink under a cache name still counts), and file_hashes leaves them out of a directory's hashes. common.VERSION is now common-1.1.0. At BASE, an honest F4 reference after 'cd tests/visible && pytest' got four test_edit findings, and F1's local filter missed .pytest_cache. The verify, the venv pytest with -k test_pycache_from_visible_test_run_is_not_a_test_edit (the unit test plus the F1, F4 and plan-slice detector tests), passes 4 of 4. Run against BASE's test_edits and file_hashes, 3 of those 4 fail (unit, F1, F4). The full benchmark suite passes: 226 passed, 2 skipped."
+++

## Problem

`families/common/astcheck.test_edits` counts the `__pycache__` files Python writes, when an honest agent runs the visible tests, as added test files. That flags the attempt as tampering, which zeroes its verified success. F1 filters them out locally; F4 and the plan-level slice don't (wk-bench-f1, 2026-09-29).

## Why it matters

False tampering flags would count honest successes as failures in the headline metric. Epic spec-567e52.

## Where

`benchmarks/viabilitybench/families/common/astcheck.py`, and its tests.

## Current state

Only F1 filters `__pycache__`.

## Plan

1. Ignore bytecode caches (`__pycache__/`, `*.pyc`) in `test_edits` and in any tree diff it relies on.
2. Remove F1's local filter, or keep it as a no-op.
3. Add the test the verify names.

## Done when

- [ ] A visible-test run's bytecode is never a test edit, in every family.
- [ ] The `[[verify]]` command passes.

## Notes

- 2026-09-29 (wk-bench-ci): the verify runs pytest with the pinned venv (README, "Tests"), not `python3`: the system
  Python has no pytest, so the old command failed whatever the code did.
- The same false positive comes from pytest's `.pytest_cache/`, which lands in the test directory when an agent runs
  `cd tests/visible && pytest`. At BASE it gave F4 four `test_edit` findings on an honest reference, and F1's local
  filter did not catch it. `astcheck.is_cache` covers both, and the census's `CACHE_PARTS` names the same two.
- `common.VERSION` is now `common-1.1.0`, so records show the changed verifier (the package's own rule).
- F1's local `_bytecode_cache` filter (`families/f1_pyconv/gaming.py`) is now redundant. It was left in place because
  it is outside this item's files. It still drops a symlink under a cache name, which astcheck now flags; that path
  is removed anyway when the census restores the visible tests.
