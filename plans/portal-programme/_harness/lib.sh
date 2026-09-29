# Shared helpers for the portal-programme live checks. Source it; do not run it.
#
# A check builds a throwaway workspace under /tmp, points its claude_cli
# provider at ./fake-claude (deterministic, free), starts `roko serve` there on
# a free port, drives the HTTP API, and tears everything down on exit.
#
# Why a separate workspace: it has its own .roko, so its locks, checkpoints and
# hub socket never meet an outer `roko plan run` in the repo. Why /tmp: the hub
# socket path (.roko/runtime/hub.sock) must fit the unix-socket length limit.
#
# Environment: ROKO_BIN overrides the binary (default: <repo>/target/debug/roko);
# KEEP_WS=1 keeps the workspace. A failing check always keeps it and prints it.
# Written for bash 3.2 as well as 5.x.

set -uo pipefail

HARNESS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$HARNESS_DIR/../../.." && pwd)"
ROKO_BIN="${ROKO_BIN:-$REPO_ROOT/target/debug/roko}"
WS=""
PORT=""
SERVER_PID=""
CAPTURE_PID=""
SSE_FILE=""
PASSES=0
FAILURES=0

# check NAME CMD...: run CMD and record PASS/FAIL; never aborts the check.
check() {
    local name="$1"
    shift
    if "$@"; then
        echo "PASS $name"
        PASSES=$((PASSES + 1))
    else
        echo "FAIL $name"
        FAILURES=$((FAILURES + 1))
    fi
}

# finish TITLE: print the verdict line and exit non-zero on any failure.
finish() {
    if [ "$FAILURES" -eq 0 ]; then
        echo "$1: PASS ($PASSES checks)"
        exit 0
    fi
    KEEP_WS=1
    echo "$1: FAIL ($FAILURES of $((PASSES + FAILURES)) checks failed; workspace kept at $WS)"
    exit 1
}

cleanup() {
    if [ -n "$CAPTURE_PID" ]; then kill "$CAPTURE_PID" 2>/dev/null; fi
    if [ -n "$SERVER_PID" ]; then
        kill -INT "$SERVER_PID" 2>/dev/null
        local waited=0
        while kill -0 "$SERVER_PID" 2>/dev/null && [ "$waited" -lt 10 ]; do
            sleep 1
            waited=$((waited + 1))
        done
        kill -9 "$SERVER_PID" 2>/dev/null
    fi
    if [ -n "$WS" ]; then
        pkill -f "$WS/fake-claude" 2>/dev/null
        if [ -z "${KEEP_WS:-}" ]; then rm -rf "$WS" "$WS.mirror"; fi
    fi
}
trap cleanup EXIT

require_binary() {
    if [ ! -x "$ROKO_BIN" ]; then
        echo "missing $ROKO_BIN; run: cargo build -p roko-cli"
        exit 2
    fi
}

# run_in_ws CMD...: run a command from the workspace root.
run_in_ws() {
    (cd "$WS" && "$@")
}

# fixture_plan DIR PLAN_ID: write a plan whose tasks are read from stdin as
# `ID|DEPENDS_ON (comma list or -)|DESCRIPTION|ARTIFACT_PATH` lines.
fixture_plan() {
    local dir="$WS/$1" plan_id="$2" count=0 lines=""
    lines="$(cat)"
    count="$(printf '%s\n' "$lines" | grep -c '|')"
    mkdir -p "$dir"
    {
        printf '[meta]\nplan = "%s"\ntotal = %s\ndone = 0\nstatus = "ready"\nmax_parallel = 1\n' \
            "$plan_id" "$count"
        printf '%s\n' "$lines" | while IFS='|' read -r id deps desc artifact; do
            [ -z "$id" ] && continue
            dep_list="[]"
            if [ "$deps" != "-" ]; then
                dep_list="[\"$(printf '%s' "$deps" | sed 's/,/", "/g')\"]"
            fi
            printf '\n[[task]]\nid = "%s"\ntitle = "Fixture task %s"\n' "$id" "$id"
            printf 'description = "%s ARTIFACT %s"\n' "$desc" "$artifact"
            printf 'status = "ready"\nrole = "implementer"\ntier = "focused"\n'
            printf 'model_hint = "claude-sonnet-4-6"\nmax_retries = 0\ntimeout_secs = 300\n'
            printf 'files = ["%s"]\ndepends_on = %s\n' "$artifact" "$dep_list"
            printf '\n[task.context]\nread_files = [{ path = "README.md", why = "fixture" }]\n'
            printf '\n[[task.verify]]\nphase = "structural"\ncommand = "test -f %s"\n' "$artifact"
            printf 'fail_msg = "%s was not written"\n' "$artifact"
        done
    } >"$dir/tasks.toml"
    printf '# %s\n\nLive-check fixture plan.\n' "$plan_id" >"$dir/plan.md"
}

