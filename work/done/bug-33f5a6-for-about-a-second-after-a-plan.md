+++
id = "bug-33f5a6"
kind = "bug"
title = "For about a second after a plan passes, the portal offers revise and run again while the run that holds the plan is still finishing; the server answers 409 (part of an active run), the portal reads it as 'This plan is already being revised', and the CI browser check's revise step fails now and then"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["apps/portal"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "92608b967"
source = "round v8 portal-v8, the CI browser check at 58ce1b9bb (2026-11-12)"
discovered_from = "agent:portal-v8 (round v8, 58ce1b9bb)"
anchors = ["apps/portal/src/components/plan/PlanPage.tsx::PlanPage", "apps/portal/src/components/plan/usePrimaryAction.ts::usePrimaryAction", "apps/portal/src/components/stage/PromptPanel.tsx::toRevise409Error", "apps/portal/src/lib/planSet.ts"]
links = { depends_on = [], blocks = [], related = ["gap-60f6b6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cd apps/portal && npx vitest run src/components/plan/finishing.accept.test.tsx"

[closed]
at = 2026-11-12
at_ts = "2026-11-12T15:40:00Z"
commit = "92608b967"
by = "portal-v8 (round v8)"
executor = "claude-agent"
via = "manual"
model = "claude-opus-5-5"
size = "S"
claimed_at = "2026-11-12T15:15:00Z"
forced = false
evidence = "runFinishing (apps/portal/src/lib/planSet.ts) holds revise and run again, with the reason 'roko is finishing the run', from the run's last plan_completed until its run_completed (15 s at most); toRevise409Error reads 'part of an active run' as a run that hasn't finished. Checks at 92608b967's tree in roko-work-bug-33f5a6: tsc clean, vitest 706/706 (finishing.accept 4 cases), orphans none, build:export OK, roko plan validate over every portal plan without errors, and the CI browser check (09-acceptance/portal-check.sh, fenced $0 copy) PASS 37 of 37 twice in a row."
+++

## Problem

For about a second after a plan passes, the portal offers revise and run again, and both fail.

The plan's own `plan_completed` arrives before the run that holds it ends: in a $0 run, `plan_completed` came at
+0 ms and `run_completed` at +816 ms (`/private/tmp/roko-live-7b3jdf`, the CI browser check at 58ce1b9bb). Until
then the server's `active_plans` still lists the run, so `POST /api/plans/{id}/revise` answers 409 "plan '<id>' is
part of an active run (run key: …); finish or cancel the run before revising the plan". In that window:

- The plan page shows "passed" with revise enabled; `usePrimaryAction` enables run again too, since
  `planSetActive` only counts members still pending or running.
- `PromptPanel` reads that 409 as "This plan is already being revised.": its mapping looks for "revis" in the
  message, and "revising" matches.
- The CI browser check's revise step clicks in that window now and then, gets the 409, and waits 120 s for a task
  that never comes (it failed once in two runs at 58ce1b9bb).

## Why it matters

Goal visibility. A person who clicks revise right after a pass is told something untrue, and the browser check is
flaky.

## Where

- `apps/portal/src/lib/planSet.ts` (whether a run still holds a plan), `apps/portal/src/components/plan/PlanPage.tsx`
  (the revise key), `apps/portal/src/components/plan/usePrimaryAction.ts` (run again),
  `apps/portal/src/components/stage/PromptPanel.tsx` (`toRevise409Error`).

## Plan

1. A run holds a plan from its `plan_set_loaded` until its `run_completed`: revise and run again stay disabled
   until then, with the reason "roko is finishing the run".
2. Read the server's 409s by what they say: an active run is "This plan is part of a run that hasn't finished;
   revise it when the run ends.", a revision in progress is "This plan is already being revised.".
3. Tests: `src/components/plan/finishing.accept.test.tsx`.

## Done when

`npx vitest run src/components/plan/finishing.accept.test.tsx` passes, and the CI browser check passes twice in a
row.
