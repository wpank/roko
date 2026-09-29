+++
id = "gap-50edb1"
kind = "gap"
title = "FAST self-development lane still experimental (benchmarks + release/CI coverage pending)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/fast"]
created = 2026-08-16
updated = 2026-09-28
source = "docs/v2/00-INDEX.md:6"
discovered_from = "audit:docs/v2/00-INDEX.md:6"
anchors = ["dev.sh fast"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
The opt-in FAST lane (./dev.sh fast) is implemented but 'remains experimental pending representative benchmarks and release/CI coverage'.

Imported without verification from:
- `docs/v2/00-INDEX.md:6`
- `docs/v2/29-FAST-DEVELOPMENT.md:339`

How to verify: Look for FAST benchmark evidence under tmp/ and CI jobs exercising ./dev.sh fast.
