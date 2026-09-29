+++
id = "gap-7d16ea"
kind = "gap"
title = "[plan-audit T2-12] Batch review TUI modal is a stub"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-12: Implement batch review TUI modal"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-12: Implement batch review TUI modal"
anchors = ["crates/roko-cli/src/tui/modals/batch_review.rs", "backlog #406"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Replace stub tui/modals/batch_review.rs with an interactive modal: syntax-colored diff panel, per-task approve/reject/skip, keyboard navigation. Backlog #406.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-12: Implement batch review TUI modal`
- `tmp/archive/plan-audit-2026-09-23/14-tui-visualization.md`

How to verify: Read batch_review.rs.
