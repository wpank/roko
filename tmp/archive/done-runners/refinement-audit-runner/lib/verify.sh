#!/usr/bin/env bash

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

write_failure_summary() {
  local batch="$1" run_id="$2" note="$3"
  local log_file failure_file
  log_file=$(run_log_file "$run_id" "$batch")
  failure_file=$(run_failure_file "$run_id" "$batch")
  {
    echo "$note"
    echo
    echo "Recent log tail:"
    tail -40 "$log_file" 2>/dev/null || true
  } > "$failure_file"
}

backup_worktree_state() {
  local run_id="$1" batch="$2" attempt="$3" worktree="$4" label="$5"
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

# ---------------------------------------------------------------------------
# Verify — dispatches to per-phase gates
# ---------------------------------------------------------------------------

verify_batch() {
  local batch="$1" run_id="$2" worktree="$3"
  local attempt="${4:-?}"

  case "$batch" in
    AUD*) _verify_phase1 "$batch" "$run_id" "$worktree" "$attempt" ;;
    PU*)  _verify_phase2 "$batch" "$run_id" "$worktree" "$attempt" ;;
    PE*)  _verify_phase3 "$batch" "$run_id" "$worktree" "$attempt" ;;
  esac
}

_verify_phase1() {
  local batch="$1" run_id="$2" worktree="$3" attempt="$4"
  local log_file
  log_file=$(run_log_file "$run_id" "$batch")

  record_status "$run_id" "$batch" "$attempt" "verify_running" "scope+diff"

  # Scope gate — nothing outside docs/
  local outside
  outside="$(git -C "$worktree" status --porcelain=v1 \
    | awk '{print $2}' | grep -vE '^docs/' | grep -vE '^$' || true)"
  if [[ -n "$outside" ]]; then
    echo "[verify] scope_gate: files outside docs/: $outside" >> "$log_file"
    record_status "$run_id" "$batch" "$attempt" "verify_failed" "scope_gate"
    log_err "$batch" "Scope violation: files changed outside docs/"
    write_failure_summary "$batch" "$run_id" "Scope gate failed: modified files outside docs/."
    return 1
  fi

  # Diff gate — must produce doc changes
  local changed
  changed="$(git -C "$worktree" status --porcelain=v1 -- docs/ | wc -l | tr -d ' ')"
  if [[ "$changed" == "0" ]]; then
    echo "[verify] diff_gate: no changes under docs/" >> "$log_file"
    record_status "$run_id" "$batch" "$attempt" "verify_failed" "diff_gate"
    log_err "$batch" "No doc changes produced"
    write_failure_summary "$batch" "$run_id" "Diff gate failed: no changes under docs/."
    return 1
  fi

  record_status "$run_id" "$batch" "$attempt" "verify_succeeded" "all gates passed"
  log_ok "$batch" "Verification passed"
  return 0
}

