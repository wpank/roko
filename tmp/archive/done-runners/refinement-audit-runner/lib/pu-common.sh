#!/usr/bin/env bash

set -uo pipefail

: "${ROKO_ROOT:=/Users/will/dev/nunchi/roko/roko}"
: "${PU_ROOT:=$ROKO_ROOT/tmp/refinement-audit-runner}"
: "${PU_LOG_ROOT:=$PU_ROOT/logs}"
: "${PU_CONTEXT_DIR:=$PU_ROOT/context-pack}"
: "${PU_PROMPTS_DIR:=$PU_ROOT/prompts}"
: "${DOCS_PARITY_DIR:=$ROKO_ROOT/tmp/docs-parity}"

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
  [[ -d "$PU_LOG_ROOT" ]] || return 1
  find "$PU_LOG_ROOT" -maxdepth 1 -mindepth 1 -type d -name 'pu-run-*' -exec test -f {}/manifest.env \; -print \
    | sort | tail -1 | sed 's|.*/||'
}

run_manifest_file()    { echo "$PU_LOG_ROOT/$1/manifest.env"; }
run_result_file()      { echo "$PU_LOG_ROOT/$1/$2.result"; }
run_log_file()         { echo "$PU_LOG_ROOT/$1/$2.log"; }
run_prompts_dir()      { echo "$PU_LOG_ROOT/$1/prompts"; }
run_prompt_snapshot()  { echo "$(run_prompts_dir "$1")/$2.prompt.md"; }
run_failure_file()     { echo "$PU_LOG_ROOT/$1/$2.failure.txt"; }
run_status_file()      { echo "$PU_LOG_ROOT/$1/status.tsv"; }
run_current_batch_file(){ echo "$PU_LOG_ROOT/$1/current-batch.env"; }
run_backups_dir()      { echo "$PU_LOG_ROOT/$1/backups"; }
batch_prompt_file()    { echo "$PU_PROMPTS_DIR/$1.prompt.md"; }
run_baseline_dir()     { echo "$PU_LOG_ROOT/$1/baselines"; }
run_batch_baseline_root() { echo "$(run_baseline_dir "$1")/$2"; }
run_batch_section_snapshot_dir() { echo "$(run_batch_baseline_root "$1" "$2").section"; }
run_batch_sections_fingerprint_file() { echo "$(run_batch_baseline_root "$1" "$2").sections.tsv"; }
run_batch_section_fingerprint_file() { echo "$(run_batch_baseline_root "$1" "$2").section.sha256"; }
section_dir()          { echo "$DOCS_PARITY_DIR/$1"; }

link_latest_run() {
  local run_id="$1"
  [[ "$run_id" == dry-run-* ]] && return 0
  ln -sfn "$PU_LOG_ROOT/$run_id" "$PU_LOG_ROOT/pu-latest"
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

repo_dirty() {
  git -C "$ROKO_ROOT" status --porcelain=v1 \
    | grep -Ev '^[ MADRCU?!]{2} (\.cargo-target/|target/)' \
    | grep -q .
}

hash_stream() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 | awk '{print $1}'
  elif command -v sha256sum >/dev/null 2>&1; then
    sha256sum | awk '{print $1}'
  elif command -v openssl >/dev/null 2>&1; then
    openssl dgst -sha256 -r | awk '{print $1}'
  else
    python3 -c 'import hashlib,sys; print(hashlib.sha256(sys.stdin.buffer.read()).hexdigest())'
  fi
}

hash_file() {
  local file="$1"
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" | awk '{print $1}'
  elif command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{print $1}'
  elif command -v openssl >/dev/null 2>&1; then
    openssl dgst -sha256 -r "$file" | awk '{print $1}'
  else
    python3 - "$file" <<'PY'
import hashlib, pathlib, sys
print(hashlib.sha256(pathlib.Path(sys.argv[1]).read_bytes()).hexdigest())
PY
  fi
}

section_fingerprint() {
  local dir="$1"
  if [[ ! -d "$dir" ]]; then
    echo "missing"
    return 0
  fi

  (
    cd "$dir"
    find . -type f | LC_ALL=C sort | while IFS= read -r rel; do
      printf 'F\t%s\t%s\n' "$rel" "$(hash_file "$rel")"
    done
    find . -type l | LC_ALL=C sort | while IFS= read -r rel; do
      printf 'L\t%s\t%s\n' "$rel" "$(readlink "$rel")"
    done
  ) | hash_stream
}

