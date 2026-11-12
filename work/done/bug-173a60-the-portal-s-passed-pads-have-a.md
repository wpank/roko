+++
id = "bug-173a60"
kind = "bug"
title = "The portal's passed pads have a green border and its steps stretch across the page, unlike roko's design system (§4.7: a passed pad keeps its plain outline; the kit caps a pad at 240px), so its run card doesn't read like the plugin's"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["apps/portal"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "bfa5867d4"
source = "plugin-wow-v8 (round v8), on the portal's run card (2026-11-12)"
discovered_from = "agent:plugin-wow-v8 (round v8)"
anchors = ["apps/portal/src/styles/portal.css", "apps/portal/src/styles/roko-kit.css"]
links = { depends_on = [], blocks = [], related = ["gap-60f6b6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cd apps/portal && npx vitest run src/styles/roko.accept.test.ts"

[closed]
at = 2026-11-12
at_ts = "2026-11-12T21:10:00Z"
commit = "bfa5867d4"
by = "portal-v8 (round v8)"
executor = "claude-agent"
via = "manual"
model = "claude-opus-5-5"
size = "S"
claimed_at = "2026-11-12T20:45:00Z"
forced = false
evidence = "apps/portal/src/styles: roko-kit.css takes the kit's .rk-seq-grid rule (minmax(152px, 240px), justify-content: start); portal.css keeps its 220px floor under the 240px cap, drops the green border on a passed pad, and lets steps fill a phone's width. At bfa5867d4's tree in roko-work-bug-173a60: tsc clean, vitest 727/727 (roko.accept: 'leave a passed pad its plain outline', 'cap a pad at the kit's 240px'), orphans none, build:export OK, plan validate over the portal plans without errors, the CI browser check PASS 37 of 37. Shots: tmp/hermes-roko-v8/design/portal/pads/ (passed pads plain at 240px, phone steps full width)."
+++

## Problem

The portal draws a plan's steps unlike roko's design system, so its run card doesn't read like the plugin's:

- **A passed pad has a green border.** `apps/portal/src/styles/portal.css` sets
  `.rk-pad[data-state="pass"] { border-color: var(--rk-pass) }`. The design system (§4.7) keeps a passed pad's plain
  outline with a green LED and green pips; only running, failed and needs-you pads get a coloured outline, so a
  finished plan reads calm.
- **Pads stretch across the page.** The portal's `.pt-seq .rk-seq-grid` gives each step `minmax(220px, 1fr)`, so
  two steps on a wide page become bars. The kit now caps a pad at 240px (`minmax(152px, 240px)`,
  `justify-content: start`), as the plugin's sequencer does, and the portal's copy of the kit predates that.

Found by plugin-wow-v8 (round v8), who owns the design system.

## Why it matters

Goal visibility: the run card is the one shape a run takes on every surface, and the guide shows the portal's beside
the plugin's.

## Where

- `apps/portal/src/styles/portal.css` (the pass border, the step width), `apps/portal/src/styles/roko-kit.css` (the
  portal's copy of `$V8/design/kit/roko-kit.css`).

## Plan

1. Copy the kit's `.rk-seq-grid` rule into the portal's kit copy.
2. Drop the pass border; keep the portal's 220px floor under the kit's 240px cap.
3. A style test pins both: no rule colours a passed pad's border, and the kit copy matches the kit's step width.

## Done when

`npx vitest run src/styles/roko.accept.test.ts` passes with those cases, and the passed shots show plain pads at
the kit's width.
