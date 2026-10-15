+++
id = "bug-2839c4"
kind = "bug"
title = "efficiency.jsonl carries each attempt's cost once, so summing its rows gives the run's real spend"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-learn/efficiency"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-48"
anchors = ["crates/roko-learn/src/efficiency.rs", "crates/roko-learn/src/feedback_service.rs::model_call_cost_record"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn efficiency_rows_count_an_attempts_cost_once' crates/ && cargo test -p roko-learn efficiency_rows_count_an_attempts_cost_once"
+++

## Problem

After agent H's live runs (2026-10-15, HEAD 7b6a5af44), `.roko/learn/efficiency.jsonl` held 30 rows for 15 attempts:
for each attempt a `kind = "model_call"` row and an outcome row, and both carry the attempt's full `cost_usd` (H,
finding N6). A reader that sums `cost_usd` over the file reports twice the spend. `learn/costs.jsonl` (15 rows) and
the verdicts' `billed_usd` are correct; this is about the efficiency stream only.

## Why it matters

Goal `truth`: cost numbers have one meaning. The efficiency stream feeds `roko learn efficiency` and model-efficiency
summaries (`crates/roko-learn/src/efficiency.rs`, averages around `:695-705`). An average over both row kinds is
probably right by accident (both carry the same amount), but a total is not, and any external analysis of the JSONL
will be off.

## Where

- `crates/roko-learn/src/efficiency.rs`: the event type (`cost_usd`, `cost_usd_without_cache`) and its summaries.
- The `model_call` row comes from `FeedbackEvent::ModelCall` (`crates/roko-learn/src/feedback_service.rs`,
  `model_call_cost_record` at `:892`). The outcome row comes from the attempt's settle path in
  `crates/roko-cli/src/graph_task_dispatch/`. Confirm both writers first.

## Current state

Not yet established whether any in-tree reader sums across both kinds. That is the first thing to check.

## Plan

1. Decide which row owns the cost: the `model_call` row (one per call, as its name says) or the outcome row (one per
   attempt). Then write `cost_usd = 0`, or better, leave the cost fields out, on the other kind and say so in the
   field docs.
2. Check every reader of `efficiency.jsonl` (`roko learn`, `roko show costs`, the TUI, `tools/`) for sums and filter
   by kind where needed.
3. Test `efficiency_rows_count_an_attempts_cost_once`: one settled attempt writes rows whose `cost_usd` sums to the
   attempt's billed cost.

## Done when

`efficiency_rows_count_an_attempts_cost_once` passes (the `[[verify]]` command).

## Notes

Evidence: `tmp/status-quo/2026-10-15/live/H-live-run.md:334` (learning-data table) and `:351` (N6). Old rows stay
as written; readers that sum across both kinds should take one kind only.
