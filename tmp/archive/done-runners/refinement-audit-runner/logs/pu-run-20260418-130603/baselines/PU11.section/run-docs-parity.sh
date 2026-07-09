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
  [M1]="Acknowledge the orchestrator-layer safety crate across the core topic-11 docs"
  [M2]="Reframe capability tokens, tool tiers, and permission surfaces around the shipping split"
  [M3]="Calibrate AuditChain, lineage, and anchoring claims against the real implementation"
  [M4]="Split shipping TaintTracker behavior from Phase-2 taint theory and verify sink coverage"
  [M5]="Regenerate Doc 16 around SafetyLayer coverage instead of a generic critical-gap headline"
  [M6]="Apply stronger frontier and informational banners to compliance, chain, kernel, and advanced-risk docs"
  [M7]="Do the final topic-wide banner, index, and housekeeping sweep"
)

declare -A BATCH_DEPS=(
  [M1]=""
  [M2]="M1"
  [M3]="M1"
  [M4]="M1"
  [M5]="M1"
  [M6]="M1"
  [M7]="M1 M2 M3 M4 M5 M6"
)

declare -A BATCH_VERIFY=(
  [M1]="rg -n \"roko-orchestrator/src/safety|capability_tokens.rs|audit_chain.rs|taint_propagation.rs|loop_guard.rs|sandboxing.rs|contract.rs\" docs/11-safety tmp/docs-parity/11"
  [M2]="rg -n \"Capability<K>|CapabilityKind|target design|ToolPermission|AgentWarrant|SignalEmit\" docs/11-safety/01-*.md docs/11-safety/04-*.md tmp/docs-parity/11"
  [M3]="rg -n \"AuditChain|AuditEntry|ContentHash|ChainWitnessEngine|canonical\" docs/11-safety/02-*.md tmp/docs-parity/11"
  [M4]="rg -n \"TaintTracker|TaintReason|is_tainted|Denning|FIDES|PCAS|Design — Phase 2|ScrubPolicy\" docs/11-safety/03-*.md tmp/docs-parity/11 && rg -n \"is_tainted\" crates/roko-agent/src/safety crates/roko-orchestrator --include=*.rs"
  [M5]="rg -n \"Critical Integration Gap|SafetyLayer Coverage|provider|ToolDispatcher|subprocess|MCP\" docs/11-safety/16-*.md tmp/docs-parity/11"
  [M6]="rg -n \"Design — Phase 2\\+|Tier 6|Positioning|compliance framework|CaMeL|Ventriloquist\" docs/11-safety tmp/docs-parity/11"
  [M7]="rg -n \"^> \\*\\*Implementation\\*\\*:|tmp/docs-parity/11/00-INDEX.md|SafetyLayer Coverage|Critical Integration Gap\" docs/11-safety tmp/docs-parity/11"
)

declare -A BATCH_FILES=(
  [M1]="A-defense-and-capabilities.md B-audit-taint-provenance.md C-runtime-guards.md E-chain-safety.md"
  [M2]="A-defense-and-capabilities.md"
  [M3]="B-audit-taint-provenance.md E-chain-safety.md"
  [M4]="B-audit-taint-provenance.md C-runtime-guards.md"
  [M5]="C-runtime-guards.md F-kernel-forensics-gap.md"
  [M6]="A-defense-and-capabilities.md C-runtime-guards.md D-threat-risk-adaptive.md E-chain-safety.md F-kernel-forensics-gap.md"
  [M7]="A-defense-and-capabilities.md B-audit-taint-provenance.md C-runtime-guards.md D-threat-risk-adaptive.md E-chain-safety.md F-kernel-forensics-gap.md"
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
- $CONTEXT_PACK/safety-summary.md
- $CONTEXT_PACK/gaps-summary.md
- $CONTEXT_PACK/repo-map.md

Also read these batch-specific files:
$(for f in ${BATCH_FILES[$batch]}; do echo "- $PARITY_DIR/$f"; done)

Execution rules:
1. Search before changing docs.
2. Stay inside the batch scope from BATCHES.md.
3. If a discovered task is out of scope, record it as deferred and do not expand the batch.
4. Prefer correcting status, ownership, and banners over inventing new implementation.
5. After changes, run the verify command and fix issues if practical.
6. Report: files changed, commands run, pass/fail status, and intentional deferrals.

Verify command: ${BATCH_VERIFY[$batch]}
PROMPT

  echo "  Prompt written to $prompt_file"
  echo "  [TODO] Run this prompt with: claude --print \"\$(cat \"$prompt_file\")\" 2>&1 | tee $log_file"

  echo "PENDING" > "$result_file"
}

if [ $# -eq 0 ]; then
  BATCHES=(M1 M2 M3 M4 M5 M6 M7)
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
