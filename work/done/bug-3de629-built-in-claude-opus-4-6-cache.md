+++
id = "bug-3de629"
kind = "bug"
title = "Built-in claude-opus-4-6 cache-read price is 0.25x input, so cached tokens are over-costed"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-learn/cost"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-ad0d39"
anchors = ["crates/roko-learn/src/cost_table.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-ad0d39"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-learn --lib cache_read_rates"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:10Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:59:22Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
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
