+++
id = "gap-32ddce"
kind = "gap"
title = "A passing try whose work conflicts with the plan branch only by adding lines to a registration file (core/CMakeLists.txt, mod.rs, Cargo.toml, __init__.py, index.ts) is thrown away and redone, instead of accepted with the union of both sides' additions"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/worktree"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
source = "round-v7-roko-ladder#union-merge"
discovered_from = "agent:hermes-pilot (round v7 phase 1 run 0bed5c58, T3 try 1)"
anchors = ["crates/roko-cli/src/orchestrator/worktree/acceptance.rs::fold_into_plan", "crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs::accept_attempt"]
links = { depends_on = [], blocks = [], related = ["bug-daa54e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_passing_try_that_only_adds_to_a_registration_file_is_accepted_with_the_union' crates/ && cargo test -p roko-cli --lib -- a_passing_try_that_only_adds_to_a_registration_file_is_accepted_with_the_union"
+++

## Problem

Round v7, Undertow phase 1 (run `0bed5c58`, plan `p1-undertow-voice`, base ff10c46). T1, T2 and T3 ran in parallel,
each in its own worktree. Each one adds its source to `core/CMakeLists.txt`. In the base, that file is:

```cmake
add_library(undertow_core STATIC
    src/params.cpp
    src/engine.cpp
)
```

T1 appended `    src/oscillator.cpp` after `src/engine.cpp` and was accepted first. T3's try 1 appended `    src/envelope.cpp`
at the same place, and **passed its checks** after 738 s ($0.1644). Its accept was refused: "passed its gates, but
its work conflicts with the plan's accepted work, so it was not accepted (… conflicted paths: core/CMakeLists.txt);
… the next attempt starts over from the plan branch". Try 2 redid the task from scratch: 457 s, $0.0903.

The two sides only added lines. The right merge keeps both added lines, but roko threw the passing work away.

## Why it matters

Every compiled project has registration files: a CMake source list, a Cargo workspace or `Cargo.toml`, a `mod.rs`,
an `__init__.py`, an `index.ts`. Parallel tasks that each add one entry to the same list clash at accept almost
every time, and each clash costs a whole try. bug-daa54e makes the planner list these files in each task's `files`
and stop serializing on them, so after it, the clashes become the normal case. Related: bug-daa54e.

## Where

- `crates/roko-cli/src/orchestrator/worktree/acceptance.rs::fold_into_plan`: folds an attempt's commit into the
  plan branch with `git merge-tree`. A conflict returns `WorktreeError::Conflict` and leaves the branch alone.
- `crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs::accept_attempt`: on `WorkspaceError::Conflict`,
  restarts the task from the plan tip with conflict feedback, and the try counts as failed.
- `crates/roko-graph/src/exclusion.rs::is_registration_file` (bug-daa54e, roko-planner-2): which files count as
  registration files. One predicate serves the scheduler, plan generation and this merge.

## Plan

1. When an accept conflicts and every conflicted path is a registration file, take a union merge, but only if, for
   each path, both sides only added lines relative to the merge base. The attempt's checkout then holds a commit
   that merges the plan branch's tip, with the union of both sides' additions in those files (`git merge-file
   --union`).
2. Re-run the task's verify steps in that checkout. If they pass, accept it: the commit descends from the plan tip,
   so the plan branch fast-forwards by compare-and-swap.
3. If anything else conflicts, a side deleted or changed a line, or the re-run fails, keep today's retry, with the
   reason in the retry feedback.
4. Say in the run's events what was merged, so a person can see it.

## Done when

A passing try that conflicts with the plan branch only by adding lines to a registration file is accepted with the
union of both sides' additions, after its checks pass on the merged tree; a conflict elsewhere still retries.
Checked by the test in `[[verify]]`: the phase-1 CMakeLists.txt case, in a scratch repo.

## Notes

- The marking is agreed with roko-planner-2 (bug-daa54e): `roko_graph::exclusion::is_registration_file`, by file
  name (`REGISTRATION_FILES`: BUILD, BUILD.bazel, CMakeLists.txt, Cargo.toml, __init__.py, index.js, index.ts,
  meson.build, mod.rs). This item depends on that predicate landing.
- Asked by the lead on 2026-10-20.
