#!/usr/bin/env bash
# run-docs-parity.sh — Execute 02-agent parity batches
#
# Usage:
#   ./run-docs-parity.sh              # Run all batches in dependency-safe order
#   ./run-docs-parity.sh G1           # Run a single batch
#   ./run-docs-parity.sh G1 G4 G7     # Run specific batches
#
# Each batch should be self-contained and leave a clear PASS / FAIL / BLOCKED result.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PARITY_DIR="$REPO_ROOT/tmp/docs-parity/02"
CONTEXT_PACK="$PARITY_DIR/context-pack"
LOG_DIR="$PARITY_DIR/logs"
RUN_ID="$(date +%Y%m%d-%H%M%S)"

mkdir -p "$LOG_DIR/$RUN_ID"

declare -A BATCH_DESC=(
  [G1]="Canonicalize response types inside roko-agent"
  [G2]="Move shared response surface into roko-core"
  [G3]="Close Anthropic HTTP tool-loop backend gap"
  [G4]="Route at least one orchestrator path through ToolDispatcher + ToolLoop"
  [G5]="Enforce max_tools and degradation caps"
  [G6]="Eliminate remaining direct safety-bypass creation sites"
  [G7]="Add typed temperament foundation"
  [G8]="Propagate temperament into runtime behavior"
)

declare -A BATCH_DEPS=(
  [G1]=""
  [G2]="G1"
  [G3]=""
  [G4]="G3"
  [G5]="G4"
  [G6]=""
  [G7]=""
  [G8]="G7 G4"
)

declare -A BATCH_VERIFY=(
  [G1]="cargo test -p roko-agent && cargo clippy -p roko-agent --no-deps -- -D warnings"
  [G2]="cargo test -p roko-core -p roko-agent -p roko-compose && cargo clippy -p roko-core -p roko-agent --no-deps -- -D warnings"
  [G3]="cargo test -p roko-agent tool_loop && cargo clippy -p roko-agent --no-deps -- -D warnings"
  [G4]="cargo test -p roko-cli -p roko-agent && cargo run -p roko-cli -- plan run plans/ --dry-run"
  [G5]="cargo test -p roko-agent -p roko-core && cargo clippy -p roko-agent -p roko-core --no-deps -- -D warnings"
  [G6]="cargo test -p roko-cli -p roko-agent"
  [G7]="cargo test -p roko-core -p roko-agent -p roko-cli && cargo clippy -p roko-core -p roko-agent -p roko-cli --no-deps -- -D warnings"
  [G8]="cargo test -p roko-agent -p roko-learn -p roko-cli -p roko-core && cargo clippy -p roko-agent -p roko-learn -p roko-core --no-deps -- -D warnings"
)

declare -A BATCH_FILES=(
  [G1]="A-core-abstractions.md C-tool-loop.md"
  [G2]="A-core-abstractions.md"
  [G3]="C-tool-loop.md B-provider-system.md"
  [G4]="C-tool-loop.md E-routing-temperament.md D-lifecycle-infrastructure.md"
  [G5]="C-tool-loop.md"
  [G6]="D-lifecycle-infrastructure.md"
  [G7]="E-routing-temperament.md A-core-abstractions.md"
  [G8]="E-routing-temperament.md B-provider-system.md C-tool-loop.md"
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
- $CONTEXT_PACK/agents-summary.md
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
  BATCHES=(G6 G1 G3 G4 G5 G2 G7 G8)
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
