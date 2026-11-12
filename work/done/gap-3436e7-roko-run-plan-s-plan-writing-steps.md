+++
id = "gap-3436e7"
kind = "gap"
title = "roko run --plan's plan-writing steps print engine lines (Step 1/2 Generating plan..., Agent completed (0s, 1075 bytes), Wrote tasks.toml (950 bytes) to <absolute path>, field-name quality warnings, T1 ids, Run this plan now?) that wrap at 80 columns; show the plan being written in the run view's style"
status = "done"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "M"
subsystem = ["roko-cli/commands", "roko-cli/plan_generate"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "6c274cd89"
source = "round v8 lead's wave 2 item 2; round v8 roko-recovery captures (tmp/hermes-roko-v8/design/terminal/before-plan)"
discovered_from = "agent:roko-recovery (round v8)"
anchors = ["crates/roko-cli/src/commands/run_cmd.rs::run_prompt_plan", "crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan"]
links = { depends_on = [], blocks = [], related = ["gap-2c87f0"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn writing_a_plan_reads_like_the_run_view' crates/roko-cli/src && cargo test -p roko-cli --lib writing_a_plan_reads_like_the_run_view"

[closed]
at = 2026-11-12
at_ts = "2026-11-12T21:43:09Z"
commit = "6c274cd89"
by = "roko-recovery"
executor = "claude-agent"
size = "M"
claimed_at = "2026-11-12T20:03:41Z"
forced = false
evidence = "6c274cd89 on work/gap-3436e7: roko run --plan shows the plan being written in the run view's style (roko_cli::plan_write_view): a header quoting the request with the planner's model and cap; on a terminal one live line of what the writer is doing (asking glm-4.7, fixing the plan, saving the plan; why it does it again faint) over time and spend on glass; in a pipe or CI a line per stage; then 'Plan written' with its directory, tasks, checks, time and spend (two lines when a terminal is narrow), its tasks as task N, waiting, after the ones they wait on, and roko's notes in plain words; then 'Run it now? [Y/n]', or after --dry-run or a kept plan 'Next  roko run plans/<slug>'; a failed write is one sentence with a command to paste. PlanRequest::quiet and agent_exec::without_progress_lines keep the generator's and its planner calls' own lines off the screen (--json now gets clean stdout too); roko plan generate and serve print as before. The run view's retry note says 'it changed nothing' for a try rejected before its checks. Branch checks: nightly fmt; roko-cli lib 4147 passed (8 ignored; writing_a_plan_reads_like_the_run_view and a_retry_says_what_the_try_failed_on among them), bin 486 passed; graph_plan_callers 2, e2e_self_host 4 and cli_fallback 27 passed; workspace clippy -D warnings clean; the binary builds. $0 captures (roko-recovery/runs/item7b, item7c, item7d: dry run, a run answered y, at 80x24 and 100x32, NO_COLOR, pipe, CI): no line reaches the last column (the old output had 332 frames that did)."
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
