+++
id = "bug-c1b845"
kind = "bug"
title = "speclint --dynamic runs only authored verify steps on the base, so SQ06 and HF3 ignore pinned acceptance tests"
status = "open"
triage = "unverified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["benchmarks/viabilitybench/speclint"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-specq's report)"
anchors = ["benchmarks/viabilitybench/speclint/dynamic.py"]
lane = "rust-cold"
parent = "spec-e57870"
links = { depends_on = [], blocks = [], related = ["bug-019f02", "gap-d14a43"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_dynamic_mode_runs_pinned_accept_tests' benchmarks/viabilitybench/speclint/test_speclint.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/speclint/test_speclint.py -k test_dynamic_mode_runs_pinned_accept_tests -q"
+++

## Problem

In `--dynamic` mode, `speclint/dynamic.py` runs a task's verify steps on the base commit, but takes them from `task.get("verify")` only (:355, :439). A task whose acceptance tests are pinned with `[task.accept]` (gap-d14a43) has those tests left out, so SQ06 and HF3, the dynamic checks that verify steps fail on the base and pass on the reference, never see them.

## Why it matters

Specs a cheap model can execute (epic spec-e57870): the dynamic lint misjudges exactly the plans that pin their tests. bug-019f02 covers the static lint. p3.

## Where

The step collection in `dynamic.py`.

## Plan

1. Include each `[task.accept]` test as a step, run the way the Graph path runs it.
2. Add `test_dynamic_mode_runs_pinned_accept_tests`.

## Done when

- [ ] `--dynamic` runs pinned acceptance tests too.
- [ ] The `[[verify]]` command passes.
