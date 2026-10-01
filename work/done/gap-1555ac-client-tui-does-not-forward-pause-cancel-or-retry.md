+++
id = "gap-1555ac"
kind = "gap"
title = "Client TUI does not forward pause, cancel or retry keys to the server"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["roko-cli/tui", "roko-serve/plans"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "plan:portal-programme/03b-backend-workspace-server#T16"
discovered_from = "plan:portal-programme/03b-backend-workspace-server#T16"
anchors = ["crates/roko-cli/src/serve_client.rs::follow_run_tui", "crates/roko-cli/src/tui/app/actions.rs", "crates/roko-cli/src/tui/app/modals.rs", "crates/roko-serve/src/routes/plans.rs::pause_plan"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'with_execution_command_sender' crates/roko-cli/src/serve_client.rs"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:33Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:55Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

When `roko plan run` submits a plan to a server and follows the run via the hub IPC mirror (`follow_run_tui`), the ratatui TUI is rendered from the hub stream. However, the TUI's keyboard bindings (pause `p`, cancel `c`, retry `r`) invoke the local `GraphExecutionController`, which writes to the local checkpoint and issues local signals. These signals are never forwarded to the server as `POST /api/plans/{id}/cancel` or equivalent API calls.

**Effect.** A user watching a server-owned run in the TUI cannot pause, cancel or retry the run from that TUI. Ctrl-C (SIGTERM) is forwarded — `run_plan_via_server` catches it and calls `cancel_plan_run` — but the in-TUI keys are silent.

**Fix path.** In `follow_run_tui`, intercept the pause/cancel/retry keyboard events and route them to `WorkspaceServerClient::cancel_plan_run` (and future `pause_plan_run` / `retry_plan_run` endpoints) instead of the local controller. The TUI's action handling for these keys is in `crates/roko-cli/src/tui/app.rs`.

Re-verified 2026-09-29: still open. The keys do not reach a local GraphExecutionController as the body says: follow_run_tui attaches no ExecutionCommandSender, so pause shows the warning 'Pause is available only during a connected plan run' and cancel/retry confirm actions do nothing. The key handling lives in crates/roko-cli/src/tui/app/actions.rs (TuiAction::TogglePause) and tui/app/modals.rs (confirm actions), not tui/app.rs. The server already has POST /api/plans/{id}/pause and /resume (roko-serve routes/plans.rs:30-31) besides cancel. A fix could attach a sender in follow_run_tui whose commands are forwarded to those endpoints.

## Notes

- 2026-10-01 (wk-childenv): implemented on work/gap-1555ac; cargo verification deferred to the batch check.
  `follow_run_tui` attaches a command channel (`server_command_bridge`, on the new generic
  `execution_control::spawn_command_bridge`): pause goes to `POST /api/plans/{run}/pause` (new
  `WorkspaceServerClient::pause_plan_run`), cancel to `/cancel`, each acknowledged in the TUI. The server has no
  resume, retry, repair, skip or approval endpoint for a run it owns, so those are rejected with that message;
  its pause stops the run at its checkpoint, so the TUI then ends and running the plan again resumes it.
