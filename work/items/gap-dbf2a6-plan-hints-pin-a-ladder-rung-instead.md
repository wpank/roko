+++
id = "gap-dbf2a6"
kind = "gap"
title = "Plan hints pin a ladder rung instead of a model name, and generated plans keep their hints"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/task_parser", "roko-cli/prd", "roko-cli/plan_generate"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "27deb61d8"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e5"
discovered_from = "tmp/cybernetic-harness/tldr/research/B1-plan-authoring.md (model_hint row: generated plans lose their hints)"
anchors = ["crates/roko-cli/src/task_parser.rs::TaskDefSerde", "crates/roko-cli/src/prd.rs::validate_and_fix_generated_plan", "crates/roko-cli/src/plan_generate.rs::PLAN_GENERATOR_SYSTEM_PROMPT", "crates/roko-cli/src/plan_authoring.rs::starter_plan_source", "crates/roko-cli/src/plan_validate.rs", "crates/roko-core/src/config/routing.rs"]
lane = "rust-hot"
parent = "spec-98f76d"
links = { depends_on = ["gap-9cbf35"], blocks = [], related = ["gap-0f3980", "gap-2623b2"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn generated_plan_keeps_its_rung_hint' crates/roko-cli/src/ && cargo test -p roko-cli --lib generated_plan_keeps_its_rung_hint"

[[verify]]
command = "grep -rqw 'fn rung_hint_sets_the_start_rung' crates/roko-core/src/ && cargo test -p roko-core --lib rung_hint_sets_the_start_rung"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 27deb61d8. A plan's rung hint pins the ladder start rung, and generated plans keep a rung that names a real ladder rung. Batch 16a gate on 4169ecaba, re-assembled as 22f4a8791 with only runstate's rustfmt commit (MAIN 27deb61d8 has the same code): cargo check --workspace --tests clean; nightly fmt clean; clippy -p roko-agent -p roko-cli -p roko-core --keep-going -D warnings clean; lib tests pass: roko-agent 2270, roko-cli 3204 (one background-writer wait flake, gate_rows_carry_the_attempts_turns_or_unknown, passes alone in 1.2 s), roko-core 1953. Verify: its test passes in that run and its static checks pass on MAIN."
+++

## Problem

The only per-task model lever is `model_hint`, a raw model slug. `plans/` holds 425 of them, and a slug ties a plan to
one provider and would bypass any ladder. Generated plans cannot carry a hint at all:

- the generator prompts say "NEVER set `model_hint`" because the runtime selects the model from the tier
  (`plan_generate.rs:217`, `:327`; `prd.rs:301`, `:1426`), which is not true today;
- `validate_and_fix_generated_plan` strips any hint (`prd.rs:2841`).

Meanwhile the plan scaffold, `plan_authoring.rs::starter_plan_source`, writes the default model as a hint (:249).

## Why it matters

tldr/05 P1 #8: the planner emits tier and role, and a hint pins a rung, never a model slug. A frontier planner that
knows a task is hard needs a portable way to say so, and it must survive generation. Step 4 of epic spec-98f76d.

## Where

- `crates/roko-cli/src/task_parser.rs::TaskDefSerde` and `TaskDef`: a new optional `rung` key.
- `crates/roko-core/src/config/routing.rs`: gap-9cbf35's ladder resolver takes the hint.
- `crates/roko-cli/src/prd.rs::validate_and_fix_generated_plan`, the generator prompts in `plan_generate.rs` and
  `prd.rs`, `plan_authoring.rs::starter_plan_source` and `plan_validate.rs`.

## Current state

Checked at `41c7ffbd6`: as described above. No `rung` key exists.

## Plan

1. Add `rung: Option<String>` to `TaskDefSerde` and `TaskDef`. It names a ladder rung (gap-9cbf35), for example
   `rung = "strong"`, and sets the task's start rung. Escalation (gap-460230) still moves up from there.
2. The resolver takes it: `resolve(role, tier, rung)`.
3. `plan validate`: an unknown rung is an error. With the ladder on, a slug `model_hint` is a warning ("pins a model
   and bypasses the ladder; use `rung`"). Hints keep working.
4. `validate_and_fix_generated_plan`: keep a valid `rung`, drop an unknown one with a message, keep stripping
   `model_hint`.
5. Prompts: say to set `tier` and `role`, and `rung` only when a task needs more than its tier's start rung. Update the
   test that asserts the old text (`plan_generate.rs:1047`).
6. `starter_plan_source` writes no hint.
7. Tests: `generated_plan_keeps_its_rung_hint` (`prd.rs`) and `rung_hint_sets_the_start_rung` (the resolver).

## Done when

- [ ] A task with `rung = "strong"` and no `model_hint` starts on that rung.
- [ ] A generated plan keeps `rung` and still loses `model_hint`; `plan validate` rejects an unknown rung.
- [ ] Both `[[verify]]` commands pass.

## Notes

- The 425 existing hints keep working; converting them is not part of this item.
- E8.2 (gap-2623b2) replaces the three generator prompts with one. If it lands first, make step 5's change there.
- gap-0f3980 also adds fields to `TaskDefSerde`; agree on placement if both are in flight.
- Implemented on `work/gap-0f3980` at `abab08a23`; cargo verification deferred to the batch check. Both verify tests
  pass there in a targeted run.
- The hint is `TaskHints.rung`, flattened into `TaskDef` beside gap-0f3980's hints, not a separate field.
  `LadderConfig::resolve(role, tier, rung_hint, runnable)` and `RoutingLadder::start(role, tier, rung_hint)` take it.
- `plan validate` (with a workdir): PLAN_040 error for a rung that names none of the task's ladder rungs; PLAN_041
  warning for `model_hint` or `preferred_model` on a task the ladder would route.
- `plan_generator.rs::DefaultPlanGenerator` (not `roko prd plan`) still strips `model_hint` and keeps unknown keys; it
  was not changed.
