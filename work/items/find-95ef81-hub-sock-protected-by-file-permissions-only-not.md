+++
id = "find-95ef81"
kind = "finding"
title = ".roko/runtime/hub.sock is protected by file permissions only, not by [serve.auth]"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
subsystem = ["roko-cli/state-hub", "roko-serve/auth"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "plan:portal-programme/03b-backend-workspace-server#T16"
discovered_from = "plan:portal-programme/03b-backend-workspace-server#T16"
anchors = ["crates/roko-cli/src/state_hub_ipc.rs::start_hub_ipc_server", "crates/roko-cli/src/state_hub_ipc.rs::handle_hub_connection", "crates/roko-serve/src/routes/middleware.rs::api_credential"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn hub_ipc_rejects_connection_without_token' crates/roko-cli/ && cargo test -p roko-cli --test hub_ipc hub_ipc_rejects_connection_without_token"
+++

`.roko/runtime/hub.sock` is a Unix domain socket bound by `start_hub_ipc_server` with mode `0600`, so only the owning user can connect to it. This is the sole access control: the socket carries a live stream of `DashboardEvent` frames (plan starts, task completions, agent output, cost/token events) with no authentication challenge.

**Threat model.** On a single-user workstation this is fine: mode `0600` is equivalent to the `[serve.auth]` API-key check because only the user's own processes can open the socket. On a multi-user host (a shared dev server or a CI runner with multiple users) it is not: any process running as the same uid, including other users' sudo-escalated processes on some configurations, can connect and read the full event stream.

**Contrast with the HTTP path.** `GET /api/events` on the HTTP server requires a valid `X-Api-Key` header or a session cookie when `[serve.auth] enabled = true` (the default).

**Fix path.** Add a one-way handshake on connect: the client sends the workspace API key or the launch token; the server closes the connection immediately on mismatch. Alternatively, document the gap and accept the current behaviour for loopback-only deployments. The decision depends on whether multi-user support is in scope.

Re-verified 2026-09-29: still as described. The socket is bound first and chmod 0600 is applied afterwards, so for a short window it has umask-default permissions. A handshake that uses a key readable from the workspace would not stop same-uid processes, which can already read that key. The decision between a handshake and documenting a loopback/single-user scope is still open. The serve-side auth check is in crates/roko-serve/src/routes/middleware.rs, not middleware/auth.rs.

## Notes

- 2026-09-30 (wk-serve-sec): Implemented on `work/bug-928add` at `b42ee33a1`; cargo verification deferred to the batch check. Decision taken: a token handshake, and the trust boundary documented. `start_hub_ipc_server` writes a random per-run token to `.roko/runtime/hub.token` (created `0600`, renamed into place) before it binds; a client whose first frame is missing, oversized (over 4 KiB), malformed or carries another token is dropped before any frame is sent. `try_connect_hub_ipc` reads the token itself, so its callers (`roko dashboard`, `run_plan_via_server`) are unchanged. This closes the bind-then-chmod window and platforms that ignore socket-file permissions. It does not separate processes of the same user or root, which can read the token file; the module docs of `state_hub_ipc.rs` say so, the same boundary as `.roko/runtime/serve.token`. Tests: `tests/hub_ipc.rs::hub_ipc_rejects_connection_without_token`, `hub_token_file_has_mode_0600_and_changes_per_run`.
