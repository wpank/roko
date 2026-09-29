+++
id = "gap-bd33b2"
kind = "gap"
title = "Portal stream deviates from design sections 5 and 11: passed checks hide output, unreached steps are unlisted, the truncation note is hidden"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "M"
subsystem = ["apps/portal"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a1adb2351"
source = "plan:portal-programme/09-acceptance#T04"
discovered_from = "plan:portal-programme/09-acceptance#T04"
anchors = ["apps/portal/src/components/stream/Checks.tsx:231", "apps/portal/src/components/stage/TaskList.tsx:38", "apps/portal/src/components/stream/Transcript.tsx:161", "apps/portal/src/components/stream/Transcript.tsx:68"]
links = { depends_on = [], blocks = [], related = ["gap-4171e8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqF --include='*.test.ts' --include='*.test.tsx' 'passed step output' apps/portal/src && (cd apps/portal && npx vitest run src/components/stream src/components/stage)"

[closed]
at = 2026-09-29
commit = "a1adb2351"
evidence = "a1adb2351: Checks lists every declared step via buildRungs (unreached as · with its command; passed output behind the raw-output toggle); failed task rows show digestHeadline under the row; focusTaskId puts a failed task before an active one and StreamPane derives the view (checks while the focused task has a failed step); the tail note sits in the tool row's <summary>; an empty entry list gets the empty-state sentence. New streamDesign.test.tsx (11 DOM tests, 8 fail on the old sources) plus digestEnding/digestHeadline unit tests. [[verify]] passes (stream+stage: 16 files, 100 tests); full portal vitest 62 files/692 tests, tsc, orphans.mjs and build:export pass."
+++

## Problem

Each point below was checked in the source on 2026-09-29.

- **Checks view:** a passed step shows its `$ command` but no output, not even behind a toggle
  (`Checks.tsx:231`: "Passed: output collapsed — nothing shown"). Declared verify steps that have not
  run are not listed. Design §5 lists every step, in `verify[i:phase]` order, with its output.
- **Failed row (§11):** the row's detail shows the first raw output line that does not start with `$`
  (`TaskList.tsx:38-50`), not the first digest line, and only once the row is expanded.
- **Stream focus (§11):** the stream switches to a failed task's checks only when that task has
  focus. Focus goes to the selection, else the first active task, else the first failed one
  (`taskRows.ts::focusTaskId`), so while another task runs, the failure is not shown.
- **Truncation note:** a tool row's "server kept only the tail" note sits inside the expanded
  `<details>` body (`Transcript.tsx:161-168`). Design §5 puts it on the collapsed row.
- **Empty transcript:** a transcript left with no entries (for example after `agent_completed` drops
  the unscreened records in trusted mode, `runState.ts:787-806`) renders blank, because the
  empty-state check is `transcript === undefined` (`Transcript.tsx:68`).

## Why it matters

Goal `visibility`. §5 and §11 are how a failure is read without leaving the plan view.

## Where

The anchors: `apps/portal/src/components/stream/Checks.tsx`, `components/stage/TaskList.tsx`,
`components/stream/Transcript.tsx`, plus `lib/taskRows.ts::focusTaskId`.

## Plan

Show passed-step output behind the same toggle a failure uses. List unreached declared steps as
`·`. Put the digest's first line on the failed row. When a task fails, point the stream at it unless
the operator has selected a task. Show the truncation note on the collapsed row. Treat an empty entry
list like a missing transcript.

## Done when

Tests whose names contain "passed step output", plus tests for the other points. The `[[verify]]`
runs them.

## Notes

gap-4171e8 (the snapshot keeps only the latest gate output per task) limits what the checks view can
show after a reload. Found by plan 09 T04 (see VERDICT).
