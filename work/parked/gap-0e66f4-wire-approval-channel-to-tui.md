+++
id = "gap-0e66f4"
kind = "gap"
title = "Wire approval channel to TUI"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/tui"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#4.6 Wire approval channel to TUI"
discovered_from = "audit:tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#4.6 Wire approval channel to TUI"
anchors = ["ApprovalChannel", "BatchReview"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
All plumbing exists: `ApprovalChannel` IPC, modal renderer, key handler, `drain_approval_requests` loop. But `RunConfig.approval` is never read, `ApprovalChannel` is never created, `BatchReview` modal has no input handler, and the graph engine rejects approval TUI.

Imported without verification from:
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#4.6 Wire approval channel to TUI`

How to verify: Source: UX audit (approval TUI); TUI parity MX.2. Check the described code path for: All plumbing exists: `ApprovalChannel` IPC, modal renderer, key handler, `drain_approval_requests` loop. But `RunConfig.approval` is never read…
