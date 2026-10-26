+++
id = "gap-b6b7a1"
kind = "gap"
title = "A dependent task's attempt checkout gets warm build trees from the person's checkout at the run's base, never from its dependency's accepted checkout: phase 4's T3, T4 and T6 took 200-214 s for their first plugin run_tests, later ones 10-35 s"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 run-analyst (tmp/hermes-roko-v7/findings/run-analyst.md), the lead's ask"
discovered_from = "agent:run-analyst (round v7)"
anchors = ["crates/roko-cli/src/graph_execution/workspaces.rs::acquire", "crates/roko-cli/src/orchestrator/worktree/build_trees.rs::give_build_trees"]
links = { depends_on = [], blocks = [], related = ["gap-6754ab", "gap-7c3e13"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_dependent_attempt_starts_from_its_dependency_s_build_trees' crates/roko-cli/src && cargo test -p roko-cli --lib a_dependent_attempt_starts_from_its_dependency_s_build_trees"
+++

## Problem

gap-6754ab gives each new attempt worktree warm CMake build trees. It clones them from the repository checkout
(`self.repo_root`): the person's checkout, at the run's base. A task that depends on earlier tasks starts on the plan
branch, with their changes in it. Its first build therefore recompiles everything they changed, in every dependent
worktree, often all at once.

In phase 4 (run c4fa953f), T1 and T2 changed `core/include/undertow/engine.h` and the plugin.
- T3, T4 and T6 started together after T2 passed. Their first plugin `run_tests` took 200, 210 and 214 s and ended
  22 s apart. Their later `run_tests` took 10–35 s.
- T4#1's 200 s was on the run's critical path.

Expected: a dependent task's checkout starts from the build trees of the newest accepted attempt it depends on,
relocated as gap-6754ab relocates the person's.

## Why it matters

Golden path, speed. Undertow's later phases depend on engine and editor changes from their first tasks, so 4.2 and 5
pay this on every dependent task. Their checks are plugin builds.

## Where

- `graph_execution/workspaces.rs::acquire`: `give_build_trees(&source, &checkout, …)` with `source =
  self.repo_root`.
- `orchestrator/worktree/build_trees.rs::give_build_trees`: the clone and relocate. It already refuses trees that
  name untracked paths, and syncs the mtimes of unchanged files.

## Current state

- gap-6754ab (merged 22667b513) cut phase 0's check time from 323 s to 53 s on the same plan.
- **The records can't split rebuild from waiting.** All three builds went through the round's one-slot cpp lane
  (heavy.sh). roko keeps only the last 1,024 bytes of a tool result, so the compile counts and heavy.sh's slot-wait
  lines are lost (see the telemetry gap's `cpu_ms` and `units_compiled`).
- **Measure first:** a $0 fixture with T1, then T2 ∥ T3, where T1 changes a header both include. Compare the first
  checks in T2 and T3 against T1's.
- gap-7c3e13 is the cargo version of seeding: target dirs, from the workdir.

## Plan

1. When acquiring a worktree for a task, look for its dependencies' accepted attempt checkouts that are still on
   disk. A run keeps them until it ends. Take the one whose accepted commit is the newest ancestor of the task's start
   commit, and use its build trees as the source.
2. Clone and relocate them with `give_build_trees`, with the same safety rules. Fall back to the person's checkout.
3. With several dependencies, only one tree can seed the checkout. Pick the one whose changes touch the most build
   inputs. The others' changes still rebuild, but less than from the base.

## Done when

- A dependent attempt's checkout gets its build trees from its dependency's accepted checkout. Its first build of
  an unchanged target compiles only what changed since that checkout.
- The `[[verify]]` test, `a_dependent_attempt_starts_from_its_dependency_s_build_trees`, passes.

## Notes

- Evidence: `tmp/hermes-roko-v7/findings/run-analyst.md`, section 3 ("Warm build trees") and candidate lever L6.
- Each try's first build and test call is in `run-analyst/data/tool_calls.csv`.
