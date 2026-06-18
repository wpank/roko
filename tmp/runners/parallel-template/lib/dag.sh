#!/usr/bin/env bash
# dag.sh — dependency-aware parallel scheduler
#
# The DAG scheduler finds batches whose dependencies are all satisfied,
# dispatches up to N in parallel on separate worktrees, merges completed
# batches back to the run branch, and repeats until done.
#
# Key design:
#   - Each concurrent batch gets its own sub-worktree (forked from run branch HEAD)
#   - After codex completes + AP checks pass, the sub-worktree is merged back
#   - Merges are serialized via lock to prevent race conditions
#   - If a merge conflicts, the batch is PARKED (branch preserved, you cherry-pick later)
#   - Cumulative context is snapshotted after each merge
#   - Cargo checks are deferred to wave gates (not per-batch)
#   - Disk space is checked before each dispatch wave
#   - Branches are NEVER deleted — always available for manual merge/inspection

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
source "$(dirname "${BASH_SOURCE[0]}")/spawn.sh"

# ── Worktree helpers ──

# Sub-worktree for a single batch execution (forked from main run branch)
batch_worktree_path() {
  local run_id="$1" batch="$2"
  echo "$WORKTREE_ROOT/${RUNNER_NAME}-${run_id}-${batch}"
}

# Branch name for a batch (deterministic, never deleted)
batch_branch_name() {
  local run_id="$1" batch="$2"
  echo "codex/${RUNNER_NAME}-${run_id}-${batch}"
}

create_batch_worktree() {
  local run_id="$1" batch="$2" main_branch="$3"
  local wt_path branch_name
  wt_path="$(batch_worktree_path "$run_id" "$batch")"
  branch_name="$(batch_branch_name "$run_id" "$batch")"

  if [[ -d "$wt_path" ]]; then
    # Reuse existing (retry scenario) — reset to current main branch state
    # But first: if there are uncommitted changes, commit them to a backup branch
    if git -C "$wt_path" diff --quiet 2>/dev/null && git -C "$wt_path" diff --cached --quiet 2>/dev/null; then
      : # clean — safe to reset
    else
      local backup_branch="${branch_name}-backup-$(date +%s)"
      git -C "$wt_path" add -A 2>/dev/null || true
      git -C "$wt_path" commit -m "backup: uncommitted work before retry" 2>/dev/null || true
      git -C "$ROKO_ROOT" branch "$backup_branch" "$(git -C "$wt_path" rev-parse HEAD)" 2>/dev/null || true
      log_warn "$batch" "Backed up work to $backup_branch"
    fi
    git -C "$wt_path" reset --hard "$main_branch" >/dev/null 2>&1 || true
  else
    # Aggressively clean stale refs before attempting worktree creation
    git -C "$ROKO_ROOT" worktree prune 2>/dev/null || true
    git -C "$ROKO_ROOT" branch -D "$branch_name" 2>/dev/null || true
    git -C "$ROKO_ROOT" worktree add -b "$branch_name" "$wt_path" "$main_branch" >/dev/null 2>&1 || {
      # Second attempt: even more aggressive cleanup
      rm -rf "$wt_path" 2>/dev/null || true
      git -C "$ROKO_ROOT" worktree prune 2>/dev/null || true
      git -C "$ROKO_ROOT" branch -D "$branch_name" 2>/dev/null || true
      git -C "$ROKO_ROOT" worktree add -b "$branch_name" "$wt_path" "$main_branch" >/dev/null 2>&1 || {
        log_err "$batch" "Failed to create worktree after cleanup"
        return 1
      }
    }
  fi
  if [[ ! -d "$wt_path" ]]; then
    log_err "$batch" "Worktree directory missing after creation: $wt_path"
    return 1
  fi
  echo "$wt_path"
}

