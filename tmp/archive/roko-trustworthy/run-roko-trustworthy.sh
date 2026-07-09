#!/usr/bin/env bash

set -euo pipefail
IFS=$'\n\t'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/spawn.sh"
source "$SCRIPT_DIR/lib/verify.sh"

DRY_RUN=0
FORCE=0
VERIFY_ONLY=0
LIST_ONLY=0
CONTINUE_RUN=""
SELECTED_BATCHES=()
SELECTED_GROUPS=()

print_usage() {
  cat <<'EOF'
run-roko-trustworthy.sh - Codex runner for the Roko-first self-hosting trust roadmap

Usage:
  bash tmp/roko-trustworthy/run-roko-trustworthy.sh
  bash tmp/roko-trustworthy/run-roko-trustworthy.sh --only RT00,RT01
  bash tmp/roko-trustworthy/run-roko-trustworthy.sh --group kernel
  bash tmp/roko-trustworthy/run-roko-trustworthy.sh --continue last
  bash tmp/roko-trustworthy/run-roko-trustworthy.sh --dry-run --only RT00
  bash tmp/roko-trustworthy/run-roko-trustworthy.sh --list

Options:
  --only LIST         Comma-separated batch ids (RT00-RT23)
  --group LIST        Comma-separated groups:
                      gate, kernel, policy, runtime, selfhost, core,
                      parity, dashboard, advanced
  --continue RUN      Continue a prior run id, or 'last'
  --dry-run           Compose prompts and show gates; do not spawn Codex
  --force             Re-run even successful batches
  --verify-only       Skip Codex and only run verification gates
  --list              Show the batch manifest and exit
  --model MODEL       Override model (default: gpt-5.5)
  --reasoning LEVEL   Override reasoning effort (default: high)
  --timeout SECONDS   Per-batch timeout (default: 7200 = 2 hours)
  --retries N         Attempts per batch, including first attempt (default: 2)
  --base-ref REF      Base git ref for a new worktree (default: HEAD)
  --max-batches N     Hard cap on batches per run (default: 0 = unlimited)
  --cleanup-every N   Clean Rust temp artifacts every N processed batches
                      (default: 1, use 0 to disable periodic cleanup)
  --cleanup-stale-days N
                      Delete stale runner cargo target dirs older than N days
                      (default: 3)
  --keep-failed-targets
                      Preserve failed-attempt cargo target dirs for debugging

Environment overrides:
  ROKO_ROOT, RT_MODEL, RT_REASONING, RT_TIMEOUT, RT_MAX_RETRIES,
  RT_BASE_REF, RT_MAX_BATCHES, RT_CLEANUP_EVERY, RT_CLEANUP_STALE_DAYS,
  RT_CLEAN_FAILED_ATTEMPTS, NO_COLOR
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --only) IFS=',' read -r -a SELECTED_BATCHES <<< "$2"; shift 2 ;;
    --only=*) IFS=',' read -r -a SELECTED_BATCHES <<< "${1#*=}"; shift ;;
    --group) IFS=',' read -r -a SELECTED_GROUPS <<< "$2"; shift 2 ;;
    --group=*) IFS=',' read -r -a SELECTED_GROUPS <<< "${1#*=}"; shift ;;
    --continue) CONTINUE_RUN="$2"; shift 2 ;;
    --continue=*) CONTINUE_RUN="${1#*=}"; shift ;;
    --dry-run) DRY_RUN=1; shift ;;
    --force) FORCE=1; shift ;;
    --verify-only) VERIFY_ONLY=1; shift ;;
    --list) LIST_ONLY=1; shift ;;
    --model) RT_MODEL="$2"; shift 2 ;;
    --model=*) RT_MODEL="${1#*=}"; shift ;;
    --reasoning) RT_REASONING="$2"; shift 2 ;;
    --reasoning=*) RT_REASONING="${1#*=}"; shift ;;
    --timeout) RT_TIMEOUT="$2"; shift 2 ;;
    --timeout=*) RT_TIMEOUT="${1#*=}"; shift ;;
    --retries) RT_MAX_RETRIES="$2"; shift 2 ;;
    --retries=*) RT_MAX_RETRIES="${1#*=}"; shift ;;
    --base-ref) RT_BASE_REF="$2"; shift 2 ;;
    --base-ref=*) RT_BASE_REF="${1#*=}"; shift ;;
    --max-batches) RT_MAX_BATCHES="$2"; shift 2 ;;
    --max-batches=*) RT_MAX_BATCHES="${1#*=}"; shift ;;
    --cleanup-every) RT_CLEANUP_EVERY="$2"; shift 2 ;;
    --cleanup-every=*) RT_CLEANUP_EVERY="${1#*=}"; shift ;;
    --cleanup-stale-days) RT_CLEANUP_STALE_DAYS="$2"; shift 2 ;;
    --cleanup-stale-days=*) RT_CLEANUP_STALE_DAYS="${1#*=}"; shift ;;
    --keep-failed-targets) RT_CLEAN_FAILED_ATTEMPTS=0; shift ;;
    -h|--help) print_usage; exit 0 ;;
    *) log_err "cli" "Unknown argument: $1"; print_usage; exit 1 ;;
  esac
