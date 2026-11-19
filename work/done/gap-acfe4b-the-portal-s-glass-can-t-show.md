+++
id = "gap-acfe4b"
kind = "gap"
title = "The portal's glass can't show a chat-started run's spend cap, and says 'no cap set' when it doesn't know it: roko serve keeps the run's max_usd nowhere a host can read, so Hermes shows 'of $9.00' while the portal shows the spend with 'no cap set'"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "M"
subsystem = ["apps/portal"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "66a214ee2"
source = "round v8 portal-v8, the three-surface capture of phase 6 (2026-11-12); the lead's decision"
discovered_from = "agent:portal-v8 (round v8)"
anchors = ["crates/roko-serve/src/state.rs::PlanHandle", "crates/roko-serve/src/routes/plans/reads.rs::get_plan", "apps/portal/src/lib/runCard.ts", "apps/portal/src/api/queries.ts::usePlanCap"]
links = { depends_on = [], blocks = [], related = ["gap-6d2dd2"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_detail_names_its_runs_cap' crates/roko-serve/src && cargo test -p roko-serve --lib plan_detail_names_its_runs_cap"

[[verify]]
command = "cd apps/portal && npx vitest run src/components/plan/cap.accept.test.tsx"

[closed]
at = 2026-11-12
at_ts = "2026-11-12T22:19:38Z"
commit = "66a214ee2"
by = "portal-v8 (round v8)"
executor = "unknown"
forced = false
evidence = "roko serve: a plan run's handle keeps its request's max_usd (PlanHandle.max_usd), and GET /api/plans/{id} returns it as last_run.max_usd from the plan's live run, else its newest ended one (run_cap). Portal: SPENT's cap is last_run.max_usd (useRunCap: read at the run's start and end, every 5 s while it runs without one), else the plan's budget; an unknown cap shows the spend alone, no meter, never 'no cap set'. Red first at 008781c7f (plan_detail_names_its_runs_cap: no max_usd while the run goes; cap.accept: 'of $3.00' for a $9.00 run and 'no cap set'). At 66a214ee2's tree in roko-work-bug-36ce72: plan_detail_names_its_runs_cap and 81 more matching tests pass, roko-serve lib 1100/1100, nightly fmt and clippy -D warnings clean; portal tsc clean, vitest 736/736 (cap.accept 3/3)."
+++

## Problem

The portal's glass can't show SPENT against its cap for a run started from a chat host, and it says something false
when it doesn't know the cap:

- **No cap.** A chat host starts a run with a cap (`plan_run`'s `max_usd`, $3.00 or $9.00 in round v8's phase 6),
  and Hermes shows "spent $0.40 of $3.00". roko serve keeps that cap nowhere a host can read. `GET
  /api/plans/{id}/costs` says the plan's budget is unlimited, and neither the run manifest nor the plan's checkpoint
  records it. The portal's SPENT has no "of $X".
- **"no cap set".** With no cap to show, the glass says "no cap set". That's false: the run has a cap, the portal
  just doesn't know it. Unknown isn't unlimited.

Seen in round v8's three-surface capture of phase 6's resume (2026-11-12, 21:56 UTC). The web glass read "$0.59, no
cap set" while Hermes read "of $9.00".

## Why it matters

Goal visibility: SPENT against its cap is one of the four readings every surface shows (the design system's §4.2).
The guide's three-surface capture shows the same moment of the same run in Hermes, the portal and the terminal, so
the readings must match.

## Where

- `crates/roko-serve/src/state.rs::PlanHandle`: a plan run's handle; `routes/plans/run_control.rs` starts runs with
  the request's `max_usd` (9116), but the handle doesn't keep it.
- `crates/roko-serve/src/routes/plans/reads.rs::get_plan`: `GET /api/plans/{id}`, with `last_run` from the plan's
  checkpoint (gap-6d2dd2).
- `apps/portal/src/lib/runCard.ts` (the SPENT reading), `apps/portal/src/api/queries.ts::usePlanCap`,
  `apps/portal/src/components/plan/PlanPage.tsx`, and `apps/portal/src/lib/planSet.ts::detailStamp` (when the plan's
  detail is read again).

## Plan

The lead's decision (round v8, 2026-11-12):

1. roko serve: a plan run's handle keeps the cap its request set (`max_usd`). `GET /api/plans/{id}` returns it as
   `last_run.max_usd`, from the plan's live run, else its newest ended one that serve still holds. A resume's cap
   counts the whole run, which is the run `last_run` describes. Without a run that set one, there's no field, as
   before.
2. Portal: SPENT's cap is `last_run.max_usd`, else the plan's budget (`/costs`), else unknown. When it's unknown the
   glass shows the spend alone, with no meter, and never "no cap set". The plan's detail is read again when its run
   starts, and every 5 s while it runs without a cap, so the cap shows once the run's checkpoint exists.
3. Tests: `plan_detail_names_its_runs_cap` (roko-serve), `cap.accept.test.tsx` (portal).

## Done when

- A chat host's run shows "of $X" on the portal's glass, and an unknown cap shows the spend alone.
- The `[[verify]]` passes.

## Notes

- Round v8 portal-v8. The guide's phase 7 capture needs it.
