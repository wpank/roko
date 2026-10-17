+++
id = "bug-eb55eb"
kind = "bug"
title = "A Claude CLI attempt's verdict records its cost as source unknown with no API-equivalent, and /api/models gives claude-sonnet-4-6 no cache-write price, so estimates miss most of a CLI attempt's cost"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch", "roko-agent/claude_cli"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "round-v6-dogfood-will#R4-3"
discovered_from = "agent:dogfood-will (round v6 run 4, 2026-10-17)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/attempt.rs", "crates/roko-agent/src/claude_cli_agent.rs::ClaudeCliAgent", "crates/roko-core/src/config/provider.rs::ModelProfile"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-ceffb3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn cli_attempt_cost_is_priced_and_sourced' crates/roko-cli/ && cargo test -p roko-cli cli_attempt_cost_is_priced_and_sourced"
+++

## Problem

A Claude CLI attempt's verdict records its cost with no known source, although the model is priced. Any estimate
drawn from roko's price table also misses most of what such an attempt costs.

Round v6 dogfood run 4 (2026-10-17, roko b2970cb12) pinned pocket-synth's SVF task to the top rung (claude-sonnet-4-6
through the Claude CLI). The run is `graph-svf-g4-4624823f-…`; it passed first try in 3 turns and 61 s.
- **The verdict's cost:** `billed_usd: null`, `api_equiv_usd: null`, `price_snapshot_id: null`,
  `vendor_usd: 0.2165`, `source: "unknown"`. `costs.jsonl` matches: `cost_usd 0.2165`, `api_equiv_usd null`,
  `cost_source "unknown"`.
- **The price table:** `GET /api/models` prices claude-sonnet-4-6 at $3.00 / $15.00 per M, with cache reads at
  $0.30 (`priced_by: "builtin"`). It has no cache-write price.
- **The usage:** 4 input, 4,334 output, 41,276 cache-read and 23,178 one-hour cache-write tokens. At Anthropic's
  prices that is $0.000 + $0.065 + $0.012 + $0.139 (1-hour writes at 2× input) = $0.2165, exactly what the CLI
  reported. The cache writes are 64% of the cost.
- **What a table estimate gives:** without a cache-write price, an estimate gets $0.08 for this attempt. The Hermes
  plugin's Receipt prices cache writes at 1.25× input (the 5-minute rate) and gets $0.16 (`rundata.py::_top_rung_cost`).

## Why it matters

Goals `visibility` and `truth`: the top rung's cost is the comparison the pitch rests on ("cheap first saves X against
the top rung"). A verdict whose cost source is "unknown" can't back that claim. A price table without cache-write rates
makes every "what would top cost" estimate low for a CLI rung that writes a 1-hour cache, as Claude Code does.
Related: gap-ceffb3 (no dated price snapshot covers claude-sonnet-4-6).

## Where

- The CLI attempt's cost settlement, which keeps the CLI's `total_cost_usd` as `vendor_usd` and leaves
  `api_equiv_usd` and `source` empty: `crates/roko-cli/src/graph_task_dispatch/attempt.rs` (the verdict's `cost`).
  `crates/roko-agent/src/claude_cli_agent.rs` parses the CLI's usage.
- `GET /api/models`, effective prices: `crates/roko-serve/src/routes/` (the models route). The model profile has
  `cost_cache_write_per_m` (`crates/roko-core/src/config/provider.rs::ModelProfile`), but the built-in claude-sonnet-4-6
  entry sets none, and nothing distinguishes 5-minute from 1-hour writes.

## Current state

- The run total (`$0.22`) and serve's summary (`cost_usd 0.2165`) count the attempt correctly, from the vendor
  figure. Only the verdict's labels and the price table are wrong.
- Evidence:
  - the run record: `tmp/hermes-roko-v6/dogfood-will/evidence/run4/` (the top run's `attempts.jsonl`, `costs.jsonl`);
  - the price table: `tmp/hermes-roko-v6/dogfood-will/logs/run4-api-models.json`;
  - the finding: `tmp/hermes-roko-v6/findings/dogfood-will.md`, Run 4 (G4).

## Plan

1. For a CLI attempt, compute `api_equiv_usd` from its usage at the model's prices, cache writes included, and label
   the vendor figure `source: "vendor_reported"` rather than "unknown". When the two differ by more than a few
   percent, log both.
2. Give the built-in Anthropic models their cache-write prices (5-minute 1.25×, 1-hour 2× input), and publish them on
   `/api/models` (`cost_cache_write_per_m`, and a 1-hour field or a note), so hosts can estimate a CLI rung correctly.
3. Add the test `cli_attempt_cost_is_priced_and_sourced`: a Claude CLI result with usage and `total_cost_usd`
   settles with `api_equiv_usd` computed from the table and a known `source`.

## Done when

- The `[[verify]]` passes.
- A Claude CLI attempt's verdict carries an API-equivalent cost and a known source.

## Notes

The plugin's Receipt (owned by plugin-runs) should price 1-hour cache writes at 2× input once roko publishes the rate.
Tell its owners when this lands.
