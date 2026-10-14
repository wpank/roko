+++
id = "bug-06bd18"
kind = "bug"
title = "A new run of a plan cannot start while a failed run's attempt checkout is kept"
status = "open"
triage = "verified"
severity = "p1"
goal = "core"
size = "M"
subsystem = ["roko-cli/orchestrator"]
created = 2026-10-14
updated = 2026-10-14
last_verified = 2026-10-14
source = "e2e-v4 dogfood, roko-stream worker, 2026-10-14 (roko main 86d629363)"
anchors = ["crates/roko-cli/src/orchestrator/worktree/acceptance.rs::create_attempt_locked", "crates/roko-cli/src/orchestrator/worktree/mod.rs::create_locked", "crates/roko-cli/src/orchestrator/worktree/tests.rs"]
links = { depends_on = [], blocks = [], related = ["bug-056b40"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_new_run_sets_aside_the_checkouts_another_run_kept' crates/roko-cli/ && cargo test -p roko-cli a_new_run_sets_aside_the_checkouts_another_run_kept"
+++

## Problem

Running a plan again after a failed run fails at once with worktree isolation on: the first task's attempt cannot
get its checkout.

```
T1 did not finish: agent error (worktree-isolation): failed to acquire worktree for tally-budgets/T1/attempt-0:
workspace I/O error: cannot safely reattach worktree `attempt-54aa029051f97b0496bb`: branch
`roko/attempt/attempt-54aa029051f97b0496bb` is already checked out in this repository.
```

Seen in the e2e-v4 dogfood (run 216d7e62 of `tally-budgets`, after run 4af3845a failed; roko main 86d629363).
Every task then fails or is blocked; only removing the kept checkouts by hand lets the plan run again.

## Why it matters

Goal `core`: "fix it and run it again" is the normal loop after a failed plan, and it is the loop a host such as
the Hermes plugin offers. bug-056b40 made a resumed run re-attach its own kept checkouts; a fresh run of the same
plan still cannot start.

## Where

- `crates/roko-cli/src/orchestrator/worktree/acceptance.rs::create_attempt_locked`: re-attaches a kept checkout only
  when the run that made it (`roko-run` in its admin dir) is this run; otherwise calls `create_locked`.
- `crates/roko-cli/src/orchestrator/worktree/mod.rs::create_locked`: refuses when the attempt's branch is checked out
  (`ReattachRejected`).
- Attempt checkout ids and branches (`format_attempt_worktree_id`, `format_attempt_branch_name`) depend on plan, task
  and attempt only, so attempt 0 of a task has the same checkout in every run.

## Current state

A failed attempt's checkout is kept for post-mortem (`RetainForFailure`), and a failed run keeps its attempts'
checkouts. One plan executor runs per workspace (the runner lock), so the run that made a kept checkout is no longer
running when another run asks for the same attempt.

## Plan

When the canonical checkout of an attempt exists and another run (or no recorded run) made it, set it aside before
making the new one: rename its branch to `roko/attempt/<id>-<run>` and move the checkout to
`.roko/worktrees/<id>-<run>` with `git worktree move`, both under the repository mutation lock, rolling the rename
back if the move fails. Nothing is deleted: the kept attempt stays inspectable under its new name. A checkout that
fails the identity checks is still refused.

## Done when

`[[verify]]` passes: a run that is not the one that kept an attempt's checkout gets a fresh checkout from its base,
and the kept checkout, its branch and its uncommitted files survive under the set-aside name.

## Notes

The resume path (same run) is unchanged. `a_resumed_run_starts_attempts_from_the_plan_branch` asserted the old
refusal for another run; it now checks the set-aside instead.
