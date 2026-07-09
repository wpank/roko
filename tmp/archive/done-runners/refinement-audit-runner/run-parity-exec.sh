#!/usr/bin/env bash
# run-parity-exec.sh — Phase 3: Parity code execution runner (PE00-PE12)
#
# Runs code changes from docs-parity materials. Edits crates/ in a worktree.
# Uses `codex exec --sandbox workspace-write --full-auto` — same as ux-followup-runner.
#
# Usage:
#   bash tmp/refinement-audit-runner/run-parity-exec.sh
#   bash tmp/refinement-audit-runner/run-parity-exec.sh --only PE01,PE02
#   bash tmp/refinement-audit-runner/run-parity-exec.sh --continue last
#   bash tmp/refinement-audit-runner/run-parity-exec.sh --dry-run
#   bash tmp/refinement-audit-runner/run-parity-exec.sh --list

set -euo pipefail
IFS=$'\n\t'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/pe-common.sh"
source "$SCRIPT_DIR/lib/pe-spawn.sh"
source "$SCRIPT_DIR/lib/pe-verify.sh"

: "${PE_MODEL:=gpt-5.4}"
: "${PE_REASONING:=high}"
: "${PE_TIMEOUT:=5400}"
: "${PE_MAX_RETRIES:=2}"
: "${PE_BASE_REF:=HEAD}"
: "${PE_MAX_BATCHES:=0}"

DRY_RUN=0
FORCE=0
VERIFY_ONLY=0
LIST_ONLY=0
CONTINUE_RUN=""
SELECTED_BATCHES=()

print_usage() {
  cat <<'EOF'
run-parity-exec.sh — Phase 3: Parity code execution (PE00-PE12)

Runs code changes from docs-parity materials. Edits crates/ in a worktree.
Uses `codex exec --sandbox workspace-write --full-auto`.
Isolated CARGO_TARGET_DIR in /tmp/.

Usage:
  bash tmp/refinement-audit-runner/run-parity-exec.sh
  bash tmp/refinement-audit-runner/run-parity-exec.sh --only PE01,PE02
  bash tmp/refinement-audit-runner/run-parity-exec.sh --continue last
  bash tmp/refinement-audit-runner/run-parity-exec.sh --dry-run --only PE01
  bash tmp/refinement-audit-runner/run-parity-exec.sh --verify-only --continue last
  bash tmp/refinement-audit-runner/run-parity-exec.sh --list

Options:
  --only LIST         Comma-separated batch ids (PE00-PE12)
  --continue RUN      Continue a prior run id, or 'last'
  --dry-run           Show what would run; no Codex spawn
  --force             Re-run even successful batches
  --verify-only       Skip Codex, only run verify gates
  --list              Show batch manifest + exit
  --model MODEL       Override model (default: gpt-5.4)
  --reasoning LEVEL   Override reasoning (default: high)
  --timeout SECONDS   Per-batch timeout (default: 5400 = 90 min)
  --retries N         Automatic retries per batch (default: 2)
  --base-ref REF      Base git ref for a new worktree (default: HEAD)
  --max-batches N     Hard cap on batches per run (default: 0 = unlimited)

Environment overrides (all optional):
  PE_MODEL, PE_REASONING, PE_TIMEOUT, PE_MAX_RETRIES, PE_BASE_REF,
  PE_MAX_BATCHES, NO_COLOR
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
    --model) PE_MODEL="$2"; shift 2 ;;
    --model=*) PE_MODEL="${1#*=}"; shift ;;
    --reasoning) PE_REASONING="$2"; shift 2 ;;
    --reasoning=*) PE_REASONING="${1#*=}"; shift ;;
    --timeout) PE_TIMEOUT="$2"; shift 2 ;;
    --timeout=*) PE_TIMEOUT="${1#*=}"; shift ;;
    --retries) PE_MAX_RETRIES="$2"; shift 2 ;;
    --retries=*) PE_MAX_RETRIES="${1#*=}"; shift ;;
    --base-ref) PE_BASE_REF="$2"; shift 2 ;;
    --base-ref=*) PE_BASE_REF="${1#*=}"; shift ;;
    --max-batches) PE_MAX_BATCHES="$2"; shift 2 ;;
    --max-batches=*) PE_MAX_BATCHES="${1#*=}"; shift ;;
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
# Run lifecycle — worktree-based (same as ux-followup-runner)
# ---------------------------------------------------------------------------

