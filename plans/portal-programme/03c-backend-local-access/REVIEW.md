# 03c-backend-local-access — Access-Check Review

## Acceptance-check output

```
PASS roko serve prints the portal URL with a launch token
PASS the token file holds the same token
PASS the token file is private (0600)
PASS an API request without a credential is refused
PASS the event stream refuses a client without a credential
PASS the launch token works as a bearer credential
PASS ROKO_SPA_DIR serves the UI at /
PASS a client-side route falls back to the UI's index
PASS the UI's assets are served
PASS a missing asset is a 404, not the index page
PASS an unknown /api path is a JSON 404, never a UI page
PASS the demo app is served under /demo, built for that base
PASS a demo client route falls back to the demo's index
PASS a wrong token cannot open a session
PASS the launch token opens a session (204 and a cookie)
PASS the session cookie is HttpOnly, SameSite=Strict, Path=/
PASS the cookie authenticates the API
PASS the cookie authenticates the event stream
PASS a cross-origin write with the cookie is refused
PASS a same-origin write with the cookie passes auth
PASS roko plan run authenticates to the server with the token file
PASS ending the session revokes the cookie
PASS the server stops on Ctrl-C
PASS the token file is removed on shutdown
ACCESS-CHECK: PASS (24 checks)
```

## Fixes applied before the run reached PASS

Two assertions failed on the first run (22 of 24 passed). Both were bugs in
the production code; no assertion was weakened or removed.

### Fix 1 — `a client-side route falls back to the UI's index`

**Root cause.** `GET /runs/{id}` is registered in `shared_runs::public_routes()`
as a public (no-auth) endpoint that renders a self-contained HTML page for
shared run transcripts.  When the ID does not match any stored transcript the
handler returned `StatusCode::NOT_FOUND` directly, which stopped Axum from
delegating to the SPA fallback.  The check path `/runs/a-client-route` hit
this handler before the SPA fallback and received a bare 404 instead of the
portal `index.html`.

**Fix.**  Added `crate::embedded::serve_portal_index()` to `embedded.rs` and
changed the `TranscriptLookup::Missing` arm in `shared_runs::get_run_html` to
call it.  An unknown share ID is now treated as a client-side portal route
(e.g. a live run-detail page) and the SPA is served so the browser's router
can handle it.

Files changed:
- `crates/roko-serve/src/embedded.rs` — new `pub async fn serve_portal_index()`
- `crates/roko-serve/src/routes/shared_runs.rs` — `TranscriptLookup::Missing` arm

### Fix 2 — `roko plan run authenticates to the server with the token file`

**Root cause.** `run_plan_via_server` built its `WorkspaceServerClient` by
calling `WorkspaceServerClient::new(endpoint, &ServeAuthConfig::default(), wd)`.
That calls `resolve_api_key`, which consults `~/.roko/credentials.json` (stored
by a previous `roko login`).  If such a credential exists on the machine it
takes priority over the workspace launch token — but the credential belongs to
a different server, so the test workspace server returned HTTP 401.

**Fix.**  Changed `run_plan_via_server` to use `new_with_resolved` with only
the `ROKO_API_KEY` env var checked explicitly, leaving `resolved = None` when
the env var is absent so the function falls through to `read_launch_token(wd)`.
Globally-stored `roko login` credentials are never consulted for workspace-local
connections, eliminating the shadowing.

Files changed:
- `crates/roko-cli/src/serve_client.rs` — lines ~600-602

## Verdict

ACCESS-CHECK: PASS
