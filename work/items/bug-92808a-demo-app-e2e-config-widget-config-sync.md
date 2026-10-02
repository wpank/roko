+++
id = "bug-92808a"
kind = "bug"
title = "demo-app e2e config-widget/config-sync specs use pre-redirect routes and expect ConfigWidget where it is never mounted"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["demo-app/e2e"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "backlog wave reports 2026-10-02 (PK81 gap-a63e3c)"
discovered_from = "gap-a63e3c"
anchors = ["demo/demo-app/e2e/config-widget.spec.ts", "demo/demo-app/e2e/config-sync.spec.ts", "demo/demo-app/src/pages/Demo/index.tsx::ConfigWidget", "demo/demo-app/e2e/navigation.spec.ts"]
lane = "frontend"
parent = "spec-0b3a32"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q \"'/lab/bench'\" demo/demo-app/e2e/config-widget.spec.ts"
+++

## Problem

`demo/demo-app/e2e/config-widget.spec.ts` and `config-sync.spec.ts` navigate to `/`, `/bench`, `/demo`, `/settings`
and `/explorer` and expect `.cw-pill` (the `ConfigWidget` pill) to be visible there. Neither premise holds:

- `<ConfigWidget />` is mounted in exactly one place in the whole app:
  `demo/demo-app/src/pages/Demo/index.tsx:393`, the scenario player rendered at `/lab/demo`. It is not in
  `AppShell.tsx` (the shared shell every route renders through) or in `Overview`, `Bench`, `Settings` or
  `Explorer`, so `.cw-pill` cannot appear on any of the pages these two spec files visit.
- `/bench`, `/settings`, `/explorer` and `/demo` are not even current routes. `demo/demo-app/src/main.tsx:139-161`
  nests those pages under `/lab/*` (`/lab/bench`, `/lab/settings`, `/lab/explorer`); `/demo` has no route at all.
  `demo/demo-app/e2e/navigation.spec.ts:24` names exactly these as `OLD_PATHS`, redirected to their `/lab/*`
  equivalent by `<ToLab>` (`main.tsx:165`) — `navigation.spec.ts` was updated for this redirect (it is one of
  gap-a63e3c's own anchors), but `config-widget.spec.ts` / `config-sync.spec.ts` were not.

So both files fail today (or did whenever they last ran) independent of anything PK81 (gap-a63e3c) changed: `/`
never renders the pill, and the other four paths are stale pre-redirect routes that, even once redirected, land
on pages that still don't mount `ConfigWidget`.

## Why it matters

Goal `proof` (demo-app readiness, same epic as gap-a63e3c, spec-0b3a32): a demo-app e2e suite with permanently
red (or silently excluded) specs hides real regressions in the config-pill feature and gives false confidence
when the suite is reported green. `playwright.config.ts`'s only `testIgnore` is `**/showcase/**`, so these two
files are not excluded — they run as part of the "legacy specs" project.

## Where

- `demo/demo-app/e2e/config-widget.spec.ts` — all 5+ tests `page.goto` one of `/`, `/bench`, `/demo`, `/settings`,
  `/explorer` and assert on `.cw-pill`/`.cw-panel`/`.cw-section`.
- `demo/demo-app/e2e/config-sync.spec.ts` — same pattern, plus `/bench`'s `.bench-model-display`/`.bench-model-hint`
  assertions (those may be independently valid for `Bench.tsx`, but still navigate to the stale `/bench` path).
- `demo/demo-app/src/pages/Demo/index.tsx:393` — the only `<ConfigWidget />` mount point.
- `demo/demo-app/src/main.tsx:131-165` — the real route table (`/` → `Overview`, `/lab/bench`, `/lab/settings`,
  `/lab/explorer`, `/lab/demo`).
- `demo/demo-app/e2e/navigation.spec.ts:7-24` — `OLD_PATHS`/`ToLab`, the precedent for how the redirect is tested.

## Current state

Checked at HEAD (2026-10-02). `gap-a63e3c` (done) updated `navigation.spec.ts` and the route/redirect code but did
not touch `config-widget.spec.ts` or `config-sync.spec.ts`; its anchors list confirms this. Nothing else in
`work/items/` or `work/done/` mentions either file.

## Plan

1. Decide the right fix for each assertion:
   - If the config pill is meant to be global (visible on every page, as `config-widget.spec.ts`'s own test name
     "pill is visible on every page" suggests), mount `<ConfigWidget />` in `AppShell.tsx` instead of only in
     `Demo/index.tsx`, and update both spec files' paths to the current `/lab/*` routes (and `/` if it should
     stay global, or drop `/` if the pill is meant to be lab-only).
   - If the pill is meant to stay scenario-player-only (current behavior), rewrite both spec files to test that
     scope instead of a global one, and fix their paths to `/lab/demo`, `/lab/bench`, `/lab/settings`,
     `/lab/explorer`.
2. Either way, replace every stale `/bench`, `/settings`, `/explorer`, `/demo` literal with its `/lab/*` route,
   matching `navigation.spec.ts`'s `OLD_PATHS` → `/lab/*` mapping.
3. Run the two spec files locally against a dev server to confirm they pass before relying on CI.

## Done when

- `config-widget.spec.ts` and `config-sync.spec.ts` navigate only to routes that exist today, and their
  `.cw-pill` assertions match wherever `ConfigWidget` is actually mounted.
- The `[[verify]]` command passes.

## Notes

- `config-sync.spec.ts`'s `.bench-model-display` / `.bench-model-hint` checks are about `Bench.tsx` itself, not
  `ConfigWidget` — keep those, just fix the path to `/lab/bench`.
- Don't assume which fix (global pill vs. scenario-local) is correct without checking product intent; both are a
  real option given the current code.
