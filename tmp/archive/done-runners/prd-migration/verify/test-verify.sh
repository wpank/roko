#!/usr/bin/env bash
# test-verify.sh — self-test for the verify_topic() function.
#
# Creates two mock topic outputs:
#   1. A "good" topic that passes all checks
#   2. A "bad" topic with forbidden terms, short sub-docs, missing INDEX
#
# Expects:
#   Good topic: verify returns 0 (pass) or 2 (pass with warnings)
#   Bad topic: verify returns 1 (hard failures)
#
# Exit 0 if both match expectations, else exit 1.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MIGRATION_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Point OUTPUT_ROOT at a temp directory for the test
export OUTPUT_ROOT="$MIGRATION_ROOT/logs/verify-test-output"
rm -rf "$OUTPUT_ROOT"
mkdir -p "$OUTPUT_ROOT"

# Source after overriding OUTPUT_ROOT
# shellcheck source=../lib/common.sh
source "$MIGRATION_ROOT/lib/common.sh"
# shellcheck source=../lib/verify.sh
source "$MIGRATION_ROOT/lib/verify.sh"

PASS=0
FAIL=0

pass_test() {
    printf '  %s[PASS]%s %s\n' "$C_GREEN" "$C_RESET" "$1"
    PASS=$((PASS + 1))
}
fail_test() {
    printf '  %s[FAIL]%s %s\n' "$C_RED" "$C_RESET" "$1"
    FAIL=$((FAIL + 1))
}

# --- Helper: generate a sub-doc with N lines of content --------------------

generate_subdoc() {
    local file="$1"
    local title="$2"
    local lines="$3"
    {
        echo "# $title"
        echo
        echo "> Test sub-doc for verify-topic smoke test"
        echo
        echo "**Topic**: [Test](./INDEX.md)"
        echo
        echo "---"
        echo
        echo "## Abstract"
        echo
        echo "This is a test sub-doc used to verify that the verify_topic function correctly"
        echo "handles sub-docs of the required length. It discusses the Synapse Architecture"
        echo "and the Engram data type as part of Roko (formerly the bardo framework)."
        echo "The 6 Synapse traits are Substrate, Scorer, Gate, Router, Composer, Policy."
        echo "This topic integrates with Layer 2 (Scaffold) and Layer 3 (Harness)."
        echo
        echo "Citations: Liu et al. 2023 (arXiv:2307.03172), Sumers et al. 2023 (arXiv:2309.02427),"
        echo "Kanerva 2009 (Cognitive Computation 1(2)), Meta-Harness Lee et al. 2026,"
        echo "FrugalGPT Chen et al. 2023 (arXiv:2305.05176)."
        echo
        echo "---"
        echo
        echo "## Detailed discussion"
        echo
        # Pad to reach the target line count
        local i
        for (( i=0 ; i<lines ; i++ )); do
            echo "Line $i: The Engram is the content-addressed unit of agent cognition. Roko's Synapse Architecture encodes this via BLAKE3 hashing and 6 trait composition. See topic 00-architecture for details."
        done
    } > "$file"
}

# --- Generate a GOOD topic (should pass) -----------------------------------

echo
log_header "TEST 1: Good topic should PASS"

GOOD_TOPIC="00-architecture"  # pretend it's 00-architecture
GOOD_DIR="$OUTPUT_ROOT/$GOOD_TOPIC"
mkdir -p "$GOOD_DIR"

# Write a substantial INDEX.md (≥50 lines)
{
    echo "# Topic 00 — Test Architecture Index"
    echo
    echo "> Test INDEX for the verify-topic self-test. Covers Roko / Synapse / Engrams."
    echo
    echo "**Part of**: Roko PRD"
    echo "**Status**: Test fixture"
    echo "**Model**: claude-opus-4-6"
    echo "**Last generated**: 2026-04-11"
    echo "**Prerequisites**: None"
    echo
    echo "---"
    echo
    echo "## Abstract"
    echo
    echo "This is a test INDEX.md for verifying that the verify_topic() function"
    echo "passes correctly-structured topics. It references the Synapse Architecture,"
    echo "Engrams as core data, and the 6 Synapse traits. This is a synthetic fixture"
    echo "used only by the test harness; it is not part of the real PRD output."
    echo
    echo "---"
    echo
    echo "## Contents"
    echo
    echo "| # | Sub-doc | What it covers |"
    echo "|---|---|---|"
    for i in 00 01 02 03 04 05 06 07 08 09; do
        echo "| $i | [\`$i-test.md\`](./$i-test.md) | Test sub-doc $i — Engram / Synapse content |"
    done
    echo
    echo "---"
    echo
    echo "## Prerequisites"
    echo
    echo "None — this is a test fixture for the verify-topic self-test."
    echo
    echo "---"
    echo
    echo "## Key academic foundations"
    echo
    echo "- Liu et al. 2023 (arXiv:2307.03172) — Lost in the Middle"
    echo "- Sumers et al. 2023 CoALA (arXiv:2309.02427) — 9-step cognitive pipeline"
    echo "- Kanerva 2009 (Cognitive Computation 1(2)) — HDC / VSA"
    echo "- Meta-Harness Lee et al. 2026 — scaffold engineering"
    echo "- FrugalGPT Chen et al. 2023 (arXiv:2305.05176) — tier routing"
    echo "- Woolley et al. 2010 Science 330(6004) — C-Factor"
    echo "- Beer 1972 Brain of the Firm — VSM"
    echo "- Conant & Ashby 1970 Good Regulator Theorem"
    echo
    echo "---"
    echo
    echo "## Cross-references"
    echo
    echo "- [01-orchestration](../01-orchestration/INDEX.md) — Layer 4 Orchestration"
    echo "- [02-agents](../02-agents/INDEX.md) — Layer 1 Framework"
    echo "- [06-neuro](../06-neuro/INDEX.md) — Knowledge cross-cut"
    echo
    echo "---"
    echo
    echo "## Current status and implementation gaps"
    echo
    echo "Test fixture only. Not used in production. The real 00-architecture topic"
    echo "will be generated by the full migration runner with actual content."
    echo
    echo "---"
    echo
    echo "## Generation Notes"
    echo
    echo "- **Generated**: test-fixture"
    echo "- **Model**: claude-opus-4-6 (simulated)"
    echo "- **Sub-docs produced**: 10"
    echo "- **Total lines**: ~2500"
} > "$GOOD_DIR/INDEX.md"

