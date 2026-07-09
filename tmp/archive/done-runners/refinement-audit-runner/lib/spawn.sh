#!/usr/bin/env bash

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

: "${RAR_MODEL:=gpt-5.4}"
: "${RAR_REASONING:=high}"
: "${RAR_TIMEOUT:=5400}"

# ---------------------------------------------------------------------------
# Shared context emitter — injected into every prompt
# ---------------------------------------------------------------------------

emit_shared_context_pack() {
  cat <<'EOF'
## Shared Context Pack

EOF

  local file title
  for file in \
    "$CONTEXT_DIR/00-AUDIT-RULES.md" \
    "$CONTEXT_DIR/01-PRIORITY-QUEUE.md" \
    "$CONTEXT_DIR/02-DOCS-TREE-MAP.md" \
    "$CONTEXT_DIR/03-WORKSPACE-TOPOLOGY.md" \
    "$CONTEXT_DIR/04-DELEGATION-GUIDANCE.md"; do
    [[ -f "$file" ]] || continue
    title="$(basename "$file" .md)"
    printf '### %s\n\n' "$title"
    cat "$file"
    printf '\n'
  done
}

# ---------------------------------------------------------------------------
# Audit source emitter — injects the relevant audit files for AUD batches
# ---------------------------------------------------------------------------

emit_audit_sources() {
  local batch="$1"
  local files_str
  files_str="$(batch_audit_files "$batch")"
  [[ -z "$files_str" ]] && return 0

  echo "## Audit Source Files"
  echo
  echo "These are the critique/triage documents that drive your edits."
  echo "Read them carefully — they contain specific verdicts (keep/narrow/defer/rewrite)"
  echo "and codebase reality checks."
  echo

  local -a files_arr=()
  IFS=' ' read -ra files_arr <<< "$files_str"
  for f in "${files_arr[@]}"; do
    if [[ -f "$f" ]]; then
      printf -- '--- BEGIN %s ---\n\n' "$(basename "$f")"
      cat "$f"
      printf '\n--- END %s ---\n\n' "$(basename "$f")"
    fi
  done

  # Always include master summary as reference for non-AUD01 batches
  if [[ "$batch" != "AUD01" ]] && [[ -f "$AUDIT_DIR/00-MASTER-SUMMARY.md" ]]; then
    echo "## Master Summary (reference)"
    echo
    cat "$AUDIT_DIR/00-MASTER-SUMMARY.md"
    echo
  fi

  # Always include refinement matrix for triage lookup
  if [[ "$batch" != "AUD01" ]] && [[ -f "$AUDIT_DIR/05-refinement-matrix.md" ]]; then
    echo "## Refinement Matrix (per-REF verdicts)"
    echo
    cat "$AUDIT_DIR/05-refinement-matrix.md"
    echo
  fi
}

# ---------------------------------------------------------------------------
# Prompt composition — context pack + failure context + per-batch prompt file
#
# This mirrors the ux-followup-runner pattern exactly:
#   1. Header with run metadata
#   2. Failure context from prior attempt (if any)
#   3. Shared context pack (5 files)
#   4. Audit sources (for AUD batches)
#   5. The per-batch prompt file (the detailed spec)
# ---------------------------------------------------------------------------

compose_prompt_snapshot() {
  local batch="$1" run_id="$2" attempt="$3" failure_file="$4"
  local out
  out="$(run_prompt_snapshot "$run_id" "$batch")"
  ensure_dir "$(dirname "$out")"

  {
    # Header
    echo "# Refinement Audit Runner — Batch $batch"
    echo
    echo "Run id: $run_id"
    echo "Attempt: $attempt"
    echo "Model: $RAR_MODEL"
    echo "Reasoning: $RAR_REASONING"
    echo

    # Failure context from prior attempt
    if [[ -s "$failure_file" ]]; then
      echo "## Previous attempt failure context"
      echo
      cat "$failure_file"
      echo
      echo "Use that context to avoid repeating the same failure."
      echo
    fi

    # Shared context pack
    emit_shared_context_pack

    # Audit source files (only for AUD batches — rich context)
    case "$batch" in
      AUD*) emit_audit_sources "$batch" ;;
    esac

    # The per-batch prompt file — the detailed implementation spec
    cat "$(batch_prompt_file "$batch")"
  } > "$out"

  echo "$out"
}

# ---------------------------------------------------------------------------
# Codex invocation
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
  local batch="$1" run_id="$2" worktree="$3" attempt="$4" failure_file="$5"

  local prompt_snapshot
  prompt_snapshot=$(compose_prompt_snapshot "$batch" "$run_id" "$attempt" "$failure_file")
  local log_file
  log_file=$(run_log_file "$run_id" "$batch")
  local last_message_file
  last_message_file=$(run_last_message_file "$run_id" "$batch")
  local target_dir
  target_dir=$(batch_target_dir "$run_id" "$batch" "codex" "$attempt")
  : > "$last_message_file"
  rm -rf "$target_dir"
  mkdir -p "$target_dir"

  local start_ts exit_code=0
  start_ts=$(date +%s)

  {
    echo "=== Batch: $batch ($(batch_title "$batch")) ==="
    echo "=== Started: $(date -Iseconds) ==="
    echo "=== Worktree: $worktree ==="
    echo "=== Model: $RAR_MODEL ==="
    echo "=== Reasoning: $RAR_REASONING ==="
    echo "=== Timeout: $RAR_TIMEOUT ==="
    echo "=== Cargo target: $target_dir ==="
    echo "=== Prompt file: $(batch_prompt_file "$batch") ==="
    echo "=== Prompt snapshot: $prompt_snapshot ==="
    echo
  } > "$log_file"

  # PU batches edit tmp/docs-parity/ which codex workspace-write sandbox
  # treats as outside the source tree (overlay FS discards writes).
  # Fix: run PU batches in the MAIN repo with --full-auto (sandbox still on
  # but the files are real, not in a worktree overlay).
  # AUD/PE batches run in the worktree as normal.
  local work_dir="$worktree"
  case "$batch" in
    PU*) work_dir="$ROKO_ROOT" ;;
  esac

  record_status "$run_id" "$batch" "$attempt" "spawn_started" "codex exec (cd=$work_dir)"

  do_timeout "$RAR_TIMEOUT" \
    env CARGO_TARGET_DIR="$target_dir" \
    codex exec \
      --model "$RAR_MODEL" \
      --full-auto \
      -c "model_reasoning_effort=$RAR_REASONING" \
      --cd "$work_dir" \
      -o "$last_message_file" \
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
    record_status "$run_id" "$batch" "$attempt" "spawn_succeeded" "codex exec completed"
    log_ok "$batch" "Codex completed in $(fmt_duration "$elapsed")"
    return 0
  fi

  if [[ "$exit_code" -eq 124 ]]; then
    record_status "$run_id" "$batch" "$attempt" "spawn_timed_out" "codex exec timed out"
    log_err "$batch" "Timed out after $(fmt_duration "$RAR_TIMEOUT")"
    return 124
  fi

  record_status "$run_id" "$batch" "$attempt" "spawn_failed" "codex exec exited with $exit_code"
  log_err "$batch" "Codex exited with code $exit_code"
  return "$exit_code"
}
