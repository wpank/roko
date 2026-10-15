+++
id = "gap-5420f3"
kind = "gap"
title = "A check that hides its test runner's output fails silently; re-run the runner on its own to recover the failure text for the retry and the person"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "bug-59ed90 (tmp/hermes-roko-v5/findings/fresh-guide.md item 28, 2026-10-15)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs::run_verify_steps", "crates/roko-gate/src/spec_quality/shell.rs::hidden_runner_output"]
links = { depends_on = ["bug-59ed90"], blocks = [], related = ["bug-59ed90", "bug-11901c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_silent_check_quotes_what_its_runner_printed' crates/roko-cli/src && cargo test -p roko-cli --lib a_silent_check_quotes_what_its_runner_printed"
+++

## Problem

A verify step that hides its test runner's output (pipes it into `grep`/`awk`, sends it to `/dev/null`, captures it
in `$(…)`) fails with nothing to show: the retry prompt and the person get only "✗ exit status 1". bug-59ed90 stops
the planner from writing such steps, warns about them in `roko plan validate` (PLAN_050) and makes the failure name
the runner command that would show why. Hand-written and older plans still have them, and nothing recovers the
failure text itself.

Seen 2026-10-15 (tmp/hermes-roko-v5/findings/fresh-guide.md item 28): four retries of a task whose check was
`python3 -m unittest tests.test_cli 2>&1 | grep -E '^Ran [0-9]+ test' | awk '{if ($2 >= 11) exit 0; else exit 1}'`
got no failure text; the cause (one failing test) was found only by running the tests by hand.

## Why it matters

The retry cannot fix what it cannot see, so every attempt after a silent failure is wasted spend.

## Where

- `crates/roko-cli/src/graph_task_dispatch/verification.rs::run_verify_steps`: where a failed step's output becomes
  the retry's feedback.
- `crates/roko-gate/src/spec_quality/shell.rs::hidden_runner_output`: finds the hidden runner and the command that
  runs it on its own (`HiddenRunnerOutput::runner`).

## Current state

No mechanism re-runs a check's runner for diagnosis (checked at a8e27e4b7: `verify_focus` narrows cargo tests,
`sibling_settle` re-runs a step after a sibling's edit). bug-59ed90 adds the runner's command to the failure text.

## Plan

1. When a step fails, printed nothing, and `hidden_runner_output` finds its runner, run that runner once on its own
   (same workdir, env and timeout, under the compile permit when it builds), and put the tail of what it printed in
   the step's failure text as "what `<runner>` printed". The verdict stays the step's.
2. Bound it: once per failed step, skipped when the step timed out or the attempt was stopped.

## Done when

`[[verify]]` passes: a failing `… | grep -q OK` step's retry prompt quotes the runner's own failure line.
