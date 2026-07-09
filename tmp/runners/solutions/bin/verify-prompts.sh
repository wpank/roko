#!/usr/bin/env bash
# verify-prompts.sh — structural lint over generated prompts.
# Run after `bin/generate-prompts.py` to catch format drift.
# Uses ripgrep in batch mode rather than per-file grep loops.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUNNER_DIR="$(dirname "$SCRIPT_DIR")"
PROMPTS_DIR="$RUNNER_DIR/prompts"
TRACKER_MD="$RUNNER_DIR/ISSUE-TRACKER.md"
BATCHES_TOML="$RUNNER_DIR/batches.toml"

fail() { echo "FAIL: $*" >&2; exit 1; }

[[ -d "$PROMPTS_DIR" ]] || fail "no prompts directory at $PROMPTS_DIR"
[[ -f "$TRACKER_MD" ]] || fail "no $TRACKER_MD"
[[ -f "$BATCHES_TOML" ]] || fail "no $BATCHES_TOML"
command -v rg >/dev/null || fail "ripgrep (rg) is required"

prompt_count=$(find "$PROMPTS_DIR" -maxdepth 1 -name '*.prompt.md' -type f | wc -l | tr -d ' ')
batch_count=$(rg -c '^\[\[batch\]\]' "$BATCHES_TOML" || echo 0)
tracker_rows=$(rg -c '<a id="[a-z0-9-]+"></a> \[' "$TRACKER_MD" || echo 0)

echo "prompts:       $prompt_count"
echo "batches.toml:  $batch_count"
echo "tracker rows:  $tracker_rows"

[[ "$prompt_count" == "$batch_count" ]] || fail "prompts ($prompt_count) != batches ($batch_count)"
[[ "$prompt_count" == "$tracker_rows" ]] || fail "prompts ($prompt_count) != tracker rows ($tracker_rows)"

# Each canonical section must appear exactly N times across all prompts (where
# N = prompt_count). ripgrep multi-file with -c gives a per-file count, then we
# sum.
expect_section() {
  local pattern="$1" name="$2"
  local total
  total=$(rg --no-filename -c "$pattern" "$PROMPTS_DIR" 2>/dev/null | awk '{s+=$1} END {print s+0}')
  if [[ "$total" -lt "$prompt_count" ]]; then
    fail "section '$name' found $total times, expected >= $prompt_count"
  fi
  echo "section $name: $total"
}

expect_section '^## Tracker$'              'Tracker'
expect_section '^## Problem$'              'Problem'
expect_section '^## Exact Changes$'        'Exact Changes'
expect_section '^## Write Scope$'          'Write Scope'
expect_section '^## Verification Checklist$' 'Verification Checklist'
expect_section '^## Verify Recipe$'        'Verify Recipe'
expect_section '^## Acceptance Criteria$'  'Acceptance Criteria'
expect_section '^## Do NOT$'               'Do NOT'

# Every prompt's tracker anchor must exist in ISSUE-TRACKER.md.
# Extract anchors referenced from prompts (one line per prompt).
mapfile -t referenced < <(rg -oh 'ISSUE-TRACKER\.md#[a-z0-9-]+' "$PROMPTS_DIR" | sed 's|.*#||' | sort -u)
mapfile -t defined    < <(rg -oh '<a id="[a-z0-9-]+">'           "$TRACKER_MD"  | sed 's|<a id="||;s|">||' | sort -u)

missing=$(comm -23 <(printf '%s\n' "${referenced[@]}") <(printf '%s\n' "${defined[@]}"))
if [[ -n "$missing" ]]; then
  echo "missing anchors (referenced from prompts but not defined in tracker):" >&2
  printf '  - %s\n' $missing >&2
  fail "broken tracker anchors"
fi

# Every batches.toml id must have a prompt file.
mapfile -t batch_ids < <(awk -F'"' '/^id = "[A-Z]/ { print $2 }' "$BATCHES_TOML")
miss=0
for id in "${batch_ids[@]}"; do
  if [[ ! -f "$PROMPTS_DIR/${id}.prompt.md" ]]; then
    echo "batch $id has no prompt file" >&2
    miss=$((miss + 1))
  fi
done
[[ "$miss" -eq 0 ]] || fail "$miss batch id(s) had no prompt"

echo "OK: $prompt_count prompts pass structural lint"
