+++
id = "bug-b4c565"
kind = "bug"
title = "The T0 reflex shortcut can pass a task without running the workspace rungs"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "b128de876"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-gates's report, checked on work/bug-5b43a9 at 7b25c478b)"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs"]
lane = "rust-hot"
parent = "spec-e9d7ec"
links = { depends_on = ["bug-5b43a9"], blocks = [], related = ["gap-3506f1", "bug-5b43a9", "bug-94151f", "gap-191ecd"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_reflex_pass_still_runs_the_workspace_rungs' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_reflex_pass_still_runs_the_workspace_rungs"
+++

## Problem

With `[learning] t0_reflexes` on, the dispatch of a task without verify steps checks for a matching reflex first (`graph_task_dispatch.rs:181-186` on the gates branch). A hit settles the task through the reflex shortcut (near :630), which on wk-gates' branch skips the workspace rungs that now run on plan run.

## Why it matters

Honest verdicts (epic spec-e9d7ec): a reflex pass would count as verified without the gates having run. bug-94151f fixed the reflex's gate credit; this is the same shortcut skipping the new rungs.

## Where

The reflex path in `graph_task_dispatch.rs`.

## Plan

1. Run the workspace rungs after a reflex hit (the reflex replaces only the dispatch, not the verification), or mark the result unverified.
2. Add `a_reflex_pass_still_runs_the_workspace_rungs`.

## Done when

- [ ] No task passes through the reflex shortcut without the rungs having run, or it is marked unverified.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on the gates branch.
- 2026-09-30 (wk-gates): Implemented on `work/gap-3506f1b` at `48232c4ec`; cargo verification deferred to the batch check. The reflex shortcut now serves only a task that no verify step checks: neither its own steps nor its plan's workspace rungs. A task the rungs check is dispatched and its rungs run. Running the rungs on a reflex's cached output instead would pass a task on a tree the reflex never changed, and a failing rung would repeat the same reflex on every retry.
