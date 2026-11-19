+++
id = "bug-b59d8e"
kind = "bug"
title = "A generated plan that lists a shared registration file in every task that edits it (core/CMakeLists.txt in Undertow phase 2's four tasks) fails the generated-plan budget (at most 2 serial owners per file, and the advisory concurrent overlap treated as fatal), the fix-up repeats the same draft, and roko writes no plan"
status = "done"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/plan-generate", "roko-cli/plan_policy"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
last_verified_rev = "313e4fbea"
source = "round-v7-hermes-pilot#E-p2-plan"
discovered_from = "agent:hermes-pilot (round v7 phase 2, op c31152b2)"
anchors = ["crates/roko-cli/src/plan_policy.rs::validate_plan_budgets", "crates/roko-cli/src/plan_policy.rs::is_advisory_code", "crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan"]
links = { depends_on = [], blocks = [], related = ["bug-daa54e", "bug-96115c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_shared_source_list_in_every_module_task_still_writes_a_plan' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_shared_source_list_in_every_module_task_still_writes_a_plan"

[closed]
at = 2026-10-20
at_ts = "2026-10-20T11:38:57Z"
commit = "c60ec9ec2"
by = "roko-planner (round v7)"
executor = "claude-agent"
via = "manual"
forced = false
evidence = "Fixed by bug-daa54e's change (c60ec9ec2, merged 313e4fbea): a build-registration file (roko_graph::exclusion::is_registration_file) no longer counts toward PLAN_FRAGMENTED_OWNERSHIP's serial owners, and two tasks that share only one do not overlap (first_overlap, which PLAN_CONCURRENT_OVERLAP and the engine's scheduler use), so a plan of phase 2's shape is written and T2 and T3 still run side by side. Verify at 313e4fbea: a_shared_source_list_in_every_module_task_still_writes_a_plan passes (T1 lfo; T2 rumble and T3 output after T1; T4 engine after all three; each lists core/CMakeLists.txt; one planner call; T2 stays parallel to T3). $0 replay of the real failed write (op c31152b2: the chat's 14,572-char prompt, a fake planner serving the pilot's drafts, a copy of Undertow at e2b57b2; harness tmp scratch p2/replay.py): with draft 1, 5de243ec8 makes 3 calls, rejects each for PLAN_FRAGMENTED_OWNERSHIP (4 serial owners) and PLAN_CONCURRENT_OVERLAP (T2/T3), and writes no plan; 313e4fbea writes it after 1 call, 20 of 20 source checks kept, spec quality A x4, plan validate 0 diagnostics, and the report line names core/CMakeLists.txt (T1, T2, T3, T4). That run used draft 1 with T2's and T3's max_loc at their tier's 150: the drafts as sent also ask 250 and 220 lines of integrative tasks, which the budget error hid (generation stops at the first failing gate) and which the planner's fix-up now sees as PLAN_TIER_SIZE. The accept-time union merge of such files is roko-ladder's item."
+++

## Problem

Round v7, Undertow phase 2: roko 5de243ec8, planner gpt-5-4-mini, plan `p2-undertow-rumble`, operation `c31152b2`,
asked from a Hermes chat. The spec's notes for the planner say tasks 1 to 3 run in parallel and that "each adds its own
source to `core/CMakeLists.txt`; task 4 makes sure all are listed". `core/CMakeLists.txt` lists the library's sources
one by one, so each of the four tasks edits it.

The planner listed the file in all four tasks' `files`, which is the honest choice bug-daa54e asks for. It also made T2
and T3 depend on T1. The generated-plan budget then rejected every draft:

```
generated plan violates the `default` structural budget:
  - PLAN_FRAGMENTED_OWNERSHIP: `core/CMakeLists.txt` is split across 4 serial tasks (T1, T2, T3, T4); merge shared-context edits into at most 2 cohesive owners
  - PLAN_CONCURRENT_OVERLAP: tasks T2 and T3 can run at the same time and both write `core/CMakeLists.txt`; make one depend on the other, or give them disjoint files
```

The two fix-up calls were given this text and came back with the same structure: all three drafts have identical
`files` and `depends_on`. After 3 calls ($0.126, 73 s), roko wrote no plan.

Two rules combine:
- `PlanExecutionPolicy::generated` allows at most 2 serial owners per file (`FAST_MAX_SERIAL_OWNERS_PER_FILE`). A source
  list that N tasks append to has N owners. The cap guards shared logic files, where splitting a file splits its
  context. A one-line registration doesn't have that cost.
- Generation fails on every budget finding. That includes `PLAN_CONCURRENT_OVERLAP`, which `is_advisory_code` says
  leaves a plan runnable, since the engine runs such tasks one after the other.

Phase 1 had the same shape and passed only because the planner left `core/CMakeLists.txt` out of T1 to T3's `files`.
The clash then came at accept time: run `0bed5c58` refused T3's passing first try, and the redo cost 457 s and $0.090.
So for this pattern, no plan is both honest about its files and allowed.

## Why it matters

Every compiled project registers new modules in shared files: a CMake source list, `Cargo.toml` members, a `mod.rs`,
an `index.ts`, an `__init__.py`. A spec that splits work into parallel module tasks, each with its own checks, is the
shape roko wants for cheap parallel work. Today such a spec gets either a plan with dishonest `files`, followed by
accept clashes, or no plan at all.

## Where

- `crates/roko-cli/src/plan_policy.rs`: `validate_plan_budgets` (`max_serial_owners_per_file`) and `is_advisory_code`.
- `crates/roko-cli/src/plan_generate/pipeline.rs`: the budget check after the repairs, which fails on any finding.
- The fix-up feedback the planner gets for these two codes.

## Plan

One of these, or a combination (the owner decides):
1. At generation, don't fail on advisory findings (`PLAN_CONCURRENT_OVERLAP`). Keep them as warnings in the plan's
   report.
2. Give the planner a way out it can take. The generator prompt and the fix-up feedback say how to keep a
   registration file within the budget: one task registers for everyone (the build picks sources up by glob, or that
   task lists every new source), and the other tasks depend on it.
3. Count registration edits differently. Appends to a build manifest's list don't count toward
   `max_serial_owners_per_file`, and the engine merges such accepts (a union merge) instead of refusing them.

## Done when

A plan of the phase-2 shape gets written: three parallel module tasks and one wiring task, each listing a shared source
list in `files`. Checked by the test in `[[verify]]`.

## Notes

- Evidence is in `tmp/hermes-roko-v7/pilot/evidence/p2-plan/`: the chat's tool call, the operation, the log lines and
  the three rejected drafts.
- Once bug-daa54e is fixed (list the shared file in `files`), every plan of this shape hits this bug.
