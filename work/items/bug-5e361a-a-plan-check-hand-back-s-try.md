+++
id = "bug-5e361a"
kind = "bug"
title = "A plan-check hand-back's try dies in pre-validation on a declared line range that the plan's own work shortened: phase 5's rehearsal T6#2, line range 1-123 outside core/src/filter.cpp (122 lines)"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph-execution"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 lead (phase 5 rehearsal 20261026-162312 and run 1f677933)"
discovered_from = "agent:team-lead (round v7, phase 5)"
anchors = ["crates/roko-cli/src/graph_execution/plan_handback.rs"]
links = { depends_on = [], blocks = [], related = ["bug-fc05a3", "bug-3b3bb6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_hand_back_reads_a_file_the_plan_changed_whole' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_hand_back_reads_a_file_the_plan_changed_whole"
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
