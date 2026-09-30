+++
id = "gap-9ca898"
kind = "gap"
title = "plan_generator.rs's DefaultPlanGenerator doesn't know rung"
status = "open"
triage = "unverified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan_generator"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-taskdef's report, checked on work/gap-0f3980 at b27c02717)"
anchors = ["crates/roko-cli/src/plan_generator.rs"]
lane = "rust-cold"
parent = "spec-e57870"
links = { depends_on = [], blocks = [], related = ["gap-0f3980"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn default_plan_generator_emits_task_rungs' crates/roko-cli/src/ && cargo test -p roko-cli --lib default_plan_generator_emits_task_rungs"
+++

## Problem

Tasks can name the rung they start on (`rung`, the tier ladder's entry point), but `plan_generator.rs`'s `DefaultPlanGenerator` never mentions `rung`: the file has no reference to it at b27c02717. Generated plans never set a starting rung, so every generated task starts at the default.

## Why it matters

Specs a cheap model can execute (epic spec-e57870): the planner is where a task's difficulty is best known. Without `rung`, the ladder can't start a hard task higher. p3.

## Where

`DefaultPlanGenerator` in `plan_generator.rs`, and the plan schema it writes.

## Plan

1. Let the generator emit `rung` (from the task's estimated complexity, or the planner's output), and document it in the generation prompt.
2. Add `default_plan_generator_emits_task_rungs`.

## Done when

- [ ] Generated plans carry a starting rung where the planner chose one.
- [ ] The `[[verify]]` command passes.
