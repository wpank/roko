+++
id = "gap-987064"
kind = "gap"
title = "Integration test C6: independent tasks still run after a failure, and tasks with overlapping files never run together"
status = "open"
triage = "unverified"
severity = "p1"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/tests"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e7"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W8-roko-as-executor.md (gate G6, canary C6)"
anchors = ["crates/roko-cli/tests/scheduler_canary.rs"]
lane = "rust-cold"
parent = "spec-a78d57"
links = { depends_on = ["gap-4d835d", "gap-96d348", "gap-439794", "gap-272448"], blocks = [], related = ["gap-3aa9cb"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn scheduler_canary' crates/roko-cli/tests/ && cargo test -p roko-cli --test scheduler_canary"
+++

## Problem

The scheduler fixes each have unit tests, in `roko-graph` or `plan_runner.rs`. Nothing checks, through the built
`roko` binary, that one plan run gets all of them together:

- a task that does not depend on a failed task still runs, even when it becomes ready after the failure;
- the failed task's dependants are skipped;
- tasks whose `files` overlap never run at the same time, and tasks whose `files` are disjoint do.

## Why it matters

This is the exit check for epic spec-a78d57 and canary C6 for W8's gate G6. It joins the golden-path suite (E11.1,
gap-3aa9cb).

## Where

- **New file:** `crates/roko-cli/tests/scheduler_canary.rs`.
- **Pattern to copy:** `crates/roko-cli/tests/graph_budget_resume.rs` (a scripted fake provider in `roko.toml`, and
  the built binary run with `assert_cmd`).
- **Surfaces to assert:** the fake provider's start and end log, the Graph checkpoint and `roko plan status`.

## Current state

No such test at `41c7ffbd6`. The ready queue and `SkipFailed` are merged (`bbf6517fc`); write-set admission
(gap-439794) and auto width (gap-272448) are not.

## Plan

1. Fake provider: it reads the task id from the prompt, logs `start <id> <ns>`, works for about a second, logs
   `end <id> <ns>`, and writes the file the task's verify step checks (never for T1).
2. Fixture plan with no `max_parallel` and `files` declared on every task:
   - T1: its verify fails, `max_retries = 0`; T2 depends on T1;
   - T3 depends on T4, a slower root, so T3 becomes ready only after T1 has failed;
   - T5 and T6: roots that list the same file; T7 and T8: roots with disjoint files.
3. Assert:
   - T1 failed, T2 was skipped as upstream-failed, and T3 passed, having started after T1 ended;
   - the intervals of T5 and T6 never overlap, and those of T7 and T8 do;
   - the plan reports failure only after every other task settled.

## Done when

- [ ] The test exists and passes in under a minute, with the fake provider only.
- [ ] It fails on a build without write-set admission (T5 and T6 overlap). Check this once by hand and say so in the
      closing evidence.
- [ ] The `[[verify]]` command passes.

## Notes

- The test edits no hot file. Write it while gap-439794 and gap-272448 are in progress, and merge it last.
- Compare the provider's own timestamps, and give each task at least 500 ms of work so that overlap is visible.
