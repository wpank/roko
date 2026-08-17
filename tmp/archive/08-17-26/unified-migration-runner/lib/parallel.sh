#!/usr/bin/env bash

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

# ---------------------------------------------------------------------------
# parallel.sh — 4-agent parallel execution engine
#
# Manages N worktrees (one per agent), warm cargo caches, sync points,
# dynamic mega-batch computation, and prompt composition.
# ---------------------------------------------------------------------------

: "${NUM_AGENTS:=4}"
: "${MB_TARGET_SIZE:=20}"

# Agent names (A=0, B=1, C=2, D=3)
AGENT_NAMES=("A" "B" "C" "D")

# Crate partitioning — each agent owns writes to these crates
declare -A AGENT_CRATE_PARTITION
AGENT_CRATE_PARTITION[A]="roko-core roko-primitives roko-fs roko-std"
AGENT_CRATE_PARTITION[B]="roko-gate roko-compose roko-learn roko-neuro"
AGENT_CRATE_PARTITION[C]="roko-orchestrator roko-runtime roko-conductor roko-dreams roko-daimon"
AGENT_CRATE_PARTITION[D]="roko-cli roko-serve roko-agent roko-agent-server roko-mcp-code roko-mcp-github roko-mcp-slack roko-mcp-scripts roko-mcp-stdio roko-chain roko-index"

# Per-agent worktree paths and branches (populated by create_agent_worktrees)
declare -A AGENT_WORKTREES
declare -A AGENT_BRANCHES
declare -A AGENT_TARGET_DIRS

# ---------------------------------------------------------------------------
# Worktree management
# ---------------------------------------------------------------------------

create_agent_worktrees() {
  local run_id="$1"
  local base_ref="${2:-HEAD}"
  local n="${3:-$NUM_AGENTS}"

  local i agent wt branch
  for i in $(seq 0 $((n - 1))); do
    agent="${AGENT_NAMES[$i]}"
    wt="$WORKTREE_ROOT/migration-${run_id}-agent-${agent}"
    branch="claude/migration-${run_id}-agent-${agent}"

    log_info "parallel" "Creating worktree for agent $agent: $wt"
    git -C "$ROKO_ROOT" worktree add -b "$branch" "$wt" "$base_ref" >/dev/null 2>&1

    AGENT_WORKTREES[$agent]="$wt"
    AGENT_BRANCHES[$agent]="$branch"

    # Shared cargo target dir per agent (warm, persistent across MBs)
    local target_dir="${TMPDIR:-/tmp}/roko-migration-agent-${agent}"
    mkdir -p "$target_dir"
    AGENT_TARGET_DIRS[$agent]="$target_dir"
  done

  log_ok "parallel" "Created $n agent worktrees"
}

reattach_agent_worktrees() {
  local run_id="$1"
  local n="${2:-$NUM_AGENTS}"

  local i agent wt
  for i in $(seq 0 $((n - 1))); do
    agent="${AGENT_NAMES[$i]}"
    wt="$WORKTREE_ROOT/migration-${run_id}-agent-${agent}"

    if [[ ! -d "$wt" ]]; then
      log_warn "parallel" "Agent $agent worktree missing: $wt — recreating"
      local branch="claude/migration-${run_id}-agent-${agent}"
      # Try to reuse the branch if it exists
      if git -C "$ROKO_ROOT" rev-parse --verify "$branch" >/dev/null 2>&1; then
        git -C "$ROKO_ROOT" worktree add "$wt" "$branch" >/dev/null 2>&1
      else
        git -C "$ROKO_ROOT" worktree add -b "$branch" "$wt" HEAD >/dev/null 2>&1
      fi
    else
      log_info "parallel" "Reattaching agent $agent: $wt"
    fi

    AGENT_WORKTREES[$agent]="$wt"
    AGENT_BRANCHES[$agent]="claude/migration-${run_id}-agent-${agent}"

    local target_dir="${TMPDIR:-/tmp}/roko-migration-agent-${agent}"
    mkdir -p "$target_dir"
    AGENT_TARGET_DIRS[$agent]="$target_dir"
  done

  log_ok "parallel" "Reattached $n agent worktrees"
}

remove_agent_worktrees() {
  local agent wt
  for agent in "${AGENT_NAMES[@]}"; do
    wt="${AGENT_WORKTREES[$agent]:-}"
    [[ -z "$wt" || ! -d "$wt" ]] && continue
    log_info "parallel" "Removing worktree for agent $agent"
    git -C "$ROKO_ROOT" worktree remove --force "$wt" 2>/dev/null || true
  done
}

