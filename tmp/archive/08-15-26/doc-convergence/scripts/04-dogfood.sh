#!/usr/bin/env bash
# Phase 4: Dogfood into roko's PRD/task system (single agent)
source "$(dirname "$0")/_common.sh"

if [[ ! -f "$OUTPUT_DIR/00-SYNTHESIS.md" ]]; then
  log "ERROR: 00-SYNTHESIS.md not found. Run Phase 3 first."
  exit 1
fi

log "Phase 4: Converting converged docs into roko PRDs and task files..."

prompt=$(<"$PROMPTS_DIR/04-dogfood.md")

claude -p "$prompt" \
  --allowedTools 'Read,Grep,Glob,Bash(description:*),Write,Edit' \
  --output-format text \
  > "$STATUS_DIR/dogfood-agent.log" 2>&1

if [[ -f "$STATUS_DIR/DOGFOOD-REPORT.md" ]]; then
  log "Phase 4 complete. Report: $STATUS_DIR/DOGFOOD-REPORT.md"
else
  log "WARN: DOGFOOD-REPORT.md not created. Check $STATUS_DIR/dogfood-agent.log"
fi

# Show what was created
log "PRD drafts created:"
ls -1 "$ROKO_ROOT/.roko/prd/drafts/" 2>/dev/null
log "Plan dirs created:"
ls -1d "$ROKO_ROOT/plans/"*/ 2>/dev/null
