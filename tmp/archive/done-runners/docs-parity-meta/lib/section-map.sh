#!/usr/bin/env bash
# section-map.sh — Section registry: the source of truth for the generator.
#
# Each section has: number, slug, display name, target crates, priority, group,
# dependency list (as batch IDs), and template type.

set -uo pipefail

_SECTION_MAP_LOADED=1

# ---------------------------------------------------------------------------
# Section registry
# ---------------------------------------------------------------------------
# Format per entry:  NUM|SLUG|DISPLAY|CRATES|PRIORITY|GROUP|DEPS|TEMPLATE
#
# CRATES: comma-separated crate names (or "cross-cutting" for section 20)
# DEPS:   space-separated batch IDs (e.g. "DP00 DP01")
# TEMPLATE: code | stub | crosscut

SECTION_REGISTRY=(
  "00|architecture|Architecture|roko-core|P0|core||code"
  "01|orchestration|Orchestration|roko-orchestrator,roko-cli|P0|core|DP00|code"
  "02|agents|Agents|roko-agent|P0|core|DP00|code"
  "03|composition|Composition|roko-compose|P0|core|DP00|code"
  "04|verification|Verification|roko-gate|P0|core|DP00 DP01|code"
  "05|learning|Learning|roko-learn|P0|core|DP00|code"
  "06|neuro|Neuro|roko-neuro,roko-primitives|P1|extensions|DP00 DP05|code"
  "07|conductor|Conductor|roko-conductor|P1|extensions|DP00 DP01|code"
  "08|chain|Chain|roko-chain|P2|phase2|DP00|stub"
  "09|daimon|Daimon|roko-daimon|P2|phase2|DP00|stub"
  "10|dreams|Dreams|roko-dreams|P2|phase2|DP00|stub"
  "11|safety|Safety|roko-agent|P0|safety-iface|DP00 DP02|code"
  "12|interfaces|Interfaces|roko-cli,roko-serve,roko-agent-server|P0|safety-iface|DP00 DP01 DP02|code"
  "13|coordination|Coordination|roko-orchestrator|P1|infra|DP00 DP01|code"
  "14|identity-economy|Identity & Economy|roko-chain|P2|phase2|DP00 DP08|stub"
  "15|code-intelligence|Code Intelligence|roko-index,roko-mcp-code,roko-lang-rust,roko-lang-typescript,roko-lang-go|P1|infra|DP00|code"
  "16|heartbeat|Heartbeat|roko-runtime|P1|infra|DP00 DP01|code"
  "17|lifecycle|Lifecycle|roko-agent,roko-runtime|P1|infra|DP00 DP02|code"
  "18|tools|Tools|roko-std,roko-agent|P1|infra|DP00 DP02|code"
  "19|deployment|Deployment|roko-cli|P1|infra|DP00 DP12|code"
  "20|technical-analysis|Technical Analysis|cross-cutting|P2|phase2|DP00 DP04 DP05|crosscut"
)

# Section 21 (references) is intentionally excluded — docs-only, no code.

# Canonical batch order for the runner
ALL_DP_BATCHES=(
  "DP00" "DP01" "DP02" "DP03" "DP04" "DP05"
  "DP06" "DP07"
  "DP08" "DP09" "DP10"
  "DP11" "DP12"
  "DP13"
  "DP14" "DP15" "DP16" "DP17" "DP18" "DP19"
  "DP20"
)

# ---------------------------------------------------------------------------
# Field accessors
# ---------------------------------------------------------------------------
# Usage: section_field <index> <field>
# Fields: 0=NUM, 1=SLUG, 2=DISPLAY, 3=CRATES, 4=PRIORITY, 5=GROUP, 6=DEPS, 7=TEMPLATE

_section_entry() {
  local num="$1"
  for entry in "${SECTION_REGISTRY[@]}"; do
    local entry_num="${entry%%|*}"
    if [[ "$entry_num" == "$num" ]]; then
      echo "$entry"
      return 0
    fi
  done
  return 1
}

section_field() {
  local entry="$1"
  local field_idx="$2"
  echo "$entry" | awk -F'|' -v f="$((field_idx + 1))" '{print $f}'
}

section_num()      { section_field "$1" 0; }
section_slug()     { section_field "$1" 1; }
section_display()  { section_field "$1" 2; }
section_crates()   { section_field "$1" 3; }
section_priority() { section_field "$1" 4; }
section_group()    { section_field "$1" 5; }
section_deps()     { section_field "$1" 6; }
section_template() { section_field "$1" 7; }

# Batch ID from section number: "00" → "DP00"
batch_id_for() {
  local num="$1"
  printf 'DP%s' "$num"
}

# Section number from batch ID: "DP00" → "00"
section_num_for_batch() {
  echo "${1#DP}"
}

# Full docs directory for a section number
docs_dir_for() {
  local num="$1"
  local entry
  entry="$(_section_entry "$num")" || return 1
  local slug
  slug="$(section_slug "$entry")"
  echo "docs/${num}-${slug}"
}

# Subdir filter for section 11 (safety → roko-agent/src/safety/)
# Returns "" for most sections, "safety/" for section 11
crate_subdir_for() {
  local num="$1"
  if [[ "$num" == "11" ]]; then
    echo "safety/"
  else
    echo ""
  fi
}

# List of verify commands for a batch
verify_commands_for() {
  local num="$1"
  local entry
  entry="$(_section_entry "$num")" || return 1
  local crates_csv
  crates_csv="$(section_crates "$entry")"

  if [[ "$crates_csv" == "cross-cutting" ]]; then
    cat <<'EOF'
cargo check --workspace
cargo clippy --workspace --no-deps -- -D warnings
EOF
    return
  fi

  # Build -p flags from crate list
  local pkg_flags=""
  IFS=',' read -r -a crate_arr <<< "$crates_csv"
  for c in "${crate_arr[@]}"; do
    pkg_flags="$pkg_flags -p $c"
  done

  cat <<EOF
cargo check${pkg_flags}
cargo test${pkg_flags} --lib --no-run
cargo clippy${pkg_flags} --no-deps -- -D warnings
EOF
}

# ---------------------------------------------------------------------------
# Group helpers
# ---------------------------------------------------------------------------

all_groups() {
  echo "core extensions safety-iface infra phase2"
}

sections_in_group() {
  local target_group="$1"
  for entry in "${SECTION_REGISTRY[@]}"; do
    local grp
    grp="$(section_group "$entry")"
    if [[ "$grp" == "$target_group" ]]; then
      section_num "$entry"
    fi
  done
}

# ---------------------------------------------------------------------------
# Cross-cutting crates for DP20
# ---------------------------------------------------------------------------
CROSSCUT_CRATES="roko-core,roko-orchestrator,roko-gate,roko-learn,roko-neuro,roko-primitives"