remove_batch_worktree() {
  local run_id="$1" batch="$2"
  local wt_path
  wt_path="$(batch_worktree_path "$run_id" "$batch")"
  if [[ -d "$wt_path" ]]; then
    git -C "$ROKO_ROOT" worktree remove --force "$wt_path" 2>/dev/null || rm -rf "$wt_path"
  fi
  # NEVER delete branches — always preserve for inspection and manual merge
}

# ── Per-batch target directory ──
# When PER_BATCH_TARGET=1, each batch gets its own cargo target dir.
# This eliminates lock contention when 40+ batches compile simultaneously.
# Trade-off: more disk usage, but zero cargo lock waits.

batch_target_dir() {
  local run_id="$1" batch="$2"
  if (( ${PER_BATCH_TARGET:-0} )); then
    echo "${TMPDIR:-/tmp}/roko-par-${RUNNER_NAME}/${run_id}/${batch}"
  else
    run_target_dir "$run_id"
  fi
}

# ── Anti-pattern checks (fast, no cargo) ──

run_ap_checks() {
  (( ${SKIP_AP_CHECKS:-0} )) && return 0
  local batch="$1" worktree="$2" run_id="$3"
  local scope_files failed=0
  scope_files="$(batch_scope "$batch")"

  for f in $scope_files; do
    local fp="$worktree/$f"
    [[ -f "$fp" ]] || continue

    # AP-1: Stubs that silently pass
    if grep -qn 'Verdict::pass.*stub\|Verdict::pass.*always\|Verdict::pass.*noop\|Verdict::pass.*todo\|Verdict::pass.*placeholder' "$fp" 2>/dev/null; then
      log_err "$batch" "AP-1: silent-pass stub in $f"; failed=1
    fi

    # AP-2: block_on in async
    if grep -qn 'futures::executor::block_on\|futures::executor::LocalPool' "$fp" 2>/dev/null; then
      log_err "$batch" "AP-2: block_on in $f"; failed=1
    fi

    # AP-3: Duplicate traits (vs foundation.rs)
    if [[ "$f" != *"foundation.rs" ]]; then
      local traits
      traits="$(grep -oP 'pub trait \K\w+' "$fp" 2>/dev/null || true)"
      for t in $traits; do
        if grep -q "pub trait $t" "$worktree/crates/roko-core/src/foundation.rs" 2>/dev/null; then
          log_err "$batch" "AP-3: dup trait $t in $f"; failed=1
        fi
      done
    fi

    # AP-5: Shell out to CLI
    if grep -qn 'Command::new.*"claude"\|Command::new.*"codex"' "$fp" 2>/dev/null; then
      log_err "$batch" "AP-5: shell out to CLI in $f"; failed=1
    fi

    # AP-6: Inline prompt strings
    if grep -qn 'format!.*"You are a\|format!.*"You are an' "$fp" 2>/dev/null; then
      log_err "$batch" "AP-6: inline prompt in $f"; failed=1
    fi

    # AP-7: std::sync::Mutex + .await
    if grep -q 'std::sync::Mutex' "$fp" 2>/dev/null; then
      if awk '/fn /{fl=0;fa=0} /\.lock\(\)/{fl=1} /\.await/{fa=1} fl&&fa{exit 1}' "$fp" 2>/dev/null; then
        : # ok
      else
        log_err "$batch" "AP-7: std Mutex + await in $f"; failed=1
      fi
    fi

    # AP-8: Empty/stub function bodies (fn foo() {} or fn foo() { todo!() })
    if grep -Pqn 'fn \w+\([^)]*\)[^{]*\{\s*\}' "$fp" 2>/dev/null; then
      log_err "$batch" "AP-8: empty function body in $f"; failed=1
    fi

    # AP-9: Unreachable/unimplemented macros left behind
    if grep -qn 'unimplemented!()\|unreachable!("TODO' "$fp" 2>/dev/null; then
      log_err "$batch" "AP-9: unimplemented!/unreachable! in $f"; failed=1
    fi

    # AP-10: Hardcoded localhost/port in non-test code
    if [[ "$f" != *"test"* ]] && grep -qn '"127\.0\.0\.1\|"localhost:' "$fp" 2>/dev/null; then
      log_err "$batch" "AP-10: hardcoded localhost in $f"; failed=1
    fi
  done

  return "$failed"
}

