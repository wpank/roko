+++
id = "bug-4862cf"
kind = "bug"
title = "The rich-topology gate adapter uses GatesConfig::default(), not the run's [gates]"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/runner/gate_adapter"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "a4298a644"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-integrate's report, checked on work/spec-f830c4 at 8268c7498)"
anchors = ["crates/roko-cli/src/runner/gate_adapter.rs"]
lane = "rust-hot"
parent = "spec-a0e40a"
links = { depends_on = ["spec-f830c4"], blocks = [], related = ["spec-f830c4", "bug-8835bc", "gap-6daad9", "bug-9c5973"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn rich_topology_gates_use_the_runs_gates_config' crates/roko-cli/src/ && cargo test -p roko-cli --lib rich_topology_gates_use_the_runs_gates_config"

[closed]
at = 2026-09-30
commit = "a4298a644"
by = "wk-integrate"
evidence = "Fixed by 7920e6141 (plan_cell_resources gives the gate adapter the run's [gates]). rich_topology_gates_use_the_runs_gates_config (roko-cli lib, a4298a644) passes: the run's [gates] reaches the gate pipeline and its max_rung bounds it."
+++

## Problem

On `work/spec-f830c4`, the rich topology's gates run through `RunnerProductionGateAdapter` (`crates/roko-cli/src/runner/gate_adapter.rs`). Its `SharedGateEvaluator` implementation builds `let mut gates_config = roko_core::config::GatesConfig::default();` (:340), instead of using the run's `[gates]` from `roko.toml`. Custom rungs, timeouts, `clippy_enabled` and the other gate settings are ignored under `--rich-topology`.

## Why it matters

Integration and a whole-plan check (epic spec-a0e40a): the rich topology gates with settings nobody chose.

## Where

`RunnerProductionGateAdapter` in `gate_adapter.rs`, and where it's constructed.

## Plan

1. Pass the run's `GatesConfig` into the adapter when the rich topology is built.
2. Add `rich_topology_gates_use_the_runs_gates_config`.

## Done when

- [ ] Rich-topology gates use the run's `[gates]`.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on spec-f830c4's branch. Fix it with bug-9c5973.
