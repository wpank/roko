#!/usr/bin/env bash

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

# ---------------------------------------------------------------------------
# state.sh — JSON state management for parallel migration runner
#
# State file: logs/<run>/state.json
# Per-task: status, agent, timing, source refs, commit sha
# Per-MB: verify result, agents, timing
# Per-phase: counts
# Overall: summary + ETA
# ---------------------------------------------------------------------------

# Requires: jq (checked in preflight)

state_file() {
  echo "$LOG_ROOT/$1/state.json"
}

# Initialize state.json for a new run
init_state() {
  local run_id="$1"
  local num_agents="$2"
  local total_tasks="$3"
  local total_mbs="$4"

  local sf
  sf="$(state_file "$run_id")"
  ensure_dir "$(dirname "$sf")"

  cat > "$sf" <<EOF
{
  "run_id": "$run_id",
  "num_agents": $num_agents,
  "total_tasks": $total_tasks,
  "total_megabatches": $total_mbs,
  "started_at": "$(date -Iseconds)",
  "updated_at": "$(date -Iseconds)",
  "status": "running",
  "tasks": {},
  "megabatches": {},
  "phases": {
    "phase0": {"total": 0, "done": 0, "failed": 0},
    "phase1": {"total": 0, "done": 0, "failed": 0},
    "phase2": {"total": 0, "done": 0, "failed": 0},
    "phase3": {"total": 0, "done": 0, "failed": 0}
  },
  "summary": {
    "mbs_completed": 0,
    "mbs_failed": 0,
    "tasks_completed": 0,
    "tasks_failed": 0,
    "syncs_completed": 0,
    "reviews_completed": 0
  }
}
EOF
}

# Update a task's state
update_task_state() {
  local run_id="$1"
  local task_id="$2"
  local status="$3"
  local agent="${4:-}"
  local commit_sha="${5:-}"
  local note="${6:-}"

  local sf
  sf="$(state_file "$run_id")"
  [[ -f "$sf" ]] || return 0

  if ! command -v jq >/dev/null 2>&1; then
    return 0
  fi

  local tmp="${sf}.tmp"
  jq --arg tid "$task_id" \
     --arg st "$status" \
     --arg ag "$agent" \
     --arg sha "$commit_sha" \
     --arg note "$note" \
     --arg now "$(date -Iseconds)" \
     '.tasks[$tid] = {
        "status": $st,
        "agent": $ag,
        "commit_sha": $sha,
        "note": $note,
        "updated_at": $now
      } | .updated_at = $now' \
     "$sf" > "$tmp" && mv "$tmp" "$sf"
}

# Update a mega-batch's state
update_megabatch_state() {
  local run_id="$1"
  local mb="$2"
  local status="$3"
  local agents="${4:-}"
  local verify_result="${5:-}"
  local elapsed="${6:-0}"

  local sf
  sf="$(state_file "$run_id")"
  [[ -f "$sf" ]] || return 0

  if ! command -v jq >/dev/null 2>&1; then
    return 0
  fi

  local tmp="${sf}.tmp"
  jq --arg mb "$mb" \
     --arg st "$status" \
     --arg ag "$agents" \
     --arg vr "$verify_result" \
     --argjson el "$elapsed" \
     --arg now "$(date -Iseconds)" \
     '.megabatches[$mb] = {
        "status": $st,
        "agents": $ag,
        "verify_result": $vr,
        "elapsed_seconds": $el,
        "updated_at": $now
      } | .updated_at = $now' \
     "$sf" > "$tmp" && mv "$tmp" "$sf"
}

# Increment summary counters
increment_summary() {
  local run_id="$1"
  local field="$2"
  local amount="${3:-1}"

  local sf
  sf="$(state_file "$run_id")"
  [[ -f "$sf" ]] || return 0

  if ! command -v jq >/dev/null 2>&1; then
    return 0
  fi

  local tmp="${sf}.tmp"
  jq --arg f "$field" --argjson a "$amount" \
     '.summary[$f] = (.summary[$f] // 0) + $a | .updated_at = (now | todate)' \
     "$sf" > "$tmp" && mv "$tmp" "$sf"
}

# Update phase counts
update_phase_counts() {
  local run_id="$1"

  local sf
  sf="$(state_file "$run_id")"
  [[ -f "$sf" ]] || return 0

  if ! command -v jq >/dev/null 2>&1; then
    return 0
  fi

  # Recount from task states
  local phase batch group
  local -A phase_total phase_done phase_failed
  for phase in phase0 phase1 phase2 phase3; do
    phase_total[$phase]=0
    phase_done[$phase]=0
    phase_failed[$phase]=0
  done

  for batch in "${ALL_BATCHES[@]}"; do
    group="$(batch_group "$batch")"
    phase_total[$group]=$(( ${phase_total[$group]} + 1 ))
  done

  local tmp="${sf}.tmp"
  jq --argjson p0t "${phase_total[phase0]}" \
     --argjson p1t "${phase_total[phase1]}" \
     --argjson p2t "${phase_total[phase2]}" \
     --argjson p3t "${phase_total[phase3]}" \
     '.phases.phase0.total = $p0t |
      .phases.phase1.total = $p1t |
      .phases.phase2.total = $p2t |
      .phases.phase3.total = $p3t' \
     "$sf" > "$tmp" && mv "$tmp" "$sf"
}

