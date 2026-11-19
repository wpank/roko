+++
id = "bug-aee1cb"
kind = "bug"
title = "The pre-verify scope check knows build output only by directory name, so output in a directory its tool marked (a CMake build tree, cargo under another CARGO_TARGET_DIR), which acceptance leaves out since bug-ba0c40, is an outside_scope finding and under diff_scope = enforce rejects the attempt"
status = "open"
triage = "verified"
severity = "p3"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
source = "bug-ba0c40 worker, 2026-10-20"
discovered_from = "bug-ba0c40"
anchors = ["crates/roko-gate/src/attempt_diff.rs::is_generated", "crates/roko-cli/src/graph_task_dispatch/red_flags.rs::attempt_diff_red_flag", "crates/roko-cli/src/orchestrator/worktree/build_output.rs::left_out"]
links = { depends_on = [], blocks = [], related = ["bug-ba0c40"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn scope_check_passes_over_tool_marked_build_output' crates/roko-cli/src && cargo test -p roko-cli --lib scope_check_passes_over_tool_marked_build_output"
+++

## Problem

The pre-verify screen's scope check (`roko_gate::attempt_diff::check_attempt_diff`) knows build output only by
the name of a directory on its path (`is_generated`: `target`, `node_modules`, `__pycache__`, `.pytest_cache`,
`.pyc`). Since bug-ba0c40, acceptance also treats a directory as build output when its tool marked it: one
holding a `CACHEDIR.TAG` (cargo writes one into its target directory, whatever `CARGO_TARGET_DIR` calls it) or a
`CMakeCache.txt` (a CMake build tree). It leaves that output out of the attempt's commit unless the task's
`files` name it (`crates/roko-cli/src/orchestrator/worktree/build_output.rs`).

So an attempt that runs `cmake -B build` where the repository does not ignore `build/`, or cargo with
`CARGO_TARGET_DIR=out` from a crate's directory, gets one `outside_scope` finding per output file. Each one goes
into the verdict's `scope_findings`, and with `[gates] diff_scope = "enforce"` they reject the attempt as out of
scope. Its next attempt then starts over in a fresh checkout, and the work is lost, although acceptance would
not have committed that output.

## Why it matters

The screen and acceptance should agree on what an attempt changed. Hundreds of findings for build output bury
real scope findings in the verdict, and under `enforce` they cost an attempt for output that never lands.

## Where

- `crates/roko-gate/src/attempt_diff.rs::is_generated`: the name-only rule that the scope check passes over.
- `crates/roko-cli/src/graph_task_dispatch/red_flags.rs::attempt_diff_red_flag`: the scope findings and the
  `enforce` rejection.
- `crates/roko-cli/src/orchestrator/worktree/build_output.rs::left_out`: acceptance's rule (names and marks).

## Plan

1. In `attempt_diff_red_flag`, drop the scope findings for paths that `build_output::left_out` leaves out. It
   needs the attempt diff's added paths, the directories of its base tree (`git ls-tree -r -d --name-only -z
   <base>`), the working tree and the task's `files`. Make the module `pub(crate)` for that.
2. Test: under `diff_scope = "enforce"`, an attempt that leaves a `build/` holding `CMakeCache.txt` beside its
   declared change is not rejected, and its verdict has no scope finding for `build/`.

## Done when

- [ ] The `[[verify]]` test passes, and fails before the fix.

## Notes

- Found while fixing bug-ba0c40 (2026-10-20). The name-based exemption by itself is older (`is_generated`).
