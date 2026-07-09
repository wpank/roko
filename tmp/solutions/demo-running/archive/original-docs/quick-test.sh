#!/usr/bin/env bash
# Quick iteration test script for the demo pipeline.
# Uses debug binary, short timeouts, minimal overhead.
#
# Usage:
#   ./tmp/solutions/demo-running/quick-test.sh          # run all steps
#   ./tmp/solutions/demo-running/quick-test.sh plan-run  # just plan run (reuses existing workspace)
#   ./tmp/solutions/demo-running/quick-test.sh prd       # full prd pipeline via dev.sh
#
# Environment:
#   ROKO_BIN=path/to/roko   Override binary path
#   MODEL=gpt54-mini        Override model (default: gpt54-mini)
#   WS=/tmp/my-workspace    Reuse a specific workspace
#   RUST_LOG=info            Set log level for tracing output

set -euo pipefail
cd "$(dirname "$0")/../../.."  # project root

ROKO="${ROKO_BIN:-./target/debug/roko}"
MODEL="${MODEL:-gpt54-mini}"
BOLD=$'\033[1m'
DIM=$'\033[2m'
GREEN=$'\033[32m'
RED=$'\033[31m'
RESET=$'\033[0m'

timer_start() { STEP_START=$SECONDS; }
timer_show() {
  local elapsed=$(( SECONDS - STEP_START ))
  echo "  ${DIM}⏱ ${elapsed}s${RESET}"
}

step() {
  local label="$1"; shift
  echo ""
  echo "${BOLD}▶ $label${RESET}"
  echo "${DIM}\$ $*${RESET}"
  timer_start
  if "$@"; then
    echo "${GREEN}  ✓ $label${RESET}"
  else
    echo "${RED}  ✗ $label (exit $?)${RESET}"
    timer_show
    return 1
  fi
  timer_show
}

setup_workspace() {
  if [ -n "${WS:-}" ] && [ -d "$WS/.roko" ]; then
    echo "${DIM}Reusing workspace: $WS${RESET}"
    return 0
  fi
  WS=$(mktemp -d /tmp/roko-quick-XXXX)
  echo "${DIM}New workspace: $WS${RESET}"

  step "init" $ROKO --repo "$WS" init
  cp roko.toml "$WS/roko.toml"

  # Git init (needed for gates)
  (cd "$WS" && git init -q && git config user.email "test@test.local" && git config user.name "Test" && git add -A && git commit -q -m "init" --allow-empty)
  echo "${DIM}Workspace ready${RESET}"
}

cmd_full() {
  setup_workspace
  step "prd idea"     $ROKO --repo "$WS" --model "$MODEL" prd idea "Simple hello world CLI"
  step "prd draft"    $ROKO --repo "$WS" --model "$MODEL" prd draft new hello-cli
  step "prd promote"  $ROKO --repo "$WS" --model "$MODEL" prd draft promote hello-cli
  step "prd plan"     $ROKO --repo "$WS" --model "$MODEL" prd plan hello-cli
  step "plan validate" $ROKO --repo "$WS" --model "$MODEL" plan validate .roko/plans
  step "plan run"     $ROKO --repo "$WS" --model "$MODEL" plan run .roko/plans --max-retries 1
  step "status"       $ROKO --repo "$WS" --model "$MODEL" status
  echo ""
  echo "${BOLD}Workspace: $WS${RESET}"
}

cmd_plan_run() {
  if [ -z "${WS:-}" ]; then
    echo "WS not set. Run full test first or set WS=/path/to/workspace"
    exit 1
  fi
  step "plan run" $ROKO --repo "$WS" --model "$MODEL" plan run .roko/plans --max-retries 1 --fresh
  step "status"   $ROKO --repo "$WS" --model "$MODEL" status
}

cmd_prd() {
  # Use dev.sh pipeline
  exec ./dev.sh pipeline prd --model "$MODEL"
}

cmd_build() {
  step "cargo build" cargo build -p roko-cli
}

# Main
TOTAL_START=$SECONDS
case "${1:-full}" in
  full)      cmd_full ;;
  plan-run)  cmd_plan_run ;;
  prd)       cmd_prd ;;
  build)     cmd_build ;;
  *)
    echo "Usage: $0 [full|plan-run|prd|build]"
    exit 1
    ;;
esac

echo ""
echo "${BOLD}Total: $(( SECONDS - TOTAL_START ))s${RESET}"
