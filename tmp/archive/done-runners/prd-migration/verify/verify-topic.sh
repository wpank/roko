#!/usr/bin/env bash
# verify-topic.sh — standalone wrapper to verify a single topic's output.
#
# Usage:
#   verify-topic.sh 00-architecture
#   verify-topic.sh 00-architecture --verbose
#   verify-topic.sh 00                       # short form — uses prefix match
#
# Exit codes:
#   0 — passed all quality checks
#   1 — failed (hard failures)
#   2 — passed with warnings only
#
# This is a thin wrapper around lib/verify.sh's verify_topic() function.
# You can run it manually after a single-topic run, or in a loop to re-verify
# everything.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MIGRATION_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# shellcheck source=../lib/common.sh
source "$MIGRATION_ROOT/lib/common.sh"
# shellcheck source=../lib/verify.sh
source "$MIGRATION_ROOT/lib/verify.sh"

VERBOSE=0
TOPIC=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --verbose|-v) VERBOSE=1; shift ;;
        -h|--help)
            echo "Usage: $0 <topic> [--verbose]"
            echo
            echo "Topics:"
            printf '  %s\n' "${ALL_TOPICS[@]}"
            exit 0
            ;;
        *) TOPIC="$1"; shift ;;
    esac
done

if [[ -z "$TOPIC" ]]; then
    log_err "verify" "No topic specified. Run with --help for usage."
    exit 1
fi

# Resolve short forms (e.g., "00" → "00-architecture")
RESOLVED=""
for candidate in "${ALL_TOPICS[@]}"; do
    if [[ "$candidate" == "$TOPIC" ]] || [[ "$candidate" == "$TOPIC-"* ]]; then
        RESOLVED="$candidate"
        break
    fi
done

if [[ -z "$RESOLVED" ]]; then
    log_err "verify" "Unknown topic: $TOPIC"
    exit 1
fi

TOPIC="$RESOLVED"

log_header "VERIFYING $TOPIC"

if (( VERBOSE == 1 )); then
    TOPIC_DIR=$(topic_output_dir "$TOPIC")
    log_info "$TOPIC" "Output dir: $TOPIC_DIR"
    if [[ -d "$TOPIC_DIR" ]]; then
        log_info "$TOPIC" "Files:"
        find "$TOPIC_DIR" -maxdepth 1 -type f -name '*.md' -exec basename {} \; | sort | sed "s/^/    /"
    fi
fi

verify_topic "$TOPIC"
exit $?
