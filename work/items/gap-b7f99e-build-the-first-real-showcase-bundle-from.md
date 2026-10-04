+++
id = "gap-b7f99e"
kind = "gap"
title = "Build the first real showcase bundle from the pilot runs and test R1 against it, tasks 9315 and 9316"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
hold = "needs the pilot runs: Pilot A (gap-c33709) and Pilot B (gap-327242)"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "tmp/backlog/2026-10-02-complete-and-wire 9315, 9316 (held in wave 12, PK82)"
discovered_from = "gap-9ecd37"
anchors = ["benchmarks/viabilitybench/showcase"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["gap-9ecd37", "gap-c33709", "gap-327242"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 benchmarks/viabilitybench/showcase/verify_bundle.py .roko/showcase/bundles/b-pilot-p1 && grep -q '\"kind\": \"replay\"' .roko/showcase/bundles/b-pilot-p1/bundle.json"

[[verify]]
command = "test -f demo/demo-app/scripts/stage-showcase-bundle.mjs && test -f demo/demo-app/scripts/showcase-bundle-budget.mjs && cd demo/demo-app && npx playwright test -c playwright.showcase-static.config.ts && node scripts/showcase-bundle-budget.mjs"
+++

## Problem

Tasks 9315 and 9316 of PK82 (gap-9ecd37, gated in wave 12) build, verify and keep the first real showcase bundle from
the pilot runs (the golden path, `.roko/showcase/bundles/b-pilot-p1`), then run the `showcase-static` Playwright config,
the R1 specs and the bundle budget against it. No pilot has run, so there is no bundle to build.

## Why it matters

The showcase's first view (R1) is read from this bundle; until it exists the static showcase can't be tested on real
data or deployed (PK87).

## Where

`benchmarks/viabilitybench/showcase/build_bundle.py` and `verify_bundle.py` (9314, merged), the pilot records under
`$VB_RESULTS`, and `demo/demo-app` (`scripts/stage-showcase-bundle.mjs`, `scripts/showcase-bundle-budget.mjs`,
`playwright.showcase-static.config.ts`).

## Current state

The builder and verifier pass on a fixture bundle; Pilots A (gap-c33709) and B (gap-327242) haven't run.

## Plan

1. After the pilot runs, build and verify `b-pilot-p1` as 9315 says, and keep it.
2. Write the staging and budget scripts and the R1 specs as 9316 says, and run them against the staged bundle.

## Done when

- [ ] Both `[[verify]]` commands pass.

## Notes

- Full specs: `tmp/backlog/2026-10-02-complete-and-wire/9315-*.md` and `9316-*.md`.
- Left PK82's package item at gate 12b (2026-10-04).
