#!/usr/bin/env bash
# test-runner-e2e.sh — Full end-to-end test of run-migration.sh.
#
# Runs the REAL run-migration.sh entry point with a single synthetic "test-smoke"
# topic. Exercises:
#   - Argument parsing
#   - Preflight check
#   - Parallel topic launcher subshell (process_topic) backgrounded with &
#   - spawn_topic() array-based command building
#   - Real claude CLI invocation
#   - verify_topic() with relaxed thresholds
#   - Master index generation
#
# Cost: ~$0.40 per run. Runtime: ~60-90 seconds.
#
# This is the ONLY test that exercises the exact same code path as the real
# overnight run. Use it as the final sanity check before launching the full
# migration.

set -uo pipefail
IFS=$'\n\t'   # match run-migration.sh's IFS

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MIGRATION_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# ANSI colors
if [[ -t 1 && "${NO_COLOR:-}" == "" ]]; then
    G=$'\e[32m'; R=$'\e[31m'; Y=$'\e[33m'; B=$'\e[34m'; M=$'\e[35m'; D=$'\e[2m'; X=$'\e[0m'
else
    G='' R='' Y='' B='' M='' D='' X=''
fi

header() { printf '\n%s=== %s ===%s\n\n' "$M" "$1" "$X"; }
pass()   { printf '  %s[PASS]%s %s\n' "$G" "$X" "$1"; PASS=$((PASS + 1)); }
fail()   { printf '  %s[FAIL]%s %s\n' "$R" "$X" "$1"; FAIL=$((FAIL + 1)); }
info()   { printf '  %s[INFO]%s %s\n' "$B" "$X" "$1"; }

PASS=0
FAIL=0

# --- Pre-test cleanup ------------------------------------------------------

OUTPUT_DIR="/Users/will/dev/nunchi/roko/roko/docs/test-smoke"
rm -rf "$OUTPUT_DIR"

# Clean old test runs
rm -rf "$MIGRATION_ROOT"/logs/run-test-smoke-*

header "TEST-RUNNER-E2E: Full run-migration.sh pipeline"

info "This test runs the REAL ./run-migration.sh entry point."
info "It exercises: arg parsing → preflight → subshell → backgrounding → spawn_topic → verify_topic → master index"
info "Estimated cost: ~\$0.40"
info "Estimated time: 60-90 seconds"
echo

# --- Environment setup -----------------------------------------------------

# Enable test mode: common.sh will load only the single "test-smoke" topic.
export ROKO_MIGRATION_TEST_MODE=1

# Tight cost and time limits
export ROKO_MIGRATION_BUDGET_USD=2
export ROKO_MIGRATION_TIMEOUT=300
export ROKO_MIGRATION_PARALLEL=1

# Relaxed verification thresholds (the test-smoke output is intentionally small)
export MIN_INDEX_LINES=15
export MIN_SUBDOCS=3
export MIN_SUBDOC_LINES=30
export MIN_TOPIC_TOTAL_LINES=150

# --- Run the real runner ---------------------------------------------------

header "Launching ./run-migration.sh"

START=$(date +%s)
RUNNER_OUTPUT="$MIGRATION_ROOT/logs/test-runner-output.txt"
RC=0
"$MIGRATION_ROOT/run-migration.sh" > "$RUNNER_OUTPUT" 2>&1 || RC=$?
END=$(date +%s)
DURATION=$((END - START))

info "Runner exit code: $RC"
info "Runner wall time: ${DURATION}s"
echo

# --- Check runner exit code ------------------------------------------------

header "Runner exit code"
if [[ $RC -eq 0 ]]; then
    pass "run-migration.sh returned 0"
else
    fail "run-migration.sh returned $RC"
fi

# --- Check output structure ------------------------------------------------

header "Output structure"

if [[ -d "$OUTPUT_DIR" ]]; then
    pass "Output directory exists: $OUTPUT_DIR"
else
    fail "Output directory MISSING: $OUTPUT_DIR"
fi

if [[ -s "$OUTPUT_DIR/INDEX.md" ]]; then
    lines=$(wc -l < "$OUTPUT_DIR/INDEX.md" | tr -d ' ')
    pass "INDEX.md exists ($lines lines)"
else
    fail "INDEX.md missing or empty"
fi

if [[ -s "$OUTPUT_DIR/00-engram.md" ]]; then
    lines=$(wc -l < "$OUTPUT_DIR/00-engram.md" | tr -d ' ')
    pass "00-engram.md exists ($lines lines)"
else
    fail "00-engram.md missing or empty"
fi

if [[ -s "$OUTPUT_DIR/01-synapse-traits.md" ]]; then
    lines=$(wc -l < "$OUTPUT_DIR/01-synapse-traits.md" | tr -d ' ')
    pass "01-synapse-traits.md exists ($lines lines)"
else
    fail "01-synapse-traits.md missing or empty"
fi

if [[ -s "$OUTPUT_DIR/02-naming.md" ]]; then
    lines=$(wc -l < "$OUTPUT_DIR/02-naming.md" | tr -d ' ')
    pass "02-naming.md exists ($lines lines)"
