+++
id = "bug-bca443"
kind = "bug"
title = "In roko bench swe's proxy scoring, a model patch can edit the tests that grade it"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-cli/bench"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "0b84bc9fa"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-honestbench's report, checked on work/bug-28becc at abb181f65)"
anchors = ["crates/roko-cli/src/bench.rs"]
lane = "rust-cold"
links = { depends_on = ["bug-28becc"], blocks = [], related = ["bug-28becc"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn swe_scoring_restores_the_grading_tests' crates/roko-cli/src/ && cargo test -p roko-cli --lib swe_scoring_restores_the_grading_tests"
+++

## Problem

SWE-bench grades a patch by applying the task's `test_patch` over the model's patch, so the model can't change the tests that grade it. roko's swe proxy scoring, on bug-28becc's branch, has no `test_patch` handling anywhere in `roko-cli` or `roko-serve`. A model patch that edits or deletes the grading tests is scored against its own edits.

## Why it matters

Release: a pass the model awarded itself isn't a pass.

## Where

The swe scoring path in `crates/roko-cli/src/bench.rs`.

## Plan

1. Before scoring, restore or apply the grading tests (`test_patch`, or the listed test files from the base commit) over the model's patch. Record whether the patch touched them.
2. Add `swe_scoring_restores_the_grading_tests`.

## Done when

- [ ] Grading always runs the task's own tests, whatever the patch did to them.
- [ ] The `[[verify]]` command passes.

## Notes

- **wk-honestbench (2026-09-30):** Implemented on `work/bug-32d57f` at `0152e1e67`; cargo verification deferred to the batch check. Custom datasets must now name their grading tests (`test_files` and/or `test_patch`); `demo/demo-resources/benchmark-flow/README.md` documents it. The two swe tests need `python3`.
