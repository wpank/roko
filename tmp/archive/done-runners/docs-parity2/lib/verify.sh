#!/usr/bin/env bash

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

write_failure_summary() {
  local batch="$1" run_id="$2" note="$3"
  local log_file failure_file
  log_file=$(run_log_file "$run_id" "$batch")
  failure_file=$(run_failure_file "$run_id" "$batch")
  { echo "$note"; echo; echo "Recent log tail:"; tail -40 "$log_file" 2>/dev/null || true; } > "$failure_file"
}

backup_worktree_state() {
  local run_id="$1" batch="$2" attempt="$3" worktree="$4" label="$5"
  local backup_dir prefix
  backup_dir="$(run_backups_dir "$run_id")"
  prefix="$backup_dir/${batch}-attempt-${attempt}-${label}"
  ensure_dir "$backup_dir"
  git -C "$worktree" status --short -- . ':(exclude).cargo-target' ':(exclude)target' > "${prefix}.status"
  git -C "$worktree" diff -- . ':(exclude).cargo-target' ':(exclude)target' > "${prefix}.patch"
  { echo "run_id=$run_id"; echo "batch=$batch"; echo "attempt=$attempt"; echo "label=$label"; echo "captured_at=$(date -Iseconds)"; echo "worktree=$worktree"; } > "${prefix}.meta"
}

reset_runner_worktree() {
  git -C "$1" reset --hard HEAD >/dev/null 2>&1 || true
  git -C "$1" clean -fd >/dev/null 2>&1 || true
}

verify_batch() {
  local batch="$1" run_id="$2" worktree="$3"
  local log_file attempt target_dir
  log_file=$(run_log_file "$run_id" "$batch")
  attempt="${4:-?}"
  target_dir=$(batch_target_dir "$run_id" "$batch" "verify" "$attempt")
  rm -rf "$target_dir"; mkdir -p "$target_dir"

  while IFS= read -r cmd; do
    [[ -z "$cmd" ]] && continue
    record_status "$run_id" "$batch" "$attempt" "verify_running" "$cmd"
    echo "[verify] CARGO_TARGET_DIR=$target_dir $cmd" >> "$log_file"
    if ! ( cd "$worktree" && env CARGO_TARGET_DIR="$target_dir" bash -lc "$cmd" ) >> "$log_file" 2>&1; then
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
  local run_id="$1" batch="$2"
  local target_root; target_root="$(tmp_target_root)/$run_id/$batch"
  if [[ -d "$target_root" ]]; then
    local freed; freed=$(du -sh "$target_root" 2>/dev/null | awk '{print $1}')
    rm -rf "$target_root"
    log_info "$batch" "Freed tmp targets: $freed at $target_root"
  fi
}

# Cleanup ALL tmp targets for a batch — called after final failure too.
# Prevents disk from filling up during long overnight runs.
cleanup_all_batch_targets() {
  local run_id="$1" batch="$2"
  cleanup_batch_tmp_targets "$run_id" "$batch"
  # Also clean any leftover codex/verify attempt dirs
  local run_target_root; run_target_root="$(tmp_target_root)/$run_id"
  if [[ -d "$run_target_root" ]]; then
    local freed; freed=$(du -sh "$run_target_root" 2>/dev/null | awk '{print $1}')
    if [[ "$freed" != "0B" && "$freed" != "0" ]]; then
      log_info "$batch" "Run target dir size after cleanup: $freed"
    fi
  fi
}

commit_batch_if_needed() {
  local batch="$1" worktree="$2" run_id="${3:-}" attempt="${4:-?}"
  local title; title=$(batch_title "$batch")
  rm -rf "$worktree/.cargo-target" "$worktree/target"
  git -C "$worktree" add -A
  if git -C "$worktree" diff --cached --quiet; then
    if [[ -n "$run_id" ]]; then
      record_status "$run_id" "$batch" "$attempt" "commit_noop" "no staged changes"
      cleanup_batch_tmp_targets "$run_id" "$batch"
    fi
    log_warn "$batch" "No changes staged after successful verification"
    return 10
  fi
  git -C "$worktree" commit -m "$(cat <<EOF
docs-parity2(${batch}): ${title}

Automated implementation via tmp/docs-parity2/run-docs-parity2.sh
EOF
)" >/dev/null
  if [[ -n "$run_id" ]]; then
    record_status "$run_id" "$batch" "$attempt" "commit_succeeded" "$(git -C "$worktree" rev-parse --short HEAD)"
    cleanup_batch_tmp_targets "$run_id" "$batch"
  fi
  log_ok "$batch" "Committed: $(git -C "$worktree" log --oneline -1)"
  return 0
}
