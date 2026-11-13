+++
id = "bug-b6d5cb"
kind = "bug"
title = "The portal's top bar and plan list count a run's time from when its tasks began, not from roko accepting it, and the list's spend leaves out running calls: 7.2's list read 21m 11s beside TIME 22:13"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["apps/portal"]
created = 2026-11-13
updated = 2026-11-13
last_verified = 2026-11-13
last_verified_rev = "9f6d53d6b"
source = "round v8 portal-v8, phase 7.2's web stills (2026-11-13)"
discovered_from = "agent:portal-v8 (round v8)"
anchors = ["apps/portal/src/lib/statusChip.ts", "apps/portal/src/lib/planRows.ts", "apps/portal/src/components/plan/PlanPage.tsx"]
links = { depends_on = [], blocks = [], related = ["gap-c1726f", "bug-f8c943"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cd apps/portal && npx vitest run src/components/shell/runClock.accept.test.tsx"

[closed]
at = 2026-11-13
at_ts = "2026-11-13T01:26:20Z"
commit = "9f6d53d6b"
by = "portal-v8 (round v8)"
executor = "unknown"
forced = false
evidence = "lib/runReadings.ts holds one TIME (runClock: from last_run.started_at, else the tasks' start; roko's run_completed.duration_ms once ended when the run held this plan alone) and one SPENT (runSpentUsd: settled tries + running tries' calls + carried), used by the glass, the top bar chip, the list rows, the header and the rows' spend; useRunStarts shares useRunFacts' query key. runClock.accept was red at fb0832a44 (chip '1 minute 40 seconds' for a run accepted 200 s ago; row 'passed · 0.0s · $0.32' where TIME read 22:13 and SPENT $1.32) and passes at 9f6d53d6b in roko-work-bug-36ce72: portal tsc clean, vitest 753/753."
+++

## Problem

The run card's TIME counts from when roko accepted the run (gap-c1726f). The page's other readings of the same
run still count from when the plan's tasks began, about a minute later on phase 7.2's run (583b4be5). Its web stills
show two times side by side:

- 01:04:32 UTC: the top bar's chip read 14:05 while TIME read 15:06.
- 01:11:46 (B): the plan's row in the list read "21m 11s" while TIME read 22:13.
- The header said "started 01:50" while TIME's note said "started 01:49".

The row's spend counts settled tries only. SPENT also counts the running tries' calls and a resume's earlier spend
(bug-f8c943), so the row and the glass disagree during a try and on a resume.

## Why it matters

Goal visibility: the guide's three-surface section says the readings match. A still with the top bar or the list
shows two times for one run.

## Where

- `apps/portal/src/lib/statusChip.ts` (the chip's clock)
- `apps/portal/src/lib/planRows.ts` (the row's time)
- `apps/portal/src/components/plan/PlanPage.tsx` (`whenText`)
- `apps/portal/src/components/shell/Workspace.tsx` (the row's spend)
- `apps/portal/src/lib/runCard.ts` (TIME and SPENT)

## Plan

- One helper each for a run's TIME and SPENT, used by the glass, the chip, the row and the header.
- The chip and the rows read when roko accepted each run from the query the plan's page already makes
  (`last_run.started_at`), so the list and the open plan share one request.
- Test: the chip, the row and the header agree with TIME while a run runs. After it ends, the row reads roko's own
  duration and the glass's spend.

## Done when

- Every time and spend the page shows for one run is the same number.
- The `[[verify]]` passes.

## Notes

- Round v8 portal-v8, found in 7.2's web stills; it extends gap-c1726f's definitions to the rest of the page.
