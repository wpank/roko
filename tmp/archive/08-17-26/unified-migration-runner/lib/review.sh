#!/usr/bin/env bash

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

: "${MR_REVIEW_MODEL:=gpt-5.4}"
: "${MR_REVIEW_REASONING:=high}"
: "${MR_REVIEW_INTERVAL:=3}"     # Review every N batches
: "${MR_REVIEW_TIMEOUT:=3600}"   # 60 min per review pass

should_review() {
  local batch_count="$1"
  (( batch_count > 0 && batch_count % MR_REVIEW_INTERVAL == 0 ))
}

compose_review_prompt() {
  local run_id="$1"
  local worktree="$2"
  shift 2
  local batches=("$@")  # Array of batch IDs covered since last review

  local out="$LOG_ROOT/$run_id/review-after-${batches[-1]}.prompt.md"

  {
    echo "# Codex Review Pass"
    echo
    echo "Run id: $run_id"
    echo "Model: $MR_REVIEW_MODEL"
    echo "Reasoning: $MR_REVIEW_REASONING"
    echo "Batches since last review: ${batches[*]}"
    echo
    echo "## Task"
    echo
    echo "You are a senior Rust reviewer. Review the changes made by the last"
    echo "${#batches[@]} migration batches. Focus on:"
    echo
    echo "1. **Correctness**: Do the renames and rewirings preserve semantics?"
    echo "2. **Consistency**: Are all old names replaced? Any partial renames?"
    echo "3. **Dead code**: Was anything left behind that should have been removed?"
    echo "4. **Test coverage**: Do changed code paths have tests?"
    echo "5. **Clippy compliance**: Will \`cargo clippy --workspace --no-deps -- -D warnings\` pass?"
    echo
    echo "For each issue found, fix it directly. Do not leave TODO comments."
    echo
    echo "## Changes to review"
    echo
    echo "Run \`git log --oneline -${#batches[@]}\` to see the commits."
    echo "Run \`git diff HEAD~${#batches[@]}..HEAD --stat\` to see the file scope."
    echo
    echo "## Context"
    echo
    emit_shared_context_pack
    echo
    echo "## Constraints"
    echo
    echo "- Do NOT revert batch changes. Only fix issues within them."
    echo "- If you find a bug that requires reverting a batch, note it in a file"
    echo "  at \`tmp/unified-migration-runner/logs/$run_id/review-issues.md\`"
    echo "  and continue with other fixes."
  } > "$out"

  echo "$out"
}

run_review_pass() {
  local run_id="$1"
  local worktree="$2"
  shift 2
  local batches=("$@")
  local batch_label="${batches[-1]}"

  local prompt_file
  prompt_file=$(compose_review_prompt "$run_id" "$worktree" "${batches[@]}")
  local log_file="$LOG_ROOT/$run_id/review-after-${batch_label}.log"
  local target_dir
  target_dir="$(tmp_target_root)/$run_id/review-${batch_label}"
  rm -rf "$target_dir"
  mkdir -p "$target_dir"

  log_header "REVIEW PASS (after $batch_label)"
  log_info "review" "Reviewing ${#batches[@]} batches: ${batches[*]}"
  record_status "$run_id" "$batch_label" "review" "review_started" "codex review pass"

  local start_ts exit_code=0
  start_ts=$(date +%s)

  if command -v codex >/dev/null 2>&1; then
    do_timeout "$MR_REVIEW_TIMEOUT" \
      env CARGO_TARGET_DIR="$target_dir" \
      codex exec \
        --model "$MR_REVIEW_MODEL" \
        --sandbox workspace-write \
        --full-auto \
        -c "model_reasoning_effort=$MR_REVIEW_REASONING" \
        --cd "$worktree" \
        - \
        < "$prompt_file" >> "$log_file" 2>&1 || exit_code=$?
  else
    log_warn "review" "codex CLI not found; skipping review pass"
    record_status "$run_id" "$batch_label" "review" "review_skipped" "codex not available"
    return 0
  fi

  local end_ts elapsed
  end_ts=$(date +%s)
  elapsed=$((end_ts - start_ts))

  if [[ "$exit_code" -eq 0 ]]; then
    record_status "$run_id" "$batch_label" "review" "review_succeeded" "$(fmt_duration "$elapsed")"
    log_ok "review" "Review completed in $(fmt_duration "$elapsed")"

    # Commit review fixes if any
    rm -rf "$worktree/.cargo-target" "$worktree/target"
    git -C "$worktree" add -A
    if ! git -C "$worktree" diff --cached --quiet; then
      git -C "$worktree" commit -m "$(cat <<EOF
review(${batch_label}): codex review pass

Automated review of batches ${batches[*]}
EOF
)" >/dev/null
      log_ok "review" "Review fixes committed"
    else
      log_info "review" "No fixes needed"
    fi
  else
    record_status "$run_id" "$batch_label" "review" "review_failed" "exit code $exit_code"
    log_warn "review" "Review pass failed (exit $exit_code); continuing"
  fi

  # Clean up review targets
  rm -rf "$target_dir"
  return 0
}
