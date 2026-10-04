#!/usr/bin/env bash
# deploy/showcase/preflight.sh -- S11 section 7's hard gates, P1-P14 (task 9337).
#
#   preflight.sh <base-url> [--live-checks]
#   preflight.sh --local
#
# "Never deploy before preflight passes" (S11): this is the F1 exit check, and the regression
# check after every deploy or secret rotation. It prints one line per check (PASS/FAIL/SKIP) and
# exits with the number of FAILs (0 when every check passed).
#
# Secrets (SHOWCASE_PASSPHRASE, SHOWCASE_ADMIN_KEY) are read from the environment only, never a
# CLI argument, and never logged. ROKO_TEST_PRIVY_JWT is optional (P3): a real Privy JWT from
# `roko login`, when set; otherwise P3 uses a well-formed-but-fake one, since showcase mode must
# refuse every Bearer JWT regardless (bug-7eef96, G0a).
#
# --local builds nothing. It starts the locally built `roko` (ROKO_BIN, else
# <repo>/target/debug/roko) as a showcase-mode `roko serve` on 127.0.0.1, over plain HTTP, with a
# freshly generated passphrase and admin key, runs every check but the live-only ones, then stops
# it and removes its temporary workspace.
#
# P8's stream check and P9's admin-freeze check are each SKIPped, not FAILed, when the route
# answers 404: `/api/showcase/stream` and `/api/showcase/admin/freeze` are not wired yet at
# 2026-10-02 (grepped crates/roko-serve/src/routes/showcase/mod.rs). They stop being skipped the
# day a route answers something other than 404.
#
# `bash -n` keeps this parseable on macOS's bash 3.2: no associative arrays, `${x,,}`, or other
# bash-4-only syntax.

set -u

FAILURES=0
CHECKS=0
WORKDIR=""
SERVE_PID=""

usage() {
  echo "usage: preflight.sh <base-url> [--live-checks] | --local" >&2
  exit 2
}

cleanup() {
  if [ -n "$SERVE_PID" ]; then
    kill "$SERVE_PID" >/dev/null 2>&1
    wait "$SERVE_PID" 2>/dev/null
  fi
  if [ -n "$WORKDIR" ] && [ -d "$WORKDIR" ]; then
    rm -rf "$WORKDIR"
  fi
}
trap cleanup EXIT

# ---------------------------------------------------------------------------- reporting

pass() {
  CHECKS=$((CHECKS + 1))
  echo "PASS $1"
}

fail() {
  CHECKS=$((CHECKS + 1))
  FAILURES=$((FAILURES + 1))
  echo "FAIL $1: $2"
}

skip() {
  CHECKS=$((CHECKS + 1))
  echo "SKIP $1: $2"
}

# ---------------------------------------------------------------------------- http helpers

# status_of METHOD PATH [extra curl args...] -> prints the HTTP status code.
status_of() {
  method=$1
  path=$2
  shift 2
  curl -s -o /dev/null -w '%{http_code}' -X "$method" "$@" "$BASE_URL$path"
}

# expect_status NAME METHOD PATH EXPECTED [extra curl args...]
expect_status() {
  name=$1
  method=$2
  path=$3
  expected=$4
  shift 4
  got=$(status_of "$method" "$path" "$@")
  if [ "$got" = "$expected" ]; then
    pass "$name"
  else
    fail "$name" "$method $path: expected $expected, got $got"
  fi
}

# A JSON string literal's contents, with \ and " escaped (no control characters expected here).
json_escape() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

# ---------------------------------------------------------------------------- argument parsing

LOCAL=0
LIVE_CHECKS=0
BASE_URL=""

for arg in "$@"; do
  case "$arg" in
    --local)
      LOCAL=1
      ;;
    --live-checks)
      LIVE_CHECKS=1
      ;;
    --*)
      echo "preflight.sh: unknown option $arg" >&2
      usage
      ;;
    *)
      BASE_URL=$arg
      ;;
  esac
done

if [ "$LOCAL" = 1 ] && [ -n "$BASE_URL" ]; then
  echo "preflight.sh: --local takes no base-url" >&2
  usage
fi
if [ "$LOCAL" = 0 ] && [ -z "$BASE_URL" ]; then
  usage
fi

# ---------------------------------------------------------------------------- --local setup

