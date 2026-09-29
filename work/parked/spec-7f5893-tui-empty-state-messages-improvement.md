+++
id = "spec-7f5893"
kind = "spec"
title = "TUI Empty State Messages Improvement"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/tui"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/236-tui-empty-state-messages.md#236 — TUI Empty State Messages Improvement"
discovered_from = "audit:tmp/backlog/archive/236-tui-empty-state-messages.md#236 — TUI Empty State Messages Improvement"
anchors = ["crates/roko-cli/src/tui/views/dashboard_view.rs", "crates/roko-cli/src/tui/views/plans_view.rs", "crates/roko-cli/src/tui/views/agents_view.rs", "crates/roko-cli/src/tui/views/config_view.rs", "crates/roko-cli/src/tui/views/learning_view.rs", "crates/roko-cli/src/tui/views/affect_view.rs", "crates/roko-cli/src/tui/widgets/token_sparkline.rs", "token_sparkline.rs:183"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
14 generic "no data" / "waiting for data" strings provide no context about what the user should do to populate the view or why data is absent.. The TUI has 48 empty-state strings across all views. 14 of these are high-priority because they appear in panels that are visible during a normal `plan…

Imported without verification from:
- `tmp/backlog/archive/236-tui-empty-state-messages.md#236 — TUI Empty State Messages Improvement`

Some cited files are gone: `crates/roko-cli/src/tui/views/affect_view.rs`.

How to verify: Check: During a plan run, empty panels show messages that explain why they are empty and what will populate them.; In standalone dashboard mode, empty panels tell the user what command to run.; No generic "no data" / "waiting for data" strings… [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): S | 4 |]
