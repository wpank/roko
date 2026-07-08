#!/usr/bin/env bash
# generate.sh — Main entry point for the docs-parity meta-script system.
#
# Generates tmp/docs-parity2/ with all infrastructure, context, and prompts.
# Fully idempotent — re-run when docs change to regenerate.
#
# Usage:
#   bash tmp/docs-parity-meta/generate.sh
#   bash tmp/docs-parity-meta/generate.sh --clean   # rm docs-parity2/ first

set -euo pipefail
IFS=$'\n\t'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

export ROKO_ROOT="${ROKO_ROOT:-/Users/will/dev/nunchi/roko/roko}"
export META_ROOT="$SCRIPT_DIR"
export OUT_ROOT="$ROKO_ROOT/tmp/docs-parity2"

# Source all library files
source "$SCRIPT_DIR/lib/section-map.sh"
source "$SCRIPT_DIR/lib/scan-docs.sh"
source "$SCRIPT_DIR/lib/scan-crates.sh"
source "$SCRIPT_DIR/lib/generate-prompts.sh"
source "$SCRIPT_DIR/lib/generate-context-pack.sh"
source "$SCRIPT_DIR/lib/generate-infrastructure.sh"

# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

if [[ "${1:-}" == "--clean" ]]; then
  echo "Cleaning $OUT_ROOT..."
  rm -rf "$OUT_ROOT"
fi

echo "============================================"
echo "  docs-parity-meta generator"
echo "  Output: $OUT_ROOT"
echo "============================================"
echo

# ---------------------------------------------------------------------------
# Step 1: Create output directory structure
# ---------------------------------------------------------------------------
echo "[1/5] Creating directory structure..."
mkdir -p "$OUT_ROOT"/{lib,prompts,context-pack,logs}

# ---------------------------------------------------------------------------
# Step 2: Generate runner infrastructure
# ---------------------------------------------------------------------------
echo "[2/5] Generating runner infrastructure..."
generate_infrastructure

# ---------------------------------------------------------------------------
# Step 3: Generate context pack
# ---------------------------------------------------------------------------
echo "[3/5] Generating context pack..."
generate_context_pack

# ---------------------------------------------------------------------------
# Step 4: Generate prompts
# ---------------------------------------------------------------------------
echo "[4/5] Generating prompts..."
generate_all_prompts

# ---------------------------------------------------------------------------
# Step 5: Verify output
# ---------------------------------------------------------------------------
echo "[5/5] Verifying output..."

errors=0

# Check runner exists and is executable
if [[ -x "$OUT_ROOT/run-docs-parity2.sh" ]]; then
  echo "  OK: run-docs-parity2.sh exists and is executable"
else
  echo "  ERR: run-docs-parity2.sh missing or not executable"
  errors=$((errors + 1))
fi

# Check lib files
for f in common.sh spawn.sh verify.sh; do
  if [[ -f "$OUT_ROOT/lib/$f" ]]; then
    echo "  OK: lib/$f"
  else
    echo "  ERR: lib/$f missing"
    errors=$((errors + 1))
  fi
done

# Check context pack
for f in 00-DOCS-PARITY-RULES.md 01-SECTION-CRATE-MAP.md 02-WORKSPACE-TOPOLOGY.md \
         03-EXISTING-PARITY-SUMMARY.md 04-CODE-CONVENTIONS.md 05-PHASE2-STUB-GUIDANCE.md; do
  if [[ -f "$OUT_ROOT/context-pack/$f" ]]; then
    echo "  OK: context-pack/$f"
  else
    echo "  ERR: context-pack/$f missing"
    errors=$((errors + 1))
  fi
done

# Check prompts
for entry in "${SECTION_REGISTRY[@]}"; do
  local_num="$(section_num "$entry")"
  batch_id="$(batch_id_for "$local_num")"
  prompt_file="$OUT_ROOT/prompts/${batch_id}.prompt.md"
  if [[ -f "$prompt_file" ]]; then
    echo "  OK: prompts/${batch_id}.prompt.md"
  else
    echo "  ERR: prompts/${batch_id}.prompt.md missing"
    errors=$((errors + 1))
  fi
done

# Check BATCHES.md
if [[ -f "$OUT_ROOT/BATCHES.md" ]]; then
  echo "  OK: BATCHES.md"
else
  echo "  ERR: BATCHES.md missing"
  errors=$((errors + 1))
fi

echo
if (( errors > 0 )); then
  echo "FAILED: $errors error(s) detected"
  exit 1
else
  echo "SUCCESS: All files generated"
  echo
  echo "Generated files:"
  find "$OUT_ROOT" -type f | sort | sed "s|$ROKO_ROOT/||" | sed 's/^/  /'
  echo
  echo "Next steps:"
  echo "  bash tmp/docs-parity2/run-docs-parity2.sh --list"
  echo "  bash tmp/docs-parity2/run-docs-parity2.sh --dry-run --only DP00"
fi
