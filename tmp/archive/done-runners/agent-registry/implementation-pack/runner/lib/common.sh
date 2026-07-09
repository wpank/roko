#!/usr/bin/env bash

set -uo pipefail

: "${ROKO_ROOT:=/Users/will/dev/nunchi/roko/roko}"
: "${AR_ROOT:=$ROKO_ROOT/tmp/agent-registry/implementation-pack}"
: "${RUNNER_ROOT:=$AR_ROOT/runner}"
: "${LOG_ROOT:=$RUNNER_ROOT/logs}"
: "${PROMPTS_DIR:=$AR_ROOT/prompts}"
: "${CONTEXT_DIR:=$AR_ROOT/context-pack}"
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
else
  C_RESET='' C_BOLD='' C_DIM='' C_RED='' C_GREEN='' C_YELLOW='' C_BLUE='' C_MAGENTA=''
fi

log_info() { printf '%s[INFO]%s  %s%-10s%s %s\n' "$C_BLUE" "$C_RESET" "$C_DIM" "$1" "$C_RESET" "$2"; }
log_ok()   { printf '%s[OK]%s    %s%-10s%s %s\n' "$C_GREEN" "$C_RESET" "$C_DIM" "$1" "$C_RESET" "$2"; }
log_warn() { printf '%s[WARN]%s  %s%-10s%s %s\n' "$C_YELLOW" "$C_RESET" "$C_DIM" "$1" "$C_RESET" "$2" >&2; }
log_err()  { printf '%s[ERR]%s   %s%-10s%s %s\n' "$C_RED" "$C_RESET" "$C_DIM" "$1" "$C_RESET" "$2" >&2; }
log_header() { printf '\n%s=== %s ===%s\n\n' "$C_BOLD$C_MAGENTA" "$1" "$C_RESET"; }
require_command() {
  command -v "$1" >/dev/null 2>&1 || {
    log_err "bootstrap" "Missing required command: $1"
    exit 1
  }
}

ALL_BATCHES=("AR01" "AR02" "AR03" "AR04" "AR05" "AR06" "AR07" "AR08")

success_status() {
  case "${1:-}" in
    success|success_noop|skipped) return 0 ;;
    *) return 1 ;;
  esac
}

terminal_failure_status() {
  case "${1:-}" in
    spawn_failed|verify_failed|commit_failed|timeout|blocked_failed_dep) return 0 ;;
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
  find "$LOG_ROOT" -maxdepth 1 -mindepth 1 -type d -name 'run-*' -exec test -f {}/manifest.env \; -print \
    | sort | tail -1 | sed 's|.*/||'
}

run_manifest_file()    { echo "$LOG_ROOT/$1/manifest.env"; }
run_result_file()      { echo "$LOG_ROOT/$1/$2.result"; }
run_log_file()         { local suffix=""; [[ -n "${3:-}" ]] && suffix=".attempt-$3"; echo "$LOG_ROOT/$1/$2$suffix.log"; }
run_prompts_dir()      { echo "$LOG_ROOT/$1/prompts"; }
run_prompt_snapshot()  { local suffix=""; [[ -n "${3:-}" ]] && suffix=".attempt-$3"; echo "$(run_prompts_dir "$1")/$2$suffix.prompt.md"; }
run_last_message_file(){ local suffix=""; [[ -n "${3:-}" ]] && suffix=".attempt-$3"; echo "$LOG_ROOT/$1/$2$suffix.last.txt"; }
run_failure_file()     { local suffix=""; [[ -n "${3:-}" ]] && suffix=".attempt-$3"; echo "$LOG_ROOT/$1/$2$suffix.failure.txt"; }
run_status_file()      { echo "$LOG_ROOT/$1/status.tsv"; }
run_current_batch_file(){ echo "$LOG_ROOT/$1/current-batch.env"; }
run_backups_dir()      { echo "$LOG_ROOT/$1/backups"; }
batch_prompt_file() {
  case "$1" in
    AR01) echo "$PROMPTS_DIR/AR01-contracts-and-fork-bootstrap.prompt.md" ;;
    AR02) echo "$PROMPTS_DIR/AR02-relay.prompt.md" ;;
    AR03) echo "$PROMPTS_DIR/AR03-agent-serve.prompt.md" ;;
    AR04) echo "$PROMPTS_DIR/AR04-registration-and-relay-client.prompt.md" ;;
    AR05) echo "$PROMPTS_DIR/AR05-mirage-runtime-docker.prompt.md" ;;
    AR06) echo "$PROMPTS_DIR/AR06-static-demo-ui.prompt.md" ;;
    AR07) echo "$PROMPTS_DIR/AR07-remote-demo-validation.prompt.md" ;;
    AR08) echo "$PROMPTS_DIR/AR08-kauri-dashboard.prompt.md" ;;
    *) return 1 ;;
  esac
}
tmp_target_root()      { echo "${TMPDIR:-/tmp}/roko-agent-registry-targets"; }
batch_target_dir()     { echo "$(tmp_target_root)/$1/$2/$3-attempt-$4"; }
manifest_value() {
  local file="$1"
  local key="$2"
  awk -v key="$key" '
    index($0, key "=") == 1 {
      value = $0
      sub("^[^=]+=\\047", "", value)
      sub("\\047$", "", value)
      print value
      exit
    }
  ' "$file"
}

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
  git -C "$worktree" status --porcelain=v1 --ignored=matching -uall \
    | grep -Ev '^([ MADRCU?!]{2}|!!) (\.cargo-target(/|$)|target(/|$))' \
    | grep -q .
}