create_run() {
  RUN_ID="pe-run-$(date +%Y%m%d-%H%M%S)"
  WORKTREE="$ROKO_ROOT"
  BRANCH="$(git -C "$ROKO_ROOT" rev-parse --abbrev-ref HEAD)"

  ensure_dir "$PE_LOG_ROOT/$RUN_ID"
  ensure_dir "$(run_prompts_dir "$RUN_ID")"
  : > "$(run_status_file "$RUN_ID")"
  link_latest_run "$RUN_ID"

  cat > "$(run_manifest_file "$RUN_ID")" <<EOF
RUN_ID='$RUN_ID'
WORKTREE='$ROKO_ROOT'
BRANCH='$BRANCH'
PE_MODEL='$PE_MODEL'
PE_REASONING='$PE_REASONING'
PE_TIMEOUT='$PE_TIMEOUT'
PE_MAX_RETRIES='$PE_MAX_RETRIES'
PE_BASE_REF='$PE_BASE_REF'
PE_MAX_BATCHES='$PE_MAX_BATCHES'
CREATED_AT='$(date -Iseconds)'
EOF
}

create_dry_run() {
  RUN_ID="dry-run-$(date +%Y%m%d-%H%M%S)"
  WORKTREE="$ROKO_ROOT"
  BRANCH="(dry-run)"
  ensure_dir "$PE_LOG_ROOT/$RUN_ID"
  ensure_dir "$(run_prompts_dir "$RUN_ID")"
  : > "$(run_status_file "$RUN_ID")"

  cat > "$(run_manifest_file "$RUN_ID")" <<EOF
RUN_ID='$RUN_ID'
WORKTREE='$ROKO_ROOT'
BRANCH='(dry-run)'
PE_MODEL='$PE_MODEL'
PE_REASONING='$PE_REASONING'
PE_TIMEOUT='$PE_TIMEOUT'
PE_MAX_RETRIES='$PE_MAX_RETRIES'
PE_BASE_REF='$PE_BASE_REF'
PE_MAX_BATCHES='$PE_MAX_BATCHES'
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
  WORKTREE="${WORKTREE:-$ROKO_ROOT}"
  link_latest_run "$RUN_ID"
}

# ---------------------------------------------------------------------------
# Batch execution (same pattern as ux-followup-runner)
# ---------------------------------------------------------------------------

batch_status() {
  local batch="$1"
  local result_file
  result_file="$(run_result_file "$RUN_ID" "$batch")"
  [[ -f "$result_file" ]] && cat "$result_file" || true
}

