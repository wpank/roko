+++
id = "bug-7f0dc8"
kind = "bug"
title = "roko-acp saves gate-thresholds.json without the update lock"
status = "open"
triage = "unverified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-acp"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-e0f472"
anchors = ["crates/roko-acp/src/runner.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-e0f472"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -n 'save_gate_thresholds\\|thresholds.save' crates/roko-acp/src/runner.rs"
+++

## Problem

bug-e0f472 moved Graph runs' gate-threshold updates into a locked read-modify-write (`GateThresholds::update_locked`). `crates/roko-acp/src/runner.rs` (around line 1793) still loads, updates and saves the file without the lock, so an ACP session and a plan run can drop each other's observations. `persist.rs::maybe_flush_gate_thresholds` is also called only by its tests.

## Plan

Use the locked update in roko-acp, and delete `maybe_flush_gate_thresholds` or give it a caller.

## Done when

- The verify passes, and roko-acp's gate tests pass.

## Notes

- Reported on 2026-10-01 by the worker on bug-e0f472, during the evening close-out round.
