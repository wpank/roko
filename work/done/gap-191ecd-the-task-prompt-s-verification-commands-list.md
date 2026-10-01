+++
id = "gap-191ecd"
kind = "gap"
title = "The task prompt's Verification Commands list only the task's own steps, not the workspace rungs that will also run"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/dispatch/prompt_builder"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "9c0b9aed0"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-gates's report, checked on work/bug-5b43a9 at 7b25c478b)"
anchors = ["crates/roko-cli/src/dispatch/prompt_builder.rs", "crates/roko-cli/src/task_parser.rs::TaskDef"]
lane = "rust-hot"
parent = "spec-e9d7ec"
links = { depends_on = ["bug-5b43a9"], blocks = [], related = ["gap-3506f1", "bug-5b43a9", "bug-b4c565"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn task_prompts_list_the_workspace_rungs' crates/roko-cli/src/ && cargo test -p roko-cli --lib task_prompts_list_the_workspace_rungs"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in d965aeceb. Task prompts list the workspace rungs after the task's own steps. Batch 15a gate on 9005da604, re-assembled as 8a2ee8bca with only settle's rustfmt commit changing two files' formatting (MAIN 9c0b9aed0 has the same code): cargo check --workspace --tests clean; nightly fmt clean; clippy -p roko-agent -p roko-cli -p roko-compose -p roko-core -p roko-learn -p roko-serve --keep-going -D warnings clean; lib tests pass: roko-cli 3190 (two flakes, the turn_policy escalated-timeout test and graph_run_routing_observations_survive_a_crash, pass alone and in their module), roko-agent 2268, roko-core 1952, roko-learn 1204, roko-serve 986, roko-compose 560; cargo test -p roko-cli --test learning_wiring_census: 2 passed. Verify: its test passes in that run and its static checks pass on MAIN."
+++

## Problem

wk-gates' branch runs the workspace's `[[gates.rungs]]` on `roko plan run` (gap-3506f1). The task prompt's "## Verification Commands" section (`dispatch/prompt_builder.rs:1977`) still lists only the task's own verify steps, so agents aren't told that the workspace rungs (lint, tests, …) will also judge their work.

## Why it matters

Honest verdicts (epic spec-e9d7ec): an agent that doesn't know about a gate fails it, and burns retries on something it could have checked.

## Where

The Verification Commands section in `prompt_builder.rs`, and `TaskDef::build_prompt`.

## Plan

1. List the workspace rungs that will run for the task (by name, with their command), after the task's own steps.
2. Add `task_prompts_list_the_workspace_rungs`.

## Done when

- [ ] Prompts show every check that will judge the task.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on the gates branch.
- 2026-09-30 (wk-gates): Implemented on `work/gap-3506f1b` at `6f29511f4`; cargo verification deferred to the batch check. `plan_dispatch` plans the prompt from the task with every step it will run, its own and then its plan's rungs (`prompt_task`), so both prompt sections that list verify commands (`# Verify` and "Verification Commands") show the rungs. They show each rung by its command, not its name, since those sections list commands only. Routing reads no verify step, so only the prompt changes.
