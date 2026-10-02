+++
id = "bug-147b45"
kind = "bug"
title = "roko graph validate accepts graphs whose agent nodes are pass-through stubs"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-graph"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "4dc345a29"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-91a34e"
anchors = ["crates/roko-graph/src/engine.rs", "crates/roko-cli/src/commands/graph.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-91a34e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn validate_flags_stub_cells' crates/roko-graph/src/engine.rs && cargo test -p roko-graph --lib validate_flags_stub_cells"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T00:38:58Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T19:02:31Z"
forced = false
evidence = "Gate 6e on ddf47dbd6 plus its fixes, re-run at 3ac297a00 and merged as 4dc345a29 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 10 crates; lib tests pass (roko-cli 3420, roko-agent 2289, roko-core 1984, roko-learn 1230, roko-serve 1013, roko-graph 488, roko-conductor 316, roko-acp 220, roko-execution 193, roko-dreams 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp integration, smoke, graph_plan_callers, graph_timeout_matrix and plan_conversion pass; bin 445; scripts/test_run_evidence_graph.py 9/9 against the gate binary; Cargo.lock unchanged. Implemented in this round; the item's notes name the change and its test."
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
- 2026-10-02 (wk-climain, gate 6e fix): `roko graph run` first runs `AuthoredGraphController::preflight`
  (roko-execution), whose structural check built an engine without `with_allow_test_stubs` and so started
  refusing the `noop` test stub. That broke roko-cli's `graph_command::tests::graph_run_with_json_flag_produces_json_output`
  and `graph_run_with_quiet_flag_succeeds`, whose run engine allows stubs under `cfg!(test)`. The preflight now
  allows stubs and leaves them to the start, which knows the run's policy. That is where stub refusal lived before
  this item. In production `graph run` still refuses a stub graph, now at the run engine's `validate()`, and
  `graph validate` still reports it. Test: `preflight_leaves_stub_cells_to_the_start` (roko-execution).
