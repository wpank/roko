#!/usr/bin/env bash

set -uo pipefail
# NOTE: no `set -e` — background jobs + errexit = silent death.
# All error handling is explicit via return codes.
IFS=$'\n\t'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ---------------------------------------------------------------------------
# Cleanup trap — kill all children on Ctrl-C / exit
# ---------------------------------------------------------------------------
CHILD_PIDS=()

cleanup() {
  echo
  echo "[WARN]  runner     Caught signal — killing all children..." >&2

  # 1. Kill tracked background PIDs (the spawn subshells)
  local pid
  for pid in "${CHILD_PIDS[@]}"; do
    kill -TERM "$pid" 2>/dev/null || true
  done

  # 2. Kill codex exec processes spawned by us (match on worktree paths)
  local wt_root="${WORKTREE_ROOT:-/Users/will/dev/nunchi/roko/roko/.roko/worktrees}"
  pkill -TERM -f "codex exec.*${wt_root}" 2>/dev/null || true
  pkill -TERM -f "roko-migration-agent-" 2>/dev/null || true

  sleep 2

  # 3. Force kill stragglers
  for pid in "${CHILD_PIDS[@]}"; do
    kill -9 "$pid" 2>/dev/null || true
  done
  pkill -9 -f "codex exec.*${wt_root}" 2>/dev/null || true
  pkill -9 -f "roko-migration-agent-" 2>/dev/null || true

  echo "[WARN]  runner     Cleanup complete. Resume with: --continue ${RUN_ID:-last}" >&2
  exit 130
}

trap cleanup INT TERM HUP

source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/spawn.sh"
source "$SCRIPT_DIR/lib/review.sh"
source "$SCRIPT_DIR/lib/verify.sh"
source "$SCRIPT_DIR/lib/merge.sh"
source "$SCRIPT_DIR/lib/parallel.sh"
source "$SCRIPT_DIR/lib/state.sh"

: "${MR_MODEL:=gpt-5.4}"
: "${MR_REASONING:=high}"
: "${MR_TIMEOUT:=7200}"
: "${MR_MAX_RETRIES:=2}"
: "${MR_BASE_REF:=HEAD}"
: "${MR_MAX_BATCHES:=0}"              # 0 = unlimited per run
: "${MR_MERGE_INTERVAL:=5}"           # Merge to source every N successful batches
: "${MR_REVIEW_INTERVAL:=3}"          # Codex review every N batches
: "${MR_CLEANUP_INTERVAL:=5}"         # cargo clean equivalent every N batches
: "${MR_SOURCE_BRANCH:=}"             # Auto-detected if empty
: "${NUM_AGENTS:=4}"                   # Parallel agents (1-4)
: "${MB_TARGET_SIZE:=20}"              # Tasks per mega-batch target

DRY_RUN=0
FORCE=0
VERIFY_ONLY=0
LIST_ONLY=0
RESET_RUN=""
CONTINUE_RUN=""
MERGE_NOW=0
PARALLEL_MODE=1                        # v2 default: parallel
SELECTED_BATCHES=()
SELECTED_GROUPS=()
SELECTED_MBS=()
RUN_MODE="parallel"                    # parallel | sequential

