+++
id = "bug-430473"
kind = "bug"
title = "The TUI's PatternMiner learning row counts CrossEpisodeConsolidator patterns"
status = "open"
triage = "unverified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/tui"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-4adfa7"
anchors = ["crates/roko-cli/src/tui/dashboard_model.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-4adfa7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -n 'PatternMiner' crates/roko-cli/src/tui/dashboard_model.rs"
+++

## Problem

The learning row labelled PatternMiner (`dashboard_model.rs:879`) counts CrossEpisodeConsolidator meta-patterns. gap-4adfa7 deleted the runtime miner, so the label names something that no longer exists.

## Plan

Rename the row after what it counts.

## Done when

- The verify passes.

## Notes

- Reported on 2026-10-01 by wk-learn2, working on gap-4adfa7, during the evening close-out round.
