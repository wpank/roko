+++
id = "gap-f67a72"
kind = "gap"
title = "Leftover attempt checkouts are never removed: reclaim_idle has no production caller"
status = "open"
triage = "unverified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/orchestrator"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-bdfb1d"
anchors = ["crates/roko-cli/src/orchestrator/worktree/cleanup.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-bdfb1d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rn 'reclaim_idle' crates/roko-cli/src --include=*.rs"
+++

## Problem

Attempt checkouts left by crashed or interrupted runs are never removed: `reclaim_idle` has no production caller, and `roko doctor disk` only reports orphans. Resumed runs re-attach kept checkouts (bug-056b40), so cleanup needs a retention rule.

## Plan

Decide the retention rule (for example: no live checkpoint references the checkout, and it is older than N days), then call the cleanup from `roko doctor disk --fix` or at run start.

## Done when

- A retention rule is recorded, and a test shows an orphaned checkout removed while a resumable one is kept.

## Notes

- Reported on 2026-10-01 by the worker on bug-bdfb1d, during the evening close-out round.
