+++
id = "bug-9c5973"
kind = "bug"
title = "Each rich-topology rung request re-runs the gate pipeline up to that rung, so compile runs three times per gate"
status = "open"
triage = "unverified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-graph/cells/plan_gate", "roko-cli/runner/gate_adapter"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-integrate's report, checked on work/spec-f830c4 at 8268c7498)"
anchors = ["crates/roko-graph/src/cells/plan_gate.rs", "crates/roko-cli/src/runner/gate_adapter.rs"]
lane = "rust-hot"
parent = "spec-a0e40a"
links = { depends_on = ["spec-f830c4"], blocks = [], related = ["spec-f830c4", "bug-4862cf", "bug-8835bc"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_gate_runs_each_rung_once' crates/roko-graph/src/ && cargo test -p roko-graph --lib a_gate_runs_each_rung_once"
+++

## Problem

On `work/spec-f830c4`, `PlanGateCell` asks for its rungs one request at a time (`run_rungs`, `crates/roko-graph/src/cells/plan_gate.rs:254`). Each request runs the gate pipeline from the first rung up to the requested one. The early rungs, compile above all, run again for every later rung: three times per gate with three rungs (wk-integrate).

## Why it matters

Integration and a whole-plan check (epic spec-a0e40a): every gate costs about three compiles, which dominates the time of a rich-topology run.

## Where

`run_rungs` in `plan_gate.rs`, and the adapter's pipeline invocation.

## Plan

1. Run the pipeline once, up to the highest rung needed, and read each rung's verdict from that one run. Or cache the rung results per commit within a gate.
2. Add `a_gate_runs_each_rung_once`.

## Done when

- [ ] Each rung runs once per gate.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on spec-f830c4's branch.
