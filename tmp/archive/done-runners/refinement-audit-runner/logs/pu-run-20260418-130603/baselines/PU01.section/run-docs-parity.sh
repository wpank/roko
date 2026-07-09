#!/usr/bin/env bash
# run-docs-parity.sh — Execute 01-orchestration parity batches
#
# Usage:
#   ./run-docs-parity.sh              # Run active batches in dependency-safe order
#   ./run-docs-parity.sh O1           # Run a single active batch
#   ./run-docs-parity.sh O1 O3 O5     # Run specific batches
#   ./run-docs-parity.sh O6           # Print the deferred coordination/domain note only
#
# Each active batch should be small enough to prove one live path and leave one
# explicit deferral behind.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PARITY_DIR="$REPO_ROOT/tmp/docs-parity/01"
CONTEXT_PACK="$PARITY_DIR/context-pack"
LOG_DIR="$PARITY_DIR/logs"
RUN_ID="$(date +%Y%m%d-%H%M%S)"

mkdir -p "$LOG_DIR/$RUN_ID"

VALID_BATCHES=(O1 O2 O3 O4 O5 O6)
DEFAULT_ACTIVE_BATCHES=(O1 O5 O2 O3 O4)
DEFERRED_BATCHES=(O6)

declare -A BATCH_DESC=(
  [O1]="Validate recovery inputs before trusting them"
  [O2]="Make speculative executor actions runtime-reachable"
  [O3]="Use UnifiedTaskDag on one live path"
  [O4]="Turn one background conductor finding into one bounded runtime effect"
  [O5]="Improve worktree liveness and one safe health check"
  [O6]="Deferred lane only: keep docs 12-13 out of batch 01 code work"
)

declare -A BATCH_DEPS=(
  [O1]=""
  [O2]=""
  [O3]=""
  [O4]=""
  [O5]=""
  [O6]=""
)

declare -A BATCH_VERIFY=(
  [O1]="cargo test -p roko-orchestrator -p roko-cli"
  [O2]="cargo test -p roko-cli -p roko-orchestrator"
  [O3]="cargo test -p roko-cli -p roko-orchestrator && cargo run -p roko-cli -- plan run plans/ --dry-run"
  [O4]="cargo test -p roko-cli -p roko-conductor"
  [O5]="cargo test -p roko-cli -p roko-orchestrator"
  [O6]="printf 'Deferred by audit: no code batch for O6\n'"
)

declare -A BATCH_FILES=(
  [O1]="C-persistence-recovery.md A-core-orchestration.md"
  [O2]="A-core-orchestration.md"
  [O3]="A-core-orchestration.md"
  [O4]="D-monitoring-conductor.md A-core-orchestration.md"
  [O5]="B-isolation-merge.md"
  [O6]="E-coordination-domains.md"
)

contains_batch() {
  local needle="$1"
  local batch
  for batch in "${VALID_BATCHES[@]}"; do
    if [ "$batch" = "$needle" ]; then
      return 0
    fi
  done
  return 1
}

is_deferred_batch() {
  local needle="$1"
  local batch
  for batch in "${DEFERRED_BATCHES[@]}"; do
    if [ "$batch" = "$needle" ]; then
      return 0
    fi
  done
  return 1
}

check_deps() {
  local batch="$1"
  local deps="${BATCH_DEPS[$batch]}"
  if [ -z "$deps" ]; then return 0; fi
  local dep
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

  if [ "$batch" = "O6" ]; then
    echo "  [DEFERRED] Docs 12-13 stay documented as future-state framing in batch 01"
    printf 'DEFERRED\n' > "$result_file"
    return 0
  fi

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
- $CONTEXT_PACK/orchestration-summary.md
- $CONTEXT_PACK/gaps-summary.md

Also read these batch-specific files:
$(for f in ${BATCH_FILES[$batch]}; do echo "- $PARITY_DIR/$f"; done)

Execution rules:
1. Prove one live runtime path or one bounded hardening seam.
2. Stay inside the batch scope from BATCHES.md.
3. If the task starts requiring new abstractions, domain models, or broad architecture cleanup, stop and record a deferral.
4. After changes, run the verify command and fix issues if practical.
5. Report: files changed, commands run, pass/fail status, and intentional deferrals.
6. Do not widen into docs 12-13, event-unification cleanup, or architecture-wide layer fixes.

Verify command: ${BATCH_VERIFY[$batch]}
PROMPT

  echo "  Prompt written to $prompt_file"
  echo "  [TODO] Run this prompt with: claude --print \"\$(cat "$prompt_file")\" 2>&1 | tee $log_file"

  echo "PENDING" > "$result_file"
}

if [ $# -eq 0 ]; then
  BATCHES=("${DEFAULT_ACTIVE_BATCHES[@]}")
else
  BATCHES=("$@")
fi

for batch in "${BATCHES[@]}"; do
  if ! contains_batch "$batch"; then
    echo "Unknown batch: $batch" >&2
    echo "Valid batches: ${VALID_BATCHES[*]}" >&2
    exit 1
  fi
done

echo "Docs-Parity Run: $RUN_ID"
echo "Batches: ${BATCHES[*]}"
echo "Logs: $LOG_DIR/$RUN_ID/"
echo "Active defaults: ${DEFAULT_ACTIVE_BATCHES[*]}"
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
  if is_deferred_batch "$batch"; then
    echo "  $batch: $result (docs-only deferral lane)"
  else
    echo "  $batch: $result"
  fi
done
