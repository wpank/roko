+++
id = "gap-dd4826"
kind = "gap"
title = "TaskBlocked is not in the run event log's lifecycle set, so it is not written to disk immediately"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-runstate's report on gap-f59fe9)"
anchors = ["crates/roko-cli/src/graph_execution/event_log.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-f59fe9", "bug-230de6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'TaskBlocked' crates/roko-cli/src/graph_execution/event_log.rs"
+++

## Problem

event_log.rs (about line 333) syncs lifecycle events immediately and appends the rest relaxed; the new TaskBlocked event is not in the lifecycle set.

## Why it matters

A crash right after a block can lose the event that explains the run's end state.

## Plan

Add TaskBlocked to the lifecycle set.

## Done when

- [ ] TaskBlocked is synced like other lifecycle events
- [ ] The `[[verify]]` command passes.

## Notes

2026-10-01 (wk-planrun): implemented on work/gap-dd4826; cargo verification deferred to the batch check.
The workspace event log's lifecycle set moved into `is_lifecycle_event` (`graph_execution/event_log.rs`) and now includes `TaskBlocked`, so a blocked task is synced to `.roko/events.jsonl` and flushes the run's index as it is written. Test: `workspace_event_log_syncs_a_blocked_task`.
