+++
id = "gap-d81125"
kind = "gap"
title = "TP2-34 #110/#122/#148: TUI architectural debt (JSONL-as-truth, legacy page system, god objects)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/tui-parity2/34-BACKLOG-CROSSWALK.md#Missing"
discovered_from = "audit:tmp/tui-parity2/34-BACKLOG-CROSSWALK.md#Missing"
anchors = ["tui/app.rs", "tui/state.rs", "tui/dashboard.rs", "tui/pages/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
JSONL readers/writers remain the standalone truth instead of StateHub (#110); PageId/PageScaffold legacy pages remain (now documented CLI-text-only); app.rs/state.rs/dashboard.rs remain god objects (#148).

Imported without verification from:
- `tmp/tui-parity2/34-BACKLOG-CROSSWALK.md#Missing`
- `tmp/archive/efficienty-tui-audit/05-dead-and-placeholder-code.md#6. Checklist`

Some cited files are gone: `tui/app.rs`, `tui/pages/`, `tui/state.rs`.

How to verify: wc -l the TUI god files; grep PageScaffold users.
