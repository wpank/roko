#!/usr/bin/env bash
# run-docs-parity.sh — Execute 08-chain parity batches
#
# Usage:
#   ./run-docs-parity.sh              # Run all batches in recommended order
#   ./run-docs-parity.sh K1           # Run a single batch
#   ./run-docs-parity.sh K1 K3 K6     # Run specific batches
#
# Each batch should be self-contained and leave a clear PASS / FAIL / BLOCKED result.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PARITY_DIR="$REPO_ROOT/tmp/docs-parity/08"
CONTEXT_PACK="$PARITY_DIR/context-pack"
LOG_DIR="$PARITY_DIR/logs"
RUN_ID="$(date +%Y%m%d-%H%M%S)"

mkdir -p "$LOG_DIR/$RUN_ID"

declare -A BATCH_DESC=(
  [K1]="Reconfirm the shipping chain foundation and fix any anchor drift"
  [K2]="Make the 7 demo Solidity contracts visible and correctly scoped"
  [K3]="Build a module-by-module mirage chain scaffold inventory and fix feature/RPC drift"
  [K4]="Mark gossip, market, and reputation frontier docs explicitly and add shipping precursors"
  [K5]="Reconcile WitnessEngine and ChainWitnessEngine naming and link the shipping observer surfaces"
  [K6]="Correct the payments, ISFR, privacy, and futures status story"
  [K7]="Regenerate Doc 24 as the canonical status summary"
  [K8]="Do the final global banner and housekeeping pass across topic 08"
)

declare -A BATCH_DEPS=(
  [K1]=""
  [K2]=""
  [K3]=""
  [K4]="K2"
  [K5]=""
  [K6]=""
  [K7]="K1 K2 K3 K5 K6"
  [K8]="K4 K7"
)

declare -A BATCH_VERIFY=(
  [K1]="grep -oE '[A-Za-z0-9_./-]+\\.(rs|sol|md):[0-9]+(-[0-9]+)?' tmp/docs-parity/08/F-built-foundation.md | sort -u"
  [K2]="rg -n \"contracts/src|AgentRegistry.sol|WorkerRegistry.sol|BountyMarket.sol|ConsortiumValidator.sol\" docs/08-chain"
  [K3]="grep -c \"korai_\" apps/mirage-rs/src/chain_rpc.rs && rg -n \"chain-extensions|korai_\" docs/08-chain/01-*.md docs/08-chain/18-*.md"
  [K4]="rg -n \"Design — Phase 2\\+|InsightBus|PheromoneBus|BountyMarket.sol|WorkerRegistry.sol\" docs/08-chain"
  [K5]="rg -n \"ChainWitnessEngine|WitnessEngine|roko-chain-watcher\" docs/08-chain crates/roko-chain"
  [K6]="rg -n \"Implementation: Built|Proxy-only|ISFR_SERVICE_URL|localhost:8546\" docs/08-chain/20-23*.md apps/mirage-rs/src/http_api/isfr.rs"
  [K7]="rg -n \"roko-chain|roko-primitives|mirage-rs|roko-chain-watcher|roko_bridge|contracts/src\" docs/08-chain/24-*.md && rg -n \"bardo-primitives\" docs/08-chain/24-*.md"
  [K8]="rg -n \"^> \\*\\*Implementation\\*\\*:\" docs/08-chain/*.md"
)

declare -A BATCH_FILES=(
  [K1]="F-built-foundation.md"
  [K2]="A-vision-and-chain-spec.md D-job-market-and-reputation.md"
  [K3]="F-built-foundation.md A-vision-and-chain-spec.md"
  [K4]="C-gossip-and-p2p-network.md D-job-market-and-reputation.md"
  [K5]="E-witness-triage-heartbeat.md F-built-foundation.md"
  [K6]="G-payments-settlement-privacy.md"
  [K7]="A-vision-and-chain-spec.md F-built-foundation.md"
  [K8]="A-vision-and-chain-spec.md B-identity-and-on-chain-trust.md C-gossip-and-p2p-network.md D-job-market-and-reputation.md E-witness-triage-heartbeat.md G-payments-settlement-privacy.md"
)

check_deps() {
  local batch="$1"
  local deps="${BATCH_DEPS[$batch]}"
  if [ -z "$deps" ]; then return 0; fi
  for dep in $deps; do
    local result_file="$LOG_DIR/$RUN_ID/$dep.result"
    if [ ! -f "$result_file" ] || [ "$(cat "$result_file")" != "PASS" ]; then
      echo "  [BLOCKED] $batch depends on $dep (not yet completed)"
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
You are executing batch $batch of the docs-parity project for Roko.

## Task: $desc

Read these files for full context:
- $PARITY_DIR/00-INDEX.md
- $PARITY_DIR/BATCHES.md (find the "$batch" section)
- $PARITY_DIR/SOURCE-INDEX.md
- $CONTEXT_PACK/agent-runbook.md
- $CONTEXT_PACK/carry-forward-map.md
- $CONTEXT_PACK/repo-map.md
- $CONTEXT_PACK/chain-summary.md
- $CONTEXT_PACK/gaps-summary.md

Also read these batch-specific files:
$(for f in ${BATCH_FILES[$batch]}; do echo "- $PARITY_DIR/$f"; done)

Execution rules:
1. Search before changing docs.
2. Stay inside the batch scope from BATCHES.md.
3. If a discovered task is out of scope, record it as deferred and do not expand the batch.
4. After changes, run the verify command and fix issues if practical.
5. Report: files changed, commands run, pass/fail status, and intentional deferrals.

Verify command: ${BATCH_VERIFY[$batch]}
PROMPT

  echo "  Prompt written to $prompt_file"
  echo "  [TODO] Run this prompt with: claude --print \"$(cat "$prompt_file")\" 2>&1 | tee $log_file"

  echo "PENDING" > "$result_file"
}

if [ $# -eq 0 ]; then
  BATCHES=(K1 K2 K3 K5 K6 K4 K7 K8)
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
