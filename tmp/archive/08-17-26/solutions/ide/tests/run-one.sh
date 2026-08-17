#!/bin/bash
# ============================================================================
# Run a Single Test Suite (or filter within it)
# ============================================================================
# Designed for agents running individual test suites in parallel.
# Each invocation is fully isolated — safe for concurrent execution.
#
# Usage:
#   ./run-one.sh core                   # Run test-core.sh
#   ./run-one.sh models                 # Run test-models.sh
#   ./run-one.sh mcp                    # Run test-mcp.sh
#   ./run-one.sh edge                   # Run test-edge-cases.sh
#   ./run-one.sh lifecycle              # Run test-session-lifecycle.sh
#   ./run-one.sh streaming              # Run test-streaming.sh
#   ./run-one.sh toolloop               # Run test-tool-loop.sh
#   ./run-one.sh config                 # Run test-config-options.sh
#
# Options:
#   ./run-one.sh core --bail            # Stop on first failure
#   ./run-one.sh core --json            # JSON output (for agents)
#   ./run-one.sh core --verbose         # Show raw JSON-RPC traffic
#   ./run-one.sh core --quick           # Skip slow tests
#   ./run-one.sh core --filter=cancel   # Only tests matching "cancel"
#
# Multiple agents can run different suites simultaneously:
#   Agent 1: ./run-one.sh core --json --bail
#   Agent 2: ./run-one.sh models --json --bail
#   Agent 3: ./run-one.sh mcp --json --bail
#   Agent 4: ./run-one.sh edge --json --bail
#
# Exit code:
#   0 = all passed
#   N = number of failures
# ============================================================================

set -uo pipefail
cd "$(dirname "$0")"

if [ $# -lt 1 ] || [ "$1" = "--help" ] || [ "$1" = "-h" ]; then
  echo "Usage: $0 <suite> [--bail] [--json] [--verbose] [--quick] [--filter=PATTERN]"
  echo ""
  echo "Suites: core, models, mcp, edge, lifecycle, streaming, toolloop, config"
  exit 0
fi

SUITE="$1"
shift

# Map suite name to script
case "$SUITE" in
  core)       SCRIPT="test-core.sh" ;;
  models)     SCRIPT="test-models.sh" ;;
  mcp)        SCRIPT="test-mcp.sh" ;;
  edge)       SCRIPT="test-edge-cases.sh" ;;
  lifecycle)  SCRIPT="test-session-lifecycle.sh" ;;
  streaming)  SCRIPT="test-streaming.sh" ;;
  toolloop)   SCRIPT="test-tool-loop.sh" ;;
  config)     SCRIPT="test-config-options.sh" ;;
  *)
    echo "Unknown suite: $SUITE"
    echo "Available: core, models, mcp, edge, lifecycle, streaming, toolloop, config"
    exit 1
    ;;
esac

if [ ! -f "$SCRIPT" ]; then
  echo "Script not found: $SCRIPT"
  exit 1
fi

exec bash "$SCRIPT" "$@"
