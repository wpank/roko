#!/usr/bin/env bash
# run_benchmark_evidence.sh -- Collect cold/warm benchmark evidence for the dev-audit.
#
# This is a DOCUMENTED TEMPLATE. It will exit before executing any real work
# unless you set BENCHMARK_EXECUTE=1. Read the instructions below, review the
# matrix preview, adjust parameters, then enable execution.
#
# Prerequisites:
#   1. Rust 1.91+ (rustup default stable && rustup update stable)
#   2. python3 on PATH
#   3. A clean working tree on the commit you want to benchmark
#   4. A pre-built debug binary from that exact commit:
#        cargo build -p roko-cli
#   5. Sufficient disk (at least 10 GiB free; cold samples create temporary targets)
#   6. Provider credentials configured (Anthropic/OpenAI) -- lanes use real LLMs
#   7. Network access for provider calls
#
# Usage:
#   # 1. Review the dry-run matrix (always safe, no execution):
#   ./scripts/run_benchmark_evidence.sh
#
#   # 2. When satisfied, run with execution enabled:
#   BENCHMARK_EXECUTE=1 ./scripts/run_benchmark_evidence.sh
#
#   # 3. After the session completes, refresh the history dashboard:
#   ./dev.sh benchmark history
#
#   # 4. Review results:
#   ls -la .roko/benchmarks/
#   cat .roko/benchmarks/HISTORY.md
#
# Customization:
#   BASE_SHA        -- commit to benchmark (default: HEAD)
#   ROKO_BIN        -- path to prebuilt binary (default: ./target/debug/roko)
#   REPETITIONS     -- samples per fixture/cache combination (default: 5)
#   MAX_COST_USD    -- provider spend ceiling (default: 50.0)
#   FIXTURES        -- space-separated fixture IDs to run (default: all 7)
#   LANES           -- space-separated lane IDs (default: current-roko roko-fast)
#   CACHES          -- space-separated cache strategies (default: cold warm)
#
set -euo pipefail
cd "$(dirname "$0")/.."

# ── Configuration (override via environment) ─────────────────
BASE_SHA="${BASE_SHA:-HEAD}"
ROKO_BIN="${ROKO_BIN:-./target/debug/roko}"
REPETITIONS="${REPETITIONS:-5}"
MAX_COST_USD="${MAX_COST_USD:-50.0}"
BENCHMARK_EXECUTE="${BENCHMARK_EXECUTE:-0}"

# Fixture IDs from benchmarks/dev-audit/manifest.json:
#   enum-config          one-line enum/string/config change
#   local-types          local type block with derives/imports
#   store-logic          pure store/matching logic
#   cli-output           CLI parser plus human/JSON output
#   http-endpoint        HTTP endpoint behavior
#   persistence-concurrency  persistence/concurrency invariant
#   tui-visual           TUI/web visual change
FIXTURES="${FIXTURES:-enum-config local-types store-logic cli-output http-endpoint persistence-concurrency tui-visual}"

# Lanes from benchmarks/dev-audit/manifest.json:
#   current-roko         Standard Roko plan run
#   roko-fast            FAST mode (skip preflight, focused gates, task-verify-only)
#   manual-codex         Manual Codex samples (import-only, not executable here)
#   manual-claude        Manual Claude samples (import-only, not executable here)
LANES="${LANES:-current-roko roko-fast}"

# Cache strategies:
#   cold    fresh CARGO_TARGET_DIR per sample (measures full incremental compile)
#   warm    shared stable target per lane (measures cached incremental compile)
CACHES="${CACHES:-cold warm}"

# ── Colors ───────────────────────────────────────────────────
if [ -t 1 ]; then
  BOLD=$'\033[1m' DIM=$'\033[2m' GREEN=$'\033[32m' YELLOW=$'\033[33m'
  BLUE=$'\033[34m' RED=$'\033[31m' RESET=$'\033[0m'
else
  BOLD="" DIM="" GREEN="" YELLOW="" BLUE="" RED="" RESET=""
fi

info()  { echo "${BLUE}[bench]${RESET} $*"; }
ok()    { echo "${GREEN}[bench]${RESET} $*"; }
warn()  { echo "${YELLOW}[bench]${RESET} $*"; }
err()   { echo "${RED}[bench]${RESET} $*" >&2; }
die()   { err "$@"; exit 1; }

# ── Preflight checks ────────────────────────────────────────
info "Benchmark evidence collection template"
echo ""

# Resolve BASE_SHA to a full hash
RESOLVED_SHA="$(git rev-parse "${BASE_SHA}")"
info "Base commit: ${RESOLVED_SHA} (from ${BASE_SHA})"

# Verify the binary exists
if [ ! -x "${ROKO_BIN}" ]; then
  warn "Binary not found at ${ROKO_BIN}"
  warn "Build it first:  cargo build -p roko-cli"
  warn "Continuing with dry-run preview..."
  echo ""
fi

# Verify python3
command -v python3 >/dev/null 2>&1 || die "python3 is required (used by scripts/dev_benchmark.py)"

# Verify the manifest exists
[ -f "benchmarks/dev-audit/manifest.json" ] || die "Manifest not found at benchmarks/dev-audit/manifest.json"

