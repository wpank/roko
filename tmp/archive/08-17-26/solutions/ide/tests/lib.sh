#!/bin/bash
# ============================================================================
# ACP Test Library — shared utilities for roko IDE integration tests
# ============================================================================
# Usage: source this file from any test script
#   source "$(dirname "$0")/lib.sh"
#
# Flags (set via env or CLI before sourcing):
#   BAIL=true         Exit on first failure
#   JSON=true         Machine-readable JSON output (one line per test)
#   VERBOSE=true      Show raw JSON exchanges
#   QUICK=true        Skip slow tests
#   FILTER="pattern"  Only run tests matching pattern
#
# Environment:
#   ROKO_BIN          Path to roko binary (auto-detected)
#   ROKO_CONFIG       Path to roko.toml config (default: ~/.nunchi/roko/roko.toml)
#   ACP_TIMEOUT       Default timeout in seconds for ACP reads (default: 15)
#   BUILD_FIRST       If "true", run cargo build before tests (default: false)
#
# Isolation:
#   Each test script instance gets its own:
#   - FIFO directory (mktemp, unique per PID+timestamp)
#   - Log directory (unique per instance)
#   - ACP process (started/stopped per test)
#   Multiple agents can run different test scripts simultaneously without
#   any shared state or file conflicts.
# ============================================================================

set -uo pipefail
# Not set -e: we handle errors ourselves

# --- Parse CLI flags (before any test script logic) -------------------------

for arg in "$@"; do
  case "$arg" in
    --bail)     BAIL=true ;;
    --json)     JSON=true ;;
    --verbose)  VERBOSE=true ;;
    --quick)    QUICK=true ;;
    --filter=*) FILTER="${arg#--filter=}" ;;
    --help|-h)
      echo "Usage: $0 [--bail] [--json] [--verbose] [--quick] [--filter=PATTERN]"
      echo ""
      echo "  --bail       Exit immediately on first failure"
      echo "  --json       Output JSON (one object per test, for machine consumption)"
      echo "  --verbose    Show raw JSON-RPC traffic"
      echo "  --quick      Skip slow tests (multi-turn, thinking, etc.)"
      echo "  --filter=X   Only run tests whose name matches X (grep pattern)"
      exit 0
      ;;
  esac
done

BAIL="${BAIL:-false}"
JSON="${JSON:-false}"
VERBOSE="${VERBOSE:-false}"
QUICK="${QUICK:-false}"
FILTER="${FILTER:-}"
ACP_TIMEOUT="${ACP_TIMEOUT:-15}"

# --- Instance isolation (critical for parallel execution) -------------------

# Unique instance ID: PID + nanosecond timestamp — guarantees no collisions
# even when multiple agents launch tests in the same millisecond.
INSTANCE_ID="$$_$(date +%s%N 2>/dev/null || date +%s)"

# --- Config -----------------------------------------------------------------

ROKO_BIN="${ROKO_BIN:-$(command -v roko 2>/dev/null || echo "/Users/will/dev/nunchi/roko/roko/target/release/roko")}"
ROKO_CONFIG="${ROKO_CONFIG:-$HOME/.nunchi/roko/roko.toml}"
NUNCHI_MCP="${NUNCHI_MCP:-$(find "$HOME/dev/nunchi/roko/demo-ide/nunchi-mcp/target" -name nunchi-mcp -type f 2>/dev/null | head -1)}"
BRIDGE_TOKEN_FILE="${BRIDGE_TOKEN_FILE:-$HOME/.nunchi/bridge-token}"
BRIDGE_URL="${BRIDGE_URL:-http://127.0.0.1:6678}"

# Isolated log directory per instance — never shared
LOG_DIR="${LOG_DIR:-/tmp/roko-ide-tests/${INSTANCE_ID}}"
mkdir -p "$LOG_DIR"

# Isolated FIFO directory (never shared between instances)
FIFO_DIR=$(mktemp -d "/tmp/roko_fifo_${INSTANCE_ID}_XXXXXX")

# --- Cleanup trap -----------------------------------------------------------

