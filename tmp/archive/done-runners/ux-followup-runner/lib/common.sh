#!/usr/bin/env bash

set -uo pipefail

: "${ROKO_ROOT:=/Users/will/dev/nunchi/roko/roko}"
: "${UX_ROOT:=$ROKO_ROOT/tmp/ux-followup-runner}"
: "${LOG_ROOT:=$UX_ROOT/logs}"
: "${PROMPTS_DIR:=$UX_ROOT/prompts}"
: "${CONTEXT_DIR:=$UX_ROOT/context-pack}"
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

# Execution order respects dependency DAG.
#
# Group A (self-hosting P0s) first: UX01-UX04.
# Group B (TUI event parity) next: UX05-UX11.
# Group C (session/state mgmt): UX12-UX14.
# Group D (observability): UX15-UX22.
# Group E (partially-wired): UX23-UX29.
# Group F (backends): UX30-UX34.
# Group G (drift + hygiene): UX35-UX42.
# Group H (docs + runner): UX43-UX47.
ALL_BATCHES=(
  "UX01" "UX02" "UX03" "UX04"
  "UX05" "UX06" "UX07" "UX08" "UX09" "UX10" "UX11"
  "UX12" "UX13" "UX14"
  "UX15" "UX16" "UX17" "UX18" "UX19" "UX20" "UX21" "UX22"
  "UX23" "UX24" "UX25" "UX26" "UX27" "UX28" "UX29"
  "UX30" "UX31" "UX32" "UX33" "UX34"
  "UX35" "UX36" "UX37" "UX38" "UX39" "UX40" "UX41" "UX42"
  "UX43" "UX44" "UX45" "UX46" "UX47"
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

ensure_dir() {
  mkdir -p "$1"
}

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
    | sort \
    | tail -1 \
    | sed 's|.*/||'
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
tmp_target_root()     { echo "${TMPDIR:-/tmp}/roko-ux-targets"; }
batch_target_dir()    { echo "$(tmp_target_root)/$1/$2/$3-attempt-$4"; }

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
  git -C "$worktree" status --porcelain=v1 -uall \
    | grep -Ev '^[ MADRCU?!]{2} (\.cargo-target/|target/)' \
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
    UX01) echo "Gate-failure → plan-revision feedback loop (self-hosting P0)" ;;
    UX02) echo "PRD-publish event → orchestrator auto-trigger (self-hosting P0)" ;;
    UX03) echo "End-to-end self-hosting smoke test" ;;
    UX04) echo "roko plan validate CLI command" ;;

    UX05) echo "Standalone TUI: in-process StateHub + delete polling fallback" ;;
    UX06) echo "Notify file-watcher replaces 500 ms .roko/ polling thread" ;;
    UX07) echo "Incremental tailers for signals / episodes / events" ;;
    UX08) echo "Task-output directory watcher + per-file incremental tail" ;;
    UX09) echo "Agent sidecar /stream WebSocket consumer on Agents tab" ;;
    UX10) echo "Git view fs-watch replaces 3 s git CLI polling" ;;
    UX11) echo "TUI channel backpressure + durable dashboard-gen counter" ;;

    UX12) echo "ExecutorSnapshot schema_version + migration framework" ;;
    UX13) echo "Resume: validate plan-discovery vs snapshot consistency" ;;
    UX14) echo "ProcessSupervisor SIGTERM escalation + Drop + CancellationToken" ;;

    UX15) echo "Verdicts substrate reader + per-gate trend widget" ;;
    UX16) echo "Conductor diagnosis TUI panel + HTTP endpoint" ;;
    UX17) echo "Efficiency-events trend aggregator + Learning sparkline" ;;
    UX18) echo "Metrics schema alignment: roko-core vs roko-agent-server" ;;
    UX19) echo "Prompt experiment winners on Learning tab" ;;
    UX20) echo "Agent topology TUI widget" ;;
    UX21) echo "Sidecar /logs endpoint + aggregator proxy" ;;
    UX22) echo "GET /api/c-factor/trend endpoint + trend widget" ;;

    UX23) echo "Gate pipeline: wire remaining 4 rungs in run_gate_rung" ;;
    UX24) echo "Playbook store query seam (dispatch / prompt builder)" ;;
    UX25) echo "HDC fingerprint per-episode" ;;
    UX26) echo "Safety contract enforcement wiring" ;;
    UX27) echo "Role-based tool whitelist enforcement" ;;
    UX28) echo "Enrichment pipeline Enriching-phase wiring" ;;
    UX29) echo "Phase-2 build-surface reality check + MCP audit" ;;

    UX30) echo "Codex backend conformance test harness" ;;
    UX31) echo "Cursor backend streaming path wiring" ;;
    UX32) echo "Backend test parity (happy/stream/tool/error/session)" ;;
    UX33) echo "ExecAgent vs ClaudeCliAgent consolidation + backend dir cleanup" ;;
    UX34) echo "Cascade router + model router integration tests" ;;

    UX35) echo "Adaptive gate thresholds load-path audit + wiring" ;;
    UX36) echo "roko.toml unused keys: consume or remove" ;;
    UX37) echo "SystemPromptBuilder 6-layer snapshot tests" ;;
    UX38) echo "Top-10 unwrap() cleanup" ;;
    UX39) echo "HTTP route validation + OpenAPI surface" ;;
    UX40) echo "Episode struct: explicit backend dispatcher field" ;;
    UX41) echo "cargo llvm-cov coverage scaffold" ;;
    UX42) echo "clippy missing_* doc sweep + timeout-flake audit" ;;

    UX43) echo "MORI-PARITY-CHECKLIST mechanical regeneration tool" ;;
    UX44) echo "Smoke tests for CLAUDE.md 'What to work on' items 1-9" ;;
    UX45) echo "Terminology sweep + stale-snapshot sidecar" ;;
    UX46) echo "tmp/implementation-plans status refresh + MORI paths audit" ;;
    UX47) echo "tui-parity runner hardening + CI dry-run + log retention" ;;

    *) return 1 ;;
  esac
}