print_usage() {
  cat <<'EOF'
run.sh — parallel Claude Opus 4.6 runner for unified migration (v2)

Usage:
  bash tmp/unified-migration-runner/run.sh                     # 4-agent parallel (default)
  bash tmp/unified-migration-runner/run.sh --agents 1          # sequential mode
  bash tmp/unified-migration-runner/run.sh --list              # show mega-batch schedule
  bash tmp/unified-migration-runner/run.sh --dry-run           # preview full schedule
  bash tmp/unified-migration-runner/run.sh --dry-run --only MB01  # preview one mega-batch
  bash tmp/unified-migration-runner/run.sh --continue last     # resume from state.json
  bash tmp/unified-migration-runner/run.sh --only MB01,MB02    # run specific mega-batches
  bash tmp/unified-migration-runner/run.sh --group phase1      # run all phase1 MBs

Options:
  --agents N          Number of parallel agents (1-4, default: 4)
  --only LIST         Comma-separated MB ids (MB01-MB99) or batch ids (M001-M999)
  --group LIST        Comma-separated phase groups: phase0, phase1, phase2, phase3
  --reset RUN         Kill processes, reset worktrees, clear state for a run ('last' ok)
  --continue RUN      Continue a prior run id, or 'last'
  --dry-run           Show what would run; no Claude spawn
  --force             Re-run even successful batches
  --verify-only       Skip Claude, only run verify gates
  --list              Show mega-batch schedule + exit
  --merge-now         Merge current worktrees to source, then exit
  --model MODEL       Override model (default: claude-opus-4-6)
  --timeout SECONDS   Per-batch timeout (default: 7200 = 2 hours)
  --retries N         Automatic retries per batch (default: 2)
  --base-ref REF      Base git ref for worktrees (default: HEAD)
  --max-batches N     Hard cap on mega-batches per run (default: 0 = unlimited)
  --merge-interval N  Sync to source every N mega-batches (default: 5)
  --review-interval N Codex review every N mega-batches (default: 3)
  --source-branch BR  Branch to merge into (default: auto-detect)
  --mb-size N         Tasks per mega-batch target (default: 20)

Environment overrides (all optional):
  MR_MODEL, MR_TIMEOUT, MR_MAX_RETRIES, MR_BASE_REF, MR_MAX_BATCHES,
  MR_MERGE_INTERVAL, MR_REVIEW_INTERVAL, MR_CLEANUP_INTERVAL,
  MR_SOURCE_BRANCH, NUM_AGENTS, MB_TARGET_SIZE, NO_COLOR
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --agents) NUM_AGENTS="$2"; shift 2 ;;
    --agents=*) NUM_AGENTS="${1#*=}"; shift ;;
    --only) IFS=',' read -r -a SELECTED_MBS <<< "$2"; shift 2 ;;
    --only=*) IFS=',' read -r -a SELECTED_MBS <<< "${1#*=}"; shift ;;
    --group) IFS=',' read -r -a SELECTED_GROUPS <<< "$2"; shift 2 ;;
    --group=*) IFS=',' read -r -a SELECTED_GROUPS <<< "${1#*=}"; shift ;;
    --reset) RESET_RUN="$2"; shift 2 ;;
    --reset=*) RESET_RUN="${1#*=}"; shift ;;
    --continue) CONTINUE_RUN="$2"; shift 2 ;;
    --continue=*) CONTINUE_RUN="${1#*=}"; shift ;;
    --dry-run) DRY_RUN=1; shift ;;
    --force) FORCE=1; shift ;;
    --verify-only) VERIFY_ONLY=1; shift ;;
    --list) LIST_ONLY=1; shift ;;
    --merge-now) MERGE_NOW=1; shift ;;
    --model) MR_MODEL="$2"; shift 2 ;;
    --model=*) MR_MODEL="${1#*=}"; shift ;;
    --timeout) MR_TIMEOUT="$2"; shift 2 ;;
    --timeout=*) MR_TIMEOUT="${1#*=}"; shift ;;
    --retries) MR_MAX_RETRIES="$2"; shift 2 ;;
    --retries=*) MR_MAX_RETRIES="${1#*=}"; shift ;;
    --base-ref) MR_BASE_REF="$2"; shift 2 ;;
    --base-ref=*) MR_BASE_REF="${1#*=}"; shift ;;
    --max-batches) MR_MAX_BATCHES="$2"; shift 2 ;;
    --max-batches=*) MR_MAX_BATCHES="${1#*=}"; shift ;;
    --merge-interval) MR_MERGE_INTERVAL="$2"; shift 2 ;;
    --merge-interval=*) MR_MERGE_INTERVAL="${1#*=}"; shift ;;
    --review-interval) MR_REVIEW_INTERVAL="$2"; shift 2 ;;
    --review-interval=*) MR_REVIEW_INTERVAL="${1#*=}"; shift ;;
    --source-branch) MR_SOURCE_BRANCH="$2"; shift 2 ;;
    --source-branch=*) MR_SOURCE_BRANCH="${1#*=}"; shift ;;
    --mb-size) MB_TARGET_SIZE="$2"; shift 2 ;;
    --mb-size=*) MB_TARGET_SIZE="${1#*=}"; shift ;;
    -h|--help) print_usage; exit 0 ;;
    *) log_err "cli" "Unknown argument: $1"; print_usage; exit 1 ;;
  esac
done

# Validate agent count
if (( NUM_AGENTS < 1 || NUM_AGENTS > 4 )); then
  log_err "cli" "--agents must be 1-4"
  exit 1
fi

# Sequential mode when agents=1
if (( NUM_AGENTS == 1 )); then
  PARALLEL_MODE=0
  RUN_MODE="sequential"
fi

if (( DRY_RUN == 1 )) && [[ -n "$CONTINUE_RUN" ]]; then
  log_err "cli" "--dry-run cannot be combined with --continue"
  exit 1
fi

# Auto-detect source branch
if [[ -z "$MR_SOURCE_BRANCH" ]]; then
  MR_SOURCE_BRANCH=$(git -C "$ROKO_ROOT" branch --show-current)
fi

# ---------------------------------------------------------------------------
# Batch selection helpers (v1 compat)
# ---------------------------------------------------------------------------

group_contains() {
  local needle="$1"
  shift
  local g
  for g in "$@"; do
    [[ "$g" == "$needle" ]] && return 0
  done
  return 1
}

