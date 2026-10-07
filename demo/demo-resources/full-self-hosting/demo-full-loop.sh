#!/bin/bash
# Full self-hosting loop demo: prompt -> plan -> jobs -> match -> run -> observe.
# Usage: bash demo-full-loop.sh [serve-url]
# Requires: roko serve running, agents seeded, a provider configured

set -euo pipefail

ROKO="${ROKO:-roko}"
BASE="${1:-http://127.0.0.1:6677}/api"
PROMPT="Wire knowledge store queries into matchmaking scoring"
# The plan's directory name: the prompt's words, lowercased and joined by dashes.
PLAN_DIR="plans/$(printf '%s' "$PROMPT" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9]+/-/g; s/^-+//; s/-+$//')"

pause() {
    echo ""
    read -rp "  [press enter to continue] " < /dev/tty
    echo ""
}

echo "═══════════════════════════════════════════"
echo "  FULL SELF-HOSTING LOOP"
echo "═══════════════════════════════════════════"

# --- ACT 1: PLAN ---

echo ""
echo "ACT 1: WRITE A PLAN FROM A PROMPT"
echo "─────────────────────────────────"
echo "roko plan generate \"$PROMPT\""
$ROKO plan generate "$PROMPT" 2>&1
echo ""
$ROKO plan validate "$PLAN_DIR" 2>&1
echo ""
$ROKO plan run "$PLAN_DIR" --dry-run 2>&1
pause

# --- ACT 2: JOBS ---

echo "ACT 2: CREATE WORK ITEMS"
echo "────────────────────────"
$ROKO job create "Wire knowledge into matchmaking" --type coding_task --description "Query neuro store during match scoring. Agents with relevant past experience should rank higher." --priority high 2>&1
$ROKO job create "Research cold archival patterns" --type research --description "Survey cron-based and event-driven archival in distributed signal stores." 2>&1
echo ""
$ROKO job list 2>&1
pause

# --- ACT 3: MATCH ---

echo "ACT 3: FIND AGENTS"
echo "──────────────────"
echo "Matching for Rust coding task..."
curl -sf -X POST "$BASE/jobs/match" -H 'Content-Type: application/json' \
    -d '{"title":"Wire knowledge into matchmaking","skills":["rust","systems"],"reward":"2000 KORAI","minTier":"Verified"}' | python3 -c "
import sys, json
d = json.load(sys.stdin)
print(f'   {len(d[\"candidates\"])} candidate(s), fee: {d[\"totalFee\"]}, eta: ~{d[\"etaHours\"]}h')
for c in d['candidates']:
    print(f'   • {c[\"label\"]} ({c[\"tier\"]}, rep {c[\"reputation\"]}) → {c[\"bidShare\"]}')
" 2>/dev/null || echo "   (matchmaking not available — seed agents first)"
echo ""
echo "Matching for research task..."
curl -sf -X POST "$BASE/jobs/match" -H 'Content-Type: application/json' \
    -d '{"title":"Research cold archival","skills":["analysis","distributed systems"],"reward":"1000 KORAI"}' | python3 -c "
import sys, json
d = json.load(sys.stdin)
print(f'   {len(d[\"candidates\"])} candidate(s)')
for c in d['candidates']:
    print(f'   • {c[\"label\"]} ({c[\"tier\"]})')
" 2>/dev/null || echo "   (matchmaking not available)"
pause

# --- ACT 4: RUN ---

echo "ACT 4: RUN THE PLAN"
echo "───────────────────"
read -rp "  Run $PLAN_DIR now? This dispatches agents. [y/N] " answer < /dev/tty
if [[ "$answer" == [yY] ]]; then
    $ROKO run "$PLAN_DIR" 2>&1
else
    echo "   Skipped. Run it later with: roko run $PLAN_DIR"
fi
pause

# --- ACT 5: OBSERVE ---

echo "ACT 5: SYSTEM STATE"
echo "───────────────────"
echo "Health:"
curl -sf "$BASE/health" | python3 -c "
import sys, json
d = json.load(sys.stdin)
print(f'   status: {d[\"status\"]}, agents: {d[\"active_agents\"]}, plans: {d[\"active_plans\"]}, runs: {d[\"active_runs\"]}')
" 2>/dev/null

echo ""
echo "Job stats:"
curl -sf "$BASE/jobs/stats" | python3 -c "
import sys, json
d = json.load(sys.stdin)
print(f'   total: {d[\"total\"]}, by state: {d[\"by_state\"]}, by type: {d[\"by_type\"]}')
" 2>/dev/null

echo ""
echo "Fleet:"
curl -sf "$BASE/managed-agents" | python3 -c "
import sys, json
agents = json.load(sys.stdin)
print(f'   {len(agents)} agent(s)')
for a in agents[:5]:
    print(f'   • {a[\"id\"]:20} {a.get(\"tier\",\"—\"):10} {a.get(\"status\",\"?\")}')
" 2>/dev/null

echo ""
echo "Learning:"
echo "   Efficiency events: $(wc -l < .roko/learn/efficiency.jsonl 2>/dev/null || echo '0') entries"
echo "   Episodes: $(wc -l < .roko/episodes.jsonl 2>/dev/null || echo '0') entries"
echo "   Cascade router: $(test -f .roko/learn/cascade-router.json && echo 'present' || echo 'empty')"
echo "   Gate thresholds: $(test -f .roko/learn/gate-thresholds.json && echo 'present' || echo 'empty')"

echo ""
echo "═══════════════════════════════════════════"
echo "  Loop complete. Places to explore:"
echo ""
echo "  roko dashboard:  F2 Plans, F3 Agents, F9 Learning"
echo "  Portal:          the plan, its tasks.toml, Run"
echo "  Network:         Agents, Jobs, Learning, Swarm"
echo "═══════════════════════════════════════════"
