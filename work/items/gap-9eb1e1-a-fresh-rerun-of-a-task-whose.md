+++
id = "gap-9eb1e1"
kind = "gap"
title = "A --fresh rerun of a task whose correct output is already in the tree fails as pre_verify:no_changes"
status = "open"
triage = "unverified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "wk-runstate report on gap-568056 (2026-09-30)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/red_flags.rs"]
lane = "rust-hot"
parent = "spec-9230a9"
links = { depends_on = [], blocks = [], related = ["gap-b72761"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_fresh_rerun_of_a_satisfied_task_is_not_rejected_as_no_changes' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_fresh_rerun_of_a_satisfied_task_is_not_rejected_as_no_changes"
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
