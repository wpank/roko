#!/usr/bin/env bash

set -euo pipefail
IFS=$'\n\t'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/spawn.sh"
source "$SCRIPT_DIR/lib/verify.sh"

: "${REF_MODEL:=gpt-5.4}"
: "${REF_REASONING:=high}"
: "${REF_TIMEOUT:=5400}"
: "${REF_MAX_RETRIES:=2}"
: "${REF_BASE_REF:=HEAD}"
: "${REF_MAX_BATCHES:=0}"           # 0 = unlimited per run

DRY_RUN=0
FORCE=0
VERIFY_ONLY=0
LIST_ONLY=0
CONTINUE_RUN=""
SELECTED_BATCHES=()
SELECTED_GROUPS=()

print_usage() {
  cat <<'EOF'
run-refinements.sh — overnight Codex runner for tmp/refinements-runner

Propagates the 35 refinements in tmp/refinements/ into the canonical docs
tree under docs/. Docs-only — never touches code.

Usage:
  bash tmp/refinements-runner/run-refinements.sh
  bash tmp/refinements-runner/run-refinements.sh --only REF01,REF02
  bash tmp/refinements-runner/run-refinements.sh --group foundation
  bash tmp/refinements-runner/run-refinements.sh --continue last
  bash tmp/refinements-runner/run-refinements.sh --dry-run --only REF01
  bash tmp/refinements-runner/run-refinements.sh --verify-only --continue last
  bash tmp/refinements-runner/run-refinements.sh --list

Options:
  --only LIST         Comma-separated batch ids (REF01-REF35)
  --group LIST        Comma-separated groups: foundation, learning, moat,
                      ux-core, ux-surface, integrator
  --continue RUN      Continue a prior run id, or 'last'
  --dry-run           Show what would run; no Codex spawn; does not update latest run
  --force             Re-run even successful batches
  --verify-only       Skip Codex, only run verify gates; does not mark completion
  --list              Show batch manifest + exit
  --model MODEL       Override model (default: gpt-5.4)
  --reasoning LEVEL   Override reasoning (default: high)
  --timeout SECONDS   Per-batch timeout (default: 5400 = 90 min)
  --retries N         Automatic retries per batch (default: 2)
  --base-ref REF      Base git ref for a new worktree (default: HEAD)
  --max-batches N     Hard cap on batches per run (default: 0 = unlimited)

Environment overrides:
  REF_MODEL, REF_REASONING, REF_TIMEOUT, REF_MAX_RETRIES, REF_BASE_REF,
  REF_MAX_BATCHES, REF_LINK_CHECK_STRICT, NO_COLOR
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
    --model) REF_MODEL="$2"; shift 2 ;;
    --model=*) REF_MODEL="${1#*=}"; shift ;;
    --reasoning) REF_REASONING="$2"; shift 2 ;;
    --reasoning=*) REF_REASONING="${1#*=}"; shift ;;
    --timeout) REF_TIMEOUT="$2"; shift 2 ;;
    --timeout=*) REF_TIMEOUT="${1#*=}"; shift ;;
    --retries) REF_MAX_RETRIES="$2"; shift 2 ;;
    --retries=*) REF_MAX_RETRIES="${1#*=}"; shift ;;
    --base-ref) REF_BASE_REF="$2"; shift 2 ;;
    --base-ref=*) REF_BASE_REF="${1#*=}"; shift ;;
    --max-batches) REF_MAX_BATCHES="$2"; shift 2 ;;
    --max-batches=*) REF_MAX_BATCHES="${1#*=}"; shift ;;
    -h|--help) print_usage; exit 0 ;;
    *) log_err "cli" "Unknown argument: $1"; print_usage; exit 1 ;;
  esac
done

if (( DRY_RUN == 1 )) && [[ -n "$CONTINUE_RUN" ]]; then
  log_err "cli" "--dry-run cannot be combined with --continue"
  exit 1
fi