select_batches() {
  local -a pool=()
  local batch group

  if [[ ${#SELECTED_BATCHES[@]} -gt 0 ]]; then
    local raw candidate found
    for raw in "${SELECTED_BATCHES[@]}"; do
      found=0
      for candidate in "${ALL_BATCHES[@]}"; do
        if [[ "$candidate" == "$raw" ]]; then
          pool+=("$candidate")
          found=1
          break
        fi
      done
      if (( found == 0 )); then
        log_err "cli" "Unknown batch: $raw"
        exit 1
      fi
    done
  elif [[ ${#SELECTED_GROUPS[@]} -gt 0 ]]; then
    for batch in "${ALL_BATCHES[@]}"; do
      group="$(batch_group "$batch")"
      if group_contains "$group" "${SELECTED_GROUPS[@]}"; then
        pool+=("$batch")
      fi
    done
  else
    pool=("${ALL_BATCHES[@]}")
  fi

  local candidate raw
  for candidate in "${ALL_BATCHES[@]}"; do
    for raw in "${pool[@]}"; do
      if [[ "$candidate" == "$raw" ]]; then
        echo "$candidate"
      fi
    done
  done
}

# ---------------------------------------------------------------------------
# MB selection — filter mega-batches by --only or --group
# ---------------------------------------------------------------------------

select_megabatches() {
  local -a pool=()
  local mb

  if [[ ${#SELECTED_MBS[@]} -gt 0 ]]; then
    local raw found
    for raw in "${SELECTED_MBS[@]}"; do
      # Check if it's an MB id (MB##) or an M### id
      if [[ "$raw" == MB* ]]; then
        found=0
        for mb in "${MEGA_BATCHES[@]}"; do
          if [[ "$mb" == "$raw" ]]; then
            pool+=("$mb")
            found=1
            break
          fi
        done
        if (( found == 0 )); then
          log_err "cli" "Unknown mega-batch: $raw"
          exit 1
        fi
      fi
    done
    # If M### style, find containing MBs
    if [[ ${#pool[@]} -eq 0 ]]; then
      for mb in "${MEGA_BATCHES[@]}"; do
        local mb_task_str="${MB_TASKS[$mb]}"
        for raw in "${SELECTED_MBS[@]}"; do
          if [[ " $mb_task_str " == *" $raw "* ]]; then
            local already=0
            for p in "${pool[@]}"; do
              [[ "$p" == "$mb" ]] && already=1 && break
            done
            (( already == 0 )) && pool+=("$mb")
          fi
        done
      done
    fi
  elif [[ ${#SELECTED_GROUPS[@]} -gt 0 ]]; then
    for mb in "${MEGA_BATCHES[@]}"; do
      local phase="${MB_PHASE[$mb]}"
      if group_contains "$phase" "${SELECTED_GROUPS[@]}"; then
        pool+=("$mb")
      fi
    done
  else
    pool=("${MEGA_BATCHES[@]}")
  fi

  printf '%s\n' "${pool[@]}"
}

# ---------------------------------------------------------------------------
# Run lifecycle
# ---------------------------------------------------------------------------

create_parallel_run() {
  RUN_ID="run-$(date +%Y%m%d-%H%M%S)"
  ensure_dir "$LOG_ROOT/$RUN_ID"
  ensure_dir "$LOG_ROOT/$RUN_ID/prompts"
  ensure_dir "$LOG_ROOT/$RUN_ID/archive"
  : > "$(run_status_file "$RUN_ID")"
  link_latest_run "$RUN_ID"

  cat > "$(run_manifest_file "$RUN_ID")" <<EOF
RUN_ID='$RUN_ID'
RUN_MODE='$RUN_MODE'
NUM_AGENTS='$NUM_AGENTS'
MR_MODEL='$MR_MODEL'
MR_TIMEOUT='$MR_TIMEOUT'
MR_MAX_RETRIES='$MR_MAX_RETRIES'
MR_BASE_REF='$MR_BASE_REF'
MR_MAX_BATCHES='$MR_MAX_BATCHES'
MR_MERGE_INTERVAL='$MR_MERGE_INTERVAL'
MR_REVIEW_INTERVAL='$MR_REVIEW_INTERVAL'
MR_SOURCE_BRANCH='$MR_SOURCE_BRANCH'
MB_TARGET_SIZE='$MB_TARGET_SIZE'
CREATED_AT='$(date -Iseconds)'
EOF
}

create_dry_run() {
  RUN_ID="dry-run-$(date +%Y%m%d-%H%M%S)"
  ensure_dir "$LOG_ROOT/$RUN_ID"
  ensure_dir "$LOG_ROOT/$RUN_ID/prompts"
  : > "$(run_status_file "$RUN_ID")"
  link_latest_run "$RUN_ID"
  cat > "$(run_manifest_file "$RUN_ID")" <<EOF
RUN_ID='$RUN_ID'
RUN_MODE='dry-run'
NUM_AGENTS='$NUM_AGENTS'
MR_MODEL='$MR_MODEL'
CREATED_AT='$(date -Iseconds)'
EOF
}

load_run() {
  if [[ "$CONTINUE_RUN" == "last" ]]; then
    CONTINUE_RUN="$(latest_run_id || true)"
  fi
  if [[ -z "$CONTINUE_RUN" ]]; then
    log_err "cli" "No prior run available to continue"
    exit 1
  fi
  local manifest
  manifest="$(run_manifest_file "$CONTINUE_RUN")"
  if [[ ! -f "$manifest" ]]; then
    log_err "cli" "Missing manifest for run: $CONTINUE_RUN"
    exit 1
  fi
  # shellcheck disable=SC1090
  source "$manifest"
  RUN_ID="$CONTINUE_RUN"
  link_latest_run "$RUN_ID"
}

# ---------------------------------------------------------------------------
# Status helpers (v1 compat)
# ---------------------------------------------------------------------------

batch_status() {
  local batch="$1"
  local result_file
  result_file="$(run_result_file "$RUN_ID" "$batch")"
  [[ -f "$result_file" ]] && cat "$result_file" || true
}

deps_satisfied() {
  local dep status
  local -a deps=()
  IFS=' ' read -r -a deps <<< "$(batch_deps "$1")"
  for dep in "${deps[@]}"; do
    [[ -z "$dep" ]] && continue
    status="$(batch_status "$dep")"
    success_status "$status" || return 1
  done
  return 0
}

deps_terminal_failure() {
  local dep status
  local -a deps=()
  IFS=' ' read -r -a deps <<< "$(batch_deps "$1")"
  for dep in "${deps[@]}"; do
    [[ -z "$dep" ]] && continue
    status="$(batch_status "$dep")"
    terminal_failure_status "$status" && return 0
  done
  return 1
}

# ---------------------------------------------------------------------------
# Dry run — preview schedule
# ---------------------------------------------------------------------------

dry_run_parallel() {
  local -a selected_mbs=()
  mapfile -t selected_mbs < <(select_megabatches)

  log_header "DRY RUN — $NUM_AGENTS-AGENT PARALLEL SCHEDULE"

  printf '  Model:           %s\n' "$MR_MODEL"
  printf '  Agents:          %s\n' "$NUM_AGENTS"
  printf '  Mega-batches:    %s\n' "${#selected_mbs[@]}"
  printf '  Total tasks:     %s\n' "${#ALL_BATCHES[@]}"
  printf '  MB target size:  %s tasks\n' "$MB_TARGET_SIZE"
  printf '  Source branch:   %s\n' "$MR_SOURCE_BRANCH"
  echo

  local mb tasks agents phase task_count
  local sync_num=0
  for mb in "${selected_mbs[@]}"; do
    tasks="${MB_TASKS[$mb]}"
    agents="${MB_AGENTS[$mb]}"
    phase="${MB_PHASE[$mb]}"
    if [[ -z "$tasks" ]]; then
      task_count=0
    else
      task_count=$(echo "$tasks" | wc -w | tr -d ' ')
    fi

    printf '%s=== %s (%s) — %d tasks, agents: %s ===%s\n' \
      "$C_BOLD" "$mb" "$phase" "$task_count" "$agents" "$C_RESET"

    # Show per-agent breakdown
    local -a agents_arr2=() tasks_arr2=()
    IFS=' ' read -ra agents_arr2 <<< "$agents"
    IFS=' ' read -ra tasks_arr2 <<< "$tasks"
    local agent
    for agent in "${agents_arr2[@]}"; do
      local -a atasks=()
      local t
      for t in "${tasks_arr2[@]}"; do
        if [[ "$(agent_for_batch "$t")" == "$agent" ]]; then
          atasks+=("$t")
        fi
      done
      if [[ ${#atasks[@]} -gt 0 ]]; then
        printf '  Agent %s: %s\n' "$agent" "${atasks[*]}"

        # Show prompt preview for --dry-run --only
        if (( DRY_RUN == 1 )); then
          local prompt_file
          prompt_file=$(compose_megabatch_prompt "$mb" "$agent" "$RUN_ID" 2>/dev/null || true)
          if [[ -n "$prompt_file" && -f "$prompt_file" ]]; then
            printf '    Prompt: %s (%s bytes)\n' "$prompt_file" "$(wc -c < "$prompt_file" | tr -d ' ')"
          fi
        fi
      fi
    done

    # Show sync point after every MR_MERGE_INTERVAL MBs
    sync_num=$((sync_num + 1))
    if (( sync_num % MR_MERGE_INTERVAL == 0 )); then
      printf '  %s>> SYNC-%d <<\n%s' "$C_CYAN" "$((sync_num / MR_MERGE_INTERVAL))" "$C_RESET"
    fi
    echo
  done
}

# ---------------------------------------------------------------------------
# Print summary
# ---------------------------------------------------------------------------

print_parallel_summary() {
  log_header "RUN SUMMARY ($RUN_ID)"

  if command -v jq >/dev/null 2>&1; then
    print_dashboard "$RUN_ID"
  else
    printf '  run_id=%s\n' "$RUN_ID"
    printf '  mode=%s\n' "$RUN_MODE"
    printf '  agents=%s\n' "$NUM_AGENTS"
    printf '  source_branch=%s\n' "$MR_SOURCE_BRANCH"
    printf '  logs=%s\n' "$LOG_ROOT/$RUN_ID"
  fi
}

# Update STATE.md with current progress
update_state_file() {
  cat > "$MR_ROOT/STATE.md" <<EOF
# Runner State (v2 Parallel)

| Field | Value |
|---|---|
| Run ID | \`$RUN_ID\` |
| Mode | \`$RUN_MODE\` |
| Agents | \`$NUM_AGENTS\` |
| Source branch | \`$MR_SOURCE_BRANCH\` |
| Model | \`$MR_MODEL\` |
| Last updated | $(date -Iseconds) |

## Resume

\`\`\`bash
bash tmp/unified-migration-runner/run.sh --continue $RUN_ID
# or
bash tmp/unified-migration-runner/run.sh --continue last
\`\`\`
EOF
}

# ---------------------------------------------------------------------------
# v1 sequential loop (backward compat with --agents 1)
# ---------------------------------------------------------------------------

resume_preserved_batch() {
  local batch="$1"
  local current_batch result
  [[ -n "$CONTINUE_RUN" ]] || return 1
  current_batch="$(current_batch_name "$RUN_ID" 2>/dev/null || true)"
  [[ "$current_batch" == "$batch" ]] || return 1
  result="$(batch_status "$batch")"
  success_status "$result" && return 1
  worktree_dirty "$WORKTREE"
}

run_sequential_loop() {
  local WORKTREE BRANCH
  WORKTREE="$WORKTREE_ROOT/migration-$RUN_ID"
  BRANCH="claude/migration-$RUN_ID"

  if [[ -z "$CONTINUE_RUN" ]]; then
    git -C "$ROKO_ROOT" worktree add -b "$BRANCH" "$WORKTREE" "$MR_BASE_REF" >/dev/null
    log_info "runner" "Created worktree $WORKTREE"
  fi

  mapfile -t SELECTED < <(select_batches)

  local batch_failed=0 processed=0 success_count=0
  local review_batches=()

  for batch in "${SELECTED[@]}"; do
    if (( MR_MAX_BATCHES > 0 )) && (( processed >= MR_MAX_BATCHES )); then
      log_warn "runner" "Reached MR_MAX_BATCHES=$MR_MAX_BATCHES; stopping"
      break
    fi

    if deps_satisfied "$batch"; then
      processed=$((processed + 1))

      local result_file log_file failure_file
      result_file="$(run_result_file "$RUN_ID" "$batch")"
      log_file="$(run_log_file "$RUN_ID" "$batch")"
      failure_file="$(run_failure_file "$RUN_ID" "$batch")"

      local existing
      existing="$(batch_status "$batch")"
      if [[ -n "$existing" ]] && success_status "$existing" && (( FORCE == 0 )); then
        log_info "$batch" "Already successful; skipping"
        continue
      fi

      : > "$failure_file"
      local attempt success=0
      for attempt in $(seq 1 "$MR_MAX_RETRIES"); do
        set_current_batch "$RUN_ID" "$batch" "$attempt"
        log_header "$batch ATTEMPT $attempt/$MR_MAX_RETRIES"

        if spawn_batch "$batch" "$RUN_ID" "$WORKTREE" "$attempt" "$failure_file"; then
          if verify_batch "$batch" "$RUN_ID" "$WORKTREE" "$attempt"; then
            commit_batch_if_needed "$batch" "$WORKTREE" "$RUN_ID" "$attempt" || true
            echo "success" > "$result_file"
            success=1
            break
          fi
          echo "verify_failed" > "$result_file"
        else
          echo "spawn_failed" > "$result_file"
        fi
      done

      if (( success == 1 )); then
        success_count=$((success_count + 1))
        review_batches+=("$batch")
        clear_current_batch "$RUN_ID"

        if should_review "$success_count"; then
          run_review_pass "$RUN_ID" "$WORKTREE" "${review_batches[@]}"
          review_batches=()
        fi
        if should_merge "$success_count"; then
          periodic_merge "$RUN_ID" "$WORKTREE" "$MR_SOURCE_BRANCH" "$batch" || {
            batch_failed=1; break
          }
        fi
      else
        batch_failed=1
      fi
    elif deps_terminal_failure "$batch"; then
      log_warn "$batch" "Blocked by failed dependency"
      echo "blocked" > "$(run_result_file "$RUN_ID" "$batch")"
    else
      log_warn "$batch" "Dependencies not yet satisfied"
    fi

    update_state_file
  done

  # Final review + merge
  if (( DRY_RUN == 0 && success_count > 0 )); then
    if [[ ${#review_batches[@]} -gt 0 ]]; then
      run_review_pass "$RUN_ID" "$WORKTREE" "${review_batches[@]}"
    fi
    if (( batch_failed == 0 )); then
      periodic_merge "$RUN_ID" "$WORKTREE" "$MR_SOURCE_BRANCH" "final" || true
    fi
  fi

  return "$batch_failed"
}

# ---------------------------------------------------------------------------
# v2 parallel loop — main execution path
# ---------------------------------------------------------------------------

run_parallel_loop() {
  # Create or reattach agent worktrees
  if [[ -n "$CONTINUE_RUN" ]]; then
    reattach_agent_worktrees "$RUN_ID" "$NUM_AGENTS"
  else
    create_agent_worktrees "$RUN_ID" "$MR_BASE_REF" "$NUM_AGENTS"
  fi

  # Warm caches
  if (( DRY_RUN == 0 )); then
    warm_cache "$NUM_AGENTS"
  fi

  # Select mega-batches to run
  local -a selected_mbs=()
  mapfile -t selected_mbs < <(select_megabatches)

  # Initialize or reload state
  local sf
  sf="$(state_file "$RUN_ID")"
  if [[ -n "$CONTINUE_RUN" && -f "$sf" ]]; then
    log_info "runner" "Resuming from existing state.json"
  else
    init_state "$RUN_ID" "$NUM_AGENTS" "${#ALL_BATCHES[@]}" "${#selected_mbs[@]}"
  fi
  update_phase_counts "$RUN_ID"

  local mb_failed=0 processed=0 sync_num=0

  log_info "runner" "Starting parallel execution: ${#selected_mbs[@]} mega-batches, $NUM_AGENTS agents"

  for mb in "${selected_mbs[@]}"; do
    if (( MR_MAX_BATCHES > 0 )) && (( processed >= MR_MAX_BATCHES )); then
      log_warn "runner" "Reached MR_MAX_BATCHES=$MR_MAX_BATCHES; stopping"
      break
    fi

    local tasks="${MB_TASKS[$mb]}"
    local agents="${MB_AGENTS[$mb]}"
    local phase="${MB_PHASE[$mb]}"

    # Skip already-completed MBs on --continue (unless --force)
    if [[ -n "$CONTINUE_RUN" ]] && (( FORCE == 0 )); then
      local sf
      sf="$(state_file "$RUN_ID")"
      if [[ -f "$sf" ]] && command -v jq >/dev/null 2>&1; then
        local mb_prev_status
        mb_prev_status=$(jq -r --arg m "$mb" '.megabatches[$m].status // ""' "$sf")
        if [[ "$mb_prev_status" == "completed" || "$mb_prev_status" == "verified" ]]; then
          log_info "$mb" "Already completed in prior run; skipping"
          processed=$((processed + 1))
          continue
        fi
      fi
    fi

    # Skip fixup MB if no failures
    if [[ "$phase" == "fixup" ]] && (( mb_failed == 0 )); then
      log_info "$mb" "Fixup MB skipped (no failures)"
      update_megabatch_state "$RUN_ID" "$mb" "skipped" "" "" "0"
      continue
    fi

    # Skip empty MBs
    if [[ -z "$tasks" && "$phase" != "fixup" ]]; then
      log_info "$mb" "No tasks, skipping"
      update_megabatch_state "$RUN_ID" "$mb" "skipped" "" "" "0"
      continue
    fi

    local mb_start_ts
    mb_start_ts=$(date +%s)

    log_header "MEGA-BATCH $mb ($phase)"
    update_megabatch_state "$RUN_ID" "$mb" "running" "$agents" "" "0"

    # Dispatch to all assigned agents in parallel
    local -a pids=()
    local -a dispatched_agents=()
    local -a agents_arr=() tasks_arr=()
    IFS=' ' read -ra agents_arr <<< "$agents"
    IFS=' ' read -ra tasks_arr <<< "$tasks"
    local agent

    for agent in "${agents_arr[@]}"; do
      # Build task list for this agent in this MB
      local -a agent_tasks=()
      local task
      for task in "${tasks_arr[@]}"; do
        if [[ "$(agent_for_batch "$task")" == "$agent" ]]; then
          agent_tasks+=("$task")
        fi
      done

      if [[ ${#agent_tasks[@]} -eq 0 ]]; then
        continue
      fi

      local wt="${AGENT_WORKTREES[$agent]:-}"
      local target="${AGENT_TARGET_DIRS[$agent]:-}"
      if [[ -z "$wt" || -z "$target" ]]; then
        log_warn "$mb" "Agent $agent: no worktree (agents=$NUM_AGENTS), skipping"
        continue
      fi

      log_info "$mb" "Agent $agent: ${#agent_tasks[@]} tasks (${agent_tasks[*]})"

      if (( DRY_RUN == 0 )); then
        spawn_megabatch "$mb" "$agent" "$RUN_ID" "$wt" "$target" "${agent_tasks[@]}" &
        pids+=($!)
        CHILD_PIDS+=($!)
        dispatched_agents+=("$agent")
      else
        # Dry run: just compose the prompt
        compose_megabatch_prompt_for_agent "$mb" "$agent" "$RUN_ID" "${agent_tasks[@]}" >/dev/null
        log_info "$mb" "Agent $agent: [DRY RUN] prompt composed"
      fi
    done

    if (( DRY_RUN == 1 )); then
      update_megabatch_state "$RUN_ID" "$mb" "dry_run" "$agents" "" "0"
      processed=$((processed + 1))
      continue
    fi

    # Wait for all agents
    local i exit_code agent_failed=0
    for i in "${!pids[@]}"; do
      exit_code=0
      wait "${pids[$i]}" || exit_code=$?
      if [[ "$exit_code" -ne 0 ]]; then
        log_err "$mb" "Agent ${dispatched_agents[$i]} failed (exit $exit_code)"
        agent_failed=$((agent_failed + 1))
      fi
    done

    # Commit + verify each agent's work
    for agent in "${dispatched_agents[@]}"; do
      local wt="${AGENT_WORKTREES[$agent]}"
      local target="${AGENT_TARGET_DIRS[$agent]}"

      # Commit
      rm -rf "$wt/.cargo-target" "$wt/target"
      git -C "$wt" add -A
      if ! git -C "$wt" diff --cached --quiet; then
        local task_list=""
        local t
        for t in "${tasks_arr[@]}"; do
          if [[ "$(agent_for_batch "$t")" == "$agent" ]]; then
            task_list="${task_list:+$task_list }$t"
          fi
        done
        git -C "$wt" commit -m "$(cat <<EOF
migration(${mb}): agent ${agent} — ${task_list}

Automated parallel migration via unified-migration-runner v2
Phase: ${phase}
EOF
)" >/dev/null
        log_ok "$mb" "Agent $agent committed"
      fi

      # Verify
      local -a atasks=()
      local t
      for t in "${tasks_arr[@]}"; do
        if [[ "$(agent_for_batch "$t")" == "$agent" ]]; then
          atasks+=("$t")
        fi
      done
      if [[ ${#atasks[@]} -gt 0 ]]; then
        if ! verify_megabatch "$mb" "$agent" "$RUN_ID" "$wt" "$target" "${atasks[@]}"; then
          agent_failed=$((agent_failed + 1))
        fi
      fi

      # Cleanup if needed
      cleanup_if_needed "$target"
    done

    local mb_end_ts mb_elapsed
    mb_end_ts=$(date +%s)
    mb_elapsed=$((mb_end_ts - mb_start_ts))

    if (( agent_failed > 0 )); then
      update_megabatch_state "$RUN_ID" "$mb" "failed" "$agents" "agent_failures=$agent_failed" "$mb_elapsed"
      increment_summary "$RUN_ID" "mbs_failed"
      mb_failed=$((mb_failed + 1))
    else
      update_megabatch_state "$RUN_ID" "$mb" "completed" "$agents" "verified" "$mb_elapsed"
      increment_summary "$RUN_ID" "mbs_completed"
    fi

    processed=$((processed + 1))

    # SYNC point — merge all agents to source, rebase
    sync_num=$((sync_num + 1))
    if (( sync_num % MR_MERGE_INTERVAL == 0 )); then
      local sync_label=$((sync_num / MR_MERGE_INTERVAL))
      if ! sync_all_agents "$RUN_ID" "$MR_SOURCE_BRANCH" "$sync_label" "$NUM_AGENTS" \
           "${AGENT_NAMES[@]:0:$NUM_AGENTS}"; then
        log_err "runner" "Sync failed; pausing execution"
        mb_failed=$((mb_failed + 1))
        break
      fi
      increment_summary "$RUN_ID" "syncs_completed"
    fi

    # Full workspace test every 5th MB
    if (( processed % 5 == 0 )) && (( processed > 0 )); then
      local test_agent="${AGENT_NAMES[0]}"
      local test_wt="${AGENT_WORKTREES[$test_agent]}"
      local test_target="${AGENT_TARGET_DIRS[$test_agent]}"
      verify_workspace_full "$RUN_ID" "$test_wt" "$test_target" "after-$mb" || true
    fi

    update_state_file
  done

  # Final sync
  if (( DRY_RUN == 0 && processed > 0 )); then
    log_info "runner" "Final sync to source"
    sync_all_agents "$RUN_ID" "$MR_SOURCE_BRANCH" "final" "$NUM_AGENTS" \
      "${AGENT_NAMES[@]:0:$NUM_AGENTS}" || true
    increment_summary "$RUN_ID" "syncs_completed"
  fi

  if (( mb_failed > 0 )); then
    finalize_state "$RUN_ID" "completed_with_failures"
  else
    finalize_state "$RUN_ID" "completed"
  fi

  return "$mb_failed"
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

preflight_check

# Handle --reset: nuke state for a run, reset worktrees to base
if [[ -n "$RESET_RUN" ]]; then
  if [[ "$RESET_RUN" == "last" ]]; then
    RESET_RUN="$(latest_run_id || true)"
  fi
  if [[ -z "$RESET_RUN" ]]; then
    log_err "cli" "No run found to reset"
    exit 1
  fi

  log_header "RESETTING RUN $RESET_RUN"

  # Kill any running processes for this run
  pkill -f "migration-${RESET_RUN}" 2>/dev/null || true
  pkill -f "roko-migration-agent-" 2>/dev/null || true
  sleep 1

  # Reset worktrees to HEAD
  for agent in A B C D; do
    wt="$WORKTREE_ROOT/migration-${RESET_RUN}-agent-${agent}"
    if [[ -d "$wt" ]]; then
      log_info "reset" "Resetting agent $agent worktree"
      git -C "$wt" reset --hard HEAD 2>/dev/null || true
      git -C "$wt" clean -fd 2>/dev/null || true
    fi
  done

  # Clear state files
  run_dir="$LOG_ROOT/$RESET_RUN"
  if [[ -d "$run_dir" ]]; then
    rm -f "$run_dir"/*.log "$run_dir"/*.result "$run_dir"/status.tsv "$run_dir"/state.json
    rm -rf "$run_dir"/prompts "$run_dir"/archive
    mkdir -p "$run_dir/prompts" "$run_dir/archive"
    : > "$run_dir/status.tsv"
    log_ok "reset" "Cleared logs and state for $RESET_RUN"
  fi

  log_ok "reset" "Run $RESET_RUN reset. Re-run with: --continue $RESET_RUN"
  exit 0
fi

# Also check jq for state management
if ! command -v jq >/dev/null 2>&1; then
  log_warn "preflight" "jq not found — state.json features will be disabled"
fi

if [[ ${#ALL_BATCHES[@]} -eq 0 ]]; then
  log_warn "runner" "No batches registered yet."
  log_warn "runner" "Run the INGEST-AND-AUDIT-PROMPT.md for each docs/ folder to populate batches."
  exit 0
fi

# Compute mega-batches
compute_megabatches

if (( LIST_ONLY == 1 )); then
  list_megabatches
  exit 0
fi

# Create or resume run
if [[ -n "$CONTINUE_RUN" ]]; then
  load_run
  log_info "runner" "Continuing run $RUN_ID"
else
  if (( DRY_RUN == 1 )); then
    create_dry_run
  else
    create_parallel_run
  fi
  log_info "runner" "Created $RUN_MODE run $RUN_ID"
fi

# Handle --merge-now
if (( MERGE_NOW == 1 )); then
  if (( PARALLEL_MODE == 1 )); then
    create_agent_worktrees "$RUN_ID" "$MR_BASE_REF" "$NUM_AGENTS"
    sync_all_agents "$RUN_ID" "$MR_SOURCE_BRANCH" "manual" "$NUM_AGENTS" \
      "${AGENT_NAMES[@]:0:$NUM_AGENTS}"
  fi
  exit $?
fi

# Dry run preview
if (( DRY_RUN == 1 )); then
  dry_run_parallel
  exit 0
fi

log_info "runner" "Mode: $RUN_MODE ($NUM_AGENTS agents)"
log_info "runner" "Model: $MR_MODEL (timeout: $MR_TIMEOUT s, retries: $MR_MAX_RETRIES)"
log_info "runner" "Merge interval: $MR_MERGE_INTERVAL  Review interval: $MR_REVIEW_INTERVAL"
log_info "runner" "Max batches per run: $MR_MAX_BATCHES (0 = unlimited)"
log_info "runner" "Source branch: $MR_SOURCE_BRANCH"
log_info "runner" "Mega-batches: ${#MEGA_BATCHES[@]} (${#ALL_BATCHES[@]} tasks, ~$MB_TARGET_SIZE/MB)"

# Dispatch
run_failed=0
if (( PARALLEL_MODE == 1 )); then
  run_parallel_loop || run_failed=1
else
  run_sequential_loop || run_failed=1
fi

print_parallel_summary
update_state_file
exit "$run_failed"
