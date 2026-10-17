+++
id = "bug-425274"
kind = "bug"
title = "demo-app e2e bench-suites.spec.ts uses the pre-redirect /bench route"
status = "open"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["demo-app/e2e"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "found while fixing bug-636085, 2026-10-17"
discovered_from = "bug-636085"
anchors = ["demo/demo-app/e2e/bench-suites.spec.ts", "demo/demo-app/src/main.tsx"]
lane = "frontend"
links = { depends_on = [], blocks = [], related = ["bug-636085"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q \"'/lab/bench'\" demo/demo-app/e2e/bench-suites.spec.ts"
+++

## Problem

`demo/demo-app/e2e/bench-suites.spec.ts` has the same stale-route pattern as bug-92808a and
bug-636085, in a fourth file neither of those anchors cover:

- All seven `page.goto` calls across its five UI tests use `/bench` — a pre-redirect route.
  The current route table (`demo/demo-app/src/main.tsx`) nests Bench at `/lab/bench`; `/bench`
  only works via the `<ToLab>` redirect (`main.tsx`'s `LAB_PATHS` map, tested in
  `navigation.spec.ts`), landing on `/lab/bench`.
- Unlike bug-92808a/bug-636085's files, none of this file's assertions touch `.cw-pill` /
  `ConfigWidget`; they check `.bench-page-title`, `.suite-card`, `.bench-model-display` and the
  "RUN BENCHMARK" button, which are `Bench.tsx`'s own markup. Because `<ToLab>` is a client-side
  `<Navigate replace>`, these tests likely still pass today (Playwright's `page.goto` resolves
  before the redirect, but the assertions below it have their own timeouts and should see the
  settled `/lab/bench` page) — this was not independently confirmed with a live Playwright run.
  Two API-only tests (`bench API returns correct suite data`, `bench API returns models`) don't
  navigate at all and are unaffected.

## Why it matters

Goal `proof` (demo-app e2e readiness, same epic as bug-92808a/bug-636085/gap-a63e3c, spec-0b3a32):
even if these tests currently pass via the redirect, a stale route is fragile (it depends on
`<ToLab>`/`LAB_PATHS` continuing to list `bench` forever) and inconsistent with
`navigation.spec.ts`'s own `OLD_PATHS` -> `/lab/*` convention, which is how the other three
sibling spec files in this series were fixed.

## Where

- `demo/demo-app/e2e/bench-suites.spec.ts` — lines 5, 18, 32, 43, 54: `page.goto('/bench')` in
  the five UI tests (`bench page loads and displays suites`, `suite cards show task count`,
  `"No suites" message is NOT shown when server is running`, `bench models load from API`,
  `RUN BENCHMARK button is present`).
- `demo/demo-app/src/main.tsx` — the real route table (`/lab/bench`) and the `LAB_PATHS` redirect
  list that makes `/bench` work only as a redirect source, not a real route.
- `demo/demo-app/src/pages/Bench.tsx` — `.bench-page-title`, `.suite-card`, `.suite-card-count`,
  `.bench-model-display` are this page's own markup; not expected to need any change beyond the
  spec's path.

## Current state

Found while fixing bug-636085 (2026-10-17): a repo-wide grep for pre-redirect `goto` literals in
`demo/demo-app/e2e/` turned up `bench-suites.spec.ts` alongside the already-tracked
`config-widget.spec.ts` / `config-sync.spec.ts` (bug-92808a, done) and `bench-config.spec.ts`
(bug-636085, done). Nothing in `work/items/` or `work/done/` mentions `bench-suites.spec.ts`.
Not confirmed live (no `node_modules` in that worktree); unlike the other three files, this one
has no `.cw-pill` assertion, so it may already be green today via the client-side redirect — the
bug is the stale path, not necessarily a currently-red assertion.

## Plan

1. Replace all five `page.goto('/bench')` calls with `page.goto('/lab/bench')`.
2. Run the file against a live `npm run dev` + Playwright to confirm it still passes (this is the
   one file in the series with no `.cw-pill`/`ConfigWidget` question to resolve, so the fix should
   be a pure path substitution with no assertion rewrites).

## Done when

- `bench-suites.spec.ts` navigates only to routes that exist today.
- The `[[verify]]` command passes.

## Notes

- This is a static fix: no `node_modules` is required to make the edit, only to run Playwright
  against it.
- See bug-92808a (done, 2026-10-16) and bug-636085 (done, 2026-10-17) for the precedent and for why
  `/bench` redirects but is still considered worth fixing.
