+++
id = "gap-970f67"
kind = "gap"
title = "A verify step that needs a file only a later task writes can never pass when its task ends, and plan validate does not say so (round v7 phase-0 finding #2; the mori-imported 05-terminal-widgets plan)"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan-validate"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
last_verified_rev = "07270e625"
source = "round-v7-team-lead#phase-0-finding-2"
discovered_from = "agent:roko-planner (round v7, from the lead's phase-0 finding #2)"
anchors = ["crates/roko-cli/src/plan_generate/runnable.rs::problems", "crates/roko-cli/src/plan_validate.rs"]
links = { depends_on = [], blocks = [], related = ["gap-ff78e1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_check_needing_a_later_tasks_file_is_plan_058' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_check_needing_a_later_tasks_file_is_plan_058"

[closed]
at = 2026-10-20
at_ts = "2026-10-20T17:18:39Z"
commit = "bf82f912b"
by = "roko-planner (round v7)"
executor = "claude-agent"
via = "manual"
forced = false
evidence = "Merged dad2356d2 (work/plan-058: bf82f912b). runnable::needed_files gives the files a &&-only step cannot pass without (what its grep, file test, cat, head, tail or wc reads, its `<` inputs, a script it runs by path; a negated command needs nothing); runnable::later_files finds such a file the base lacks that only tasks the step's task does not wait for write. plan validate reports PLAN_058: an error when every writer waits for the task, a warning when one may run alongside; generation's fix-up names the writer (\"…does not exist until T3 writes it, and T3 runs after T2: check it in T3, or have T2 write it\"). The generator prompt's principle 9 says a check passes when its task ends and that later work (a later task's file, a suite whose tests later tasks add, the phase-0 ctest case roko cannot see) belongs to that task or to [[meta.verify]]. Verify at dad2356d2: a_check_needing_a_later_tasks_file_is_plan_058 passes (the mori 05-terminal-widgets shape: T2's check of a file only T3, after it, writes is an error; a parallel writer's file a warning; a negated test, an || chain, an existing file, the task's own and a dependency's file are not flagged; plan validate exits 1). Branch: roko-cli lib 3927 passed, 1 failed (plan_verify's heartbeat test, a load flake filed as bug-e362a1; it passed at dad2356d2); clippy -D warnings clean; integration plan_validation 38/38, plan_validate 23/25 (the 2 known bug-2a31bc failures). No PLAN_058 hit in the repo's plans, test fixtures or benchmarks (164 tasks.toml); the v6 mori imports have two (05-terminal-widgets, 71-mvp-gate)."
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
