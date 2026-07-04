#!/usr/bin/env bash
# Seed roko-serve with demo data so the dashboard has content to display.
# Run AFTER roko-serve is up. Covers the demo-rehearsal Flow 2 happy path.
#
# Usage:  ./demo-seed.sh [base_url]
# Default base_url: http://127.0.0.1:6677

set -euo pipefail

BASE="${1:-http://127.0.0.1:6677}"
API="${BASE}/api"

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "=== Seeding demo data ==="
echo "Target: $API"
echo ""

# ─── 1. Create ideas ──────────────────────────────────────────────────
echo -e "${YELLOW}1. Creating PRD ideas${NC}"

IDEA1=$(curl -sf -X POST "${API}/prds/ideas" \
    -H 'Content-Type: application/json' \
    -d '{"text": "Add health check endpoint to roko-agent-server"}')
SLUG1=$(echo "$IDEA1" | jq -r '.slug')
echo -e "  ${GREEN}idea${NC} $SLUG1"

IDEA2=$(curl -sf -X POST "${API}/prds/ideas" \
    -H 'Content-Type: application/json' \
    -d '{"text": "Implement agent matchmaking with VCG auction mechanism"}')
SLUG2=$(echo "$IDEA2" | jq -r '.slug')
echo -e "  ${GREEN}idea${NC} $SLUG2"

IDEA3=$(curl -sf -X POST "${API}/prds/ideas" \
    -H 'Content-Type: application/json' \
    -d '{"text": "Wire dream consolidation into scheduled cron trigger"}')
SLUG3=$(echo "$IDEA3" | jq -r '.slug')
echo -e "  ${GREEN}idea${NC} $SLUG3"

# ─── 2. Draft + promote first idea ───────────────────────────────────
echo ""
echo -e "${YELLOW}2. Drafting + promoting first idea${NC}"

curl -sf -X POST "${API}/prds/${SLUG1}/draft" >/dev/null 2>&1 || true
echo -e "  ${GREEN}draft${NC} $SLUG1"

curl -sf -X POST "${API}/prds/${SLUG1}/promote" >/dev/null 2>&1 || true
echo -e "  ${GREEN}promoted${NC} $SLUG1"

# ─── 3. Create a plan ────────────────────────────────────────────────
echo ""
echo -e "${YELLOW}3. Creating a demo plan${NC}"

PLAN=$(curl -sf -X POST "${API}/plans" \
    -H 'Content-Type: application/json' \
    -d "{
        \"title\": \"Wire agent health checks\",
        \"description\": \"Add /health endpoint to roko-agent-server sidecar for liveness probes\",
        \"tasks\": [
            {\"id\": \"T1\", \"description\": \"Add health handler to agent-server routes\", \"files\": [\"crates/roko-agent-server/src/lib.rs\"]},
            {\"id\": \"T2\", \"description\": \"Include uptime and memory stats in health response\", \"depends_on\": [\"T1\"], \"files\": [\"crates/roko-agent-server/src/state.rs\"]},
            {\"id\": \"T3\", \"description\": \"Add integration test for health endpoint\", \"depends_on\": [\"T2\"], \"files\": [\"crates/roko-agent-server/tests/\"]},
            {\"id\": \"T4\", \"description\": \"Wire health probe into serve aggregator\", \"depends_on\": [\"T2\"], \"files\": [\"crates/roko-serve/src/routes/agents.rs\"]}
        ]
    }")
PLAN_ID=$(echo "$PLAN" | jq -r '.id')
echo -e "  ${GREEN}plan${NC} $PLAN_ID (4 tasks)"

# ─── 4. Create jobs ──────────────────────────────────────────────────
echo ""
echo -e "${YELLOW}4. Creating demo jobs${NC}"

JOB1=$(curl -sf -X POST "${API}/jobs" \
    -H 'Content-Type: application/json' \
    -d '{
        "title": "Research DeFi lending protocol patterns",
        "description": "Survey Aave, Compound, and Morpho architectures. Produce a comparison matrix.",
        "job_type": "research",
        "posted_by": "demo-operator"
    }')
JOB1_ID=$(echo "$JOB1" | jq -r '.id')
echo -e "  ${GREEN}job${NC} $JOB1_ID (research, open)"

JOB2=$(curl -sf -X POST "${API}/jobs" \
    -H 'Content-Type: application/json' \
    -d "{
        \"title\": \"Implement WebSocket heartbeat monitor\",
        \"description\": \"Build a TUI widget that shows agent heartbeat latency.\",
        \"job_type\": \"coding_task\",
        \"posted_by\": \"demo-operator\",
        \"plan_id\": \"$PLAN_ID\"
    }")
JOB2_ID=$(echo "$JOB2" | jq -r '.id')
echo -e "  ${GREEN}job${NC} $JOB2_ID (coding_task, open)"

# Assign + start the coding job
curl -sf -X POST "${API}/jobs/${JOB2_ID}/assign" \
    -H 'Content-Type: application/json' \
    -d '{"agent_id": "demo-agent"}' >/dev/null 2>&1
curl -sf -X POST "${API}/jobs/${JOB2_ID}/start" >/dev/null 2>&1
echo -e "  ${GREEN}started${NC} $JOB2_ID (now in_progress)"

# ─── 5. Register a demo agent ────────────────────────────────────────
echo ""
echo -e "${YELLOW}5. Registering a demo agent${NC}"

curl -sf -X POST "${API}/agents/register" \
    -H 'Content-Type: application/json' \
    -d '{
        "agent_id": "demo-agent",
        "label": "demo-implementer",
        "capabilities": ["coding", "research"],
        "domain_tags": ["rust", "web3"],
        "tier": "Verified",
        "reputation": 72,
        "skills": ["rust", "typescript", "solidity"],
        "past_jobs_completed": 14,
        "max_concurrent_jobs": 3
    }' >/dev/null 2>&1
echo -e "  ${GREEN}registered${NC} demo-agent (Verified, rep=72)"

# ─── Summary ──────────────────────────────────────────────────────────
echo ""
echo "========================================="
echo -e "${GREEN}Demo data seeded.${NC}"
echo ""
echo "Dashboard should now show:"
echo "  - PRDs tab: 3 ideas, 1 published"
echo "  - Plans tab: 1 plan with 4 tasks"
echo "  - Marketplace: 2 jobs (1 open, 1 in-progress)"
echo "  - Agents: 1 discovered agent (demo-agent)"
echo ""
echo "Try in the Atelier chat:"
echo "  /idea \"Build a gossip protocol for agent coordination\""
echo "  /plan $SLUG1"
echo "  /run $PLAN_ID"
