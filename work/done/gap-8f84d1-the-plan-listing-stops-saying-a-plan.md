+++
id = "gap-8f84d1"
kind = "gap"
title = "The plan listing stops saying a plan is a demo once it has run, so hosts cannot hide fixtures, group demos or keep demo output out of Merge"
status = "done"
triage = "verified"
severity = "p3"
goal = "hermes"
size = "S"
subsystem = ["roko-serve/plans"]
created = 2026-10-15
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "70182b957"
source = "tmp/hermes-roko-v5/findings/roko-demos.md (roko-demos, 2026-10-15)"
discovered_from = "hermes-roko-v5 round: demo plans run from Hermes in the roko repo"
anchors = ["crates/roko-serve/src/plan_types.rs::PlanSummaryDto", "crates/roko-cli/src/plan.rs::overlay_graph_checkpoint_status"]
links = { depends_on = [], blocks = [], related = ["bug-0c7f25", "gap-8409d0"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_listing_marks_fixture_plans_after_a_run' crates/roko-cli/ && cargo test -p roko-cli --lib plan_listing_marks_fixture_plans_after_a_run"

[[verify]]
command = "grep -rqw 'fn run_summary_result_marks_a_fixture_run' crates/roko-serve/ && cargo test -p roko-serve --lib run_summary_result_marks_a_fixture_run"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T13:05:48Z"
commit = "dc6296f07"
size = "S"
claimed_at = "2026-10-16T10:35:49Z"
forced = false
evidence = "PlanSummary and PlanSummaryDto gain fixture, set from [meta].status == 'fixture' before the Graph checkpoint overlay replaces status, so it survives a run; every struct literal carries it. An ended run's summary result gains fixture, true when every plan the run ran is a fixture (routes/runs.rs::plans_are_fixtures, through the runtime's plan summary). HTTP API doc updated; the verify is split into a roko-cli test (the listing) and a roko-serve test (the result). Fix dc6296f07, merged into main as 70182b957. Both verifies and clippy -p roko-cli -p roko-serve pass on the merged tree 70182b957; at b35c56b26 the roko-cli lib suite (3715), the roko-serve lib suite (1040) and the 5 integration files whose literals changed passed. Live, roko serve on a scratch workspace with the 11 demo plans, a plain plan and a fake agent: after a demo run, GET /api/plans gave status 'done (12s ago)' and no flag before (roko-main-1630fa1bf); after, 'done (3h ago)' with fixture true, the plain plan fixture false before and after its own run, and run summaries' result.fixture true for the demo run and false for the plain one."
+++

## Problem

`GET /api/plans` reports a demo plan's `[meta] status = "fixture"` as its `status` only until the plan has a Graph
checkpoint; `overlay_graph_checkpoint_status` then replaces it with "done (2m ago)", "failed (…)" and so on. Neither
the listing nor a run's summary keeps saying that the plan is a demo once it has run. So a host such as Hermes cannot
hide fixtures, put the demos together, or show a demo run's output as "demo output, leave it on its branch" instead
of offering Merge. (2026-10-15: in the roko repo the Plans tab lists 18 `portal-programme` plans and 4 other real
plans ahead of the 11 demos and preselects `01-backend-plan-service`; after a demo run the plugin's merge check
offers a fast-forward of the demo's output into the checkout's branch, which is Will's `main`.)

`group` (the plan set, e.g. `demos/parallel-plans`) is already in the listing; the plugin drops it today.

## Why it matters

Hermes' first-run experience in the roko repo: the demo a newcomer should start with should be easy to find, and a
demo should never invite merging its output into `main`. Filed rather than built (round v5 brief: a listing field
for Hermes is an item, not a side change).

## Where

- `crates/roko-serve/src/plan_types.rs::PlanSummaryDto` and `crates/roko-cli/src/plan.rs::overlay_graph_checkpoint_status`.
- The run summary's `result` (gap-8409d0), where hosts decide whether to offer Merge.

## Plan

1. Add `fixture: bool` (from `[meta] status == "fixture"`, unaffected by the overlay) to `PlanSummaryDto` and to
   the plan-run summary's result.
2. Document both in the serve API docs, then ask the plugin to group and flag demos and to keep Merge off for them.

## Done when

`plan_listing_marks_fixture_plans_after_a_run` (roko-cli, where the listing is built) and
`run_summary_result_marks_a_fixture_run` (roko-serve, where the run summary is) pass: the two `[[verify]]` commands.
The first version of this item named one roko-serve test for both, but roko-serve's tests cannot reach roko-cli's
listing code.
