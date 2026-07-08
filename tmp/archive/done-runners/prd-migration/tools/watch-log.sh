#!/usr/bin/env bash
# watch-log.sh — tail a topic's stream-json log and pretty-print events live.
#
# Usage:
#   ./tools/watch-log.sh                         # watch latest run, all topics
#   ./tools/watch-log.sh 00-architecture         # watch one topic in latest run
#   ./tools/watch-log.sh <run_id> 00-architecture # specific run + topic
#
# Each stream-json event is parsed and formatted as:
#   [10:23:15] Read     /path/to/file.md
#   [10:23:18] Edit     /path/to/file.md
#   [10:23:20] Write    /path/to/output.md  (4521 bytes)
#   [10:23:22] text     The Engram is a content-addressed unit of cognition...
#   [10:24:00] result   ok ($0.42, 2m 16s)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MIGRATION_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# --- Argument parsing -------------------------------------------------------

if [[ $# -eq 0 ]]; then
    # Watch all topics in the latest run
    LATEST=$(ls -dt "$MIGRATION_ROOT"/logs/run-* 2>/dev/null | head -1)
    if [[ -z "$LATEST" ]]; then
        echo "No run directories found at $MIGRATION_ROOT/logs/run-*"
        exit 1
    fi
    RUN_DIR="$LATEST"
    TOPIC=""
elif [[ $# -eq 1 ]]; then
    # Watch one topic in the latest run
    LATEST=$(ls -dt "$MIGRATION_ROOT"/logs/run-* 2>/dev/null | head -1)
    if [[ -z "$LATEST" ]]; then
        echo "No run directories found"
        exit 1
    fi
    RUN_DIR="$LATEST"
    TOPIC="$1"
else
    # Specific run + topic
    if [[ -d "$MIGRATION_ROOT/logs/$1" ]]; then
        RUN_DIR="$MIGRATION_ROOT/logs/$1"
    elif [[ -d "$1" ]]; then
        RUN_DIR="$1"
    else
        echo "Run directory not found: $1"
        exit 1
    fi
    TOPIC="$2"
fi

# --- Pick the log file to watch --------------------------------------------

if [[ -z "$TOPIC" ]]; then
    # Watch all topics — tail -f multiple files
    LOG_FILES=()
    for f in "$RUN_DIR"/*.log; do
        [[ -f "$f" ]] && LOG_FILES+=("$f")
    done
    if [[ ${#LOG_FILES[@]} -eq 0 ]]; then
        echo "No log files in $RUN_DIR yet. The run may still be starting."
        exit 1
    fi
    echo "Watching ${#LOG_FILES[@]} log files in $RUN_DIR"
    echo "Press Ctrl-C to stop."
    echo
    # When watching multiple files, prefix each line with the filename
    tail -f "${LOG_FILES[@]}" | python3 "$SCRIPT_DIR/watch-log-formatter.py" --multi
else
    LOG_FILE="$RUN_DIR/${TOPIC}.log"
    if [[ ! -f "$LOG_FILE" ]]; then
        # Try short-form matching
        for f in "$RUN_DIR"/*.log; do
            base=$(basename "$f" .log)
            if [[ "$base" == "$TOPIC" || "$base" == "$TOPIC-"* ]]; then
                LOG_FILE="$f"
                break
            fi
        done
    fi

    if [[ ! -f "$LOG_FILE" ]]; then
        echo "Log file not found for topic '$TOPIC' in $RUN_DIR"
        echo "Available:"
        ls "$RUN_DIR"/*.log 2>/dev/null | xargs -n1 basename | sed 's/^/  /'
        exit 1
    fi

    echo "Watching: $LOG_FILE"
    echo "Press Ctrl-C to stop."
    echo
    tail -f "$LOG_FILE" | python3 "$SCRIPT_DIR/watch-log-formatter.py"
fi