_verify_phase2() {
  local batch="$1" run_id="$2" worktree="$3" attempt="$4"
  local section="${batch#PU}"
  local log_file
  log_file=$(run_log_file "$run_id" "$batch")
  # PU batches run in main repo, not worktree
  local check_dir="$ROKO_ROOT"

  record_status "$run_id" "$batch" "$attempt" "verify_running" "scope+diff (main repo)"

  local baseline_file outside before after current_fp baseline_fp changed_sections=() s
  baseline_file="$(run_batch_sections_fingerprint_file "$run_id" "$batch")"
  if [[ ! -f "$baseline_file" ]]; then
    echo "[verify] baseline_gate: missing baseline file $baseline_file" >> "$log_file"
    record_status "$run_id" "$batch" "$attempt" "verify_failed" "baseline_missing"
    log_err "$batch" "Missing parity baseline for verification"
    write_failure_summary "$batch" "$run_id" "Baseline gate failed."
    return 1
  fi

  # Scope gate — tracked changes outside tmp/docs-parity/ are never allowed.
  outside="$(git -C "$check_dir" status --porcelain=v1 \
    | awk '{print $2}' | grep -vE '^tmp/docs-parity/' | grep -vE '^$' || true)"
  if [[ -n "$outside" ]]; then
    echo "[verify] scope_gate: files outside tmp/docs-parity/: $outside" >> "$log_file"
    record_status "$run_id" "$batch" "$attempt" "verify_failed" "scope_gate"
    log_err "$batch" "Scope violation: files changed outside tmp/docs-parity/"
    write_failure_summary "$batch" "$run_id" "Scope gate failed."
    return 1
  fi

  for s in "${PHASE2_SECTIONS[@]}"; do
    before="$(baseline_sections_fingerprint "$run_id" "$batch" "$s" || true)"
    after="$(section_fingerprint "$(section_dir "$s")")"
    if [[ "$before" != "$after" ]]; then
      changed_sections+=("$s")
    fi
  done

  current_fp="$(section_fingerprint "$(section_dir "$section")")"
  baseline_fp="$(baseline_section_fingerprint "$run_id" "$batch")"
  if [[ "$current_fp" == "$baseline_fp" ]]; then
    echo "[verify] diff_gate: no changes in tmp/docs-parity/$section/" >> "$log_file"
    record_status "$run_id" "$batch" "$attempt" "verify_failed" "diff_gate"
    log_err "$batch" "No parity content changes produced"
    write_failure_summary "$batch" "$run_id" "Diff gate failed."
    return 1
  fi

  if (( ${#changed_sections[@]} > 1 )) || [[ "${changed_sections[*]:-}" != "$section" ]]; then
    echo "[verify] section_scope_gate: changed sections=${changed_sections[*]:-(none)}" >> "$log_file"
    record_status "$run_id" "$batch" "$attempt" "verify_failed" "section_scope_gate"
    log_err "$batch" "Scope violation: parity changes escaped section $section"
    write_failure_summary "$batch" "$run_id" "Section scope gate failed."
    return 1
  fi

  record_status "$run_id" "$batch" "$attempt" "verify_succeeded" "changes found in tmp/docs-parity/$section/"
  log_ok "$batch" "Verification passed"
  return 0
}

_verify_phase3() {
  local batch="$1" run_id="$2" worktree="$3" attempt="$4"
  local log_file
  log_file=$(run_log_file "$run_id" "$batch")
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

# ---------------------------------------------------------------------------
# Cleanup — frees 6-13 GB per batch on /tmp
# ---------------------------------------------------------------------------

cleanup_batch_tmp_targets() {
  local run_id="$1" batch="$2"
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
# Commit — per-phase staging rules
# ---------------------------------------------------------------------------

commit_batch_if_needed() {
  local batch="$1" worktree="$2" run_id="${3:-}" attempt="${4:-?}"
  local title
  title=$(batch_title "$batch")

  # PU batches run in main repo, not worktree
  local commit_dir="$worktree"
  case "$batch" in
    PU*) commit_dir="$ROKO_ROOT" ;;
  esac

  # Never stage build artifacts into runner commits.
  rm -rf "$commit_dir/.cargo-target" "$commit_dir/target"

  case "$batch" in
    AUD*) git -C "$commit_dir" add -A -- docs/ ;;
    PU*)
      local section="${batch#PU}"
      if [[ -n "$run_id" ]]; then
        record_status "$run_id" "$batch" "$attempt" "commit_skipped" "tmp/docs-parity is gitignored; keeping verified local files"
        cleanup_batch_tmp_targets "$run_id" "$batch"
      fi
      log_info "$batch" "Verified local parity refresh retained in tmp/docs-parity/$section/ (no git commit for ignored tmp/ paths)"
      return 10
      ;;
    PE*)  git -C "$commit_dir" add -A -- crates/ ;;
  esac

  if git -C "$commit_dir" diff --cached --quiet; then
    if [[ -n "$run_id" ]]; then
      record_status "$run_id" "$batch" "$attempt" "commit_noop" "no staged changes after verify"
      cleanup_batch_tmp_targets "$run_id" "$batch"
    fi
    log_warn "$batch" "No changes staged after successful verification"
    return 10
  fi

  local prefix
  case "$batch" in
    AUD*) prefix="audit-refinement" ;;
    PU*)  prefix="parity-refresh" ;;
    PE*)  prefix="parity-exec" ;;
  esac

  git -C "$commit_dir" commit -m "$(cat <<EOF
${prefix}(${batch}): ${title}

Automated via tmp/refinement-audit-runner/run-all.sh
EOF
)" >/dev/null
  if [[ -n "$run_id" ]]; then
    record_status "$run_id" "$batch" "$attempt" "commit_succeeded" "$(git -C "$commit_dir" rev-parse --short HEAD)"
    cleanup_batch_tmp_targets "$run_id" "$batch"
  fi
  log_ok "$batch" "Committed: $(git -C "$commit_dir" log --oneline -1)"
  return 0
}
