+++
id = "bug-91a34e"
kind = "bug"
title = "claude-agent and act graph cells build a pass-through stub, so roko graph run accepts agent nodes that dispatch nothing"
status = "open"
triage = "unverified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-graph"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-3d5cce"
anchors = ["crates/roko-graph/src/cells/cognitive.rs", "examples/graphs/task-execution.toml"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-3d5cce"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-graph --lib stub_cells_are_refused"
+++

## Problem

The `claude-agent` and `act` cell types build the pass-through ActCell and are not marked as stubs. A production `roko graph run` therefore accepts graphs like `examples/graphs/task-execution.toml`, and their agent nodes pass input through without dispatching anything.

## Plan

Refuse stub cells in production graph runs (or mark them so validation warns), until gap-3d5cce wires ActCell. Add a test named `stub_cells_are_refused_*`.

## Done when

- `cargo test -p roko-graph --lib stub_cells_are_refused` passes.

## Notes

- Reported on 2026-10-01 by wk-filer4, working on gap-3d5cce, during the evening close-out round.
