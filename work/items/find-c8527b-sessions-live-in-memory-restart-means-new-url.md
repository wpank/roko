+++
id = "find-c8527b"
kind = "finding"
title = "Sessions live in memory; restarting roko serve means opening the newly printed URL"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-serve/auth"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "plan:portal-programme/03c-backend-local-access#T10"
discovered_from = "plan:portal-programme/03c-backend-local-access#T10"
anchors = ["crates/roko-serve/src/state.rs::LocalAccess", "crates/roko-serve/src/routes/auth_session.rs", "crates/roko-cli/src/commands/server.rs:50"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn session_survives_restart' crates/roko-serve/ && cargo test -p roko-serve session_survives_restart"
+++

The `POST /api/auth/session` sessions are stored in memory in the server process.
When `roko serve` stops and restarts (crash, upgrade, SIGTERM), all active sessions
are lost. On the next start a fresh launch token is minted, and the portal's cookie
holds a stale session ID that returns 401.

The operator must open the newly printed URL (`portal: http://127.0.0.1:<port>/#token=<token>`)
to re-exchange for a valid session cookie.

This is an accepted design trade-off: sessions are intentionally ephemeral (no
persistent session store, no key-wrapping), matching the "local operator only"
trust model. The finding is recorded so operators are not surprised by the logout
behaviour after server restarts.

A future improvement could persist sessions to `.roko/runtime/sessions.json` (mode
0600) and reload them on startup, or accept the launch token directly as a
session for the first request, avoiding the explicit exchange step.

Re-checked 2026-09-29: unchanged. Sessions are held in LocalAccess (crates/roko-serve/src/state.rs:402), the exchange route is crates/roko-serve/src/routes/auth_session.rs (not routes/auth.rs), and the launch token is regenerated per start in crates/roko-cli/src/commands/server.rs:50.
