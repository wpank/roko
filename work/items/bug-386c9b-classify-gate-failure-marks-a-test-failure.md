+++
id = "bug-386c9b"
kind = "bug"
title = "classify_gate_failure marks a test failure only when the gate's name starts with \"test\", so Graph verify and rung failures classify as unknown"
status = "open"
triage = "unverified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-gate"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-honestbench's report on gap-1a7f9c, branch work/gap-1a7f9c at 9ccdf916a)"
anchors = ["crates/roko-gate/src/compile_errors.rs::classify_gate_failure"]
lane = "rust-cold"
parent = "spec-6ac537"
links = { depends_on = [], blocks = [], related = ["gap-1a7f9c", "gap-ebd656"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn graph_step_names_classify_test_failures' crates/roko-gate/src/ && cargo test -p roko-gate --lib graph_step_names_classify_test_failures"
+++

## Problem

`classify_gate_failure` (`crates/roko-gate/src/compile_errors.rs`, about line 533) adds `TestExpectationFailure` only when `gate.starts_with("test")`. Graph verify steps are named `verify[i:test]` or `rung[test]`, and W13 records use `graph-verify`, so a failing test on the Graph path classifies as `unknown` (wk-honestbench).

## Why it matters

The failure class feeds the conductor's watchers (gap-1a7f9c wires them to Graph gate verdicts), the retry context and learning. With the class lost, test-failure-budget and the learners see an unknown failure instead of a failing test.

## Plan

1. Recognise a test step by its phase or kind rather than a name prefix: the Graph step's `test` phase (`verify[i:test]`, `rung[test]`) as well as names starting with `test`. Prefer passing the phase in over parsing labels, if the callers have it.
2. Add `graph_step_names_classify_test_failures`: a failing `cargo test` output under each of `test`, `verify[0:test]`, `rung[test]` and `graph-verify` with a test phase classifies as `TestExpectationFailure`, and a compile failure under the same names does not.

## Done when

- [ ] Graph test failures classify as `TestExpectationFailure`.
- [ ] The `[[verify]]` command passes.
