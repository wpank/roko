+++
id = "bug-4e5a59"
kind = "bug"
title = "The portal's run-state reducer ignores a task completion it never saw start"
status = "done"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["apps/portal/run-state"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "2b86a3be7"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-gates's report, checked on work/bug-7e1b6b at 63b77c9f5)"
anchors = ["apps/portal/src/lib/runState.ts"]
lane = "rust-hot"
parent = "spec-e9d7ec"
links = { depends_on = [], blocks = [], related = ["bug-7e1b6b", "bug-54c729"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqF 'applies a task completion it never saw start' apps/portal/src/lib/ && (cd apps/portal && npx vitest run src/lib)"

[closed]
at = 2026-10-01
commit = "2b86a3be7"
by = "commit trailer"
evidence = "runState.ts: a task_completed for a task the reducer never saw start now creates its record (unknown start time, phase completed, as the server snapshot does since bug-7e1b6b) and counts its outcome; a repeat completion is still ignored and a later start is a retry. The item's verify passes: vitest src/lib 501/501, including 'applies a task completion it never saw start'. The whole portal suite (789/789) and tsc --noEmit pass."
+++

## Problem

The portal folds run events into a run state (`apps/portal/src/lib/runState.ts`). A `task_completed` event for a task the reducer never saw start, because the page connected mid-run or the start event was dropped, is ignored. The task then never shows its outcome (wk-gates).

## Why it matters

Honest verdicts (epic spec-e9d7ec): the portal under-reports completed and failed tasks whenever it joins late. p3.

## Where

The `task_completed` case of the reducer in `runState.ts`.

## Plan

1. Create the task's entry on a completion without a prior start, with an unknown start time.
2. Add a test named "applies a task completion it never saw start".

## Done when

- [ ] Late-joining views show every completed task.
- [ ] The `[[verify]]` command passes.

## Notes

- Fixed on `work/bug-4e5a59` at `2b86a3be7`, stacked on wk-gates' `work/bug-7e1b6b` (5c90262ec), whose outcome
  classes and snapshot change it mirrors.
- The new record has an empty title, `attempts` 1 and no agent, as `fromSnapshot` builds a task. Task rows take the
  title and role from the plan's task list, and show no duration without a start time.
- This is plan step 3 of bug-151964 (insert the task on `task_completed`), in the portal. That item's step 1, a
  `TaskStarted` before a replayed task's completion, is not done here.