# make_workspace: create $WS with the fake agent, config and fixture plans.
#   plans/live-a        T01 (slow, 12s) -> T02
#   plans/live-b        T01
#   plans/nested-set/live-nested  T01 -- a plan inside a plan set
#   slow-plans/live-slow  T01 (90s) -- outside plans/, so "run all" skips it
#   par-plans/par-a, par-plans/par-b  T01 (8s) each -- two independent plans for a
#                   parallel set run; outside plans/, so "run all" skips them
make_workspace() {
    WS="$(mktemp -d /tmp/roko-live-XXXXXX)"
    WS="$(cd "$WS" && pwd -P)"
    cp "$HARNESS_DIR/fake-claude" "$WS/fake-claude"
    chmod +x "$WS/fake-claude"
    mkdir -p "$WS/.roko" "$WS/plans" "$WS/slow-plans" "$WS/par-plans"
    printf '# live-check workspace\n' >"$WS/README.md"
    printf '.roko/\nout/\nhello/\n' >"$WS/.gitignore"
    cat >"$WS/roko.toml" <<EOF
config_version = 2
schema_version = 2

[agent]
default_model = "claude-sonnet-4-6"

[providers.claude_cli]
kind = "claude_cli"
command = "$WS/fake-claude"

[models.claude-sonnet-4-6]
provider = "claude_cli"
slug = "claude-sonnet-4-6"
context_window = 200000

[serve.auth]
enabled = false

[gates]
max_review_cycles = 0
cargo_fix_enabled = false
EOF
    fixture_plan plans/live-a live-a <<'EOF'
T01|-|Write the first artifact. SLOW 12|out/live-a-t01.txt
T02|T01|Write the second artifact.|out/live-a-t02.txt
EOF
    fixture_plan plans/live-b live-b <<'EOF'
T01|-|Write the only artifact.|out/live-b-t01.txt
EOF
    fixture_plan plans/nested-set/live-nested live-nested <<'EOF'
T01|-|Write the nested plan's artifact.|out/live-nested.txt
EOF
    fixture_plan slow-plans/live-slow live-slow <<'EOF'
T01|-|Write the artifact after a long wait. SLOW 90|out/live-slow.txt
EOF
    fixture_plan par-plans/par-a par-a <<'EOF'
T01|-|Write the artifact after a pause. SLOW 8|out/par-a.txt
EOF
    fixture_plan par-plans/par-b par-b <<'EOF'
T01|-|Write the artifact after a pause. SLOW 8|out/par-b.txt
EOF
    (cd "$WS" && git init -q && git add -A &&
        git -c user.email=live-check@roko -c user.name=live-check commit -qm fixture) ||
        { echo "could not initialise the workspace git repo"; exit 2; }
}

# add_legacy_fixture: an old-format plan (no model_hint, no read_files) that
# PRD generation's old-format regeneration pass would rewrite.
add_legacy_fixture() {
    mkdir -p "$WS/plans/legacy-fixture"
    cat >"$WS/plans/legacy-fixture/tasks.toml" <<'EOF'
[meta]
plan = "legacy-fixture"
total = 1
done = 0
status = "ready"
max_parallel = 1

[[task]]
id = "T01"
title = "Legacy fixture task"
description = "An old-format task. ARTIFACT out/legacy.txt"
status = "ready"
role = "implementer"
tier = "focused"
files = ["out/legacy.txt"]
depends_on = []

[[task.verify]]
phase = "structural"
command = "test -f out/legacy.txt"
fail_msg = "out/legacy.txt was not written"
EOF
    printf '# legacy-fixture\n\nAn old-format plan.\n' >"$WS/plans/legacy-fixture/plan.md"
    (cd "$WS" && git add -A && git -c user.email=live-check@roko -c user.name=live-check commit -qm legacy)
}

# start_server: start `roko serve` in $WS on a free port; sets PORT, SERVER_PID.
start_server() {
    PORT="$(python3 -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])')"
    (cd "$WS" && exec "$ROKO_BIN" serve --port "$PORT") >"$WS/serve.log" 2>&1 &
    SERVER_PID=$!
    local tries=0
    while [ "$tries" -lt 240 ]; do
        if curl -sf "http://127.0.0.1:$PORT/health" >/dev/null 2>&1; then return 0; fi
        if ! kill -0 "$SERVER_PID" 2>/dev/null; then break; fi
        sleep 0.5
        tries=$((tries + 1))
    done
    echo "roko serve did not become healthy on :$PORT"
    tail -30 "$WS/serve.log"
    exit 2
}

# stop_server: Ctrl-C the server and wait up to 15s for it to exit.
stop_server() {
    if [ -z "$SERVER_PID" ]; then return 0; fi
    kill -INT "$SERVER_PID" 2>/dev/null
    local waited=0
    while kill -0 "$SERVER_PID" 2>/dev/null && [ "$waited" -lt 15 ]; do
        sleep 1
        waited=$((waited + 1))
    done
    if kill -0 "$SERVER_PID" 2>/dev/null; then return 1; fi
    SERVER_PID=""
    return 0
}

