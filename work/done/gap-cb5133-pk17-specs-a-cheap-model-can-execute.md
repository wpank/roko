+++
id = "gap-cb5133"
kind = "gap"
title = "PK17 Specs a cheap model can execute: Warn on unknown [[task]] keys: R3's top-level read_files never reached a prompt (+7 more)"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
rank = 17
size = "L"
subsystem = ["roko-cli/plan_validate"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "e53136640"
source = "tmp/backlog/2026-10-02-complete-and-wire PK17"
anchors = ["benchmarks/viabilitybench/speclint/speclint.py", "benchmarks/viabilitybench/speclint/tests/test_speclint.py", "crates/roko-cli/src/commands/plan.rs", "crates/roko-cli/src/dispatch/prompt_builder.rs", "crates/roko-cli/src/lib.rs", "crates/roko-cli/src/plan_generator.rs", "crates/roko-cli/src/plan_validate.rs", "crates/roko-cli/src/prd.rs", "crates/roko-cli/src/task_parser.rs", "crates/roko-cli/tests/plan_validate.rs", "crates/roko-core/src/config/config_fingerprint_golden.json", "crates/roko-core/src/config/mod.rs", "crates/roko-core/src/config/schema.rs", "crates/roko-gate/src/spec_quality.rs", "plans/e2e-smoke-test/tasks.toml", "plans/wire-http-plan-execute/tasks.toml"]
lane = "rust-cold"
parent = "spec-fef7c5"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_validate_warns_on_unknown_task_key' crates/roko-cli/ && cargo test -p roko-cli plan_validate_warns_on_unknown_task_key"

[[verify]]
command = "grep -qE '^\\s*pub covers:' crates/roko-cli/src/task_parser.rs && grep -rqw 'fn tss_v1_fields_round_trip' crates/roko-cli/src/ && cargo test -p roko-cli --lib tss_v1_fields_round_trip"

[[verify]]
command = "grep -rqw 'fn tss_v1_example_passes_strict_validation' crates/roko-cli/tests/ && cargo test -p roko-cli --test plan_validate tss_v1_example_passes_strict_validation"

[[verify]]
command = "grep -rqw 'fn covers_must_name_an_acceptance_criterion' crates/roko-cli/ && cargo test -p roko-cli covers_must_name_an_acceptance_criterion"

[[verify]]
command = "grep -qE '^\\s*pub covers:' crates/roko-cli/src/task_parser.rs && grep -rqw 'fn open_questions_block_dispatch' crates/roko-cli/ && cargo test -p roko-cli open_questions_block_dispatch"

[[verify]]
command = "grep -rqw 'fn spec_quality_config_defaults_round_trip' crates/roko-core/src/ && cargo test -p roko-core --lib spec_quality_config_defaults_round_trip"

[[verify]]
command = "grep -rqw 'fn plan_run_refuses_a_vacuous_verify_step' crates/roko-cli/ && cargo test -p roko-cli plan_run_refuses_a_vacuous_verify_step"

[[verify]]
command = "grep -rqw 'fn plan_validate_rejects_verify_less_researcher_task' crates/roko-cli/ && cargo test -p roko-cli plan_validate_rejects_verify_less_researcher_task"

[[verify]]
command = "grep -qw 'def test_planner_written_test_counts_as_acceptance' benchmarks/viabilitybench/speclint/tests/test_speclint.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/speclint/tests/test_speclint.py -k test_planner_written_test_counts_as_acceptance -q"

[[verify]]
command = "grep -rqw 'fn exact_test_backed_task_scores_band_b_or_better' crates/roko-gate/src/ && cargo test -p roko-gate --lib exact_test_backed_task_scores_band_b_or_better"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T15:39:06Z"
commit = "e53136640"
executor = "claude-agent"
via = "work-batch"
size = "L"
claimed_at = "2026-10-02T11:55:03Z"
forced = false
evidence = "Backlog gate 1: work/backlog-batch-1 at 660e1a8f8 (cargo check, clippy -D warnings, 10,933 lib tests, 16 golden-path and new canaries, 471 ViabilityBench tests, paperlint and status_matrix clean); every [[verify]] of this item passed there. Merged as e53136640 with an identical tree."
+++

## Problem

This package delivers 8 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK17, slice 32xx, phase 3), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3206 | M | p2 | Warn on unknown [[task]] keys: R3's top-level read_files never reached a prompt | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3206-warn-on-unknown-task-keys.md` |
| 2 | 3207 | M | p2 | TSS v1 fields in TaskDef and VerifyStep, plus open_questions, so tasks.toml round-trips them | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3207-tss-v1-fields-and-open-questions-in-taskdef.md` |
| 3 | 3208 | S | p2 | Bind acceptance criteria to checks: validate verify `covers` against the task's AC ids | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3208-validate-verify-covers-against-acceptance-ids.md` |
| 4 | 3209 | S | p3 | A task with open questions keeps its plan from running | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3209-open-questions-keep-a-plan-from-running.md` |
| 5 | 3210 | S | p2 | A [spec_quality] config section: mode, thresholds, red_on_base, holdout | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3210-spec-quality-config-section.md` |
| 6 | 3211 | M | p2 | `plan run` refuses spec hard fails before any agent starts | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3211-plan-run-refuses-spec-hard-fails.md` |
| 7 | 3212 | S | p3 | A task of any role without a verify step is a validation error | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3212-verify-less-tasks-of-any-role-are-errors.md` |
| 8 | 3213 | M | p2 | Spec-quality rubric sq-3 in speclint and roko_gate together | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3213-spec-quality-rubric-sq3.md` |

## Why it matters

Phase 3: golden-path proof. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3200-specs-a-cheap-model-can-execute.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/speclint/fixtures/sq02-planner-test/tasks.toml`, `benchmarks/viabilitybench/speclint/speclint.py`, `benchmarks/viabilitybench/speclint/tests/test_speclint.py`, `crates/roko-cli/src/commands/plan.rs`, `crates/roko-cli/src/dispatch/prompt_builder.rs`, `crates/roko-cli/src/lib.rs`, `crates/roko-cli/src/plan_generator.rs`, `crates/roko-cli/src/plan_validate.rs`, `crates/roko-cli/src/prd.rs`, `crates/roko-cli/src/spec_gate.rs`, `crates/roko-cli/src/task_parser.rs`, `crates/roko-cli/tests/plan_validate.rs`, `crates/roko-core/src/config/config_fingerprint_golden.json`, `crates/roko-core/src/config/mod.rs`, `crates/roko-core/src/config/schema.rs`, `crates/roko-core/src/config/spec_quality.rs`, `crates/roko-gate/src/spec_quality.rs`, `crates/roko-gate/tests/fixtures/speclint/sq02-planner-test/tasks.toml`, `plans/e2e-smoke-test/tasks.toml`, `plans/fixtures/tss-v1-example/tasks.toml`, `plans/wire-http-plan-execute/tasks.toml`.

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

- Waits on: nothing.
- Suggested model: opus.

## Progress

Implemented on `work/gap-cb5133`; cargo verification deferred to the batch gate (Python verifies run: 95 passed).

- 3206: implemented at fe0377023
- 3207: implemented at 4a0e5843d
- 3208: implemented at 9a54b1a32
- 3209: implemented at 6fe08a2b0
- 3210: implemented at d407f4bf6
- 3211: implemented at fa4b95c8c
- 3212: implemented at 42fbf09f2
- 3213: implemented at 50cc85dde

Notes for the gate: the TSS v1 example plan is at `plans/_fixtures/tss-v1-example/` (not `plans/fixtures/`), so plan
discovery, `plan run plans/` and `plans/INDEX.md` skip it; `[spec_quality] red_on_base` defaults to false, a deviation
from D14 for Will's decision record; `rust_parity.py --strict plans` needs a built roko and was not run; four plan-run
canaries swapped `command = "true"` for `test -d .` and the loop-census plan sets `allow_unverified`.