batch_deps() {
  case "$1" in
    UX02) echo "UX01" ;;                # feedback loop must land before PRD publish subscriber drives it
    UX03) echo "UX01 UX02 UX04" ;;      # E2E test exercises full self-hosting
    UX07) echo "UX06" ;;                # incremental readers sit on top of the notify watcher
    UX08) echo "UX06" ;;
    UX13) echo "UX12" ;;                # resume validation needs schema_version + migration framework
    UX15) echo "UX24" ;;                # verdict widget depends on having a reader seam
    UX16) echo "UX05" ;;                # conductor diagnosis TUI panel uses StateHub push
    UX19) echo "UX05" ;;                # experiments widget + StateHub push
    UX20) echo "UX05" ;;
    UX22) echo "UX17" ;;                # c-factor trend reuses efficiency aggregator pattern
    UX32) echo "UX30 UX31" ;;           # test parity cross-backends
    UX34) echo "UX32" ;;                # integration tests leverage the test harness
    UX37) echo "UX38" ;;                # snapshot tests cleaner after unwrap sweep
    UX39) echo "UX38" ;;                # HTTP validation easier after unwrap sweep
    UX44) echo "UX03" ;;                # CLAUDE.md smoke tests reuse e2e infra
    *) echo "" ;;
  esac
}

batch_group() {
  case "$1" in
    UX01|UX02|UX03|UX04) echo "selfhost" ;;
    UX05|UX06|UX07|UX08|UX09|UX10|UX11) echo "tui-stream" ;;
    UX12|UX13|UX14) echo "state" ;;
    UX15|UX16|UX17|UX18|UX19|UX20|UX21|UX22) echo "observ" ;;
    UX23|UX24|UX25|UX26|UX27|UX28|UX29) echo "wired" ;;
    UX30|UX31|UX32|UX33|UX34) echo "backends" ;;
    UX35|UX36|UX37|UX38|UX39|UX40|UX41|UX42) echo "hygiene" ;;
    UX43|UX44|UX45|UX46|UX47) echo "docs" ;;
    *) echo "misc" ;;
  esac
}

batch_verify_commands() {
  case "$1" in
    UX01|UX02)
      cat <<'EOF'
cargo check -p roko-cli -p roko-orchestrator -p roko-runtime -p roko-learn
cargo test -p roko-orchestrator --lib --no-run
cargo clippy -p roko-cli -p roko-orchestrator --no-deps -- -D warnings
EOF
      ;;
    UX03)
      cat <<'EOF'
cargo check -p roko-cli
cargo test -p roko-cli --test e2e_self_host --no-run
EOF
      ;;
    UX04)
      cat <<'EOF'
cargo check -p roko-cli
cargo test -p roko-cli --lib --no-run -- plan::validate
cargo clippy -p roko-cli --no-deps -- -D warnings
EOF
      ;;
    UX05|UX06|UX07|UX08|UX09|UX10|UX11)
      cat <<'EOF'
cargo check -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
EOF
      ;;
    UX12|UX13)
      cat <<'EOF'
