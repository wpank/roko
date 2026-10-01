+++
id = "gap-fab2cc"
kind = "gap"
title = "Graph checkpoints record no stop cause, so a deadline or conductor stop shows only in the summary"
status = "open"
triage = "unverified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/graph_checkpoint"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-9efe8e"
anchors = ["crates/roko-cli/src/graph_checkpoint.rs", "crates/roko-cli/src/graph_execution/plan_runner.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-9efe8e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib checkpoint_records_the_stop_cause"
+++

## Problem

gap-9efe8e added a `Deadline` interrupt. Graph checkpoints have no stop-cause field, so `deadline` shows only in the run summary and the `--log-file`, and a conductor stop still requests Terminate. `commands/plan.rs:2452` and `do_cmd.rs:785` carry stale "exit 130/143" comments.

## Plan

Add a stop cause to the checkpoint (interrupt label), and fix the comments. Add a test named `checkpoint_records_the_stop_cause`.

## Done when

- `cargo test -p roko-cli --lib checkpoint_records_the_stop_cause` passes.

## Notes

- Reported on 2026-10-01 by wk-planrun, working on gap-9efe8e, during the evening close-out round.
