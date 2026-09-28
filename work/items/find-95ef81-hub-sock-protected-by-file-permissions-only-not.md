+++
id = "find-95ef81"
kind = "finding"
title = ".roko/runtime/hub.sock is protected by file permissions only, not by [serve.auth]"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-cli/state-hub", "roko-serve/auth"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "plan:portal-programme/03b-backend-workspace-server#T16"
discovered_from = "plan:portal-programme/03b-backend-workspace-server#T16"
anchors = ["crates/roko-cli/src/state_hub_ipc.rs::start_hub_ipc_server", "crates/roko-serve/src/middleware/auth.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -n "mode\|0600\|permissions\|auth\|api_key" crates/roko-cli/src/state_hub_ipc.rs | head -10'
+++

`.roko/runtime/hub.sock` is a Unix domain socket bound by `start_hub_ipc_server` with mode `0600`, so only the owning user can connect to it. This is the sole access control: the socket carries a live stream of `DashboardEvent` frames (plan starts, task completions, agent output, cost/token events) with no authentication challenge.

**Threat model.** On a single-user workstation this is fine: mode `0600` is equivalent to the `[serve.auth]` API-key check because only the user's own processes can open the socket. On a multi-user host (a shared dev server or a CI runner with multiple users) it is not: any process running as the same uid, including other users' sudo-escalated processes on some configurations, can connect and read the full event stream.

**Contrast with the HTTP path.** `GET /api/events` on the HTTP server requires a valid `X-Api-Key` header or a session cookie when `[serve.auth] enabled = true` (the default).

**Fix path.** Add a one-way handshake on connect: the client sends the workspace API key or the launch token; the server closes the connection immediately on mismatch. Alternatively, document the gap and accept the current behaviour for loopback-only deployments. The decision depends on whether multi-user support is in scope.
