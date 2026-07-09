#!/usr/bin/env bash
# scan-crates.sh — Scan crate source files, extract public API declarations.
#
# Functions:
#   enumerate_crate_files <crate> [subdir]  — list all .rs files
#   extract_public_api <crate> [subdir]     — grep pub struct/trait/fn/enum
#   crate_loc <crate>                       — line count of .rs files

set -uo pipefail

_SCAN_CRATES_LOADED=1
_SCAN_CRATES_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Guard: only source section-map if not already loaded
if [[ -z "${_SECTION_MAP_LOADED:-}" ]]; then
  source "$_SCAN_CRATES_DIR/section-map.sh"
fi

: "${ROKO_ROOT:=/Users/will/dev/nunchi/roko/roko}"

# List all .rs files in a crate's src/, optionally scoped to a subdir.
# Output is relative to repo root.
enumerate_crate_files() {
  local crate="$1"
  local subdir="${2:-}"
  local src_dir="$ROKO_ROOT/crates/$crate/src/${subdir}"
  [[ -d "$src_dir" ]] || return 0

  find "$src_dir" -name '*.rs' -print \
    | sort \
    | while IFS= read -r path; do
      echo "${path#"$ROKO_ROOT/"}"
    done
}

# List all .rs files for a section's target crates.
# Handles the section-11 special case (safety/ subdir).
enumerate_section_crate_files() {
  local num="$1"
  local entry
  entry="$(_section_entry "$num")" || return 1
  local crates_csv
  crates_csv="$(section_crates "$entry")"

  if [[ "$crates_csv" == "cross-cutting" ]]; then
    # For section 20, list files from the crosscut crate set
    IFS=',' read -r -a crate_arr <<< "$CROSSCUT_CRATES"
    for c in "${crate_arr[@]}"; do
      enumerate_crate_files "$c"
    done
    return
  fi

  local subdir
  subdir="$(crate_subdir_for "$num")"

  IFS=',' read -r -a crate_arr <<< "$crates_csv"
  for c in "${crate_arr[@]}"; do
    enumerate_crate_files "$c" "$subdir"
  done
}

# Extract public API declarations from a crate.
extract_public_api() {
  local crate="$1"
  local subdir="${2:-}"
  local src_dir="$ROKO_ROOT/crates/$crate/src/${subdir}"
  [[ -d "$src_dir" ]] || return 0

  grep -rn 'pub struct\|pub trait\|pub fn\|pub enum\|pub type\|pub const' \
    "$src_dir" --include='*.rs' 2>/dev/null \
    | sed "s|$ROKO_ROOT/||" \
    | sort
}

# Extract public API for all crates in a section.
extract_section_public_api() {
  local num="$1"
  local entry
  entry="$(_section_entry "$num")" || return 1
  local crates_csv
  crates_csv="$(section_crates "$entry")"

  if [[ "$crates_csv" == "cross-cutting" ]]; then
    IFS=',' read -r -a crate_arr <<< "$CROSSCUT_CRATES"
    for c in "${crate_arr[@]}"; do
      extract_public_api "$c"
    done
    return
  fi

  local subdir
  subdir="$(crate_subdir_for "$num")"

  IFS=',' read -r -a crate_arr <<< "$crates_csv"
  for c in "${crate_arr[@]}"; do
    extract_public_api "$c" "$subdir"
  done
}

# Line count for a crate's .rs files.
crate_loc() {
  local crate="$1"
  local src_dir="$ROKO_ROOT/crates/$crate/src/"
  [[ -d "$src_dir" ]] || { echo "0"; return; }
  find "$src_dir" -name '*.rs' -exec cat {} + 2>/dev/null | wc -l | tr -d ' '
}

# Write scope string for a section (used in prompts).
write_scope_for() {
  local num="$1"
  local entry
  entry="$(_section_entry "$num")" || return 1
  local crates_csv
  crates_csv="$(section_crates "$entry")"

  if [[ "$crates_csv" == "cross-cutting" ]]; then
    echo "Any crate under crates/ (cross-cutting batch)"
    return
  fi

  local subdir
  subdir="$(crate_subdir_for "$num")"

  IFS=',' read -r -a crate_arr <<< "$crates_csv"
  for c in "${crate_arr[@]}"; do
    echo "crates/$c/src/${subdir}"
  done
}
