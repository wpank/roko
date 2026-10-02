+++
id = "bug-3a3968"
kind = "bug"
title = "A verify step's waits for siblings and for the compile lock don't watch for a stop"
status = "open"
triage = "unverified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-02
updated = 2026-10-02
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-c33c6e"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-c33c6e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib verify_waits_end_on_a_stop"
+++

## Problem

The verify loop waits before a step: the sibling wait in `begin_step`, and the compile-lock wait. Neither watches for a stop. Behind another process's long cargo run, an attempt can outlast the 3 s drain and 2 s settle; its flow is then abandoned, and the attempt never gets a verdict.

## Plan

Race both waits against the stop signal, and settle a stopped wait as cancelled. Add a test named `verify_waits_end_on_a_stop`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-02 by wk-scheduler, working on bug-c33c6e, during the overnight close-out round.
