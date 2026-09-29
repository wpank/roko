+++
id = "bug-12d48c"
kind = "bug"
title = "Portal event stream sends no credential, so it fails with 401 when serve auth is on"
status = "done"
triage = "verified"
severity = "p2"
subsystem = ["apps/portal", "roko-serve/auth"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["apps/portal/src/api/sse-client.ts:204"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-09-29
by = "plan:portal-programme/03c-backend-local-access#T10"
run_id = "graph-03c-backend-local-access-ecb447db-1cfe-4ad3-9f93-2bc34ba59f34"
evidence = "ACCESS-CHECK: PASS (24 checks). 'PASS the cookie authenticates the event stream' confirmed that POST /api/auth/session exchanges the launch token for an HttpOnly SameSite=Strict cookie which the browser sends automatically on /api/events. The native EventSource now authenticates without a header."
+++

`sse-client.ts` opens a bare `new EventSource(this.buildUrl())` (`:204`) carrying only `?lastEventId=`.
`EventSource` cannot set `X-Api-Key` and roko-serve accepts no query-string or cookie credential, so with auth enabled `/api/events` returns 401 and the portal freezes on its initial snapshot.
Fix: a fetch-based SSE client that sends the header, or a short-lived stream-scoped token accepted by serve for `/api/events` only.
Natural home: `plans/portal-programme/05-portal-foundation` T05 (already touches `sse-client.ts`), not in its current acceptance criteria.
