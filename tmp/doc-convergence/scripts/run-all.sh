#!/usr/bin/env bash
# Run the full doc-convergence pipeline
#
# Usage:
#   ./run-all.sh                    # Run all phases
#   ./run-all.sh --from 3           # Resume from Phase 3
#   MAX_PARALLEL=3 ./run-all.sh     # Limit parallel agents in Phase 2
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/_common.sh"

FROM_PHASE="${1:-1}"
if [[ "$FROM_PHASE" == "--from" ]]; then
  FROM_PHASE="${2:-1}"
fi

mkdir -p "$OUTPUT_DIR" "$STATUS_DIR"

log "============================================"
log "  Doc Convergence Pipeline"
log "  Starting from Phase $FROM_PHASE"
log "  Max parallel agents: $MAX_PARALLEL"
log "============================================"
log ""

STARTED_AT=$(date +%s)

if (( FROM_PHASE <= 1 )); then
  log ">>> PHASE 1: Build Topic Matrix"
  "$SCRIPT_DIR/01-build-matrix.sh"
  log ""
fi

if (( FROM_PHASE <= 2 )); then
  log ">>> PHASE 2: Per-Topic Convergence (parallel)"
  "$SCRIPT_DIR/02-converge-topics.sh"
  log ""
fi

if (( FROM_PHASE <= 3 )); then
  log ">>> PHASE 3: Cross-Topic Synthesis"
  "$SCRIPT_DIR/03-synthesize.sh"
  log ""
fi

if (( FROM_PHASE <= 4 )); then
  log ">>> PHASE 4: Dogfood into Roko"
  "$SCRIPT_DIR/04-dogfood.sh"
  log ""
fi

if (( FROM_PHASE <= 5 )); then
  log ">>> PHASE 5: Architecture Redesign"
  "$SCRIPT_DIR/05-redesign.sh"
  log ""
fi

ENDED_AT=$(date +%s)
ELAPSED=$(( ENDED_AT - STARTED_AT ))

log "============================================"
log "  Pipeline Complete"
log "  Elapsed: $(( ELAPSED / 60 ))m $(( ELAPSED % 60 ))s"
log "============================================"
log ""
log "Output files:"
ls -1 "$OUTPUT_DIR/"*.md 2>/dev/null | while read -r f; do
  echo "  $(basename "$f") ($(wc -l < "$f") lines)"
done
log ""
log "Status files:"
ls -1 "$STATUS_DIR/"*.md 2>/dev/null | while read -r f; do
  echo "  $(basename "$f")"
done
log ""
log "Next steps:"
log "  1. Review output/00-SYNTHESIS.md for the cross-topic analysis"
log "  2. Review output/00-REDESIGN.md for architecture proposals"
log "  3. Check status/DOGFOOD-REPORT.md for what was fed into roko"
log "  4. Run 'roko prd list' to see the new PRDs"
log "  5. Copy output/*.md to docs/v3/ when ready to make it canonical"
