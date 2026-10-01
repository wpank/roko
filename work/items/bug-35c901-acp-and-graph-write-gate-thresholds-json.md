+++
id = "bug-35c901"
kind = "bug"
title = "ACP and Graph write gate-thresholds.json with different schemas, so each writer drops the other's fields"
status = "open"
triage = "unverified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-acp", "roko-cli/runner"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-7f0dc8"
anchors = ["crates/roko-acp/src/runner.rs", "crates/roko-cli/src/runner/persist.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-7f0dc8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-acp --lib gate_thresholds_schema"
+++

## Problem

roko-acp's `AdaptiveThresholds` and the Graph path's `GateThresholds` are different schemas persisted to the same `gate-thresholds.json`. When either saves, it drops the fields only the other knows, even under the shared lock (bug-7f0dc8).

## Plan

Use one schema, or keep unknown fields round-trip (`#[serde(flatten)] extra`). Add a test named `gate_thresholds_schema_*` that loads and saves one writer's file with the other and checks no field is lost.

## Done when

- `cargo test -p roko-acp --lib gate_thresholds_schema` passes.

## Notes

- Reported on 2026-10-01 by wk-honestbench, working on bug-7f0dc8, during the evening close-out round.
