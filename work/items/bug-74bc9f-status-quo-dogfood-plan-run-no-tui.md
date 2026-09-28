+++
id = "bug-74bc9f"
kind = "bug"
title = "[status-quo dogfood] `plan run --no-tui` prints no progress to stdout"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/commands/plan"]
created = 2026-09-10
updated = 2026-09-28
source = "tmp/archive/status-quo-audit-2026-09-21/10-dogfood-proof.md#Known display gaps (not failures)"
discovered_from = "audit:tmp/archive/status-quo-audit-2026-09-21/10-dogfood-proof.md#Known display gaps (not failures)"
anchors = ["roko plan run --no-tui", "crates/roko-cli/src/graph_execution/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Headless plan runs produce no stdout; progress only visible via TUI or by inspecting .roko/state/graph/ afterwards.

Imported without verification from:
- `tmp/archive/status-quo-audit-2026-09-21/10-dogfood-proof.md#Known display gaps (not failures)`

How to verify: Run `roko plan run <dir> --no-tui` and observe stdout.