_CLEANUP_PIDS=()
_cleanup() {
  # Kill any ACP processes we started
  for pid in "${_CLEANUP_PIDS[@]}"; do
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  done
  # Remove FIFOs
  rm -rf "$FIFO_DIR" 2>/dev/null || true
}
trap _cleanup EXIT INT TERM

# --- Colors & Formatting ----------------------------------------------------

if [ "$JSON" = true ] || [ ! -t 1 ]; then
  # No colors in JSON mode or when not attached to terminal
  RED='' GREEN='' YELLOW='' BLUE='' MAGENTA='' CYAN='' DIM='' BOLD='' RESET=''
  PASS_MARK='PASS' FAIL_MARK='FAIL' WARN_MARK='WARN' SKIP_MARK='SKIP'
else
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  YELLOW='\033[0;33m'
  BLUE='\033[0;34m'
  MAGENTA='\033[0;35m'
  CYAN='\033[0;36m'
  DIM='\033[2m'
  BOLD='\033[1m'
  RESET='\033[0m'
  PASS_MARK="${GREEN}✓${RESET}"
  FAIL_MARK="${RED}✗${RESET}"
  WARN_MARK="${YELLOW}⚠${RESET}"
  SKIP_MARK="${DIM}○${RESET}"
fi

# --- Counters ---------------------------------------------------------------

TESTS_PASSED=0
TESTS_FAILED=0
TESTS_WARNED=0
TESTS_SKIPPED=0
TEST_START_TIME=""
TEST_NAME=""
SUITE_NAME=""
SUITE_START_TIME=""

# --- Timing (fast, no python dependency for timing) -------------------------

_now_ms() {
  perl -MTime::HiRes=time -e 'printf "%d\n", time()*1000' 2>/dev/null \
    || python3 -c "import time; print(int(time.time()*1000))"
}

elapsed_ms() {
  local now=$(_now_ms)
  echo $(( now - TEST_START_TIME ))
}

# --- Output Helpers ----------------------------------------------------------

print_header() {
  SUITE_NAME="$1"
  SUITE_START_TIME=$(_now_ms)
  if [ "$JSON" = true ]; then
    : # no header in JSON mode
  else
    echo ""
    echo -e "${BOLD}${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    echo -e "${BOLD}${CYAN}  $1${RESET}"
    echo -e "${BOLD}${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    echo ""
  fi
}

print_section() {
  if [ "$JSON" = true ]; then
    : # no sections in JSON mode
  else
    echo ""
    echo -e "  ${BOLD}$1${RESET}"
    echo -e "  ${DIM}$(printf '%.0s─' {1..60})${RESET}"
  fi
}

print_env() {
  if [ "$JSON" = true ]; then
    return
  fi
  echo -e "  ${DIM}roko:${RESET}     $ROKO_BIN"
  echo -e "  ${DIM}config:${RESET}   $ROKO_CONFIG"
  echo -e "  ${DIM}mcp:${RESET}      ${NUNCHI_MCP:-not found}"
  echo -e "  ${DIM}logs:${RESET}     $LOG_DIR"
  echo -e "  ${DIM}instance:${RESET} $INSTANCE_ID"
  echo ""
}

# --- Test Framework ----------------------------------------------------------

# Check if this test should be skipped by filter
_should_skip_filter() {
  if [ -n "$FILTER" ]; then
    if ! echo "$TEST_NAME" | grep -qi "$FILTER"; then
      return 0  # should skip
    fi
  fi
  return 1  # should NOT skip
}

test_start() {
  TEST_NAME="$1"
  TEST_START_TIME=$(_now_ms)

  # Apply filter — skip silently
  if _should_skip_filter; then
    return 1  # caller should skip this test
  fi

  if [ "$JSON" != true ]; then
    printf "  %-55s " "$TEST_NAME"
  fi
  return 0
}

