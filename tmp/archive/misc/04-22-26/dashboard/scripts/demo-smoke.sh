#!/usr/bin/env bash
# Smoke test the roko-serve ↔ dashboard API contract.
# Run this AFTER starting roko-serve (via dev-start.sh --serve-only or directly).
#
# Usage:  ./demo-smoke.sh [base_url]
# Default base_url: http://127.0.0.1:6677

set -euo pipefail

BASE="${1:-http://127.0.0.1:6677}"
API="${BASE}/api"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

PASS=0
FAIL=0

check() {
    local label="$1"
    local expr="$2"
    if eval "$expr" >/dev/null 2>&1; then
        echo -e "  ${GREEN}PASS${NC} $label"
        PASS=$((PASS + 1))
    else
        echo -e "  ${RED}FAIL${NC} $label"
        FAIL=$((FAIL + 1))
    fi
}

echo "=== roko-serve API smoke test ==="
echo "Base: $BASE"
echo ""

# ─── Health ────────────────────────────────────────────────────────────
echo -e "${YELLOW}1. Health${NC}"
HEALTH=$(curl -sf "${API}/health" || echo '{}')
check "GET /health returns JSON"         '[ -n "$HEALTH" ] && echo "$HEALTH" | jq . >/dev/null'
check "status is ok|degraded|down"       'echo "$HEALTH" | jq -e ".status | test(\"ok|degraded|down\")"'
check "has version"                      'echo "$HEALTH" | jq -e ".version"'
check "has uptime_secs (number)"         'echo "$HEALTH" | jq -e ".uptime_secs | type == \"number\""'
check "has active_agents (number)"       'echo "$HEALTH" | jq -e ".active_agents | type == \"number\""'
check "has active_plans (number)"        'echo "$HEALTH" | jq -e ".active_plans | type == \"number\""'

# ─── PRDs ──────────────────────────────────────────────────────────────
echo ""
echo -e "${YELLOW}2. PRDs${NC}"

# Create an idea
IDEA=$(curl -sf -X POST "${API}/prds/ideas" \
    -H 'Content-Type: application/json' \
    -d '{"text": "Smoke test idea for dashboard integration"}' || echo '{}')
check "POST /prds/ideas returns slug"    'echo "$IDEA" | jq -e ".slug"'
SLUG=$(echo "$IDEA" | jq -r '.slug // "none"')

# List PRDs
PRDS=$(curl -sf "${API}/prds" || echo '[]')
check "GET /prds returns array"          'echo "$PRDS" | jq -e "type == \"array\""'
check "PRD has slug field"               'echo "$PRDS" | jq -e ".[0].slug"'
check "PRD has title field"              'echo "$PRDS" | jq -e ".[0].title"'
check "PRD has status field"             'echo "$PRDS" | jq -e ".[0].status"'
check "PRD has has_plan field"           'echo "$PRDS" | jq -e ".[0] | has(\"has_plan\")"'
check "idea status is 'idea'"            "echo '$PRDS' | jq -e '.[] | select(.slug == \"$SLUG\") | .status == \"idea\"'"

# ─── Plans ─────────────────────────────────────────────────────────────
echo ""
echo -e "${YELLOW}3. Plans${NC}"

# Create a plan
PLAN=$(curl -sf -X POST "${API}/plans" \
    -H 'Content-Type: application/json' \
    -d '{"title":"Smoke test plan","description":"Test","tasks":[{"id":"T1","description":"do thing"}]}' || echo '{}')
check "POST /plans returns id"           'echo "$PLAN" | jq -e ".id"'
PLAN_ID=$(echo "$PLAN" | jq -r '.id // "none"')

# List plans
PLANS=$(curl -sf "${API}/plans" || echo '[]')
check "GET /plans returns array"               'echo "$PLANS" | jq -e "type == \"array\""'
check "plan has completed_task_count"          'echo "$PLANS" | jq -e ".[] | select(.id == \"$PLAN_ID\") | has(\"completed_task_count\")"'

# Plan detail
if [ "$PLAN_ID" != "none" ]; then
    DETAIL=$(curl -sf "${API}/plans/${PLAN_ID}" || echo '{}')
    check "GET /plans/:id has tasks array"     'echo "$DETAIL" | jq -e ".tasks | type == \"array\""'
    check "task has status field"              'echo "$DETAIL" | jq -e ".tasks[0].status"'
    check "task status is pending|completed"   'echo "$DETAIL" | jq -e ".tasks[0].status | test(\"pending|completed|running|failed\")"'
fi

# ─── Agents ────────────────────────────────────────────────────────────
echo ""
echo -e "${YELLOW}4. Agents${NC}"
AGENTS=$(curl -sf "${API}/managed-agents" || echo '[]')
check "GET /managed-agents returns array"   'echo "$AGENTS" | jq -e "type == \"array\""'
# If agents exist, check enrichment fields
if echo "$AGENTS" | jq -e '.[0]' >/dev/null 2>&1; then
    check "agent has id (number)"           'echo "$AGENTS" | jq -e ".[0].id | type == \"number\""'
    check "agent has status"                'echo "$AGENTS" | jq -e ".[0] | has(\"status\")"'
    check "agent has role"                  'echo "$AGENTS" | jq -e ".[0] | has(\"role\")"'
    check "agent has tier"                  'echo "$AGENTS" | jq -e ".[0] | has(\"tier\")"'
else
    echo -e "  ${YELLOW}SKIP${NC} no agents running — enrichment fields not tested"
fi

# ─── Jobs ──────────────────────────────────────────────────────────────
echo ""
echo -e "${YELLOW}5. Jobs${NC}"

