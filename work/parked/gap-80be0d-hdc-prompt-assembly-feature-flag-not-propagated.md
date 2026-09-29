+++
id = "gap-80be0d"
kind = "gap"
title = "HDC prompt-assembly feature flag not propagated downstream"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-compose/hdc"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/39-ROADMAP.md#3.3 Half-Implemented Features"
discovered_from = "audit:docs/v3/39-ROADMAP.md#3.3 Half-Implemented Features"
anchors = ["roko-compose hdc feature"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
HDC Prompt Assembly needs its feature flag propagated to downstream consumers; the CLI audit called the HDC feature-flag pipeline 'severed'. Listed under roadmap §3.3 half-implemented features.

Imported without verification from:
- `docs/v3/39-ROADMAP.md#3.3 Half-Implemented Features`
- `tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#6. CLI Audit`

How to verify: Locate the scaffolding named in the roadmap row and confirm no runtime caller.