REPO_ROOT=$(cd "$(dirname "$0")/../.." && pwd)

if [ "$LOCAL" = 1 ]; then
  ROKO_BIN=${ROKO_BIN:-"$REPO_ROOT/target/debug/roko"}
  if [ ! -x "$ROKO_BIN" ]; then
    echo "preflight.sh --local: no roko binary at $ROKO_BIN; build it (cargo build -p roko-cli)" \
      "or set ROKO_BIN" >&2
    exit 2
  fi

  PORT=${ROKO_SHOWCASE_PREFLIGHT_PORT:-6691}
  BASE_URL="http://127.0.0.1:$PORT"
  WORKDIR=$(mktemp -d)
  SHOWCASE_PASSPHRASE="preflight test passphrase $$"
  SHOWCASE_ADMIN_KEY="preflight-admin-key-$$"

  PASSPHRASE_HASH=$(printf '%s' "$SHOWCASE_PASSPHRASE" | "$ROKO_BIN" showcase passphrase hash)
  if [ -z "$PASSPHRASE_HASH" ]; then
    echo "preflight.sh --local: $ROKO_BIN showcase passphrase hash produced no output" >&2
    exit 2
  fi

  cat >"$WORKDIR/roko.toml" <<EOF
[serve]
public_routes = ["health", "ready"]

[serve.auth]
enabled = true
enforcement_mode = "enforce"
api_key = "$SHOWCASE_ADMIN_KEY"

[showcase]
enabled = true
public_origin = "$BASE_URL"
session = { cookie_name = "roko_session", cookie_secure = false }
EOF

  ROKO_SHOWCASE_PASSPHRASE_HASH=$PASSPHRASE_HASH \
    "$ROKO_BIN" serve --bind 127.0.0.1 --port "$PORT" --workdir "$WORKDIR" \
    >"$WORKDIR/serve.log" 2>&1 &
  SERVE_PID=$!

  ready=0
  i=0
  while [ "$i" -lt 50 ]; do
    if curl -fso /dev/null "$BASE_URL/ready"; then
      ready=1
      break
    fi
    if ! kill -0 "$SERVE_PID" 2>/dev/null; then
      break
    fi
    i=$((i + 1))
    sleep 0.2
  done
  if [ "$ready" != 1 ]; then
    echo "preflight.sh --local: $ROKO_BIN serve never answered $BASE_URL/ready; log:" >&2
    cat "$WORKDIR/serve.log" >&2
    exit 2
  fi

  export SHOWCASE_PASSPHRASE
  export SHOWCASE_ADMIN_KEY
fi

if [ -z "${SHOWCASE_PASSPHRASE:-}" ]; then
  echo "preflight.sh: SHOWCASE_PASSPHRASE is required (environment only, never an argument)" >&2
  exit 2
fi

U=$BASE_URL
HTTPS=0
case "$U" in
  https://*) HTTPS=1 ;;
esac

# ---------------------------------------------------------------------------- P1 health

expect_status "P1 health" GET /health 200

# ---------------------------------------------------------------------------- P2 anonymous 401

for path in /api/showcase/manifest /api/config /api/events; do
  expect_status "P2 anonymous $path" GET "$path" 401
done

# ---------------------------------------------------------------------------- P3 Privy JWT 401 (G0a)

# A real Privy JWT from `roko login`, when the operator has one; otherwise any well-formed Bearer
# JWT does, since showcase mode refuses every one regardless (the fixture here is
# crates/roko-serve/src/routes/middleware.rs's own test value: header.payload.signature, base64).
PRIVY_JWT=${ROKO_TEST_PRIVY_JWT:-eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiJkaWQ6cHJpdnk6eCJ9.c2lnbmF0dXJl}
for path in /api/showcase/manifest /api/config; do
  expect_status "P3 privy-jwt $path" GET "$path" 401 -H "Authorization: Bearer $PRIVY_JWT"
done

# ---------------------------------------------------------------------------- P4-P6 404 (G1)

for path in /metrics /api/shared/x /runs/x /ws/terminal/x /api/terminal/sessions; do
  expect_status "P4-P6 $path" GET "$path" 404
done

# ---------------------------------------------------------------------------- P10 CSRF and Origin

