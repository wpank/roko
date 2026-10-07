+++
id = "find-c8527b"
kind = "finding"
title = "Sessions live in memory; restarting roko serve means opening the newly printed URL"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-serve/auth"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
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

## Notes

- 2026-10-01 (wk-serve2): blocked on a decision; no code changed. Still true at BASE `ebdc0f5d5`: `LocalAccess` keeps
  only SHA-256 hashes of session ids in memory (`crates/roko-serve/src/state.rs:403`), sessions never expire, the
  cookie has no `Max-Age` (`routes/auth_session.rs:62`), and `run_server` installs a fresh `LocalAccess` with the
  newly minted launch token (`crates/roko-serve/src/lib.rs:1179`). Persisting sessions would end the property that
  restarting `roko serve` revokes every session, which this finding records as an accepted trade-off, so it needs
  Will: (a) should sessions survive a restart, (b) opt-in (`[serve.auth]` flag) or default, (c) what lifetime.
  Next step if yes: `LocalAccess::with_store(launch_token, path)` that loads and saves `{hash, created_at}` records
  in `.roko/runtime/sessions.json` (mode 0600), drops expired ones, persists `end_session`, and is installed at
  `lib.rs:1179`; tests `session_survives_restart` plus one proving the default still drops sessions on restart.
- 2026-10-04 (coordinator): the decision this item waits on now also covers showcase mode. PK83's task 9322
  (`gap-4119fb`, in progress — new session records carrying scope, absolute and idle TTLs, and a passphrase
  generation) is in-memory only today, same as the rest of `LocalAccess`. 9322's own file says: "Land after
  find-c8527b and persist the new fields in its file (`.roko/showcase/sessions.json` in showcase mode, only
  `sha256(sid)` stored)." So whatever Will decides here (persist or not, opt-in or default, lifetime) should
  also settle whether showcase mode's richer session records persist across a restart, not just the base
  session-id hash this item already describes.
