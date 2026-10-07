+++
id = "bug-ef82eb"
kind = "bug"
title = "After a blank answer, the retried attempt ends gate_failed instead of completed (shakedown D1)"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-cli/graph-task-dispatch"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "425e256d5"
source = "PK36 shakedown via gap-821c93 (2026-10-03)"
discovered_from = "gap-821c93"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs", "crates/roko-cli/src/runner/gate_dispatch.rs", "benchmarks/viabilitybench/driver/test_shakedown.py"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-31c0e8", "gap-821c93"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn retry_after_a_blank_answer_completes' crates/roko-cli/src/ && cargo test -p roko-cli retry_after_a_blank_answer_completes"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T08:36:03Z"
commit = "425e256d5"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-03T07:38:41Z"
forced = false
evidence = "Gate 8a (work/backlog-batch-8a, merged into main as 425e256d5): cargo check --workspace --tests, nightly fmt, cargo clippy --workspace -D warnings, nextest --lib 7,831 tests over roko-cli, -core, -learn and -serve (one MCP tool-count assertion fixed in 99b7b19a9), roko-cli bin 430 passed and the golden-path canaries pass (plan_validate: only bug-2a31bc's two known alias tests fail); every [[verify]] passes. The premise was false: roko's retry after a blank answer is sound. The shakedown's D1 stub wrote calc/ops.py without reading it, which RequireToolBeforeEdit refuses, so every attempt changed nothing. The stub now reads first (D1 passes; the shakedown is 6/8 with the batch binary, D3/D7 under bug-0b7695), and failover::tests::retry_after_a_blank_answer_completes pins the recovery. The feedback gap it exposed is gap-2e455d."
+++

## Problem

After a blank answer, the retried attempt ends `gate_failed` instead of `completed`. PK36's shakedown (task 3314,
`benchmarks/viabilitybench/driver/test_shakedown.py`, scenario D1), run on 2026-10-03 against a binary built from
main at 823f2cfca with `VB_REQUIRE_REAL_ROKO=1`: the stub answers the first call blank and the next ones properly.
The blank-answer isolation fix holds (at least two attempts run after the blank reply), but the task's final verdict
is `gate_failed` (4 turns, $0.0024 metered), where the scenario expects `completed`.

## Why it matters

A blank answer is a common cheap-model failure (live-run defect D1). If the retry that recovers from it still ends
gate-failed, the cheap arms look worse than they are and the ladder climbs for nothing.

## Where

Not yet localised. Candidates: the gate re-run for a retried attempt (`crates/roko-cli/src/runner/gate_dispatch.rs`,
`run_gate_once`) and the retry loop in `crates/roko-cli/src/graph_task_dispatch.rs` (what the retry inherits from
the blank attempt: its workdir state, gate feedback, or a stale verdict).

## Current state

D2, D4, D5, D6 and D10 pass on the same binary; D3 and D7 fail on a different cause (bug filed alongside).

## Plan

1. Reproduce with the shakedown's D1 (or a Rust test with a fake provider that answers blank once, then properly)
   and read the second attempt's gate input and output.
2. Fix the cause; add the Rust test.

## Done when

- [ ] The `[[verify]]` command passes.
- [ ] Shakedown D1 passes against a binary built from the fix.

## Notes

- Found by gap-821c93's run of the shakedown (w3-pk21, 2026-10-03).

## Progress

- bug-ef82eb: implemented on work/bug-ef82eb at b67a1428d; cargo verification deferred to the batch gate. Cause: D1's stub wrote `calc/ops.py`, which the base holds, without reading it, and the implementer contract (`RequireToolBeforeEdit`) refused that write on every attempt, so each ended `pre_verify:no_changes`; D6 shows the same refusal with no blank answer. The retry after a blank answer inherits nothing from it. D1's stub now reads the file first: D1 passes against main's binary (823f2cfca), with 6 of 8 shakedown scenarios passing (D3 and D7 are bug-0b7695). Rust test: `graph_task_dispatch::failover::tests::retry_after_a_blank_answer_completes`.
