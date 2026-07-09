#!/usr/bin/env bash
# run-docs-parity.sh — Execute 12-interfaces parity batches

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PARITY_DIR="$REPO_ROOT/tmp/docs-parity/12"
CONTEXT_PACK="$PARITY_DIR/context-pack"
LOG_DIR="$PARITY_DIR/logs"
RUN_ID="$(date +%Y%m%d-%H%M%S)"

mkdir -p "$LOG_DIR/$RUN_ID"

declare -A BATCH_DESC=(
  [M1]="Refresh CLI and config truth: shipping command surface, missing roko new, and explain-command drift"
  [M2]="Refresh HTTP and realtime truth: roko-serve as shipping control plane, sidecar routes, and 9090 vs 6677 drift"
  [M3]="Refresh TUI and Rosedust truth: 58K LOC headline, F1-F7 tabs, ratatui wiring, and narrowed design-language claims"
  [M4]="Defer the zero-code halo: Spectre, first-party web UI, onboarding UI, and A2UI while keeping CLI onboarding honest"
  [M5]="Refresh status, accessibility, innovation, and IDE notes around the shipping core plus deferred halo"
  [M6]="Refresh pack scaffolding, source anchors, context pack, and final consistency checks"
)

declare -A BATCH_DEPS=(
  [M1]=""
  [M2]=""
  [M3]=""
  [M4]=""
  [M5]="M1 M2 M3 M4"
  [M6]="M1 M2 M3 M4 M5"
)

declare -A BATCH_VERIFY=(
  [M1]="rg -n \"roko new|PrdDraftCmd::New|model route|--explain|config\" crates/roko-cli/src/main.rs docs/12-interfaces/00-*.md docs/12-interfaces/03-*.md tmp/docs-parity/12/A-cli-and-config.md"
  [M2]="rg -n \"9090|6677|/api/events|/stream|/message|routing/explain|OpenAPI|gRPC\" crates/roko-cli/src/main.rs crates/roko-serve/README.md crates/roko-serve/src/routes crates/roko-agent-server/src docs/12-interfaces/05-*.md docs/12-interfaces/06-*.md tmp/docs-parity/12/B-http-and-websocket.md"
  [M3]="rg -n \"F1|F7|Rosedust|PostFX|29-screen|ratatui|command palette|global search\" crates/roko-cli/src/tui docs/12-interfaces/07-*.md docs/12-interfaces/08-*.md docs/12-interfaces/09-*.md tmp/docs-parity/12/C-tui-and-rosedust.md"
  [M4]="rg -n \"Spectre|Svelte|portal|A2UI|onboarding|wizard\" crates docs/12-interfaces/10-*.md docs/12-interfaces/13-*.md docs/12-interfaces/14-*.md docs/12-interfaces/15-*.md tmp/docs-parity/12/D-spectre-creatures.md tmp/docs-parity/12/E-web-onboarding-generative.md"
  [M5]="rg -n \"Scaffold|shipping core|sonification|ACP|VS Code|roko-mcp-code|StateHub|CLI parity\" docs/12-interfaces/16-*.md docs/12-interfaces/17-*.md docs/12-interfaces/18-*.md docs/12-interfaces/20-*.md tmp/docs-parity/12/F-access-innovation-ide.md"
  [M6]="rg -n \"200\\+ routes|30K LOC|58K LOC|9090|6677|roko new|roko explain|deferred|ship soon\" tmp/docs-parity/12 && bash -n tmp/docs-parity/12/run-docs-parity.sh"
)

declare -A BATCH_FILES=(
  [M1]="A-cli-and-config.md"
  [M2]="B-http-and-websocket.md"
  [M3]="C-tui-and-rosedust.md"
  [M4]="D-spectre-creatures.md E-web-onboarding-generative.md"
  [M5]="F-access-innovation-ide.md"
  [M6]="00-INDEX.md BATCHES.md SOURCE-INDEX.md context-pack/agent-runbook.md context-pack/carry-forward-map.md context-pack/gaps-summary.md context-pack/interfaces-summary.md context-pack/repo-map.md run-docs-parity.sh"
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
- $CONTEXT_PACK/interfaces-summary.md
- $CONTEXT_PACK/gaps-summary.md
- $CONTEXT_PACK/repo-map.md

Also read these batch-specific files:
$(for f in ${BATCH_FILES[$batch]}; do echo "- $PARITY_DIR/$f"; done)

Execution rules:
1. Search before changing docs.
2. Stay inside the batch scope from BATCHES.md.
3. Split shipping from planned; never describe deferred surfaces in present tense.
4. Prefer narrowing claims over adding new architecture.
5. After changes, run the verify command and fix issues if practical.
6. Report: files changed, commands run, pass/fail status, and intentional deferrals.

Verify command: ${BATCH_VERIFY[$batch]}
PROMPT

  echo "  Prompt written to $prompt_file"
  echo "  [TODO] Run this prompt with: claude --print \"\$(cat \"$prompt_file\")\" 2>&1 | tee $log_file"

  echo "PENDING" > "$result_file"
}

if [ $# -eq 0 ]; then
  BATCHES=(M1 M2 M3 M4 M5 M6)
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
