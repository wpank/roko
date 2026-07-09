#!/usr/bin/env bash

source "$(dirname "${BASH_SOURCE[0]}")/pu-common.sh"

: "${PU_MODEL:=gpt-5.4}"
: "${PU_REASONING:=high}"
: "${PU_TIMEOUT:=5400}"

# ---------------------------------------------------------------------------
# Shared context emitter — injected into every prompt
# ---------------------------------------------------------------------------

emit_shared_context_pack() {
  cat <<'EOF'
## Shared Context Pack

EOF

  local file title
  for file in \
    "$PU_CONTEXT_DIR/00-AUDIT-RULES.md" \
    "$PU_CONTEXT_DIR/01-PRIORITY-QUEUE.md" \
    "$PU_CONTEXT_DIR/02-DOCS-TREE-MAP.md" \
    "$PU_CONTEXT_DIR/03-WORKSPACE-TOPOLOGY.md" \
    "$PU_CONTEXT_DIR/04-DELEGATION-GUIDANCE.md"; do
    [[ -f "$file" ]] || continue
    title="$(basename "$file" .md)"
    printf '### %s\n\n' "$title"
    cat "$file"
    printf '\n'
  done
}

# ---------------------------------------------------------------------------
# Prompt composition — context pack + failure context + per-batch prompt file
# ---------------------------------------------------------------------------

compose_prompt_snapshot() {
  local batch="$1" run_id="$2" attempt="$3" failure_file="$4"
  local out
  out="$(run_prompt_snapshot "$run_id" "$batch")"
  ensure_dir "$(dirname "$out")"

  {
    echo "# Parity Refresh — Batch $batch"
    echo
    echo "Run id: $run_id"
    echo "Attempt: $attempt"
    echo "Model: $PU_MODEL"
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

    cat "$(batch_prompt_file "$batch")"
  } > "$out"

  echo "$out"
}

# ---------------------------------------------------------------------------
# Codex invocation — uses --dangerously-bypass-approvals-and-sandbox
#
# PU batches edit tmp/docs-parity/ which codex --sandbox workspace-write
# blocks. --dangerously-bypass-approvals-and-sandbox disables the sandbox
# entirely so writes to tmp/ persist on disk.
# ---------------------------------------------------------------------------

do_timeout() {
  local seconds="$1"; shift
  if command -v timeout >/dev/null 2>&1; then
    timeout "$seconds" "$@"
  elif command -v gtimeout >/dev/null 2>&1; then
    gtimeout "$seconds" "$@"
  else
    "$@"
  fi
}

spawn_batch() {
  local batch="$1" run_id="$2" _worktree="$3" attempt="$4" failure_file="$5"

  local prompt_snapshot
  prompt_snapshot=$(compose_prompt_snapshot "$batch" "$run_id" "$attempt" "$failure_file")
  local log_file
  log_file=$(run_log_file "$run_id" "$batch")

  local start_ts exit_code=0
  start_ts=$(date +%s)

  {
    echo "=== Batch: $batch ($(batch_title "$batch")) ==="
    echo "=== Started: $(date -Iseconds) ==="
    echo "=== Workdir: $ROKO_ROOT (main repo) ==="
    echo "=== Model: $PU_MODEL ==="
    echo "=== Reasoning: $PU_REASONING ==="
    echo "=== Timeout: $PU_TIMEOUT ==="
    echo "=== Prompt snapshot: $prompt_snapshot ==="
    echo
  } > "$log_file"

  record_status "$run_id" "$batch" "$attempt" "spawn_started" "codex exec started"

  do_timeout "$PU_TIMEOUT" \
    codex exec \
      --model "$PU_MODEL" \
      --full-auto \
      -c "model_reasoning_effort=$PU_REASONING" \
      --cd "$ROKO_ROOT" \
      - \
      < "$prompt_snapshot" >> "$log_file" 2>&1 || exit_code=$?

  local end_ts elapsed
  end_ts=$(date +%s)
  elapsed=$((end_ts - start_ts))

  {
    echo
    echo "=== Finished: $(date -Iseconds) ==="
    echo "=== Duration: $(fmt_duration "$elapsed") ==="
    echo "=== Exit code: $exit_code ==="
  } >> "$log_file"

  if [[ "$exit_code" -eq 0 ]]; then
    record_status "$run_id" "$batch" "$attempt" "spawn_succeeded" "codex completed"
    log_ok "$batch" "Codex completed in $(fmt_duration "$elapsed")"
    return 0
  fi

  if [[ "$exit_code" -eq 124 ]]; then
    record_status "$run_id" "$batch" "$attempt" "spawn_timed_out" "codex timed out"
    log_err "$batch" "Timed out after $(fmt_duration "$PU_TIMEOUT")"
    return 124
  fi

  record_status "$run_id" "$batch" "$attempt" "spawn_failed" "codex exited with $exit_code"
  log_err "$batch" "Codex exited with code $exit_code"
  return "$exit_code"
}
