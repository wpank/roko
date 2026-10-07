+++
id = "gap-6533bf"
kind = "gap"
title = "Standalone roko plan run does not serve its hub, so roko dashboard beside it stays file-polled"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["roko-cli/state-hub", "roko-cli/tui"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "plan:portal-programme/03b-backend-workspace-server#T16"
discovered_from = "plan:portal-programme/03b-backend-workspace-server#T16"
anchors = ["crates/roko-cli/src/state_hub_ipc.rs::start_hub_ipc_server", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan", "crates/roko-cli/src/commands/plan.rs:565", "crates/roko-cli/src/tui/fs_watch.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq 'start_hub_ipc_server' crates/roko-cli/src/commands/plan.rs crates/roko-cli/src/graph_execution/"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:32Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:13:33Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

When `roko plan run` runs without a server present, it publishes plan and task events into an in-process `SharedStateHub` (created via `SharedStateHub::new_in_process()`). This hub is not exposed over `hub.sock`, so a concurrent `roko dashboard` in the same workspace cannot connect to it and falls back to polling the checkpoint file on disk (`crates/roko-cli/src/tui/fs_watch.rs`).

**Effect.** Live agent heartbeats, token usage and task-level events are invisible to `roko dashboard` during a standalone CLI run. The dashboard shows stale state until a checkpoint write completes.

**Fix path.** When `roko plan run` starts without a server, optionally bind `start_hub_ipc_server` on `.roko/runtime/hub.sock` so that `roko dashboard` and other clients can connect to the IPC mirror, receiving the same event stream that the server path delivers. `start_hub_ipc_server` is already implemented in `crates/roko-cli/src/state_hub_ipc.rs`; it has no production caller on the standalone run path (gap-7f5eb6 closed the server path; this gap covers the standalone path).

Re-verified 2026-09-29: still open. The standalone path uses the process-global shared_state_hub() rather than SharedStateHub::new_in_process(), and run_plan in plan_runner.rs does not exist: the entry point is run_graph_plan. The no-server branch is in commands/plan.rs after discover_workspace_server.

## Notes

- 2026-10-01 (wk-streams): implemented on work/gap-b35a57; cargo verification deferred to the batch check.
  Two halves were missing at BASE: a standalone run's hub was private (`shared_state_hub()` makes a new one per
  call), and `roko dashboard` tried the socket only when a server owns the workspace. Now `cmd_plan_run_engine`
  (`commands/plan.rs`) makes the run's hub, serves it with `start_hub_ipc_server` for the length of the run
  (best-effort; a bind failure is a warning), passes it to `run_graph_plan`, and afterwards waits until the socket
  and token files are removed; `commands/dashboard.rs::dashboard_hub` tries the socket whenever no hub was handed
  in. Test: `dashboard_follows_the_hub_a_standalone_run_serves` (bin unit test in `commands/dashboard.rs`).
