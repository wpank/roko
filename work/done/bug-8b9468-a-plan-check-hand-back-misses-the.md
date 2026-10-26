+++
id = "bug-8b9468"
kind = "bug"
title = "A plan-check hand-back misses the task whose verify step runs the failing test when the test's name is built from plain words: phase 5 run 1f677933 sent spec_p5_presets (T1's tools/spec.sh 5 presets) to T8"
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
command = "grep -rqw 'fn the_task_that_runs_the_failing_ctest_target_leads' crates/roko-cli/src && cargo test -p roko-cli --lib -- the_task_that_runs_the_failing_ctest_target_leads"

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
evidence = "c953445b3: runs_target maps a failing test target named after a verify command's arguments to its task (tools/spec.sh 5 presets runs spec_p5_presets; tools/spec.sh 4_2 layout runs spec_p4_2_layout; a p before a number ignored; commands that select cases are read by the selector rule alone, which now skips each simple command's own program). A runner leads the hand-back; the tasks are the runners and the owners of files named on the failure's lines, and when no plan file is named the other changed files are offered second. The description, the diagnosis and the hand-back event name the runners. Verify: the_task_that_runs_the_failing_ctest_target_leads passes on run 1f677933's check log (fixture p5_presets_check.txt): T1 runs spec_p5_presets, leads and is the task it goes back to; the warnings in presets.cpp, filter.cpp, Displays.cpp and PluginProcessor.cpp name nothing. Chain26 on c953445b3's tree (base 99745fefa): the three verify tests 3/3; plan_handback:: and plan_runner's hand-back tests (a_plan_check_is_handed_back_at_most_max_handbacks_times, a_hand_back_whose_try_fails_ends_the_plan, a_hand_back_that_changes_nothing_is_not_satisfied_while_the_plan_check_fails, a_failure_across_two_tasks_goes_back_as_one_try_at_both_files, a_plan_started_during_a_failing_check_does_not_build_on_its_merge, a_cancel_stops_the_whole_plan_check_and_frees_the_workspace) 17/17; dispatch::prompt_builder, retry_feedback and graph_task_dispatch::verification 97/97; roko-learn failure_text and error_enrichment 9/9; nightly fmt and clippy -D warnings clean on roko-cli and roko-learn."
+++

## Problem

A plan-check hand-back (bug-fc05a3) is led by a task whose own verify steps run the failing test
(`runs_failing_test`). That rule matches only a test-name-like word of the task's command: one with a digit, `_`, `.`
or `*`. A runner's test target named after the command's plain words is missed.

Phase 5's live run `1f677933` (roko 74c3594bc) shows it.
- The plan check failed on core's `spec_p5_presets`: P5.03, RUMBLE Dark Room, `presets_test.cpp:141`.
- That is T1's test. Its verify step, `tools/spec.sh 5 presets`, builds and runs `spec_p5_presets`.
- "presets" is a plain word and "5" is not distinctive, so no task matched. roko handed the failure back to T8, the
  highest-tier fallback.
- The hand-back's notice named 9 files from anywhere in the output (4 WAVs, filter.cpp, presets.cpp,
  PluginProcessor.h/.cpp, Displays.cpp). It opened T1, T2, T4, T6 and T8's files to the try.

The evidence is in Undertow's `.roko/events.jsonl` (graph.plan_check_handback at about 15:58Z) and
`.roko/state/graph/p5-ship/delivery-check.log`.

## Plan

- A task runs a failing test when its verify command's arguments end the test's name. `tools/spec.sh 5 presets`
  runs `spec_p5_presets`, and `tools/spec.sh 4_2 layout` runs `spec_p4_2_layout`. A `p` before a number is ignored.
- Such a task leads the hand-back ahead of file mentions.
- The tasks it goes back to are the failing tests' tasks and the owners of files named on the failure's own lines.
  When neither names a task, it is every task that changed a file.
- The try may change their files. When no file of the plan is named on the failure's lines, it may also change the
  other files the plan changed, listed second: a failure such as P4.06's, or this one, can lie in another task's
  work.

## Design

roko-exact, 2026-10-26, built with bug-32f73a and bug-5e361a.

- **`runs_target`.** A simple command of a task's verify step (between `&&`, `;` and `|`) runs a test target named
  after its arguments.
  - The arguments are the words after the program, without flags and paths (`/`, `=`), split at `_`, lower case,
    with `p5` read as `5`. There must be at least two.
  - A failing name matches when, split at `_`, it has more parts and ends with them. `tools/spec.sh 5 presets` runs
    `spec_p5_presets`, and `tools/spec.sh 4_2 layout` runs `spec_p4_2_layout`.
  - A command with a quote or a glob selects cases, and only the selector rule reads it. So T5's
    `tools/spec.sh 4 display 'P4.05*'` runs no P4.06.
- **Failing names** come from the failure's own lines (bug-32f73a). A test that passed between two failing ones is
  not one.
- **The selector rule** reads each simple command with its own program skipped. A program after `&&`
  (`tools/spec.sh`) is no test's name.
- **The lead** is a runner, the highest tier among runners and the last in plan order among equals. With no runner,
  it is the highest tier among the tasks, as before.
- **Scope.** The tasks are the runners and the owners of named files. When neither names a task, it is every task
  that changed a file.
  - The try may change their files.
  - When no file of the plan is named on the failure's lines, the plan's other changed files are listed second:
    "The failure names no file of the plan, so its cause may lie in another task's work: it may also change …".
  - The borrowed checks stay its tasks' checks and the plan's check.
- **Feedback.** The diagnosis says "The test that failed is one task(s) T1 run among their own checks."
- **On the live run's output** (fixture `crates/roko-cli/tests/fixtures/p5_presets_check.txt`): no file is named,
  the runner is T1, and the failure goes back to T1, which leads. The warnings in `presets.cpp`, `filter.cpp`,
  `Displays.cpp` and `PluginProcessor.cpp` name nothing.

## Done when

The test in `[[verify]]` passes. Given phase 5's output, T1 leads and is the task the failure goes back to, and the
other tasks' files are offered second.
