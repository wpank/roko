+++
id = "gap-3505fb"
kind = "gap"
title = "PlanMerger and the roko-execution workflow templates' gate builders have no production caller"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/runner/merge", "roko-execution/workflow"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-integrate's report, checked on work/bug-453481 at 2eeda438d)"
anchors = ["crates/roko-cli/src/runner/merge.rs", "crates/roko-execution/src/workflow/templates.rs"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = ["bug-453481"], blocks = [], related = ["bug-453481", "bug-8835bc"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rq 'pub struct PlanMerger' crates/"
+++

## Problem

On bug-453481's branch, delivery no longer uses `PlanMerger` (`crates/roko-cli/src/runner/merge.rs:125`), which leaves it with no production use. wk-integrate also found that the gate builders in `crates/roko-execution/src/workflow/templates.rs` have no production caller. The rich-topology templates wire their gate edges there (bug-8835bc).

## Why it matters

Hygiene (epic spec-9a3131): dead merge and template code gets maintained and misleads readers about how plans deliver. p3.

## Where

The anchors.

## Plan

1. Delete `PlanMerger` and its config once bug-453481 lands; at f48cc207d it still has a production use, which that branch removes.
2. Delete the unused template gate builders, or wire them where the rich topology is built (bug-8835bc decides which).

## Done when

- [ ] `PlanMerger` is gone, and the template gate builders are wired or gone.
- [ ] The `[[verify]]` command passes (it checks that `PlanMerger` is gone; check the template builders by hand).

## Notes

- 2026-09-30 (wk-integrate): Implemented on `work/spec-f830c4` at `f0b8f3e59`; cargo verification deferred to the batch check.
  - Deleted `PlanMerger` with the rest of Runner v2's merge path in `runner/merge.rs` (config, `MergeBackend`, `RegressionGate`, launch and dispatch types, their tests). The module keeps the git plumbing that delivery, batch integration and acceptance share. `tests/merge_proof.rs` and `tests/runner_integration.rs` lost their PlanMerger cases.
  - Deleted the workflow templates' `build_generation_subgraph`, `build_autofix_subgraph`, `node_ids` and `cell_types`: nothing called them and no `workflow.*` cell is registered; the rich topology's gates live in `ProductionPlanTopology` (bug-8835bc). `docs/v3/04-EXECUTION.md` §11 updated.
  - Now used only by tests: `MergeQueue`; unused: `RUNG_MERGE` and Runner v2 types such as `GateCompletionKind`.
