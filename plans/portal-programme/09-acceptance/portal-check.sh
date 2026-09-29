#!/usr/bin/env bash
# portal-check.sh — end-to-end acceptance for the Roko portal.
#
# A. Two-action flow in a fresh authenticated workspace with the fake agent.
# B. Two plans running in parallel via browser run-all.
# C. Copy all evidence to $REPO_ROOT/tmp/portal-audit/evidence/portal-check/.
#
# Usage (from repo root, after `cargo build -p roko-cli` and a portal build):
#   bash plans/portal-programme/09-acceptance/portal-check.sh
# KEEP_WS=1 retains workspaces. ROKO_BIN overrides the binary.

# The portal must come from apps/portal/out, not from an env override.
unset ROKO_SPA_DIR

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../_harness/lib.sh"

require_binary

BROWSER_FLOW="$SCRIPT_DIR/browser-flow.cjs"
EVIDENCE_ROOT="$REPO_ROOT/tmp/portal-audit/evidence/portal-check"
mkdir -p "$EVIDENCE_ROOT"

# Collect every workspace created in this run so we can clean them all.
ALL_WS=()

_portal_extra_cleanup() {
    # lib.sh's cleanup (already wired via trap cleanup EXIT when we sourced)
    # handles the current $WS.  Clean any earlier workspaces here.
    local ws_path
    for ws_path in "${ALL_WS[@]:-}"; do
        [ "$ws_path" = "${WS:-}" ] && continue
        pkill -f "$ws_path/fake-claude" 2>/dev/null || true
        if [ -z "${KEEP_WS:-}" ] && [ "${FAILURES:-0}" -eq 0 ]; then
            rm -rf "$ws_path" "$ws_path.mirror" 2>/dev/null || true
        fi
    done
}
# Override lib.sh's single-workspace trap with one that handles all of them.
trap 'cleanup; _portal_extra_cleanup' EXIT

# ── TOKEN and authed helpers (set by run_flow_check) ─────────────────────────
TOKEN=""

authed_api() {
    local method="$1" path="$2"
    if [ $# -ge 3 ]; then
        curl -s -o "$WS/last.json" -w '%{http_code}' -X "$method" \
            -H 'content-type: application/json' \
            -H "Authorization: Bearer $TOKEN" \
            --data "$3" "http://127.0.0.1:$PORT$path"
    else
        curl -s -o "$WS/last.json" -w '%{http_code}' -X "$method" \
            -H "Authorization: Bearer $TOKEN" \
            "http://127.0.0.1:$PORT$path"
    fi
}

authed_capture() {
    stop_capture
    SSE_FILE="$WS/${1:-events.sse}"
    curl -sN -H "Authorization: Bearer $TOKEN" \
        "http://127.0.0.1:$PORT/api/events?n=9999999999" >"$SSE_FILE" 2>/dev/null &
    CAPTURE_PID=$!
    sleep 1
}

# ══════════════════════════════════════════════════════════════════════════════
# A: Two-action flow
# ══════════════════════════════════════════════════════════════════════════════

# make_portal_workspace: lib.sh's make_workspace without any fixture plans,
# with auth enabled and the fake-agent slow knob set to 8 s.
make_portal_workspace() {
    make_workspace
    ALL_WS+=("$WS")
    rm -rf "$WS/plans" "$WS/slow-plans" "$WS/par-plans"
    # Enable auth (same form as roko init writes it)
    python3 - "$WS/roko.toml" <<'PY'
import sys
path = sys.argv[1]
text = open(path).read()
open(path, "w").write(
    text.replace(
        "[serve.auth]\nenabled = false",
        '[serve.auth]\nenabled = true\napi_key = ""',
        1
    )
)
PY
    touch "$WS/.roko/fake-claude-live"
    printf '20\n' >"$WS/.roko/fake-claude-slow"
    (cd "$WS" && git add -A &&
        git -c user.email=portal-check@roko -c user.name=portal-check \
            commit -qm 'portal workspace: no plans, auth on, live+slow')
}

FLOW_WS=""
FLOW_EVIDENCE=""

# ── HTTP check helpers (use $PORT and $TOKEN globals) ────────────────────────
portal_serves_portal() {
    curl -sf "http://127.0.0.1:$PORT/" 2>/dev/null | grep -q 'name="roko-portal"'
}
portal_static_js_ok() {
    local first_js
    first_js="$(curl -sf "http://127.0.0.1:$PORT/" 2>/dev/null \
        | grep -oE '/_next/static/[^"'"'"']+\.js' | head -1)"
    [ -n "$first_js" ] || return 1
    [ "$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:$PORT$first_js")" = 200 ]
}
demo_app_ok() {
    curl -sf "http://127.0.0.1:$PORT/demo/" 2>/dev/null | grep -q '/demo/assets/'
}
plans_unauthorized() {
    [ "$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:$PORT/api/plans")" = 401 ]
}
plans_empty_authed() {
    local code
    code="$(curl -s -o "$WS/plans-check.json" -w '%{http_code}' \
        -H "Authorization: Bearer $TOKEN" "http://127.0.0.1:$PORT/api/plans")"
    [ "$code" = 200 ] || return 1
    python3 -c \
        'import json, sys; sys.exit(0 if json.load(open(sys.argv[1])) == [] else 1)' \
        "$WS/plans-check.json"
}
hello_prints_hello() {
    (cd "$FLOW_WS" && ./hello/hello-bin 2>/dev/null | grep -q 'hello world')
}

