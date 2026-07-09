#!/usr/bin/env bash
# test-spawn-integration.sh — End-to-end integration test.
#
# Invokes spawn_topic() (from lib/spawn.sh) with a special "test-smoke" topic
# that points at a tiny smoke-test prompt. This runs the FULL runner pipeline:
# - claude CLI with all the flags (including env -u CLAUDECODE)
# - Prompt piped via stdin
# - Output directory created
# - Log file written
#
# Verifies that:
# - Exit code is 0
# - Log file is non-empty
# - Output file (smoke-test-result.md) is created at the expected path
# - The result file contains expected verbatim quotes from the source files
#
# Cost: ~$0.50 per run. Skip with `SKIP_LIVE_TEST=1`.

set -uo pipefail
# Match run-migration.sh's IFS. This was the root cause of a production bug
# where `$timeout_cmd` in spawn.sh did not word-split on spaces because
# IFS=$'\n\t' stripped space from the separator set. Arrays bypass this.
IFS=$'\n\t'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MIGRATION_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

if [[ "${SKIP_LIVE_TEST:-0}" == "1" ]]; then
    echo "SKIP_LIVE_TEST=1 — skipping live spawn test"
    exit 0
fi

# Override OUTPUT_ROOT for the test
export OUTPUT_ROOT="$MIGRATION_ROOT/logs/spawn-integration-output"
rm -rf "$OUTPUT_ROOT"
mkdir -p "$OUTPUT_ROOT"

# shellcheck source=../lib/common.sh
source "$MIGRATION_ROOT/lib/common.sh"

# Override the prompt file path lookup so we can point at our test prompt
topic_prompt_file() {
    echo "$MIGRATION_ROOT/logs/test-smoke.prompt.md"
}

# shellcheck source=../lib/spawn.sh
source "$MIGRATION_ROOT/lib/spawn.sh"
# Re-override after sourcing (spawn.sh sources common.sh which resets the function)
topic_prompt_file() {
    echo "$MIGRATION_ROOT/logs/test-smoke.prompt.md"
}

# We need to make sure the smoke-test prompt exists
if [[ ! -f "$MIGRATION_ROOT/logs/test-smoke.prompt.md" ]]; then
    echo "ERROR: smoke-test prompt not found at $MIGRATION_ROOT/logs/test-smoke.prompt.md"
    exit 1
fi

# The smoke-test prompt writes to logs/smoke-test-result.md, not OUTPUT_ROOT.
# So we need to remove any stale result file first.
rm -f "$MIGRATION_ROOT/logs/smoke-test-result.md"

log_header "END-TO-END SPAWN INTEGRATION TEST"

RUN_ID="spawn-test-$(date +%Y%m%d-%H%M%S)"
LOG_ROOT="$MIGRATION_ROOT/logs"
export LOG_ROOT

# Use a short timeout for safety (3 minutes)
export ROKO_MIGRATION_TIMEOUT=180
# Small budget cap
export ROKO_MIGRATION_BUDGET_USD=2

# Topic name is "test-smoke" (not a real migration topic)
TOPIC="test-smoke"

# Invoke spawn_topic directly
START=$(date +%s)
RC=0
spawn_topic "$TOPIC" "$RUN_ID" || RC=$?
END=$(date +%s)
DURATION=$((END - START))

echo
log_info "test" "spawn_topic returned $RC after ${DURATION}s"
echo

# --- Verify output ---

PASS=0
FAIL=0
pass_test() { printf '  %s[PASS]%s %s\n' "$C_GREEN" "$C_RESET" "$1"; PASS=$((PASS + 1)); }
fail_test() { printf '  %s[FAIL]%s %s\n' "$C_RED" "$C_RESET" "$1"; FAIL=$((FAIL + 1)); }

log_header "VERIFICATION"

# Check 1: exit code
if [[ $RC -eq 0 ]]; then
    pass_test "spawn_topic exited with 0"
else
    fail_test "spawn_topic exited with $RC"