# check_csrf_and_origin runs before the passphrase is even checked, so a throwaway POST (wrong
# passphrase, no real cookie needed) proves it without touching P11's lockout counter.
THROWAWAY_BODY="{\"passphrase\":\"$(json_escape "not the real passphrase")\"}"
expect_status "P10 missing X-Roko-CSRF" POST /api/auth/session 403 \
  -H "Origin: $U" -H 'Content-Type: application/json' --data "$THROWAWAY_BODY"
expect_status "P10 foreign Origin" POST /api/auth/session 403 \
  -H 'X-Roko-CSRF: 1' -H 'Origin: https://not-the-showcase.example' \
  -H 'Content-Type: application/json' --data "$THROWAWAY_BODY"

# ---------------------------------------------------------------------------- P7 login

LOGIN_BODY="{\"passphrase\":\"$(json_escape "$SHOWCASE_PASSPHRASE")\"}"
LOGIN_HEADERS=$(mktemp)
LOGIN_STATUS=$(curl -s -o /dev/null -D "$LOGIN_HEADERS" -w '%{http_code}' -X POST \
  -H 'X-Roko-CSRF: 1' -H "Origin: $U" -H 'Content-Type: application/json' --data "$LOGIN_BODY" \
  "$U/api/auth/session")
SET_COOKIE=$(grep -i '^set-cookie:' "$LOGIN_HEADERS" | tail -1 | tr -d '\r')
rm -f "$LOGIN_HEADERS"
COOKIE_PAIR=$(printf '%s' "$SET_COOKIE" | sed -E 's/^[Ss]et-[Cc]ookie: *//' | cut -d';' -f1)

if [ "$LOGIN_STATUS" = "204" ] && [ -n "$COOKIE_PAIR" ]; then
  pass "P7 login"
else
  fail "P7 login" "expected 204 with a Set-Cookie, got status $LOGIN_STATUS, cookie '$COOKIE_PAIR'"
fi

case "$SET_COOKIE" in
  *HttpOnly*) pass "P7 cookie HttpOnly" ;;
  *) fail "P7 cookie HttpOnly" "Set-Cookie did not have HttpOnly" ;;
esac
case "$SET_COOKIE" in
  *SameSite=Strict*) pass "P7 cookie SameSite=Strict" ;;
  *) fail "P7 cookie SameSite=Strict" "Set-Cookie did not have SameSite=Strict" ;;
