+++
id = "bug-83ce35"
kind = "bug"
title = "roko init's cheap rung costs more per try than mid and strong on agent loops: Cerebras bills cached input at its full $0.35/M, round v7's tries were 94% cached input, and nothing prices the ladder's order with cache reads"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/dispatch", "roko-learn/self_model"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
source = "round-v8-run-analyst-v8#cheap-rung-estimate"
discovered_from = "agent:run-analyst-v8 (round v8, the lead's ask, 2026-11-12)"
anchors = ["crates/roko-cli/src/commands/init.rs::MODELS", "crates/roko-cli/src/commands/init.rs::InitModels::ladder", "crates/roko-cli/src/dispatch/model_routing.rs::ladder_report", "crates/roko-learn/src/self_model/cost.rs::TYPICAL_ATTEMPT", "crates/roko-learn/src/telemetry/rung_stats.rs::misordered", "config/prices/2026-10-27.toml"]
links = { depends_on = [], blocks = [], related = ["gap-3217a0", "find-c3da60", "find-183c12", "gap-f3944c", "gap-ceffb3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn ladder_price_order_counts_cache_reads' crates/roko-cli/src && cargo test -p roko-cli --lib ladder_price_order_counts_cache_reads"
+++

## Problem

roko's ladder starts a mechanical or focused task on its `cheap` rung. Which model is cheap is fixed when `roko init`
writes the ladder: its built-in table (decision D11) puts gpt-oss-120b on Cerebras on `cheap`, glm-4.7 (or kimi) on
`mid` and gpt-5.4-mini on `strong`. The order follows list input and output prices.

Cerebras bills cached input at its full input rate, $0.35/M (`config/prices/2026-10-27.toml`; roko's billing knows,
`bills_cached_input_at_input_rate`, find-c3da60). Agent loops are mostly cached input, and the cache is where the
order breaks:

- **Round v7 (Undertow):** 93.6% of the input tokens of all 110 tries were cache reads (65.0M of 69.4M), and output
  was 1.7% of input. Priced on those same tokens at `prices-2026-10-27`:

  | Model | Rung | Cost |
  |---|---|---|
  | gpt-oss-120b | cheap | $25.20 |
  | glm-4.7 | mid | $12.44 |
  | gpt-5.4-mini | strong | $13.57 |
  | gpt-5.4 | strong from phase 4 | $45.25 |

  The cheap rung costs twice what mid does.
- **The cheap rung's own tasks:** v7's 17 mechanical and focused tasks (25 tries) cost $1.65 on mid and strong.
  - With the same tokens on gpt-oss-120b they would have cost $2.91.
  - If the cheap rung had failed twice per task before the climb, up to $5.21.
  - These are estimates. Token use would differ on another model.
- **Break-even at v7's output share:** gpt-oss-120b is cheaper than glm-4.7 only below 56% cached input, and cheaper
  than gpt-5.4-mini below 69%.
- **Not only Undertow:** agent H's live run on roko's own small tasks (2026-10-15, find-183c12) sent gpt-oss-120b
  2.15M prompt tokens, 1.85M of them cached (86%). That is above both break-evens too.

The cheap rung never ran in v7: Cerebras has answered 402 since 2026-10-20 (gap-f3944c). So nothing was lost. With
credit, its tries would have cost more than mid's.

## Why it matters

The ladder exists to run work on the cheapest model that passes and to climb only on failure (goal `golden-path`). A
cheap rung that costs more per try than the rung above it breaks that silently. Its tries would be the dearest and
also the most likely to fail, and nothing says so before money is spent. The round v8 lead was about to ask Will to top
up Cerebras to save money; this finding stopped that.

## Where

- **`crates/roko-cli/src/commands/init.rs::MODELS`:** the D11 table. gpt-oss-120b sits on `cheap` with
  `cost_cache_read_per_m = 0.35`.
- **`InitModels::ladder`:** writes the rungs "cheapest first" from that table without pricing them.
- **`crates/roko-cli/src/dispatch/model_routing.rs::ladder_report`:** the ladder as `roko doctor` and `roko init`
  show it. It says which rungs can run, never what they cost.
- **`crates/roko-learn/src/self_model/cost.rs::TYPICAL_ATTEMPT`:** the self-model's cost prior for an arm it hasn't
  seen: 6k input, 24k cache read (80%), 3k output.
  - At that mix, gpt-oss-120b ($0.0128) ties glm-4.7 ($0.0128) and beats gpt-5.4-mini ($0.0198).
  - At v7's measured mix (92% cached, about 470 output tokens per 30k input), it costs $0.0109 against $0.0055 and
    $0.0060.
- **`crates/roko-learn/src/telemetry/rung_stats.rs::misordered`** (gap-3217a0) flags a higher rung that is cheaper
  and faster per verified pass. It uses realized cost, caching included, but only once each rung has 3 verified
  passes. It can't warn before the cheap rung spends.

## Current state

- **Billing is right.** `pricing_snapshot.rs` prices cache reads at the snapshot's rate. A Cerebras profile that
  names no cache rate takes its input rate (`ModelProfile::with_provider_cache_rate`).
- **What's missing:** nothing prices the rungs against each other, cache reads included, before a run. The ladder's
  order is the D11 table's.

## Plan

1. Price one attempt on each rung's model from the price snapshot, cache reads included. Use the workspace's
   measured token mix (input, cache-read and output shares over its settled tries) once it has, say, 10 tries, else
   `TYPICAL_ATTEMPT`.
2. Report in `ladder_report` (so `roko doctor` and `roko init` print it) when a lower rung costs more per attempt
   than the rung above it, for example: "cheap (gpt-oss-120b) costs $0.0109 an attempt at this workspace's 92% cached
   input, more than mid (glm-4.7) at $0.0055: Cerebras bills cached input at its input rate."
3. Then pick what to do about it. This is a design choice:
   - (a) report only;
   - (b) `roko init` orders the rungs by the priced attempt;
   - (c) the start rule skips a rung that costs more than the next for the task's role.

   Suggested: (a) now, and refresh `TYPICAL_ATTEMPT` from round v7's measured mix: 93.6% cached, output 1.7% of
   input.

## Done when

- `roko doctor` prints the line in step 2 for a workspace with these two properties:
  - its ladder has gpt-oss-120b on Cerebras below glm-4.7 or gpt-5.4-mini;
  - its tries are at least 70% cached input.
- `cargo test -p roko-cli --lib ladder_price_order_counts_cache_reads` passes. The test builds that ladder, prices it
  at a 92% cached attempt, and asserts the report names the cheap rung as dearer than mid.

## Notes

- **Numbers and method:** round v8's run-analyst-v8 (`tmp/hermes-roko-v8/data/`): `v7_cheap_rung_estimate.json` and
  `.csv`, `v7-records/tries.csv`, `tools/cheap_rung.py`. All are $0 and read from Undertow's run records.
- **Don't change billing.** It's right.
- **Related:**
  - gap-3217a0: the misordered-ladder notice;
  - find-c3da60: Cerebras cache billing;
  - find-183c12: the cheap rung's turn caps, which re-pay the whole cached context each turn;
  - gap-f3944c: Cerebras' 402s;
  - gap-ceffb3: the price snapshot.
