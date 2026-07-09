#!/usr/bin/env bash

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

: "${ACP_MODEL:=gpt-5.4}"
: "${ACP_REASONING:=high}"
: "${ACP_TIMEOUT:=5400}"

emit_shared_context_pack() {
  cat <<'EOF'
## Shared Context Pack

EOF

  local file title
  for file in \
    "$CONTEXT_DIR/00-ACP-RULES.md" \
    "$CONTEXT_DIR/01-ACP-PROTOCOL-PRIMER.md" \
    "$CONTEXT_DIR/02-ROKO-ARCHITECTURE.md" \
    "$CONTEXT_DIR/03-TYPE-REFERENCE.md" \
    "$CONTEXT_DIR/04-EXISTING-PATTERNS.md"; do
    [[ -f "$file" ]] || continue
    title="$(basename "$file" .md)"
    printf '### %s\n\n' "$title"
    cat "$file"
    printf '\n'
  done
}

emit_delegation_guidance() {
  local batch="$1"
  cat <<'EOF'
## Delegation Requirement

You are authorized to use subagents. Prefer multiple parallel agents when
the target file set is large.

Required delegation behavior:

- Form a plan first — for each target file, decide (a) does it need changes,
  (b) how big, (c) is it self-contained.
- For large independent modules, spawn a worker per file with a disjoint
  write scope.
- Every subagent gets the same context pack.
- Do not wait idly for subagents if you can progress locally.
- If subagents are unavailable in this environment, continue locally.
EOF

  printf '\nSuggested parallel split for batch `%s`:\n\n' "$batch"

  case "$batch" in
    ACP01)
      cat <<'EOF'
- worker: create Cargo.toml and lib.rs with all module declarations
- worker: create all stub module files (types.rs, transport.rs, etc.)
- worker: wire roko-acp into workspace Cargo.toml
EOF
      ;;
    ACP02)
      cat <<'EOF'
- worker: implement JSON-RPC base types and error codes
- worker: implement initialize + session types
- worker: implement content, update, config, command, permission, elicitation types
- worker: write serde unit tests
EOF
      ;;
    ACP03)
      cat <<'EOF'
- worker: implement StdioTransport struct and read/write methods
- worker: implement bidirectional request flow (pending_requests map)
- worker: write unit tests with mock readers/writers
EOF
      ;;
    ACP04)
      cat <<'EOF'
- worker: implement run_acp_server main loop and method dispatch table
- worker: implement logging setup and error handling
- worker: implement stub handlers for later-batch methods
EOF
      ;;
    ACP05)
      cat <<'EOF'
- worker: implement AcpSession + SessionConfigState + SessionManager
- worker: implement AcpConfig + CancelToken
- worker: write unit tests for session lifecycle
EOF
      ;;
    ACP06)
      cat <<'EOF'
- worker: implement CognitiveEvent enum and event-to-notification mapping
- worker: implement stream_events_to_editor async loop
- worker: implement handle_session_prompt with busy check and cancellation
EOF
      ;;
    ACP07)
      cat <<'EOF'
- worker: add Acp variant to Commands enum in main.rs
- worker: ensure roko-acp re-exports and Cargo.toml dep is wired
EOF
      ;;
    ACP08)
      cat <<'EOF'
- worker: create test infrastructure (TestClient with mock channels)
- worker: implement initialize + session lifecycle tests
- worker: implement error handling tests (unknown method, invalid JSON, etc.)
EOF
      ;;
    ACP09|ACP10|ACP11|ACP12|ACP13|ACP14)
      cat <<'EOF'
- worker: implement the bridge struct and all methods
- worker: implement editor-mediated path + local fallback
EOF
      ;;
    ACP15)
      cat <<'EOF'
- worker: implement build_config_options with all 7 options
- worker: implement handle_config_update with dependent updates
- worker: implement legacy set_mode handler + unit tests
EOF
      ;;
    ACP16)
      cat <<'EOF'
- worker: define 8 slash commands with descriptions
- worker: implement dynamic_commands filtering + parse_slash_command
- worker: implement dispatch_command with placeholder responses
EOF
      ;;
    ACP17)
      cat <<'EOF'
- worker: implement request_elicitation JSON-RPC flow
- worker: build gate_config_schema and research_source_schema
- worker: implement basic response validation
EOF
      ;;
    ACP18)
      cat <<'EOF'
- worker: extract TestClient to shared test helper
- worker: implement config option + slash command lifecycle tests
- worker: implement session management + error handling tests
EOF
      ;;
    *)
      cat <<'EOF'
- worker: targeted edits within the batch's write scope
EOF
      ;;
  esac

  echo
}

do_timeout() {
  local seconds="$1"
  shift
  if command -v timeout >/dev/null 2>&1; then
    timeout "$seconds" "$@"
  elif command -v gtimeout >/dev/null 2>&1; then
    gtimeout "$seconds" "$@"
  else
    "$@"
  fi
}

compose_prompt_snapshot() {
  local batch="$1"
  local run_id="$2"
  local attempt="$3"
  local failure_file="$4"
  local out
  out="$(run_prompt_snapshot "$run_id" "$batch")"
  ensure_dir "$(dirname "$out")"

  {
    echo "# ACP Batch $batch"
    echo
    echo "Run id: $run_id"
    echo "Attempt: $attempt"
    echo "Model: $ACP_MODEL"
    echo "Reasoning: $ACP_REASONING"
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
  : > "$last_message_file"

  local start_ts
  start_ts=$(date +%s)
  local exit_code=0

  {
    echo "=== Batch: $batch ($(batch_title "$batch")) ==="
    echo "=== Started: $(date -Iseconds) ==="
    echo "=== Worktree: $worktree ==="
    echo "=== Model: $ACP_MODEL ==="
    echo "=== Reasoning: $ACP_REASONING ==="
    echo "=== Timeout: $ACP_TIMEOUT ==="
    echo "=== Prompt snapshot: $prompt_snapshot ==="
    echo
  } > "$log_file"

  record_status "$run_id" "$batch" "$attempt" "spawn_started" "codex exec started"

  do_timeout "$ACP_TIMEOUT" \
    codex exec \
      --model "$ACP_MODEL" \
      --sandbox workspace-write \
      --full-auto \
      -c "model_reasoning_effort=$ACP_REASONING" \
      --cd "$worktree" \
      -o "$last_message_file" \
      - \
      < "$prompt_snapshot" >> "$log_file" 2>&1 || exit_code=$?

  local end_ts
  end_ts=$(date +%s)
  local elapsed=$((end_ts - start_ts))

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
    log_err "$batch" "Timed out after $(fmt_duration "$ACP_TIMEOUT")"
    return 124
  fi

  record_status "$run_id" "$batch" "$attempt" "spawn_failed" "codex exec exited with $exit_code"
  log_err "$batch" "Codex exited with code $exit_code"
  return "$exit_code"
}
