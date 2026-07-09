#!/usr/bin/env bash
# Phase 3: Cross-topic synthesis (single agent)
source "$(dirname "$0")/_common.sh"

# Check that Phase 2 produced output
completed=$(find "$OUTPUT_DIR" -name '*.md' ! -name '00-*' 2>/dev/null | wc -l | tr -d ' ')
if (( completed < 5 )); then
  log "ERROR: Only $completed topic docs found in $OUTPUT_DIR."
  log "Run Phase 2 first: ./02-converge-topics.sh"
  exit 1
fi

log "Phase 3: Synthesizing $completed topic docs..."

prompt=$(<"$PROMPTS_DIR/03-synthesize.md")

claude -p "$prompt" \
  --allowedTools 'Read,Grep,Glob,Bash(description:*)' \
  --output-format text \
  > "$STATUS_DIR/synthesis-agent.log" 2>&1

if [[ -f "$OUTPUT_DIR/00-SYNTHESIS.md" ]]; then
  log "Phase 3 complete. Synthesis written to: $OUTPUT_DIR/00-SYNTHESIS.md"
else
  log "ERROR: 00-SYNTHESIS.md was not created. Check $STATUS_DIR/synthesis-agent.log"
  exit 1
fi