done

if (( DRY_RUN == 1 )) && [[ -n "$CONTINUE_RUN" ]]; then
  log_err "cli" "--dry-run cannot be combined with --continue"
  exit 1
fi

group_contains() {
  local needle="$1"; shift
  local group
  for group in "$@"; do
    [[ "$group" == "$needle" ]] && return 0
  done
  return 1
}

select_batches() {
  local -a pool=()
  local batch group
  if [[ ${#SELECTED_BATCHES[@]} -gt 0 ]]; then
    local raw candidate found
    for raw in "${SELECTED_BATCHES[@]}"; do
      found=0
      for candidate in "${ALL_BATCHES[@]}"; do
        if [[ "$candidate" == "$raw" ]]; then
          pool+=("$candidate")
          found=1
          break
        fi
      done
      (( found == 0 )) && { log_err "cli" "Unknown batch: $raw"; exit 1; }
    done
  elif [[ ${#SELECTED_GROUPS[@]} -gt 0 ]]; then
    for batch in "${ALL_BATCHES[@]}"; do
      group="$(batch_group "$batch")"
      group_contains "$group" "${SELECTED_GROUPS[@]}" && pool+=("$batch")
    done
  else
    pool=("${ALL_BATCHES[@]}")
  fi

  local candidate selected
  for candidate in "${ALL_BATCHES[@]}"; do
    for selected in "${pool[@]}"; do
      [[ "$candidate" == "$selected" ]] && echo "$candidate"
    done
  done
}

list_batches() {
  printf '%s%-6s %-62s %-12s %s%s\n' "$C_BOLD" "ID" "TITLE" "GROUP" "DEPS" "$C_RESET"
  local batch
  for batch in "${ALL_BATCHES[@]}"; do
    printf '%-6s %-62s %-12s %s\n' \
      "$batch" "$(batch_title "$batch")" "$(batch_group "$batch")" "$(batch_deps "$batch")"
  done
}

create_run() {
  RUN_ID="run-$(date +%Y%m%d-%H%M%S)"
  WORKTREE="$WORKTREE_ROOT/roko-trustworthy-$RUN_ID"
  BRANCH="codex/roko-trustworthy-$RUN_ID"
  ensure_dir "$LOG_ROOT/$RUN_ID"
  ensure_dir "$(run_prompts_dir "$RUN_ID")"
  : > "$(run_status_file "$RUN_ID")"
  link_latest_run "$RUN_ID"
  git -C "$ROKO_ROOT" worktree add -b "$BRANCH" "$WORKTREE" "$RT_BASE_REF" >/dev/null
  cat > "$(run_manifest_file "$RUN_ID")" <<EOF
RUN_ID='$RUN_ID'
WORKTREE='$WORKTREE'
BRANCH='$BRANCH'
RT_MODEL='$RT_MODEL'
RT_REASONING='$RT_REASONING'
RT_TIMEOUT='$RT_TIMEOUT'
RT_MAX_RETRIES='$RT_MAX_RETRIES'
RT_BASE_REF='$RT_BASE_REF'
RT_MAX_BATCHES='$RT_MAX_BATCHES'
RT_CLEANUP_EVERY='$RT_CLEANUP_EVERY'
RT_CLEANUP_STALE_DAYS='$RT_CLEANUP_STALE_DAYS'
RT_CLEAN_FAILED_ATTEMPTS='$RT_CLEAN_FAILED_ATTEMPTS'
CREATED_AT='$(date -Iseconds)'
EOF
}

create_dry_run() {
  RUN_ID="dry-run-$(date +%Y%m%d-%H%M%S)"
  WORKTREE="$WORKTREE_ROOT/roko-trustworthy-$RUN_ID"
  BRANCH="(not-created)"
  ensure_dir "$LOG_ROOT/$RUN_ID"
  ensure_dir "$(run_prompts_dir "$RUN_ID")"
  : > "$(run_status_file "$RUN_ID")"
  link_latest_run "$RUN_ID"
  cat > "$(run_manifest_file "$RUN_ID")" <<EOF
RUN_ID='$RUN_ID'
WORKTREE='$WORKTREE'
BRANCH='$BRANCH'
RT_MODEL='$RT_MODEL'
RT_REASONING='$RT_REASONING'
RT_TIMEOUT='$RT_TIMEOUT'
RT_MAX_RETRIES='$RT_MAX_RETRIES'
RT_BASE_REF='$RT_BASE_REF'
RT_MAX_BATCHES='$RT_MAX_BATCHES'
RT_CLEANUP_EVERY='$RT_CLEANUP_EVERY'
RT_CLEANUP_STALE_DAYS='$RT_CLEANUP_STALE_DAYS'
RT_CLEAN_FAILED_ATTEMPTS='$RT_CLEAN_FAILED_ATTEMPTS'
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
  if [[ ! -d "$WORKTREE" ]]; then
    log_err "cli" "Worktree missing for run $RUN_ID: $WORKTREE"
    exit 1
  fi
  link_latest_run "$RUN_ID"
}

batch_status() {
  local result_file
  result_file="$(run_result_file "$RUN_ID" "$1")"
  [[ -f "$result_file" ]] && cat "$result_file" || true
}

deps_satisfied() {
  local dep status
  local -a deps=()
  IFS=' ' read -r -a deps <<< "$(batch_deps "$1")"
  for dep in "${deps[@]}"; do
    [[ -z "$dep" ]] && continue
    status="$(batch_status "$dep")"
    success_status "$status" || return 1
  done
  return 0
}

deps_terminal_failure() {
  local dep status
  local -a deps=()
  IFS=' ' read -r -a deps <<< "$(batch_deps "$1")"
  for dep in "${deps[@]}"; do
    [[ -z "$dep" ]] && continue
    status="$(batch_status "$dep")"
    terminal_failure_status "$status" && return 0
  done
  return 1
}

resume_preserved_batch() {
  local batch="$1"
  [[ -n "$CONTINUE_RUN" ]] || return 1
  local current_batch
  current_batch="$(current_batch_name "$RUN_ID" 2>/dev/null || true)"
  [[ "$current_batch" == "$batch" ]] || return 1
  local result
  result="$(batch_status "$batch")"
  success_status "$result" && return 1
  worktree_dirty "$WORKTREE"
}

run_one_batch() {
  local batch="$1"
  local result_file failure_file
  result_file="$(run_result_file "$RUN_ID" "$batch")"
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
    if verify_batch "$batch" "$RUN_ID" "$WORKTREE" "verify-only"; then
      echo "verify_only" > "$result_file"
      record_status "$RUN_ID" "$batch" "verify-only" "verify_only" "verification passed"
      log_ok "$batch" "Verification passed (verify-only)"
      return 0
    fi
    echo "verify_failed" > "$result_file"
    return 1
  fi

  local attempt spawn_rc commit_rc
  local preserve_dirty_resume=0 preserve_failed_state=0 start_attempt=1

  if resume_preserved_batch "$batch"; then
    preserve_dirty_resume=1
    start_attempt="$(current_batch_attempt "$RUN_ID" 2>/dev/null || echo 1)"
    log_warn "$batch" "Resuming interrupted batch from preserved dirty worktree state"
  fi

  : > "$failure_file"
  for attempt in $(seq "$start_attempt" "$RT_MAX_RETRIES"); do
    set_current_batch "$RUN_ID" "$batch" "$attempt"
    log_header "$batch ATTEMPT $attempt/$RT_MAX_RETRIES"
    if (( preserve_dirty_resume == 1 && attempt == start_attempt )); then
      record_status "$RUN_ID" "$batch" "$attempt" "attempt_resumed" "reusing dirty worktree"
      log_info "$batch" "Keeping current worktree changes"
    elif (( preserve_failed_state == 1 && attempt == start_attempt + 1 )); then
      record_status "$RUN_ID" "$batch" "$attempt" "attempt_preserved_retry" "reusing failed state"
      log_info "$batch" "Keeping failed-attempt changes for retry"
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
            record_status "$RUN_ID" "$batch" "$attempt" "attempt_succeeded" "verified with no new changes"
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
      if (( attempt < RT_MAX_RETRIES )) && worktree_dirty "$WORKTREE"; then
        backup_worktree_state "$RUN_ID" "$batch" "$attempt" "$WORKTREE" "verify-failed"
        preserve_failed_state=1
      fi
      if (( RT_CLEAN_FAILED_ATTEMPTS == 1 )); then
        cleanup_batch_tmp_targets "$RUN_ID" "$batch"
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
      if (( attempt < RT_MAX_RETRIES )) && worktree_dirty "$WORKTREE"; then
        backup_worktree_state "$RUN_ID" "$batch" "$attempt" "$WORKTREE" "spawn-failed"
        preserve_failed_state=1
      fi
      if (( RT_CLEAN_FAILED_ATTEMPTS == 1 )); then
        cleanup_batch_tmp_targets "$RUN_ID" "$batch"
      fi
    fi

    if (( attempt < RT_MAX_RETRIES )); then
      if (( preserve_failed_state == 1 )); then
        log_warn "$batch" "Retrying with preserved failed-attempt changes"
      else
        log_warn "$batch" "Retrying after failure"
      fi
    fi
  done

  cleanup_all_batch_targets "$RUN_ID" "$batch"
  return 1
}

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
  printf '\n  success=%d  failed=%d  other=%d\n' "$success" "$fail" "$other"
  printf '  run_id=%s\n  worktree=%s\n  branch=%s\n  logs=%s\n' \
    "$RUN_ID" "$WORKTREE" "$BRANCH" "$LOG_ROOT/$RUN_ID"
}

preflight_check

if (( LIST_ONLY == 1 )); then
  list_batches
  exit 0
fi

mapfile -t SELECTED < <(select_batches)

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

log_info "runner" "Model: $RT_MODEL (reasoning: $RT_REASONING, timeout: $RT_TIMEOUT s, retries: $RT_MAX_RETRIES)"
log_info "runner" "Max batches per run: ${RT_MAX_BATCHES} (0 = unlimited)"
log_info "runner" "Cleanup: every ${RT_CLEANUP_EVERY} batch(es), stale target dirs older than ${RT_CLEANUP_STALE_DAYS} day(s)"
log_info "runner" "Selected batches: $(printf '%s,' "${SELECTED[@]}" | sed 's/,$//')"

batch_failed=0
processed=0
for batch in "${SELECTED[@]}"; do
  if (( RT_MAX_BATCHES > 0 )) && (( processed >= RT_MAX_BATCHES )); then
    log_warn "runner" "Reached RT_MAX_BATCHES=$RT_MAX_BATCHES; stopping"
    break
  fi
  if deps_satisfied "$batch"; then
    processed=$((processed + 1))
    run_one_batch "$batch" || batch_failed=1
    if (( RT_CLEANUP_EVERY > 0 )) && (( processed % RT_CLEANUP_EVERY == 0 )); then
      cleanup_ephemeral_rust_artifacts "$RUN_ID" "$WORKTREE" "after $processed processed batch(es)"
    fi
  elif deps_terminal_failure "$batch"; then
    log_warn "$batch" "Blocked by failed dependency: $(batch_deps "$batch")"
    echo "blocked" > "$(run_result_file "$RUN_ID" "$batch")"
  else
    log_warn "$batch" "Skipping; dependencies not yet satisfied"
    echo "blocked" > "$(run_result_file "$RUN_ID" "$batch")"
  fi
done

print_summary
exit "$batch_failed"
