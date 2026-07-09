#!/usr/bin/env bash
# generate-infrastructure.sh — Generate the runner infrastructure for docs-parity2.
#
# Produces:
#   docs-parity2/run-docs-parity2.sh  — main orchestrator
#   docs-parity2/lib/common.sh        — batch metadata
#   docs-parity2/lib/spawn.sh         — codex invocation
#   docs-parity2/lib/verify.sh        — verification + commit
#   docs-parity2/BATCHES.md           — batch manifest

set -uo pipefail

_GEN_INFRA_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Guard: only source section-map if not already loaded
if [[ -z "${_SECTION_MAP_LOADED:-}" ]]; then
  source "$_GEN_INFRA_DIR/section-map.sh"
fi

: "${ROKO_ROOT:=/Users/will/dev/nunchi/roko/roko}"
: "${OUT_ROOT:=$ROKO_ROOT/tmp/docs-parity2}"

generate_infrastructure() {
  mkdir -p "$OUT_ROOT/lib"

  echo "  run-docs-parity2.sh"
  _gen_runner > "$OUT_ROOT/run-docs-parity2.sh"
  chmod +x "$OUT_ROOT/run-docs-parity2.sh"

  echo "  lib/common.sh"
  _gen_common > "$OUT_ROOT/lib/common.sh"

  echo "  lib/spawn.sh"
  _gen_spawn > "$OUT_ROOT/lib/spawn.sh"

  echo "  lib/verify.sh"
  _gen_verify > "$OUT_ROOT/lib/verify.sh"

  echo "  BATCHES.md"
  _gen_batches_md > "$OUT_ROOT/BATCHES.md"
}

# ---------------------------------------------------------------------------
# run-docs-parity2.sh
# ---------------------------------------------------------------------------
_gen_runner() {
  cat <<'RUNNER'
#!/usr/bin/env bash

set -euo pipefail
IFS=$'\n\t'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/spawn.sh"
source "$SCRIPT_DIR/lib/verify.sh"

: "${DP_MODEL:=gpt-5.4}"
: "${DP_REASONING:=high}"
: "${DP_TIMEOUT:=5400}"
: "${DP_MAX_RETRIES:=2}"
: "${DP_BASE_REF:=HEAD}"
: "${DP_MAX_BATCHES:=0}"

DRY_RUN=0
FORCE=0
VERIFY_ONLY=0
LIST_ONLY=0
CONTINUE_RUN=""
SELECTED_BATCHES=()
SELECTED_GROUPS=()

print_usage() {
  cat <<'EOF'
run-docs-parity2.sh — overnight Codex runner for docs-parity2

Usage:
  bash tmp/docs-parity2/run-docs-parity2.sh
  bash tmp/docs-parity2/run-docs-parity2.sh --only DP00,DP01
  bash tmp/docs-parity2/run-docs-parity2.sh --group core
  bash tmp/docs-parity2/run-docs-parity2.sh --continue last
  bash tmp/docs-parity2/run-docs-parity2.sh --dry-run --only DP00
  bash tmp/docs-parity2/run-docs-parity2.sh --list

Options:
  --only LIST         Comma-separated batch ids (DP00-DP20)
  --group LIST        Comma-separated groups: core, extensions, safety-iface,
                      infra, phase2
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
  DP_MODEL, DP_REASONING, DP_TIMEOUT, DP_MAX_RETRIES, DP_BASE_REF,
  DP_MAX_BATCHES, NO_COLOR
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
    --model) DP_MODEL="$2"; shift 2 ;;
    --model=*) DP_MODEL="${1#*=}"; shift ;;
    --reasoning) DP_REASONING="$2"; shift 2 ;;
    --reasoning=*) DP_REASONING="${1#*=}"; shift ;;
    --timeout) DP_TIMEOUT="$2"; shift 2 ;;
    --timeout=*) DP_TIMEOUT="${1#*=}"; shift ;;
    --retries) DP_MAX_RETRIES="$2"; shift 2 ;;
    --retries=*) DP_MAX_RETRIES="${1#*=}"; shift ;;
    --base-ref) DP_BASE_REF="$2"; shift 2 ;;
    --base-ref=*) DP_BASE_REF="${1#*=}"; shift ;;
    --max-batches) DP_MAX_BATCHES="$2"; shift 2 ;;
    --max-batches=*) DP_MAX_BATCHES="${1#*=}"; shift ;;
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
  local g; for g in "$@"; do [[ "$g" == "$needle" ]] && return 0; done
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
        if [[ "$candidate" == "$raw" ]]; then pool+=("$candidate"); found=1; break; fi
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
  local candidate raw
  for candidate in "${ALL_BATCHES[@]}"; do
    for raw in "${pool[@]}"; do
      [[ "$candidate" == "$raw" ]] && echo "$candidate"
    done
  done
}

