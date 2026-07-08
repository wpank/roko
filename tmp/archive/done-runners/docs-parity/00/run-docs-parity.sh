#!/usr/bin/env bash
# run-docs-parity.sh — Prompt helper for docs-parity topic 00
#
# Usage:
#   ./run-docs-parity.sh            # emit all batch prompts in order
#   ./run-docs-parity.sh P2         # emit one batch prompt
#   ./run-docs-parity.sh P2 P4      # emit specific batch prompts

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PARITY_DIR="$REPO_ROOT/tmp/docs-parity/00"
CONTEXT_PACK="$PARITY_DIR/context-pack"
LOG_DIR="$PARITY_DIR/logs"
RUN_ID="$(date +%Y%m%d-%H%M%S)"

mkdir -p "$LOG_DIR/$RUN_ID"

declare -A BATCH_DESC=(
  [P1]="Tighten the parity contract and context pack"
  [P2]="Refresh foundation, trait, and loop analyses with keep/narrow/defer framing"
  [P3]="Correct architecture-layer status and implementation-scale facts"
  [P4]="Rewrite advanced and meta docs as deferred research or planning material"
  [P5]="Refresh source anchors and runner wording"
)

declare -A BATCH_DEPS=(
  [P1]=""
  [P2]="P1"
  [P3]="P1"
  [P4]="P2 P3"
  [P5]="P4"
)

declare -A BATCH_VERIFY=(
  [P1]="rg -n \"36 workspace members|322,088 Rust LOC|200\\+ routes|58K LOC|two live RokoEvent variants\" tmp/docs-parity/00/00-INDEX.md tmp/docs-parity/00/BATCHES.md tmp/docs-parity/00/context-pack/*.md"
  [P2]="rg -n \"planned|deferred|target-state|target narrative|exactly two live RokoEvent variants\" tmp/docs-parity/00/A-foundation.md tmp/docs-parity/00/B-trait-system.md tmp/docs-parity/00/C-cognitive-loop.md && ! rg -n \"Pulse is the live|Datum is the current contract|Bus is the shipped seventh trait\" tmp/docs-parity/00/A-foundation.md tmp/docs-parity/00/B-trait-system.md tmp/docs-parity/00/C-cognitive-loop.md"
  [P3]="rg -n \"36 workspace members|322,088 Rust LOC|200\\+ routes|58K LOC|wired\" tmp/docs-parity/00/D-architecture-layers.md tmp/docs-parity/00/E-implementation-details.md
! rg -n \"HTTP API not[[:space:]]wired|Text-mode dashboard[[:space:]]only|177[Kk]|18\\+[[:space:]]crates\" tmp/docs-parity/00/D-architecture-layers.md tmp/docs-parity/00/E-implementation-details.md"
  [P4]="rg -n \"aspirational fiction|planning artifact|dependency ordering|single-developer-plus-agents\" tmp/docs-parity/00/F-advanced-capabilities.md tmp/docs-parity/00/G-innovation-meta.md"
  [P5]="bash -n tmp/docs-parity/00/run-docs-parity.sh
rg -n \"verification, not evidence|spot-check anchors|23-architectural-analysis-improvements.*34|24-cross-section-integration-map.*165|30-cross-pollination-innovations.*34|34-synergy-integration-map.*25|35-consolidated-roadmap.*40\" tmp/docs-parity/00/SOURCE-INDEX.md"
)

declare -A BATCH_FILES=(
  [P1]="00-INDEX.md BATCHES.md context-pack/agent-runbook.md context-pack/architecture-summary.md context-pack/gaps-summary.md context-pack/carry-forward-map.md context-pack/repo-map.md"
  [P2]="A-foundation.md B-trait-system.md C-cognitive-loop.md"
  [P3]="D-architecture-layers.md E-implementation-details.md"
  [P4]="F-advanced-capabilities.md G-innovation-meta.md"
  [P5]="SOURCE-INDEX.md run-docs-parity.sh"
)

check_deps() {
  local batch="$1"
  local deps="${BATCH_DEPS[$batch]}"
  if [ -z "$deps" ]; then
    return 0
  fi

  for dep in $deps; do
    local result_file="$LOG_DIR/$RUN_ID/$dep.result"
    if [ ! -f "$result_file" ] || [ "$(cat "$result_file")" != "PASS" ]; then
      echo "  [BLOCKED] $batch depends on $dep"
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
  local prompt_file="$LOG_DIR/$RUN_ID/$batch.prompt"

  echo "=== [$batch] $desc ==="
  echo "  Log: $log_file"

  if ! check_deps "$batch"; then
    echo "BLOCKED" > "$result_file"
    return 1
  fi

  cat > "$prompt_file" <<PROMPT
You are executing docs-parity batch $batch for Roko topic 00.

Task: $desc

Scope rules:
- Edit only files under $PARITY_DIR
- Do not touch crates/, docs/, or any other directory
- Treat this as a docs-only truthfulness pass, not an implementation batch
- Separate shipped code from planned architecture
- Use the audit baseline: 36 workspace members, 322,088 Rust LOC, roko-serve wired with 200+ routes, TUI wired at ~58K LOC
- The live runtime bus is still a narrow utility with exactly two RokoEvent variants: PlanRevision and PrdPublished
- Do not describe Pulse, Datum, Demurrage, Worldview, or Custody as shipping code
- Prefer Engram-centered wording in parity materials
- Treat roadmap staffing language as planning-only; preserve dependency ordering, not a 5-7 engineer execution stance
- Treat SOURCE-INDEX anchors as verification aids, not proof that planned concepts exist

Read first:
- $PARITY_DIR/00-INDEX.md
- $PARITY_DIR/BATCHES.md
- $PARITY_DIR/SOURCE-INDEX.md
- $CONTEXT_PACK/agent-runbook.md
- $CONTEXT_PACK/architecture-summary.md
- $CONTEXT_PACK/gaps-summary.md
- $CONTEXT_PACK/carry-forward-map.md
- $CONTEXT_PACK/repo-map.md
- $REPO_ROOT/tmp/refinements-audit/00-MASTER-SUMMARY.md

Then read these batch files:
$(for f in ${BATCH_FILES[$batch]}; do echo "- $PARITY_DIR/$f"; done)

After editing:
- run the verify command
- summarize docs-only parity updates inside $PARITY_DIR, commands run, and any intentional deferrals

Verify command:
${BATCH_VERIFY[$batch]}
PROMPT

  echo "  Prompt written to $prompt_file"
  echo "  [TODO] Run with your agent of choice and tee output to $log_file"
  echo "PENDING" > "$result_file"
}

if [ $# -eq 0 ]; then
  BATCHES=(P1 P2 P3 P4 P5)
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
