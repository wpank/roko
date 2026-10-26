+++
id = "bug-56a806"
kind = "bug"
title = "Plan generation wraps a spec's exact check commands in planner-invented guards: every step of Undertow phase 2's third plan (op 1a965a97) is grep -q '<name>' <file> && tools/spec.sh 2 …, T4's grep for AllocationCounter (a spec-test helper) can never pass on correct code, and coverage still says 24 of 24 kept because a step that contains a check's command counts as running it"
status = "done"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
last_verified_rev = "e87ec04e3"
source = "round-v7-hermes-pilot#E-p2-plan4"
discovered_from = "agent:team-lead (round v7 phase 2, op 1a965a97)"
anchors = ["crates/roko-cli/src/plan_generate/source_checks.rs::apply_runs", "crates/roko-cli/src/plan_generate/source_checks.rs::assign_runs", "crates/roko-cli/src/plan_generate/source_checks.rs::SourceCoverage", "crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan"]
links = { depends_on = [], blocks = [], related = ["bug-72dc0d", "bug-3cdc58", "bug-8fe462"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn a_step_runs_the_spec_commands_exactly' crates/roko-cli/src/plan_generate/source_checks.rs && grep -qw 'fn a_planners_guards_around_spec_commands_are_dropped' crates/roko-cli/src/plan_generate/pipeline.rs && cargo test -p roko-cli --lib -- a_step_runs_the_spec_commands_exactly a_planners_guards_around_spec_commands_are_dropped"

[closed]
at = 2026-10-20
at_ts = "2026-10-20T17:55:55Z"
commit = "e87ec04e3"
by = "roko-exact (round v7)"
executor = "claude-agent"
via = "manual"
model = "claude-opus-5-5"
forced = false
evidence = "e87ec04e3: source_checks::run_exactly, a pass after apply_runs and assign_runs (pipeline.rs), makes a step that runs any source check's command run those commands alone (in its order, each once, as the source writes them); a step that runs none is left alone; a step that becomes the same as an earlier step of its task joins it (covers move), so PLAN_VERIFY_DUPLICATE cannot refuse the plan; [[meta.verify]] likewise for the plan's own checks; one repair line per step changed. apply_runs and assign_runs keep no guards (own_guards removed); at the step limit assign_runs joins the last step that runs source commands. Coverage keeps a check only by a step that is exactly source commands; a guarded step reads as covered by a step that runs another command. Planner and regeneration prompts say a run: command is the step's whole command. Red to green: a_planners_guards_around_spec_commands_are_dropped (generation with rumble-3's plan as the reply) failed on 91ee178ed (all 15 guards written, Source checks: 24 of 24 kept) and passes now (1 call, 15 exact spec commands, 24 of 24 kept). cargo test -p roko-cli --lib plan_generate: 156 passed; --test plan_validation: 38 passed; --test plan_validate: 23 passed, 2 failed (the known bug-2a31bc); clippy -p roko-cli -D warnings clean; nightly fmt clean. 0-dollar replay of op 1a965a97 (Undertow at de7b49b, the pilot's two drafts, harness scratchpad roko-exact/replay): before (main a1d12fc1c and 9f0870e9a) 2 calls, 24 of 24 kept with 15 grep guards, p2check 15 findings; after: 2 calls, 24 of 24 kept, every step exactly a spec command, validate 0 diagnostics, p2check 0 findings."
+++

## Problem

Round v7, Undertow phase 2, third plan write: roko main 9f0870e9a, planner gpt-5-4-mini, operation `1a965a97` from
a Hermes chat, plan `~/dev/undertow/plans/p2-undertow-rumble-3` (17:34). The phase spec gives each of its 23 checks
a command (`tools/spec.sh 2 lfo 'P2.01*'`, 15 distinct commands; rows that share one say so) and its notes say each
task's verify steps are those checks "with their commands unchanged". Every one of the plan's 15 task steps wraps a
spec command in a guard the planner invented, `grep -q '<name>' <file> && tools/spec.sh 2 …`:

- T4 step 4, `grep -q 'AllocationCounter' core/src/engine.cpp && tools/spec.sh 2 engine 'P2.18*,P2.19*'`:
  `spec::AllocationCounter` is a spec-test helper (`tests/spec/support/spec_alloc.h`, used by the tests). Correct
  product code never contains it, so the step can never pass, and the run would spend every rung's tries on it.
- Fragile on correct code: `grep -q 'duckGain' core/src/rumble.cpp` (an inline accessor in the header fails it),
  `grep -q 'flush' core/src/rumble.cpp`, `grep -q 'master_gain' core/src/output_stage.cpp` (the spec passes
  parameters by index), `grep -q 'valueAt' core/src/lfo.cpp`.
- roko still reported "Source checks: 24 of 24 kept as verify steps": a step that contains a check's command counts
  as running it.

Phase 1's plan ran the spec's commands bare and passed 5 of 5.

## Why it matters

Phase 2 of the Undertow build is on round v7's critical path, and its plan cannot pass as written while roko says it
keeps every check. A spec's check command is the person's acceptance test; roko's pitch is that it runs it as
written. Related: bug-72dc0d (c54fe8e23 made `apply_runs` keep a step's greps), bug-3cdc58 (coverage counts what a
step runs), bug-8fe462.

## Where

- `crates/roko-cli/src/plan_generate/source_checks.rs`:
  - `apply_runs` chains `own_guards(&step.command, &task.files)` in front of the source commands (c54fe8e23);
    `assign_runs` does the same for a step that cannot run, and at the tier's step limit appends to the last step.
  - `SourceCoverage::of` keeps a check by any step for which `runs_command` holds, which is "contains".
  - `planner_block` asks for "a `grep -q` for it before the run", then says a `run:` command is used "unchanged".
- `crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan`: the repairs a draft goes through.

## Current state

The planner writes the guards itself: the TSS rule's example (`grep -q 'fn new_test' … && cargo test …`, in
`plan_generate.rs`) and the source-check block teach a guard before every `fail_on_base` run. A guarded step can
run, so no repair touches it, and coverage counts it as keeping the check. The pilot's draft and the written plan
have the same 15 steps (`~/dev/undertow/.roko/plan-drafts/p2-undertow-rumble-3/rejected-1.md`).

## Plan

1. One normalisation pass after `apply_runs` and `assign_runs`: a step that runs any source check's command runs
   exactly the source commands it runs, in its order, each once, as the source writes them, and nothing else (greps,
   file tests, other runners go). A step that runs none is left alone. A step that becomes the same as an earlier
   step of its task joins it (its `covers` move there): the generated-plan budget refuses a repeated verify command
   (PLAN_VERIFY_DUPLICATE). `[[meta.verify]]` likewise, for the plan's own checks. One repair line per step changed.
2. `apply_runs` and `assign_runs` write the exact commands themselves, with no guards.
3. Coverage is exact: a check is kept only by a step whose command is exactly its command, or exactly a ` && ` join
   of source commands that includes it.
4. The planner block says a `run:` command is its step's whole command, with no guard.

## Done when

- rumble-3's plan through generation's repairs: every step is exactly a spec command, the AllocationCounter grep is
  gone, and coverage keeps all 24 (23 by exact task steps, SC24 by `[[meta.verify]]`); read before the repairs,
  the same plan keeps only SC24 (23 "covered by a step that runs another command").
- Plan 3's fixture (bug-72dc0d) through `apply_runs` gives T1 step 1 exactly `tools/spec.sh 2 lfo 'P2.01*'`.
- Two rows that share one command still run it once.
- A $0 replay of op 1a965a97's draft writes the exact commands.
- The `[[verify]]` command passes.

## Notes

- `plan_generate.rs`, `runnable.rs` and `plan_validate.rs` are roko-planner-2's work in progress (work/plan-058):
  leave them alone.
- Steps that run another task's source command are left to PLAN_VERIFY_DUPLICATE, as today.
