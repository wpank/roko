#!/usr/bin/env bash
# run-overnight.sh — Run PU (remaining) then PE (all), sequentially, no gaps.
#
# PU: updates tmp/docs-parity/ files (codex --full-auto in main repo)
# PE: updates crates/ code (codex --full-auto in main repo, commits to current branch)
#
# Usage:
#   bash tmp/refinement-audit-runner/run-overnight.sh

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROKO_ROOT="/Users/will/dev/nunchi/roko/roko"

echo "============================================"
echo "  Refinement Audit Runner — Overnight Chain"
echo "  $(date -Iseconds)"
echo "============================================"
echo

# ---------------------------------------------------------------
# Phase 2: Finish PU (parity content refresh)
# PU00-PU05 already done, this continues from where it left off
# ---------------------------------------------------------------
echo "=== PHASE 2: Parity content refresh (PU00-PU12) ==="
echo

bash "$SCRIPT_DIR/run-parity-refresh.sh" --continue last
PU_EXIT=$?

echo
if [[ $PU_EXIT -eq 0 ]]; then
  echo "Phase 2 completed successfully."
else
  echo "Phase 2 had failures (exit=$PU_EXIT). Continuing to Phase 3 anyway."
fi
echo

# ---------------------------------------------------------------
# Phase 3: Run PE (code updates) — directly in main repo
# Commits land on the current branch (agent-refinements)
# ---------------------------------------------------------------
echo "=== PHASE 3: Code parity execution (PE00-PE12) ==="
echo

bash "$SCRIPT_DIR/run-parity-exec.sh"
PE_EXIT=$?

echo
echo "============================================"
echo "  DONE"
echo "  Phase 2 exit: $PU_EXIT"
echo "  Phase 3 exit: $PE_EXIT"
echo "  $(date -Iseconds)"
echo "============================================"
