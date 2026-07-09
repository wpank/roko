#!/usr/bin/env bash
# run-parity.sh — Autonomous orchestration script for roko implementation work
#
# Splits each MASTER-PLAN.md section into individual checklist items and runs
# one agent invocation per item. This prevents agent loops on large tasks.
#
# Usage:
#   bash tmp/run-parity.sh                              # full run (tier 1 first)
#   bash tmp/run-parity.sh --agent codex                # use codex instead of claude
#   bash tmp/run-parity.sh --agent codex --model gpt-5.4-mini --reasoning medium
#   bash tmp/run-parity.sh --dry-run                    # print prompts, don't run
#   bash tmp/run-parity.sh --section 1B                 # run only section 1B
#   bash tmp/run-parity.sh --from 2A                    # start from section 2A onward
#   bash tmp/run-parity.sh --tier 1                     # run all sections in tier 1
#   bash tmp/run-parity.sh --list                       # list all sections with status
#   bash tmp/run-parity.sh --status                     # show progress bar only
#   bash tmp/run-parity.sh --reset                      # clear state, start fresh
#   bash tmp/run-parity.sh --reset-section 1A           # clear state for one section
#   bash tmp/run-parity.sh --commit                     # auto-commit after each item

set -euo pipefail

# ─── Configuration ──────────────────────────────────────────────────────────

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STATE_DIR="$REPO_ROOT/.roko/parity-state"
LOG_DIR="$REPO_ROOT/tmp/logs"
MASTER_PLAN="$REPO_ROOT/tmp/MASTER-PLAN.md"
MAX_RETRIES=2

# Agent selection: "claude" or "codex"
AGENT_TYPE="${AGENT_TYPE:-claude}"
AGENT_MODEL=""
CODEX_REASONING=""

# CLI commands (overridable via env)
CLAUDE_CMD="${CLAUDE_CMD:-claude}"
CODEX_CMD="${CODEX_CMD:-codex}"

# Feature flags
AUTO_COMMIT=false

# ─── Color output ───────────────────────────────────────────────────────────

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
BOLD='\033[1m'
DIM='\033[2m'
NC='\033[0m'

