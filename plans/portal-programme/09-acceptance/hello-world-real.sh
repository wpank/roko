#!/usr/bin/env bash
# hello-world-real.sh — the user's path, with the real model, in a browser.
#
# A fresh folder, `roko init --profile rust`, `roko serve`, then the portal in a
# browser: sign in by the printed link, generate "a rust app that prints hello
# world", Run. The result must print hello world. No fixtures, no fake agent,
# no ROKO_SPA_DIR, auth as `roko init` leaves it. Dispatches real agents.
#
# Usage (from repo root, after `cargo build -p roko-cli` and a portal build):
#   bash plans/portal-programme/09-acceptance/hello-world-real.sh
# KEEP_WS=1 retains the workspace. ROKO_BIN overrides the binary.

unset ROKO_SPA_DIR

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../_harness/lib.sh"

require_binary

BROWSER_FLOW="$SCRIPT_DIR/browser-flow.cjs"
EVIDENCE_ROOT="$REPO_ROOT/tmp/portal-audit/evidence/hello-world-real"
TOKEN=""

authed_capture() {
    stop_capture
    SSE_FILE="$WS/${1:-events.sse}"
    curl -sN -H "Authorization: Bearer $TOKEN" \
        "http://127.0.0.1:$PORT/api/events?n=9999999999" >"$SSE_FILE" 2>/dev/null &
    CAPTURE_PID=$!
    sleep 1
}

portal_serves_portal() {
    curl -sf "http://127.0.0.1:$PORT/" 2>/dev/null | grep -q 'name="roko-portal"'
}
api_unauthorized() {
    [ "$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:$PORT/api/plans")" = 401 ]
}

# The Cargo.toml the plan created, outside target/ and .roko/.
find_cargo_toml() {
    find "$WS" -name Cargo.toml -not -path '*/target/*' -not -path '*/.roko/*' \
        -not -path '*/.git/*' 2>/dev/null | head -1
}
# The .rs file holding `fn main`, outside target/ and .roko/.
find_main_rs() {
    grep -rl --include='*.rs' 'fn main' "$WS" 2>/dev/null \
        | grep -v -e '/target/' -e '/.roko/' | head -1
}
# Run a command with a timeout (seconds); macOS has no `timeout`.
run_with_timeout() {
    local secs="$1"
    shift
    python3 - "$secs" "$@" <<'PY'
import subprocess, sys
secs = int(sys.argv[1])
try:
    r = subprocess.run(sys.argv[2:], capture_output=True, text=True, timeout=secs)
except subprocess.TimeoutExpired:
    print("TIMEOUT", file=sys.stderr)
    sys.exit(124)
sys.stdout.write(r.stdout)
sys.stderr.write(r.stderr)
sys.exit(r.returncode)
PY
}
says_hello() {
    grep -qiE 'hello,? world' "$1"
}
program_prints_hello() {
    local manifest main_rs
    manifest="$(find_cargo_toml)"
    if [ -n "$manifest" ]; then
        (cd "$(dirname "$manifest")" &&
            run_with_timeout 300 cargo run --quiet) >"$WS/program.out" 2>"$WS/program.err" ||
            return 1
    else
        main_rs="$(find_main_rs)"
        [ -n "$main_rs" ] || return 1
        rustc -o "$WS/hello-real-bin" "$main_rs" >"$WS/program.err" 2>&1 || return 1
        run_with_timeout 60 "$WS/hello-real-bin" >"$WS/program.out" 2>>"$WS/program.err" ||
            return 1
    fi
    says_hello "$WS/program.out"
}

# ── 1. A fresh folder ─────────────────────────────────────────────────────────
WS="$(mktemp -d /tmp/roko-hello-XXXXXX)"
WS="$(cd "$WS" && pwd -P)"
(cd "$WS" && git init -q) || { echo "git init failed"; exit 2; }
if ! (cd "$WS" && "$ROKO_BIN" init --profile rust . >"$WS/init.log" 2>&1); then
    echo "roko init --profile rust failed:"
    tail -20 "$WS/init.log"
    KEEP_WS=1
    exit 2
fi

# ── 2. Serve, sign-in link, portal, auth ─────────────────────────────────────
start_server
tries=0
while [ "$tries" -lt 60 ]; do
    TOKEN="$(sed -n 's/.*#token=\([0-9A-Za-z_-]*\).*/\1/p' "$WS/serve.log" 2>/dev/null | head -1)"
    [ -n "$TOKEN" ] && break
    sleep 0.5
    tries=$((tries + 1))
done
check "serve.log prints the portal link with a launch token" [ -n "$TOKEN" ]
check "/ serves the portal with no extra configuration" portal_serves_portal
check "an API request without a credential answers 401" api_unauthorized

