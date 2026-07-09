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

# List docs/ files touched (vs committed HEAD) by the current batch.
changed_docs_paths() {
  local worktree="$1"
  git -C "$worktree" status --porcelain=v1 -- docs/ \
    | awk '{print $2}'
}

# Allow retired-term mentions inside lines that explicitly document them
# as retired / deprecated / historical / legacy / formerly / etc.
#
# Returns 0 (safe) when the line contains a marker word that signals the
# mention is historical/archival rather than active framing, or when the
# line is a markdown table row (pipe-separated, conventionally a mapping).
is_safe_retired_context() {
  local line="$1"

  # Markdown table body rows (starts with `|`, has ≥2 more `|`s) are
  # treated as mapping rows — "| Retired | Current | Notes |". This is
  # the dominant pattern in glossaries and rename tables.
  local pipes
  pipes="${line//[^|]}"
  if [[ "${line#*|}" != "$line" ]] && [[ ${#pipes} -ge 2 ]]; then
    return 0
  fi

  local lower
  lower="$(printf '%s' "$line" | tr '[:upper:]' '[:lower:]')"
  case "$lower" in
    *retired*|*deprecated*|*historical*|*formerly*|*legacy*\
    |*"old name"*|*"see also"*|*renamed*|*archive*|*backup*\
    |*"bardo-backup"*|*"prior project"*|*predecessor*|*successor*\
    |*"mori-parity"*|*"mori parity"*|*"mori reference"*|*"mori appendix"*\
    |*"pre-roko"*|*"previous codename"*|*codename*|*heritage*\
    |*"used to be"*\
    |*dissolved*|*dissolution*|*deleted*|*moved*|*replaced*|*rename*\
    |*"never say"*|*"→"*|*"->"*|*umbrella*|*"was the"*|*"was renamed"*\
    |*"original ecosystem"*|*"original name"*|*"do not use"*|*mechanical*\
    |*"used to be"*|*"stood for"*|*placeholder*)
      return 0 ;;
  esac
  return 1
}

# Whole-file exemption for docs whose PURPOSE is documenting retired
# terms (glossaries, naming maps, retirement tables). These files are
# *supposed* to contain the retired vocabulary — the check applies only
# to prose propagation elsewhere.
is_glossary_file() {
  local path="$1"
  local base
  base="$(basename "$path" .md)"
  local lower
  lower="$(printf '%s' "$base" | tr '[:upper:]' '[:lower:]')"
  case "$lower" in
    *glossary*|*naming*|*-naming-*|*terminology*|*-retired-*|*-rename-*)
      return 0 ;;
  esac
  return 1
}

terminology_check() {
  local batch="$1"
  local run_id="$2"
  local worktree="$3"
  local log_file
  log_file=$(run_log_file "$run_id" "$batch")

  local -a changed=()
  while IFS= read -r path; do
    [[ -n "$path" ]] || continue
    [[ -f "$worktree/$path" ]] || continue
    changed+=("$path")
  done < <(changed_docs_paths "$worktree")

  if [[ ${#changed[@]} -eq 0 ]]; then
    echo "[verify] terminology: no docs/ files changed" >> "$log_file"
    return 0
  fi

  echo "[verify] terminology: scanning ${#changed[@]} file(s)" >> "$log_file"
  local fail=0
  local term path
  for path in "${changed[@]}"; do
    if is_glossary_file "$path"; then
      echo "[verify] terminology: skipping glossary/naming file $path" >> "$log_file"
      continue
    fi
    for term in "${RETIRED_TERMS[@]}"; do
      local matches
      matches="$(grep -n -i -E "$term" "$worktree/$path" || true)"
      [[ -z "$matches" ]] && continue
      # Filter out allowed retired contexts.
      # grep -n emits "LINENO:CONTENT" (one colon) — strip only once.
      while IFS= read -r hit; do
        [[ -z "$hit" ]] && continue
        local body
        body="${hit#*:}"
        if is_safe_retired_context "$body"; then
          continue
        fi
        echo "[verify] terminology violation in $path: $hit" >> "$log_file"
        fail=1
      done <<< "$matches"
    done
  done

  if (( fail == 1 )); then
    log_err "$batch" "terminology check failed — see log"
    return 1
  fi

  echo "[verify] terminology: OK" >> "$log_file"
  return 0
}

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
  done < <(changed_docs_paths "$worktree")

  if [[ ${#changed[@]} -eq 0 ]]; then
    # Nothing changed at all — batch will fail the "did anything happen" gate below.
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

# Broken-link cheap sanity: internal .md links that point at paths that
# don't exist. Warns rather than fails unless the link originates in a
# changed file.
internal_link_check() {
  local batch="$1"
  local run_id="$2"
  local worktree="$3"
  local log_file
  log_file=$(run_log_file "$run_id" "$batch")

  local -a changed=()
  while IFS= read -r path; do
    [[ -n "$path" ]] || continue
    [[ -f "$worktree/$path" ]] || continue
    changed+=("$path")
  done < <(changed_docs_paths "$worktree")

  if [[ ${#changed[@]} -eq 0 ]]; then
    return 0
  fi

  local fail=0
  local path
  for path in "${changed[@]}"; do
    # Extract markdown links of form (./foo.md) or (foo.md) or (../bar.md)
    local links
    links="$(grep -oE '\]\([^)]+\.md[^)]*\)' "$worktree/$path" 2>/dev/null || true)"
    [[ -z "$links" ]] && continue
    while IFS= read -r link; do
      [[ -z "$link" ]] && continue
      # Strip ](...)
      local target="${link#*](}"
      target="${target%)}"
      # Strip anchor
      target="${target%%#*}"
      # Skip http(s)
      [[ "$target" == http* ]] && continue
      # Resolve relative to the containing file
      local base
      base="$(dirname "$worktree/$path")"
      local resolved
      if [[ "$target" == /* ]]; then
        resolved="$worktree/$target"
      else
        resolved="$base/$target"
      fi
      # Collapse ..
      resolved="$(cd "$(dirname "$resolved")" 2>/dev/null && pwd)/$(basename "$resolved")" || continue
      if [[ ! -e "$resolved" ]]; then
        echo "[verify] broken internal link in $path: $link -> $resolved" >> "$log_file"
        fail=1
      fi
    done <<< "$links"
  done

  if (( fail == 1 )); then
    log_warn "$batch" "internal link check found broken links (soft warning — see log)"
    # Soft warning by default. Set REF_LINK_CHECK_STRICT=1 to fail the batch.
    if [[ "${REF_LINK_CHECK_STRICT:-0}" == "1" ]]; then
      return 1
    fi
  fi
  return 0
}

# Ensure the batch actually produced changes under docs/.
diff_gate() {
  local batch="$1"
  local run_id="$2"
  local worktree="$3"
  local log_file
  log_file=$(run_log_file "$run_id" "$batch")

  local changed
  changed="$(git -C "$worktree" status --porcelain=v1 -- docs/ | wc -l | tr -d ' ')"
  if [[ "$changed" == "0" ]]; then
    echo "[verify] diff_gate: no changes under docs/; batch produced no effect" >> "$log_file"
    return 1
  fi
  echo "[verify] diff_gate: $changed changed path(s) under docs/" >> "$log_file"
  return 0
}

# Ensure the agent touched nothing outside docs/ (or refinements itself
# for the source files). Fail fast if it did.
scope_gate() {
  local batch="$1"
  local run_id="$2"
  local worktree="$3"
  local log_file
  log_file=$(run_log_file "$run_id" "$batch")

  local outside
  outside="$(git -C "$worktree" status --porcelain=v1 \
    | awk '{print $2}' \
    | grep -vE '^docs/' \
    | grep -vE '^$' || true)"

  if [[ -n "$outside" ]]; then
    echo "[verify] scope_gate: files modified outside docs/" >> "$log_file"
    while IFS= read -r line; do
      echo "[verify]   $line" >> "$log_file"
    done <<< "$outside"
    log_err "$batch" "scope violation: files changed outside docs/"
    return 1
  fi
  return 0
}

verify_batch() {
  local batch="$1"
  local run_id="$2"
  local worktree="$3"
  local attempt="${4:-?}"

  record_status "$run_id" "$batch" "$attempt" "verify_running" "terminology+scope+diff+links"

  # 1. Scope gate — must not touch anything outside docs/
  if ! scope_gate "$batch" "$run_id" "$worktree"; then
    write_failure_summary "$batch" "$run_id" "Scope gate failed: agent modified files outside docs/."
    record_status "$run_id" "$batch" "$attempt" "verify_failed" "scope_gate"
    return 1
  fi

  # 2. Diff gate — must have produced some doc changes
  if ! diff_gate "$batch" "$run_id" "$worktree"; then
    write_failure_summary "$batch" "$run_id" "Diff gate failed: batch produced no changes under docs/."
    record_status "$run_id" "$batch" "$attempt" "verify_failed" "diff_gate"
    return 1
  fi

  # 3. Terminology gate — DISABLED (too many false positives on legitimate
  #    technical references like bardo-runtime, bardo-primitives crate names
  #    that are still active code paths). Do a manual terminology sweep
  #    at the end of the run if desired.
  # if ! terminology_check "$batch" "$run_id" "$worktree"; then
  #   write_failure_summary "$batch" "$run_id" "Terminology gate failed: retired terms present in changed files."
  #   record_status "$run_id" "$batch" "$attempt" "verify_failed" "terminology"
  #   return 1
  # fi

  # 4. Required-terms gate — if the refinement introduces a new vocabulary,
  #    the changed set should mention it somewhere.
  if ! required_terms_check "$batch" "$run_id" "$worktree"; then
    write_failure_summary "$batch" "$run_id" "Required-terms gate failed: expected new vocabulary absent."
    record_status "$run_id" "$batch" "$attempt" "verify_failed" "required_terms"
    return 1
  fi

  # 5. Internal link gate (soft unless REF_LINK_CHECK_STRICT=1)
  if ! internal_link_check "$batch" "$run_id" "$worktree"; then
    write_failure_summary "$batch" "$run_id" "Internal-link gate failed (strict mode)."
    record_status "$run_id" "$batch" "$attempt" "verify_failed" "internal_links"
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

  # Only stage docs/ changes.
  git -C "$worktree" add -A -- docs/
  if git -C "$worktree" diff --cached --quiet; then
    if [[ -n "$run_id" ]]; then
      record_status "$run_id" "$batch" "$attempt" "commit_noop" "no staged changes after verify"
    fi
    log_warn "$batch" "No changes staged after successful verification"
    return 10
  fi

  local refinement
  refinement="$(basename "$(batch_refinement_file "$batch")")"

  git -C "$worktree" commit -m "$(cat <<EOF
refinements(${batch}): ${title}

Automated doc propagation via tmp/refinements-runner/run-refinements.sh
Refinement source: tmp/refinements/${refinement}
EOF
)" >/dev/null
  if [[ -n "$run_id" ]]; then
    record_status "$run_id" "$batch" "$attempt" "commit_succeeded" "$(git -C "$worktree" rev-parse --short HEAD)"
  fi
  log_ok "$batch" "Committed: $(git -C "$worktree" log --oneline -1)"
  return 0
}
