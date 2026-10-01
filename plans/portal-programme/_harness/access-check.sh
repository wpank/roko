#!/usr/bin/env bash
# Acceptance check for 03c-backend-local-access.
#
# A fresh workspace keeps `[serve.auth] enabled = true` with no key, as `roko init`
# writes it. Its loopback `roko serve` must then mint a launch token, print the
# portal URL carrying it, and let a browser trade it for a session cookie that
# authenticates both the API and the event stream. CLI clients use the 0600 token
# file. The server also serves the UI at / (here a stand-in export through
# ROKO_SPA_DIR) and the demo app under /demo. The agent is ./fake-claude.
#
# Usage (from the repo root, after `cargo build -p roko-cli`):
#   bash plans/portal-programme/_harness/access-check.sh
# Prints one PASS/FAIL line per assertion and a final ACCESS-CHECK verdict. On a
# build without the demo app, the two /demo checks print SKIP with the reason.

source "$(cd "$(dirname "$0")" && pwd)/lib.sh"

TOKEN=""
TOKEN_SEEN=""
SESSION=""
base() { printf 'http://127.0.0.1:%s' "$PORT"; }
status_of() { curl -s -m 5 -o /dev/null -w '%{http_code}' "$@"; }
token_printed() {
    TOKEN="$(sed -n 's/.*#token=\([0-9A-Za-z_-]*\).*/\1/p' "$WS/serve.log" | head -1)"
    [ -n "$TOKEN" ]
}
token_file_matches() {
    [ -n "$TOKEN" ] && [ "$(cat "$WS/.roko/runtime/serve.token" 2>/dev/null)" = "$TOKEN" ] || return 1
    TOKEN_SEEN=1
}
token_file_private() {
    [ "$(python3 -c 'import os, stat, sys; print(oct(stat.S_IMODE(os.stat(sys.argv[1]).st_mode)))' \
        "$WS/.roko/runtime/serve.token" 2>/dev/null)" = "0o600" ]
}
open_session() {
    local code
    code="$(curl -s -m 5 -D "$WS/session.headers" -o /dev/null -w '%{http_code}' -X POST \
        -H 'content-type: application/json' --data "{\"token\":\"$TOKEN\"}" "$(base)/api/auth/session")"
    SESSION="$(tr -d '\r' <"$WS/session.headers" | sed -n 's/^[Ss]et-[Cc]ookie: roko_session=\([^;]*\).*/\1/p' | head -1)"
    [ "$code" = 204 ] && [ -n "$SESSION" ]
}
cookie_flags() {
    local line
    line="$(tr -d '\r' <"$WS/session.headers" | grep -i '^set-cookie: roko_session=')"
    printf '%s' "$line" | grep -qi 'HttpOnly' && printf '%s' "$line" | grep -qi 'SameSite=Strict' &&
        printf '%s' "$line" | grep -qi 'Path=/'
}
with_cookie() { [ -n "$SESSION" ] && status_of -b "roko_session=$SESSION" "$@"; }
same_origin_write_passes_auth() {
    local code
    code="$(with_cookie -X POST -H "Origin: $(base)" "$(base)/api/plans/live-b/cancel")"
    [ -n "$code" ] && [ "$code" != 401 ] && [ "$code" != 403 ]
}
submitted_with_token() { grep -q "submitted to roko serve" "$WS/client.out" && [ "$CLIENT_RC" = 0 ]; }
logout_revokes() {
    [ "$(with_cookie -X DELETE "$(base)/api/auth/session")" = 204 ] &&
        [ "$(with_cookie "$(base)/api/plans")" = 401 ]
}
removed_after_seen() { [ -n "$1" ] && test ! -e "$2"; }
body_of() { curl -s -m 5 "$@"; }
ui_serves_fixture() { body_of "$(base)/" | grep -q PORTAL-FIXTURE; }
ui_route_falls_back() { body_of "$(base)/runs/a-client-route" | grep -q PORTAL-FIXTURE; }
ui_asset_served() { body_of "$(base)/_next/static/app.js" | grep -q 'portal fixture'; }
api_unknown_is_json_404() {
    local code
    [ -n "$TOKEN" ] || return 1
    code="$(curl -s -m 5 -o "$WS/api404.json" -w '%{http_code}' \
        -H "Authorization: Bearer $TOKEN" "$(base)/api/no-such-route")"
    [ "$code" = 404 ] && jcheck "$WS/api404.json" 'd.get("error") == "not_found"'
}
demo_served() { body_of "$(base)/demo/" | grep -q '/demo/assets/'; }
demo_route_falls_back() { body_of "$(base)/demo/settings" | grep -q '/demo/assets/'; }
# A build made without demo/demo-app/dist embeds roko-serve's fallback page in
# place of the demo app (crates/roko-serve/build.rs), and /demo serves it.
demo_not_built() { body_of "$(base)/demo/" | grep -q 'Roko API is running'; }

