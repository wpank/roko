+++
id = "gap-de0e96"
kind = "gap"
title = "Nothing publishes efficiency_trend_updated, so no host sees the cost per verified task fall from run to run"
status = "done"
triage = "verified"
severity = "p1"
goal = "hermes"
size = "S"
subsystem = ["roko-cli/graph_execution", "roko-core/dashboard"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "7d8eeb8e7"
source = "tmp/hermes-roko-v6/findings/learning-research.md §4 item 8 (learning-research, 2026-10-16)"
discovered_from = "hermes-roko-v6 round: make learning visible on the hub"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan_body", "crates/roko-cli/src/runner/tui_bridge.rs::TuiBridge::efficiency_trend_updated", "crates/roko-core/src/dashboard_snapshot.rs::EfficiencyBucket"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn run_end_publishes_cost_per_verified_task' crates/roko-cli && cargo test -p roko-cli --lib run_end_publishes_cost_per_verified_task"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T21:14:27Z"
commit = "7d8eeb8e7"
by = "commit trailer"
executor = "unknown"
forced = false
evidence = "7d8eeb8e7: EfficiencyBucket gains run_id, cost_usd, tasks, verified_tasks and cost_per_verified_usd; a Graph run's end publishes efficiency_trend_updated, one bucket per recent plan run (at most 20, oldest first, this run last) from run-metrics.jsonl; /health feature run_cost_trend; docs/v3/26-HTTP-API.md. Verify: run_end_publishes_cost_per_verified_task and the_run_trend_gives_each_runs_cost_per_verified_task pass. Checked as a stack on f870523c0 (main 8357603f0 + gap-655ea5 + gap-de0e96 + bug-ea8c66): roko-cli lib 3738 passed (6 ignored, gist skipped), roko-serve lib 1047, roko-core lib 2035, the roko bin's learn test, clippy -D warnings clean on roko-core, roko-cli and roko-serve, fmt, plan_branch_integration (4) and connected_efficiency_render (1), with a scratch HOME. Scripted showcase, $0: the trend published at the end of p1, p2 and p3 reads $0.013183, $0.003542, $0.003612 per verified task."
+++

## Problem

The pitch's evidence that roko "gets better the more you use it" is the cost per verified task falling from run to
run (learning-research 3.4: $0.068 → $0.045 → $0.016 over three similar tasks). Nothing publishes it:
`DashboardEvent::EfficiencyTrendUpdated` and `TuiBridge::efficiency_trend_updated` exist, but nothing calls them, and
its buckets carry cost in whole cents and no verified-task count, so even a published trend could not say it.
`run-metrics.jsonl` keeps each run's cost and verified tasks; no host reads it live (learning-research §4 item 8;
analyze.py computes the number by hand).

## Why it matters

Goal `hermes`: the plugin's Learning view and Insights (spec-v3 P5, P6, D7) draw "cost per verified task, by run";
the single chart that tells the learning story.

## Where

- `crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan_body`: appends the run's `RunMetricsRecord`.
- `crates/roko-cli/src/runner/tui_bridge.rs::TuiBridge::efficiency_trend_updated`: the unused publisher.
- `crates/roko-core/src/dashboard_snapshot.rs::EfficiencyBucket`: the bucket.

## Plan

1. `EfficiencyBucket` gains optional run fields: `run_id`, `cost_usd` (unrounded), `tasks`, `verified_tasks`,
   `cost_per_verified_usd`.
2. As a Graph run ends, after its record is appended, publish `efficiency_trend_updated` with one bucket per recent
   plan run (at most 20, oldest first, this run last) from `run-metrics.jsonl`.
3. `/health` feature `run_cost_trend`; docs/v3/26-HTTP-API.md.

## Done when

The `[[verify]]` test passes: a run's end publishes the trend, its bucket last with its cost, its verified tasks and
its cost per verified task.

## Notes

The end-of-run gate-threshold flush still publishes no `gate_thresholds_updated` (the other half of learning-research
item 8); not done here.
