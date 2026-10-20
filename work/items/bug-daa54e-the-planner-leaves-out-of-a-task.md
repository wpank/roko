+++
id = "bug-daa54e"
kind = "bug"
title = "The planner leaves out of a task's files a shared file the spec says that task edits (core/CMakeLists.txt for phase 1's three parallel tasks), so the edit is out of scope and parallel tasks clash on it"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "round-v7-hermes-pilot#D-p1-plan"
discovered_from = "agent:hermes-pilot (round v7 phase 1, op f8dc4c50)"
anchors = ["crates/roko-cli/src/plan_generate.rs::build_generator_system_prompt", "crates/roko-cli/src/plan_generate/runnable.rs::repair_write_tools"]
links = { depends_on = [], blocks = [], related = ["bug-96115c", "gap-af4082", "bug-d758f6", "bug-3cdc58"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_shared_file_the_spec_says_a_task_edits_joins_its_files' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_shared_file_the_spec_says_a_task_edits_joins_its_files"
+++

## Problem

Round v7, Undertow phase 1 (roko f33fd217c, planner gpt-5-4-mini, plan `p1-undertow-voice`, operation `f8dc4c50`). The
spec's notes for the planner say, of the three parallel tasks:

> 1. **Oscillators** (`oscillator.h`, `oscillator.cpp`) … 2. **The filter** … 3. **The envelope** … Tasks 1 to 3 can
> run in parallel. Each adds its own source to `core/CMakeLists.txt`; the engine task makes sure all four are listed.

The plan gives T1, T2 and T3 only their own header and source:

```toml
files = ["core/include/undertow/oscillator.h", "core/src/oscillator.cpp"]   # T1; T2 and T3 alike
```

`core/CMakeLists.txt` lists the library's sources one by one (`add_library(undertow_core STATIC src/params.cpp
src/engine.cpp)`), so none of the three tests can link until its task edits that file. So:
- each of T1–T3 must edit a file outside its `files`. With `diff_scope = "record"` that is recorded, not refused;
- three parallel tasks edit the same lines of the same file, so their accepts onto the plan branch can clash. A
  refused accept retries the task from the updated branch, which costs a try.

## Why it matters

Every compiled project has shared registration files: a CMake source list, a Cargo workspace, a `mod.rs`, an
`index.ts`, a router table. A spec that says which tasks edit one should get a plan whose `files` say so too. Then
the scope check is honest and roko can order or serialize the conflicting edits.

## Where

- `crates/roko-cli/src/plan_generate.rs`: the generator prompt's `files` guidance.
- `crates/roko-cli/src/plan_generate/runnable.rs`: generation-time repairs (`repair_write_tools` is the pattern). A
  repair could add to a task's `files` the paths the spec text says it edits.

## Plan

1. Prompt: "When the spec says a task edits a shared file (a build file, a registry, a module list), put that file
   in the task's `files`, even if other tasks also list it."
2. A repair at generation: a path that the source names next to a task's own work and says it is edited ("adds its
   own source to `X`") joins that task's `files`.
3. With a shared path in several parallel tasks' `files`, roko can order their accepts deliberately, or warn about
   it (`plan validate`, a PLAN_05x note).

## Done when

Generated from phase 1's spec, with a fake planner replaying this plan, T1–T3's `files` include
`core/CMakeLists.txt`. Checked by the test in `[[verify]]`.

## Notes

- Evidence: `~/dev/undertow/plans/p1-undertow-voice/tasks.toml`; the spec is `~/dev/undertow/docs/specs/phase-1-voice.md`
  ("Notes for the planner").
- The run of this plan (round v7, from 17:47) records what the clash costs: see
  `tmp/hermes-roko-v7/findings/hermes-pilot.md` §D.
- Related:
  - gap-af4082: a later try rewriting an earlier task's file;
  - bug-d758f6: tool lists.
