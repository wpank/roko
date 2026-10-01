+++
id = "bug-147b45"
kind = "bug"
title = "roko graph validate accepts graphs whose agent nodes are pass-through stubs"
status = "open"
triage = "unverified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-graph"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-91a34e"
anchors = ["crates/roko-graph/src/engine.rs", "crates/roko-cli/src/commands/graph.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-91a34e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-graph --lib validate_flags_stub_cells"
+++

## Problem

bug-91a34e made `roko graph run` refuse stub cells, and `show` marks them. `roko graph validate` still calls such a graph valid, because `GraphEngine::validate` checks only cycles and unknown types.

## Plan

Make validate report stub cells: an error, or a warning that `run` will refuse them. Add a test named `validate_flags_stub_cells`.

## Done when

- `cargo test -p roko-graph --lib validate_flags_stub_cells` passes.

## Notes

- Reported on 2026-10-01 by wk-filer4, working on bug-91a34e, during the evening close-out round.