run_flow_check() {
    make_portal_workspace
    FLOW_WS="$WS"
    FLOW_EVIDENCE="$WS/flow-evidence"
    mkdir -p "$FLOW_EVIDENCE"

    start_server

    # Read the launch token from serve.log
    local tries=0
    while [ "$tries" -lt 60 ]; do
        TOKEN="$(sed -n 's/.*#token=\([0-9A-Za-z_-]*\).*/\1/p' "$WS/serve.log" 2>/dev/null | head -1)"
        [ -n "$TOKEN" ] && break
        sleep 0.5
        tries=$((tries + 1))
    done
    check "03c: launch token present in serve.log" [ -n "$TOKEN" ]

    authed_capture

    # ── HTTP checks ───────────────────────────────────────────────────────
    check "serve.log announces tool_steps live mode" \
        grep -q 'live agent output: tool_steps' "$WS/serve.log"
    check "/ serves the portal (name=\"roko-portal\")" \
        portal_serves_portal
    check "first /_next/static JS answers 200" \
        portal_static_js_ok
    check "/demo/ serves the demo app (/demo/assets/ in HTML)" \
        demo_app_ok
    check "GET /api/plans is 401 without credential" \
        plans_unauthorized
    check "GET /api/plans returns [] with credential" \
        plans_empty_authed

    # ── Browser flow ──────────────────────────────────────────────────────
    local PORTAL_WS_NAME browser_exit
    PORTAL_WS_NAME="$(basename "$WS")"
    browser_exit=0
    node "$BROWSER_FLOW" flow \
        "http://127.0.0.1:$PORT/#token=$TOKEN" \
        "$FLOW_EVIDENCE" "$PORTAL_WS_NAME" \
        >"$WS/browser-flow.out" 2>&1 || browser_exit=$?

    if [ "$browser_exit" = 3 ]; then
        local reason
        reason="$(grep '^BROWSER-UNAVAILABLE ' "$WS/browser-flow.out" 2>/dev/null \
            | sed 's/^BROWSER-UNAVAILABLE //' | head -1)"
        check "browser unavailable: ${reason:-unknown}" false
    else
        # Turn each BROWSER step line into a check
        while IFS= read -r line; do
            case "$line" in
                BROWSER\ *:\ PASS*)
                    local step_name
                    step_name="${line#BROWSER }"
                    step_name="${step_name%%:*}"
                    check "browser $step_name" true
                    ;;
                BROWSER\ *:\ FAIL*)
                    local step_name step_detail
                    step_name="${line#BROWSER }"
                    step_name="${step_name%%:*}"
                    step_detail="${line#*: FAIL }"
                    check "browser $step_name: $step_detail" false
                    ;;
            esac
        done <"$WS/browser-flow.out"
    fi

    # Extract slug and action count from browser output
    local SLUG ACTIONS
    SLUG="$(grep '^BROWSER-SLUG ' "$WS/browser-flow.out" 2>/dev/null \
        | sed 's/^BROWSER-SLUG //' | head -1)"
    ACTIONS="$(grep '^BROWSER-ACTIONS ' "$WS/browser-flow.out" 2>/dev/null \
        | sed 's/^BROWSER-ACTIONS //' | head -1)"

    # ── Post-flow checks ──────────────────────────────────────────────────
    check "hello/hello-bin prints hello world" hello_prints_hello
    check "BROWSER-ACTIONS <= 2" \
        bash -c '[ -n "$1" ] && [ "$1" -le 2 ]' _ "$ACTIONS"

    # ── SSE capture assertions ────────────────────────────────────────────
    if [ -n "$SLUG" ]; then
        check "sse: task_started for plan $SLUG" \
            sse has task_started "plan_id=$SLUG"
        check "sse: live Write hello/main.rs before task_completed" \
            sse before agent_output "plan_id=$SLUG" \
                'content~"kind":"tool_start"' \
                'content~"live":true' \
                'content~"target":"hello/main.rs"' \
                -- task_completed "plan_id=$SLUG"
        check "sse: agent_heartbeat for plan $SLUG" \
            sse has agent_heartbeat "plan_id=$SLUG"
        check "sse: task_completed outcome=passed" \
            sse has task_completed "plan_id=$SLUG" outcome=passed
        check "sse: run_completed succeeded with positive duration_ms" \
            sse positive run_completed duration_ms outcome=succeeded
        check "sse: at least 2 plan_started events (two runs: generate + run-again)" \
            sse count plan_started "plan_id=$SLUG" --ge 2
    fi

    stop_capture
    check "server stops on Ctrl-C (flow)" stop_server
}

