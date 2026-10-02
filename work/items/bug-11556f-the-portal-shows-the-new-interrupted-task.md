+++
id = "bug-11556f"
kind = "bug"
title = "The portal shows the new 'interrupted' task outcome as unknown"
status = "open"
triage = "unverified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["portal"]
created = 2026-10-02
updated = 2026-10-02
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-60ccba"
anchors = ["apps/portal/src/lib/runState.ts", "apps/portal/src/lib/taskRows.ts"]
lane = "frontend"
links = { depends_on = [], blocks = [], related = ["bug-60ccba"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cd apps/portal && npx vitest run src/lib/taskRows.test.ts"
+++

## Problem

bug-60ccba added the task outcome `interrupted` (TASK_OUTCOME_INTERRUPTED), but the portal's run-state mapping doesn't know it, so it renders as an unknown outcome.

## Plan

Map `interrupted` as a failed (not passed) outcome with its own label, and add a vitest case.

## Done when

- The portal tests pass, and an interrupted task shows its label.

## Notes

- Reported on 2026-10-02 by wk-streams, working on bug-60ccba, during the overnight close-out round.
