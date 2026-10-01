+++
id = "bug-779ae7"
kind = "bug"
title = "Three lib tests fail only under heavy load: a roko-gate tautology-filter test and two dispatcher timing tests"
status = "done"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["tests"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "bf40f3269"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (21:10, coordinator batch gates 2, 5 and 7)"
anchors = ["crates/roko-gate/src/generated.rs", "crates/roko-cli/src/graph_task_dispatch/turn_policy.rs", "crates/roko-cli/src/graph_task_dispatch/verification.rs"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "for i in $(seq 1 20); do cargo test -p roko-gate --lib generated::tests::tautology_filter_ -- --test-threads=32 >/dev/null 2>&1 || exit 1; done"

[closed]
at = 2026-10-01
commit = "db1cab1d0"
by = "coordinator (session 7622b882)"
evidence = "Merged in batch 19 (db1cab1d0, work/bug-779ae7 675b82910): one write per JSONL row, test-visible background writes, event-based timeouts, WAL unlock on drop, distinct probe crate names. Its verify loop ran in the batch-20b extras on cad1a56e1: the tautology_filter tests passed 20 of 20 at --test-threads=32 under load ~100-140. roko-cli lib ran clean in batch 19 (3254/3254) and batch 20b (3261/3261). --force only because work.py static_prefix treats the for-loop verify as static (bug-1440cd)."
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
- Implemented on `work/bug-779ae7` at `2c61e9053`; cargo verification deferred to the batch check. Targeted tests
  and the loops below ran in wk-specq's own target clone.
- 2026-09-30 (wk-specq): the causes, each fixed where it starts:
  - Tautology probe: both probe tests built a crate named `demo`, and the probe's nested cargo inherits
    `CARGO_TARGET_DIR`. In a shared target dir, cargo reuses a same-named crate built from another directory
    whenever this crate's sources are older than that build, so one probe ran the other test's binary. With a
    shared target dir, 5 of 10 runs of both tests failed; without one, 0 of 10. Two hand-made `demo` crates show
    cargo running the other crate's test binary without compiling. Each test now names its own crate. The verify
    now runs both probe tests, because one test alone cannot collide with itself.
  - Background rows: `background_writes::spawn` counts each spawned efficiency, cost, gate-failure and retrieval
    write until it ends, and the test helpers wait on `settled(dir)` before they read. Also,
    `append_jsonl_line_async` wrote a row and its newline in two calls, so rows appended at once interleaved and
    were lost. That, not slow writers, is why `verified_outcome_drives_output_verdict_and_feedback` and
    `gate_rows_carry_the_attempts_turns_or_unknown` sometimes never saw their rows. A 64-writer check lost rows in
    20 of 20 rounds with split writes and 0 of 20 with one write per row. Each row is now one write.
  - Timeout tests (`turn_policy`, `budget`, roko-agent's `claude_cli_agent`): each tries attempt budgets of 1, 2,
    4, 8 and 16 s, and asserts once the fake provider reached what the test checks (both prompts recorded, usage
    streamed, both messages read). The provider records its prompt with a rename.
  - Fixture attempts and verify steps had 5 s and 10 s limits. The loops hit
    `gate_rows_carry_the_attempts_turns_or_unknown` and `graph_task_cell_reaches_real_provider_runtime` timing
    out after 5000 ms at load 33 to 70. They now use a 120 s hang guard (`FIXTURE_HANG_GUARD_SECS`).
  - Sibling pair: the fake sibling ends once `InFlightTasks::settling_began` reports that the settle began.
  - `graph_run_routing_observations_survive_a_crash`: a WAL segment's flock stayed held after its writer dropped
    while a child spawned at that moment still held a copy of the descriptor, so recovery skipped the segment as
    live. `WalSegment` now unlocks on drop; a roko-learn test holds a cloned descriptor across the drop.
- 2026-09-30 (wk-specq): proof, with clippy and other workers' builds running (load 21 to 72). Each passed 20 runs
  in a row under `--test-threads=32`:
  - both tautology tests, with a shared target dir;
  - roko-agent `claude_cli_agent::` (47 tests);
  - roko-learn `wal::` and `model_call_feedback::` (15 tests);
  - the 16 fixed roko-cli tests;
  - roko-cli's `graph_task_dispatch`, `runtime_feedback` and `background_writes` modules (191 tests).
  The verify passes in both forms, the old single-test filter and the new one.
