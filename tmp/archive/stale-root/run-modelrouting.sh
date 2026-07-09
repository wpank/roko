#!/usr/bin/env bash
# run-modelrouting.sh — Autonomous orchestration for model routing implementation
#
# Reads task specs from tmp/implementation-plans/modelrouting/*.md, extracts
# checklist items (### 2X.NN sections), and runs one agent per task with strict
# verification gates and proper state tracking.
#
# Key improvements over run-parity.sh:
#   - Verification gates BLOCK marking done (agent success ≠ done)
#   - Stale detection: if agent claims success but gate fails, retry includes
#     the failure context so the agent can fix its own mistake
#   - Rollback on gate failure: git stash the broken changes before retry
#   - Per-task timeout with SIGTERM (no infinite agent loops)
#   - JSON state file (not just .done/.failed files) with attempt history
#   - --resume replays from last incomplete task
#   - Codex 5.4 mini as first-class option
#
# Usage:
#   bash tmp/run-modelrouting.sh                                    # full run
#   bash tmp/run-modelrouting.sh --agent codex --model gpt-5.4-mini --reasoning high
#   bash tmp/run-modelrouting.sh --agent claude --model sonnet       # claude sonnet
#   bash tmp/run-modelrouting.sh --doc 02                           # single doc
#   bash tmp/run-modelrouting.sh --task 2A.03                       # single task
#   bash tmp/run-modelrouting.sh --phase 1                          # all Phase 1 docs
#   bash tmp/run-modelrouting.sh --list                             # show all tasks
#   bash tmp/run-modelrouting.sh --status                           # progress summary
#   bash tmp/run-modelrouting.sh --dry-run --doc 02                 # preview prompts
#   bash tmp/run-modelrouting.sh --reset                            # clear all state
#   bash tmp/run-modelrouting.sh --reset-task 2A.03                 # clear one task
#   bash tmp/run-modelrouting.sh --resume                           # pick up where left off
#   bash tmp/run-modelrouting.sh --commit                           # auto-commit per task

set -u
# NOTE: `set -e` and `pipefail` intentionally omitted — the script handles
# errors explicitly in run_task/run_gates/run_agent. With -e or pipefail,
# tee pipelines and gate failures kill the script before state is persisted.

# ─── Configuration ──────────────────────────────────────────────────────────

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PLANS_DIR="$REPO_ROOT/tmp/implementation-plans/modelrouting"
STATE_DIR="$REPO_ROOT/.roko/modelrouting-state"
STATE_FILE="$STATE_DIR/state.json"
LOG_DIR="$REPO_ROOT/tmp/logs/modelrouting"
MAX_RETRIES=3
TASK_TIMEOUT=900       # 15 min per task (SIGTERM after this)
GATE_TIMEOUT=120       # 2 min per gate step
MAX_BUDGET_USD="${MAX_BUDGET_USD:-3.00}"  # per-task cost cap (claude only)
PROGRESS_FILE="$LOG_DIR/progress.log"    # easy tail -f target

# macOS doesn't have `timeout` — use gtimeout if available, else a bash fallback
if command -v gtimeout &>/dev/null; then
  TIMEOUT_CMD="gtimeout"
elif command -v timeout &>/dev/null; then
  TIMEOUT_CMD="timeout"
else
  # Bash fallback: run command with background + wait + kill
  run_with_timeout() {
    local secs="$1"; shift
    "$@" &
    local pid=$!
    ( sleep "$secs" && kill -TERM "$pid" 2>/dev/null ) &
    local watchdog=$!
    wait "$pid" 2>/dev/null
    local rc=$?
    kill "$watchdog" 2>/dev/null; wait "$watchdog" 2>/dev/null
    return $rc
  }
  TIMEOUT_CMD="__fallback__"
fi

do_timeout() {
  local secs="$1"; shift
  if [[ "$TIMEOUT_CMD" == "__fallback__" ]]; then
    run_with_timeout "$secs" "$@"
  else
    "$TIMEOUT_CMD" "$secs" "$@"
  fi
}

# Agent config
AGENT_TYPE="${AGENT_TYPE:-claude}"
AGENT_MODEL=""
CODEX_REASONING=""
CLAUDE_CMD="${CLAUDE_CMD:-claude}"
CODEX_CMD="${CODEX_CMD:-codex}"

# Feature flags
AUTO_COMMIT=false
DRY_RUN=false
STOP_ON_FAILURE=false

# Filter flags
FILTER_DOC=""
FILTER_TASK=""
FILTER_PHASE=""
DO_LIST=false
DO_STATUS=false
DO_RESUME=false

