#!/usr/bin/env bash

set -uo pipefail

: "${ROKO_ROOT:=/Users/will/dev/nunchi/roko/roko}"
: "${PE_ROOT:=$ROKO_ROOT/tmp/refinement-audit-runner}"
: "${PE_LOG_ROOT:=$PE_ROOT/logs}"
: "${PE_CONTEXT_DIR:=$PE_ROOT/context-pack}"
: "${PE_PROMPTS_DIR:=$PE_ROOT/prompts}"
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

ensure_dir() { mkdir -p "$1"; }

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    log_err "bootstrap" "Missing file: $path"
    exit 1
  fi
}

fmt_duration() {
  local s="${1:-0}"
  local h=$((s / 3600))
  local m=$(((s % 3600) / 60))
  local sec=$((s % 60))
  if (( h > 0 )); then printf '%dh %dm %ds' "$h" "$m" "$sec"
  elif (( m > 0 )); then printf '%dm %ds' "$m" "$sec"
  else printf '%ds' "$sec"; fi
}

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

latest_run_id() {
  [[ -d "$PE_LOG_ROOT" ]] || return 1
  find "$PE_LOG_ROOT" -maxdepth 1 -mindepth 1 -type d -name 'pe-run-*' -exec test -f {}/manifest.env \; -print \
    | sort | tail -1 | sed 's|.*/||'
}

run_manifest_file()    { echo "$PE_LOG_ROOT/$1/manifest.env"; }
run_result_file()      { echo "$PE_LOG_ROOT/$1/$2.result"; }
run_log_file()         { echo "$PE_LOG_ROOT/$1/$2.log"; }
run_prompts_dir()      { echo "$PE_LOG_ROOT/$1/prompts"; }
run_prompt_snapshot()  { echo "$(run_prompts_dir "$1")/$2.prompt.md"; }
run_last_message_file(){ echo "$PE_LOG_ROOT/$1/$2.last.txt"; }
run_failure_file()     { echo "$PE_LOG_ROOT/$1/$2.failure.txt"; }
run_status_file()      { echo "$PE_LOG_ROOT/$1/status.tsv"; }
run_current_batch_file(){ echo "$PE_LOG_ROOT/$1/current-batch.env"; }
run_backups_dir()      { echo "$PE_LOG_ROOT/$1/backups"; }
batch_prompt_file()    { echo "$PE_PROMPTS_DIR/$1.prompt.md"; }
tmp_target_root()      { echo "${TMPDIR:-/tmp}/roko-pe-targets"; }
batch_target_dir()     { echo "$(tmp_target_root)/$1/$2/$3-attempt-$4"; }

link_latest_run() {
  local run_id="$1"
  [[ "$run_id" == dry-run-* ]] && return 0
  ln -sfn "$PE_LOG_ROOT/$run_id" "$PE_LOG_ROOT/pe-latest"
}

current_batch_value() {
  local run_id="$1" key="$2"
  local file
  file="$(run_current_batch_file "$run_id")"
  [[ -f "$file" ]] || return 1
  awk -F= -v key="$key" '$1 == key { gsub(/\047/, "", $2); print $2 }' "$file"
}

current_batch_name()    { current_batch_value "$1" "BATCH"; }
current_batch_attempt() { current_batch_value "$1" "ATTEMPT"; }

worktree_dirty() {
  local worktree="$1"
  git -C "$worktree" status --porcelain=v1 -uall \
    | grep -Ev '^[ MADRCU?!]{2} (\.cargo-target/|target/)' \
    | grep -q .
}

record_status() {
  local run_id="$1" batch="$2" attempt="$3" status="$4" note="${5:-}"
  printf '%s\t%s\t%s\t%s\t%s\n' \
    "$(date -Iseconds)" "$batch" "$attempt" "$status" "$note" \
    >> "$(run_status_file "$run_id")"
}

set_current_batch() {
  local run_id="$1" batch="$2" attempt="$3"
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
# Batch manifest — 13 batches: PE00-PE12 (all independent)
# ---------------------------------------------------------------------------

ALL_BATCHES=(
  "PE00" "PE01" "PE02" "PE03" "PE04" "PE05" "PE06"
  "PE07" "PE08" "PE09" "PE10" "PE11" "PE12"
)

phase3_section_name() {
  case "$1" in
    00) echo "architecture" ;; 01) echo "orchestration" ;; 02) echo "agents" ;;
    03) echo "composition" ;; 04) echo "verification" ;; 05) echo "learning" ;;
    06) echo "neuro" ;; 07) echo "conductor" ;; 08) echo "chain" ;;
    09) echo "daimon" ;; 10) echo "dreams" ;; 11) echo "safety" ;;
    12) echo "interfaces" ;; *) echo "unknown" ;;
  esac
}

