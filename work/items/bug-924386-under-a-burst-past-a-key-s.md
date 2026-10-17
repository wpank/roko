+++
id = "bug-924386"
kind = "bug"
title = "Under a burst past a key's rate limit, roko serve resets connections instead of answering 429, and it rate-limits /health"
status = "open"
triage = "unverified"
severity = "p3"
goal = "hermes"
size = "S"
subsystem = ["roko-serve/routes"]
created = 2026-10-17
updated = 2026-10-17
source = "hermes-roko round v6, plugin-setup, 2026-10-16: the Hermes desktop page flashed OFFLINE right after a run start (tmp/hermes-roko-v6/findings/plugin-setup.md)"
anchors = ["crates/roko-serve/src/routes/mod.rs::keyed_rate_limit_middleware", "crates/roko-serve/src/routes/mod.rs::rate_limit_middleware", "crates/roko-serve/src/routes/mod.rs::top_level_health"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn rate_limited_requests_leave_their_connection_usable' crates/roko-serve && cargo test -p roko-serve --lib rate_limited_requests_leave_their_connection_usable"

[[verify]]
command = "grep -rqw 'fn health_answers_past_the_key_rate_limit' crates/roko-serve && cargo test -p roko-serve --lib health_answers_past_the_key_rate_limit"
+++

## Problem

A client that sends one key's requests in a concurrent burst past roko serve's per-key limit
(`[server] rate_limit_per_key_per_sec`, 30 by default) runs into two things:

1. Some of its connections are torn down instead of answered. The client sees `ECONNRESET` ("Connection reset by
   peer"), `EBADF`, or "Server disconnected without sending a response" on requests that never got an answer.
2. `GET /health`, the public liveness route, counts against the key's limit. So a client's own look at whether roko
   is up gets 429 during its bursts.

Seen on 2026-10-16 in hermes-roko round v6. Right after a run started from the Hermes desktop page, the Hermes plugin
sent roko serve 45 requests in one second. Five got 429, and one GET came back with its connection reset. In 2 of 3
recorded takes the page's look at /health was among such failures, so the page flashed OFFLINE ("Hermes cannot reach
roko serve") for about 6 s. All that time roko answered everything else, /health within 0.05 s.

Repro: roko serve 47b3cdd20 with auth on and one key.
- 8–16 Python threads each send 12 requests through one keep-alive httpx client, alternating `POST /mcp`
  (tools/call `run_status`) and `GET /health`.
- About 56–110 of the answers are 429, /health included.
- 0–30 requests per burst fail with ReadError `ECONNRESET`, ReadError `EBADF`, or RemoteProtocolError.
- The same requests sent one after another on one connection get only 429s.

Expected:
- Every refused request gets its 429, on a connection that stays usable, or one that says `Connection: close` and
  closes cleanly.
- `/health` and `/ready` answer however much of a key's budget is left.

## Why it matters

A client cannot tell a reset connection from roko serve being down. The Hermes plugin showed a false OFFLINE in the
guide's video takes. The plugin now keeps itself under 25 requests a second, and retries 429s and dropped GETs
(hermes-plugin-roko `client.py`, round v6). Other clients still hit this: the web dashboard, scripts, and a second
Hermes process with the same key.

## Where

- `crates/roko-serve/src/routes/mod.rs::keyed_rate_limit_middleware` and `rate_limit_middleware` answer 429 before
  the handler runs, and before the request body is read.
- The router assembly in the same file (the `.layer(...)` calls after `build_keyed_rate_limiter`) puts both limiters
  around every route, `/health` (`top_level_health`) included.

## Current state

Not looked at in the Rust yet. The likely cause of the resets:
- The middleware answers 429 without reading the request body.
- hyper then closes a keep-alive connection that still has unread bytes, and the TCP stack resets it.
- A request the client had already written on that connection fails.

This fits what was seen: traffic sent one request at a time, whose small body arrives with its headers, shows no
resets, and concurrent traffic does.

## Plan

1. Take the liveness routes (`/health`, `/ready`) out of both limiters: mount them outside the layered router, or
   skip them in the middleware.
2. Make a rate-limited answer leave the connection usable. Either drain the request body (up to the body limit)
   before answering 429, or answer 429 with `Connection: close` so that clients do not reuse the connection.
3. Tests:
   - A concurrent client sends a burst past a small limit. Every request gets an HTTP answer (200 or 429), and none
     a transport error.
   - `/health` answers 200 after the key's budget is spent.

## Done when

Both `[[verify]]` commands pass: `rate_limited_requests_leave_their_connection_usable` and
`health_answers_past_the_key_rate_limit`.

## Notes

- Evidence: `tmp/hermes-roko-v6/findings/plugin-setup.md` (the OFFLINE section) and the logs in
  `tmp/hermes-roko-v6/plugin-setup/flash/logs/`. `calls.jsonl` has the second with 45 requests, and
  `transport.jsonl` the reset.
- The plugin's own fix (pacing, a /health look that tolerates one failure) is in hermes-plugin-roko and does not
  wait on this item.
