#!/bin/sh
set -eu
D="$(cd "$(dirname "$0")" && pwd)"
R="$(cd "$D/../.." && pwd)"

# Validate inventory.json structure and cross-check against Cargo.toml
python3 - "$D/inventory.json" "$R/Cargo.toml" <<'PYEOF'
import json, sys
inv = json.load(open(sys.argv[1]))
assert inv["schema_version"] == 1, "bad schema_version"
srcs = inv["sources"]
for o in inv["observations"]:
    assert o["source"] in srcs, f"observation source {o['source']} not in inventory sources"
assert inv["unknowns"], "missing unknowns"
assert inv["required_checks"], "missing required_checks"
cargo = open(sys.argv[2]).read()
assert "[workspace.metadata.dist]" in cargo, "workspace.metadata.dist missing from Cargo.toml"
assert "[workspace.package]" in cargo, "workspace.package missing from Cargo.toml"
PYEOF

# Check every required memo heading exists in go-no-go.md
for h in Recommendation Evidence "Blocking unknowns" "Required checks" "Rollback triggers" "Decision owner"; do
  rg -q "^## $h" "$D/go-no-go.md"
done

# Check every inventory source is referenced in the memo
for s in $(python3 -c "import json; print(*json.load(open('$D/inventory.json'))['sources'])"); do
  rg -qF "$s" "$D/go-no-go.md"
done

# Confirm unknowns remain unchecked (blocking items still have - [ ])
rg -q '^\- \[ \]' "$D/go-no-go.md"

echo "release-readiness demo: PASS"
