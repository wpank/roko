+++
id = "gap-0eb763"
kind = "gap"
title = "The portal's Run and Run again start a plan over without asking when its last run can be resumed: execute with no body runs fresh (contract P-7), so passed tasks, or a run whose delivery a crash cut, are set aside and redone; read resumable_run first and offer Resume or Start over (design B, Will's call of 2026-10-27: P-7 kept, option A declined)"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["apps/portal"]
created = 2026-10-27
updated = 2026-10-28
last_verified = 2026-10-27
source = "round-v7 roko-ladder, moved from bug-4fadf8's Run button half, the lead's call (2026-10-27)"
discovered_from = "agent:roko-recovery (round v7 kill matrix) and agent:roko-ladder"
anchors = ["apps/portal/src/components/stage/PlanView.tsx", "apps/portal/src/api/queries.ts::useRunPlan", "crates/roko-serve/src/routes/plans/reads.rs"]
links = { depends_on = [], blocks = [], related = ["bug-4fadf8", "bug-cbdb6b", "gap-b78706"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqF --include='*.test.tsx' 'asks before a fresh run sets a resumable run aside' apps/portal/src && (cd apps/portal && npx vitest run src/components/stage/runChoice.accept.test.tsx)"
+++

## Problem

The portal's Run and Run again buttons can throw away a run that roko could resume, and redo every task. Both send
`POST /api/plans/{id}/execute` with no body (`apps/portal/src/components/stage/PlanView.tsx`, cases `run` and
`run-again`, through `useRunPlan`). Under the portal contract P-7 (`tmp/portal-audit/03-CONTRACT.md`, 2026-09-28),
that runs the plan fresh.

When the plan's last run stopped part way, the fresh run sets the checkpoint aside and runs every task again. A run
stopped part way means:
- tasks passed and tasks left (bug-cbdb6b); or
- every task passed but a crash cut the delivery (bug-4fadf8, roko-recovery's kill points e and e').

The answer and the `checkpoint.set_aside` event name what was set aside, but the person is never asked. Only Retry
(`{resume: true}`) continues a run.

bug-4fadf8 fixed the chat side: a chat host's plain `plan_run` is refused and must say `resume` or `fresh`. The HTTP
route keeps P-7, so the portal is the one that has to ask.

## Why it matters

- A stopped run's passed tasks were paid for. Starting over pays for them again. In roko-recovery's matrix the
  Run button redid T1 and T2 at kill points e and e', for $0.246 on a run that had spent $0.164.
- Run is the plan page's default action, and Run again appears after a run ends.
- Goal: core.

## Where

- `apps/portal/src/components/stage/PlanView.tsx`: the `run` and `run-again` cases of the plan's action handler.
- `apps/portal/src/api/queries.ts::useRunPlan`: the execute call.
- `crates/roko-serve/src/routes/plans/reads.rs`: `GET /api/plans/{id}` already says `resumable_run` (gap-b78706):
  `run_id`, `status`, `passed_tasks`, `tasks_left`, `spent_usd`, and since bug-4fadf8 `delivery_pending`. It is
  `null` when no run can be resumed.

## Current state

- The portal never reads `resumable_run`.
- roko serve answers execute with `set_aside` when it starts over (bug-cbdb6b), but the portal doesn't show it.

## Plan (design B, the lead's call on 2026-10-27)

1. Before Run or Run again, the portal reads the plan's `resumable_run`. When there is one, it asks instead of
   running.
   - Copy: "Its last run (8 chars of the id, status) passed T1 and T2, and T3 is left." When `delivery_pending` is
     true: "…passed every task, and only the delivery is left."
   - "Resume" sends `{resume: true}`, so only what is left runs. "Start over" sends no body, which sets the run
     aside and runs every task again.
   - With no resumable run, Run behaves as today.
2. After a fresh run, show the answer's `set_aside` as a notice: the run, its passed tasks, and
   `roko resume <run>` to bring it back.
3. Test: a plan whose `resumable_run` is set asks before running. Resume posts `{resume: true}`, and Start over
   posts no body. A plan without one runs at once.

**Will's call (2026-10-27): contract P-7 is kept, and option B is chosen.** A body-less `execute` still runs fresh,
and the portal asks first, as above. Option A is declined: roko serve answering a body-less `execute` over a
resumable run with 409 `resumable_run`.

## Done when

- The plan page asks before a Run or Run again that would set a resumable run aside.
- The portal test passes.

## Notes

- Moved here from bug-4fadf8, which closed on the chat half. Will's call of 2026-10-27 put it in round v7 after all,
  so it is coded this round.
- The Hermes plugin never calls `execute`. It asks from `resumable_run` already (P-57).
