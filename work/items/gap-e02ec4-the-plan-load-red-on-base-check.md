+++
id = "gap-e02ec4"
kind = "gap"
title = "The plan-load red-on-base check builds every C++ check cold, twice per task, and repeats identical checks: 171 s of a 283 s run on a 4-task JUCE plan, before any agent starts"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "M"
subsystem = ["roko-cli/spec-gate"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "round-v7-roko-speed#baseline"
discovered_from = "agent:roko-speed (round v7 baseline, 2026-10-19)"
anchors = ["crates/roko-cli/src/spec_red_on_base.rs::BaseTrees", "crates/roko-cli/src/spec_red_on_base.rs::check_task"]
links = { depends_on = [], blocks = [], related = ["gap-0ee70b", "gap-d21da9", "gap-7c3e13"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn warm_keeps_build_trees_between_runs' crates/roko-cli/src && cargo test -p roko-cli --lib spec_red_on_base::tests::warm_"
+++

## Problem

Before any agent starts, every `plan run` runs the plan-load spec gate's red-on-base check (`[spec_quality] red_on_base`,
on by default): each implementer task's verify steps run twice on a clean checkout of the base. `plan validate
--spec-quality --dynamic` runs the same check. Between runs, and between tasks, the base checkout is reset with
`git clean -ffdx`, which deletes the project's build directory. So on a C++ project every run of every step
configures and builds from nothing.

Measured on round v7's JUCE fixture (`tmp/hermes-roko-v7/roko-speed`, roko 3b34aed92, stand-in model, 4 tasks: 2
plugin tasks, 2 core-library tasks), one `plan run` took 282.5 s wall. **170.7 s (60 %) of it was this pre-check**:

- each plugin task's step is a cold JUCE configure (14-21 s, mostly building juceaide) plus a cold build (22-36 s at
  -j4), and it runs twice;
- the same step warm takes 3-4 s;
- the agents themselves then ran for under a second each.

A plan with 6 plugin tasks would wait about 10 minutes before its first agent. `validate --dynamic` before the run
pays the whole cost again, and so does every re-validation after a spec fix, even for tasks whose checks did not
change.

## Why it matters

Round v7 builds a VST3 synth (Metakosmia Undertow) from Hermes, as many small C++ tasks under per-run caps. Plan
iteration (write the plan, then `validate --dynamic`, fix the spec, validate again, run) is the person's loop. Today
every step of that loop costs minutes of builds that a warm tree or an earlier identical result already answered.
The person waits through it with nothing on screen (telemetry is a separate item).

## Where

- `crates/roko-cli/src/spec_red_on_base.rs::BaseTrees`, `reset`: `git clean -ffdxq` between runs.
- `crates/roko-cli/src/spec_red_on_base.rs::check_task`, `run_steps`: two runs per task, no reuse of an earlier
  identical check.
- Entry points: `spec_gate::gate_plans`, called from `graph_execution/plan_runner.rs` before dispatch, and
  `commands/plan.rs` (`plan validate --spec-quality --dynamic`).

## Current state

Cargo steps already keep their build across resets: `CARGO_TARGET_DIR` points inside the scratch dir. They are
skipped by default anyway (gap-0ee70b). Nothing keeps a CMake (or other) build tree. Nothing caches a result.

## Plan

1. Keep build trees between resets. Before `git clean`, list the base checkout's git-ignored directories. Keep
   (`git clean -e /<dir>/`) each one that holds a CMake build tree (`CMakeCache.txt` at its top or one or two
   levels down). The second run of a task, and later tasks on the same base, then build incrementally. A step that
   timed out still ends the check after run 1, so a slow cold run can't turn into "flaky".
2. Cache decided results (`fail` and `pass` only) in `.roko/cache/red-on-base/`, keyed by a digest of:
   - the base sha;
   - the task's steps (command, timeout, expect; pinned tests carry their sha256);
   - the cargo mode;
   - the plan directory's files other than `tasks.toml`;
   - roko's version.

   An entry expires after 12 hours. A cached result says so (`cached`), and the text summary counts them.
   `plan validate --dynamic --fresh` ignores the cache. `--fixture` never uses it.
3. Measure the same 4-task run before and after.

## Done when

- A task whose step leaves a file in an ignored CMake build tree sees it again in run 2.
- A second check of an unchanged task runs nothing, and reports the first check's result as cached.
- `cargo test -p roko-cli --lib spec_red_on_base::tests::warm_` passes.
- The fixture run's pre-check drops from 170.7 s, with the before/after numbers recorded in the closing evidence.

## Notes

- Keep the check's safety rules: nothing runs in the person's checkout, and everything the check created is removed.
  Cache files are the only new thing written, under `.roko/cache/`.
- Seeding the base checkout's build tree from the person's own build is a separate item: the warm build dirs item
  (clone plus path relocation).
- Round v7 evidence: `tmp/hermes-roko-v7/findings/roko-speed.md`.
