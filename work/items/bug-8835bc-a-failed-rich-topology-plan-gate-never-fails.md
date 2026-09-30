+++
id = "bug-8835bc"
kind = "bug"
title = "A failed rich-topology plan gate never fails its task: PlanGateCell returns Ok, and the gate's success edge is EdgeCondition::Success"
status = "open"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-graph/cells", "roko-execution/workflow"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-integrate's report, branch work/bug-50caf2 at f0445319f)"
anchors = ["crates/roko-graph/src/cells/plan_gate.rs", "crates/roko-execution/src/workflow/templates.rs"]
lane = "rust-hot"
parent = "spec-a0e40a"
links = { depends_on = [], blocks = [], related = ["bug-50caf2", "bug-056b40", "gap-6daad9"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_failed_plan_gate_fails_its_task' crates/roko-graph/src/ crates/roko-execution/src/ crates/roko-cli/src/ && cargo test -p roko-graph -p roko-execution -p roko-cli --lib a_failed_plan_gate_fails_its_task"
+++

## Problem

`PlanGateCell` (`crates/roko-graph/src/cells/plan_gate.rs`) returns `Ok` when its checks fail. It reports the failure only in its output: `passed: false` (:341) and the tag `gate.passed = false` (:451). The rich-topology templates wire the gate's edge to the success path with `EdgeCondition::Success` (`crates/roko-execution/src/workflow/templates.rs:308`, :334, :360, :435, :459), and that condition only looks at whether the cell returned `Ok`. So a failed gate still takes the success edge, and its task never fails. wk-integrate found this while working on bug-50caf2; it predates that branch.

## Why it matters

Integration and a whole-plan check (epic spec-a0e40a), p1: under `--rich-topology`, gates don't gate. Tasks whose checks failed are recorded and delivered as successes, which is the dishonest-verdict failure the project is built to prevent.

## Where

`PlanGateCell::run` and its output. The edge conditions in the templates, and how the engine evaluates `EdgeCondition::Success`.

## Plan

Pick one:

- **(a)** Make `PlanGateCell` return an error, or a failed status the engine treats as failure, when its checks fail.
- **(b)** Route the gate's edges on its tag, with a conditional edge on `gate.passed`.

(a) matches how every other cell reports failure. Then add `a_failed_plan_gate_fails_its_task`, which runs a rich topology whose gate fails.

## Done when

- [ ] A failed gate fails its task under `--rich-topology`.
- [ ] The `[[verify]]` command passes.

## Notes

- 2026-09-30 (wk-integrate): Implemented on `work/bug-453481` at `58aac9a3b`; cargo verification deferred to the batch check.
  - In the worktree: `cargo check -p roko-cli -p roko-graph --lib --tests` and `cargo clippy -p roko-cli -p roko-graph -p roko-execution --no-deps -D warnings` clean; nightly rustfmt clean. `cargo test -p roko-graph --lib`: 472 passed; the verify's filter passes in roko-graph, roko-execution and roko-cli.
  - Option (a): a failed `PlanGateCell` returns `RokoError::Verify { gate: "plan.gate" }`, after settling any handed-on worktree (kept `RetainForFailure`). The message names the attempt and each failed rung with its evidence (400 bytes each), or that no rung ran; a failed rung without evidence keeps its reasons.
  - The item's `templates.rs` lines are the `roko run` workflow templates: their builders have no production caller and no `workflow.gate` cell is registered. The live rich topology is `ProductionPlanTopology` (`plan.gate` → `plan.success-boundary` on `EdgeCondition::Success`), which this fixes. `cell_types::GATE` now documents that a gate cell must fail with an error.
  - Test: `a_failed_plan_gate_fails_its_task` (runs a two-task rich topology through `GraphEngine`: the gate node fails, its success boundary never completes, the graph fails, and the dependent task's executor never runs).