# ─── Colors ─────────────────────────────────────────────────────────────────

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[0;33m'
BLUE='\033[0;34m'; MAGENTA='\033[0;35m'; CYAN='\033[0;36m'
BOLD='\033[1m'; DIM='\033[2m'; NC='\033[0m'

log_info()  { echo -e "${BLUE}[INFO]${NC}  $(date +%H:%M:%S) $*"; }
log_ok()    { echo -e "${GREEN}[OK]${NC}    $(date +%H:%M:%S) $*"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC}  $(date +%H:%M:%S) $*"; }
log_err()   { echo -e "${RED}[ERR]${NC}   $(date +%H:%M:%S) $*"; }
log_gate()  { echo -e "${MAGENTA}[GATE]${NC}  $(date +%H:%M:%S) $*"; }
log_step()  { echo -e "\n${BOLD}${CYAN}━━━ $* ━━━${NC}"; }
log_phase() { echo -e "\n${BOLD}${MAGENTA}╔══════════════════════════════════════════════════╗${NC}"; \
              echo -e "${BOLD}${MAGENTA}║  $*${NC}"; \
              echo -e "${BOLD}${MAGENTA}╚══════════════════════════════════════════════════╝${NC}\n"; }

# ─── Trap: reset "running" tasks on exit/crash ────────────────────────────

_CURRENT_TASK_ID=""
_cleanup_on_exit() {
  if [[ -n "$_CURRENT_TASK_ID" ]]; then
    log_warn "Script interrupted — resetting task $_CURRENT_TASK_ID from running"
    state_set "$_CURRENT_TASK_ID" "status=pending" 2>/dev/null || true
  fi
}
trap _cleanup_on_exit EXIT INT TERM

# ─── State management (JSON-based) ─────────────────────────────────────────

mkdir -p "$STATE_DIR" "$LOG_DIR"

# Initialize state file if it doesn't exist
if [[ ! -f "$STATE_FILE" ]]; then
  echo '{}' > "$STATE_FILE"
fi

# Read a task's state field. Returns empty string if not found.
state_get() {
  local task_id="$1" field="$2"
  python3 -c "
import json, sys
with open('$STATE_FILE') as f: s = json.load(f)
t = s.get('$task_id', {})
print(t.get('$field', ''))
" 2>/dev/null || echo ""
}

# Set a task's state field(s). Pass key=value pairs.
state_set() {
  local task_id="$1"; shift
  python3 -c "
import json, sys, datetime
with open('$STATE_FILE') as f: s = json.load(f)
t = s.setdefault('$task_id', {})
for arg in sys.argv[1:]:
    k, v = arg.split('=', 1)
    if v in ('true','false'): v = v == 'true'
    elif v.isdigit(): v = int(v)
    t[k] = v
t['updated_at'] = datetime.datetime.now(datetime.timezone.utc).isoformat()
with open('$STATE_FILE', 'w') as f: json.dump(s, f, indent=2)
" "$@"
}

# Append to a task's attempts array
state_add_attempt() {
  local task_id="$1" result="$2" gate_result="$3" duration="$4" error="${5:-}"
  python3 -c "
import json, datetime
with open('$STATE_FILE') as f: s = json.load(f)
t = s.setdefault('$task_id', {})
attempts = t.setdefault('attempts', [])
attempts.append({
    'timestamp': datetime.datetime.now(datetime.timezone.utc).isoformat(),
    'result': '$result',
    'gate_result': '$gate_result',
    'duration_s': int('$duration'),
    'error': '$error' if '$error' else None,
    'agent': '$AGENT_TYPE',
    'model': '${AGENT_MODEL:-default}',
})
with open('$STATE_FILE', 'w') as f: json.dump(s, f, indent=2)
"
}

is_done() { [[ "$(state_get "$1" status)" == "done" ]]; }
is_failed() { [[ "$(state_get "$1" status)" == "failed" ]]; }

_log_progress() {
  # Append a timestamped line to the progress file for easy monitoring
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" >> "$PROGRESS_FILE"
}

mark_done() {
  state_set "$1" "status=done"
  log_ok "$1 ✅ DONE (verified)"
  _log_progress "DONE  $1"
}

mark_failed() {
  local task_id="$1" reason="$2"
  state_set "$task_id" "status=failed" "failure_reason=$reason"
  log_err "$task_id ❌ FAILED: $reason"
  _log_progress "FAIL  $task_id — $reason"
}

mark_running() {
  state_set "$1" "status=running" "attempt=$2"
}

clear_task_state() {
  python3 -c "
import json
with open('$STATE_FILE') as f: s = json.load(f)
s.pop('$1', None)
with open('$STATE_FILE', 'w') as f: json.dump(s, f, indent=2)
"
}

# ─── Doc → Phase mapping ──────────────────────────────────────────────────

# Phase 1: foundation (sequential)
# Phase 2: model backends (parallel after Phase 1)
# Phase 3: learning & events (parallel, can start during Phase 2)
# Phase 4: advanced (after Phase 3)
PHASE_1_DOCS="02 03 04"
PHASE_2_DOCS="05 06 07 20 21"
PHASE_3_DOCS="08 09 10"
PHASE_4_DOCS="12 13 14 15 16 17 18"

doc_to_phase() {
  local doc="$1"
  case "$doc" in
    02|03|04) echo 1 ;;
    05|06|07|20|21) echo 2 ;;
    08|09|10) echo 3 ;;
    12|13|14|15|16|17|18) echo 4 ;;
    *) echo 0 ;;
  esac
}

# Doc dependencies (doc can't start until these docs complete)
doc_depends_on() {
  local doc="$1"
  case "$doc" in
    03) echo "02" ;;
    04) echo "03" ;;
    05|06) echo "02 03 04" ;;
    07) echo "02 03" ;;
    08) echo "" ;;               # can start in parallel
    09) echo "02" ;;
    10) echo "08" ;;
    12) echo "08 03" ;;
    13) echo "02 03 04" ;;
    14) echo "02 03 04" ;;
    15|16|17|18) echo "02 03" ;;
    20|21) echo "02 03" ;;
    *) echo "" ;;
  esac
}

doc_deps_satisfied() {
  local doc="$1"
  local deps
  deps=$(doc_depends_on "$doc")
  [[ -z "$deps" ]] && return 0
  for dep in $deps; do
    if ! is_done "doc-${dep}"; then return 1; fi
  done
  return 0
}

# ─── Task extraction from spec files ───────────────────────────────────────

# Extract all ### or #### 2X.NN task blocks from a doc file
# Returns: task_id|title (one per line)
# Handles both ### (docs 02-18) and #### (docs 20-21) heading levels
extract_tasks_from_doc() {
  local doc_num="$1"
  local doc_file
  doc_file=$(find "$PLANS_DIR" -name "${doc_num}-*.md" -type f | head -1)
  [[ -z "$doc_file" ]] && return

  grep -n '^###\+ 2[A-Z]\.[0-9]' "$doc_file" | while IFS= read -r line; do
    local task_id title
    task_id=$(echo "$line" | sed -E 's/^[0-9]+:#{3,4} (2[A-Z]\.[0-9]+).*/\1/')
    title=$(echo "$line" | sed -E 's/^[0-9]+:#{3,4} 2[A-Z]\.[0-9]+ — (.*)/\1/')
    echo "${task_id}|${title}"
  done
}

