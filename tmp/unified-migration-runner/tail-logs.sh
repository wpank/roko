#!/usr/bin/env bash
# tail-logs.sh — Human-readable live tail of migration agent logs
#
# Usage:
#   bash tmp/unified-migration-runner/tail-logs.sh              # all agents, latest run
#   bash tmp/unified-migration-runner/tail-logs.sh A            # just agent A
#   bash tmp/unified-migration-runner/tail-logs.sh run-XXXXX    # specific run
#   bash tmp/unified-migration-runner/tail-logs.sh --raw        # raw JSONL (no parsing)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_ROOT="$SCRIPT_DIR/logs"

RAW=0
RUN_ID=""
AGENT_FILTER=""

for arg in "$@"; do
  case "$arg" in
    --raw) RAW=1 ;;
    [A-D]) AGENT_FILTER="$arg" ;;
    run-*) RUN_ID="$arg" ;;
  esac
done

if [[ -z "$RUN_ID" || ! -d "$LOG_ROOT/$RUN_ID" ]]; then
  RUN_ID=$(ls -1d "$LOG_ROOT"/run-* 2>/dev/null | sort | tail -1 | xargs basename 2>/dev/null)
fi

if [[ -z "$RUN_ID" ]]; then
  echo "No runs found in $LOG_ROOT"
  exit 1
fi

RUN_DIR="$LOG_ROOT/$RUN_ID"

# Build glob pattern
if [[ -n "$AGENT_FILTER" ]]; then
  PATTERN="$RUN_DIR/*agent-${AGENT_FILTER}*.log $RUN_DIR/M*.log"
else
  PATTERN="$RUN_DIR/*.log"
fi

echo "=== Tailing $RUN_ID (Ctrl-C to stop) ==="
echo "=== Pattern: $PATTERN ==="
echo

# Wait for log files to appear
attempts=0
while ! ls $PATTERN >/dev/null 2>&1; do
  echo "Waiting for log files... (${attempts}s)"
  sleep 2
  attempts=$((attempts + 2))
  if (( attempts > 60 )); then
    echo "Timed out waiting for logs"
    exit 1
  fi
done

if (( RAW == 1 )) || ! command -v jq >/dev/null 2>&1; then
  exec tail -f $PATTERN
fi

# Human-readable: parse JSONL events, pass through non-JSON lines
tail -f $PATTERN 2>/dev/null | while IFS= read -r line; do
  # Non-JSON lines (headers, footers) — pass through
  if [[ "$line" != "{"* ]]; then
    echo "$line"
    continue
  fi
  # Parse JSONL
  echo "$line" | jq -r '
    if .type == "item.completed" then
      if .item.type == "agent_message" then "💬 " + (.item.text | split("\n")[0])[0:200]
      elif .item.type == "command_execution" then "🔧 " + .item.command + " → exit " + (.item.exit_code | tostring)
      elif .item.type == "file_edit" then "📝 " + (.item.filename // "?")
      elif .item.type == "file_read" then "📖 " + (.item.filename // "?")
      else "  " + .item.type
      end
    elif .type == "turn.completed" then "📊 turn: " + (.usage.input_tokens | tostring) + " in, " + (.usage.output_tokens | tostring) + " out"
    elif .type == "thread.started" then "🧵 thread " + .thread_id
    elif .type == "turn.started" then "🔄 turn started"
    else empty
    end
  ' 2>/dev/null || echo "$line"
done