_emit_json() {
  local status="$1" detail="$2" ms="$3"
  if [ "$JSON" = true ]; then
    # Escape special chars in detail for valid JSON
    local escaped_detail
    escaped_detail=$(echo "$detail" | sed 's/\\/\\\\/g; s/"/\\"/g' | tr '\n' ' ' | head -c 200)
    printf '{"suite":"%s","test":"%s","status":"%s","ms":%s,"detail":"%s","instance":"%s","log":"%s"}\n' \
      "$SUITE_NAME" "$TEST_NAME" "$status" "$ms" \
      "$escaped_detail" \
      "$INSTANCE_ID" \
      "${ACP_LOG:-}"
  fi
}

test_pass() {
  local detail="${1:-}"
  local elapsed=$(elapsed_ms)
  if [ "$JSON" = true ]; then
    _emit_json "pass" "$detail" "$elapsed"
  else
    echo -e "${PASS_MARK} ${DIM}${elapsed}ms${RESET}${detail:+ ${DIM}($detail)${RESET}}"
  fi
  TESTS_PASSED=$((TESTS_PASSED + 1))
}

test_fail() {
  local detail="${1:-}"
  local elapsed=$(elapsed_ms)
  if [ "$JSON" = true ]; then
    _emit_json "fail" "$detail" "$elapsed"
  else
    echo -e "${FAIL_MARK} ${RED}FAIL${RESET} ${DIM}${elapsed}ms${RESET}${detail:+ ${RED}($detail)${RESET}}"
    # Always show log location on failure for debugging
    if [ -n "${ACP_LOG:-}" ] && [ -f "${ACP_LOG:-}" ]; then
      echo -e "    ${DIM}log: $ACP_LOG${RESET}"
      # Show last 5 lines of stderr log for immediate debugging context
      echo -e "    ${DIM}--- stderr tail ---${RESET}"
      tail -5 "$ACP_LOG" 2>/dev/null | while IFS= read -r logline; do
        echo -e "    ${DIM}  $logline${RESET}"
      done
      echo -e "    ${DIM}--- end ---${RESET}"
    fi
  fi
  TESTS_FAILED=$((TESTS_FAILED + 1))
  if [ "$BAIL" = true ]; then
    if [ "$JSON" != true ]; then
      echo ""
      echo -e "  ${RED}${BOLD}BAIL: stopping after first failure${RESET}"
    fi
    _emit_summary
    exit 1
  fi
}

test_skip() {
  local reason="${1:-}"
  if [ "$JSON" = true ]; then
    _emit_json "skip" "$reason" "0"
  else
    echo -e "${SKIP_MARK} ${DIM}SKIP${RESET}${reason:+ ${DIM}($reason)${RESET}}"
  fi
  TESTS_SKIPPED=$((TESTS_SKIPPED + 1))
}

test_warn() {
  local detail="${1:-}"
  local elapsed=$(elapsed_ms)
  if [ "$JSON" = true ]; then
    _emit_json "warn" "$detail" "$elapsed"
  else
    echo -e "${WARN_MARK} ${YELLOW}WARN${RESET} ${DIM}${elapsed}ms${RESET}${detail:+ ${YELLOW}($detail)${RESET}}"
  fi
  TESTS_WARNED=$((TESTS_WARNED + 1))
}

_emit_summary() {
  local suite_elapsed=$(( $(_now_ms) - SUITE_START_TIME ))
  if [ "$JSON" = true ]; then
    printf '{"suite":"%s","summary":true,"passed":%d,"failed":%d,"warned":%d,"skipped":%d,"ms":%d,"instance":"%s","log_dir":"%s"}\n' \
      "$SUITE_NAME" "$TESTS_PASSED" "$TESTS_FAILED" "$TESTS_WARNED" "$TESTS_SKIPPED" "$suite_elapsed" "$INSTANCE_ID" "$LOG_DIR"
  fi
}

