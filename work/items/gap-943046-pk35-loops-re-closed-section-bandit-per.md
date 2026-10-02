+++
id = "gap-943046"
kind = "gap"
title = "PK35 Loops re-closed: Section bandit: per-section posteriors and exclusion probabilities (+7 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 35
size = "L"
hold = "workflow-audit migration in progress (session roko-7d, at Will's request, 2026-10-02): it removes the PRD pipeline and folds roko do/develop into roko run; check with roko-7d before starting work that edits PRD code, do_cmd.rs or the Run/Do/Prd parts of main.rs"
subsystem = ["roko-learn/error-patterns"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK35"
anchors = ["crates/roko-cli/src/commands/learn.rs", "crates/roko-cli/src/commands/plan.rs", "crates/roko-cli/src/graph_task_dispatch/attempt.rs", "crates/roko-cli/src/graph_task_dispatch/streaming.rs", "crates/roko-cli/src/prd.rs", "crates/roko-cli/src/runtime_feedback/error_patterns.rs", "crates/roko-core/src/config/mod.rs", "crates/roko-core/src/config/schema.rs", "crates/roko-learn/src/error_pattern_store.rs", "crates/roko-learn/src/feedback_service.rs", "crates/roko-learn/src/section_effect.rs"]
lane = "rust-hot"
parent = "spec-446a41"
links = { depends_on = ["gap-cc5051", "gap-b5caf3", "gap-aea13a", "gap-ac2611"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn section_bandit_excludes_with_logged_propensity_and_updates_posterior' crates/roko-learn/src/ && cargo test -p roko-learn section_bandit_excludes_with_logged_propensity_and_updates_posterior"

[[verify]]
command = "grep -rqw 'fn verified_retry_records_resolution_on_its_failure_pattern' crates/roko-cli/src/ && cargo test -p roko-cli verified_retry_records_resolution_on_its_failure_pattern"

[[verify]]
command = "grep -rqw 'fn planner_prompt_lists_resolved_patterns_for_touched_crates' crates/roko-cli/src/ && cargo test -p roko-cli planner_prompt_lists_resolved_patterns_for_touched_crates"

[[verify]]
command = "grep -rqw 'fn graduation_candidates_need_recurrence_and_a_resolution' crates/roko-learn/src/ && cargo test -p roko-learn graduation_candidates_need_recurrence_and_a_resolution"

[[verify]]
command = "grep -rqw 'fn arm_set_draws_once_per_chain_and_maximize_draws_nothing' crates/roko-learn/src/ && cargo test -p roko-learn arm_set_draws_once_per_chain_and_maximize_draws_nothing"

[[verify]]
command = "grep -rqw 'fn arm_set_is_chain_stable_and_logged' crates/roko-cli/src/ && cargo test -p roko-cli arm_set_is_chain_stable_and_logged"

[[verify]]
command = "grep -rqw 'fn no_holdout_flag_sets_maximize_mode' crates/roko-cli/src/ && cargo test -p roko-cli no_holdout_flag_sets_maximize_mode"

[[verify]]
command = "grep -rqw 'fn placebo_decisions_have_identical_proposals' crates/roko-cli/src/ && cargo test -p roko-cli placebo_decisions_have_identical_proposals"
+++

## Problem

This package delivers 8 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK35, slice 41xx, phase 4), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 4122 | M | p2 | Section bandit: per-section posteriors and exclusion probabilities | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4122-section-bandit-posteriors-and-exclusion-odds.md` |
| 2 | 4125 | M | p2 | A verified pass after a failure records the fix on that failure's error pattern | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4125-verified-pass-records-fix-on-error-pattern.md` |
| 3 | 4126 | M | p3 | The planner sees recurring failure patterns and their fixes for the crates a plan touches | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4126-planner-sees-recurring-failure-patterns.md` |
| 4 | 4128 | S | p3 | `roko learn` lists failure patterns ready to graduate into a lint or verify step | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4128-learn-lists-failure-patterns-ready-to-graduate.md` |
| 5 | 4116 | M | p2 | ArmSet: draw each chain's arms once over the loop-audit layers | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4116-armset-draws-each-chains-arms-once.md` |
| 6 | 4117 | M | p2 | Graph attempts open with their chain's ArmSet, and retries inherit it | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4117-graph-attempts-open-with-chain-armset.md` |
| 7 | 4118 | S | p3 | `roko plan run --no-holdout` runs with randomisation off | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4118-plan-run-no-holdout-flag-sets-maximize.md` |
| 8 | 4119 | S | p3 | Every attempt logs an L-placebo decision with identical arms | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4119-every-attempt-logs-a-placebo-decision.md` |

## Why it matters

Phase 4: loops re-closed (S02). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4100-loops-reclosed-on-verified-outcomes.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/commands/learn.rs`, `crates/roko-cli/src/commands/plan.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, `crates/roko-cli/src/graph_task_dispatch/attempt.rs`, `crates/roko-cli/src/graph_task_dispatch/streaming.rs`, `crates/roko-cli/src/prd.rs`, `crates/roko-cli/src/runtime_feedback/error_patterns.rs`, `crates/roko-core/src/config/experiments.rs`, `crates/roko-core/src/config/mod.rs`, `crates/roko-core/src/config/schema.rs`, `crates/roko-learn/src/error_pattern_store.rs`, `crates/roko-learn/src/feedback_service.rs`, `crates/roko-learn/src/loop_audit/arm_set.rs`, `crates/roko-learn/src/loop_audit/mod.rs`, `crates/roko-learn/src/section_effect.rs`.

It also edits the hot file(s) `crates/roko-cli/src/graph_task_dispatch.rs`, which are left out of this item's anchors so that two hot packages can run at once; the coordinator resolves any merge conflict.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: PK09 (gap-cc5051), PK32 (gap-b5caf3), PK33 (gap-aea13a), PK34 (gap-ac2611).
- Suggested model: opus.
