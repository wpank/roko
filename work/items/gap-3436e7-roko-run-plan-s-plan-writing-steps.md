+++
id = "gap-3436e7"
kind = "gap"
title = "roko run --plan's plan-writing steps print engine lines (Step 1/2 Generating plan..., Agent completed (0s, 1075 bytes), Wrote tasks.toml (950 bytes) to <absolute path>, field-name quality warnings, T1 ids, Run this plan now?) that wrap at 80 columns; show the plan being written in the run view's style"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "M"
subsystem = ["roko-cli/commands", "roko-cli/plan_generate"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
source = "round v8 lead's wave 2 item 2; round v8 roko-recovery captures (tmp/hermes-roko-v8/design/terminal/before-plan)"
discovered_from = "agent:roko-recovery (round v8)"
anchors = ["crates/roko-cli/src/commands/run_cmd.rs::run_prompt_plan", "crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan"]
links = { depends_on = [], blocks = [], related = ["gap-2c87f0"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn writing_a_plan_reads_like_the_run_view' crates/roko-cli/src && cargo test -p roko-cli --lib writing_a_plan_reads_like_the_run_view"
+++

## Problem

`roko run --plan "<request>"` writes a plan, shows it, asks, and runs it. Its run already speaks in the run view's
style (gap-2c87f0); the plan-writing steps before it don't. A $0 capture (round v8:
`tmp/hermes-roko-v8/design/terminal/before-plan/`) prints:

```
◇ Plan  writing a plan first (--plan)
◇ Step 1/2  Generating plan...
📋 Generating a plan from the prompt: add-a-farewell-command
  Generating plan: add-a-farewell-command
  ✓ Agent completed (0s, 1075 bytes) [fake-model]
  ✓ Plan generated for: add-a-farewell-command
📋 Wrote tasks.toml (950 bytes) to /Users/…/ws/plans/add-a-farewell-command
📋 Wrote plan.md (98 bytes) to /Users/…/ws/plans/add-a-farewell-command
⚠️  Plan quality warnings for /Users/…/plans/add-a-farewell-command/tasks.toml:
  - T1: missing context.read_files; agent won't have file context
…
Plan written to /Users/…/plans/add-a-farewell-command:
  add-a-farewell-command (2 tasks)
    T1  Add farewell to the greeting module
    T2  Say goodbye from the command line  (after T1)
Run this plan now? [Y/n]
◇ Step 2/2  Executing plan (2 tasks)...
```

The problems:
- engine words: "Step 1/2", "Agent completed", bytes, field names;
- absolute paths, which wrap at 80 and 100 columns (332 frames reach the last column);
- task ids instead of "task 1";
- emoji;
- no time or spend while the plan is written;
- no Next after `--dry-run` that a person can paste.

## Why it matters

Goal tooling. `roko run --plan` is how a person starts anything bigger than a one-task change. The round v8 lead
asked for it (terminal output overhaul, wave 2 item 2).

## Where

- `crates/roko-cli/src/commands/run_cmd.rs`: `run_prompt_plan`, `confirm_plan_run`, `generate_prompt_plan`.
- `crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan`, whose lines print whoever calls it.
- `crates/roko-cli/src/agent_exec.rs`, whose "Agent working" and "Agent completed" lines print for every planner
  call.

## Plan

1. A view in roko's design system:
   - the header, with the request quoted, the planner's model and the cap;
   - on a terminal, a live "Writing the plan" line that says what the writer is doing, from `PlanProgress`
     (asking the model, fixing the plan, writing it), with time and spend on glass;
   - in a pipe or CI, one line per stage.
2. When the plan is written:
   - `✓ Plan written  plans/<slug> · 2 tasks · 2 checks · a plan check · time · spend`;
   - its tasks as "task N", each after the ones it waits on;
   - one dim line for the quality notes, in plain words.

   Then the question `Run it now? [Y/n]`, or, after `--dry-run`, `Next  roko run plans/<slug>`. A plan the person
   keeps ends with the same Next.
3. A failed write is one sentence, with what to do next.
4. `PlanRequest::quiet` keeps `generate_plan` and its planner calls from printing their own lines when the caller
   shows the generation. `roko plan generate` and serve are unchanged.

## Done when

- The captures at 80x24, 100x32 and 120x40, with NO_COLOR, in a pipe and in CI show the view, with no line at the
  last column.
- The exact-text test of the view's lines passes, and the `[[verify]]` passes.
