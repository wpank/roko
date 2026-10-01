+++
id = "spec-c3f994"
kind = "spec"
title = "PB-002: ROSEDUST design system tokens"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["apps/portal"]
created = 2026-09-23
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/portal-backlog/PB-002-rosedust-tokens.md#PB-002"
discovered_from = "audit:tmp/portal-backlog/PB-002-rosedust-tokens.md#PB-002"
anchors = ["apps/portal/src/styles/rosedust.css", "apps/portal/src/styles/globals.css", "apps/portal/src/lib/motion-tokens.ts", "apps/portal/src/styles/atmosphere.css"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f apps/portal/src/styles/rosedust.css && test -f apps/portal/src/lib/motion-tokens.ts && grep -q -- '--radius-sm: *0px' apps/portal/src/styles/globals.css"

[closed]
at = 2026-09-28
commit = "91b4745f8"
evidence = "Built: src/styles/rosedust.css defines the ROSEDUST custom properties. globals.css maps them into a Tailwind v4 @theme block, which is the CSS-first replacement for tailwind.config.ts, and sets every radius token to 0px (:111-117). src/lib/motion-tokens.ts has outExpo [0.16,1,0.3,1], inOutCubic [0.4,0,0.2,1] and 80/200/350 ms durations. src/styles/atmosphere.css has CSS-only grain and scanlines. Landed in 9c6ec420c. plans/portal-programme/05-portal-foundation T08 builds tokens.css on top of this palette."
+++
ROSEDUST design system tokens. Implement the ROSEDUST design system as CSS custom properties and Tailwind config extensions. This is the visual foundation — every component inherits these tokens.

Imported without verification from:
- `tmp/portal-backlog/PB-002-rosedust-tokens.md#PB-002`

How to verify: Check portal app for this feature (9 acceptance criteria, e.g. `src/styles/rosedust.css` with all CSS custom properties:; `tailwind.config.ts` extends colors from CSS variables (e.g., `rose: 'var(--rose)'`)); cross-check plans/portal-programme/* and existing web app dirs.

Verified 2026-09-28: built (rosedust.css, @theme, motion tokens). See [closed].
