+++
id = "gap-6605f7"
kind = "gap"
title = "POST /api/plans/{id}/estimate gives every task one global average with low confidence: no start rung, no rung prices, no retries or climbs"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "M"
subsystem = ["roko-serve/plans"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "4f256a1fb"
source = "tmp/hermes-roko-v6/findings/dogfood-will.md (dogfood-will Run 3, U9) and tmp/hermes-roko-v6/design/spec-v3.md (R7), via plugin-runs' triage"
anchors = ["crates/roko-serve/src/routes/plans/reads.rs::plan_estimate", "crates/roko-serve/src/runtime.rs::CliRuntime"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_estimate_prices_each_task_on_its_start_rung_and_its_climbs' crates/roko-cli/ && cargo test -p roko-cli --lib plan_estimate_prices_each_task_on_its_start_rung_and_its_climbs"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T14:37:38Z"
commit = "57742e17a"
forced = false
evidence = "POST /api/plans/{id}/estimate now prices each remaining task on its own route: the rung dispatch would start it on (the ladder start for its role, tier and hint; a pinned model stays on it; an authored max_retries is kept), then its climbs (2 attempts a rung, at most 2 climbs), at roko-core effective_rates, with the median tokens and seconds of the 200 most recent attempts of similar tasks (same role and tier, else tier, else any) from .roko/runs. Per task: start_rung, model, start_cost_usd, likely (weighted by the first-try rate) and worst (every attempt climbing) cost and time, worst_rungs, basis, samples; totals with cap_usd, by_rung and confidence. Live at zero cost on a copy of the pocket-synth run: before, every task 0.0826 USD and 30 s (total 0.3302); after, each task starts on cheap (gpt-oss-120b) at 0.0487 USD an attempt, the same as roko billed for that run ENV attempt, worst 0.3225 over cheap, cheap, mid, mid, strong, strong; plan likely 0.1948, worst 1.29, cap 0.25. On 4f256a1fb (main b2970cb12 merged in): the verify, the 3 roko-learn task_estimate tests, 68 roko-serve routes::plans tests, clippy -D warnings (roko-learn, roko-serve, roko-cli) and nightly fmt pass."
+++

## Problem

`POST /api/plans/{id}/estimate` (`routes/plans/reads.rs::plan_estimate`) gives every remaining task the same estimate:
the average tokens, cost and time over every record in `.roko/learn/efficiency.jsonl`. With no records it gives $0.05
and 60 s per task.

- It ignores the task's tier, role and start rung.
- It prices nothing at a rung's rates.
- It says nothing about retries or climbs.
- Its confidence is "low" or "medium" only by whether that file has records.

Seen in dogfood-will's Run 3 (U9, `tmp/hermes-roko-v6/findings/dogfood-will.md`): the Run dialog said "est $0.20 · ~4m
(confidence low)". The plan took $0.30 and about 18 minutes over two runs. Another plan was estimated at "$0.62 ·
~1m 30s" and cost $0.041 in 34 s.

## Why it matters

A person decides from this number whether to run a plan, and at what cap. The composer's "likely $0.02 · about a
minute" and the plan sheet's "estimate by rung" depend on it (`tmp/hermes-roko-v6/design/spec-v3.md`: D8, D10, P7, R7).

## Where

- `crates/roko-serve/src/routes/plans/reads.rs::plan_estimate` and `estimate_task_from_history`.
- The ladder as dispatch binds it: `crates/roko-cli/src/dispatch/model_routing.rs::RoutingLadder`. The climb rules are
  in `graph_task_dispatch/ladder.rs`:
  - two agent-blamed failures on a rung before a climb;
  - at most two climbs;
  - a retry budget of five, unless the task authors `max_retries` or a pinned model.
- Prices: `roko_core::pricing_snapshot::PriceSnapshot`, then a model's `[models.*]` rates, then built-in pricing.
- History: `.roko/runs/*/attempts.jsonl`. Its open lines carry role and tier; its verdicts carry usage, rung, timing
  and the learning label.

## Plan

1. roko-learn: read the recent settled attempts. Take the similar ones (same role and tier, else same tier, else all;
   defaults with none). Give their median tokens and time per attempt and their first-try pass rate. Add the math of
   a chain of attempts: likely (each attempt weighted by the chance the ones before failed) and worst (every attempt
   fails).
2. roko-cli: for each remaining task, find what dispatch would do:
   - its start rung and climb path, from `RoutingLadder`;
   - its retry budget;
   - each rung model's rates;
   - a pinned model stays put.
3. Per task, the response gives:
   - the start rung and its model;
   - the cost of one attempt there;
   - the likely and worst cost and time;
   - the basis and the number of samples.

   Per plan, it gives:
   - totals, and the likely cost by start rung;
   - the cap (`[budget] max_plan_usd`);
   - the tasks with no price;
   - a confidence from the sample counts.
4. Serve: the route asks the runtime (`CliRuntime::estimate_plan`). It keeps the fields the plugin reads:
   `estimate.total_cost_usd`, `total_duration_secs`, `confidence` and `note`.
5. Tests in roko-learn and roko-cli.

## Done when

`plan_estimate_prices_each_task_on_its_start_rung_and_its_climbs` passes (the `[[verify]]` command).
