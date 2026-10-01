+++
id = "gap-666ab3"
kind = "gap"
title = "The orchestrator ExecutorConfig, ParallelExecutor and the CLI Config.executor field are parsed but never used in production"
status = "open"
triage = "unverified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/config"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-7a3527"
anchors = ["crates/roko-cli/src/config.rs", "crates/roko-cli/src/orchestrator/"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-7a3527"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rn 'ParallelExecutor' crates/roko-cli/src --include=*.rs | grep -v test"
+++

## Problem

`ExecutorConfig`, `ParallelExecutor` and `Config.executor` are parsed but have no production reader, so `[executor]` settings look effective when they are not.

## Plan

Delete them, or wire them where the Graph path needs them. A removed key goes into REMOVED_CONFIG_KEYS (gap-6bc156) with a warning on load.

## Done when

- The verify passes, and old roko.toml files still load.

## Notes

- Reported on 2026-10-01 by the worker on gap-7a3527, during the evening close-out round.