list_batches() {
  printf '%s%-6s %-50s %-14s %s%s\n' "$C_BOLD" "ID" "TITLE" "GROUP" "DEPS" "$C_RESET"
  local batch
  for batch in "${ALL_BATCHES[@]}"; do
    printf '%-6s %-50s %-14s %s\n' \
      "$batch" "$(batch_title "$batch")" "$(batch_group "$batch")" "$(batch_deps "$batch")"
  done
}

create_run() {
  RUN_ID="run-$(date +%Y%m%d-%H%M%S)"
  WORKTREE="$WORKTREE_ROOT/docs-parity2-$RUN_ID"
  BRANCH="codex/docs-parity2-$RUN_ID"
  ensure_dir "$LOG_ROOT/$RUN_ID"
  ensure_dir "$(run_prompts_dir "$RUN_ID")"
  : > "$(run_status_file "$RUN_ID")"
  link_latest_run "$RUN_ID"
  git -C "$ROKO_ROOT" worktree add -b "$BRANCH" "$WORKTREE" "$DP_BASE_REF" >/dev/null
  cat > "$(run_manifest_file "$RUN_ID")" <<EOF
RUN_ID='$RUN_ID'
WORKTREE='$WORKTREE'
BRANCH='$BRANCH'
DP_MODEL='$DP_MODEL'
DP_REASONING='$DP_REASONING'
DP_TIMEOUT='$DP_TIMEOUT'
DP_MAX_RETRIES='$DP_MAX_RETRIES'
DP_BASE_REF='$DP_BASE_REF'
DP_MAX_BATCHES='$DP_MAX_BATCHES'
CREATED_AT='$(date -Iseconds)'
EOF
}

create_dry_run() {
  RUN_ID="dry-run-$(date +%Y%m%d-%H%M%S)"
  WORKTREE="$WORKTREE_ROOT/docs-parity2-$RUN_ID"
  BRANCH="(not-created)"
  ensure_dir "$LOG_ROOT/$RUN_ID"
  ensure_dir "$(run_prompts_dir "$RUN_ID")"
  : > "$(run_status_file "$RUN_ID")"
  link_latest_run "$RUN_ID"
  cat > "$(run_manifest_file "$RUN_ID")" <<EOF
RUN_ID='$RUN_ID'
WORKTREE='$WORKTREE'
BRANCH='$BRANCH'
DP_MODEL='$DP_MODEL'
DP_REASONING='$DP_REASONING'
DP_TIMEOUT='$DP_TIMEOUT'
DP_MAX_RETRIES='$DP_MAX_RETRIES'
DP_BASE_REF='$DP_BASE_REF'
DP_MAX_BATCHES='$DP_MAX_BATCHES'
CREATED_AT='$(date -Iseconds)'
EOF
}

load_run() {
  if [[ "$CONTINUE_RUN" == "last" ]]; then
    CONTINUE_RUN="$(latest_run_id || true)"
  fi
  if [[ -z "$CONTINUE_RUN" ]]; then
    log_err "cli" "No prior run available to continue"; exit 1
  fi
  local manifest; manifest="$(run_manifest_file "$CONTINUE_RUN")"
  if [[ ! -f "$manifest" ]]; then
    log_err "cli" "Missing manifest for run: $CONTINUE_RUN"; exit 1
  fi
  # shellcheck disable=SC1090
  source "$manifest"
  RUN_ID="$CONTINUE_RUN"
  if [[ ! -d "$WORKTREE" ]]; then
    log_err "cli" "Worktree missing for run $RUN_ID: $WORKTREE"; exit 1
  fi
  link_latest_run "$RUN_ID"
}

