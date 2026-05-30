#!/bin/bash
# ============================================================================
# Run All IDE Integration Tests
# ============================================================================
#
# Modes:
#   ./run-all.sh                        # Sequential, all suites
#   ./run-all.sh --parallel             # All 8 suites in parallel (fastest)
#   ./run-all.sh --bail                 # Stop on first failure
#   ./run-all.sh --quick                # Skip slow tests
#   ./run-all.sh --json                 # Machine-readable JSON (for agents)
#
# Run specific suites:
#   SUITES=core ./run-all.sh            # Just core
#   SUITES=core,models ./run-all.sh     # Core + models
#   SUITES=mcp,edge ./run-all.sh        # MCP + edge cases
#
# Agent-friendly (Claude, CI):
#   ./run-all.sh --json --bail          # JSON output, stop on first fail
#   ./run-all.sh --json --parallel      # Parallel JSON (fastest, for agents)
#   SUITES=core ./run-all.sh --json     # Single suite, JSON
#
# Each suite runs in complete isolation:
#   - Own ACP process per test
#   - Own FIFO directory (mktemp)
#   - Own log directory
#   - No shared state between suites or between parallel runs
#
# Multiple agents CAN run this script simultaneously.
# ============================================================================

set -uo pipefail
cd "$(dirname "$0")"

# --- Parse args ---------------------------------------------------------------

PARALLEL=false
BAIL=false
QUICK=false
JSON=false
VERBOSE=false
FILTER=""
BUILD=false
PASSTHROUGH_ARGS=()

for arg in "$@"; do
  case "$arg" in
    --parallel) PARALLEL=true ;;
    --bail)     BAIL=true; PASSTHROUGH_ARGS+=("--bail") ;;
    --quick)    QUICK=true; PASSTHROUGH_ARGS+=("--quick") ;;
    --json)     JSON=true; PASSTHROUGH_ARGS+=("--json") ;;
    --verbose)  VERBOSE=true; PASSTHROUGH_ARGS+=("--verbose") ;;
    --filter=*) FILTER="${arg#--filter=}"; PASSTHROUGH_ARGS+=("$arg") ;;
    --build)    BUILD=true ;;
    --help|-h)
      head -30 "$0" | grep "^#" | sed 's/^# \?//'
      exit 0
      ;;
  esac
done

export BAIL QUICK JSON VERBOSE FILTER

# --- Suite definitions -------------------------------------------------------

declare -a SUITE_SCRIPTS=(
  "test-core.sh"
  "test-models.sh"
  "test-mcp.sh"
  "test-edge-cases.sh"
  "test-session-lifecycle.sh"
  "test-streaming.sh"
  "test-tool-loop.sh"
  "test-config-options.sh"
)

declare -a SUITE_NAMES=(
  "Core Protocol"
  "Model & Provider"
  "MCP Integration"
  "Edge Cases"
  "Session Lifecycle"
  "Streaming Protocol"
  "Tool Loop"
  "Config Options"
)

# Filter suites if SUITES env is set
if [ -n "${SUITES:-}" ]; then
  IFS=',' read -ra WANTED <<< "$SUITES"
  FILTERED_SCRIPTS=()
  FILTERED_NAMES=()
  for i in "${!SUITE_SCRIPTS[@]}"; do
    for w in "${WANTED[@]}"; do
      if echo "${SUITE_SCRIPTS[$i]}" | grep -qi "$w"; then
        FILTERED_SCRIPTS+=("${SUITE_SCRIPTS[$i]}")
        FILTERED_NAMES+=("${SUITE_NAMES[$i]}")
        break
      fi
    done
  done
  SUITE_SCRIPTS=("${FILTERED_SCRIPTS[@]}")
  SUITE_NAMES=("${FILTERED_NAMES[@]}")
fi

# --- Instance ID for this run -----------------------------------------------

RUN_ID="$$_$(date +%s%N 2>/dev/null || date +%s)"
RUN_LOG_DIR="/tmp/roko-ide-tests/run_${RUN_ID}"
mkdir -p "$RUN_LOG_DIR"

# --- Optional build step (for agents) ----------------------------------------

if [ "$BUILD" = true ]; then
  if [ "$JSON" != true ]; then
    echo -e "  \033[2mBuilding roko...\033[0m"
  fi
  cd /Users/will/dev/nunchi/roko/roko
  if ! cargo build --release -p roko-cli 2>"$RUN_LOG_DIR/build-stderr.log"; then
    if [ "$JSON" = true ]; then
      printf '{"error":"build_failed","log":"%s"}\n' "$RUN_LOG_DIR/build-stderr.log"
    else
      echo -e "  \033[31mBuild failed!\033[0m See: $RUN_LOG_DIR/build-stderr.log"
      tail -15 "$RUN_LOG_DIR/build-stderr.log"
    fi
    exit 1
  fi
  cd "$(dirname "$0")"
fi

# --- Header -------------------------------------------------------------------

if [ "$JSON" != true ]; then
  echo ""
  echo -e "\033[1m\033[36m  ╔══════════════════════════════════════════════════════════════╗\033[0m"
  echo -e "\033[1m\033[36m  ║           Roko ACP — IDE Integration Test Suite             ║\033[0m"
  echo -e "\033[1m\033[36m  ╚══════════════════════════════════════════════════════════════╝\033[0m"
  echo ""
  if [ "$PARALLEL" = true ]; then
    echo -e "  \033[2mMode: parallel (${#SUITE_SCRIPTS[@]} suites)\033[0m"
  else
    echo -e "  \033[2mMode: sequential (${#SUITE_SCRIPTS[@]} suites)\033[0m"
  fi
  if [ -n "$FILTER" ]; then
    echo -e "  \033[2mFilter: $FILTER\033[0m"
  fi
  echo -e "  \033[2mLogs: $RUN_LOG_DIR\033[0m"
  echo ""
