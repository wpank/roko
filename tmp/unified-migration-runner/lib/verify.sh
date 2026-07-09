#!/usr/bin/env bash

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

write_failure_summary() {
  local batch="$1"
  local run_id="$2"
  local note="$3"
  local log_file
  log_file=$(run_log_file "$run_id" "$batch")
  local failure_file
  failure_file=$(run_failure_file "$run_id" "$batch")

  {
    echo "$note"
    echo
    echo "Recent log tail:"
    tail -40 "$log_file" 2>/dev/null || true
  } > "$failure_file"
}

backup_worktree_state() {
  local run_id="$1"
  local batch="$2"
  local attempt="$3"
  local worktree="$4"
  local label="$5"
  local backup_dir prefix
  backup_dir="$(run_backups_dir "$run_id")"
  prefix="$backup_dir/${batch}-attempt-${attempt}-${label}"
  ensure_dir "$backup_dir"

  git -C "$worktree" status --short -- . ':(exclude).cargo-target' ':(exclude)target' \
    > "${prefix}.status"
  git -C "$worktree" diff -- . ':(exclude).cargo-target' ':(exclude)target' \
    > "${prefix}.patch"
  {
    echo "run_id=$run_id"
    echo "batch=$batch"
    echo "attempt=$attempt"
    echo "label=$label"
    echo "captured_at=$(date -Iseconds)"
    echo "worktree=$worktree"
  } > "${prefix}.meta"
}

reset_runner_worktree() {
  local worktree="$1"
  git -C "$worktree" reset --hard HEAD >/dev/null 2>&1 || true
  git -C "$worktree" clean -fd >/dev/null 2>&1 || true
}

verify_batch() {
  local batch="$1"
  local run_id="$2"
  local worktree="$3"
  local log_file
  log_file=$(run_log_file "$run_id" "$batch")
  local attempt="${4:-?}"
  local target_dir
  target_dir=$(batch_target_dir "$run_id" "$batch" "verify" "$attempt")
  rm -rf "$target_dir"
  mkdir -p "$target_dir"

  while IFS= read -r cmd; do
    [[ -z "$cmd" ]] && continue
    record_status "$run_id" "$batch" "$attempt" "verify_running" "$cmd"
    echo "[verify] CARGO_TARGET_DIR=$target_dir $cmd" >> "$log_file"
    if ! (
      cd "$worktree" &&
      env CARGO_TARGET_DIR="$target_dir" bash -lc "$cmd"
    ) >> "$log_file" 2>&1; then
      record_status "$run_id" "$batch" "$attempt" "verify_failed" "$cmd"
      log_err "$batch" "Verify failed: $cmd"
      write_failure_summary "$batch" "$run_id" "Verification failed for command: $cmd"
      return 1
    fi
  done < <(batch_verify_commands "$batch")

  record_status "$run_id" "$batch" "$attempt" "verify_succeeded" "all verification commands passed"
  log_ok "$batch" "Verification passed"
  return 0
}

cleanup_batch_tmp_targets() {
  local run_id="$1"
  local batch="$2"
  local target_root
  target_root="$(tmp_target_root)/$run_id/$batch"
  if [[ -d "$target_root" ]]; then
    local freed
    freed=$(du -sh "$target_root" 2>/dev/null | awk '{print $1}')
    rm -rf "$target_root"
    log_info "$batch" "Freed tmp targets: $freed at $target_root"
  fi
}

# ---------------------------------------------------------------------------
# Mega-batch verification (v2 parallel mode)
# ---------------------------------------------------------------------------