require_binary
make_workspace
python3 - "$WS/roko.toml" <<'PY'
import sys
path = sys.argv[1]
text = open(path).read()
assert "[serve.auth]\nenabled = false" in text
open(path, "w").write(text.replace("[serve.auth]\nenabled = false", "[serve.auth]\nenabled = true\napi_key = \"\"", 1))
PY
# A stand-in portal export, served through ROKO_SPA_DIR, the override for the UI at /.
mkdir -p "$WS/portal-fixture/_next/static"
printf '<!doctype html><title>fixture</title><p>PORTAL-FIXTURE</p>\n' >"$WS/portal-fixture/index.html"
printf 'console.log("portal fixture");\n' >"$WS/portal-fixture/_next/static/app.js"
export ROKO_SPA_DIR="$WS/portal-fixture"
start_server

# ── The launch token ───────────────────────────────────────────────────────
check "roko serve prints the portal URL with a launch token" token_printed
check "the token file holds the same token" token_file_matches
check "the token file is private (0600)" token_file_private
check "an API request without a credential is refused" [ "$(status_of "$(base)/api/plans")" = 401 ]
check "the event stream refuses a client without a credential" [ "$(status_of "$(base)/api/events")" = 401 ]
check "the launch token works as a bearer credential" \
    [ "$(status_of -H "Authorization: Bearer $TOKEN" "$(base)/api/plans")" = 200 ]

# ── The UIs: the portal at /, the demo app under /demo ─────────────────────
check "ROKO_SPA_DIR serves the UI at /" ui_serves_fixture
check "a client-side route falls back to the UI's index" ui_route_falls_back
check "the UI's assets are served" ui_asset_served
check "a missing asset is a 404, not the index page" \
    [ "$(status_of "$(base)/_next/static/missing.js")" = 404 ]
check "an unknown /api path is a JSON 404, never a UI page" api_unknown_is_json_404
if demo_not_built; then
    NO_DEMO="this roko was built without demo/demo-app/dist; /demo serves the fallback page"
    skip "the demo app is served under /demo, built for that base" "$NO_DEMO"
    skip "a demo client route falls back to the demo's index" "$NO_DEMO"
else
    check "the demo app is served under /demo, built for that base" demo_served
    check "a demo client route falls back to the demo's index" demo_route_falls_back
fi

# ── The browser session ────────────────────────────────────────────────────
check "a wrong token cannot open a session" \
    [ "$(status_of -X POST -H 'content-type: application/json' --data '{"token":"nope"}' "$(base)/api/auth/session")" = 401 ]
check "the launch token opens a session (204 and a cookie)" open_session
check "the session cookie is HttpOnly, SameSite=Strict, Path=/" cookie_flags
check "the cookie authenticates the API" [ "$(with_cookie "$(base)/api/plans")" = 200 ]
check "the cookie authenticates the event stream" [ "$(with_cookie "$(base)/api/events")" = 200 ]
check "a cross-origin write with the cookie is refused" \
    [ "$(with_cookie -X POST -H 'Origin: http://evil.example' "$(base)/api/plans/live-b/cancel")" = 403 ]
check "a same-origin write with the cookie passes auth" same_origin_write_passes_auth

# ── A CLI client ───────────────────────────────────────────────────────────
run_in_ws "$ROKO_BIN" plan run plans/live-b --no-tui >"$WS/client.out" 2>&1
CLIENT_RC=$?
check "roko plan run authenticates to the server with the token file" submitted_with_token

# ── Ending the session, and shutdown ───────────────────────────────────────
check "ending the session revokes the cookie" logout_revokes
check "the server stops on Ctrl-C" stop_server
check "the token file is removed on shutdown" removed_after_seen "$TOKEN_SEEN" "$WS/.roko/runtime/serve.token"

finish "ACCESS-CHECK"