batch_title() {
  case "$1" in
    PE0[0-9]|PE1[0-2])
      local s="${1#PE}"
      echo "Execute docs-parity/$s ($(phase3_section_name "$s")) code updates"
      ;;
    *) return 1 ;;
  esac
}

batch_group() {
  echo "parity-exec"
}

batch_deps() {
  # All PE batches are independent of each other
  echo ""
}

# Phase 3 verify commands — cargo check + clippy per section
batch_verify_commands() {
  case "$1" in
    PE00) cat <<'EOF'
cargo check -p roko-core
cargo clippy -p roko-core --no-deps -- -D warnings
EOF
      ;;
    PE01) cat <<'EOF'
cargo check -p roko-orchestrator -p roko-cli
cargo clippy -p roko-orchestrator -p roko-cli --no-deps -- -D warnings
EOF
      ;;
    PE02) cat <<'EOF'
cargo check -p roko-agent
cargo clippy -p roko-agent --no-deps -- -D warnings
EOF
      ;;
    PE03) cat <<'EOF'
cargo check -p roko-compose
cargo clippy -p roko-compose --no-deps -- -D warnings
EOF
      ;;
    PE04) cat <<'EOF'
cargo check -p roko-gate
cargo clippy -p roko-gate --no-deps -- -D warnings
EOF
      ;;
    PE05) cat <<'EOF'
cargo check -p roko-learn
cargo clippy -p roko-learn --no-deps -- -D warnings
EOF
      ;;
    PE06) cat <<'EOF'
cargo check -p roko-neuro
cargo clippy -p roko-neuro --no-deps -- -D warnings
EOF
      ;;
    PE07) cat <<'EOF'
cargo check -p roko-conductor
cargo clippy -p roko-conductor --no-deps -- -D warnings
EOF
      ;;
    PE08) cat <<'EOF'
cargo check -p roko-chain
cargo clippy -p roko-chain --no-deps -- -D warnings
EOF
      ;;
    PE09) cat <<'EOF'
cargo check -p roko-daimon
cargo clippy -p roko-daimon --no-deps -- -D warnings
EOF
      ;;
    PE10) cat <<'EOF'
cargo check -p roko-dreams
cargo clippy -p roko-dreams --no-deps -- -D warnings
EOF
      ;;
    PE11) cat <<'EOF'
cargo check -p roko-agent
cargo clippy -p roko-agent --no-deps -- -D warnings
EOF
      ;;
    PE12) cat <<'EOF'
cargo check -p roko-cli -p roko-serve -p roko-agent-server
cargo clippy -p roko-cli -p roko-serve -p roko-agent-server --no-deps -- -D warnings
EOF
      ;;
    *) echo "" ;;
  esac
}

# ---------------------------------------------------------------------------
# Preflight
# ---------------------------------------------------------------------------

preflight_check() {
  local errors=0
  log_header "PREFLIGHT (Phase 3 — Parity Execution)"

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

  ensure_dir "$PE_LOG_ROOT"
  ensure_dir "$WORKTREE_ROOT"

  local batch missing_prompts=0
  for batch in "${ALL_BATCHES[@]}"; do
    if [[ ! -f "$(batch_prompt_file "$batch")" ]]; then
      log_err "preflight" "Missing prompt file: $(batch_prompt_file "$batch")"
      missing_prompts=$((missing_prompts + 1))
    fi
  done
  if (( missing_prompts > 0 )); then
    log_err "preflight" "$missing_prompts prompt file(s) missing"
    errors=$((errors + missing_prompts))
  else
    log_ok "preflight" "All ${#ALL_BATCHES[@]} prompt files present"
  fi

  local dirty_count
  dirty_count=$(git -C "$ROKO_ROOT" status --porcelain | wc -l | tr -d ' ')
  if (( dirty_count > 0 )); then
    log_warn "preflight" "repo has $dirty_count uncommitted change(s); worktree starts from committed HEAD only"
  else
    log_ok "preflight" "repo is clean"
  fi

  return "$errors"
}