cargo check -p roko-cli
cargo test -p roko-cli --lib --no-run -- snapshot
cargo clippy -p roko-cli --no-deps -- -D warnings
EOF
      ;;
    UX14)
      cat <<'EOF'
cargo check -p roko-runtime -p roko-cli
cargo test -p roko-runtime --lib --no-run
cargo clippy -p roko-runtime -p roko-cli --no-deps -- -D warnings
EOF
      ;;
    UX15|UX16|UX17|UX19|UX20|UX22)
      cat <<'EOF'
cargo check -p roko-cli -p roko-serve
cargo clippy -p roko-cli -p roko-serve --no-deps -- -D warnings
EOF
      ;;
    UX18)
      cat <<'EOF'
cargo check -p roko-core -p roko-agent-server -p roko-serve
cargo test -p roko-core --lib --no-run -- obs::metrics
cargo clippy -p roko-core -p roko-agent-server -p roko-serve --no-deps -- -D warnings
EOF
      ;;
    UX21)
      cat <<'EOF'
cargo check -p roko-agent-server -p roko-serve
cargo clippy -p roko-agent-server -p roko-serve --no-deps -- -D warnings
EOF
      ;;
    UX23)
      cat <<'EOF'
cargo check -p roko-cli -p roko-gate
cargo test -p roko-gate --lib --no-run
cargo clippy -p roko-cli -p roko-gate --no-deps -- -D warnings
EOF
      ;;
    UX24)
      cat <<'EOF'
cargo check -p roko-cli -p roko-learn -p roko-compose
cargo test -p roko-learn --lib --no-run -- playbook
cargo clippy -p roko-cli -p roko-learn -p roko-compose --no-deps -- -D warnings
EOF
      ;;
    UX25)
      cat <<'EOF'
cargo check -p roko-learn -p roko-primitives
cargo test -p roko-learn --lib --no-run -- episode_logger
cargo clippy -p roko-learn -p roko-primitives --no-deps -- -D warnings
EOF
      ;;
    UX26|UX27)
      cat <<'EOF'
cargo check -p roko-agent
cargo test -p roko-agent --lib --no-run -- safety
cargo clippy -p roko-agent --no-deps -- -D warnings
EOF
      ;;
    UX28)
      cat <<'EOF'
cargo check -p roko-compose -p roko-cli
cargo test -p roko-compose --lib --no-run -- enrichment
cargo clippy -p roko-compose -p roko-cli --no-deps -- -D warnings
EOF
      ;;
    UX29)
      cat <<'EOF'
cargo check
cargo check --workspace
cargo clippy --workspace --no-deps -- -D warnings
test -s tmp/ux-followup-runner/mcp-audit.md
EOF
      ;;
    UX30|UX31|UX32|UX33|UX34)
      cat <<'EOF'
cargo check -p roko-agent
cargo test -p roko-agent --lib --no-run
cargo clippy -p roko-agent --no-deps -- -D warnings
EOF
      ;;
    UX35)
      cat <<'EOF'
cargo check -p roko-gate -p roko-cli
cargo test -p roko-gate --lib --no-run -- adaptive_threshold
cargo clippy -p roko-gate -p roko-cli --no-deps -- -D warnings
EOF
      ;;
    UX36)
      cat <<'EOF'
cargo check -p roko-cli -p roko-core
cargo test -p roko-cli --lib --no-run -- agent_config
cargo clippy -p roko-cli -p roko-core --no-deps -- -D warnings
EOF
      ;;
    UX37)
      cat <<'EOF'
cargo check -p roko-compose
cargo test -p roko-compose --lib --no-run -- system_prompt_builder
cargo clippy -p roko-compose --no-deps -- -D warnings
EOF
      ;;
    UX38)
      cat <<'EOF'
cargo check -p roko-compose -p roko-serve -p roko-gate -p roko-runtime
cargo clippy -p roko-compose -p roko-serve -p roko-gate -p roko-runtime --no-deps -- -D warnings
EOF
      ;;
    UX39)
      cat <<'EOF'
cargo check -p roko-serve
cargo test -p roko-serve --lib --no-run
cargo clippy -p roko-serve --no-deps -- -D warnings
EOF
      ;;
    UX40)
      cat <<'EOF'
cargo check -p roko-learn -p roko-agent
cargo test -p roko-learn --lib --no-run -- episode_logger
cargo clippy -p roko-learn -p roko-agent --no-deps -- -D warnings
EOF
      ;;
    UX41)
      cat <<'EOF'
test -f .github/workflows/coverage.yml
grep -q "cargo llvm-cov" .github/workflows/coverage.yml
test -x tools/coverage.sh
grep -q "cargo llvm-cov" tools/coverage.sh
EOF
      ;;
    UX42)
      cat <<'EOF'
