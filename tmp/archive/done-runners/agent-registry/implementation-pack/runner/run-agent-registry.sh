#!/usr/bin/env bash

set -euo pipefail
IFS=$'\n\t'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/spawn.sh"
source "$SCRIPT_DIR/lib/verify.sh"

: "${AR_MODEL:=gpt-5.4}"
: "${AR_REASONING:=high}"
: "${AR_TIMEOUT:=7200}"
: "${AR_MAX_RETRIES:=2}"
: "${AR_BASE_REF:=HEAD}"
: "${AR_MAX_BATCHES:=0}"
: "${AR_CODEX_FAST_PROFILE:=fast}"

RUN_ID=""
WORKTREE=""
BRANCH=""
SELECTED=()
RUNNER_EXIT_TRAPPED=0

DRY_RUN=0
FORCE=0
VERIFY_ONLY=0
LIST_ONLY=0
CONTINUE_RUN=""
SELECTED_BATCHES=()

print_usage() {
  cat <<'EOF'
run-agent-registry.sh — Codex batch runner for tmp/agent-registry/implementation-pack

Usage:
  bash tmp/agent-registry/implementation-pack/runner/run-agent-registry.sh
  bash tmp/agent-registry/implementation-pack/runner/run-agent-registry.sh --only AR01,AR02
  bash tmp/agent-registry/implementation-pack/runner/run-agent-registry.sh --continue last
  bash tmp/agent-registry/implementation-pack/runner/run-agent-registry.sh --dry-run --only AR01
  bash tmp/agent-registry/implementation-pack/runner/run-agent-registry.sh --verify-only --continue last
  bash tmp/agent-registry/implementation-pack/runner/run-agent-registry.sh --list

Options:
  --only LIST         Comma-separated batch ids (AR01-AR08)
  --continue RUN      Continue a prior run id, or 'last'
  --dry-run           Show what would run; no Codex spawn; no worktree
  --force             Re-run even successful batches
  --verify-only       Skip Codex, only run verification gates
  --list              Show batch manifest + exit
  --model MODEL       Override model (default: gpt-5.4)
  --reasoning LEVEL   Override fallback reasoning if no fast profile exists (default: high)
  --timeout SECONDS   Per-batch timeout (default: 7200)
  --retries N         Automatic retries after the first failed attempt (default: 2)
  --base-ref REF      Base git ref for a new worktree (default: HEAD)
  --max-batches N     Hard cap on batches per run (default: 0 = unlimited)
EOF
}

require_option_value() {
  local flag="$1"
  local argc="$2"
  if (( argc < 2 )); then
    log_err "cli" "Missing value for $flag"
    exit 1
  fi
}

require_uint() {
  local name="$1"
  local value="$2"
  if [[ ! "$value" =~ ^[0-9]+$ ]]; then
    log_err "cli" "$name must be a non-negative integer, got: $value"
    exit 1
  fi
}

attempt_total() {
  echo $((AR_MAX_RETRIES + 1))
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --only)
      require_option_value "$1" "$#"
      IFS=',' read -r -a SELECTED_BATCHES <<< "$2"
      shift 2
      ;;
    --only=*) IFS=',' read -r -a SELECTED_BATCHES <<< "${1#*=}"; shift ;;
    --continue)
      require_option_value "$1" "$#"
      CONTINUE_RUN="$2"
      shift 2
      ;;
    --continue=*) CONTINUE_RUN="${1#*=}"; shift ;;
    --dry-run) DRY_RUN=1; shift ;;
    --force) FORCE=1; shift ;;
    --verify-only) VERIFY_ONLY=1; shift ;;
    --list) LIST_ONLY=1; shift ;;
    --model)
      require_option_value "$1" "$#"
      AR_MODEL="$2"
      shift 2
      ;;
    --model=*) AR_MODEL="${1#*=}"; shift ;;
    --reasoning)
      require_option_value "$1" "$#"
      AR_REASONING="$2"
      shift 2
      ;;
    --reasoning=*) AR_REASONING="${1#*=}"; shift ;;
    --timeout)
      require_option_value "$1" "$#"
      AR_TIMEOUT="$2"
      shift 2
      ;;
    --timeout=*) AR_TIMEOUT="${1#*=}"; shift ;;
    --retries)
      require_option_value "$1" "$#"
      AR_MAX_RETRIES="$2"
      shift 2
      ;;
    --retries=*) AR_MAX_RETRIES="${1#*=}"; shift ;;
    --base-ref)
      require_option_value "$1" "$#"
      AR_BASE_REF="$2"
      shift 2
      ;;
    --base-ref=*) AR_BASE_REF="${1#*=}"; shift ;;
    --max-batches)
      require_option_value "$1" "$#"
      AR_MAX_BATCHES="$2"
      shift 2
      ;;
    --max-batches=*) AR_MAX_BATCHES="${1#*=}"; shift ;;
    -h|--help) print_usage; exit 0 ;;
    *) log_err "cli" "Unknown argument: $1"; print_usage; exit 1 ;;
  esac
