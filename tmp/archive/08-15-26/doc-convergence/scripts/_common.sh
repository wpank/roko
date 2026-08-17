#!/usr/bin/env bash
# Shared variables for doc-convergence scripts

set -euo pipefail

# Paths
export ROKO_ROOT="/Users/will/dev/nunchi/roko/roko"
export DOCS_V1="$ROKO_ROOT/docs/v1"
export DOCS_V2="$ROKO_ROOT/docs/v2"
export DOCS_V2_DEPTH="$ROKO_ROOT/docs/v2-depth"
export TMP_PRDS="$ROKO_ROOT/tmp/prds"
export CRATES="$ROKO_ROOT/crates"
export BARDO_BACKUP="/Users/will/dev/nunchi/roko/bardo-backup"
export CONVERGENCE_DIR="$ROKO_ROOT/tmp/doc-convergence"
export OUTPUT_DIR="$CONVERGENCE_DIR/output"
export STATUS_DIR="$CONVERGENCE_DIR/status"
export PROMPTS_DIR="$CONVERGENCE_DIR/prompts"

# Max parallel agents (tune based on API rate limits)
export MAX_PARALLEL="${MAX_PARALLEL:-5}"

# The 28 v2 topics + their v1/tmp/prds mappings
# Format: "v2_doc|v1_folders|tmp_prd_files|primary_crates"
TOPICS=(
  "01-SIGNAL|00-architecture/02-engram,00-architecture/02b-pulse,00-architecture/04-decay,00-architecture/05-provenance,00-architecture/18-decay-tier,00-architecture/19-compositional,00-architecture/25-attention|PRD-01|roko-core,roko-fs"
  "02-CELL|00-architecture/06-synapse,00-architecture/07-substrate,00-architecture/07b-bus,00-architecture/08-scorer,03-composition,04-verification|PRD-03,PRD-04|roko-core,roko-compose,roko-gate"
  "03-GRAPH|01-orchestration|PRD-01|roko-orchestrator"
  "04-EXECUTION|00-architecture/09-universal-cognitive,00-architecture/21-performance,00-architecture/22-error|PRD-02|roko-cli/src/orchestrate.rs"
  "05-AGENT|02-agents,09-daimon,16-heartbeat|PRD-02|roko-agent,roko-daimon,roko-runtime"
  "06-MEMORY|06-neuro,10-dreams|PRD-05|roko-neuro,roko-dreams"
  "07-LEARNING|05-learning|PRD-03|roko-learn"
  "08-GATEWAY|00-architecture/10-three-cognitive,00-architecture/11-dual-process|PRD-03|roko-agent/src/dispatcher"
  "09-FEEDS||PRD-06|roko-conductor"
  "10-GROUPS|13-coordination||roko-runtime"
  "11-CONNECTIVITY|||roko-acp,roko-mcp-code"
  "12-EXTENSIONS|||roko-agent/src/safety"
  "13-TRIGGERS|07-conductor||roko-conductor"
  "14-TOOLS|18-tools||roko-std,roko-core/src/tools"
  "15-TELEMETRY|00-architecture/14-c-factor,00-architecture/32-test||roko-learn,roko-cli/src/tui"
  "16-SECURITY|11-safety|PRD-02|roko-agent/src/safety"
  "17-AUTH|14-identity-economy/01-erc-8004||roko-serve/src/routes"
  "18-PAYMENTS|14-identity-economy||roko-chain"
  "19-CONFIG|00-architecture/20-configuration||roko-core/src/config"
  "20-SURFACES|12-interfaces|PRD-08,PRD-10|roko-cli,roko-serve"
  "21-MARKETPLACE|14-identity-economy|PRD-06|roko-chain"
  "22-REGISTRIES|08-chain/06-erc-8004|PRD-09|roko-chain"
  "23-ARENAS||PRD-06|"
  "24-DEFI|08-chain/21-isfr,14-identity-economy/13-isfr,14-identity-economy/14-knowledge-futures|PRD-07|roko-chain/src/isfr.rs"
  "25-DEPLOYMENT|19-deployment|PRD-08|roko-cli/src/deploy"
  "26-CROSS-CUTS|00-architecture/13-cognitive-cross||roko-neuro,roko-daimon,roko-dreams"
  "27-ORCHESTRATOR|01-orchestration|IMPL-01|roko-cli/src/orchestrate.rs,roko-orchestrator"
  "28-ROADMAP|00-architecture/33-refactor,00-architecture/35-consolidated||"
)

export TOPICS

# Run a claude agent with a prompt file, writing output to a file
# Usage: run_agent "prompt_file" "output_file" [--background]
run_agent() {
  local prompt_file="$1"
  local output_file="$2"
  local background="${3:-}"
  local prompt
  prompt=$(<"$prompt_file")

  if [[ "$background" == "--background" ]]; then
    claude -p "$prompt" \
      --allowedTools 'Read,Grep,Glob,Bash(description:*)' \
      --output-format text \
      > "$output_file" 2>&1 &
    echo $!
  else
    claude -p "$prompt" \
      --allowedTools 'Read,Grep,Glob,Bash(description:*)' \
      --output-format text \
      > "$output_file" 2>&1
  fi
}

# Wait for background PIDs with a max-parallel throttle
# Usage: throttled_wait "${pids[@]}"
throttled_wait() {
  local -a pids=("$@")
  while (( ${#pids[@]} >= MAX_PARALLEL )); do
    local -a remaining=()
    for pid in "${pids[@]}"; do
      if kill -0 "$pid" 2>/dev/null; then
        remaining+=("$pid")
      else
        wait "$pid" || true
      fi
    done
    pids=("${remaining[@]}")
    if (( ${#pids[@]} >= MAX_PARALLEL )); then
      sleep 2
    fi
  done
  echo "${pids[*]}"
}

log() {
  echo "[$(date '+%H:%M:%S')] $*"
}
