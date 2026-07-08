#!/usr/bin/env bash
# run-all-tests.sh — comprehensive pre-run sanity check.
#
# Runs all tests in sequence:
#   1. Preflight check (no cost) — validates environment
#   2. Verify function self-test (no cost) — tests verify_topic against mock input
#   3. Live spawn integration test (~$0.40) — exercises spawn_topic directly
#   4. Full runner end-to-end test (~$0.40) — runs the REAL ./run-migration.sh
#      entry point with a synthetic test-smoke topic
#
# Use this before launching an overnight run to catch issues early.
# Total cost: ~$0.80. Total time: ~3-4 minutes.
#
# Options:
#   --no-live    Skip the live tests (tests 3 and 4)
#   -h, --help   Show help

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MIGRATION_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# shellcheck source=../lib/common.sh
source "$MIGRATION_ROOT/lib/common.sh"

SKIP_LIVE=0
while [[ $# -gt 0 ]]; do
    case "$1" in
        --no-live) SKIP_LIVE=1; shift ;;
        -h|--help)
            echo "Usage: $0 [--no-live]"
            echo "  --no-live   Skip the live spawn integration test (saves ~$0.50 and ~60s)"
            exit 0
            ;;
        *) log_err "cli" "Unknown arg: $1"; exit 1 ;;
    esac
done

OVERALL_RC=0

log_header "TEST 1: Preflight check"
if "$MIGRATION_ROOT/run-migration.sh" --dry-run --only 00-architecture >/dev/null 2>&1; then
    log_ok "test-1" "Preflight + dry-run passed"
else
    log_err "test-1" "Preflight or dry-run failed — see ./run-migration.sh --dry-run for details"
    OVERALL_RC=1
fi

log_header "TEST 2: Verify function self-test"
if "$SCRIPT_DIR/test-verify.sh" >/dev/null 2>&1; then
    log_ok "test-2" "verify_topic() self-test passed (8/8 assertions)"
else
    log_err "test-2" "verify_topic() self-test failed — run ./verify/test-verify.sh for details"
    OVERALL_RC=1
fi

if (( SKIP_LIVE == 0 )); then
    log_header "TEST 3: Live spawn integration test (~\$0.40, ~60s)"
    if "$SCRIPT_DIR/test-spawn-integration.sh" >/dev/null 2>&1; then
        log_ok "test-3" "Live spawn integration test passed (13/13 assertions)"
    else
        log_err "test-3" "Live spawn integration test failed — run ./verify/test-spawn-integration.sh for details"
        OVERALL_RC=1
    fi

    log_header "TEST 4: Full runner end-to-end test (~\$0.40, ~2 min)"
    if "$SCRIPT_DIR/test-runner-e2e.sh" >/dev/null 2>&1; then
        log_ok "test-4" "Full runner e2e test passed (25/25 assertions)"
        log_info "test-4" "This test ran the REAL ./run-migration.sh entry point."
    else
        log_err "test-4" "Full runner e2e test failed — run ./verify/test-runner-e2e.sh for details"
        OVERALL_RC=1
    fi
else
    log_info "test-3" "Skipping live spawn test (--no-live)"
    log_info "test-4" "Skipping full runner e2e test (--no-live)"
fi

echo
log_header "OVERALL RESULT"
if (( OVERALL_RC == 0 )); then
    log_ok "all-tests" "ALL CHECKS PASSED — ready for overnight run"
    echo
    printf '  Next step: %s./run-migration.sh%s (full run) or %s./run-migration.sh --only 00-architecture%s (single topic)\n' \
        "$C_CYAN" "$C_RESET" "$C_CYAN" "$C_RESET"
else
    log_err "all-tests" "ONE OR MORE CHECKS FAILED — do not start overnight run yet"
fi

exit $OVERALL_RC
