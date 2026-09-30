+++
id = "bug-9a6799"
kind = "bug"
title = "EfficiencySummaryRecord::from_episode copies an unknown cost into efficiency-summaries.jsonl as 0"
status = "open"
triage = "unverified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-learn/runtime_feedback"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-honestbench's report)"
anchors = ["crates/roko-learn/src/runtime_feedback/records.rs", "crates/roko-learn/src/episode_logger.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = [], blocks = [], related = ["bug-a445eb", "bug-ac5432"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn efficiency_summaries_keep_an_unknown_cost_unknown' crates/roko-learn/src/ && cargo test -p roko-learn --lib efficiency_summaries_keep_an_unknown_cost_unknown"
+++

## Problem

`EfficiencySummaryRecord::from_episode` (`crates/roko-learn/src/runtime_feedback/records.rs`) copies `cost_usd: episode.usage.cost_usd` (and the without-cache cost) as they are. When the episode's cost is unknown, which `Episode::mark_cost_unknown` (`episode_logger.rs:399`) marks, the summary still records 0. So `efficiency-summaries.jsonl` reports unknown costs as free.

## Why it matters

One settled record per attempt (epic spec-b7303f): the summaries feed cost reports. A 0 for an unknown cost pulls every average down, the same error bug-a445eb fixed for bench episodes.

## Where

`EfficiencySummaryRecord::from_episode`, and the fields it copies.

## Plan

1. When the episode's cost is marked unknown, record the summary's cost as unknown (null, or an explicit marker), and make readers skip it in sums.
2. Add `efficiency_summaries_keep_an_unknown_cost_unknown`.

## Done when

- [ ] An episode with an unknown cost never yields a summary with cost 0.
- [ ] The `[[verify]]` command passes.

## Notes

- bug-ac5432 (plan generation adds a $0 cost record beside the real one) can reuse `Episode::mark_cost_unknown` for its unknown costs.
