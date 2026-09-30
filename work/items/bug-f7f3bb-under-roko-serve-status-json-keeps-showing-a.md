+++
id = "bug-f7f3bb"
kind = "bug"
title = "Under roko serve, status.json keeps showing a finished run as active, because the serve PID is still alive"
status = "open"
triage = "unverified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["roko-cli/runner/status_file"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-runstate's report, checked on work/bug-4c4eea at 3cb4a818f)"
anchors = ["crates/roko-cli/src/runner/status_file.rs"]
lane = "tracker"
parent = "spec-f2463d"
links = { depends_on = [], blocks = [], related = ["bug-4c4eea"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_finished_run_is_not_active_under_serve' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_finished_run_is_not_active_under_serve"
+++

## Problem

`.roko/state/status.json` treats a run as stale only when the file is older than 60 seconds and the writer's PID is dead (`crates/roko-cli/src/runner/status_file.rs:11`). Under `roko serve`, the writer is the serve process, which stays alive after a run finishes. So a finished run keeps showing as active.

## Why it matters

A record of how the backlog gets done (epic spec-f2463d): status views and dashboards show runs that ended as still running. p3.

## Where

The staleness rule and the writer in `status_file.rs`.

## Plan

1. Write an explicit terminal state (finished, failed, cancelled) when a run ends, and treat a terminal state as inactive whatever the PID. Or key liveness on a per-run id, not the PID.
2. Add `a_finished_run_is_not_active_under_serve`.

## Done when

- [ ] A run that finishes under serve shows as finished.
- [ ] The `[[verify]]` command passes.
