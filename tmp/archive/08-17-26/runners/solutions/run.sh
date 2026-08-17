#!/usr/bin/env bash
# solutions runner — drives the 735-task backlog from tmp/solutions/roko/
#
# See README.md for overview, ISSUE-TRACKER.md for the unfixed-issue checklist,
# context-pack/ for global rules, and bin/generate-prompts.py for how prompts
# are produced from the source task files.

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export RUNNER_NAME="solutions"
export RUNNER_ROOT="$SCRIPT_DIR"
export LOG_ROOT="$SCRIPT_DIR/logs"
export PROMPTS_DIR="$SCRIPT_DIR/prompts"
export CONTEXT_DIR="$SCRIPT_DIR/context-pack"

: "${CONV_MODEL:=gpt-5.4-mini}"
: "${CONV_REASONING:=xhigh}"

: "${AUDIT_MODEL:=gpt-5.4-mini}"
: "${AUDIT_REASONING:=xhigh}"
: "${AUDIT_ENABLED:=0}"

: "${PARALLEL:=20}"

export CONV_MODEL CONV_REASONING AUDIT_MODEL AUDIT_REASONING AUDIT_ENABLED PARALLEL

exec bash "$SCRIPT_DIR/../parallel-template/run-parallel.sh" "$@"
