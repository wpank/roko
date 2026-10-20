+++
id = "bug-6fa025"
kind = "bug"
title = "A verify step whose command is not found (exit 127) is retried as the agent's failure until the ladder is exhausted"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-13
updated = 2026-10-15
last_verified = 2026-10-13
source = "hermes-plugin-roko dogfood, 2026-10-13 (roko serve :6716 built from 6ab6fde47)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs::run_verify_steps", "crates/roko-cli/src/graph_task_dispatch/ladder.rs"]
links = { depends_on = [], blocks = [], related = ["bug-369c21"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_missing_verify_command_fails_once_without_retry' crates/roko-cli/ && cargo test -p roko-cli a_missing_verify_command_fails_once_without_retry"
+++

## Problem

Plan `project-tally-a-small-python-cli-expense-ledger`, run `e7fef800-c269-44c2-b4db-7548f4207898` (Hermes plugin
dogfood, 2026-10-13, a stdlib-only Python project on macOS):

- T1's authored verify step was `grep -q 'class BudgetStore' tally/budgets.py && python -m py_compile tally/budgets.py`.
  It failed on every attempt with `bash: line 1: python: command not found`, exit 127. On this machine `python` is
  only a zsh alias; `/usr/bin/python3` exists.
- roko retried T1 six times on glm-5.1, taking 9m26s and $0.165, and ended with diagnosis `ladder_exhausted`.
  T2 to T5 were blocked or skipped.
- Each retry told the agent that its work failed a check, although no change to the code could ever pass it.
- The planner wrote `python` while the workspace's own gate rung runs `python3 -m unittest …`. The spec gate scored
  every task `allow` (84 to 100, band A), and nothing ran the check before the plan started.
- The same `agent_spawned` rows carry `attempt: 0` six times (filed separately).

## Why it matters

A missing command is an environment or plan defect, not the agent's failure. Retrying it wastes money and time,
fails the whole plan, blames the agent (so learners are fed a false failure), and hides the one-line fix from the
person. A first run on a Mac without `python` hits this (goal `core`).

## Where

- `crates/roko-cli/src/graph_task_dispatch/verification.rs::run_verify_steps`: each step's verdict and exit code
  (`step_exit_code`) are known here.
- `crates/roko-cli/src/graph_task_dispatch/ladder.rs`: the retry and escalation decisions.
- The planner's verify-command authoring in `crates/roko-cli/src/plan_generate/`. `baseline_verify.rs` judges only
  test output, so it does not catch this case.

## Current state

No code treats exit 126 or 127 (or "command not found" on stderr) from a verify step specially. The failure is
retried like a failing test.

## Plan

1. Classify exit 126 or 127 from a verify step (and a shell's "command not found" or "Permission denied" for the
   command itself) as an environment or check defect. Fail the task once with a diagnosis that names the missing
   command and the step, with no retry, no escalation and no agent blame.
2. Optionally, preflight authored verify commands when a plan loads (`command -v` for the first word of each
   `&&`/`||`/`;` part) and in `roko plan validate`, so the plan is refused before anything is spent.
3. Have the planner use the interpreter and commands that the workspace's gate rungs already use (`python3` here).

## Done when

`[[verify]]` passes: a verify step whose command does not exist ends the task after one attempt, with a diagnosis
that names the command, and nothing is retried.

## Notes

Evidence: run `e7fef800` in the dogfood's scratch workspace, saved by the plugin work as
`failed_run_events.json`, which is not in this repo. The Hermes model diagnosed it correctly from the error rows,
which `bug-103bce` made available.

2026-10-15 (status-quo assessment, finding 9): still open at b42578bab. No commit since 2026-10-13 touches `graph_task_dispatch/verification.rs` or `plan_generate/`, and no other item links here. The assessment's 49-run sweep of the 10-13/10-14 dogfood workspaces found `ladder_exhausted` twice, both from this cause, and the spec gate scored the uncheckable task `allow` (`tmp/status-quo/2026-10-15/programme/E2a-matrix-retag-AU-EX-IS-QA-RC.md`, rows AU3, EX6, EX7, QA1). Decision 5 (2026-10-15) funds the golden-path live reruns (gap-c013c5, then gap-f30b8e). A missing command retried up the ladder would burn that money, so this should land first.
