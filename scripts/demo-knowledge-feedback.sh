#!/usr/bin/env bash
# demo-knowledge-feedback.sh — deterministic knowledge-feedback illustration.
#
# The former --live mode targeted plans/live-demo-phase1 and
# plans/live-demo-phase2. Those manifests were deleted in 7899494d and are not
# current runnable roots. Live execution therefore fails closed; this script
# never creates those roots or passes them to `roko plan run`.
#
# The default mode writes two fixed simulated episodes and demonstrates the
# current keyword-overlap idea without network or model calls. It is an
# illustration, not proof that a live runner completed either historical plan.

set -euo pipefail

usage() {
    cat <<'EOF'
Usage:
  bash scripts/demo-knowledge-feedback.sh          Run deterministic simulation
  bash scripts/demo-knowledge-feedback.sh --help   Show this help
  bash scripts/demo-knowledge-feedback.sh --live   Fail closed (historical mode removed)

Environment:
  ROKO_DEMO_STATE_DIR  State directory for simulated output (default: <repo>/.roko)

The simulation performs no cargo build, API request, model dispatch, or plan run.
Set ROKO_DEMO_STATE_DIR to a temporary directory to keep the checkout unchanged.
EOF
}

case "${1:-}" in
    "") ;;
    --help|-h)
        usage
        exit 0
        ;;
    --live)
        cat >&2 <<'EOF'
ERROR: live knowledge-feedback demo is unavailable.

The historical live-demo-phase1 and live-demo-phase2 manifests were deleted in
commit 7899494d and are not current runnable plan roots. This script will not
recreate or execute them. Use the deterministic simulated mode, or author and
review a new current live fixture before restoring live execution.
EOF
        exit 2
        ;;
    *)
        echo "ERROR: unknown argument: $1" >&2
        usage >&2
        exit 2
        ;;
esac

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

command -v python3 >/dev/null
STATE_ROOT="$(python3 -c 'import os, sys; print(os.path.realpath(sys.argv[1]))' "${ROKO_DEMO_STATE_DIR:-$REPO_ROOT/.roko}")"
case "$STATE_ROOT/" in
    "$REPO_ROOT/plans/live-demo-phase1/"*|"$REPO_ROOT/plans/live-demo-phase2/"*)
        echo "ERROR: refusing to use a removed live-demo plan root as simulation state" >&2
        exit 2
        ;;
esac
EPISODES_FILE="$STATE_ROOT/learn/episodes.jsonl"

echo "=== Knowledge-Feedback Simulation ==="
echo "Mode: SIMULATED (no network, model, build, or plan execution)"
echo "State: $STATE_ROOT"
echo ""

echo "=== Step 1: Verify current source anchors ==="
test -f crates/roko-cli/src/runtime_feedback/episodes.rs
test -f crates/roko-cli/src/dispatch/prompt_builder.rs
echo "PASS: current episode sink and prompt-builder sources exist"
echo ""

mkdir -p "$STATE_ROOT/learn"

echo "=== Step 2: Write two fixed simulated episodes ==="
cat > "$EPISODES_FILE" <<'EOF'
{"kind":"agent_turn","id":"ep-001","timestamp":"2026-04-27T01:00:00Z","agent_id":"simulated-greeting/T1","task_id":"simulated-greeting/T1","model":"simulated-model","success":true,"turns":3,"tokens_used":4500,"cost_usd":0.0,"duration_secs":12.5,"gate_verdicts":[{"gate":"compile","passed":true}],"reasoning_summary":"Simulated prior work used a focused edit for a greeting helper.","failure_reason":null}
{"kind":"agent_turn","id":"ep-002","timestamp":"2026-04-27T01:01:00Z","agent_id":"simulated-greeting/T2","task_id":"simulated-greeting/T2","model":"simulated-model","success":true,"turns":2,"tokens_used":3200,"cost_usd":0.0,"duration_secs":8.0,"gate_verdicts":[{"gate":"test","passed":true}],"reasoning_summary":"Simulated prior work added a focused greeting test.","failure_reason":null}
EOF
echo "  Created 2 simulated episodes in $EPISODES_FILE"
echo ""

echo "=== Step 3: Show simulated prior episodes ==="
EPISODES_FILE="$EPISODES_FILE" python3 <<'PYEOF'
import json
import os

with open(os.environ["EPISODES_FILE"], encoding="utf-8") as stream:
    for line in stream:
        if not line.strip():
            continue
        episode = json.loads(line)
        status = "PASSED" if episode.get("success") else "FAILED"
        print(f"  {episode['task_id']}: {status} (model: {episode['model']})")
        print(f"    Simulated insight: {episode['reasoning_summary']}")
        print()
PYEOF

echo "=== Step 4: Demonstrate deterministic keyword matching ==="
echo ""
echo "Synthetic query: 'Add farewell helper function to greeting module'"
echo ""

EPISODES_FILE="$EPISODES_FILE" python3 <<'PYEOF'
import json
import os

task_title = "Add farewell helper function to greeting module"
keywords = {word.lower() for word in task_title.split() if len(word) > 2}

with open(os.environ["EPISODES_FILE"], encoding="utf-8") as stream:
    episodes = [json.loads(line) for line in stream if line.strip()]

scored = []
for episode in episodes:
    text = " ".join(
        [
            episode["task_id"],
            episode.get("failure_reason") or "",
            episode.get("reasoning_summary") or "",
        ]
    ).lower()
    overlap = sum(keyword in text for keyword in keywords)
    if overlap:
        scored.append((overlap, episode))

scored.sort(key=lambda item: (-item[1].get("success", False), -item[0]))

print("# Learned patterns from simulated prior episodes")
print()
for index, (_, episode) in enumerate(scored[:5], start=1):
    outcome = "passed" if episode.get("success") else "failed"
    print(f"{index}. {episode['task_id']} ({outcome}, {episode['model']})")
    print(f"   {episode['reasoning_summary']}")
PYEOF

echo ""
echo "=== Current production anchors (not exercised by this simulation) ==="
echo "  Write: crates/roko-cli/src/runtime_feedback/episodes.rs (EpisodeSink)"
echo "  Query/inject: crates/roko-cli/src/dispatch/prompt_builder.rs"
echo "                (collect_episode_knowledge / episode_knowledge section)"
echo ""
echo "=== Simulation complete ==="
