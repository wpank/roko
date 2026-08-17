#!/usr/bin/env bash

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

write_failure_summary() {
  local batch="$1"
  local run_id="$2"
  local note="$3"
  local log_file
  log_file=$(run_log_file "$run_id" "$batch")
  local failure_file
  failure_file=$(run_failure_file "$run_id" "$batch")

  {
    echo "$note"
    echo
    echo "Recent log tail:"
    tail -40 "$log_file" 2>/dev/null || true
  } > "$failure_file"
}

backup_worktree_state() {
  local run_id="$1"
  local batch="$2"
  local attempt="$3"
  local worktree="$4"
  local label="$5"
  local backup_dir prefix
  backup_dir="$(run_backups_dir "$run_id")"
  prefix="$backup_dir/${batch}-attempt-${attempt}-${label}"
  ensure_dir "$backup_dir"

  git -C "$worktree" status --short > "${prefix}.status"
  git -C "$worktree" diff > "${prefix}.patch"
  {
    echo "run_id=$run_id"
    echo "batch=$batch"
    echo "attempt=$attempt"
    echo "label=$label"
    echo "captured_at=$(date -Iseconds)"
    echo "worktree=$worktree"
  } > "${prefix}.meta"
}

reset_runner_worktree() {
  local worktree="$1"
  git -C "$worktree" reset --hard HEAD >/dev/null 2>&1 || true
  git -C "$worktree" clean -fd >/dev/null 2>&1 || true
}

# List crates/roko-acp/ files touched (vs committed HEAD).
changed_acp_paths() {
  local worktree="$1"
  git -C "$worktree" status --porcelain=v1 -- crates/roko-acp/ \
    | awk '{print $2}'
}

# Scope gate — only allowed paths may change.
scope_gate() {
  local batch="$1"
  local run_id="$2"
  local worktree="$3"
  local log_file
  log_file=$(run_log_file "$run_id" "$batch")

  local allowed
  allowed="$(batch_allowed_paths "$batch")"

  # Build grep -vE pattern from allowed paths.
  # NOTE: IFS is \n\t in run-acp.sh, so we must split on space explicitly.
  local exclude_pattern=""
  local path
  local IFS=' '
  for path in $allowed; do
    [[ -n "$exclude_pattern" ]] && exclude_pattern="${exclude_pattern}|"
    # Escape dots for regex
    exclude_pattern="${exclude_pattern}^${path//./\\.}"
  done

  local outside
  outside="$(git -C "$worktree" status --porcelain=v1 \
    | awk '{print $2}' \
    | grep -vE "$exclude_pattern" \
    | grep -vE '^$' || true)"

  if [[ -n "$outside" ]]; then
    echo "[verify] scope_gate: files modified outside allowed paths" >> "$log_file"
    while IFS= read -r line; do
      echo "[verify]   $line" >> "$log_file"
    done <<< "$outside"
    log_err "$batch" "scope violation: files changed outside allowed paths"
    return 1
  fi
  return 0
}

# Diff gate — ensure the batch actually produced changes.
diff_gate() {
  local batch="$1"
  local run_id="$2"
  local worktree="$3"
  local log_file
  log_file=$(run_log_file "$run_id" "$batch")

  local changed
  changed="$(git -C "$worktree" status --porcelain=v1 -- crates/roko-acp/ | wc -l | tr -d ' ')"

  # ACP07 also touches roko-cli
  if [[ "$batch" == "ACP07" ]]; then
    local cli_changed
    cli_changed="$(git -C "$worktree" status --porcelain=v1 -- crates/roko-cli/ | wc -l | tr -d ' ')"
    changed=$((changed + cli_changed))
  fi

  # ACP01 also touches root Cargo.toml
  if [[ "$batch" == "ACP01" ]]; then
    local root_changed
    root_changed="$(git -C "$worktree" status --porcelain=v1 -- Cargo.toml | wc -l | tr -d ' ')"
    changed=$((changed + root_changed))
  fi

  if [[ "$changed" == "0" ]]; then
    echo "[verify] diff_gate: no changes; batch produced no effect" >> "$log_file"
    return 1
  fi
  echo "[verify] diff_gate: $changed changed path(s)" >> "$log_file"
  return 0
}

