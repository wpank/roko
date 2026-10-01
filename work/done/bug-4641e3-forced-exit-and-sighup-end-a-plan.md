+++
id = "bug-4641e3"
kind = "bug"
title = "Forced exit and SIGHUP end a plan run without finalizing its checkpoint"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-cli/graph_execution", "roko-cli/tui"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e2-planset"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::force_exit", "crates/roko-cli/src/tui/app/mod.rs::install_terminal_signal_cleanup"]
links = { depends_on = [], blocks = [], related = ["bug-5c2d01"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "sed -n '/^fn force_exit/,/^}/p' crates/roko-cli/src/graph_execution/plan_runner.rs | grep -q 'interrupted' && grep -q 'SignalKind::hangup' crates/roko-cli/src/graph_execution/plan_runner.rs"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:14Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:57Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

bug-5c2d01 fixed the graceful path: the first SIGINT/SIGTERM finalizes the in-flight plan's checkpoint as `interrupted`. The fallbacks do not. `force_exit` (after a second signal, or when the 10 s graceful deadline elapses) restores the terminal and calls `process::exit`, logging that "the interrupted plan's checkpoint may still read `running`". SIGHUP is still claimed by the TUI's reset-and-reraise handler (`install_terminal_signal_cleanup`), so hanging up the terminal kills the run without finalizing it either. Resume still works, but anything that reads the checkpoint sees a run that looks alive.

Fix: write the `interrupted` status (best effort, bounded) before a forced exit, and route SIGHUP through the plan-run interrupt handler.

## Notes

2026-10-01 (wk-planrun): implemented on work/gap-dd4826; cargo verification deferred to the batch check.
Forced exit: `run_one_plan` lists its checkpoint manifest (`RunningPlanCheckpoint`) until it returns, and `force_exit` marks every listed checkpoint that still reads `running` as `interrupted` through the new `graph_checkpoint::mark_running_checkpoint_interrupted`, on a helper thread bounded by `FORCED_EXIT_CHECKPOINT_TIMEOUT` (2 s). SIGHUP: `install_plan_run_signal_handlers` claims it unless it is ignored (as under `nohup`) and records it as the new `PlanRunInterrupt::Hangup` (exit 129, cancelled in `run.completed`); the TUI leaves SIGHUP to the run through the new `App::with_host_hangup_signal`. Tests: `a_forced_exit_marks_running_checkpoints_interrupted`, `a_hangup_stops_the_plan_run` (child process, also checks the nohup case), `a_running_checkpoint_is_marked_interrupted_and_a_finished_one_kept`, and the extended `interrupt_exit_codes_follow_shell_convention` and `interrupted_run_is_cancelled`.
Comments left stale on purpose, outside this packet's files: `commands/plan.rs:2452` and `commands/do_cmd.rs:785` say "exit 130/143", and `GraphCheckpointStatus::Interrupted` says "SIGINT/SIGTERM".
