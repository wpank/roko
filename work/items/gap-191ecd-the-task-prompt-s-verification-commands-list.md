+++
id = "gap-191ecd"
kind = "gap"
title = "The task prompt's Verification Commands list only the task's own steps, not the workspace rungs that will also run"
status = "open"
triage = "unverified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/dispatch/prompt_builder"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-gates's report, checked on work/bug-5b43a9 at 7b25c478b)"
anchors = ["crates/roko-cli/src/dispatch/prompt_builder.rs", "crates/roko-cli/src/task_parser.rs::TaskDef"]
lane = "rust-hot"
parent = "spec-e9d7ec"
links = { depends_on = ["bug-5b43a9"], blocks = [], related = ["gap-3506f1", "bug-5b43a9", "bug-b4c565"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn task_prompts_list_the_workspace_rungs' crates/roko-cli/src/ && cargo test -p roko-cli --lib task_prompts_list_the_workspace_rungs"
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
