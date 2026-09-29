+++
id = "gap-8844bd"
kind = "gap"
title = "[plan-audit deferred] Dynamic role reassignment (Conductor escalation)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-conductor"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/00-INDEX.md#Gaps Deferred to Product Roadmap"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/00-INDEX.md#Gaps Deferred to Product Roadmap"
anchors = ["backlog #401"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Roles are fixed in TOML; no runtime escalation (e.g. Implementer -> Architect on repeated failure). Depends on conductor wiring (#401).

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/00-INDEX.md#Gaps Deferred to Product Roadmap`
- `tmp/archive/plan-audit-2026-09-23/01-GAP-MATRIX.md`

How to verify: Check graph dispatch for role escalation policy.
