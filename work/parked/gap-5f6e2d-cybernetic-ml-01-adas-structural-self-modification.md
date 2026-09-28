+++
id = "gap-5f6e2d"
kind = "gap"
title = "[cybernetic ML-01] ADAS / structural self-modification (L4 learning loop) has no code"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-learn"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/cybernetic-audit/28-feedback-loops.md#ML-01: ADAS / Structural Self-Modification (L4 Learning Loop)"
discovered_from = "audit:tmp/archive/cybernetic-audit/28-feedback-loops.md#ML-01: ADAS / Structural Self-Modification (L4 Learning Loop)"
anchors = ["docs/v2-depth/10/self-improvement-frameworks.md", "R04 meta-agent lineage"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Automated Design of Agentic Systems (search over architectures, tool configs, prompt structures, learning-rule compositions) is documented only. R04 meta-agent lifecycle explicitly excludes ADAS/Loop 4. Audit rates gap HIGH.

Imported without verification from:
- `tmp/archive/cybernetic-audit/28-feedback-loops.md#ML-01: ADAS / Structural Self-Modification (L4 Learning Loop)`

Warning: every file this item cites is gone (`docs/v2-depth/10/self-improvement-frameworks.md`) — likely obsolete or moved.

How to verify: grep -ri 'adas\|loop 4' crates/ for implementation beyond docs.
