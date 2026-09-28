+++
id = "gap-185ed0"
kind = "gap"
title = "TUX W3-02/W3-05: Syntax highlighting in detail modals and system clipboard"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/archive/tui-ux-audit/30-OPEN-GAPS.md#Remaining Product Work (not audit gaps)"
discovered_from = "audit:tmp/archive/tui-ux-audit/30-OPEN-GAPS.md#Remaining Product Work (not audit gaps)"
anchors = ["tui/modals/plan_detail.rs", "views/logs_view.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Code blocks in plan/task detail modals are not syntax highlighted (needs syntect/tree-sitter); log `y` copies to state only, not the system clipboard (needs arboard/copypasta).

Imported without verification from:
- `tmp/archive/tui-ux-audit/30-OPEN-GAPS.md#Remaining Product Work (not audit gaps)`
- `tmp/archive/tui-ux-audit/20-IMPLEMENTATION-CHECKLIST.md#Wave 3 — Stretch (P3) — Partial implementation`

How to verify: grep syntect/arboard in roko-cli Cargo.toml.
