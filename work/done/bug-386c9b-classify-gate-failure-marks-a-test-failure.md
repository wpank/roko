+++
id = "bug-386c9b"
kind = "bug"
title = "classify_gate_failure marks a test failure only when the gate's name starts with \"test\", so Graph verify and rung failures classify as unknown"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-gate"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "1bf49188d"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-honestbench's report on gap-1a7f9c, branch work/gap-1a7f9c at 9ccdf916a)"
anchors = ["crates/roko-gate/src/compile_errors.rs::classify_gate_failure"]
lane = "rust-cold"
parent = "spec-6ac537"
links = { depends_on = [], blocks = [], related = ["gap-1a7f9c", "gap-ebd656"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn graph_step_names_classify_test_failures' crates/roko-gate/src/ && cargo test -p roko-gate --lib graph_step_names_classify_test_failures"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T09:00:22Z"
by = "coordinator (session 7622b882)"
forced = false
evidence = "Batch 20c gate on fcdaf32ae/ca5645373 (MAIN 1bf49188d has the same crates and portal): check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-acp/agent/cli/core/dreams/gate/learn/neuro/serve; lib tests roko-cli 3273, roko-agent 2278, roko-core 1962, roko-learn 1209, roko-serve 989, roko-gate 692, roko-neuro 239, roko-acp 199, roko-dreams 100 all pass; extras: C1 1/1, C7 2/2, learn_paths 7, cost_comparison 1, bin 429, verify loop 10/10, speclint 91, including graph_step_names_classify_test_failures. Merged 1bf49188d."
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
