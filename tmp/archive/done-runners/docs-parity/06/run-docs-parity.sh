#!/usr/bin/env bash
# run-docs-parity.sh — Execute 06-neuro docs-parity refresh batches

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PARITY_DIR="$REPO_ROOT/tmp/docs-parity/06"
CONTEXT_PACK="$PARITY_DIR/context-pack"
LOG_DIR="$PARITY_DIR/logs"
RUN_ID="$(date +%Y%m%d-%H%M%S)"

mkdir -p "$LOG_DIR/$RUN_ID"

declare -A BATCH_DESC=(
  [N1]="Refresh overview, context pack, and execution posture"
  [N2]="Refresh knowledge, tiers, and decay parity"
  [N3]="Refresh HDC foundations around the real HdcVector"
  [N4]="Refresh query/context docs and defer cross-domain transfer honestly"
  [N5]="Refresh distillation, somatic, exchange, and backup scope"
  [N6]="Refresh frontier status, source anchors, audit log, and runner text"
)

declare -A BATCH_DEPS=(
  [N1]=""
  [N2]="N1"
  [N3]="N1"
  [N4]="N1"
  [N5]="N1"
  [N6]="N2 N3 N4 N5"
)

declare -A BATCH_VERIFY=(
  [N1]="rg -n \"HDC fingerprint|deferred|target-state\" \"$PARITY_DIR\""
  [N2]="rg -n \"demurrage|tier progression|Engram\" \"$PARITY_DIR/A-knowledge-types-tiers-decay.md\""
  [N3]="rg -n \"HdcVector|roko-hdc|Engram\" \"$PARITY_DIR/B-hdc-foundations-operations.md\""
  [N4]="rg -n \"query_similar|Substrate|cross-domain|deferred\" \"$PARITY_DIR/C-query-crossdomain-context.md\""
  [N5]="rg -n \"Distiller|TierProgression|Library of Babel|deferred\" \"$PARITY_DIR/D-distillation-progression.md\" \"$PARITY_DIR/E-somatic-exchange-backup.md\""
  [N6]="bash -n \"$PARITY_DIR/run-docs-parity.sh\""
)

declare -A BATCH_FILES=(
  [N1]="00-INDEX.md BATCHES.md context-pack/neuro-summary.md context-pack/gaps-summary.md context-pack/carry-forward-map.md context-pack/repo-map.md context-pack/agent-runbook.md"
  [N2]="A-knowledge-types-tiers-decay.md"
  [N3]="B-hdc-foundations-operations.md"
  [N4]="C-query-crossdomain-context.md"
  [N5]="D-distillation-progression.md E-somatic-exchange-backup.md"
  [N6]="F-status-frontier.md SOURCE-INDEX.md AUDIT-LOG.md run-docs-parity.sh"
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
You are executing docs-parity batch $batch for PU06 under $PARITY_DIR.

Task: $desc

Read:
- $PARITY_DIR/00-INDEX.md
- $PARITY_DIR/BATCHES.md
- $PARITY_DIR/SOURCE-INDEX.md
- $CONTEXT_PACK/agent-runbook.md
- $CONTEXT_PACK/neuro-summary.md
- $CONTEXT_PACK/gaps-summary.md
- $CONTEXT_PACK/repo-map.md

Batch-owned files:
$(for f in ${BATCH_FILES[$batch]}; do echo "- $PARITY_DIR/$f"; done)

Rules:
1. Edit only files under $PARITY_DIR.
2. Keep this as a docs refresh, not a code implementation plan.
3. Treat HDC-on-Engram as the highest-value next follow-up.
4. Mark unbuilt concepts as deferred or target-state.
5. Run the verify command before finishing.

Verify command: ${BATCH_VERIFY[$batch]}
PROMPT

  echo "  Prompt written to $prompt_file"
  echo "  [TODO] Run the batch prompt with your preferred agent runner and capture output in $log_file"
  echo "PENDING" > "$result_file"
}

if [ $# -eq 0 ]; then
  BATCHES=(N1 N2 N3 N4 N5 N6)
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
