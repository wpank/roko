+++
id = "gap-bb4207"
kind = "gap"
title = "A passed run card's pads read differently: phase 7's B still shows pad 01 'passed' and pad 02 '1m 20s', because a passed pad shows its time only when its row has an actual time"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["apps/portal"]
created = 2026-11-13
updated = 2026-11-13
last_verified = 2026-11-13
source = "round v8 portal-v8, phase 7's capture (2026-11-12); the lead asked for it filed"
discovered_from = "agent:portal-v8 (round v8)"
anchors = ["apps/portal/src/lib/runCard.ts::padFoot"]
links = { depends_on = [], blocks = [], related = ["gap-c1726f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cd apps/portal && npx vitest run src/lib/runCard.test.ts"
+++

## Problem

On one passed run card, the task pads' feet disagree: one pad reads "passed" and the other reads its time.
Phase 7's B still (run 31e52490, `captures/three-surfaces/portal-raw/p7-31e52490/web-B-day.png`) shows pad 01
"passed" and pad 02 "1m 20s". `padFoot` shows a passed pad's duration only when its row has an actual time, and
"passed" otherwise. T1's row had no actual time when the run ended; T2's did.

## Why it matters

Goal visibility, low: the guide's stills show the card, and a reader takes the two words to mean different things.
The design system's §4.7 gives a passed pad one foot.

## Where

`apps/portal/src/lib/runCard.ts::padFoot`, and where a task row gets its time (`lib/planRows.ts`, `lib/runState.ts`).

## Plan

Give every passed pad the same foot: its time when the run recorded one for every task (its settled tries'
`duration_ms`), else "passed" for all of them. Test it with phase 7's events.

## Done when

- A passed card's pads read alike.
- The `[[verify]]` passes.

## Notes

- Round v8 portal-v8, seen in phase 7's capture; the lead asked for it filed as p3.
