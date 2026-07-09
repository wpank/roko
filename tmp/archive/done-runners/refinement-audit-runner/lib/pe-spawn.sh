#!/usr/bin/env bash

source "$(dirname "${BASH_SOURCE[0]}")/pe-common.sh"

: "${PE_MODEL:=gpt-5.4}"
: "${PE_REASONING:=high}"
: "${PE_TIMEOUT:=5400}"

# ---------------------------------------------------------------------------
# Shared context emitter — injected into every prompt
# ---------------------------------------------------------------------------

emit_shared_context_pack() {
  cat <<'EOF'
## Shared Context Pack

EOF

  local file title
  for file in \
    "$PE_CONTEXT_DIR/00-AUDIT-RULES.md" \
    "$PE_CONTEXT_DIR/01-PRIORITY-QUEUE.md" \
    "$PE_CONTEXT_DIR/02-DOCS-TREE-MAP.md" \
    "$PE_CONTEXT_DIR/03-WORKSPACE-TOPOLOGY.md" \
    "$PE_CONTEXT_DIR/04-DELEGATION-GUIDANCE.md"; do
    [[ -f "$file" ]] || continue
    title="$(basename "$file" .md)"
    printf '### %s\n\n' "$title"
    cat "$file"
    printf '\n'
  done
}

# ---------------------------------------------------------------------------
# Delegation guidance — same pattern as ux-followup-runner
# ---------------------------------------------------------------------------

emit_delegation_guidance() {
  local batch="$1"
  cat <<'EOF'
## Delegation Requirement

You are explicitly authorized to use multiple subagents for this batch.
Use them aggressively where it helps, but keep the immediate blocking work local.

Required delegation behavior:

- Before coding, form a short plan and identify 2-4 concrete sidecar subtasks.
- Spawn explorers for targeted codebase questions and workers for bounded code edits.
- Each subagent gets the same context pack plus its specific task.
- Give each worker a disjoint write scope and tell them they are not alone in the codebase.
- Do not wait idly for subagents if you can make progress locally.
- If subagents are unavailable in this environment, continue locally without failing.
EOF

  printf 'Suggested parallel split for batch `%s`:\n' "$batch"

  local section="${batch#PE}"
  case "$section" in
    00)
      cat <<'EOF'
- explorer: read roko-core types, Signal struct, 6 verb traits, config module
- worker: apply doc-parity updates to roko-core module-level docs + type signatures
- worker: update roko-core tests for any signature changes
EOF
      ;;
    01)
      cat <<'EOF'
- explorer: read orchestrate.rs plan runner + roko-orchestrator plan DAG
- worker: apply doc-parity updates to orchestrator/CLI orchestration modules
- worker: update orchestrator integration points and tests
EOF
      ;;
    02)
      cat <<'EOF'
- explorer: read roko-agent dispatcher, backends, pool, MCP modules
- worker: apply doc-parity updates to agent dispatch + backend modules
- worker: update agent safety integration + tests
EOF
      ;;
    03)
      cat <<'EOF'
- explorer: read roko-compose system_prompt_builder + templates + enrichment
- worker: apply doc-parity updates to composition modules
- worker: update template tests for any changes
EOF
      ;;
    04)
      cat <<'EOF'
- explorer: read roko-gate gate implementations + rung pipeline + adaptive thresholds
- worker: apply doc-parity updates to gate module docs + signatures
- worker: update gate pipeline tests
EOF
      ;;
    05)
      cat <<'EOF'
- explorer: read roko-learn episodes, playbooks, bandits, experiments, efficiency
- worker: apply doc-parity updates to learning modules
- worker: update learning integration tests
EOF
      ;;
    06)
      cat <<'EOF'
- explorer: read roko-neuro durable knowledge store + distillation + tier progression
- worker: apply doc-parity updates to neuro modules
- worker: update neuro tests
EOF
      ;;
    07)
      cat <<'EOF'
- explorer: read roko-conductor watchers + circuit breaker + diagnosis
- worker: apply doc-parity updates to conductor modules
- worker: update conductor tests
EOF
      ;;
    08)
      cat <<'EOF'
- explorer: read roko-chain witness primitives + chain types
- worker: apply doc-parity updates to chain modules
- worker: update chain tests
EOF
      ;;
    09)
      cat <<'EOF'
- explorer: read roko-daimon behavior primitives
- worker: apply doc-parity updates to daimon modules
- worker: update daimon tests
EOF
      ;;
    10)
      cat <<'EOF'
- explorer: read roko-dreams offline consolidation modules
- worker: apply doc-parity updates to dreams modules
- worker: update dreams tests
EOF
      ;;
    11)
      cat <<'EOF'
- explorer: read roko-agent safety module: contracts, role auth, pre/post checks
- worker: apply doc-parity updates to safety modules
- worker: update safety integration tests
EOF
      ;;
    12)
      cat <<'EOF'
- explorer: read roko-cli TUI + roko-serve HTTP routes + roko-agent-server sidecar
- worker: apply doc-parity updates to CLI/serve/sidecar modules
- worker: update interface integration tests
EOF
      ;;
    *)
      cat <<'EOF'
- explorer: targeted read-only architecture questions per the batch prompt
- worker: first bounded implementation slice
- worker: second bounded implementation slice
EOF
      ;;
  esac

  echo
}

# ---------------------------------------------------------------------------
# Prompt composition
# ---------------------------------------------------------------------------

compose_prompt_snapshot() {
  local batch="$1" run_id="$2" attempt="$3" failure_file="$4"
  local out
  out="$(run_prompt_snapshot "$run_id" "$batch")"
  ensure_dir "$(dirname "$out")"

  {
    echo "# Parity Execution — Batch $batch"
    echo
    echo "Run id: $run_id"
    echo "Attempt: $attempt"
    echo "Model: $PE_MODEL"
    echo "Reasoning: $PE_REASONING"
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

# ---------------------------------------------------------------------------
# Codex invocation — uses `codex exec --sandbox workspace-write`
# Identical to ux-followup-runner pattern
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
    echo "=== Model: $PE_MODEL ==="
    echo "=== Reasoning: $PE_REASONING ==="
    echo "=== Timeout: $PE_TIMEOUT ==="
    echo "=== Cargo target: $target_dir ==="
    echo "=== Prompt snapshot: $prompt_snapshot ==="
    echo
  } > "$log_file"

  record_status "$run_id" "$batch" "$attempt" "spawn_started" "codex exec started"

  do_timeout "$PE_TIMEOUT" \
    env CARGO_TARGET_DIR="$target_dir" \
    codex exec \
      --model "$PE_MODEL" \
      --sandbox workspace-write \
      --full-auto \
      -c "model_reasoning_effort=$PE_REASONING" \
      --cd "$worktree" \
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
    log_err "$batch" "Timed out after $(fmt_duration "$PE_TIMEOUT")"
    return 124
  fi

  record_status "$run_id" "$batch" "$attempt" "spawn_failed" "codex exec exited with $exit_code"
  log_err "$batch" "Codex exited with code $exit_code"
  return "$exit_code"
}
