+++
id = "bug-aa93e9"
kind = "bug"
title = "A plan written from a spec leaves the files the spec says to delete or edit out of its tasks' files: a generated task may own 8 files, deletions counted, so phase 1's planner dropped 21 retired files and 2 headers, and rule 5 of the implementer prompt keeps every agent off them"
status = "open"
triage = "verified"
severity = "p1"
goal = "core"
size = "M"
subsystem = ["roko-cli/plan_generate", "roko-cli/plan_policy", "roko-cli/plan_validate"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "6d4403d9b"
source = "round-v8 hermes-pilot-v8, phase 1 plan write op 19bfbbb8 (Undertow 19943c9, roko bd93521ee), tmp/hermes-roko-v8/pilot/evidence/p1-*"
discovered_from = "agent:hermes-pilot-v8 (round v8)"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan", "crates/roko-cli/src/plan_policy.rs::validate_plan_budgets", "crates/roko-cli/src/plan_policy.rs::validate_tier_sizes", "crates/roko-cli/src/plan_validate.rs::validate_plans_dir_impl", "crates/roko-cli/src/plan_generate.rs::PLAN_GENERATOR_SYSTEM_PROMPT"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-60ad88", "bug-3b3bb6", "bug-7c6f1b", "bug-ce1e4b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_specs_named_deletions_and_edits_join_their_tasks_files' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_specs_named_deletions_and_edits_join_their_tasks_files"

[[verify]]
command = "grep -rqw 'fn generation_gives_each_file_the_spec_deletes_to_its_task' crates/roko-cli/src/ && cargo test -p roko-cli --lib generation_gives_each_file_the_spec_deletes_to_its_task"

[[verify]]
command = "grep -rqw 'fn plan_validate_warns_of_a_file_the_sources_change_that_no_task_lists' crates/roko-cli/src/ && cargo test -p roko-cli --lib plan_validate_warns_of_a_file_the_sources_change_that_no_task_lists"
+++

## Problem

A plan written from a spec leaves the files the spec tells a task to delete, or to edit, out of every task's `files`.
The implementer prompt then keeps every agent off them: rule 5 says "Only modify files listed in the task's `files`
field", and rule 10 says a failure caused outside `files` gets a `Blocked: <path>` line, which stops a run when no task
owns that path (`graph_task_dispatch/blocked.rs::classify`).

Round v8's phase 1 plan write (Undertow 19943c9, roko bd93521ee, op 19bfbbb8, 2026-11-12 14:38) shows it:

- The spec (`docs/specs/phase-1-foundation.md`) retires twelve core files (`core/include/undertow/oscillator.h`,
  `core/src/oscillator.cpp`, `filter`, `envelope`, `lfo`, `voice`, `sequencer`) and nine `plugin/ui/` files
  (`Displays`, `Look`, `Status`, `UnitArray`, `UiFrame.h`). It also cuts `plugin/ui/Theme.h` down and replaces
  `plugin/PluginEditor.h`.
- The plan's T1 lists exactly 8 files, its 8 edits. T2 lists 7. None of the 21 deletions and neither of the 2 headers
  is in any task's `files`.
- The core must compile whole before any of T1's checks pass, and the retired files name v1's parameters. So T1
  could only fail, or say `Blocked:`, which stops the run.

The cause is not only the planner. A generated plan's task may own at most 8 `files`
(`PlanExecutionPolicy::generated`: `max_files_per_task` is FAST's 8, PLAN_BUDGET_FILES). Architectural's tier limit at
generation is also 8 (PLAN_TIER_SIZE), and the planner prompt says so: "architectural at most 8 files ... These limits
bind". The deletions count toward it. T1 had exactly 8 files: the planner met the limit by dropping the deletions.
The spec, fixed to name every path in each task's notes (Undertow e7131fb: T1 "Its files are all 20 of those paths",
T2 18), still cannot be planned as written: a draft that lists them all is refused (20 > 8) and sent back, a paid
call.

## Why it matters

- Phase 1 of Undertow 2 stopped before its run (hermes-pilot-v8, 14:52). Every later phase deletes or edits named
  files too.
- A person who reads a spec's file lists expects the plan to change those files. roko should never write a plan whose
  agents are forbidden to do what its spec asks.
- Goal: core (plan writing).

## Where

- `crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan`: the draft repairs and the budget checks
  (`validate_plan_budgets`, `validate_tier_sizes`, `sizes::raise_tiers`).
- `crates/roko-cli/src/plan_policy.rs`: `PlanExecutionPolicy::generated` (8 files), `validate_plan_budgets`,
  `validate_tier_sizes`, `TierSizeLimits`.
- `crates/roko-cli/src/plan_generate.rs`: the generator prompt's File Path Rules and checklist.
- `crates/roko-cli/src/plan_validate.rs::validate_plans_dir_impl`: where a plan's `[meta] sources` are checked
  (PLAN_061).
- `crates/roko-compose/src/templates/implementer.rs`: rules 5 and 10 (left as they are).

## Plan

1. **Lift the files a source names for change.** A sentence of the source (outside code blocks) whose first word,
   after list markers, emphasis and a `label:` lead, is a verb of change names its files for change. "Delete these
   nine:", "Deletes:", "Remove", "Retire" name deletions. "Edits:", "Edit", "Change", "Update", "Rewrite", "Replace",
   "Cut", "Add", "Fix" name edits. Only full workspace paths in code spans count, for files the workspace has and git
   doesn't ignore. A sentence that says never, not, keep or as it is names none.
2. **Repair at generation, with no planner call.** Each such file that no task lists joins the files of the task
   that owns its neighbours. That is the task owning most of the other files its sentence names, else the one whose
   files share the deepest directory with it. A tie goes to the first in plan order. Logged as a repair.
3. **A deleted file is no work.** A file a task deletes counts toward none of its size limits: PLAN_BUDGET_FILES,
   PLAN_TIER_SIZE and the tier raises at generation, and PLAN_TIER_SIZE in `plan validate`. That is the same rule as
   the sources in the read budget (bug-60ad88).
4. **Prompt.** A File Path Rule and a checklist line: a file the source says to delete or edit goes in the `files` of
   the task that does it, and the files a task deletes don't count toward its tier's file limit.
5. **plan validate.** PLAN_062, a warning: the plan's `[meta] sources` say to delete or change a file that no task
   lists in its `files`.
6. **Tests, red first.**
   - `a_specs_named_deletions_and_edits_join_their_tasks_files`: the fixed phase-1 spec and the plan as written. All
     21 deletions and the 2 headers land in T1 and T2, and the deletions count toward no limit.
   - `generation_gives_each_file_the_spec_deletes_to_its_task`: a fake planner answers with a plan that leaves them
     out. One call, and the plan written lists them.
   - `plan_validate_warns_of_a_file_the_sources_change_that_no_task_lists`: PLAN_062 on the plan as written, and
     nothing once its files list them.

## Done when

- All three `[[verify]]` commands pass.
- The plan as written, generated again from the fixed spec through a fake planner, lists every deleted and edited path
  in the right task's `files`, in one planner call.

## Notes

- Found by hermes-pilot-v8 (evidence: `tmp/hermes-roko-v8/pilot/evidence/p1-*`).
- T2 of the fixed spec has 9 edits, still over the generated cap of 8, even with its deletions exempt. That needs a
  split in the spec, or a raised cap: the lead's call, sent 2026-11-12.
- Related: bug-60ad88 (sources outside the read budget), bug-3b3bb6 (Blocked lines), bug-7c6f1b and bug-ce1e4b (file
  paths at generation).
