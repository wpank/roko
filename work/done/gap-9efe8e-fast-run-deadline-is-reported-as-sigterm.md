+++
id = "gap-9efe8e"
kind = "gap"
title = "FAST run deadline is reported as SIGTERM (exit 143) with no deadline reason"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:w2-migrate"
anchors = ["crates/roko-cli/src/graph_execution/fast_lane.rs::arm_plan_deadline", "crates/roko-cli/src/graph_execution/plan_runner.rs::PlanRunInterrupt"]
links = { depends_on = [], blocks = [], related = ["gap-4a6dcb"], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'sed -n "/pub enum PlanRunInterrupt/,/^}/p" crates/roko-cli/src/graph_execution/plan_runner.rs | grep -q Deadline'

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:45Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:57Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

When `ROKO_FAST_PLAN_DEADLINE_SECS` elapses, `fast_lane::arm_plan_deadline` fires the run's interrupt handle: the checkpoint is finalized as `interrupted` and the run exits 143, exactly as on SIGTERM (fast_lane.rs:8). `PlanRunInterrupt` has only `Interrupt` and `Terminate`, so the evidence bundle, the checkpoint and `run.completed` cannot say that the deadline, not an operator, stopped the run.

Fix: add a `Deadline` interrupt variant with its own outcome label, keeping exit 143 if dev.sh relies on it.

## Notes

2026-10-01 (wk-planrun): implemented on work/gap-dd4826; cargo verification deferred to the batch check.
`PlanRunInterrupt::Deadline` (label `deadline`, exit 143 like SIGTERM, which `./dev.sh fast` and the evidence wrapper expect) is what `fast_lane`'s timer now requests; the timer moved into `arm_deadline` so a test can arm it without the environment. The run summary's `interrupted_by`, the "Graph Engine interrupted by ..." line and the stop logs say `deadline`. `RunEventLog::with_interrupt` gives the `--log-file` recorder the run's stop handle (`event_log::run_recorded` passes it), and `run.completed` gains `interrupted_by` when its run was cancelled by a stop request. Tests: `an_elapsed_deadline_stops_the_run_as_a_deadline` (fast_lane.rs), `run_completed_names_the_stop_that_ended_the_run` (event_log.rs), and `interrupt_exit_codes_follow_shell_convention`.
Left as they were: the checkpoint status stays `interrupted` (the manifest has no stop-cause field; adding one is a schema change in `graph_checkpoint.rs`), and `RunCompleted`/`status.json` still say `cancelled`. The conductor's stop (`run_graph_plan_body`, the conductor ticker callback) still requests `Terminate`.