record_status() {
  local run_id="$1"
  local batch="$2"
  local attempt="$3"
  local status="$4"
  local note="${5:-}"
  printf '%s\t%s\t%s\t%s\t%s\n' \
    "$(date -Iseconds)" "$batch" "$attempt" "$status" "$note" >> "$(run_status_file "$run_id")"
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

batch_title() {
  case "$1" in
    AR01) echo "Contracts and mirage fork bootstrap" ;;
    AR02) echo "Relay binary" ;;
    AR03) echo "roko agent serve" ;;
    AR04) echo "Agent relay client and chain registration" ;;
    AR05) echo "Mirage runtime, proxy, Docker, Railway shape" ;;
    AR06) echo "In-repo mirage demo UI and quickstart" ;;
    AR07) echo "Remote demo verification and operator docs" ;;
    AR08) echo "Optional Kauri dashboard migration" ;;
    *) return 1 ;;
  esac
}

batch_deps() {
  case "$1" in
    AR04) echo "AR01 AR02 AR03" ;;
    AR05) echo "AR02" ;;
    AR06) echo "AR01 AR02 AR03 AR04 AR05" ;;
    AR07) echo "AR04 AR05 AR06" ;;
    AR08) echo "AR01 AR02 AR04 AR05" ;;
    *) echo "" ;;
  esac
}

batch_group() {
  case "$1" in
    AR01|AR02|AR03) echo "foundation" ;;
    AR04|AR05) echo "integration" ;;
    AR06) echo "demo" ;;
    AR07) echo "validation" ;;
    AR08) echo "optional" ;;
    *) return 1 ;;
  esac
}

batch_required_tools() {
  case "$1" in
    AR01) echo "forge" ;;
    AR05) echo "docker" ;;
    AR06) echo "node" ;;
    AR08) echo "npm node" ;;
    *) echo "" ;;
  esac
}

batch_verify_commands() {
  case "$1" in
    AR01)
      cat <<'EOF'
test -f contracts/src/IdentityRegistry.sol
test -f contracts/src/ReputationRegistry.sol
test -f contracts/src/ValidationRegistry.sol
rg -n 'function updateAgentCardUri\s*\(\s*uint256\s+passportId,\s*string\s+calldata\s+newUri\s*\)' contracts/src/IdentityRegistry.sol
cd contracts && forge build
cargo check -p mirage-rs --features "binary,chain"
EOF
      ;;
    AR02)
      cat <<'EOF'
test -d apps/agent-relay
cargo check -p agent-relay --all-targets
cargo test -p agent-relay
EOF
      ;;
    AR03)
      cat <<'EOF'
test -f crates/roko-cli/src/main.rs
rg -n 'agent serve|Agent.*Serve|Serve \{' crates/roko-cli/src
cargo check -p roko-cli
cargo test -p roko-cli --test agent_serve
EOF
      ;;
    AR04)
      cat <<'EOF'
test -f crates/roko-agent-server/src/registration.rs
test -f crates/roko-agent-server/src/features/relay_client.rs
rg -n 'updateAgentCardUri\(uint256,string\)' crates/roko-agent-server/src/registration.rs
cargo check -p roko-agent-server
cargo test -p roko-agent-server --tests
EOF
      ;;
    AR05)
      cat <<'EOF'
rg -n 'agent-relay|entrypoint' docker/mirage.Dockerfile
rg -n '/relay' apps/mirage-rs/src/rpc.rs apps/mirage-rs/src/http_api/mod.rs
cargo check -p mirage-rs --features "binary,chain"
cargo test -p mirage-rs --test http_api relay_proxy
docker build -f docker/mirage.Dockerfile .
EOF
      ;;
    AR06)
      cat <<'EOF'
test -f apps/mirage-rs/static/quickstart.sh
test -f apps/mirage-rs/static/js/agent_registry_smoke.mjs
bash -n apps/mirage-rs/static/quickstart.sh
node --experimental-default-type=module --check apps/mirage-rs/static/js/api.js
node --experimental-default-type=module --check apps/mirage-rs/static/js/polling.js
node --experimental-default-type=module --check apps/mirage-rs/static/js/state.js
node --experimental-default-type=module --check apps/mirage-rs/static/js/main.js
node apps/mirage-rs/static/js/agent_registry_smoke.mjs --check
EOF
      ;;
    AR07)
      cat <<'EOF'
test -f tmp/agent-registry/remote-demo-runbook.md
test -f tmp/agent-registry/scripts/remote-demo-check.sh
bash -n tmp/agent-registry/scripts/remote-demo-check.sh
bash tmp/agent-registry/scripts/remote-demo-check.sh --dry-run
rg -n 'Railway|remote|local laptop agent|relay restart|mirage demo UI' tmp/agent-registry/remote-demo-runbook.md
EOF
      ;;
    AR08)
      cat <<'EOF'
test -d /Users/will/dev/nunchi/nunchi-dashboard
npm --prefix /Users/will/dev/nunchi/nunchi-dashboard ci
npm --prefix /Users/will/dev/nunchi/nunchi-dashboard run typecheck
npm --prefix /Users/will/dev/nunchi/nunchi-dashboard run build
EOF
      ;;
    *)
      return 1
      ;;
  esac
}

preflight_check() {
  ensure_dir "$LOG_ROOT"
  ensure_dir "$WORKTREE_ROOT"

  require_file "$AR_ROOT/README.md"
  require_file "$AR_ROOT/BATCHES.md"
  require_file "$CONTEXT_DIR/00-READ-FIRST.md"
  require_file "$CONTEXT_DIR/01-TARGET-STATE.md"
  require_file "$CONTEXT_DIR/02-CODE-MAP.md"
  require_file "$CONTEXT_DIR/03-VERIFICATION-MATRIX.md"

  local batch
  for batch in "${ALL_BATCHES[@]}"; do
    require_file "$(batch_prompt_file "$batch")"
  done

  require_command bash
  require_command git
  require_command rg
}