# ── Build the argument arrays ────────────────────────────────
LANE_ARGS=()
for lane in ${LANES}; do
  LANE_ARGS+=(--lane "${lane}")
done

FIXTURE_ARGS=()
for fixture in ${FIXTURES}; do
  FIXTURE_ARGS+=(--fixture "${fixture}")
done

CACHE_ARGS=()
for cache in ${CACHES}; do
  CACHE_ARGS+=(--cache "${cache}")
done

# ── Compute the matrix size ──────────────────────────────────
n_lanes=$(echo "${LANES}" | wc -w | tr -d ' ')
n_fixtures=$(echo "${FIXTURES}" | wc -w | tr -d ' ')
n_caches=$(echo "${CACHES}" | wc -w | tr -d ' ')
total_runs=$(( n_lanes * n_fixtures * n_caches * REPETITIONS ))

echo "${BOLD}Matrix:${RESET}"
echo "  Lanes:       ${LANES}"
echo "  Fixtures:    ${FIXTURES}"
echo "  Caches:      ${CACHES}"
echo "  Repetitions: ${REPETITIONS}"
echo "  Total runs:  ${total_runs} (${n_lanes} lanes x ${n_fixtures} fixtures x ${n_caches} caches x ${REPETITIONS} reps)"
echo "  Cost ceiling: \$${MAX_COST_USD} USD"
echo ""

# ── Step 1: Dry-run preview (always runs) ────────────────────
info "Step 1/3: Dry-run matrix preview"
echo ""

python3 scripts/dev_benchmark.py run \
  --base "${RESOLVED_SHA}" \
  --repetitions "${REPETITIONS}" \
  --max-cost-usd "${MAX_COST_USD}" \
  "${LANE_ARGS[@]}" \
  "${FIXTURE_ARGS[@]}" \
  "${CACHE_ARGS[@]}" \
  --dry-run

echo ""

# ── Guard: stop here unless execution is enabled ─────────────
if [ "${BENCHMARK_EXECUTE}" != "1" ]; then
  warn "Dry-run only. No benchmarks were executed."
  echo ""
  echo "To run the full matrix:"
  echo "  ${BOLD}BENCHMARK_EXECUTE=1 ./scripts/run_benchmark_evidence.sh${RESET}"
  echo ""
  echo "To run a single fixture first (recommended):"
  echo "  ${BOLD}FIXTURES=enum-config REPETITIONS=1 BENCHMARK_EXECUTE=1 ./scripts/run_benchmark_evidence.sh${RESET}"
  echo ""
  echo "To narrow to warm-only for faster iteration:"
  echo "  ${BOLD}CACHES=warm BENCHMARK_EXECUTE=1 ./scripts/run_benchmark_evidence.sh${RESET}"
  echo ""
  exit 0
fi

# ── Step 2: Execute the benchmark matrix ─────────────────────
info "Step 2/3: Executing benchmark matrix (${total_runs} runs)"
echo ""

# Verify the binary was built from the base commit
if [ ! -x "${ROKO_BIN}" ]; then
  die "Cannot execute: binary not found at ${ROKO_BIN}. Run: cargo build -p roko-cli"
fi

python3 scripts/dev_benchmark.py run \
  --base "${RESOLVED_SHA}" \
  --roko-bin "${ROKO_BIN}" \
  --binary-base "${RESOLVED_SHA}" \
  --repetitions "${REPETITIONS}" \
  --max-cost-usd "${MAX_COST_USD}" \
  --allow-network \
  "${LANE_ARGS[@]}" \
  "${FIXTURE_ARGS[@]}" \
  "${CACHE_ARGS[@]}"

BENCH_EXIT=$?

echo ""
if [ "${BENCH_EXIT}" -ne 0 ]; then
  err "Benchmark run exited with code ${BENCH_EXIT}"
  warn "Partial results may exist under .roko/benchmarks/"
  warn "Run:  python3 scripts/dev_benchmark.py summarize .roko/benchmarks/<session-id>"
fi

# ── Step 3: Refresh the history dashboard ────────────────────
info "Step 3/3: Refreshing historical dashboard and regression alerts"
echo ""

python3 scripts/dev_benchmark.py history
HISTORY_EXIT=$?

echo ""
if [ "${HISTORY_EXIT}" -ne 0 ]; then
  warn "History dashboard flagged regressions (exit ${HISTORY_EXIT})"
  warn "Review: .roko/benchmarks/HISTORY.md"
else
  ok "No regressions detected"
fi

# ── Summary ──────────────────────────────────────────────────
echo ""
echo "${BOLD}Results:${RESET}"
echo "  Session data:  .roko/benchmarks/<session-id>/"
echo "  Scorecard:     .roko/benchmarks/<session-id>/SCORECARD.md"
echo "  Raw data:      .roko/benchmarks/<session-id>/scorecard.json"
echo "  History:       .roko/benchmarks/HISTORY.md"
echo "  History JSON:  .roko/benchmarks/history.json"
echo ""
echo "Next steps:"
echo "  1. Review SCORECARD.md for p50/p95 latency per lane/fixture/cache"
echo "  2. Review HISTORY.md for regression alerts"
echo "  3. Import manual Codex/Claude baselines via --baseline flag"
echo "  4. Commit evidence bundles if results are promotion-worthy"
echo ""
ok "Benchmark evidence collection complete"
