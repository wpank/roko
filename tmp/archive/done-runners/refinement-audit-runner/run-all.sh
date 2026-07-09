#!/usr/bin/env bash
# run-all.sh — chain wrapper for Phase 2 + Phase 3 runners
#
# Phase 1 (AUD01-AUD08) is DONE on branch codex/audit-runner-run-20260417-214125.
# This wrapper chains the remaining two phases:
#   Phase 2 (PU00-PU12): Parity content refresh (codex exec, main repo)
#   Phase 3 (PE00-PE12): Parity code execution (codex exec, worktree)
#
# All arguments are forwarded to both scripts.
#
# Usage:
#   bash tmp/refinement-audit-runner/run-all.sh
#   bash tmp/refinement-audit-runner/run-all.sh --dry-run
#   bash tmp/refinement-audit-runner/run-all.sh --list

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=== Phase 2: Parity content refresh ==="
bash "$SCRIPT_DIR/run-parity-refresh.sh" "$@" || echo "Phase 2 had failures"

echo "=== Phase 3: Parity code execution ==="
bash "$SCRIPT_DIR/run-parity-exec.sh" "$@" || echo "Phase 3 had failures"
