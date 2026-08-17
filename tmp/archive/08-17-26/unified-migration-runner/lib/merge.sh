#!/usr/bin/env bash

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

# ---------------------------------------------------------------------------
# merge_to_source — safe merge from worktree branch back to source branch
#
# Protocol:
#   1. Push worktree branch to remote (backup before merge)
#   2. Check source branch for uncommitted changes (abort if dirty)
#   3. Try fast-forward merge
#   4. If not ff-able, create a merge commit (no force, no rebase)
#   5. If conflict, abort and alert — never force
#
# On failure: worktree branch is preserved; all subsequent batches pause.
# ---------------------------------------------------------------------------

merge_to_source() {
  local worktree="$1"
  local source_branch="$2"
  local run_id="${3:-}"
  local batch="${4:-}"

  local worktree_branch
  worktree_branch=$(git -C "$worktree" branch --show-current)

  if [[ -z "$worktree_branch" ]]; then
    log_err "merge" "Worktree is in detached HEAD state"
    return 1
  fi

  # Record pre-merge state
  local pre_merge_sha
  pre_merge_sha=$(git -C "$ROKO_ROOT" rev-parse "$source_branch" 2>/dev/null || echo "unknown")

  if [[ -n "$run_id" ]]; then
    {
      echo "PRE_MERGE_SHA='$pre_merge_sha'"
      echo "WORKTREE_BRANCH='$worktree_branch'"
      echo "SOURCE_BRANCH='$source_branch'"
      echo "BATCH='$batch'"
      echo "TIMESTAMP='$(date -Iseconds)'"
    } >> "$LOG_ROOT/$run_id/merge-log.env"
  fi

  # 1. Push worktree branch to remote as backup
  log_info "merge" "Pushing $worktree_branch to remote (backup)"
  if ! git -C "$worktree" push -u origin "$worktree_branch" 2>/dev/null; then
    log_warn "merge" "Could not push to remote; continuing with local merge"
  fi

  # 2. Check source branch for uncommitted changes
  local source_worktree="$ROKO_ROOT"
  local current_branch
  current_branch=$(git -C "$source_worktree" branch --show-current)

  if [[ "$current_branch" != "$source_branch" ]]; then
    log_err "merge" "Source repo is on '$current_branch', expected '$source_branch'"
    log_err "merge" "Checkout '$source_branch' before running the merger"
    return 1
  fi

  if ! git -C "$source_worktree" diff --quiet 2>/dev/null || \
     ! git -C "$source_worktree" diff --cached --quiet 2>/dev/null; then
    log_err "merge" "Source branch '$source_branch' has uncommitted changes. Aborting merge."
    log_err "merge" "Stash or commit your changes, then re-run with --continue"
    return 1
  fi

  # 3. Try fast-forward merge
  log_info "merge" "Attempting fast-forward merge of $worktree_branch into $source_branch"
  if git -C "$source_worktree" merge --ff-only "$worktree_branch" 2>/dev/null; then
    log_ok "merge" "Fast-forward merge succeeded"
    if [[ -n "$run_id" ]]; then
      record_status "$run_id" "$batch" "merge" "merge_succeeded" "fast-forward"
    fi
    return 0
  fi

  # 4. If not ff-able, try a merge commit
  log_info "merge" "Fast-forward not possible; creating merge commit"
  if git -C "$source_worktree" merge --no-ff "$worktree_branch" \
     -m "merge: migration batch $batch from runner ($run_id)"; then
    log_ok "merge" "Merge commit created"
    if [[ -n "$run_id" ]]; then
      record_status "$run_id" "$batch" "merge" "merge_succeeded" "merge-commit"
    fi
    return 0
  fi

  # 5. Conflict — abort and alert
  git -C "$source_worktree" merge --abort 2>/dev/null || true
  log_err "merge" "CONFLICT: Cannot merge $worktree_branch into $source_branch"
  log_err "merge" "Worktree branch preserved at: $worktree_branch"
  log_err "merge" "Pre-merge SHA was: $pre_merge_sha"
  log_err "merge" "Manual resolution required."

  if [[ -n "$run_id" ]]; then
    record_status "$run_id" "$batch" "merge" "merge_failed" "conflict"
  fi

  return 1
}

# ---------------------------------------------------------------------------
# periodic_merge — called from main loop after every N successful batches
# ---------------------------------------------------------------------------

: "${MR_MERGE_INTERVAL:=5}"  # Merge every N batches

should_merge() {
  local success_count="$1"
  (( success_count > 0 && success_count % MR_MERGE_INTERVAL == 0 ))
}

