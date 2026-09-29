+++
id = "gap-84e98e"
kind = "gap"
title = "[plan-audit T1-05] Automatic complexity-gated reviewer injection in plan runs"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-05: Implement automatic reviewer injection"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-05: Implement automatic reviewer injection"
anchors = ["classify_complexity", "ACP WorkflowPipeline review_strictness", "backlog #398"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Review only happens if authored as tasks; ACP WorkflowPipeline has implement->review->merge but the plan runner does not. Use classify_complexity(): Trivial none, Standard QuickReviewer, Complex Architect+Auditor. Backlog #398.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-05: Implement automatic reviewer injection`
- `tmp/archive/plan-audit-2026-09-23/06-auto-review-pipeline.md`

How to verify: Check graph_execution for reviewer task injection.
