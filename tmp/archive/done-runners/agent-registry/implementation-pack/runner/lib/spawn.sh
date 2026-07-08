#!/usr/bin/env bash

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

: "${AR_MODEL:=gpt-5.4}"
: "${AR_REASONING:=high}"
: "${AR_TIMEOUT:=7200}"
: "${AR_CODEX_FAST_PROFILE:=fast}"

has_codex_profile() {
  local profile="${1:-}"
  local config_file="${HOME}/.codex/config.toml"
  [[ -n "$profile" && -f "$config_file" ]] || return 1
  rg -n "^\[profiles\\.${profile//./\\.}\]$" "$config_file" >/dev/null 2>&1
}

emit_codex_mode_summary() {
  if has_codex_profile "$AR_CODEX_FAST_PROFILE"; then
    printf 'profile:%s' "$AR_CODEX_FAST_PROFILE"
  else
    printf 'fallback:model_reasoning_effort=%s' "$AR_REASONING"
  fi
}

emit_shared_context_pack() {
  cat <<'EOF'
## Shared Context Pack

EOF

  local file title
  for file in \
    "$CONTEXT_DIR/00-READ-FIRST.md" \
    "$CONTEXT_DIR/01-TARGET-STATE.md" \
    "$CONTEXT_DIR/02-CODE-MAP.md" \
    "$CONTEXT_DIR/03-VERIFICATION-MATRIX.md"; do
    [[ -f "$file" ]] || continue
    title="$(basename "$file" .md)"
    printf '### %s\n\n' "$title"
    cat "$file"
    printf '\n'
  done
}

emit_runner_expectations() {
  cat <<'EOF'
## Runner Expectations

- Work in the current git worktree only.
- Satisfy the prompt's acceptance criteria before stopping.
- Run the prompt's verification commands when possible.
- If a verification command fails, fix the code instead of just reporting it.
- Commit is handled by the outer runner; do not create or amend commits yourself
  unless the prompt explicitly requires it.
- Use multiple subagents if useful, but keep the immediate blocking work local.
EOF
}

emit_delegation_guidance() {
  local batch="$1"
  cat <<'EOF'
## Delegation Requirement

You are explicitly authorized to use multiple subagents for this batch.

Required delegation behavior:

- Form a short plan first.
- Spawn explorers for targeted codebase questions.
- Spawn workers for bounded code edits with disjoint write scopes.
- Every subagent gets the same context pack and the same batch prompt.
- Do not wait idly for subagents if you can make progress locally.
- If subagents are unavailable, continue locally without failing.
EOF

  printf '\nSuggested parallel split for batch `%s`:\n\n' "$batch"
  case "$batch" in
    AR01)
      cat <<'EOF'
- explorer: inspect current contract and mirage bootstrap surfaces
- worker: target contract files and tests
- worker: mirage boot/fork integration and legacy registry deprecation
EOF
      ;;
    AR02)
      cat <<'EOF'
- explorer: inspect axum/tokio patterns and minimal relay architecture
- worker: relay state/routes
- worker: WS protocol + forwarding tests
EOF
      ;;
    AR03)
      cat <<'EOF'
- explorer: inspect CLI command tree and existing agent-server smoke tests
- worker: CLI command wiring
- worker: tests and config/runtime hookup
EOF
      ;;
    AR04)
      cat <<'EOF'
- explorer: inspect registration ABI mismatch and relay integration seams
- worker: relay client
- worker: target ABI chain registration path and tests
EOF
      ;;
    AR05)
      cat <<'EOF'
- explorer: inspect Docker/runtime/proxy seams
- worker: Docker and entrypoint/runtime wiring
- worker: same-origin relay path exposure
EOF
      ;;
    AR06)
      cat <<'EOF'
- explorer: inspect static demo discovery and transport assumptions
- worker: static UI API/state changes
- worker: quickstart and message-path changes
EOF
      ;;
    AR07)
      cat <<'EOF'
- explorer: identify exact operator gaps for the remote mixed-topology demo
- worker: remote mirage + relay operator docs/assets
- worker: remote/local agent and demo UI operator docs/assets
EOF
      ;;
    AR08)
      cat <<'EOF'