# ══════════════════════════════════════════════════════════════════════════════
# B: Two plans in parallel
# ══════════════════════════════════════════════════════════════════════════════

PAR_WS=""
PAR_EVIDENCE=""

run_parallel_check() {
    make_workspace        # auth off (lib.sh default)
    ALL_WS+=("$WS")
    PAR_WS="$WS"
    PAR_EVIDENCE="$WS/par-evidence"
    mkdir -p "$PAR_EVIDENCE"

    # Keep only par-a and par-b under plans/
    mv "$WS/par-plans/par-a" "$WS/plans/"
    mv "$WS/par-plans/par-b" "$WS/plans/"
    rm -rf "$WS/plans/live-a" "$WS/plans/live-b" "$WS/plans/nested-set" \
           "$WS/slow-plans" "$WS/par-plans"

    # Allow two plans to run at once
    printf '\n[conductor]\nmax_parallel_plans = 2\n' >>"$WS/roko.toml"

    (cd "$WS" && git add -A &&
        git -c user.email=portal-check@roko -c user.name=portal-check \
            commit -qm 'parallel workspace: par-a, par-b, max_parallel_plans=2')

    start_server
    start_capture par-events.sse

    local browser_exit=0
    node "$BROWSER_FLOW" parallel "http://127.0.0.1:$PORT/" "$PAR_EVIDENCE" \
        >"$WS/browser-parallel.out" 2>&1 || browser_exit=$?

    if [ "$browser_exit" = 3 ]; then
        local reason
        reason="$(grep '^BROWSER-UNAVAILABLE ' "$WS/browser-parallel.out" 2>/dev/null \
            | sed 's/^BROWSER-UNAVAILABLE //' | head -1)"
        check "browser unavailable (parallel): ${reason:-unknown}" false
    else
        while IFS= read -r line; do
            case "$line" in
                BROWSER\ *:\ PASS*)
                    local step_name
                    step_name="${line#BROWSER }"
                    step_name="${step_name%%:*}"
                    check "parallel: browser $step_name" true
                    ;;
                BROWSER\ *:\ FAIL*)
                    local step_name step_detail
                    step_name="${line#BROWSER }"
                    step_name="${step_name%%:*}"
                    step_detail="${line#*: FAIL }"
                    check "parallel: browser $step_name: $step_detail" false
                    ;;
            esac
        done <"$WS/browser-parallel.out"
    fi

    check "sse: second plan_started before first plan_completed" \
        sse between plan_started -- plan_started -- plan_completed
    check "out/par-a.txt exists" test -f "$WS/out/par-a.txt"
    check "out/par-b.txt exists" test -f "$WS/out/par-b.txt"

    stop_capture
    check "server stops on Ctrl-C (parallel)" stop_server
}

# ══════════════════════════════════════════════════════════════════════════════
# Run all sections
# ══════════════════════════════════════════════════════════════════════════════
run_flow_check
run_parallel_check

# ══════════════════════════════════════════════════════════════════════════════
# C: Collect evidence
# ══════════════════════════════════════════════════════════════════════════════
_copy_evidence() {
    local ws_path="$1"
    [ -d "$ws_path" ] || return
    # SSE captures
    find "$ws_path" -maxdepth 1 -name '*.sse' -exec cp -f {} "$EVIDENCE_ROOT/" \; 2>/dev/null || true
    # Server log
    [ -f "$ws_path/serve.log" ] && cp -f "$ws_path/serve.log" "$EVIDENCE_ROOT/serve-$(basename "$ws_path").log" 2>/dev/null || true
    # Browser JSON and screenshots
    find "$ws_path" -name 'browser-*.json' -exec cp -f {} "$EVIDENCE_ROOT/" \; 2>/dev/null || true
    find "$ws_path" -name '*.png'          -exec cp -f {} "$EVIDENCE_ROOT/" \; 2>/dev/null || true
    # Evidence sub-directories (flow-evidence, par-evidence)
    for evdir in "$ws_path/flow-evidence" "$ws_path/par-evidence"; do
        [ -d "$evdir" ] && cp -rf "$evdir/." "$EVIDENCE_ROOT/" 2>/dev/null || true
    done
}
for ws_path in "${ALL_WS[@]}"; do
    _copy_evidence "$ws_path"
done

finish "PORTAL-CHECK"