batch_status() {
  local result_file; result_file="$(run_result_file "$RUN_ID" "$1")"
  [[ -f "$result_file" ]] && cat "$result_file" || true
}

deps_satisfied() {
  local dep status; local -a deps=()
  IFS=' ' read -r -a deps <<< "$(batch_deps "$1")"
  for dep in "${deps[@]}"; do
    [[ -z "$dep" ]] && continue
    status="$(batch_status "$dep")"
    success_status "$status" || return 1
  done; return 0
}

deps_terminal_failure() {
  local dep status; local -a deps=()
  IFS=' ' read -r -a deps <<< "$(batch_deps "$1")"
  for dep in "${deps[@]}"; do
    [[ -z "$dep" ]] && continue
    status="$(batch_status "$dep")"
    terminal_failure_status "$status" && return 0
  done; return 1
}

resume_preserved_batch() {
  local batch="$1"
  [[ -n "$CONTINUE_RUN" ]] || return 1
  local current_batch; current_batch="$(current_batch_name "$RUN_ID" 2>/dev/null || true)"
  [[ "$current_batch" == "$batch" ]] || return 1
  local result; result="$(batch_status "$batch")"
  success_status "$result" && return 1
  worktree_dirty "$WORKTREE"
}

run_one_batch() {
  local batch="$1"
  local result_file log_file failure_file
  result_file="$(run_result_file "$RUN_ID" "$batch")"
  log_file="$(run_log_file "$RUN_ID" "$batch")"
  failure_file="$(run_failure_file "$RUN_ID" "$batch")"

  local existing; existing="$(batch_status "$batch")"
  if [[ -n "$existing" ]] && success_status "$existing" && (( FORCE == 0 )) && (( VERIFY_ONLY == 0 )); then
    log_info "$batch" "Already successful; skipping"; return 0
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
      record_status "$RUN_ID" "$batch" "verify-only" "verify_only" "verification passed"
      log_ok "$batch" "Verification passed (verify-only)"; return 0
    fi
    echo "verify_failed" > "$result_file"; return 1
  fi

  local attempt spawn_rc commit_rc
  local preserve_dirty_resume=0 preserve_failed_state=0 start_attempt=1

  if resume_preserved_batch "$batch"; then
    preserve_dirty_resume=1
    start_attempt="$(current_batch_attempt "$RUN_ID" 2>/dev/null || echo 1)"
    log_warn "$batch" "Resuming interrupted batch from preserved dirty worktree state"
  fi

  : > "$failure_file"
  for attempt in $(seq "$start_attempt" "$DP_MAX_RETRIES"); do
    set_current_batch "$RUN_ID" "$batch" "$attempt"
    log_header "$batch ATTEMPT $attempt/$DP_MAX_RETRIES"
    if (( preserve_dirty_resume == 1 && attempt == start_attempt )); then
      record_status "$RUN_ID" "$batch" "$attempt" "attempt_resumed" "reusing dirty worktree"
      log_info "$batch" "Keeping current worktree changes; skipping reset"
    elif (( preserve_failed_state == 1 && attempt == start_attempt + 1 )); then
      record_status "$RUN_ID" "$batch" "$attempt" "attempt_preserved_retry" "reusing failed state"
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
            record_status "$RUN_ID" "$batch" "$attempt" "attempt_succeeded" "verified with no new changes"
          else
            echo "commit_failed" > "$result_file"
            record_status "$RUN_ID" "$batch" "$attempt" "commit_failed" "commit step failed"
            write_failure_summary "$batch" "$RUN_ID" "Commit step failed."
            return 1
          fi
        fi
        clear_current_batch "$RUN_ID"; return 0
      fi
      echo "verify_failed" > "$result_file"
      record_status "$RUN_ID" "$batch" "$attempt" "attempt_failed" "verification failed"
      if (( attempt < DP_MAX_RETRIES )) && worktree_dirty "$WORKTREE"; then
        backup_worktree_state "$RUN_ID" "$batch" "$attempt" "$WORKTREE" "verify-failed"
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
      if (( attempt < DP_MAX_RETRIES )) && worktree_dirty "$WORKTREE"; then
        backup_worktree_state "$RUN_ID" "$batch" "$attempt" "$WORKTREE" "spawn-failed"
        preserve_failed_state=1
      fi
    fi

    if (( attempt < DP_MAX_RETRIES )); then
      if (( preserve_failed_state == 1 )); then
        log_warn "$batch" "Retrying with preserved failed-attempt changes"
      else
        log_warn "$batch" "Retrying after failure"
      fi
    fi
  done
  # All retries exhausted — clean up tmp targets to prevent disk fill
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
    if success_status "$result"; then success=$((success + 1))
    elif terminal_failure_status "$result"; then fail=$((fail + 1))
    else other=$((other + 1)); fi
  done
  printf '\n  success=%d  failed=%d  other=%d\n' "$success" "$fail" "$other"
  printf '  run_id=%s\n  worktree=%s\n  branch=%s\n  logs=%s\n' \
    "$RUN_ID" "$WORKTREE" "$BRANCH" "$LOG_ROOT/$RUN_ID"
}

