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

ALL_BATCHES=(
  "DP00" "DP01" "DP02" "DP03" "DP04" "DP05" 
  "DP06" "DP07" "DP08" "DP09" "DP10" "DP11" 
  "DP12" "DP13" "DP14" "DP15" "DP16" "DP17" 
  "DP18" "DP19" "DP20" 
)

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

batch_title() {
  case "$1" in
    DP00) echo "00-architecture: Architecture" ;;
    DP01) echo "01-orchestration: Orchestration" ;;
    DP02) echo "02-agents: Agents" ;;
    DP03) echo "03-composition: Composition" ;;
    DP04) echo "04-verification: Verification" ;;
    DP05) echo "05-learning: Learning" ;;
    DP06) echo "06-neuro: Neuro" ;;
    DP07) echo "07-conductor: Conductor" ;;
    DP08) echo "08-chain: Chain" ;;
    DP09) echo "09-daimon: Daimon" ;;
    DP10) echo "10-dreams: Dreams" ;;
    DP11) echo "11-safety: Safety" ;;
    DP12) echo "12-interfaces: Interfaces" ;;
    DP13) echo "13-coordination: Coordination" ;;
    DP14) echo "14-identity-economy: Identity & Economy" ;;
    DP15) echo "15-code-intelligence: Code Intelligence" ;;
    DP16) echo "16-heartbeat: Heartbeat" ;;
    DP17) echo "17-lifecycle: Lifecycle" ;;
    DP18) echo "18-tools: Tools" ;;
    DP19) echo "19-deployment: Deployment" ;;
    DP20) echo "20-technical-analysis: Technical Analysis" ;;
    *) return 1 ;;
  esac
}

batch_deps() {
  case "$1" in
    DP01) echo "DP00" ;;
    DP02) echo "DP00" ;;
    DP03) echo "DP00" ;;
    DP04) echo "DP00 DP01" ;;
    DP05) echo "DP00" ;;
    DP06) echo "DP00 DP05" ;;
    DP07) echo "DP00 DP01" ;;
    DP08) echo "DP00" ;;
    DP09) echo "DP00" ;;
    DP10) echo "DP00" ;;
    DP11) echo "DP00 DP02" ;;
    DP12) echo "DP00 DP01 DP02" ;;
    DP13) echo "DP00 DP01" ;;
    DP14) echo "DP00 DP08" ;;
    DP15) echo "DP00" ;;
    DP16) echo "DP00 DP01" ;;
    DP17) echo "DP00 DP02" ;;
    DP18) echo "DP00 DP02" ;;
    DP19) echo "DP00 DP12" ;;
    DP20) echo "DP00 DP04 DP05" ;;
    *) echo "" ;;
  esac
}

batch_group() {
  case "$1" in
    DP00|DP01|DP02|DP03|DP04|DP05) echo "core" ;;
    DP06|DP07) echo "extensions" ;;
    DP08|DP09|DP10) echo "phase2" ;;
    DP11|DP12) echo "safety-iface" ;;
    DP13) echo "infra" ;;
    DP14) echo "phase2" ;;
    DP15|DP16|DP17|DP18|DP19) echo "infra" ;;
    DP20) echo "phase2" ;;
    *) echo "misc" ;;
  esac
}

batch_verify_commands() {
  case "$1" in
    DP00)
      cat <<'EOF'
cargo check -p roko-core
cargo test -p roko-core --lib --no-run
cargo clippy -p roko-core --no-deps -- -D warnings
EOF
      ;;
    DP01)
      cat <<'EOF'
cargo check -p roko-orchestrator -p roko-cli
cargo test -p roko-orchestrator -p roko-cli --lib --no-run
cargo clippy -p roko-orchestrator -p roko-cli --no-deps -- -D warnings
EOF
      ;;
    DP02)
      cat <<'EOF'
cargo check -p roko-agent
cargo test -p roko-agent --lib --no-run
cargo clippy -p roko-agent --no-deps -- -D warnings
EOF
      ;;
    DP03)
      cat <<'EOF'
cargo check -p roko-compose
cargo test -p roko-compose --lib --no-run
cargo clippy -p roko-compose --no-deps -- -D warnings
EOF
      ;;
    DP04)
      cat <<'EOF'
