+++
id = "bug-309c28"
kind = "bug"
title = "roko diagnose finds no failure for a prompt run whose task failed: it reads plans/, not the run's records"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/diagnose"]
created = 2026-10-13
updated = 2026-10-13
last_verified = 2026-10-13
source = "hermes-plugin-roko docs walkthrough 2026-10-13 (roko 5de43f9ab)"
anchors = ["crates/roko-cli/src/commands"]
links = { depends_on = [], blocks = [], related = ["bug-103bce"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn diagnose_names_a_prompt_runs_failed_task' crates/roko-cli/ && cargo test -p roko-cli diagnose_names_a_prompt_runs_failed_task"
+++

## Problem

`roko diagnose <run id>` for a prompt run (a one-task plan named after the run) says "No task failure was
recorded for this run, so the plan failed outside its tasks" and "plan '<run id>' was not found under plans",
although the run's task failed (`task_completed {outcome: failed}`, attempt verdict `provider_error`). Seen
2026-10-13 on run 5b2e8076 (roko `5de43f9ab`).

## Why it matters

`roko diagnose` is the documented way to learn why a run failed; for prompt runs, which chat hosts start, it
reads the wrong place and finds nothing.

## Where

`crates/roko-cli/src/commands/diagnose*.rs`: reads the plan directory under `plans/`, not the run's
`.roko/runs/<run>/` records.

## Plan

1. For a run with no plan directory, read its tasks and attempt verdicts from `.roko/runs/<run>/attempts.jsonl`
   and the run's event index, including the error events bug-103bce adds.
2. Test: diagnosing a prompt run whose task failed names the task and its reason.

## Done when

`diagnose_names_a_prompt_runs_failed_task` passes (the `[[verify]]` command).
