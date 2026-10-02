+++
id = "gap-c1f4ac"
kind = "gap"
title = "PK15 Defaults that apply: One resolver for [runner] worktree_per_task, and roko run follows it (+4 more)"
status = "open"
triage = "verified"
severity = "p1"
goal = "golden-path"
rank = 15
size = "L"
subsystem = ["roko-cli/tests"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK15"
anchors = [".github/workflows/ci.yml", "crates/roko-cli/src/commands/do_cmd.rs", "crates/roko-cli/src/commands/plan.rs", "crates/roko-cli/src/graph_execution/batch.rs", "crates/roko-cli/src/prd.rs", "crates/roko-cli/src/run.rs", "crates/roko-cli/src/worker/cloud.rs", "crates/roko-cli/tests/plan_branch_integration.rs"]
lane = "rust-cold"
parent = "spec-fef7c5"
links = { depends_on = ["gap-997366"], blocks = [], related = ["gap-f30b8e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'worktree_per_task: false' crates/roko-cli/src/run.rs"

[[verify]]
command = "grep -rqw 'fn roko_run_follows_worktree_per_task' crates/roko-cli/tests/ && cargo test -p roko-cli --test plan_branch_integration roko_run_follows_worktree_per_task"

[[verify]]
command = "! grep -q 'worktree_per_task: false' crates/roko-cli/src/prd.rs crates/roko-cli/src/commands/do_cmd.rs crates/roko-cli/src/worker/cloud.rs"

[[verify]]
command = "grep -rqw 'fn golden_path_fixture_is_red_on_the_seed' crates/roko-cli/tests/ && cargo test -p roko-cli --test golden_path_acceptance golden_path_fixture_is_red_on_the_seed"

[[verify]]
command = "grep -rqw 'fn golden_path_fixture_plan_merges_green' crates/roko-cli/tests/ && cargo test -p roko-cli --test golden_path_acceptance golden_path_fixture_plan_merges_green"

[[verify]]
command = "grep -B4 'fn golden_path_live' crates/roko-cli/tests/golden_path_acceptance.rs | grep -q 'ignore' && cargo test -p roko-cli --test golden_path_acceptance --no-run"
+++

## Problem

This package delivers 5 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK15, slice 31xx, phase 3), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3112 | M | p3 | One resolver for [runner] worktree_per_task, and roko run follows it | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3112-one-worktree-resolver-and-roko-run-follows-it.md` |
| 2 | 3113 | S | p3 | roko do, roko prd and the cloud worker follow [runner] worktree_per_task | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3113-do-prd-and-cloud-worker-follow-worktree-per-task.md` |
| 3 | 3115 | M | p1 | Golden-path fixture: a seed repo and a spec-first plan whose checks are red on the seed | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3115-golden-path-fixture-red-on-the-seed.md` |
| 4 | 3116 | M | p1 | Golden-path scripted run: the fixture plan is delivered, escalates once, and its merge is green | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3116-golden-path-scripted-run-merges-green.md` |
| 5 | 3117 | S | p1 | Golden-path live harness: the same fixture on real cheap models, ignored by default | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3117-golden-path-live-harness.md` |

## Why it matters

Phase 3: golden-path proof. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3100-defaults-that-apply-parallelism-ladder-integration.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `.github/workflows/ci.yml`, `crates/roko-cli/src/commands/do_cmd.rs`, `crates/roko-cli/src/commands/plan.rs`, `crates/roko-cli/src/graph_execution/batch.rs`, `crates/roko-cli/src/prd.rs`, `crates/roko-cli/src/run.rs`, `crates/roko-cli/src/worker/cloud.rs`, `crates/roko-cli/tests/fixtures/golden_path/`, `crates/roko-cli/tests/fixtures/golden_path/replay/`, `crates/roko-cli/tests/golden_path_acceptance.rs`, `crates/roko-cli/tests/plan_branch_integration.rs`.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: PK14 (gap-997366).
- Existing work items this package covers or touches: gap-f30b8e. When its tasks are done, close those whose verify then passes.
- Suggested model: opus.