preflight_check

if (( LIST_ONLY == 1 )); then list_batches; exit 0; fi

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

log_info "runner" "Model: $DP_MODEL (reasoning: $DP_REASONING, timeout: $DP_TIMEOUT s, retries: $DP_MAX_RETRIES)"
log_info "runner" "Max batches per run: ${DP_MAX_BATCHES} (0 = unlimited)"
log_info "runner" "Selected batches: $(printf '%s,' "${SELECTED[@]}" | sed 's/,$//')"

batch_failed=0
processed=0
for batch in "${SELECTED[@]}"; do
  if (( DP_MAX_BATCHES > 0 )) && (( processed >= DP_MAX_BATCHES )); then
    log_warn "runner" "Reached DP_MAX_BATCHES=$DP_MAX_BATCHES; stopping"; break
  fi
  if deps_satisfied "$batch"; then
    processed=$((processed + 1))
    run_one_batch "$batch" || batch_failed=1
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
RUNNER
}

# ---------------------------------------------------------------------------
# lib/common.sh
# ---------------------------------------------------------------------------
_gen_common() {
  # Header
  cat <<'COMMON_HEADER'
#!/usr/bin/env bash

set -uo pipefail

: "${ROKO_ROOT:=/Users/will/dev/nunchi/roko/roko}"
: "${DP_ROOT:=$ROKO_ROOT/tmp/docs-parity2}"
: "${LOG_ROOT:=$DP_ROOT/logs}"
: "${PROMPTS_DIR:=$DP_ROOT/prompts}"
: "${CONTEXT_DIR:=$DP_ROOT/context-pack}"
: "${WORKTREE_ROOT:=$ROKO_ROOT/.roko/worktrees}"

if [[ -t 1 && -z "${NO_COLOR:-}" ]]; then
  C_RESET=$'\e[0m' C_BOLD=$'\e[1m' C_DIM=$'\e[2m'
  C_RED=$'\e[31m' C_GREEN=$'\e[32m' C_YELLOW=$'\e[33m'
  C_BLUE=$'\e[34m' C_MAGENTA=$'\e[35m' C_CYAN=$'\e[36m'
else
  C_RESET='' C_BOLD='' C_DIM='' C_RED='' C_GREEN='' C_YELLOW='' C_BLUE='' C_MAGENTA='' C_CYAN=''
fi

log_info()   { printf '%s[INFO]%s  %s%-10s%s %s\n' "$C_BLUE"   "$C_RESET" "$C_DIM" "$1" "$C_RESET" "$2"; }
log_ok()     { printf '%s[OK]%s    %s%-10s%s %s\n' "$C_GREEN"  "$C_RESET" "$C_DIM" "$1" "$C_RESET" "$2"; }
log_warn()   { printf '%s[WARN]%s  %s%-10s%s %s\n' "$C_YELLOW" "$C_RESET" "$C_DIM" "$1" "$C_RESET" "$2" >&2; }
log_err()    { printf '%s[ERR]%s   %s%-10s%s %s\n' "$C_RED"    "$C_RESET" "$C_DIM" "$1" "$C_RESET" "$2" >&2; }
log_header() { printf '\n%s=== %s ===%s\n\n' "$C_BOLD$C_MAGENTA" "$1" "$C_RESET"; }

COMMON_HEADER

  # ALL_BATCHES array
  printf 'ALL_BATCHES=(\n'
  printf '  '
  local count=0
  for batch_id in "${ALL_DP_BATCHES[@]}"; do
    printf '"%s" ' "$batch_id"
    count=$((count + 1))
    if (( count % 6 == 0 )); then
      printf '\n  '
    fi
  done
  printf '\n)\n\n'

  cat <<'COMMON_HELPERS'
success_status() {
  case "${1:-}" in success|success_noop|skipped) return 0 ;; *) return 1 ;; esac
}

