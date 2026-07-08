#!/usr/bin/env bash
# generate-prompts.sh — Render per-section prompt files into docs-parity2/prompts/.
#
# For each section: reads template, substitutes docs file lists, code file lists,
# public API, gap data, verify commands. Writes to docs-parity2/prompts/.

set -uo pipefail

_GEN_PROMPTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Guard: only source dependencies if not already loaded
if [[ -z "${_SECTION_MAP_LOADED:-}" ]]; then
  source "$_GEN_PROMPTS_DIR/section-map.sh"
fi
if [[ -z "${_SCAN_DOCS_LOADED:-}" ]]; then
  source "$_GEN_PROMPTS_DIR/scan-docs.sh"
fi
if [[ -z "${_SCAN_CRATES_LOADED:-}" ]]; then
  source "$_GEN_PROMPTS_DIR/scan-crates.sh"
fi

: "${ROKO_ROOT:=/Users/will/dev/nunchi/roko/roko}"
: "${META_ROOT:=$ROKO_ROOT/tmp/docs-parity-meta}"
: "${OUT_ROOT:=$ROKO_ROOT/tmp/docs-parity2}"

# ---------------------------------------------------------------------------
# Template rendering
# ---------------------------------------------------------------------------

# Format a file list as markdown bullet points
format_file_list() {
  while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    echo "- \`$line\`"
  done
}

# Format public API as a code block
format_public_api() {
  local api_text="$1"
  if [[ -z "$api_text" ]]; then
    echo "(No public API declarations found — crate may be empty or new)"
    return
  fi
  echo '```'
  echo "$api_text" | head -80
  local total
  total=$(echo "$api_text" | wc -l | tr -d ' ')
  if (( total > 80 )); then
    echo "... ($total total declarations, showing first 80)"
  fi
  echo '```'
}

# Compute gap items: identifiers in docs but not in code public API
compute_gaps() {
  local doc_ids="$1"
  local pub_api="$2"

  if [[ -z "$doc_ids" ]]; then
    echo "(No identifiers extracted from docs)"
    return
  fi

  local gaps=""
  while IFS= read -r ident; do
    [[ -z "$ident" ]] && continue
    if ! echo "$pub_api" | grep -qF "$ident"; then
      gaps="${gaps}- \`$ident\` — referenced in docs but not found in code public API\n"
    fi
  done <<< "$doc_ids"

  if [[ -z "$gaps" ]]; then
    echo "(All documented identifiers appear to have code counterparts)"
  else
    printf '%b' "$gaps"
  fi
}

# Load parity analysis files for a section (if they exist in tmp/docs-parity/)
enumerate_parity_files() {
  local num="$1"
  local parity_dir="$ROKO_ROOT/tmp/docs-parity/$num"
  if [[ ! -d "$parity_dir" ]]; then
    echo "(No prior parity analysis available for section $num)"
    return
  fi

  find "$parity_dir" -name '*.md' -print \
    | sort \
    | while IFS= read -r path; do
      echo "- \`${path#"$ROKO_ROOT/"}\`"
    done
}

