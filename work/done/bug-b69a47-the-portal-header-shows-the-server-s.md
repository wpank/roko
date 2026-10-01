+++
id = "bug-b69a47"
kind = "bug"
title = "The portal header shows the server's lifetime cost instead of the running run's cost"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["apps/portal"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "3fb11e23e"
source = "plan:portal-programme/09-acceptance#T04"
discovered_from = "plan:portal-programme/09-acceptance#T04"
anchors = ["apps/portal/src/lib/runState.ts:968", "apps/portal/src/lib/runState.ts:1268", "apps/portal/src/components/shell/Header.tsx:116"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqF --include='*.test.ts' --include='*.test.tsx' 'cost restarts with each run' apps/portal/src && (cd apps/portal && npx vitest run src/lib/runState src/components/shell)"

[closed]
at = 2026-09-29
commit = "3fb11e23e"
evidence = "Header.tsx shows runCostUsd(run), the cost of the current run's plans (plan set, else running plans), each reset with its run, instead of run.totals.costUsd. [[verify]] passes at 3fb11e23e: Header.cost.test.tsx ('cost restarts with each run': a second run starts at $0, then shows $0.21 not $10.41; after a reload it reads the snapshot's plan costs, not stats.cost_usd_total) and runState.cost.test.ts; the header test fails when the header reads run.totals.costUsd again."
+++

## Problem

While anything runs, the header shows `formatCost(run.totals.costUsd)` (`Header.tsx:116`).
`totals.costUsd` starts from the snapshot's `stats.cost_usd_total`, which is every cost the server has
recorded since it started (`runState.ts:1268-1272`), and adds the cost of every `efficiency_event`
after that (`runState.ts:968-976`). Nothing resets it when a run starts: `plan_set_loaded` and
`plan_started` do not touch `totals`. After a server has run $10 of plans, the header of the next run
reads $10.xx. Design §2 and §12 put the run's cost there
(`▶ hello-world 2/3 · 1m12s · ~2m · $0.21`).

## Why it matters

Goal `visibility`. The header is the only place the portal shows spend (the run band's BURN cell
deliberately shows tokens only), so a cumulative number there misreports every run after the first.

## Where

`apps/portal/src/lib/runState.ts` (the `totals` fold and `fromSnapshot`) and
`apps/portal/src/components/shell/Header.tsx`.

## Current state

The per-plan cost (`PlanRun.costUsd`) is folded correctly from live events. It is lost on reload,
because the snapshot has no per-plan cost (see the reload item filed alongside this one).

## Plan

Show the run's cost: sum the `costUsd` of the plans in the current run (the plan set, else the
running plan), or reset a run baseline on `plan_set_loaded` or the first `plan_started`.

## Done when

A test whose name contains "cost restarts with each run" shows that a second run's header starts at
its own cost. The `[[verify]]` runs it.

## Notes

Found by plan 09 T04 (see VERDICT).
