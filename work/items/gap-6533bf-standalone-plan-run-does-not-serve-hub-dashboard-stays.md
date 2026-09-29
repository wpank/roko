+++
id = "gap-6533bf"
kind = "gap"
title = "Standalone roko plan run does not serve its hub, so roko dashboard beside it stays file-polled"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["roko-cli/state-hub", "roko-cli/tui"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "plan:portal-programme/03b-backend-workspace-server#T16"
discovered_from = "plan:portal-programme/03b-backend-workspace-server#T16"
anchors = ["crates/roko-cli/src/state_hub_ipc.rs::start_hub_ipc_server", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan", "crates/roko-cli/src/commands/plan.rs:565", "crates/roko-cli/src/tui/fs_watch.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq 'start_hub_ipc_server' crates/roko-cli/src/commands/plan.rs crates/roko-cli/src/graph_execution/"
+++

When `roko plan run` runs without a server present, it publishes plan and task events into an in-process `SharedStateHub` (created via `SharedStateHub::new_in_process()`). This hub is not exposed over `hub.sock`, so a concurrent `roko dashboard` in the same workspace cannot connect to it and falls back to polling the checkpoint file on disk (`crates/roko-cli/src/tui/fs_watch.rs`).

**Effect.** Live agent heartbeats, token usage and task-level events are invisible to `roko dashboard` during a standalone CLI run. The dashboard shows stale state until a checkpoint write completes.

**Fix path.** When `roko plan run` starts without a server, optionally bind `start_hub_ipc_server` on `.roko/runtime/hub.sock` so that `roko dashboard` and other clients can connect to the IPC mirror, receiving the same event stream that the server path delivers. `start_hub_ipc_server` is already implemented in `crates/roko-cli/src/state_hub_ipc.rs`; it has no production caller on the standalone run path (gap-7f5eb6 closed the server path; this gap covers the standalone path).

Re-verified 2026-09-29: still open. The standalone path uses the process-global shared_state_hub() rather than SharedStateHub::new_in_process(), and run_plan in plan_runner.rs does not exist: the entry point is run_graph_plan. The no-server branch is in commands/plan.rs after discover_workspace_server.
