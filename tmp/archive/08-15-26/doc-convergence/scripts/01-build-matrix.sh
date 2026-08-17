#!/usr/bin/env bash
# Phase 1: Build the topic matrix (single agent)
source "$(dirname "$0")/_common.sh"

log "Phase 1: Building topic matrix..."
log "This runs a single agent that reads all doc indexes and code to produce MATRIX.md"

prompt=$(<"$PROMPTS_DIR/01-build-matrix.md")

claude -p "$prompt" \
  --allowedTools 'Read,Grep,Glob,Bash(description:*)' \
  --output-format text \
  > "$STATUS_DIR/matrix-agent-output.txt" 2>&1

if [[ -f "$STATUS_DIR/MATRIX.md" ]]; then
  log "Phase 1 complete. Matrix written to: $STATUS_DIR/MATRIX.md"
  # Count topics
  topic_count=$(grep -c '^### [0-9]' "$STATUS_DIR/MATRIX.md" 2>/dev/null || echo "?")
  log "Topics found: $topic_count"
else
  log "ERROR: MATRIX.md was not created. Check $STATUS_DIR/matrix-agent-output.txt"
  exit 1
fi
