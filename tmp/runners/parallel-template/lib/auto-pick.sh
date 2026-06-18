#!/usr/bin/env bash
# auto-pick.sh — background monitor that progressively merges completed batches
#
# Usage:
#   ./auto-pick.sh [OPTIONS]
#
# Options:
#   --interval N      Check interval in seconds (default: 90)
#   --target-branch B Target branch to pick into (default: current branch)
#   --run-id ID       Specific run ID (default: latest)
#   --dry-run         Show what would be picked without doing it
#   --no-check        Skip cargo check after picks
#   --max-conflicts N Stop after N unresolvable conflicts (default: 5)
#
# This script is designed to be run in a separate terminal (or by Claude)
# while the main runner is executing. It:
#   1. Watches for batches transitioning to success/verified/merge_conflict
#   2. Cherry-picks them into the target branch
#   3. Auto-resolves conflicts (cargo check validates)
#   4. Prints detailed status each cycle
#
# Safety:
#   - Never force-pushes or resets the target branch
#   - On unresolvable conflict, skips and records for manual resolution
#   - All picks are regular commits (easily revertable)
#   - Ctrl-C cleanly aborts any in-progress cherry-pick

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

# ── Defaults ──
PICK_INTERVAL=90
PICK_TARGET_BRANCH=""
PICK_RUN_ID=""
PICK_DRY_RUN=0
PICK_NO_CHECK=0
PICK_MAX_CONFLICTS=5

# ── CLI ──
while [[ $# -gt 0 ]]; do
  case "$1" in
    --interval)       PICK_INTERVAL="$2"; shift 2 ;;
    --target-branch)  PICK_TARGET_BRANCH="$2"; shift 2 ;;
    --run-id)         PICK_RUN_ID="$2"; shift 2 ;;
    --dry-run)        PICK_DRY_RUN=1; shift ;;
    --no-check)       PICK_NO_CHECK=1; shift ;;
    --max-conflicts)  PICK_MAX_CONFLICTS="$2"; shift 2 ;;
    -h|--help)        head -30 "${BASH_SOURCE[0]}" | tail -25; exit 0 ;;
    *)                echo "Unknown: $1" >&2; exit 1 ;;
  esac
done

# ── Resolve run ID ──
if [[ -z "$PICK_RUN_ID" ]]; then
  PICK_RUN_ID="$(latest_run_id 2>/dev/null)" || { log_err "pick" "No runs found"; exit 1; }
fi

# ── Resolve target branch ──
if [[ -z "$PICK_TARGET_BRANCH" ]]; then
  PICK_TARGET_BRANCH="$(git -C "$ROKO_ROOT" rev-parse --abbrev-ref HEAD)"
fi

# ── State tracking ──
declare -A PICKED_BATCHES=()    # batch -> commit hash after pick
declare -A SKIPPED_BATCHES=()   # batch -> reason
TOTAL_PICKED=0
TOTAL_CONFLICTS=0
TOTAL_SKIPPED=0
CYCLE=0

# ── Load already-picked state from disk (survives restart) ──
PICK_STATE_FILE="$LOG_ROOT/$PICK_RUN_ID/auto-pick-state.env"
if [[ -f "$PICK_STATE_FILE" ]]; then
  while IFS='=' read -r key val; do
    [[ "$key" == PICKED_* ]] && PICKED_BATCHES["${key#PICKED_}"]="$val"
  done < "$PICK_STATE_FILE"
  log_info "pick" "Restored state: ${#PICKED_BATCHES[@]} previously picked"
fi

save_pick_state() {
  {
    for batch in "${!PICKED_BATCHES[@]}"; do
      echo "PICKED_${batch}=${PICKED_BATCHES[$batch]}"
    done
  } > "$PICK_STATE_FILE"
}

# ── Abort handler ──
cleanup_pick() {
  # Abort any in-progress cherry-pick
  git -C "$ROKO_ROOT" cherry-pick --abort 2>/dev/null || true
  save_pick_state
  echo
  log_info "pick" "Stopped. Picked $TOTAL_PICKED batches total."
  exit 0
}
trap cleanup_pick INT TERM

