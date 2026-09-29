+++
id = "gap-eb4a65"
kind = "gap"
title = "Portal dev server on another port cannot use session cookie without credentialed CORS"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["apps/portal", "roko-serve/auth"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "plan:portal-programme/03c-backend-local-access#T10"
discovered_from = "plan:portal-programme/03c-backend-local-access#T10"
anchors = ["crates/roko-serve/src/routes/middleware.rs::cors_layer", "crates/roko-serve/src/routes/auth_session.rs::session_cookie", "apps/portal/src/lib/env.ts::getRokoServeUrl", "apps/portal/next.config.ts"]
links = { depends_on = [], blocks = [], related = ["bug-12d48c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q \"allow_credentials(true)\" crates/roko-serve/src/routes/middleware.rs && grep -rq \"credentials: 'include'\" apps/portal/src/api"
+++

When `roko serve` runs on port 6677 and the portal dev server runs on a different
port (e.g. 3000 or 5173), the session cookie set by `POST /api/auth/session` is
`SameSite=Strict`. Browsers do not send `SameSite=Strict` cookies on cross-origin
requests, and the server's CORS middleware does not set `Access-Control-Allow-Credentials`
for the dev origin. The portal's `fetch` calls succeed only when `credentials: "include"`
and a matching `Access-Control-Allow-Origin` header are both present, neither of which
is configured.

**Impact:** portal development against a live `roko serve` on a different port cannot
use the cookie-based auth path. The two workarounds that do work today:

1. Set `ROKO_API_KEY` and let the portal dev server send `X-Api-Key` on every request.
2. Set `[serve.auth] enabled = false` in `roko.toml` during development.

A fix would require either (a) a configurable dev-CORS allowlist that adds
`Access-Control-Allow-Credentials` and a matching `Access-Control-Allow-Origin` for
the listed origin, or (b) a query-string token accepted by the server for dev
sessions only. Option (a) is the standard approach (Vite's proxy also routes around
it). Neither option is in scope for the portal programme; this gap is recorded for
a later iteration.

Re-checked 2026-09-29: the gap is narrower than described. Under `next dev`, lib/env.ts getRokoServeUrl returns '' and next.config.ts rewrites /api/* and /ws/* to roko serve, so the session POST and later requests are same-origin through the proxy and the cookie should work (not yet confirmed live). The gap remains when the portal talks to serve cross-origin (a saved roko-connection-url profile, or NEXT_PUBLIC_ROKO_SERVE_URL in a non-dev build): middleware.rs cors_layer sets no allow_credentials and the portal client never sends credentials: 'include'. SameSite=Strict is not itself the blocker, because localhost:3000 and localhost:6677 are the same site. The cookie code is in routes/auth_session.rs, not routes/auth.rs.