# ── Portable lock with stale detection (mkdir-based, works on macOS and Linux) ──

acquire_lock() {
  local lock_dir="$1" timeout="${2:-120}"
  local waited=0
  while ! mkdir "$lock_dir" 2>/dev/null; do
    # Stale lock detection: if the PID that holds the lock is dead, break it
    if [[ -f "$lock_dir/pid" ]]; then
      local holder_pid
      holder_pid="$(cat "$lock_dir/pid" 2>/dev/null || echo 0)"
      if (( holder_pid > 0 )) && ! kill -0 "$holder_pid" 2>/dev/null; then
        log_warn "lock" "Breaking stale lock (PID $holder_pid dead)"
        rm -rf "$lock_dir"
        continue
      fi
    fi
    if (( waited >= timeout )); then
      log_err "lock" "Lock timeout after ${timeout}s (holder: $(cat "$lock_dir/pid" 2>/dev/null || echo unknown))"
      return 1
    fi
    sleep 1
    waited=$((waited + 1))
  done
  # Store PID for stale-lock detection
  echo $$ > "$lock_dir/pid"
  return 0
}

release_lock() {
  local lock_dir="$1"
  rm -rf "$lock_dir"
}

# ── Merge batch back to main run branch (serialized via lock) ──
#
# On conflict: aborts merge, marks batch as "parked" (branch preserved).
# You can manually merge parked batches later via --pick.

merge_batch_to_main() {
  local batch="$1" run_id="$2" main_worktree="$3"
  local batch_wt
  batch_wt="$(batch_worktree_path "$run_id" "$batch")"
  local batch_branch
  batch_branch="$(batch_branch_name "$run_id" "$batch")"
  local lock_dir
  lock_dir="$(run_merge_lock "$run_id").d"

  # Commit in the batch worktree first (outside lock — no contention)
  git -C "$batch_wt" add -A
  if git -C "$batch_wt" diff --cached --quiet 2>/dev/null; then
    log_warn "$batch" "No changes to merge"
    return 10  # noop
  fi

  local title
  title="$(batch_title "$batch")"
  git -C "$batch_wt" commit -m "$(printf '%s(%s): %s' "$RUNNER_NAME" "$batch" "$title")" >/dev/null 2>&1

  # Record the commit hash so --pick can find it even after worktree cleanup
  local batch_hash
  batch_hash="$(git -C "$batch_wt" rev-parse HEAD)"
  echo "$batch_hash" > "$(run_result_file "$run_id" "$batch").hash"

  # Serialize merges with mkdir lock to prevent concurrent git operations
  acquire_lock "$lock_dir" 180 || {
    log_err "$batch" "Failed to acquire merge lock after 180s"
    return 1
  }

  local merge_rc=0
  {
    # Try fast-forward first, fall back to merge commit
    git -C "$main_worktree" merge --no-edit --no-stat "$batch_branch" >/dev/null 2>&1 || {
      # Merge conflict — abort and park the branch
      log_warn "$batch" "Merge conflict — parking branch $batch_branch for manual merge"
      git -C "$main_worktree" merge --abort 2>/dev/null || true
      merge_rc=1

      # Record conflict details for --pick
      {
        echo "## Merge conflict for $batch"
        echo
        echo "Branch: $batch_branch"
        echo "Hash: $batch_hash"
        echo
        echo "Conflicting with main at: $(git -C "$main_worktree" rev-parse --short HEAD)"
        echo
        echo "To merge manually:"
        echo '```bash'
        echo "git cherry-pick $batch_hash"
        echo "# or"
        echo "git merge $batch_branch"
        echo '```'
      } > "$(run_failure_file "$run_id" "$batch")"
    }

    if (( merge_rc == 0 )); then
      local hash
      hash="$(git -C "$main_worktree" rev-parse --short HEAD)"
      log_ok "$batch" "Merged → $hash"

      # Snapshot cumulative context while holding the lock (reads main worktree)
      snapshot_cumulative_context "$run_id" "$batch" "$main_worktree"
    fi
  }

  release_lock "$lock_dir"
  return "$merge_rc"
}

