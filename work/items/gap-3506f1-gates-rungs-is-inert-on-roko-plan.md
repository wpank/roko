+++
id = "gap-3506f1"
kind = "gap"
title = "[[gates.rungs]] is inert on roko plan run, so workspace gate rungs guard only roko run and roko do"
status = "open"
triage = "unverified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-29
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (20:01, wk-bench-rokoarm's report on gap-b7ab99)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/inert_settings.rs::graph_engine_inert_settings", "crates/roko-cli/src/graph_task_dispatch/verification.rs::settle_task_verification", "crates/roko-cli/src/run.rs::prompt_verify_steps", "crates/roko-core/src/config/gates.rs::GatesConfig"]
lane = "rust-hot"
parent = "spec-e9d7ec"
links = { depends_on = [], blocks = [], related = ["gap-7a3527", "gap-1426e4", "bug-50caf2", "gap-b7ab99"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q '\"gates.rungs\"' crates/roko-cli/src/graph_task_dispatch/inert_settings.rs && grep -rqw 'fn plan_run_task_runs_the_workspace_gate_rungs' crates/roko-cli/src/graph_task_dispatch/ && cargo test -p roko-cli --lib plan_run_task_runs_the_workspace_gate_rungs"
+++

## Problem

`[[gates.rungs]]` (`GatesConfig::custom_rungs`, `config/gates.rs:186`, serde name `rungs`) is the workspace's list of gate commands, and only some commands read it:

- **`roko run` and `roko do` read it.** `prompt_verify_steps` (run.rs:620-630) turns the required rungs into the prompt run's verify steps.
- **The legacy gate pipeline reads it** (`runner/gate_dispatch.rs::run_gate_once`, :956), but no plan-run path calls that pipeline.
- **`roko plan run` ignores it.** It verifies each task with the task's authored `[[task.verify]]` steps only (`graph_task_dispatch/verification.rs::settle_task_verification`).

`graph_engine_inert_settings` lists `gates.rungs` as inert, so a non-default value only produces a warning at dispatch and in `roko config doctor`. The warning's reason is stale: "only the legacy Runner-v2 gate pipeline (--engine legacy) reads it" (inert_settings.rs:66-69), and `--engine legacy` is now rejected.

## Why it matters

Honest verdicts (epic spec-e9d7ec):

- `roko init --profile …` writes rungs (commands/init.rs:100), and the README's quick start presents them as the workspace's gates.
- A user who adds a lint or test rung expects it to guard plan tasks. On plan run, a task "passes its gates" while those rungs never ran.
- The ViabilityBench Roko arm ran into this. `driver/planemit.py` writes explicit `[[gates.rungs]]` and notes that on plan run they only guard other paths (planemit.py:11-17).

## Where

- `settle_task_verification`: the plan-run verify step.
- `prompt_verify_steps`: how `roko run` turns rungs into steps.
- `graph_engine_inert_settings`: the warning list.
- `GatesConfig::effective_rungs` and `has_custom_rungs`.

## Current state

At BASE (4315add32), the rungs guard `roko run` and `roko do` but not plan runs. gap-7a3527 covers other dead keys and the stale `LEGACY_GATES` wording, but not wiring the rungs into plan run.

## Plan

There are two options:

- **(a) Wire them.** After a task's authored verify steps, run the workspace's required rungs (`effective_rungs`, `required`, non-empty command) as extra steps labelled with the rung name. Record the rungs in the verdict and the gate output. Let a plan opt out explicitly (for example `[meta] workspace_rungs = false`), and have `plan validate` show which rungs will run.
- **(b) Declare them prompt-only.** Document that rungs apply only to `roko run` and `roko do`, and make `plan validate` warn when a workspace defines rungs a plan will ignore.

(a) is recommended: it makes the configured gates mean the same thing on every path. Either way:

1. Remove `gates.rungs` from `graph_engine_inert_settings` once it is read, or fix its reason text.
2. Add `plan_run_task_runs_the_workspace_gate_rungs`: a task whose authored verify passes, but whose workspace rung fails, does not pass.

## Done when

- [ ] On `roko plan run`, a failing required workspace rung fails the task (or the chosen alternative is documented and warned about in `plan validate`).
- [ ] `gates.rungs` is no longer listed as inert.
- [ ] The `[[verify]]` command passes.

## Notes

- Watch the cost: rungs run per task and attempt. Consider running them once per task after the authored steps pass.
- gap-1426e4 (adaptive verify-command scoping) and bug-50caf2 (PlanGateCell gates the process cwd) touch the same verify path.
- 2026-09-30 (wk-gates): the fix adds a per-plan opt-out, `[meta] workspace_rungs = false`, because a rung that already fails on the base would otherwise fail every task (gap-161be1).