# Required-terms gate — new vocabulary present in changed files.
required_terms_check() {
  local batch="$1"
  local run_id="$2"
  local worktree="$3"
  local log_file
  log_file=$(run_log_file "$run_id" "$batch")
  local pattern
  pattern="$(batch_required_terms "$batch" || true)"
  [[ -z "$pattern" ]] && return 0

  local -a changed=()
  while IFS= read -r path; do
    [[ -n "$path" ]] || continue
    [[ -f "$worktree/$path" ]] || continue
    changed+=("$path")
  done < <(changed_acp_paths "$worktree")

  if [[ ${#changed[@]} -eq 0 ]]; then
    return 0
  fi

  local found=0
  local path
  for path in "${changed[@]}"; do
    if grep -q -i -E "$pattern" "$worktree/$path"; then
      found=1
      break
    fi
  done

  if (( found == 0 )); then
    echo "[verify] required-term check: none of the changed files mention pattern: $pattern" >> "$log_file"
    log_err "$batch" "required-term check failed (pattern: $pattern)"
    return 1
  fi
  echo "[verify] required-term check: OK (pattern: $pattern)" >> "$log_file"
  return 0
}

# Cargo check gate — ensure the code compiles.
cargo_check_gate() {
  local batch="$1"
  local run_id="$2"
  local worktree="$3"
  local log_file
  log_file=$(run_log_file "$run_id" "$batch")

  local packages
  packages="$(batch_check_packages "$batch")"

  # Use a temporary target dir to avoid disk fill
  local tmp_target
  tmp_target="$(mktemp -d "${TMPDIR:-/tmp}/roko-acp-check-XXXXXX")"

  # Build -p flags. Must use IFS=' ' because run-acp.sh sets IFS=$'\n\t'.
  local pkg_args=""
  local pkg
  local IFS=' '
  for pkg in $packages; do
    pkg_args="$pkg_args -p $pkg"
  done

  echo "[verify] cargo check: CARGO_TARGET_DIR=$tmp_target cargo check${pkg_args}" >> "$log_file"

  local rc=0
  CARGO_TARGET_DIR="$tmp_target" eval "cargo check${pkg_args}" >> "$log_file" 2>&1 || rc=$?

  # Cleanup temp target dir
  rm -rf "$tmp_target" 2>/dev/null || true

  if (( rc != 0 )); then
    echo "[verify] cargo check: FAILED (exit $rc)" >> "$log_file"
    log_err "$batch" "cargo check failed"
    return 1
  fi
  echo "[verify] cargo check: OK" >> "$log_file"
  return 0
}

# Clippy gate — no warnings.
clippy_gate() {
  local batch="$1"
  local run_id="$2"
  local worktree="$3"
  local log_file
  log_file=$(run_log_file "$run_id" "$batch")

  local packages
  packages="$(batch_check_packages "$batch")"

  local tmp_target
  tmp_target="$(mktemp -d "${TMPDIR:-/tmp}/roko-acp-clippy-XXXXXX")"

  local pkg_args=""
  local pkg
  local IFS=' '
  for pkg in $packages; do
    pkg_args="$pkg_args -p $pkg"
  done

  echo "[verify] clippy: CARGO_TARGET_DIR=$tmp_target cargo clippy${pkg_args} --no-deps -- -D warnings" >> "$log_file"

  local rc=0
  CARGO_TARGET_DIR="$tmp_target" eval "cargo clippy${pkg_args} --no-deps -- -D warnings" >> "$log_file" 2>&1 || rc=$?

  rm -rf "$tmp_target" 2>/dev/null || true

  if (( rc != 0 )); then
    echo "[verify] clippy: FAILED (exit $rc)" >> "$log_file"
    log_err "$batch" "clippy failed"
    return 1
  fi
  echo "[verify] clippy: OK" >> "$log_file"
  return 0
}

# Test gate — run batch-specific tests.
test_gate() {
  local batch="$1"
  local run_id="$2"
  local worktree="$3"
  local log_file
  log_file=$(run_log_file "$run_id" "$batch")

  local test_cmd
  test_cmd="$(batch_test_command "$batch")"
  [[ -z "$test_cmd" ]] && return 0

  local tmp_target
  tmp_target="$(mktemp -d "${TMPDIR:-/tmp}/roko-acp-test-XXXXXX")"

  echo "[verify] test: CARGO_TARGET_DIR=$tmp_target $test_cmd" >> "$log_file"

  local rc=0
  CARGO_TARGET_DIR="$tmp_target" eval "$test_cmd" >> "$log_file" 2>&1 || rc=$?

  rm -rf "$tmp_target" 2>/dev/null || true

  if (( rc != 0 )); then
    echo "[verify] test: FAILED (exit $rc)" >> "$log_file"
    log_err "$batch" "tests failed"
    return 1
  fi
  echo "[verify] test: OK" >> "$log_file"
  return 0
}

verify_batch() {
  local batch="$1"
  local run_id="$2"
  local worktree="$3"
  local attempt="${4:-?}"

  record_status "$run_id" "$batch" "$attempt" "verify_running" "scope+diff+terms+check+clippy+test"

  # 1. Scope gate
  if ! scope_gate "$batch" "$run_id" "$worktree"; then
    write_failure_summary "$batch" "$run_id" "Scope gate failed: agent modified files outside allowed paths."
    record_status "$run_id" "$batch" "$attempt" "verify_failed" "scope_gate"
    return 1
  fi

  # 2. Diff gate
  if ! diff_gate "$batch" "$run_id" "$worktree"; then
    write_failure_summary "$batch" "$run_id" "Diff gate failed: batch produced no changes."
    record_status "$run_id" "$batch" "$attempt" "verify_failed" "diff_gate"
    return 1
  fi

  # 3. Required-terms gate
  if ! required_terms_check "$batch" "$run_id" "$worktree"; then
    write_failure_summary "$batch" "$run_id" "Required-terms gate failed: expected vocabulary absent."
    record_status "$run_id" "$batch" "$attempt" "verify_failed" "required_terms"
    return 1
  fi

  # 4. Cargo check — must compile
  if ! cargo_check_gate "$batch" "$run_id" "$worktree"; then
    write_failure_summary "$batch" "$run_id" "Cargo check failed: code does not compile."
    record_status "$run_id" "$batch" "$attempt" "verify_failed" "cargo_check"
    return 1
  fi

  # 5. Clippy — no warnings
  if ! clippy_gate "$batch" "$run_id" "$worktree"; then
    write_failure_summary "$batch" "$run_id" "Clippy failed: warnings present."
    record_status "$run_id" "$batch" "$attempt" "verify_failed" "clippy"
    return 1
  fi

  # 6. Tests — batch-specific
  if ! test_gate "$batch" "$run_id" "$worktree"; then
    write_failure_summary "$batch" "$run_id" "Tests failed."
    record_status "$run_id" "$batch" "$attempt" "verify_failed" "test"
    return 1
  fi

  record_status "$run_id" "$batch" "$attempt" "verify_succeeded" "all gates passed"
  log_ok "$batch" "Verification passed"
  return 0
}

commit_batch_if_needed() {
  local batch="$1"
  local worktree="$2"
  local run_id="${3:-}"
  local attempt="${4:-?}"
  local title
  title=$(batch_title "$batch")

  # Stage changes based on batch scope
  local paths
  paths="$(batch_allowed_paths "$batch")"
  local path
  local IFS=' '
  for path in $paths; do
    git -C "$worktree" add -A -- "$path"
  done

  if git -C "$worktree" diff --cached --quiet; then
    if [[ -n "$run_id" ]]; then
      record_status "$run_id" "$batch" "$attempt" "commit_noop" "no staged changes after verify"
    fi
    log_warn "$batch" "No changes staged after successful verification"
    return 10
  fi

  git -C "$worktree" commit -m "$(cat <<EOF
acp(${batch}): ${title}

Automated ACP implementation via tmp/acp-runner/run-acp.sh
EOF
)" >/dev/null
  if [[ -n "$run_id" ]]; then
    record_status "$run_id" "$batch" "$attempt" "commit_succeeded" "$(git -C "$worktree" rev-parse --short HEAD)"
  fi
  log_ok "$batch" "Committed: $(git -C "$worktree" log --oneline -1)"
  return 0
}