print_summary() {
  _emit_summary
  if [ "$JSON" = true ]; then
    return
  fi
  echo ""
  echo -e "  ${DIM}$(printf '%.0s─' {1..60})${RESET}"
  local total=$((TESTS_PASSED + TESTS_FAILED + TESTS_WARNED + TESTS_SKIPPED))
  local parts="${GREEN}$TESTS_PASSED passed${RESET}, ${RED}$TESTS_FAILED failed${RESET}"
  if [ $TESTS_WARNED -gt 0 ]; then
    parts="$parts, ${YELLOW}$TESTS_WARNED warned${RESET}"
  fi
  parts="$parts, ${DIM}$TESTS_SKIPPED skipped${RESET}"
  local suite_elapsed=$(( $(_now_ms) - SUITE_START_TIME ))
  echo -e "  ${BOLD}Results:${RESET} $parts (${total} total, ${DIM}${suite_elapsed}ms${RESET})"
  echo -e "  ${DIM}Logs: $LOG_DIR${RESET}"

  if [ $TESTS_FAILED -eq 0 ] && [ $TESTS_WARNED -eq 0 ]; then
    echo -e "  ${GREEN}${BOLD}All tests passed!${RESET}"
  elif [ $TESTS_FAILED -eq 0 ]; then
    echo -e "  ${YELLOW}${BOLD}Passed with warnings.${RESET}"
  else
    echo -e "  ${RED}${BOLD}$TESTS_FAILED test(s) failed.${RESET}"
  fi
  echo ""
}

# --- ACP Session Helpers (isolated per instance) ----------------------------

ACP_PID=""
ACP_FIFO_IN=""
ACP_FIFO_OUT=""
ACP_LOG=""
ACP_SESSION_ID=""
ACP_RESPONSE=""
ACP_RAW=""
ACP_TOOL_CALLS=0
ACP_STOP_REASON=""
ACP_ERROR=""
ACP_NEW_RESULT=""

# Start an ACP process with isolated FIFOs
# Usage: acp_start [config_path]
acp_start() {
  local config="${1:-$ROKO_CONFIG}"
  local fifo_id="${INSTANCE_ID}_$(date +%s%N 2>/dev/null || echo $RANDOM)"

  ACP_FIFO_IN="$FIFO_DIR/in_${fifo_id}"
  ACP_FIFO_OUT="$FIFO_DIR/out_${fifo_id}"
  ACP_LOG="$LOG_DIR/acp_${fifo_id}.log"
  mkfifo "$ACP_FIFO_IN" "$ACP_FIFO_OUT"

  "$ROKO_BIN" acp --quiet --no-serve --config "$config" \
    < "$ACP_FIFO_IN" > "$ACP_FIFO_OUT" 2>"$ACP_LOG" &
  ACP_PID=$!
  _CLEANUP_PIDS+=("$ACP_PID")

  exec 3>"$ACP_FIFO_IN"
  exec 4<"$ACP_FIFO_OUT"

  if [ "$VERBOSE" = true ] && [ "$JSON" != true ]; then
    echo -e "    ${DIM}[acp] pid=$ACP_PID config=$config log=$ACP_LOG${RESET}"
  fi
}

# Stop the ACP process and clean up FIFOs
acp_stop() {
  exec 3>&- 2>/dev/null || true
  exec 4<&- 2>/dev/null || true
  if [ -n "$ACP_PID" ]; then
    kill "$ACP_PID" 2>/dev/null || true
    wait "$ACP_PID" 2>/dev/null || true
  fi
  rm -f "$ACP_FIFO_IN" "$ACP_FIFO_OUT" 2>/dev/null || true
  ACP_PID=""
}

# Send a JSON-RPC message
acp_send() {
  local msg="$1"
  if [ "$VERBOSE" = true ] && [ "$JSON" != true ]; then
    echo -e "    ${DIM}>>> $(echo "$msg" | head -c 120)${RESET}" >&2
  fi
  echo "$msg" >&3
}

# Read lines from ACP until a pattern matches or timeout
acp_read_until() {
  local pattern="$1"
  local timeout="${2:-$ACP_TIMEOUT}"
  ACP_LINES=""
  ACP_LAST_LINE=""

  while IFS= read -r -t "$timeout" line <&4; do
    ACP_LINES="${ACP_LINES}${line}"$'\n'
    if [ "$VERBOSE" = true ] && [ "$JSON" != true ]; then
      echo -e "    ${DIM}<<< $(echo "$line" | head -c 120)${RESET}" >&2
    fi
    if echo "$line" | grep -q "$pattern"; then
      ACP_LAST_LINE="$line"
      return 0
    fi
  done
  return 1
}