# Extract the full task spec block (from ### or #### to next ### or #### or ---)
extract_task_spec() {
  local doc_num="$1" task_id="$2"
  local doc_file
  doc_file=$(find "$PLANS_DIR" -name "${doc_num}-*.md" -type f | head -1)
  [[ -z "$doc_file" ]] && return

  awk -v tid="$task_id" '
    /^#{3,4} / && index($0, tid) > 0 { found=1; next }
    found && /^---$/ { exit }
    found && /^#{3,4} 2[A-Z]\.[0-9]/ { exit }
    found && /^## / { exit }
    found { print }
  ' "$doc_file"
}

# Extract the doc title
extract_doc_title() {
  local doc_num="$1"
  local doc_file
  doc_file=$(find "$PLANS_DIR" -name "${doc_num}-*.md" -type f | head -1)
  [[ -z "$doc_file" ]] && return
  head -1 "$doc_file" | sed 's/^# //' | sed 's/^[0-9]* — //'
}

# Map task ID prefix to doc number
task_to_doc() {
  local task_id="$1"
  local letter="${task_id:1:1}"  # second char: A,B,C,...
  case "$letter" in
    A) echo "02" ;; B) echo "03" ;; C) echo "04" ;;
    D) echo "05" ;; E) echo "06" ;; F) echo "07" ;;
    G) echo "08" ;; H) echo "09" ;; I) echo "10" ;;
    J) echo "12" ;; K) echo "13" ;; L) echo "14" ;;
    M) echo "15" ;; N) echo "16" ;; O) echo "17" ;;
    P) echo "18" ;; Q) echo "20" ;; R) echo "21" ;;
    *) echo "" ;;
  esac
}

# Get all implementable docs (have tasks, not reference-only)
get_impl_docs() {
  echo "02 03 04 05 06 07 08 09 10 12 13 14 15 16 17 18 20 21"
}

# ─── Context files for each doc ────────────────────────────────────────────

context_files_for_doc() {
  local doc="$1"
  local files=("$REPO_ROOT/CLAUDE.md")

  # Always include the spec file itself
  local spec_file
  spec_file=$(find "$PLANS_DIR" -name "${doc}-*.md" -type f | head -1)
  [[ -n "$spec_file" ]] && files+=("$spec_file")

  # Architecture doc for reference
  files+=("$PLANS_DIR/01-architecture.md")

  # Doc-specific context
  case "$doc" in
    02) files+=(
      "$REPO_ROOT/crates/roko-core/src/agent.rs"
      "$REPO_ROOT/crates/roko-core/src/config/schema.rs"
      "$REPO_ROOT/roko.toml"
    ) ;;
    03) files+=(
      "$REPO_ROOT/crates/roko-agent/src/lib.rs"
      "$REPO_ROOT/crates/roko-agent/src/openai_agent.rs"
      "$REPO_ROOT/crates/roko-agent/src/codex_agent.rs"
      "$REPO_ROOT/crates/roko-agent/src/claude_cli_agent.rs"
      "$REPO_ROOT/crates/roko-agent/src/cursor_agent.rs"
      "$REPO_ROOT/crates/roko-agent/src/ollama_agent.rs"
      "$REPO_ROOT/crates/roko-cli/src/orchestrate.rs"
    ) ;;
    04) files+=(
      "$REPO_ROOT/crates/roko-agent/src/translate/mod.rs"
    ) ;;
    05|06) files+=(
      "$REPO_ROOT/crates/roko-agent/src/openai_agent.rs"
      "$REPO_ROOT/crates/roko-agent/src/codex_agent.rs"
      "$REPO_ROOT/crates/roko-agent/src/translate/mod.rs"
    ) ;;
    07) files+=(
      "$REPO_ROOT/crates/roko-agent/src/codex_agent.rs"
      "$REPO_ROOT/crates/roko-agent/src/http.rs"
    ) ;;
    08) files+=(
      "$REPO_ROOT/crates/roko-learn/src/cascade_router.rs"
      "$REPO_ROOT/crates/roko-learn/src/model_router.rs"
      "$REPO_ROOT/crates/roko-learn/src/efficiency.rs"
    ) ;;
    09) files+=(
      "$REPO_ROOT/crates/roko-learn/src/costs_db.rs"
    ) ;;
    10) files+=(
      "$REPO_ROOT/crates/roko-learn/src/prompt_experiment.rs"
    ) ;;
    12) files+=(
      "$REPO_ROOT/crates/roko-learn/src/cascade_router.rs"
      "$REPO_ROOT/crates/roko-learn/src/model_router.rs"
    ) ;;
    20) files+=(
      "$REPO_ROOT/crates/roko-cli/src/research.rs"
      "$REPO_ROOT/crates/roko-agent/src/openai_agent.rs"
    ) ;;
    21) files+=(
      "$REPO_ROOT/crates/roko-cli/src/research.rs"
      "$REPO_ROOT/crates/roko-agent/src/openai_agent.rs"
      "$REPO_ROOT/crates/roko-agent/src/translate/mod.rs"
    ) ;;
  esac

  # Filter to files that actually exist
  for f in "${files[@]}"; do
    [[ -f "$f" ]] && echo "$f"
  done
}

# ─── Prompt generation ──────────────────────────────────────────────────────

