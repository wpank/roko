#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export RUNNER_NAME="audit-2026-05-01"
export RUNNER_ROOT="$SCRIPT_DIR"
export LOG_ROOT="$SCRIPT_DIR/logs"
export PROMPTS_DIR="$SCRIPT_DIR/prompts"
export CONTEXT_DIR="$SCRIPT_DIR/context-pack"

# ── Model config ──
# Implementation: GPT-5.4 mini at max reasoning (mechanical edits, lots of files)
: "${CONV_MODEL:=gpt-5.4-mini}"
: "${CONV_REASONING:=xhigh}"

: "${AUDIT_MODEL:=gpt-5.4-mini}"
: "${AUDIT_REASONING:=xhigh}"
: "${AUDIT_ENABLED:=0}"

# ── Parallelism: 12 concurrent batches (some prompts touch large files) ──
: "${PARALLEL:=12}"

export CONV_MODEL CONV_REASONING AUDIT_MODEL AUDIT_REASONING AUDIT_ENABLED PARALLEL

exec bash "$SCRIPT_DIR/../parallel-template/run-parallel.sh" "$@"
