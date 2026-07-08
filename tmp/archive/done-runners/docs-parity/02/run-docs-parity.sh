#!/usr/bin/env bash
# run-docs-parity.sh — Prepare 02-agent parity refresh batches
#
# Usage:
#   ./run-docs-parity.sh              # Prepare all batches in dependency-safe order
#   ./run-docs-parity.sh G1           # Prepare a single batch
#   ./run-docs-parity.sh G1 G4 G6     # Prepare specific batches
#
# Each batch should be realistic for one agent in about 90 minutes and should
# leave behind explicit current-state findings plus clear deferrals.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PARITY_DIR="$REPO_ROOT/tmp/docs-parity/02"
CONTEXT_PACK="$PARITY_DIR/context-pack"
LOG_DIR="$PARITY_DIR/logs"
RUN_ID="$(date +%Y%m%d-%H%M%S)"

mkdir -p "$LOG_DIR/$RUN_ID"

declare -A BATCH_DESC=(
  [G1]="Reset the parity pack around audited scope and corrected source anchors"
  [G2]="Refresh core abstractions and the live agent surface"
  [G3]="Refresh provider-system parity against current runtime families"
  [G4]="Refresh tool runtime, MCP, sidecar, and lifecycle ownership"
  [G5]="Narrow routing and temperament claims to what is actually wired"
  [G6]="Defer domain/plugin overreach and keep only shipped advanced surfaces"
)

declare -A BATCH_DEPS=(
  [G1]=""
  [G2]="G1"
  [G3]="G1"
  [G4]="G1"
  [G5]="G1"
  [G6]="G1"
)

declare -A BATCH_VERIFY=(
  [G1]="bash -n tmp/docs-parity/02/run-docs-parity.sh && rg -n \"docs calibration|16-tool built-in registry|ProcessSupervisor|duplicate AgentEvent\" tmp/docs-parity/02/00-INDEX.md tmp/docs-parity/02/SOURCE-INDEX.md"
  [G2]="rg -n \"19 Agent impls|28 variants|Claude CLI/API|duplicate AgentEvent\" tmp/docs-parity/02/A-core-abstractions.md tmp/docs-parity/02/context-pack/agents-summary.md"
  [G3]="rg -n \"6 ProviderKind variants|OpenAiCompat|Perplexity|Gemini|Ollama\" tmp/docs-parity/02/B-provider-system.md tmp/docs-parity/02/context-pack/repo-map.md"
  [G4]="rg -n \"ToolLoop|ToolDispatcher|agent.mcp_config|AgentServer|ProcessSupervisor\" tmp/docs-parity/02/C-tool-loop.md tmp/docs-parity/02/D-lifecycle-infrastructure.md tmp/docs-parity/02/context-pack/agent-runbook.md"
  [G5]="rg -n \"CascadeRouter|active inference|temperament\" tmp/docs-parity/02/E-routing-temperament.md tmp/docs-parity/02/context-pack/gaps-summary.md"
  [G6]="rg -n \"DEFERRED|domain profiles|plugin SPI tiers 4-5|CompositeAgent|MorphableAgent\" tmp/docs-parity/02/F-advanced-capabilities.md tmp/docs-parity/02/context-pack/carry-forward-map.md"
)

declare -A BATCH_FILES=(
  [G1]="00-INDEX.md BATCHES.md SOURCE-INDEX.md run-docs-parity.sh"
  [G2]="A-core-abstractions.md context-pack/agents-summary.md"
  [G3]="B-provider-system.md context-pack/repo-map.md"
  [G4]="C-tool-loop.md D-lifecycle-infrastructure.md context-pack/agent-runbook.md"
  [G5]="E-routing-temperament.md context-pack/gaps-summary.md"
  [G6]="F-advanced-capabilities.md context-pack/carry-forward-map.md"
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
You are auditing batch $batch of the docs-parity project for Roko.

## Task: $desc

Read these files for full context:
- $PARITY_DIR/00-INDEX.md
- $PARITY_DIR/BATCHES.md (find the "$batch" section)
- $PARITY_DIR/SOURCE-INDEX.md
- $CONTEXT_PACK/agent-runbook.md
- $CONTEXT_PACK/carry-forward-map.md
- $CONTEXT_PACK/repo-map.md
- $CONTEXT_PACK/agents-summary.md
- $CONTEXT_PACK/gaps-summary.md

Also read these batch-specific files:
$(for f in ${BATCH_FILES[$batch]}; do echo "- $PARITY_DIR/$f"; done)

Execution rules:
1. Search and trace the current code before asserting any doc claim.
2. Stay inside the batch scope from BATCHES.md and keep the work realistic for one agent in about 90 minutes.
3. Prefer documenting what is wired, partial, or deferred over proposing broad implementation work.
4. If a discovered task is out of scope, record it as deferred with an owning follow-on batch and do not expand the batch.
5. Run the verify command as evidence gathering.
6. Report: files changed, commands run, current-state findings, pass/fail/block status, and intentional deferrals.

Verify command: ${BATCH_VERIFY[$batch]}
PROMPT

  echo "  Prompt written to $prompt_file"
  echo "  [TODO] Run this prompt with your preferred agent runner and tee output to $log_file"

  echo "PENDING" > "$result_file"
}

if [ $# -eq 0 ]; then
  BATCHES=(G1 G2 G3 G4 G5 G6)
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
