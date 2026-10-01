+++
id = "bug-2dfd23"
kind = "bug"
title = "roko research search records no spend, and research runs record only provider-reported usage"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli/research"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "825d45f97"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-86ff56"
anchors = ["crates/roko-cli/src/research.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-86ff56"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib research_search_records_spend"
+++

## Problem

`roko research search` (the Perplexity Search API) records no spend. The Perplexity and Gemini research runs record only what the provider reports, with no pricing backfill when the report lacks cost.

## Plan

Record each search call, priced from the registry, and backfill research-run cost from usage when the provider omits it. Add a test named `research_search_records_spend`.

## Done when

- `cargo test -p roko-cli --lib research_search_records_spend` passes.

## Notes

- Reported on 2026-10-01 by wk-learn2, working on bug-86ff56, during the evening close-out round.
- 2026-10-01 (wk-learn2): implemented on work/gap-14f08e; cargo verification deferred to the batch check.
  `roko_cli::research::record_search_spend` records each search at the new registry price
  `PERPLEXITY_SEARCH_REQUEST_USD` ($5 per 1,000 requests), under model `perplexity-search`. The research-run
  recorder moved into the library as `record_run_spend`, which prices a run that reported no dollar amount from its
  tokens (`fill_usage_cost_from_pricing`: the model's configured rates, else the registry's). Tests:
  `research_search_records_spend`, `research_runs_without_a_reported_cost_are_priced`.
