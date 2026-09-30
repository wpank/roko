+++
id = "bug-ccc7c4"
kind = "bug"
title = "roko run now writes a second run directory under .roko/runs beside its own"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["cli"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (20:00, wk-attempt-ctx's report on gap-96f7ed)"
anchors = ["crates/roko-cli/src/commands/do_cmd.rs", "crates/roko-cli/src/graph_task_dispatch/attempt.rs"]
lane = "rust-cold"
parent = "spec-b7303f"
links = { depends_on = ["gap-96f7ed"], blocks = [], related = ["gap-96f7ed"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn roko_run_uses_one_run_directory' crates/roko-cli/src/ && cargo test -p roko-cli --lib roko_run_uses_one_run_directory"
+++

## Problem

After gap-96f7ed, each attempt writes its open line to `.roko/runs/<checkpoint run>/attempts.jsonl`. `roko run` already creates its own `.roko/runs/run-<ts>/` directory, so it now leaves a second one, `graph-<plan>-<uuid>/`, beside it (wk-attempt-ctx, 2026-09-29).

## Why it matters

One run should have one directory. Evidence bundles and the run index read it. Epic spec-b7303f.

## Where

`roko run`'s run-directory setup (`commands/do_cmd.rs`) and the attempt log path (`graph_task_dispatch/attempt.rs`).

## Current state

Two directories per `roko run`.

## Plan

1. Pass `roko run`'s run directory, or its run id, into the Graph run, so attempts land in the same place.
2. Add the test the verify names.

## Done when

- [ ] `roko run` writes one run directory.
- [ ] The `[[verify]]` command passes.

## Notes

- Implemented on `work/gap-8cb382` at `c501bb8ac`; cargo verification deferred to the batch check. On the branch, `roko_run_uses_one_run_directory` passes.
- The run directory is created in `run.rs::run_prompt` (the library, called by `roko run` and `roko do`'s simple path), not in `commands/do_cmd.rs`. The Graph checkpoint now takes the caller's run id (`graph_checkpoint::prepare_graph_checkpoint_for_run`, `plan_runner::run_graph_plan_in_run`), so `attempt.rs` needed no change.
