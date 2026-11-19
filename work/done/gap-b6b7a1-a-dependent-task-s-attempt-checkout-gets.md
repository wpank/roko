+++
id = "gap-b6b7a1"
kind = "gap"
title = "A dependent task's attempt checkout gets warm build trees from the person's checkout at the run's base, never from its dependency's accepted checkout: phase 4's T3, T4 and T6 took 200-214 s for their first plugin run_tests, later ones 10-35 s"
status = "done"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "f177be7e9"
source = "round-v7 run-analyst (tmp/hermes-roko-v7/findings/run-analyst.md), the lead's ask"
discovered_from = "agent:run-analyst (round v7)"
anchors = ["crates/roko-cli/src/graph_execution/workspaces.rs::acquire", "crates/roko-cli/src/orchestrator/worktree/build_trees.rs::give_build_trees"]
links = { depends_on = [], blocks = [], related = ["gap-6754ab", "gap-7c3e13"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_dependent_attempt_starts_from_its_dependency_s_build_trees' crates/roko-cli/src && cargo test -p roko-cli --lib a_dependent_attempt_starts_from_its_dependency_s_build_trees"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T16:35:23Z"
commit = "f177be7e9"
by = "roko-speed"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-26T15:04:10Z"
forced = false
evidence = "Measured first, $0: the fixture plan dep-bench in phase 4's shape (D1 adds a header every plugin translation unit includes plus a CONFIGURE_DEPENDS ui/*.cpp glob; D2 depends on D1; D3 and D4 on D2), the bench cmake shim logging each build's slot wait, time and objects (runs/b6b7a1-dep/builds.txt): each dependent's first build recompiled exactly its dependencies' changed translation units (D2, D3, D4: 3 objects in 4.6-5.4 s, D4 after a 6 s slot wait), its own change then 1. Built: ExecutionWorkspaceProvider::acquire_seeded, the dispatcher recording each task's accepted checkout and seeding a dependent's new checkout from the dependency accepted last. chain46 on main eab74f5b2 (with reg-d2deba): fmt and clippy clean (with roko-graph and roko-std), a_dependent_attempt_starts_from_its_dependency_s_build_trees and acquire_gives_the_checkout_warm_build_trees passed, lib suites green: roko-agent 2400, roko-cli 4017, roko-core 2048, roko-learn 1401, roko-runtime 213, roko-serve 1083."
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

## Measured (roko-speed, 2026-10-26, $0)

- **The bench:** fixture plan `dep-bench` (`tmp/hermes-roko-v7/roko-speed/fixture-src/plans/dep-bench`), in phase
  4's shape, on roko-main-e90161ec4, with `--worktree-per-task`, the person's checkout built first, and the agents
  running their own checks.
  - D1 adds a header every plugin translation unit includes, through PluginProcessor.h, and a CONFIGURE_DEPENDS
    glob of plugin/ui/*.cpp.
  - D2 depends on D1 and edits the editor. D3 and D4 both depend on D2, and each adds a ui/*.cpp.
  - The bench's cmake shim logged every build: slot wait, seconds, objects compiled. `bench/builds.py` puts each
    build down to its task.
- **Result** (`runs/b6b7a1-dep/builds.txt`):
  - Each dependent task's first build in its new checkout, before any edit of its own, recompiled exactly its
    dependencies' changed translation units: D2 3 objects in 4.6 s, D3 3 in 4.9 s, D4 3 in 5.4 s.
  - D4 also waited 6 s for the one cpp-proof slot behind D3.
  - Their own changes then compiled 1 object each.
- **For phase 4:** the cost scales with the translation units the dependencies' headers reach.
  - Phase 4's T1 and T2 changed engine.h, PluginProcessor.h and five ui headers, which most plugin translation
    units include.
  - So each of T3, T4 and T6 rebuilt all of them, three at once through one slot: 200–214 s, against 10–35 s for
    their later builds.
  - JUCE's own modules did not rebuild in the fixture: a CMakeLists change that only globs sources leaves them
    alone.

## What was built (roko-speed, 2026-10-26)

- `ExecutionWorkspaceProvider::acquire_seeded(attempt, seed)` (roko-graph) warms a new checkout's build trees from
  `seed` instead of the repository checkout. The default ignores `seed`. The worktree provider passes it to
  `give_build_trees` with the same relocation and safety rules.
- The dispatcher records each task's accepted checkout (`accept_attempt`). Accepted checkouts stay on disk for
  review until the plan is delivered.
- A task's new checkout is seeded from the dependency it names in `depends_on` that was accepted last and whose
  checkout is still there. Without one, the repository checkout serves, as before.
- With several dependencies, what the last one's tree lacks still rebuilds, as from the base, but less.
- Test: `a_dependent_attempt_starts_from_its_dependency_s_build_trees`. A's check builds a target in its tree. B,
  which depends on A, finds it in its own checkout, in a tree relocated to that checkout.

## Done when

- A dependent attempt's checkout gets its build trees from its dependency's accepted checkout. Its first build of
  an unchanged target compiles only what changed since that checkout.
- The `[[verify]]` test, `a_dependent_attempt_starts_from_its_dependency_s_build_trees`, passes.

## Notes

- Evidence: `tmp/hermes-roko-v7/findings/run-analyst.md`, section 3 ("Warm build trees") and candidate lever L6.
- Each try's first build and test call is in `run-analyst/data/tool_calls.csv`.
