#!/usr/bin/env bash

set -uo pipefail

: "${ROKO_ROOT:=/Users/will/dev/nunchi/roko/roko}"
: "${ACP_ROOT:=$ROKO_ROOT/tmp/acp-runner}"
: "${LOG_ROOT:=$ACP_ROOT/logs}"
: "${PROMPTS_DIR:=$ACP_ROOT/prompts}"
: "${CONTEXT_DIR:=$ACP_ROOT/context-pack}"
: "${WORKTREE_ROOT:=$ROKO_ROOT/.roko/worktrees}"

if [[ -t 1 && -z "${NO_COLOR:-}" ]]; then
  C_RESET=$'\e[0m'
  C_BOLD=$'\e[1m'
  C_DIM=$'\e[2m'
  C_RED=$'\e[31m'
  C_GREEN=$'\e[32m'
  C_YELLOW=$'\e[33m'
  C_BLUE=$'\e[34m'
  C_MAGENTA=$'\e[35m'
  C_CYAN=$'\e[36m'
else
  C_RESET='' C_BOLD='' C_DIM='' C_RED='' C_GREEN='' C_YELLOW='' C_BLUE='' C_MAGENTA='' C_CYAN=''
fi

log_info()   { printf '%s[INFO]%s  %s%-10s%s %s\n' "$C_BLUE"   "$C_RESET" "$C_DIM" "$1" "$C_RESET" "$2"; }
log_ok()     { printf '%s[OK]%s    %s%-10s%s %s\n' "$C_GREEN"  "$C_RESET" "$C_DIM" "$1" "$C_RESET" "$2"; }
log_warn()   { printf '%s[WARN]%s  %s%-10s%s %s\n' "$C_YELLOW" "$C_RESET" "$C_DIM" "$1" "$C_RESET" "$2" >&2; }
log_err()    { printf '%s[ERR]%s   %s%-10s%s %s\n' "$C_RED"    "$C_RESET" "$C_DIM" "$1" "$C_RESET" "$2" >&2; }
log_header() { printf '\n%s=== %s ===%s\n\n' "$C_BOLD$C_MAGENTA" "$1" "$C_RESET"; }

# Execution order — 18 batches in 4 groups
# Group 1 (scaffold): ACP01..ACP03
# Group 2 (core):     ACP04..ACP08
# Group 3 (bridges):  ACP09..ACP14
# Group 4 (config):   ACP15..ACP18
ALL_BATCHES=(
  "ACP01" "ACP02" "ACP03"
  "ACP04" "ACP05" "ACP06" "ACP07" "ACP08"
  "ACP09" "ACP10" "ACP11" "ACP12" "ACP13" "ACP14"
  "ACP15" "ACP16" "ACP17" "ACP18"
)

success_status() {
  case "${1:-}" in
    success|success_noop|skipped) return 0 ;;
    *) return 1 ;;
  esac
}

terminal_failure_status() {
  case "${1:-}" in
    spawn_failed|verify_failed|commit_failed|timeout|blocked) return 0 ;;
    *) return 1 ;;
  esac
}

fmt_duration() {
  local s="${1:-0}"
  local h=$((s / 3600))
  local m=$(((s % 3600) / 60))
  local sec=$((s % 60))
  if (( h > 0 )); then
    printf '%dh %dm %ds' "$h" "$m" "$sec"
  elif (( m > 0 )); then
    printf '%dm %ds' "$m" "$sec"
  else
    printf '%ds' "$sec"
  fi
}

ensure_dir() { mkdir -p "$1"; }

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    log_err "bootstrap" "Missing file: $path"
    exit 1
  fi
}

latest_run_id() {
  [[ -d "$LOG_ROOT" ]] || return 1
  local id
  id="$(find "$LOG_ROOT" -maxdepth 1 -mindepth 1 -type d -name 'run-*' \
        -exec test -f {}/manifest.env \; -print 2>/dev/null \
      | sort \
      | tail -1 \
      | sed 's|.*/||')"
  if [[ -z "$id" ]]; then
    return 1
  fi
  printf '%s\n' "$id"
}

