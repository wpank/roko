#!/usr/bin/env bash
# run-docs-parity.sh — Execute 06-neuro parity batches
#
# Usage:
#   ./run-docs-parity.sh              # Run all batches in recommended order
#   ./run-docs-parity.sh N1           # Run a single batch
#   ./run-docs-parity.sh N1 N3 N5     # Run specific batches
#
# Each batch should be self-contained and leave a clear PASS / FAIL / BLOCKED result.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PARITY_DIR="$REPO_ROOT/tmp/docs-parity/06"
CONTEXT_PACK="$PARITY_DIR/context-pack"
LOG_DIR="$PARITY_DIR/logs"
RUN_ID="$(date +%Y%m%d-%H%M%S)"

mkdir -p "$LOG_DIR/$RUN_ID"

declare -A BATCH_DESC=(
  [N1]="Activate ContextAssembler on a real orchestrator path"
  [N2]="Make the neuro query contract explicit and less doc-driven"
  [N3]="Harden the real distillation and promotion contract"
  [N4]="Resolve scheduler and quality-report ambiguity around distillation"
  [N5]="Make neuro ingest/source ownership honest and bounded"
  [N6]="Add or explicitly demote neuro backup/restore/publish surfaces"
  [N7]="Make doc-08 cross-domain transfer honest and optionally ship one tiny seam"
  [N8]="Triage advanced HDC enablers and defer the rest explicitly"
  [N9]="Clean up stale meta-docs, schema drift, and contradictory frontier claims"
)

declare -A BATCH_DEPS=(
  [N1]=""
  [N2]="N1"
  [N3]=""
  [N4]="N3"
  [N5]=""
  [N6]="N5"
  [N7]="N2"
  [N8]="N2"
  [N9]="N4 N6 N7 N8"
)

declare -A BATCH_VERIFY=(
  [N1]="cargo test -p roko-cli -p roko-neuro -p roko-compose"
  [N2]="cargo test -p roko-neuro -p roko-cli"
  [N3]="cargo test -p roko-neuro -p roko-dreams -p roko-cli"
  [N4]="cargo test -p roko-neuro -p roko-dreams -p roko-cli"
  [N5]="cargo test -p roko-neuro -p roko-cli"
  [N6]="cargo test -p roko-cli -p roko-neuro -p roko-fs"
  [N7]="rg -n \"Resonance|TransferRisk|DomainProfile|ConfirmationTracker|AnalogyResult\" crates docs/06-neuro tmp/docs-parity/06"
  [N8]="cargo test -p roko-primitives -p roko-neuro -p roko-index -p roko-learn"
  [N9]="rg -n \"roko-golem|Fact|FACT_HALF_LIFE_DAYS|KnowledgeCrystal|Pheromone|Dreams cycle|cross-domain transfer\" docs CLAUDE.md tmp/docs-parity/06"
)

declare -A BATCH_FILES=(
  [N1]="C-query-crossdomain-context.md E-somatic-exchange-backup.md"
  [N2]="B-hdc-foundations-operations.md C-query-crossdomain-context.md"
  [N3]="D-distillation-progression.md A-knowledge-types-tiers-decay.md"
  [N4]="D-distillation-progression.md"
  [N5]="E-somatic-exchange-backup.md A-knowledge-types-tiers-decay.md"
  [N6]="E-somatic-exchange-backup.md"
  [N7]="C-query-crossdomain-context.md F-status-frontier.md"
  [N8]="B-hdc-foundations-operations.md C-query-crossdomain-context.md"
  [N9]="A-knowledge-types-tiers-decay.md F-status-frontier.md"
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
- $CONTEXT_PACK/neuro-summary.md
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
  BATCHES=(N1 N2 N3 N5 N6 N4 N7 N8 N9)
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
