+++
id = "bug-d88298"
kind = "bug"
title = "The plan estimate's note says 'similar tasks' when it rests on any task's attempts, and calls that medium confidence"
status = "open"
triage = "unverified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-learn"]
created = 2026-10-17
updated = 2026-10-17
source = "team-lead's review of R7 (gap-6605f7)"
anchors = ["crates/roko-learn/src/task_estimate.rs::note"]
links = { depends_on = [], blocks = [], related = ["gap-6605f7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn the_estimate_note_names_its_basis' crates/roko-learn/ && cargo test -p roko-learn --lib the_estimate_note_names_its_basis"
+++

## Problem

`POST /api/plans/{id}/estimate` (gap-6605f7) prices each task from the recent attempts of similar tasks, using the
best match it has:

1. tasks with the same role and tier;
2. else tasks of the same tier;
3. else any task;
4. else defaults.

The plan's `note` always says "from up to N recent attempts of similar tasks", even when the estimate rests on
attempts of *any* task, which are not similar. Its `confidence` stays `medium` in that case. Only defaults or an
unpriced model make it `low`. The lead found this reviewing R7, and the composer shows the note as the estimate's
basis.

## Why it matters

The note and the confidence are what a person reads to judge the figure. Calling unrelated tasks "similar"
overstates how well the estimate is grounded.

## Where

`crates/roko-learn/src/task_estimate.rs::note` and `::confidence`.

## Plan

1. `note` names the weakest basis among the tasks, and how many tasks rest on it:
   - the same role and tier;
   - the same tier;
   - any task, which says "low confidence";
   - defaults, which also says "low confidence".
2. `confidence` is `low` when a task rests on any task's attempts or on defaults.
3. Test `the_estimate_note_names_its_basis`.

## Done when

The `[[verify]]` passes.
