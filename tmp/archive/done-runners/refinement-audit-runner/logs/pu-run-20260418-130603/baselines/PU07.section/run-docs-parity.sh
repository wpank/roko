#!/usr/bin/env bash
# run-docs-parity.sh — Execute 07-conductor parity batches
#
# Usage:
#   ./run-docs-parity.sh              # Run all batches in recommended order
#   ./run-docs-parity.sh C1           # Run a single batch
#   ./run-docs-parity.sh C1 C3 C5     # Run specific batches
#
# Each batch should be self-contained and leave a clear PASS / FAIL / BLOCKED result.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PARITY_DIR="$REPO_ROOT/tmp/docs-parity/07"
CONTEXT_PACK="$PARITY_DIR/context-pack"
LOG_DIR="$PARITY_DIR/logs"
RUN_ID="$(date +%Y%m%d-%H%M%S)"

mkdir -p "$LOG_DIR/$RUN_ID"

declare -A BATCH_DESC=(
  [C1]="Wire HealthMonitor into the orchestrator and resolve the health naming holdover"
  [C2]="Wire StuckDetector and MetaCognitionHook into a real runtime cadence"
  [C3]="Pick one canonical owner for agent and process accounting"
  [C4]="Persist circuit-breaker state across snapshots and restarts"
  [C5]="Make the state-machine, timeout, and attempt-tracking contract honest and bounded"
  [C6]="Clean up diagnosis, watcher, cooldown, and cognitive-signal contract drift"
  [C7]="Fix conductor status and meta-doc honesty around RoutingBias, signal kinds, and bandit wiring"
  [C8]="Mark design-only conductor theory sections explicitly and leave clean handoffs"
)

declare -A BATCH_DEPS=(
  [C1]=""
  [C2]="C1"
  [C3]=""
  [C4]=""
  [C5]="C1"
  [C6]="C2"
  [C7]="C1 C3 C5 C6"
  [C8]="C7"
)

declare -A BATCH_VERIFY=(
  [C1]="cargo test -p roko-cli -p roko-conductor"
  [C2]="cargo test -p roko-cli -p roko-conductor"
  [C3]="cargo test -p roko-cli -p roko-runtime -p roko-agent"
  [C4]="cargo test -p roko-orchestrator -p roko-conductor -p roko-cli"
  [C5]="cargo test -p roko-cli -p roko-conductor -p roko-learn"
  [C6]="cargo test -p roko-conductor -p roko-cli"
  [C7]="rg -n \"RoutingBias|conductor\\.intervention|conductor:alert|ConductorBandit|golem_status\" docs CLAUDE.md tmp/docs-parity/07 crates/roko-conductor crates/roko-cli"
  [C8]="rg -n \"Design — not yet implemented|Planned extension|Scaffold|PressureBandit|FlowDetector|CognitiveSignal|SelfHealingConductor\" docs/07-conductor tmp/docs-parity/07"
)

declare -A BATCH_FILES=(
  [C1]="E-health-adaptive.md A-architecture.md"
  [C2]="D-diagnosis-stuck.md B-watchers-signals.md"
  [C3]="E-health-adaptive.md"
  [C4]="C-decision-space.md"
  [C5]="E-health-adaptive.md C-decision-space.md"
  [C6]="D-diagnosis-stuck.md C-decision-space.md B-watchers-signals.md"
  [C7]="A-architecture.md F-theory-learning.md B-watchers-signals.md"
  [C8]="E-health-adaptive.md F-theory-learning.md B-watchers-signals.md"
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
- $CONTEXT_PACK/conductor-summary.md
- $CONTEXT_PACK/gaps-summary.md

Also read these batch-specific files:
$(for f in ${BATCH_FILES[$batch]}; do echo "- $PARITY_DIR/$f"; done)

Execution rules:
1. Search before building new code.
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
  BATCHES=(C1 C2 C3 C4 C5 C6 C7 C8)
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
