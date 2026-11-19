+++
id = "bug-5e361a"
kind = "bug"
title = "A plan-check hand-back's try dies in pre-validation on a declared line range that the plan's own work shortened: phase 5's rehearsal T6#2, line range 1-123 outside core/src/filter.cpp (122 lines)"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph-execution"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "c953445b3"
source = "round-v7 lead (phase 5 rehearsal 20261026-162312 and run 1f677933)"
discovered_from = "agent:team-lead (round v7, phase 5)"
anchors = ["crates/roko-cli/src/graph_execution/plan_handback.rs"]
links = { depends_on = [], blocks = [], related = ["bug-fc05a3", "bug-3b3bb6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_hand_back_reads_a_file_the_plan_changed_whole' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_hand_back_reads_a_file_the_plan_changed_whole"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T19:23:24Z"
commit = "c953445b3"
by = "roko-exact (round v7)"
executor = "claude-agent"
via = "manual"
size = "S"
model = "claude-opus-5-5"
forced = false
evidence = "c953445b3: handback_task drops the declared line range of every read_files entry whose file a task of the plan changed, so the hand-back's try reads it whole and pre-validation never sees a range older than the plan's work; a range in a file nothing changed stays. Verify: a_hand_back_reads_a_file_the_plan_changed_whole passes (T6's 1-123 of core/src/filter.cpp dropped, 1-40 of an unchanged header kept). Retries of a normal task are the p1 filed next (the same defect on every try). Chain26 on c953445b3's tree (base 99745fefa): the three verify tests 3/3; plan_handback:: and plan_runner's hand-back tests (a_plan_check_is_handed_back_at_most_max_handbacks_times, a_hand_back_whose_try_fails_ends_the_plan, a_hand_back_that_changes_nothing_is_not_satisfied_while_the_plan_check_fails, a_failure_across_two_tasks_goes_back_as_one_try_at_both_files, a_plan_started_during_a_failing_check_does_not_build_on_its_merge, a_cancel_stops_the_whole_plan_check_and_frees_the_workspace) 17/17; dispatch::prompt_builder, retry_feedback and graph_task_dispatch::verification 97/97; roko-learn failure_text and error_enrichment 9/9; nightly fmt and clippy -D warnings clean on roko-cli and roko-learn."
+++

## Problem

A plan-check hand-back's try (bug-fc05a3) runs the lead task's definition with its declared line ranges as planned.
When the plan's work has since changed a declared file, the try dies before its agent runs.

Phase 5's $0 rehearsal (on roko-main-74c3594bc) shows it. T6's own change removed a line from `core/src/filter.cpp`,
and its hand-back try (T6#2) then ended: "planning error: pre-validation failed: line range `1-123` is outside
`core/src/filter.cpp` (122 lines)". harness_error, in 0.1 s.

A normal try fits a declared range to a file an earlier task changed ("fitted a declared line range …",
bug-7ff055). That fitting runs only for a task that has dependencies, and only for files they changed. A hand-back
try clears its dependencies, and its own file changed under it. Any real run whose hand-back target shortened its
own file hits this.

## Plan

In the hand-back's task definition, drop the line range of every declared file that a task of the plan changed, so
the try reads it whole, as the plan's accepted work holds it. A range in a file nothing changed stays.

## Design

roko-exact, 2026-10-26, built with bug-32f73a and bug-8b9468.

- `handback_task` drops the `lines` of each `context.read_files` entry whose path a task of the plan changed
  (`owners`, `./` trimmed). Pre-validation then never sees a range that predates the plan's work, and the agent
  reads the file as the accepted work holds it. A range in a file nothing changed stays.
- Not `fit_ranges_to_earlier_changes`. That fits a range only for a task with dependencies, and only in files they
  changed. A hand-back clears its dependencies, and here the lead's own file changed. Reading the file whole is
  always valid, and these are files the try may change anyway.

## Done when

The test in `[[verify]]` passes: the hand-back's task reads the plan-changed file whole, and keeps the range of an
unchanged one.