cargo clippy --workspace --no-deps -- -D warnings -D clippy::missing_errors_doc -D clippy::missing_panics_doc
cargo test --workspace --no-run
EOF
      ;;
    UX43)
      cat <<'EOF'
test -x tools/mori-parity-check/mori-parity-check.sh || test -x tmp/ux-followup-runner/tools/mori-parity.sh
EOF
      ;;
    UX44)
      cat <<'EOF'
cargo check -p roko-cli
cargo test -p roko-cli --test smoke --no-run || cargo test -p roko-cli --test smoke_claude_md --no-run
EOF
      ;;
    UX45)
      cat <<'EOF'
bash -c 'hits=$(grep -rn "grimoire\\|styx\\|clade\\|mortal\\|death\\|reincarnation" CLAUDE.md README.md tmp/implementation-plans tmp/tui-parity --include="*.md" || true); test -z "$hits"'
test -s tmp/ux-followup-runner/bardo-backup-stale-snapshot.md
grep -q "Historical snapshot" tmp/ux-followup-runner/bardo-backup-stale-snapshot.md
EOF
      ;;
    UX46)
      cat <<'EOF'
test -s tmp/MORI-PARITY-CHECKLIST-CURRENT.md
grep -q "Path mapping" tmp/MORI-PARITY-CHECKLIST-CURRENT.md
grep -q "Last refreshed" tmp/implementation-plans/00-INDEX.md
EOF
      ;;
    UX47)
      cat <<'EOF'
bash tmp/tui-parity/run-tui-parity.sh --dry-run --only T9 >/dev/null
grep -q "TUI_PARITY_MAX_BATCHES" tmp/tui-parity/lib/common.sh
grep -q "TUI_PARITY_MAX_RETRIES" tmp/tui-parity/lib/common.sh
EOF
      ;;
    *)
      return 1
      ;;
  esac
}

batch_catalog_refs() {
  # Maps UXnn → catalogue item IDs (files 01–15)
  case "$1" in
    UX01) echo "06 89" ;;
    UX02) echo "05 51 90" ;;
    UX03) echo "60 26 43" ;;
    UX04) echo "12" ;;
    UX05) echo "68" ;;
    UX06) echo "69 76" ;;
    UX07) echo "71 73 74" ;;
    UX08) echo "72" ;;
    UX09) echo "70" ;;
    UX10) echo "75" ;;
    UX11) echo "77 78" ;;
    UX12) echo "79 81 60d" ;;
    UX13) echo "82" ;;
    UX14) echo "80 60e 18" ;;
    UX15) echo "35c 83 87" ;;
    UX16) echo "10 31 84" ;;
    UX17) echo "85" ;;
    UX18) echo "35 86" ;;
    UX19) echo "20 88" ;;
    UX20) echo "14" ;;
    UX21) echo "13" ;;
    UX22) echo "09" ;;
    UX23) echo "35a" ;;
    UX24) echo "35b 94" ;;
    UX25) echo "11 30 93" ;;
    UX26) echo "35d 91" ;;
    UX27) echo "35e 92" ;;
    UX28) echo "29 95" ;;
    UX29) echo "32 33 34" ;;
    UX30) echo "36" ;;
    UX31) echo "37" ;;
    UX32) echo "38" ;;
    UX33) echo "39 40" ;;
    UX34) echo "40a 60c" ;;
    UX35) echo "08 48a" ;;
    UX36) echo "48b" ;;
    UX37) echo "19" ;;
    UX38) echo "55" ;;
    UX39) echo "60a" ;;
    UX40) echo "60b" ;;
    UX41) echo "59" ;;
    UX42) echo "56 58" ;;
    UX43) echo "46" ;;
    UX44) echo "45" ;;
    UX45) echo "47 64 65 66" ;;
    UX46) echo "67 67a" ;;
    UX47) echo "27 28 27a 28a" ;;
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

  require_file "$UX_ROOT/README.md"
  require_file "$UX_ROOT/BATCHES.md"

  local batch
  for batch in "${ALL_BATCHES[@]}"; do
    require_file "$(batch_prompt_file "$batch")"
  done

  local dirty_count
  dirty_count=$(git -C "$ROKO_ROOT" status --porcelain | wc -l | tr -d ' ')
  if (( dirty_count > 0 )); then
    log_warn "preflight" "main repo has $dirty_count uncommitted change(s); the overnight worktree starts from committed HEAD only"
  else
    log_ok "preflight" "main repo is clean"
  fi

  return "$errors"
}
