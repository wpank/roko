+++
id = "bug-993e7e"
kind = "bug"
title = "ViabilityBench astcheck counts the __pycache__ an honest agent's test run writes as an added test file"
status = "open"
triage = "unverified"
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
command = "grep -rqw 'def test_pycache_from_visible_test_run_is_not_a_test_edit' benchmarks/viabilitybench/families/common/ && cd benchmarks/viabilitybench && python3 -m pytest -q families/common -k test_pycache_from_visible_test_run_is_not_a_test_edit"
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
