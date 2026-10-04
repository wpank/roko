+++
id = "gap-dbe8e7"
kind = "gap"
title = "The bundle builder's R1 views don't follow the page's contracts: overview tiles lack pillar and claim_state, head-to-head and m4-audits crash the page, and the provenance lacks the drawer's fields"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "M"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "gate 13c: PK86's fly-smoke spec (9338) against build_bundle.py's overview (9314)"
discovered_from = "gap-3516d6"
anchors = ["benchmarks/viabilitybench/showcase/build_bundle.py"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["gap-9ecd37", "gap-3516d6", "gap-fcb44c", "gap-b7f99e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/showcase/test_bundle.py -q -k 'overview_tiles_follow_the_claims_board or r1_views_follow_the_page_contracts'"

[[verify]]
command = "test -f demo/demo-app/playwright.fly-smoke.config.ts && test -f demo/demo-app/e2e/showcase/fly-smoke.spec.ts && cd demo/demo-app && npx playwright test -c playwright.fly-smoke.config.ts"
+++

## Problem

`build_bundle.py`'s overview view (task 9314, PK82) isn't the claims board the page draws. Its `tiles` are one per
MetricRecord (`metric`, `arms`, `ladder`, `value`, `ci`, `n`, `metric_ref`), while S10 §5.2 gives
`/api/showcase/overview` as `{tiles:[{id, pillar, hypothesis, claim_state, value, ci, n, metric_ref}], negatives:[…]}`
and the demo app's `OverviewTile` (`demo/demo-app/src/showcase/contracts.ts`) wants `id`, `pillar` (P1 or P2),
`mechanism`, `title`, `hypothesis`, `claim_state`, `rows` (TileRow, each naming a metric_ref), `planned_in` and `view`.
`Overview.tsx` keeps the tiles whose `pillar` matches, so a built bundle's Overview renders with no tile at all.
Gate 13c found it: after the `metrics` index fix (012c75cf1) the Overview no longer refuses, but PK86's fly-smoke spec
(A1-A4) waits for a `[data-tile]` that never appears.

## Why it matters

The Overview is the showcase's first view (R1). No real bundle (the pilot's, gap-b7f99e) can show its claims until
the builder writes them in the page's shape; the hand-written fixtures hide it.

## Where

`benchmarks/viabilitybench/showcase/build_bundle.py` (`project_views`) and `verify_bundle.py` (it re-derives the
views), `showcase/test_bundle.py`, `demo/demo-app/src/showcase/contracts.ts` (`OverviewTile`, `TileRow`),
`demo/demo-app/src/pages/showcase/Overview.tsx`, the view JSON schemas in the demo app, and S10 §4.3 A and §5.2.

## Current state

At the gate-13c merge: the builder's views carry `metrics` and `negatives`, the guard admits them, and the Overview
draws the badges and the negatives strip but no tiles.

## Plan

1. Write the R1 tile catalogue the builder projects into (the P1 tiles and the M4 tiles of S10 §4.3 A): each tile's
   id, pillar, mechanism, title, hypothesis, the metric names its rows show, and the view it opens.
2. Take `claim_state` from the experiment's recorded decisions when the results hold them, else `not_measured` with
   `planned_in`; never compute a statistic in the builder. Rows copy MetricRecord values by metric_ref, as now.
3. Validate each built view against the demo app's JSON Schema for its view (tiles included) in test_bundle.py, and
   make verify_bundle.py re-derive the new shape.
4. Bring `p1-head-to-head` and `m4-audits` to contracts.ts too: with the metrics index in place they pass the guard
   and the pages throw ("reading 'frontier_arms'", "reading 'state'") instead of showing RefusedPanel.
5. Give the provenance envelope the fields the provenance drawer reads (n, seeds, window, estimator,
   record_filter, ci), copied from the records and the manifest.

## Done when

- [ ] Both `[[verify]]` commands pass.

## Notes

- 2026-10-04 (coordinator): widened after gap-fcb44c-fix (012c75cf1). w3-pk73 probed the fixture bundle: head-to-head
  and m4-audits crash the page once they pass the guard, and the provenance envelope lacks n, seeds, window,
  estimator, record_filter and ci, which the drawer reads. All of it must land before 9315 (gap-b7f99e).

## Progress

- gap-dbe8e7: implemented at 08879cfb2 and 36cd6730a. The overview is the R1 claims board; p1-head-to-head follows contracts.ts; m4-audits is left out until S05's audit records exist (the page shows "not yet measured"); the provenance envelope has the drawer's fields; claims stay NOT_YET_MEASURED until a results directory records S09 verdicts. Bench venv pytest showcase/ 24/24 (verify 0: 6 passed); fly-smoke A1-A4 (verify 1) and showcase-serve 6/6, with its new head-to-head, audits and drawer check, pass with a copy of the batch binary.
