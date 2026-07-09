#!/usr/bin/env bash
#
# Run demo implementation tasks via Claude Code agents.
#
# Usage:
#   ./run-tasks.sh                       # Run all tasks in dependency order
#   ./run-tasks.sh T1.1 T1.3            # Run specific tasks
#   ./run-tasks.sh --batch 1             # Run batch 1
#   ./run-tasks.sh --dry-run             # Print what would run
#   ./run-tasks.sh --verify T1.3         # Run verification only (no impl)
#   ./run-tasks.sh --worktree T1.1       # Run in isolated git worktree
#
# Prerequisites:
#   - claude CLI installed and authenticated
#   - Working directory: /Users/will/dev/nunchi/roko/roko
#   - mirage-rs available for integration tests (Tier 2+)

set -euo pipefail

TASK_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="/Users/will/dev/nunchi/roko/roko"
LOG_DIR="$TASK_DIR/logs"
ERRATA_FILE="$TASK_DIR/ERRATA.md"
PROMPT_DIR="$TASK_DIR/.prompts"
mkdir -p "$LOG_DIR" "$PROMPT_DIR"

# ── Task registry ──────────────────────────────────────────────
declare -A TASKS=(
  [T1.1]="T1.1-real-llm-providers.md"
  [T1.2]="T1.2-yield-routing-skeleton.md"
  [T1.3]="T1.3-fee-distributor-contract.md"
  [T1.4]="T1.4-event-stream-infrastructure.md"
  [T2.1]="T2.1-full-yield-routing-with-llm.md"
  [T2.2]="T2.2-knowledge-loop-integration.md"
  [T2.3]="T2.3-fee-distribution-wiring.md"
  [T2.4]="T2.4-cfactor-benchmark.md"
  [T2.5]="T2.5-insightboard-enhancements.md"
  [T3.1]="T3.1-tui-demo-mode.md"
  [T3.2]="T3.2-multi-model-labeling.md"
  [T3.3]="T3.3-multi-round-tournament.md"
  [T3.4]="T3.4-knowledge-graph-json.md"
  [T3.5]="T3.5-reputation-persistence.md"
  [T3.6]="T3.6-one-click-agent-registration.md"
  [T3.7]="T3.7-autonomous-agent-loop.md"
  [T3.8]="T3.8-adversarial-agent-slashing.md"
)

# ── Execution order (strictly sequential within each batch) ────
# Batch 1: Foundation — safe to parallelize T1.3 and T2.5 only.
# T1.1 and T1.4 both touch main.rs → run sequentially.
BATCH_1=(T2.5 T1.3 T1.1 T1.4 T1.2)
BATCH_2=(T2.1)
BATCH_3=(T2.2 T2.3)
BATCH_4=(T2.4)
BATCH_5=(T3.1 T3.2 T3.3 T3.4 T3.5 T3.6 T3.7 T3.8)

# ── CLI parsing ────────────────────────────────────────────────
DRY_RUN=false
VERIFY_ONLY=false
USE_WORKTREE=false
SPECIFIC_TASKS=()
BATCH_NUM=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)    DRY_RUN=true; shift ;;
    --verify)     VERIFY_ONLY=true; shift ;;
    --worktree)   USE_WORKTREE=true; shift ;;
    --batch)      BATCH_NUM="$2"; shift 2 ;;
    T*)           SPECIFIC_TASKS+=("$1"); shift ;;
    -h|--help)
      sed -n '2,13p' "$0" | sed 's/^#//'
      exit 0
      ;;
    *)            echo "Unknown arg: $1"; exit 1 ;;
  esac
done

# ── Prompt assembly ────────────────────────────────────────────
# Writes the combined prompt (errata + task) to a temp file so we
# don't hit shell argument-length limits on long specs.
assemble_prompt() {
  local task_id="$1"
  local task_file="${TASKS[$task_id]}"
  local task_path="$TASK_DIR/$task_file"
  local prompt_file="$PROMPT_DIR/${task_id}.prompt.md"

  cat > "$prompt_file" <<HEADER
You are implementing a specific task for the roko-demo crate.
Working directory: $REPO_ROOT

IMPORTANT: Read the errata section below FIRST — it corrects known errors
in the task spec. Where the errata and the task conflict, the errata wins.

---
# ERRATA (corrections to all task specs)
HEADER

  cat "$ERRATA_FILE" >> "$prompt_file"

  cat >> "$prompt_file" <<SEPARATOR

---
# TASK SPEC
SEPARATOR

  cat "$task_path" >> "$prompt_file"

  cat >> "$prompt_file" <<FOOTER

---
# FINAL INSTRUCTIONS
- Read all files listed in "Files to read first" BEFORE making any changes.
- Follow the errata corrections above for contract APIs and Rust syntax.
- Run verification commands at the end to confirm your work.
- If something doesn't compile, read the actual source files to understand
  the real API rather than trusting the spec blindly.
FOOTER

  echo "$prompt_file"
}

