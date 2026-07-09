#!/usr/bin/env bash
# run-docs-parity.sh - Prepare 07-conductor docs-refresh batches
#
# Usage:
#   ./run-docs-parity.sh              # Prepare all refresh batches
#   ./run-docs-parity.sh C1           # Prepare a single refresh batch
#   ./run-docs-parity.sh C1 C3        # Prepare specific refresh batches
#
# Each batch is docs-only and should stay inside the owned files under
# tmp/docs-parity/07/.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PARITY_DIR="$REPO_ROOT/tmp/docs-parity/07"
CONTEXT_PACK="$PARITY_DIR/context-pack"
LOG_DIR="$PARITY_DIR/logs"
RUN_ID="$(date +%Y%m%d-%H%M%S)"

mkdir -p "$LOG_DIR/$RUN_ID"

OWNED_FILES=(
  "$PARITY_DIR/00-INDEX.md"
  "$PARITY_DIR/BATCHES.md"
  "$PARITY_DIR/A-architecture.md"
  "$PARITY_DIR/B-watchers-signals.md"
  "$PARITY_DIR/C-decision-space.md"
  "$PARITY_DIR/D-diagnosis-stuck.md"
  "$PARITY_DIR/E-health-adaptive.md"
  "$PARITY_DIR/F-theory-learning.md"
  "$PARITY_DIR/AUDIT-LOG.md"
  "$PARITY_DIR/SOURCE-INDEX.md"
  "$CONTEXT_PACK/conductor-summary.md"
  "$CONTEXT_PACK/gaps-summary.md"
  "$CONTEXT_PACK/carry-forward-map.md"
  "$CONTEXT_PACK/repo-map.md"
  "$CONTEXT_PACK/agent-runbook.md"
  "$PARITY_DIR/run-docs-parity.sh"
)

declare -A BATCH_DESC=(
  [C1]="Reset section posture and architecture claims"
  [C2]="Refresh watcher and decision docs around what is actually live"
  [C3]="Refresh diagnosis, stuck, health, and process-support status"
  [C4]="Mark theory and learning chapters as informational or Phase 2+"
  [C5]="Refresh source anchors and operator context"
  [C6]="Append refresh audit note and update the runner script"
)

declare -A BATCH_DEPS=(
  [C1]=""
  [C2]="C1"
  [C3]="C2"
  [C4]="C3"
  [C5]="C4"
  [C6]="C5"
)

declare -A BATCH_VERIFY=(
  [C1]="rg -n \"10 watchers|RoutingBias|Bus<E>|Phase 2\\+\" \"$PARITY_DIR/00-INDEX.md\" \"$PARITY_DIR/A-architecture.md\""
  [C2]="rg -n \"conductor:alert|conductor.decision|CognitiveSignal|Continue \\| Restart \\| Fail\" \"$PARITY_DIR/B-watchers-signals.md\" \"$PARITY_DIR/C-decision-space.md\""
  [C3]="rg -n \"34 built-in patterns|6 heuristics|HealthMonitor|ProcessSupervisor|ownership split\" \"$PARITY_DIR/D-diagnosis-stuck.md\" \"$PARITY_DIR/E-health-adaptive.md\""
  [C4]="rg -n \"informational|Phase 2\\+|ConductorBandit|Yerkes-Dodson|Good Regulator\" \"$PARITY_DIR/F-theory-learning.md\""
  [C5]="rg -n \"conductor\\.rs:82-99|event_bus\\.rs:101-130|workspace members: 36|322,088\" \"$PARITY_DIR/SOURCE-INDEX.md\" \"$CONTEXT_PACK/repo-map.md\""
  [C6]="bash -n \"$PARITY_DIR/run-docs-parity.sh\""
)

declare -A BATCH_FILES=(
  [C1]="00-INDEX.md A-architecture.md"
  [C2]="B-watchers-signals.md C-decision-space.md"
  [C3]="D-diagnosis-stuck.md E-health-adaptive.md"
  [C4]="F-theory-learning.md"
  [C5]="SOURCE-INDEX.md context-pack/conductor-summary.md context-pack/gaps-summary.md context-pack/carry-forward-map.md context-pack/repo-map.md context-pack/agent-runbook.md"
  [C6]="AUDIT-LOG.md run-docs-parity.sh"
)

check_deps() {
  local batch="$1"
  local deps="${BATCH_DEPS[$batch]}"
  if [ -z "$deps" ]; then
    return 0
  fi
  for dep in $deps; do
    local result_file="$LOG_DIR/$RUN_ID/$dep.result"
    if [ ! -f "$result_file" ] || [ "$(cat "$result_file")" != "READY" ]; then
      echo "  [BLOCKED] $batch depends on $dep (not yet prepared)"
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
You are executing docs-refresh batch $batch for tmp/docs-parity/07/.

## Task

$desc

## Scope Rules

- This is a docs-only parity refresh.
- Read the current target files before editing them.
- Edit only these owned files:
$(for f in "${OWNED_FILES[@]}"; do echo "- $f"; done)
- Use source reads from the repo to confirm status, counts, and terminology.
- Use tmp/docs-parity/07/00-INDEX.md and tmp/docs-parity/07/BATCHES.md as the current execution contract.
- If a finding requires Rust code changes, record the handoff and stop.

## Suggested Reads

- $PARITY_DIR/AUDIT-LOG.md
- $PARITY_DIR/00-INDEX.md
- $PARITY_DIR/BATCHES.md
- $CONTEXT_PACK/conductor-summary.md
- $CONTEXT_PACK/gaps-summary.md
- $CONTEXT_PACK/carry-forward-map.md
- $CONTEXT_PACK/repo-map.md
- $CONTEXT_PACK/agent-runbook.md
- $PARITY_DIR/run-docs-parity.sh

## Batch Files

$(for f in ${BATCH_FILES[$batch]}; do echo "- $PARITY_DIR/$f"; done)

## Verify

${BATCH_VERIFY[$batch]}

## Report

Include:
- files changed,
- commands run,
- whether scope stayed docs-only,
- and intentional deferrals.
PROMPT

  echo "  Prompt written to $prompt_file"
  echo "  [TODO] Run this prompt with: claude --print \"\$(cat \"$prompt_file\")\" 2>&1 | tee $log_file"

  echo "READY" > "$result_file"
}

if [ $# -eq 0 ]; then
  BATCHES=(C1 C2 C3 C4 C5 C6)
else
  BATCHES=("$@")
fi

echo "Docs-Parity Refresh Run: $RUN_ID"
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
