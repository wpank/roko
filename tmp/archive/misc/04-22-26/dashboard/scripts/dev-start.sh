#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

cd "$PROJECT_ROOT"

echo "Starting roko up..."
cargo run -p roko-cli -- up &
ROKO_PID=$!

# Wait for serve to be ready
sleep 3

# Start dashboard if available
DASHBOARD_DIR="/Users/will/dev/nunchi/nunchi-dashboard"
if [ -d "$DASHBOARD_DIR" ]; then
    echo "Starting dashboard..."
    cd "$DASHBOARD_DIR" && npm run dev &
    DASH_PID=$!
fi

echo "Ready! Serve: http://localhost:6677  Dashboard: http://localhost:5173"
echo "Press Ctrl+C to stop all."

trap 'kill $ROKO_PID ${DASH_PID:-} 2>/dev/null; wait' INT TERM
wait $ROKO_PID
