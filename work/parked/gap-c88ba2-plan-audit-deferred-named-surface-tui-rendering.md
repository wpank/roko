+++
id = "gap-c88ba2"
kind = "gap"
title = "[plan-audit deferred] Named-surface TUI rendering (E37 residual)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/00-INDEX.md#Gaps Deferred to Product Roadmap"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/00-INDEX.md#Gaps Deferred to Product Roadmap"
anchors = ["V2Surface", "Workbench/Inbox/Canvas/Minimap/Autonomy projections", "/api/projections/workbench", "tabs.rs V2Surface"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Full named-surface TUI rendering remains an E37 product residual (portal spec covers it).

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/00-INDEX.md#Gaps Deferred to Product Roadmap`
- `tmp/archive/ux-audit-2026-09-21/18-backlog-cross-reference.md#16. Named surface projections`
- `tmp/archive/ux-audit-2026-09-21/18-backlog-cross-reference.md#Items Needing New Backlog Specs`

Warning: every file this item cites is gone (`/api/projections/workbench`, `Workbench/Inbox/Canvas/Minimap/Autonomy`) — likely obsolete or moved.

How to verify: Check TUI for dedicated named-surface views. / grep TUI for projection rendering.

Merged 2 mined candidates: m3-160, m4-171.
