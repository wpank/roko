#!/usr/bin/env bash

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

write_failure_summary() {
  local batch="$1"
  local run_id="$2"
  local attempt="$3"
  local note="$4"
  local log_file failure_file
  log_file=$(run_log_file "$run_id" "$batch" "$attempt")
  failure_file=$(run_failure_file "$run_id" "$batch" "$attempt")

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

  git -C "$worktree" status --short -- . ':(exclude).cargo-target' ':(exclude)target' > "${prefix}.status"
  git -C "$worktree" diff -- . ':(exclude).cargo-target' ':(exclude)target' > "${prefix}.patch"
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
  git -C "$worktree" reset --hard HEAD >/dev/null
  git -C "$worktree" clean -ffdx >/dev/null
  ! worktree_dirty "$worktree"
}

verify_batch() {
  local batch="$1"
  local run_id="$2"
  local worktree="$3"
  local attempt="${4:-?}"
  local log_file target_dir cmd
  log_file=$(run_log_file "$run_id" "$batch" "$attempt")
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
      write_failure_summary "$batch" "$run_id" "$attempt" "Verification failed for command: $cmd"
      rm -rf "$target_dir"
      return 1
    fi
  done < <(batch_verify_commands "$batch")

  record_status "$run_id" "$batch" "$attempt" "verify_succeeded" "all verification commands passed"
  log_ok "$batch" "Verification passed"
  rm -rf "$target_dir"
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
    log_info "$batch" "Freed tmp targets: ${freed:-unknown} at $target_root"
  fi
}

cleanup_worktree_rust_artifacts() {
  local worktree="$1"
  [[ -d "$worktree" ]] || return 0
  rm -rf "$worktree/.cargo-target" "$worktree/target"
}

staged_path_denylist() {
  cat <<'EOF'
^\.roko/
^tmp/agent-registry/implementation-pack/runner/logs/
^target/
^\.cargo-target/
EOF
}

commit_batch_if_needed() {
  local batch="$1"
  local worktree="$2"
  local run_id="${3:-}"
  local attempt="${4:-?}"
  local title
  title=$(batch_title "$batch")

  cleanup_worktree_rust_artifacts "$worktree"

  git -C "$worktree" add -A -- . \
    ':(exclude).cargo-target' \
    ':(exclude)target' \
    ':(exclude).roko' \
    ':(exclude)tmp/agent-registry/implementation-pack/runner/logs'
  if git -C "$worktree" diff --cached --quiet; then
    if [[ -n "$run_id" ]]; then
      record_status "$run_id" "$batch" "$attempt" "commit_noop" "no staged changes after verify"
      cleanup_batch_tmp_targets "$run_id" "$batch"
      cleanup_worktree_rust_artifacts "$worktree"
    fi
    log_warn "$batch" "No changes staged after successful verification"
    return 10
  fi

  local staged_paths
  staged_paths="$(git -C "$worktree" diff --cached --name-only)"
  if [[ -n "$staged_paths" ]] && printf '%s\n' "$staged_paths" | rg -n -f <(staged_path_denylist) >/dev/null; then
    if [[ -n "$run_id" ]]; then
      record_status "$run_id" "$batch" "$attempt" "commit_failed" "unexpected staged runner or build artifacts"
      write_failure_summary "$batch" "$run_id" "$attempt" "Commit blocked: unexpected staged runner or build artifacts."
    fi
    log_err "$batch" "Unexpected staged runner or build artifacts; refusing commit"
    return 11
  fi

  if ! git -C "$worktree" commit -m "$(cat <<EOF
agent-registry(${batch}): ${title}

Automated implementation via tmp/agent-registry/implementation-pack/runner/run-agent-registry.sh
EOF
)" >/dev/null; then
    if [[ -n "$run_id" ]]; then
      record_status "$run_id" "$batch" "$attempt" "commit_failed" "git commit failed"
      write_failure_summary "$batch" "$run_id" "$attempt" "Commit step failed while creating the batch commit."
    fi
    log_err "$batch" "git commit failed"
    return 12
  fi

  if [[ -n "$run_id" ]]; then
    record_status "$run_id" "$batch" "$attempt" "commit_succeeded" "$(git -C "$worktree" rev-parse --short HEAD)"
    cleanup_batch_tmp_targets "$run_id" "$batch"
    cleanup_worktree_rust_artifacts "$worktree"
  fi
  log_ok "$batch" "Committed: $(git -C "$worktree" log --oneline -1)"
  return 0
}