periodic_merge() {
  local run_id="$1"
  local worktree="$2"
  local source_branch="$3"
  local batch="$4"

  log_header "MERGE TO SOURCE (after $batch)"

  if merge_to_source "$worktree" "$source_branch" "$run_id" "$batch"; then
    return 0
  fi

  log_err "merge" "Merge failed after batch $batch"
  log_err "merge" "All subsequent batches PAUSED to prevent divergence accumulation"
  return 1
}

# ---------------------------------------------------------------------------
# sync_all_agents — N-agent merge + rebase coordination (v2 parallel mode)
#
# Protocol:
#   1. For each agent A..D in order:
#      a. Merge agent branch → source (ff or merge commit)
#      b. Rebase all subsequent agents onto updated source
#   2. Final rebase: all agents onto source tip
#
# On conflict: abort + preserve branch, return error
# ---------------------------------------------------------------------------

sync_all_agents() {
  local run_id="$1"
  local source_branch="$2"
  local sync_label="$3"
  local n="${4:-4}"
  shift 4
  local -a agent_names=("$@")

  # Require the agent worktree/branch arrays from parallel.sh
  # AGENT_WORKTREES and AGENT_BRANCHES must be set by caller

  log_header "SYNC-${sync_label} ($n agents → $source_branch)"

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

  # Push all agent branches as backup
  local i agent wt branch
  for i in $(seq 0 $((n - 1))); do
    agent="${agent_names[$i]}"
    wt="${AGENT_WORKTREES[$agent]}"
    branch="${AGENT_BRANCHES[$agent]}"
    log_info "sync" "Pushing agent $agent ($branch) to remote"
    git -C "$wt" push -u origin "$branch" 2>/dev/null || \
      log_warn "sync" "Could not push agent $agent to remote"
  done

  # Sequential merge: each agent merges to source, others rebase
  for i in $(seq 0 $((n - 1))); do
    agent="${agent_names[$i]}"
    wt="${AGENT_WORKTREES[$agent]}"
    branch="${AGENT_BRANCHES[$agent]}"

    # Skip if nothing new
    local source_sha agent_sha
    source_sha=$(git -C "$source_wt" rev-parse HEAD)
    agent_sha=$(git -C "$wt" rev-parse HEAD)
    if [[ "$source_sha" == "$agent_sha" ]]; then
      log_info "sync" "Agent $agent: no new commits"
      continue
    fi
    if git -C "$source_wt" merge-base --is-ancestor "$agent_sha" "$source_sha" 2>/dev/null; then
      log_info "sync" "Agent $agent: already merged"
      continue
    fi

    # Try ff merge, then merge commit
    log_info "sync" "Merging agent $agent → $source_branch"
    if git -C "$source_wt" merge --ff-only "$branch" 2>/dev/null; then
      log_ok "sync" "Agent $agent: fast-forward"
      record_status "$run_id" "SYNC-${sync_label}" "agent-$agent" "merge_succeeded" "fast-forward"
    elif git -C "$source_wt" merge --no-ff "$branch" \
         -m "sync(${sync_label}): merge agent ${agent} (${run_id})" 2>/dev/null; then
      log_ok "sync" "Agent $agent: merge commit"
      record_status "$run_id" "SYNC-${sync_label}" "agent-$agent" "merge_succeeded" "merge-commit"
    else
      git -C "$source_wt" merge --abort 2>/dev/null || true
      log_err "sync" "Agent $agent: CONFLICT — manual resolution needed"
      log_err "sync" "Branch preserved: $branch in $wt"
      record_status "$run_id" "SYNC-${sync_label}" "agent-$agent" "merge_failed" "conflict"
      return 1
    fi

    # Rebase all subsequent agents onto updated source
    local j other_agent other_wt
    for j in $(seq $((i + 1)) $((n - 1))); do
      other_agent="${agent_names[$j]}"
      other_wt="${AGENT_WORKTREES[$other_agent]}"
      log_info "sync" "Rebasing agent $other_agent onto updated source"
      if ! git -C "$other_wt" rebase "$source_branch" 2>/dev/null; then
        git -C "$other_wt" rebase --abort 2>/dev/null || true
        log_warn "sync" "Agent $other_agent: rebase conflict — resetting to source tip"
        git -C "$other_wt" reset --hard "$source_branch" 2>/dev/null || true
      fi
    done
  done

  # Final pass: ensure all agents are on source tip
  for i in $(seq 0 $((n - 1))); do
    agent="${agent_names[$i]}"
    wt="${AGENT_WORKTREES[$agent]}"
    git -C "$wt" rebase "$source_branch" 2>/dev/null || {
      git -C "$wt" rebase --abort 2>/dev/null || true
      git -C "$wt" reset --hard "$source_branch" 2>/dev/null || true
    }
  done

  log_ok "sync" "SYNC-${sync_label} complete"
  return 0
}