terminal_failure_status() {
  case "${1:-}" in spawn_failed|verify_failed|commit_failed|timeout|blocked) return 0 ;; *) return 1 ;; esac
}

fmt_duration() {
  local s="${1:-0}" h=$((s / 3600)) m=$(((s % 3600) / 60)) sec=$((s % 60))
  if (( h > 0 )); then printf '%dh %dm %ds' "$h" "$m" "$sec"
  elif (( m > 0 )); then printf '%dm %ds' "$m" "$sec"
  else printf '%ds' "$sec"; fi
}

ensure_dir() { mkdir -p "$1"; }

require_file() {
  if [[ ! -f "$1" ]]; then log_err "bootstrap" "Missing file: $1"; exit 1; fi
}

latest_run_id() {
  [[ -d "$LOG_ROOT" ]] || return 1
  find "$LOG_ROOT" -maxdepth 1 -mindepth 1 -type d -name 'run-*' -exec test -f {}/manifest.env \; -print \
    | sort | tail -1 | sed 's|.*/||'
}

run_manifest_file()    { echo "$LOG_ROOT/$1/manifest.env"; }
run_result_file()      { echo "$LOG_ROOT/$1/$2.result"; }
run_log_file()         { echo "$LOG_ROOT/$1/$2.log"; }
run_prompts_dir()      { echo "$LOG_ROOT/$1/prompts"; }
run_prompt_snapshot()  { echo "$(run_prompts_dir "$1")/$2.prompt.md"; }
run_last_message_file(){ echo "$LOG_ROOT/$1/$2.last.txt"; }
run_failure_file()     { echo "$LOG_ROOT/$1/$2.failure.txt"; }
run_status_file()      { echo "$LOG_ROOT/$1/status.tsv"; }
run_current_batch_file(){ echo "$LOG_ROOT/$1/current-batch.env"; }
run_backups_dir()      { echo "$LOG_ROOT/$1/backups"; }
batch_prompt_file()    { echo "$PROMPTS_DIR/$1.prompt.md"; }
tmp_target_root()      { echo "${TMPDIR:-/tmp}/roko-dp-targets"; }
batch_target_dir()     { echo "$(tmp_target_root)/$1/$2/$3-attempt-$4"; }

link_latest_run() {
  local run_id="$1"
  [[ "$run_id" == dry-run-* ]] && return 0
  ln -sfn "$LOG_ROOT/$run_id" "$LOG_ROOT/latest"
}

current_batch_value() {
  local file; file="$(run_current_batch_file "$1")"
  [[ -f "$file" ]] || return 1
  awk -F= -v key="$2" '$1 == key { gsub(/\047/, "", $2); print $2 }' "$file"
}
current_batch_name()    { current_batch_value "$1" "BATCH"; }
current_batch_attempt() { current_batch_value "$1" "ATTEMPT"; }

worktree_dirty() {
  git -C "$1" status --porcelain=v1 -uall \
    | grep -Ev '^[ MADRCU?!]{2} (\.cargo-target/|target/)' \
    | grep -q .
}

record_status() {
  printf '%s\t%s\t%s\t%s\t%s\n' "$(date -Iseconds)" "$2" "$3" "$4" "${5:-}" \
    >> "$(run_status_file "$1")"
}

set_current_batch() {
  cat > "$(run_current_batch_file "$1")" <<EOF
BATCH='$2'
ATTEMPT='$3'
UPDATED_AT='$(date -Iseconds)'
EOF
}

clear_current_batch() { rm -f "$(run_current_batch_file "$1")"; }