fi

# Check 2: log file exists and is non-empty
LOG_FILE="$LOG_ROOT/$RUN_ID/$TOPIC.log"
if [[ -s "$LOG_FILE" ]]; then
    SIZE=$(stat -f '%z' "$LOG_FILE")
    pass_test "Log file exists: $SIZE bytes"
else
    fail_test "Log file missing or empty: $LOG_FILE"
fi

# Check 3: output file exists
RESULT_FILE="$MIGRATION_ROOT/logs/smoke-test-result.md"
if [[ -f "$RESULT_FILE" ]]; then
    SIZE=$(stat -f '%z' "$RESULT_FILE")
    pass_test "Result file exists: $SIZE bytes"
else
    fail_test "Result file missing: $RESULT_FILE"
fi

# Check 4: result file contains verbatim content from the source
if [[ -f "$RESULT_FILE" ]]; then
    if grep -q "Synapse Architecture" "$RESULT_FILE"; then
        pass_test "Result contains 'Synapse Architecture' (from 01-naming-map.md)"
    else
        fail_test "Result does NOT contain 'Synapse Architecture'"
    fi

    if grep -q "KORAI" "$RESULT_FILE"; then
        pass_test "Result contains 'KORAI' (from 01-naming-map.md)"
    else
        fail_test "Result does NOT contain 'KORAI'"
    fi

    if grep -q "DAEJI" "$RESULT_FILE"; then
        pass_test "Result contains 'DAEJI' (from 01-naming-map.md)"
    else
        fail_test "Result does NOT contain 'DAEJI'"
    fi

    if grep -q "Engram" "$RESULT_FILE"; then
        pass_test "Result contains 'Engram' (from 01-synapse-architecture.md)"
    else
        fail_test "Result does NOT contain 'Engram'"
    fi

    if grep -q "confidence" "$RESULT_FILE" && grep -q "novelty" "$RESULT_FILE" && grep -q "utility" "$RESULT_FILE"; then
        pass_test "Result lists Score axes (from 01-synapse-architecture.md)"
    else
        fail_test "Result missing Score axes"
    fi

    if grep -q "Meta-Harness" "$RESULT_FILE"; then
        pass_test "Result contains 'Meta-Harness' citation (from refactoring-prd/00-overview.md)"
    else
        fail_test "Result does NOT contain 'Meta-Harness' citation"
    fi

    if grep -qE 'Signal|signal' "$RESULT_FILE"; then
        pass_test "Result acknowledges current code uses 'Signal' (from roko-core/src/lib.rs)"
    else
        fail_test "Result does NOT reference 'Signal' in code"
    fi

    # Verify the agent did NOT use forbidden terms (except in quotes)
    if grep -q "Thanatopsis" "$RESULT_FILE"; then
        fail_test "Result contains forbidden 'Thanatopsis' term"
    else
        pass_test "Result does NOT contain 'Thanatopsis'"
    fi

    if grep -qiE '^\s*-\s*Clade.*fleet|fleet.*Clade' "$RESULT_FILE"; then
        fail_test "Result contains the forbidden 'Clade→fleet' mapping"
    else
        pass_test "Result does NOT contain 'Clade→fleet' mapping"
    fi
fi

# Check 5: log file mentions CLAUDECODE-unset command shape
if [[ -f "$LOG_FILE" ]]; then
    if grep -q "Budget cap" "$LOG_FILE"; then
        pass_test "Log file contains budget cap header"
    else
        fail_test "Log file missing budget cap header"
    fi
fi

echo
log_header "SUMMARY"
printf '  %sPASS:%s %d  %sFAIL:%s %d\n\n' "$C_GREEN" "$C_RESET" "$PASS" "$C_RED" "$C_RESET" "$FAIL"

if (( FAIL > 0 )); then
    echo "Log file tail (for debugging):"
    tail -40 "$LOG_FILE" 2>/dev/null | sed 's/^/  /'
    exit 1
fi
exit 0
