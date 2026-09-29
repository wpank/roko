+++
id = "gap-004aa7"
kind = "gap"
title = "[plan-audit deferred] Cross-plan task-level DAG"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/plan"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/00-INDEX.md#Gaps Deferred to Product Roadmap"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/00-INDEX.md#Gaps Deferred to Product Roadmap"
anchors = ["CrossPlanDag"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Only plan-level dependencies exist; individual task dependencies across plans are unsupported (deferred: plan-level deps cover ~90%).

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/00-INDEX.md#Gaps Deferred to Product Roadmap`

How to verify: Check tasks.toml depends_on for cross-plan task refs.