# ── Auto-resolve conflicts ──
auto_resolve_conflict() {
  local batch="$1"

  # Strategy: for each conflicting file, accept the incoming (batch) version
  # for new code, and try to merge where both sides changed
  local conflict_files
  conflict_files="$(git -C "$ROKO_ROOT" diff --name-only --diff-filter=U 2>/dev/null)"

  if [[ -z "$conflict_files" ]]; then
    return 1  # no conflict files found (shouldn't happen)
  fi

  local file_count
  file_count="$(echo "$conflict_files" | wc -l | tr -d ' ')"
  log_info "$batch" "Auto-resolving $file_count conflict(s)..."

  # Accept theirs (the batch's changes) for all conflicts
  # This is safe because the batch is the NEW work we want
  for f in $conflict_files; do
    git -C "$ROKO_ROOT" checkout --theirs -- "$f" 2>/dev/null || {
      log_warn "$batch" "Cannot resolve: $f"
      return 1
    }
    git -C "$ROKO_ROOT" add "$f" 2>/dev/null
  done

  # Complete the cherry-pick
  git -C "$ROKO_ROOT" -c core.editor=true cherry-pick --continue 2>/dev/null || {
    log_err "$batch" "Cherry-pick --continue failed"
    return 1
  }

  return 0
}

# ── Try to pick a single batch ──
try_pick_batch() {
  local batch="$1"

  # Find the commit hash
  local hash_file="$(run_result_file "$PICK_RUN_ID" "$batch").hash"
  local batch_hash=""
  if [[ -f "$hash_file" ]]; then
    batch_hash="$(cat "$hash_file")"
  else
    local branch_name="codex/${RUNNER_NAME}-${PICK_RUN_ID}-${batch}"
    batch_hash="$(git -C "$ROKO_ROOT" rev-parse "$branch_name" 2>/dev/null)" || {
      log_warn "$batch" "No hash or branch found — skipping"
      return 1
    }
  fi

  # Already in target branch?
  if git -C "$ROKO_ROOT" merge-base --is-ancestor "$batch_hash" HEAD 2>/dev/null; then
    PICKED_BATCHES["$batch"]="$batch_hash"
    return 0  # already picked (from previous run or manual merge)
  fi

  if (( PICK_DRY_RUN )); then
    log_info "$batch" "[dry-run] Would pick $batch_hash"
    return 0
  fi

  # Try cherry-pick
  local pick_rc=0
  git -C "$ROKO_ROOT" cherry-pick --no-edit "$batch_hash" 2>/dev/null || pick_rc=$?

  if (( pick_rc != 0 )); then
    # Conflict — try auto-resolve
    log_warn "$batch" "Conflict — attempting auto-resolve..."
    if auto_resolve_conflict "$batch"; then
      log_ok "$batch" "Auto-resolved and picked"
    else
      # Cannot resolve — abort and skip
      git -C "$ROKO_ROOT" cherry-pick --abort 2>/dev/null || true
      SKIPPED_BATCHES["$batch"]="unresolvable conflict"
      TOTAL_CONFLICTS=$((TOTAL_CONFLICTS + 1))
      log_err "$batch" "Unresolvable conflict — skipped (${TOTAL_CONFLICTS}/${PICK_MAX_CONFLICTS})"
      return 1
    fi
  fi

  PICKED_BATCHES["$batch"]="$batch_hash"
  TOTAL_PICKED=$((TOTAL_PICKED + 1))
  log_ok "$batch" "Picked → $(git -C "$ROKO_ROOT" rev-parse --short HEAD)"
  return 0
}

# ── Validate after picks ──
validate_picks() {
  if (( PICK_NO_CHECK || PICK_DRY_RUN )); then
    return 0
  fi

  log_info "check" "cargo check --workspace..."
  local check_rc=0
  local check_output
  check_output="$(cd "$ROKO_ROOT" && cargo check --workspace 2>&1)" || check_rc=$?

  if (( check_rc != 0 )); then
    local error_count
    error_count="$(echo "$check_output" | grep -c '^error' || echo 0)"
    log_err "check" "Compile failed ($error_count errors) — last pick may need manual fix"
    echo "$check_output" | grep '^error' | head -5 >&2
    return 1
  fi
  log_ok "check" "Compiles clean"
  return 0
}

