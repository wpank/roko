+++
id = "bug-3de629"
kind = "bug"
title = "Built-in claude-opus-4-6 cache-read price is 0.25x input, so cached tokens are over-costed"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-learn/cost"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-ad0d39"
anchors = ["crates/roko-learn/src/cost_table.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-ad0d39"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-learn --lib cache_read_rates"
+++

## Problem

The built-in cost table prices claude-opus-4-6 cache reads at 0.25x the input rate. Anthropic's published cache-read multiplier is 0.1x the base input price, so recorded costs overstate cached usage. The other built-in rates have no dated source (gap-ad0d39's notes).

## Plan

Check every built-in rate against the published price page, record its source and date beside the table, and add a test named `cache_read_rates_*` pinning the multipliers.

## Done when

- `cargo test -p roko-learn --lib cache_read_rates` passes, and the table cites a dated source.

## Notes

- Reported on 2026-10-01 by the worker on gap-ad0d39, during the evening close-out round.
- 2026-10-01 (wk-model-truth): implemented on work/bug-3aa61f; cargo verification deferred to the batch check.
  Every `BUILTIN_PRICING` row (roko-core `config/model_registry.rs`) was checked against its provider's price
  page on 2026-10-01; each provider block cites the page and the date. Opus 4.6 is $5/$25 with $0.50 cache reads,
  Haiku 4.5 $1/$5, and the OpenAI, Gemini, GLM-5 and Sonar rows changed; a cache write is billed as input where no
  write price is published. kimi-k2.5 and codex-mini are on no current price page, so they keep their rates and are
  listed in `UNVERIFIED_PRICING`. Tests: `cache_read_rates_match_each_providers_price_page`,
  `cache_read_rates_price_cached_opus_tokens_at_a_tenth_of_input`. Not changed: the separate tables in
  `provider_catalog.rs`, the `costs_db.rs` overrides, roko-compose `estimate.rs` and `codex_cli/stream.rs`.