COMMON_HELPERS

  # batch_title case statement
  printf 'batch_title() {\n  case "$1" in\n'
  for entry in "${SECTION_REGISTRY[@]}"; do
    local num slug display
    num="$(section_num "$entry")"
    slug="$(section_slug "$entry")"
    display="$(section_display "$entry")"
    local batch_id
    batch_id="$(batch_id_for "$num")"
    printf '    %s) echo "%s-%s: %s" ;;\n' "$batch_id" "$num" "$slug" "$display"
  done
  printf '    *) return 1 ;;\n  esac\n}\n\n'

  # batch_deps case statement
  printf 'batch_deps() {\n  case "$1" in\n'
  for entry in "${SECTION_REGISTRY[@]}"; do
    local num deps
    num="$(section_num "$entry")"
    deps="$(section_deps "$entry")"
    local batch_id
    batch_id="$(batch_id_for "$num")"
    if [[ -n "$deps" ]]; then
      printf '    %s) echo "%s" ;;\n' "$batch_id" "$deps"
    fi
  done
  printf '    *) echo "" ;;\n  esac\n}\n\n'

  # batch_group case statement
  printf 'batch_group() {\n  case "$1" in\n'
  local prev_group=""
  local batch_list=""
  for entry in "${SECTION_REGISTRY[@]}"; do
    local num grp batch_id
    num="$(section_num "$entry")"
    grp="$(section_group "$entry")"
    batch_id="$(batch_id_for "$num")"
    if [[ "$grp" != "$prev_group" && -n "$prev_group" ]]; then
      printf '    %s) echo "%s" ;;\n' "$batch_list" "$prev_group"
      batch_list=""
    fi
    if [[ -n "$batch_list" ]]; then
      batch_list="${batch_list}|${batch_id}"
    else
      batch_list="$batch_id"
    fi
    prev_group="$grp"
  done
  if [[ -n "$batch_list" ]]; then
    printf '    %s) echo "%s" ;;\n' "$batch_list" "$prev_group"
  fi
  printf '    *) echo "misc" ;;\n  esac\n}\n\n'

  # batch_verify_commands case statement
  printf 'batch_verify_commands() {\n  case "$1" in\n'
  for entry in "${SECTION_REGISTRY[@]}"; do
    local num batch_id
    num="$(section_num "$entry")"
    batch_id="$(batch_id_for "$num")"
    printf '    %s)\n      cat <<'"'"'EOF'"'"'\n' "$batch_id"
    verify_commands_for "$num"
    printf 'EOF\n      ;;\n'
  done
  printf '    *) return 1 ;;\n  esac\n}\n\n'

  # preflight_check
  cat <<'PREFLIGHT'
preflight_check() {
  local errors=0
  log_header "PREFLIGHT"
  if command -v codex >/dev/null 2>&1; then
    log_ok "preflight" "codex CLI: $(command -v codex)"
  else
    log_err "preflight" "codex CLI not found"; errors=$((errors + 1))
  fi
  if git -C "$ROKO_ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    log_ok "preflight" "git repo detected"
  else
    log_err "preflight" "ROKO_ROOT is not a git repo: $ROKO_ROOT"; errors=$((errors + 1))
  fi
  ensure_dir "$LOG_ROOT"
  ensure_dir "$WORKTREE_ROOT"
  local batch
  for batch in "${ALL_BATCHES[@]}"; do
    require_file "$(batch_prompt_file "$batch")"
  done
  local dirty_count
  dirty_count=$(git -C "$ROKO_ROOT" status --porcelain | wc -l | tr -d ' ')
  if (( dirty_count > 0 )); then
    log_warn "preflight" "main repo has $dirty_count uncommitted change(s)"
  else
    log_ok "preflight" "main repo is clean"
  fi
  # Check disk space — each batch needs ~12-20GB for cargo targets
  local avail_gb
  avail_gb=$(df -g "$ROKO_ROOT" 2>/dev/null | awk 'NR==2{print $4}' || echo "?")
  if [[ "$avail_gb" =~ ^[0-9]+$ ]]; then
    if (( avail_gb < 20 )); then
      log_warn "preflight" "Only ${avail_gb}GB free — batches need ~12-20GB each for cargo targets"
    else
      log_ok "preflight" "${avail_gb}GB free disk space"
    fi
  fi
  return "$errors"
}
PREFLIGHT
}