# ── Task execution ─────────────────────────────────────────────
run_task() {
  local task_id="$1"
  local task_file="${TASKS[$task_id]}"
  local log_file="$LOG_DIR/${task_id}-$(date +%Y%m%d-%H%M%S).log"

  if [[ ! -f "$TASK_DIR/$task_file" ]]; then
    echo "ERROR: Task file not found: $TASK_DIR/$task_file"
    return 1
  fi

  echo ""
  echo "┌──────────────────────────────────────────────┐"
  echo "│ $task_id: ${task_file%.md}                    "
  echo "└──────────────────────────────────────────────┘"

  if $DRY_RUN; then
    echo "  [dry-run] Would assemble prompt and run claude"
    echo "  Task: $TASK_DIR/$task_file ($(wc -l < "$TASK_DIR/$task_file") lines)"
    echo "  Errata: $ERRATA_FILE ($(wc -l < "$ERRATA_FILE") lines)"
    return 0
  fi

  # Assemble the combined prompt
  local prompt_file
  prompt_file=$(assemble_prompt "$task_id")
  echo "  Prompt assembled: $prompt_file ($(wc -l < "$prompt_file") lines)"

  # Read prompt from file to avoid shell arg length limits
  local prompt
  prompt=$(cat "$prompt_file")

  cd "$REPO_ROOT"

  # Run claude with the assembled prompt
  echo "  Running claude..."
  if claude -p "$prompt" \
      --allowedTools "Edit,Write,Read,Glob,Grep,Bash" \
      2>&1 | tee "$log_file"; then
    echo "  ✓ $task_id completed (log: $log_file)"
  else
    echo "  ✗ $task_id FAILED (log: $log_file)"
    return 1
  fi

  # Post-task compilation check
  echo "  Running post-task build check..."
  if ! cargo build -p roko-demo 2>>"$log_file"; then
    echo "  ⚠ WARNING: cargo build failed after $task_id"
    echo "  Check log: $log_file"
    # Don't fail — let the user decide whether to continue
  fi
}

run_batch() {
  local batch_name="$1"
  shift
  local tasks=("$@")

  echo ""
  echo "═══════════════════════════════════════════════════"
  echo "  $batch_name"
  echo "  Tasks: ${tasks[*]}"
  echo "═══════════════════════════════════════════════════"

  for task in "${tasks[@]}"; do
    run_task "$task" || {
      echo ""
      echo "FATAL: $task failed. Stopping batch."
      echo "Fix the issue, then resume with: $0 ${tasks[*]:1}"
      return 1
    }
  done

  echo ""
  echo "  ✓ $batch_name complete"
}

# ── Main ───────────────────────────────────────────────────────
echo "roko-demo task runner"
echo "Repo:   $REPO_ROOT"
echo "Tasks:  $TASK_DIR"
echo "Logs:   $LOG_DIR"
echo ""

if [[ ${#SPECIFIC_TASKS[@]} -gt 0 ]]; then
  for task in "${SPECIFIC_TASKS[@]}"; do
    run_task "$task"
  done
elif [[ -n "$BATCH_NUM" ]]; then
  case "$BATCH_NUM" in
    1) run_batch "Batch 1: Foundation (T2.5, T1.3, T1.1, T1.4, T1.2)" "${BATCH_1[@]}" ;;
    2) run_batch "Batch 2: Wire events + LLM into scenario (T2.1)" "${BATCH_2[@]}" ;;
    3) run_batch "Batch 3: Knowledge + fees (T2.2, T2.3)" "${BATCH_3[@]}" ;;
    4) run_batch "Batch 4: C-Factor benchmark (T2.4)" "${BATCH_4[@]}" ;;
    5) run_batch "Batch 5: Polish + stretch (T3.x)" "${BATCH_5[@]}" ;;
    *) echo "Unknown batch: $BATCH_NUM (1-5)"; exit 1 ;;
  esac
else
  run_batch "Batch 1: Foundation" "${BATCH_1[@]}"
  run_batch "Batch 2: Wire events + LLM" "${BATCH_2[@]}"
  run_batch "Batch 3: Knowledge + fees" "${BATCH_3[@]}"
  run_batch "Batch 4: C-Factor benchmark" "${BATCH_4[@]}"
  run_batch "Batch 5: Polish + stretch" "${BATCH_5[@]}"
  echo ""
  echo "═══════════════════════════════════════════════════"
  echo "  All 17 tasks complete!"
  echo "═══════════════════════════════════════════════════"
fi