# Drain buffered notifications (1s timeout — fast drain)
acp_drain() {
  while IFS= read -r -t 1 line <&4; do
    if [ "$VERBOSE" = true ] && [ "$JSON" != true ]; then
      echo -e "    ${DIM}[drain] $(echo "$line" | head -c 80)${RESET}" >&2
    fi
  done
  return 0
}

# Create a session and set ACP_SESSION_ID
# Also captures the full session/new result in ACP_NEW_RESULT for inspection
acp_session_new() {
  local model="${1:-sonnet}"
  local extra="${2:-}"
  local params="{\"model\":\"$model\""
  if [ -n "$extra" ]; then
    params="$params,$extra"
  fi
  params="$params}"

  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/new\",\"id\":1,\"params\":$params}"

  ACP_SESSION_ID=""
  ACP_NEW_RESULT=""
  while IFS= read -r -t "$ACP_TIMEOUT" line <&4; do
    if [ "$VERBOSE" = true ] && [ "$JSON" != true ]; then
      echo -e "    ${DIM}<<< $(echo "$line" | head -c 120)${RESET}" >&2
    fi
    local sid
    sid=$(echo "$line" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('result',{}).get('sessionId',''))" 2>/dev/null) || true
    if [ -n "$sid" ] && echo "$sid" | grep -q "sess_"; then
      ACP_SESSION_ID="$sid"
      ACP_NEW_RESULT="$line"
      break
    fi
    # Also check for error responses
    if echo "$line" | grep -q '"error"'; then
      ACP_NEW_RESULT="$line"
      break
    fi
  done

  acp_drain

  if [ -z "$ACP_SESSION_ID" ]; then
    return 1
  fi
  return 0
}

