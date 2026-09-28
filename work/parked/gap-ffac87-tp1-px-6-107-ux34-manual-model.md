+++
id = "gap-ffac87"
kind = "gap"
title = "TP1-PX.6 / #107 UX34: Manual model/backend overrides and routing learning"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-learn/cascade-router"]
created = 2026-09-03
updated = 2026-09-28
source = "tmp/tui-parity/00-INDEX.md#Parity-only items (from checklist, not in v2 priorities)"
discovered_from = "audit:tmp/tui-parity/00-INDEX.md#Parity-only items (from checklist, not in v2 priorities)"
anchors = ["runtime_feedback/routing.rs model_source", "ModelChoiceSource::Override"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
No warning when --force-backend bypasses learned routing; override-learning isolation (#90) was claimed implemented 09-03, but UX34 remains listed as open work.

Imported without verification from:
- `tmp/tui-parity/00-INDEX.md#Parity-only items (from checklist, not in v2 priorities)`
- `tmp/tui-parity/audits/mori-parity-status.md#Full Checklist`
- `tmp/archive/dogfood-audit-2026-09-03/06-status-update-2026-09-03.md#Summary`

How to verify: Check routing.rs for the override early-return and a CLI warning.
