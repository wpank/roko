#!/usr/bin/env bash

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

# ---------------------------------------------------------------------------
# spawn.sh — Agent dispatch via codex exec
#
# Uses: codex exec --model gpt-5.4 --full-auto --cd <worktree> --json - < prompt
#   --cd:        sets working directory (codex native, no cd hacks)
#   --json:      streams JSONL events to stdout (every tool call, message, result)
#   --full-auto: no confirmation prompts
#   -s workspace-write: can write files in the worktree
#   stdin:       prompt piped in (no arg length limits)
#
# Logs: everything (stdout+stderr) goes to the log file. tail -f works.
# ---------------------------------------------------------------------------

: "${MR_MODEL:=gpt-5.4}"
: "${MR_REASONING:=high}"
: "${MR_TIMEOUT:=7200}"

# ---------------------------------------------------------------------------
# Shared context emission (unchanged)
# ---------------------------------------------------------------------------

emit_shared_context_pack() {
  cat <<'EOF'
## Shared Context Pack

EOF

  local file title
  for file in \
    "$CONTEXT_DIR/00-orientation.md" \
    "$CONTEXT_DIR/01-unified-vocabulary.md" \
    "$CONTEXT_DIR/02-migration-rules.md" \
    "$CONTEXT_DIR/03-coding-conventions.md" \
    "$CONTEXT_DIR/04-verification-gates.md" \
    "$CONTEXT_DIR/05-architecture-context.md"; do
    [[ -f "$file" ]] || continue
    title="$(basename "$file" .md)"
    printf '### %s\n\n' "$title"
    cat "$file"
    printf '\n'
  done
}

