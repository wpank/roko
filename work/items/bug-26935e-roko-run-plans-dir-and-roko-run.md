+++
id = "bug-26935e"
kind = "bug"
title = "roko run plans/<dir> and roko run --plan leave the operator checkout as they found it, without rebuilding plans/INDEX.md after the run"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/run"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-3"
anchors = ["crates/roko-cli/src/commands/run_cmd.rs::run_plan_dir", "crates/roko-cli/src/commands/run_cmd.rs::run_prompt_plan", "crates/roko-cli/src/main.rs::finish_with_index_rebuild", "crates/roko-cli/tests/operator_checkout_clean.rs::plan_run_leaves_operator_checkout_clean"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-4b890c", "gap-af73a8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn run_plan_dir_leaves_operator_checkout_clean' crates/roko-cli/tests/ && cargo test -p roko-cli --test operator_checkout_clean run_plan_dir_leaves_operator_checkout_clean"

[[verify]]
command = "test \"$(grep -c 'index::rebuild_all' crates/roko-cli/src/commands/run_cmd.rs)\" -le 1"
+++

## Problem

`roko run plans/<dir>` and `roko run --plan "<prompt>"` rebuild the plan indexes after a run that succeeds, so they
rewrite the tracked `plans/INDEX.md` (and the `.roko` indexes) in the operator's checkout. `roko plan run` stopped
doing this in `6a229ef51` (2026-10-03, task 1222 of PK06 / gap-4b890c), but that fix changed only `main.rs`; the two
calls in `roko run` came from `0debf4769` (2026-10-02) and are still there:

- `crates/roko-cli/src/commands/run_cmd.rs::run_plan_dir`, `:447-451`: "`roko plan run` rebuilds the plan indexes
  after a run that succeeds (main.rs wraps it); this is the same run, so it does too." The comment is false since
  `6a229ef51`: `PlanCmd::should_rebuild_indexes` returns false for `Run` (`crates/roko-cli/src/main.rs:1333-1342`).
- `run_cmd.rs::run_prompt_plan`, `:653-655`: "As after `roko plan run`, the indexes show the run's result."

Expected: a plan run, however it is started, leaves the checkout as it found it; its results go to the plan branch
and `.roko/`.

## Why it matters

`roko run plans/<dir>` is step 4 of CLAUDE.md's self-hosting workflow, so dogfood defect D13 (a plan run writes
`plans/INDEX.md` into the operator checkout) still recurs on the documented path. A dirty tracked file blocks the
next merge into the checkout and gets swept into unrelated commits. The canary
`plan_run_leaves_operator_checkout_clean` (`crates/roko-cli/tests/operator_checkout_clean.rs`) covers `roko plan run`
only, so it passes.

## Where

- `crates/roko-cli/src/commands/run_cmd.rs::run_plan_dir` (`roko run plans/<dir>`) and `run_prompt_plan`
  (`roko run --plan`, and `roko run "<prompt>"` when sizing picks a plan).
- `crates/roko-cli/src/main.rs::finish_with_index_rebuild` and `PlanCmd::should_rebuild_indexes`: how `roko plan` decides.
- `crates/roko-cli/tests/operator_checkout_clean.rs`: the canary; `crates/roko-cli/tests/common/mod.rs::run_plan`
  runs `roko --json plan run plans/<plan> --workdir <repo>`.

## Current state

Three `rebuild_all` calls in `run_cmd.rs`: `:450` (after `run_plan_dir`), `:610` (after `--plan` writes a plan, as
`roko plan generate` does) and `:654` (after `--plan` runs it). Only `:610` is wanted: generation itself adds a plan
directory, so listing it in the index is part of that change.

## Plan

1. Delete the post-run rebuilds at `run_cmd.rs:449-451` and `:653-655`, with their comments; keep `:610`.
2. In `operator_checkout_clean.rs`, add `run_plan_dir_leaves_operator_checkout_clean`: the same fixture and
   assertions, but started as `roko --json run plans/clean-checkout --workdir <repo>` (add a sibling of
   `ScriptedPlanWorkspace::run_plan` in `tests/common/mod.rs` that takes the subcommand, or a `run_args` helper).
3. Optionally cover `roko run --plan` the same way if the scripted provider can answer the planner call; otherwise
   the static check below covers its call site.

## Done when

Both `[[verify]]` commands pass: `run_plan_dir_leaves_operator_checkout_clean` passes (the checkout's `git status`
shows nothing outside `.roko/` and `plans/INDEX.md` is not written), and `run_cmd.rs` has at most one
`index::rebuild_all` call (the one after generation).

## Notes

Found by A01 (`tmp/status-quo/2026-10-15/backlog/A01-11-12-safe-runs.md:85`, D13 row at `:163`); re-checked at
b42578bab by agent F1. The `operator_checkout_clean` tests start the real `roko` binary with a scripted provider, so
they cost nothing.
