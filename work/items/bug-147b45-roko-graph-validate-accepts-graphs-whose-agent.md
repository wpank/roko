+++
id = "bug-147b45"
kind = "bug"
title = "roko graph validate accepts graphs whose agent nodes are pass-through stubs"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-graph"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "c7560e213"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-91a34e"
anchors = ["crates/roko-graph/src/engine.rs", "crates/roko-cli/src/commands/graph.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-91a34e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn validate_flags_stub_cells' crates/roko-graph/src/engine.rs && cargo test -p roko-graph --lib validate_flags_stub_cells"
+++

## Problem

bug-91a34e made `roko graph run` refuse stub cells, and `show` marks them. `roko graph validate` still calls such a graph valid, because `GraphEngine::validate` checks only cycles and unknown types.

## Plan

Make validate report stub cells: an error, or a warning that `run` will refuse them. Add a test named `validate_flags_stub_cells`.

## Done when

- `cargo test -p roko-graph --lib validate_flags_stub_cells` passes.

## Notes

- Reported on 2026-10-01 by wk-filer4, working on bug-91a34e, during the evening close-out round.
- 2026-10-02 (wk-climain): implemented on work/bug-28c193; cargo verification deferred to the batch check.
  `GraphEngine::validate` reports each node whose descriptor is a stub ("node 'act' is a stub cell ('claude-agent');
  production starts refuse it"), sorted, unless the engine was built `with_allow_test_stubs(true)`. That is the
  check `validate_for_start` makes, so `roko graph validate` now fails where `roko graph run` refuses, and
  `graph run`'s preflight `validate()` reports the stub before the engine starts. Error, not warning: such a graph
  cannot run in production. The example graphs that use `act`/`claude-agent` (cognitive-loop, conditional-branch,
  score-compose, task-execution) now report it.
  `roko-graph/tests/plan_conversion.rs::cognitive_loop_loads_and_validates` expected no issues for the loop's
  `claude-agent` node; it now expects that one issue, and no issues with stubs allowed. Test:
  `validate_flags_stub_cells`. The verify now guards the cargo filter with a grep for the test.