- explorer: inspect current Kauri dashboard transport/config seams
- worker: dashboard API/constants changes
- worker: AskPanel and agent routing changes
EOF
      ;;
    *)
      cat <<'EOF'
- explorer: targeted read-only codebase questions
- worker: first bounded implementation slice
- worker: second bounded implementation slice
EOF
      ;;
  esac
}

do_timeout() {
  local seconds="$1"
  shift
  if command -v timeout >/dev/null 2>&1; then
    timeout "$seconds" "$@"
  elif command -v gtimeout >/dev/null 2>&1; then
    gtimeout "$seconds" "$@"
  else
    log_err "runner" "Neither timeout nor gtimeout is available"
    return 127
  fi
}

compose_prompt_snapshot() {
  local batch="$1"
  local run_id="$2"
  local attempt="$3"
  local failure_file="$4"
  local out
  out="$(run_prompt_snapshot "$run_id" "$batch" "$attempt")"
  ensure_dir "$(dirname "$out")"

  {
    echo "# Agent Registry Batch $batch"
    echo
    echo "Run id: $run_id"
    echo "Attempt: $attempt"
    echo "Model: $AR_MODEL"
    echo "Codex mode: $(emit_codex_mode_summary)"
    echo "Group: $(batch_group "$batch")"
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
    emit_runner_expectations
    emit_delegation_guidance "$batch"
    cat "$(batch_prompt_file "$batch")"
  } > "$out"

  echo "$out"
}

spawn_batch() {
  local batch="$1"
  local run_id="$2"
  local worktree="$3"
  local attempt="$4"
  local failure_file="$5"

  local prompt_snapshot log_file last_message_file target_dir
  prompt_snapshot=$(compose_prompt_snapshot "$batch" "$run_id" "$attempt" "$failure_file")
  log_file=$(run_log_file "$run_id" "$batch" "$attempt")
  last_message_file=$(run_last_message_file "$run_id" "$batch" "$attempt")
  target_dir=$(batch_target_dir "$run_id" "$batch" "codex" "$attempt")

  : > "$last_message_file"
  rm -rf "$target_dir"
  mkdir -p "$target_dir"

  local start_ts exit_code end_ts elapsed
  start_ts=$(date +%s)
  exit_code=0

  {
    echo "=== Batch: $batch ($(batch_title "$batch")) ==="
    echo "=== Started: $(date -Iseconds) ==="
    echo "=== Worktree: $worktree ==="
    echo "=== Model: $AR_MODEL ==="
    echo "=== Codex mode: $(emit_codex_mode_summary) ==="
    echo "=== Timeout: $AR_TIMEOUT ==="
    echo "=== Cargo target: $target_dir ==="
    echo "=== Prompt snapshot: $prompt_snapshot ==="
    echo
  } > "$log_file"

  record_status "$run_id" "$batch" "$attempt" "spawn_started" "codex exec started"

  local -a codex_cmd=(
    codex exec
    --model "$AR_MODEL"
    --sandbox workspace-write
    --full-auto
    --cd "$worktree"
    -o "$last_message_file"
  )

  if has_codex_profile "$AR_CODEX_FAST_PROFILE"; then
    codex_cmd+=(--profile "$AR_CODEX_FAST_PROFILE")
  else
    codex_cmd+=(-c "model_reasoning_effort=$AR_REASONING")
  fi

  codex_cmd+=(-)

  do_timeout "$AR_TIMEOUT" \
    env CARGO_TARGET_DIR="$target_dir" \
    "${codex_cmd[@]}" \
    < "$prompt_snapshot" >> "$log_file" 2>&1 || exit_code=$?

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
    rm -rf "$target_dir"
    return 0
  fi

  if [[ "$exit_code" -eq 124 ]]; then
    record_status "$run_id" "$batch" "$attempt" "spawn_timed_out" "codex exec timed out"
    log_err "$batch" "Timed out after $(fmt_duration "$AR_TIMEOUT")"
    rm -rf "$target_dir"
    return 124
  fi

  record_status "$run_id" "$batch" "$attempt" "spawn_failed" "codex exec exited with $exit_code"
  log_err "$batch" "Codex exited with code $exit_code"
  rm -rf "$target_dir"
  return "$exit_code"
}
