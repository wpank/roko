+++
id = "bug-636085"
kind = "bug"
title = "demo-app e2e bench-config.spec.ts uses the pre-redirect /bench route and expects ConfigWidget's pill there"
status = "done"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["demo-app/e2e"]
created = 2026-10-16
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "663342ec7"
source = "found while fixing bug-92808a, 2026-10-16"
discovered_from = "bug-92808a"
anchors = ["demo/demo-app/e2e/bench-config.spec.ts", "demo/demo-app/src/components/AppShell.tsx", "demo/demo-app/src/main.tsx"]
lane = "frontend"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q \"'/lab/bench'\" demo/demo-app/e2e/bench-config.spec.ts"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T02:18:00Z"
commit = "663342ec7"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-17T02:10:41Z"
forced = false
evidence = "Confirmed against current main.tsx/AppShell.tsx: '/bench' redirects to /lab/bench via <ToLab> (LAB_PATHS), and <ConfigWidget/> (.cw-pill) mounts only in pages/Demo/index.tsx:393, visible only at /lab/demo (AppShell.tsx's isDemo keeps it display:none elsewhere) -- the same facts bug-92808a established, re-checked here since this file was outside its anchors. bench-config.spec.ts (663342ec7): all three goto('/bench') -> goto('/lab/bench'); 'config pill is visible on bench page' renamed to 'config pill is not visible on bench page' and its assertion flipped to not.toBeVisible(), matching bug-92808a's resolution of the identical question in config-widget.spec.ts/config-sync.spec.ts; the first and third tests' .bench-model-display/.bench-model-hint/.model-picker assertions kept as-is beyond the path fix (confirmed those classes still exist in src/pages/Bench.tsx). Verify passes: grep -q \"'/lab/bench'\" demo/demo-app/e2e/bench-config.spec.ts. Static fix only, no node_modules in this worktree: Playwright itself was not run. Filed bug-425274 for the same stale-route pattern in demo/demo-app/e2e/bench-suites.spec.ts (five goto('/bench') calls, no .cw-pill assertion so likely still green via the redirect), which this item's anchors did not cover."
+++

## Problem

`demo/demo-app/e2e/bench-config.spec.ts` has the same bug class as bug-92808a, in a third file that
bug-92808a's anchors did not cover:

- All three tests `page.goto('/bench')` — a pre-redirect route. The current route table
  (`demo/demo-app/src/main.tsx`) nests Bench at `/lab/bench`; `/bench` only works via the `<ToLab>`
  redirect (`main.tsx`'s `LAB_PATHS` map, tested in `navigation.spec.ts`), landing on `/lab/bench`.
- `'config pill is visible on bench page'` then asserts `.cw-pill` (the `ConfigWidget` pill) is visible
  on that page. It never is: `<ConfigWidget />` is mounted only inside the scenario player
  (`demo/demo-app/src/pages/Demo/index.tsx:393`), and `AppShell.tsx` only displays it
  (`display: isDemo ? 'contents' : 'none'`) when `location.pathname === '/lab/demo'`. On every other
  page, including `/lab/bench`, it is either unmounted or hidden — never visible.

So this spec fails today independent of anything bug-92808a's fix touches.

## Why it matters

Goal `proof` (demo-app e2e readiness, same epic as bug-92808a/gap-a63e3c, spec-0b3a32): a red (or
silently excluded) e2e spec hides real regressions and gives false confidence when the suite reports
green. `playwright.config.ts`'s only `testIgnore` is `**/showcase/**`, so this file runs as part of the
"legacy specs" project.

## Where

- `demo/demo-app/e2e/bench-config.spec.ts` — all three tests `page.goto('/bench')`; the second test
  asserts `.cw-pill` is visible there.
- `demo/demo-app/src/components/AppShell.tsx` — the `isDemo`/`demoVisitedRef` logic that confines the
  scenario player (and therefore `ConfigWidget`) to `/lab/demo`; it is explicitly "retired from the
  showcase" and never shown on other lab pages either.
- `demo/demo-app/src/main.tsx` — the real route table (`/lab/bench`) and the `LAB_PATHS` redirect list
  that makes `/bench` work only as a redirect source, not a real route.
- `demo/demo-app/src/pages/Bench.tsx` — `.bench-model-display` / `.bench-model-hint` (first and third
  tests): these are about `Bench.tsx` itself, not `ConfigWidget`, and remain valid once the path is
  fixed.

## Current state

Checked at HEAD (2026-10-16), while fixing bug-92808a (`config-widget.spec.ts`, `config-sync.spec.ts`),
which has the identical bug pattern (stale pre-redirect routes, `.cw-pill` expected where it never
renders) but does not anchor this file. Nothing in `work/items/` or `work/done/` mentions
`bench-config.spec.ts`.

## Plan

1. Replace every `page.goto('/bench')` with `page.goto('/lab/bench')` (same fix bug-92808a applied to
   `config-sync.spec.ts`'s bench test).
2. Fix or replace the second test's premise (`'config pill is visible on bench page'`): per the current
   `AppShell.tsx`, the pill is never visible on `/lab/bench`. Follow bug-92808a's resolution for the
   identical question in `config-widget.spec.ts` / `config-sync.spec.ts`: rewrite the assertion to match
   where the widget actually renders (e.g. assert `.cw-pill` is *not* visible on `/lab/bench`, consistent
   with the sibling specs), rather than deleting the test.
3. Leave the first and third tests' `.bench-model-display` / `.model-picker` assertions as-is beyond the
   path fix — they test `Bench.tsx`, not `ConfigWidget`, and are independently valid.

## Done when

- `bench-config.spec.ts` navigates only to routes that exist today, and its `.cw-pill` assertion matches
  wherever `ConfigWidget` is actually mounted (consistent with how bug-92808a resolved the same question
  for `config-widget.spec.ts` / `config-sync.spec.ts`).
- The `[[verify]]` command passes.

## Notes

- This is a static fix: no `node_modules` is required to make the edit, only to run Playwright against it.
- See bug-92808a's resolution (closed 2026-10-16) for the precedent on how `.cw-pill` assertions were
  rewritten once `/`, `/bench`, `/demo`, `/settings`, `/explorer` were confirmed stale and `ConfigWidget`
  confirmed scenario-player-only.
