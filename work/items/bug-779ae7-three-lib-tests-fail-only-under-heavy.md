+++
id = "bug-779ae7"
kind = "bug"
title = "Three lib tests fail only under heavy load: a roko-gate tautology-filter test and two dispatcher timing tests"
status = "open"
triage = "unverified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["tests"]
created = 2026-09-29
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (21:10, coordinator batch gates 2, 5 and 7)"
anchors = ["crates/roko-gate/src/generated.rs", "crates/roko-cli/src/graph_task_dispatch/turn_policy.rs", "crates/roko-cli/src/graph_task_dispatch/verification.rs"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "for i in $(seq 1 20); do cargo test -p roko-gate --lib generated::tests::tautology_filter_discards_preexisting_passing_tests -- --test-threads=32 >/dev/null 2>&1 || exit 1; done"
+++

## Problem

Three lib tests fail only when the machine is loaded, and pass alone or on rerun:

- roko-gate's `generated::tests::tautology_filter_discards_preexisting_passing_tests` failed in one batch-7 run and passed in the next.
- roko-cli's `graph_task_dispatch::turn_policy::tests::a_timed_out_attempt_is_resumed_with_an_escalated_timeout` depends on a 1 s task timeout.
- roko-cli's `graph_task_dispatch::verification::tests::verified_outcome_drives_output_verdict_and_feedback` waits at most 60 s for background efficiency writers.

The coordinator's batch gates saw each of these fail at least once on 2026-09-29, with load averages between 20 and 99.

## Why it matters

Flaky tests hide real regressions and make CI red at random. Epic spec-9a3131.

## Where

The three tests named above.

## Current state

They pass alone and fail under full parallel load.

## Plan

1. Remove the dependence on wall-clock time: inject clocks and deadlines, or wait on a signal instead of polling a file.
2. For the tautology filter, find what it waits on (a subprocess build?) and make that deterministic.
3. Prove it by running each test 20 times under `--test-threads=32`.

## Done when

- [ ] Each test passes 20 times in a row under heavy parallel load.
- [ ] The `[[verify]]` command passes.

## Notes

- 2026-09-30 (wk-tiers, on work/gap-8c0a20): two more tests fail under load and pass alone, the sibling-verify pair that bug-ea9959 is about: `a_verify_failure_left_in_a_sibling_file_blames_the_sibling` and `a_verify_failure_beside_an_editing_sibling_is_rerun_once_it_settles` (both use `dispatch_beside_editing_sibling`, `graph_task_dispatch/verification.rs`).
- 2026-09-30 (wave 12): two more flakes. `a_timed_out_attempt_reports_the_usage_it_streamed` (roko-agent) failed at load 77 and passes alone. The batch-11 sibling tests race (`a_verify_failure_beside_an_editing_sibling_is_rerun_once_it_settles` and `a_verify_failure_left_in_a_sibling_file_blames_the_sibling`). wk-model-truth found the cause: the fake sibling unregisters as soon as `failed-once` exists, but the failing verify creates that file itself, so if the gate takes more than about 20 ms, `begin_settle` finds no writer. The sibling should wait until the settle has begun.