# Compute ETA based on completed MBs and elapsed time
compute_eta() {
  local run_id="$1"

  local sf
  sf="$(state_file "$run_id")"
  [[ -f "$sf" ]] || return 0

  if ! command -v jq >/dev/null 2>&1; then
    echo "unknown"
    return 0
  fi

  local completed total started_at
  completed=$(jq -r '.summary.mbs_completed' "$sf")
  total=$(jq -r '.total_megabatches' "$sf")
  started_at=$(jq -r '.started_at' "$sf")

  if (( completed == 0 )); then
    echo "calculating..."
    return 0
  fi

  local now_ts start_ts elapsed remaining_mbs rate_per_mb eta_seconds
  now_ts=$(date +%s)
  start_ts=$(date -j -f "%Y-%m-%dT%H:%M:%S%z" "$started_at" +%s 2>/dev/null || date -d "$started_at" +%s 2>/dev/null || echo "$now_ts")
  elapsed=$((now_ts - start_ts))
  remaining_mbs=$((total - completed))
  rate_per_mb=$((elapsed / completed))
  eta_seconds=$((remaining_mbs * rate_per_mb))

  fmt_duration "$eta_seconds"
}

# Print a dashboard summary
print_dashboard() {
  local run_id="$1"

  local sf
  sf="$(state_file "$run_id")"

  if [[ ! -f "$sf" ]] || ! command -v jq >/dev/null 2>&1; then
    log_warn "state" "No state file or jq not available"
    return 0
  fi

  log_header "DASHBOARD ($run_id)"

  local status mbs_done mbs_fail tasks_done tasks_fail total_mbs total_tasks syncs reviews
  status=$(jq -r '.status' "$sf")
  mbs_done=$(jq -r '.summary.mbs_completed' "$sf")
  mbs_fail=$(jq -r '.summary.mbs_failed' "$sf")
  tasks_done=$(jq -r '.summary.tasks_completed' "$sf")
  tasks_fail=$(jq -r '.summary.tasks_failed' "$sf")
  total_mbs=$(jq -r '.total_megabatches' "$sf")
  total_tasks=$(jq -r '.total_tasks' "$sf")
  syncs=$(jq -r '.summary.syncs_completed' "$sf")
  reviews=$(jq -r '.summary.reviews_completed' "$sf")

  printf '  Status:          %s\n' "$status"
  printf '  Mega-batches:    %s/%s completed, %s failed\n' "$mbs_done" "$total_mbs" "$mbs_fail"
  printf '  Tasks:           %s/%s completed, %s failed\n' "$tasks_done" "$total_tasks" "$tasks_fail"
  printf '  Syncs:           %s\n' "$syncs"
  printf '  Reviews:         %s\n' "$reviews"
  printf '  ETA:             %s\n' "$(compute_eta "$run_id")"
  echo

  # Per-MB status
  printf '  %s%-6s %-8s %-10s %-12s %s%s\n' \
    "$C_BOLD" "MB" "PHASE" "STATUS" "ELAPSED" "AGENTS" "$C_RESET"

  local mb mb_status mb_elapsed mb_agents
  for mb in "${MEGA_BATCHES[@]}"; do
    mb_status=$(jq -r --arg m "$mb" '.megabatches[$m].status // "pending"' "$sf")
    mb_elapsed=$(jq -r --arg m "$mb" '.megabatches[$m].elapsed_seconds // 0' "$sf")
    mb_agents=$(jq -r --arg m "$mb" '.megabatches[$m].agents // ""' "$sf")
    printf '  %-6s %-8s %-10s %-12s %s\n' \
      "$mb" "${MB_PHASE[$mb]:-?}" "$mb_status" "$(fmt_duration "$mb_elapsed")" "$mb_agents"
  done
}

# Find last completed MB for crash recovery
find_resume_point() {
  local run_id="$1"

  local sf
  sf="$(state_file "$run_id")"

  if [[ ! -f "$sf" ]] || ! command -v jq >/dev/null 2>&1; then
    echo ""
    return 0
  fi

  # Find the last MB with status "completed" or "verified"
  local last_completed
  last_completed=$(jq -r '
    [.megabatches | to_entries[] | select(.value.status == "completed" or .value.status == "verified")] |
    sort_by(.key) | last | .key // ""
  ' "$sf")

  echo "$last_completed"
}

# Mark run as finished
finalize_state() {
  local run_id="$1"
  local final_status="${2:-completed}"

  local sf
  sf="$(state_file "$run_id")"
  [[ -f "$sf" ]] || return 0

  if ! command -v jq >/dev/null 2>&1; then
    return 0
  fi

  local tmp="${sf}.tmp"
  jq --arg st "$final_status" \
     --arg now "$(date -Iseconds)" \
     '.status = $st | .finished_at = $now | .updated_at = $now' \
     "$sf" > "$tmp" && mv "$tmp" "$sf"
}
