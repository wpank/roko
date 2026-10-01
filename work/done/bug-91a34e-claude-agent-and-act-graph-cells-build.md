+++
id = "bug-91a34e"
kind = "bug"
title = "claude-agent and act graph cells build a pass-through stub, so roko graph run accepts agent nodes that dispatch nothing"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-graph"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-3d5cce"
anchors = ["crates/roko-graph/src/cells/cognitive.rs", "examples/graphs/task-execution.toml"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-3d5cce"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-graph --lib stub_cells_are_refused"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:19Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T18:14:48Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

The `claude-agent` and `act` cell types build the pass-through ActCell and are not marked as stubs. A production `roko graph run` therefore accepts graphs like `examples/graphs/task-execution.toml`, and their agent nodes pass input through without dispatching anything.

## Plan

Refuse stub cells in production graph runs (or mark them so validation warns), until gap-3d5cce wires ActCell. Add a test named `stub_cells_are_refused_*`.

## Done when

- `cargo test -p roko-graph --lib stub_cells_are_refused` passes.

## Notes

- Reported on 2026-10-01 by wk-filer4, working on gap-3d5cce, during the evening close-out round.
- 2026-10-01 (wk-filer4): implemented on work/gap-cd51b7; cargo verification deferred to the batch check.
- 2026-10-01 (wk-filer4): What changed: the `act` and `claude-agent` descriptors are now stubs (new `CellDescriptor::with_stub`), and ActCell reports `is_stub`. Production starts (`roko graph run`, authored graphs, Hot Graphs) therefore refuse graphs that use these cells, and `roko graph show` marks them. `roko agent serve --allow-stub-cognitive-loop` opts in through the new `HotCheckpointOptions::allow_test_stubs`. New tests: `stub_cells_are_refused_for_agent_nodes_in_production_starts` (engine.rs) and `stub_cells_are_refused_by_hot_graph_starts_unless_allowed` (hot.rs). The cognitive-loop example test now checks the refusal, and the four example graphs that use the cell say they cannot run yet.
- 2026-10-01 (wk-filer4): `roko graph validate` still reports such a graph as valid: `GraphEngine::validate` checks only cycles and unknown cell types, while the stub check runs at start.
