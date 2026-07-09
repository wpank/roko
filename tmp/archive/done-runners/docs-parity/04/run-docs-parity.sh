#!/usr/bin/env bash
# run-docs-parity.sh — Execute 04-verification parity refresh batches
#
# Usage:
#   ./run-docs-parity.sh            # Run all batches in dependency-safe order
#   ./run-docs-parity.sh V1         # Run a single batch
#   ./run-docs-parity.sh V1 V3 V5   # Run specific batches
#
# This runner is docs-only. It exists to keep tmp/docs-parity/04 aligned with
# the audit-backed runtime truth.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PARITY_DIR="$REPO_ROOT/tmp/docs-parity/04"
CONTEXT_PACK="$PARITY_DIR/context-pack"
LOG_DIR="$PARITY_DIR/logs"
RUN_ID="$(date +%Y%m%d-%H%M%S)"

mkdir -p "$LOG_DIR/$RUN_ID"

declare -A BATCH_DESC=(
  [V1]="Reset the overall posture and gate-foundation story around shipped verification"
  [V2]="Refresh artifact, ratchet, threshold, and feedback sections with narrowed wording"
  [V3]="Mark process rewards and autonomous eval / EvoSkills as deferred research"
  [V4]="Split live verdict signals from deferred forensic replay and refresh source anchors"
  [V5]="Refresh runbooks, repo map, and runner metadata for final consistency"
)

declare -A BATCH_DEPS=(
  [V1]=""
  [V2]="V1"
  [V3]="V1"
  [V4]="V1 V2 V3"
  [V5]="V4"
)

declare -A BATCH_VERIFY=(
  [V1]="rg -n 'substantially shipped|7-rung|rung_dispatch|Gate trait' tmp/docs-parity/04"
  [V2]="rg -n 'ArtifactStore|GateRatchet|gate-thresholds.json|EMA' tmp/docs-parity/04"
  [V3]="rg -n 'DEFERRED|research|target-state' tmp/docs-parity/04/E-process-rewards-lifecycle.md tmp/docs-parity/04/F-autonomous-evoskills.md"
  [V4]="rg -n 'GateVerdict|forensic|deferred|rung_dispatch' tmp/docs-parity/04"
  [V5]="bash -n tmp/docs-parity/04/run-docs-parity.sh"
)

declare -A BATCH_FILES=(
  [V1]="00-INDEX.md A-gate-foundation.md B-pipeline-rungs.md"
  [V2]="C-artifacts-ratcheting.md D-feedback-thresholds.md context-pack/verification-summary.md context-pack/gaps-summary.md"
  [V3]="E-process-rewards-lifecycle.md F-autonomous-evoskills.md"
  [V4]="G-forensic-verdict-signals.md SOURCE-INDEX.md context-pack/carry-forward-map.md"
  [V5]="context-pack/agent-runbook.md context-pack/repo-map.md run-docs-parity.sh"
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
You are executing batch $batch of the docs-parity refresh for Roko verification.

## Task
$desc

This is a docs-only batch.

Read these files first:
- $PARITY_DIR/00-INDEX.md
- $PARITY_DIR/SOURCE-INDEX.md
- $CONTEXT_PACK/agent-runbook.md

Also read these batch files:
$(for f in ${BATCH_FILES[$batch]}; do echo "- $PARITY_DIR/$f"; done)

Execution rules:
1. Edit only files under $PARITY_DIR.
2. Prefer shipped/partial/deferred wording over large implementation plans.
3. Keep A-D grounded in current runtime truth.
4. Keep E-F explicitly deferred.
5. Refresh stale source anchors instead of repeating them.
6. Report files changed, commands run, and any intentional deferrals.

Verify command:
${BATCH_VERIFY[$batch]}
PROMPT

  echo "  Prompt written to $prompt_file"
  echo "  [TODO] Run this prompt with your preferred agent runner and capture output to $log_file"

  echo "PENDING" > "$result_file"
}

if [ $# -eq 0 ]; then
  BATCHES=(V1 V2 V3 V4 V5)
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
