#!/usr/bin/env bash

set -euo pipefail
IFS=$'\n\t'

ROOT="/Users/will/dev/nunchi/roko/roko"
LOG_ROOT="$ROOT/tmp/ux-refactoring/logs"
INTERVAL="${1:-120}"
STALL_SECONDS="${STALL_SECONDS:-900}"

latest_run_dir() {
  local target
  target="$(readlink "$LOG_ROOT/latest" 2>/dev/null || true)"
  [[ -n "$target" ]] || return 1
  printf '%s\n' "$target"
}

latest_run_id() {
  basename "$(latest_run_dir)"
}

status_file() {
  printf '%s/status.tsv\n' "$(latest_run_dir)"
}

current_batch_file() {
  printf '%s/current-batch.env\n' "$(latest_run_dir)"
}

monitor_log() {
  printf '%s/monitor.log\n' "$(latest_run_dir)"
}

runner_ps() {
  ps -axo pid,ppid,etime,stat,command | rg 'bash tmp/ux-refactoring/run-ux-refactoring.sh|codex exec --model gpt-5.4 .*ux-refactoring' || true
}

log_line() {
  printf '%s %s\n' "$(date -Iseconds)" "$*" | tee -a "$(monitor_log)"
}

snapshot() {
  local run_id status_path current_path last_status_line batch attempt batch_log
  run_id="$(latest_run_id)" || return 0
  status_path="$(status_file)"
  current_path="$(current_batch_file)"
  last_status_line="$(tail -n 1 "$status_path" 2>/dev/null || true)"

  batch="(none)"
  attempt="?"
  if [[ -f "$current_path" ]]; then
    # shellcheck disable=SC1090
    source "$current_path"
    batch="${BATCH:-$batch}"
    attempt="${ATTEMPT:-$attempt}"
  fi

  batch_log="$(latest_run_dir)/${batch}.log"
  local log_age="na"
  if [[ -f "$batch_log" ]]; then
    local now epoch
    now="$(date +%s)"
    epoch="$(stat -f %m "$batch_log" 2>/dev/null || echo "$now")"
    log_age="$((now - epoch))"
  fi

  log_line "run=$run_id batch=$batch attempt=$attempt log_age_s=$log_age status='${last_status_line:-none}'"

  local stalled=0
  if [[ -f "$status_path" ]]; then
    local now epoch
    now="$(date +%s)"
    epoch="$(stat -f %m "$status_path" 2>/dev/null || echo "$now")"
    if (( now - epoch > STALL_SECONDS )); then
      stalled=1
    fi
  fi

  if [[ "$stalled" -eq 1 ]]; then
    log_line "warning=stalled threshold_s=$STALL_SECONDS"
    runner_ps | sed 's/^/ps: /' | tee -a "$(monitor_log)"
  fi
}

mkdir -p "$LOG_ROOT"

while true; do
  snapshot || true
  sleep "$INTERVAL"
done
