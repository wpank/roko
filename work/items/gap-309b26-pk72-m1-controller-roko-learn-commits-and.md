+++
id = "gap-309b26"
kind = "gap"
title = "PK72 M1 controller: roko learn commits and roko learn rollback for guarded stores"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 72
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK72"
anchors = ["crates/roko-cli/src/commands/learn.rs"]
lane = "rust-cold"
parent = "spec-635697"
links = { depends_on = ["gap-099513"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn learn_rollback_restores_router_version' crates/roko-cli/ && cargo test -p roko-cli learn_rollback_restores_router_version"
+++

## Problem

This package delivers 1 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK72, slice 81xx, phase 8), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 8139 | S | p2 | roko learn commits and roko learn rollback for guarded stores | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8139-roko-learn-commits-and-rollback.md` |

## Why it matters

Phase 8: M1 controller and guarded commit (S06). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/8100-m1-ultrastable-controller-and-guarded-commit.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/commands/learn.rs`, `crates/roko-cli/src/commands/learn_commits.rs`.

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

- Waits on: PK71 (gap-099513).
- Suggested model: opus.

## Progress

Implemented by w4-pk02 on `work/gap-309b26` (from batch 13a, 004cb1b2f); cargo verification deferred to the batch gate.

- 8139: implemented at 70c00ec2f. `roko learn commits [--store] [--json]` and `roko learn rollback <store> --to <version>` in the new `commands/learn_commits.rs`. A rollback goes through `GuardedStore::rollback` (a `restored` row, actor `human`) and is refused while a plan run holds the runner lock. Router: the version is written back into `cascade-router.json` under the file's lock. Knowledge: the entries of batches committed after the version are deleted. Harness: M1 adopts the version at its next start.
