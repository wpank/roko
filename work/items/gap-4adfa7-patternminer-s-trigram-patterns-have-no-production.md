+++
id = "gap-4adfa7"
kind = "gap"
title = "PatternMiner's trigram patterns have no production reader"
status = "open"
triage = "unverified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-learn"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-d0f52f"
anchors = ["crates/roko-learn/src/pattern_discovery.rs", "crates/roko-learn/src/runtime_feedback/mod.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-d0f52f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rn 'pattern_miner()' crates/roko-cli/src --include=*.rs"
+++

## Problem

`LearningRuntime::pattern_miner()` is only fed episodes. Nothing in production reads its trigram patterns, so the loop is dead.

## Plan

Wire the patterns into something that uses them (prompt hints or routing), with a test, or delete the miner.

## Done when

- The verify passes with a production reader, or the miner is gone.

## Notes

- Reported on 2026-10-01 by wk-learn2, working on gap-d0f52f, during the evening close-out round.