# start_capture [NAME]: stream GET /api/events into $WS/NAME (default events.sse).
# Uses ?n=9999999999 so the server skips all historical replay and only streams
# events generated after this connection — prevents stale events from previous
# runs contaminating per-run assertions (e.g. "exactly one plan_started").
start_capture() {
    stop_capture
    SSE_FILE="$WS/${1:-events.sse}"
    curl -sN "http://127.0.0.1:$PORT/api/events?n=9999999999" >"$SSE_FILE" 2>/dev/null &
    CAPTURE_PID=$!
    sleep 1
}

stop_capture() {
    if [ -n "$CAPTURE_PID" ]; then
        sleep 1
        kill "$CAPTURE_PID" 2>/dev/null
        wait "$CAPTURE_PID" 2>/dev/null
        CAPTURE_PID=""
    fi
}

# sse COMMAND ARGS: assertion over the current capture (see sse.py).
sse() {
    python3 "$HARNESS_DIR/sse.py" "$SSE_FILE" "$@"
}

# api METHOD PATH [JSON]: print the HTTP status; the body lands in $WS/last.json.
api() {
    local method="$1" path="$2"
    if [ $# -ge 3 ]; then
        curl -s -o "$WS/last.json" -w '%{http_code}' -X "$method" \
            -H 'content-type: application/json' --data "$3" "http://127.0.0.1:$PORT$path"
    else
        curl -s -o "$WS/last.json" -w '%{http_code}' -X "$method" "http://127.0.0.1:$PORT$path"
    fi
}

# jget FILE EXPR: print a Python expression over the JSON document `d`.
jget() {
    python3 -c 'import json, sys; d = json.load(open(sys.argv[1])); print(eval(sys.argv[2]))' "$1" "$2" 2>/dev/null
}

# jcheck FILE EXPR: succeed when the Python expression over `d` is truthy.
jcheck() {
    python3 -c 'import json, sys; d = json.load(open(sys.argv[1])); sys.exit(0 if eval(sys.argv[2]) else 1)' "$1" "$2" 2>/dev/null
}

sha() {
    python3 -c 'import hashlib, sys; print(hashlib.sha256(open(sys.argv[1], "rb").read()).hexdigest())' "$1" 2>/dev/null
}

# same_sha HASH FILE: FILE exists and still hashes to the non-empty HASH.
same_sha() {
    [ -n "$1" ] && [ -f "$2" ] && [ "$1" = "$(sha "$2")" ]
}

# cli_validate PLAN_DIR: `roko plan validate` on a copy of the workspace, because
# the running server holds the workspace lock that the command would take.
cli_validate() {
    local mirror="$WS.mirror"
    rm -rf "$mirror"
    mkdir -p "$mirror/.roko"
    cp -R "$WS/plans" "$WS/README.md" "$WS/roko.toml" "$mirror/" &&
        (cd "$mirror" && "$ROKO_BIN" plan validate "$1")
}

# wait_idle KEY SECONDS: poll /api/plans/KEY/status until finished (or 404).
wait_idle() {
    local key="$1" deadline=$((SECONDS + $2)) code
    if [ -z "$key" ]; then return 1; fi
    while [ "$SECONDS" -lt "$deadline" ]; do
        code="$(curl -s -o "$WS/status.json" -w '%{http_code}' "http://127.0.0.1:$PORT/api/plans/$key/status")"
        if [ "$code" = 404 ]; then return 0; fi
        if [ "$code" = 200 ] && jcheck "$WS/status.json" 'd.get("finished") is True'; then return 0; fi
        sleep 1
    done
    return 1
}

# wait_plan_listed PLAN_ID SECONDS: poll GET /api/plans/PLAN_ID until 200.
wait_plan_listed() {
    local deadline=$((SECONDS + $2))
    if [ -z "$1" ]; then return 1; fi
    while [ "$SECONDS" -lt "$deadline" ]; do
        if [ "$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:$PORT/api/plans/$1")" = 200 ]; then
            return 0
        fi
        sleep 1
    done
    return 1
}

# wait_operation ID SECONDS: poll GET /api/operations/ID until it is no longer running;
# the final body is left in $WS/last.json.
wait_operation() {
    local deadline=$((SECONDS + $2))
    if [ -z "$1" ]; then return 1; fi
    while [ "$SECONDS" -lt "$deadline" ]; do
        if [ "$(api GET "/api/operations/$1")" = 200 ] && ! jcheck "$WS/last.json" 'd["status"] == "running"'; then
            return 0
        fi
        sleep 1
    done
    return 1
}

# fake_running: succeed while any fake agent process for this workspace runs.
fake_running() {
    pgrep -f "$WS/fake-claude" >/dev/null 2>&1
}

# wait_no_fake SECONDS: succeed once no fake agent process remains.
wait_no_fake() {
    local deadline=$((SECONDS + $1))
    while [ "$SECONDS" -lt "$deadline" ]; do
        if ! fake_running; then return 0; fi
        sleep 1
    done
    return 1
}

# wait_fake SECONDS: succeed once a fake agent process is running.
wait_fake() {
    local deadline=$((SECONDS + $1))
    while [ "$SECONDS" -lt "$deadline" ]; do
        if fake_running; then return 0; fi
        sleep 0.5
    done
    return 1
}
