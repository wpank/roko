+++
id = "gap-a6e2c3"
kind = "gap"
title = "Plan generation and revision spend never reaches the event stream, so the portal and stats.cost_usd_total leave it out"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "M"
subsystem = ["roko-serve/plans", "roko-cli/prd"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "9a7e8a1cb"
source = "plan:portal-programme/09-acceptance#T04"
discovered_from = "plan:portal-programme/09-acceptance#T04"
anchors = ["crates/roko-serve/src/routes/plans.rs:1818", "crates/roko-serve/src/routes/plans.rs:2029", "crates/roko-cli/src/prd.rs::generate_plan_from_prd"]
links = { depends_on = [], blocks = [], related = ["find-6a5b62", "bug-690dc6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo build -p roko-cli && bash -c 'source plans/portal-programme/_harness/lib.sh && require_binary && make_workspace && start_server && start_capture && [ \"$(api POST /api/plans/generate \"{\\\"prompt\\\": \\\"a rust app that prints hello world\\\"}\")\" = 202 ] && wait_operation \"$(jget \"$WS/last.json\" \"d[\\\"id\\\"]\")\" 120 && sleep 2 && stop_capture && sse between efficiency_event metric=cost_usd -- event_log_entry event_type=plan_generate.started -- event_log_entry event_type=plan_generate.completed'"

[closed]
at = 2026-09-29
commit = "9a7e8a1cb"
by = "commit trailer"
evidence = "9a7e8a1cb: plan_authoring::AuthoringSpend records every generation and revision agent call as task dispatch records its spend (CostRecord in costs.jsonl, an efficiency.jsonl row, token and cost efficiency_events on the server's StateHub via RokoCliRuntime), attributed to the plan under task generate or revise; retries and failed calls count too. The [[verify]] passes: efficiency_event metric=cost_usd (fake-claude's 0.0012) lies between plan_generate.started and plan_generate.completed. On the same harness, /api/statehub/snapshot stats.cost_usd_total reads 0.0012 after a generate, 0.0024 after a revise, and 0.0024 after a server restart. Tests: plan_authoring::tests::authoring_spend_* and serve_runtime::tests_authoring_spend (fake Claude CLI with a known cost); authoring-check.sh and revision-check.sh pass. Decision: the portal shows generation cost when each turn returns (providers report usage at the end); no portal change."
+++

## Problem

The first 09 real-model run (`tmp/portal-audit/evidence/hello-world-real-run1/events.sse`) generated its plan
with claude-sonnet-4-6 in 29.4 s (`plan_generate.started` at 1790672049774 ms, `.completed` at
1790672079192 ms), then ran one task. The stream carries `efficiency_event` usage and
`cost_usd = 0.0587` for the task only, and nothing between the two generate events. The snapshot's
`stats.cost_usd_total` is fed by those events, so both the portal's cost and the server's total leave
out generation. Revision presumably behaves the same (it reuses the generation pipeline, 04b) but was
not checked. The 09 verdict could not state the real run's total cost.

The re-run at 11:40, with the script's patched snapshot lookup, printed
`cost_usd_total=0.061694398522377014`. That is exactly its task's one `cost_usd` event, although its
generation turn ran 20.5 s (`serve.log`: `agent=20528ms`; `tmp/portal-audit/evidence/hello-world-real/`).
The snapshot total leaves generation out.

Reproduced with the fake agent, which reports `total_cost_usd` for generation too: after a generate,
there is no `efficiency_event metric=cost_usd` between `plan_generate.started` and
`plan_generate.completed`.

## Why it matters

Goal `visibility`. Generation is the first paid step of the portal flow, and the design's header
cost (§2) cannot include it.

## Where

The generate operation in `crates/roko-serve/src/routes/plans.rs` (`plan_generate.started` at
:1818, revise at :2029) calls `crates/roko-cli/src/prd.rs::generate_plan_from_prd`. The provider's
usage is not published as `efficiency_event`.

## Plan

Publish the usage the generate and revise operations receive (tokens and cost) as
`efficiency_event`s attributed to the plan (with an empty task id, or `generate`), and record it
where plan costs are kept. Decide whether the portal shows the running cost while generating.

## Done when

The `[[verify]]` (fake-agent harness) finds a `cost_usd` efficiency event between the generate
start and completion events.

## Notes

find-6a5b62 (parked, unverified) saw $0 plan-operation costs in `.roko/learn/costs.jsonl` in an older
audit. This item covers the event-stream side, verified on 2026-09-29. Found by plan 09 T04 (see
VERDICT).
