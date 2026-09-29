+++
id = "gap-aa56c4"
kind = "gap"
title = "Express mode (skip strategist for trivial fixes) not wired"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/runner"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/39-ROADMAP.md#3.3 Half-Implemented Features"
discovered_from = "audit:docs/v3/39-ROADMAP.md#3.3 Half-Implemented Features"
anchors = ["roko develop"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Express Mode has scaffolding/types but lacks final wiring (backlog #05). Listed under roadmap §3.3 half-implemented features.

Imported without verification from:
- `docs/v3/39-ROADMAP.md#3.3 Half-Implemented Features`

How to verify: Locate the scaffolding named in the roadmap row and confirm no runtime caller.
