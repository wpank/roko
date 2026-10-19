+++
id = "gap-770cb5"
kind = "gap"
title = "An agent has no build or check tool for CMake projects (run_tests' build enum has no cmake or ctest), so a try can't iterate on compile errors and every one costs a whole try"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-std/tools"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "round-v7-hermes-pilot#C-p0-tools"
discovered_from = "agent:hermes-pilot (round v7 phase 0, 2026-10-19)"
anchors = ["crates/roko-std/src/tool/builtin/run_tests.rs::tool_def", "crates/roko-cli/src/graph_task_dispatch/verification.rs"]
links = { depends_on = [], blocks = [], related = ["bug-d758f6", "gap-e02ec4", "gap-6754ab", "gap-6a319c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_cmake_task_can_run_its_checks_mid_try' crates/ && cargo test -p roko-cli --lib -- a_cmake_task_can_run_its_checks_mid_try"
+++

## Problem

An agent working on a CMake project has no tool that builds the code or runs the task's checks. The only way is a
`bash` the planner usually takes away (bug-d758f6).

- `run_tests`, the built-in made for this, takes a `build` enum of cargo, npm, go, pytest, forge and make, and runs
  that ecosystem's whole suite: `make test` for make. There is no cmake or ctest. In a CMake project, `make test` at
  the root finds no Makefile: the build lives in `build/<preset>/`.
- Round v7's Undertow (C++/JUCE) checks are commands like `tools/spec.sh 0 params 'P0.01*'`: configure the preset,
  build one target, run it. No built-in tool runs them.
- So a try that needs one compile-and-fix loop can't do it inside the try. It hands off, the gate runs the check
  cold, and the failure comes back as a new try, with a new model call and the whole prompt again.

## Why it matters

The cheap rungs need fast feedback the most: a try that can compile and read the error fixes most mistakes itself.
Without it, compile errors become climbs and spend. It is also time: the gate's check is the slowest part of a C++
try. A cold plugin build is about 77 s, and the red-on-base pre-check does two (gap-e02ec4).

## Where

- `crates/roko-std/src/tool/builtin/run_tests.rs`: `tool_def` (the `build` enum) and its handler (the command per
  ecosystem).
- Dispatch knows each task's own `[[task.verify]]` commands (`graph_task_dispatch/verification.rs`), but no tool
  offers them to the agent mid-try.

## Plan

Two shapes, for the owner to choose:
1. **A tool that runs this task's own checks mid-try** (`run_checks`, say): it runs the task's verify steps as the
   gate does (the same env and timeout, through the round's shims), returns pass or fail and the output tail, and
   counts toward the try's limits. It is ecosystem-neutral, and the agent iterates on exactly what will judge it.
2. **Extend `run_tests`** with `cmake`/`ctest` (configure and build a preset, then `ctest --preset`) and a `command`
   mode limited to the task's checks.

Shape 1 also helps every other ecosystem, because a task's checks are more specific than its whole suite.

## Done when

An agent in a CMake project can run its task's check during a try without bash, and sees the compiler's errors.
Checked by the test in `[[verify]]` (a CMake fixture task whose agent calls the tool mid-try).

## Notes

- Owner: to be decided by the lead (2026-10-19).
- Related:
  - bug-d758f6 (planners strip bash);
  - gap-e02ec4: the red-on-base check builds C++ checks cold;
  - gap-6754ab: attempt worktrees build cold;
  - gap-6a319c: per-try timing telemetry.
