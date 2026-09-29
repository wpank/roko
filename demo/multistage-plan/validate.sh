#!/bin/sh
set -eu

# Verify required artefacts exist.
for f in demo/multistage-plan/discovery.md demo/multistage-plan/evidence.json demo/multistage-plan/decision.md; do
  test -f "$f"
done

# Verify required Markdown headings are present.
rg -q '^## Repository signals' demo/multistage-plan/discovery.md
rg -q '^## Decision' demo/multistage-plan/decision.md

# Verify cross‑file references inside the decision document.
rg -q 'discovery\.md' demo/multistage-plan/decision.md
rg -q 'evidence\.json' demo/multistage-plan/decision.md

# JSON validation using Python standard library.
# Checks performed:
#   * schema_version == 1
#   * non‑empty subject string
#   * non‑empty sources array
#   * at least four stages with required kinds (discovery, decision, validation, review)
#   * acceptance object with three required keys
python3 - <<'PY'
import json, sys, pathlib

path = pathlib.Path('demo/multistage-plan/evidence.json')
try:
    data = json.loads(path.read_text())
except Exception as e:
    sys.exit(f'Failed to parse JSON: {e}')

# schema_version
if data.get('schema_version') != 1:
    sys.exit('Invalid schema_version')

# subject
subject = data.get('subject')
if not isinstance(subject, str) or not subject:
    sys.exit('Invalid subject')

# sources
sources = data.get('sources')
if not isinstance(sources, list) or not sources:
    sys.exit('Invalid sources')

# stages
stages = data.get('stages')
if not isinstance(stages, list) or len(stages) < 4:
    sys.exit('Invalid stages')
required_kinds = {'discovery', 'decision', 'validation', 'review'}
found = {s.get('kind') for s in stages if isinstance(s, dict)}
if not required_kinds.issubset(found):
    sys.exit('Missing required stage kinds')

# acceptance keys
accept = data.get('acceptance')
if not isinstance(accept, dict):
    sys.exit('Missing acceptance')
required_keys = {'human_readable', 'machine_readable', 'executable'}
if not required_keys.issubset(accept.keys()):
    sys.exit('Missing acceptance keys')
PY

echo 'multi-stage demo: PASS'
exit 0
