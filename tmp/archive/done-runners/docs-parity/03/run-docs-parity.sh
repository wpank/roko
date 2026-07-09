#!/usr/bin/env bash
# run-docs-parity.sh — Execute PU03 composition audit batches
#
# Usage:
#   ./run-docs-parity.sh              # Run all batches in dependency-safe order
#   ./run-docs-parity.sh P1           # Run a single batch
#   ./run-docs-parity.sh P1 P4 P7     # Run specific batches
#
# Each batch should be self-contained and leave a clear PASS / FAIL / BLOCKED result.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PARITY_DIR="$REPO_ROOT/tmp/docs-parity/03"
CONTEXT_PACK="$PARITY_DIR/context-pack"
LOG_DIR="$PARITY_DIR/logs"
RUN_ID="$(date +%Y%m%d-%H%M%S)"

mkdir -p "$LOG_DIR/$RUN_ID"

declare -A BATCH_DESC=(
  [P1]="Audit static role-budget source-of-truth on the live prompt path"
  [P2]="Check one wired complexity-budget path without redesigning policy"
  [P3]="Tighten prompt-build observability on the live composer path"
  [P4]="Close the remaining live role-identity and prompt-glue gaps"
  [P5]="Audit the existing enrichment runtime seam and document its real scope"
  [P6]="Check the wired ContextProvider to ContextAssembler path and defer larger context theory"
  [P7]="Verify cache-marker and MCP-stanza behavior on the real builder path"
  [P8]="Make advanced-scorer naming honest without expanding into active-inference theory"
)

declare -A BATCH_DEPS=(
  [P1]=""
  [P2]="P1"
  [P3]="P2"
  [P4]=""
  [P5]=""
  [P6]=""
  [P7]=""
  [P8]=""
)

declare -A BATCH_VERIFY=(
  [P1]="cargo test -p roko-compose && rg -n \"budget_for\\(\" crates/roko-compose/src/templates crates/roko-compose/src/role_prompts.rs crates/roko-compose/src/budget.rs"
  [P2]="cargo test -p roko-compose -p roko-cli && rg -n \"adjusted_budget_for|Complexity\" crates/roko-compose crates/roko-cli"
  [P3]="cargo test -p roko-compose && rg -n \"PromptBuild|dropped\" crates/roko-compose/src/prompt.rs crates/roko-cli/src/prompting.rs"
  [P4]="cargo test -p roko-compose && rg -n \"Researcher|Conductor|Refactorer|tool_allowlist_instructions\" crates/roko-compose/src/role_prompts.rs"
  [P5]="cargo test -p roko-compose -p roko-cli && rg -n \"EnrichmentPipeline::new|run_steps|EnrichmentRuntimeClient\" crates/roko-cli/src/orchestrate.rs crates/roko-compose/src/enrichment"
  [P6]="cargo test -p roko-neuro -p roko-compose -p roko-cli && rg -n \"ContextProvider::new|resolve\\(|ContextAssembler|semantic_similarity|text_fingerprint\" crates/roko-cli/src/orchestrate.rs crates/roko-compose/src/context_provider.rs crates/roko-neuro/src/context.rs"
  [P7]="cargo test -p roko-compose && rg -n \"with_cache_markers|cache:system|cache:session|MCP_TOOLS_STANZA\" crates/roko-compose/src"
  [P8]="cargo test -p roko-compose -p roko-cli && rg -n \"ActiveInferenceScorer|SectionScorer\" crates/roko-compose/src/scorer.rs crates/roko-compose/src/role_prompts.rs crates/roko-cli/src/orchestrate.rs"
)

declare -A BATCH_FILES=(
  [P1]="C-role-templates.md E-budget-management.md"
  [P2]="C-role-templates.md E-budget-management.md"
  [P3]="A-composer-core.md E-budget-management.md"
  [P4]="C-role-templates.md"
  [P5]="D-enrichment-context.md"
  [P6]="D-enrichment-context.md"
  [P7]="B-system-prompt-builder.md C-role-templates.md"
  [P8]="F-advanced-allocation.md"
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
- $CONTEXT_PACK/composition-summary.md
- $CONTEXT_PACK/gaps-summary.md

Also read these batch-specific files:
$(for f in ${BATCH_FILES[$batch]}; do echo "- $PARITY_DIR/$f"; done)

Audit posture:
- Emphasize the wired composition path that actually ships.
- Document dormant or partial helper paths honestly.
- Defer VCG, MVT, distributed-context, and eval-theory work unless a small wired-path fix directly depends on them.
- Keep the task realistic for one agent in roughly 90 minutes.

Execution rules:
1. Trace the live path in code before editing.
2. Stay inside the batch scope from BATCHES.md, but prefer the narrowed audit posture above when the broader docs over-claim.
3. If a discovered task is out of scope, record it as deferred and do not expand the batch.
4. After changes, run the verify command and fix issues if practical.
5. Report: files changed, commands run, pass/fail status, exact runtime path checked, and intentional deferrals.

Verify command: ${BATCH_VERIFY[$batch]}
PROMPT

  echo "  Prompt written to $prompt_file"
  echo "  [TODO] Run this prompt with: claude --print \"\$(cat "$prompt_file")\" 2>&1 | tee $log_file"

  echo "PENDING" > "$result_file"
}

if [ $# -eq 0 ]; then
  BATCHES=(P1 P4 P7 P2 P3 P6 P5 P8)
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
