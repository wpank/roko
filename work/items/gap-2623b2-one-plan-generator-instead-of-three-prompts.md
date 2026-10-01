+++
id = "gap-2623b2"
kind = "gap"
title = "One plan generator instead of three prompts, without the 8,000-character PRD and 5-file caps"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/prd", "roko-cli/plan_generate"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "286c5e53a"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e8"
discovered_from = "tmp/cybernetic-harness/tldr/research/B1-plan-authoring.md (duplicate generators; planner input budget)"
anchors = ["crates/roko-cli/src/prd.rs::generate_plan_from_prd_with_outcome", "crates/roko-cli/src/plan_generate.rs::build_generation_prompt", "crates/roko-cli/src/commands/plan.rs::cmd_plan", "crates/roko-cli/src/commands/do_cmd.rs::run_standard_path_inner"]
lane = "rust-cold"
parent = "spec-e57870"
links = { depends_on = ["gap-853b31"], blocks = [], related = ["find-84bfa8", "bug-8b1bf8", "bug-a5cd6b", "find-70edcb"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn generator_prompt_keeps_a_long_prd_whole' crates/roko-cli/src/ && cargo test -p roko-cli --lib generator_prompt_keeps_a_long_prd_whole && ! grep -rq 'build_generation_prompt' crates/roko-cli/src/commands/"
+++

## Problem

Plans come from three prompts, each with its own post-processing (research note B1):

- `prd::generate_plan_from_prd_with_outcome` serves `roko prd plan`, `roko do`'s complex band and serve. It is the
  strict path: `validate_and_fix_generated_plan`, `PlanExecutionPolicy::generated`, retries and model escalation.
- `roko plan generate` (`commands/plan.rs::cmd_plan`, four call sites) and `roko do`'s standard band (`do_cmd.rs:514`
  and `:872`) call `plan_generate::build_generation_prompt` directly, with their own checks. `roko do`'s prompt asks
  for `model_hint` and "the cheapest model tier", which the shared system prompt forbids.
- `plan_generator.rs::DefaultPlanGenerator` duplicates the post-processing and has no production caller
  (find-84bfa8).

The strict path also starves a frontier planner. It cuts the PRD at 8,000 characters (`prd.rs:1398`, "to keep prompt
size manageable for smaller models"), and the planner "may read up to 5 codebase files" (`prd.rs:1413`).

## Why it matters

Planning fixes (the planner model, spec rules, tier hints) land in one path and miss the others. A frontier planner
with a large context window should see the whole PRD. tldr/05 P1 #9, and decision 11 (default: PRDs stay optional
input, with one generator). Part of epic spec-e57870.

## Where

- `crates/roko-cli/src/prd.rs::generate_plan_from_prd_with_outcome`: the pipeline to keep and generalise.
- `plan_generate.rs::build_generation_prompt`, and its direct callers `commands/plan.rs::cmd_plan` and
  `commands/do_cmd.rs::run_standard_path_inner`.

## Current state

Checked at `41c7ffbd6`: all three paths exist as described. `regenerate_old_format_plan` (`prd.rs:296`) is a fourth
caller of `build_generation_prompt`, and bug-a5cd6b reports that `roko prd plan` runs it by default.

## Plan

1. Generalise the strict pipeline to take a source (a PRD file, prompt text or an old plan) and an output
   directory. Keep its validation, policy and retries.
2. Route `roko plan generate`, `roko do`'s standard band and `regenerate_old_format_plan` through it, and delete their
   direct prompt building. Delete `DefaultPlanGenerator`, or make it a thin wrapper (find-84bfa8).
3. Replace the fixed caps with budgets derived from the planner model's `context_window` (`[models.*]`): the whole
   PRD when it fits in, say, a quarter of the window, and a file-read budget scaled the same way. Keep today's
   budgets for small-context planners.
4. Drop the `model_hint` and "cheapest model tier" wording from `roko do`'s prompt. Tier hints belong to epic E5
   (gap-dbf2a6).

## Done when

- [ ] `commands/` no longer calls `build_generation_prompt`, and every generate path runs the one pipeline.
- [ ] A 20,000-character PRD reaches a large-context planner untruncated (test
      `generator_prompt_keeps_a_long_prd_whole`).
- [ ] The `[[verify]]` command passes.

## Notes

- Waits for gap-853b31 (the planner model). Keep `roko plan generate`'s output location (`.roko/plans/`).
- S07.10 (the TSS checklist) will later edit the same prompt in `prd.rs`.
- Implemented on `work/gap-2623b2`; cargo verification deferred to the batch check (its verify passes there).
- `prd::generate_plan(PlanRequest)` is the one pipeline: the strict path's validation, repair, policy, retries and
  model escalation for a `PlanSource`: a PRD, text, or a plan regenerated in place. It now serves `roko prd plan`,
  serve and `roko do`'s complex band (their wrappers stay), `roko plan generate` (prompt, file and `--from-notes`; the
  output stays `.roko/plans/`), `roko do`'s standard band and its fallback, `roko plan regenerate`, and the old-format
  refresh. `build_generation_prompt` is deleted.
- Budgets: a quarter of the planner's `[models.*] context_window` for the source (about 4 chars a token) and one file
  per 2,000 tokens of another quarter, never below the old 8,000 chars and 5 files.
- Behaviour changes: each `plan generate` or `roko do` call makes one plan (slug from the file stem or the prompt); the
  planner is read-only and roko writes the validated plan; `plan regenerate` writes only a validated plan, so it has
  nothing to restore, and no longer runs `validate_modern_fields` afterwards (that check wanted a `model_hint`, which
  generated plans never carry, until bug-a5cd6b); planning env comes from `[agent] env`, not the legacy gateway lines.
- Not done: `DefaultPlanGenerator` is untouched (find-84bfa8; gap-9ca898 edits it in batch 20), and
  `plan generate --from-backlog` still builds its own prompt (`build_backlog_generation_prompt`).