# ── Pick: cherry-pick a parked/completed batch into a target branch ──

pick_batch() {
  local batch="$1" run_id="$2" target_dir="$3"
  local batch_branch
  batch_branch="$(batch_branch_name "$run_id" "$batch")"

  # Find the commit hash
  local hash_file="$(run_result_file "$run_id" "$batch").hash"
  local batch_hash=""
  if [[ -f "$hash_file" ]]; then
    batch_hash="$(cat "$hash_file")"
  else
    # Try to resolve from branch
    batch_hash="$(git -C "$ROKO_ROOT" rev-parse "$batch_branch" 2>/dev/null)" || {
      log_err "$batch" "Cannot find branch $batch_branch"
      return 1
    }
  fi

  log_info "$batch" "Cherry-picking $batch_hash into $(git -C "$target_dir" rev-parse --abbrev-ref HEAD)"

  local pick_rc=0
  git -C "$target_dir" cherry-pick --no-edit "$batch_hash" 2>/dev/null || pick_rc=$?

  if (( pick_rc != 0 )); then
    log_err "$batch" "Cherry-pick conflict — resolve manually in $target_dir"
    log_err "$batch" "  git -C $target_dir status"
    log_err "$batch" "  # resolve conflicts, then: git -C $target_dir cherry-pick --continue"
    return 1
  fi

  log_ok "$batch" "Picked into $(git -C "$target_dir" rev-parse --short HEAD)"
  return 0
}

# ── List branches for a run ──

list_run_branches() {
  local run_id="$1"
  git -C "$ROKO_ROOT" branch --list "codex/${RUNNER_NAME}-${run_id}-*" 2>/dev/null | sed 's/^[* ]*//'
}

# ── Wave gate (cargo check + clippy, once per group) ──

run_wave_gate() {
  local wave="$1" run_id="$2" worktree="$3"
  local target_dir
  target_dir="$(run_target_dir "$run_id")"
  local gate_log="$LOG_ROOT/$run_id/gate-${wave}.log"

  log_header "WAVE GATE: $wave"

  log_info "gate" "cargo check --workspace"
  local output exit_code=0
  output="$(cd "$worktree" && env CARGO_TARGET_DIR="$target_dir" \
    cargo check --workspace 2>&1)" || exit_code=$?
  echo "$output" > "$gate_log"

  if (( exit_code != 0 )); then
    log_err "gate" "Workspace check failed after wave $wave"
    echo "$output" | grep '^error' | head -5 >&2
    return 1
  fi
  log_ok "gate" "Workspace compiles"

  log_info "gate" "cargo clippy --workspace"
  exit_code=0
  output="$(cd "$worktree" && env CARGO_TARGET_DIR="$target_dir" \
    cargo clippy --workspace --no-deps -- -D warnings 2>&1)" || exit_code=$?
  echo "$output" >> "$gate_log"

  if (( exit_code != 0 )); then
    log_err "gate" "Clippy failed after wave $wave"
    echo "$output" | grep '^error' | head -5 >&2
    return 1
  fi
  log_ok "gate" "Clippy clean"

  record_status "$run_id" "gate:$wave" "1" "gate_ok" "check+clippy"
  return 0
}

# ── End-of-run test gate ──

