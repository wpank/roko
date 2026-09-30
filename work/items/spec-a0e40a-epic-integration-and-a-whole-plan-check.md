+++
id = "spec-a0e40a"
kind = "spec"
title = "Epic: integration and a whole-plan check"
status = "open"
triage = "unverified"
severity = "p1"
goal = "golden-path"
size = "L"
subsystem = ["roko-cli/graph_execution", "roko-cli/orchestrator", "roko-graph/cells"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e6"
discovered_from = "tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md (P1 #12; tldr/04 step 9)"
anchors = ["crates/roko-cli/src/orchestrator/worktree/mod.rs::accept_attempt", "crates/roko-cli/src/graph_execution/delivery.rs::GitDeliveryBackend", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan", "crates/roko-cli/src/task_parser.rs::TaskMeta"]
doc = "tmp/cybernetic-harness/workstreams/PLAN.md"
lane = "rust-hot"
links = { depends_on = ["gap-3b5361", "bug-a3760a", "spec-f830c4", "gap-60233f", "bug-50caf2", "gap-4ec59f", "gap-af00b1", "gap-0d64d5", "bug-aaa924", "bug-453481", "bug-207f35", "bug-056b40", "bug-8835bc", "gap-6daad9", "bug-8cf581"], blocks = [], related = ["gap-d58ae8", "gap-415c54", "bug-53475e", "gap-439794"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn c3_each_passed_task_commits_once_on_the_plan_branch' crates/roko-cli/tests/ && grep -rqw 'fn c4_meta_verify_catches_tasks_that_break_together' crates/roko-cli/tests/ && cargo test -p roko-cli --test plan_branch_integration"
+++

## Problem

On the Graph path nothing integrates a plan's results or checks them as a whole:

- No task result is committed, merged or attributed. `accept_attempt` has no production caller, and neither
  `GitDeliveryBackend` nor `CliCompletionDeliveryService` is constructed outside its own tests.
- The merge backend that would be wired runs `git checkout <target>` in your working tree.
- `--worktree-per-task` strands successful edits: the dirty attempt worktree cannot be released, and nothing
  merges it back.
- There is no plan-level verify: `[meta]` has no `verify`, and only per-task verify steps run.

On 09-28 every portal plan was green while the assembled product was unusable (CASE-006). Each of the five hand
merges on 09-28/29 had a semantic break that the text merge let through (research note B2).

## Why it matters

tldr/04 step 9 ("Integrate") is ORPHANED / MISSING. It is P1 #12 in tldr/05 and gates G3–G4 in assessment W8.
Without it, "the plan passed" says nothing about the combined result, and Roko's output still has to be merged and
checked by hand. Epics E9 (diff check) and E11 (acceptance) build on it.

## Where

- `crates/roko-cli/src/orchestrator/worktree/mod.rs`: `accept_attempt`, and `format_branch_name`
  (`roko/plan/<plan_id>`).
- `crates/roko-cli/src/graph_execution/delivery.rs`: `GitDeliveryBackend` (`git_merge`, `run_regression`) and
  `CliCompletionDeliveryService`.
- `crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan`: where a plan's outcome is decided
  (`plan_outcome`).
- `crates/roko-cli/src/task_parser.rs::TaskMeta`: the `[meta]` table.
- `crates/roko-graph/src/cells/plan_gate.rs`: the rich topology's per-task gate.

## Current state

Checked at `41c7ffbd6`:

- All five existing children are still open:
  - no `.accept_attempt(` call outside `orchestrator/worktree/tests.rs`;
  - `delivery.rs:119` still runs `checkout`;
  - `plan_gate.rs:86` still sends attempt 0, and `plan_gate.rs:150` still uses `current_dir()`;
  - `default_use_worktrees()` still returns `false`.
- `spawn_plan_verify` (`runner/gate_dispatch.rs:1619`), Runner-v2's plan-level verify, has only test callers.
  Mine it for the whole-plan gate.
- Today's `bbf6517fc` made plan runs skip only a failed task's dependants, so independent tasks now finish after a
  failure.
- The portal session still has `fix/hermetic-child-env`, `fix/diagnose-graph-runs` and
  `feat/learning-completion-loops` open. Check them for `graph_execution/` changes before starting.
- **gap-4ec59f's premise is partly wrong.**
  - It flips `ExecutorConfig::use_worktrees`, which only `config.rs` reads. The Graph path never reads it, and
    tldr/05 §4 lists it as dead code.
  - "On by default" conflicts with tldr/05 decision 4 (default: per-task worktrees stay opt-in).
  - Its `[[verify]]` pins that dead default to `true`. Rewrite it before anyone picks it up (see Notes).
- **spec-f830c4 predates this design.** It builds a `roko/batch/<run-id>` branch, and its `[[verify]]` requires
  `CliCompletionDeliveryService::new` in `plan_runner.rs`.
- bug-50caf2 is reachable only through `--rich-topology`, which tldr/05 §3 proposes to park.

## Plan

This is the implementation plan. It follows tldr/05 decision 4's default: a plan branch, delivered as a PR or
fast-forward that you merge, with per-task worktrees opt-in.

1. **Make merging safe** (bug-a3760a): merge with git plumbing and a compare-and-swap ref update, never a checkout in
   your tree. Independent; can start now.
2. **Fix the rich-topology gate** (bug-50caf2): gate the attempt's own tree, and fail closed. Independent.
3. **Commit per task on the plan branch** (gap-3b5361, after E4.2 gap-96f7ed): when a task's gate passes, commit
   exactly its pathspec on `roko/plan/<plan_id>` and record `accepted_commit` in the checkpoint.
4. **Deliver the plan branch** (spec-f830c4, split as in Notes): a fast-forward or PR through the fixed
   `GitDeliveryBackend`, whose regression step becomes the plan's `[meta] verify`.
5. **Add the whole-plan gate** (gap-60233f): `[meta] verify` on the integrated tree, defaulting to fmt, clippy and
   the affected crates' tests.
6. **Retarget worktree isolation** (gap-4ec59f): opt-in worktrees based on the plan-branch tip and merged back
   through `MergeQueue`, with startup repair wired.
7. **Prove it** with integration tests C3 and C4 (gap-af00b1). They join the golden-path suite (epic E11).

Order: 1 and 2 now. Then 3 → 4 → 5 → 6 → 7, one at a time, because they all touch `graph_execution/`.

## Done when

- [x] gap-3b5361: Successful plan attempts are never accepted: accept_attempt has no production caller (existing item)
- [x] bug-a3760a: The Graph engine's merge step runs git checkout in the user's working tree (existing item)
- [ ] spec-f830c4: #404 — Batch Branch Integration (existing item)
- [ ] gap-60233f: [meta] verify: a whole-plan gate that runs on the integrated result
- [x] bug-50caf2: PlanGateCell gates the process working directory as attempt 0 (existing item)
- [ ] gap-4ec59f: Worktree Isolation: Flip Default and Add Startup Repair (existing item)
- [ ] gap-af00b1: Integration tests C3 and C4: per-task commits on a plan branch, and a whole-plan gate that catches
- [ ] gap-0d64d5: Golden-path step 10: an opt-in hold that shows each task's diff and waits for approval before it merges
- [x] bug-aaa924: The delivery regression check builds the workspace from a cold target dir on every delivery
- [x] bug-453481: Delivery merges the branch head instead of the verified commit_oid, so later commits land unverified
- [x] bug-207f35: GitMergeBackend still merges in, and auto-commits, the checkout it is given
- [ ] bug-056b40: On resume, attempts start from HEAD instead of the plan branch, and retained attempt worktrees aren't re-attached
- [x] bug-8835bc: A failed rich-topology plan gate never fails its task: PlanGateCell returns Ok, and the gate's success edge is EdgeCondition::Success
- [ ] gap-6daad9: plan_runner injects no CellResources, so --rich-topology still stops at every gate
- [ ] bug-8cf581: Each delivery's regression checkout gets a new temporary path, so workspace crates rebuild every delivery and leave stale artifacts
      tasks that break together
- [ ] The epic's `[[verify]]` command (tests C3 and C4) passes on the merged branch.

## Notes

- **Existing children keep their goal and severity.** gap-4ec59f stays p0 in `core`; the others stay in `core` or
  `features`. A move to `golden-path` is proposed in `PLAN.md` §5, not applied.
- **Proposed splits (not applied):**
  - spec-f830c4 (L) into two M items:
    - (a) the per-task pathspec commit on the plan branch plus `accepted_commit` (shares code with gap-3b5361);
    - (b) delivery of the plan branch as a fast-forward or PR, with `[meta] verify` as the regression step.

    Retire the `roko/batch/<run-id>` naming, or keep it only for multi-plan runs.
  - gap-4ec59f (L) into two M items:
    - (a) Graph-path isolation stays opt-in in `GraphPlanRunParams`; attempts start from the plan-branch tip and are
      merged back; delete `ExecutorConfig::use_worktrees`;
    - (b) wire `clear_stale_locks` and `prune` into the Graph preflight.

    Its `[[verify]]` must be rewritten to match.
- **Hot files:** `graph_execution/` (`plan_runner.rs`, `delivery.rs`, `workspaces.rs`) and `graph_task_dispatch.rs`.
  Steps 3–6 wait for the portal session's branches and the dispatch-file split (E15.4, gap-c8e1f1).
- Never delete worktrees or plan branches that Roko did not create for the current merge.
