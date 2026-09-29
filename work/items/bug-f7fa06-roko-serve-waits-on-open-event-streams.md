+++
id = "bug-f7fa06"
kind = "bug"
title = "roko serve waits on open event streams at Ctrl-C: about 60 s with a portal tab open, forever with curl"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-serve/shutdown"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "f813e0486"
source = "session:roko-b6 2026-09-29 portal close-out"
anchors = ["crates/roko-serve/src/routes/sse.rs::until_shutdown", "crates/roko-serve/src/lib.rs::serve_until_cancelled", "crates/roko-serve/src/lib.rs::drain_within", "crates/roko-serve/src/routes/ws.rs::handle_ws", "crates/roko-serve/tests/lifecycle.rs::serve_shutdown_ends_open_event_streams"]
links = { depends_on = [], blocks = [], related = ["bug-a5dcaa"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn serve_shutdown_ends_open_event_streams' crates/roko-serve/tests/lifecycle.rs && grep -qw 'fn stream_ends_when_the_server_shuts_down' crates/roko-serve/src/routes/sse.rs && grep -qw 'fn shutdown_drain_abandons_connections_open_past_the_grace_period' crates/roko-serve/src/lib.rs && cargo test -p roko-serve --test lifecycle serve_shutdown_ends_open_event_streams && cargo test -p roko-serve --lib -- routes::sse::tests::stream_ends_when_the_server_shuts_down tests::shutdown_drain_abandons_connections_open_past_the_grace_period"

[closed]
at = 2026-09-29
commit = "f813e0486"
evidence = "f813e0486: every SSE route wraps its stream in routes::sse::until_shutdown (ends it when AppState.cancel fires): /api/events and /api/sse, run events, /api/workflow/events, bench events, projection and workflow streams; /ws sends a 1001 close frame at shutdown; both axum serve paths (ServerBuilder::start_background for roko serve, run_server_with_state) go through lib.rs::serve_until_cancelled, whose drain_within gives open connections 5 s (SHUTDOWN_DRAIN_GRACE) after draining starts. [[verify]] passes: lifecycle::serve_shutdown_ends_open_event_streams (real listener, /api/events and /ws held open, cancel: server exits within 3 s, SSE body ends, /ws gets 1001; the lifecycle file ran in 0.14 s; on the pre-fix sources the test fails with 'server did not shut down within 3 seconds with event streams open'), routes::sse::tests::stream_ends_when_the_server_shuts_down and tests::shutdown_drain_abandons_connections_open_past_the_grace_period. Also run: lib tests for routes::{sse,runs,projections,workflows,ws,bench} and routes::tests (75 pass), api_integration (102 pass, incl. jobs_events_are_visible_over_websocket); clippy -p roko-serve -D warnings clean. Not checked by hand against a rebuilt roko binary."
+++

## Problem

With a portal tab open, `roko serve` took about 60 s to exit after Ctrl-C. With a `curl /api/events`
client connected it never exited. Ctrl-C cancels the server and axum starts a graceful shutdown, which
waits for every open connection to finish, and an SSE response never finishes by itself. The portal's
connection closed only when its 60 s silence watchdog gave up (bug-a5dcaa); curl never gives up.

Expected: Ctrl-C ends open event streams and the server exits within seconds.

## Why it matters

Goal `visibility`: `roko serve` is the portal's backend, and restarting it is routine. A server that
hangs on exit gets killed, which skips the shutdown work (state snapshot, endpoint file cleanup).

## Where

- `crates/roko-serve/src/routes/sse.rs::until_shutdown`: ends a stream when `AppState.cancel` fires. Every
  SSE route wraps its stream in it: `/api/events` and `/api/sse` (`sse_handler`), the run event stream
  (`routes/runs.rs`), `/api/workflow/events` (`routes/mod.rs`), bench events (`routes/bench.rs`),
  projection streams (`routes/projections.rs`) and workflow streams (`routes/workflows.rs`).
- `crates/roko-serve/src/lib.rs::serve_until_cancelled` / `drain_within`: the axum server of both
  `ServerBuilder::start_background` (used by `roko serve`) and `run_server_with_state`. Once graceful
  shutdown starts draining, connections get `SHUTDOWN_DRAIN_GRACE` (5 s) to finish, and whatever is still
  open is abandoned.
- `crates/roko-serve/src/routes/ws.rs::handle_ws`: `/ws` sends a close frame (1001, going away) at
  shutdown.

## Current state

Fixed as described under Where. The other WebSocket routes (workflow, aggregator, terminal, relay and RPC
proxies) never held up the drain: hyper hands a socket off at the upgrade, so graceful shutdown does not
track it. Those sockets close when the process exits. The TLS listener (`trigger_tls::serve`) already
aborted its connections on cancel.

## Plan

Done as described under Where.

## Done when

The `[[verify]]` passes:

- `lifecycle::serve_shutdown_ends_open_event_streams` holds `/api/events` and `/ws` open against a real
  listener, cancels, and requires the server to exit within 3 s, the SSE body to end and `/ws` to get
  close code 1001. On the old code it fails ("server did not shut down within 3 seconds").
- `routes::sse::tests::stream_ends_when_the_server_shuts_down` checks the stream end.
- `tests::shutdown_drain_abandons_connections_open_past_the_grace_period` checks the 5 s bound.

## Notes

`roko_runtime::cancel::CancelToken::cancelled` checks the flag before it creates its `Notified` future, so
a `cancel()` that lands between the two is missed. That race is not fixed here. If a stream misses the
cancel that way, the 5 s drain bound still ends the shutdown.