group_contains() {
  local needle="$1"
  shift
  local g
  for g in "$@"; do
    [[ "$g" == "$needle" ]] && return 0
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
      if (( found == 0 )); then
        log_err "cli" "Unknown batch: $raw"
        exit 1
      fi
    done
  elif [[ ${#SELECTED_GROUPS[@]} -gt 0 ]]; then
    for batch in "${ALL_BATCHES[@]}"; do
      group="$(batch_group "$batch")"
      if group_contains "$group" "${SELECTED_GROUPS[@]}"; then
        pool+=("$batch")
      fi
    done
  else
    pool=("${ALL_BATCHES[@]}")
  fi

  local candidate raw
  for candidate in "${ALL_BATCHES[@]}"; do
    for raw in "${pool[@]}"; do
      if [[ "$candidate" == "$raw" ]]; then
        echo "$candidate"
      fi
    done
  done
}

list_batches() {
  printf '%s%-7s %-78s %-12s %s%s\n' \
    "$C_BOLD" "ID" "TITLE" "GROUP" "DEPS" "$C_RESET"
  local batch
  for batch in "${ALL_BATCHES[@]}"; do
    printf '%-7s %-78s %-12s %s\n' \
      "$batch" \
      "$(batch_title "$batch")" \
      "$(batch_group "$batch")" \
      "$(batch_deps "$batch")"
  done
}

create_run() {
  RUN_ID="run-$(date +%Y%m%d-%H%M%S)"
  WORKTREE="$WORKTREE_ROOT/refinements-$RUN_ID"
  BRANCH="codex/refinements-$RUN_ID"

  ensure_dir "$LOG_ROOT/$RUN_ID"
  ensure_dir "$(run_prompts_dir "$RUN_ID")"
  : > "$(run_status_file "$RUN_ID")"
  link_latest_run "$RUN_ID"
  git -C "$ROKO_ROOT" worktree add -b "$BRANCH" "$WORKTREE" "$REF_BASE_REF" >/dev/null

  cat > "$(run_manifest_file "$RUN_ID")" <<EOF
RUN_ID='$RUN_ID'
WORKTREE='$WORKTREE'
BRANCH='$BRANCH'
REF_MODEL='$REF_MODEL'
REF_REASONING='$REF_REASONING'
REF_TIMEOUT='$REF_TIMEOUT'
REF_MAX_RETRIES='$REF_MAX_RETRIES'
REF_BASE_REF='$REF_BASE_REF'
REF_MAX_BATCHES='$REF_MAX_BATCHES'
CREATED_AT='$(date -Iseconds)'
EOF
}

create_dry_run() {
  RUN_ID="dry-run-$(date +%Y%m%d-%H%M%S)"
  WORKTREE="$WORKTREE_ROOT/refinements-$RUN_ID"
  BRANCH="(not-created)"
  ensure_dir "$LOG_ROOT/$RUN_ID"
  ensure_dir "$(run_prompts_dir "$RUN_ID")"
  : > "$(run_status_file "$RUN_ID")"

  cat > "$(run_manifest_file "$RUN_ID")" <<EOF
RUN_ID='$RUN_ID'
WORKTREE='(not-created)'
BRANCH='(not-created)'
REF_MODEL='$REF_MODEL'
REF_REASONING='$REF_REASONING'
REF_TIMEOUT='$REF_TIMEOUT'
REF_MAX_RETRIES='$REF_MAX_RETRIES'
REF_BASE_REF='$REF_BASE_REF'
REF_MAX_BATCHES='$REF_MAX_BATCHES'
CREATED_AT='$(date -Iseconds)'
DRY_RUN=1
EOF
}

resume_run() {
  local target="$1"
  if [[ "$target" == "last" ]]; then
    target="$(latest_run_id)" || {
      log_err "cli" "No prior run to continue"
      exit 1
    }
  fi
  local manifest
  manifest="$(run_manifest_file "$target")"
  if [[ ! -f "$manifest" ]]; then
    log_err "cli" "Missing manifest: $manifest"
    exit 1
  fi
  # shellcheck disable=SC1090
  source "$manifest"
  RUN_ID="$target"
  # Re-link latest to the resumed run
  link_latest_run "$RUN_ID"
}

batch_completed() {
  local run_id="$1"
  local batch="$2"
  local result
  result="$(run_result_file "$run_id" "$batch")"
  [[ -f "$result" ]] || return 1
  local status
  status="$(cat "$result")"
  success_status "$status"
}

batch_dep_status() {
  local run_id="$1"
  local batch="$2"
  # Need to split the space-separated dep list into individual tokens.
  # The script sets IFS=$'\n\t' at the top (no space), so a bare
  # `for dep in $(batch_deps ...)` would treat "REF02 REF03" as ONE
  # token. Use an explicit `read -ra` with space IFS instead.
  local -a deps=()
  local deps_str
  deps_str="$(batch_deps "$batch")"
  [[ -z "$deps_str" ]] && { echo ""; return 0; }
  IFS=' ' read -ra deps <<< "$deps_str"
  local dep
  for dep in "${deps[@]}"; do
    [[ -z "$dep" ]] && continue
    if ! batch_completed "$run_id" "$dep"; then
      echo "$dep"
      return 0
    fi
  done
  echo ""
}

execute_batch() {
  local batch="$1"
  local run_id="$2"
  local worktree="$3"

  local blocker
  blocker="$(batch_dep_status "$run_id" "$batch")"
  if [[ -n "$blocker" ]]; then
    record_status "$run_id" "$batch" 0 "blocked" "waiting on $blocker"
    echo "blocked" > "$(run_result_file "$run_id" "$batch")"
    log_warn "$batch" "Blocked by dependency: $blocker"
    return 2
  fi

  local attempt=1
  local failure_file
  failure_file="$(run_failure_file "$run_id" "$batch")"
  : > "$failure_file"

  while (( attempt <= REF_MAX_RETRIES + 1 )); do
    set_current_batch "$run_id" "$batch" "$attempt"
    log_header "$batch — attempt $attempt — $(batch_title "$batch")"

    if (( VERIFY_ONLY == 1 )); then
      if verify_batch "$batch" "$run_id" "$worktree" "$attempt"; then
        log_ok "$batch" "Verify-only: passed"
        return 0
      else
        log_err "$batch" "Verify-only: failed"
        return 1
      fi
    fi

    local spawn_rc=0
    spawn_batch "$batch" "$run_id" "$worktree" "$attempt" "$failure_file" || spawn_rc=$?

    if (( spawn_rc == 124 )); then
      echo "timeout" > "$(run_result_file "$run_id" "$batch")"
      backup_worktree_state "$run_id" "$batch" "$attempt" "$worktree" "timeout"
      reset_runner_worktree "$worktree"
      attempt=$((attempt + 1))
      continue
    fi
    if (( spawn_rc != 0 )); then
      echo "spawn_failed" > "$(run_result_file "$run_id" "$batch")"
      backup_worktree_state "$run_id" "$batch" "$attempt" "$worktree" "spawn_failed"
      reset_runner_worktree "$worktree"
      attempt=$((attempt + 1))
      continue
    fi

    if ! verify_batch "$batch" "$run_id" "$worktree" "$attempt"; then
      backup_worktree_state "$run_id" "$batch" "$attempt" "$worktree" "verify_failed"
      echo "verify_failed" > "$(run_result_file "$run_id" "$batch")"
      # Preserve worktree for retry; the failure_file carries the context.
      attempt=$((attempt + 1))
      continue
    fi

    if ! commit_batch_if_needed "$batch" "$worktree" "$run_id" "$attempt"; then
      echo "commit_failed" > "$(run_result_file "$run_id" "$batch")"
      record_status "$run_id" "$batch" "$attempt" "commit_failed" "no changes staged"
      return 1
    fi

    echo "success" > "$(run_result_file "$run_id" "$batch")"
    clear_current_batch "$run_id"
    return 0
  done

  log_err "$batch" "Exhausted all retries"
  echo "timeout" > "$(run_result_file "$run_id" "$batch")"
  return 1
}

main() {
  if (( LIST_ONLY == 1 )); then
    list_batches
    exit 0
  fi

  if preflight_check; then :; else
    log_err "main" "Preflight errors — aborting"
    exit 1
  fi

  if [[ -n "$CONTINUE_RUN" ]]; then
    resume_run "$CONTINUE_RUN"
    log_info "main" "Resuming run $RUN_ID (worktree=$WORKTREE, branch=$BRANCH)"
  elif (( DRY_RUN == 1 )); then
    create_dry_run
    log_info "main" "Dry-run $RUN_ID"
  else
    create_run
    log_info "main" "Started run $RUN_ID (worktree=$WORKTREE, branch=$BRANCH)"
  fi

  local -a batches
  mapfile -t batches < <(select_batches)

  if [[ ${#batches[@]} -eq 0 ]]; then
    log_warn "main" "No batches selected"
    exit 0
  fi

  log_info "main" "Selected ${#batches[@]} batch(es): ${batches[*]}"

  if (( DRY_RUN == 1 )); then
    local b
    for b in "${batches[@]}"; do
      local snap
      snap="$(compose_prompt_snapshot "$b" "$RUN_ID" 1 /dev/null)"
      log_info "$b" "Prompt snapshot: $snap ($(wc -l < "$snap") lines)"
    done
    log_ok "main" "Dry-run complete; no Codex invoked"
    exit 0
  fi

  local done_count=0
  local failed=0
  local batch
  for batch in "${batches[@]}"; do
    if (( REF_MAX_BATCHES > 0 )) && (( done_count >= REF_MAX_BATCHES )); then
      log_warn "main" "Hit --max-batches=$REF_MAX_BATCHES limit"
      break
    fi

    if (( FORCE == 0 )) && batch_completed "$RUN_ID" "$batch"; then
      log_info "$batch" "Already completed successfully — skipping"
      continue
    fi

    local rc=0
    execute_batch "$batch" "$RUN_ID" "$WORKTREE" || rc=$?
    done_count=$((done_count + 1))
    if (( rc != 0 )); then
      failed=$((failed + 1))
      if (( rc == 2 )); then
        # blocked; don't count as a terminal failure, try later batches
        failed=$((failed - 1))
      fi
    fi
  done

  log_header "SUMMARY"
  log_info "main" "Run id: $RUN_ID"
  log_info "main" "Worktree: $WORKTREE"
  log_info "main" "Branch: $BRANCH"
  log_info "main" "Executed: $done_count, failed: $failed"

  if (( failed > 0 )); then
    exit 1
  fi
  exit 0
}

main "$@"
