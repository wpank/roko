+++
id = "bug-19ae56"
kind = "bug"
title = "prompt_builder::from_task computes its own skip_enrichment, ignoring the task's real [meta] flag"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/dispatch"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-6 follow-up reports 2026-10-03 (PK14 gap-997366)"
discovered_from = "gap-997366"
anchors = ["crates/roko-cli/src/dispatch/prompt_builder.rs::from_task", "crates/roko-cli/src/task_parser.rs", "crates/roko-cli/src/plan_brief.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn task_meta_skip_enrichment_suppresses_prompt_builder_sections' crates/roko-cli/ && cargo test -p roko-cli task_meta_skip_enrichment_suppresses_prompt_builder_sections"
+++

## Problem

A task's `[meta] skip_enrichment = true` (the flag several call sites set deliberately to suppress enrichment —
e.g. `crates/roko-cli/src/run.rs:787`, and several plan-authoring/revision task templates
`graph_execution/plan_runner.rs:4264,4386,4452,4697,7174`) has no effect on the actual prompt-building code that
decides what gets enriched. `crates/roko-cli/src/dispatch/prompt_builder.rs::from_task` (line ~195) computes its
own, same-named local variable from entirely different inputs:

```rust
let skip_enrichment = bounded_context_only || factor == 0;
```

`bounded_context_only` comes from `PlanExecutionPolicy::for_environment()` (an execution-policy setting, not the
task's own meta flag) and `factor` from the task's `context_weight` hint. `from_task` takes a `&TaskDef` but never
reads any `skip_enrichment` field from it or from the task's meta — the name is shared by coincidence, not by
wiring. So a task explicitly authored with `[meta] skip_enrichment = true` only actually skips enrichment here if
`bounded_context_only` happens to also be true, or its `context_weight` happens to yield `factor == 0` — otherwise
the meta flag is silently ignored and the task gets full enrichment anyway.

## Why it matters

Several call sites set `[meta] skip_enrichment = true` specifically to keep a small, generated task (plan
authoring/revision) from being handed a full workspace map, tasks.toml dump and plan brief it doesn't need — per
`plan_brief.rs:388-389`'s own risk note ("`skip_enrichment = true`: tasks run unenriched"). If `prompt_builder.rs`
doesn't actually honor it, those tasks may be getting more context (and burning more prompt budget) than their
authors intended, for no benefit.

## Where

- `crates/roko-cli/src/dispatch/prompt_builder.rs::from_task` (~line 195-260) — the disconnected local variable.
- `crates/roko-cli/src/task_parser.rs:50` — the real `skip_enrichment` field (on the meta struct `TaskDef`/its
  config wraps, not read here).
- `crates/roko-cli/src/plan_brief.rs:388-389` — documents the flag's intended effect.
- Callers that set the real flag expecting it to matter: `run.rs:787`, `graph_execution/plan_runner.rs` (several
  generated-task templates).

## Current state

Confirmed by reading `from_task`: no reference to the task's own `skip_enrichment` field anywhere in the
function. The two mechanisms (policy/context-weight-derived local variable vs. task-meta flag) are unconnected.

## Plan

1. Decide the intended relationship: should the task's own `[meta] skip_enrichment` flag OR the policy/weight
   derivation suppress enrichment (logical OR, matching the existing local variable's shape), or should the task
   flag take precedence?
2. Thread the real flag from `TaskDef`/its meta into `from_task`'s `skip_enrichment` computation:
   `let skip_enrichment = task.meta_skip_enrichment() || bounded_context_only || factor == 0;` (exact accessor
   name depends on where the field actually lives on `TaskDef`).
3. Regression test: a task with `[meta] skip_enrichment = true`, `bounded_context_only = false` and a
   `context_weight` that would NOT otherwise zero `factor`, confirming `from_task` produces empty
   `workspace_map`/`tasks_toml`/`workspace_context`/`plan_brief` sections.

## Done when

- A task's own `[meta] skip_enrichment = true` suppresses enrichment in `prompt_builder::from_task` regardless of
  `bounded_context_only` or `context_weight`.
- The `[[verify]]` command passes.

## Notes

- Confirm `TaskDef` actually carries the meta flag by the time it reaches `from_task` (it may currently be a
  property of a wrapping struct, not `TaskDef` itself) before assuming the one-line fix above is sufficient.

## Progress

- 2026-10-04 (w4-length): implemented on `work/bug-19ae56` at c309ab07c; cargo verification deferred to the batch
  gate. The flag lives in the plan's `TaskMeta`, not in `TaskDef`. `DispatchContext` gains `skip_enrichment`, which
  both Graph dispatch paths fill from the plan's meta (`plan_skips_enrichment`, through `read_plan_meta`).
  `from_task` ORs it with the policy and `context_weight` checks. Every other `DispatchContext` literal sets it to
  `false`, and `TaskMeta`'s doc says what the flag now does. Test:
  `task_meta_skip_enrichment_suppresses_prompt_builder_sections`.
- Effect to know about: `roko run`'s prompt plans (run.rs) and every ViabilityBench Roko-arm plan (planemit's
  `skip_enrichment = true` default) now run unenriched. Their prompts drop the `tasks.toml` dump, workspace map,
  workspace context and plan brief. That changes the bench arms' prompts before the pre-registration lock.
