#!/usr/bin/env bash
# run-docs-parity.sh — Execute 11-safety parity batches

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PARITY_DIR="$REPO_ROOT/tmp/docs-parity/11"
CONTEXT_PACK="$PARITY_DIR/context-pack"
LOG_DIR="$PARITY_DIR/logs"
RUN_ID="$(date +%Y%m%d-%H%M%S)"

mkdir -p "$LOG_DIR/$RUN_ID"

declare -A BATCH_DESC=(
  [M1]="Refresh core shipped-safety notes in tmp/docs-parity/11"
  [M2]="Refresh threat/risk and coverage-status notes in tmp/docs-parity/11"
  [M3]="Mark chain-safety work deferred in tmp/docs-parity/11"
  [M4]="Refresh context-pack source-of-truth in tmp/docs-parity/11"
  [M5]="Run the final tmp/docs-parity/11 verify sweep"
)

declare -A BATCH_DEPS=(
  [M1]=""
  [M2]="M1"
  [M3]="M1"
  [M4]="M1 M2 M3"
  [M5]="M1 M2 M3 M4"
)

declare -A BATCH_VERIFY=(
  [M1]="rg -n '7,183|two crates|SafetyLayer|Capability<K>|AuditChain|TaintTracker|LoopGuard|SandboxEnforcer' \"$PARITY_DIR\"/00-INDEX.md \"$PARITY_DIR\"/SOURCE-INDEX.md \"$PARITY_DIR\"/A-defense-and-capabilities.md \"$PARITY_DIR\"/B-audit-taint-provenance.md \"$PARITY_DIR\"/C-runtime-guards.md"
  [M2]="rg -n 'coverage status|status refresh|shipping|defer|frontier|Phase 2|Doc 16' \"$PARITY_DIR\"/D-threat-risk-adaptive.md \"$PARITY_DIR\"/F-kernel-forensics-gap.md"
  [M3]="rg -n 'defer|deferred|Tier 6|Phase 2|not this batch|chain' \"$PARITY_DIR\"/E-chain-safety.md"
  [M4]="rg -n 'M1|M2|M3|M4|M5|7,183|two crates|doc/status refresh|defer|compliance|chain|kernel|frontier' \"$PARITY_DIR\"/context-pack/*.md"
  [M5]="rg -n '7,183|two crates|coverage status|doc/status refresh|defer|M1|M2|M3|M4|M5' \"$PARITY_DIR\"/*.md \"$PARITY_DIR\"/context-pack/*.md \"$PARITY_DIR\"/run-docs-parity.sh"
)

declare -A BATCH_FILES=(
  [M1]="00-INDEX.md SOURCE-INDEX.md A-defense-and-capabilities.md B-audit-taint-provenance.md C-runtime-guards.md"
  [M2]="D-threat-risk-adaptive.md F-kernel-forensics-gap.md"
  [M3]="E-chain-safety.md"
  [M4]="context-pack/agent-runbook.md context-pack/carry-forward-map.md context-pack/safety-summary.md context-pack/gaps-summary.md context-pack/repo-map.md"
  [M5]="00-INDEX.md SOURCE-INDEX.md A-defense-and-capabilities.md B-audit-taint-provenance.md C-runtime-guards.md D-threat-risk-adaptive.md E-chain-safety.md F-kernel-forensics-gap.md context-pack/agent-runbook.md context-pack/carry-forward-map.md context-pack/safety-summary.md context-pack/gaps-summary.md context-pack/repo-map.md run-docs-parity.sh"
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
- $PARITY_DIR/BATCHES.md
- $PARITY_DIR/SOURCE-INDEX.md
- $CONTEXT_PACK/agent-runbook.md
- $CONTEXT_PACK/carry-forward-map.md
- $CONTEXT_PACK/safety-summary.md
- $CONTEXT_PACK/gaps-summary.md
- $CONTEXT_PACK/repo-map.md

Also read these batch-specific files:
$(for f in ${BATCH_FILES[$batch]}; do echo "- $PARITY_DIR/$f"; done)

Execution rules:
1. Stay inside tmp/docs-parity/11 unless a quick code-anchor check is needed.
2. Use the batch framing: the shipped safety system spans two crates and 7,183 LOC.
3. Acknowledge the shipping safety system before describing any remaining gap.
4. Narrow the work to doc/status refresh; defer compliance, chain, kernel, and frontier design work.
5. After changes, run the verify command and fix issues if practical.
6. Report: files changed, commands run, pass/fail status, and intentional deferrals.

Verify command: ${BATCH_VERIFY[$batch]}
PROMPT

  echo "  Prompt written to $prompt_file"
  echo "  [TODO] Run this prompt with: claude --print \"\$(cat \"$prompt_file\")\" 2>&1 | tee $log_file"

  echo "PENDING" > "$result_file"
}

if [ $# -eq 0 ]; then
  BATCHES=(M1 M2 M3 M4 M5)
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
