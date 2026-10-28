+++
id = "gap-af4082"
kind = "gap"
title = "A later task's try can rewrite a file an earlier task delivered and checked: diff_scope = record only logs it, nothing re-runs the earlier task's checks or tells the agent (T3 try 4 broke T1's CMakePresets.json after try 3 passed 5 of 6 tests)"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-10-19
updated = 2026-10-27
last_verified = 2026-10-27
last_verified_rev = "15335e2f9"
source = "round-v7-hermes-pilot#C-p0-run1"
discovered_from = "agent:hermes-pilot (round v7 phase 0, run 6988f0b8, T3 try 4)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/red_flags.rs", "crates/roko-core/src/config/gates.rs::DiffScope"]
links = { depends_on = [], blocks = [], related = ["spec-9230a9", "gap-abbd22", "bug-c22bd9"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_try_that_breaks_an_earlier_tasks_checked_file_fails_with_its_name' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_try_that_breaks_an_earlier_tasks_checked_file_fails_with_its_name"
+++

## Problem

Round v7, Undertow phase 0, run `6988f0b8` (roko 3b34aed92), task T3, the plugin. T3's `files` are
`plugin/CMakeLists.txt`, `plugin/PluginProcessor.h/.cpp`, `plugin/PluginEditor.h/.cpp` and `tools/check.sh`.

- **Try 3 (strong, gpt-5.4-mini) came close.** The plugin built and linked, and 5 of 6 plugin spec tests passed.
- **Try 4 broke T1's file.** It rewrote `CMakePresets.json`, a file T1 had delivered and whose checks (P0.01, P0.02)
  had passed. It flattened each test preset's `output`/`execution` objects into invalid fields. Every check then
  failed at configure in about 1 s (`Invalid extra field "noTestsAction" in Preset`), and the ladder was exhausted.
- **roko only logged it:** `WARN … red_flags: attempt changed paths outside its task's files … outside_scope
  CMakePresets.json: the task's files do not name it`.
  - Nothing re-ran T1's checks.
  - Nothing told the agent during the try.
  - The page's "why it did not pass" quoted `"outputOnFailure": true,` without saying the try had broken an earlier
    task's checked file.

## Why it matters

A plan's tasks build on each other's checked work. A late try that breaks an earlier task's delivered file turns a
near-pass into a failure, and nothing says so. With a multi-phase project like Undertow, where later phases add to
earlier ones, this can quietly break work that already passed.

## Where

- `crates/roko-cli/src/graph_task_dispatch/red_flags.rs`: changes outside the task's `files` are logged, and
  rejected only under `[gates] diff_scope = "enforce"` (module docs).
- `crates/roko-core/src/config/gates.rs`: `DiffScope`, default `Record`. `roko init` writes `diff_scope = "record"`.
- There's no tool-level scope check. write_file, edit_file and bash can change any path in the worktree. The
  post-dispatch safety check blocks only paths outside the working tree, secrets, and writes by roles that may not
  write.
- Decided 2026-09-29 (Will, in spec-9230a9): edits outside a task's `files` are recorded, and fail a try only under
  `enforce`. This item keeps that decision and covers the narrower case of an earlier task's checked file.

## Plan

Options, not exclusive:
1. When a try changes a file that an earlier task of the plan delivered, and whose checks passed, re-run that
   earlier task's checks with the try's checks. A failure there fails the try, with feedback naming the file and
   the earlier task.
2. Warn the agent at the edit: the tool result of a write or edit to such a file says "T1 delivered and checked this
   file; changing it may break T1's checks".
3. Name the out-of-scope change first in the verdict's why and the next try's feedback when the try's checks fail
   (here it was the cause).

## Done when

A try that breaks an earlier task's checked file is told so in its feedback, and an earlier task's failed re-check
fails it. Checked by the test in `[[verify]]`.

## Notes

- Evidence: `tmp/hermes-roko-v7/pilot/evidence/p0-run1/`, including the log excerpt with the WARN line. The broken
  file is in the worktree `.roko/worktrees/attempt-afa08b39d03a16f8ee04` (`git diff -- CMakePresets.json`).
- Related:
  - spec-9230a9: the attempt-diff epic, and Will's decision;
  - gap-abbd22: the attempt-diff check;
  - bug-c22bd9: without a finished build inside a try, the agent couldn't see what its edit broke.

Checked 2026-10-27: 2026-10-26 recheck at 15335e2f9: the three touched commits in red_flags.rs are unrelated features (bug-cbbaa8 hand-off continuation, bug-ba0c40 build-output scoping, a reasoning-cutoff re-ask); none re-runs an earlier task's checks when a later try breaks its file. Still open as described.
