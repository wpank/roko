+++
id = "bug-f8c943"
kind = "bug"
title = "The portal's SPENT stands still while tasks work: it counts a try only when the try settles, so 7.2's web glass read $0.15 while Hermes, counting calls as they return (call_cost), read $0.84"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["apps/portal"]
created = 2026-11-13
updated = 2026-11-13
last_verified = 2026-11-13
source = "round v8 portal-v8, phase 7.2's capture (2026-11-13)"
discovered_from = "agent:portal-v8 (round v8)"
anchors = ["apps/portal/src/lib/runState.ts", "apps/portal/src/lib/runCard.ts"]
links = { depends_on = [], blocks = [], related = ["gap-c1726f", "gap-5c3347"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cd apps/portal && npx vitest run src/components/plan/readings.accept.test.tsx -t 'moves as each call returns'"
+++

## Problem

The portal's SPENT counts a try's spend only when the try settles. A run's spend therefore stands still while its
tasks work. On phase 7.2's run (583b4be5), the web glass read "$0.15 of $5.00" at 00:53:24 UTC while three tasks
were mid-try. Hermes read "spent $0.84 of $5.00" at 00:53:50. Hermes counts each model call as it returns
(`call_cost`, gap-5c3347), and roko's own run summary counts the live calls too (`live_cost_usd`). The portal folded
`efficiency_event` alone, and that comes once per settled try.

## Why it matters

Goal visibility: SPENT is one of the four readings. The guide's three-surface section claims they match, and a
person watching a long try sees the money stand still.

## Where

`apps/portal/src/lib/runState.ts` (the event fold), `apps/portal/src/lib/runCard.ts` (the SPENT reading).

## Plan

SPENT is what the run's calls have cost so far, each priced as it returns:
- each settled try's cost;
- each running try's latest `call_cost.attempt_cost_usd`, until the try settles and its own cost replaces it;
- on a resume, what earlier processes spent.

Test: calls move SPENT, and a settled try counts once.

## Done when

- SPENT moves as calls return, and a settled try isn't counted twice.
- The `[[verify]]` passes.

## Notes

- Round v8 portal-v8, found in 7.2's capture; gap-c1726f's TIME and CHECKS definitions, extended to SPENT.
