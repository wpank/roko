+++
id = "bug-979636"
kind = "bug"
title = "The rail filter changes what runs and what is selected: group Run starts only matching plans, and a hidden selection is cleared"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["apps/portal"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "f99e45dba"
source = "plan:portal-programme/09-acceptance#T04"
discovered_from = "plan:portal-programme/09-acceptance#T04"
anchors = ["apps/portal/src/lib/planRows.ts::buildPlanRows", "apps/portal/src/components/rail/PlanRail.tsx:140", "apps/portal/src/components/rail/PlanRail.tsx:343", "apps/portal/src/components/shell/Workspace.tsx:89", "apps/portal/src/components/shell/Header.tsx::Header"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqF --include='*.test.ts' --include='*.test.tsx' 'filter does not change' apps/portal/src && (cd apps/portal && npx vitest run src/lib/planRows src/components/rail src/components/shell)"
+++

## Problem

`buildPlanRows` applies the rail filter (`/`) first, then derives `order`, `runningPlanIds` and
`count` (`count: filtered.length`) from the filtered rows. Actions and state that should not depend
on a view filter consume them:

- A group's ▶ runs `group.rows`, which holds only the plans the filter shows, and its confirm says
  "the N plans in <group>" with the filtered N (`PlanRail.tsx:343-345`).
- ▶ Run all posts `{}`, which runs every plan, while its confirm says "Run all N plans" with the
  filtered N (`PlanRail.tsx:140`).
- The header's run summary and its ■ come from `runningPlanIds`. A filter that hides the running
  plans therefore hides the run and the only header cancel. The ■ confirm and the header's step to
  the next running plan count only visible plans.
- `resolveSelection` receives `planIds: rows.order` (`Workspace.tsx:89-101`). A filter that hides the
  selected plan therefore clears `?plan=` from the URL, and may select a running plan the filter
  shows; clearing the filter does not restore the selection.

Found by reading the code in the 09 T04 design audit; not observed in a browser run.

## Why it matters

Goal `visibility`. Design §3 makes the filter a view, §9 says the selection never jumps on its own,
and §12 says Run all and group runs confirm what they run. A confirm that misstates what will run
starts paid agent work the operator did not intend.

## Where

The anchors: `apps/portal/src/lib/planRows.ts::buildPlanRows` (filter, then `order`,
`runningPlanIds`, `count`), `components/rail/PlanRail.tsx` (Run all at :140, group ▶ at :343),
`components/shell/Workspace.tsx:89` (selection) and `components/shell/Header.tsx` (run summary).

## Current state

Unit tests cover the filter's matching (`planRows.test.ts`), not its effect on runs, the header or
the selection.

## Plan

Compute `order`, `runningPlanIds` and the counts from all rows, and apply the filter only to what
the rail renders. A group ▶ runs the whole group, or its confirm says "N of M, matching the filter".
Resolve the selection against every plan id.

## Done when

Tests whose names contain "filter does not change" cover the group ▶ ids, the Run all confirm count,
the header's running summary and the selection, each with a filter set. The `[[verify]]` runs them.

## Notes

Found by plan 09 T04 (see VERDICT).
