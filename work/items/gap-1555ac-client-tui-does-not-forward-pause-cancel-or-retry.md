+++
id = "gap-1555ac"
kind = "gap"
title = "Client TUI does not forward pause, cancel or retry keys to the server"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-cli/tui", "roko-serve/plans"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "plan:portal-programme/03b-backend-workspace-server#T16"
discovered_from = "plan:portal-programme/03b-backend-workspace-server#T16"
anchors = ["crates/roko-cli/src/serve_client.rs::follow_run_tui", "crates/roko-cli/src/tui/app.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -n "cancel_plan_run\|POST.*cancel\|key.*Cancel\|Pause\|Retry" crates/roko-cli/src/serve_client.rs | head -10'
+++

When `roko plan run` submits a plan to a server and follows the run via the hub IPC mirror (`follow_run_tui`), the ratatui TUI is rendered from the hub stream. However, the TUI's keyboard bindings (pause `p`, cancel `c`, retry `r`) invoke the local `GraphExecutionController`, which writes to the local checkpoint and issues local signals. These signals are never forwarded to the server as `POST /api/plans/{id}/cancel` or equivalent API calls.

**Effect.** A user watching a server-owned run in the TUI cannot pause, cancel or retry the run from that TUI. Ctrl-C (SIGTERM) is forwarded — `run_plan_via_server` catches it and calls `cancel_plan_run` — but the in-TUI keys are silent.

**Fix path.** In `follow_run_tui`, intercept the pause/cancel/retry keyboard events and route them to `WorkspaceServerClient::cancel_plan_run` (and future `pause_plan_run` / `retry_plan_run` endpoints) instead of the local controller. The TUI's action handling for these keys is in `crates/roko-cli/src/tui/app.rs`.
