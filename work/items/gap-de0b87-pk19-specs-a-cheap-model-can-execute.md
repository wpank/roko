+++
id = "gap-de0b87"
kind = "gap"
title = "PK19 Specs a cheap model can execute: The generator prompt sizes tasks for their executor tier (+9 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
rank = 19
size = "L"
subsystem = ["roko-cli/plan_policy"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK19"
anchors = ["apps/portal/src/components/stage/PlanView.tsx", "crates/roko-cli/src/commands/learn.rs", "crates/roko-cli/src/commands/mod.rs", "crates/roko-cli/src/commands/plan.rs", "crates/roko-cli/src/main.rs", "crates/roko-cli/src/plan_generate.rs", "crates/roko-cli/src/plan_policy.rs", "crates/roko-cli/src/plan_validate.rs", "crates/roko-cli/src/plan_generate/pipeline.rs", "crates/roko-cli/tests/plan_validate.rs", "crates/roko-gate/src/acceptance_contract.rs", "crates/roko-gate/src/lib.rs"]
lane = "rust-hot"
parent = "spec-fef7c5"
links = { depends_on = ["gap-f61823", "gap-cb5133", "gap-e4bfbf"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'Prefer the fewest cohesive tasks' crates/roko-cli/src/plan_generate.rs && grep -rqw 'fn generator_prompt_sizes_tasks_by_tier' crates/roko-cli/src/ && cargo test -p roko-cli --lib generator_prompt_sizes_tasks_by_tier"

[[verify]]
command = "grep -rqw 'fn generation_rejects_a_task_over_its_tier_size' crates/roko-cli/src/ && cargo test -p roko-cli --lib generation_rejects_a_task_over_its_tier_size"

[[verify]]
command = "grep -rqw 'fn learn_sizing_reports_pass_rate_by_tier_and_size' crates/roko-cli/ && cargo test -p roko-cli learn_sizing_reports_pass_rate_by_tier_and_size"

[[verify]]
command = "grep -q 'roko learn sizing' crates/roko-cli/src/plan_policy.rs && grep -rqw 'fn tier_size_limits_match_the_recorded_sizing_report' crates/roko-cli/src/ && cargo test -p roko-cli --lib tier_size_limits_match_the_recorded_sizing_report"

[[verify]]
command = "grep -rqw 'fn plan_revise_cli_prints_the_plan_diff' crates/roko-cli/ && cargo test -p roko-cli plan_revise_cli_prints_the_plan_diff"

[[verify]]
command = "test -f apps/portal/src/components/stage/revisionDiff.accept.test.tsx && cd apps/portal && npx vitest run src/components/stage/revisionDiff.accept.test.tsx"

[[verify]]
command = "! grep -rqw 'fn validate_evidence' crates/roko-gate/src/ && grep -rqw 'fn acceptance_contract_is_reported_as_not_enforced' crates/roko-cli/ && cargo test -p roko-cli acceptance_contract_is_reported_as_not_enforced"

[[verify]]
command = "grep -rqw 'fn plan_run_records_spec_quality_and_gate_per_task' crates/roko-cli/ && cargo test -p roko-cli plan_run_records_spec_quality_and_gate_per_task"

[[verify]]
command = "grep -rqw 'fn spec_gate_holdout_tasks_skip_score_blocks' crates/roko-cli/ && cargo test -p roko-cli spec_gate_holdout_tasks_skip_score_blocks"

[[verify]]
command = "grep -qw 'def test_degrade_is_idempotent' benchmarks/viabilitybench/specops/tests/test_degrade.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/specops/tests/test_degrade.py -q"
+++

## Problem

This package delivers 10 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK19, slice 32xx, phase 3), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3224 | S | p2 | The generator prompt sizes tasks for their executor tier | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3224-generator-prompt-sizes-tasks-by-tier.md` |
| 2 | 3225 | S | p2 | Generation rejects a task over its tier's size limits | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3225-generation-rejects-task-over-tier-size.md` |
| 3 | 3226 | M | p2 | `roko learn sizing`: verified pass rate by tier and task size | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3226-learn-sizing-pass-rate-by-tier-and-size.md` |
| 4 | 3227 | S | p3 | Set the tier size limits from the sizing report | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3227-set-tier-size-limits-from-measurements.md` |
| 5 | 3228 | M | p3 | `roko plan revise` on the CLI, and revise and regenerate print the plan diff | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3228-plan-revise-cli-and-printed-plan-diffs.md` |
| 6 | 3229 | S | p3 | The portal shows the plan diff after a revision | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3229-portal-shows-plan-diff-after-revision.md` |
| 7 | 3230 | S | p3 | Retire the AcceptanceContract evaluator | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3230-retire-acceptance-contract-evaluator.md` |
| 8 | 3231 | M | p2 | Plan-load spec gate: spec.quality and spec.gate records before dispatch | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3231-plan-load-spec-gate-records-before-dispatch.md` |
| 9 | 3232 | S | p3 | A 5% gate-off holdout for score-based spec-gate decisions | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3232-spec-gate-holdout-for-score-decisions.md` |
| 10 | 3233 | S | p2 | Spec-degradation operator D-v1 with its manifest | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3233-spec-degradation-operator-d-v1.md` |

## Why it matters

Phase 3: golden-path proof. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3200-specs-a-cheap-model-can-execute.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `apps/portal/src/components/stage/PlanView.tsx`, `apps/portal/src/components/stage/RevisionDiff.tsx`, `apps/portal/src/components/stage/revisionDiff.accept.test.tsx`, `benchmarks/viabilitybench/specops/__init__.py`, `benchmarks/viabilitybench/specops/degrade.py`, `benchmarks/viabilitybench/specops/fixtures/`, `benchmarks/viabilitybench/specops/manifest.py`, `benchmarks/viabilitybench/specops/tests/test_degrade.py`, `crates/roko-cli/src/commands/learn.rs`, `crates/roko-cli/src/commands/learn_sizing.rs`, `crates/roko-cli/src/commands/mod.rs`, `crates/roko-cli/src/commands/plan.rs`, `crates/roko-cli/src/graph_execution/plan_runner.rs`, `crates/roko-cli/src/main.rs`, `crates/roko-cli/src/plan_generate.rs`, `crates/roko-cli/src/plan_policy.rs`, `crates/roko-cli/src/plan_validate.rs`, `crates/roko-cli/src/prd.rs`, `crates/roko-cli/src/spec_gate.rs`, `crates/roko-cli/tests/plan_spec_gate.rs`, `crates/roko-cli/tests/plan_validate.rs`, `crates/roko-gate/src/acceptance_contract.rs`, `crates/roko-gate/src/lib.rs`.

It also edits the hot file(s) `crates/roko-cli/src/graph_execution/plan_runner.rs`, which are left out of this item's anchors so that two hot packages can run at once; the coordinator resolves any merge conflict.

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

- Waits on: PK10 (gap-f61823), PK17 (gap-cb5133), PK18 (gap-e4bfbf).
- Suggested model: opus.
- 2026-10-02 (roko-7d): the workflow-audit migration (merge bfd36512f) removed the PRD pipeline, `roko do` and `roko develop`; `roko run` is the one entry point and plans come from a prompt. The generator prompt and its retries moved from `prd.rs` to `crates/roko-cli/src/plan_generate/pipeline.rs` (anchor re-pointed); a task that names `prd.rs` means that file. Hold lifted.
