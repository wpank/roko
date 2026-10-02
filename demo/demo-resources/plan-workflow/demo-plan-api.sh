#!/bin/bash
# Demo the plan workflow via the HTTP API (the same endpoints the portal uses).
# Usage: bash demo-plan-api.sh [base-url] ["<prompt>"]
# Requires: roko serve running. Run from the served workspace so the launch
# token in .roko/runtime/serve.token can be read, or set ROKO_API_KEY.

set -euo pipefail

BASE="${1:-${ROKO_SERVE_URL:-http://127.0.0.1:6677}}"
BASE="${BASE%/}/api"
PROMPT="${2:-Wire knowledge into matchmaking}"
KEY="${ROKO_API_KEY:-$(cat .roko/runtime/serve.token 2>/dev/null || true)}"
if [[ -z "$KEY" ]]; then
    echo "No API key: set ROKO_API_KEY, or run from the workspace roko serve is serving." >&2
    exit 1
fi
AUTH=(-H "X-Api-Key: $KEY")

pause() {
    echo ""
    read -rp "  [press enter to continue] " < /dev/tty
    echo ""
}

json_field() {
    python3 -c 'import json, sys
value = json.load(sys.stdin)
for key in sys.argv[1].split("."):
    value = value.get(key, "") if isinstance(value, dict) else ""
print(value)' "$1"
}

echo "═══════════════════════════════════════════"
echo "  PLAN WORKFLOW DEMO (HTTP API)"
echo "═══════════════════════════════════════════"

echo ""
echo "STEP 1: WRITE A PLAN FROM A PROMPT"
echo "   POST /api/plans/generate"
BODY="$(python3 -c 'import json, sys; print(json.dumps({"prompt": sys.argv[1]}))' "$PROMPT")"
R=$(curl -sf -X POST "$BASE/plans/generate" "${AUTH[@]}" \
    -H 'Content-Type: application/json' -d "$BODY") || {
    echo "   POST /api/plans/generate failed; is roko serve running at $BASE?"
    exit 1
}
OP=$(echo "$R" | json_field id)
PLAN=$(echo "$R" | json_field plan_id)
echo "   operation: $OP"
echo "   plan:      $PLAN"

echo ""
echo "STEP 2: WAIT FOR THE PLAN"
echo "   GET /api/operations/$OP"
while :; do
    S=$(curl -sf "$BASE/operations/$OP" "${AUTH[@]}") || {
        echo ""
        echo "   GET /api/operations/$OP failed"
        exit 1
    }
    case "$(echo "$S" | json_field status)" in
        running) printf '.'; sleep 5 ;;
        completed) echo ""; echo "   $S"; break ;;
        *) echo ""; echo "   plan generation failed: $S"; exit 1 ;;
    esac
done
[[ -n "$PLAN" ]] || PLAN=$(echo "$S" | json_field result.slug)
pause

echo "STEP 3: REVIEW THE PLAN"
echo "   GET /api/plans/$PLAN/source (first 40 lines)"
curl -sf "$BASE/plans/$PLAN/source" "${AUTH[@]}" | json_field toml | sed -n '1,40p'
echo ""
echo "   PUT /api/plans/$PLAN/source with {\"toml\": \"...\"} saves an edit;"
echo "   the server validates it first and rejects an invalid plan with 422."
pause

echo "STEP 4: VALIDATE THE PLAN"
echo "   POST /api/plans/$PLAN/validate"
curl -sf -X POST "$BASE/plans/$PLAN/validate" "${AUTH[@]}" | python3 -m json.tool
pause

echo "STEP 5: RUN THE PLAN"
read -rp "  Run it now? This dispatches agents. [y/N] " answer < /dev/tty
if [[ "$answer" == [yY] ]]; then
    echo "   POST /api/plans/$PLAN/execute"
    curl -sf -X POST "$BASE/plans/$PLAN/execute" "${AUTH[@]}" | python3 -m json.tool
    echo "   GET /api/plans/$PLAN/status"
    curl -sf "$BASE/plans/$PLAN/status" "${AUTH[@]}" | python3 -m json.tool
else
    echo "   Skipped. Run it later with POST /api/plans/$PLAN/execute."
fi

echo ""
echo "═══════════════════════════════════════════"
echo "  Done. Watch the plan in the portal, or in"
echo "  roko dashboard (F2 Plans, F3 Agents)."
echo "═══════════════════════════════════════════"
