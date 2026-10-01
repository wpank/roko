+++
id = "bug-5e71d0"
kind = "bug"
title = "Disabled portal buttons explain themselves only in tooltips that can never show (pointer-events: none)"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["apps/portal"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "270526b3a"
source = "plan:portal-programme/09-acceptance#T04"
discovered_from = "plan:portal-programme/09-acceptance#T04"
anchors = ["apps/portal/src/components/atoms/Button.tsx:70", "apps/portal/src/styles/globals.css:425", "apps/portal/src/components/rail/PlanRail.tsx:139"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqF --include='*.test.ts' --include='*.test.tsx' 'disabled reason is visible' apps/portal/src && (cd apps/portal && npx vitest run src/components)"

[closed]
at = 2026-09-29
commit = "270526b3a"
by = "commit trailer"
evidence = "270526b3a: button:disabled (globals.css) and Button's disabled classes no longer set pointer-events: none; disabled keeps cursor not-allowed and hover styling applies only to enabled buttons. apps/portal/src/components/disabledReason.test.tsx ('disabled reason is visible', 2 tests) renders the rail (Run all, group ▶) and the plan view while running (✦ Revise, ✎ Edit) with the portal's button rules applied: each button is disabled, carries its reason as its accessible description, and computes pointer-events other than none with cursor not-allowed; both tests fail on the parent commit. [[verify]] passes (vitest src/components: 30 files, 167 tests)."
+++

## Problem

`Button` adds `pointer-events-none` when disabled (`Button.tsx:70`), and `globals.css:425-429` sets
`button:disabled { pointer-events: none }`. A disabled button therefore never receives hover, and its
`title` never shows. Reasons lost this way:

- ✦ Revise and ✎ Edit while the plan runs ("Cannot revise while the plan is running", in
  `PlanView.tsx`).
- The rail's ▶ Run all and group ▶ while a run is active (`PlanRail.tsx:139` and `:342`).

The plan's own Run shows its reason as visible text (design §4.1), so only these lose theirs.

## Why it matters

Goal `visibility`. A greyed-out button with no reason reads as a broken UI.

## Where

`apps/portal/src/components/atoms/Button.tsx`, `apps/portal/src/styles/globals.css`, and the rail's
run buttons.

## Plan

Put the title on a wrapper that receives hover, or render the reason as text as Run does, or drop
`pointer-events: none` for disabled buttons and keep `cursor: not-allowed`.

## Done when

A test whose name contains "disabled reason is visible". The `[[verify]]` runs it.

## Notes

Found by plan 09 T04 (see VERDICT).