# ---------------------------------------------------------------------------
# Warm cache — cargo check once per agent at startup
# ---------------------------------------------------------------------------

warm_cache() {
  local n="${1:-$NUM_AGENTS}"
  local pids=()
  local agent wt target_dir

  log_header "WARMING CACHES ($n agents)"

  for i in $(seq 0 $((n - 1))); do
    agent="${AGENT_NAMES[$i]}"
    wt="${AGENT_WORKTREES[$agent]}"
    target_dir="${AGENT_TARGET_DIRS[$agent]}"

    log_info "warm" "Agent $agent: cargo check --workspace (background)"
    (
      cd "$wt" &&
      env CARGO_TARGET_DIR="$target_dir" cargo check --workspace >/dev/null 2>&1
    ) &
    pids+=($!)
  done

  local failed=0
  for pid in "${pids[@]}"; do
    if ! wait "$pid"; then
      failed=$((failed + 1))
    fi
  done

  if (( failed > 0 )); then
    log_warn "warm" "$failed agent(s) failed cache warmup (non-fatal)"
  else
    log_ok "warm" "All $n agents warmed"
  fi
}

# ---------------------------------------------------------------------------
# Agent assignment — which agent should run a given task
# ---------------------------------------------------------------------------

# Returns the primary agent for a batch based on its affected crates.
# Strategy: prefer non-kernel agents (B/C/D) when the task spans multiple crates.
# Only assign to A when the primary work is truly in roko-core/primitives/fs/std.
# NOTE: Uses read -ra to split on spaces regardless of IFS setting.
agent_for_batch() {
  local batch="$1"
  local -a crates_arr=()
  IFS=' ' read -ra crates_arr <<< "$(batch_crates "$batch")"

  # First pass: try agents B, C, D (skip A) — catches tasks where roko-core
  # is a secondary dependency but the real work is in another crate
  local agent crate owned
  local -a owned_arr=()
  for agent in B C D; do
    IFS=' ' read -ra owned_arr <<< "${AGENT_CRATE_PARTITION[$agent]}"
    for crate in "${crates_arr[@]}"; do
      for owned in "${owned_arr[@]}"; do
        if [[ "$crate" == "$owned" ]]; then
          echo "$agent"
          return 0
        fi
      done
    done
  done

  # Second pass: if no B/C/D crate matched, it's a kernel-only task → Agent A
  echo "A"
}

# ---------------------------------------------------------------------------
# Dynamic mega-batch computation
# ---------------------------------------------------------------------------

# Compute mega-batches from registered tasks at startup.
# Groups by phase + agent assignment, targets ~MB_TARGET_SIZE tasks per MB.
# Output: sets MEGA_BATCHES array and MB_TASKS / MB_AGENTS / MB_PHASE associative arrays.
declare -a MEGA_BATCHES=()
declare -A MB_TASKS    # MB_TASKS[MB01]="M001 M002 M003 ..."
declare -A MB_AGENTS   # MB_AGENTS[MB01]="A C"
declare -A MB_PHASE    # MB_PHASE[MB01]="phase0"

