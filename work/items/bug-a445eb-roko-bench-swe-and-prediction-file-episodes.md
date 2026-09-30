+++
id = "bug-a445eb"
kind = "bug"
title = "roko bench swe and prediction-file episodes record cost_usd 0.0 for cost nobody measured"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-cli/bench"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "0b84bc9fa"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-honestbench's report, checked on work/bug-28becc at abb181f65)"
anchors = ["crates/roko-cli/src/bench.rs"]
lane = "rust-cold"
links = { depends_on = ["bug-28becc"], blocks = [], related = ["bug-28becc"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn swe_episodes_record_unmeasured_cost_as_unknown' crates/roko-cli/src/ && cargo test -p roko-cli --lib swe_episodes_record_unmeasured_cost_as_unknown"
+++

## Problem

On bug-28becc's branch, the episodes that `roko bench swe` writes for its command mode and its prediction-file mode set `cost_usd: 0.0` (`crates/roko-cli/src/bench.rs:726`, :768). Nobody measured those costs. $0 reads as free, and it pulls every average down.

## Why it matters

Release: the benchmark's results are only honest if an unknown cost is recorded as unknown.

## Where

The two episode constructions in `bench.rs`.

## Plan

1. Record the cost as unknown (null, or a `cost_known = false` marker in the episode's extra) when no usage was observed. Readers must then exclude it from sums, not treat it as $0.
2. Add `swe_episodes_record_unmeasured_cost_as_unknown`.

## Done when

- [ ] No bench episode records $0 for a cost it didn't measure.
- [ ] The `[[verify]]` command passes.

## Notes

- **wk-honestbench (2026-09-30):** Implemented on `work/bug-32d57f` at `a261d070b` and `0b073b879`; cargo verification deferred to the batch check. Reader contract: `Episode::cost_known()` / `COST_KNOWN_KEY` in roko-learn; `derive_cost_record` skips such episodes, and `compute_compounding_metrics` leaves them out of cost per success (that function has no caller yet: gap-14f08e). Not covered: `EfficiencySummaryRecord::from_episode` still copies the 0 into efficiency-summaries.jsonl, which has no unknown-cost flag.