run_manifest_file()   { echo "$LOG_ROOT/$1/manifest.env"; }
run_result_file()     { echo "$LOG_ROOT/$1/$2.result"; }
run_log_file()        { echo "$LOG_ROOT/$1/$2.log"; }
run_prompts_dir()     { echo "$LOG_ROOT/$1/prompts"; }
run_prompt_snapshot() { echo "$(run_prompts_dir "$1")/$2.prompt.md"; }
run_last_message_file(){ echo "$LOG_ROOT/$1/$2.last.txt"; }
run_failure_file()    { echo "$LOG_ROOT/$1/$2.failure.txt"; }
run_status_file()     { echo "$LOG_ROOT/$1/status.tsv"; }
run_current_batch_file(){ echo "$LOG_ROOT/$1/current-batch.env"; }
run_backups_dir()     { echo "$LOG_ROOT/$1/backups"; }
batch_prompt_file()   { echo "$PROMPTS_DIR/$1.prompt.md"; }

link_latest_run() {
  local run_id="$1"
  [[ "$run_id" == dry-run-* ]] && return 0
  ln -sfn "$LOG_ROOT/$run_id" "$LOG_ROOT/latest"
}

current_batch_value() {
  local run_id="$1"
  local key="$2"
  local file
  file="$(run_current_batch_file "$run_id")"
  [[ -f "$file" ]] || return 1
  awk -F= -v key="$key" '$1 == key { gsub(/\047/, "", $2); print $2 }' "$file"
}

current_batch_name()    { current_batch_value "$1" "BATCH"; }
current_batch_attempt() { current_batch_value "$1" "ATTEMPT"; }

worktree_dirty() {
  local worktree="$1"
  git -C "$worktree" status --porcelain=v1 \
    | grep -q .
}

record_status() {
  local run_id="$1"
  local batch="$2"
  local attempt="$3"
  local status="$4"
  local note="${5:-}"
  printf '%s\t%s\t%s\t%s\t%s\n' \
    "$(date -Iseconds)" \
    "$batch" \
    "$attempt" \
    "$status" \
    "$note" >> "$(run_status_file "$run_id")"
}

set_current_batch() {
  local run_id="$1"
  local batch="$2"
  local attempt="$3"
  cat > "$(run_current_batch_file "$run_id")" <<EOF
BATCH='$batch'
ATTEMPT='$attempt'
UPDATED_AT='$(date -Iseconds)'
EOF
}

clear_current_batch() {
  local run_id="$1"
  rm -f "$(run_current_batch_file "$run_id")"
}

# ---------------------------------------------------------------------------
# Batch metadata
# ---------------------------------------------------------------------------

batch_title() {
  case "$1" in
    ACP01) echo "Scaffold roko-acp crate + workspace wire" ;;
    ACP02) echo "ACP JSON-RPC types (inline, no SDK dep)" ;;
    ACP03) echo "Stdio transport layer" ;;

    ACP04) echo "Handler dispatch loop" ;;
    ACP05) echo "Session management" ;;
    ACP06) echo "Prompt handling + event streaming" ;;
    ACP07) echo "roko acp CLI subcommand" ;;
    ACP08) echo "Protocol conformance tests" ;;

    ACP09) echo "File system bridge" ;;
    ACP10) echo "Terminal bridge" ;;
    ACP11) echo "Permission bridge" ;;
    ACP12) echo "Gate result bridge" ;;
    ACP13) echo "Plan phase bridge" ;;
    ACP14) echo "Usage/cost bridge" ;;

    ACP15) echo "Session config options" ;;
    ACP16) echo "Slash commands" ;;
    ACP17) echo "Elicitation forms" ;;
    ACP18) echo "Lifecycle integration tests" ;;

    *) return 1 ;;
  esac
}

batch_deps() {
  case "$1" in
    # scaffold: linear chain
    ACP02) echo "ACP01" ;;
    ACP03) echo "ACP02" ;;

    # core: linear chain from ACP03
    ACP04) echo "ACP03" ;;
    ACP05) echo "ACP04" ;;
    ACP06) echo "ACP05" ;;
    ACP07) echo "ACP06" ;;
    ACP08) echo "ACP07" ;;

    # bridges: all depend on ACP06 only
    ACP09) echo "ACP06" ;;
    ACP10) echo "ACP06" ;;
    ACP11) echo "ACP06" ;;
    ACP12) echo "ACP06" ;;
    ACP13) echo "ACP06" ;;
    ACP14) echo "ACP06" ;;

    # config: ACP15-17 depend on ACP05, ACP18 depends on ACP07+ACP15+ACP16
    ACP15) echo "ACP05" ;;
    ACP16) echo "ACP05" ;;
    ACP17) echo "ACP05" ;;
    ACP18) echo "ACP07 ACP15 ACP16" ;;

    *) echo "" ;;
  esac
}

