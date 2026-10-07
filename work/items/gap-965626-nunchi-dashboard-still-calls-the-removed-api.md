+++
id = "gap-965626"
kind = "gap"
title = "nunchi-dashboard still calls the removed /api/prds and enhance-prd routes"
status = "open"
triage = "verified"
severity = "p3"
size = "S"
subsystem = ["external/nunchi-dashboard"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/workflow-audit (roko-7d migration, 2026-10-02)"
discovered_from = "audit:tmp/workflow-audit/"
anchors = ["crates/roko-serve/src/routes/plans/authoring.rs::GenerateRequest"]
lane = "frontend"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f ../../nunchi-dashboard/src/services/rokoApi.ts && ! grep -qE '/prds|enhance-prd' ../../nunchi-dashboard/src/services/rokoApi.ts"
+++

## Problem

The external `nunchi-dashboard` app (github.com/Nunchi-trade/nunchi-dashboard, local checkout
`/Users/will/dev/nunchi/nunchi-dashboard`) still calls roko-serve routes that were removed with the PRD pipeline
on 2026-10-02 (`tmp/workflow-audit/`). roko-serve now answers them with a JSON 404:

- `src/services/rokoApi.ts`: `GET /prds` (:256), `GET /prds/{slug}` (:272), `GET /prds/status` (:473) and
  `POST /research/enhance-prd/{slug}` (:667)
- `src/components/atelier/`: the Atelier (PRD workshop) views built on them

## Why it matters

Pointed at a current roko-serve, the dashboard's PRD views fail. The replacements are
`POST /api/plans/generate {"prompt": "..."}` (prompt only; `slug` is refused with 422), the plan routes
(`GET/PUT /api/plans/{id}/source`) and `POST /api/research/enhance-plan/{plan}`.

## Where

- roko side (anchor): `crates/roko-serve/src/routes/plans/authoring.rs::GenerateRequest`
- dashboard side: the files above (another repository: changes go through its own PR)

## Current state

roko's own surfaces (TUI, ACP, the portal in `apps/portal`, demo-app) were migrated in the same change.

## Plan

In the dashboard repository: replace the PRD queries with plan queries, or remove the Atelier views. Will owns
that repository; ask before opening a PR there.

## Done when

- The dashboard calls no `/prds` or `enhance-prd` route. Verify (needs the sibling checkout):
  `test -f ../../nunchi-dashboard/src/services/rokoApi.ts && ! grep -qE '/prds|enhance-prd' ../../nunchi-dashboard/src/services/rokoApi.ts`

## Notes

- Found by the workflow-audit migration (session roko-7d, 2026-10-02).
