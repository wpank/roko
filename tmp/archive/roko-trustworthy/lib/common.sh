#!/usr/bin/env bash

set -uo pipefail

: "${ROKO_ROOT:=/Users/will/dev/nunchi/roko/roko}"
: "${RT_ROOT:=$ROKO_ROOT/tmp/roko-trustworthy}"
: "${LOG_ROOT:=$RT_ROOT/logs}"
: "${CONTEXT_DIR:=$RT_ROOT/context-pack}"
: "${PROMPTS_DIR:=$RT_ROOT/prompts}"
: "${WORKTREE_ROOT:=$ROKO_ROOT/.roko/worktrees}"

: "${RT_MODEL:=gpt-5.5}"
: "${RT_REASONING:=high}"
: "${RT_TIMEOUT:=7200}"
: "${RT_MAX_RETRIES:=2}"
: "${RT_BASE_REF:=HEAD}"
: "${RT_MAX_BATCHES:=0}"
: "${RT_CLEANUP_EVERY:=1}"
: "${RT_CLEANUP_STALE_DAYS:=3}"
: "${RT_CLEAN_FAILED_ATTEMPTS:=1}"

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
  if (( h > 0 )); then
    printf '%dh %dm %ds' "$h" "$m" "$sec"
  elif (( m > 0 )); then
    printf '%dm %ds' "$m" "$sec"
  else
    printf '%ds' "$sec"
  fi
}

success_status() {
  case "${1:-}" in
    success|success_noop|skipped|verify_only) return 0 ;;
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
  [[ -d "$LOG_ROOT" ]] || return 1
  find "$LOG_ROOT" -maxdepth 1 -mindepth 1 -type d -name 'run-*' -exec test -f {}/manifest.env \; -print \
    | sort \
    | tail -1 \
    | sed 's|.*/||'
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
tmp_target_root()      { echo "${TMPDIR:-/tmp}/roko-trustworthy-targets"; }
batch_target_dir()     { echo "$(tmp_target_root)/$1/$2/$3-attempt-$4"; }

link_latest_run() {
  local run_id="$1"
  [[ "$run_id" == dry-run-* ]] && return 0
  ln -sfn "$LOG_ROOT/$run_id" "$LOG_ROOT/latest"
}

current_batch_value() {
  local run_id="$1" key="$2" file
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

ALL_BATCHES=(
  "RT00"
  "RT01" "RT02" "RT03" "RT04" "RT05" "RT06" "RT07" "RT08" "RT09"
  "RT10" "RT11" "RT12" "RT13" "RT14" "RT15" "RT16" "RT17"
  "RT18" "RT19" "RT20" "RT21" "RT22" "RT23"
)

batch_title() {
  case "$1" in
    RT00) echo "Define the done gate and parity ledger contract" ;;
    RT01) echo "Structured review verdict parsing and fail-closed contracts" ;;
    RT02) echo "Compile error classification and pre-agent cargo-fix path" ;;
    RT03) echo "Gate failure memory and cross-agent error pattern sharing" ;;
    RT04) echo "Post-gate reflection loop and playbook rule extraction" ;;
    RT05) echo "Context injection scoping and knowledge controls" ;;
    RT06) echo "Warm agent spawning, reuse, and interruption-safe resume" ;;
    RT07) echo "Gate failure replanning and plan revision flow" ;;
    RT08) echo "Provider/model pass-rate feedback and reward telemetry" ;;
    RT09) echo "A-MAC and knowledge admission for trustworthy memory" ;;
    RT10) echo "Remove live Mori/Bardo prompt leakage from Roko roles" ;;
    RT11) echo "RoleProfile and PromptPolicy manifest contracts" ;;
    RT12) echo "Manifest-backed built-in roles: architect, implementer, scribe" ;;
    RT13) echo "CognitiveWorkspace audit object and context provenance" ;;
    RT14) echo "ContextEngine bidder registry with cold-start policies" ;;
    RT15) echo "Cybernetic telemetry for prompt/context section outcomes" ;;
    RT16) echo "LearningBidder posteriors and attention policy controls" ;;
    RT17) echo "Contextual bandits for routing, model, and context decisions" ;;
    RT18) echo "Durable runtime and control plane essentials" ;;
    RT19) echo "Finish the trustworthy self-hosting workflow loop" ;;
    RT20) echo "Core architecture implementation queue handoff" ;;
    RT21) echo "Docs parity enforcement after runtime gates" ;;
    RT22) echo "Dashboard and product surfaces consume stable projections" ;;
    RT23) echo "Chain, economy, and advanced surfaces deferral packet" ;;
    *) return 1 ;;
  esac
}

