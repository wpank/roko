#!/usr/bin/env bash
# scan-docs.sh — Scan docs/ sections, enumerate .md files, extract identifiers.
#
# Functions:
#   enumerate_docs_files <section_num>  — list all .md files in the section dir
#   extract_doc_identifiers <section_num> — extract backtick-quoted Rust identifiers
#   docs_section_summary <section_num>  — line count + file count

set -uo pipefail

_SCAN_DOCS_LOADED=1
_SCAN_DOCS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Guard: only source section-map if not already loaded
if [[ -z "${_SECTION_MAP_LOADED:-}" ]]; then
  source "$_SCAN_DOCS_DIR/section-map.sh"
fi

: "${ROKO_ROOT:=/Users/will/dev/nunchi/roko/roko}"

# List all .md files in a section's docs directory, INDEX.md first.
enumerate_docs_files() {
  local num="$1"
  local dir="$ROKO_ROOT/$(docs_dir_for "$num")"
  if [[ ! -d "$dir" ]]; then
    return 1
  fi

  # INDEX.md first, then sorted numbered files
  if [[ -f "$dir/INDEX.md" ]]; then
    echo "$(docs_dir_for "$num")/INDEX.md"
  fi
  find "$dir" -maxdepth 1 -name '*.md' ! -name 'INDEX.md' -print \
    | sort \
    | while IFS= read -r path; do
      # Emit relative to repo root
      echo "${path#"$ROKO_ROOT/"}"
    done
}

# Extract Rust-like identifiers from backtick-quoted terms in docs.
# Captures: `FooBar`, `some_function`, `SomeStruct<T>` etc.
extract_doc_identifiers() {
  local num="$1"
  local dir="$ROKO_ROOT/$(docs_dir_for "$num")"
  [[ -d "$dir" ]] || return 1

  grep -oP '`[A-Z][a-zA-Z0-9_]+(?:<[^>]+>)?`' "$dir"/*.md 2>/dev/null \
    | sed 's/.*:`//; s/`$//' \
    | sed 's/<.*>//' \
    | sort -u
}

# Summary stats for a section's docs.
docs_section_summary() {
  local num="$1"
  local dir="$ROKO_ROOT/$(docs_dir_for "$num")"
  [[ -d "$dir" ]] || return 1

  local file_count line_count
  file_count=$(find "$dir" -maxdepth 1 -name '*.md' | wc -l | tr -d ' ')
  line_count=$(cat "$dir"/*.md 2>/dev/null | wc -l | tr -d ' ')
  echo "${file_count} files, ${line_count} lines"
}

# Enumerate all sections that have docs directories.
enumerate_all_sections() {
  for entry in "${SECTION_REGISTRY[@]}"; do
    local num
    num="$(section_num "$entry")"
    local dir="$ROKO_ROOT/$(docs_dir_for "$num")"
    if [[ -d "$dir" ]]; then
      echo "$num"
    fi
  done
}
