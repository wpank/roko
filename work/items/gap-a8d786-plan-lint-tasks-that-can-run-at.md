+++
id = "gap-a8d786"
kind = "gap"
title = "Plan lint: tasks that can run at the same time must not share files"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan_policy"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "8a88c6267"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e8"
discovered_from = "tmp/cybernetic-harness/tldr/research/B2-dag-worktrees-merge.md (intra-plan write-set check, proposed item W3)"
anchors = ["crates/roko-cli/src/plan_policy.rs::validate_plan_budgets"]
lane = "rust-cold"
parent = "spec-e57870"
links = { depends_on = ["gap-439794"], blocks = [], related = ["gap-272448", "find-d1a883"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn concurrent_tasks_sharing_a_file_are_flagged' crates/roko-cli/src/ && cargo test -p roko-cli --lib concurrent_tasks_sharing_a_file_are_flagged"
+++

## Problem

`roko plan validate` does not check whether two tasks that can run at the same time declare the same file. Two tasks
can be in flight together when neither depends on the other, directly or transitively. The only file rule,
`PLAN_FRAGMENTED_OWNERSHIP` in `plan_policy.rs`, counts the *serial* owners of a file, and only under the
generated-plan policy. `runner/plan_dag.rs::CrateOverlap` warns about overlap across plans, not within one.

## Why it matters

- tldr/04 design rule 2: parallel siblings need disjoint write sets.
- gap-439794 (E7.3) makes the runtime serialize overlapping tasks. That is safe, but it silently gives up the
  parallelism the planner intended; this lint tells the author at plan time.
- gap-272448 (E7.4) defaults `max_parallel` to the DAG's width only when write sets are disjoint, so this rule is what
  lets a plan run wide.

Assessment W8's canary C4 asks for it. Part of epic spec-e57870.

## Where

- `crates/roko-cli/src/plan_policy.rs::validate_plan_budgets`: it already builds the per-file owner map and uses
  `depends_on_transitively` for `PLAN_FRAGMENTED_OWNERSHIP`. The new rule is its complement: two owners of a file
  where neither depends on the other.
- The overlap definition: the helper gap-439794 adds (equal paths, or one path a directory prefix of the other,
  mirroring `sibling_settle::declares`).
- Reported by `plan validate` through `validate_plan_context` (`plan_validate.rs:158`).

## Current state

Checked at `41c7ffbd6`: no overlap rule in `plan_policy.rs` or `plan_validate.rs`.

## Plan

1. Add `PLAN_CONCURRENT_OVERLAP`: for each pair of tasks with overlapping `files` and no dependency path between
   them, report both task ids and the shared path.
2. Severity: an error when `max_parallel > 1`, or when E7.4's DAG-width default applies; a warning otherwise.
   `--strict` fails on both.
3. Tests: `concurrent_tasks_sharing_a_file_are_flagged`, and a companion showing that the same pair with a
   dependency edge is not flagged.

## Done when

- [ ] Two independent tasks that both declare `src/lib.rs` are reported; adding `depends_on` between them clears it.
- [ ] Running the rule over `plans/` lists today's offenders in the commit message.
- [ ] The `[[verify]]` command passes.

## Notes

- Waits for gap-439794, so that the lint and the runtime agree on what "overlap" means.
- Tasks without `files` declare nothing. Leave them to find-d1a883 (footprints trust declared files).
- Implemented on `work/gap-a8d786` at `81b944eed`; cargo verification deferred to the batch check.
  - `PLAN_CONCURRENT_OVERLAP` is reported from `validate_plan_budgets` when `max_parallel > 1`, using the engine's
    own rule (`roko_graph::exclusion::first_overlap`, now public). `plan validate` shows it as an error.
    `load_plan` only logs it (`PlanPolicyViolation::is_advisory`), because the engine already runs such tasks one
    at a time. The generated-plan budget rejects it like its other findings.
  - Not done: the warning for `max_parallel = 1` plans. `plan validate` maps every policy finding to an error, and
    `plan_validate.rs` was being edited in another worktree. gap-272448's auto `max_parallel` should also count as
    "can run together" once it lands.
  - Offenders at `8a88c6267`, all archived plans: architecture-core-queue (8 pairs),
    architecture-production-residuals (1), self-dev-extras (1), self-dev-ux (217).
