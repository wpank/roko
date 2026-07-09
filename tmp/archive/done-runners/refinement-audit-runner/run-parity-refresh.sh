#!/usr/bin/env bash
# run-parity-refresh.sh — Phase 2: Parity content refresh runner (PU00-PU12)
#
# Updates tmp/docs-parity/{NN}/ files using `codex exec` in the main repo.
# Runs in the MAIN REPO (not a worktree) because codex --sandbox workspace-write
# blocks writes to tmp/ (it only allows recognized source dirs like crates/ and docs/).
#
# Usage:
#   bash tmp/refinement-audit-runner/run-parity-refresh.sh
#   bash tmp/refinement-audit-runner/run-parity-refresh.sh --only PU01,PU02
#   bash tmp/refinement-audit-runner/run-parity-refresh.sh --continue last
#   bash tmp/refinement-audit-runner/run-parity-refresh.sh --dry-run
#   bash tmp/refinement-audit-runner/run-parity-refresh.sh --list

set -euo pipefail
IFS=$'\n\t'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/pu-common.sh"
source "$SCRIPT_DIR/lib/pu-spawn.sh"
source "$SCRIPT_DIR/lib/pu-verify.sh"

: "${PU_MODEL:=gpt-5.4}"
: "${PU_TIMEOUT:=5400}"
: "${PU_MAX_RETRIES:=2}"
: "${PU_MAX_BATCHES:=0}"

DRY_RUN=0
FORCE=0
VERIFY_ONLY=0
LIST_ONLY=0
CONTINUE_RUN=""
SELECTED_BATCHES=()

print_usage() {
  cat <<'EOF'
run-parity-refresh.sh — Phase 2: Parity content refresh (PU00-PU12)

Updates tmp/docs-parity/{NN}/ files with audit-refined documentation.
Uses `codex exec --dangerously-bypass-approvals-and-sandbox`.
Runs in the main repo, not a worktree.

Usage:
  bash tmp/refinement-audit-runner/run-parity-refresh.sh
  bash tmp/refinement-audit-runner/run-parity-refresh.sh --only PU01,PU02
  bash tmp/refinement-audit-runner/run-parity-refresh.sh --continue last
  bash tmp/refinement-audit-runner/run-parity-refresh.sh --dry-run --only PU01
  bash tmp/refinement-audit-runner/run-parity-refresh.sh --verify-only --continue last
  bash tmp/refinement-audit-runner/run-parity-refresh.sh --list

Options:
  --only LIST         Comma-separated batch ids (PU00-PU12)
  --continue RUN      Continue a prior run id, or 'last'
  --dry-run           Show what would run; no Claude spawn
  --force             Re-run even successful batches
  --verify-only       Skip Claude, only run verify gates
  --list              Show batch manifest + exit
  --model MODEL       Override model (default: gpt-5.4)
  --timeout SECONDS   Per-batch timeout (default: 5400 = 90 min)
  --retries N         Automatic retries per batch (default: 2)
  --max-batches N     Hard cap on batches per run (default: 0 = unlimited)

Environment overrides (all optional):
  PU_MODEL, PU_TIMEOUT, PU_MAX_RETRIES, PU_MAX_BATCHES, NO_COLOR
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --only) IFS=',' read -r -a SELECTED_BATCHES <<< "$2"; shift 2 ;;
    --only=*) IFS=',' read -r -a SELECTED_BATCHES <<< "${1#*=}"; shift ;;
    --continue) CONTINUE_RUN="$2"; shift 2 ;;
    --continue=*) CONTINUE_RUN="${1#*=}"; shift ;;
    --dry-run) DRY_RUN=1; shift ;;
    --force) FORCE=1; shift ;;
    --verify-only) VERIFY_ONLY=1; shift ;;
    --list) LIST_ONLY=1; shift ;;
    --model) PU_MODEL="$2"; shift 2 ;;
    --model=*) PU_MODEL="${1#*=}"; shift ;;
    --timeout) PU_TIMEOUT="$2"; shift 2 ;;
    --timeout=*) PU_TIMEOUT="${1#*=}"; shift ;;
    --retries) PU_MAX_RETRIES="$2"; shift 2 ;;
    --retries=*) PU_MAX_RETRIES="${1#*=}"; shift ;;
    --max-batches) PU_MAX_BATCHES="$2"; shift 2 ;;
    --max-batches=*) PU_MAX_BATCHES="${1#*=}"; shift ;;
    -h|--help) print_usage; exit 0 ;;
    *) log_err "cli" "Unknown argument: $1"; print_usage; exit 1 ;;
  esac