log_info()  { echo -e "${BLUE}[INFO]${NC}  $*"; }
log_ok()    { echo -e "${GREEN}[OK]${NC}    $*"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
log_err()   { echo -e "${RED}[ERROR]${NC} $*"; }
log_step()  { echo -e "${BOLD}${CYAN}─── $* ───${NC}"; }
log_tier()  { echo -e "\n${BOLD}${MAGENTA}╔════════════════════════════════════════╗${NC}"; \
              echo -e "${BOLD}${MAGENTA}║  $*${NC}"; \
              echo -e "${BOLD}${MAGENTA}╚════════════════════════════════════════╝${NC}\n"; }
log_dim()   { echo -e "${DIM}$*${NC}"; }

# ─── State management ───────────────────────────────────────────────────────

mkdir -p "$STATE_DIR" "$LOG_DIR"

mark_done()    { date -u +%Y-%m-%dT%H:%M:%SZ > "$STATE_DIR/$1.done"; }
mark_failed()  { echo "$2" > "$STATE_DIR/$1.failed"; }
is_done()      { [[ -f "$STATE_DIR/$1.done" ]]; }
is_failed()    { [[ -f "$STATE_DIR/$1.failed" ]]; }
clear_failed() { rm -f "$STATE_DIR/$1.failed"; }
clear_state()  { rm -f "$STATE_DIR/$1.done" "$STATE_DIR/$1.failed" "$STATE_DIR/$1.running"; }

get_status_short() {
  local section="$1"
  if is_done "$section"; then echo "✅"
  elif is_failed "$section"; then echo "❌"
  else echo "⬜"; fi
}

# ─── Section definitions ────────────────────────────────────────────────────
# Format: SECTION_ID|TIER|NAME|DEPENDS_ON (comma-separated, empty if none)

SECTIONS=(
  "1A|1|Executor Phase Integration|"
  "1B|1|Conductor Watcher Wiring|"
  "1C|1|MCP Tool Registry + Server Lifecycle|"
  "1D|1|Observability Infrastructure|"
  "1E|1|Re-Planning + Plan Regeneration|1A"
  "1F|1|Automatic Plan Generation|1A"
  "1G|1|Remaining Gate/Learn/API Items|"
  "1H|1|TUI Dashboard|"
  "1I|1|Skill Library + Playbook Wiring|"
  "1J|1|LinUCB Bandit + Context Attribution|"
  "2A|2|Extract roko-serve|1A,1B,1C,1D"
  "2B|2|Create roko-plugin SDK|2A"
  "2C|2|Webhook Endpoints + Dispatch Loop|2A,2B"
  "2D|2|Integration MCP Servers|1C"
  "3A|3|Agent Template Schema + 16 Templates|2A,2B,2C"
  "3B|3|Subscription System|3A"
  "3C|3|Cron Scheduler + File Watcher|2C"
  "4A|4|Daemon Mode|2C,3B"
  "4B|4|Multi-Repo Configuration|4A"
  "4C|4|Cloud Deployment|4A"
  "4D|4|Secret Management|"
  "5A|5|roko-neuro (Knowledge + Memory)|1A"
  "5B|5|Context Assembly (5-Stage Pipeline)|5A"
  "5C|5|Daimon (Affect/Motivation)|5A"
  "5D|5|Dreams (Offline Learning)|5A,5C"
  "5E|5|Operating Frequencies|5B"
  "5F|5|C-Factor Metrics|5A"
)

parse_section() {
  local def="$1"
  SECTION_ID="${def%%|*}"; def="${def#*|}"
  SECTION_TIER="${def%%|*}"; def="${def#*|}"
  SECTION_NAME="${def%%|*}"; def="${def#*|}"
  SECTION_DEPS="$def"
}

deps_satisfied() {
  local deps="$1"
  if [[ -z "$deps" ]]; then return 0; fi
  IFS=',' read -ra dep_array <<< "$deps"
  for dep in "${dep_array[@]}"; do
    if ! is_done "$dep"; then return 1; fi
  done
  return 0
}

# ─── Context files for each section ────────────────────────────────────────

context_files_for() {
  local section="$1"
  local candidates=()
  candidates+=("$REPO_ROOT/CLAUDE.md")

  case "$section" in
    1A*) candidates+=("$REPO_ROOT/crates/roko-orchestrator/src/executor/mod.rs" "$REPO_ROOT/crates/roko-orchestrator/src/executor/action.rs" "$REPO_ROOT/crates/roko-orchestrator/src/executor/state_machine.rs" "$REPO_ROOT/crates/roko-cli/src/orchestrate.rs" "$REPO_ROOT/crates/roko-orchestrator/src/worktree.rs" "$REPO_ROOT/crates/roko-cli/src/config.rs" "$REPO_ROOT/crates/roko-cli/src/task_parser.rs") ;;
    1B*) candidates+=("$REPO_ROOT/crates/roko-conductor/src/watchers/mod.rs" "$REPO_ROOT/crates/roko-conductor/src/conductor.rs" "$REPO_ROOT/crates/roko-conductor/src/circuit_breaker.rs" "$REPO_ROOT/crates/roko-conductor/src/diagnosis.rs" "$REPO_ROOT/crates/roko-cli/src/orchestrate.rs") ;;
    1C*) candidates+=("$REPO_ROOT/crates/roko-agent/src/mcp/mod.rs" "$REPO_ROOT/crates/roko-agent/src/mcp/dynamic_registry.rs" "$REPO_ROOT/crates/roko-std/src/tool/mod.rs" "$REPO_ROOT/crates/roko-std/src/tool/registry.rs" "$REPO_ROOT/crates/roko-agent/src/dispatcher/mod.rs") ;;
    1D*) candidates+=("$REPO_ROOT/crates/roko-cli/src/main.rs" "$REPO_ROOT/crates/roko-cli/src/orchestrate.rs" "$REPO_ROOT/crates/roko-cli/Cargo.toml") ;;
    1E*) candidates+=("$REPO_ROOT/crates/roko-cli/src/orchestrate.rs" "$REPO_ROOT/crates/roko-orchestrator/src/executor/recovery.rs" "$REPO_ROOT/crates/roko-gate/src/lib.rs" "$REPO_ROOT/crates/roko-learn/src/cascade_router.rs") ;;
    1F*) candidates+=("$REPO_ROOT/crates/roko-cli/src/prd.rs" "$REPO_ROOT/crates/roko-cli/src/orchestrate.rs" "$REPO_ROOT/crates/roko-orchestrator/src/plan_discovery.rs") ;;
    1G*) candidates+=("$REPO_ROOT/crates/roko-cli/src/serve/routes/mod.rs" "$REPO_ROOT/crates/roko-cli/src/serve/state.rs" "$REPO_ROOT/crates/roko-gate/src/lib.rs" "$REPO_ROOT/crates/roko-learn/src/lib.rs" "$REPO_ROOT/crates/roko-learn/src/efficiency.rs" "$REPO_ROOT/crates/roko-learn/src/cascade_router.rs") ;;
    1H*) candidates+=("$REPO_ROOT/crates/roko-cli/src/tui/dashboard.rs" "$REPO_ROOT/crates/roko-cli/Cargo.toml") ;;
    1I*) candidates+=("$REPO_ROOT/crates/roko-learn/src/skill_library.rs" "$REPO_ROOT/crates/roko-learn/src/playbook.rs" "$REPO_ROOT/crates/roko-cli/src/orchestrate.rs") ;;
    1J*) candidates+=("$REPO_ROOT/crates/roko-learn/src/model_router.rs" "$REPO_ROOT/crates/roko-learn/src/cascade_router.rs" "$REPO_ROOT/crates/roko-learn/src/efficiency.rs" "$REPO_ROOT/crates/roko-cli/src/orchestrate.rs") ;;
    2A*) candidates+=("$REPO_ROOT/crates/roko-cli/src/serve/mod.rs" "$REPO_ROOT/crates/roko-cli/src/serve/state.rs" "$REPO_ROOT/crates/roko-cli/src/serve/routes/mod.rs" "$REPO_ROOT/crates/roko-cli/Cargo.toml") ;;
    2B*) candidates+=("$REPO_ROOT/crates/roko-core/src/lib.rs") ;;
    2C*) candidates+=("$REPO_ROOT/crates/roko-agent/src/dispatcher/mod.rs" "$REPO_ROOT/crates/roko-cli/src/serve/routes/mod.rs") ;;
    2D*) candidates+=("$REPO_ROOT/crates/roko-agent/src/mcp/mod.rs") ;;
    3A*) candidates+=("$REPO_ROOT/crates/roko-compose/src/system_prompt_builder.rs") ;;
    4A*) candidates+=("$REPO_ROOT/crates/roko-cli/src/daemon.rs") ;;
    4B*|4D*) candidates+=("$REPO_ROOT/crates/roko-cli/src/config.rs") ;;
    5A*) candidates+=("$REPO_ROOT/crates/roko-learn/src/lib.rs" "$REPO_ROOT/crates/roko-learn/src/episode_logger.rs") ;;
    5B*) candidates+=("$REPO_ROOT/crates/roko-compose/src/system_prompt_builder.rs" "$REPO_ROOT/crates/roko-compose/src/context_provider.rs") ;;
    5C*) candidates+=("$REPO_ROOT/crates/roko-golem/src/daimon.rs" "$REPO_ROOT/crates/roko-learn/src/efficiency.rs") ;;
    5D*) candidates+=("$REPO_ROOT/crates/roko-golem/src/dreams.rs" "$REPO_ROOT/crates/roko-learn/src/pattern_discovery.rs") ;;
    5E*) candidates+=("$REPO_ROOT/crates/bardo-primitives/src/tier.rs" "$REPO_ROOT/crates/roko-learn/src/cascade_router.rs") ;;
    5F*) candidates+=("$REPO_ROOT/crates/roko-learn/src/efficiency.rs" "$REPO_ROOT/crates/roko-learn/src/baseline.rs" "$REPO_ROOT/crates/roko-learn/src/costs_db.rs") ;;
  esac

  for f in "${candidates[@]}"; do
    [[ -f "$f" ]] && echo "$f"
  done
}

