+++
id = "gap-1d1fa6"
kind = "gap"
title = "Task size limits per executor tier in plan validate"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan_policy"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-30
last_verified_rev = "8a88c6267"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e8"
discovered_from = "tmp/cybernetic-harness/tldr/04-FRONTIER-PLANS-CHEAP-EXECUTES.md (step 3); tldr/research/B1-plan-authoring.md (tier LOC budgets)"
anchors = ["crates/roko-cli/src/plan_policy.rs::PlanExecutionPolicy", "crates/roko-cli/src/plan_generate.rs::TaskTier"]
lane = "rust-cold"
parent = "spec-e57870"
links = { depends_on = ["gap-8c0a20"], blocks = [], related = ["gap-9cbf35"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn task_over_its_tier_limits_is_flagged' crates/roko-cli/src/ && cargo test -p roko-cli --lib task_over_its_tier_limits_is_flagged"
+++

## Problem

Nothing checks that a task is small enough for its tier:

- `plan_generate::TaskTier::max_loc` sets budgets of 20 (mechanical, "Haiku-capable"), 50 (focused), 150
  (integrative) and 300 (architectural) lines, but only a test reads them.
- `TaskDef.max_loc` has no reader on the Graph path.
- `PlanExecutionPolicy` limits files, read files and verify steps per task, with the same limits for every tier.

## Why it matters

- tldr/04 step 3 ("Size and split") is MISSING.
- Design rule 1: size tasks for the executor. Success falls with task length; models' 80%-success horizons are 4–6×
  shorter than their 50% horizons (`kwa2025measuring`).
- Once the ladder routes mechanical tasks to cheap models (epic E5), an oversized mechanical task is a predictable
  failure and an escalation.

tldr/05 decision 2 (default: size tasks by the executor tier's measured pass rate). Part of epic spec-e57870.

## Where

- `crates/roko-cli/src/plan_policy.rs::PlanExecutionPolicy`: add a table of limits per tier.
- `crates/roko-cli/src/plan_generate.rs::TaskTier`: today's LOC budgets. The shared tier enum from gap-8c0a20 (E5.1)
  replaces it as the key.

## Current state

Checked at `41c7ffbd6`: no tier-aware rule in `plan_policy.rs` or `plan_validate.rs`. Most tier strings route as
"Standard" (research note B1); gap-8c0a20 fixes the vocabulary.

## Plan

1. Limits per tier: `files`, declared `max_loc` (at most the tier's budget), description words and verify steps.
   Defaults come from `TaskTier::max_loc` and today's normal policy.
2. `PLAN_TIER_SIZE`: a task over its tier's limits is a warning, and an error under `--strict`, with a hint to split
   the task or raise its tier.
3. Give the generator the limits: one line in the system prompt.

## Done when

- [ ] A `mechanical` task declaring 6 files and `max_loc = 200` is flagged; the same task as `integrative` is not.
- [ ] The `[[verify]]` command passes.

## Notes

- Waits for gap-8c0a20, the shared tier enum. Do not key the limits on raw tier strings.
- Keep the check static. Measuring each attempt's actual diff size belongs with the diff check (epic E9).
- Refitting the defaults from per-tier pass rates needs the attempt records (epic E4) and is later work.
- 2026-09-30 (wk-tiers): implemented on `work/gap-1d1fa6` at `afd17b79e`; cargo verification deferred to the batch check (no cargo was allowed for this item).
- The Done-when example conflicts with plan step 1: its `max_loc = 200` is over integrative's 150-line budget, so the task is flagged as integrative too (it fits architectural). `task_over_its_tier_limits_is_flagged` uses `max_loc = 150`, which only a lower tier flags.
- Default limits (files, description words, verify steps): mechanical 3/300/3, focused 5/350/4, integrative 10/500/6, architectural 32/800/8; the line budgets are 20/50/150/300. On 2026-09-30 they flag 107 of the 551 tasks in `plans/` (56 of 132 plans), 73 of them for a declared `max_loc` over the tier's budget, so `plan validate --strict` now fails those plans.