done

if (( DRY_RUN == 1 )) && [[ -n "$CONTINUE_RUN" ]]; then
  log_err "cli" "--dry-run cannot be combined with --continue"
  exit 1
fi

select_batches() {
  local -a pool=()
  if [[ ${#SELECTED_BATCHES[@]} -gt 0 ]]; then
    local raw candidate found
    for raw in "${SELECTED_BATCHES[@]}"; do
      found=0
      for candidate in "${ALL_BATCHES[@]}"; do
        [[ "$candidate" == "$raw" ]] && { pool+=("$candidate"); found=1; break; }
      done
      (( found == 0 )) && { log_err "cli" "Unknown batch: $raw"; exit 1; }
    done
  else
    pool=("${ALL_BATCHES[@]}")
  fi
  local candidate raw
  for candidate in "${ALL_BATCHES[@]}"; do
    for raw in "${pool[@]}"; do
      [[ "$candidate" == "$raw" ]] && echo "$candidate"
    done
  done
}

list_batches() {
  printf '%s%-6s %-72s %s%s\n' \
    "$C_BOLD" "ID" "TITLE" "GROUP" "$C_RESET"
  for batch in "${ALL_BATCHES[@]}"; do
    printf '%-6s %-72s %s\n' \
      "$batch" "$(batch_title "$batch")" "$(batch_group "$batch")"
  done
}

# ---------------------------------------------------------------------------
# Run lifecycle — no worktree, operates on main repo
# ---------------------------------------------------------------------------

create_run() {
  RUN_ID="pu-run-$(date +%Y%m%d-%H%M%S)"
  WORK_DIR="$ROKO_ROOT"

  ensure_dir "$PU_LOG_ROOT/$RUN_ID"
  ensure_dir "$(run_prompts_dir "$RUN_ID")"
  : > "$(run_status_file "$RUN_ID")"
  link_latest_run "$RUN_ID"

  cat > "$(run_manifest_file "$RUN_ID")" <<EOF
RUN_ID='$RUN_ID'
WORK_DIR='$WORK_DIR'
PU_MODEL='$PU_MODEL'
PU_TIMEOUT='$PU_TIMEOUT'
PU_MAX_RETRIES='$PU_MAX_RETRIES'
PU_MAX_BATCHES='$PU_MAX_BATCHES'
CREATED_AT='$(date -Iseconds)'
EOF
}

create_dry_run() {
  RUN_ID="dry-run-$(date +%Y%m%d-%H%M%S)"
  WORK_DIR="$ROKO_ROOT"
  ensure_dir "$PU_LOG_ROOT/$RUN_ID"
  ensure_dir "$(run_prompts_dir "$RUN_ID")"
  : > "$(run_status_file "$RUN_ID")"

  cat > "$(run_manifest_file "$RUN_ID")" <<EOF
RUN_ID='$RUN_ID'
WORK_DIR='$WORK_DIR'
PU_MODEL='$PU_MODEL'
PU_TIMEOUT='$PU_TIMEOUT'
PU_MAX_RETRIES='$PU_MAX_RETRIES'
PU_MAX_BATCHES='$PU_MAX_BATCHES'
DRY_RUN=1
CREATED_AT='$(date -Iseconds)'
EOF
}

load_run() {
  if [[ "$CONTINUE_RUN" == "last" ]]; then
    CONTINUE_RUN="$(latest_run_id || true)"
  fi
  if [[ -z "$CONTINUE_RUN" ]]; then
    log_err "cli" "No prior run available to continue"
    exit 1
  fi
  local manifest
  manifest="$(run_manifest_file "$CONTINUE_RUN")"
  if [[ ! -f "$manifest" ]]; then
    log_err "cli" "Missing manifest for run: $CONTINUE_RUN"
    exit 1
  fi
  # shellcheck disable=SC1090
  source "$manifest"
  RUN_ID="$CONTINUE_RUN"
  WORK_DIR="$ROKO_ROOT"
  link_latest_run "$RUN_ID"
}

# ---------------------------------------------------------------------------
# Batch execution
# ---------------------------------------------------------------------------

batch_status() {
  local batch="$1"
  local result_file
  result_file="$(run_result_file "$RUN_ID" "$batch")"
  [[ -f "$result_file" ]] && cat "$result_file" || true
}

deps_satisfied() {
  # All PU batches are independent — always satisfied
  return 0
}

resume_preserved_batch() {
  local batch="$1"
  local current_batch result
  [[ -n "$CONTINUE_RUN" ]] || return 1
  current_batch="$(current_batch_name "$RUN_ID" 2>/dev/null || true)"
  [[ "$current_batch" == "$batch" ]] || return 1
  result="$(batch_status "$batch")"
  success_status "$result" && return 1
  repo_dirty
}

run_one_batch() {
  local batch="$1"
  local result_file log_file failure_file
  result_file="$(run_result_file "$RUN_ID" "$batch")"
  log_file="$(run_log_file "$RUN_ID" "$batch")"
  failure_file="$(run_failure_file "$RUN_ID" "$batch")"

  local existing
  existing="$(batch_status "$batch")"
  if [[ -n "$existing" ]] && success_status "$existing" && (( FORCE == 0 )) && (( VERIFY_ONLY == 0 )); then
    log_info "$batch" "Already successful; skipping"
    return 0
  fi

  if (( DRY_RUN == 1 )); then
    compose_prompt_snapshot "$batch" "$RUN_ID" "dry-run" "$failure_file" >/dev/null
    record_status "$RUN_ID" "$batch" "dry-run" "dry_run" "batch preview only"
    log_info "$batch" "[DRY RUN] $(batch_title "$batch")"
    log_info "$batch" "Prompt: $(batch_prompt_file "$batch")"
    echo "dry_run" > "$result_file"
    return 0
  fi

  if (( VERIFY_ONLY == 1 )); then
    if verify_batch "$batch" "$RUN_ID" "$ROKO_ROOT"; then
      echo "verify_only" > "$result_file"
      record_status "$RUN_ID" "$batch" "verify-only" "verify_only" "passed"
      log_ok "$batch" "Verification passed (verify-only)"
      return 0
    fi
    echo "verify_failed" > "$result_file"
    return 1
  fi

  local attempt spawn_rc commit_rc
  local preserve_dirty_resume=0
  local start_attempt=1

  if resume_preserved_batch "$batch"; then
    preserve_dirty_resume=1
    start_attempt="$(current_batch_attempt "$RUN_ID" 2>/dev/null || echo 1)"
    log_warn "$batch" "Resuming interrupted batch from preserved repo state"
  fi

  : > "$failure_file"
  for attempt in $(seq "$start_attempt" "$PU_MAX_RETRIES"); do
    set_current_batch "$RUN_ID" "$batch" "$attempt"
    log_header "$batch ATTEMPT $attempt/$PU_MAX_RETRIES"
    if (( preserve_dirty_resume == 1 && attempt == start_attempt )); then
      record_status "$RUN_ID" "$batch" "$attempt" "attempt_resumed" "reusing repo state from interrupted run"
      log_info "$batch" "Keeping current repo changes; skipping reset"
    else
      local section="${batch#PU}"
      if [[ ! -f "$(run_batch_sections_fingerprint_file "$RUN_ID" "$batch")" ]]; then
        capture_batch_baseline "$RUN_ID" "$batch"
        record_status "$RUN_ID" "$batch" "$attempt" "attempt_baselined" "captured parity section baseline"
      fi
      if [[ "$(section_fingerprint "$(section_dir "$section")")" != "$(baseline_section_fingerprint "$RUN_ID" "$batch")" ]]; then
        backup_repo_state "$RUN_ID" "$batch" "$attempt" "pre-reset"
        record_status "$RUN_ID" "$batch" "$attempt" "attempt_backed_up" "saved dirty state"
        reset_parity_section "$section"
      fi
      record_status "$RUN_ID" "$batch" "$attempt" "attempt_started" "$(batch_title "$batch")"
    fi

    spawn_rc=0
    if spawn_batch "$batch" "$RUN_ID" "$ROKO_ROOT" "$attempt" "$failure_file"; then
      if verify_batch "$batch" "$RUN_ID" "$ROKO_ROOT" "$attempt"; then
        commit_rc=0
        if commit_batch_if_needed "$batch" "$ROKO_ROOT" "$RUN_ID" "$attempt"; then
          echo "success" > "$result_file"
          record_status "$RUN_ID" "$batch" "$attempt" "attempt_succeeded" "verified and committed"
        else
          commit_rc=$?
          if [[ "$commit_rc" -eq 10 ]]; then
            echo "success_noop" > "$result_file"
            record_status "$RUN_ID" "$batch" "$attempt" "attempt_succeeded" "verified, no new changes"
          else
            echo "commit_failed" > "$result_file"
            record_status "$RUN_ID" "$batch" "$attempt" "commit_failed" "commit step failed"
            write_failure_summary "$batch" "$RUN_ID" "Commit step failed."
            return 1
          fi
        fi
        clear_current_batch "$RUN_ID"
        return 0
      fi
      echo "verify_failed" > "$result_file"
      record_status "$RUN_ID" "$batch" "$attempt" "attempt_failed" "verification failed"
      if (( attempt < PU_MAX_RETRIES )); then
        backup_repo_state "$RUN_ID" "$batch" "$attempt" "verify-failed"
      fi
    else
      spawn_rc=$?
      if [[ "$spawn_rc" -eq 124 ]]; then
        echo "timeout" > "$result_file"
        record_status "$RUN_ID" "$batch" "$attempt" "attempt_failed" "codex timed out"
        write_failure_summary "$batch" "$RUN_ID" "Codex timed out."
      else
        echo "spawn_failed" > "$result_file"
        record_status "$RUN_ID" "$batch" "$attempt" "attempt_failed" "codex exited unsuccessfully"
        write_failure_summary "$batch" "$RUN_ID" "Codex exited unsuccessfully."
      fi
      if (( attempt < PU_MAX_RETRIES )); then
        local section="${batch#PU}"
        if [[ "$(section_fingerprint "$(section_dir "$section")")" != "$(baseline_section_fingerprint "$RUN_ID" "$batch")" ]]; then
          backup_repo_state "$RUN_ID" "$batch" "$attempt" "spawn-failed"
        fi
      fi
    fi

    if (( attempt < PU_MAX_RETRIES )); then
      log_warn "$batch" "Retrying after failure"
    fi
  done
  return 1
}

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------

print_summary() {
  local batch result success=0 fail=0 other=0
  log_header "RUN SUMMARY ($RUN_ID)"
  for batch in "${SELECTED[@]}"; do
    result="$(batch_status "$batch")"
    result="${result:-pending}"
    printf '  %-6s %-14s %s\n' "$batch" "$result" "$(batch_title "$batch")"
    if success_status "$result"; then
      success=$((success + 1))
    elif terminal_failure_status "$result"; then
      fail=$((fail + 1))
    else
      other=$((other + 1))
    fi
  done
  printf '\n'
  printf '  success=%d  failed=%d  other=%d\n' "$success" "$fail" "$other"
  printf '  run_id=%s\n' "$RUN_ID"
  printf '  workdir=%s\n' "$ROKO_ROOT"
  printf '  logs=%s\n' "$PU_LOG_ROOT/$RUN_ID"
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

preflight_check

if (( LIST_ONLY == 1 )); then
  list_batches
  exit 0
fi

SELECTED=()
while IFS= read -r batch; do
  [[ -n "$batch" ]] && SELECTED+=("$batch")
done < <(select_batches)

if [[ -n "$CONTINUE_RUN" ]]; then
  load_run
  log_info "runner" "Continuing run $RUN_ID"
else
  if (( DRY_RUN == 1 )); then
    create_dry_run
    log_info "runner" "Created dry-run manifest $RUN_ID"
  else
    create_run
    log_info "runner" "Created run $RUN_ID"
  fi
fi

log_info "runner" "Model: $PU_MODEL (timeout: ${PU_TIMEOUT}s, retries: $PU_MAX_RETRIES)"
log_info "runner" "Max batches per run: ${PU_MAX_BATCHES} (0 = unlimited)"
log_info "runner" "Workdir: $ROKO_ROOT (main repo, no worktree)"
log_info "runner" "Selected batches: $(printf '%s,' "${SELECTED[@]}" | sed 's/,$//')"

batch_failed=0
processed=0
for batch in "${SELECTED[@]}"; do
  if (( PU_MAX_BATCHES > 0 )) && (( processed >= PU_MAX_BATCHES )); then
    log_warn "runner" "Reached PU_MAX_BATCHES=$PU_MAX_BATCHES; stopping for this session"
    break
  fi

  processed=$((processed + 1))
  run_one_batch "$batch" || batch_failed=1
done

print_summary
exit "$batch_failed"
