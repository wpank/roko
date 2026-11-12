+++
id = "gap-4cca5f"
kind = "gap"
title = "assign_runs counts a task's step room against the planner's tier before raise_tiers lifts it for its files, so missing checks still join an && step and the task may start a rung lower (phase 5.2's drops draft: P5.2.07-08 joined, render-demos twice, T1 focused not integrative)"
status = "done"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-26
updated = 2026-10-28
last_verified = 2026-10-28
last_verified_rev = "38bedecea"
source = "round-v7-roko-planner#phase-5-2-replay"
discovered_from = "agent:roko-planner (round v7, phase 5.2 $0 replay on roko-main-00ecbd158), the lead's ask"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs", "crates/roko-cli/src/plan_generate/sizes.rs::raise_tiers"]
links = { depends_on = [], blocks = [], related = ["gap-4dd77d", "gap-77220d", "gap-8b1f4f", "gap-de0b87"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn missing_checks_get_the_step_room_of_the_raised_tier' crates/roko-cli/src && cargo test -p roko-cli --lib missing_checks_get_the_step_room_of_the_raised_tier"

[closed]
at = 2026-10-28
at_ts = "2026-10-28T10:34:27Z"
commit = "38bedecea"
by = "roko-planner (round v7)"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-28T09:49:58Z"
forced = false
evidence = "Merged 38bedecea (branch 466d975d7, with a one-line nightly-fmt fix for the spec gate's contradiction test, a83d5d67d; main 8da34f886 merged in at 4b0ad9d18, no file in common). Will's call of 2026-10-27, keeping gap-4dd77d's raise over decision 3203's never-raise rule, is recorded in gap-de0b87 (c0bd40d64). Post-merge verify in my worktree, detached at 38bedecea: the grep finds the test, and missing_checks_get_the_step_room_of_the_raised_tier passes (1 of 1). The test runs phase 5.2's drops draft (fixture tests/fixtures/p5_2_criteria/drops.toml): T1 is raised before assign_runs, counting a step for each check it is given, from mechanical to integrative; P5.2.05-08 each get a step of their own, no step runs tools/render-demos.sh twice, 9 of 9 checks kept, one planner call. Branch suites: plan_generate and plan_policy 211/211 (with a_task_is_raised_for_the_steps_its_checks_will_take); roko-cli lib 4094 passed, 0 failed; nightly fmt; clippy -p roko-cli -D warnings clean. $0 replays of 5.2 (drops, as written, ideal), 5 (8 tasks), 4.3 and 4.2: only the drops draft changes, from focused with 3 steps (one running render-demos twice) to integrative with 5, as the spec's own draft; the others keep their tiers and steps; every check kept, plan validate --strict 0 diagnostics each."
+++

## Problem

`source_checks::assign_runs` gives a task's missing source checks steps of their own while the task has room under
its tier's step limit (gap-77220d), and joins the rest into the last one with `&&`. It reads that limit from the
planner's tier. `sizes::raise_tiers` (gap-4dd77d) then runs last and raises the tier for the task's files. By then
the checks are already joined, and fewer steps can mean a lower tier and a lower start rung.

Phase 5.2's drops draft, replayed at $0 on roko-main-00ecbd158 (Undertow ac16976,
`tmp/hermes-roko-v7/roko-planner/replay-p52/drops.toml`), shows it. The draft is one mechanical task with
presets.cpp, the 4 showcase WAVs and the P5.2.01-04 step, and P5.2.05-08 listed in `dropped_checks`:

- gap-8b1f4f refuses the four drops, and assign_runs adds P5.2.05 and P5.2.06 as steps of their own, which fills
  mechanical's 3 steps.
- P5.2.07 and P5.2.08 join step 3: `tools/spec.sh 5_1 render && tools/render-demos.sh && tools/spec.sh 5_1 demos &&
  tools/render-demos.sh && tools/spec.sh 5 demos`. It runs render-demos twice, and its first failure hides the rest.
- raise_tiers then lifts T1 only to focused, for its 5 files. With 3 steps it starts on cheap.
- The notes' own draft (`ideal.toml`) has 5 steps and is raised to integrative, so it starts on mid.

## Why it matters

For the same spec, the joined step decides both how failures read and which rung the task starts on.

## Where

- `crates/roko-cli/src/plan_generate/pipeline.rs`: the order of `assign_runs` and `sizes::raise_tiers` among the
  draft's repairs.
- `crates/roko-cli/src/plan_generate/sizes.rs::raise_tiers`.

## Plan

Unblocked 2026-10-27: Will keeps gap-4dd77d's raise, which supersedes decision 3203's rule that tiers are never
raised automatically (1d4116847; the dated note is in gap-de0b87).

- Raise a task's tier before assign_runs, as raise_tiers does last, so missing checks get steps up to the tier the
  task will have. The step raise stays last.

## Done when

Phase 5.2's drops draft, replayed, gives P5.2.05-08 steps of their own up to the raised tier's limit, and no step
runs render-demos twice. The `[[verify]]` passes.