# Verify a mega-batch by running targeted crate checks for each task's crates
# Uses the agent's shared CARGO_TARGET_DIR for warm incremental builds
verify_megabatch() {
  local mb="$1"
  local agent="$2"
  local run_id="$3"
  local worktree="$4"
  local target_dir="$5"
  shift 5
  local -a task_ids=("$@")

  local log_file="$LOG_ROOT/$run_id/${mb}-agent-${agent}-verify.log"
  local failed=0

  log_info "$mb" "Agent $agent: verifying ${#task_ids[@]} tasks"

  # Collect unique verify commands across all tasks in this MB for this agent
  local -a seen_cmds=()
  local task cmd already_seen
  for task in "${task_ids[@]}"; do
    while IFS= read -r cmd; do
      [[ -z "$cmd" ]] && continue
      already_seen=0
      local s
      for s in "${seen_cmds[@]}"; do
        if [[ "$s" == "$cmd" ]]; then
          already_seen=1
          break
        fi
      done
      if (( already_seen == 0 )); then
        seen_cmds+=("$cmd")
      fi
    done < <(batch_verify_commands "$task")
  done

  # Run deduplicated verify commands
  for cmd in "${seen_cmds[@]}"; do
    record_status "$run_id" "$mb" "agent-$agent" "verify_running" "$cmd"
    echo "[verify] CARGO_TARGET_DIR=$target_dir $cmd" >> "$log_file"
    if ! (
      cd "$worktree" &&
      env CARGO_TARGET_DIR="$target_dir" bash -lc "$cmd"
    ) >> "$log_file" 2>&1; then
      record_status "$run_id" "$mb" "agent-$agent" "verify_failed" "$cmd"
      log_err "$mb" "Agent $agent verify failed: $cmd"
      failed=1
      break
    fi
  done

  if (( failed == 0 )); then
    record_status "$run_id" "$mb" "agent-$agent" "verify_succeeded" "all checks passed"
    log_ok "$mb" "Agent $agent verification passed"
    return 0
  fi
  return 1
}

# Full workspace test (run every Nth MB as a deeper check)
verify_workspace_full() {
  local run_id="$1"
  local worktree="$2"
  local target_dir="$3"
  local label="${4:-full}"

  local log_file="$LOG_ROOT/$run_id/verify-${label}.log"

  log_info "verify" "Full workspace test ($label)"

  if ! (
    cd "$worktree" &&
    env CARGO_TARGET_DIR="$target_dir" cargo test --workspace
  ) >> "$log_file" 2>&1; then
    log_err "verify" "Full workspace test FAILED"
    return 1
  fi

  log_ok "verify" "Full workspace test passed"
  return 0
}

# Cleanup shared target dir if it exceeds size threshold
cleanup_if_needed() {
  local target_dir="$1"
  local threshold_gb="${2:-20}"

  if [[ ! -d "$target_dir" ]]; then
    return 0
  fi

  local size_kb
  size_kb=$(du -sk "$target_dir" 2>/dev/null | awk '{print $1}')
  local threshold_kb=$((threshold_gb * 1024 * 1024))

  if (( size_kb > threshold_kb )); then
    local size_human
    size_human=$(du -sh "$target_dir" 2>/dev/null | awk '{print $1}')
    log_warn "cleanup" "Target dir $target_dir is ${size_human} (>${threshold_gb}GB), cleaning"
    rm -rf "$target_dir"
    mkdir -p "$target_dir"
  fi
}

commit_batch_if_needed() {
  local batch="$1"
  local worktree="$2"
  local run_id="${3:-}"
  local attempt="${4:-?}"
  local title
  title=$(batch_title "$batch")

  # Never stage build artifacts
  rm -rf "$worktree/.cargo-target" "$worktree/target"

  git -C "$worktree" add -A
  if git -C "$worktree" diff --cached --quiet; then
    if [[ -n "$run_id" ]]; then
      record_status "$run_id" "$batch" "$attempt" "commit_noop" "no staged changes after verify"
      cleanup_batch_tmp_targets "$run_id" "$batch"
    fi
    log_warn "$batch" "No changes staged after successful verification"
    return 10
  fi

  git -C "$worktree" commit -m "$(cat <<EOF
migration(${batch}): ${title}

Automated implementation via tmp/unified-migration-runner/run.sh
Phase ref: $(batch_phase_ref "$batch")
EOF
)" >/dev/null
  if [[ -n "$run_id" ]]; then
    record_status "$run_id" "$batch" "$attempt" "commit_succeeded" "$(git -C "$worktree" rev-parse --short HEAD)"
    cleanup_batch_tmp_targets "$run_id" "$batch"
  fi
  log_ok "$batch" "Committed: $(git -C "$worktree" log --oneline -1)"
  return 0
}
