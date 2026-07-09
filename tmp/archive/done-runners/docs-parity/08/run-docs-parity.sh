#!/usr/bin/env bash
# run-docs-parity.sh — Execute 08-chain parity refresh batches
#
# Usage:
#   ./run-docs-parity.sh
#   ./run-docs-parity.sh K1
#   ./run-docs-parity.sh K2 K4

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PARITY_DIR="$REPO_ROOT/tmp/docs-parity/08"
CONTEXT_PACK="$PARITY_DIR/context-pack"
LOG_DIR="$PARITY_DIR/logs"
RUN_ID="$(date +%Y%m%d-%H%M%S)"

mkdir -p "$LOG_DIR/$RUN_ID"

declare -A BATCH_DESC=(
  [K1]="Reset the top-level batch framing around a Phase 2+ docs-honesty posture"
  [K2]="Rewrite chain-spec and identity parity notes as target-state or deferred"
  [K3]="Rewrite gossip, market, payments, and privacy parity notes as deferred"
  [K4]="Re-anchor witness notes to witness primitives plus Solidity demo precursors only"
  [K5]="Refresh SOURCE-INDEX and the context pack with minimal witness/demo anchors"
  [K6]="Verify the runner metadata and shell syntax"
)

declare -A BATCH_DEPS=(
  [K1]=""
  [K2]="K1"
  [K3]="K1"
  [K4]="K2 K3"
  [K5]="K1 K4"
  [K6]="K1 K2 K3 K4 K5"
)

declare -A BATCH_VERIFY=(
  [K1]="rg -n 'Phase 2\\+|doc-honesty|defer' tmp/docs-parity/08/00-INDEX.md tmp/docs-parity/08/BATCHES.md"
  [K2]="rg -n 'target-state|Phase 2\\+|DEFERRED' tmp/docs-parity/08/A-vision-and-chain-spec.md tmp/docs-parity/08/B-identity-and-on-chain-trust.md"
  [K3]="rg -n 'DEFERRED|Phase 2\\+|not shipping' tmp/docs-parity/08/C-gossip-and-p2p-network.md tmp/docs-parity/08/D-job-market-and-reputation.md tmp/docs-parity/08/G-payments-settlement-privacy.md"
  [K4]="rg -n 'ChainWitnessEngine|roko-chain-watcher|WitnessEngine|contracts/src|demo' tmp/docs-parity/08/E-witness-triage-heartbeat.md tmp/docs-parity/08/F-built-foundation.md"
  [K5]="rg -n 'witness primitives|Solidity demo|Phase 2\\+|DEFERRED|deferred|contracts/src' tmp/docs-parity/08/SOURCE-INDEX.md tmp/docs-parity/08/context-pack/*.md"
  [K6]="bash -n tmp/docs-parity/08/run-docs-parity.sh"
)

declare -A BATCH_FILES=(
  [K1]="00-INDEX.md BATCHES.md"
  [K2]="A-vision-and-chain-spec.md B-identity-and-on-chain-trust.md"
  [K3]="C-gossip-and-p2p-network.md D-job-market-and-reputation.md G-payments-settlement-privacy.md"
  [K4]="E-witness-triage-heartbeat.md F-built-foundation.md"
  [K5]="SOURCE-INDEX.md context-pack/chain-summary.md context-pack/gaps-summary.md context-pack/carry-forward-map.md context-pack/repo-map.md context-pack/agent-runbook.md"
  [K6]="run-docs-parity.sh"
)

check_deps() {
  local batch="$1"
  local deps="${BATCH_DEPS[$batch]}"
  if [ -z "$deps" ]; then
    return 0
  fi

  for dep in $deps; do
    local result_file="$LOG_DIR/$RUN_ID/$dep.result"
    if [ ! -f "$result_file" ] || [ "$(cat "$result_file")" != "PASS" ]; then
      echo "  [BLOCKED] $batch depends on $dep"
      return 1
    fi
  done

  return 0
}

run_batch() {
  local batch="$1"
  local desc="${BATCH_DESC[$batch]}"
  local log_file="$LOG_DIR/$RUN_ID/$batch.log"
  local result_file="$LOG_DIR/$RUN_ID/$batch.result"

  echo "=== [$batch] $desc ==="
  echo "  Log: $log_file"

  if ! check_deps "$batch"; then
    echo "BLOCKED" > "$result_file"
    return 1
  fi

  local prompt_file="$LOG_DIR/$RUN_ID/$batch.prompt"
  cat > "$prompt_file" <<PROMPT
You are executing batch $batch of the docs-parity refresh for topic 08.

## Task

$desc

Read these files first:
- $PARITY_DIR/00-INDEX.md
- $PARITY_DIR/BATCHES.md
- $PARITY_DIR/SOURCE-INDEX.md
- $CONTEXT_PACK/agent-runbook.md
- $CONTEXT_PACK/chain-summary.md
- $CONTEXT_PACK/gaps-summary.md
- $CONTEXT_PACK/carry-forward-map.md
- $CONTEXT_PACK/repo-map.md

Batch-owned files:
$(for f in ${BATCH_FILES[$batch]}; do echo "- $PARITY_DIR/$f"; done)

Rules:
1. Stay inside tmp/docs-parity/08/.
2. Keep the batch docs-only.
3. Use present tense only for witness primitives and Solidity demos.
4. Mark larger chain material as target-state, Phase 2+, or deferred.
5. Run the verify command before reporting PASS or FAIL.

Verify command: ${BATCH_VERIFY[$batch]}
PROMPT

  echo "  Prompt written to $prompt_file"
  echo "  [TODO] Execute the prompt with your preferred agent runner and write output to $log_file"
  echo "PENDING" > "$result_file"
}

if [ $# -eq 0 ]; then
  BATCHES=(K1 K2 K3 K4 K5 K6)
else
  BATCHES=("$@")
fi

echo "Docs-Parity Run: $RUN_ID"
echo "Batches: ${BATCHES[*]}"
echo "Logs: $LOG_DIR/$RUN_ID/"
echo ""

for batch in "${BATCHES[@]}"; do
  run_batch "$batch"
  echo ""
done

echo "=== Run $RUN_ID complete ==="
echo "Results:"
for batch in "${BATCHES[@]}"; do
  result_file="$LOG_DIR/$RUN_ID/$batch.result"
  result="$(cat "$result_file" 2>/dev/null || echo 'N/A')"
  echo "  $batch: $result"
done
