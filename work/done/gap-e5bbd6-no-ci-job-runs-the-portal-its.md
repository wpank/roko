+++
id = "gap-e5bbd6"
kind = "gap"
title = "No CI job runs the portal: its unit tests, type check, static export and the fake-agent browser check run only inside roko plans"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
size = "M"
subsystem = ["ci", "apps/portal"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "4cbd49d76"
source = "plan:portal-programme/09-acceptance#T04"
discovered_from = "plan:portal-programme/09-acceptance#T04"
anchors = [".github/workflows/ci.yml", "apps/portal/package.json", "plans/portal-programme/09-acceptance/portal-check.sh"]
links = { depends_on = [], blocks = [], related = ["gap-2122bd", "find-8cc7ac", "spec-87be33"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'apps/portal' .github/workflows/ci.yml && grep -q 'build:export' .github/workflows/ci.yml && grep -rq 'portal-check.sh' .github/workflows/"

[closed]
at = 2026-09-29
commit = "4cbd49d76"
by = "rs-release (roko-b6)"
evidence = "4cbd49d76: ci.yml gains a portal job (npm ci, typecheck, vitest, scripts/orphans.mjs, build:export, roko-portal marker check) and a portal-browser job (cargo build -p roko-cli, portal export, demo build, Playwright Chromium, plans/portal-programme/09-acceptance/portal-check.sh, evidence artifact; continue-on-error until it has a record on Linux). Local checks on macOS: typecheck, 676 vitest tests, orphans and build:export pass; with this branch's debug build portal-check.sh prints PORTAL-CHECK: PASS (35 checks) in about 100 s, also with no git identity and a fresh HOME; actionlint reports nothing for ci.yml; [[verify]] passes. Only a GitHub Actions run proves the jobs on ubuntu-latest."
+++

## Problem

`.github/workflows/ci.yml` has four jobs (`test`, `fmt`, `layer-check`, `cli-feature-matrix`), and
none touches `apps/portal`. No workflow runs any of these:

- `npm test`: 61 files and 676 tests on 2026-09-29.
- `npm run typecheck`.
- `npm run build:export`.
- `plans/portal-programme/09-acceptance/portal-check.sh`: free with the fake agent, about 90 s, needs
  a debug `roko` build and a headless Chromium.

Only `release.yml` runs npm, and only for `demo/demo-app`. The portal's gates ran only inside roko
plans 05–08g and 09.

## Why it matters

Goal `release`. In this programme every 05–08f gate was green while the first browser run found
three first-run blockers (fixed by 08g). The 09 checks passed while every validate call the portal
made was rejected (filed alongside this item). Without CI, the next change to roko-serve's contract
or to the portal can break the two-action flow silently.

## Where

`.github/workflows/ci.yml`; the scripts in `apps/portal/package.json`; the check in
`plans/portal-programme/09-acceptance/` and its harness `plans/portal-programme/_harness/`.

## Plan

1. A job in `apps/portal`: `npm ci`, `npm test`, `npm run typecheck`, `npm run build:export`.
2. A job that builds `roko-cli` (debug), installs Playwright's Chromium in `demo/demo-app`, and runs
   `portal-check.sh` (no API keys needed). Upload `tmp/portal-audit/evidence/portal-check/` as an
   artifact.
3. Consider making `browser-flow.cjs` fail on console errors.

## Done when

`ci.yml` runs the portal suite and the export, and a workflow runs `portal-check.sh`. The
`[[verify]]` checks the workflow files.

## Notes

gap-2122bd covers release and Docker builds shipping without the portal export. spec-87be33 (parked)
is an older, unverified "Playwright E2E suite" idea. Found by plan 09 T04 (see VERDICT).
