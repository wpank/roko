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
  [L1]="Use the shipped playbook-rule and skill surfaces in a smaller learned-context path"
  [L2]="Expose slice-aware regressions and the existing iteration threshold"
  [L3]="Canonicalize the shipped prediction and routing-log calibration path"
  [L4]="Decide whether subscriber and drift modules are live or explicitly demoted"
  [L5]="Make existing cost pressure visible in routing decisions without redesigning the router"
  [L6]="Persist experiment outcomes in an operator-visible artifact"
  [L7]="Align episode, pattern, and tier-progression docs with shipped behavior"
  [L8]="Demote demurrage, worldviews, and framework-heavy vision content to future work"
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
  [L1]="cargo test -p roko-cli -p roko-learn -p roko-compose && rg -n \"build_learned_context|MatchContext|SkillQuery|search_by_tag\" crates/roko-cli crates/roko-learn crates/roko-compose"
  [L2]="cargo test -p roko-learn -p roko-cli && rg -n \"detect_regressions|iterations_increase|slice:\" crates/roko-learn crates/roko-cli"
  [L3]="cargo test -p roko-learn -p roko-core -p roko-cli && rg -n \"PredictionRecord|CalibrationTracker|PredictionPolicy|PredictiveScorer|routing_log\" crates/roko-learn crates/roko-core crates/roko-cli"
  [L4]="cargo test -p roko-learn -p roko-cli && rg -n \"run_learning_subscriber|DriftDetector\" crates/roko-learn crates/roko-cli"
  [L5]="cargo test -p roko-learn -p roko-cli && rg -n \"BudgetGuardrail|BudgetAction|apply_cost_pressure|cascade_router\" crates/roko-learn crates/roko-cli"
  [L6]="cargo test -p roko-learn -p roko-cli && rg -n \"ExperimentStore|prompt_experiment|model_experiment|cascade_router\" crates/roko-learn crates/roko-cli"
  [L7]="rg -n \"EpisodeLogger|compact|PatternMiner|pattern_discovery|TierProgression|HeuristicRule\" crates/roko-learn crates/roko-neuro docs/05-learning tmp/docs-parity/05"
  [L8]="rg -n \"demurrage|worldview|replication-ledger|FEP|Friston|Viable System Model|constitutional\" docs/05-learning tmp/docs-parity/05 tmp/refinements-audit"
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
