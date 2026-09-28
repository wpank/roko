+++
id = "bug-12d48c"
kind = "bug"
title = "Portal event stream sends no credential, so it fails with 401 when serve auth is on"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["apps/portal", "roko-serve/auth"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["apps/portal/src/api/sse-client.ts:204"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

`sse-client.ts` opens a bare `new EventSource(this.buildUrl())` (`:204`) carrying only `?lastEventId=`.
`EventSource` cannot set `X-Api-Key` and roko-serve accepts no query-string or cookie credential, so with auth enabled `/api/events` returns 401 and the portal freezes on its initial snapshot.
Fix: a fetch-based SSE client that sends the header, or a short-lived stream-scoped token accepted by serve for `/api/events` only.
Natural home: `plans/portal-programme/05-portal-foundation` T05 (already touches `sse-client.ts`), not in its current acceptance criteria.