# Send a prompt and collect the full response
acp_prompt() {
  local message="$1"
  local timeout="${2:-$ACP_TIMEOUT}"
  local id="${3:-2}"

  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/prompt\",\"id\":$id,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"prompt\":[{\"type\":\"text\",\"text\":\"$message\"}]}}"

  ACP_RESPONSE=""
  ACP_RAW=""
  ACP_TOOL_CALLS=0
  ACP_STOP_REASON=""
  ACP_ERROR=""

  while IFS= read -r -t "$timeout" line <&4; do
    [ -z "$line" ] && continue
    ACP_RAW="${ACP_RAW}${line}"$'\n'

    if [ "$VERBOSE" = true ] && [ "$JSON" != true ]; then
      echo -e "    ${DIM}<<< $(echo "$line" | head -c 120)${RESET}" >&2
    fi

    local text
    text=$(echo "$line" | python3 -c "
import sys,json
try:
  d=json.load(sys.stdin)
  u=d.get('params',{}).get('update',{})
  if u.get('sessionUpdate')=='agent_message_chunk':
    print(u.get('content',{}).get('text',''), end='')
  elif u.get('sessionUpdate')=='tool_call' and u.get('status')=='in_progress':
    print('__TOOL__', end='')
except: pass
" 2>/dev/null) || true

    if [ "$text" = "__TOOL__" ]; then
      ACP_TOOL_CALLS=$((ACP_TOOL_CALLS + 1))
    elif [ -n "$text" ]; then
      ACP_RESPONSE="${ACP_RESPONSE}${text}"
    fi

    if echo "$line" | grep -q "\"id\":$id"; then
      ACP_STOP_REASON=$(echo "$line" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('result',{}).get('stopReason',''))" 2>/dev/null) || true
      ACP_ERROR=$(echo "$line" | python3 -c "import sys,json; d=json.load(sys.stdin); e=d.get('error',{}); print(e.get('message',''))" 2>/dev/null) || true
      break
    fi
  done
}

# Extract a field from the session/new result (uses ACP_NEW_RESULT)
# Usage: acp_config_value "provider"  → prints the currentValue for the "provider" config option
acp_config_value() {
  local option_id="$1"
  echo "$ACP_NEW_RESULT" | python3 -c "
import sys,json
d=json.load(sys.stdin)
for o in d.get('result',{}).get('configOptions',[]):
  if o.get('id')=='$option_id':
    print(o.get('currentValue',''))
    break
" 2>/dev/null || true
}

# Extract option values list from session/new result
# Usage: acp_config_options "model"  → prints comma-separated option values
acp_config_options() {
  local option_id="$1"
  echo "$ACP_NEW_RESULT" | python3 -c "
import sys,json
d=json.load(sys.stdin)
for o in d.get('result',{}).get('configOptions',[]):
  if o.get('id')=='$option_id':
    vals=[v.get('value','') for v in o.get('options',[])]
    print(','.join(vals))
    break
" 2>/dev/null || true
}

# Count config options returned
acp_config_option_count() {
  echo "$ACP_NEW_RESULT" | python3 -c "
import sys,json
d=json.load(sys.stdin)
print(len(d.get('result',{}).get('configOptions',[])))
" 2>/dev/null || echo "0"
}

# --- Prerequisite Checks -----------------------------------------------------

check_prereqs() {
  local ok=true

  if [ ! -x "$ROKO_BIN" ] && ! command -v roko &>/dev/null; then
    if [ "$JSON" = true ]; then
      echo '{"error":"roko binary not found","path":"'"$ROKO_BIN"'"}'
    else
      echo -e "  ${FAIL_MARK} roko binary not found at $ROKO_BIN"
      echo -e "    ${DIM}Build with: cd /Users/will/dev/nunchi/roko/roko && cargo build --release -p roko-cli${RESET}"
    fi
    ok=false
  fi

  if [ ! -f "$ROKO_CONFIG" ]; then
    if [ "$JSON" = true ]; then
      echo '{"error":"config not found","path":"'"$ROKO_CONFIG"'"}'
    else
      echo -e "  ${FAIL_MARK} config not found at $ROKO_CONFIG"
    fi
    ok=false
  fi

  if ! command -v python3 &>/dev/null; then
    if [ "$JSON" != true ]; then
      echo -e "  ${FAIL_MARK} python3 not found (needed for JSON parsing)"
    fi
    ok=false
  fi

  if [ "$ok" = false ]; then
    if [ "$JSON" != true ]; then
      echo ""
      echo -e "  ${RED}Prerequisites not met. Fix the above and re-run.${RESET}"
    fi
    exit 1
  fi
}

# Check if bridge is running
check_bridge() {
  if [ ! -f "$BRIDGE_TOKEN_FILE" ]; then
    return 1
  fi
  local token
  token=$(cat "$BRIDGE_TOKEN_FILE")
  if curl -sf -H "Authorization: Bearer $token" "${BRIDGE_URL}/health" >/dev/null 2>&1; then
    return 0
  fi
  return 1
}

# --- Build Helper (optional, for agents that need to ensure binary exists) ---

# Ensure the roko binary is built. Agents can call this before running tests.
# Uses a lockfile so parallel agents don't race on cargo build.
ensure_built() {
  local lockfile="/tmp/roko-build.lock"
  local marker="/tmp/roko-build-done-$(git -C /Users/will/dev/nunchi/roko/roko rev-parse HEAD 2>/dev/null || echo unknown)"

  # If already built at this commit, skip
  if [ -f "$marker" ]; then
    return 0
  fi

  # Acquire lock (flock is atomic — safe for parallel agents)
  (
    flock -x 200
    # Double-check after acquiring lock
    if [ -f "$marker" ]; then
      return 0
    fi
    if [ "$JSON" != true ]; then
      echo -e "  ${DIM}Building roko (this blocks parallel agents until done)...${RESET}"
    fi
    cd /Users/will/dev/nunchi/roko/roko
    if cargo build --release -p roko-cli 2>"$LOG_DIR/build-stderr.log"; then
      touch "$marker"
      if [ "$JSON" != true ]; then
        echo -e "  ${PASS_MARK} Build succeeded"
      fi
    else
      if [ "$JSON" != true ]; then
        echo -e "  ${FAIL_MARK} Build failed — see $LOG_DIR/build-stderr.log"
        tail -10 "$LOG_DIR/build-stderr.log"
      fi
      exit 1
    fi
  ) 200>"$lockfile"
}
