+++
id = "bug-c1b845"
kind = "bug"
title = "speclint --dynamic runs only authored verify steps on the base, so SQ06 and HF3 ignore pinned acceptance tests"
status = "done"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["benchmarks/viabilitybench/speclint"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "e12249d1e"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-specq's report)"
anchors = ["benchmarks/viabilitybench/speclint/dynamic.py"]
lane = "rust-cold"
parent = "spec-e57870"
links = { depends_on = [], blocks = [], related = ["bug-019f02", "gap-d14a43"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_dynamic_mode_runs_pinned_accept_tests' benchmarks/viabilitybench/speclint/tests/test_dynamic.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/speclint/tests/test_dynamic.py -k test_dynamic_mode_runs_pinned_accept_tests -q"

[closed]
at = 2026-09-30
commit = "e12249d1e"
by = "wk-specq"
evidence = "speclint/dynamic.py runs each well-formed [task.accept] test first, as the Graph path's pinned step does (copy src over dest, run runner, exactly count passes via the same awk), and HF3 treats pinned tests as expecting red. Verify passes: test_dynamic_mode_runs_pinned_accept_tests (red accept-only task, HF3 for an accept test green on the base, exact-count failure); it failed on BASE 0b84bc9fa. speclint tests: 90 pass."
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

## Notes

- 2026-09-30: the verify command named `speclint/test_speclint.py`, which does not exist. The test lives with
  the other dynamic-mode tests in `speclint/tests/test_dynamic.py`, and the command now points there.
- `dynamic.py` runs each well-formed `[task.accept]` test first, as the Graph path's pinned step does: it copies
  the plan's `src` over `dest` (the base has no pin store and no hash to check), runs `runner`, and requires
  exactly `count` passes, counted by a verbatim copy of `task_accept.rs` `PASSED_COUNT_AWK`
  (`test_passed_count_awk_matches_task_accept` guards the copy).
- In `speclint.py`, HF3 now counts a task with a pinned test as expecting red, even when every one of its own
  steps declares `pass_on_base`. Static records are unchanged. The Rust port's `hard_fails` needs the same one
  line (`task.accept_tests > 0`) before it runs a dynamic mode; it has none today.
