+++
id = "bug-96341e"
kind = "bug"
title = "Legacy DELETE /api/jobs/{id} bypasses the shared JobExecutionService and never signals a running job"
status = "open"
triage = "unverified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-serve/routes"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-2a9ed7"
anchors = ["crates/roko-serve/src/routes/jobs.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-2a9ed7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-serve --lib legacy_job_delete_signals"
+++

## Problem

gap-2a9ed7 made job cancellation go through one shared `JobExecutionService`. The legacy `DELETE /api/jobs/{id}` route still bypasses it, so it sends a running job no signal. Only the runner's re-read keeps the status right.

## Plan

Route the legacy delete through the shared service, or retire it. Add a test named `legacy_job_delete_signals_*`.

## Done when

- `cargo test -p roko-serve --lib legacy_job_delete_signals` passes.

## Notes

- Reported on 2026-10-01 by wk-serve2, working on gap-2a9ed7, during the evening close-out round.
