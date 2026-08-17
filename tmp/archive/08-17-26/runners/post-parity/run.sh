#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export RUNNER_NAME="post-parity"
export RUNNER_ROOT="$SCRIPT_DIR"
export LOG_ROOT="$SCRIPT_DIR/logs"
export PROMPTS_DIR="$SCRIPT_DIR/prompts"
export CONTEXT_DIR="$SCRIPT_DIR/context-pack"

# ── Model config ──
# Implementation: GPT-5.4 mini (fast, cheap) at max reasoning
: "${CONV_MODEL:=gpt-5.4-mini}"
: "${CONV_REASONING:=xhigh}"

# Audit: disabled by default — run manual audit pass after batches complete
: "${AUDIT_MODEL:=gpt-5.4-mini}"
: "${AUDIT_REASONING:=xhigh}"
: "${AUDIT_ENABLED:=0}"

# ── Parallelism: 20 concurrent batches ──
: "${PARALLEL:=20}"

export CONV_MODEL CONV_REASONING AUDIT_MODEL AUDIT_REASONING AUDIT_ENABLED PARALLEL

exec bash "$SCRIPT_DIR/../parallel-template/run-parallel.sh" "$@"
