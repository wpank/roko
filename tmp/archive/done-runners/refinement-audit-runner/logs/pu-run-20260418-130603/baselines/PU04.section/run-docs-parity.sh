#!/usr/bin/env bash
# run-docs-parity.sh — Execute 04-verification parity batches
#
# Usage:
#   ./run-docs-parity.sh              # Run all batches in dependency-safe order
#   ./run-docs-parity.sh V1           # Run a single batch
#   ./run-docs-parity.sh V1 V4 V8     # Run specific batches
#
# Each batch should be self-contained and leave a clear PASS / FAIL / BLOCKED result.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PARITY_DIR="$REPO_ROOT/tmp/docs-parity/04"
CONTEXT_PACK="$PARITY_DIR/context-pack"
LOG_DIR="$PARITY_DIR/logs"
RUN_ID="$(date +%Y%m%d-%H%M%S)"

mkdir -p "$LOG_DIR/$RUN_ID"

declare -A BATCH_DESC=(
  [V1]="Replace ad-hoc numeric dispatch with one canonical runtime rung contract"
  [V2]="Activate select_rungs and GatePipeline on a production path"
  [V3]="Make adaptive thresholds affect retries and skip decisions"
  [V4]="Feed structured GateFeedback into the AutoFix path"
  [V5]="Persist verification artifacts to disk with content-addressed metadata"
  [V6]="Activate and persist GateRatchet for long-running convergence"
  [V7]="Make higher-rung gates reachable only when runtime inputs actually exist"
  [V8]="Harden GateVerdict signal decay, tags, and chain integrity"
)

declare -A BATCH_DEPS=(
  [V1]=""
  [V2]="V1"
  [V3]="V2"
  [V4]="V2"
  [V5]="V2"
  [V6]="V2"
  [V7]="V2 V5"
  [V8]="V2"
)

declare -A BATCH_VERIFY=(
  [V1]="cargo test -p roko-cli -p roko-gate"
  [V2]="cargo test -p roko-cli -p roko-gate"
  [V3]="cargo test -p roko-cli -p roko-gate"
  [V4]="cargo test -p roko-cli -p roko-gate"
  [V5]="cargo test -p roko-cli -p roko-gate"
  [V6]="cargo test -p roko-cli -p roko-gate"
  [V7]="cargo test -p roko-cli -p roko-gate"
  [V8]="cargo test -p roko-core -p roko-cli"
)

declare -A BATCH_FILES=(
  [V1]="A-gate-foundation.md B-pipeline-rungs.md"
  [V2]="B-pipeline-rungs.md"
  [V3]="D-feedback-thresholds.md B-pipeline-rungs.md"
  [V4]="D-feedback-thresholds.md"
  [V5]="C-artifacts-ratcheting.md G-forensic-verdict-signals.md"
  [V6]="C-artifacts-ratcheting.md"
  [V7]="B-pipeline-rungs.md C-artifacts-ratcheting.md F-autonomous-evoskills.md"
  [V8]="G-forensic-verdict-signals.md"
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
- $CONTEXT_PACK/verification-summary.md
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
  BATCHES=(V1 V2 V3 V4 V5 V6 V7 V8)
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
