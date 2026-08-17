#!/usr/bin/env bash
# Perf runner — thin wrapper around tmp/runners/parallel-template.
#
# Drives the 21 PERF_NN batches defined in batches.toml.
# See README.md for the workflow and ISSUE-TRACKER.md for the master
# checklist.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export RUNNER_NAME="perf"
export RUNNER_ROOT="$SCRIPT_DIR"
export LOG_ROOT="$SCRIPT_DIR/logs"
export PROMPTS_DIR="$SCRIPT_DIR/prompts"
export CONTEXT_DIR="$SCRIPT_DIR/context-pack"

# ── Model config ──
# Implementation: GPT-5.4 mini (fast, cheap) at max reasoning — same
# default the post-parity / solutions runners use.
: "${CONV_MODEL:=gpt-5.4-mini}"
: "${CONV_REASONING:=xhigh}"

# Audit pass: disabled by default — kick off a manual audit run after a
# wave completes if you want a second-opinion review.
: "${AUDIT_MODEL:=gpt-5.4-mini}"
: "${AUDIT_REASONING:=xhigh}"
: "${AUDIT_ENABLED:=0}"

# ── Parallelism ──
# Default 15 because Wave 1 has 17 independent batches and the
# common dev box runs comfortably with 15-20 parallel codex sessions.
: "${PARALLEL:=15}"

export CONV_MODEL CONV_REASONING AUDIT_MODEL AUDIT_REASONING AUDIT_ENABLED PARALLEL

exec bash "$SCRIPT_DIR/../parallel-template/run-parallel.sh" "$@"
