+++
id = "bug-1e272c"
kind = "bug"
title = "The hub's retry_feedback event lists no error line for a check that fails with a plain assertion: errors holds only compile, test and clippy lines"
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
source = "team-lead, round v6 (roko-caps' gap-2c9412 zero-cost run: errors=[] for 'AssertionError: answer.txt holds 41, not 42')"
discovered_from = "gap-2c9412"
anchors = ["crates/roko-cli/src/graph_task_dispatch/retry_feedback.rs::retry_feedback_event", "crates/roko-cli/src/graph_task_dispatch/verification.rs::step_failure"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-655ea5", "gap-2c9412"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn retry_feedback_carries_each_failing_checks_key_line' crates/roko-cli/src && cargo test -p roko-cli --lib retry_feedback_carries_each_failing_checks_key_line"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T20:12:49Z"
commit = "c9b7757f9"
forced = false
evidence = "retry_feedback_carries_each_failing_checks_key_line and key_line_picks_the_line_that_says_why pass. key_line handles cargo test, rustc, unittest, import errors, pytest and silent checks. At c9b7757f9: roko-cli lib 3815 passed, 1 failed (bug-8d8f98's load flake in plan_verify; alone at load 140 it failed 2 of 3); clippy and fmt clean. Zero-cost plan run (tmp/hermes-roko-v5/roko-caps/e2e-judged, keyline-wrong): both retry_feedback events in .roko/events.jsonl carry errors=['AssertionError: answer.txt holds 41, not 42'], where the run before this change published errors=[]."
+++

## Problem

The hub's `retry_feedback` event (gap-655ea5) builds `errors` only from the compile, test and clippy lines that
`GateFeedback::from_raw` recognises. A check that fails with a plain assertion line therefore publishes
`errors: []`. In the gap-2c9412 zero-cost run, the check printed "AssertionError: answer.txt holds 41, not 42",
and the next prompt showed that line under "Raw gate output", but the event did not carry it. `failing_steps` names
only the step and how it ended ("exit code: 1").

## Why it matters

A host that shows a climb from hub events, as the showcase does, cannot show why the check failed.

## Where

- `crates/roko-cli/src/graph_task_dispatch/retry_feedback.rs::retry_feedback_event`
- `crates/roko-cli/src/graph_task_dispatch/verification.rs::step_failure`, the format of each failing step's entry

## Plan

Lead `errors` with each failing step's key line: the line of its output that says why it failed, whatever form the
output takes. The compile, test and clippy lines follow, without repeats.

## Done when

`retry_feedback_carries_each_failing_checks_key_line` passes: a check that prints a plain `AssertionError` line and
exits 1 publishes `retry_feedback` with that line in `errors`. The zero-cost run's events show it too.