deps_satisfied() {
  # All PE batches are independent — always satisfied
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
  worktree_dirty "$WORKTREE"
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
    log_info "$batch" "Verify commands:"
    batch_verify_commands "$batch" | sed 's/^/  /'
    echo "dry_run" > "$result_file"
    return 0
  fi

  if (( VERIFY_ONLY == 1 )); then
    if verify_batch "$batch" "$RUN_ID" "$WORKTREE"; then
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
  local preserve_failed_state=0
  local start_attempt=1

  if resume_preserved_batch "$batch"; then
    preserve_dirty_resume=1
    start_attempt="$(current_batch_attempt "$RUN_ID" 2>/dev/null || echo 1)"
    log_warn "$batch" "Resuming interrupted batch from preserved dirty worktree state"
  fi

  : > "$failure_file"
  for attempt in $(seq "$start_attempt" "$PE_MAX_RETRIES"); do
    set_current_batch "$RUN_ID" "$batch" "$attempt"
    log_header "$batch ATTEMPT $attempt/$PE_MAX_RETRIES"
    if (( preserve_dirty_resume == 1 && attempt == start_attempt )); then
      record_status "$RUN_ID" "$batch" "$attempt" "attempt_resumed" "reusing dirty worktree"
      log_info "$batch" "Keeping current worktree changes; skipping reset"
    elif (( preserve_failed_state == 1 && attempt == start_attempt + 1 )); then
      record_status "$RUN_ID" "$batch" "$attempt" "attempt_preserved_retry" "reusing dirty state"
      log_info "$batch" "Keeping failed-attempt changes for retry; skipping reset"
    else
      if worktree_dirty "$WORKTREE"; then
        backup_worktree_state "$RUN_ID" "$batch" "$attempt" "$WORKTREE" "pre-reset"
        record_status "$RUN_ID" "$batch" "$attempt" "attempt_backed_up" "saved dirty worktree"
      fi
      record_status "$RUN_ID" "$batch" "$attempt" "attempt_started" "$(batch_title "$batch")"
      reset_runner_worktree "$WORKTREE"
    fi

    spawn_rc=0
    if spawn_batch "$batch" "$RUN_ID" "$WORKTREE" "$attempt" "$failure_file"; then
      if verify_batch "$batch" "$RUN_ID" "$WORKTREE" "$attempt"; then
        commit_rc=0
        if commit_batch_if_needed "$batch" "$WORKTREE" "$RUN_ID" "$attempt"; then
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
      if (( attempt < PE_MAX_RETRIES )) && worktree_dirty "$WORKTREE"; then
        backup_worktree_state "$RUN_ID" "$batch" "$attempt" "$WORKTREE" "verify-failed"
        record_status "$RUN_ID" "$batch" "$attempt" "attempt_backed_up" "saved failed verify state"
        preserve_failed_state=1
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
      if (( attempt < PE_MAX_RETRIES )) && worktree_dirty "$WORKTREE"; then
        backup_worktree_state "$RUN_ID" "$batch" "$attempt" "$WORKTREE" "spawn-failed"
        record_status "$RUN_ID" "$batch" "$attempt" "attempt_backed_up" "saved failed spawn state"
        preserve_failed_state=1
      fi
    fi

    if (( attempt < PE_MAX_RETRIES )); then
      if (( preserve_failed_state == 1 )); then
        log_warn "$batch" "Retrying with preserved failed-attempt changes"
      else
        log_warn "$batch" "Retrying after failure"
      fi
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
  printf '  worktree=%s\n' "$WORKTREE"
  printf '  branch=%s\n' "$BRANCH"
  printf '  logs=%s\n' "$PE_LOG_ROOT/$RUN_ID"
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
  log_info "runner" "Continuing run $RUN_ID in $WORKTREE"
else
  if (( DRY_RUN == 1 )); then
    create_dry_run
    log_info "runner" "Created dry-run manifest $RUN_ID"
  else
    create_run
    log_info "runner" "Created run $RUN_ID in $WORKTREE"
  fi
fi

log_info "runner" "Model: $PE_MODEL (reasoning: $PE_REASONING, timeout: ${PE_TIMEOUT}s, retries: $PE_MAX_RETRIES)"
log_info "runner" "Max batches per run: ${PE_MAX_BATCHES} (0 = unlimited)"
log_info "runner" "Selected batches: $(printf '%s,' "${SELECTED[@]}" | sed 's/,$//')"

batch_failed=0
processed=0
for batch in "${SELECTED[@]}"; do
  if (( PE_MAX_BATCHES > 0 )) && (( processed >= PE_MAX_BATCHES )); then
    log_warn "runner" "Reached PE_MAX_BATCHES=$PE_MAX_BATCHES; stopping for this session"
    break
  fi

  processed=$((processed + 1))
  run_one_batch "$batch" || batch_failed=1
done

print_summary
exit "$batch_failed"
