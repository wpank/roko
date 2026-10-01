+++
id = "bug-4e5a59"
kind = "bug"
title = "The portal's run-state reducer ignores a task completion it never saw start"
status = "open"
triage = "unverified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["apps/portal/run-state"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-gates's report, checked on work/bug-7e1b6b at 63b77c9f5)"
anchors = ["apps/portal/src/lib/runState.ts"]
lane = "rust-hot"
parent = "spec-e9d7ec"
links = { depends_on = [], blocks = [], related = ["bug-7e1b6b", "bug-54c729"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqF 'applies a task completion it never saw start' apps/portal/src/lib/ && (cd apps/portal && npx vitest run src/lib)"
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
