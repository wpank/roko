+++
id = "bug-8bcd98"
kind = "bug"
title = "Spend books are incomplete: a model call that fails writes no call_cost row, a verdict's billed_usd leaves out the calls refused before its failover (7b8cb121 T2#1: $0.0354 of $0.2430), and a try killed mid-call has no cost row (6059db44: the plugin's Build pad showed $0.04 of $0.0627)"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 run-analyst (tmp/hermes-roko-v7/findings/run-analyst.md), the lead's ask; plugin-wow's finding on 6059db44 (tmp/hermes-roko-v7/findings/plugin-wow.md)"
discovered_from = "agent:run-analyst (round v7)"
anchors = ["crates/roko-agent/src/tool_loop/mod.rs::run_inner", "crates/roko-cli/src/graph_task_dispatch/call_cost.rs::MeteredCalls", "crates/roko-learn/src/telemetry/records.rs::AttemptCost", "crates/roko-learn/src/feedback_service.rs::model_call_cost_record"]
links = { depends_on = [], blocks = [], related = ["gap-36f3fb", "bug-dceafd", "gap-5cfa9c", "gap-1ba0ac", "gap-ece700"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'refused_usd' crates/roko-learn/src/telemetry/records.rs && grep -rqw 'fn a_call_that_fails_writes_its_call_cost_row' crates/roko-cli/src && cargo test -p roko-cli --lib a_call_that_fails_writes_its_call_cost_row"
+++

## Problem

roko keeps spend in several books:
- **per call:** the `call_cost` events, which the run summary and the live spend sum;
- **per try:** the verdict's `cost` in `attempts.jsonl`, and the `learn/costs.jsonl` rows written from
  `FeedbackEvent::ModelCall`;
- **per plan:** the budget ledger, `state/graph/<plan>/costs.json`.

In round v7 they disagree in three ways.

1. **A model call that fails without a usage report writes no `call_cost` row.** This covers response timeouts, 429s
   and HTTP 4xx refusals.
   - bdc4461e T1#1 and T1#2 each lost a call to the 120 s response wait. 4 min of their model time has no call row.
   - 16fc88ae's four tries were refused (HTTP 400) and left no row.
   - The per-call book loses the call, its time and its outcome.
   - A call cut off mid-stream is booked through the stream tally (bug-72d775). A call billed with no usage report
     would be lost.
2. **A verdict's `cost.billed_usd` leaves out the routed model's calls that a failover refused.**
   - 7b8cb121 T2#1 billed $0.0354, but its calls cost $0.2430. The difference, $0.2076, was glm-4.7's work before the
     z.ai 429 failover.
   - 20c780ae's tries leave out $0.027.
   - The run summary and `learn/costs.jsonl` (role `failover_refused`) carry the refused calls; the verdict doesn't.
     gap-5cfa9c is the same gap on the run summary's `rungs[]` side.
3. **A try killed mid-call has no verdict and no `learn/costs.jsonl` row.** This was 6059db44 T1#1, killed by the
   2026-10-26 reboot.
   - `FeedbackEvent::ModelCall` is sent only when the dispatch returns.
   - The try's $0.0221 is in the run summary (its `call_cost` events) and in the plan ledger ($0.0627 for the run).
   - plugin-wow found the Build pad, which reads the cost rows, showing $0.04 for phase 4.1.
   - roko-recovery (round v7, `tmp/hermes-roko-v7/findings/roko-recovery.md`) is characterizing the kill case
     (gap-36f3fb, bug-dceafd). This item names it and leaves its fix there.

Expected: every call a try makes, failed or refused, appears in each book with its outcome, and the books agree.

## Why it matters

Truth: every number in the reports and every learning signal reads these books.
- The round's accounting had to re-price each try from its `call_cost` rows to match the pilot's ledger, because the
  verdicts don't add up to the run's spend.
- A failed call's missing time makes model time look like it has no calls.
- The cost rows feed the self-model and the plugin's spend.

## Where

- `roko-agent/src/tool_loop/mod.rs::run_inner`: a call's `Err` path. It records usage only when the stream tally has
  some (`budget.record_turn`).
- `graph_task_dispatch/call_cost.rs::MeteredCalls`: publishes `call_cost` from `record_model_call`, for calls that
  return usage.
- `roko-learn/src/telemetry/records.rs::AttemptCost`: the verdict's `cost`.
- `roko-learn/src/feedback_service.rs::model_call_cost_record`: the `learn/costs.jsonl` row, from
  `FeedbackEvent::ModelCall`.

## Current state

run-analyst checked it over round v7 (`tmp/hermes-roko-v7/run-analyst`).
- **Per run:** the sum of `call_cost` equals the plan ledger and the pilot's ledger to the cent, for every run.
- **Per try:** the verdict differs wherever a failover refused calls.
- **Cost rows:** every try except the killed one has a `learn/costs.jsonl` row. Provider errors and cancelled tries
  included.

## Plan

1. **Failed calls:** on a call's failure, report it with its outcome and wall time, and zero usage unless the
   provider billed some. For example, give `TurnBudget::record_turn` an outcome, or add a
   `record_failed_call(outcome, wall_ms)`. `MeteredCalls` then publishes a `call_cost` row with `outcome` (ok,
   timeout, rate_limited, refused, error) and `wall_ms`.
2. **The verdict:** add `cost.refused_usd`, what the try's refused calls cost, beside `billed_usd`. A try's spend is
   billed plus refused. The verdict's writer and every reader of `billed_usd` that means "the try's spend" use the
   sum.
3. **Killed tries:** leave them to roko-recovery's item. It is expected that a resume or serve's settling writes the
   abandoned try's cost row from its `call_cost` events.

## Done when

- A call that times out writes a `call_cost` row with outcome `timeout` and its `wall_ms`. Test:
  `a_call_that_fails_writes_its_call_cost_row`.
- A verdict whose try failed over carries `cost.refused_usd`, and billed plus refused equals the try's `call_cost`
  sum.
- The `[[verify]]` passes.

## Notes

- Not here: `reasoning_tokens` and `finish_reason` on `call_cost`. roko-effort takes those with gap-1ba0ac.
- The time fields on every call (`wall_ms`, `ttft_ms`, provider waits) are in gap-ece700.
- Evidence: `tmp/hermes-roko-v7/findings/run-analyst.md`, "How the numbers were made" and telemetry gaps T1 and T8.
