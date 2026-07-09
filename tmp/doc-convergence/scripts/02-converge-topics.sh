#!/usr/bin/env bash
# Phase 2: Converge topics (parallel agents)
#
# Usage:
#   ./02-converge-topics.sh              # Run all topics
#   ./02-converge-topics.sh 05-AGENT     # Run single topic
#   MAX_PARALLEL=3 ./02-converge-topics.sh  # Limit parallelism
source "$(dirname "$0")/_common.sh"

TEMPLATE=$(<"$PROMPTS_DIR/02-converge-topic.md")

# Topic source mappings — expand as needed after Phase 1 refines the matrix
# These are the v2 topic IDs and their source file mappings
declare -A TOPIC_V1_FILES
declare -A TOPIC_V2_DEPTH_FILES
declare -A TOPIC_TMP_PRD_FILES
declare -A TOPIC_CRATE_PATHS
declare -A TOPIC_TITLES

TOPIC_TITLES=(
  ["01-SIGNAL"]="Signal, Pulse, Bus, Store, Demurrage, HDC"
  ["02-CELL"]="Cell, Protocols, Composition, Verification"
  ["03-GRAPH"]="Graph, DAG Composition, Hot Graphs"
  ["04-EXECUTION"]="Execution Engine, Cognitive Loop, Runtime"
  ["05-AGENT"]="Agent Runtime, Daimon, Heartbeat, Lifecycle"
  ["06-MEMORY"]="Memory, Knowledge, Neuro, Dreams"
  ["07-LEARNING"]="Learning Loops, Calibration, Experiments"
  ["08-GATEWAY"]="Inference Gateway, Model Routing, Caching"
  ["09-FEEDS"]="Feeds, Recipes, Data Streams"
  ["10-GROUPS"]="Groups, Coordination, Stigmergy"
  ["11-CONNECTIVITY"]="Connectivity, Relay, MCP, A2A"
  ["12-EXTENSIONS"]="Extension System, Hooks, CaMeL IFC"
  ["13-TRIGGERS"]="Trigger System, Conductor, Watchers"
  ["14-TOOLS"]="Tool Catalog, MCP Integration, Plugins"
  ["15-TELEMETRY"]="Telemetry, StateHub, c-factor, Metrics"
  ["16-SECURITY"]="Security Model, Taint, Immune System, Sandbox"
  ["17-AUTH"]="Authentication, Team Workspaces, Tokens"
  ["18-PAYMENTS"]="Payments, x402, Settlement"
  ["19-CONFIG"]="Configuration, Schema, Migration"
  ["20-SURFACES"]="CLI, HTTP API, TUI, Web Dashboard"
  ["21-MARKETPLACE"]="Marketplace, Reputation, Commerce"
  ["22-REGISTRIES"]="On-Chain Registries, ERC-8004, ZK-HDC"
  ["23-ARENAS"]="Arenas, Evals, Leaderboards, Bounties"
  ["24-DEFI"]="DeFi, ISFR Oracle, Yield Perps, Clearing"
  ["25-DEPLOYMENT"]="Deployment, Railway, Fly, Docker, Daemon"
  ["26-CROSS-CUTS"]="Cross-Cut Functors, Memory/Daimon/Dreams"
  ["27-ORCHESTRATOR"]="Orchestrator, Plan Runner, Mori Parity"
  ["28-ROADMAP"]="Roadmap, Phases, Priorities"
)

