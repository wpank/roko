#!/usr/bin/env bash
# check-metrics.sh — P1-05: Headline metrics CI check
#
# Reads .roko/learn/efficiency.jsonl and validates four headline metrics:
#   1. First-attempt pass rate > 60%
#   2. Iterations per plan < 2.0
#   3. Prompt tokens per spawn < 50K
#   4. Cost trend declining (last 50 tasks vs previous 50)
#
# Exit 0 if all metrics pass, exit 1 if any fail.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EFFICIENCY_FILE="${WORKSPACE_ROOT}/.roko/learn/efficiency.jsonl"

# ── Thresholds ────────────────────────────────────────────────────────────────
THRESHOLD_PASS_RATE=60        # percent
THRESHOLD_ITERATIONS=2.0      # per plan
THRESHOLD_PROMPT_TOKENS=50000 # per spawn
# cost trend: last 50 vs previous 50 — must be declining (last < prev)

# ── Dependency check ──────────────────────────────────────────────────────────
if ! command -v jq &>/dev/null; then
    echo "ERROR: 'jq' is required but not installed." >&2
    echo "" >&2
    echo "Install it with one of:" >&2
    echo "  macOS:   brew install jq" >&2
    echo "  Ubuntu:  apt-get install -y jq" >&2
    echo "  Fedora:  dnf install -y jq" >&2
    echo "  Alpine:  apk add jq" >&2
    exit 2
fi

if ! command -v bc &>/dev/null; then
    echo "ERROR: 'bc' is required but not installed." >&2
    exit 2
fi

# ── File check ────────────────────────────────────────────────────────────────
if [[ ! -f "${EFFICIENCY_FILE}" ]]; then
    echo "ERROR: efficiency.jsonl not found at: ${EFFICIENCY_FILE}" >&2
    echo "Run 'roko plan run' at least once to generate efficiency data." >&2
    exit 2
fi

line_count=$(wc -l < "${EFFICIENCY_FILE}" | tr -d ' ')
if [[ "${line_count}" -eq 0 ]]; then
    echo "ERROR: efficiency.jsonl is empty. No data to evaluate." >&2
    exit 2
fi

echo "========================================="
echo "  Roko Headline Metrics CI Check"
echo "========================================="
echo "  Source: ${EFFICIENCY_FILE}"
echo "  Records: ${line_count}"
echo "========================================="
echo ""

# ── Helper: compare floats with bc ────────────────────────────────────────────
float_lt() {
    # returns 0 (true) if $1 < $2
    local result
    result=$(echo "${1} < ${2}" | bc -l)
    [[ "${result}" -eq 1 ]]
}

float_gt() {
    # returns 0 (true) if $1 > $2
    local result
    result=$(echo "${1} > ${2}" | bc -l)
    [[ "${result}" -eq 1 ]]
}

