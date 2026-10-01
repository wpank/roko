+++
id = "gap-9eb1e1"
kind = "gap"
title = "A --fresh rerun of a task whose correct output is already in the tree fails as pre_verify:no_changes"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "8ad7daa10"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "wk-runstate report on gap-568056 (2026-09-30)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/red_flags.rs"]
lane = "rust-hot"
parent = "spec-9230a9"
links = { depends_on = [], blocks = [], related = ["gap-b72761"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_fresh_rerun_of_a_satisfied_task_is_not_rejected_as_no_changes' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_fresh_rerun_of_a_satisfied_task_is_not_rejected_as_no_changes"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 8ad7daa10. An attempt that changes nothing at a task with authored verify steps runs them on the unchanged tree; a pass settles as the new AlreadySatisfied verdict (no blame, null learning label, counted apart from passed), a failure is still rejected as pre_verify:no_changes with the verify summary. Batch 18: gated together with taskdef's branch on 2d93c11db (check, clippy -p roko-cli -p roko-graph -p roko-learn -D warnings clean; lib tests roko-cli 3247 with failures only in taskdef's new streaming test (sent back) and the two writer/WAL flakes fixed by bug-779ae7, roko-graph 476, roko-learn 1204; --test attempt_diff_canary (C5) 2 passed), then re-checked without taskdef on 31285d1c4 (same code as MAIN): cargo check --workspace --tests and nightly fmt clean; the items' tests pass. Verify: a_fresh_rerun_of_a_satisfied_task_is_not_rejected_as_no_changes passes; C5 (with a --fresh rerun expecting already_satisfied) passes."
+++

## Problem

`red_flags.rs` exempts only resumed runs (attempt number above 0) from the no-changes check. A `--fresh` run starts again at attempt 0. If the tree already holds the task's correct output, for example because an earlier run's changes are still there or were committed, every attempt changes nothing and is rejected as `pre_verify:no_changes`, so the task fails after its retries.

## Why it matters

Re-running a plan is normal, and failing a task whose work is already done is wrong, both as behaviour and as evidence: it counts as a failure. The screen exists to catch agents that do nothing, not tasks that have nothing left to do. Epic spec-9230a9.

## Where

`crates/roko-cli/src/graph_task_dispatch/red_flags.rs` (the no-changes exemption), and the settle path that records the outcome.

## Current state

Found by wk-runstate: the evidence fixture's `--fresh` rerun was rejected until the test deleted the first run's artifact.

## Plan

1. When an attempt changes nothing, run the task's verify steps on the unchanged tree before rejecting it.
2. If they pass, settle the attempt as already satisfied: not a learning success and not a failure. Record the reason and let the task complete. If they fail, reject as now.
3. Decide whether an already-satisfied attempt counts toward the dashboard's passed tasks. Recommendation: its own outcome, not passed.
4. Test: `a_fresh_rerun_of_a_satisfied_task_is_not_rejected_as_no_changes`.

## Done when

- [ ] A fresh rerun of a satisfied task completes without an agent change and is recorded as already satisfied.
- [ ] The `[[verify]]` command passes.

## Notes

- **wk-tamper (2026-09-30):** Implemented on `work/gap-9eb1e1` at `0c71c240e`; cargo verification deferred to the
  batch check. New verdict `already_satisfied` (`TaskGateVerdict`, `GateVerdictTag`, `AttemptOutcome`): no blame, a
  null learning label, replayed on resume, counted as `tasks_already_satisfied` and not as passed; the plan can still
  succeed. It applies only to tasks with authored verify steps (pinned acceptance steps count). Tasks without them,
  gitignored declared files and resumed runs keep the old handling. The verify run on the unchanged tree is a probe:
  no auto-fix and no learning records (gate thresholds, gaming detector, holdout, gate-pass efficiency, retrieval);
  CodingOracle observations and the TUI's gate events still happen. Tests: the lib test named in `[[verify]]`, the
  C5 canary's new `--fresh` rerun, and `already_satisfied_verdict_is_not_reported_as_passed`.
- A vacuous verify step (`true`) lets a do-nothing attempt settle as already satisfied. The label keeps that visible.
- Still open elsewhere: `classify_task_outcome` and the portal's `runState.ts` classify `already_satisfied` by
  fallback (wk-gates, bug-7e1b6b), and viabilitybench's `run-record.schema.json` `gate_verdict` enum and
  `analysis/metrics.py` `NOT_PASSED` do not list it.
