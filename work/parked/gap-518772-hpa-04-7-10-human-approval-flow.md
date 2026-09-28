+++
id = "gap-518772"
kind = "gap"
title = "HPA-04 §7.10: Human-approval flow is TUI-only with no documented API sequence"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-serve/inbox"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.10. Approval/human-in-the-loop flow not exposed via API"
discovered_from = "audit:tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.10. Approval/human-in-the-loop flow not exposed via API"
anchors = ["/api/plans/:id/tasks/:task_id/review", "InboxItemReceived"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Review endpoint and Inbox events exist, but detecting a needed approval, rendering it and submitting a decision remotely requires stitching undocumented endpoints.

Imported without verification from:
- `tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.10. Approval/human-in-the-loop flow not exposed via API`

Warning: every file this item cites is gone (`/api/plans/:id/tasks/:task_id/review`) — likely obsolete or moved.

How to verify: Trace an approval-required gate end-to-end over HTTP.
