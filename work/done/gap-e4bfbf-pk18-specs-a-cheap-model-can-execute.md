+++
id = "gap-e4bfbf"
kind = "gap"
title = "PK18 Specs a cheap model can execute: `plan validate --spec-quality --dynamic`: red-on-base proof inside Roko (+9 more)"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
rank = 18
size = "L"
subsystem = ["roko-cli/prd"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "509e3e807"
source = "tmp/backlog/2026-10-02-complete-and-wire PK18"
anchors = ["crates/roko-cli/src/commands/plan.rs", "crates/roko-cli/src/lib.rs", "crates/roko-cli/src/main.rs", "crates/roko-cli/src/plan_authoring.rs", "crates/roko-cli/src/plan_generate.rs", "crates/roko-cli/src/prd.rs", "crates/roko-cli/src/serve_runtime.rs", "crates/roko-cli/tests/prd_pipeline_workspace.rs", "crates/roko-serve/src/plan_types.rs", "roko.toml"]
lane = "rust-cold"
parent = "spec-fef7c5"
links = { depends_on = ["gap-cb5133"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn spec_quality_dynamic_flags_a_verify_green_on_base' crates/roko-cli/ && cargo test -p roko-cli spec_quality_dynamic_flags_a_verify_green_on_base"

[[verify]]
command = "grep -rqw 'fn revision_prompt_carries_failed_gate_output' crates/roko-cli/src/ && cargo test -p roko-cli --lib revision_prompt_carries_failed_gate_output"

[[verify]]
command = "grep -rqw 'fn revise_response_includes_plan_diff' crates/roko-cli/ && cargo test -p roko-cli revise_response_includes_plan_diff"

[[verify]]
command = "grep -q '^\\[authoring\\]' roko.toml && grep -qE '^planner_model *= *\"claude-opus' roko.toml && grep -q '^\\[models.claude-opus\\]' roko.toml"

[[verify]]
command = "grep -rqw 'fn generation_regenerates_once_on_a_spec_hard_fail' crates/roko-cli/ && cargo test -p roko-cli generation_regenerates_once_on_a_spec_hard_fail"

[[verify]]
command = "grep -q 'open_questions' crates/roko-cli/src/plan_generate.rs && grep -rqw 'fn generator_prompt_carries_the_tss_checklist' crates/roko-cli/src/ && cargo test -p roko-cli --lib generator_prompt_carries_the_tss_checklist"

[[verify]]
command = "! grep -rq build_backlog_generation_prompt crates/roko-cli/src/"

[[verify]]
command = "grep -rqw 'fn generation_writes_planner_accept_tests_into_the_plan' crates/roko-cli/ && cargo test -p roko-cli generation_writes_planner_accept_tests_into_the_plan"

[[verify]]
command = "grep -q 'task.accept' crates/roko-cli/src/plan_generate.rs && grep -rqw 'fn generator_prompt_teaches_task_accept' crates/roko-cli/src/ && cargo test -p roko-cli --lib generator_prompt_teaches_task_accept"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T18:37:11Z"
commit = "509e3e807"
executor = "claude-agent"
via = "work-batch"
size = "L"
claimed_at = "2026-10-02T16:02:39Z"
forced = false
evidence = "Gate 3a on work/backlog-batch-3 (merged into main as 509e3e807; main differs from the gated tree only in work/ files): cargo check --workspace --tests, clippy -D warnings, nextest --lib 11,514 passed over 10 crates, golden-path canaries 13/13 (7 targets), roko-agent sse_replay 1/1; every [[verify]] passes (lib tests named in each verify passed; integration tests run by target; static parts rc=0)."
+++

## Problem

This package delivers 10 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK18, slice 32xx, phase 3), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3214 | M | p2 | `plan validate --spec-quality --dynamic`: red-on-base proof inside Roko | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3214-plan-validate-dynamic-red-on-base-in-roko.md` |
| 2 | 3215 | S | p3 | Plan revision reads the failed attempts' gate output, not a one-line summary | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3215-revise-reads-failed-attempts-gate-output.md` |
| 3 | 3216 | M | p3 | Revise returns a task-level plan diff | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3216-revise-returns-a-task-level-plan-diff.md` |
| 4 | 3217 | S | p2 | Configure the frontier planner in this workspace | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3217-configure-frontier-planner-in-this-workspace.md` |
| 5 | 3218 | M | p2 | Generation scores its plan and regenerates once on a hard fail or a low score | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3218-generation-scores-plan-and-regenerates-once.md` |
| 6 | 3219 | S | p2 | The generator prompt carries the TSS v1 checklist and open questions | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3219-generator-prompt-carries-tss-checklist.md` |
| 7 | 3220 | S | p3 | `plan generate --from-backlog` goes through the one generation pipeline | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3220-from-backlog-uses-the-one-generation-pipeline.md` |
| 8 | 3221 | M | p2 | The generator can write planner-written acceptance tests into the plan | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3221-generator-writes-planner-accept-tests.md` |
| 9 | 3222 | S | p2 | The generator prompt teaches [task.accept] | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3222-generator-prompt-teaches-task-accept.md` |
| 10 | 3223 | S | p2 | Live check: plans generated from five sample PRDs score at least 70 | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3223-live-check-five-prd-plans-score-70.md` |

## Why it matters

Phase 3: golden-path proof. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3200-specs-a-cheap-model-can-execute.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/commands/plan.rs`, `crates/roko-cli/src/lib.rs`, `crates/roko-cli/src/main.rs`, `crates/roko-cli/src/plan_authoring.rs`, `crates/roko-cli/src/plan_generate.rs`, `crates/roko-cli/src/prd.rs`, `crates/roko-cli/src/prd/accept_blocks.rs`, `crates/roko-cli/src/serve_runtime.rs`, `crates/roko-cli/src/spec_red_on_base.rs`, `crates/roko-cli/tests/fixtures/spec-gen-sample/`, `crates/roko-cli/tests/prd_pipeline_workspace.rs`, `crates/roko-serve/src/plan_types.rs`, `roko.toml`.

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

- Waits on: PK17 (gap-cb5133).
- Suggested model: opus.

## Progress

Worker claude-agent on `work/gap-e4bfbf` from `79ba9911f`, 2026-10-02. Rust tasks are implemented, not done: cargo
verification is deferred to the batch gate. Python checks run: speclint's tests (98 passed, `rust_parity.py` included).

- 3214: implemented at e6558f666 (`spec_red_on_base.rs`, `plan validate --spec-quality --dynamic [--base --timeout --scratch --fixture]`, `runs_program` in roko-gate's shell parser, `rust_parity.py` compares `fixtures/dynamic/red-on-base`)
- 3215: implemented at 053a96707 (`last_run_failure_context`, budget a quarter of the planner window, 8,000-character floor)
- 3216: implemented at 943cce681 (`plan_diff`, `RevisionOutcome.diff`, `RevisionDto.diff`)
- 3217: implemented at 08688a721 (verify passes; `roko config validate` 0 errors with the wave-1 batch binary; the `prd plan --dry-run` planner check not run, as it calls the planner)
- 3218: implemented at 9432ffcbb (spec hard fails retried, one regeneration for a weak plan, `GenerationOutcome.spec_quality`)
- 3219: implemented at 501d55fa1 (TSS v1 fields, open questions, checklist; example checks made red on the base)
- 3220: implemented at 894833848 (verify passes: the two backlog prompt builders are gone)
- 3221: implemented at e765a734c (`prd/accept_blocks.rs`)
- 3222: implemented at 1785ec795
- 3223: blocked: needs a roko built with 3217–3222 (the batch gate) and live planner runs on the Claude CLI subscription; a static worker runs no cargo, and the fixtures must not be hand-written

- 2026-10-02 (coordinator, gate 3): task 3223's live check moved to its own item (its verify left this one): it needs a roko built with 3217–3222 plus live planner calls, and session roko-7d is replacing PRD-first generation with plan-first generation at Will's request, so its five sample inputs change too.