batch_deps() {
  case "$1" in
    RT01|RT02|RT05|RT06|RT08|RT10) echo "RT00" ;;
    RT03) echo "RT01 RT02" ;;
    RT04) echo "RT03" ;;
    RT07) echo "RT03 RT04" ;;
    RT09) echo "RT08" ;;
    RT11) echo "RT10" ;;
    RT12) echo "RT11" ;;
    RT13) echo "RT11" ;;
    RT14) echo "RT13" ;;
    RT15) echo "RT13" ;;
    RT16) echo "RT14 RT15" ;;
    RT17) echo "RT08 RT16" ;;
    RT18) echo "RT00 RT06" ;;
    RT19) echo "RT01 RT02 RT03 RT04 RT05 RT06 RT07 RT08 RT10 RT11 RT13 RT15" ;;
    RT20) echo "RT19" ;;
    RT21) echo "RT20" ;;
    RT22) echo "RT18 RT20" ;;
    RT23) echo "RT20 RT21" ;;
    *) echo "" ;;
  esac
}

batch_group() {
  case "$1" in
    RT00) echo "gate" ;;
    RT01|RT02|RT03|RT04|RT05|RT06|RT07|RT08|RT09) echo "kernel" ;;
    RT10|RT11|RT12|RT13|RT14|RT15|RT16|RT17) echo "policy" ;;
    RT18) echo "runtime" ;;
    RT19) echo "selfhost" ;;
    RT20) echo "core" ;;
    RT21) echo "parity" ;;
    RT22) echo "dashboard" ;;
    RT23) echo "advanced" ;;
    *) return 1 ;;
  esac
}

batch_verify_commands() {
  case "$1" in
    RT00)
      cat <<'EOF'
cargo check -p roko-core -p roko-gate -p roko-cli
cargo test -p roko-gate --lib --no-run
cargo test -p roko-cli --test plan_validate --no-run
EOF
      ;;
    RT01)
      cat <<'EOF'
cargo check -p roko-gate -p roko-orchestrator -p roko-cli
cargo test -p roko-gate --lib --no-run
cargo test -p roko-orchestrator --lib --no-run
EOF
      ;;
    RT02)
      cat <<'EOF'
cargo check -p roko-gate -p roko-agent -p roko-cli
cargo test -p roko-gate --test compile_real_project --no-run
cargo test -p roko-cli --test e2e_self_host --no-run
EOF
      ;;
    RT03)
      cat <<'EOF'
cargo check -p roko-orchestrator -p roko-gate -p roko-learn
cargo test -p roko-learn --lib --no-run
cargo test -p roko-orchestrator --lib --no-run
EOF
      ;;
    RT04)
      cat <<'EOF'
cargo check -p roko-learn -p roko-conductor -p roko-orchestrator
cargo test -p roko-conductor --lib --no-run
cargo test -p roko-learn --lib --no-run
EOF
      ;;
    RT05)
      cat <<'EOF'
cargo check -p roko-compose -p roko-orchestrator -p roko-core
cargo test -p roko-compose --lib --no-run
cargo test -p roko-orchestrator --lib --no-run
EOF
      ;;
    RT06)
      cat <<'EOF'
cargo check -p roko-agent -p roko-runtime -p roko-cli
cargo test -p roko-runtime --test process_supervisor --no-run
cargo test -p roko-agent --test process_integration --no-run
EOF
      ;;
    RT07)
      cat <<'EOF'
cargo check -p roko-orchestrator -p roko-cli -p roko-serve
cargo test -p roko-cli --test e2e_self_host --no-run
cargo test -p roko-serve --test job_runner_integration --no-run
EOF
      ;;
    RT08)
      cat <<'EOF'
cargo check -p roko-learn -p roko-agent -p roko-serve
cargo test -p roko-learn --lib --no-run
cargo test -p roko-agent --test provider_integration --no-run
EOF
      ;;
    RT09)
      cat <<'EOF'
cargo check -p roko-neuro -p roko-learn -p roko-core
cargo test -p roko-neuro --lib --no-run
cargo test -p roko-learn --lib --no-run
EOF
      ;;
    RT10)
      cat <<'EOF'
cargo check -p roko-compose -p roko-orchestrator -p roko-agent -p roko-cli
cargo test -p roko-agent --test role_tools --no-run
cargo test -p roko-compose --lib --no-run
EOF
      ;;
    RT11)
      cat <<'EOF'