compute_megabatches() {
  local -a batches=("${ALL_BATCHES[@]}")
  local total=${#batches[@]}
  local mb_count=$(( (total + MB_TARGET_SIZE - 1) / MB_TARGET_SIZE ))

  # Minimum 2 MBs, plus 1 fixup MB
  (( mb_count < 2 )) && mb_count=2

  log_info "parallel" "Computing mega-batches: $total tasks → ~$mb_count MBs (target $MB_TARGET_SIZE/MB)"

  # Group tasks by phase first
  local -a phase0=() phase1=() phase2=() phase3=()
  local batch group
  for batch in "${batches[@]}"; do
    group="$(batch_group "$batch")"
    case "$group" in
      phase0) phase0+=("$batch") ;;
      phase1) phase1+=("$batch") ;;
      phase2) phase2+=("$batch") ;;
      phase3) phase3+=("$batch") ;;
      *)      phase3+=("$batch") ;;
    esac
  done

  local mb_num=0
  local -a current_batch_acc=()
  local -A current_agents_acc=()
  local current_phase=""

  flush_megabatch() {
    if [[ ${#current_batch_acc[@]} -eq 0 ]]; then
      return
    fi
    mb_num=$((mb_num + 1))
    local mb_id
    mb_id=$(printf "MB%02d" "$mb_num")
    MEGA_BATCHES+=("$mb_id")
    # Join with spaces using printf (IFS-safe)
    MB_TASKS[$mb_id]="$(printf '%s ' "${current_batch_acc[@]}" | sed 's/ $//')"
    MB_PHASE[$mb_id]="$current_phase"

    # Collect unique agents
    local agents_str=""
    local a
    for a in "${!current_agents_acc[@]}"; do
      agents_str="${agents_str:+$agents_str }$a"
    done
    MB_AGENTS[$mb_id]="$agents_str"

    current_batch_acc=()
    current_agents_acc=()
  }

  process_phase_tasks() {
    local phase_name="$1"
    shift
    local -a phase_tasks=("$@")

    current_phase="$phase_name"

    local task agent
    for task in "${phase_tasks[@]}"; do
      agent="$(agent_for_batch "$task")"
      current_batch_acc+=("$task")
      current_agents_acc[$agent]=1

      if (( ${#current_batch_acc[@]} >= MB_TARGET_SIZE )); then
        flush_megabatch
        current_phase="$phase_name"
      fi
    done

    # Flush remainder at phase boundary
    flush_megabatch
  }

  if (( ${#phase0[@]} > 0 )); then
    process_phase_tasks "phase0" "${phase0[@]}"
  fi
  if (( ${#phase1[@]} > 0 )); then
    process_phase_tasks "phase1" "${phase1[@]}"
  fi
  if (( ${#phase2[@]} > 0 )); then
    process_phase_tasks "phase2" "${phase2[@]}"
  fi
  if (( ${#phase3[@]} > 0 )); then
    process_phase_tasks "phase3" "${phase3[@]}"
  fi

  # Add fixup MB at the end
  mb_num=$((mb_num + 1))
  local fix_id
  fix_id=$(printf "MB%02d" "$mb_num")
  MEGA_BATCHES+=("$fix_id")
  MB_TASKS[$fix_id]=""
  MB_AGENTS[$fix_id]="A B C D"
  MB_PHASE[$fix_id]="fixup"

  log_ok "parallel" "Computed ${#MEGA_BATCHES[@]} mega-batches (including fixup)"
}

# ---------------------------------------------------------------------------
# List mega-batches
# ---------------------------------------------------------------------------

list_megabatches() {
  printf '%s%-6s %-8s %-10s %-8s %s%s\n' \
    "$C_BOLD" "MB" "PHASE" "AGENTS" "TASKS" "TASK IDS" "$C_RESET"

  local mb tasks agents phase task_count
  for mb in "${MEGA_BATCHES[@]}"; do
    tasks="${MB_TASKS[$mb]}"
    agents="${MB_AGENTS[$mb]}"
    phase="${MB_PHASE[$mb]}"
    if [[ -z "$tasks" ]]; then
      task_count=0
    else
      task_count=$(echo "$tasks" | wc -w | tr -d ' ')
    fi
    printf '%-6s %-8s %-10s %-8s %s\n' \
      "$mb" "$phase" "$agents" "$task_count" "$tasks"
  done
}

# ---------------------------------------------------------------------------
# Mega-batch prompt composition
# ---------------------------------------------------------------------------

compose_megabatch_prompt() {
  local mb="$1"
  local agent="$2"
  local run_id="$3"

  local tasks="${MB_TASKS[$mb]}"
  local phase="${MB_PHASE[$mb]}"
  local out
  out="$LOG_ROOT/$run_id/prompts/${mb}-agent-${agent}.prompt.md"
  ensure_dir "$(dirname "$out")"

  # Filter tasks to those assigned to this agent
  local -a all_tasks_arr=() agent_tasks=()
  IFS=' ' read -ra all_tasks_arr <<< "$tasks"
  local task
  for task in "${all_tasks_arr[@]}"; do
    if [[ "$(agent_for_batch "$task")" == "$agent" ]]; then
      agent_tasks+=("$task")
    fi
  done

  if [[ ${#agent_tasks[@]} -eq 0 ]]; then
    echo ""
    return 0
  fi

  {
    echo "# Mega-Batch $mb: ${phase} — Agent $agent"
    echo
    echo "Run: $run_id"
    echo "Agent: $agent (${AGENT_CRATE_PARTITION[$agent]})"
    echo "Tasks: ${agent_tasks[*]}"
    echo "Model: $MR_MODEL"
    echo

    # Source tracking
    local first_task="${agent_tasks[0]}"
    echo "Source: $(batch_phase_ref "$first_task")"
    echo

    # Context pack
    emit_shared_context_pack

    # Agent-specific crate context
    echo "## Agent $agent Crate Ownership"
    echo
    echo "You own writes to: ${AGENT_CRATE_PARTITION[$agent]}"
    echo "You may READ but NOT WRITE to crates owned by other agents."
    echo "If you need changes in another agent's crates, leave a TODO comment."
    echo

    # Execution rules
    echo "## Execution Rules"
    echo
    echo "1. Complete tasks in listed order (dependencies noted per task)"
    echo "2. Do NOT run cargo check/test — the runner handles verification externally"
    echo "3. Reference tmp/unified-migration/ and tmp/architecture/ for naming"
    echo "4. Commit changes after each task with message: migration(MXXX): <title>"
    echo "5. You are one of multiple parallel agents — stay within your crate scope"
    echo "6. Write unit tests for any new public functions"
    echo

    # Concatenate micro-task prompts
    local i=0 task prompt_file
    for task in "${agent_tasks[@]}"; do
      i=$((i + 1))
      echo "## Task $i/${#agent_tasks[@]}: $task — $(batch_title "$task")"
      echo
      echo "Phase ref: $(batch_phase_ref "$task")"
      echo "Dependencies: $(batch_deps "$task")"
      echo

      prompt_file="$(batch_prompt_file "$task")"
      if [[ -f "$prompt_file" ]]; then
        cat "$prompt_file"
      else
        echo "[WARNING: prompt file not found: $prompt_file]"
      fi
      echo
      echo "---"
      echo
    done
  } > "$out"

  echo "$out"
}

# run_megabatch: see spawn.sh spawn_megabatch() — single source of truth

# ---------------------------------------------------------------------------
# Commit all agent work in a mega-batch
# ---------------------------------------------------------------------------

commit_megabatch() {
  local mb="$1"
  local agent="$2"
  local run_id="$3"

  local wt="${AGENT_WORKTREES[$agent]}"
  local tasks="${MB_TASKS[$mb]}"

  # Filter to this agent's tasks
  local -a all_tasks_arr=() agent_tasks=()
  IFS=' ' read -ra all_tasks_arr <<< "$tasks"
  local task
  for task in "${all_tasks_arr[@]}"; do
    if [[ "$(agent_for_batch "$task")" == "$agent" ]]; then
      agent_tasks+=("$task")
    fi
  done

  [[ ${#agent_tasks[@]} -eq 0 ]] && return 0

  # Stage everything (exclude build artifacts)
  rm -rf "$wt/.cargo-target" "$wt/target"
  git -C "$wt" add -A

  if git -C "$wt" diff --cached --quiet; then
    log_info "$mb" "Agent $agent: no changes to commit"
    return 0
  fi

  local task_list="${agent_tasks[*]}"
  git -C "$wt" commit -m "$(cat <<EOF
migration(${mb}): agent ${agent} — ${task_list}

Automated parallel migration via unified-migration-runner v2
Phase: ${MB_PHASE[$mb]}
Tasks: ${task_list}
EOF
)" >/dev/null

  log_ok "$mb" "Agent $agent committed: $(git -C "$wt" log --oneline -1)"
}

# ---------------------------------------------------------------------------
# Sync all agents — sequential merge to source, then rebase all
# ---------------------------------------------------------------------------

sync_agents() {
  local run_id="$1"
  local source_branch="$2"
  local sync_label="$3"
  local n="${4:-$NUM_AGENTS}"

  log_header "SYNC-${sync_label} ($n agents → $source_branch)"

  local i agent wt branch
  local source_wt="$ROKO_ROOT"

  # Verify source branch is checked out and clean
  local current_branch
  current_branch=$(git -C "$source_wt" branch --show-current)
  if [[ "$current_branch" != "$source_branch" ]]; then
    log_err "sync" "Source repo on '$current_branch', expected '$source_branch'"
    return 1
  fi
  if ! git -C "$source_wt" diff --quiet 2>/dev/null || \
     ! git -C "$source_wt" diff --cached --quiet 2>/dev/null; then
    log_err "sync" "Source branch has uncommitted changes"
    return 1
  fi

  # Sequential merge: each agent merges to source, then all others rebase
  for i in $(seq 0 $((n - 1))); do
    agent="${AGENT_NAMES[$i]}"
    wt="${AGENT_WORKTREES[$agent]}"
    branch="${AGENT_BRANCHES[$agent]}"

    # Skip if agent has no new commits
    local source_sha agent_sha
    source_sha=$(git -C "$source_wt" rev-parse HEAD)
    agent_sha=$(git -C "$wt" rev-parse HEAD)
    if [[ "$source_sha" == "$agent_sha" ]]; then
      log_info "sync" "Agent $agent: no new commits, skipping"
      continue
    fi

    # Check if agent branch is ancestor of source (nothing to merge)
    if git -C "$source_wt" merge-base --is-ancestor "$agent_sha" "$source_sha" 2>/dev/null; then
      log_info "sync" "Agent $agent: already merged, skipping"
      continue
    fi

    # Merge agent → source
    log_info "sync" "Merging agent $agent ($branch) → $source_branch"
    if git -C "$source_wt" merge --ff-only "$branch" 2>/dev/null; then
      log_ok "sync" "Agent $agent: fast-forward merge"
    elif git -C "$source_wt" merge --no-ff "$branch" \
         -m "sync(${sync_label}): merge agent ${agent} (${run_id})" 2>/dev/null; then
      log_ok "sync" "Agent $agent: merge commit created"
    else
      git -C "$source_wt" merge --abort 2>/dev/null || true
      log_err "sync" "Agent $agent: CONFLICT — manual resolution needed"
      log_err "sync" "Branch preserved: $branch in $wt"
      return 1
    fi

    # Rebase all subsequent agents onto updated source
    local j other_agent other_wt
    for j in $(seq 0 $((n - 1))); do
      [[ "$j" -le "$i" ]] && continue
      other_agent="${AGENT_NAMES[$j]}"
      other_wt="${AGENT_WORKTREES[$other_agent]}"
      log_info "sync" "Rebasing agent $other_agent onto updated source"
      if ! git -C "$other_wt" rebase "$source_branch" 2>/dev/null; then
        git -C "$other_wt" rebase --abort 2>/dev/null || true
        log_err "sync" "Agent $other_agent: rebase conflict — resetting to source"
        git -C "$other_wt" reset --hard "$source_branch" 2>/dev/null || true
      fi
    done
  done

  # Final: rebase all agents onto source
  for i in $(seq 0 $((n - 1))); do
    agent="${AGENT_NAMES[$i]}"
    wt="${AGENT_WORKTREES[$agent]}"
    log_info "sync" "Final rebase: agent $agent"
    git -C "$wt" rebase "$source_branch" 2>/dev/null || {
      git -C "$wt" rebase --abort 2>/dev/null || true
      git -C "$wt" reset --hard "$source_branch" 2>/dev/null || true
    }
  done

  log_ok "sync" "SYNC-${sync_label} complete"
}

# ---------------------------------------------------------------------------
# Archive completed mega-batch artifacts
# ---------------------------------------------------------------------------

archive_megabatch() {
  local mb="$1"
  local run_id="$2"

  local archive_dir="$LOG_ROOT/$run_id/archive/$mb"
  ensure_dir "$archive_dir"

  # Move logs and prompts to archive
  local f
  for f in "$LOG_ROOT/$run_id/${mb}-"*.log "$LOG_ROOT/$run_id/prompts/${mb}-"*.prompt.md; do
    [[ -f "$f" ]] && mv "$f" "$archive_dir/" 2>/dev/null || true
  done

  log_info "$mb" "Archived to $archive_dir"
}

# ---------------------------------------------------------------------------
# Dispatch a mega-batch round (parallel across agents)
# ---------------------------------------------------------------------------

dispatch_megabatch_round() {
  local mb="$1"
  local run_id="$2"
  local n="${3:-$NUM_AGENTS}"

  local agents="${MB_AGENTS[$mb]}"
  local -a agents_arr=() pids=() dispatched_agents=()
  IFS=' ' read -ra agents_arr <<< "$agents"

  log_header "MEGA-BATCH $mb (${MB_PHASE[$mb]})"

  local agent
  for agent in "${agents_arr[@]}"; do
    # Only dispatch to agents we have worktrees for
    local idx
    for idx in $(seq 0 $((n - 1))); do
      if [[ "${AGENT_NAMES[$idx]}" == "$agent" ]]; then
        log_info "$mb" "Dispatching to agent $agent"
        run_megabatch "$mb" "$agent" "$run_id" &
        pids+=($!)
        dispatched_agents+=("$agent")
        break
      fi
    done
  done

  # Wait for all agents to complete
  local i exit_code failed=0
  for i in "${!pids[@]}"; do
    exit_code=0
    wait "${pids[$i]}" || exit_code=$?
    if [[ "$exit_code" -ne 0 ]]; then
      log_err "$mb" "Agent ${dispatched_agents[$i]} failed (exit $exit_code)"
      failed=$((failed + 1))
    fi
  done

  # Commit all agent work
  for agent in "${dispatched_agents[@]}"; do
    commit_megabatch "$mb" "$agent" "$run_id"
  done

  if (( failed > 0 )); then
    log_warn "$mb" "$failed agent(s) failed in this mega-batch"
    return 1
  fi

  return 0
}
