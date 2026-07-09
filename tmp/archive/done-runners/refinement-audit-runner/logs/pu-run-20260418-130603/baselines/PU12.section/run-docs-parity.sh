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
  [M1]="Resolve the CLI truth surface: real commands, missing scaffolders, and explain-command drift"
  [M2]="Resolve the server and sidecar truth surface, including the 9090 vs 6677 drift"
  [M3]="Reconcile TUI, Rosedust, and the 29-screen spec with the shipping tab/modal reality"
  [M4]="Regenerate Doc 17 as the canonical mixed-status doc for topic 12"
  [M5]="Apply uniform frontier framing to Spectre visualization docs"
  [M6]="Split backend-ready web/onboarding reality from frontend and A2UI frontier"
  [M7]="Apply frontier framing to sonification, UX innovation, and IDE strategy while crediting shipping MCP coverage"
  [M8]="Do the final banner, index, and consistency sweep"
)

declare -A BATCH_DEPS=(
  [M1]=""
  [M2]=""
  [M3]=""
  [M4]="M1 M2 M3"
  [M5]="M4"
  [M6]="M4"
  [M7]="M4"
  [M8]="M1 M2 M3 M4 M5 M6 M7"
)

declare -A BATCH_VERIFY=(
  [M1]="rg -n \"roko new|roko explain|model route|agent_|daemon|event-sources|provider|subscription\" docs/12-interfaces/01-*.md docs/12-interfaces/02-*.md docs/12-interfaces/03-*.md crates/roko-cli/src/main.rs tmp/docs-parity/12"
  [M2]="rg -n \"9090|6677|/api/events|/stream|/message|Implementation: Scaffold|Route Groups|/api/models|/api/routing/explain\" docs/12-interfaces/05-*.md docs/12-interfaces/06-*.md docs/12-interfaces/17-*.md crates/roko-cli/src/main.rs crates/roko-serve/README.md crates/roko-agent-server/src tmp/docs-parity/12"
  [M3]="rg -n \"29 screens|F1|F7|PostFX|Rosedust|command palette|global search|Inspect\" docs/12-interfaces/07-*.md docs/12-interfaces/08-*.md docs/12-interfaces/09-*.md crates/roko-cli/src/tui tmp/docs-parity/12"
  [M4]="rg -n \"Implementation|Shipping|Partial|Frontier|9090|6677|TUI|roko-serve|agent-server|WCAG\" docs/12-interfaces/17-*.md tmp/docs-parity/12"
  [M5]="rg -n \"Design — Phase 2\\+|Tier 2M|Spectre|status_bar|token_sparkline\" docs/12-interfaces/10-*.md docs/12-interfaces/11-*.md docs/12-interfaces/12-*.md tmp/docs-parity/12"
  [M6]="rg -n \"Design — Phase 2\\+|roko-serve|CLI onboarding|A2UI|frontend|portal\" docs/12-interfaces/13-*.md docs/12-interfaces/14-*.md docs/12-interfaces/15-*.md tmp/docs-parity/12"
  [M7]="rg -n \"Design — Phase 2\\+|Proposed|roko-mcp-code|ACP|VS Code|sonification|voice|gesture|multimodal\" docs/12-interfaces/16-*.md docs/12-interfaces/18-*.md docs/12-interfaces/20-*.md tmp/docs-parity/12"
  [M8]="rg -n \"^> \\*\\*Implementation\\*\\*:|^> \\*\\*Status\\*\\*:|tmp/docs-parity/12/00-INDEX.md\" docs/12-interfaces/*.md tmp/docs-parity/12"
)

declare -A BATCH_FILES=(
  [M1]="A-cli-and-config.md"
  [M2]="B-http-and-websocket.md F-access-innovation-ide.md"
  [M3]="C-tui-and-rosedust.md"
  [M4]="F-access-innovation-ide.md A-cli-and-config.md B-http-and-websocket.md C-tui-and-rosedust.md"
  [M5]="D-spectre-creatures.md"
  [M6]="E-web-onboarding-generative.md"
  [M7]="F-access-innovation-ide.md"
  [M8]="A-cli-and-config.md B-http-and-websocket.md C-tui-and-rosedust.md D-spectre-creatures.md E-web-onboarding-generative.md F-access-innovation-ide.md"
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
- $CONTEXT_PACK/interfaces-summary.md
- $CONTEXT_PACK/gaps-summary.md
- $CONTEXT_PACK/repo-map.md

Also read these batch-specific files:
$(for f in ${BATCH_FILES[$batch]}; do echo "- $PARITY_DIR/$f"; done)

Execution rules:
1. Search before changing docs.
2. Stay inside the batch scope from BATCHES.md.
3. If a discovered task is out of scope, record it as deferred and do not expand the batch.
4. Prefer correcting truth claims, status banners, and acceptance criteria over inventing implementation.
5. After changes, run the verify command and fix issues if practical.
6. Report: files changed, commands run, pass/fail status, and intentional deferrals.

Verify command: ${BATCH_VERIFY[$batch]}
PROMPT

  echo "  Prompt written to $prompt_file"
  echo "  [TODO] Run this prompt with: claude --print \"\$(cat \"$prompt_file\")\" 2>&1 | tee $log_file"

  echo "PENDING" > "$result_file"
}

if [ $# -eq 0 ]; then
  BATCHES=(M1 M2 M3 M4 M5 M6 M7 M8)
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