fi

# --- Run suites ---------------------------------------------------------------

START_TIME=$(perl -MTime::HiRes=time -e 'printf "%d\n", time()*1000' 2>/dev/null || python3 -c "import time; print(int(time.time()*1000))")

run_suite() {
  local script="$1"
  local name="$2"
  local idx="$3"

  if [ ! -f "$script" ]; then
    if [ "$JSON" = true ]; then
      printf '{"suite":"%s","error":"script not found","script":"%s"}\n' "$name" "$script"
    else
      echo -e "  \033[2m○ $name (not found: $script)\033[0m"
    fi
    return 1
  fi

  # Each suite gets its own LOG_DIR — fully isolated
  local suite_log_dir="$RUN_LOG_DIR/suite_${idx}_$(echo "$name" | tr ' ' '_' | tr '[:upper:]' '[:lower:]')"
  mkdir -p "$suite_log_dir"

  LOG_DIR="$suite_log_dir" bash "$script" "${PASSTHROUGH_ARGS[@]}"
  return $?
}

TOTAL_FAIL=0

if [ "$PARALLEL" = true ]; then
  # --- Parallel: all suites at once -------------------------------------------
  declare -a PIDS=()
  declare -a OUTPUTS=()

  for i in "${!SUITE_SCRIPTS[@]}"; do
    local_output="$RUN_LOG_DIR/suite_${i}_output.txt"
    OUTPUTS+=("$local_output")
    (
      run_suite "${SUITE_SCRIPTS[$i]}" "${SUITE_NAMES[$i]}" "$i"
    ) > "$local_output" 2>&1 &
    PIDS+=($!)
  done

  if [ "$JSON" != true ]; then
    echo -e "  \033[2mLaunched ${#PIDS[@]} suites in parallel...\033[0m"
    echo ""
  fi

  # Wait for all and collect exit codes
  declare -a EXIT_CODES=()
  for i in "${!PIDS[@]}"; do
    wait "${PIDS[$i]}" 2>/dev/null
    EXIT_CODES+=($?)
  done

  # Print results in order
  for i in "${!SUITE_SCRIPTS[@]}"; do
    if [ "$JSON" != true ]; then
      echo -e "\033[1m\033[36m  ── ${SUITE_NAMES[$i]} (exit: ${EXIT_CODES[$i]}) ──\033[0m"
    fi
    cat "${OUTPUTS[$i]}"
    TOTAL_FAIL=$((TOTAL_FAIL + ${EXIT_CODES[$i]}))
  done

else
  # --- Sequential -------------------------------------------------------------
  for i in "${!SUITE_SCRIPTS[@]}"; do
    run_suite "${SUITE_SCRIPTS[$i]}" "${SUITE_NAMES[$i]}" "$i"
    ec=$?
    TOTAL_FAIL=$((TOTAL_FAIL + ec))

    if [ "$BAIL" = true ] && [ $ec -gt 0 ]; then
      if [ "$JSON" != true ]; then
        echo -e "  \033[31m\033[1mBAIL: suite '${SUITE_NAMES[$i]}' failed (exit $ec)\033[0m"
      fi
      break
    fi
  done
fi

# --- Final summary -------------------------------------------------------------

END_TIME=$(perl -MTime::HiRes=time -e 'printf "%d\n", time()*1000' 2>/dev/null || python3 -c "import time; print(int(time.time()*1000))")
TOTAL_MS=$((END_TIME - START_TIME))

# Collect per-suite result files
GRAND_PASS=0
GRAND_FAIL=0
GRAND_SKIP=0
for f in "$RUN_LOG_DIR"/suite_*/; do
  for rf in "$f"*-results.txt; do
    if [ -f "$rf" ]; then
      read -r p f s < "$rf" 2>/dev/null || true
      GRAND_PASS=$((GRAND_PASS + ${p:-0}))
      GRAND_FAIL=$((GRAND_FAIL + ${f:-0}))
      GRAND_SKIP=$((GRAND_SKIP + ${s:-0}))
    fi
  done
done

if [ "$JSON" = true ]; then
  printf '{"total":true,"passed":%d,"failed":%d,"skipped":%d,"failed_suites":%d,"ms":%d,"log_dir":"%s"}\n' \
    "$GRAND_PASS" "$GRAND_FAIL" "$GRAND_SKIP" "$TOTAL_FAIL" "$TOTAL_MS" "$RUN_LOG_DIR"
else
  echo ""
  echo -e "\033[1m\033[36m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\033[0m"
  echo -e "\033[1m  Total time: \033[2m${TOTAL_MS}ms\033[0m"
  echo -e "\033[1m  Log dir:    \033[2m$RUN_LOG_DIR\033[0m"
  if [ $TOTAL_FAIL -gt 0 ]; then
    echo -e "\033[1m  Status:     \033[31m$TOTAL_FAIL suite(s) had failures\033[0m"
  else
    echo -e "\033[1m  Status:     \033[32mAll suites passed\033[0m"
  fi
  echo -e "\033[1m\033[36m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\033[0m"
  echo ""
fi

exit $TOTAL_FAIL
