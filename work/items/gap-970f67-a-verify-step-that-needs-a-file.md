+++
id = "gap-970f67"
kind = "gap"
title = "A verify step that needs a file only a later task writes can never pass when its task ends, and plan validate does not say so (round v7 phase-0 finding #2; the mori-imported 05-terminal-widgets plan)"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan-validate"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
source = "round-v7-team-lead#phase-0-finding-2"
discovered_from = "agent:roko-planner (round v7, from the lead's phase-0 finding #2)"
anchors = ["crates/roko-cli/src/plan_generate/runnable.rs::problems", "crates/roko-cli/src/plan_validate.rs"]
links = { depends_on = [], blocks = [], related = ["gap-ff78e1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_check_needing_a_later_tasks_file_is_plan_058' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_check_needing_a_later_tasks_file_is_plan_058"
+++

## Problem

A task's checks run when that task ends, before the tasks that wait for it. A check that needs a file only a later
task writes can therefore never pass. Its task burns every try, or its agent writes the later task's file out of
scope to get past it.

- Round v7, phase 0 (the lead's finding #2): T1's `ctest --preset core` could pass only once T2 and T3 had made
  their spec tests present. roko cannot see through ctest; that case is the spec's and the prompt's to prevent.
- The case roko can see is a step that names the file. A scan of the v6 mori imports found one: the
  `05-terminal-widgets` plan. Its T2 writes `src/widgets/mod.rs` and checks `test -f src/widgets/sparkline.rs` (and
  six more widget files), which only T3–T9 write, and they all wait for T2.

`plan generate` already sends such a step back ("needs `X`, which does not exist and no task writes before it
runs"), but it does not say which task writes it. `plan validate` does not report it at all.

## Why it matters

A plan that cannot pass should be refused before it runs, whoever wrote it: a planner, a person or an importer.

## Where

- `crates/roko-cli/src/plan_generate/runnable.rs`: the step checks (`problems`, `step_problem`).
- `crates/roko-cli/src/plan_validate.rs`: the PLAN_05x checks.
- `crates/roko-cli/src/plan_generate.rs`: the generator prompt's core principles.

## Plan

- Report a `&&`-only step that reads a file (grep, a file test, cat …, or a script run by its path) when that file
  is absent from the base and only tasks the step's task does not wait for write it. This is PLAN_058: an error when
  every writer runs after the task, a warning when one may run alongside it.
- Generation names the later writer in its fix-up.
- The prompt states the rule, including the whole-suite case.

## Done when

Plan validate reports PLAN_058 for such a step, and nothing for a negated test, an `||` chain, or a file that exists
or that the task or a task it waits for writes. Checked by the test in `[[verify]]`.

## Notes

- Related: gap-ff78e1 (plan validate does not warn about other steps that cannot pass).
