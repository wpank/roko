+++
id = "gap-23fa38"
kind = "gap"
title = "TP1-MX.2 / #255: Approval flow not connected end to end"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/tui"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/tui-parity/00-INDEX.md#Medium-term items (from v1 audit)"
discovered_from = "audit:tmp/tui-parity/00-INDEX.md#Medium-term items (from v1 audit)"
anchors = ["ApprovalChannel", "RunConfig.approval", "backlog #255"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
RunConfig.approval was never read, ApprovalChannel never created, BatchReview had no input handler (later deleted as dead), and the Graph engine rejected the approval TUI in TTYs; remote approval also lacks an API sequence (see HPA-04 §7.10).

Imported without verification from:
- `tmp/tui-parity/00-INDEX.md#Medium-term items (from v1 audit)`
- `tmp/tui-parity/00-INDEX.md#3. New items from the UX audit not in this tracker`
- `tmp/archive/dogfood-audit-2026-09-03/03-dogfood-runbook.md#Approval TUI auto-enables but is disconnected`
- `tmp/archive/ux-audit-2026-09-21/18-backlog-cross-reference.md#5. Wire ApprovalChannel to TUI`

How to verify: grep ApprovalChannel construction; run a plan requiring approval under the Graph engine.
