+++
id = "spec-ddc287"
kind = "spec"
title = "PB-005: SSE client + reconnection"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["apps/portal"]
created = 2026-09-23
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/portal-backlog/PB-005-sse-client.md#PB-005"
discovered_from = "audit:tmp/portal-backlog/PB-005-sse-client.md#PB-005"
anchors = ["apps/portal/src/api/sse-client.ts", "apps/portal/src/hooks/useStateHubSSE.ts", "apps/portal/src/api/client.ts"]
links = { depends_on = [], blocks = [], related = ["gap-eb4a65", "bug-12d48c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qF 'replays from the last event id after a reconnect' apps/portal/src/api/sse-client.test.ts && (cd apps/portal && npx vitest run src/api/sse-client)"
+++

## Problem

PB-005, imported from the 2026-09-23 portal backlog, specified the SSE client behind the portal's live
view. It covers a hook and a standalone client for `GET /api/events`, cursor replay, reconnect with
backoff, gap recovery, a keepalive watchdog, typed events, a connection status, capped buffers, a
health probe, and access with and without a bearer token. The portal has been redesigned since then
(`tmp/portal-audit/02-DESIGN.md`, `03-CONTRACT.md`). Each criterion is now either met by the code or
replaced by that design. The client itself had no tests.

## Why it matters

Goal `visibility`: almost every live update reaches the portal through this stream.

## Where

- `apps/portal/src/api/sse-client.ts` (`SseClient`).
- `apps/portal/src/hooks/useStateHubSSE.ts`, which wires the client into `startLiveState`
  (`lib/bootstrap.ts`) and the dashboard store.
- `apps/portal/src/api/contracts.ts` (wire types).

## Current state (checked 2026-09-29)

| PB-005 criterion | State | Evidence |
|---|---|---|
| `hooks/useStateHubSSE.ts` hook | met | runs `startLiveState` with an `SseClient` |
| `api/sse-client.ts` standalone class | met | no React import; `api/sse-client.test.ts` |
| connect to `${ROKO_SERVE_URL}/api/events` | superseded | roko-serve serves the portal, so every request is same-origin (02-DESIGN §0); test "connects to /api/events on the page origin…" |
| track `Last-Event-ID`, send it on reconnect | met | sent as `?lastEventId=`, because EventSource cannot set headers; test "replays from the last event id after a reconnect" |
| backoff 1, 2, 4, 8, 16 s cap | met | tests "backs off 1 s, 2 s, 4 s, 8 s, then 16 s…" and "starts the backoff over once a connection opens" |
| a gap event replaces the snapshot | met | test "replaces the state with the snapshot a gap frame carries…"; `bootstrap.test.ts` "gap frame → replace" |
| reconnect after 60 s without an event | met | test "reconnects when nothing arrives for 60 s…" |
| typed `DashboardEvent` union of ~35 types | met, narrowed | `WireDashboardEvent` in `api/contracts.ts` holds the 18 events the run view folds (03-CONTRACT §3.2); `runState.applyEvent` ignores other types; test "passes typed events through and drops frames it cannot read" |
| `api/types.ts` | superseded | the camelCase `types.ts` was deleted in 39e72deef; `api/contracts.ts` matches the snake_case wire (03-CONTRACT §3.1) |
| status `connected/connecting/disconnected/error` | met | `ConnectionStatus` in `lib/bootstrap.ts`, shown in the header and alerts; test "reports each status change…"; `bootstrap.test.ts` "sets status error on fetch failure and retries" |
| buffer caps (256 gates, 128 episodes, 64 errors, 200 log entries) | superseded | there is no episode or event-log view (02-DESIGN §13); `lib/runState.ts` caps transcripts at 2000 entries, errors at 50 and usage samples at 600, and keeps one check per gate per task |
| `GET /health` before the first connection | superseded | `startLiveState` fetches `/api/statehub/snapshot` before it opens the stream, retrying 1→16 s (03-CONTRACT §3.1); `bootstrap.test.ts` "installs the snapshot before opening the stream" |
| local (no auth) and remote (bearer) access | met, bearer form superseded | EventSource cannot send a header. Plan 03c exchanges the launch token, or a configured API key, for an HttpOnly session cookie (`POST /api/auth/session`, 03-CONTRACT §4.3), and same-origin `fetch` and `EventSource` send that cookie (bug-12d48c, closed on ACCESS-CHECK). A remote roko-serve takes the same path through `#token=<key>`. A portal served from a different origin than the server is outside the design; gap-eb4a65 tracks its credentialed-CORS gap |

The design also cut the persisted cursor (02-DESIGN §13). The cursor stays in memory across reconnects,
and each page load starts from a fresh snapshot. The gap-recovery store work was spec-52eeb9 (done).

## Done when

`apps/portal/src/api/sse-client.test.ts` covers the lifecycle rows above. The `[[verify]]` runs it.

## Notes

The server's keepalive is an SSE comment (`: keepalive`, every 8 s), and `EventSource` never passes
comments to script. The 60 s watchdog therefore also reopens an idle but healthy stream, from its cursor,
so no events are lost. The comment on `KEEPALIVE_TIMEOUT_MS` now says this. A named keepalive event from
roko-serve would let the watchdog tell an idle stream from a dead one.

## Original notes

SSE client + reconnection. Implement the SSE event client that connects to `GET /api/events` on roko-serve. This is the primary real-time data source — 99% of live updates flow through SSE.

Imported without verification from:
- `tmp/portal-backlog/PB-005-sse-client.md#PB-005`

How to verify: Check portal app for this feature (9 acceptance criteria, e.g. `src/hooks/useStateHubSSE.ts` — custom hook wrapping EventSource; `src/api/sse-client.ts` — standalone client class (usable outside React)); cross-check plans/portal-programme/* and existing web app dirs.

Verified 2026-09-28: built except for remote auth. apps/portal/src/api/sse-client.ts (389 lines) provides the standalone client, cursor replay via Last-Event-ID and ?lastEventId, exponential backoff capped at 16 s (:49), a 60 s keepalive watchdog (:53), and turns gap frames into snapshot events. hooks/useStateHubSSE.ts dispatches into the store, and api/types.ts holds the DashboardEvent and DashboardSnapshot types. A /api/health probe runs from AppShell.tsx:53. Still unmet: the SSE client sends no credentials (no bearer header and no query token), so 'remote (bearer token) mode' does not work, even though the REST client does send Bearer (api/client.ts:43-54). Gap-recovery store defects are tracked under spec-52eeb9. Severity lowered p1 to p2.

Re-checked 2026-09-29: same-origin SSE is now authenticated by the session cookie that useStateHubSSE.ts obtains by posting the launch token to /api/auth/session (plan 03c). Remote (bearer token) mode is still unmet: sse-client.ts buildUrl adds only lastEventId, the EventSource is created without withCredentials or a token, and no query-token handling was found in roko-serve routes/middleware.rs or routes/sse.rs. The cross-origin cookie side of the same problem is gap-eb4a65.