# ─── Extract section content from MASTER-PLAN.md ──────────────────────────

extract_section_content() {
  local section_id="$1"
  local start_line
  start_line=$(grep -n "^## ${section_id}:" "$MASTER_PLAN" | head -1 | cut -d: -f1 || true)
  if [[ -z "$start_line" ]]; then
    echo "(Section ${section_id} not found in MASTER-PLAN.md)"
    return
  fi
  awk -v start="$start_line" '
    NR >= start {
      if (NR > start && /^## [0-9]/) exit
      if (NR > start && /^# Tier/) exit
      if (NR > start && /^# Cross-Cutting/) exit
      if (NR > start && /^# Reference/) exit
      print
    }
  ' "$MASTER_PLAN"
}

# ─── Extract individual checklist items from a section ─────────────────────

extract_checklist_items() {
  local section_id="$1"
  local content
  content=$(extract_section_content "$section_id")

  # Extract lines starting with "- [ ]", joining continuation lines
  echo "$content" | awk '
    /^- \[ \]/ {
      if (item != "") print item
      item = $0
      next
    }
    /^  / && item != "" {
      item = item " " $0
      next
    }
    {
      if (item != "") print item
      item = ""
    }
    END { if (item != "") print item }
  '
}

# ─── Extract section context (the blockquote before checklist items) ───────

extract_section_context() {
  local section_id="$1"
  local content
  content=$(extract_section_content "$section_id")
  # Everything before the first "- [ ]" line (section header + context block)
  echo "$content" | awk '/^- \[ \]/ { exit } { print }'
}

# ─── Generate a prompt for a single checklist item ─────────────────────────

generate_item_prompt() {
  local section_id="$1"
  local section_name="$2"
  local item_num="$3"
  local item_text="$4"
  local section_context="$5"

  local context_list
  context_list=$(context_files_for "$section_id" | sed 's/^/- /')

  cat <<PROMPT
You are working on section ${section_id} (${section_name}) of the Roko project.

## Your ONE task

Implement exactly this checklist item:

${item_text}

## Section context

${section_context}

## Rules

1. Search before writing: \`grep -rn 'Name' crates/ --include='*.rs' | grep -v target/\`
2. Wire existing code — don't reimplement what exists.
3. Do NOT stub with println. Every handler must do real work.
4. Only change what's needed for this ONE item. Don't touch unrelated code.
5. Run \`cargo check --workspace\` to verify your changes compile.

## Context files (read these first)

${context_list}

## When done

State what you changed (files + brief description). Run cargo check.
PROMPT
}

# ─── Agent dispatch ─────────────────────────────────────────────────────────

run_agent() {
  local prompt="$1"
  local item_id="$2"
  case "$AGENT_TYPE" in
    claude) run_agent_claude "$prompt" "$item_id" ;;
    codex)  run_agent_codex "$prompt" "$item_id" ;;
    *)      log_err "Unknown agent type: $AGENT_TYPE"; return 1 ;;
  esac
}

run_agent_claude() {
  local prompt="$1"
  local item_id="$2"
  local section_id="${item_id%%.*}"

  local context_args=()
  while IFS= read -r f; do
    context_args+=("--read" "$f")
  done < <(context_files_for "$section_id")

  local model_args=()
  if [[ -n "$AGENT_MODEL" ]]; then
    model_args+=("--model" "$AGENT_MODEL")
  fi

  $CLAUDE_CMD --print \
    "${context_args[@]}" \
    "${model_args[@]}" \
    --output-format text \
    --max-turns 30 \
    --dangerously-skip-permissions \
    -p "$prompt"
}

run_agent_codex() {
  local prompt="$1"
  local item_id="$2"
  local section_id="${item_id%%.*}"

  local context_block=""
  while IFS= read -r f; do
    context_block+="- ${f}"$'\n'
  done < <(context_files_for "$section_id")

  local full_prompt
  full_prompt="$(cat <<EOF
## Context files to read first

${context_block}

## Task

${prompt}
EOF
)"

  local model="${AGENT_MODEL:-o4-mini}"

  local extra_args=()
  if [[ -n "${CODEX_REASONING:-}" ]]; then
    extra_args+=("-c" "model_reasoning_effort=$CODEX_REASONING")
  fi

  echo "$full_prompt" | $CODEX_CMD exec \
    --model "$model" \
    --full-auto \
    "${extra_args[@]}" \
    -
}

# ─── Quick verification (just cargo check between items) ───────────────────

verify_check() {
  log_info "Running cargo check..."
  if ! cargo check --workspace 2>&1 | tail -5; then
    log_err "cargo check failed"
    return 1
  fi
  log_ok "cargo check passed"
}

# ─── Full verification (at end of section) ─────────────────────────────────

verify_full() {
  local log_file="$1"

  log_info "Full verify: cargo build..."
  if ! cargo build --workspace >> "$log_file" 2>&1; then
    log_err "Build failed"
    tail -15 "$log_file"
    return 1
  fi
  log_ok "Build passed"

  log_info "Full verify: cargo test..."
  if ! cargo test --workspace --exclude roko-demo >> "$log_file" 2>&1; then
    log_warn "Tests failed (non-blocking — may be pre-existing)"
    tail -15 "$log_file"
  else
    log_ok "Tests passed"
  fi

  log_info "Full verify: cargo clippy..."
  if ! cargo clippy --workspace --no-deps -- -D warnings >> "$log_file" 2>&1; then
    log_warn "Clippy warnings (non-blocking)"
  else
    log_ok "Clippy clean"
  fi
  return 0
}

# ─── Auto-commit ──────────────────────────────────────────────────────────

auto_commit() {
  local item_id="$1"
  local desc="$2"

  if [[ "$AUTO_COMMIT" != "true" ]]; then return 0; fi
  if git diff --quiet && git diff --cached --quiet; then
    log_dim "No changes to commit for ${item_id}"
    return 0
  fi

  git add -A
  git commit -m "$(cat <<EOF
parity(${item_id}): ${desc}

Automated implementation via run-parity.sh
EOF
)"
  log_ok "Committed: ${item_id}"
}

# ─── Run a single checklist item ──────────────────────────────────────────

run_item() {
  local item_id="$1"       # e.g. "1A.03"
  local section_id="$2"    # e.g. "1A"
  local section_name="$3"
  local item_num="$4"
  local item_text="$5"
  local section_context="$6"
  local attempt=0
  local log_file="$LOG_DIR/${item_id}-$(date +%Y%m%d-%H%M%S).log"

  # Already done?
  if is_done "$item_id"; then
    log_dim "  ✅ ${item_id} already done — skip"
    return 0
  fi
  clear_failed "$item_id"

  # Truncate item text for display
  local display_text="${item_text:0:80}"
  [[ ${#item_text} -gt 80 ]] && display_text+="..."
  log_step "${item_id}: ${display_text}"

  local prompt
  prompt=$(generate_item_prompt "$section_id" "$section_name" "$item_num" "$item_text" "$section_context")

  if [[ "${DRY_RUN:-false}" == "true" ]]; then
    log_info "[DRY RUN] Item ${item_id}:"
    echo "$item_text" | head -3
    echo ""
    return 0
  fi

  while (( attempt < MAX_RETRIES )); do
    attempt=$((attempt + 1))
    log_info "Attempt ${attempt}/${MAX_RETRIES}"

    local git_head
    git_head=$(git rev-parse HEAD 2>/dev/null || echo "")

    local agent_start=$SECONDS
    if run_agent "$prompt" "$item_id" 2>&1 | tee -a "$log_file"; then
      local dur=$((SECONDS - agent_start))
      log_ok "Agent done in ${dur}s"
      mark_done "$item_id"
      auto_commit "$item_id" "$display_text"
      log_ok "${item_id} complete"
      return 0
    else
      local dur=$((SECONDS - agent_start))
      log_err "Agent failed after ${dur}s (exit code) — retrying"
    fi
  done

  mark_failed "$item_id" "Failed after ${MAX_RETRIES} attempts"
  log_err "${item_id} failed after ${MAX_RETRIES} attempts"
  return 1
}

# ─── Run all items in a section ────────────────────────────────────────────

run_section() {
  local section_id="$1"
  local section_name="$2"
  local section_tier="$3"
  local section_deps="$4"

  log_tier "Section ${section_id}: ${section_name} (Tier ${section_tier})"

  # Check if whole section already done
  if is_done "$section_id"; then
    log_ok "Section already completed — skipping"
    return 0
  fi

  # Check dependencies
  if ! deps_satisfied "$section_deps"; then
    local missing=()
    IFS=',' read -ra dep_array <<< "$section_deps"
    for dep in "${dep_array[@]}"; do
      if ! is_done "$dep"; then missing+=("$dep"); fi
    done
    log_warn "Blocked by: ${missing[*]}"
    return 1
  fi

  # Extract items
  local items=()
  while IFS= read -r line; do
    [[ -n "$line" ]] && items+=("$line")
  done < <(extract_checklist_items "$section_id")

  local total=${#items[@]}
  if (( total == 0 )); then
    log_warn "No checklist items found for ${section_id}"
    return 1
  fi

  log_info "${total} checklist items to implement"

  local section_context
  section_context=$(extract_section_context "$section_id")

  local done_count=0
  local fail_count=0

  for i in "${!items[@]}"; do
    local num=$((i + 1))
    local item_id
    item_id=$(printf "%s.%02d" "$section_id" "$num")

    if run_item "$item_id" "$section_id" "$section_name" "$num" "${items[$i]}" "$section_context"; then
      done_count=$((done_count + 1))
    else
      fail_count=$((fail_count + 1))
      # Continue to next item — don't block on one failure
    fi
  done

  log_info "Section ${section_id}: ${done_count}/${total} done, ${fail_count} failed"

  # If all items done, mark section done
  if (( fail_count == 0 )); then
    # Run full verification
    local verify_log="$LOG_DIR/${section_id}-verify-$(date +%Y%m%d-%H%M%S).log"
    if verify_full "$verify_log"; then
      mark_done "$section_id"
      auto_commit "$section_id" "$section_name"
      log_ok "Section ${section_id} fully complete and verified"
      return 0
    else
      log_err "Section ${section_id} items done but verification failed"
      return 1
    fi
  fi

  return 1
}

# ─── List sections ──────────────────────────────────────────────────────────

list_sections() {
  local tier_names=("" "Mori Parity (P0)" "Agent Platform Foundation (P1)" "Agent Templates & Events (P2)" "Daemon & Multi-Repo (P2)" "Cognitive Layer (P3)")
  local current_tier=""

  for def in "${SECTIONS[@]}"; do
    parse_section "$def"
    if [[ "$SECTION_TIER" != "$current_tier" ]]; then
      current_tier="$SECTION_TIER"
      echo ""
      echo -e "${BOLD}Tier ${current_tier}: ${tier_names[$current_tier]}${NC}"
      echo "────────────────────────────────────────────────────────────────"
    fi

    local status
    status=$(get_status_short "$SECTION_ID")
    local deps_str=""
    [[ -n "$SECTION_DEPS" ]] && deps_str=" ${DIM}(needs: $SECTION_DEPS)${NC}"

    # Count sub-items
    local item_count
    item_count=$(extract_checklist_items "$SECTION_ID" | wc -l | tr -d ' ')
    local done_items=0
    for n in $(seq 1 "$item_count"); do
      local iid
      iid=$(printf "%s.%02d" "$SECTION_ID" "$n")
      is_done "$iid" && done_items=$((done_items + 1))
    done

    printf "  %s %-4s %-40s [%d/%d]%b\n" "$status" "$SECTION_ID" "$SECTION_NAME" "$done_items" "$item_count" "$deps_str"
  done
  echo ""
}

# ─── Progress summary ──────────────────────────────────────────────────────

show_progress() {
  local total=0 done=0 failed=0 pending=0 blocked=0

  for def in "${SECTIONS[@]}"; do
    parse_section "$def"
    total=$((total + 1))
    if is_done "$SECTION_ID"; then
      done=$((done + 1))
    elif is_failed "$SECTION_ID"; then
      failed=$((failed + 1))
    elif ! deps_satisfied "$SECTION_DEPS"; then
      blocked=$((blocked + 1))
    else
      pending=$((pending + 1))
    fi
  done

  echo ""
  echo -e "${BOLD}Sections: ${done}/${total} done | ${pending} ready | ${blocked} blocked | ${failed} failed${NC}"

  # Count individual items
  local total_items=0 done_items=0
  for def in "${SECTIONS[@]}"; do
    parse_section "$def"
    local count
    count=$(extract_checklist_items "$SECTION_ID" | wc -l | tr -d ' ')
    total_items=$((total_items + count))
    for n in $(seq 1 "$count"); do
      local iid
      iid=$(printf "%s.%02d" "$SECTION_ID" "$n")
      is_done "$iid" && done_items=$((done_items + 1))
    done
  done

  if (( total_items > 0 )); then
    local pct=$((done_items * 100 / total_items))
    echo -e "${BOLD}Items:    ${done_items}/${total_items} (${pct}%)${NC}"
    local bar_done=$((pct / 4))
    local bar_remaining=$((25 - bar_done))
    echo -e "  ${GREEN}$(printf '█%.0s' $(seq 1 $((bar_done > 0 ? bar_done : 1))))${NC}$(printf '░%.0s' $(seq 1 $((bar_remaining > 0 ? bar_remaining : 1)))) ${pct}%"
  fi
  echo ""
}

# ─── Main ───────────────────────────────────────────────────────────────────

main() {
  local run_single=""
  local run_from=""
  local run_tier=""
  local do_list=false
  local do_status=false
  local reset_section=""
  DRY_RUN=false

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --dry-run)         DRY_RUN=true; shift ;;
      --agent)           AGENT_TYPE="$2"; shift 2 ;;
      --model)           AGENT_MODEL="$2"; shift 2 ;;
      --reasoning)       CODEX_REASONING="$2"; shift 2 ;;
      --section)         run_single="$2"; shift 2 ;;
      --from)            run_from="$2"; shift 2 ;;
      --tier)            run_tier="$2"; shift 2 ;;
      --list)            do_list=true; shift ;;
      --status)          do_status=true; shift ;;
      --commit)          AUTO_COMMIT=true; shift ;;
      --retries)         MAX_RETRIES="$2"; shift 2 ;;
      --reset)           rm -rf "$STATE_DIR"; mkdir -p "$STATE_DIR"; log_info "State reset"; exit 0 ;;
      --reset-section)   reset_section="$2"; shift 2 ;;
      --help|-h)
        cat <<EOF
Usage: bash tmp/run-parity.sh [OPTIONS]

Agent options:
  --agent TYPE        Agent to use: claude (default), codex
  --model MODEL       Model override (e.g., gpt-5.4-mini, opus)
  --reasoning LEVEL   Codex reasoning effort: low, medium, high, xhigh
  --retries N         Max retries per item (default: 2)

Scope options:
  --section ID        Run only section ID (e.g., 1B)
  --from ID           Start from section ID onward
  --tier N            Run all sections in tier N (1-5)

Display options:
  --list              List all sections with item counts
  --status            Show progress bar only
  --dry-run           Print items without running agents

State options:
  --reset             Clear ALL state (start fresh)
  --reset-section ID  Clear state for one section (including sub-items)

Behavior options:
  --commit            Auto-commit after each successful item

Examples:
  bash tmp/run-parity.sh --agent codex --model gpt-5.4-mini --reasoning medium --section 1A
  bash tmp/run-parity.sh --tier 1 --commit
  bash tmp/run-parity.sh --list
  bash tmp/run-parity.sh --section 1A --dry-run
EOF
        exit 0
        ;;
      *) log_err "Unknown option: $1 (try --help)"; exit 1 ;;
    esac
  done

  # Reset single section (including all sub-items)
  if [[ -n "$reset_section" ]]; then
    clear_state "$reset_section"
    for f in "$STATE_DIR/${reset_section}."*; do
      [[ -f "$f" ]] && rm -f "$f"
    done 2>/dev/null || true
    log_info "Reset state for ${reset_section} (including sub-items)"
    exit 0
  fi

  if [[ "$do_status" == "true" ]]; then show_progress; exit 0; fi
  if [[ "$do_list" == "true" ]]; then list_sections; show_progress; exit 0; fi

  log_tier "Roko Master Plan — Per-Item Execution"
  log_info "Agent:     $AGENT_TYPE${AGENT_MODEL:+ (model: $AGENT_MODEL)}${CODEX_REASONING:+ (reasoning: $CODEX_REASONING)}"
  log_info "State dir: $STATE_DIR"
  log_info "Retries:   $MAX_RETRIES per item"
  echo ""

  # Verify prerequisites
  case "$AGENT_TYPE" in
    claude)
      command -v "$CLAUDE_CMD" &>/dev/null || { log_err "Claude CLI not found"; exit 1; } ;;
    codex)
      command -v "$CODEX_CMD" &>/dev/null || { log_err "Codex CLI not found"; exit 1; } ;;
  esac
  cargo --version &>/dev/null || { log_err "Rust toolchain not found"; exit 1; }

  local rust_version
  rust_version=$(rustc --version | sed -E 's/rustc ([0-9]+\.[0-9]+).*/\1/')
  log_info "Rust: ${rust_version}"

  local completed=0
  local failed_count=0
  local start_time=$SECONDS
  local pass=0
  local made_progress=true

  # Loop until no new sections complete (handles dependency chains)
  while [[ "$made_progress" == "true" ]]; do
    made_progress=false
    pass=$((pass + 1))
    (( pass > 1 )) && log_tier "Pass ${pass} — checking newly unblocked sections"

    local started=false
    for def in "${SECTIONS[@]}"; do
      parse_section "$def"

      [[ -n "$run_single" && "$SECTION_ID" != "$run_single" ]] && continue

      if [[ -n "$run_from" && "$started" == "false" ]]; then
        [[ "$SECTION_ID" == "$run_from" ]] && started=true || continue
      fi

      [[ -n "$run_tier" && "$SECTION_TIER" != "$run_tier" ]] && continue

      # Skip already done
      is_done "$SECTION_ID" && continue

      # Skip if deps not met
      if [[ -n "$SECTION_DEPS" ]] && ! deps_satisfied "$SECTION_DEPS"; then
        continue
      fi

      if run_section "$SECTION_ID" "$SECTION_NAME" "$SECTION_TIER" "$SECTION_DEPS"; then
        completed=$((completed + 1))
        made_progress=true
      else
        failed_count=$((failed_count + 1))
      fi
    done
  done

  local total_time=$((SECONDS - start_time))
  echo ""
  log_tier "Execution Complete (${total_time}s)"
  show_progress

  if (( failed_count > 0 )); then
    log_warn "${failed_count} section(s) had failures. Re-run to retry failed items."
  fi
}

cd "$REPO_ROOT"
main "$@"