generate_task_prompt() {
  local task_id="$1" title="$2" spec="$3" doc_num="$4"
  local prev_failure="${5:-}"

  local doc_title
  doc_title=$(extract_doc_title "$doc_num")

  local context_list=""
  while IFS= read -r f; do
    context_list+="- ${f}"$'\n'
  done < <(context_files_for_doc "$doc_num")

  local failure_context=""
  if [[ -n "$prev_failure" ]]; then
    failure_context="
## PREVIOUS ATTEMPT FAILED — READ THIS FIRST

Your previous attempt at this task failed verification. Here's what went wrong:

${prev_failure}

Fix the issues from the previous attempt. Do NOT repeat the same mistake.
"
  fi

  cat <<PROMPT
You are implementing task ${task_id} from the model routing plan: ${doc_title}

## Task: ${task_id} — ${title}
${failure_context}
${spec}

## Critical rules

1. **Search before writing**: \`grep -rn 'StructName\\|TraitName' crates/ --include='*.rs' | grep -v target/\`
2. **Wire existing code** — don't reimplement what exists.
3. **Only change what's needed** for this ONE task. Don't touch unrelated code.
4. **Run verification**: after your changes, run:
   - \`cargo check --workspace\` — MUST pass
   - \`cargo test --workspace --no-run\` — MUST compile
   - If the task spec has a Verification section, run those exact commands
5. **If something fails**, fix it before finishing. Don't leave broken code.

## Context files (read these first)

${context_list}

## When done

1. State what files you changed and why (brief)
2. Show the output of \`cargo check --workspace\`
3. If applicable, show test output
PROMPT
}

# ─── Verification gates ─────────────────────────────────────────────────────

# Gate 1: cargo check (fast, catches compile errors)
gate_check() {
  log_gate "Gate 1/3: cargo check --workspace"
  local output
  if output=$(do_timeout "$GATE_TIMEOUT" cargo check --workspace 2>&1); then
    log_ok "cargo check passed"
    return 0
  else
    log_err "cargo check FAILED"
    echo "$output" | tail -30
    echo "$output" | tail -30 > "$LOG_DIR/last-gate-failure.log"
    return 1
  fi
}

# Gate 2: cargo test --no-run (catches test compilation errors)
gate_test_compile() {
  log_gate "Gate 2/3: cargo test --workspace --no-run"
  local output
  if output=$(do_timeout "$GATE_TIMEOUT" cargo test --workspace --no-run 2>&1); then
    log_ok "test compile passed"
    return 0
  else
    log_warn "test compile had issues (non-blocking)"
    echo "$output" | grep -E "^error" | head -10
    # Non-blocking — some pre-existing test failures
    return 0
  fi
}

# Gate 3: targeted crate test (if we can determine the affected crate)
gate_crate_test() {
  local task_id="$1"
  local doc_num
  doc_num=$(task_to_doc "$task_id")

  local crate=""
  case "$doc_num" in
    02) crate="roko-core" ;;
    03|04|05|06|07|20|21) crate="roko-agent" ;;
    08|09|10|12) crate="roko-learn" ;;
    13|14) crate="roko-agent" ;;
    15|16|17|18) crate="" ;;  # cross-cutting
  esac

  if [[ -n "$crate" ]]; then
    log_gate "Gate 3/3: cargo test -p $crate --no-run"
    local output
    if output=$(do_timeout "$GATE_TIMEOUT" cargo test -p "$crate" --no-run 2>&1); then
      log_ok "crate test compile passed: $crate"
      return 0
    else
      log_err "crate test compile FAILED: $crate"
      echo "$output" | tail -20
      echo "$output" | tail -20 >> "$LOG_DIR/last-gate-failure.log"
      return 1
    fi
  else
    log_info "Gate 3/3: skipped (no specific crate for $task_id)"
    return 0
  fi
}

# Run all gates. Returns 0 if all pass, 1 if any fail.
# Writes failure details to $LOG_DIR/last-gate-failure.log
# Set SKIP_GATE2=true to skip the slow workspace test compile gate.
SKIP_GATE2="${SKIP_GATE2:-false}"

run_gates() {
  local task_id="$1"
  rm -f "$LOG_DIR/last-gate-failure.log"

  if ! gate_check; then return 1; fi
  if [[ "$SKIP_GATE2" != "true" ]]; then
    if ! gate_test_compile; then return 1; fi
  else
    log_info "Gate 2/3: skipped (SKIP_GATE2=true)"
  fi
  if ! gate_crate_test "$task_id"; then return 1; fi

  return 0
}

# ─── Agent dispatch ─────────────────────────────────────────────────────────

run_agent() {
  local prompt="$1" task_id="$2" doc_num="$3"

  case "$AGENT_TYPE" in
    claude) run_agent_claude "$prompt" "$task_id" "$doc_num" ;;
    codex)  run_agent_codex "$prompt" "$task_id" "$doc_num" ;;
    *)      log_err "Unknown agent: $AGENT_TYPE"; return 1 ;;
  esac
}

run_agent_claude() {
  local prompt="$1" task_id="$2" doc_num="$3"

  # Collect unique directories the agent might need access to
  local -A seen_dirs=()
  local deduped_args=()
  while IFS= read -r f; do
    local dir
    dir=$(dirname "$f")
    if [[ -z "${seen_dirs[$dir]+x}" ]]; then
      seen_dirs[$dir]=1
      deduped_args+=("--add-dir" "$dir")
    fi
  done < <(context_files_for_doc "$doc_num")

  local cmd_args=()
  [[ ${#deduped_args[@]} -gt 0 ]] && cmd_args+=("${deduped_args[@]}")
  [[ -n "${AGENT_MODEL:-}" ]] && cmd_args+=("--model" "$AGENT_MODEL")

  do_timeout "$TASK_TIMEOUT" $CLAUDE_CMD --print \
    "${cmd_args[@]}" \
    --output-format text \
    --max-turns 30 \
    --max-budget-usd "$MAX_BUDGET_USD" \
    --dangerously-skip-permissions \
    -p "$prompt"
}

run_agent_codex() {
  local prompt="$1" task_id="$2" doc_num="$3"

  local context_block=""
  while IFS= read -r f; do
    context_block+="- ${f}"$'\n'
  done < <(context_files_for_doc "$doc_num")

  local full_prompt
  full_prompt="$(cat <<EOF
## Context files to read first

${context_block}

## Task

${prompt}
EOF
)"

  local model="${AGENT_MODEL:-gpt-5.4-mini}"
  local extra_args=()
  [[ -n "${CODEX_REASONING:-}" ]] && extra_args+=("-c" "model_reasoning_effort=$CODEX_REASONING")

  do_timeout "$TASK_TIMEOUT" bash -c "echo $(printf '%q' "$full_prompt") | $CODEX_CMD exec \
    --model '$model' \
    --full-auto \
    ${extra_args[*]} \
    -"
}

# ─── Auto-commit ────────────────────────────────────────────────────────────

auto_commit() {
  local task_id="$1" title="$2"
  [[ "$AUTO_COMMIT" != "true" ]] && return 0
  if git diff --quiet && git diff --cached --quiet; then
    log_info "No changes to commit for $task_id"
    return 0
  fi
  git add -A
  git commit -m "$(cat <<EOF
modelrouting(${task_id}): ${title}

Automated implementation via run-modelrouting.sh
Agent: ${AGENT_TYPE}${AGENT_MODEL:+ (${AGENT_MODEL})}
EOF
)"
  log_ok "Committed: $task_id"
}

# ─── Run a single task ──────────────────────────────────────────────────────

run_task() {
  local task_id="$1" title="$2" doc_num="$3"
  local log_file="$LOG_DIR/${task_id}-$(date +%Y%m%d-%H%M%S).log"

  # Already done?
  if is_done "$task_id"; then
    echo -e "  ${DIM}✅ ${task_id} — ${title} (done)${NC}"
    return 0
  fi

  log_step "${task_id}: ${title}"
  _CURRENT_TASK_ID="$task_id"

  # Dry run?
  if [[ "$DRY_RUN" == "true" ]]; then
    local spec
    spec=$(extract_task_spec "$doc_num" "$task_id")
    log_info "[DRY RUN] Would run: ${task_id} — ${title}"
    echo -e "${DIM}${spec:0:200}...${NC}"
    echo ""
    return 0
  fi

  local spec
  spec=$(extract_task_spec "$doc_num" "$task_id")
  if [[ -z "$spec" ]]; then
    log_warn "No spec found for $task_id in doc $doc_num — skipping"
    return 0
  fi

  local attempt=0
  local prev_failure=""

  while (( attempt < MAX_RETRIES )); do
    attempt=$((attempt + 1))
    mark_running "$task_id" "$attempt"
    log_info "Attempt ${attempt}/${MAX_RETRIES}"

    # Save git state for potential rollback
    local pre_head
    pre_head=$(git rev-parse HEAD 2>/dev/null || echo "none")
    local had_changes=false
    if ! git diff --quiet 2>/dev/null; then had_changes=true; fi

    # Generate prompt (include failure context on retries)
    local prompt
    prompt=$(generate_task_prompt "$task_id" "$title" "$spec" "$doc_num" "$prev_failure")

    # Run agent
    local agent_start=$SECONDS
    local agent_exit=0
    run_agent "$prompt" "$task_id" "$doc_num" 2>&1 | tee -a "$log_file"
    agent_exit=${PIPESTATUS[0]}
    local agent_duration=$((SECONDS - agent_start))

    if (( agent_exit != 0 )); then
      # Agent itself crashed/timed out
      log_err "Agent process failed (exit $agent_exit) after ${agent_duration}s"
      state_add_attempt "$task_id" "agent_error" "not_run" "$agent_duration" "exit code $agent_exit"
      prev_failure="Agent process crashed with exit code $agent_exit after ${agent_duration}s. The task was NOT completed."

      # Rollback if agent left broken changes
      if ! git diff --quiet 2>/dev/null && [[ "$pre_head" != "none" ]]; then
        log_warn "Rolling back broken changes from failed agent"
        git checkout -- . 2>/dev/null || true
        git clean -fd 2>/dev/null || true
      fi
      continue
    fi

    # Agent completed — now run verification gates
    log_info "Agent finished in ${agent_duration}s — running gates..."

    local gate_start=$SECONDS
    local gate_exit=0
    run_gates "$task_id" 2>&1 | tee -a "$log_file"
    gate_exit=${PIPESTATUS[0]}
    if (( gate_exit == 0 )); then
      local gate_duration=$((SECONDS - gate_start))
      state_add_attempt "$task_id" "success" "pass" "$agent_duration" ""
      mark_done "$task_id"
      _CURRENT_TASK_ID=""
      auto_commit "$task_id" "$title"
      return 0
    else
      local gate_duration=$((SECONDS - gate_start))
      log_err "Gates FAILED after agent reported success — this is a false positive"

      # Capture the gate failure for the next attempt's context
      prev_failure=""
      if [[ -f "$LOG_DIR/last-gate-failure.log" ]]; then
        prev_failure=$(cat "$LOG_DIR/last-gate-failure.log" | tail -40)
      fi
      prev_failure="The agent completed but verification FAILED. Gate output:
\`\`\`
${prev_failure}
\`\`\`
The code changes did NOT pass \`cargo check --workspace\`. Fix the compilation errors."

      state_add_attempt "$task_id" "agent_ok" "gate_fail" "$agent_duration" "gate failed"

      # Rollback broken changes so next attempt starts clean
      if [[ "$pre_head" != "none" ]]; then
        log_warn "Stashing broken changes (git stash) for inspection"
        git stash push -m "modelrouting: ${task_id} attempt ${attempt} (gate failed)" 2>/dev/null || true
      fi
    fi
  done

  # All retries exhausted
  mark_failed "$task_id" "Failed after ${MAX_RETRIES} attempts (last: gate failure)"
  _CURRENT_TASK_ID=""
  if [[ "$STOP_ON_FAILURE" == "true" ]]; then
    log_err "Stopping due to --stop-on-failure"
    exit 1
  fi
  return 1
}

# ─── Run all tasks in a doc ─────────────────────────────────────────────────

run_doc() {
  local doc_num="$1"
  local doc_title
  doc_title=$(extract_doc_title "$doc_num")
  local phase
  phase=$(doc_to_phase "$doc_num")

  log_phase "Doc ${doc_num}: ${doc_title} (Phase ${phase})"

  # Check if doc already done
  if is_done "doc-${doc_num}"; then
    log_ok "Doc already complete — skipping"
    return 0
  fi

  # Check dependencies
  if ! doc_deps_satisfied "$doc_num"; then
    local deps
    deps=$(doc_depends_on "$doc_num")
    local missing=""
    for dep in $deps; do
      if ! is_done "doc-${dep}"; then missing+=" $dep"; fi
    done
    log_warn "Blocked by unfinished docs:${missing}"
    return 2  # 2 = blocked (not failure)
  fi

  # Extract tasks
  local tasks=()
  while IFS= read -r line; do
    [[ -n "$line" ]] && tasks+=("$line")
  done < <(extract_tasks_from_doc "$doc_num")

  local total=${#tasks[@]}
  if (( total == 0 )); then
    log_warn "No tasks found in doc $doc_num"
    return 0
  fi

  log_info "${total} tasks to implement"

  local done_count=0 fail_count=0 skip_count=0

  for task_def in "${tasks[@]}"; do
    local task_id="${task_def%%|*}"
    local title="${task_def#*|}"

    if run_task "$task_id" "$title" "$doc_num"; then
      done_count=$((done_count + 1))
    else
      fail_count=$((fail_count + 1))
    fi
  done

  log_info "Doc ${doc_num}: ${done_count}/${total} done, ${fail_count} failed"

  # Mark doc complete if all tasks passed
  if (( fail_count == 0 && done_count == total )); then
    state_set "doc-${doc_num}" "status=done"
    log_ok "Doc ${doc_num} fully complete ✅"
    return 0
  fi

  return 1
}

# ─── List all tasks ─────────────────────────────────────────────────────────

list_tasks() {
  echo -e "${BOLD}Model Routing Implementation Tasks${NC}"
  echo ""

  for doc_num in $(get_impl_docs); do
    local doc_title
    doc_title=$(extract_doc_title "$doc_num")
    local phase
    phase=$(doc_to_phase "$doc_num")
    local doc_status="⬜"
    is_done "doc-${doc_num}" && doc_status="✅"

    local deps
    deps=$(doc_depends_on "$doc_num")
    local deps_str=""
    [[ -n "$deps" ]] && deps_str=" ${DIM}(needs: ${deps})${NC}"

    echo -e "${BOLD}${doc_status} Doc ${doc_num}: ${doc_title} [Phase ${phase}]${deps_str}${NC}"

    local tasks=()
    while IFS= read -r line; do
      [[ -n "$line" ]] && tasks+=("$line")
    done < <(extract_tasks_from_doc "$doc_num")

    for task_def in "${tasks[@]}"; do
      local task_id="${task_def%%|*}"
      local title="${task_def#*|}"
      local status="⬜"
      local extra=""

      if is_done "$task_id"; then
        status="✅"
      elif is_failed "$task_id"; then
        status="❌"
        local reason
        reason=$(state_get "$task_id" "failure_reason")
        extra=" ${DIM}(${reason:0:50})${NC}"
      elif [[ "$(state_get "$task_id" status)" == "running" ]]; then
        status="🔄"
      fi

      local attempts
      attempts=$(state_get "$task_id" "attempt")
      local attempts_str=""
      [[ -n "$attempts" && "$attempts" != "0" ]] && attempts_str=" [${attempts} attempts]"

      printf "  %s %-8s %-60s%s%b\n" "$status" "$task_id" "${title:0:60}" "$attempts_str" "$extra"
    done
    echo ""
  done
}

# ─── Progress summary ───────────────────────────────────────────────────────

show_status() {
  local total=0 done=0 failed=0 running=0 blocked=0 pending=0

  for doc_num in $(get_impl_docs); do
    local tasks=()
    while IFS= read -r line; do
      [[ -n "$line" ]] && tasks+=("$line")
    done < <(extract_tasks_from_doc "$doc_num")

    for task_def in "${tasks[@]}"; do
      local task_id="${task_def%%|*}"
      total=$((total + 1))
      local s
      s=$(state_get "$task_id" "status")
      case "$s" in
        done)    done=$((done + 1)) ;;
        failed)  failed=$((failed + 1)) ;;
        running) running=$((running + 1)) ;;
        *)       pending=$((pending + 1)) ;;
      esac
    done
  done

  echo ""
  echo -e "${BOLD}Model Routing Progress${NC}"
  echo -e "───────────────────────────────────"
  echo -e "  Total:   ${total}"
  echo -e "  Done:    ${GREEN}${done}${NC}"
  echo -e "  Failed:  ${RED}${failed}${NC}"
  echo -e "  Running: ${YELLOW}${running}${NC}"
  echo -e "  Pending: ${pending}"
  echo ""

  if (( total > 0 )); then
    local pct=$((done * 100 / total))
    local bar_done=$((pct / 2))
    local bar_remaining=$((50 - bar_done))
    (( bar_done < 1 )) && bar_done=1
    (( bar_remaining < 0 )) && bar_remaining=0
    printf "  ${GREEN}%s${NC}%s %d%%\n" \
      "$(printf '█%.0s' $(seq 1 "$bar_done"))" \
      "$(printf '░%.0s' $(seq 1 "$bar_remaining"))" \
      "$pct"
  fi

  echo ""
  echo -e "${BOLD}Per-Phase:${NC}"
  for p in 1 2 3 4; do
    local phase_docs=""
    case $p in
      1) phase_docs="$PHASE_1_DOCS" ;;
      2) phase_docs="$PHASE_2_DOCS" ;;
      3) phase_docs="$PHASE_3_DOCS" ;;
      4) phase_docs="$PHASE_4_DOCS" ;;
    esac

    local p_total=0 p_done=0
    for doc_num in $phase_docs; do
      while IFS= read -r line; do
        [[ -z "$line" ]] && continue
        local tid="${line%%|*}"
        p_total=$((p_total + 1))
        is_done "$tid" && p_done=$((p_done + 1))
      done < <(extract_tasks_from_doc "$doc_num")
    done

    local p_pct=0
    (( p_total > 0 )) && p_pct=$((p_done * 100 / p_total))
    printf "  Phase %d: %3d/%3d (%d%%)\n" "$p" "$p_done" "$p_total" "$p_pct"
  done
  echo ""
}

# ─── Parse CLI args ─────────────────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
  case "$1" in
    --agent)            AGENT_TYPE="$2"; shift 2 ;;
    --model)            AGENT_MODEL="$2"; shift 2 ;;
    --reasoning)        CODEX_REASONING="$2"; shift 2 ;;
    --retries)          MAX_RETRIES="$2"; shift 2 ;;
    --timeout)          TASK_TIMEOUT="$2"; shift 2 ;;
    --doc)              FILTER_DOC="$2"; shift 2 ;;
    --task)             FILTER_TASK="$2"; shift 2 ;;
    --phase)            FILTER_PHASE="$2"; shift 2 ;;
    --dry-run)          DRY_RUN=true; shift ;;
    --commit)           AUTO_COMMIT=true; shift ;;
    --stop-on-failure)  STOP_ON_FAILURE=true; shift ;;
    --continuous)       CONTINUOUS=true; shift ;;
    --continuous-wait)  CONTINUOUS_WAIT="$2"; shift 2 ;;
    --preflight-wait)   PREFLIGHT_WAIT="$2"; shift 2 ;;
    --max-budget)       MAX_BUDGET_USD="$2"; shift 2 ;;
    --list)             DO_LIST=true; shift ;;
    --status)           DO_STATUS=true; shift ;;
    --resume)           DO_RESUME=true; shift ;;
    --reset)
      rm -rf "$STATE_DIR"
      mkdir -p "$STATE_DIR"
      echo '{}' > "$STATE_FILE"
      log_info "All state cleared"
      exit 0 ;;
    --reset-task)
      clear_task_state "$2"
      log_info "Cleared state for $2"
      exit 0 ;;
    --reset-doc)
      while IFS= read -r line; do
        [[ -z "$line" ]] && continue
        clear_task_state "${line%%|*}"
      done < <(extract_tasks_from_doc "$2")
      clear_task_state "doc-$2"
      log_info "Cleared state for doc $2 and all its tasks"
      exit 0 ;;
    --help|-h)
      cat <<'HELP'
Usage: bash tmp/run-modelrouting.sh [OPTIONS]

Agent options:
  --agent TYPE        Agent: claude (default) or codex
  --model MODEL       Model override (e.g., gpt-5.4-mini, sonnet, opus)
  --reasoning LEVEL   Codex reasoning: low, medium, high, xhigh
  --retries N         Max retries per task (default: 3)
  --timeout SECS      Per-task timeout in seconds (default: 600)

Scope options:
  --doc NN            Run only doc NN (e.g., 02, 20, 21)
  --task ID           Run only task ID (e.g., 2A.03, 2Q.06)
  --phase N           Run all docs in phase N (1-4)

Display options:
  --list              List all tasks with status
  --status            Show progress summary
  --dry-run           Preview prompts without running

Execution options:
  --commit            Auto-commit after each verified task
  --stop-on-failure   Stop on first task failure
  --resume            Pick up from last incomplete task
  --continuous        Keep looping: retry failed tasks until all done (overnight mode)
  --continuous-wait S Seconds between retry cycles (default: 120)
  --preflight-wait S  Seconds between pre-flight compile retries (default: 60)
  --max-budget USD    Per-task cost cap in USD (default: 3.00, claude only)

State options:
  --reset             Clear ALL state
  --reset-task ID     Clear state for one task
  --reset-doc NN      Clear state for a doc and all its tasks

Examples:
  # Run Phase 1 foundation with Claude
  bash tmp/run-modelrouting.sh --phase 1 --commit

  # Run Perplexity integration with Codex 5.4 mini
  bash tmp/run-modelrouting.sh --doc 20 --agent codex --model gpt-5.4-mini --reasoning high

  # Run Gemini integration with Claude Sonnet
  bash tmp/run-modelrouting.sh --doc 21 --agent claude --model sonnet

  # Run a single task
  bash tmp/run-modelrouting.sh --task 2Q.06

  # Check progress
  bash tmp/run-modelrouting.sh --status
HELP
      exit 0 ;;
    *) log_err "Unknown option: $1 (try --help)"; exit 1 ;;
  esac
done

# ─── Display modes ──────────────────────────────────────────────────────────

if [[ "$DO_LIST" == "true" ]]; then list_tasks; show_status; exit 0; fi
if [[ "$DO_STATUS" == "true" ]]; then show_status; exit 0; fi

# ─── Pre-flight checks ─────────────────────────────────────────────────────

log_phase "Model Routing — Automated Implementation"
log_info "Agent:     ${AGENT_TYPE}${AGENT_MODEL:+ (model: ${AGENT_MODEL})}${CODEX_REASONING:+ (reasoning: ${CODEX_REASONING})}"
log_info "Retries:   ${MAX_RETRIES} per task (with gate verification)"
log_info "Timeout:   ${TASK_TIMEOUT}s per task, ${GATE_TIMEOUT}s per gate"
log_info "State:     ${STATE_FILE}"
log_info "Logs:      ${LOG_DIR}/"
echo ""

case "$AGENT_TYPE" in
  claude) command -v "$CLAUDE_CMD" &>/dev/null || { log_err "claude CLI not found"; exit 1; } ;;
  codex)  command -v "$CODEX_CMD" &>/dev/null || { log_err "codex CLI not found"; exit 1; } ;;
esac
cargo --version &>/dev/null || { log_err "Rust toolchain not found"; exit 1; }

# Reset any stale "running" tasks from a previous crashed run
log_info "Checking for stale running tasks..."
python3 -c "
import json
with open('$STATE_FILE') as f: s = json.load(f)
changed = False
for k, v in s.items():
    if isinstance(v, dict) and v.get('status') == 'running':
        print(f'  Resetting stale task: {k}')
        v['status'] = 'pending'
        changed = True
if changed:
    with open('$STATE_FILE', 'w') as f: json.dump(s, f, indent=2)
else:
    print('  None found')
"

# Verify workspace compiles before starting (retry forever for overnight runs)
PREFLIGHT_WAIT="${PREFLIGHT_WAIT:-60}"   # seconds between retries
PREFLIGHT_MAX="${PREFLIGHT_MAX:-0}"      # 0 = infinite retries
_preflight_attempt=0
while true; do
  _preflight_attempt=$((_preflight_attempt + 1))
  log_info "Pre-flight: cargo check... (attempt ${_preflight_attempt})"
  if cargo check --workspace 2>&1 | tail -3; then
    log_ok "Workspace compiles"
    break
  fi
  if (( PREFLIGHT_MAX > 0 && _preflight_attempt >= PREFLIGHT_MAX )); then
    log_err "Workspace doesn't compile after ${PREFLIGHT_MAX} attempts — giving up"
    exit 1
  fi
  log_warn "Workspace doesn't compile — retrying in ${PREFLIGHT_WAIT}s (attempt ${_preflight_attempt})"
  sleep "$PREFLIGHT_WAIT"
done
echo ""

# ─── Single task mode ───────────────────────────────────────────────────────

if [[ -n "$FILTER_TASK" ]]; then
  _doc_num=$(task_to_doc "$FILTER_TASK")
  if [[ -z "$_doc_num" ]]; then
    log_err "Can't determine doc for task $FILTER_TASK"
    exit 1
  fi

  # Find the task title
  _title=""
  while IFS= read -r line; do
    _tid="${line%%|*}"
    if [[ "$_tid" == "$FILTER_TASK" ]]; then
      _title="${line#*|}"
      break
    fi
  done < <(extract_tasks_from_doc "$_doc_num")

  if [[ -z "$_title" ]]; then
    log_err "Task $FILTER_TASK not found in doc $_doc_num"
    exit 1
  fi

  run_task "$FILTER_TASK" "$_title" "$_doc_num"
  show_status
  exit $?
fi

# ─── Main execution loop ───────────────────────────────────────────────────

cd "$REPO_ROOT"

# Determine which docs to run
docs_to_run=""
if [[ -n "$FILTER_DOC" ]]; then
  docs_to_run="$FILTER_DOC"
elif [[ -n "$FILTER_PHASE" ]]; then
  case "$FILTER_PHASE" in
    1) docs_to_run="$PHASE_1_DOCS" ;;
    2) docs_to_run="$PHASE_2_DOCS" ;;
    3) docs_to_run="$PHASE_3_DOCS" ;;
    4) docs_to_run="$PHASE_4_DOCS" ;;
    *) log_err "Invalid phase: $FILTER_PHASE (1-4)"; exit 1 ;;
  esac
else
  docs_to_run=$(get_impl_docs)
fi

# ─── Resilient outer loop (--continuous for overnight runs) ──────────────
#
# In normal mode: run once and exit.
# With --continuous or CONTINUOUS=true: keep looping, retrying failed/pending
# tasks until everything is done or the user kills it.
CONTINUOUS="${CONTINUOUS:-false}"
CONTINUOUS_WAIT="${CONTINUOUS_WAIT:-120}"  # seconds between full passes

_run_one_pass() {
  local completed=0 failed_count=0 blocked_count=0
  local start_time=$SECONDS

  # Multi-pass loop to handle dependency chains
  local made_progress=true
  local pass=0

  while [[ "$made_progress" == "true" ]]; do
    made_progress=false
    pass=$((pass + 1))
    (( pass > 1 )) && log_phase "Pass ${pass} — checking newly unblocked docs"

    for doc_num in $docs_to_run; do
      # Skip already done
      is_done "doc-${doc_num}" && continue

      local result=0
      run_doc "$doc_num" || result=$?

      case $result in
        0) completed=$((completed + 1)); made_progress=true ;;
        1) failed_count=$((failed_count + 1)) ;;
        2) blocked_count=$((blocked_count + 1)) ;;
      esac
    done
  done

  local total_time=$((SECONDS - start_time))
  echo ""
  log_phase "Execution Complete (${total_time}s)"
  show_status

  if (( failed_count > 0 )); then
    log_warn "${failed_count} doc(s) had failures. Run --list to see details, --resume to retry."
  fi
  if (( blocked_count > 0 )); then
    log_info "${blocked_count} doc(s) still blocked by dependencies."
  fi

  # Return 0 if everything is done, 1 if there's still work
  local all_done=true
  for doc_num in $docs_to_run; do
    is_done "doc-${doc_num}" || { all_done=false; break; }
  done
  [[ "$all_done" == "true" ]] && return 0 || return 1
}

_reset_stale_for_retry() {
  for doc_num in $docs_to_run; do
    while IFS= read -r line; do
      [[ -z "$line" ]] && continue
      local tid="${line%%|*}"
      local st
      st=$(state_get "$tid" "status")
      if [[ "$st" == "failed" || "$st" == "running" ]]; then
        log_info "Resetting $st task $tid for retry"
        clear_task_state "$tid"
      fi
    done < <(extract_tasks_from_doc "$doc_num")
    # Also reset doc-level failure/running so run_doc re-enters
    local dst
    dst=$(state_get "doc-${doc_num}" "status")
    if [[ "$dst" == "failed" || "$dst" == "running" ]]; then
      clear_task_state "doc-${doc_num}"
    fi
  done
}

if [[ "$CONTINUOUS" == "true" ]]; then
  _cycle=0
  while true; do
    _cycle=$((_cycle + 1))
    log_phase "Continuous run — cycle ${_cycle}"
    _log_progress "=== Cycle ${_cycle} starting ==="

    # Reset failed and stale "running" tasks so they get retried
    _reset_stale_for_retry

    # Re-verify workspace compiles between cycles (agents may have broken it)
    log_info "Pre-cycle compile check..."
    if ! cargo check --workspace 2>&1 | tail -3; then
      log_warn "Workspace broken — attempting auto-fix with git checkout"
      # Discard unstaged changes that broke the build
      git checkout -- . 2>/dev/null || true
      if ! cargo check --workspace >/dev/null 2>&1; then
        log_err "Workspace still broken after checkout — waiting ${CONTINUOUS_WAIT}s"
        _log_progress "BLOCKED — workspace won't compile"
        sleep "$CONTINUOUS_WAIT"
        continue
      fi
      log_ok "Workspace fixed after checkout"
    fi

    if _run_one_pass; then
      log_ok "All docs complete — exiting continuous loop"
      _log_progress "=== ALL DONE ==="
      break
    fi

    _log_progress "=== Cycle ${_cycle} done, waiting ${CONTINUOUS_WAIT}s ==="
    log_info "Waiting ${CONTINUOUS_WAIT}s before next cycle..."
    sleep "$CONTINUOUS_WAIT"
  done
else
  _run_one_pass || true
fi
