+++
id = "gap-2118f0"
kind = "gap"
title = "The portal never says which task blocked another, although TaskRun carries blocked_by"
status = "open"
triage = "unverified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["apps/portal"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-runstate's report on gap-f59fe9)"
anchors = ["apps/portal/src/lib/runState.ts", "apps/portal/src/lib/taskRows.ts"]
lane = "frontend"
links = { depends_on = [], blocks = [], related = ["gap-f59fe9"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq 'blocked by' apps/portal/src/lib/taskRows.ts"
+++

## Problem

gap-f59fe9 added DashboardEvent::TaskBlocked with blocked_by, and the portal counts such tasks, but no portal view shows 'blocked by T1'.

## Why it matters

Visibility: a blocked task looks merely skipped.

## Plan

Show the blocker on the task row and in the task detail; add a taskRows test.

## Done when

- [ ] A blocked task shows which task blocked it
- [ ] The `[[verify]]` command passes.