# ---------------------------------------------------------------------------
# lib/spawn.sh
# ---------------------------------------------------------------------------
_gen_spawn() {
  cat <<'SPAWN'
#!/usr/bin/env bash

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

: "${DP_MODEL:=gpt-5.4}"
: "${DP_REASONING:=high}"
: "${DP_TIMEOUT:=5400}"

emit_shared_context_pack() {
  cat <<'PACK_EOF'
## Shared Context Pack

PACK_EOF

  local file title
  for file in \
    "$CONTEXT_DIR/00-DOCS-PARITY-RULES.md" \
    "$CONTEXT_DIR/01-SECTION-CRATE-MAP.md" \
    "$CONTEXT_DIR/02-WORKSPACE-TOPOLOGY.md" \
    "$CONTEXT_DIR/03-EXISTING-PARITY-SUMMARY.md" \
    "$CONTEXT_DIR/04-CODE-CONVENTIONS.md" \
    "$CONTEXT_DIR/05-PHASE2-STUB-GUIDANCE.md"; do
    [[ -f "$file" ]] || continue
    title="$(basename "$file" .md)"
    printf '### %s\n\n' "$title"
    cat "$file"
    printf '\n'
  done
}

emit_delegation_guidance() {
  local batch="$1"
  cat <<'DELEG_EOF'
## Delegation Requirement

You are explicitly authorized to use multiple subagents for this batch.
Use them aggressively where it helps, but keep the immediate blocking work local.

Required delegation behavior:

- Before coding, form a short plan and identify 2-4 concrete sidecar subtasks.
- Spawn explorers for targeted codebase questions and workers for bounded code edits.
- Each subagent gets the same context pack (`context-pack/00-05`) plus its specific task.
- Give each worker a disjoint write scope and tell them they are not alone in the codebase.
- Do not wait idly for subagents if you can make progress locally.
- If subagents are unavailable in this environment, continue locally without failing.
DELEG_EOF
  echo
}

do_timeout() {
  local seconds="$1"; shift
  if command -v timeout >/dev/null 2>&1; then timeout "$seconds" "$@"
  elif command -v gtimeout >/dev/null 2>&1; then gtimeout "$seconds" "$@"
  else "$@"; fi
}

compose_prompt_snapshot() {
  local batch="$1" run_id="$2" attempt="$3" failure_file="$4"
  local out; out="$(run_prompt_snapshot "$run_id" "$batch")"
  ensure_dir "$(dirname "$out")"
  {
    echo "# Docs-Parity Batch $batch"
    echo
    echo "Run id: $run_id"
    echo "Attempt: $attempt"
    echo "Model: $DP_MODEL"
    echo "Reasoning: $DP_REASONING"
    echo
    if [[ -s "$failure_file" ]]; then
      echo "## Previous attempt failure context"
      echo
      cat "$failure_file"
      echo
      echo "Use that context to avoid repeating the same failure."
      echo
    fi
    emit_shared_context_pack
    emit_delegation_guidance "$batch"
    cat "$(batch_prompt_file "$batch")"
  } > "$out"
  echo "$out"
}

spawn_batch() {
  local batch="$1" run_id="$2" worktree="$3" attempt="$4" failure_file="$5"
  local prompt_snapshot log_file last_message_file target_dir
  prompt_snapshot=$(compose_prompt_snapshot "$batch" "$run_id" "$attempt" "$failure_file")
  log_file=$(run_log_file "$run_id" "$batch")
  last_message_file=$(run_last_message_file "$run_id" "$batch")
  target_dir=$(batch_target_dir "$run_id" "$batch" "codex" "$attempt")
  : > "$last_message_file"
  rm -rf "$target_dir"; mkdir -p "$target_dir"

  local start_ts exit_code=0
  start_ts=$(date +%s)
  {
    echo "=== Batch: $batch ($(batch_title "$batch")) ==="
    echo "=== Started: $(date -Iseconds) ==="
    echo "=== Worktree: $worktree ==="
    echo "=== Model: $DP_MODEL ==="
    echo "=== Reasoning: $DP_REASONING ==="
    echo "=== Timeout: $DP_TIMEOUT ==="
    echo "=== Cargo target: $target_dir ==="
    echo "=== Prompt snapshot: $prompt_snapshot ==="
    echo
  } > "$log_file"

  record_status "$run_id" "$batch" "$attempt" "spawn_started" "codex exec started"
  do_timeout "$DP_TIMEOUT" \
    env CARGO_TARGET_DIR="$target_dir" \
    codex exec \
      --model "$DP_MODEL" \
      --sandbox workspace-write \
      --full-auto \
      -c "model_reasoning_effort=$DP_REASONING" \
      --cd "$worktree" \
      -o "$last_message_file" \
      - \
      < "$prompt_snapshot" >> "$log_file" 2>&1 || exit_code=$?

  local end_ts elapsed
  end_ts=$(date +%s); elapsed=$((end_ts - start_ts))
  { echo; echo "=== Finished: $(date -Iseconds) ==="; echo "=== Duration: $(fmt_duration "$elapsed") ==="; echo "=== Exit code: $exit_code ==="; } >> "$log_file"

  if [[ "$exit_code" -eq 0 ]]; then
    record_status "$run_id" "$batch" "$attempt" "spawn_succeeded" "codex completed"
    log_ok "$batch" "Codex completed in $(fmt_duration "$elapsed")"; return 0
  fi
  if [[ "$exit_code" -eq 124 ]]; then
    record_status "$run_id" "$batch" "$attempt" "spawn_timed_out" "timed out"
    log_err "$batch" "Timed out after $(fmt_duration "$DP_TIMEOUT")"; return 124
  fi
  record_status "$run_id" "$batch" "$attempt" "spawn_failed" "exited with $exit_code"
  log_err "$batch" "Codex exited with code $exit_code"; return "$exit_code"
}
SPAWN
}