TOPIC_V1_FILES=(
  ["01-SIGNAL"]="- /Users/will/dev/nunchi/roko/roko/docs/v1/00-architecture/02-engram-data-type.md
- /Users/will/dev/nunchi/roko/roko/docs/v1/00-architecture/02b-pulse-ephemeral-event.md
- /Users/will/dev/nunchi/roko/roko/docs/v1/00-architecture/04-decay-variants.md
- /Users/will/dev/nunchi/roko/roko/docs/v1/00-architecture/05-provenance-and-attestation.md
- /Users/will/dev/nunchi/roko/roko/docs/v1/00-architecture/18-decay-tier-matrix.md
- /Users/will/dev/nunchi/roko/roko/docs/v1/00-architecture/19-compositional-kinds.md
- /Users/will/dev/nunchi/roko/roko/docs/v1/00-architecture/25-attention-as-currency.md"
  ["02-CELL"]="- /Users/will/dev/nunchi/roko/roko/docs/v1/00-architecture/06-synapse-traits.md
- /Users/will/dev/nunchi/roko/roko/docs/v1/00-architecture/07-substrate-trait.md
- /Users/will/dev/nunchi/roko/roko/docs/v1/00-architecture/07b-bus-transport-fabric.md
- /Users/will/dev/nunchi/roko/roko/docs/v1/00-architecture/08-scorer-gate-router-composer-policy.md
- All files in /Users/will/dev/nunchi/roko/roko/docs/v1/03-composition/
- All files in /Users/will/dev/nunchi/roko/roko/docs/v1/04-verification/"
  ["03-GRAPH"]="- All files in /Users/will/dev/nunchi/roko/roko/docs/v1/01-orchestration/"
  ["04-EXECUTION"]="- /Users/will/dev/nunchi/roko/roko/docs/v1/00-architecture/09-universal-cognitive-loop.md
- /Users/will/dev/nunchi/roko/roko/docs/v1/00-architecture/21-performance-numerical-stability.md
- /Users/will/dev/nunchi/roko/roko/docs/v1/00-architecture/22-error-handling-recovery.md"
  ["05-AGENT"]="- All files in /Users/will/dev/nunchi/roko/roko/docs/v1/02-agents/
- All files in /Users/will/dev/nunchi/roko/roko/docs/v1/09-daimon/
- All files in /Users/will/dev/nunchi/roko/roko/docs/v1/16-heartbeat/"
  ["06-MEMORY"]="- All files in /Users/will/dev/nunchi/roko/roko/docs/v1/06-neuro/
- All files in /Users/will/dev/nunchi/roko/roko/docs/v1/10-dreams/"
  ["07-LEARNING"]="- All files in /Users/will/dev/nunchi/roko/roko/docs/v1/05-learning/"
  ["08-GATEWAY"]="- /Users/will/dev/nunchi/roko/roko/docs/v1/00-architecture/10-three-cognitive-speeds.md
- /Users/will/dev/nunchi/roko/roko/docs/v1/00-architecture/11-dual-process-and-active-inference.md"
  ["09-FEEDS"]="(no direct v1 equivalent — check 20-technical-analysis/ for oracle/feed concepts)"
  ["10-GROUPS"]="- All files in /Users/will/dev/nunchi/roko/roko/docs/v1/13-coordination/"
  ["11-CONNECTIVITY"]="(check docs/v1/08-chain/ for relay/networking content)"
  ["12-EXTENSIONS"]="(check docs/v1/02-agents/12-extensibility.md)"
  ["13-TRIGGERS"]="- All files in /Users/will/dev/nunchi/roko/roko/docs/v1/07-conductor/"
  ["14-TOOLS"]="- All files in /Users/will/dev/nunchi/roko/roko/docs/v1/18-tools/"
  ["15-TELEMETRY"]="- /Users/will/dev/nunchi/roko/roko/docs/v1/00-architecture/14-c-factor-collective-intelligence.md
- /Users/will/dev/nunchi/roko/roko/docs/v1/00-architecture/32-comprehensive-test-strategy.md"
  ["16-SECURITY"]="- All files in /Users/will/dev/nunchi/roko/roko/docs/v1/11-safety/"
  ["17-AUTH"]="- /Users/will/dev/nunchi/roko/roko/docs/v1/14-identity-economy/01-erc-8004-three-registries.md"
  ["18-PAYMENTS"]="- All files in /Users/will/dev/nunchi/roko/roko/docs/v1/14-identity-economy/"
  ["19-CONFIG"]="- /Users/will/dev/nunchi/roko/roko/docs/v1/00-architecture/20-configuration-schema.md"
  ["20-SURFACES"]="- All files in /Users/will/dev/nunchi/roko/roko/docs/v1/12-interfaces/"
  ["21-MARKETPLACE"]="- /Users/will/dev/nunchi/roko/roko/docs/v1/14-identity-economy/00-vision-and-a16z-framing.md
- /Users/will/dev/nunchi/roko/roko/docs/v1/14-identity-economy/12-three-hiring-models.md"
  ["22-REGISTRIES"]="- /Users/will/dev/nunchi/roko/roko/docs/v1/08-chain/06-erc-8004-registries.md
- /Users/will/dev/nunchi/roko/roko/docs/v1/08-chain/23-knowledge-futures-market.md"
  ["23-ARENAS"]="(check docs/v1/14-identity-economy/ for arena and eval content)"
  ["24-DEFI"]="- /Users/will/dev/nunchi/roko/roko/docs/v1/08-chain/21-isfr-clearing-settlement.md
- /Users/will/dev/nunchi/roko/roko/docs/v1/14-identity-economy/13-isfr-clearing-settlement.md
- /Users/will/dev/nunchi/roko/roko/docs/v1/14-identity-economy/14-knowledge-futures-market.md
- /Users/will/dev/nunchi/roko/roko/docs/v1/20-technical-analysis/02-chain-oracles.md"
  ["25-DEPLOYMENT"]="- All files in /Users/will/dev/nunchi/roko/roko/docs/v1/19-deployment/"
  ["26-CROSS-CUTS"]="- /Users/will/dev/nunchi/roko/roko/docs/v1/00-architecture/13-cognitive-cross-cuts.md"
  ["27-ORCHESTRATOR"]="- All files in /Users/will/dev/nunchi/roko/roko/docs/v1/01-orchestration/"
  ["28-ROADMAP"]="- /Users/will/dev/nunchi/roko/roko/docs/v1/00-architecture/33-refactor-plan-phases.md
- /Users/will/dev/nunchi/roko/roko/docs/v1/00-architecture/35-consolidated-roadmap.md"
)