emit_batch_context_files() {
  local batch="$1"
  local context_file="$MR_ROOT/context/$batch"
  if [[ -d "$context_file" ]]; then
    echo "## Batch-Specific Context"
    echo
    local f
    for f in "$context_file"/*.md; do
      [[ -f "$f" ]] || continue
      printf '### %s\n\n' "$(basename "$f" .md)"
      cat "$f"
      printf '\n'
    done
  elif [[ -f "${context_file}.md" ]]; then
    echo "## Batch-Specific Context"
    echo
    cat "${context_file}.md"
    echo
  fi
}

# ---------------------------------------------------------------------------
# Timeout wrapper (macOS compat)
# ---------------------------------------------------------------------------

do_timeout() {
  local seconds="$1"
  shift
  if command -v timeout >/dev/null 2>&1; then
    timeout --signal=TERM --kill-after=30 "$seconds" "$@"
  elif command -v gtimeout >/dev/null 2>&1; then
    gtimeout --signal=TERM --kill-after=30 "$seconds" "$@"
  else
    "$@"
  fi
}

# ---------------------------------------------------------------------------
# Prompt composition
# ---------------------------------------------------------------------------

compose_prompt_snapshot() {
  local batch="$1"
  local run_id="$2"
  local attempt="$3"
  local failure_file="$4"
  local out
  out="$(run_prompt_snapshot "$run_id" "$batch")"
  ensure_dir "$(dirname "$out")"

  {
    echo "# Unified Migration Batch $batch"
    echo
    echo "Run id: $run_id"
    echo "Attempt: $attempt"
    echo "Model: $MR_MODEL (reasoning: $MR_REASONING)"
    echo "Phase ref: $(batch_phase_ref "$batch")"
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
    emit_batch_context_files "$batch"
    echo "## Batch Prompt"
    echo
    cat "$(batch_prompt_file "$batch")"
  } > "$out"

  echo "$out"
}

compose_megabatch_prompt_for_agent() {
  local mb="$1"
  local agent="$2"
  local run_id="$3"
  local -a task_ids=()
  shift 3
  task_ids=("$@")

  local out="$LOG_ROOT/$run_id/prompts/${mb}-agent-${agent}.prompt.md"
  ensure_dir "$(dirname "$out")"

  {
    echo "# Mega-Batch $mb: Agent $agent"
    echo
    echo "Run: $run_id"
    echo "Model: $MR_MODEL (reasoning: $MR_REASONING)"
    echo "Tasks: $(printf '%s ' "${task_ids[@]}" | sed 's/ $//')"
    echo

    emit_shared_context_pack

    cat <<'RULES'
## Execution Rules

1. Complete tasks in listed order (dependencies noted per task).
2. Do NOT run cargo check/test — the runner handles verification externally.
3. Reference tmp/unified-migration/ and tmp/architecture/ for naming.
4. Commit changes after each task: `migration(MXXX): <title>`.
5. You are one of multiple parallel agents — stay within your crate scope.
6. Write unit tests for any new public functions.
7. If you need changes in another agent's crates, leave a TODO comment.
RULES
    echo

    local i=0 task prompt_file
    for task in "${task_ids[@]}"; do
      i=$((i + 1))
      echo "---"
      echo
      echo "## Task $i/${#task_ids[@]}: $task — $(batch_title "$task")"
      echo
      echo "Phase ref: $(batch_phase_ref "$task")"
      echo "Dependencies: $(batch_deps "$task")"
      echo "Affected crates: $(batch_crates "$task")"
      echo

      prompt_file="$(batch_prompt_file "$task")"
      if [[ -f "$prompt_file" ]]; then
        cat "$prompt_file"
      else
        echo "[WARNING: prompt file not found: $prompt_file]"
      fi
      echo
    done
  } > "$out"

  echo "$out"
}

# ---------------------------------------------------------------------------
# Core spawn function — all dispatch goes through here
#
# Args: label log_file worktree target_dir prompt_file
# Returns: 0=success, 124=timeout, other=failure
# Logs: JSONL stream + header/footer to log_file (tail -f friendly)
# ---------------------------------------------------------------------------

_spawn_codex() {
  local label="$1"
  local run_id="$2"
  local log_file="$3"
  local worktree="$4"
  local target_dir="$5"
  local prompt_file="$6"
  local last_message_file="${7:-}"

  # Write header
  {
    echo "=== $label ==="
    echo "=== Started: $(date -Iseconds) ==="
    echo "=== Worktree: $worktree ==="
    echo "=== Model: $MR_MODEL (reasoning: $MR_REASONING) ==="
    echo "=== Timeout: ${MR_TIMEOUT}s ==="
    echo "=== Cargo target: $target_dir ==="
    echo "=== Prompt: $prompt_file ($(wc -c < "$prompt_file" | tr -d ' ') bytes) ==="
    echo
  } > "$log_file"

  local start_ts exit_code=0
  start_ts=$(date +%s)

  # Dispatch: codex exec with JSONL streaming
  #   --json:  streams every event (tool_call, message, result) as JSONL
  #   --cd:    native working directory (no cd hacks)
  #   --full-auto: no prompts
  #   stdin:   prompt (no arg length limits)
  do_timeout "$MR_TIMEOUT" \
    env CARGO_TARGET_DIR="$target_dir" \
    codex exec \
      --model "$MR_MODEL" \
      -c "model_reasoning_effort=$MR_REASONING" \
      --sandbox workspace-write \
      --full-auto \
      --json \
      --cd "$worktree" \
      - < "$prompt_file" \
      >> "$log_file" 2>&1 || exit_code=$?

  local end_ts elapsed
  end_ts=$(date +%s)
  elapsed=$((end_ts - start_ts))

  # Write footer
  {
    echo
    echo "=== Finished: $(date -Iseconds) ==="
    echo "=== Duration: $(fmt_duration "$elapsed") ==="
    echo "=== Exit code: $exit_code ==="
  } >> "$log_file"

  # Extract last message if requested
  if [[ -n "$last_message_file" ]] && command -v jq >/dev/null 2>&1; then
    grep '"type"' "$log_file" 2>/dev/null \
      | jq -r 'select(.type == "message") | .content // empty' 2>/dev/null \
      | tail -1 > "$last_message_file" 2>/dev/null || true
  fi

  return "$exit_code"
}

# ---------------------------------------------------------------------------
# spawn_megabatch — dispatch a mega-batch to a specific agent's worktree
# ---------------------------------------------------------------------------

spawn_megabatch() {
  local mb="$1"
  local agent="$2"
  local run_id="$3"
  local worktree="$4"
  local target_dir="$5"
  shift 5
  local -a task_ids=("$@")

  [[ ${#task_ids[@]} -eq 0 ]] && return 0

  local prompt_file
  prompt_file=$(compose_megabatch_prompt_for_agent "$mb" "$agent" "$run_id" "${task_ids[@]}")

  local log_file="$LOG_ROOT/$run_id/${mb}-agent-${agent}.log"
  local label="Mega-Batch: $mb, Agent: $agent, Tasks: $(printf '%s ' "${task_ids[@]}" | sed 's/ $//')"

  record_status "$run_id" "$mb" "agent-$agent" "spawn_started" "codex exec started"

  local exit_code=0
  _spawn_codex "$label" "$run_id" "$log_file" "$worktree" "$target_dir" "$prompt_file" || exit_code=$?

  if [[ "$exit_code" -eq 0 ]]; then
    record_status "$run_id" "$mb" "agent-$agent" "spawn_succeeded" "completed"
    log_ok "$mb" "Agent $agent completed ($(tail -3 "$log_file" | grep Duration | sed 's/.*: //'))"
    return 0
  elif [[ "$exit_code" -eq 124 ]]; then
    record_status "$run_id" "$mb" "agent-$agent" "spawn_timed_out" "timed out"
    log_err "$mb" "Agent $agent timed out"
    return 124
  else
    record_status "$run_id" "$mb" "agent-$agent" "spawn_failed" "exit code $exit_code"
    log_err "$mb" "Agent $agent failed (exit $exit_code)"
    return "$exit_code"
  fi
}

# ---------------------------------------------------------------------------
# spawn_batch — v1 sequential mode (backward compat)
# ---------------------------------------------------------------------------

spawn_batch() {
  local batch="$1"
  local run_id="$2"
  local worktree="$3"
  local attempt="$4"
  local failure_file="$5"

  local prompt_snapshot
  prompt_snapshot=$(compose_prompt_snapshot "$batch" "$run_id" "$attempt" "$failure_file")
  local log_file
  log_file=$(run_log_file "$run_id" "$batch")
  local last_message_file
  last_message_file=$(run_last_message_file "$run_id" "$batch")
  local target_dir
  target_dir=$(batch_target_dir "$run_id" "$batch" "codex" "$attempt")
  : > "$last_message_file"
  mkdir -p "$target_dir"

  local label="Batch: $batch ($(batch_title "$batch")), Attempt: $attempt"

  record_status "$run_id" "$batch" "$attempt" "spawn_started" "codex exec started"

  local exit_code=0
  _spawn_codex "$label" "$run_id" "$log_file" "$worktree" "$target_dir" "$prompt_snapshot" "$last_message_file" || exit_code=$?

  if [[ "$exit_code" -eq 0 ]]; then
    record_status "$run_id" "$batch" "$attempt" "spawn_succeeded" "codex completed"
    log_ok "$batch" "Codex completed ($(tail -3 "$log_file" | grep Duration | sed 's/.*: //'))"
  elif [[ "$exit_code" -eq 124 ]]; then
    record_status "$run_id" "$batch" "$attempt" "spawn_timed_out" "codex timed out"
    log_err "$batch" "Timed out after $(fmt_duration "$MR_TIMEOUT")"
  else
    record_status "$run_id" "$batch" "$attempt" "spawn_failed" "codex exited with $exit_code"
    log_err "$batch" "Codex exited with code $exit_code"
  fi

  return "$exit_code"
}
