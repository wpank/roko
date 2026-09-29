+++
id = "gap-6ff814"
kind = "gap"
title = "The portal never shows the design's run summaries: the all-dispatched, finished and stopped-at sentences are unreachable"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["apps/portal"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "ffffcb4fb"
source = "plan:portal-programme/09-acceptance#T04"
discovered_from = "plan:portal-programme/09-acceptance#T04"
anchors = ["apps/portal/src/lib/emptyState.ts::describeEmpty", "apps/portal/src/components/stream/StreamPane.tsx:351", "apps/portal/src/lib/taskRows.ts::focusTaskId"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqF --include='*.test.ts' --include='*.test.tsx' 'shows the finished sentence' apps/portal/src && (cd apps/portal && npx vitest run src/components/stream src/components/stage src/lib/emptyState)"

[closed]
at = 2026-09-29
commit = "ffffcb4fb"
by = "commit trailer"
evidence = "ffffcb4fb: the run-state sentence lives in the stage, under the status line, once the plan has run (PlanView renders describePlan(planState(...)) as data-region=run-summary, whatever task has focus; before a run the status line carries tasks and waves). lib/emptyState.planState passes waves, the first failed task and its failed check, and the duration; StreamPane uses it ('Ready — 2 tasks in 2 waves.') and gets the plan count from Workspace, so an open stream with plans but none selected asks to select one. Tests: components/stage/runSummary.test.tsx (6: 'shows the finished sentence' for a completed plan with T01 selected, the accepted variant, stopped-at T02 with its structural check, all-dispatched, placement, none before a run), components/stream/streamSentence.test.tsx (4), lib/emptyState.test.ts (+5); the [[verify]] passes (18 files, 125 tests)."
+++

## Problem

`describeEmpty` implements every sentence in design §7 and is unit-tested (`emptyState.test.ts`).
The stream, however, renders it only when no task has focus (`StreamPane.tsx:351-355`,
`focusedId === null`). `focusTaskId` always focuses a task once any task is active, failed or
finished (the selection, else the first active, else the first failed, else the last finished). So
these sentences never appear anywhere:

- "Every task is dispatched; checks are running."
- "Finished in 4m12s — 3 of 3 verified.", and its forced-accept variant.
- "Stopped at T03 — test failed. Retry resumes from T03." No caller passes `failedTaskId` or
  `failedCheck`.

Also:

- "Ready — 3 tasks in 3 waves." renders without the wave count, because no caller passes `waves`. It
  appears only in the stream body, which is closed before a run.
- With plans present but none selected, an open stream says there are no plans: `StreamPane.tsx:152`
  counts 0 plans when nothing is selected.

## Why it matters

Goal `visibility`. §7 exists so that an idle region "says what the system is doing". Right now the
run summaries are dead code.

## Where

`apps/portal/src/lib/emptyState.ts::describeEmpty`, `apps/portal/src/components/stream/StreamPane.tsx`
and `apps/portal/src/lib/taskRows.ts::focusTaskId`.

## Plan

Decide where the run-state sentence lives, for example in the stage under the status line, and
render it from plan state regardless of task focus. Pass the wave count and the failure fields.

## Done when

A test whose name contains "shows the finished sentence" renders it for a completed plan. The
`[[verify]]` runs it.

## Notes

Found by plan 09 T04 (see VERDICT).
