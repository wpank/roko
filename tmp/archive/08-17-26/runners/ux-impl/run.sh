#!/usr/bin/env bash
# ux-impl runner — applies the 12 implementation plans in
# `tmp/ux/implementation-plans/` as Codex-driven batches against a fresh
# git worktree, gated per-wave by cargo check + clippy and finally by
# cargo test --workspace.
#
# Each batch corresponds to exactly one row in ISSUE-TRACKER.md.
#
# Usage:
#   bash tmp/runners/ux-impl/run.sh --list                # show all batches
#   bash tmp/runners/ux-impl/run.sh --dry-run             # preview wave schedule
#   bash tmp/runners/ux-impl/run.sh                       # run everything
#   bash tmp/runners/ux-impl/run.sh --group AG            # one wave only
#   bash tmp/runners/ux-impl/run.sh --only AG02,AG03      # specific batches
#   bash tmp/runners/ux-impl/run.sh --continue            # resume latest run
#   bash tmp/runners/ux-impl/run.sh --no-test             # skip end-of-run test gate
#   bash tmp/runners/ux-impl/run.sh --pause               # pause between waves
#   bash tmp/runners/ux-impl/run.sh --status              # latest run status
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

export RUNNER_NAME="ux-impl"
export RUNNER_ROOT="$SCRIPT_DIR"
export LOG_ROOT="$SCRIPT_DIR/logs"
export PROMPTS_DIR="$SCRIPT_DIR/prompts"
export CONTEXT_DIR="$SCRIPT_DIR/context-pack"

exec bash "$SCRIPT_DIR/../parallel-template/run-parallel.sh" "$@"
