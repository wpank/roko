+++
id = "gap-69a56e"
kind = "gap"
title = "The gate profile hints quality_profile and test_invariants are parsed but don't choose gate rungs"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/task_parser", "roko-cli/runner/gate_dispatch"]
created = 2026-09-30
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-taskdef's report, checked on work/gap-0f3980 at b27c02717)"
anchors = ["crates/roko-cli/src/task_parser.rs::TaskDef", "crates/roko-cli/src/runner/gate_dispatch.rs"]
lane = "rust-cold"
links = { depends_on = ["gap-0f3980"], blocks = [], related = ["gap-0f3980", "gap-3506f1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn quality_profile_and_test_invariants_select_gate_rungs' crates/roko-cli/src/ && cargo test -p roko-cli --lib quality_profile_and_test_invariants_select_gate_rungs"
+++

## Problem

gap-0f3980's branch parses the TaskDef gate-profile hints `quality_profile` and `test_invariants`. Nothing in gate dispatch (`runner/gate_dispatch.rs`) or Graph dispatch reads them, so they don't choose which gate rungs run for the task.

## Why it matters

The planner can say how strictly a task should be checked, but the gates ignore it. Plans get the same gates whatever they ask for. p3.

## Where

The two fields on `TaskDef`, and rung selection in gate dispatch (and in the Graph path's workspace rungs, gap-3506f1).

## Plan

1. Map `quality_profile` to a rung selection, and `test_invariants` to required test steps, in the Graph verify path. Or drop the fields from the parser if they won't be used.
2. Add `quality_profile_and_test_invariants_select_gate_rungs`.

## Done when

- [ ] The hints change which rungs run, or they are gone.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on gap-0f3980's branch.
- 2026-10-01 (wk-gates): the premise holds at BASE. gap-0f3980 is merged and both hints parse. `test_invariants`
  reaches only the prompt's `## Specification` section, and `quality_profile` is listed by `TaskDef::unused_hints`
  (PLAN_039).
- 2026-10-01 (wk-gates): implemented on work/bug-951930; cargo verification deferred to the batch check.
  Graph verify now picks workspace rungs per task with `task_runs_rung`, through `GraphTaskDispatcher::task_rungs`,
  formerly `plan_rungs`, which `red_flags.rs` also calls. Every required rung runs, as before. A
  `quality_profile = "hardened"` task also runs the optional (`required = false`) `[[gates.rungs]]`, and a task that
  names `test_invariants` also runs the optional rungs that run tests. Nothing ever runs fewer checks than before.
  `quality_profile` left `unused_hints`, and the PLAN_039 test in `plan_validate.rs` follows. Test:
  `quality_profile_and_test_invariants_select_gate_rungs`.
