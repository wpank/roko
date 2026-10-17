+++
id = "gap-d364d2"
kind = "gap"
title = "The planner prompt never states the planner's turn cap, and the file budget it states can exceed what that cap allows"
status = "open"
triage = "unverified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan_generate"]
created = 2026-10-16
updated = 2026-10-16
source = "bug-09cc5f worker, 2026-10-16: the planner prompt's read budget (25 files at 200k tokens, 125 at 1M) against PLANNER_MAX_TURNS (30)"
discovered_from = "bug-09cc5f"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs::plan_task_prompt", "crates/roko-cli/src/plan_generate/pipeline.rs::PlannerBudget"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-09cc5f", "gap-2623b2", "gap-f8a6ea"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn planner_prompt_states_its_turn_cap' crates/roko-cli/src && cargo test -p roko-cli --lib -- planner_prompt_states_its_turn_cap"
+++

## Problem

Since bug-09cc5f every planner call runs under `PLANNER_MAX_TURNS` (30, `OperatingFrequency::Delta`), but nothing
tells the planner so. Its task prompt instead states a file budget sized from its context window
(`PlannerBudget::for_context_window`, gap-2623b2): "You may read up to {read_files} codebase files", one file per
2,000 tokens of a quarter of the window, at least 5. For this workspace's planner (`[models.claude-opus]`, 200,000
tokens) that is 25 files; for a 1M-token window it is 125. Each read is a tool call, so a planner that spends the
stated budget a file or two per turn reaches the cap before it writes the plan: a 200k planner after about 25 reads
and its searches, a 1M planner long before its stated budget. The call then ends `agent turn cap reached`, which stops
the generation without a retry.

## Why it matters

A capped planner call costs a full frontier call and writes nothing (bug-09cc5f's live check: 5 of 18 planner calls
hit the old 10-turn cap after 48–68 s, $0.39–0.60 API-equivalent each). A planner that knows its turn budget can
batch its reads into fewer turns or stop exploring in time.

## Where

- `crates/roko-cli/src/plan_generate/pipeline.rs::plan_task_prompt`: renders "You may read up to {read_files}
  codebase files" from `PlannerBudget`.
- `crates/roko-cli/src/plan_generate/pipeline.rs::PlannerBudget`: `read_files` grows with the context window and is
  not bounded by the turn cap.
- `crates/roko-cli/src/plan_generate/pipeline.rs::PLANNER_MAX_TURNS`: the cap every planner call runs under.

## Current state

Unchanged since bug-09cc5f: the prompt states the file budget and never the turn cap. Not measured live; the
numbers above are from the code and `roko.toml`.

## Plan

1. State the planner's turn budget in its task prompt, from `PLANNER_MAX_TURNS`, and that reads made together in one
   turn save turns.
2. Bound the stated file budget by what those turns allow, or drop the file count in favour of the turn budget.
3. Test: `planner_prompt_states_its_turn_cap`, the prompt of a planner with a 1M-token window states the turn cap
   and no file budget the cap cannot cover.

## Done when

- [ ] `planner_prompt_states_its_turn_cap` passes (the `[[verify]]` command).

## Notes

- Found while fixing bug-09cc5f on 2026-10-16 (worktree agent-a9cb59b33b95c3f51).
