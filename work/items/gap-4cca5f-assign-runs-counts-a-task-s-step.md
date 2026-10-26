+++
id = "gap-4cca5f"
kind = "gap"
title = "assign_runs counts a task's step room against the planner's tier before raise_tiers lifts it for its files, so missing checks still join an && step and the task may start a rung lower (phase 5.2's drops draft: P5.2.07-08 joined, render-demos twice, T1 focused not integrative); waits on Will's call on decision 3203"
status = "blocked"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7-roko-planner#phase-5-2-replay"
discovered_from = "agent:roko-planner (round v7, phase 5.2 $0 replay on roko-main-00ecbd158), the lead's ask"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs", "crates/roko-cli/src/plan_generate/sizes.rs::raise_tiers"]
links = { depends_on = [], blocks = [], related = ["gap-4dd77d", "gap-77220d", "gap-8b1f4f"], supersedes = [], duplicate_of = "" }
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

**Blocked: waits on Will's call on decision 3203** ("tiers are never raised automatically", 1d4116847,
2026-10-03), which gap-4dd77d reversed. The lead has put it to Will.

- If Will keeps gap-4dd77d: raise a task's tier for its files, LOC and description before assign_runs, so missing
  checks get steps up to the tier the task will have. The step raise stays last.
- If he reverts gap-4dd77d, this item is moot.

## Done when

Phase 5.2's drops draft, replayed, gives P5.2.05-08 steps of their own up to the raised tier's limit, and no step
runs render-demos twice.