TOPIC_TMP_PRD_FILES=(
  ["01-SIGNAL"]="- /Users/will/dev/nunchi/roko/roko/tmp/prds/PRD-01-OVERVIEW.md (§2-4 on Signal types)"
  ["02-CELL"]="- /Users/will/dev/nunchi/roko/roko/tmp/prds/PRD-03-COGNITIVE-ENGINE.md
- /Users/will/dev/nunchi/roko/roko/tmp/prds/PRD-04-CONTEXT-ENGINEERING.md
- /Users/will/dev/nunchi/roko/roko/tmp/prds/IMPL-02-COGNITIVE-ENGINE.md
- /Users/will/dev/nunchi/roko/roko/tmp/prds/IMPL-03-CONTEXT.md"
  ["03-GRAPH"]="- /Users/will/dev/nunchi/roko/roko/tmp/prds/PRD-01-OVERVIEW.md (orchestration sections)"
  ["04-EXECUTION"]="- /Users/will/dev/nunchi/roko/roko/tmp/prds/PRD-02-AGENT-RUNTIME.md
- /Users/will/dev/nunchi/roko/roko/tmp/prds/IMPL-01-RUNTIME.md"
  ["05-AGENT"]="- /Users/will/dev/nunchi/roko/roko/tmp/prds/PRD-02-AGENT-RUNTIME.md
- /Users/will/dev/nunchi/roko/roko/tmp/prds/IMPL-01-RUNTIME.md"
  ["06-MEMORY"]="- /Users/will/dev/nunchi/roko/roko/tmp/prds/PRD-05-KNOWLEDGE-AND-STIGMERGY.md
- /Users/will/dev/nunchi/roko/roko/tmp/prds/IMPL-04-KNOWLEDGE.md"
  ["07-LEARNING"]="- /Users/will/dev/nunchi/roko/roko/tmp/prds/PRD-03-COGNITIVE-ENGINE.md (cascade router sections)
- /Users/will/dev/nunchi/roko/roko/tmp/prds/IMPL-02-COGNITIVE-ENGINE.md"
  ["08-GATEWAY"]="- /Users/will/dev/nunchi/roko/roko/tmp/prds/PRD-03-COGNITIVE-ENGINE.md (§inference gateway)"
  ["09-FEEDS"]="(no direct tmp/prds coverage)"
  ["10-GROUPS"]="(no direct tmp/prds coverage)"
  ["11-CONNECTIVITY"]="(no direct tmp/prds coverage)"
  ["12-EXTENSIONS"]="(no direct tmp/prds coverage)"
  ["13-TRIGGERS"]="(no direct tmp/prds coverage)"
  ["14-TOOLS"]="(no direct tmp/prds coverage)"
  ["15-TELEMETRY"]="(no direct tmp/prds coverage)"
  ["16-SECURITY"]="- /Users/will/dev/nunchi/roko/roko/tmp/prds/PRD-02-AGENT-RUNTIME.md (safety sections)"
  ["17-AUTH"]="(no direct tmp/prds coverage)"
  ["18-PAYMENTS"]="(no direct tmp/prds coverage)"
  ["19-CONFIG"]="(no direct tmp/prds coverage)"
  ["20-SURFACES"]="- /Users/will/dev/nunchi/roko/roko/tmp/prds/PRD-08-DEPLOYMENT-AND-UX.md
- /Users/will/dev/nunchi/roko/roko/tmp/prds/PRD-10-DASHBOARD-AND-TUI.md
- /Users/will/dev/nunchi/roko/roko/tmp/prds/IMPL-08-SURFACES.md
- /Users/will/dev/nunchi/roko/roko/tmp/prds/IMPL-10-DASHBOARD-AND-TUI.md
- /Users/will/dev/nunchi/roko/roko/tmp/prds/IMPL-10-DEMO.md"
  ["21-MARKETPLACE"]="- /Users/will/dev/nunchi/roko/roko/tmp/prds/PRD-06-DOMAINS-AND-ARENAS.md"
  ["22-REGISTRIES"]="- /Users/will/dev/nunchi/roko/roko/tmp/prds/PRD-09-EXTENSIBILITY-AND-MULTICHAIN.md"
  ["23-ARENAS"]="- /Users/will/dev/nunchi/roko/roko/tmp/prds/PRD-06-DOMAINS-AND-ARENAS.md"
  ["24-DEFI"]="- /Users/will/dev/nunchi/roko/roko/tmp/prds/PRD-07-ISFR-AND-INSTRUMENTS.md
- /Users/will/dev/nunchi/roko/roko/tmp/prds/IMPL-06-ISFR.md"
  ["25-DEPLOYMENT"]="- /Users/will/dev/nunchi/roko/roko/tmp/prds/PRD-08-DEPLOYMENT-AND-UX.md"
  ["26-CROSS-CUTS"]="(covered across multiple PRDs)"
  ["27-ORCHESTRATOR"]="- /Users/will/dev/nunchi/roko/roko/tmp/prds/IMPL-01-RUNTIME.md"
  ["28-ROADMAP"]="- /Users/will/dev/nunchi/roko/roko/tmp/prds/00-INDEX.md"
)