cargo check -p roko-core -p roko-compose -p roko-agent
cargo test -p roko-core --lib --no-run
cargo test -p roko-compose --lib --no-run
EOF
      ;;
    RT12)
      cat <<'EOF'
cargo check -p roko-agent -p roko-compose -p roko-cli
cargo test -p roko-agent --test role_tools --no-run
cargo test -p roko-cli --test agent_config --no-run
EOF
      ;;
    RT13)
      cat <<'EOF'
cargo check -p roko-compose -p roko-core -p roko-orchestrator
cargo test -p roko-compose --lib --no-run
cargo test -p roko-core --lib --no-run
EOF
      ;;
    RT14)
      cat <<'EOF'
cargo check -p roko-compose -p roko-learn -p roko-orchestrator
cargo test -p roko-compose --lib --no-run
cargo test -p roko-learn --lib --no-run
EOF
      ;;
    RT15)
      cat <<'EOF'
cargo check -p roko-learn -p roko-compose -p roko-serve
cargo test -p roko-learn --lib --no-run
cargo test -p roko-serve --test job_lifecycle --no-run
EOF
      ;;
    RT16)
      cat <<'EOF'
cargo check -p roko-learn -p roko-compose -p roko-orchestrator
cargo test -p roko-learn --lib --no-run
cargo test -p roko-compose --lib --no-run
EOF
      ;;
    RT17)
      cat <<'EOF'
cargo check -p roko-learn -p roko-agent -p roko-serve -p roko-cli
cargo test -p roko-learn --lib --no-run
cargo test -p roko-cli --test agent_config --no-run
EOF
      ;;
    RT18)
      cat <<'EOF'
cargo check -p roko-runtime -p roko-orchestrator -p roko-serve -p roko-cli
cargo test -p roko-runtime --test process_supervisor --no-run
cargo test -p roko-cli --test e2e_self_host --no-run
EOF
      ;;
    RT19)
      cat <<'EOF'
cargo check -p roko-cli -p roko-orchestrator -p roko-gate -p roko-compose -p roko-learn -p roko-agent
cargo test -p roko-cli --test e2e_self_host --no-run
cargo test -p roko-gate --lib --no-run
EOF
      ;;
    RT20)
      cat <<'EOF'
cargo check -p roko-core -p roko-runtime -p roko-agent -p roko-orchestrator -p roko-compose -p roko-gate -p roko-learn -p roko-neuro -p roko-serve -p roko-cli
cargo test -p roko-orchestrator --lib --no-run
cargo test -p roko-cli --test plan_validate --no-run
EOF
      ;;
    RT21)
      cat <<'EOF'
cargo check -p roko-cli -p roko-gate -p roko-orchestrator
cargo test -p roko-cli --test plan_validate --no-run
cargo test -p roko-gate --lib --no-run
EOF
      ;;
    RT22)
      cat <<'EOF'
cargo check -p roko-serve -p roko-agent-server -p roko-cli
cargo test -p roko-serve --test api_integration --no-run
cargo test -p roko-agent-server --test relay_registration --no-run
EOF
      ;;
    RT23)
      cat <<'EOF'
cargo check -p roko-chain -p roko-agent-server -p roko-serve -p roko-demo
cargo test -p roko-chain --lib --no-run
cargo test -p roko-demo --lib --no-run
EOF
      ;;
    *)
      return 1
      ;;
  esac
}

preflight_check() {
  if ! git -C "$ROKO_ROOT" rev-parse --show-toplevel >/dev/null 2>&1; then
    log_err "bootstrap" "ROKO_ROOT is not a git checkout: $ROKO_ROOT"
    exit 1
  fi
  if [[ "${DRY_RUN:-0}" -eq 0 ]] && [[ "${LIST_ONLY:-0}" -eq 0 ]] && ! command -v codex >/dev/null 2>&1; then
    log_err "bootstrap" "Missing codex CLI in PATH"
    exit 1
  fi
  local file batch
  for file in \
    "$CONTEXT_DIR/00-TRUSTWORTHY-RULES.md" \
    "$CONTEXT_DIR/01-ROKO-FIRST-ROADMAP.md" \
    "$CONTEXT_DIR/02-WORKSPACE-TOPOLOGY.md" \
    "$CONTEXT_DIR/03-ARCHITECTURE-PLAN-MAP.md" \
    "$CONTEXT_DIR/04-SELF-HOSTING-GATES.md" \
    "$CONTEXT_DIR/05-CYBERNETIC-POLICY-PRIMER.md"; do
    require_file "$file"
  done
  for batch in "${ALL_BATCHES[@]}"; do
    require_file "$(batch_prompt_file "$batch")"
  done
}
