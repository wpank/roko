#!/usr/bin/env bash
set -euo pipefail

echo "=== Waiting for PU run (PID $1) to finish ==="
wait "$1" 2>/dev/null || true

echo "=== PU finished. Starting PE ==="
bash tmp/refinement-audit-runner/run-parity-exec.sh 2>&1 | tee tmp/refinement-audit-runner/logs/pe-full.log