# ---------------------------------------------------------------------------
# lib/verify.sh
# ---------------------------------------------------------------------------
_gen_verify() {
  cat <<'VERIFY'
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
VERIFY
}

# ---------------------------------------------------------------------------
# BATCHES.md
# ---------------------------------------------------------------------------
_gen_batches_md() {
  cat <<'HEADER'
# Docs-Parity2 Batch Manifest

Generated by `tmp/docs-parity-meta/generate.sh`. Do not edit manually.

## Batch Overview

| Batch | Section | Crate(s) | Priority | Group | Deps | Template |
|-------|---------|----------|----------|-------|------|----------|
HEADER

  for entry in "${SECTION_REGISTRY[@]}"; do
    local num slug crates pri grp deps tmpl batch_id
    num="$(section_num "$entry")"
    slug="$(section_slug "$entry")"
    crates="$(section_crates "$entry")"
    pri="$(section_priority "$entry")"
    grp="$(section_group "$entry")"
    deps="$(section_deps "$entry")"
    tmpl="$(section_template "$entry")"
    batch_id="$(batch_id_for "$num")"
    printf '| %s | %s-%s | %s | %s | %s | %s | %s |\n' \
      "$batch_id" "$num" "$slug" "$crates" "$pri" "$grp" "${deps:-none}" "$tmpl"
  done

  cat <<'FOOTER'

## Recommended Execution Order

```bash
# Night 1: Core foundation (6 batches, ~9h)
bash tmp/docs-parity2/run-docs-parity2.sh --group core

# Night 2: Safety + interfaces (2 batches, ~3h)
bash tmp/docs-parity2/run-docs-parity2.sh --continue last --group safety-iface

# Night 3: Extensions + infra (8 batches, ~12h)
bash tmp/docs-parity2/run-docs-parity2.sh --continue last --group extensions,infra

# Night 4 (optional): Phase 2+ stubs (5 batches, ~7.5h)
bash tmp/docs-parity2/run-docs-parity2.sh --continue last --group phase2
```

## Dependency DAG

```
DP00 ─┬─> DP01 ─┬─> DP04
      │         ├─> DP07
      │         ├─> DP13
      │         └─> DP16
      ├─> DP02 ─┬─> DP11
      │         ├─> DP17
      │         └─> DP18
      ├─> DP03
      ├─> DP05 ─┬─> DP06
      │         └─> DP20 (also DP04)
      ├─> DP08 ──> DP14
      ├─> DP09
      ├─> DP10
      ├─> DP15
      └─> DP12 ──> DP19
               (also DP01, DP02)
```
FOOTER
}
