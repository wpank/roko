+++
id = "bug-a5dcaa"
kind = "bug"
title = "The /api/events keepalive is an SSE comment EventSource never delivers, so the portal reopens idle streams every 60 s"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-serve/sse"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "1bff443b0"
source = "session:roko-b6 2026-09-29 portal close-out"
anchors = ["crates/roko-serve/src/routes/sse.rs::keepalive_event", "crates/roko-serve/src/routes/sse.rs::sse_handler", "apps/portal/src/api/sse-client.ts::SseClient"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn idle_stream_sends_a_named_keepalive_event_every_8_seconds' crates/roko-serve/src/routes/sse.rs && grep -qF 'keeps an idle stream open while keepalive frames arrive' apps/portal/src/api/sse-client.test.ts && cargo test -p roko-serve --lib routes::sse::tests::idle_stream_sends_a_named_keepalive_event && (cd apps/portal && npx vitest run src/api/sse-client.test.ts)"

[closed]
at = 2026-09-29
commit = "1bff443b0"
evidence = "1bff443b0: the /api/events keep-alive (crates/roko-serve/src/routes/sse.rs::keepalive_event, used by sse_handler) is now a named event, 'event: keepalive' with data '{}', every 8 s while idle, instead of an SSE comment that EventSource never delivers; apps/portal/src/api/sse-client.ts resets its 60 s watchdog on it (one addEventListener line). EventSource clients without the listener never see it; roko-cli's sse_stream client and the harness sse.py skip it. [[verify]] passes: routes::sse::tests::idle_stream_sends_a_named_keepalive_event_every_8_seconds (reads exactly 'event: keepalive\\ndata: {}\\n\\n' at 8 s and 16 s of idle) and the portal sse-client.test.ts (10 tests; the new 'keeps an idle stream open while keepalive frames arrive, and passes none of them on' fails without the listener). Portal tsc --noEmit and clippy -p roko-serve -D warnings are clean."
+++

## Problem

`GET /api/events` sent its keep-alive every 8 s as an SSE comment (`: keepalive`). `EventSource` never
dispatches comments to script, so the portal's `SseClient` watchdog (`KEEPALIVE_TIMEOUT_MS`, 60 s) took an
idle but healthy stream for a dead one and closed and reopened it every 60 s. The reopen resumes from the
cursor, so no event is lost, but the connection churns and its status flips to `disconnected` and back.

Expected: an idle, healthy stream stays open.

## Why it matters

Goal `visibility`: the portal's live stream. Every reopen is a new request and a status flicker, and
through a proxy it is a new upstream connection.

## Where

- `crates/roko-serve/src/routes/sse.rs::sse_handler`: the `KeepAlive` of `/api/events` and `/api/sse`, built
  by `keepalive_event`.
- `apps/portal/src/api/sse-client.ts::SseClient`: `openConnection` registers the listeners that reset the
  watchdog (`resetKeepalive`).

## Current state

The keep-alive is now a named event, `event: keepalive` with data `{}` (a browser drops an event that has
no data). `EventSource` clients without a `keepalive` listener never see it. The CLI's SSE client
(`roko-cli/src/runner/sse_stream.rs`) and the portal harness's `sse.py` skip it, because it is not a
`DashboardEvent` and has no `type`. The portal listens for it and resets its watchdog (one line in
`openConnection`). The other SSE endpoints (runs, workflow, bench, projections) keep their comment
keep-alives; the portal does not use them.

## Plan

Done as described under Current state.

## Done when

The `[[verify]]` passes: `routes::sse::tests::idle_stream_sends_a_named_keepalive_event_every_8_seconds`
reads `event: keepalive\ndata: {}\n\n` from an idle stream every 8 s, and the portal test "keeps an idle
stream open while keepalive frames arrive, and passes none of them on" holds the stream open through 80 s
of keepalives. The portal test fails without the listener.

## Notes

Do not send the keep-alive as an unnamed `data:` frame. The portal passes every unnamed frame to its
store, warns on frames it cannot parse, and Rust clients parse them as `DashboardEvent`.
