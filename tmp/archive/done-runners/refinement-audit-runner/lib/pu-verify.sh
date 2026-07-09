#!/usr/bin/env bash

source "$(dirname "${BASH_SOURCE[0]}")/pu-common.sh"

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

backup_repo_state() {
  local run_id="$1" batch="$2" attempt="$3" label="$4"
  local backup_dir prefix
  backup_dir="$(run_backups_dir "$run_id")"
  prefix="$backup_dir/${batch}-attempt-${attempt}-${label}"
  ensure_dir "$backup_dir"
  local section="${batch#PU}"
  {
    echo "section=$section"
    echo "fingerprint=$(section_fingerprint "$(section_dir "$section")")"
    echo "repo_dirty=$(repo_dirty && echo yes || echo no)"
  } > "${prefix}.status"
  rm -rf "${prefix}.tree"
  mkdir -p "${prefix}.tree"
  if [[ -d "$(section_dir "$section")" ]]; then
    (
      cd "$(section_dir "$section")"
      tar -cf - .
    ) | (
      cd "${prefix}.tree"
      tar -xf -
    )
  fi
  {
    echo "run_id=$run_id"
    echo "batch=$batch"
    echo "attempt=$attempt"
    echo "label=$label"
    echo "captured_at=$(date -Iseconds)"
    echo "workdir=$ROKO_ROOT"
  } > "${prefix}.meta"
}

reset_parity_section() {
  local section="$1" batch="PU${section}" current_run=""
  current_run="${RUN_ID:-}"
  [[ -n "$current_run" ]] || current_run="$(latest_run_id || true)"
  if [[ -n "$current_run" ]] && [[ -f "$(run_batch_sections_fingerprint_file "$current_run" "$batch")" ]]; then
    restore_batch_section_baseline "$current_run" "$batch"
    return 0
  fi

  local dir
  dir="$(section_dir "$section")"
  rm -rf "$dir"
  mkdir -p "$dir"
}

# ---------------------------------------------------------------------------
# Verify — scope + diff gate for PU batches
# ---------------------------------------------------------------------------

verify_batch() {
  local batch="$1" run_id="$2" _worktree="$3"
  local attempt="${4:-?}"
  local section="${batch#PU}"
  local log_file
  log_file=$(run_log_file "$run_id" "$batch")

  record_status "$run_id" "$batch" "$attempt" "verify_running" "scope+diff (main repo)"

  local baseline_file current_fp baseline_fp s before after changed_sections=()
  baseline_file="$(run_batch_sections_fingerprint_file "$run_id" "$batch")"
  if [[ ! -f "$baseline_file" ]]; then
    echo "[verify] baseline_gate: missing baseline file $baseline_file" >> "$log_file"
    record_status "$run_id" "$batch" "$attempt" "verify_failed" "baseline_missing"
    log_err "$batch" "Missing parity baseline for verification"
    write_failure_summary "$batch" "$run_id" "Baseline gate failed: missing saved state for tmp/docs-parity/$section/."
    return 1
  fi

  # Scope gate — tracked repo changes outside tmp/docs-parity are never allowed.
  local outside
  outside="$(git -C "$ROKO_ROOT" status --porcelain=v1 \
    | awk '{print $2}' | grep -vE '^tmp/docs-parity/' | grep -vE '^$' || true)"
  if [[ -n "$outside" ]]; then
    echo "[verify] scope_gate: tracked changes outside tmp/docs-parity/: $outside" >> "$log_file"
    record_status "$run_id" "$batch" "$attempt" "verify_failed" "scope_gate"
    log_err "$batch" "Scope violation: tracked files changed outside tmp/docs-parity/"
    write_failure_summary "$batch" "$run_id" "Scope gate failed: tracked files changed outside tmp/docs-parity/."
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
    write_failure_summary "$batch" "$run_id" "Diff gate failed: no changes in tmp/docs-parity/$section/."
    return 1
  fi

  if (( ${#changed_sections[@]} > 1 )) || [[ "${changed_sections[*]:-}" != "$section" ]]; then
    echo "[verify] section_scope_gate: changed sections=${changed_sections[*]:-(none)}" >> "$log_file"
    record_status "$run_id" "$batch" "$attempt" "verify_failed" "section_scope_gate"
    log_err "$batch" "Scope violation: parity changes escaped section $section"
    write_failure_summary "$batch" "$run_id" "Section scope gate failed: changed sections were ${changed_sections[*]:-(none)}; expected only $section."
    return 1
  fi

  record_status "$run_id" "$batch" "$attempt" "verify_succeeded" "changes found in tmp/docs-parity/$section/"
  log_ok "$batch" "Verification passed"
  return 0
}

# ---------------------------------------------------------------------------
# Commit — stages only the specific section directory
# ---------------------------------------------------------------------------

commit_batch_if_needed() {
  local batch="$1" _worktree="$2" run_id="${3:-}" attempt="${4:-?}"
  local title section
  title=$(batch_title "$batch")
  section="${batch#PU}"

  if [[ -n "$run_id" ]]; then
    record_status "$run_id" "$batch" "$attempt" "commit_skipped" "tmp/docs-parity is gitignored; keeping verified local files"
  fi
  log_info "$batch" "Verified local parity refresh retained in tmp/docs-parity/$section/ (no git commit for ignored tmp/ paths)"
  return 10
}