run_test_gate() {
  local run_id="$1" worktree="$2"
  local target_dir
  target_dir="$(run_target_dir "$run_id")"

  log_header "TEST GATE"
  log_info "test" "cargo test --workspace"

  local output exit_code=0
  output="$(cd "$worktree" && env CARGO_TARGET_DIR="$target_dir" \
    cargo test --workspace 2>&1)" || exit_code=$?
  echo "$output" > "$LOG_ROOT/$run_id/gate-test.log"

  if (( exit_code != 0 )); then
    log_err "test" "Tests failed"
    echo "$output" | grep -E '^test .* FAILED|^failures:' | head -20 >&2
    return 1
  fi

  local count
  count="$(echo "$output" | grep -oP '\d+ passed' | tail -1)"
  log_ok "test" "All tests passed ($count)"
  return 0
}

# ── DAG scheduler ──

# Returns batch IDs that are ready to run (all deps satisfied, not yet done)
dag_ready_batches() {
  local run_id="$1"
  shift
  local -a candidates=("$@")

  for batch in "${candidates[@]}"; do
    # Skip already completed/failed
    local result_file
    result_file="$(run_result_file "$run_id" "$batch")"
    if [[ -f "$result_file" ]]; then
      local status; status="$(cat "$result_file")"
      success_status "$status" && continue
      terminal_failure "$status" && continue
      [[ "$status" == "in_progress" ]] && continue
    fi

    # Check deps
    local -a deps=()
    IFS=' ' read -r -a deps <<< "$(batch_deps "$batch")"
    local all_met=1
    for dep in "${deps[@]}"; do
      [[ -z "$dep" ]] && continue
      local dep_file
      dep_file="$(run_result_file "$run_id" "$dep")"
      if [[ ! -f "$dep_file" ]] || ! success_status "$(cat "$dep_file")"; then
        all_met=0; break
      fi
    done

    (( all_met )) && echo "$batch"
  done
}

# Check if any dep has terminally failed (so this batch is blocked forever)
dag_blocked_batches() {
  local run_id="$1"
  shift
  local -a candidates=("$@")

  for batch in "${candidates[@]}"; do
    local result_file
    result_file="$(run_result_file "$run_id" "$batch")"
    [[ -f "$result_file" ]] && continue  # already has a result

    local -a deps=()
    IFS=' ' read -r -a deps <<< "$(batch_deps "$batch")"
    for dep in "${deps[@]}"; do
      [[ -z "$dep" ]] && continue
      local dep_file
      dep_file="$(run_result_file "$run_id" "$dep")"
      if [[ -f "$dep_file" ]] && terminal_failure "$(cat "$dep_file")"; then
        echo "$batch"; break
      fi
    done
  done
}

# Count batches with a given status category
dag_count() {
  local run_id="$1" category="$2"
  shift 2
  local -a batches=("$@")
  local count=0
  for batch in "${batches[@]}"; do
    local rf; rf="$(run_result_file "$run_id" "$batch")"
    [[ -f "$rf" ]] || continue
    local s; s="$(cat "$rf")"
    case "$category" in
      success) success_status "$s" && count=$((count+1)) ;;
      failed)  terminal_failure "$s" && count=$((count+1)) ;;
    esac
  done
  echo "$count"
}

# Check if all batches in a group are completed (for gate triggering)
dag_group_complete() {
  local run_id="$1" group="$2"
  shift 2
  local -a batches=("$@")
  for batch in "${batches[@]}"; do
    local grp
    grp="$(batch_group "$batch")"
    [[ "$grp" == "$group" ]] || continue
    local rf
    rf="$(run_result_file "$run_id" "$batch")"
    if [[ ! -f "$rf" ]]; then
      return 1  # pending
    fi
    local s
    s="$(cat "$rf")"
    if [[ "$s" == "in_progress" ]]; then
      return 1  # still running
    fi
    # success or failed — either way, this batch is done
  done
  return 0  # all batches in this group have a terminal result
}
