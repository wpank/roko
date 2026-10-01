+++
id = "bug-3de629"
kind = "bug"
title = "Built-in claude-opus-4-6 cache-read price is 0.25x input, so cached tokens are over-costed"
status = "open"
triage = "unverified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-learn/cost"]
created = 2026-10-01
updated = 2026-10-01
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