else
    fail "02-naming.md missing or empty"
fi

# --- Check content signatures ----------------------------------------------

header "Content signatures (proves agent read sources)"

CONTENT_ALL=""
for f in "$OUTPUT_DIR"/*.md; do
    [[ -f "$f" ]] && CONTENT_ALL="$CONTENT_ALL$(cat "$f")"
done

check_contains() {
    local term="$1"
    local label="$2"
    if echo "$CONTENT_ALL" | grep -q -- "$term"; then
        pass "Found: $label"
    else
        fail "Missing: $label"
    fi
}

check_not_contains() {
    local term="$1"
    local label="$2"
    if echo "$CONTENT_ALL" | grep -qE -- "$term"; then
        fail "Forbidden term present: $label"
    else
        pass "Forbidden term absent: $label"
    fi
}

check_contains "Roko" "word 'Roko'"
check_contains "Engram" "word 'Engram'"
check_contains "Synapse" "word 'Synapse'"
check_contains "confidence" "Score axis 'confidence'"
check_contains "novelty" "Score axis 'novelty'"
check_contains "salience" "Score axis 'salience' (extended axis)"
check_contains "coherence" "Score axis 'coherence' (extended axis)"
check_contains "KORAI" "token name 'KORAI'"
check_contains "DAEJI" "token name 'DAEJI'"

# NOTE: we intentionally do NOT grep for forbidden terms here (Thanatopsis,
# Necrocracy, GNOS token, etc.) — verify_topic() already performs the full
# forbidden-term check with smart quote/rename-context handling, and that
# check is reported separately as "Topic result: success/verify_failed"
# below. A naïve grep would flag legitimate rename-context uses of soft
# forbidden terms (e.g., the agent writing `| GNOS token | KORAI token |`
# in a rename map). Trust verify_topic for this.

# --- Check log files -------------------------------------------------------

header "Log files"

LOG_DIR=$(ls -dt "$MIGRATION_ROOT"/logs/run-* 2>/dev/null | head -1)
if [[ -n "$LOG_DIR" && -d "$LOG_DIR" ]]; then
    pass "Log directory created: $LOG_DIR"

    if [[ -s "$LOG_DIR/test-smoke.log" ]]; then
        log_size=$(wc -c < "$LOG_DIR/test-smoke.log" | tr -d ' ')
        pass "test-smoke.log exists ($log_size bytes)"

        # Check the log contains the command line (proves array expansion worked)
        if grep -q "Command:" "$LOG_DIR/test-smoke.log"; then
            pass "Log contains command trace (array expansion worked)"
        else
            fail "Log missing command trace"
        fi

        # Check there's no "command not found" error
        if grep -qi "command not found" "$LOG_DIR/test-smoke.log"; then
            fail "Log contains 'command not found' — IFS/array bug regression!"
        else
            pass "No 'command not found' errors in log"
        fi
    else
        fail "test-smoke.log missing or empty"
    fi

    if [[ -f "$LOG_DIR/test-smoke.result" ]]; then
        result=$(cat "$LOG_DIR/test-smoke.result")
        case "$result" in
            success|success_warnings)
                pass "Topic result: $result"
                ;;
            *)
                fail "Topic result: $result (expected success/success_warnings)"
                ;;
        esac
    else
        fail "test-smoke.result missing"
    fi
else
    fail "No log directory created"
fi

# --- Check master index ----------------------------------------------------

header "Master index"

MASTER_INDEX="/Users/will/dev/nunchi/roko/roko/docs/INDEX.md"
if [[ -s "$MASTER_INDEX" ]]; then
    pass "Master INDEX.md created at $MASTER_INDEX"
    if grep -q "test-smoke" "$MASTER_INDEX"; then
        pass "Master INDEX references test-smoke topic"
    else
        fail "Master INDEX does not reference test-smoke"
    fi
else
    fail "Master INDEX.md missing"
fi

# --- Summary ---------------------------------------------------------------

header "SUMMARY"
printf '  %sPASS:%s %d  %sFAIL:%s %d\n\n' "$G" "$X" "$PASS" "$R" "$X" "$FAIL"

if (( FAIL > 0 )); then
    echo "Runner output (tail):"
    tail -40 "$RUNNER_OUTPUT" 2>/dev/null | sed 's/^/  /'
    echo
    echo "Topic log (tail):"
    [[ -n "$LOG_DIR" && -f "$LOG_DIR/test-smoke.log" ]] && \
        tail -40 "$LOG_DIR/test-smoke.log" 2>/dev/null | sed 's/^/  /'
    exit 1
fi

echo "${G}Full end-to-end pipeline verified.${X}"
echo
echo "Cleanup: to remove test-smoke output, run:"
echo "  rm -rf /Users/will/dev/nunchi/roko/roko/docs/test-smoke"
echo "  rm -f /Users/will/dev/nunchi/roko/roko/docs/INDEX.md"
exit 0