# ── 3. Event capture ─────────────────────────────────────────────────────────
authed_capture

# ── 4. The two actions, in a browser, with the real model ────────────────────
mkdir -p "$WS/evidence"
browser_exit=0
node "$BROWSER_FLOW" real "http://127.0.0.1:$PORT/#token=$TOKEN" \
    "$WS/evidence" "$(basename "$WS")" >"$WS/browser-real.out" 2>&1 || browser_exit=$?
if [ "$browser_exit" = 3 ]; then
    reason="$(sed -n 's/^BROWSER-UNAVAILABLE //p' "$WS/browser-real.out" | head -1)"
    check "browser available (${reason:-unknown})" false
else
    while IFS= read -r line; do
        case "$line" in
            BROWSER\ *:\ PASS*)
                step="${line#BROWSER }"
                check "browser ${step%%:*}" true
                ;;
            BROWSER\ *:\ FAIL*)
                step="${line#BROWSER }"
                check "browser ${step%%:*}: ${line#*: FAIL }" false
                ;;
        esac
    done <"$WS/browser-real.out"
fi
SLUG="$(sed -n 's/^BROWSER-SLUG //p' "$WS/browser-real.out" | head -1)"
ACTIONS="$(sed -n 's/^BROWSER-ACTIONS //p' "$WS/browser-real.out" | head -1)"
check "the browser reported the generated plan's slug" [ -n "$SLUG" ]
check "BROWSER-ACTIONS <= 2" bash -c '[ -n "$1" ] && [ "$1" -le 2 ]' _ "$ACTIONS"

# ── 5. The event stream saw the run ──────────────────────────────────────────
sleep 2
if [ -n "$SLUG" ]; then
    check "sse: task_started for $SLUG" sse has task_started "plan_id=$SLUG"
    check "sse: agent_output for $SLUG" sse has agent_output "plan_id=$SLUG"
    check "sse: task_completed outcome=passed for $SLUG" \
        sse has task_completed "plan_id=$SLUG" "outcome=passed"
fi
check "sse: run_completed outcome=succeeded" sse has run_completed "outcome=succeeded"

# ── 6. The program prints hello world ────────────────────────────────────────
check "the program the plan built prints hello world" program_prints_hello

# ── 7. Cost and evidence ─────────────────────────────────────────────────────
COST="$(curl -s -H "Authorization: Bearer $TOKEN" \
    "http://127.0.0.1:$PORT/api/statehub/snapshot" 2>/dev/null |
    python3 -c 'import json, sys
def find(node):
    # The snapshot is wrapped in a state frame; search for stats.cost_usd_total.
    if isinstance(node, dict):
        if "cost_usd_total" in node:
            return node["cost_usd_total"]
        for value in node.values():
            found = find(value)
            if found is not None:
                return found
    elif isinstance(node, list):
        for value in node:
            found = find(value)
            if found is not None:
                return found
    return None
try:
    found = find(json.load(sys.stdin))
    print("unknown" if found is None else found)
except Exception:
    print("unknown")')"
echo "HELLO-WORLD-REAL cost_usd_total=$COST"

stop_capture
rm -rf "$EVIDENCE_ROOT"
mkdir -p "$EVIDENCE_ROOT"
cp "$WS/serve.log" "$WS/browser-real.out" "$EVIDENCE_ROOT/" 2>/dev/null
cp "$SSE_FILE" "$EVIDENCE_ROOT/events.sse" 2>/dev/null
cp "$WS"/evidence/* "$EVIDENCE_ROOT/" 2>/dev/null
cp "$WS/program.out" "$WS/program.err" "$EVIDENCE_ROOT/" 2>/dev/null
if [ -n "$SLUG" ]; then
    for dir in "$WS/plans/$SLUG" "$WS/.roko/plans/$SLUG"; do
        [ -f "$dir/tasks.toml" ] && cp "$dir/tasks.toml" "$EVIDENCE_ROOT/tasks.toml" && break
    done
fi
manifest="$(find_cargo_toml)"
if [ -n "$manifest" ]; then
    mkdir -p "$EVIDENCE_ROOT/program"
    cp "$manifest" "$EVIDENCE_ROOT/program/"
    [ -d "$(dirname "$manifest")/src" ] && cp -R "$(dirname "$manifest")/src" "$EVIDENCE_ROOT/program/"
else
    main_rs="$(find_main_rs)"
    [ -n "$main_rs" ] && cp "$main_rs" "$EVIDENCE_ROOT/"
fi
printf 'cost_usd_total=%s\nworkspace=%s\n' "$COST" "$WS" >"$EVIDENCE_ROOT/summary.txt"

finish "HELLO-WORLD-REAL"
