#!/usr/bin/env bash

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

write_failure_summary() {
  local batch="$1" run_id="$2" note="$3"
  local log_file failure_file
  log_file="$(run_log_file "$run_id" "$batch")"
  failure_file="$(run_failure_file "$run_id" "$batch")"
  { echo "$note"; echo; echo "Recent log tail:"; tail -60 "$log_file" 2>/dev/null || true; } > "$failure_file"
}

backup_worktree_state() {
  local run_id="$1" batch="$2" attempt="$3" worktree="$4" label="$5"
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
  git -C "$worktree" reset --hard HEAD >/dev/null 2>&1 || true
  git -C "$worktree" clean -fd -e .cargo-target -e target >/dev/null 2>&1 || true
}

verify_batch() {
  local batch="$1" run_id="$2" worktree="$3"
  local attempt="${4:-?}" log_file target_dir
  log_file="$(run_log_file "$run_id" "$batch")"
  target_dir="$(batch_target_dir "$run_id" "$batch" "verify" "$attempt")"
  rm -rf "$target_dir"
  mkdir -p "$target_dir"

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
  local target_root
  target_root="$(tmp_target_root)/$run_id/$batch"
  if [[ -d "$target_root" ]]; then
    local freed
    freed="$(du -sh "$target_root" 2>/dev/null | awk '{print $1}')"
    rm -rf "$target_root"
    log_info "$batch" "Freed tmp targets: $freed at $target_root"
  fi
}

cleanup_all_batch_targets() {
  local run_id="$1" batch="$2"
  cleanup_batch_tmp_targets "$run_id" "$batch"
}

cleanup_stale_tmp_target_runs() {
  local current_run_id="$1"
  local root stale_days dir base
  root="$(tmp_target_root)"
  stale_days="${RT_CLEANUP_STALE_DAYS:-3}"
  [[ "$stale_days" =~ ^[0-9]+$ ]] || stale_days=3
  [[ -d "$root" ]] || return 0

  while IFS= read -r dir; do
    base="$(basename "$dir")"
    [[ "$base" == "$current_run_id" ]] && continue
    rm -rf "$dir"
  done < <(find "$root" -maxdepth 1 -mindepth 1 -type d \( -name 'run-*' -o -name 'dry-run-*' \) -mtime "+$stale_days" -print 2>/dev/null)
}

cleanup_worktree_rust_artifacts() {
  local worktree="$1"
  [[ -d "$worktree" ]] || return 0
  rm -rf "$worktree/.cargo-target" "$worktree/target" 2>/dev/null || true
}

cleanup_ephemeral_rust_artifacts() {
  local run_id="$1" worktree="$2" reason="${3:-periodic}"
  cleanup_worktree_rust_artifacts "$worktree"
  cleanup_stale_tmp_target_runs "$run_id"
  log_info "cleanup" "Cleaned ephemeral Rust artifacts ($reason)"
}

commit_batch_if_needed() {
  local batch="$1" worktree="$2" run_id="${3:-}" attempt="${4:-?}"
  local title
  title="$(batch_title "$batch")"
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
roko-trustworthy(${batch}): ${title}

Automated implementation via tmp/roko-trustworthy/run-roko-trustworthy.sh
EOF
)" >/dev/null
  if [[ -n "$run_id" ]]; then
    record_status "$run_id" "$batch" "$attempt" "commit_succeeded" "$(git -C "$worktree" rev-parse --short HEAD)"
    cleanup_batch_tmp_targets "$run_id" "$batch"
  fi
  log_ok "$batch" "Committed: $(git -C "$worktree" log --oneline -1)"
  return 0
}
