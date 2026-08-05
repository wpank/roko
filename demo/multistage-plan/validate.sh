#!/bin/sh
# validate.sh — portable POSIX acceptance harness for the multi-stage demo.
#
# Validates the completed decision package (discovery.md, evidence.json,
# decision.md) without modifying any files or accessing the network.
# Requires only python3 (standard library) and rg beyond POSIX sh.
#
# On success prints exactly:  multi-stage demo: PASS
set -eu

# Resolve the demo directory relative to this script (portable POSIX).
DEMO_DIR="$(cd "$(dirname "$0")" && pwd)"

#
# 1. Verify that all three required artifacts exist.
#
for f in discovery.md evidence.json decision.md; do
    if [ ! -f "$DEMO_DIR/$f" ]; then
        echo "validate.sh: missing required artifact: $f" >&2
        exit 1
    fi
done

#
# 2. Validate evidence.json structure using only the Python standard library.
#    Checks: schema_version, subject, sources, four stage kinds, and three
#    acceptance keys (each must be present and non-empty).
#
python3 - "$DEMO_DIR/evidence.json" <<'PYEOF'
import json
import sys

with open(sys.argv[1]) as fh:
    d = json.load(fh)

# --- schema_version must be exactly 1 ---
sv = d.get("schema_version")
assert sv == 1, "schema_version must be 1, got %r" % (sv,)

# --- subject must match the expected value ---
subj = d.get("subject")
assert subj == "roko-onboarding-demo", "subject mismatch: %r" % (subj,)

# --- sources must be exactly the three inspected repository files ---
expected_sources = {
    "README.md",
    "Cargo.toml",
    "docs/v1/00-architecture/15-crate-map.md",
}
actual_sources = set(d.get("sources", []))
assert expected_sources == actual_sources, (
    "sources mismatch: expected %r, got %r" % (expected_sources, actual_sources)
)

# --- all four required stage kinds must be present ---
kinds = {stage.get("kind") for stage in d.get("stages", [])}
required_kinds = {"discovery", "decision", "validation", "review"}
missing_kinds = required_kinds - kinds
assert not missing_kinds, "missing stage kinds: %r" % (missing_kinds,)

# --- three acceptance keys must be present and non-empty ---
acc = d.get("acceptance", {})
for key in ("human_readable", "machine_readable", "executable"):
    val = acc.get(key)
    assert val, "acceptance key %r missing or empty" % (key,)
PYEOF

#
# 3. Verify required Markdown headings in discovery.md and decision.md.
#

# discovery.md — top-level note, signals, facts, journey, risks.
rg -q '^# Discovery Note' "$DEMO_DIR/discovery.md"
rg -q '^## Repository signals' "$DEMO_DIR/discovery.md"
rg -q '^### Observed facts' "$DEMO_DIR/discovery.md"
rg -q '^## User journey' "$DEMO_DIR/discovery.md"
rg -q '^## Risks' "$DEMO_DIR/discovery.md"

# decision.md — memo, decision, evidence, alternatives, acceptance, follow-up.
rg -q '^# Decision Memo' "$DEMO_DIR/decision.md"
rg -q '^## Decision$' "$DEMO_DIR/decision.md"
rg -q '^## Evidence considered' "$DEMO_DIR/decision.md"
rg -q '^## Alternatives' "$DEMO_DIR/decision.md"
rg -q '^## Acceptance criteria' "$DEMO_DIR/decision.md"
rg -q '^## Follow-up' "$DEMO_DIR/decision.md"

#
# 4. Verify cross-file references bind the package together.
#

# decision.md must reference the upstream discovery and evidence artifacts.
rg -q 'discovery\.md' "$DEMO_DIR/decision.md"
rg -q 'evidence\.json' "$DEMO_DIR/decision.md"

# discovery.md must reference the three inspected repository sources.
rg -q 'README\.md' "$DEMO_DIR/discovery.md"
rg -q 'Cargo\.toml' "$DEMO_DIR/discovery.md"
rg -q 'crate-map\.md' "$DEMO_DIR/discovery.md"

echo "multi-stage demo: PASS"
