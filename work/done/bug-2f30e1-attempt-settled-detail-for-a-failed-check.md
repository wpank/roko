+++
id = "bug-2f30e1"
kind = "bug"
title = "attempt_settled.detail for a failed check is the reason's summary line ('verify: 1/1 verify step(s) failed ...'), not the failing step's key line"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph-task-dispatch"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "c9b7757f9"
source = "team-lead, round v6 (roko-caps' gap-2c9412 zero-cost run)"
discovered_from = "gap-2c9412"
anchors = ["crates/roko-cli/src/graph_task_dispatch/ladder.rs::publish_attempt_settled", "crates/roko-cli/src/graph_task_dispatch/verification.rs::verify_failure_summary"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-4e852d", "gap-2c9412"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn attempt_settled_names_the_failing_checks_key_line' crates/roko-cli/src && cargo test -p roko-cli --lib attempt_settled_names_the_failing_checks_key_line"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T20:12:50Z"
commit = "c9b7757f9"
forced = false
evidence = "attempt_settled_names_the_failing_checks_key_line passes. each_try_publishes_attempt_settled now expects 'exit code: 1' for a silent check. At c9b7757f9: roko-cli lib 3815 passed, 1 failed (bug-8d8f98's load flake). Zero-cost plan run (keyline-wrong): both gate_failed attempt_settled events carry detail='AssertionError: answer.txt holds 41, not 42' (failed_check verify:0/test), where the run before this change had detail='verify: 1/1 verify step(s) failed for task ...'. The pass on the strong rung keeps an empty detail."
+++

## Problem

`attempt_settled.detail` (gap-4e852d) is the first line of the attempt's failure reason. For a failed check, that is
the summary line, "verify: 1/1 verify step(s) failed for task `Write the answer`:", which never says why the check
failed. The check's own line ("AssertionError: answer.txt holds 41, not 42" in the gap-2c9412 zero-cost run) sits
further down the reason.

## Why it matters

`attempt_settled` is the event a host reads to show each try of a climb. For a try that failed its check, the detail
should be the check's key line.

## Where

- `crates/roko-cli/src/graph_task_dispatch/ladder.rs::publish_attempt_settled`
- `crates/roko-cli/src/graph_task_dispatch/verification.rs::verify_failure_summary` and `step_failure`, the reason's
  format

## Plan

For a `gate_failed` try whose reason lists failing steps, set `detail` to the first failing step's key line (the
same line `retry_feedback.errors` leads with), else to how that step ended. Every other outcome keeps the reason's
first line.

## Done when

`attempt_settled_names_the_failing_checks_key_line` passes: a try whose check prints a plain `AssertionError` line
publishes `attempt_settled` with that line as `detail`. The zero-cost run's events show it too.
