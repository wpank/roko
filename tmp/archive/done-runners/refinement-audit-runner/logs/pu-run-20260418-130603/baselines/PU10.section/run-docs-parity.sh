#!/usr/bin/env bash
# run-docs-parity.sh — Execute 10-dreams parity batches

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PARITY_DIR="$REPO_ROOT/tmp/docs-parity/10"
CONTEXT_PACK="$PARITY_DIR/context-pack"
LOG_DIR="$PARITY_DIR/logs"
RUN_ID="$(date +%Y%m%d-%H%M%S)"

mkdir -p "$LOG_DIR/$RUN_ID"

declare -A BATCH_DESC=(
  [M1]="Lock in trigger/scheduling/runtime ownership and correct manual/scheduled trigger drift"
  [M2]="Reconcile NREM/REM/consolidation docs with replay/imagination reality and simpler staging path"
  [M3]="Correct hypnagogia and threat-simulation ownership and strengthen frontier banners"
  [M4]="Frontier pass for evolution, sleep-time compute, hauntology, rendering, oneirography, and advanced concepts"
  [M5]="Sharpen mixed integration/status docs and split shipped report journaling from future sharing/nightmare systems"
  [M6]="Regenerate Doc 16 from current runtime and supporting infrastructure"
  [M7]="Rebuild top-level INDEX.md claims and stale generation notes"
  [M8]="Do the final global banner and housekeeping pass across topic 10"
)

declare -A BATCH_DEPS=(
  [M1]=""
  [M2]=""
  [M3]=""
  [M4]=""
  [M5]="M1 M2 M3 M4"
  [M6]="M1 M2 M3 M5"
  [M7]="M4 M6"
  [M8]="M1 M2 M3 M4 M5 M6 M7"
)

declare -A BATCH_VERIFY=(
  [M1]="rg -n \"scheduled|manual trigger|DreamTrigger|scheduled_cron|dream run|DreamHeartbeatPolicy\" docs/10-dreams/00-*.md docs/10-dreams/01-*.md docs/10-dreams/13-*.md crates/roko-dreams crates/roko-cli"
  [M2]="rg -n \"Mattar-Daw|Counterfactual|Boden|SQLite staging|KnowledgeEntry|utility_score\" docs/10-dreams/02-*.md docs/10-dreams/03-*.md docs/10-dreams/04-*.md crates/roko-dreams"
  [M3]="rg -n \"HypnagogiaEngine|ThreatScenario|roko-golem|Targeted Dream Incubation|alpha|Constitutional\" docs/10-dreams/07-*.md docs/10-dreams/08-*.md docs/10-dreams/09-*.md crates/roko-dreams"
  [M4]="rg -n \"Design — Phase 2\\+|MAP-Elites|Sleepwalker|rethink_memory|hauntology|Oneirography|world model|nightmare|lucid\" docs/10-dreams/05-*.md docs/10-dreams/06-*.md docs/10-dreams/10-*.md docs/10-dreams/11-*.md docs/10-dreams/12-*.md docs/10-dreams/14-*.md docs/10-dreams/17-*.md"
  [M5]="rg -n \"mesh|nightmare|dream journal|lucid|oneirography|DreamCycleReport|Design — Phase 2\\+|roko-golem\" docs/10-dreams/15-*.md docs/10-dreams/16-*.md docs/10-dreams/17-*.md"
  [M6]="rg -n \"roko-golem|Mattar-Daw|Counterfactual|Hypnagogia|Threat simulation|dream run|scheduled trigger\" docs/10-dreams/16-*.md"
  [M7]="rg -n \"roko-golem|Sleepwalker|Oneirography|Hypnagogia|Threat simulation|Mattar-Daw\" docs/10-dreams/INDEX.md"
  [M8]="rg -n \"^> \\*\\*Implementation\\*\\*:\" docs/10-dreams/*.md"
)

declare -A BATCH_FILES=(
  [M1]="A-vision-and-cycle.md"
  [M2]="B-nrem-rem-consolidation.md"
  [M3]="D-hypnagogia-divergence-threat.md"
  [M4]="C-hdc-evolution-compute.md F-frontier-concepts.md"
  [M5]="E-integration-status.md F-frontier-concepts.md"
  [M6]="A-vision-and-cycle.md B-nrem-rem-consolidation.md D-hypnagogia-divergence-threat.md E-integration-status.md"
  [M7]="E-integration-status.md F-frontier-concepts.md"
  [M8]="A-vision-and-cycle.md B-nrem-rem-consolidation.md C-hdc-evolution-compute.md D-hypnagogia-divergence-threat.md E-integration-status.md F-frontier-concepts.md"
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
- $CONTEXT_PACK/dreams-summary.md
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
