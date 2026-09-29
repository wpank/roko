+++
id = "gap-d14a43"
kind = "gap"
title = "Planner-written acceptance tests in [task.accept], stored out of the agent's reach"
status = "done"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/task_parser", "roko-cli/plan_validate"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "b11ca807d"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e8"
discovered_from = "tmp/cybernetic-harness/tldr/research/B4-gates-qa-safety.md (the accept/ convention; tldr/05 P1 #10)"
anchors = ["crates/roko-cli/src/task_parser.rs::TaskDef", "crates/roko-cli/src/plan_validate.rs::validate_tasks_file", "crates/roko-cli/src/task_accept.rs"]
lane = "rust-cold"
parent = "spec-e57870"
links = { depends_on = [], blocks = [], related = ["gap-abbd22", "gap-b3fa0a", "find-70edcb"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn accept_table_compiles_to_a_pinned_verify_step' crates/roko-cli/src/ && cargo test -p roko-cli --lib accept_table_compiles_to_a_pinned_verify_step"

[[verify]]
command = "grep -rqw 'fn accept_store_rejects_a_changed_source' crates/roko-cli/src/ && cargo test -p roko-cli --lib accept_store_rejects_a_changed_source"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "[task.accept] (TaskDef.accept, task_accept.rs): sources pinned to ~/.roko/accept/<ws>/<plan>/<task>/ by sha256 at load, one generated verify step per test (TAMPERED on hash mismatch, exact passing count required); PLAN_038 lint; a 3-line hook in run_graph_plan_body; 08f-final-polish migrated (d81c8872c + 1b5a5b2ae + rustfmt 37b6d7c95, merged af51b7a61). Batch 6 gate (work/rust-batch-5 tree plus fmt-only and unused-import fixes fdb2a9b72, 579ffd0e6, a8dd7f09f, 2aa55ab1f): cargo check --workspace --tests clean; clippy -p roko-cli -p roko-core -p roko-agent -p roko-serve -p roko-learn -p roko-gateway --no-deps -D warnings clean; lib tests roko-cli 3087, roko-agent 2249, roko-core 1922, roko-learn 1177, roko-serve 955, roko-gate 685, roko-gateway 41, 0 failed; merged MAIN tree re-checked (cargo check --workspace --tests clean)."
+++

## Problem

The one independent check that worked on real runs is a hand convention. Portal plans 08b–08g keep acceptance tests
in `plans/<plan>/accept/` (6 plans, 48 files), and each gate copies a test over the agent's copy and runs it with an
exact passing count (`plans/portal-programme/08f-final-polish/tasks.toml:106`). But the sources sit in the working
tree, where the agent can edit them; the copy, count and runner are hand-written into every verify command; and
`TaskDef` has no field for them, so nothing can check, hash or report them.

## Why it matters

tldr/04 design rules 3 and 4: the planner writes the gating checks, and cheap models game visible checks most
(`zhao2026specbench`). tldr/05 decision 6 (default): implementers may not change planner-written tests. E9's diff
check (gap-abbd22) needs to know which files those are. Part of epic spec-e57870.

## Where

- `crates/roko-cli/src/task_parser.rs::TaskDef`: add `accept: Option<TaskAccept>`.
- **New file:** `crates/roko-cli/src/task_accept.rs`: the store and the generated verify step.
- `crates/roko-cli/src/plan_validate.rs::validate_tasks_file`: a new `PLAN_0xx` code. Also update the known-field
  lists in `prd.rs` and `plan_generator.rs`, and `TasksFile::validate_against_schema`, so `--strict` accepts the key.

## Current state

Checked at `41c7ffbd6`: `TaskDef` has no accept field; the `accept/` directories are plain files in the tree.

## Plan

1. Schema: `[task.accept]` with
   `files = [{ src = "accept/x.test.ts", dest = "apps/portal/src/x.test.ts", runner = "node scripts/vitest-min.mjs {dest}", count = 16 }]`.
   `src` is relative to the plan directory.
2. At plan load, copy each `src` into a store outside the workdir (`~/.roko/accept/<workspace>/<plan>/<task>/`),
   and record its sha256 in the checkpoint.
3. Compile each entry into a verify step that runs before the task's own steps. It checks the stored hash, copies
   the stored file over `dest`, runs `runner`, and requires exactly `count` passing tests.
4. `plan validate`: an error when a `src` is missing or `count` is 0; a warning when a verify step still hand-copies
   from `accept/`.
5. Migrate one portal plan (08f) as the worked example.

## Done when

- [ ] A task with `[task.accept]` runs the pinned test from the store, whatever the agent did to `dest` or `src`.
- [ ] Changing the stored file makes the step fail with a tamper message.
- [ ] Both `[[verify]]` commands pass.

## Notes

- No hot-file edit: the generated step runs through the existing verify path.
- With no OS sandbox the store is tamper-evident, not tamper-proof; gap-abbd22 flags edits to `accept/` sources.
- Not filed here: S07.8's `[task.hidden]`, which never holds tests. Red-on-base proof is gap-b3fa0a's.
- Implemented on `work/gap-d14a43` at `115e87d36`; cargo verification deferred to the batch check.
- For gap-abbd22: a task's accept files are `TaskDef.accept` (`src` relative to the plan dir, `dest` to the working
  tree), and `task_accept::is_pinned_step` recognizes the generated steps at the front of `verify`.