# ── Metric 1: First-attempt pass rate > 60% ──────────────────────────────────
#
# A "first attempt" is any record where iteration == 1.
# "Passed" means gate_passed == true on that record.
# We look at the last gate_passed value for iteration==1 records per task_id
# (in case duplicates exist), then compute pass rate.
#
metric1_data=$(jq -r '
    select(.iteration == 1) |
    [.task_id, (.gate_passed | tostring)] | @tsv
' "${EFFICIENCY_FILE}" 2>/dev/null)

if [[ -z "${metric1_data}" ]]; then
    echo "[METRIC 1] First-attempt pass rate"
    echo "  Value:  N/A (no first-attempt records found)"
    echo "  Status: SKIP (insufficient data)"
    echo ""
    metric1_status="skip"
else
    # Count pass/fail — take final value per task_id to deduplicate
    metric1_counts=$(echo "${metric1_data}" | sort -k1,1 | awk '
        { last[$1] = $2 }
        END {
            pass = 0; fail = 0
            for (k in last) {
                if (last[k] == "true") pass++
                else fail++
            }
            print pass, fail
        }
    ')
    m1_pass=$(echo "${metric1_counts}" | awk '{print $1}')
    m1_fail=$(echo "${metric1_counts}" | awk '{print $2}')
    m1_total=$((m1_pass + m1_fail))

    if [[ "${m1_total}" -eq 0 ]]; then
        pass_rate="0"
        metric1_status="skip"
    else
        pass_rate=$(echo "scale=1; ${m1_pass} * 100 / ${m1_total}" | bc -l)
        if float_gt "${pass_rate}" "${THRESHOLD_PASS_RATE}"; then
            metric1_status="pass"
        else
            metric1_status="fail"
        fi
    fi

    status_icon="PASS"
    [[ "${metric1_status}" == "fail" ]] && status_icon="FAIL"
    [[ "${metric1_status}" == "skip" ]] && status_icon="SKIP"

    echo "[METRIC 1] First-attempt pass rate > ${THRESHOLD_PASS_RATE}%"
    echo "  Tasks evaluated: ${m1_total} (${m1_pass} passed, ${m1_fail} failed)"
    echo "  Value:  ${pass_rate}%"
    echo "  Status: ${status_icon}"
    echo ""
fi

# ── Metric 2: Iterations per plan < 2.0 ──────────────────────────────────────
#
# For each plan_id, take the maximum iteration value seen.
# Average those maxima across all plans.
#
metric2_data=$(jq -r '[.plan_id, .iteration] | @tsv' "${EFFICIENCY_FILE}" 2>/dev/null)

if [[ -z "${metric2_data}" ]]; then
    echo "[METRIC 2] Iterations per plan < ${THRESHOLD_ITERATIONS}"
    echo "  Value:  N/A"
    echo "  Status: SKIP (insufficient data)"
    echo ""
    metric2_status="skip"
else
    avg_iterations=$(echo "${metric2_data}" | awk '
        {
            plan = $1; iter = $2 + 0
            if (iter > max[plan]) max[plan] = iter
        }
        END {
            total = 0; count = 0
            for (p in max) { total += max[p]; count++ }
            if (count > 0) printf "%.2f\n", total / count
            else print "0"
        }
    ')

    if float_lt "${avg_iterations}" "${THRESHOLD_ITERATIONS}"; then
        metric2_status="pass"
        m2_icon="PASS"
    else
        metric2_status="fail"
        m2_icon="FAIL"
    fi

    echo "[METRIC 2] Iterations per plan < ${THRESHOLD_ITERATIONS}"
    echo "  Value:  ${avg_iterations}"
    echo "  Status: ${m2_icon}"
    echo ""
fi

# ── Metric 3: Prompt tokens per spawn < 50K ──────────────────────────────────
#
# Compute the mean of total_prompt_tokens across all records.
#
metric3_data=$(jq -r '.total_prompt_tokens // 0' "${EFFICIENCY_FILE}" 2>/dev/null)

if [[ -z "${metric3_data}" ]]; then
    echo "[METRIC 3] Prompt tokens per spawn < ${THRESHOLD_PROMPT_TOKENS}"
    echo "  Value:  N/A"
    echo "  Status: SKIP (insufficient data)"
    echo ""
    metric3_status="skip"
else
    avg_tokens=$(echo "${metric3_data}" | awk '
        { sum += $1; count++ }
        END { if (count > 0) printf "%.0f\n", sum / count; else print 0 }
    ')

    if float_lt "${avg_tokens}" "${THRESHOLD_PROMPT_TOKENS}"; then
        metric3_status="pass"
        m3_icon="PASS"
    else
        metric3_status="fail"
        m3_icon="FAIL"
    fi

    echo "[METRIC 3] Prompt tokens per spawn < ${THRESHOLD_PROMPT_TOKENS}"
    printf  "  Value:  %'.0f\n" "${avg_tokens}" 2>/dev/null || echo "  Value:  ${avg_tokens}"
    echo "  Status: ${m3_icon}"
    echo ""
fi

# ── Metric 4: Cost trend declining (last 50 vs previous 50) ──────────────────
#
# Read cost_usd values in file order (chronological).
# Compute average cost of the last 50 records vs the previous 50.
# Pass if last_avg < prev_avg (declining trend).
#
metric4_data=$(jq -r '.cost_usd // 0' "${EFFICIENCY_FILE}" 2>/dev/null)
m4_total=$(echo "${metric4_data}" | wc -l | tr -d ' ')

if [[ "${m4_total}" -lt 100 ]]; then
    echo "[METRIC 4] Cost trend declining (last 50 vs previous 50 tasks)"
    echo "  Records: ${m4_total} (need >= 100 for meaningful comparison)"
    echo "  Status: SKIP (insufficient data)"
    echo ""
    metric4_status="skip"
else
    # last 50: tail; previous 50: lines (N-99) through (N-50)
    prev_avg=$(echo "${metric4_data}" | tail -n 100 | head -n 50 | awk '
        { sum += $1; count++ }
        END { if (count > 0) printf "%.6f\n", sum / count; else print 0 }
    ')
    last_avg=$(echo "${metric4_data}" | tail -n 50 | awk '
        { sum += $1; count++ }
        END { if (count > 0) printf "%.6f\n", sum / count; else print 0 }
    ')

    if float_lt "${last_avg}" "${prev_avg}"; then
        metric4_status="pass"
        m4_icon="PASS"
        trend_arrow="↓ declining"
    else
        metric4_status="fail"
        m4_icon="FAIL"
        trend_arrow="↑ not declining"
    fi

    echo "[METRIC 4] Cost trend declining (last 50 vs previous 50 tasks)"
    echo "  Previous 50 avg cost: \$${prev_avg}"
    echo "  Last 50 avg cost:     \$${last_avg}"
    echo "  Trend:  ${trend_arrow}"
    echo "  Status: ${m4_icon}"
    echo ""
fi

# ── Summary ───────────────────────────────────────────────────────────────────
echo "========================================="
echo "  Summary"
echo "========================================="

all_pass=true
any_fail=false

statuses=("${metric1_status}" "${metric2_status}" "${metric3_status}" "${metric4_status}")
labels=(
    "First-attempt pass rate > ${THRESHOLD_PASS_RATE}%"
    "Iterations per plan < ${THRESHOLD_ITERATIONS}"
    "Prompt tokens per spawn < ${THRESHOLD_PROMPT_TOKENS}"
    "Cost trend declining"
)

for i in 0 1 2 3; do
    s="${statuses[$i]}"
    l="${labels[$i]}"
    case "${s}" in
        pass) echo "  [PASS] ${l}" ;;
        fail) echo "  [FAIL] ${l}"; any_fail=true; all_pass=false ;;
        skip) echo "  [SKIP] ${l}" ;;
    esac
done

echo ""
if [[ "${all_pass}" == "true" ]] && [[ "${any_fail}" == "false" ]]; then
    if [[ "${metric1_status}" == "skip" ]] || \
       [[ "${metric2_status}" == "skip" ]] || \
       [[ "${metric3_status}" == "skip" ]] || \
       [[ "${metric4_status}" == "skip" ]]; then
        echo "  Result: PASS with skips (some metrics had insufficient data)"
    else
        echo "  Result: ALL METRICS PASS"
    fi
    exit 0
else
    echo "  Result: FAILED — one or more metrics did not meet threshold"
    exit 1
fi