# Create job
JOB=$(curl -sf -X POST "${API}/jobs" \
    -H 'Content-Type: application/json' \
    -d '{"title":"Smoke test job","description":"Testing API contract","job_type":"research","posted_by":"smoke-test"}' || echo '{}')
check "POST /jobs returns JSON"              'echo "$JOB" | jq . >/dev/null'
check "job uses 'state' field (not status)"  'echo "$JOB" | jq -e "has(\"state\")"'
check "job state is 'open'"                  'echo "$JOB" | jq -e ".state == \"open\""'
check "job has metadata field"               'echo "$JOB" | jq -e "has(\"metadata\")"'
check "job has required_capabilities"        'echo "$JOB" | jq -e "has(\"required_capabilities\")"'
JOB_ID=$(echo "$JOB" | jq -r '.id // "none"')

# List jobs
JOBS=$(curl -sf "${API}/jobs" || echo '[]')
check "GET /jobs returns array"              'echo "$JOBS" | jq -e "type == \"array\""'

# Job lifecycle: assign → start → submit → evaluate
if [ "$JOB_ID" != "none" ]; then
    ASSIGNED=$(curl -sf -X POST "${API}/jobs/${JOB_ID}/assign" \
        -H 'Content-Type: application/json' \
        -d '{"agent_id":"smoke-agent"}' || echo '{}')
    check "assign → state=assigned"          'echo "$ASSIGNED" | jq -e ".state == \"assigned\""'

    STARTED=$(curl -sf -X POST "${API}/jobs/${JOB_ID}/start" || echo '{}')
    check "start → state=in_progress"        'echo "$STARTED" | jq -e ".state == \"in_progress\""'

    SUBMITTED=$(curl -sf -X POST "${API}/jobs/${JOB_ID}/submit" \
        -H 'Content-Type: application/json' \
        -d '{"result_summary":"done","artifacts":[]}' || echo '{}')
    check "submit → state=submitted"         'echo "$SUBMITTED" | jq -e ".state == \"submitted\""'

    EVALUATED=$(curl -sf -X POST "${API}/jobs/${JOB_ID}/evaluate" \
        -H 'Content-Type: application/json' \
        -d '{"accepted":true,"score":0.95,"feedback":"good"}' || echo '{}')
    check "evaluate → state=evaluated"       'echo "$EVALUATED" | jq -e ".state == \"evaluated\""'
    check "evaluation has score"             'echo "$EVALUATED" | jq -e ".evaluation.score == 0.95"'
fi

# ─── WebSocket ─────────────────────────────────────────────────────────
echo ""
echo -e "${YELLOW}6. WebSocket${NC}"
# Test the /roko-ws path exists (upgrade request, expect 400 not 404)
WS_STATUS=$(curl -sf -o /dev/null -w "%{http_code}" \
    -H "Connection: Upgrade" \
    -H "Upgrade: websocket" \
    -H "Sec-WebSocket-Version: 13" \
    -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" \
    "${BASE}/roko-ws" 2>/dev/null || echo "000")
check "/roko-ws responds (not 404)"     '[ "$WS_STATUS" != "404" ] && [ "$WS_STATUS" != "000" ]'

WS_OLD=$(curl -sf -o /dev/null -w "%{http_code}" \
    -H "Connection: Upgrade" \
    -H "Upgrade: websocket" \
    -H "Sec-WebSocket-Version: 13" \
    -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" \
    "${BASE}/ws" 2>/dev/null || echo "000")
check "/ws responds (backwards compat)"  '[ "$WS_OLD" != "404" ] && [ "$WS_OLD" != "000" ]'

# ─── SSE ───────────────────────────────────────────────────────────────
echo ""
echo -e "${YELLOW}7. SSE${NC}"
SSE_STATUS=$(curl -sf -o /dev/null -w "%{http_code}" \
    -H "Accept: text/event-stream" \
    --max-time 2 \
    "${API}/events" 2>/dev/null || echo "200")
check "GET /api/events responds"         '[ "$SSE_STATUS" != "404" ]'

# ─── New endpoints ─────────────────────────────────────────────────────
echo ""
echo -e "${YELLOW}8. New endpoints${NC}"

NEURO=$(curl -sf -X POST "${API}/neuro/query" \
    -H 'Content-Type: application/json' \
    -d '{"query":"test","limit":1}' || echo '{}')
check "POST /neuro/query returns JSON"   'echo "$NEURO" | jq -e "has(\"results\", \"total\")"'

CONSOLIDATE_STATUS=$(curl -sf -o /dev/null -w "%{http_code}" \
    -X POST "${API}/prds/consolidate" \
    -H 'Content-Type: application/json' \
    -d '{}' 2>/dev/null || echo "000")
check "POST /prds/consolidate responds"  '[ "$CONSOLIDATE_STATUS" != "404" ]'

DREAM=$(curl -sf -X POST "${API}/dream/run" \
    -H 'Content-Type: application/json' \
    -d '{"mode":"quick"}' || echo '{}')
check "POST /dream/run returns op id"    'echo "$DREAM" | jq -e ".id"'

# ─── Summary ───────────────────────────────────────────────────────────
echo ""
echo "========================================="
TOTAL=$((PASS + FAIL))
echo -e "Results: ${GREEN}${PASS}${NC} passed, ${RED}${FAIL}${NC} failed out of ${TOTAL} checks"
if [ "$FAIL" -eq 0 ]; then
    echo -e "${GREEN}All checks passed!${NC}"
else
    echo -e "${RED}Some checks failed — see above.${NC}"
    exit 1
fi
