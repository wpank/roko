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
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
SSE client + reconnection. Implement the SSE event client that connects to `GET /api/events` on roko-serve. This is the primary real-time data source — 99% of live updates flow through SSE.

Imported without verification from:
- `tmp/portal-backlog/PB-005-sse-client.md#PB-005`

How to verify: Check portal app for this feature (9 acceptance criteria, e.g. `src/hooks/useStateHubSSE.ts` — custom hook wrapping EventSource; `src/api/sse-client.ts` — standalone client class (usable outside React)); cross-check plans/portal-programme/* and existing web app dirs.

Verified 2026-09-28: built except for remote auth. apps/portal/src/api/sse-client.ts (389 lines) provides the standalone client, cursor replay via Last-Event-ID and ?lastEventId, exponential backoff capped at 16 s (:49), a 60 s keepalive watchdog (:53), and turns gap frames into snapshot events. hooks/useStateHubSSE.ts dispatches into the store, and api/types.ts holds the DashboardEvent and DashboardSnapshot types. A /api/health probe runs from AppShell.tsx:53. Still unmet: the SSE client sends no credentials (no bearer header and no query token), so 'remote (bearer token) mode' does not work, even though the REST client does send Bearer (api/client.ts:43-54). Gap-recovery store defects are tracked under spec-52eeb9. Severity lowered p1 to p2.

Re-checked 2026-09-29: same-origin SSE is now authenticated by the session cookie that useStateHubSSE.ts obtains by posting the launch token to /api/auth/session (plan 03c). Remote (bearer token) mode is still unmet: sse-client.ts buildUrl adds only lastEventId, the EventSource is created without withCredentials or a token, and no query-token handling was found in roko-serve routes/middleware.rs or routes/sse.rs. The cross-origin cookie side of the same problem is gap-eb4a65.
