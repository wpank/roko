+++
id = "bug-64fb48"
kind = "bug"
title = "While a plan generates, the portal requests the unwritten plan every second, logging one 404 per second"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["apps/portal"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "1c65a8c46"
source = "plan:portal-programme/09-acceptance#T04"
discovered_from = "plan:portal-programme/09-acceptance#T04"
anchors = ["apps/portal/src/lib/operation.ts::waitForOperation", "apps/portal/src/api/queries.ts::planExists"]
links = { depends_on = [], blocks = [], related = ["bug-a0f01e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqF --include='*.test.ts' 'does not poll the plan while the operation runs' apps/portal/src && (cd apps/portal && npx vitest run src/lib/operation)"

[closed]
at = 2026-09-29
commit = "1c65a8c46"
by = "commit trailer"
evidence = "waitForOperation (apps/portal/src/lib/operation.ts) now leaves a known operation to finish in both modes, and asks GET /api/plans/{id} only when the operation is unknown (404: swept after finishing, or lost in a restart). The item's other fallback, a server that never reports result, cannot be reached: generate operations that never finish predate plan 04, and those servers reject {prompt} with a 400 that the portal shows as an outdated server (authoring.accept.test.tsx). Proven by operation.test.ts 'does not poll the plan while the operation runs' (the [[verify]]; the old code requested the plan on each of 3 running polls) and its 404-fallback test, plus PromptPanel.test.tsx, which drives the panel against a stubbed server and sees no GET /api/plans/hello while the operation runs (fails on the old code). tsc, vitest (63 files, 689 tests), orphans and build:export pass."
+++

## Problem

`waitForOperation` (expecting `new-plan`) polls `GET /api/operations/{id}` every second. While the
operation is `running`, it also calls `planExists(planId)`, a `GET /api/plans/{id}` that answers 404
until the plan is written. The real-model run's console
(`tmp/portal-audit/evidence/hello-world-real-run1/browser-real.json`) holds 30 "404 (Not Found)" entries
for a 29 s generation. The 11:40 re-run holds 22 for a 20.7 s generation
(`tmp/portal-audit/evidence/hello-world-real/browser-real.json`).

Since plan 04, the server finalizes a generate operation with `status: completed` and `result.slug`.
The plan poll is only needed for servers whose operations never finish (bug-a0f01e covers other
operation producers).

## Why it matters

Goal `visibility`. The noise hid real errors: the validate 400s filed alongside this item went
unnoticed through every preview. Each poll is also a wasted request.

## Where

`apps/portal/src/lib/operation.ts::waitForOperation` and `apps/portal/src/api/queries.ts::planExists`.

## Plan

Poll the plan only when the operation is unknown (404), or when the server does not report
`result`. Otherwise wait for `completed` or `failed`.

## Done when

A test in `operation.test.ts` whose name contains "does not poll the plan while the operation runs".
The `[[verify]]` runs it.

## Notes

The portal-check parallel flow (`browser-parallel.json`) logged one 404 without any generation; a
scratch probe did not reproduce it. Found by plan 09 T04 (see VERDICT).