done

if (( DRY_RUN == 1 )) && [[ -n "$CONTINUE_RUN" ]]; then
  log_err "cli" "--dry-run cannot be combined with --continue"
  exit 1
fi

require_uint "AR_TIMEOUT" "$AR_TIMEOUT"
require_uint "AR_MAX_RETRIES" "$AR_MAX_RETRIES"
require_uint "AR_MAX_BATCHES" "$AR_MAX_BATCHES"

select_batches() {
  local -a pool=()
  local batch raw candidate found

  if [[ ${#SELECTED_BATCHES[@]} -gt 0 ]]; then
    for raw in "${SELECTED_BATCHES[@]}"; do
      found=0
      for candidate in "${ALL_BATCHES[@]}"; do
        if [[ "$candidate" == "$raw" ]]; then
          pool+=("$candidate")
          found=1
          break
        fi
      done
      if (( found == 0 )); then
        log_err "cli" "Unknown batch: $raw"
        exit 1
      fi
    done
  else
    pool=("${ALL_BATCHES[@]}")
  fi

  for candidate in "${ALL_BATCHES[@]}"; do
    for raw in "${pool[@]}"; do
      if [[ "$candidate" == "$raw" ]]; then
        echo "$candidate"
      fi
    done
  done
}

list_batches() {
  printf '%s%-6s %-56s %-12s %s%s\n' \
    "$C_BOLD" "ID" "TITLE" "GROUP" "DEPS" "$C_RESET"
  local batch
  for batch in "${ALL_BATCHES[@]}"; do
    printf '%-6s %-56s %-12s %s\n' \
      "$batch" "$(batch_title "$batch")" "$(batch_group "$batch")" "$(batch_deps "$batch")"
  done
}

ensure_timeout_command() {
  if command -v timeout >/dev/null 2>&1 || command -v gtimeout >/dev/null 2>&1; then
    return 0
  fi
  log_err "bootstrap" "This runner requires timeout or gtimeout for real Codex runs"
  exit 1
}

preflight_commit_identity() {
  local name email
  name="$(git -C "$ROKO_ROOT" config --get user.name || true)"
  email="$(git -C "$ROKO_ROOT" config --get user.email || true)"
  if [[ -z "$name" || -z "$email" ]]; then
    log_err "bootstrap" "git user.name and user.email must be configured before running committed batches"
    exit 1
  fi
}

preflight_selected_tools() {
  local batch tool
  local -a tools=()
  for batch in "${SELECTED[@]}"; do
    IFS=' ' read -r -a tools <<< "$(batch_required_tools "$batch")"
    for tool in "${tools[@]}"; do
      [[ -z "$tool" ]] && continue
      require_command "$tool"
    done
  done
}

preflight_runtime() {
  require_command git

  if (( LIST_ONLY == 1 )); then
    return 0
  fi

  if [[ -z "$CONTINUE_RUN" ]]; then
    git -C "$ROKO_ROOT" rev-parse --verify "${AR_BASE_REF}^{commit}" >/dev/null 2>&1 || {
      log_err "bootstrap" "Invalid --base-ref: $AR_BASE_REF"
      exit 1
    }
  fi

  if (( VERIFY_ONLY == 0 )) && (( DRY_RUN == 0 )); then
    require_command codex
    ensure_timeout_command
    preflight_commit_identity
  fi

  preflight_selected_tools
}

cleanup_active_state() {
  local exit_code="$1"
  local active_batch active_attempt

  [[ -n "${RUN_ID:-}" ]] || return 0
  active_batch="$(current_batch_name "$RUN_ID" 2>/dev/null || true)"
  active_attempt="$(current_batch_attempt "$RUN_ID" 2>/dev/null || true)"

  [[ -n "$active_batch" ]] || return 0

  cleanup_batch_tmp_targets "$RUN_ID" "$active_batch"
  cleanup_worktree_rust_artifacts "${WORKTREE:-}"

  if (( exit_code != 0 )); then
    record_status "$RUN_ID" "$active_batch" "${active_attempt:-?}" "runner_exit" "runner exiting with code $exit_code"
  fi
}

handle_exit() {
  local exit_code="$?"
  if (( RUNNER_EXIT_TRAPPED == 1 )); then
    return "$exit_code"
  fi
  RUNNER_EXIT_TRAPPED=1
  cleanup_active_state "$exit_code"
  return "$exit_code"
}

trap 'exit 130' INT
trap 'exit 143' TERM
trap 'handle_exit' EXIT

create_run() {
  RUN_ID="run-$(date +%Y%m%d-%H%M%S)-$$"
  WORKTREE="$WORKTREE_ROOT/agent-registry-$RUN_ID"
  BRANCH="codex/agent-registry-$RUN_ID"

  git -C "$ROKO_ROOT" worktree add -b "$BRANCH" "$WORKTREE" "$AR_BASE_REF" >/dev/null
  ensure_dir "$LOG_ROOT/$RUN_ID"
  ensure_dir "$(run_prompts_dir "$RUN_ID")"
  : > "$(run_status_file "$RUN_ID")"

  if ! cat > "$(run_manifest_file "$RUN_ID")" <<EOF
RUN_ID='$RUN_ID'
WORKTREE='$WORKTREE'
BRANCH='$BRANCH'
AR_MODEL='$AR_MODEL'
AR_REASONING='$AR_REASONING'
AR_TIMEOUT='$AR_TIMEOUT'
AR_MAX_RETRIES='$AR_MAX_RETRIES'
AR_BASE_REF='$AR_BASE_REF'
AR_MAX_BATCHES='$AR_MAX_BATCHES'
AR_CODEX_FAST_PROFILE='$AR_CODEX_FAST_PROFILE'
CREATED_AT='$(date -Iseconds)'
EOF
  then
    rm -rf "$LOG_ROOT/$RUN_ID"
    git -C "$ROKO_ROOT" worktree remove --force "$WORKTREE" >/dev/null 2>&1 || true
    log_err "runner" "Failed to create manifest for $RUN_ID"
    exit 1
  fi
  link_latest_run "$RUN_ID"
}

create_dry_run() {
  RUN_ID="dry-run-$(date +%Y%m%d-%H%M%S)-$$"
  WORKTREE="$WORKTREE_ROOT/agent-registry-$RUN_ID"
  BRANCH="(not-created)"
  ensure_dir "$LOG_ROOT/$RUN_ID"
  ensure_dir "$(run_prompts_dir "$RUN_ID")"
  : > "$(run_status_file "$RUN_ID")"
  link_latest_run "$RUN_ID"
  cat > "$(run_manifest_file "$RUN_ID")" <<EOF
RUN_ID='$RUN_ID'
WORKTREE='$WORKTREE'
BRANCH='$BRANCH'
AR_MODEL='$AR_MODEL'
AR_REASONING='$AR_REASONING'
AR_TIMEOUT='$AR_TIMEOUT'
AR_MAX_RETRIES='$AR_MAX_RETRIES'
AR_BASE_REF='$AR_BASE_REF'
AR_MAX_BATCHES='$AR_MAX_BATCHES'
AR_CODEX_FAST_PROFILE='$AR_CODEX_FAST_PROFILE'
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

  RUN_ID="$CONTINUE_RUN"
  WORKTREE="$(manifest_value "$manifest" "WORKTREE")"
  BRANCH="$(manifest_value "$manifest" "BRANCH")"
  AR_MODEL="$(manifest_value "$manifest" "AR_MODEL")"
  AR_REASONING="$(manifest_value "$manifest" "AR_REASONING")"
  AR_TIMEOUT="$(manifest_value "$manifest" "AR_TIMEOUT")"
  AR_MAX_RETRIES="$(manifest_value "$manifest" "AR_MAX_RETRIES")"
  AR_BASE_REF="$(manifest_value "$manifest" "AR_BASE_REF")"
  AR_MAX_BATCHES="$(manifest_value "$manifest" "AR_MAX_BATCHES")"
  AR_CODEX_FAST_PROFILE="$(manifest_value "$manifest" "AR_CODEX_FAST_PROFILE")"

  require_uint "AR_TIMEOUT" "$AR_TIMEOUT"
  require_uint "AR_MAX_RETRIES" "$AR_MAX_RETRIES"
  require_uint "AR_MAX_BATCHES" "$AR_MAX_BATCHES"

  [[ -n "$WORKTREE" && -n "$BRANCH" ]] || {
    log_err "cli" "Manifest for run $RUN_ID is missing required values"
    exit 1
  }

  if [[ ! -d "$WORKTREE" ]]; then
    log_err "cli" "Worktree missing for run $RUN_ID: $WORKTREE"
    exit 1
  fi

  git -C "$WORKTREE" rev-parse --is-inside-work-tree >/dev/null 2>&1 || {
    log_err "cli" "Resume path is not a git worktree: $WORKTREE"
    exit 1
  }

  [[ "$(git -C "$WORKTREE" rev-parse --show-toplevel)" == "$WORKTREE" ]] || {
    log_err "cli" "Resume path does not match recorded worktree root: $WORKTREE"
    exit 1
  }

  [[ "$(git -C "$WORKTREE" branch --show-current)" == "$BRANCH" ]] || {
    log_err "cli" "Resume branch mismatch for $RUN_ID: expected $BRANCH"
    exit 1
  }

  link_latest_run "$RUN_ID"
}

batch_status() {
  local batch="$1"
  local result_file
  result_file="$(run_result_file "$RUN_ID" "$batch")"
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
  local result_file prior_failure_file attempt_failure_file existing attempt spawn_rc commit_rc total_attempts
  result_file="$(run_result_file "$RUN_ID" "$batch")"
  total_attempts="$(attempt_total)"

  existing="$(batch_status "$batch")"
  if [[ -n "$existing" ]] && success_status "$existing" && (( FORCE == 0 )) && (( VERIFY_ONLY == 0 )); then
    log_info "$batch" "Already successful; skipping"
    return 0
  fi

  if (( DRY_RUN == 1 )); then
    compose_prompt_snapshot "$batch" "$RUN_ID" "dry-run" /dev/null >/dev/null
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
      record_status "$RUN_ID" "$batch" "verify-only" "verify_only" "verification passed; not marked complete"
      log_ok "$batch" "Verification passed (verify-only)"
      return 0
    fi
    echo "verify_failed" > "$result_file"
    return 1
  fi

  local preserve_dirty_resume=0
  local preserve_failed_state=0
  local preserve_failed_attempt=0
  local start_attempt=1
  prior_failure_file=""

  if resume_preserved_batch "$batch"; then
    preserve_dirty_resume=1
    start_attempt="$(current_batch_attempt "$RUN_ID" 2>/dev/null || echo 1)"
    log_warn "$batch" "Resuming interrupted batch from preserved dirty worktree state"
  fi

  for attempt in $(seq "$start_attempt" "$total_attempts"); do
    attempt_failure_file="$(run_failure_file "$RUN_ID" "$batch" "$attempt")"
    set_current_batch "$RUN_ID" "$batch" "$attempt"
    log_header "$batch ATTEMPT $attempt/$total_attempts"

    if (( preserve_dirty_resume == 1 && attempt == start_attempt )); then
      record_status "$RUN_ID" "$batch" "$attempt" "attempt_resumed" "reusing dirty worktree state from interrupted run"
    elif (( preserve_failed_state == 1 && attempt == preserve_failed_attempt + 1 )); then
      record_status "$RUN_ID" "$batch" "$attempt" "attempt_preserved_retry" "reusing dirty worktree state from failed prior attempt"
    else
      preserve_failed_state=0
      if worktree_dirty "$WORKTREE"; then
        backup_worktree_state "$RUN_ID" "$batch" "$attempt" "$WORKTREE" "pre-reset"
        record_status "$RUN_ID" "$batch" "$attempt" "attempt_backed_up" "saved dirty worktree before reset"
      fi
      record_status "$RUN_ID" "$batch" "$attempt" "attempt_started" "$(batch_title "$batch")"
      if ! reset_runner_worktree "$WORKTREE"; then
        echo "spawn_failed" > "$result_file"
        record_status "$RUN_ID" "$batch" "$attempt" "attempt_failed" "failed to reset worktree to a clean state"
        write_failure_summary "$batch" "$RUN_ID" "$attempt" "Worktree reset failed or left dirty state."
        return 1
      fi
    fi

    spawn_rc=0
    if spawn_batch "$batch" "$RUN_ID" "$WORKTREE" "$attempt" "$prior_failure_file"; then
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
            return 1
          fi
        fi
        clear_current_batch "$RUN_ID"
        return 0
      fi
      echo "verify_failed" > "$result_file"
      record_status "$RUN_ID" "$batch" "$attempt" "attempt_failed" "verification failed"
      if (( attempt < total_attempts )) && worktree_dirty "$WORKTREE"; then
        backup_worktree_state "$RUN_ID" "$batch" "$attempt" "$WORKTREE" "verify-failed"
        record_status "$RUN_ID" "$batch" "$attempt" "attempt_backed_up" "saved failed verify state for retry"
        preserve_failed_state=1
        preserve_failed_attempt="$attempt"
      fi
    else
      spawn_rc=$?
      if [[ "$spawn_rc" -eq 124 ]]; then
        echo "timeout" > "$result_file"
        record_status "$RUN_ID" "$batch" "$attempt" "attempt_failed" "codex timed out"
        write_failure_summary "$batch" "$RUN_ID" "$attempt" "Codex timed out."
      else
        echo "spawn_failed" > "$result_file"
        record_status "$RUN_ID" "$batch" "$attempt" "attempt_failed" "codex exited unsuccessfully"
        write_failure_summary "$batch" "$RUN_ID" "$attempt" "Codex exited unsuccessfully."
      fi
      if (( attempt < total_attempts )) && worktree_dirty "$WORKTREE"; then
        backup_worktree_state "$RUN_ID" "$batch" "$attempt" "$WORKTREE" "spawn-failed"
        record_status "$RUN_ID" "$batch" "$attempt" "attempt_backed_up" "saved failed spawn state for retry"
        preserve_failed_state=1
        preserve_failed_attempt="$attempt"
      fi
    fi

    cleanup_batch_tmp_targets "$RUN_ID" "$batch"
    cleanup_worktree_rust_artifacts "$WORKTREE"
    prior_failure_file="$attempt_failure_file"

    if (( attempt < total_attempts )); then
      if (( preserve_failed_state == 1 )); then
        log_warn "$batch" "Retrying with preserved failed-attempt changes"
      else
        log_warn "$batch" "Retrying after failure"
      fi
    fi
  done
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
  printf '\n'
  printf '  success=%d  failed=%d  other=%d\n' "$success" "$fail" "$other"
  printf '  run_id=%s\n' "$RUN_ID"
  printf '  worktree=%s\n' "$WORKTREE"
  printf '  branch=%s\n' "$BRANCH"
  printf '  logs=%s\n' "$LOG_ROOT/$RUN_ID"
}

if (( LIST_ONLY == 1 )); then
  preflight_check
  list_batches
  exit 0
fi

mapfile -t SELECTED < <(select_batches)
preflight_check
preflight_runtime

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

log_info "runner" "Model: $AR_MODEL (codex mode: $(emit_codex_mode_summary), timeout: $AR_TIMEOUT s, retries: $AR_MAX_RETRIES, total attempts: $(attempt_total))"
log_info "runner" "Max batches per run: ${AR_MAX_BATCHES} (0 = unlimited)"
log_info "runner" "Selected batches: $(printf '%s,' "${SELECTED[@]}" | sed 's/,$//')"

batch_failed=0
processed=0
for batch in "${SELECTED[@]}"; do
  if (( AR_MAX_BATCHES > 0 )) && (( processed >= AR_MAX_BATCHES )); then
    log_warn "runner" "Reached AR_MAX_BATCHES=$AR_MAX_BATCHES; stopping for this session"
    break
  fi

  if deps_satisfied "$batch"; then
    processed=$((processed + 1))
    if ! run_one_batch "$batch"; then
      batch_failed=1
      break
    fi
  elif deps_terminal_failure "$batch"; then
    log_warn "$batch" "Blocked by failed dependency: $(batch_deps "$batch")"
    echo "blocked_failed_dep" > "$(run_result_file "$RUN_ID" "$batch")"
  else
    log_warn "$batch" "Skipping for now; dependencies not yet satisfied"
    echo "waiting_on_deps" > "$(run_result_file "$RUN_ID" "$batch")"
  fi
done

print_summary
exit "$batch_failed"
