+++
id = "bug-0c0747"
kind = "bug"
title = "Other built-in price tables still carry the old Opus and Haiku rates and disagree with BUILTIN_PRICING"
status = "open"
triage = "unverified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-learn/cost"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-3de629"
anchors = ["crates/roko-core/src/provider_catalog.rs", "crates/roko-learn/src/costs_db.rs", "crates/roko-learn/src/estimate.rs", "crates/roko-agent/src/provider/codex_cli/stream.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-3de629"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-learn --lib price_tables_agree"
+++

## Problem

bug-3de629 corrected `BUILTIN_PRICING` against the providers' price pages, with dates. Other tables still carry old rates: `provider_catalog.rs` (Opus $15/$75), the `costs_db.rs` overrides (anthropic/claude-opus-4-6 at $15/$75, and Gemini rows with no cache prices), `estimate.rs`'s fallback ladder (Opus $15/$75, Haiku $0.80/$4) and `codex_cli/stream.rs` (gpt-5.6-sol $2/$0.50/$8). Configured models with no cache prices also get different defaults depending on the path: `CostTable::from_config` and task_runner price cache reads at 0.5x input, while `Usage::fill_cost_from_pricing` uses 0.1x.

## Plan

Derive the other tables from `BUILTIN_PRICING`, or delete them. Use one cache-read default everywhere. Add a test named `price_tables_agree_*`.

## Done when

- `cargo test -p roko-learn --lib price_tables_agree` passes.

## Notes

- Reported on 2026-10-01 by wk-model-truth, working on bug-3de629, during the evening close-out round.