# Write 10 sub-docs, each with ~250 lines
for i in 00 01 02 03 04 05 06 07 08 09; do
    generate_subdoc "$GOOD_DIR/$i-test.md" "Test sub-doc $i" 230
done

GOOD_RC=0
verify_topic "$GOOD_TOPIC" || GOOD_RC=$?

case "$GOOD_RC" in
    0) pass_test "Good topic returned 0 (pass)" ;;
    2) pass_test "Good topic returned 2 (pass with warnings) — acceptable" ;;
    *) fail_test "Good topic returned $GOOD_RC (expected 0 or 2)" ;;
esac

# --- Generate a BAD topic (should fail) ------------------------------------

echo
log_header "TEST 2: Bad topic with forbidden terms should FAIL"

BAD_TOPIC="01-orchestration"
BAD_DIR="$OUTPUT_ROOT/$BAD_TOPIC"
mkdir -p "$BAD_DIR"

# Write INDEX.md that's too short (< 50 lines)
{
    echo "# Bad Topic"
    echo
    echo "Too short."
} > "$BAD_DIR/INDEX.md"

# Write 3 sub-docs (< min 5) each with too few lines
for i in 00 01 02; do
    {
        echo "# Bad sub-doc $i"
        echo
        echo "This doc uses forbidden terms like Thanatopsis and Necrocracy."
        echo "It also says 'fleet' which is the wrong rename for Clade."
        echo "It mentions 'GNOS token' which is the old name."
        echo "Total: $i"
    } > "$BAD_DIR/$i-bad.md"
done

BAD_RC=0
verify_topic "$BAD_TOPIC" || BAD_RC=$?

case "$BAD_RC" in
    1) pass_test "Bad topic returned 1 (hard failures — correctly caught)" ;;
    *) fail_test "Bad topic returned $BAD_RC (expected 1)" ;;
esac

# --- Specific forbidden term detection test --------------------------------

echo
log_header "TEST 3: Forbidden term scanning"

# Check that the grep-based forbidden-term scan catches Thanatopsis
if grep -riq "Thanatopsis" "$BAD_DIR" --include='*.md' 2>/dev/null; then
    pass_test "grep finds 'Thanatopsis' in the bad topic"
else
    fail_test "grep does NOT find 'Thanatopsis' in the bad topic (broken)"
fi

if grep -riq "Necrocracy" "$BAD_DIR" --include='*.md' 2>/dev/null; then
    pass_test "grep finds 'Necrocracy' in the bad topic"
else
    fail_test "grep does NOT find 'Necrocracy' in the bad topic (broken)"
fi

if grep -riq "GNOS token" "$BAD_DIR" --include='*.md' 2>/dev/null; then
    pass_test "grep finds 'GNOS token' in the bad topic"
else
    fail_test "grep does NOT find 'GNOS token' in the bad topic (broken)"
fi

# --- Specific PASS term detection in good topic ----------------------------

echo
log_header "TEST 4: Required term scanning in good topic"

for term in Roko Engram Synapse; do
    if grep -riq "$term" "$GOOD_DIR" --include='*.md' 2>/dev/null; then
        pass_test "Required term '$term' found in good topic"
    else
        fail_test "Required term '$term' NOT found in good topic (fixture broken)"
    fi
done

# --- Summary ---------------------------------------------------------------

echo
log_header "SUMMARY"
printf '  %sPASS:%s %d  %sFAIL:%s %d\n\n' "$C_GREEN" "$C_RESET" "$PASS" "$C_RED" "$C_RESET" "$FAIL"

if (( FAIL > 0 )); then
    exit 1
fi
exit 0
