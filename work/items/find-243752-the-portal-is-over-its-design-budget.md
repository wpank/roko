+++
id = "find-243752"
kind = "finding"
title = "The portal is over its design budget: 22,684 lines in 122 files against about 9,000 in about 65"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "L"
subsystem = ["apps/portal"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "f99e45dba"
source = "plan:portal-programme/09-acceptance#T04"
discovered_from = "plan:portal-programme/09-acceptance#T04"
anchors = ["apps/portal/src/lib/runState.ts", "apps/portal/src/styles/globals.css", "plans/portal-programme/09-acceptance/VERDICT.md"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test \"$(find apps/portal/src -type f ! -name '*.test.ts' ! -name '*.test.tsx' -exec cat {} + | wc -l)\" -le 6750"
+++

## Problem

Measured on 2026-09-29 (09 T04): `apps/portal/src` holds 122 files (61 test files, 61 others) and
22,684 lines (9,688 in tests, 12,996 in the rest). `tmp/portal-audit/02-DESIGN.md` §14 budgeted about
65 files (17 of them unit-test files) and about 9,000 lines, "about a quarter tests", which leaves
about 6,750 lines of non-test code. The measurement before the programme was 111 files and 33,286
lines.

The other budget lines are met: 1 route, 0 navigation systems, 6 runtime dependencies, 2 actions to
build hello world, and 0 clicks to the live transcript.

The largest non-test files: `lib/runState.ts` 1,301 lines, `styles/globals.css` 1,081,
`lib/streamRecord.ts` 584, `components/stage/PlanView.tsx` 521, `api/contracts.ts` 413,
`api/sse-client.ts` 407, `components/shell/Workspace.tsx` 381, `components/rail/PlanRail.tsx` 377,
`components/stage/SourceEditor.tsx` 366, `components/stream/StreamPane.tsx` 365.

## Why it matters

The design argued that one screen needs about a quarter of the old code. Non-test code is 1.9× its
budget; tests are 4.3× theirs, which is not a problem in itself.

## Where

`apps/portal/src`. Budget: `tmp/portal-audit/02-DESIGN.md` §14. Measurement: `VERDICT.md` in plan 09.

## Plan

Decide whether the budget stands. If it does, start with the files above: `runState.ts` folds every
event type, and `globals.css` carries both the Tailwind layer and hand-written rules. If it does not,
close this item as wontfix and record the decision.

## Done when

Non-test code under `apps/portal/src` is at or under 6,750 lines (the `[[verify]]`), or a decision
revises the budget.

## Notes

Lines were counted with `cat | wc -l` over every file; files were counted with `find -type f`.
Found by plan 09 T04 (see VERDICT).
