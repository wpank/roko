+++
id = "gap-fab2cc"
kind = "gap"
title = "Graph checkpoints record no stop cause, so a deadline or conductor stop shows only in the summary"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/graph_checkpoint"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-9efe8e"
anchors = ["crates/roko-cli/src/graph_checkpoint.rs", "crates/roko-cli/src/graph_execution/plan_runner.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-9efe8e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib checkpoint_records_the_stop_cause"
+++

## Problem

gap-9efe8e added a `Deadline` interrupt. Graph checkpoints have no stop-cause field, so `deadline` shows only in the run summary and the `--log-file`, and a conductor stop still requests Terminate. `commands/plan.rs:2452` and `do_cmd.rs:785` carry stale "exit 130/143" comments.

## Plan

Add a stop cause to the checkpoint (interrupt label), and fix the comments. Add a test named `checkpoint_records_the_stop_cause`.

## Done when

- `cargo test -p roko-cli --lib checkpoint_records_the_stop_cause` passes.

## Notes

- Reported on 2026-10-01 by wk-planrun, working on gap-9efe8e, during the evening close-out round.

2026-10-01 (wk-planrun): implemented on work/gap-dd4826; cargo verification deferred to the batch check.
Graph checkpoints now record a stop cause in a new optional extension, `roko.run.stop@1` (`graph_checkpoint::STOP_EXTENSION`), shaped `{"by": "SIGINT" | "SIGTERM" | "SIGHUP" | "deadline" | "conductor"}`, so the manifest struct is unchanged. `run_one_plan` calls `PreparedGraphCheckpoint::record_stop_cause` on both terminal paths. It writes the cause when the plan ended `Interrupted` by a stop request (`stop_cause`) and clears it otherwise, so a later run does not inherit it. A forced exit records its signal through `mark_running_checkpoint_interrupted(manifest, by)`. `canonical_stop_cause(workdir, plan_id)` reads it back. A conductor stop now requests the new `PlanRunInterrupt::Conductor` (label `conductor`, exit 143 as before; the run still fails with the watcher's error) instead of Terminate.
Comments: the "exit 130/143" comments in `commands/plan.rs` (cmd_plan_run_engine) and `commands/do_cmd.rs` (run_plan_execution) now name SIGHUP and 129. `run.rs` (run_prompt), the `stopped_by` comment in `run_graph_plan_body` and the `GraphCheckpointStatus::Interrupted` doc were fixed the same way.
Tests: `checkpoint_records_the_stop_cause` (graph_checkpoint.rs), `an_interrupted_plan_names_its_stop` (plan_runner.rs), and the stop-cause checks added to `a_forced_exit_marks_running_checkpoints_interrupted`, `a_running_checkpoint_is_marked_interrupted_and_a_finished_one_kept` and `interrupt_exit_codes_follow_shell_convention`.
