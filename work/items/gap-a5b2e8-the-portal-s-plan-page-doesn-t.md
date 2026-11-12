+++
id = "gap-a5b2e8"
kind = "gap"
title = "The portal's plan page doesn't follow the design system's one-key rule for a passed run (§4.8): it shows run again beside the orange review, a merged run gives no file count and keeps run again, an unverified or partly delivered run has no review, and a plan revised after a pass still reads passed"
status = "open"
triage = "unverified"
severity = "p3"
goal = "visibility"
size = "M"
subsystem = ["apps/portal"]
created = 2026-11-12
updated = 2026-11-12
source = "plugin-wow-v8's §4.8 update and the lead's approval (round v8, 2026-11-12)"
discovered_from = "agent:portal-v8 (round v8)"
anchors = ["apps/portal/src/components/plan/PlanPage.tsx::PlanPage", "apps/portal/src/components/plan/Result.tsx", "plans/portal-programme/09-acceptance/browser-flow.cjs"]
links = { depends_on = [], blocks = [], related = ["gap-6d2dd2", "bug-173a60"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cd apps/portal && npx vitest run src/components/plan/onekey.accept.test.tsx"
+++

## Problem

roko's design system (§4.8, updated by plugin-wow-v8 on 2026-11-12) gives a passed run one key on every surface,
the orange `review`, and the portal's plan page doesn't follow it:

- **Passed, with work to take:** the status line shows a plain `run again` beside the orange `review`.
- **Merged:** "Merged into main." without the file count, and `run again` still offered; the design says "Merged into
  main: 3 files changed." and no key.
- **Unverified or partly delivered:** no `review` key, so a person can't open the changes of a run that finished
  without every check, or that holds only part of its work.
- **Revised after a pass:** the card still says passed when a revision added tasks that never ran.

## Why it matters

Goal visibility: the run card is the one shape a run takes everywhere, and the guide shows the portal's beside the
plugin's run page and the terminal's ending.

## Where

- `apps/portal/src/components/plan/PlanPage.tsx` (keys, sentences, the details fold), `Result.tsx` (the merged
  sentence, the changes' banner), `apps/portal/src/lib/runCard.ts` (the state of a plan revised after its run).
- `plans/portal-programme/09-acceptance/browser-flow.cjs` (its revise step).

## Plan

1. A passed plan's status line has only the orange `review`; `revise` and `run again` move into the plan's
   `▸ details` fold beside "edit tasks.toml". A plan whose revision added unrun tasks reads ready ("The plan changed
   since its last run: 1 task hasn't run.") with an orange `run again`.
2. Merged: "Merged into main: 3 files changed." and no key.
3. Unverified or partly delivered: the `review` key, with the reason in its hover and as a banner in the changes.
4. The browser check opens the fold for its revise step.

## Done when

`npx vitest run src/components/plan/onekey.accept.test.tsx` passes, the CI portal set and the browser check pass,
and the shots show the passed, merged and revised states.

## Notes

- Approved by the lead (round v8, 2026-11-12), after plugin-wow-v8's §4.8 update.
