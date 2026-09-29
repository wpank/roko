+++
id = "q-e8a3ba"
kind = "question"
title = "EFF-06: Efficiency-TUI 33-item fix set implemented on a side branch; merge unconfirmed"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/tui"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/efficienty-tui-audit/00-SUMMARY.md#Resolution (2026-09-01)"
discovered_from = "audit:tmp/archive/efficienty-tui-audit/00-SUMMARY.md#Resolution (2026-09-01)"
anchors = ["tui/widgets/cost_by_model.rs", "roko-learn/src/event_subscriber.rs", "codex_cli/stream.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
All 33 fixes (F6 config cache, codex model identity, connected efficiency push, model-classification generalization, cost data quality, dead-code sweep) were implemented on branch efficiency-tui-fixes; 09-25 still saw TUI tokens/cost at $0.00.

Imported without verification from:
- `tmp/archive/efficienty-tui-audit/00-SUMMARY.md#Resolution (2026-09-01)`
- `tmp/archive/efficienty-tui-audit/06-fix-checklist.md#Implementation status (2026-09-01)`
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P1-7. Tokens and cost are structurally $0.00`

A source claims this was fixed; confirm against current code before closing.

How to verify: git log --all for efficiency-tui-fixes merge; check is_final_turn filter and codex SystemInit model.
