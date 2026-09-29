+++
id = "bug-f47afb"
kind = "bug"
title = "After roko serve restarts, the portal says 'Lost the server; reconnecting.' instead of asking for the new sign-in link"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["apps/portal"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "f99e45dba"
source = "plan:portal-programme/09-acceptance#T04"
discovered_from = "plan:portal-programme/09-acceptance#T04"
anchors = ["apps/portal/src/api/sse-client.ts:254", "apps/portal/src/lib/alerts.ts:189", "apps/portal/src/lib/bootstrap.ts:47"]
links = { depends_on = [], blocks = [], related = ["find-c8527b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqF --include='*.test.ts' --include='*.test.tsx' 'asks for the new sign-in link' apps/portal/src && (cd apps/portal && npx vitest run src/api src/lib)"
+++

## Problem

Sessions live in server memory (find-c8527b), so a restarted `roko serve` refuses the old cookie.
Design §0 says that "after a restart the API answers 401 and the alert says to open the link roko serve
printed".

Reproduced on 2026-09-29 with a scratch Playwright probe against a harness workspace with auth on:
the probe signed in by the printed link, then restarted `roko serve` on the same port. The alert
read "Lost the server; reconnecting. [Reconnect]" at +15 s, +30 s and +60 s, although the server was
up again.

The event stream's reconnects are refused with 401, but `EventSource` exposes no status: `onerror`
only sets `disconnected` or `error` (`sse-client.ts:254-272`). The sign-in text ("Not signed in to
this roko serve. Open the portal link it printed…", `bootstrap.ts:47`) appears only when a regular
API call returns 401, for example after pressing Reconnect, which reloads the page.

## Why it matters

Goal `visibility`. The operator is told the server is gone when it is up and wants a new link. The
fix, opening the newly printed URL, is never suggested.

## Where

`apps/portal/src/api/sse-client.ts` (reconnect loop), `apps/portal/src/lib/alerts.ts:189`
(connection alert) and `apps/portal/src/lib/bootstrap.ts:47` (the sign-in message).

## Plan

After a failed reconnect, probe a cheap authenticated endpoint, such as `GET /api/status`, and
switch to the sign-in alert on 401. Alternatively, open the stream with `fetch` so its status is
visible.

## Done when

A test whose name contains "asks for the new sign-in link" shows the sign-in alert once the stream
is refused and the probe answers 401. The `[[verify]]` runs it.

## Notes

Found by plan 09 T04 (see VERDICT).