capture_batch_baseline() {
  local run_id="$1" batch="$2"
  local section snapshot_dir fingerprints_file fingerprint_file
  section="${batch#PU}"
  snapshot_dir="$(run_batch_section_snapshot_dir "$run_id" "$batch")"
  fingerprints_file="$(run_batch_sections_fingerprint_file "$run_id" "$batch")"
  fingerprint_file="$(run_batch_section_fingerprint_file "$run_id" "$batch")"

  ensure_dir "$(run_baseline_dir "$run_id")"
  rm -rf "$snapshot_dir"
  mkdir -p "$snapshot_dir"

  if [[ -d "$(section_dir "$section")" ]]; then
    (
      cd "$(section_dir "$section")"
      tar -cf - .
    ) | (
      cd "$snapshot_dir"
      tar -xf -
    )
  fi

  : > "$fingerprints_file"
  local s
  for s in "${PHASE2_SECTIONS[@]}"; do
    printf '%s\t%s\n' "$s" "$(section_fingerprint "$(section_dir "$s")")" >> "$fingerprints_file"
  done
  printf '%s\n' "$(section_fingerprint "$(section_dir "$section")")" > "$fingerprint_file"
}

restore_batch_section_baseline() {
  local run_id="$1" batch="$2"
  local section snapshot_dir dest
  section="${batch#PU}"
  snapshot_dir="$(run_batch_section_snapshot_dir "$run_id" "$batch")"
  dest="$(section_dir "$section")"

  rm -rf "$dest"
  mkdir -p "$dest"
  if [[ -d "$snapshot_dir" ]]; then
    (
      cd "$snapshot_dir"
      tar -cf - .
    ) | (
      cd "$dest"
      tar -xf -
    )
  fi
}

baseline_section_fingerprint() {
  local run_id="$1" batch="$2"
  local file
  file="$(run_batch_section_fingerprint_file "$run_id" "$batch")"
  [[ -f "$file" ]] && cat "$file" || true
}

baseline_sections_fingerprint() {
  local run_id="$1" batch="$2" section="$3"
  local file
  file="$(run_batch_sections_fingerprint_file "$run_id" "$batch")"
  [[ -f "$file" ]] || return 1
  awk -F '\t' -v section="$section" '$1 == section { print $2 }' "$file"
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
# Batch manifest — 13 batches: PU00-PU12 (all independent)
# ---------------------------------------------------------------------------

ALL_BATCHES=(
  "PU00" "PU01" "PU02" "PU03" "PU04" "PU05" "PU06"
  "PU07" "PU08" "PU09" "PU10" "PU11" "PU12"
)

PHASE2_SECTIONS=("00" "01" "02" "03" "04" "05" "06" "07" "08" "09" "10" "11" "12")

phase2_section_name() {
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
    PU0[0-9]|PU1[0-2])
      local s="${1#PU}"
      echo "Refresh docs-parity/$s ($(phase2_section_name "$s")) with audit-refined docs"
      ;;
    *) return 1 ;;
  esac
}

batch_group() {
  echo "parity-update"
}

batch_deps() {
  # All PU batches are independent of each other
  echo ""
}

# ---------------------------------------------------------------------------
# Preflight
# ---------------------------------------------------------------------------

preflight_check() {
  local errors=0
  log_header "PREFLIGHT (Phase 2 — Parity Refresh)"

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

  if [[ ! -d "$DOCS_PARITY_DIR" ]]; then
    log_err "preflight" "docs-parity dir missing: $DOCS_PARITY_DIR"
    errors=$((errors + 1))
  else
    log_ok "preflight" "docs-parity dir: $(ls -d "$DOCS_PARITY_DIR"/*/ 2>/dev/null | wc -l | tr -d ' ') sections"
  fi

  ensure_dir "$PU_LOG_ROOT"

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
    log_warn "preflight" "repo has $dirty_count uncommitted change(s)"
  else
    log_ok "preflight" "repo is clean"
  fi

  return "$errors"
}