TOPIC_CRATE_PATHS=(
  ["01-SIGNAL"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-core/src/types.rs
- /Users/will/dev/nunchi/roko/roko/crates/roko-core/src/signal.rs (if exists)
- /Users/will/dev/nunchi/roko/roko/crates/roko-fs/"
  ["02-CELL"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-core/src/ (traits: Substrate, Scorer, Gate, Router, Composer, Policy)
- /Users/will/dev/nunchi/roko/roko/crates/roko-compose/
- /Users/will/dev/nunchi/roko/roko/crates/roko-gate/"
  ["03-GRAPH"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/"
  ["04-EXECUTION"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs"
  ["05-AGENT"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-agent/
- /Users/will/dev/nunchi/roko/roko/crates/roko-daimon/
- /Users/will/dev/nunchi/roko/roko/crates/roko-runtime/"
  ["06-MEMORY"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-neuro/
- /Users/will/dev/nunchi/roko/roko/crates/roko-dreams/"
  ["07-LEARNING"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-learn/"
  ["08-GATEWAY"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/dispatcher/"
  ["09-FEEDS"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-conductor/"
  ["10-GROUPS"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-runtime/"
  ["11-CONNECTIVITY"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-acp/
- /Users/will/dev/nunchi/roko/roko/crates/roko-mcp-code/"
  ["12-EXTENSIONS"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/"
  ["13-TRIGGERS"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-conductor/"
  ["14-TOOLS"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-std/
- /Users/will/dev/nunchi/roko/roko/crates/roko-core/src/tools/"
  ["15-TELEMETRY"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-learn/
- /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/"
  ["16-SECURITY"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/"
  ["17-AUTH"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/"
  ["18-PAYMENTS"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-chain/"
  ["19-CONFIG"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/"
  ["20-SURFACES"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-cli/
- /Users/will/dev/nunchi/roko/roko/crates/roko-serve/"
  ["21-MARKETPLACE"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-chain/"
  ["22-REGISTRIES"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-chain/"
  ["23-ARENAS"]="(no crate yet)"
  ["24-DEFI"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/isfr.rs
- /Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/identity_economy_markets.rs"
  ["25-DEPLOYMENT"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/deploy/"
  ["26-CROSS-CUTS"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-neuro/
- /Users/will/dev/nunchi/roko/roko/crates/roko-daimon/
- /Users/will/dev/nunchi/roko/roko/crates/roko-dreams/"
  ["27-ORCHESTRATOR"]="- /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs
- /Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/"
  ["28-ROADMAP"]="(no crate — meta topic)"
)

# Build a prompt for a specific topic
build_topic_prompt() {
  local topic_id="$1"
  local topic_name="${topic_id#*-}"
  local title="${TOPIC_TITLES[$topic_id]:-$topic_name}"
  local v1_files="${TOPIC_V1_FILES[$topic_id]:-'(none)'}"
  local v2_depth="Check /Users/will/dev/nunchi/roko/roko/docs/v2-depth/ for folders matching this topic"
  local tmp_prds="${TOPIC_TMP_PRD_FILES[$topic_id]:-'(none)'}"
  local crates="${TOPIC_CRATE_PATHS[$topic_id]:-'(none)'}"

  local prompt="$TEMPLATE"
  prompt="${prompt//\{\{TOPIC_ID\}\}/$topic_id}"
  prompt="${prompt//\{\{TOPIC_NAME\}\}/$topic_name}"
  prompt="${prompt//\{\{TOPIC_TITLE\}\}/$title}"
  prompt="${prompt//\{\{V1_FILES\}\}/$v1_files}"
  prompt="${prompt//\{\{V2_DEPTH_FILES\}\}/$v2_depth}"
  prompt="${prompt//\{\{TMP_PRD_FILES\}\}/$tmp_prds}"
  prompt="${prompt//\{\{CRATE_PATHS\}\}/$crates}"

  echo "$prompt"
}

# Run a single topic
run_topic() {
  local topic_id="$1"
  local prompt_file
  prompt_file=$(mktemp)
  build_topic_prompt "$topic_id" > "$prompt_file"

  local output_file="$OUTPUT_DIR/${topic_id}.md"
  local log_file="$STATUS_DIR/${topic_id}-agent.log"

  log "Starting agent for $topic_id..."

  claude -p "$(cat "$prompt_file")" \
    --allowedTools 'Read,Grep,Glob,Bash(description:*)' \
    --output-format text \
    > "$log_file" 2>&1

  rm -f "$prompt_file"

  if [[ -f "$output_file" ]]; then
    log "DONE: $topic_id -> $output_file"
  else
    log "WARN: $topic_id agent finished but $output_file not found. Check $log_file"
  fi
}

# Main
if [[ $# -gt 0 ]]; then
  # Run single topic
  topic="$1"
  if [[ -z "${TOPIC_TITLES[$topic]:-}" ]]; then
    echo "Unknown topic: $topic"
    echo "Available: ${!TOPIC_TITLES[*]}"
    exit 1
  fi
  run_topic "$topic"
else
  # Run all topics in parallel batches
  log "Running all ${#TOPIC_TITLES[@]} topics (max $MAX_PARALLEL parallel)..."

  pids=()
  for topic_id in $(echo "${!TOPIC_TITLES[@]}" | tr ' ' '\n' | sort); do
    # Throttle
    while (( ${#pids[@]} >= MAX_PARALLEL )); do
      new_pids=()
      for pid in "${pids[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
          new_pids+=("$pid")
        else
          wait "$pid" || true
        fi
      done
      pids=("${new_pids[@]}")
      if (( ${#pids[@]} >= MAX_PARALLEL )); then
        sleep 5
      fi
    done

    # Launch in background
    run_topic "$topic_id" &
    pids+=($!)
    log "Launched $topic_id (PID $!), ${#pids[@]}/$MAX_PARALLEL slots used"
    sleep 2  # small delay to avoid API rate limit bursts
  done

  # Wait for all remaining
  for pid in "${pids[@]}"; do
    wait "$pid" || true
  done

  # Report
  log ""
  log "=== Phase 2 Complete ==="
  completed=$(find "$OUTPUT_DIR" -name '*.md' ! -name '00-*' | wc -l | tr -d ' ')
  total=${#TOPIC_TITLES[@]}
  log "Completed: $completed / $total topics"

  if [[ -d "$OUTPUT_DIR" ]]; then
    log "Output files:"
    ls -1 "$OUTPUT_DIR"/*.md 2>/dev/null | while read -r f; do
      echo "  $(basename "$f")"
    done
  fi
fi
