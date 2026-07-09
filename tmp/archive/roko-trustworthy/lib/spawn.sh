#!/usr/bin/env bash

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

emit_shared_context_pack() {
  cat <<'PACK_EOF'
## Shared Context Pack

This is a no-prior-context Codex run. Treat this context pack as the runner
orientation, then read the repository files and source documents named by the
batch prompt before editing.

PACK_EOF

  local file title
  for file in \
    "$CONTEXT_DIR/00-TRUSTWORTHY-RULES.md" \
    "$CONTEXT_DIR/01-ROKO-FIRST-ROADMAP.md" \
    "$CONTEXT_DIR/02-WORKSPACE-TOPOLOGY.md" \
    "$CONTEXT_DIR/03-ARCHITECTURE-PLAN-MAP.md" \
    "$CONTEXT_DIR/04-SELF-HOSTING-GATES.md" \
    "$CONTEXT_DIR/05-CYBERNETIC-POLICY-PRIMER.md"; do
    [[ -f "$file" ]] || continue
    title="$(basename "$file" .md)"
    printf '### %s\n\n' "$title"
    cat "$file"
    printf '\n'
  done
}

emit_runner_contract() {
  local batch="$1"
  cat <<CONTRACT_EOF
## Runner Contract

- Batch: $batch
- Title: $(batch_title "$batch")
- Group: $(batch_group "$batch")
- Dependencies: $(batch_deps "$batch")
- You are running in an isolated git worktree created by this runner.
- Keep edits scoped to the batch. Do not refactor unrelated code.
- Do not mark production behavior complete with stubs, fake pass gates, or noop implementations.
- Prefer structured data, manifests, schemas, and typed APIs over ad hoc strings.
- Add or update focused tests for changed behavior.
- Run the verification commands from the batch prompt before finishing when practical.
- If the repo already has a better local pattern than the prompt suggests, follow the repo and document the deviation in your final message.
- If subagents are available, use them only for bounded sidecar discovery or disjoint code edits. Keep the immediate blocking path local.

CONTRACT_EOF
}

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

compose_prompt_snapshot() {
  local batch="$1" run_id="$2" attempt="$3" failure_file="$4"
  local out
  out="$(run_prompt_snapshot "$run_id" "$batch")"
  ensure_dir "$(dirname "$out")"
  {
    echo "# Roko Trustworthy Batch $batch"
    echo
    echo "Run id: $run_id"
    echo "Attempt: $attempt"
    echo "Model: $RT_MODEL"
    echo "Reasoning: $RT_REASONING"
    echo
    if [[ -s "$failure_file" ]]; then
      echo "## Previous Attempt Failure Context"
      echo
      cat "$failure_file"
      echo
      echo "Use that context to avoid repeating the same failure."
      echo
    fi
    emit_shared_context_pack
    emit_runner_contract "$batch"
    cat "$(batch_prompt_file "$batch")"
  } > "$out"
  echo "$out"
}

spawn_batch() {
  local batch="$1" run_id="$2" worktree="$3" attempt="$4" failure_file="$5"
  local prompt_snapshot log_file last_message_file target_dir
  prompt_snapshot="$(compose_prompt_snapshot "$batch" "$run_id" "$attempt" "$failure_file")"
  log_file="$(run_log_file "$run_id" "$batch")"
  last_message_file="$(run_last_message_file "$run_id" "$batch")"
  target_dir="$(batch_target_dir "$run_id" "$batch" "codex" "$attempt")"
  : > "$last_message_file"
  rm -rf "$target_dir"
  mkdir -p "$target_dir"

  local start_ts exit_code=0
  start_ts="$(date +%s)"
  {
    echo "=== Batch: $batch ($(batch_title "$batch")) ==="
    echo "=== Started: $(date -Iseconds) ==="
    echo "=== Worktree: $worktree ==="
    echo "=== Model: $RT_MODEL ==="
    echo "=== Reasoning: $RT_REASONING ==="
    echo "=== Timeout: $RT_TIMEOUT ==="
    echo "=== Cargo target: $target_dir ==="
    echo "=== Prompt snapshot: $prompt_snapshot ==="
    echo
  } > "$log_file"

  record_status "$run_id" "$batch" "$attempt" "spawn_started" "codex exec started"
  do_timeout "$RT_TIMEOUT" \
    env CARGO_TARGET_DIR="$target_dir" \
    codex exec \
      --model "$RT_MODEL" \
      --sandbox workspace-write \
      --full-auto \
      -c "model_reasoning_effort=$RT_REASONING" \
      --cd "$worktree" \
      -o "$last_message_file" \
      - \
      < "$prompt_snapshot" >> "$log_file" 2>&1 || exit_code=$?

  local end_ts elapsed
  end_ts="$(date +%s)"
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
    record_status "$run_id" "$batch" "$attempt" "spawn_timed_out" "timed out"
    log_err "$batch" "Timed out after $(fmt_duration "$RT_TIMEOUT")"
    return 124
  fi
  record_status "$run_id" "$batch" "$attempt" "spawn_failed" "exited with $exit_code"
  log_err "$batch" "Codex exited with code $exit_code"
  return "$exit_code"
}
