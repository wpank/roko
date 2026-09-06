#!/usr/bin/env bash
# coverage.sh — P2-03: Test coverage measurement
#
# Runs cargo-tarpaulin with the workspace tarpaulin config, prints a summary,
# and checks that coverage meets the configured threshold.
#
# Exit codes:
#   0 — coverage meets or exceeds threshold
#   1 — coverage below threshold
#   2 — tool or config missing

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONFIG_FILE="${WORKSPACE_ROOT}/.config/tarpaulin.toml"
COVERAGE_DIR="${WORKSPACE_ROOT}/.roko/coverage"

# ── Dependency check ──────────────────────────────────────────────────────────
if ! command -v cargo &>/dev/null; then
    echo "ERROR: 'cargo' is not in PATH." >&2
    exit 2
fi

if ! cargo tarpaulin --version &>/dev/null 2>&1; then
    echo "ERROR: cargo-tarpaulin is not installed." >&2
    echo "" >&2
    echo "Install it with:" >&2
    echo "  cargo install cargo-tarpaulin" >&2
    echo "" >&2
    echo "Or, if you have a prebuilt binary:" >&2
    echo "  cargo binstall cargo-tarpaulin" >&2
    exit 2
fi

# ── Config check ──────────────────────────────────────────────────────────────
if [[ ! -f "${CONFIG_FILE}" ]]; then
    echo "ERROR: tarpaulin config not found at: ${CONFIG_FILE}" >&2
    exit 2
fi

# ── Ensure output directory exists ────────────────────────────────────────────
mkdir -p "${COVERAGE_DIR}"

echo "========================================="
echo "  Roko Test Coverage"
echo "========================================="
echo "  Config: ${CONFIG_FILE}"
echo "  Output: ${COVERAGE_DIR}"
echo "========================================="
echo ""

cd "${WORKSPACE_ROOT}"

# Run tarpaulin; capture output for summary parsing
tarpaulin_output=$(cargo tarpaulin \
    --config "${CONFIG_FILE}" \
    --output-dir "${COVERAGE_DIR}" \
    2>&1) || tarpaulin_exit=$?

# Print the output so CI logs capture it
echo "${tarpaulin_output}"
echo ""

# ── Parse coverage percentage from tarpaulin output ──────────────────────────
# Tarpaulin prints a line like:
#   XX.XX% coverage, Y/Z lines covered
coverage_pct=$(echo "${tarpaulin_output}" | \
    grep -oE '[0-9]+\.[0-9]+% coverage' | \
    tail -1 | \
    grep -oE '[0-9]+\.[0-9]+' || true)

# ── Read threshold from config ────────────────────────────────────────────────
threshold=$(grep -E '^failure-threshold\s*=' "${CONFIG_FILE}" 2>/dev/null | \
    grep -oE '[0-9]+' | head -1 || echo "40")

echo "========================================="
echo "  Coverage Summary"
echo "========================================="

if [[ -z "${coverage_pct}" ]]; then
    echo "  Coverage: unable to parse from tarpaulin output"
    echo "  Check HTML report at: ${COVERAGE_DIR}/tarpaulin-report.html"
    echo ""
    # Propagate tarpaulin's own exit code if it failed
    if [[ -n "${tarpaulin_exit:-}" ]] && [[ "${tarpaulin_exit}" -ne 0 ]]; then
        echo "  Status: FAIL (tarpaulin exited with code ${tarpaulin_exit})"
        exit 1
    fi
    exit 0
fi

echo "  Coverage:  ${coverage_pct}%"
echo "  Threshold: ${threshold}%"
echo ""

# Compare using bc
meets=$(echo "${coverage_pct} >= ${threshold}" | bc -l)
if [[ "${meets}" -eq 1 ]]; then
    echo "  Status: PASS"
    echo ""
    echo "  HTML report: ${COVERAGE_DIR}/tarpaulin-report.html"
    exit 0
else
    echo "  Status: FAIL (${coverage_pct}% < ${threshold}% threshold)"
    echo ""
    echo "  HTML report: ${COVERAGE_DIR}/tarpaulin-report.html"
    exit 1
fi
