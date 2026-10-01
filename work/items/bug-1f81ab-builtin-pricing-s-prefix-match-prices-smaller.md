+++
id = "bug-1f81ab"
kind = "bug"
title = "builtin_pricing's prefix match prices smaller models at their larger sibling's rate"
status = "open"
triage = "unverified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-core/pricing"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-0c0747"
anchors = ["crates/roko-core/src/config/model_registry.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-0c0747"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-core --lib builtin_pricing_prefix"
+++

## Problem

`builtin_pricing` falls back to a prefix match, so o3-mini is priced as o3, gpt-4o-mini as gpt-4o, gemini-2.5-flash-lite as 2.5 Flash, and sonar-reasoning(-pro) as sonar. costs_db's exact lookup disagrees for these models.

## Plan

Match the longest known slug, and refuse a prefix that crosses a model-family boundary (e.g. `-mini`, `-lite`). Add the missing rows, from dated price pages, or leave them unpriced. Add a test named `builtin_pricing_prefix_*`.

## Done when

- `cargo test -p roko-core --lib builtin_pricing_prefix` passes.

## Notes

- Reported on 2026-10-01 by wk-model-truth, working on bug-0c0747, during the evening close-out round.
