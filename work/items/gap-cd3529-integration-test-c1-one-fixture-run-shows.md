+++
id = "gap-cd3529"
kind = "gap"
title = "Integration test C1: one fixture run shows the same honest verdicts on every surface"
status = "open"
triage = "verified"
severity = "p1"
goal = "truth"
size = "M"
subsystem = ["roko-cli/tests"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "7490cb94b"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e2"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W8-roko-as-executor.md (canary C1)"
anchors = ["crates/roko-cli/tests/"]
lane = "rust-cold"
parent = "spec-e9d7ec"
links = { depends_on = ["bug-7e1b6b", "bug-a843d4", "bug-94151f", "bug-7eb27e", "gap-29a84b"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn honest_verdicts_canary' crates/roko-cli/tests/ && cargo test -p roko-cli --test honest_verdicts_canary"
+++

## Problem

Each fix in epic spec-e9d7ec has its own unit test. Nothing checks, end to end, that one run's verdicts agree across
every place they surface: the Graph checkpoint, the plan outcome, the run metrics, the dashboard snapshot and the
episode records. Without such a check, a later change can bring back a false pass on one of those surfaces without
anyone noticing.

## Why it matters

This test is the exit check for epic spec-e9d7ec, and it becomes C1 in the golden-path acceptance suite (plan epic E11).
The whitepaper's honest-verdicts claim cites it.

## Where

- **New file:** `crates/roko-cli/tests/honest_verdicts_canary.rs`.
- **Pattern to copy:** `crates/roko-cli/tests/graph_budget_resume.rs`. It configures a scripted fake provider through
  `roko.toml` and runs the built binary with `assert_cmd`.
- **Surfaces to assert:**
  - the checkpoint, `.roko/state/graph/<plan>/checkpoint.json`;
  - `roko plan status`;
  - `.roko/learn/run-metrics.jsonl`;
  - `classify_task_outcome` in `crates/roko-core/src/dashboard_snapshot.rs`;
  - `.roko/episodes.jsonl`.

## Current state

No such test exists. The fake-provider pattern exists, and the Graph budget and resume tests use it.

## Plan

1. Write a fixture plan with four independent tasks and a scripted fake provider:
   - T1 passes its verify step;
   - T2 has no verify step;
   - T3's role is disabled;
   - T4's verify step fails.
2. Run it with the built `roko` binary, as `graph_budget_resume.rs` does.
3. Assert:
   - the checkpoint verdicts are passed, unverified, skipped (or failed, depending on bug-a843d4's fix) and failed;
   - the plan outcome is not `succeeded`;
   - the run metrics record one passed task;
   - the dashboard snapshot counts one pass;
   - the episode records mark only T1 as a success.

   Router labels are covered by epic E4's census, not here.

## Done when

- [ ] The test exists and passes.
- [ ] Reverting any one of the epic's fixes makes it fail. Check this once by hand and say so in the closing evidence.
- [ ] The `[[verify]]` command passes.

## Notes

- The test builds `roko-cli` but edits no hot file. It can be written while the fixes are in progress, and it merges
  last.
- Keep it under a minute: fake provider only, no network.
- 2026-09-30 (wk-gates): Implemented on `work/bug-7e1b6b` at `357d8448d`, after the two fixes it depends on: bug-a843d4
  at `361f1546d` and bug-7e1b6b at `18ef631da`. The `[[verify]]` command passes in the worker's own target clone (the
  test takes about 7 s); the batch check re-runs it.
  - **Fixture:** five tasks in two plans, not four in one. T1–T4 are as planned. T5 has no verify step and is served
    by a T0 reflex rule, so that bug-94151f is exercised. T1, T2 and T5 form their own plan, so that its outcome
    depends on gap-29a84b; beside failed tasks, a plan fails either way.
  - **Surfaces:** the run's JSON report (plan outcomes), both checkpoints (status, gate verdicts, failed tasks),
    `roko plan status` (plan-level only: its task rows come from `tasks.toml`), `.roko/learn/run-metrics.jsonl` (run
    and per plan), a `DashboardSnapshot` rebuilt from the `--log-file` hub events (the events the TUI and SSE clients
    get) with `classify_task_outcome` on each outcome, the `run.completed` line's task outcomes, `.roko/episodes.jsonl`
    and the reflex store. Episodes are checked by `outcome` and `learning_label`, which learners read; `success` keeps
    its older meaning (the call succeeded and no verify step failed), so it is true for T2 too.
  - **Revert check, by hand, once each:** C1 fails with each fix reverted on its own. bug-7e1b6b (`node_outcome` says
    `passed` without a passed verdict): the dashboard outcomes differ. bug-a843d4 (a disabled role completes): the
    second checkpoint's failed tasks differ. bug-7eb27e (run metrics count tasks by their plan's outcome): the metrics
    differ. gap-29a84b (unverified counts as success): the plan outcomes differ. bug-94151f (a reflex match credits its
    rule at once): the rule's `success_count` differs.
  - **Also changed:** the run metrics were appended by a spawned task that a process exit could drop, so the canary
    could miss them. They are now written before the run returns.
- 2026-09-30 (wk-gates), notes for the two bugs this depends on, whose item files had uncommitted edits in the main
  checkout:
  - **bug-a843d4** (`361f1546d`): a task whose role is disabled now fails with a rejection naming the role and its
    config key, without a dispatch; a rejection is not retried, and a resume runs the task again. New test
    `disabled_role_task_fails_without_dispatch`; the plan runner's disabled-role fixtures now expect a failed plan.
    Its `[[verify]]` command passes.
  - **bug-7e1b6b** (`18ef631da`): the plan runner's status polls report each finished task with its gate verdict (the
    Activity log's record while the plan runs, the graph's output at the end), and a completed task without a passed
    verdict is `unverified`. `classify_task_outcome` gains `Unverified` and `Skipped` classes. The snapshot counts them
    apart; `tasks_completed` and the plan's new `tasks_passed` count passes only, while `tasks_done` stays the progress
    count. A task that finishes without a `TaskStarted` (it never ran, or it ran between two polls) is now counted. The
    TUI shows unverified tasks in amber and skipped ones as skipped; the portal gains an `unverified` task status in
    amber (`tsc` is clean, and vitest passes 788 tests in 80 files). Both its committed `[[verify]]` command and the
    main checkout's pending one (`skipped_and_unverified_tasks_are_not_counted_as_passed`) pass. Left as it was: the
    portal reducer still ignores a `task_completed` for a task it never saw start; its rows show such tasks as skipped
    once the plan ends.
- 2026-10-01 (wk-gates): `95486b03a` gives gap-9eb1e1's `already_satisfied` outcome its own class, at its author's
  request. It counts in `tasks_done` and in new `tasks_already_satisfied` counters, never in `tasks_passed` or
  `tasks_completed`. The TUI shows it in teal and the portal in info cyan (`--state-satisfied`), each with a `≡`
  glyph labelled "already satisfied". `08557bb28` merges the working branch at `a4e175c9c`. `node_outcome`
  conflicted with gap-9eb1e1, so both arms are kept, and that one now returns `TASK_OUTCOME_ALREADY_SATISFIED`.
  Re-verified at `08557bb28` in the worker's own target clone: the three `[[verify]]` commands, C1, nightly fmt,
  clippy for roko-core and roko-cli, and 940 targeted lib tests all pass. The portal passes `tsc` and 789 vitest
  tests.
