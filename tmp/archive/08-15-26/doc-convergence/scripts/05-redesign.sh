#!/usr/bin/env bash
# Phase 5: Architecture redesign (single agent)
source "$(dirname "$0")/_common.sh"

if [[ ! -f "$OUTPUT_DIR/00-SYNTHESIS.md" ]]; then
  log "ERROR: 00-SYNTHESIS.md not found. Run Phase 3 first."
  exit 1
fi

log "Phase 5: Architecture redesign pass..."

prompt=$(<"$PROMPTS_DIR/05-redesign.md")

claude -p "$prompt" \
  --allowedTools 'Read,Grep,Glob,Bash(description:*)' \
  --output-format text \
  > "$STATUS_DIR/redesign-agent.log" 2>&1

if [[ -f "$OUTPUT_DIR/00-REDESIGN.md" ]]; then
  log "Phase 5 complete. Redesign doc: $OUTPUT_DIR/00-REDESIGN.md"
else
  log "ERROR: 00-REDESIGN.md not created. Check $STATUS_DIR/redesign-agent.log"
  exit 1
fi
