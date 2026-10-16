+++
id = "bug-7a08a8"
kind = "bug"
title = "A plan written for a chat host keeps open questions its planner already answered, so it cannot run, and plan generation still reports it written"
status = "done"
triage = "verified"
severity = "p1"
goal = "hermes"
size = "M"
subsystem = ["roko-cli/plan_generate", "roko-serve/plans"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "ddf727b7c"
source = "tmp/hermes-roko-v5/findings/dogfood-desktop.md P1 (2026-10-16, roko 1630fa1bf)"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan", "crates/roko-cli/src/serve_runtime.rs::generate_plan_from_prompt", "crates/roko-serve/src/routes/plans/authoring.rs::start_plan_generation", "crates/roko-serve/src/routes/plans/run_control.rs::plan_run_rejected", "crates/roko-cli/src/commands/run_cmd.rs::generate_prompt_plan"]
links = { depends_on = [], blocks = [], related = ["bug-59ed90", "gap-1f08f4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn an_unattended_plan_decides_its_answered_questions' crates/roko-cli/src && grep -rqw 'fn generation_fails_when_the_written_plan_cannot_run' crates/roko-cli/src && grep -rqw 'fn plan_run_rejected_names_the_errors' crates/roko-serve/src && grep -rqw 'fn a_chat_host_plan_is_generated_for_a_chat_host' crates/roko-serve/src && cargo test -p roko-cli -p roko-serve --lib -- an_unattended_plan_decides_its_answered_questions generation_fails_when_the_written_plan_cannot_run plan_run_rejected_names_the_errors a_chat_host_plan_is_generated_for_a_chat_host"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T03:29:27Z"
commit = "ddf727b7c"
by = "roko-planner"
executor = "claude-agent"
via = "manual"
forced = false
evidence = "ddf727b7c: unattended plans (a chat host's /mcp request, roko run unless --dry-run) tell the planner nobody answers questions, and a post-pass folds open questions carrying their own 'Assumption:' into the task description (pipeline.rs fold_answered_questions); generate_plan returns PlanFailsValidation naming each error when the written plan fails validation, so serve's op ends failed with the errors named and roko run stops; serve routes /mcp generation through CliRuntime::generate_plan_for_chat_host; the run refusal lists the errors. Checked: the item's [[verify]] tests plus roko-cli 3694 and roko-serve 1031 lib tests, clippy -D warnings, fmt; integration failures only the known bug-2a31bc, bug-c63cca, bug-ee2752. Live (zero cost, /mcp on serve :6794, scripted planner): an unanswered question now ends the op failed naming PLAN_045, the task and the question (main's binary: completed); answered ones are folded and the op completes with a plan that validates clean. Real gpt-5.4-mini via /mcp: no open_questions, an Assumption line per task, 0 diagnostics (/bin/zsh.02). Findings: tmp/hermes-roko-v5/findings/roko-planner.md"
+++

## Problem

"Plan first" from a chat host writes a plan that cannot run, and roko reports the plan as written.

Seen in the 2026-10-16 desktop dogfood (tmp/hermes-roko-v5/findings/dogfood-desktop.md, P1; roko 1630fa1bf, Hermes
plugin 0.5.0). The desktop composer's "plan first" called `/mcp` `plan_generate` for a four-task plan:

- The planner wrote two `open_questions` on T2, each already carrying its own answer ("When n > number of unique
  words, should the list be shorter than n …? Assumption: return only as many as exist …").
- roko serve's log said "artifact validation found 1 error(s) … for add-two-small-modules-…", yet the operation
  ended `succeeded`, and the page said "roko: plan written".
- Running it was refused: "plan '…' failed validation with 1 error(s); fix them before running it". Nothing said
  which error, where, or how to fix it. The cause, PLAN_045 (open questions), showed only in a terminal's
  `roko plan validate`.

## Why it matters

A newcomer's first multi-task plan dead-ends with no explanation (goal `hermes`; it blocks chapter 02 of the guide).
Open questions exist so a person answers them before a run; a chat host's plan, or `roko run`'s, has nobody to
answer, so a question the planner already answered only blocks the run.

## Where

- `crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan`: the planner prompt, the repair pass, the
  post-write validation whose errors leave `artifact_valid = false` but still return `Ok`.
- `crates/roko-cli/src/serve_runtime.rs::generate_plan_from_prompt`: ignores the outcome, so serve's operation
  succeeds.
- `crates/roko-serve/src/routes/plans/authoring.rs::start_plan_generation`: knows whether a chat host asked
  (`RunOrigin::is_chat`), but does not pass it on.
- `crates/roko-cli/src/commands/run_cmd.rs::generate_prompt_plan`: `roko run` plans and then runs without review.
- `crates/roko-serve/src/routes/plans/run_control.rs::plan_run_rejected`: the run refusal's message.

## Plan

1. Unattended plans (a chat host's request; `roko run` unless `--dry-run`): the planner is told nobody will answer
   questions, so it decides each open choice and writes the decision into the task. A post-pass folds every
   open question that carries its own answer ("…? Assumption: …") into the task's description and removes it.
2. A generation whose written plan fails validation returns an error that names each error (rule, task, message)
   and the plan's path, so serve's operation ends `failed` with that message and `roko run` stops before running.
3. The run refusal's message lists the errors too (its `details` already carry the report).

## Done when

- An unattended plan whose questions carry assumptions is written with no `open_questions`, the assumptions in the
  task's description.
- A plan left failing validation ends generation with an error naming PLAN_045 and the task.
- `[[verify]]` passes.
