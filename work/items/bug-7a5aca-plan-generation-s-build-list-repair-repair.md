+++
id = "bug-7a5aca"
kind = "bug"
title = "Plan generation's build-list repair (repair_build_lists, bug-daa54e) reads only the base CMakeLists.txt, so it misses a glob an earlier task of the same plan adds: phase 4's plan gave plugin/CMakeLists.txt to T4-T7 beside T1, which adds the plugin/ui/*.cpp glob (5 owners where the spec allows two, parallel T4 and T6 sharing it)"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
source = "round-v7-hermes-pilot#N-p4-plan"
discovered_from = "agent:hermes-pilot (round v7, plancheck of p4-undertow-phosphor-editor, op fe5ffce6)"
anchors = ["crates/roko-cli/src/plan_generate/runnable.rs::repair_build_lists", "crates/roko-cli/src/plan_generate/runnable.rs::cmake_list_for"]
links = { depends_on = [], blocks = [], related = ["bug-daa54e", "bug-b59d8e", "bug-60ad88"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_glob_an_earlier_task_adds_keeps_the_build_list_out_of_later_tasks' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_glob_an_earlier_task_adds_keeps_the_build_list_out_of_later_tasks"
+++

## Problem

Plan generation adds a build file to the `files` of every task that creates a compiled source it doesn't see listed
(`repair_build_lists`, bug-daa54e). It reads only the base file. So it misses a glob that an earlier task of the same
plan adds, and gives the file to tasks that never need to touch it.

Round v7, Undertow phase 4: plan `p4-undertow-phosphor-editor`, op fe5ffce6, written from
`docs/specs/phase-4-ui.md`, roko 5a9884c95 + bug-5e597a.
- The spec's notes give `plugin/CMakeLists.txt` to task 1 alone. Task 1 adds
  `file(GLOB undertow_ui_sources CONFIGURE_DEPENDS "${CMAKE_CURRENT_SOURCE_DIR}/ui/*.cpp")`.
- The notes also say no file has more than two owners, and that the widget tasks' `.cpp` files are picked up by that
  glob.
- The plan's Repairs section, in roko's words:
  - "T4: added `plugin/CMakeLists.txt` to its files: it creates `plugin/ui/UnitArray.cpp`, which that CMake file lists"
  - "T5: added `plugin/CMakeLists.txt` to its files: it creates `plugin/ui/Look.cpp`, which that CMake file lists"
  - "T6: added `plugin/CMakeLists.txt` to its files: it creates `plugin/ui/Displays.cpp`, which that CMake file lists"
  - "T7: added `plugin/CMakeLists.txt` to its files: it creates `plugin/ui/Status.cpp`, which that CMake file lists"
- Result: 5 owners of `plugin/CMakeLists.txt` (T1, T4, T5, T6, T7).
  - T4 and T6 run side by side and both own it. Each agent is told the file is its own, so an edit by either risks
    an accept clash (phase 1's T3 redo).
  - The pilot's plancheck flagged it.
  - The person reviewing the plan removed the four by hand. The plan.md section "Review edits (person)" records it,
    and undertow-spec's reference passes with T4–T7 never touching the file.
  - `plan validate` didn't object: since bug-b59d8e, a build list may have many owners.

## Where

- `crates/roko-cli/src/plan_generate/runnable.rs:673` `repair_build_lists`.
- `cmake_list_for`, just below it. The nearest `CMakeLists.txt` that exists on disk decides on its own: it is "the
  list" unless its base text holds `GLOB`. The `written` set, the files earlier tasks write, is consulted only when no
  such file exists, and never for what an earlier task will make of an existing one.
- The test `a_shared_file_the_spec_says_a_task_edits_joins_its_files` (bug-daa54e, phase 1's case: parallel tasks
  each adding a source to an explicit list) must keep passing.

## Plan

Don't add the list to a task when an earlier task of the plan will make it collect the source's directory. A few
ways to tell, for the owner to choose:
- An earlier task that the creator waits for, directly or through others, has the list in its `files`, and its text
  (description or acceptance) says `file(GLOB` or `GLOB` with the source's directory, as phase 4's T1 does ("add
  `plugin/ui/*.cpp` to the plugin target with the requested CMake glob").
- The plan's source document (bug-60ad88 carries its path) has a `file(GLOB …)` naming that directory.
- Log the skip as a repair note, so a reader sees why the list stayed out.

## Done when

The test in `[[verify]]` passes on phase 4's case: an earlier task owns `plugin/CMakeLists.txt` and says it globs
`plugin/ui/*.cpp`, and the tasks creating `plugin/ui/*.cpp` don't get the list. bug-daa54e's test still passes.