# Render a single prompt
render_prompt() {
  local num="$1"
  local entry
  entry="$(_section_entry "$num")" || { echo "Unknown section: $num" >&2; return 1; }

  local batch_id slug display crates_csv priority group deps template_type
  batch_id="$(batch_id_for "$num")"
  slug="$(section_slug "$entry")"
  display="$(section_display "$entry")"
  crates_csv="$(section_crates "$entry")"
  priority="$(section_priority "$entry")"
  group="$(section_group "$entry")"
  deps="$(section_deps "$entry")"
  template_type="$(section_template "$entry")"

  local docs_dir docs_summary
  docs_dir="$(docs_dir_for "$num")"
  docs_summary="$(docs_section_summary "$num" 2>/dev/null || echo "? files, ? lines")"
  local docs_file_count="${docs_summary%% *}"

  # Select template
  local template_file="$META_ROOT/templates/prompt-${template_type}.md.tmpl"
  if [[ ! -f "$template_file" ]]; then
    echo "Missing template: $template_file" >&2
    return 1
  fi

  # Gather data
  local docs_files code_files doc_ids pub_api parity_files verify_cmds write_scope

  docs_files="$(enumerate_docs_files "$num" | format_file_list)"
  code_files="$(enumerate_section_crate_files "$num" | format_file_list)"
  doc_ids="$(extract_doc_identifiers "$num" 2>/dev/null || true)"
  pub_api="$(extract_section_public_api "$num" 2>/dev/null || true)"
  parity_files="$(enumerate_parity_files "$num")"
  verify_cmds="$(verify_commands_for "$num")"
  write_scope="$(write_scope_for "$num" | format_file_list)"

  local doc_ids_display pub_api_display gap_items

  if [[ -n "$doc_ids" ]]; then
    doc_ids_display="$(echo "$doc_ids" | while IFS= read -r id; do echo "- \`$id\`"; done)"
  else
    doc_ids_display="(Run the generator to extract identifiers from docs)"
  fi

  pub_api_display="$(format_public_api "$pub_api")"
  gap_items="$(compute_gaps "$doc_ids" "$pub_api")"

  # Format dependencies display
  local deps_display
  if [[ -z "$deps" ]]; then
    deps_display="none"
  else
    deps_display="$deps"
  fi

  # Format crates display
  local crates_display
  if [[ "$crates_csv" == "cross-cutting" ]]; then
    crates_display="cross-cutting (multiple crates)"
  else
    crates_display="$(echo "$crates_csv" | tr ',' ', ')"
  fi

  # Render by reading template line-by-line and substituting placeholders.
  # This avoids awk issues with multi-line content in -v variables.
  while IFS= read -r line || [[ -n "$line" ]]; do
    # Single-value substitutions (inline)
    line="${line//\{\{BATCH_ID\}\}/$batch_id}"
    line="${line//\{\{SECTION_NUM\}\}/$num}"
    line="${line//\{\{SECTION_SLUG\}\}/$slug}"
    line="${line//\{\{DOCS_DIR\}\}/$docs_dir}"
    line="${line//\{\{DOCS_FILE_COUNT\}\}/$docs_file_count}"
    line="${line//\{\{CRATES_DISPLAY\}\}/$crates_display}"
    line="${line//\{\{PRIORITY\}\}/$priority}"
    line="${line//\{\{GROUP\}\}/$group}"
    line="${line//\{\{DEPS_DISPLAY\}\}/$deps_display}"

    # Multi-line block substitutions: if the line IS the placeholder, emit the block
    case "$line" in
      *"{{DOCS_FILE_LIST}}"*)    echo "$docs_files" ;;
      *"{{PARITY_FILE_LIST}}"*)  echo "$parity_files" ;;
      *"{{CODE_FILE_LIST}}"*)    echo "$code_files" ;;
      *"{{DOC_IDENTIFIERS}}"*)   echo "$doc_ids_display" ;;
      *"{{PUBLIC_API}}"*)        echo "$pub_api_display" ;;
      *"{{GAP_ITEMS}}"*)         echo "$gap_items" ;;
      *"{{VERIFY_COMMANDS}}"*)   echo "$verify_cmds" ;;
      *"{{WRITE_SCOPE}}"*)       echo "$write_scope" ;;
      *)                         echo "$line" ;;
    esac
  done < "$template_file"
}

# Generate all prompts
generate_all_prompts() {
  local out_dir="$OUT_ROOT/prompts"
  mkdir -p "$out_dir"

  for entry in "${SECTION_REGISTRY[@]}"; do
    local num
    num="$(section_num "$entry")"
    local batch_id
    batch_id="$(batch_id_for "$num")"
    local out_file="$out_dir/${batch_id}.prompt.md"

    echo "  Generating $batch_id ($(section_slug "$entry"))..."
    render_prompt "$num" > "$out_file"
  done
}
