+++
id = "bug-7ff055"
kind = "bug"
title = "Line ranges in files an earlier task changes fail the task before its agent runs"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/plan_policy"]
created = 2026-10-14
updated = 2026-10-14
last_verified = 2026-10-14
last_verified_rev = "6945c03b0"
source = "e2e-v4 dogfood, roko-stream worker, 2026-10-14 (roko main 86d629363)"
anchors = ["crates/roko-cli/src/plan_policy.rs::validate_plan_context", "crates/roko-cli/src/plan_policy.rs::render_declared_context", "crates/roko-cli/src/graph_task_dispatch.rs::plan_dispatch"]
links = { depends_on = [], blocks = [], related = ["bug-1c75cc"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn ranges_in_files_an_earlier_task_changes_fit_the_file' crates/roko-cli/ && cargo test -p roko-cli ranges_in_files_an_earlier_task_changes_fit_the_file"

[closed]
at = 2026-10-14
at_ts = "2026-10-14T18:17:38Z"
commit = "6945c03b0"
by = "commit trailer"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-14T17:20:05Z"
forced = false
model = "claude-opus-5-5"
evidence = "6945c03b0: validate_plan_context leaves a line range in a file an earlier task (a transitive dependency's files) changes to be checked when the task runs (a reversed range is still refused), and the Graph dispatcher fits such ranges to the file as the earlier task left it before building the prompt (plan_policy.rs fit_ranges_to_earlier_changes, graph_task_dispatch.rs plan_dispatch/fit_declared_ranges): a range past the end is cut at the last line, one starting past it becomes an unranged entry. Ranges in files no earlier task changes stay strict at both. Verify: cargo test -p roko-cli ranges_in_files_an_earlier_task_changes_fit_the_file passes. Also roko-cli --lib 3626 passed with 1 failure (active_self_model_picks_start_rung_and_logs_propensity, date-keyed, bug-3e2d76, fails without this change); clippy -D warnings clean on roko-cli."
+++

## Problem

A task whose `read_files` names a line range in a file an earlier task creates or edits fails before its agent runs
when the earlier task left the file shorter than the planner guessed:

```
T2 did not finish: planning error: pre-validation failed: line range `1-25` is outside `tally/export.py` (17 lines).
T3/T4 did not finish: planning error: pre-validation failed: line range `1-45` is outside `tally/budgets.py` (41 lines).
```

Seen in the e2e-v4 dogfood (runs 9fc6970b and 4af3845a, roko main 86d629363): the planner wrote the ranges before
T1 had written the file, so no range it could write was known to fit.

## Why it matters

Goal `core`: generated plans routinely give later tasks context in files earlier tasks write. The run spends on T1
and then fails its dependants on a guess, and a retry fails the same way.

## Where

- `crates/roko-cli/src/plan_policy.rs::render_declared_context`: renders the task's declared context at dispatch and
  fails on a range past the end of the file (`RunnerDispatchError::PreValidationFailed`).
- `crates/roko-cli/src/plan_policy.rs::validate_plan_context`: the plan-load check; it already skips a missing file
  that a dependency produces, but checks a range in an existing file that a dependency edits against the file as it
  is before the run.
- `crates/roko-cli/src/graph_task_dispatch.rs::plan_dispatch`: where the Graph dispatcher builds the prompt.

## Current state

bug-1c75cc clamps ranges past the end of a file at plan generation, against files as they are then; hand-written
plans stay strict. Nothing adjusts a range once an earlier task has changed the file.

## Plan

1. `validate_plan_context` leaves a range in a file an earlier task (a transitive dependency's `files`) changes to
   be checked when the task runs.
2. At dispatch, the Graph dispatcher fits such ranges to the file as the earlier task left it: a range that runs past
   the end is cut to the last line, and one that starts past it shows the file as an unranged entry. Ranges in
   files no earlier task changes stay strict.

## Done when

`[[verify]]` passes: a range past the end of a file an earlier task changes passes the plan check and renders cut
to the file at dispatch, while the same range in a file no earlier task changes is still refused.