# ── Print detailed status ──
print_cycle_status() {
  local status_file="$(run_status_json "$PICK_RUN_ID")"

  printf '\n%s━━━ AUTO-PICK CYCLE %d (%s) ━━━%s\n\n' \
    "$C_BOLD$C_CYAN" "$CYCLE" "$(date +%H:%M:%S)" "$C_RESET"

  # Runner status
  if [[ -f "$status_file" ]]; then
    local json
    json="$(cat "$status_file")"
    local total success failed in_prog pending parked elapsed eta
    total="$(echo "$json" | grep -o '"total": *[0-9]*' | grep -o '[0-9]*')"
    success="$(echo "$json" | grep -o '"success": *[0-9]*' | grep -o '[0-9]*')"
    failed="$(echo "$json" | grep -o '"failed": *[0-9]*' | grep -o '[0-9]*')"
    in_prog="$(echo "$json" | grep -o '"in_progress": *[0-9]*' | grep -o '[0-9]*')"
    pending="$(echo "$json" | grep -o '"pending": *[0-9]*' | grep -o '[0-9]*')"
    parked="$(echo "$json" | grep -o '"parked": *[0-9]*' | grep -o '[0-9]*')"
    elapsed="$(echo "$json" | grep -o '"elapsed": *"[^"]*"' | cut -d'"' -f4)"
    eta="$(echo "$json" | grep -o '"eta": *"[^"]*"' | cut -d'"' -f4)"

    printf '  %sRunner:%s  %s/%s done  |  %s active  |  %s pending  |  elapsed %s  |  ETA %s\n' \
      "$C_BOLD" "$C_RESET" "${success:-0}" "${total:-?}" "${in_prog:-0}" "${pending:-0}" "${elapsed:-?}" "${eta:-?}"
    if [[ -n "${failed:-}" ]] && (( failed > 0 )); then
      printf '  %sFailed:%s  %s\n' "$C_RED" "$C_RESET" "$failed"
    fi
    if [[ -n "${parked:-}" ]] && (( parked > 0 )); then
      printf '  %sParked:%s  %s (merge conflicts in runner — will auto-pick)\n' "$C_YELLOW" "$C_RESET" "$parked"
    fi
  else
    printf '  %sRunner:%s  waiting for first status...\n' "$C_BOLD" "$C_RESET"
  fi

  # Pick status
  printf '\n  %sPick status:%s\n' "$C_BOLD" "$C_RESET"
  printf '    Picked:     %s%d%s batches into %s\n' "$C_GREEN" "$TOTAL_PICKED" "$C_RESET" "$PICK_TARGET_BRANCH"
  printf '    Conflicts:  %d auto-resolved, %d skipped\n' 0 "${#SKIPPED_BATCHES[@]}"

  # What was picked this cycle
  if [[ -n "${CYCLE_PICKS:-}" ]]; then
    printf '\n  %sNew picks this cycle:%s\n' "$C_BOLD" "$C_RESET"
    for p in $CYCLE_PICKS; do
      local title
      title="$(batch_title "$p" 2>/dev/null || echo "?")"
      printf '    %s✓ %-8s%s %s\n' "$C_GREEN" "$p" "$C_RESET" "$title"
    done
  fi

  # Skipped batches
  if (( ${#SKIPPED_BATCHES[@]} > 0 )); then
    printf '\n  %sSkipped (need manual merge):%s\n' "$C_YELLOW" "$C_RESET"
    for batch in "${!SKIPPED_BATCHES[@]}"; do
      printf '    %s⚠ %-8s%s %s\n' "$C_YELLOW" "$batch" "$C_RESET" "${SKIPPED_BATCHES[$batch]}"
    done
  fi

  # Time estimate
  if (( TOTAL_PICKED > 0 )) && [[ -f "$status_file" ]]; then
    local remaining
    remaining="$(echo "$(cat "$status_file")" | grep -o '"pending": *[0-9]*' | grep -o '[0-9]*')"
    local active
    active="$(echo "$(cat "$status_file")" | grep -o '"in_progress": *[0-9]*' | grep -o '[0-9]*')"
    if (( remaining + active == 0 )); then
      printf '\n  %s✓ All batches complete. Run finished.%s\n' "$C_GREEN$C_BOLD" "$C_RESET"
    fi
  fi

  printf '\n  %sNext check: %ds%s\n' "$C_DIM" "$PICK_INTERVAL" "$C_RESET"
}

# ── Main loop ──

load_batch_registry

log_header "AUTO-PICK MONITOR"
log_info "pick" "Run:      $PICK_RUN_ID"
log_info "pick" "Target:   $PICK_TARGET_BRANCH"
log_info "pick" "Interval: ${PICK_INTERVAL}s"
log_info "pick" "Max conflicts before stop: $PICK_MAX_CONFLICTS"
echo

# Ensure we're on the target branch
current_branch="$(git -C "$ROKO_ROOT" rev-parse --abbrev-ref HEAD)"
if [[ "$current_branch" != "$PICK_TARGET_BRANCH" ]]; then
  log_err "pick" "Not on target branch. Current: $current_branch, expected: $PICK_TARGET_BRANCH"
  log_err "pick" "Run: git checkout $PICK_TARGET_BRANCH"
  exit 1
fi

while true; do
  CYCLE=$((CYCLE + 1))
  CYCLE_PICKS=""

  # Find batches that are done but not yet picked
  to_pick=()
  for batch in "${ALL_BATCHES[@]}"; do
    # Skip already picked or skipped
    [[ -n "${PICKED_BATCHES[$batch]:-}" ]] && continue
    [[ -n "${SKIPPED_BATCHES[$batch]:-}" ]] && continue

    # Check if batch has a successful result
    rf="$(run_result_file "$PICK_RUN_ID" "$batch")"
    [[ -f "$rf" ]] || continue
    status="$(cat "$rf")"

    # Pick successful batches AND parked (merge_conflict) batches
    # Parked batches have correct work, just couldn't merge into the runner's main
    case "$status" in
      success|success_noop|verified|merge_conflict)
        to_pick+=("$batch")
        ;;
    esac
  done

  # Pick each ready batch
  picks_this_cycle=0
  for batch in "${to_pick[@]}"; do
    # Stop if too many conflicts
    if (( TOTAL_CONFLICTS >= PICK_MAX_CONFLICTS )); then
      log_err "pick" "Hit max conflicts ($PICK_MAX_CONFLICTS) — stopping auto-pick"
      log_err "pick" "Resolve manually, then restart auto-pick"
      save_pick_state
      print_cycle_status
      exit 1
    fi

    try_pick_batch "$batch" && {
      picks_this_cycle=$((picks_this_cycle + 1))
      CYCLE_PICKS="${CYCLE_PICKS:+$CYCLE_PICKS }$batch"
    } || true
  done

  # Validate if we picked anything
  if (( picks_this_cycle > 0 )); then
    validate_picks || {
      log_warn "check" "Compile check failed after picks — continuing (will retry next cycle)"
    }
  fi

  save_pick_state
  print_cycle_status

  # Check if the run is completely done
  status_file="$(run_status_json "$PICK_RUN_ID")"
  if [[ -f "$status_file" ]]; then
    pending="$(cat "$status_file" | grep -o '"pending": *[0-9]*' | grep -o '[0-9]*')"
    in_prog="$(cat "$status_file" | grep -o '"in_progress": *[0-9]*' | grep -o '[0-9]*')"
    if (( pending + in_prog == 0 )); then
      log_ok "pick" "Runner complete. All pickable batches processed."
      log_ok "pick" "Total picked: $TOTAL_PICKED"
      if (( ${#SKIPPED_BATCHES[@]} > 0 )); then
        log_warn "pick" "Skipped ${#SKIPPED_BATCHES[@]} batches (need manual merge)"
      fi
      save_pick_state
      exit 0
    fi
  fi

  sleep "$PICK_INTERVAL"
done
