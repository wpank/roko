#!/usr/bin/env bash
# run-docs-parity.sh — Execute 05-learning parity batches
#
# Usage:
#   ./run-docs-parity.sh              # Run all batches in recommended order
#   ./run-docs-parity.sh L1           # Run a single batch
#   ./run-docs-parity.sh L1 L3 L5     # Run specific batches
#
# Each batch should be self-contained and leave a clear PASS / FAIL / BLOCKED result.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PARITY_DIR="$REPO_ROOT/tmp/docs-parity/05"
CONTEXT_PACK="$PARITY_DIR/context-pack"
LOG_DIR="$PARITY_DIR/logs"
RUN_ID="$(date +%Y%m%d-%H%M%S)"

mkdir -p "$LOG_DIR/$RUN_ID"

declare -A BATCH_DESC=(
  [L1]="Activate richer learned-context matching for playbook rules and skills"
  [L2]="Make regression detection slice-aware and activate iteration regressions"
  [L3]="Canonicalize predictive calibration and add real metrics"
  [L4]="Resolve dead learning subscriber and drift scaffolding"
  [L5]="Turn budget pressure into a clearer routing input"
  [L6]="Materialize experiment winners into a durable operator-facing artifact"
  [L7]="Align episode/storage/clustering docs with the real runtime contract"
  [L8]="Demote prescriptive improvement-measurement and safety blocks to explicit handoff status"
)

declare -A BATCH_DEPS=(
  [L1]=""
  [L2]=""
  [L3]=""
  [L4]="L3"
  [L5]=""
  [L6]="L5"
  [L7]=""
  [L8]=""
)

declare -A BATCH_VERIFY=(
  [L1]="cargo test -p roko-cli -p roko-learn -p roko-compose"
  [L2]="cargo test -p roko-learn -p roko-cli"
  [L3]="cargo test -p roko-learn -p roko-core -p roko-cli"
  [L4]="cargo test -p roko-learn -p roko-cli"
  [L5]="cargo test -p roko-learn -p roko-cli"
  [L6]="cargo test -p roko-learn -p roko-cli"
  [L7]="rg -n \"compact|prune_stale|EpisodeStorageConfig|EpisodeCluster\" crates docs/05-learning tmp/docs-parity/05"
  [L8]="rg -n \"ImprovementScoreCard|SafetyInvariants|GateGamingDetector|ConstitutionalConstraints\" crates docs/05-learning tmp/docs-parity/05"
)

declare -A BATCH_FILES=(
  [L1]="B-knowledge-tiers.md E-feedback-calibration.md"
  [L2]="D-metrics-cost-health.md"
  [L3]="E-feedback-calibration.md C-routing-bandits.md"
  [L4]="E-feedback-calibration.md"
  [L5]="E-feedback-calibration.md D-metrics-cost-health.md"
  [L6]="E-feedback-calibration.md"
  [L7]="A-episodes-patterns.md B-knowledge-tiers.md"
  [L8]="F-frameworks-vision.md"
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
- $CONTEXT_PACK/learning-summary.md
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
  BATCHES=(L1 L2 L3 L5 L6 L4 L7 L8)
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
