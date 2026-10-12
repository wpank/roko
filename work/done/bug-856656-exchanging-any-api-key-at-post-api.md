+++
id = "bug-856656"
kind = "bug"
title = "Exchanging any API key at POST /api/auth/session mints an admin session, so a write or read key gains admin rights through its cookie"
status = "done"
triage = "verified"
severity = "p1"
goal = "release"
size = "S"
subsystem = ["roko-serve/auth"]
created = 2026-10-12
updated = 2026-10-12
last_verified = 2026-10-12
last_verified_rev = "b1cdade95"
source = "hermes-plugin-roko 0.2.0 UI work, roko serve probe 2026-10-11 (scratchpad roko-ui-data.md)"
anchors = ["crates/roko-serve/src/routes/auth_session.rs::create_session", "crates/roko-serve/src/routes/auth_session.rs::validate_token", "crates/roko-serve/src/state.rs::SessionGrant"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn session_from_a_write_key_is_scoped_to_write' crates/roko-serve/ && cargo test -p roko-serve session_from_a_write_key_is_scoped_to_write"

[closed]
at = 2026-10-12
at_ts = "2026-10-12T11:22:25Z"
commit = "b1cdade95"
by = "commit trailer"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-12T11:02:43Z"
forced = false
evidence = "b1cdade95: validate_token returns the matched credential's scope and create_session mints SessionGrant::exchange(scope): a named key's session has the key's scope, the launch token and legacy key stay admin. Verify passes in the worktree (session_from_a_write_key_is_scoped_to_write: write and read keys get write/read, the launch token admin; plus session_from_the_legacy_key_stays_admin); all 145 auth/session tests in roko-serve pass; clippy -p roko-serve --no-deps -D warnings clean."
+++

## Problem

`POST /api/auth/session` exchanges a credential for a `roko_session` cookie, and every exchange mints an
`admin` session, whatever the credential's own scope. A `write` (or `read`) API key that is refused an admin
route directly gets through it with the cookie minted from the same key.

Observed on 2026-10-11 against roko serve `856ed7570`: with a `write`-scoped named key, `POST /api/neuro/query`
answered `403 {"permission":"config:edit","role":"member"}`. After `POST /api/auth/session` with that key, the same
request with the cookie answered 200 (`x-auth-method: session`), and `GET /api/auth/session` reported
`"scopes":["admin"]`.

## Why it matters

A scoped key exists to limit what its holder can do. Chat hosts get `write` keys (hermes-plugin-roko mints one
named `hermes`), and anything that can read such a key can turn it into admin rights with one request. This is
a privilege escalation in the auth layer, so it belongs to the release goal's security work.

## Where

- `crates/roko-serve/src/routes/auth_session.rs::create_session` mints the session with
  `state.local_access.create_session()`, which always uses `SessionGrant::admin()`.
- `crates/roko-serve/src/routes/auth_session.rs::validate_token` already knows which credential matched (launch
  token, named key, legacy key) but returns only an actor label.
- `crates/roko-serve/src/state.rs::SessionGrant` and `create_scoped_session` already support any scope, and
  `routes/middleware.rs::session_credential` already applies the session's recorded scope (9322).

## Current state

Scoped sessions exist for showcase logins. The credential exchange was left on the `admin` grant from before
scoped sessions existed.

## Plan

1. Make `validate_token` return the matched credential's scope too: the launch token is `admin`, a named key is
   its `entry.scope`, the legacy single `api_key` keeps whatever scope `require_api_key` gives it.
2. Mint the session with that scope through `create_scoped_session`, keeping the exchange's other properties
   (no TTL, no generation, no cap).
3. Test that a session minted from a `write` key reports `write` and is refused an admin-only route, and that a
   launch-token session is still `admin`.

## Done when

- A session minted from a key carries that key's scope; `GET /api/auth/session` reports it.
- The `[[verify]]` test passes.

## Notes

Do not change showcase passphrase logins or the launch-token exchange the portal's `#token=` bootstrap uses.
