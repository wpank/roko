+++
id = "bug-12d48c"
kind = "bug"
title = "Portal event stream sends no credential, so it fails with 401 when serve auth is on"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["apps/portal", "roko-serve/auth"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["apps/portal/src/api/sse-client.ts:204", "apps/portal/src/api/sse-client.ts:222", "apps/portal/src/hooks/useStateHubSSE.ts:79", "crates/roko-serve/src/routes/middleware.rs:327"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

`sse-client.ts` opens a bare `new EventSource(this.buildUrl())` (`:204`) carrying only `?lastEventId=`.
`EventSource` cannot set `X-Api-Key` and roko-serve accepts no query-string or cookie credential, so with auth enabled `/api/events` returns 401 and the portal freezes on its initial snapshot.
Fix: a fetch-based SSE client that sends the header, or a short-lived stream-scoped token accepted by serve for `/api/events` only.
Natural home: `plans/portal-programme/05-portal-foundation` T05 (already touches `sse-client.ts`), not in its current acceptance criteria.

Verified 2026-09-28 (static check against 3d0ee4d02): apps/portal/src/api/sse-client.ts:222 still opens a bare `new EventSource(this.buildUrl())` whose URL carries only `?lastEventId=` (:197-208). Portal commit 39e72deef added a launch-token flow that POSTs `#token=` to `/api/auth/session` expecting a session cookie (hooks/useStateHubSSE.ts:79-85, lib/bootstrap.ts:4-7,158-167), but roko-serve has no such route and no cookie or query-string auth: routes/middleware.rs:327-337 accepts only `X-Api-Key` and `Authorization: Bearer` (unchanged since 244f564e1), so with auth on the stream still gets 401.
