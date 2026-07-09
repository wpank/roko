#!/usr/bin/env bash

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

: "${DP_MODEL:=gpt-5.4}"
: "${DP_REASONING:=high}"
: "${DP_TIMEOUT:=5400}"

emit_shared_context_pack() {
  cat <<'PACK_EOF'
## Shared Context Pack

PACK_EOF

  local file title
  for file in \
    "$CONTEXT_DIR/00-DOCS-PARITY-RULES.md" \
    "$CONTEXT_DIR/01-SECTION-CRATE-MAP.md" \
    "$CONTEXT_DIR/02-WORKSPACE-TOPOLOGY.md" \
    "$CONTEXT_DIR/03-EXISTING-PARITY-SUMMARY.md" \
    "$CONTEXT_DIR/04-CODE-CONVENTIONS.md" \
    "$CONTEXT_DIR/05-PHASE2-STUB-GUIDANCE.md"; do
    [[ -f "$file" ]] || continue
    title="$(basename "$file" .md)"
    printf '### %s\n\n' "$title"
    cat "$file"
    printf '\n'
  done
}

emit_delegation_guidance() {
  local batch="$1"
  cat <<'DELEG_EOF'
## Delegation Requirement

You are explicitly authorized to use multiple subagents for this batch.
Use them aggressively where it helps, but keep the immediate blocking work local.

Required delegation behavior:

- Before coding, form a short plan and identify 2-4 concrete sidecar subtasks.
- Spawn explorers for targeted codebase questions and workers for bounded code edits.
- Each subagent gets the same context pack (`context-pack/00-05`) plus its specific task.
- Give each worker a disjoint write scope and tell them they are not alone in the codebase.
- Do not wait idly for subagents if you can make progress locally.
- If subagents are unavailable in this environment, continue locally without failing.
DELEG_EOF
  echo
}

do_timeout() {
  local seconds="$1"; shift
  if command -v timeout >/dev/null 2>&1; then timeout "$seconds" "$@"
  elif command -v gtimeout >/dev/null 2>&1; then gtimeout "$seconds" "$@"
  else "$@"; fi
}

compose_prompt_snapshot() {
  local batch="$1" run_id="$2" attempt="$3" failure_file="$4"
  local out; out="$(run_prompt_snapshot "$run_id" "$batch")"
  ensure_dir "$(dirname "$out")"
  {
    echo "# Docs-Parity Batch $batch"
    echo
    echo "Run id: $run_id"
    echo "Attempt: $attempt"
    echo "Model: $DP_MODEL"
    echo "Reasoning: $DP_REASONING"
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
    emit_delegation_guidance "$batch"
    cat "$(batch_prompt_file "$batch")"
  } > "$out"
  echo "$out"
}

spawn_batch() {
  local batch="$1" run_id="$2" worktree="$3" attempt="$4" failure_file="$5"
  local prompt_snapshot log_file last_message_file target_dir
  prompt_snapshot=$(compose_prompt_snapshot "$batch" "$run_id" "$attempt" "$failure_file")
  log_file=$(run_log_file "$run_id" "$batch")
  last_message_file=$(run_last_message_file "$run_id" "$batch")
  target_dir=$(batch_target_dir "$run_id" "$batch" "codex" "$attempt")
  : > "$last_message_file"
  rm -rf "$target_dir"; mkdir -p "$target_dir"

  local start_ts exit_code=0
  start_ts=$(date +%s)
  {
    echo "=== Batch: $batch ($(batch_title "$batch")) ==="
    echo "=== Started: $(date -Iseconds) ==="
    echo "=== Worktree: $worktree ==="
    echo "=== Model: $DP_MODEL ==="
    echo "=== Reasoning: $DP_REASONING ==="
    echo "=== Timeout: $DP_TIMEOUT ==="
    echo "=== Cargo target: $target_dir ==="
    echo "=== Prompt snapshot: $prompt_snapshot ==="
    echo
  } > "$log_file"

  record_status "$run_id" "$batch" "$attempt" "spawn_started" "codex exec started"
  do_timeout "$DP_TIMEOUT" \
    env CARGO_TARGET_DIR="$target_dir" \
    codex exec \
      --model "$DP_MODEL" \
      --sandbox workspace-write \
      --full-auto \
      -c "model_reasoning_effort=$DP_REASONING" \
      --cd "$worktree" \
      -o "$last_message_file" \
      - \
      < "$prompt_snapshot" >> "$log_file" 2>&1 || exit_code=$?

  local end_ts elapsed
  end_ts=$(date +%s); elapsed=$((end_ts - start_ts))
  { echo; echo "=== Finished: $(date -Iseconds) ==="; echo "=== Duration: $(fmt_duration "$elapsed") ==="; echo "=== Exit code: $exit_code ==="; } >> "$log_file"

  if [[ "$exit_code" -eq 0 ]]; then
    record_status "$run_id" "$batch" "$attempt" "spawn_succeeded" "codex completed"
    log_ok "$batch" "Codex completed in $(fmt_duration "$elapsed")"; return 0
  fi
  if [[ "$exit_code" -eq 124 ]]; then
    record_status "$run_id" "$batch" "$attempt" "spawn_timed_out" "timed out"
    log_err "$batch" "Timed out after $(fmt_duration "$DP_TIMEOUT")"; return 124
  fi
  record_status "$run_id" "$batch" "$attempt" "spawn_failed" "exited with $exit_code"
  log_err "$batch" "Codex exited with code $exit_code"; return "$exit_code"
}
