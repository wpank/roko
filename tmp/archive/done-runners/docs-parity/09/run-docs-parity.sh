#!/usr/bin/env bash
# run-docs-parity.sh — Execute 09-daimon parity batches
#
# Usage:
#   ./run-docs-parity.sh              # Run all batches in recommended order
#   ./run-docs-parity.sh J1           # Run a single batch
#   ./run-docs-parity.sh J1 J4 J6     # Run specific batches
#
# Each batch should be self-contained and leave a clear PASS / FAIL / BLOCKED result.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PARITY_DIR="$REPO_ROOT/tmp/docs-parity/09"
CONTEXT_PACK="$PARITY_DIR/context-pack"
LOG_DIR="$PARITY_DIR/logs"
RUN_ID="$(date +%Y%m%d-%H%M%S)"

mkdir -p "$LOG_DIR/$RUN_ID"

declare -A BATCH_DESC=(
  [J1]="Reframe PAD labels and ALMA language around the shipping single-layer affect model"
  [J2]="Separate live appraisal/state behavior from richer theory and control-law prose"
  [J3]="Keep somatic and strategy-space surfaces in the shipping bucket while fencing off frontier claims"
  [J4]="Tighten emotional retrieval, prompt, and integration-point status around the live wiring"
  [J5]="Mark coding-agent integration as frontier instead of present-tense runtime behavior"
  [J6]="Treat collective contagion as explicit frontier and polish the status chapter"
  [J7]="Do the final consistency pass so topic 09 reads as mostly shipping with frontier edges"
)

declare -A BATCH_DEPS=(
  [J1]=""
  [J2]=""
  [J3]=""
  [J4]=""
  [J5]="J4"
  [J6]="J1 J3 J4 J5"
  [J7]="J1 J2 J3 J4 J5 J6"
)

declare -A BATCH_VERIFY=(
  [J1]="rg -n \"AffectOctant|Plutchik|ALMA|Personality layer|Design — Phase 2|target-state\" docs/09-daimon/01-*.md docs/09-daimon/02-*.md"
  [J2]="rg -n \"Eight-Step Pipeline|prediction-error threshold|hysteresis|dwell time|bandit\" docs/09-daimon/03-*.md docs/09-daimon/04-*.md docs/09-daimon/05-*.md"
  [J3]="rg -n \"role-aware|domain-native|mind wander|200-tick|Sub-1ms|kiddo\" docs/09-daimon/06-*.md docs/09-daimon/07-*.md docs/09-daimon/08-*.md"
  [J4]="rg -n \"Plutchik|discovery_emotion|ContextAssembler|four-factor|VCG|PromptComposer|externality|SystemPromptBuilder\" docs/09-daimon/09-*.md docs/09-daimon/10-*.md docs/09-daimon/11-*.md"
  [J5]="rg -n \"per-crate confidence|fatigue|error pattern|Design — Phase 2|target-state\" docs/09-daimon/11-*.md"
  [J6]="rg -n \"Design — Phase 2\\+|target-state|Tier 2M|Not started|roko-golem\" docs/09-daimon/12-*.md docs/09-daimon/13-*.md"
  [J7]="rg -n \"roko-golem|Implementation\\*: Built|ALMA temporal model|collective contagion|per-crate confidence\" docs/09-daimon/*.md"
)

declare -A BATCH_FILES=(
  [J1]="A-pad-and-temporal.md"
  [J2]="B-appraisal-and-states.md"
  [J3]="C-somatic-and-strategy.md"
  [J4]="D-memory-and-integration.md"
  [J5]="D-memory-and-integration.md E-collective-and-status.md"
  [J6]="E-collective-and-status.md"
  [J7]="A-pad-and-temporal.md B-appraisal-and-states.md C-somatic-and-strategy.md D-memory-and-integration.md E-collective-and-status.md"
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
- $CONTEXT_PACK/daimon-summary.md
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
  BATCHES=(J1 J2 J3 J4 J5 J6 J7)
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