esac
case "$SET_COOKIE" in
  *Path=/*) pass "P7 cookie Path=/" ;;
  *) fail "P7 cookie Path=/" "Set-Cookie did not have Path=/" ;;
esac
if [ "$HTTPS" = 1 ]; then
  case "$SET_COOKIE" in
    *Secure*) pass "P7 cookie Secure" ;;
    *) fail "P7 cookie Secure" "Set-Cookie did not have Secure" ;;
  esac
  case "$COOKIE_PAIR" in
    __Host-*) pass "P7 cookie __Host- prefix" ;;
    *) fail "P7 cookie __Host- prefix" "cookie name '$COOKIE_PAIR' has no __Host- prefix" ;;
  esac
else
  skip "P7 cookie Secure" "plain HTTP (--local): Secure and __Host- need HTTPS, checked on a live base-url"
  skip "P7 cookie __Host- prefix" "plain HTTP (--local): see above"
fi

# ---------------------------------------------------------------------------- P8 manifest and stream

expect_status "P8 manifest" GET /api/showcase/manifest 200 -H "Cookie: $COOKIE_PAIR"

STREAM_STATUS=$(status_of GET /api/showcase/stream -H "Cookie: $COOKIE_PAIR")
if [ "$STREAM_STATUS" = "404" ]; then
  skip "P8 stream" "/api/showcase/stream answers 404: not wired yet"
elif curl -s -N --max-time 5 -H "Cookie: $COOKIE_PAIR" "$U/api/showcase/stream" | head -c 1 | grep -q .; then
  pass "P8 stream"
else
  fail "P8 stream" "no frame from /api/showcase/stream within 5s"
fi

# ---------------------------------------------------------------------------- P9 scope (403)

for path in /api/config /api/secrets /api/run; do
  expect_status "P9 $path" POST "$path" 403 -H "Cookie: $COOKIE_PAIR"
done

FREEZE_STATUS=$(status_of POST /api/showcase/admin/freeze -H "Cookie: $COOKIE_PAIR")
if [ "$FREEZE_STATUS" = "404" ]; then
  skip "P9 /api/showcase/admin/freeze" "answers 404: not wired yet"
elif [ "$FREEZE_STATUS" = "403" ]; then
  pass "P9 /api/showcase/admin/freeze"
else
  fail "P9 /api/showcase/admin/freeze" "expected 403, got $FREEZE_STATUS"
fi

# ---------------------------------------------------------------------------- P13 logout

LOGOUT_STATUS=$(status_of DELETE /api/auth/session -H "Cookie: $COOKIE_PAIR" \
  -H 'X-Roko-CSRF: 1' -H "Origin: $U")
if [ "$LOGOUT_STATUS" = "204" ]; then
  pass "P13 logout"
else
  fail "P13 logout" "DELETE /api/auth/session: expected 204, got $LOGOUT_STATUS"
fi
expect_status "P13 old cookie 401" GET /api/showcase/manifest 401 -H "Cookie: $COOKIE_PAIR"

# ---------------------------------------------------------------------------- P11 lockout

# The 6th wrong passphrase from this IP gets 429 (the default per_ip_max_failures is 5); admin
# login-unlock then clears it, verified by a login that would otherwise still be blocked.
WRONG_BODY="{\"passphrase\":\"$(json_escape "definitely not the passphrase")\"}"
tries=1
blocked=""
while [ "$tries" -le 6 ]; do
  status=$(status_of POST /api/auth/session -H 'X-Roko-CSRF: 1' -H "Origin: $U" \
    -H 'Content-Type: application/json' --data "$WRONG_BODY")
  if [ "$tries" = 6 ]; then
    blocked=$status
  fi
  tries=$((tries + 1))
done
if [ "$blocked" = "429" ]; then
  pass "P11 lockout after 6 failures"
else
  fail "P11 lockout after 6 failures" "the 6th bad passphrase gave $blocked, not 429"
fi

if [ -n "${SHOWCASE_ADMIN_KEY:-}" ]; then
  UNLOCK_STATUS=$(status_of POST /api/showcase/admin/login-unlock -H "X-Api-Key: $SHOWCASE_ADMIN_KEY")
  if [ "$UNLOCK_STATUS" = "204" ] || [ "$UNLOCK_STATUS" = "200" ]; then
    RETRY_STATUS=$(status_of POST /api/auth/session -H 'X-Roko-CSRF: 1' -H "Origin: $U" \
      -H 'Content-Type: application/json' --data "$LOGIN_BODY")
    if [ "$RETRY_STATUS" = "204" ]; then
      pass "P11 admin login-unlock clears the lockout"
    else
      fail "P11 admin login-unlock clears the lockout" \
        "a login with the right passphrase still got $RETRY_STATUS after unlock"
    fi
  else
    fail "P11 admin login-unlock clears the lockout" "POST login-unlock gave $UNLOCK_STATUS"
  fi
else
  skip "P11 admin login-unlock clears the lockout" "SHOWCASE_ADMIN_KEY is not set"
fi

# ---------------------------------------------------------------------------- live-only checks

if [ "$LIVE_CHECKS" = 1 ]; then
  # P12 needs the auth-audit log entry read back, which only an operator with access to the
  # deployed Machine can do; this sends the probe and leaves the log read to them.
  curl -s -o /dev/null -H 'Fly-Client-IP: 203.0.113.7' "$U/health"
  skip "P12 Fly-Client-IP" "sent a probe with Fly-Client-IP set; read the auth-audit entry by hand"

  BUDGET_STATUS=$(status_of GET /api/showcase/budget)
  if [ "$BUDGET_STATUS" = "404" ]; then
    skip "P14 budget" "/api/showcase/budget answers 404: not wired yet"
  elif [ "$BUDGET_STATUS" = "200" ]; then
    pass "P14 budget"
  else
    fail "P14 budget" "expected 200, got $BUDGET_STATUS"
  fi
else
  skip "P12 Fly-Client-IP" "needs --live-checks"
  skip "P14 budget" "needs --live-checks"
fi

echo "preflight.sh: $CHECKS checks, $FAILURES failed"
exit "$FAILURES"