batch_group() {
  case "$1" in
    ACP01|ACP02|ACP03)                         echo "scaffold" ;;
    ACP04|ACP05|ACP06|ACP07|ACP08)             echo "core" ;;
    ACP09|ACP10|ACP11|ACP12|ACP13|ACP14)       echo "bridges" ;;
    ACP15|ACP16|ACP17|ACP18)                   echo "config" ;;
    *) echo "misc" ;;
  esac
}

# Packages to check per batch (for cargo check/clippy)
batch_check_packages() {
  case "$1" in
    ACP07) echo "roko-acp" ;;
    ACP08|ACP18) echo "roko-acp" ;;
    *) echo "roko-acp" ;;
  esac
}

# Test command per batch (empty = no tests for this batch)
batch_test_command() {
  case "$1" in
    ACP03|ACP05|ACP15) echo "cargo test -p roko-acp --lib" ;;
    ACP08|ACP18)       echo "cargo test -p roko-acp" ;;
    *)                 echo "" ;;
  esac
}

# Scope: paths that are allowed to change
batch_allowed_paths() {
  case "$1" in
    ACP07) echo "crates/roko-acp/ crates/roko-cli/ Cargo.lock" ;;
    ACP01) echo "crates/roko-acp/ Cargo.toml crates/roko-cli/Cargo.toml" ;;
    *)     echo "crates/roko-acp/" ;;
  esac
}

# Required terms in changed files (verify gate)
batch_required_terms() {
  case "$1" in
    ACP01) echo "roko-acp" ;;
    ACP02) echo "JsonRpc|SessionUpdate|ContentBlock" ;;
    ACP03) echo "StdioTransport|read_message|send_response" ;;
    ACP04) echo "run_acp_server|initialize|session" ;;
    ACP05) echo "AcpSession|SessionManager|SessionConfigState" ;;
    ACP06) echo "CognitiveEvent|stream_events|session.?update" ;;
    ACP07) echo "Acp|roko_acp" ;;
    ACP08) echo "test_initialize|test_session" ;;
    ACP09) echo "AcpFileSystem|read_file|write_file" ;;
    ACP10) echo "AcpTerminal|terminal" ;;
    ACP11) echo "PermissionGate|request_permission" ;;
    ACP12) echo "gate_started|gate_completed" ;;
    ACP13) echo "PlanEntry|phase_transition" ;;
    ACP14) echo "AcpUsageBridge|usage_notification" ;;
    ACP15) echo "build_config_options|handle_config_update" ;;
    ACP16) echo "build_available_commands|parse_slash_command" ;;
    ACP17) echo "request_elicitation|gate_config_schema" ;;
    ACP18) echo "test_config|test_slash|test_session" ;;
    *) echo "" ;;
  esac
}

preflight_check() {
  local errors=0
  log_header "PREFLIGHT"

  if command -v codex >/dev/null 2>&1; then
    log_ok "preflight" "codex CLI: $(command -v codex)"
  else
    log_err "preflight" "codex CLI not found"
    errors=$((errors + 1))
  fi

  if git -C "$ROKO_ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    log_ok "preflight" "git repo detected"
  else
    log_err "preflight" "ROKO_ROOT is not a git repo: $ROKO_ROOT"
    errors=$((errors + 1))
  fi

  ensure_dir "$LOG_ROOT"
  ensure_dir "$WORKTREE_ROOT"

  require_file "$ACP_ROOT/README.md"
  require_file "$ACP_ROOT/BATCHES.md"

  local batch
  for batch in "${ALL_BATCHES[@]}"; do
    require_file "$(batch_prompt_file "$batch")"
  done

  local dirty_count
  dirty_count=$(git -C "$ROKO_ROOT" status --porcelain | wc -l | tr -d ' ')
  if (( dirty_count > 0 )); then
    log_warn "preflight" "main repo has $dirty_count uncommitted change(s); worktree starts from committed HEAD only"
  else
    log_ok "preflight" "main repo is clean"
  fi

  return "$errors"
}