cargo check -p roko-gate
cargo test -p roko-gate --lib --no-run
cargo clippy -p roko-gate --no-deps -- -D warnings
EOF
      ;;
    DP05)
      cat <<'EOF'
cargo check -p roko-learn
cargo test -p roko-learn --lib --no-run
cargo clippy -p roko-learn --no-deps -- -D warnings
EOF
      ;;
    DP06)
      cat <<'EOF'
cargo check -p roko-neuro -p roko-primitives
cargo test -p roko-neuro -p roko-primitives --lib --no-run
cargo clippy -p roko-neuro -p roko-primitives --no-deps -- -D warnings
EOF
      ;;
    DP07)
      cat <<'EOF'
cargo check -p roko-conductor
cargo test -p roko-conductor --lib --no-run
cargo clippy -p roko-conductor --no-deps -- -D warnings
EOF
      ;;
    DP08)
      cat <<'EOF'
cargo check -p roko-chain
cargo test -p roko-chain --lib --no-run
cargo clippy -p roko-chain --no-deps -- -D warnings
EOF
      ;;
    DP09)
      cat <<'EOF'
cargo check -p roko-daimon
cargo test -p roko-daimon --lib --no-run
cargo clippy -p roko-daimon --no-deps -- -D warnings
EOF
      ;;
    DP10)
      cat <<'EOF'
cargo check -p roko-dreams
cargo test -p roko-dreams --lib --no-run
cargo clippy -p roko-dreams --no-deps -- -D warnings
EOF
      ;;
    DP11)
      cat <<'EOF'
cargo check -p roko-agent
cargo test -p roko-agent --lib --no-run
cargo clippy -p roko-agent --no-deps -- -D warnings
EOF
      ;;
    DP12)
      cat <<'EOF'
cargo check -p roko-cli -p roko-serve -p roko-agent-server
cargo test -p roko-cli -p roko-serve -p roko-agent-server --lib --no-run
cargo clippy -p roko-cli -p roko-serve -p roko-agent-server --no-deps -- -D warnings
EOF
      ;;
    DP13)
      cat <<'EOF'
cargo check -p roko-orchestrator
cargo test -p roko-orchestrator --lib --no-run
cargo clippy -p roko-orchestrator --no-deps -- -D warnings
EOF
      ;;
    DP14)
      cat <<'EOF'
cargo check -p roko-chain
cargo test -p roko-chain --lib --no-run
cargo clippy -p roko-chain --no-deps -- -D warnings
EOF
      ;;
    DP15)
      cat <<'EOF'
cargo check -p roko-index -p roko-mcp-code -p roko-lang-rust -p roko-lang-typescript -p roko-lang-go
cargo test -p roko-index -p roko-mcp-code -p roko-lang-rust -p roko-lang-typescript -p roko-lang-go --lib --no-run
cargo clippy -p roko-index -p roko-mcp-code -p roko-lang-rust -p roko-lang-typescript -p roko-lang-go --no-deps -- -D warnings
EOF
      ;;
    DP16)
      cat <<'EOF'
cargo check -p roko-runtime
cargo test -p roko-runtime --lib --no-run
cargo clippy -p roko-runtime --no-deps -- -D warnings
EOF
      ;;
    DP17)
      cat <<'EOF'
cargo check -p roko-agent -p roko-runtime
cargo test -p roko-agent -p roko-runtime --lib --no-run
cargo clippy -p roko-agent -p roko-runtime --no-deps -- -D warnings
EOF
      ;;
    DP18)
      cat <<'EOF'
cargo check -p roko-std -p roko-agent
cargo test -p roko-std -p roko-agent --lib --no-run
cargo clippy -p roko-std -p roko-agent --no-deps -- -D warnings
EOF
      ;;
    DP19)
      cat <<'EOF'
cargo check -p roko-cli
cargo test -p roko-cli --lib --no-run
cargo clippy -p roko-cli --no-deps -- -D warnings
EOF
      ;;
    DP20)
      cat <<'EOF'
cargo check --workspace
cargo clippy --workspace --no-deps -- -D warnings
EOF
      ;;
    *) return 1 ;;
  esac
}

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
