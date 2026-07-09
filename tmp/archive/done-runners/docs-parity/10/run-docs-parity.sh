#!/usr/bin/env bash
# run-docs-parity.sh - Prepare prompts for 10-dreams parity batches

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PARITY_DIR="$REPO_ROOT/tmp/docs-parity/10"
CONTEXT_PACK="$PARITY_DIR/context-pack"
LOG_DIR="$PARITY_DIR/logs"
RUN_ID="$(date +%Y%m%d-%H%M%S)"

mkdir -p "$LOG_DIR/$RUN_ID"

declare -A BATCH_DESC=(
  [M1]="Confirm trigger, schedule, budget, and daemon reality"
  [M2]="Confirm replay, imagination, and consolidation reality"
  [M3]="Confirm shipped hypnagogia and threat simulation; keep expansions frontier"
  [M4]="Frontier-tag evolution, sleep-time compute, rendering, sharing, and related theory"
  [M5]="Clarify mixed integration surfaces and separate shipped reports from future systems"
  [M6]="Regenerate Doc 16 from code evidence"
  [M7]="Regenerate top-level dreams INDEX claims and notes"
  [M8]="Run the final consistency pass across topic 10"
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
  [M1]="rg -n \"DreamTrigger|DreamSchedulePolicy|manual_enabled|scheduled_cron|DreamHeartbeatPolicy|dream run\" docs/10-dreams/00-*.md docs/10-dreams/01-*.md docs/10-dreams/13-*.md crates/roko-dreams crates/roko-cli"
  [M2]="rg -n \"DreamReplayMode|utility_score|CounterfactualQuery|ImaginationMode|KnowledgeEntry|SQLite staging\" docs/10-dreams/02-*.md docs/10-dreams/03-*.md docs/10-dreams/04-*.md crates/roko-dreams"
  [M3]="rg -n \"HypnagogiaEngine|ThreatScenario|roko-golem|Targeted Dream Incubation|alpha\" docs/10-dreams/07-*.md docs/10-dreams/08-*.md docs/10-dreams/09-*.md crates/roko-dreams"
  [M4]="rg -n \"MAP-Elites|Sleepwalker|rethink_memory|Oneirography|hauntology|nightmare|lucid|world model\" docs/10-dreams/05-*.md docs/10-dreams/06-*.md docs/10-dreams/10-*.md docs/10-dreams/11-*.md docs/10-dreams/12-*.md docs/10-dreams/14-*.md docs/10-dreams/17-*.md"
  [M5]="rg -n \"DreamCycleReport|KnowledgeStore|PlaybookStore|mesh|nightmare|lucid|roko-golem\" docs/10-dreams/15-*.md docs/10-dreams/16-*.md docs/10-dreams/17-*.md crates/roko-dreams crates/roko-learn"
  [M6]="rg -n \"DreamTrigger|scheduled_cron|utility_score|CounterfactualQuery|HypnagogiaEngine|ThreatScenario|DreamCycleReport|roko-golem\" docs/10-dreams/16-*.md crates/roko-dreams crates/roko-cli"
  [M7]="rg -n \"roko-golem|Sleepwalker|Oneirography|Hypnagogia|Threat simulation|Mattar-Daw\" docs/10-dreams/INDEX.md"
  [M8]="rg -n \"roko-golem|Sleepwalker|Oneirography|nightmare|lucid|Target-state|target-state\" docs/10-dreams/*.md"
)

declare -A BATCH_FILES=(
  [M1]="A-vision-and-cycle.md"
  [M2]="B-nrem-rem-consolidation.md"
  [M3]="D-hypnagogia-divergence-threat.md"
  [M4]="C-hdc-evolution-compute.md F-frontier-concepts.md"
  [M5]="E-integration-status.md F-frontier-concepts.md"
  [M6]="E-integration-status.md SOURCE-INDEX.md"
  [M7]="00-INDEX.md E-integration-status.md F-frontier-concepts.md"
  [M8]="00-INDEX.md A-vision-and-cycle.md B-nrem-rem-consolidation.md C-hdc-evolution-compute.md D-hypnagogia-divergence-threat.md E-integration-status.md F-frontier-concepts.md"
)

run_batch() {
  local batch="$1"
  local desc="${BATCH_DESC[$batch]}"
  local log_file="$LOG_DIR/$RUN_ID/$batch.log"
  local result_file="$LOG_DIR/$RUN_ID/$batch.result"
  local prompt_file="$LOG_DIR/$RUN_ID/$batch.prompt"

  echo "=== [$batch] $desc ==="
  echo "  Prompt: $prompt_file"
  echo "  Log:    $log_file"
  if [ -n "${BATCH_DEPS[$batch]}" ]; then
    echo "  Depends on: ${BATCH_DEPS[$batch]}"
  fi

  cat > "$prompt_file" <<PROMPT
You are preparing documentation parity changes for Roko batch $batch.

## Task
$desc

Read these files first:
- $PARITY_DIR/00-INDEX.md
- $PARITY_DIR/BATCHES.md
- $PARITY_DIR/SOURCE-INDEX.md
- $CONTEXT_PACK/agent-runbook.md
- $CONTEXT_PACK/carry-forward-map.md
- $CONTEXT_PACK/repo-map.md
- $CONTEXT_PACK/dreams-summary.md
- $CONTEXT_PACK/gaps-summary.md

Also read these batch-specific files:
$(for f in ${BATCH_FILES[$batch]}; do echo "- $PARITY_DIR/$f"; done)

Execution rules:
1. Regenerate status from code; do not rely on stale docs.
2. Keep shipped runtime, shipped support, and target-state concepts separate.
3. If the task starts requiring runtime implementation, record the seam and defer it.
4. Run the verify command after edits and summarize the result.
5. Report files changed, commands run, and any intentional deferrals.

Dependency context: ${BATCH_DEPS[$batch]:-(none)}
Verify command: ${BATCH_VERIFY[$batch]}
PROMPT

  echo "READY" > "$result_file"
  echo "  Ready command: claude --print < \"$prompt_file\" 2>&1 | tee \"$log_file\""
}

if [ $# -eq 0 ]; then
  BATCHES=(M1 M2 M3 M4 M5 M6 M7 M8)
else
  BATCHES=("$@")
fi

echo "Docs-Parity Prompt Prep: $RUN_ID"
echo "Batches: ${BATCHES[*]}"
echo "Logs: $LOG_DIR/$RUN_ID/"
echo ""

for batch in "${BATCHES[@]}"; do
  run_batch "$batch"
  echo ""
done

echo "=== Prompt prep $RUN_ID complete ==="
echo "Results:"
for batch in "${BATCHES[@]}"; do
  result_file="$LOG_DIR/$RUN_ID/$batch.result"
  result="$(cat "$result_file" 2>/dev/null || echo 'N/A')"
  echo "  $batch: $result"
done
